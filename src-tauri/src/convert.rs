//! 协议转换：把 OpenAI Chat Completions 的请求/响应，转成各家原生格式。
//!
//! 背景：上游管理里的「协议」下拉之前只换了 URL 和鉴权头，body 仍是 OpenAI 形状，
//! 接官方 Anthropic / Gemini 会直接 400。这里补上真正的转换。
//!
//! 覆盖范围：
//! - OpenAI → Anthropic Messages（含 system / tools / stop / max_tokens）
//! - Anthropic → OpenAI（含 tool_use、usage、stop_reason）
//! - OpenAI → Gemini generateContent，Gemini → OpenAI
//!
//! 未覆盖：Anthropic 的流式增量 → OpenAI SSE。上游统一按非流式请求，
//! 拿到整段再转；客户端要流式时由上层合成一个 chunk（见 gateway）。

use serde_json::{json, Value};

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}

/// 追加一条消息；同角色连续时合并成一条。
///
/// Anthropic 要求 user / assistant 交替出现，而 OpenAI 里常见
/// 「user → tool → tool」这种连续同角色，不合并会 400。
fn push_blocks(out: &mut Vec<Value>, role: &str, blocks: Vec<Value>) {
    if blocks.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if s(last, "role") == role {
            if let Some(arr) = last.get_mut("content").and_then(|c| c.as_array_mut()) {
                arr.extend(blocks);
                return;
            }
        }
    }
    out.push(json!({"role": role, "content": blocks}));
}

