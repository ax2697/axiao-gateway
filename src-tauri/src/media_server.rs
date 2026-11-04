use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use crate::store::data_dir;

static RUNNING: AtomicBool = AtomicBool::new(false);

/// 全服务共用一个 tokio runtime。
/// 原来每个请求都 Builder::new_current_thread().build() 一次，
/// 并发一高就是不停建/毁 runtime，线程和内存都扛不住。
static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// 同时在跑的上游请求上限（再多就排队，避免把本机打满）
const MAX_INFLIGHT: usize = 12;

/// 单个请求体上限。
///
/// **这是安全边界，不是性能参数**：`Content-Length` 完全由客户端声明，
/// 下面按它 `vec![0u8; length]` 分配缓冲区。不设上限的话，随便一个本机
/// 进程发 `Content-Length: 4294967295` 就能让我们去申请 4GB 内存，
/// 一次请求把进程打崩（资源耗尽，不是普通性能问题）。
/// 图生图的 base64 也就几 MB，64MB 足够宽松。
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// 单行请求头上限。理由同上：`read_line` 没有长度上限，
/// 客户端发一条不带换行的超长"头"就会一直往 String 里堆。
const MAX_LINE_BYTES: usize = 8 * 1024;

/// 全部请求头合计上限，防止用无数个小行绕开单行限制。
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// 同时存在的连接上限。每个连接一个线程（默认栈 2MB），
/// 不封顶的话几千个空连接就能把线程和内存耗光。
const MAX_CONNS: usize = 128;

static CONNS: AtomicUsize = AtomicUsize::new(0);

struct ConnGuard;
impl Drop for ConnGuard {
    fn drop(&mut self) {
        CONNS.fetch_sub(1, Ordering::SeqCst);
    }
}

fn conn_guard() -> Option<ConnGuard> {
    if CONNS.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
        CONNS.fetch_sub(1, Ordering::SeqCst);
        return None;
    }
    Some(ConnGuard)
}

/// 带长度上限地读一行。
///
/// 返回 `Ok(0)` 表示正常 EOF；返回 `Err` 表示这一行超长（没读到换行就撞上限）。
fn read_line_capped<R: BufRead>(r: &mut R, out: &mut String) -> std::io::Result<usize> {
    let n = (r as &mut R).take(MAX_LINE_BYTES as u64).read_line(out)?;
    if n == 0 {
        return Ok(0);
    }
    if !out.ends_with('\n') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "header line too long",
        ));
    }
    Ok(n)
}

static INFLIGHT: Mutex<usize> = Mutex::new(0);
/// 无指定上游时的轮转指针（多 Key 分摊）
static PICK_RR: AtomicUsize = AtomicUsize::new(0);
static PORT: Mutex<u16> = Mutex::new(7864);

fn rt() -> &'static tokio::runtime::Runtime {
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

