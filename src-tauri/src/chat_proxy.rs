//! 对话转发核心：给「本地创作 API」（7864）提供 /v1/chat/completions。
//!
//! 为什么单独一份而不是复用 `gateway::gateway_chat`：后者绑在 `tauri::State` 上，
//! 只能在 Tauri 命令里调；HTTP 服务线程拿不到 State。这里直接读 `upstreams.json`，
//! 复用同一套扩展点：模型映射、协议转换、成本计价、账本记账。
//!
//! 这一版加了**失败自动降级**：以前一个渠道挂了就把错直接甩给客户端（502/500），
//! 现在会把候选依次试一遍，谁先成功用谁。取舍参考了 LiteLLM Router 与 Portkey：
//! - 只有「限流 / 服务端故障 / 连接失败」才换渠道（`breaker::should_fallback`）
//! - 4xx（除了 429）是请求本身的问题，换十个渠道也是同样的错，直接返回
//! - 失败过的渠道按 `breaker` 的规则冷却，冷却期间跳过

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::store::data_dir;

/// 无指定上游时的轮转起点。
/// 除了分摊多 Key，它还有一个作用：让降级时"先试谁"不至于永远是同一条。
static PICK_RR: AtomicUsize = AtomicUsize::new(0);

fn upstreams() -> Vec<Value> {
    std::fs::read_to_string(data_dir().join("upstreams.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default()
}

fn endpoint(base: &str, protocol: &str) -> String {
    let b = base.trim_end_matches('/');
    let has_version = b
        .rsplit('/')
        .next()
        .map(|seg| {
            let s = seg.to_ascii_lowercase();
            s.len() > 1 && s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit())
        })
        .unwrap_or(false);
    let path = match protocol {
        "anthropic" => "messages",
        "responses" => "responses",
        _ => "chat/completions",
    };
    if has_version {
        format!("{b}/{path}")
    } else {
        format!("{b}/v1/{path}")
    }
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

/// 这个上游现在能不能用来发请求（启用了、有地址、有 Key 或是本机地址）
fn usable(u: &Value) -> bool {
    if !u.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true) {
        return false;
    }
    let has_base = u
        .get("baseUrl")
        .and_then(|x| x.as_str())
        .map(|b| !b.is_empty())
        .unwrap_or(false);
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
    has_base && (has_key || local)
}

/// 路由策略。默认轮转；`health` 会按最近成功率排序。
fn routing_strategy() -> String {
    crate::extras::get("routing")
        .get("strategy")
        .and_then(|x| x.as_str())
        .unwrap_or("rr")
        .to_string()
}

/// 把一个渠道排个序：(分组, 成功率倒数, 平均耗时)
///
/// 分组 0 = 没有任何调用记录的新渠道，**排最前面**。不给它机会的话，
/// 一旦有了历史数据它就永远轮不到，等于白配。
fn rank_key(id: &str, stats: &std::collections::HashMap<String, (f64, u64)>) -> (u8, i64, u64) {
    match stats.get(id) {
        None => (0, 0, 0),
        Some((rate, ms)) => (1, -((*rate * 100.0) as i64), *ms),
    }
}

/// 按尝试顺序给出候选。
///
/// - 指定了 upstream：只试它（用户明确要用这个，替它做主换掉不合适）
/// - 没指定：从轮转点开始环形排所有可用渠道，第一个失败了就顺着往下试
fn candidates(hint: Option<&str>) -> Vec<Value> {
    let list = upstreams();
    if let Some(w) = hint.filter(|x| !x.is_empty()) {
        return list
            .into_iter()
            .filter(|u| u.get("id").and_then(|x| x.as_str()) == Some(w))
            .collect();
    }
    let mut v: Vec<Value> = list.into_iter().filter(usable).collect();
    if v.len() <= 1 {
        return v;
    }

    if routing_strategy() == "health" {
        // 按最近 300 条账本算出来的成功率排。新渠道没有记录 → 排最前。
        let stats: std::collections::HashMap<String, (f64, u64)> =
            crate::gateway::upstream_health()
                .ok()
                .and_then(|v| v.as_array().cloned())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| {
                            let id = x.get("id").and_then(|s| s.as_str())?.to_string();
                            let rate = x.get("okRate").and_then(|s| s.as_f64()).unwrap_or(0.0);
                            let ms = x.get("avgMs").and_then(|s| s.as_u64()).unwrap_or(0);
                            Some((id, (rate, ms)))
                        })
                        .collect()
                })
                .unwrap_or_default();
        v.sort_by_key(|u| {
            let id = u.get("id").and_then(|x| x.as_str()).unwrap_or("");
            rank_key(id, &stats)
        });
        return v;
    }

    // 默认：加权轮转（起点每次往后挪一格，多 Key 才能分摊）
    let start = PICK_RR.fetch_add(1, Ordering::Relaxed) % v.len();
    v.rotate_left(start);
    v
}