/// 把 OpenAI 的 content（字符串或 parts 数组）转成 Anthropic 的 content blocks
fn content_blocks(content: &Value) -> Vec<Value> {
    match content {
        Value::String(t) => vec![json!({"type":"text","text":t})],
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| {
                let ty = s(p, "type");
                if ty == "text" {
                    Some(json!({"type":"text","text": s(p,"text")}))
                } else if ty == "image_url" {
                    let url = p
                        .get("image_url")
                        .and_then(|x| x.get("url"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    if let Some(rest) = url.strip_prefix("data:") {
                        let mut it = rest.splitn(2, ";base64,");
                        let media = it.next().unwrap_or("image/png").to_string();
                        let data = it.next().unwrap_or("").to_string();
                        Some(json!({"type":"image","source":{"type":"base64","media_type":media,"data":data}}))
                    } else {
                        // 远程 URL：Anthropic 也接受 url 类型
                        Some(json!({"type":"image","source":{"type":"url","url":url}}))
                    }
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

/// OpenAI messages → (system 文本, Anthropic messages)
///
/// 三个容易漏、但官方 API 一定会 400 的点，这里都处理了：
/// 1. `role:"system"` 不是消息，是顶层 `system` 字段
/// 2. `role:"tool"`（工具返回值）在 Anthropic 里是 **user 消息里的 tool_result block**
/// 3. assistant 的 `tool_calls` 要变成 **tool_use block**，否则多轮工具调用会断
fn split_messages(msgs: &[Value]) -> (String, Vec<Value>) {
    let mut system = String::new();
    let mut out: Vec<Value> = Vec::new();

    for m in msgs {
        let role = s(m, "role");
        let content = m.get("content").cloned().unwrap_or(Value::Null);

        // 1) system / developer → 顶层字段
        if role == "system" || role == "developer" {
            let text = match &content {
                Value::String(t) => t.clone(),
                Value::Array(parts) => parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|x| x.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            if !text.is_empty() {
                if !system.is_empty() {
                    system.push('\n');
                }
                system.push_str(&text);
            }
            continue;
        }

        // 2) 工具返回 → user + tool_result
        if role == "tool" {
            let id = m
                .get("tool_call_id")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let text = match &content {
                Value::String(t) => t.clone(),
                Value::Array(parts) => parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|x| x.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n"),
                other if !other.is_null() => other.to_string(),
                _ => String::new(),
            };
            push_blocks(
                &mut out,
                "user",
                vec![json!({"type":"tool_result","tool_use_id":id,"content":text})],
            );
            continue;
        }

        // 3) 普通消息（assistant 还要把 tool_calls 摊成 tool_use）
        let mut blocks = content_blocks(&content);
        if role == "assistant" {
            if let Some(tcs) = m.get("tool_calls").and_then(|x| x.as_array()) {
                for tc in tcs {
                    let f = tc.get("function").cloned().unwrap_or_else(|| json!({}));
                    let raw_args = f.get("arguments").and_then(|x| x.as_str()).unwrap_or("{}");
                    // OpenAI 的 arguments 是 JSON 字符串，Anthropic 的 input 是对象
                    let input: Value =
                        serde_json::from_str(raw_args).unwrap_or_else(|_| json!({}));
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tc.get("id").and_then(|x| x.as_str()).unwrap_or(""),
                        "name": f.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                        "input": input,
                    }));
                }
            }
        }

        push_blocks(&mut out, &role, blocks);
    }
    (system, out)
}

/// OpenAI 请求 → Anthropic Messages 请求
pub fn to_anthropic(body: &Value) -> Value {
    let msgs = body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
    let (system, messages) = split_messages(&msgs);

    // Anthropic 的 max_tokens 必填：取 max_tokens / max_completion_tokens，缺省 4096
    let max_tokens = num(body, "max_tokens")
        .or_else(|| num(body, "max_completion_tokens"))
        .unwrap_or(4096.0) as i64;

    let mut out = json!({
        "model": s(body, "model"),
        "max_tokens": max_tokens,
        "messages": messages,
    });
    let o = out.as_object_mut().unwrap();

    if !system.is_empty() {
        o.insert("system".into(), json!(system));
    }
    if let Some(t) = num(body, "temperature") {
        o.insert("temperature".into(), json!(t));
    }
    if let Some(t) = num(body, "top_p") {
        o.insert("top_p".into(), json!(t));
    }
    // stop：OpenAI 允许字符串或数组，Anthropic 只认数组
    if let Some(stop) = body.get("stop") {
        let seq = match stop {
            Value::String(x) => json!([x]),
            Value::Array(_) => stop.clone(),
            _ => Value::Null,
        };
        if !seq.is_null() {
            o.insert("stop_sequences".into(), seq);
        }
    }

    // tools：OpenAI 的 function 结构 → Anthropic 的 input_schema
    if let Some(tools) = body.get("tools").and_then(|x| x.as_array()) {
        let conv: Vec<Value> = tools
            .iter()
            .filter_map(|t| {
                let f = t.get("function")?;
                Some(json!({
                    "name": s(f, "name"),
                    "description": s(f, "description"),
                    "input_schema": f.get("parameters").cloned().unwrap_or_else(|| json!({"type":"object","properties":{}})),
                }))
            })
            .collect();
        if !conv.is_empty() {
            o.insert("tools".into(), json!(conv));
        }
    }

    // tool_choice：三家说法都不一样
    //   OpenAI  "auto" / "required" / {"type":"function","function":{"name":"x"}}
    //   Anthropic {"type":"auto"} / {"type":"any"} / {"type":"tool","name":"x"}
    // "none" 在 Anthropic 没有对应值，靠不传 tools 实现，这里直接忽略。
    if let Some(tc) = body.get("tool_choice") {
        let converted = match tc {
            Value::String(v) => match v.as_str() {
                "auto" => Some(json!({"type": "auto"})),
                "required" | "any" => Some(json!({"type": "any"})),
                _ => None,
            },
            Value::Object(_) => {
                let name = tc
                    .pointer("/function/name")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if name.is_empty() {
                    None
                } else {
                    Some(json!({"type": "tool", "name": name}))
                }
            }
            _ => None,
        };
        if let Some(c) = converted {
            o.insert("tool_choice".into(), c);
        }
    }

    // 这里产出的是「非流式」请求体：gateway / chat_proxy 都是整段取回再回给客户端。
    // 真正要流式时，由 chat_stream::prepare 在调用本函数后把 stream 补回去，
    // 所以这行 remove 不能删 —— 留着才能保证非流式路径不会误带 stream 被上游 400。
    o.remove("stream");
    o.remove("stream_options");
    o.remove("n");
    out
}

/// Anthropic 响应 → OpenAI chat.completion
pub fn anthropic_to_openai(v: &Value, model: &str) -> Value {
    let mut text = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();

    if let Some(blocks) = v.get("content").and_then(|x| x.as_array()) {
        for b in blocks {
            match s(b, "type").as_str() {
                "text" => text.push_str(&s(b, "text")),
                "tool_use" => tool_calls.push(json!({
                    "id": s(b, "id"),
                    "type": "function",
                    "function": {
                        "name": s(b, "name"),
                        "arguments": b.get("input").map(|x| x.to_string()).unwrap_or_else(|| "{}".into()),
                    }
                })),
                _ => {}
            }
        }
    }

    let stop = s(v, "stop_reason");
    let finish = match stop.as_str() {
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        _ => "stop",
    };

    let usage = v.get("usage").cloned().unwrap_or_else(|| json!({}));
    let pin = usage.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
    let pout = usage.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0);

    let mut message = json!({"role": "assistant", "content": text});
    if !tool_calls.is_empty() {
        message
            .as_object_mut()
            .unwrap()
            .insert("tool_calls".into(), json!(tool_calls));
    }

    json!({
        "id": format!("msg_{}", s(v, "id")),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish,
        }],
        "usage": {
            "prompt_tokens": pin,
            "completion_tokens": pout,
            "total_tokens": pin + pout,
        },
        "_converted_from": "anthropic",
    })
}

/// OpenAI 请求 → Gemini generateContent 请求（返回 (模型名, body)）
///
/// 工具调用这块跟 Anthropic 不一样：Gemini 的 `functionResponse` 认的是
/// **函数名**，而 OpenAI 的 tool 消息只有 `tool_call_id`。所以要先从前面
/// assistant 的 tool_calls 里把 id → name 记下来，否则工具结果回不去。
pub fn to_gemini(body: &Value) -> (String, Value) {
    let model = s(body, "model");
    let msgs = body.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();

    let mut system = String::new();
    let mut contents: Vec<Value> = Vec::new();
    let mut id2name: std::collections::HashMap<String, String> = Default::default();

    for m in &msgs {
        let role = s(m, "role");
        let text = match m.get("content") {
            Some(Value::String(t)) => t.clone(),
            Some(Value::Array(parts)) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|x| x.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            Some(other) if !other.is_null() && role == "tool" => other.to_string(),
            _ => String::new(),
        };

        if role == "system" || role == "developer" {
            if !system.is_empty() {
                system.push('\n');
            }
            system.push_str(&text);
            continue;
        }

        // 工具返回 → user + functionResponse（用函数名，不是 call id）
        if role == "tool" {
            let id = m.get("tool_call_id").and_then(|x| x.as_str()).unwrap_or("");
            let name = id2name
                .get(id)
                .cloned()
                .unwrap_or_else(|| "tool".to_string());
            contents.push(json!({
                "role": "user",
                "parts": [{"functionResponse": {"name": name, "response": {"result": text}}}]
            }));
            continue;
        }

        let mut parts: Vec<Value> = Vec::new();
        if !text.is_empty() {
            parts.push(json!({"text": text}));
        }
        // assistant 的 tool_calls → functionCall，同时记下 id → name
        if role == "assistant" {
            if let Some(tcs) = m.get("tool_calls").and_then(|x| x.as_array()) {
                for tc in tcs {
                    let f = tc.get("function").cloned().unwrap_or_else(|| json!({}));
                    let name = f.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    let id = tc.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    if !id.is_empty() && !name.is_empty() {
                        id2name.insert(id.to_string(), name.to_string());
                    }
                    let raw = f.get("arguments").and_then(|x| x.as_str()).unwrap_or("{}");
                    let args: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!({}));
                    parts.push(json!({"functionCall": {"name": name, "args": args}}));
                }
            }
        }
        if parts.is_empty() {
            continue;
        }
        // Gemini 只有 user / model 两种角色
        let g_role = if role == "assistant" { "model" } else { "user" };
        contents.push(json!({"role": g_role, "parts": parts}));
    }

    let mut out = json!({ "contents": contents });
    let o = out.as_object_mut().unwrap();
    if !system.is_empty() {
        o.insert("systemInstruction".into(), json!({"parts":[{"text":system}]}));
    }

    // tools：OpenAI 的 function 结构 → Gemini 的 functionDeclarations
    if let Some(tools) = body.get("tools").and_then(|x| x.as_array()) {
        let decls: Vec<Value> = tools
            .iter()
            .filter_map(|t| {
                let f = t.get("function")?;
                Some(json!({
                    "name": s(f, "name"),
                    "description": s(f, "description"),
                    "parameters": f.get("parameters").cloned()
                        .unwrap_or_else(|| json!({"type":"object","properties":{}})),
                }))
            })
            .collect();
        if !decls.is_empty() {
            o.insert("tools".into(), json!([{ "functionDeclarations": decls }]));
        }
    }
    let mut gen = serde_json::Map::new();
    if let Some(t) = num(body, "temperature") {
        gen.insert("temperature".into(), json!(t));
    }
    if let Some(t) = num(body, "max_tokens").or_else(|| num(body, "max_completion_tokens")) {
        gen.insert("maxOutputTokens".into(), json!(t as i64));
    }
    if !gen.is_empty() {
        o.insert("generationConfig".into(), Value::Object(gen));
    }
    (model, out)
}

