use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::model::{ChatRequest, ChatResult, LedgerEntry, Upstream};
use crate::state::AppState;
use crate::store::data_dir;

pub struct Gateway {
    pub upstreams: Vec<Upstream>,
    pub cooldown: HashMap<String, i64>,
    pub sticky: HashMap<String, String>,
    /// 加权轮转用的自增指针（同平台多 Key 时按此分摊）
    pub rr: std::sync::atomic::AtomicUsize,
}

fn default_upstreams() -> Vec<Upstream> {
    vec![Upstream {
        id: "workbuddy".into(),
        name: "WorkBuddy / CodeBuddy".into(),
        // 官方对话端点是 https://www.workbuddy.cn/v2/chat/completions
        // （copilot.tencent.com 也能通，但 /v1/* 一律 404）
        base_url: "https://www.workbuddy.cn/v2".into(),
        api_key: None,
        protocol: "openai".into(),
        enabled: true,
        weight: 1.0,
        model: None,
    }]
}

impl Gateway {
    pub fn new() -> Self {
        let file = data_dir().join("upstreams.json");
        let mut upstreams = fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<Upstream>>(&s).ok())
            .unwrap_or_else(default_upstreams);
        // 老版本默认写的是 copilot.tencent.com + /v1/chat/completions（404），
        // 升级后自动把这条改到正确端点，不改用户自己加的其它上游。
        let mut changed = false;
        for u in upstreams.iter_mut() {
            if u.base_url.trim_end_matches('/') == "https://copilot.tencent.com" {
                u.base_url = "https://www.workbuddy.cn/v2".into();
                changed = true;
            }
        }
        let g = Self {
            upstreams,
            cooldown: HashMap::new(),
            sticky: HashMap::new(),
            rr: std::sync::atomic::AtomicUsize::new(0),
        };
        if changed {
            let _ = g.save();
        }
        g
    }

    pub fn save(&self) -> Result<(), String> {
        let _ = fs::create_dir_all(data_dir());
        let text = serde_json::to_string_pretty(&self.upstreams).map_err(|e| e.to_string())?;
        fs::write(data_dir().join("upstreams.json"), text).map_err(|e| e.to_string())
    }

    fn available(&self, protocol: &str) -> Vec<&Upstream> {
        let now = chrono::Utc::now().timestamp();
        self.upstreams
            .iter()
            .filter(|u| u.enabled)
            .filter(|u| u.protocol == protocol || protocol.is_empty())
            .filter(|u| self.cooldown.get(&u.id).map(|t| *t < now).unwrap_or(true))
            .collect()
    }

    pub fn select(&self, protocol: &str, hint: Option<&str>, session: Option<&str>) -> Option<Upstream> {
        if let Some(h) = hint {
            if let Some(u) = self.upstreams.iter().find(|u| u.id == h || u.name == h) {
                if u.enabled {
                    return Some(u.clone());
                }
            }
        }
        if let Some(s) = session {
            if let Some(id) = self.sticky.get(s) {
                if let Some(u) = self.upstreams.iter().find(|u| &u.id == id && u.enabled) {
                    return Some(u.clone());
                }
            }
        }
        // 加权轮转：同平台多条 Key 时轮着用，不再永远压在权重最高的那条上
        let mut pool = self.available(protocol);
        if pool.is_empty() {
            return None;
        }
        pool.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        let n = pool.len();
        let start = self.rr.load(std::sync::atomic::Ordering::Relaxed) % n;
        self.rr.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        pool.rotate_left(start);
        pool.first().map(|u| (*u).clone())
    }

    /// 候选链：第一条失败时依次退到后面的（同一协议、未冷却、按权重排序）
    pub fn candidates(&self, protocol: &str, skip: &[String]) -> Vec<Upstream> {
        let mut pool = self.available(protocol);
        pool.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        pool.into_iter()
            .filter(|u| !skip.iter().any(|s| s == &u.id))
            .cloned()
            .collect()
    }

    pub fn cool(&mut self, id: &str) {
        self.cooldown
            .insert(id.to_string(), chrono::Utc::now().timestamp() + 60);
    }
}