/// 追加一行账本（与 gateway 共用同一个 ledger.jsonl）
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

/// 单次尝试：组装请求、发出去、把响应转回 OpenAI 形状。
///
/// 不管熔断也不记账 —— 那些由调用方根据结果决定，因为要跨多次尝试汇总。
async fn try_once(up: &Value, body: &Value) -> Result<(u16, Value), String> {
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

    let mut b = body.clone();
    if let Some(o) = b.as_object_mut() {
        o.remove("upstream");
        // 上游几乎都按非流式回，这里统一要整段
        o.remove("stream");
        o.remove("stream_options");
    }

    let model = b.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let real_model = crate::extras::model_map()
        .get(&model)
        .cloned()
        .unwrap_or_else(|| model.clone());
    if let Some(o) = b.as_object_mut() {
        o.insert("model".into(), json!(real_model));
    }

    let (url, payload) = if protocol == "gemini" {
        let (m, g) = crate::convert::to_gemini(&b);
        (
            format!("{base}/models/{}:generateContent", m),
            g,
        )
    } else if protocol == "anthropic" {
        (endpoint(&base, &protocol), crate::convert::to_anthropic(&b))
    } else {
        (endpoint(&base, &protocol), b.clone())
    };

    let client = crate::httpc::client_for(&base)?;
    let resp = auth(client.post(&url), &protocol, &key)
        .json(&payload)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("上游请求失败：{e}"))?;

    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let mut data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({"raw": text}));

    if status < 300 {
        if protocol == "anthropic" {
            data = crate::convert::anthropic_to_openai(&data, &real_model);
        } else if protocol == "gemini" {
            data = crate::convert::gemini_to_openai(&data, &real_model);
        }
    }
    Ok((status, data))
}

/// 转发一次对话。返回 (HTTP 状态, OpenAI 形状的响应体)。
///
/// 会按顺序尝试多个渠道：成功即返回，可降级的失败就换下一个。
pub async fn complete(
    body: Value,
    upstream_hint: Option<String>,
    req_id: Option<String>,
) -> Result<(u16, Value), String> {
    // 异步链路里读不到 thread-local 的请求 id（tokio 会切线程），
    // 所以由调用方显式传进来，日志和账本都带上它。
    let rid = req_id.unwrap_or_else(|| crate::trace::new_id());

    // 缓存命中就直接回：不用选上游、不发请求、不花 token
    if let Some(hit) = crate::cache::get(&body) {
        crate::glog!("[{rid}] 缓存命中，跳过上游请求");
        return Ok((200, hit));
    }

    let cands = candidates(upstream_hint.as_deref());
    if cands.is_empty() {
        return Err("没有可用上游：先去「上游管理」添加并填 Key".into());
    }

    let t0 = Instant::now();
    let mut tried: Vec<String> = Vec::new();
    let mut last_err = String::new();

    // 输入长度只估一次。预检要靠它判断"这个渠道的窗口装不装得下"，
    // 免得把注定失败的请求真发出去（白等一次往返、还可能白花输入的钱）。
    let est_tokens = crate::modelctx::estimate_request(&body);

    for (i, up) in cands.iter().enumerate() {
        let id = up.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let name = up
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or(&id)
            .to_string();
        // 这个渠道实际会用的模型（可能被 model_map 换过），预检要按它算
        let eff_model = {
            let m = body.get("model").and_then(|x| x.as_str()).unwrap_or("");
            crate::extras::model_map()
                .get(m)
                .cloned()
                .unwrap_or_else(|| m.to_string())
        };

        // 预检：模型窗口装不下就直接跳过。认不出的模型返回 None，不拦。
        if let Some(false) = crate::modelctx::fits(&eff_model, est_tokens) {
            if let Some(w) = crate::modelctx::context_window(&eff_model) {
                tried.push(format!("{name}(窗口 {w} 装不下 ~{est_tokens})"));
                crate::glog!(
                    "[{rid}] 跳过 {name}：估算 {est_tokens} tokens 超过窗口 {w}"
                );
                continue;
            }
        }

        if !crate::breaker::available(&id) {
            tried.push(format!("{name}(冷却中)"));
            continue;
        }

        match try_once(up, &body).await {
            Ok((status, data)) => {
                if status < 300 {
                    crate::breaker::record_success(&id);
                    crate::cache::put(&body, &data);
                    record_ledger(&id, &body, &data, status, t0, &tried, &rid);
                    if !tried.is_empty() {
                        crate::glog!(
                            "[{rid}] 降级成功：跳过 {} 后由 {name} 服务（{}ms）",
                            tried.join("、"),
                            t0.elapsed().as_millis()
                        );
                    }
                    return Ok((status, data));
                }

                let kind = crate::classify::classify(status, &data);
                crate::breaker::record_failure(&id, kind, status);
                let can_fallback = i + 1 < cands.len() && kind.fallback_worthy();
                if can_fallback {
                    last_err = format!("{name} {}（{status}）", kind.hint());
                    tried.push(format!("{name}→{}", kind.as_str()));
                    crate::gwarn!(
                        "[{rid}] {name} {}（{status}），换下一个上游重试",
                        kind.hint()
                    );
                    continue;
                }
                // 不该降级（客户端错误）或没有更多候选：如实返回
                record_ledger(&id, &body, &data, status, t0, &tried, &rid);
                return Ok((status, data));
            }
            Err(e) => {
                crate::breaker::record_network_error(&id, &e);
                last_err = e.clone();
                tried.push(format!("{name}→连接失败"));
                if i + 1 < cands.len() {
                    crate::gwarn!("[{rid}] {name} 连接失败：{e}，换下一个上游重试");
                    continue;
                }
            }
        }
    }

    crate::gwarn!(
        "[{rid}] 所有候选上游都失败（{}），最后错误：{last_err}",
        tried.join("、")
    );
    Err(format!(
        "所有候选上游都失败：{}。最后错误：{last_err}",
        tried.join("、")
    ))
}