/// 拿一个并发名额；拿不到就等（最多等 20 秒），超时直接 429 让客户端重试
struct Slot;
impl Drop for Slot {
    fn drop(&mut self) {
        if let Ok(mut n) = INFLIGHT.lock() {
            *n = n.saturating_sub(1);
        }
    }
}
/// 拿一个并发名额；拿不到就等一会儿，超时直接 429 让客户端重试。
///
/// 等待时间不能长：排队期间**连接是被占住的**，一堆请求一起排队会把
/// 连接名额（MAX_CONNS）和线程全占满，外部表现就是"服务卡住几十秒"。
/// 本机场景 12 个并发已经很宽松，等 3 秒还排不上，说明确实在打满，
/// 这时候快速失败让客户端重试，比让它干等更有用。
fn acquire_slot() -> Option<Slot> {
    let start = std::time::Instant::now();
    loop {
        if let Ok(mut n) = INFLIGHT.lock() {
            if *n < MAX_INFLIGHT {
                *n += 1;
                return Some(Slot);
            }
        }
        if start.elapsed() > Duration::from_secs(3) {
            return None;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn upstreams() -> Vec<Value> {
    std::fs::read_to_string(data_dir().join("upstreams.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<Value>>(&s).ok())
        .unwrap_or_default()
}

fn pick(id: Option<&str>) -> Result<(String, String, String, String), String> {
    let list = upstreams();
    let found = match id {
        Some(want) if !want.is_empty() => list
            .iter()
            .find(|u| u.get("id").and_then(|x| x.as_str()) == Some(want))
            .cloned(),
        _ => {
            // 同平台多把 Key：轮着用，别老压在同一条上
            let mut cands: Vec<Value> = list
                .iter()
                .filter(|u| u.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true))
                .filter(|u| {
                    let has_key = u
                        .get("apiKey")
                        .and_then(|x| x.as_str())
                        .map(|k| !k.is_empty())
                        .unwrap_or(false);
                    let local = u
                        .get("baseUrl")
                        .and_then(|x| x.as_str())
                        .map(|b| b.contains("127.0.0.1") || b.contains("localhost"))
                        .unwrap_or(false);
                    has_key || local
                })
                .cloned()
                .collect();
            if cands.is_empty() {
                cands = list.clone();
            }
            if cands.is_empty() {
                None
            } else {
                let i = PICK_RR.fetch_add(1, Ordering::Relaxed) % cands.len();
                Some(cands.swap_remove(i))
            }
        }
    };
    let u = found.ok_or_else(|| "还没有配置上游".to_string())?;
    Ok((
        u.get("baseUrl").and_then(|x| x.as_str()).unwrap_or("").trim_end_matches('/').to_string(),
        u.get("apiKey").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        u.get("protocol").and_then(|x| x.as_str()).unwrap_or("openai").to_string(),
        u.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string(),
    ))
}

fn auth(rb: reqwest::RequestBuilder, protocol: &str, key: &str) -> reqwest::RequestBuilder {
    if key.is_empty() {
        rb
    } else if protocol == "anthropic" {
        rb.header("x-api-key", key).header("anthropic-version", "2023-06-01")
    } else {
        rb.bearer_auth(key)
    }
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.split(',').last().unwrap_or(s).trim();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("非法 base64".into()),
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

async fn handle_images(body: Value) -> Result<Value, String> {
    let up_id = body.get("upstream").and_then(|x| x.as_str());
    let (base, key, protocol, name) = pick(up_id)?;

    // 除 upstream / _auth 外整体透传，各家私有字段都不丢
    let mut payload = body.clone();
    if let Some(o) = payload.as_object_mut() {
        o.remove("upstream");
        o.remove("_auth");
        if !o.contains_key("model") {
            o.insert("model".into(), json!("gpt-image-1"));
        }
        if !o.contains_key("size") {
            o.insert("size".into(), json!("1024x1024"));
        }
        if !o.contains_key("n") {
            o.insert("n".into(), json!(1));
        }
    }
    let prompt = payload.get("prompt").and_then(|x| x.as_str()).unwrap_or("").to_string();
    if prompt.is_empty() {
        return Err("缺少 prompt".into());
    }
    let img = payload.get("image").and_then(|x| x.as_str()).unwrap_or("").to_string();

    let client = crate::httpc::client_for(&base)?;
    let rb = if img.is_empty() {
        client.post(format!("{base}/images/generations")).json(&payload)
    } else {
        let bytes = b64_decode(&img)?;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("input.png")
            .mime_str("image/png")
            .map_err(|e| e.to_string())?;
        let mut form = reqwest::multipart::Form::new().part("image", part);
        if let Some(o) = payload.as_object() {
            for (k, v) in o {
                if k == "image" {
                    continue;
                }
                let text = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                form = form.text(k.clone(), text);
            }
        }
        let path = if name.contains("智谱") || name.to_lowercase().contains("bigmodel") {
            "/images/generations"
        } else {
            "/images/edits"
        };
        client.post(format!("{base}{path}")).multipart(form)
    };
    let resp = auth(rb, &protocol, &key)
        .timeout(Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if status >= 400 {
        return Err(format!("上游 {status}: {v}"));
    }
    Ok(v)
}

/// 音乐 / 音频生成：整体透传给上游的 `/audio/generations`。
///
/// 各家的音乐接口字段差异很大（时长、风格、歌词、是否带人声），
/// 这里不猜、不改、不加默认值，除了 `upstream` / `_auth` 两个本应用内部字段。
async fn handle_audio(body: Value) -> Result<Value, String> {
    let up_id = body.get("upstream").and_then(|x| x.as_str());
    let (base, key, protocol, _name) = pick(up_id)?;

    let mut payload = body.clone();
    if let Some(o) = payload.as_object_mut() {
        o.remove("upstream");
        o.remove("_auth");
    }
    let prompt = payload.get("prompt").and_then(|x| x.as_str()).unwrap_or("");
    if prompt.is_empty() {
        return Err("缺少 prompt".into());
    }

    let client = crate::httpc::client_for(&base)?;
    let resp = auth(
        client.post(format!("{base}/audio/generations")),
        &protocol,
        &key,
    )
    .json(&payload)
    .timeout(Duration::from_secs(180))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if status >= 400 {
        return Err(format!("上游 {status}: {v}"));
    }
    Ok(v)
}

async fn handle_videos(body: Value) -> Result<Value, String> {
    let up_id = body.get("upstream").and_then(|x| x.as_str());
    let (base, key, protocol, name) = pick(up_id)?;
    let zhipu = name.contains("智谱") || name.to_lowercase().contains("bigmodel");
    let bridge = name.contains("豆包桥") || base.contains(":9090");

    let mut payload = body.clone();
    if let Some(o) = payload.as_object_mut() {
        o.remove("upstream");
        o.remove("_auth");
        if !o.contains_key("model") {
            o.insert("model".into(), json!(if zhipu { "cogvideox-flash" } else { "sora-2" }));
        }
    }
    let path = if zhipu || bridge { "/videos/generations" } else { "/videos" };
    let client = crate::httpc::client_for(&base)?;
    let resp = auth(client.post(format!("{base}{path}")), &protocol, &key)
        .json(&payload)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let mut v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if status >= 400 {
        return Err(format!("上游 {status}: {v}"));
    }
    if let Some(o) = v.as_object_mut() {
        o.insert("_upstream".into(), json!(name));
        o.insert("_zhipu".into(), json!(zhipu));
        o.insert("_bridge".into(), json!(bridge));
    }
    Ok(v)
}

async fn handle_video_get(task: &str, upstream: Option<&str>, zhipu: bool) -> Result<Value, String> {
    let (base, key, protocol, _n) = pick(upstream)?;
    let bridge = base.contains(":9090");
    let path = if zhipu {
        format!("/async-result/{task}")
    } else if bridge {
        format!("/videos/generations/{task}")
    } else {
        format!("/videos/{task}")
    };
    let client = crate::httpc::client_for(&base)?;
    let resp = auth(client.get(format!("{base}{path}")), &protocol, &key)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if status >= 400 {
        return Err(format!("上游 {status}: {v}"));
    }
    Ok(v)
}

fn respond(stream: &mut TcpStream, code: u16, body: &Value) {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".into());
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        500 => "Internal Server Error",
        _ => "OK",
    };
    // 把请求 id 带回去：用户在客户端看到它，就能直接拿去日志里搜
    let rid = crate::trace::get().unwrap_or_default();
    let _ = write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nX-Request-Id: {rid}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Expose-Headers: X-Request-Id\r\nConnection: close\r\n\r\n{}",
        text.as_bytes().len(),
        text
    );
    let _ = stream.flush();
    // 先关写端再让它被 drop：客户端能读到完整的正文和 EOF。
    // 直接关闭的话，内核发现还有未读数据会发 RST，某些客户端会看到
    // "连接被中止"而不是我们的错误信息。
    let _ = stream.shutdown(std::net::Shutdown::Write);
}

/// 写一个 chunked 分片。
///
/// 为什么必须用 chunked：SSE 流的长度事先不知道，如果像普通响应那样发
/// `Content-Length`，客户端会一直等"收满这么多字节"才渲染 —— 表现就是
/// **一个 token 都看不到，直到整段结束才一次性蹦出来**。chunked 让每个分片
/// 一到就能被客户端消费。
fn write_chunk(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    write!(stream, "{:X}\r\n", data.len())?;
    stream.write_all(data)?;
    stream.write_all(b"\r\n")?;
    stream.flush()
}

/// 流式对话专用通道：先回 200 头，然后边收上游边写客户端。
fn handle_chat_stream(stream: &mut TcpStream, body: Value) {
    let t0 = std::time::Instant::now();

    if crate::access::required() {
        let got = body.get("_auth").and_then(|x| x.as_str()).unwrap_or("");
        match crate::access::verify(got) {
            Ok(k) => crate::access::record_ok(&k.id),
            Err(d) => {
                respond(
                    stream,
                    d.status(),
                    &json!({"error": {"message": d.message(), "type": "permission_error"}}),
                );
                return;
            }
        }
    }

    let _slot = match acquire_slot() {
        Some(s) => s,
        None => {
            respond(
                stream,
                429,
                &json!({"error": {"message": "并发已满，请稍后重试", "type": "rate_limit_error"}}),
            );
            return;
        }
    };

    let hint = body
        .get("upstream")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let want_usage = body.get("stream_options").is_some();
    let ctx = match crate::chat_stream::prepare(
        body.clone(),
        hint.as_deref(),
        want_usage,
        crate::trace::get().unwrap_or_else(crate::trace::new_id),
    ) {
        Ok(c) => c,
        Err(e) => {
            crate::gwarn!("流式准备失败：{e}");
            respond(
                stream,
                500,
                &json!({"error": {"message": e, "type": "config_error"}}),
            );
            return;
        }
    };

    let rid = crate::trace::get().unwrap_or_default();
    let head = format!(
        "HTTP/1.1 200 OK\r\n\
Content-Type: text/event-stream; charset=utf-8\r\n\
Cache-Control: no-cache, no-transform\r\n\
X-Accel-Buffering: no\r\n\
X-Request-Id: {rid}\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Headers: *\r\n\
Access-Control-Expose-Headers: X-Request-Id\r\n\
Transfer-Encoding: chunked\r\n\
Connection: close\r\n\r\n"
    );
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let _ = stream.flush();

    // 上游在 tokio 线程池里跑，主线程专心往 socket 写，两边用 channel 串起来。
    // 这样上游吐一帧我们就能立刻发出去，不会攒着。
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    rt().spawn(async move {
        let _ = crate::chat_stream::pump(ctx, |s| {
            let _ = tx.send(s.to_string());
        })
        .await;
    });

    let mut sent = 0usize;
    for f in rx {
        if write_chunk(stream, f.as_bytes()).is_err() {
            crate::gwarn!("客户端提前断开，已发 {sent} 帧");
            break;
        }
        sent += 1;
    }
    let _ = stream.write_all(b"0\r\n\r\n");
    let _ = stream.flush();
    crate::glog!("流式连接结束 | 共发 {sent} 帧 | {}ms", t0.elapsed().as_millis());
}


fn route(method: &str, path: &str, body: &Value) -> (u16, Value) {
    let path_only = path.split('?').next().unwrap_or(path);
    let query: std::collections::HashMap<String, String> = path
        .split('?')
        .nth(1)
        .map(|q| {
            q.split('&')
                .filter_map(|kv| {
                    let mut it = kv.splitn(2, '=');
                    match (it.next(), it.next()) {
                        (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                        _ => None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // 鉴权：控制面里配了 Key 才校验
    // 访问 Key 校验（没配置任何 Key 就是全开，保持便携版开箱即用）。
    // `/health` 例外：它是给监控和探测用的，不含敏感操作，要求带 Key 的话
    // 配好 Key 之后探活就全废了（这个问题就是被测试抓出来的）。
    if path_only != "/health" && crate::access::required() {
        let got = body.get("_auth").and_then(|x| x.as_str()).unwrap_or("");
        match crate::access::verify(got) {
            Ok(k) => crate::access::record_ok(&k.id),
            Err(d) => {
                crate::gwarn!("访问被拒：{}", d.message());
                return (
                    d.status(),
                    json!({"error": {"message": d.message(), "type": "permission_error"}}),
                );
            }
        }
    }

    // 并发闸门：满了先排队，排 20 秒还拿不到就 429，让客户端自己重试
    let _slot = match acquire_slot() {
        Some(s) => s,
        None => {
            return (
                429,
                json!({"error": {"message": "并发已满，请稍后重试", "type": "rate_limit_error"}}),
            )
        }
    };

    match (method, path_only) {
        ("POST", "/v1/images/generations") | ("POST", "/v1/images/edits") => {
            match rt().block_on(handle_images(body.clone())) {
                Ok(v) => (200, v),
                Err(e) => (500, json!({"error": {"message": e}})),
            }
        }
        ("POST", "/v1/chat/completions") => {
            // 流式请求在 serve() 里就被分流走了，走到这里的都是非流式
            let hint = body.get("upstream").and_then(|x| x.as_str()).map(|s| s.to_string());
            match rt().block_on(crate::chat_proxy::complete(body.clone(), hint, crate::trace::get())) {
                Ok((status, data)) => (status, data),
                Err(e) => (500, json!({"error": {"message": e}})),
            }
        }
        ("POST", "/v1/audio/generations") => match rt().block_on(handle_audio(body.clone())) {
            Ok(v) => (200, v),
            Err(e) => (500, json!({"error": {"message": e}})),
        },
        ("POST", "/v1/videos") => match rt().block_on(handle_videos(body.clone())) {
            Ok(v) => (200, v),
            Err(e) => (500, json!({"error": {"message": e}})),
        },
        ("GET", p) if p.starts_with("/v1/videos/") => {
            let task = p.trim_start_matches("/v1/videos/").to_string();
            let zhipu = query.get("zhipu").map(|v| v == "1").unwrap_or(false);
            let up = query.get("upstream").map(|s| s.as_str());
            match rt().block_on(handle_video_get(&task, up, zhipu)) {
                Ok(v) => (200, v),
                Err(e) => (500, json!({"error": {"message": e}})),
            }
        }
        ("GET", "/health") => (
            200,
            json!({"ok": true, "service": "axiao-gateway", "upstreams": upstreams().len(),
                     "conns": CONNS.load(Ordering::SeqCst), "maxConns": MAX_CONNS,
                     "maxBodyMb": MAX_BODY_BYTES / (1024 * 1024),
                     // 缓存统计和熔断状态放这儿，运维和自动化测试都能直接看到
                     "authRequired": crate::access::required(),
                     "cache": crate::cache::stats(),
                     "breaker": crate::breaker::snapshot(),
                     "endpoints": ["/v1/chat/completions", "/v1/images/generations", "/v1/videos", "/v1/audio/generations"]}),
        ),
        ("GET", "/v1/models") => {
            let list: Vec<Value> = upstreams()
                .iter()
                .map(|u| json!({"id": u.get("id"), "owned_by": u.get("name")}))
                .collect();
            (200, json!({"object": "list", "data": list}))
        }
        _ => (
            404,
            json!({
                "error": {"message": "未知路由",
                          "hint": "支持：POST /v1/images/generations、POST /v1/images/edits、POST /v1/videos、GET /v1/videos/{id}、GET /health"}
            }),
        ),
    }
}

fn serve(mut stream: TcpStream) {
    // 连接数到顶就立刻回 503 关掉，别让客户端干等，也别再开线程
    let _conn = match conn_guard() {
        Some(g) => g,
        None => {
            let body = r#"{"error":{"message":"too many connections","type":"rate_limit_error"}}"#;
            let _ = write!(
                stream,
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json; charset=utf-8\r\nRetry-After: 1\r\nX-Request-Id: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                crate::trace::new_id(),
                body.len(),
                body
            );
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);
            return;
        }
    };

    // 读超时：连上之后这么久还没发完整请求就断开。
    // 这个值直接决定"被大量空连接占满后多久能恢复"，所以不能太长；
    // 本机回环发个请求是毫秒级，8 秒已经非常宽松。
    // 注意它只约束**读**，不影响流式响应（那时候我们在写）。
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    match read_line_capped(&mut reader, &mut request_line) {
        Ok(0) | Err(_) => return,
        Ok(_) => {}
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut length = 0usize;
    let mut auth_header = String::new();
    // 客户端自带 X-Request-Id 时优先用它的，方便跟对方系统的日志对齐；
    // 没带就自己生成一个，返回在响应头里。
    let mut req_id = String::new();
    let mut header_total = 0usize;
    loop {
        let mut line = String::new();
        let n = match read_line_capped(&mut reader, &mut line) {
            Ok(0) => break,   // EOF
            Ok(n) => n,
            Err(_) => return, // 单行超长
        };
        header_total += n;
        if header_total > MAX_HEADER_BYTES {
            return;
        }
        if line.trim().is_empty() {
            break;
        }
        let lower = line.to_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = line.to_lowercase().strip_prefix("authorization:") {
            auth_header = v.trim().replace("bearer ", "");
        }
        if let Some(v) = lower.strip_prefix("x-request-id:") {
            let v = v.trim();
            // 只收合理长度的可见字符，避免被塞垃圾进日志
            if !v.is_empty() && v.len() <= 64 && v.chars().all(|c| c.is_ascii_graphic()) {
                req_id = v.to_string();
            }
        }
    }

    // 从这里开始，本线程的日志都会自动带上这个 id
    if req_id.is_empty() {
        req_id = crate::trace::new_id();
    }
    let _trace = crate::trace::begin(&req_id);

    if length > MAX_BODY_BYTES {
        crate::gwarn!("拒绝超大请求体：Content-Length={length}（上限 {MAX_BODY_BYTES}）");
        let body = r#"{"error":{"message":"request body too large","type":"invalid_request_error"}}"#;
        let _ = write!(
            stream,
            "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json; charset=utf-8\r\nX-Request-Id: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            crate::trace::get().unwrap_or_default(),
            body.len(),
            body
        );
        let _ = stream.flush();
        let _ = stream.shutdown(std::net::Shutdown::Write);
        return;
    }

    let mut raw = vec![0u8; length];
    if length > 0 && reader.read_exact(&mut raw).is_err() {
        return;
    }
    let mut body: Value = serde_json::from_slice(&raw).unwrap_or_else(|_| json!({}));
    if !auth_header.is_empty() {
        if let Some(o) = body.as_object_mut() {
            o.insert("_auth".into(), json!(auth_header));
        }
    }

    // CORS 预检
    if method == "OPTIONS" {
        let _ = write!(
            stream,
            "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: GET,POST,OPTIONS\r\nContent-Length: 0\r\n\r\n"
        );
        return;
    }

    let path_only = path.split('?').next().unwrap_or("/").to_string();

    // 流式对话单独分流：它要「边收上游边写客户端」，
    // 不能塞进 route() 那个「先把结果算完再一次性回复」的模型里。
    if method == "POST"
        && path_only == "/v1/chat/completions"
        && body
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    {
        crate::glog!("→ POST {path_only}  (stream)");
        handle_chat_stream(&mut stream, body);
        return;
    }

    let t0 = std::time::Instant::now();
    let (code, payload) = route(&method, &path, &body);
    crate::glog!(
        "→ {method} {path_only} → {code} ({}ms)",
        t0.elapsed().as_millis()
    );
    respond(&mut stream, code, &payload);
}

#[tauri::command]
pub fn media_server_start(port: Option<u16>) -> Result<Value, String> {
    if RUNNING.load(Ordering::SeqCst) {
        let p = *PORT.lock().map_err(|e| e.to_string())?;
        return Ok(json!({"running": true, "port": p, "base": format!("http://127.0.0.1:{p}/v1")}));
    }
    let p = port.unwrap_or(7864);
    let listener = TcpListener::bind(("127.0.0.1", p))
        .map_err(|e| format!("端口 {p} 绑定失败：{e}（换一个端口试试）"))?;
    if let Ok(mut slot) = PORT.lock() {
        *slot = p;
    }
    RUNNING.store(true, Ordering::SeqCst);
    crate::glog!("本地创作 API 已启动：http://127.0.0.1:{p}/v1");
    // 安全提醒：监听在 loopback 但**不等于**只有本应用能调——本机上任何
    // 进程（包括浏览器里的网页通过 fetch）都能访问。没设 Key 就是全开。
    // 这是便携版「开箱即用」的取舍，但必须在日志里说清楚。
    if !crate::access::required() {
        crate::gwarn!(
            "未设置访问 Key：本机任何程序都能调用这个接口（包括浏览器网页）。\
             只给自己用没问题；要把网关给局域网/别人用，先去「控制面」设 Key。"
        );
    }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            std::thread::spawn(move || serve(stream));
        }
        crate::glog!("本地创作 API listener 已退出");
    });
    Ok(json!({"running": true, "port": p, "base": format!("http://127.0.0.1:{p}/v1")}))
}

#[tauri::command]
pub fn media_server_stop() -> Result<Value, String> {
    RUNNING.store(false, Ordering::SeqCst);
    Ok(json!({"running": false}))
}

/// 供「融合组件」页判断本地创作 API 是否在跑
pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn media_server_status() -> Result<Value, String> {
    let running = RUNNING.load(Ordering::SeqCst);
    let p = *PORT.lock().map_err(|e| e.to_string())?;
    Ok(json!({
        "running": running,
        "port": p,
        "base": format!("http://127.0.0.1:{p}/v1"),
        "upstreams": upstreams().len(),
        "authRequired": crate::access::required(),
    }))
}