/// 拼出真正的请求地址。
/// 约定：`base_url` 里**可以带版本段**（…/v2、…/v4、…/api/paas/v4）。
/// 带了就不再补 `/v1`，没带才按 OpenAI 习惯补 `/v1`。
fn endpoint(base: &str, protocol: &str) -> String {
    let b = base.trim_end_matches('/');
    let has_version = b
        .rsplit('/')
        .next()
        .map(|seg| {
            let s = seg.to_ascii_lowercase();
            s.len() > 1
                && s.starts_with('v')
                && s[1..].chars().all(|c| c.is_ascii_digit())
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

fn ledger_path() -> PathBuf {
    data_dir().join("ledger.jsonl")
}

fn append_ledger(entry: &LedgerEntry) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(line) = serde_json::to_string(entry) {
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(ledger_path())
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// 上游健康度：从账本里聚合最近 300 条，给出成功率与平均耗时。
/// 账本本身只有经网关的请求，所以没走网关的上游会显示"无记录"，这是正常的。
#[tauri::command]
pub fn upstream_health() -> Result<Value, String> {
    use std::collections::HashMap;
    let text = fs::read_to_string(ledger_path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().rev().take(300).collect();

    #[derive(Default, Clone)]
    struct Acc {
        calls: u64,
        ok: u64,
        fail: u64,
        ms_sum: u64,
        ms_n: u64,
        last: String,
    }
    let mut map: HashMap<String, Acc> = HashMap::new();
    for line in lines {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let id = v.get("upstream").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let a = map.entry(id).or_default();
        a.calls += 1;
        // 老账本没有 status 字段：用「有没有拿到 completion」兜底判断，
        // 否则历史记录会被一律算成失败，健康度永远是 0%。
        let st = v.get("status").and_then(|x| x.as_u64()).unwrap_or(0);
        let completion = v.get("completion_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
        if (200..400).contains(&st) || (st == 0 && completion > 0) {
            a.ok += 1;
        } else {
            a.fail += 1;
        }
        let ms = v.get("latency_ms").and_then(|x| x.as_u64()).unwrap_or(0);
        if ms > 0 {
            a.ms_sum += ms;
            a.ms_n += 1;
        }
        if a.last.is_empty() {
            a.last = v.get("ts").and_then(|x| x.as_str()).unwrap_or("").to_string();
        }
    }

    let out: Vec<Value> = map
        .into_iter()
        .map(|(id, a)| {
            json!({
                "id": id,
                "calls": a.calls,
                "ok": a.ok,
                "fail": a.fail,
                "okRate": if a.calls > 0 { (a.ok as f64 / a.calls as f64 * 100.0).round() } else { 0.0 },
                "avgMs": if a.ms_n > 0 { a.ms_sum / a.ms_n } else { 0 },
                "last": a.last,
            })
        })
        .collect();
    Ok(json!(out))
}

#[tauri::command]
pub fn list_upstreams(state: tauri::State<'_, AppState>) -> Result<Vec<Upstream>, String> {
    let g = state.gateway.lock().map_err(|e| e.to_string())?;
    Ok(g.upstreams.clone())
}

#[tauri::command]
pub fn save_upstream(
    state: tauri::State<'_, AppState>,
    upstream: Upstream,
) -> Result<Vec<Upstream>, String> {
    let mut g = state.gateway.lock().map_err(|e| e.to_string())?;
    if let Some(u) = g.upstreams.iter_mut().find(|u| u.id == upstream.id) {
        *u = upstream;
    } else {
        g.upstreams.push(upstream);
    }
    g.save()?;
    Ok(g.upstreams.clone())
}

#[tauri::command]
pub fn delete_upstream(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<Upstream>, String> {
    let mut g = state.gateway.lock().map_err(|e| e.to_string())?;
    let before = g.upstreams.len();
    g.upstreams.retain(|u| u.id != id);
    if g.upstreams.len() == before {
        return Err(format!("上游不存在：{id}"));
    }
    g.save()?;
    Ok(g.upstreams.clone())
}

/// 拉取某个上游自己的模型列表（GET {base_url}/models）
#[tauri::command]
pub async fn upstream_models(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Value, String> {
    let (base, key, protocol) = {
        let g = state.gateway.lock().map_err(|e| e.to_string())?;
        let u = g
            .upstreams
            .iter()
            .find(|x| x.id == id)
            .ok_or_else(|| format!("上游不存在：{id}"))?;
        (
            u.base_url.trim_end_matches('/').to_string(),
            u.api_key.clone().unwrap_or_default(),
            u.protocol.clone(),
        )
    };
    let url = format!("{base}/models");
    let client = crate::httpc::client_for(&base)?;
    let mut rb = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(12))
        .header("accept", "application/json");
    if !key.is_empty() {
        rb = if protocol == "anthropic" {
            rb.header("x-api-key", &key).header("anthropic-version", "2023-06-01")
        } else {
            rb.bearer_auth(&key)
        };
    }
    let resp = rb.send().await.map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status().as_u16();
    let v: serde_json::Value = resp.json().await.unwrap_or_else(|_| serde_json::json!({}));
    let models = v
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| v.get("models").and_then(|d| d.as_array()))
        .cloned()
        .unwrap_or_default();
    let empty = models.is_empty();
    Ok(serde_json::json!({
        "status": status,
        "models": models,
        "raw": if empty { v } else { serde_json::json!({}) },
    }))
}

#[tauri::command]
pub fn ledger(limit: usize) -> Result<Vec<LedgerEntry>, String> {
    let p = ledger_path();
    if !p.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let mut out: Vec<LedgerEntry> = text
        .lines()
        .filter_map(|l| serde_json::from_str::<LedgerEntry>(l).ok())
        .collect();
    if out.len() > limit {
        out = out.split_off(out.len() - limit);
    }
    out.reverse();
    Ok(out)
}

/// 把 SSE 流（`data: {...}` 逐行）拼回一个普通的 chat.completion 结构。
/// 兼容 OpenAI 的 `choices[0].delta.content` 与 Anthropic 的 `delta.text`。
fn parse_sse(text: &str) -> Value {
    let mut content = String::new();
    let mut usage = json!({});
    let mut model = String::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("data:") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() || rest == "[DONE]" {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(rest) else {
            continue;
        };
        if model.is_empty() {
            if let Some(m) = v.get("model").and_then(|x| x.as_str()) {
                model = m.to_string();
            }
        }
        if let Some(u) = v.get("usage") {
            if u.is_object() {
                usage = u.clone();
            }
        }
        for ptr in ["/choices/0/delta/content", "/delta/text", "/choices/0/message/content"] {
            if let Some(s) = v.pointer(ptr).and_then(|x| x.as_str()) {
                content.push_str(s);
                break;
            }
        }
    }
    json!({
        "object": "chat.completion",
        "model": model,
        "usage": usage,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": "stop"
        }],
        "_stream": true
    })
}

/// 把思考档位真正写进请求体。
/// - `anthropic` 协议：`thinking: {type:"enabled", budget_tokens:N}`
/// - 其它协议：`reasoning_effort`（OpenAI 官方只认 minimal/low/medium/high，
///   `xhigh` / `max` 降级成 `high`，避免上游直接 400）
/// 调用方自己在 body 里带了同名字段时不覆盖。
fn apply_thinking(body: &mut Value, protocol: &str, level: &str) {
    let level = level.trim().to_lowercase();
    if level.is_empty() || matches!(level.as_str(), "default" | "off" | "none" | "auto") {
        return;
    }
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    if protocol == "anthropic" {
        if obj.contains_key("thinking") {
            return;
        }
        let budget = match level.as_str() {
            "low" => 1024,
            "medium" => 4096,
            "high" => 16384,
            "xhigh" => 32768,
            "max" => 65536,
            _ => 4096,
        };
        obj.insert(
            "thinking".into(),
            serde_json::json!({ "type": "enabled", "budget_tokens": budget }),
        );
    } else {
        if obj.contains_key("reasoning_effort") {
            return;
        }
        let eff = match level.as_str() {
            "minimal" | "low" | "medium" | "high" => level.as_str(),
            "xhigh" | "max" => "high",
            _ => "medium",
        };
        obj.insert("reasoning_effort".into(), serde_json::json!(eff));
    }
}

#[tauri::command]
pub async fn gateway_chat(
    state: tauri::State<'_, AppState>,
    req: ChatRequest,
) -> Result<ChatResult, String> {
    let protocol = req.protocol.clone().unwrap_or_else(|| "openai".into());
    let (upstream, _) = {
        let g = state.gateway.lock().map_err(|e| e.to_string())?;
        let u = g
            .select(&protocol, req.upstream.as_deref(), req.session_id.as_deref())
            .ok_or_else(|| "没有可用上游（可能全部冷却/禁用）".to_string())?;
        (u, ())
    };

    let url = endpoint(&upstream.base_url, &protocol);
    let client = crate::httpc::client_for(&upstream.base_url)?;
    let mut body = crate::extras::unify_roles(req.body.clone());
    let (compress_on, max_chars) = crate::extras::compress_settings();
    if compress_on {
        body = crate::extras::compress_body(body, max_chars);
    }
    // 请求里没给 model 时，用该上游配的默认模型（火山方舟必须给 ep-xxxx）
    let need_default = body
        .get("model")
        .and_then(|x| x.as_str())
        .map(|m| m.trim().is_empty())
        .unwrap_or(true);
    if need_default {
        if let Some(m) = upstream.model.as_deref().filter(|m| !m.trim().is_empty()) {
            if let Some(o) = body.as_object_mut() {
                o.insert("model".into(), json!(m));
            }
        }
    }

    // 模型别名映射：客户端写 gpt-4o，实际发给上游 ep-xxxx / doubao-pro
    let mapped_model = body
        .get("model")
        .and_then(|x| x.as_str())
        .and_then(|m| crate::extras::model_map().get(m).cloned());
    if let Some(mm) = mapped_model {
        if let Some(o) = body.as_object_mut() {
            o.insert("model".into(), json!(mm));
        }
    }

    // 思考档位：模型级 > 上游级 > 全局，真正写进请求体
    let req_model = body
        .get("model")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    apply_thinking(
        &mut body,
        &protocol,
        &crate::extras::thinking_level(&upstream.name, &req_model),
    );

    // 控制面强制执行：Key / 模型白名单
    let (allowed_keys, allowed_models) = crate::extras::control_settings();
    if !allowed_keys.is_empty() {
        let k = req.client_key.clone().unwrap_or_default();
        if !allowed_keys.iter().any(|x| x == &k) {
            return Err("控制面：Key 不在白名单".into());
        }
    }
    if !allowed_models.is_empty() {
        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("");
        if !allowed_models.iter().any(|m| model.contains(m.as_str())) {
            return Err(format!("控制面：模型 {model} 不在白名单"));
        }
    }

    // ---- 协议转换：把 OpenAI 形状的 body 换成上游原生格式 ----
    // 之前只换 URL 和鉴权头，body 还是 OpenAI 的，接官方 Claude / Gemini 直接 400。
    let _want_stream = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let (url, up_body) = if protocol == "gemini" {
        let (m, g) = crate::convert::to_gemini(&body);
        (
            format!(
                "{}/models/{}:generateContent",
                upstream.base_url.trim_end_matches('/'),
                m
            ),
            g,
        )
    } else if protocol == "anthropic" {
        (url.clone(), crate::convert::to_anthropic(&body))
    } else {
        (url.clone(), body.clone())
    };

    let make = |b: &Value| -> reqwest::RequestBuilder {
        let mut r = client.post(&url).json(b);
        if let Some(k) = upstream.api_key.as_deref().filter(|k| !k.is_empty()) {
            r = if protocol == "anthropic" {
                r.header("x-api-key", k).header("anthropic-version", "2023-06-01")
            } else if protocol == "gemini" {
                r.header("x-goog-api-key", k)
            } else {
                r.bearer_auth(k)
            };
        }
        if let Some(acc) = req.account_id.as_deref() {
            let did = crate::device::machine_id(acc);
            r = r
                .header("x-device-id", &did)
                .header("x-machine-id", &did)
                .header("x-client-id", "axiao-gateway");
        }
        r
    };

    let t0 = std::time::Instant::now();
    let resp = make(&up_body).send().await.map_err(|e| {
        if let Ok(mut g) = state.gateway.lock() {
            g.cool(&upstream.id);
        }
        e.to_string()
    })?;
    let mut status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let mut data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));

    // WorkBuddy/CodeBuddy 只接受流式：非流式会 400 "Non-stream chat request is
    // currently not supported"。遇到就自动带 stream:true 重发一次，再把 SSE 拼回
    // 一个普通的 chat.completion 结构，调用方不用管。
    if status == 400 && text.contains("Non-stream") {
        let mut b2 = body.clone();
        if let Some(o) = b2.as_object_mut() {
            o.insert("stream".into(), json!(true));
        }
        if let Ok(r2) = make(&b2).send().await {
            status = r2.status().as_u16();
            let t2 = r2.text().await.unwrap_or_default();
            data = if t2.contains("data:") {
                parse_sse(&t2)
            } else {
                serde_json::from_str(&t2).unwrap_or_else(|_| json!({}))
            };
        }
    }

    // 上游原生响应 → OpenAI 形状（客户端始终看到同一种结构）
    if status < 300 {
        if protocol == "anthropic" {
            data = crate::convert::anthropic_to_openai(&data, &model);
        } else if protocol == "gemini" {
            data = crate::convert::gemini_to_openai(&data, &model);
        }
    }

    let usage = data.get("usage").cloned().unwrap_or_else(|| json!({}));
    let prompt = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let completion = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    append_ledger(&LedgerEntry {
        ts: chrono::Utc::now().to_rfc3339(),
        account_id: req.account_id.clone().unwrap_or_default(),
        upstream: upstream.id.clone(),
        model: model.clone(),
        prompt_tokens: prompt,
        completion_tokens: completion,
        cost: crate::pricing::cost_usd(&model, prompt as u64, completion as u64),
        status: status as u16,
        latency_ms: t0.elapsed().as_millis() as u64,
    });

    let mut cooldown = false;
    if status == 429 || status >= 500 {
        cooldown = true;
        if let Ok(mut g) = state.gateway.lock() {
            g.cool(&upstream.id);
        }
    } else if let Some(sid) = req.session_id.clone() {
        if let Ok(mut g) = state.gateway.lock() {
            g.sticky.insert(sid, upstream.id.clone());
        }
    }

    Ok(ChatResult {
        upstream: upstream.id,
        status,
        data,
        cooldown,
    })
}