/// Gemini 响应 → OpenAI chat.completion
pub fn gemini_to_openai(v: &Value, model: &str) -> Value {
    let mut text = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();
    if let Some(cands) = v.get("candidates").and_then(|x| x.as_array()) {
        if let Some(c) = cands.first() {
            if let Some(parts) = c.pointer("/content/parts").and_then(|x| x.as_array()) {
                for p in parts {
                    if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                        text.push_str(t);
                    }
                    // functionCall → OpenAI 的 tool_calls
                    if let Some(fc) = p.get("functionCall") {
                        let args = fc
                            .get("args")
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| "{}".into());
                        tool_calls.push(json!({
                            "id": format!("call_{}", tool_calls.len()),
                            "type": "function",
                            "function": {
                                "name": fc.get("name").and_then(|x| x.as_str()).unwrap_or(""),
                                "arguments": args,
                            }
                        }));
                    }
                }
            }
        }
    }
    let usage = v.get("usageMetadata").cloned().unwrap_or_else(|| json!({}));
    let pin = usage.get("promptTokenCount").and_then(|x| x.as_u64()).unwrap_or(0);
    let pout = usage.get("candidatesTokenCount").and_then(|x| x.as_u64()).unwrap_or(0);

    let mut message = json!({"role": "assistant", "content": text});
    let finish = if tool_calls.is_empty() { "stop" } else { "tool_calls" };
    if !tool_calls.is_empty() {
        message
            .as_object_mut()
            .unwrap()
            .insert("tool_calls".into(), json!(tool_calls));
    }

    json!({
        "id": "chatcmpl-gemini",
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish,
        }],
        "usage": {"prompt_tokens": pin, "completion_tokens": pout, "total_tokens": pin+pout},
        "_converted_from": "gemini",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_to_anthropic_basic() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "messages": [
                {"role":"system","content":"你是助手"},
                {"role":"user","content":"你好"},
                {"role":"user","content":"再问一句"}
            ],
            "max_tokens": 256,
            "temperature": 0.3,
            "stream": true
        });
        let out = to_anthropic(&body);
        assert_eq!(out["system"], "你是助手");
        assert_eq!(out["max_tokens"], 256);
        assert_eq!(out["temperature"], 0.3);
        assert!(out.get("stream").is_none(), "stream 必须去掉");
        // 连续同角色要合并成一条
        assert_eq!(out["messages"].as_array().unwrap().len(), 1);
        assert_eq!(out["messages"][0]["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn anthropic_tools_and_default_max_tokens() {
        let body = json!({
            "model": "claude",
            "messages": [{"role":"user","content":"hi"}],
            "tools": [{"type":"function","function":{"name":"get_time","description":"拿时间","parameters":{"type":"object","properties":{}}}}]
        });
        let out = to_anthropic(&body);
        assert_eq!(out["max_tokens"], 4096, "缺省 4096");
        assert_eq!(out["tools"][0]["name"], "get_time");
        assert_eq!(out["tools"][0]["input_schema"]["type"], "object");
    }

    #[test]
    fn anthropic_response_to_openai() {
        let v = json!({
            "id": "abc",
            "content": [{"type":"text","text":"你好"},{"type":"tool_use","id":"t1","name":"f","input":{"a":1}}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let out = anthropic_to_openai(&v, "claude-3-5-sonnet");
        assert_eq!(out["choices"][0]["message"]["content"], "你好");
        assert_eq!(out["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(out["usage"]["prompt_tokens"], 10);
        assert_eq!(out["usage"]["total_tokens"], 15);
        assert_eq!(out["choices"][0]["message"]["tool_calls"][0]["function"]["name"], "f");
    }

    #[test]
    fn openai_to_gemini_roles() {
        let body = json!({
            "model": "gemini-2.5-flash",
            "messages": [
                {"role":"system","content":"sys"},
                {"role":"user","content":"q"},
                {"role":"assistant","content":"a"}
            ],
            "max_tokens": 128
        });
        let (m, out) = to_gemini(&body);
        assert_eq!(m, "gemini-2.5-flash");
        assert_eq!(out["systemInstruction"]["parts"][0]["text"], "sys");
        assert_eq!(out["contents"][0]["role"], "user");
        assert_eq!(out["contents"][1]["role"], "model");
        assert_eq!(out["generationConfig"]["maxOutputTokens"], 128);
    }

    #[test]
    fn gemini_response_to_openai() {
        let v = json!({
            "candidates": [{"content":{"parts":[{"text":"hi there"}]}}],
            "usageMetadata": {"promptTokenCount": 7, "candidatesTokenCount": 3}
        });
        let out = gemini_to_openai(&v, "gemini-2.5-flash");
        assert_eq!(out["choices"][0]["message"]["content"], "hi there");
        assert_eq!(out["usage"]["total_tokens"], 10);
    }

    /// 工具调用的多轮往返：OpenAI 的 tool 角色 / tool_calls 必须变成
    /// Anthropic 的 tool_result / tool_use，否则官方 API 直接 400。
    #[test]
    fn tool_calls_to_anthropic() {
        let body = json!({
            "model": "claude-3-5-sonnet",
            "messages": [
                {"role":"system","content":"你是助手"},
                {"role":"user","content":"北京天气怎么样"},
                {"role":"assistant","content":null,"tool_calls":[
                    {"id":"call_1","type":"function",
                     "function":{"name":"get_weather","arguments":"{\"city\":\"北京\"}"}}
                ]},
                {"role":"tool","tool_call_id":"call_1","content":"晴，25 度"},
                {"role":"tool","tool_call_id":"call_1","content":"湿度 40%"}
            ],
            "tools":[{"type":"function","function":{
                "name":"get_weather","description":"查天气",
                "parameters":{"type":"object","properties":{"city":{"type":"string"}}}}}],
            "tool_choice":"auto"
        });
        let out = to_anthropic(&body);
        let msgs = out["messages"].as_array().unwrap();

        // 角色只能出现 user / assistant，绝不能有 tool
        for m in msgs {
            let r = m["role"].as_str().unwrap();
            assert!(r == "user" || r == "assistant", "出现了非法角色 {r}");
        }
        // system 提到顶层
        assert_eq!(out["system"], "你是助手");
        // assistant 带 tool_use，arguments 字符串已解析成对象
        let asst = msgs.iter().find(|m| m["role"] == "assistant").unwrap();
        let blocks = asst["content"].as_array().unwrap();
        let tu = blocks
            .iter()
            .find(|b| b["type"] == "tool_use")
            .expect("assistant 消息里没有 tool_use");
        assert_eq!(tu["name"], "get_weather");
        assert_eq!(tu["input"]["city"], "北京");
        // 两条 tool 结果合并进同一条 user
        let tool_results: Vec<_> = msgs
            .iter()
            .filter(|m| m["role"] == "user")
            .flat_map(|m| m["content"].as_array().unwrap().clone())
            .filter(|b| b["type"] == "tool_result")
            .collect();
        assert_eq!(tool_results.len(), 2, "tool 结果丢了");
        assert_eq!(tool_results[0]["tool_use_id"], "call_1");
        assert_eq!(out["tool_choice"]["type"], "auto");
    }

    /// Gemini 的工具往返：请求侧要 functionCall / functionResponse，
    /// 而且 functionResponse 认的是**函数名**（OpenAI 只给 call id，得查回来）。
    #[test]
    fn tool_calls_to_gemini_and_back() {
        let body = json!({
            "model": "gemini-2.5-flash",
            "messages": [
                {"role":"user","content":"北京天气"},
                {"role":"assistant","content":null,"tool_calls":[
                    {"id":"call_9","type":"function",
                     "function":{"name":"get_weather","arguments":"{\"city\":\"北京\"}"}}
                ]},
                {"role":"tool","tool_call_id":"call_9","content":"晴 25 度"}
            ],
            "tools":[{"type":"function","function":{
                "name":"get_weather","parameters":{"type":"object"}}}]
        });
        let (m, out) = to_gemini(&body);
        assert_eq!(m, "gemini-2.5-flash");
        let cs = out["contents"].as_array().unwrap();
        assert_eq!(cs.len(), 3);
        assert_eq!(cs[1]["role"], "model");
        assert_eq!(
            cs[1]["parts"].as_array().unwrap()[0]["functionCall"]["name"],
            "get_weather"
        );
        // 关键：functionResponse 用函数名，不是 call_9
        assert_eq!(
            cs[2]["parts"].as_array().unwrap()[0]["functionResponse"]["name"],
            "get_weather"
        );
        assert!(out["tools"][0]["functionDeclarations"].is_array());

        // 响应侧：functionCall → tool_calls，finish_reason 要变成 tool_calls
        let v = json!({
            "candidates":[{"content":{"parts":[
                {"functionCall":{"name":"get_weather","args":{"city":"北京"}}}
            ]},"finishReason":"STOP"}],
            "usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":2}
        });
        let back = gemini_to_openai(&v, "gemini-2.5-flash");
        assert_eq!(back["choices"][0]["finish_reason"], "tool_calls");
        let tc = &back["choices"][0]["message"]["tool_calls"][0];
        assert_eq!(tc["function"]["name"], "get_weather");
        assert!(tc["function"]["arguments"].as_str().unwrap().contains("北京"));
    }

    /// stop 可以是字符串，Anthropic 只认数组
    #[test]
    fn stop_string_normalized_for_anthropic() {
        let body = json!({"model":"m","messages":[{"role":"user","content":"hi"}],"stop":"END"});
        let out = to_anthropic(&body);
        assert_eq!(out["stop_sequences"], json!(["END"]));
    }
}
