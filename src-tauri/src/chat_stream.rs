//! 真增量流式转发：上游吐一片，我们立刻转一片给客户端。
//!
//! 和 `chat_proxy::complete` + `to_sse` 的区别：
//! - 老做法：等上游把整段生成完 → 一次性切成 40 字一帧发出去。
//!   首字延迟 = 整段生成时间，用起来跟"不流式"没区别。
//! - 这里：拿到一个 TCP 分片就解析、就转发。首字延迟 = 上游第一个 token 的时间。
//!
//! 三种上游协议各走各的解码：
//! - `openai`    : 帧原样透传（保住 reasoning_content 之类各家私有字段）
//! - `anthropic` : content_block_delta → OpenAI delta
//! - `gemini`    : :streamGenerateContent?alt=sse → OpenAI delta
//!
//! 全程用 `Response::chunk()` 拉流，不引入 futures-util 依赖。

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::store::data_dir;

fn upstreams() -> Vec<Value> {
    std::fs::read_to_string(data_dir().join("upstreams.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default()
}

fn has_version(b: &str) -> bool {
    b.rsplit('/')
        .next()
        .map(|seg| {
            let s = seg.to_ascii_lowercase();
            s.len() > 1 && s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit())
        })
        .unwrap_or(false)
}

fn pick(hint: Option<&str>) -> Result<Value, String> {
    let list = upstreams();
    let found = match hint.filter(|h| !h.is_empty()) {
        Some(want) => list
            .iter()
            .find(|u| u.get("id").and_then(|x| x.as_str()) == Some(want))
            .cloned(),
        // 没指定时优先挑启用的、有 Key 的，别再选到空壳账号
        None => list
            .iter()
            .filter(|u| u.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true))
            .find(|u| {
                u.get("apiKey")
                    .and_then(|x| x.as_str())
                    .map(|k| !k.is_empty())
                    .unwrap_or(false)
                    || u.get("baseUrl")
                        .and_then(|x| x.as_str())
                        .map(|b| b.contains("127.0.0.1") || b.contains("localhost"))
                        .unwrap_or(false)
            })
            .cloned()
            .or_else(|| {
                list.iter()
                    .find(|u| u.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true))
                    .cloned()
            }),
    };
    found.ok_or_else(|| "没有可用上游：先去「上游管理」添加并填 Key".to_string())
}

fn auth(rb: reqwest::RequestBuilder, protocol: &str, key: &str) -> reqwest::RequestBuilder {
    if key.is_empty() {
        rb
    } else if protocol == "anthropic" {
        rb.header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    } else if protocol == "gemini" {
        rb.header("x-goog-api-key", key)
    } else {
        rb.bearer_auth(key)
    }
}

/// 一次流式请求的全部上下文
pub struct StreamCtx {
    /// 请求追踪 id。异步链路读不到 thread-local，只能一路带着。
    pub req_id: String,
    pub url: String,
    pub protocol: String,
    pub key: String,
    pub payload: Value,
    pub model: String,
    pub upstream_id: String,
    pub upstream_name: String,
}

/// 组装流式请求。这里只做准备，不发网络请求，方便先回 200 头再开流。
pub fn prepare(
    body: Value,
    hint: Option<&str>,
    client_wants_usage: bool,
    req_id: String,
) -> Result<StreamCtx, String> {
    let up = pick(hint)?;
    let base = up
        .get("baseUrl")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    if base.is_empty() {
        return Err("上游没填 Base URL".into());
    }
    let key = up.get("apiKey").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let protocol = up
        .get("protocol")
        .and_then(|x| x.as_str())
        .unwrap_or("openai")
        .to_string();
    let upstream_id = up.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let upstream_name = up.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();

    let mut b = body;
    if let Some(o) = b.as_object_mut() {
        o.remove("upstream");
        o.insert("stream".into(), json!(true));
    }
    let model = b.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string();
    // 模型别名映射：客户端写 gpt-4o，实际打到 deepseek-chat 这类
    let real_model = crate::extras::model_map()
        .get(&model)
        .cloned()
        .unwrap_or_else(|| model.clone());
    if let Some(o) = b.as_object_mut() {
        o.insert("model".into(), json!(real_model));
    }

    let (url, payload) = if protocol == "gemini" {
        let (m, g) = crate::convert::to_gemini(&b);
        let url = if has_version(&base) {
            format!("{base}/models/{m}:streamGenerateContent?alt=sse")
        } else {
            format!("{base}/v1beta/models/{m}:streamGenerateContent?alt=sse")
        };
        (url, g)
    } else if protocol == "anthropic" {
        let url = if has_version(&base) {
            format!("{base}/messages")
        } else {
            format!("{base}/v1/messages")
        };
        let mut p = crate::convert::to_anthropic(&b);
        // to_anthropic 面向「非流式转换」场景，末尾会主动去掉 stream（避免上游 400）。
        // 这里是流式专用通道，必须补回来，否则上游按一次性 JSON 回，
        // 我们拿到的是整段 JSON 而不是 SSE 帧 —— 表现就是"一帧都收不到"。
        if let Some(o) = p.as_object_mut() {
            o.insert("stream".into(), json!(true));
        }
        (url, p)
    } else {
        let url = if has_version(&base) {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
        };
        // 让上游在最后一帧带 usage，好记账（不支持的厂商会忽略这个字段）
        if client_wants_usage {
            if let Some(o) = b.as_object_mut() {
                o.insert("stream_options".into(), json!({"include_usage": true}));
            }
        }
        (url, b.clone())
    };

    Ok(StreamCtx {
        req_id,
        url,
        protocol,
        key,
        payload,
        model: real_model,
        upstream_id,
        upstream_name,
    })
}