/// 记账：与网关同一本账，成本按单价表算
fn record_ledger(
    up_id: &str,
    body: &Value,
    data: &Value,
    status: u16,
    t0: Instant,
    tried: &[String],
    req_id: &str,
) {
    let model = body
        .get("model")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let usage = data.get("usage").cloned().unwrap_or_else(|| json!({}));
    let pin = usage.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let pout = usage
        .get("completion_tokens")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    append_ledger(json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "accountId": "",
        "upstream": up_id,
        "model": model,
        "promptTokens": pin,
        "completionTokens": pout,
        "cost": crate::pricing::cost_usd(&model, pin, pout),
        "status": status,
        "latencyMs": t0.elapsed().as_millis() as u64,
        // 请求 id：客户端响应头里那个，能直接把这次调用和日志对上
        "requestId": req_id,
        // 降级链：这次请求之前跳过了哪些渠道，排障时一眼看出是主渠道挂了
        "skipped": tried,
    }));
}

/// 把整段响应合成 OpenAI 风格的 SSE。
///
/// 说明：真流式走 `chat_stream`，这里只给"上游不支持流式"的极端情况兜底。
#[allow(dead_code)]
pub fn to_sse(full: &Value, model: &str) -> String {
    let content = full
        .pointer("/choices/0/message/content")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let id = full.get("id").and_then(|x| x.as_str()).unwrap_or("chatcmpl-local");
    let created = chrono::Utc::now().timestamp();

    let mut out = String::new();
    out.push_str(&format!(
        "data: {}\n\n",
        json!({
            "id": id, "object": "chat.completion.chunk", "created": created, "model": model,
            "choices": [{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]
        })
    ));
    let chars: Vec<char> = content.chars().collect();
    for chunk in chars.chunks(40) {
        let piece: String = chunk.iter().collect();
        out.push_str(&format!(
            "data: {}\n\n",
            json!({
                "id": id, "object": "chat.completion.chunk", "created": created, "model": model,
                "choices": [{"index":0,"delta":{"content":piece},"finish_reason":null}]
            })
        ));
    }
    out.push_str(&format!(
        "data: {}\n\n",
        json!({
            "id": id, "object": "chat.completion.chunk", "created": created, "model": model,
            "choices": [{"index":0,"delta":{},"finish_reason":"stop"}]
        })
    ));
    out.push_str("data: [DONE]\n\n");
    out
}