/// 逐帧解析累积的缓冲区，返回被完整切出的行
fn drain_lines(buf: &mut Vec<u8>) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        match buf.iter().position(|b| *b == b'\n') {
            Some(i) => {
                let raw: Vec<u8> = buf.drain(..=i).collect();
                let s = String::from_utf8_lossy(&raw[..raw.len().saturating_sub(1)])
                    .trim_end_matches('\r')
                    .to_string();
                out.push(s);
            }
            None => break,
        }
    }
    out
}

/// 从一个 SSE 行里取 `data:` 后面那段 JSON 文本
fn sse_data(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let rest = t.strip_prefix("data:")?;
    Some(rest.trim_start())
}

/// 流式主循环。
///
/// `emit` 每收到一个可下发的 SSE 帧就调一次（已带 `data: ... \n\n` 包装），
/// 由调用方负责写 socket + flush。返回时保证已经下发结束帧和 `[DONE]`。
pub async fn pump<F: FnMut(&str)>(ctx: StreamCtx, mut emit: F) -> Result<(), String> {
    let t0 = Instant::now();
    let client = crate::httpc::stream_client_for(&ctx.url)?;

    let rid = &ctx.req_id;
    crate::glog!(
        "[{rid}] 流式 → {} | 上游={} ({}) | 协议={} | 模型={}",
        ctx.url,
        ctx.upstream_name,
        ctx.upstream_id,
        ctx.protocol,
        ctx.model
    );

    let resp = auth(client.post(&ctx.url), &ctx.protocol, &ctx.key)
        .json(&ctx.payload)
        .send()
        .await
        .map_err(|e| {
            crate::gwarn!("[{rid}] 流式上游连接失败：{e}");
            format!("上游连接失败：{e}")
        })?;

    let status = resp.status().as_u16();
    if status >= 400 {
        let body = resp.text().await.unwrap_or_default();
        let brief: String = body.chars().take(400).collect();
        crate::gwarn!("[{rid}] 流式上游 {status}：{brief}");
        // 上游报错也要以 SSE 形式回，否则客户端解析会直接崩
        emit(&frame(json!({
            "error": {"message": format!("上游 {status}: {brief}"), "type": "upstream_error"}
        })));
        emit("data: [DONE]\n\n");
        return Ok(());
    }
    crate::glog!("[{rid}] 流式上游已连上，状态 {status}，开始转发（首字耗时 {}ms）", t0.elapsed().as_millis());

    let id = format!("chatcmpl-{}", chrono::Utc::now().timestamp_millis());
    let created = chrono::Utc::now().timestamp();

    // 首帧：告诉客户端"assistant 开始说话了"，有些 SDK 靠这个初始化
    emit(&chunk_frame(&id, created, &ctx.model, json!({"role": "assistant", "content": ""}), None));

    let mut buf: Vec<u8> = Vec::new();
    let mut resp = resp;
    let mut frames_out = 0usize;
    let mut chars_out = 0usize;
    let mut first_token_ms: Option<u128> = None;
    let mut finish: Option<String> = None;
    let mut usage = json!({});
    // Anthropic 的工具调用要按 index 攒参数碎片
    let mut tool_args: std::collections::BTreeMap<u64, String> = Default::default();
    let mut saw_done = false;

    loop {
        // 90 秒没新数据才判超时。流式不能用「整请求总超时」——
        // 长回复本来就要几分钟，总超时会把正常生成掐断。
        let next = tokio::time::timeout(Duration::from_secs(90), resp.chunk()).await;
        let piece = match next {
            Err(_) => {
                crate::gwarn!("[{rid}] 流式读取超时（90s 无新数据），已转发 {frames_out} 帧");
                break;
            }
            Ok(Err(e)) => {
                crate::gwarn!("[{rid}] 流式读取中断：{e}，已转发 {frames_out} 帧");
                break;
            }
            Ok(Ok(None)) => break, // 上游正常收尾
            Ok(Ok(Some(b))) => b,
        };
        buf.extend_from_slice(&piece);

        for line in drain_lines(&mut buf) {
            let Some(data) = sse_data(&line) else { continue };
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                saw_done = true;
                break;
            }
            let v: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            match ctx.protocol.as_str() {
                "anthropic" => {
                    let ty = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
                    match ty {
                        "message_start" => {
                            if let Some(u) = v.pointer("/message/usage") {
                                usage = u.clone();
                            }
                        }
                        "content_block_start" => {
                            let idx = v.get("index").and_then(|x| x.as_u64()).unwrap_or(0);
                            let blk = v.get("content_block").cloned().unwrap_or(json!({}));
                            match blk.get("type").and_then(|x| x.as_str()) {
                                Some("text") => {
                                    let txt = blk.get("text").and_then(|x| x.as_str()).unwrap_or("");
                                    if !txt.is_empty() {
                                        emit_delta(&mut emit, &id, created, &ctx.model, json!({ "content": txt }));
                                        chars_out += txt.chars().count();
                                        frames_out += 1;
                                        first_token_ms.get_or_insert(t0.elapsed().as_millis());
                                    }
                                }
                                Some("tool_use") => {
                                    let name = blk.get("name").and_then(|x| x.as_str()).unwrap_or("");
                                    emit_delta(
                                        &mut emit,
                                        &id,
                                        created,
                                        &ctx.model,
                                        json!({"tool_calls": [{
                                            "index": idx, "id": blk.get("id"), "type": "function",
                                            "function": {"name": name, "arguments": ""}
                                        }]}),
                                    );
                                    frames_out += 1;
                                }
                                _ => {}
                            }
                        }
                        "content_block_delta" => {
                            let idx = v.get("index").and_then(|x| x.as_u64()).unwrap_or(0);
                            let d = v.get("delta").cloned().unwrap_or(json!({}));
                            match d.get("type").and_then(|x| x.as_str()) {
                                Some("text_delta") => {
                                    let txt = d.get("text").and_then(|x| x.as_str()).unwrap_or("");
                                    if !txt.is_empty() {
                                        emit_delta(
                                            &mut emit,
                                            &id,
                                            created,
                                            &ctx.model,
                                            json!({ "content": txt }),
                                        );
                                        chars_out += txt.chars().count();
                                        frames_out += 1;
                                        first_token_ms.get_or_insert(t0.elapsed().as_millis());
                                    }
                                }
                                Some("input_json_delta") => {
                                    let part = d
                                        .get("partial_json")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !part.is_empty() {
                                        tool_args.entry(idx).or_default().push_str(&part);
                                    }
                                }
                                _ => {}
                            }
                        }
                        "message_delta" => {
                            if let Some(r) = v.pointer("/delta/stop_reason").and_then(|x| x.as_str()) {
                                finish = Some(match r {
                                    "max_tokens" => "length".into(),
                                    "tool_use" => "tool_calls".into(),
                                    _ => "stop".into(),
                                });
                            }
                            if let Some(u) = v.get("usage") {
                                let mut merged = usage.clone();
                                if let (Some(a), Some(b)) =
                                    (merged.as_object_mut(), u.as_object())
                                {
                                    for (k, val) in b {
                                        a.insert(k.clone(), val.clone());
                                    }
                                }
                                usage = merged;
                            }
                        }
                        _ => {}
                    }
                }
                "gemini" => {
                    if let Some(c) = v.pointer("/candidates/0/content/parts") {
                        if let Some(arr) = c.as_array() {
                            for p in arr {
                                let txt = p.get("text").and_then(|x| x.as_str()).unwrap_or("");
                                if !txt.is_empty() {
                                    emit_delta(
                                        &mut emit,
                                        &id,
                                        created,
                                        &ctx.model,
                                        json!({ "content": txt }),
                                    );
                                    chars_out += txt.chars().count();
                                    frames_out += 1;
                                    first_token_ms.get_or_insert(t0.elapsed().as_millis());
                                }
                            }
                        }
                    }
                    if let Some(fr) = v.pointer("/candidates/0/finishReason").and_then(|x| x.as_str()) {
                        finish = Some(match fr {
                            "MAX_TOKENS" => "length".into(),
                            _ => "stop".into(),
                        });
                    }
                    if let Some(u) = v.get("usageMetadata") {
                        usage = json!({
                            "prompt_tokens": u.get("promptTokenCount"),
                            "completion_tokens": u.get("candidatesTokenCount"),
                            "total_tokens": u.get("totalTokenCount"),
                        });
                    }
                }
                // openai：原样透传 delta，各家私有字段（reasoning_content 等）都不丢
                _ => {
                    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
                        usage = u.clone();
                    }
                    let Some(delta) = v.pointer("/choices/0/delta").cloned() else {
                        continue;
                    };
                    if delta.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                        // 空 delta 但可能带 finish_reason
                    } else {
                        if let Some(t) = delta.get("content").and_then(|x| x.as_str()) {
                            chars_out += t.chars().count();
                            first_token_ms.get_or_insert(t0.elapsed().as_millis());
                        }
                        emit_delta(&mut emit, &id, created, &ctx.model, delta);
                        frames_out += 1;
                    }
                    if let Some(fr) = v
                        .pointer("/choices/0/finish_reason")
                        .and_then(|x| x.as_str())
                    {
                        finish = Some(fr.to_string());
                    }
                }
            }
        }
        if saw_done {
            break;
        }
    }

    // Anthropic 的工具参数是碎片攒的，最后一次性补一帧，客户端才拼得起来
    for (idx, args) in &tool_args {
        emit_delta(
            &mut emit,
            &id,
            created,
            &ctx.model,
            json!({"tool_calls": [{"index": idx, "function": {"arguments": args}}]}),
        );
        frames_out += 1;
    }

    // 收尾帧
    let usage = normalize_usage(usage);
    emit(&chunk_frame(
        &id,
        created,
        &ctx.model,
        json!({}),
        Some(finish.unwrap_or_else(|| "stop".to_string())),
    ));

    // usage 帧：OpenAI 规范是 choices 为空 + usage
    if usage.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        emit(&frame(json!({
            "id": id, "object": "chat.completion.chunk", "created": created, "model": ctx.model,
            "choices": [], "usage": usage,
        })));
    }
    emit("data: [DONE]\n\n");

    let ms = t0.elapsed().as_millis();
    crate::glog!(
        "[{rid}] 流式完成 | 上游={} | 帧={} | 字符={} | 首字={}ms | 总耗时={}ms",
        ctx.upstream_name,
        frames_out,
        chars_out,
        first_token_ms.unwrap_or(0),
        ms
    );

    // 记账：上游没给 usage 就用字符数粗估（1 token ≈ 1.5 汉字 / 4 字符）
    let pin = usage
        .get("prompt_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or_else(|| {
            ctx.payload
                .get("messages")
                .map(|m| m.to_string().chars().count() as u64 / 2)
                .unwrap_or(0)
        });
    let pout = usage
        .get("completion_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(chars_out as u64);
    append_ledger(json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "accountId": "",
        "upstream": ctx.upstream_id,
        "model": ctx.model,
        "promptTokens": pin,
        "completionTokens": pout,
        "cost": crate::pricing::cost_usd(&ctx.model, pin, pout),
        "status": if saw_done || chars_out > 0 { 200 } else { 499 },
        "latencyMs": ms,
        "requestId": rid,
        "stream": true,
        "firstTokenMs": first_token_ms.unwrap_or(0),
    }));

    Ok(())
}

fn frame(v: Value) -> String {
    format!("data: {v}\n\n")
}

/// 把各家 usage 字段统一成 OpenAI 命名。
///
/// 客户端（含本应用自己的统计页）只认 `prompt_tokens` / `completion_tokens` /
/// `total_tokens`。Anthropic 回的是 `input_tokens` / `output_tokens`，
/// 不转的话客户端读出来全是 0 —— 就是「Token 统计一直是 0」那类问题的来源。
fn normalize_usage(u: Value) -> Value {
    let mut u = u;
    if !u.is_object() {
        return json!({});
    }
    if let Some(o) = u.as_object_mut() {
        if !o.contains_key("prompt_tokens") {
            if let Some(v) = o.get("input_tokens").cloned() {
                o.insert("prompt_tokens".into(), v);
            }
        }
        if !o.contains_key("completion_tokens") {
            if let Some(v) = o.get("output_tokens").cloned() {
                o.insert("completion_tokens".into(), v);
            }
        }
        if !o.contains_key("total_tokens") {
            let p = o.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
            let c = o.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
            if p + c > 0 {
                o.insert("total_tokens".into(), json!(p + c));
            }
        }
        o.remove("input_tokens");
        o.remove("output_tokens");
    }
    u
}

fn chunk_frame(id: &str, created: i64, model: &str, delta: Value, finish: Option<String>) -> String {
    frame(json!({
        "id": id, "object": "chat.completion.chunk", "created": created, "model": model,
        "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]
    }))
}

fn emit_delta<F: FnMut(&str)>(emit: &mut F, id: &str, created: i64, model: &str, delta: Value) {
    emit(&chunk_frame(id, created, model, delta, None));
}

fn append_ledger(entry: Value) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(line) = serde_json::to_string(&entry) {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_dir().join("ledger.jsonl"))
        {
            let _ = writeln!(f, "{line}");
        }
    }
}
