use std::net::UdpSocket;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::store::data_dir;

fn extras_file() -> PathBuf {
    data_dir().join("extras.json")
}

fn defaults() -> Value {
    json!({
        "mobile": { "enabled": true },
        "control": { "keys": [], "whitelist": [] },
        "cli": { "enabled": true },
        "compress": { "enabled": false, "maxChars": 12000 },
        "zcode": { "window": "00:00-08:00", "queue": [] },
        "thinking": { "level": "default", "per": {} }
    })
}

/// 通用读取：按 key 拿 extras.json 里的一段
/// 写嵌套字段：`put_nested("control", "keys", json!([...]))`
/// 对应 extras.json 里的 `control.keys`，不会碰到同级的其它字段。
pub fn put_nested(section: &str, field: &str, value: Value) -> Result<(), String> {
    let mut v = load();
    if let Some(obj) = v.as_object_mut() {
        let sec = obj
            .entry(section.to_string())
            .or_insert_with(|| Value::Object(Default::default()));
        if !sec.is_object() {
            *sec = Value::Object(Default::default());
        }
        if let Some(so) = sec.as_object_mut() {
            so.insert(field.to_string(), value);
        }
    }
    save(&v)
}

pub fn get(key: &str) -> Value {
    load().get(key).cloned().unwrap_or(Value::Null)
}

/// 通用写入：只改这一段的整块内容
pub fn set(key: &str, value: Value) -> Result<(), String> {
    let mut v = load();
    if let Some(o) = v.as_object_mut() {
        o.insert(key.to_string(), value);
    }
    let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(extras_file(), text).map_err(|e| e.to_string())
}

fn load() -> Value {
    let mut base = defaults();
    if let Ok(s) = std::fs::read_to_string(extras_file()) {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            if let (Some(b), Some(v)) = (base.as_object_mut(), v.as_object()) {
                for (k, val) in v {
                    b.insert(k.clone(), val.clone());
                }
            }
        }
    }
    base
}

fn save(v: &Value) -> Result<(), String> {
    let _ = std::fs::create_dir_all(data_dir());
    std::fs::write(
        extras_file(),
        serde_json::to_string_pretty(v).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn extras_get() -> Result<Value, String> {
    Ok(load())
}

#[tauri::command]
pub fn extras_save(key: String, value: Value) -> Result<Value, String> {
    let mut v = load();
    if let Some(obj) = v.as_object_mut() {
        obj.insert(key, value);
    }
    save(&v)?;
    Ok(v)
}

/// 生效的思考档位，优先级：模型级 `thinking.models[model]` → 上游级
/// `thinking.per[上游名]` → 全局 `thinking.level`。
/// 返回 "default" / "off" 表示不干预请求体。
/// 模型别名映射：把客户端请求的 model 换成上游实际认的名字
/// （火山方舟的 ep-xxxx、自定义中转的别名，都靠这个）
pub fn model_map() -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    if let Some(o) = get("modelMap").as_object() {
        for (k, v) in o {
            if let Some(to) = v.as_str() {
                if !k.trim().is_empty() && !to.trim().is_empty() {
                    out.insert(k.trim().to_string(), to.trim().to_string());
                }
            }
        }
    }
    out
}

pub fn thinking_level(upstream_name: &str, model: &str) -> String {
    let v = load();
    let t = v.get("thinking").cloned().unwrap_or_else(|| json!({}));

    // 1) 模型级：每个模型支持的档位本来就不同（面板的 supported_efforts）
    if !model.is_empty() {
        if let Some(x) = t
            .get("models")
            .and_then(|m| m.get(model))
            .and_then(|x| x.as_str())
        {
            if !x.trim().is_empty() {
                return x.trim().to_string();
            }
        }
    }
    // 2) 上游级
    if let Some(x) = t
        .get("per")
        .and_then(|p| p.get(upstream_name))
        .and_then(|x| x.as_str())
    {
        if !x.trim().is_empty() {
            return x.trim().to_string();
        }
    }
    // 3) 全局
    t.get("level")
        .and_then(|x| x.as_str())
        .unwrap_or("default")
        .to_string()
}

fn lan_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            Ok(s.local_addr()?.ip().to_string())
        })
        .unwrap_or_else(|_| "127.0.0.1".into())
}

/// 移动端常驻：局域网访问地址
#[tauri::command]
pub fn mobile_info() -> Result<Value, String> {
    let cfg = crate::config::local_config_get()
        .ok()
        .and_then(|v| v.get("config").cloned())
        .unwrap_or_else(|| json!({}));
    let listen = cfg
        .get("listen")
        .and_then(|x| x.as_str())
        .unwrap_or("127.0.0.1:7863")
        .to_string();
    let ip = lan_ip();
    let port = listen.rsplit(':').next().unwrap_or("7863");
    Ok(json!({
        "lanIp": ip,
        "listen": listen,
        "lanReady": listen.starts_with("0.0.0.0"),
        "panelUrl": format!("http://{ip}:{port}/panel/"),
        "apiBase": format!("http://{ip}:{port}/v1"),
    }))
}

/// 角色统一：developer → system（兼容 Anthropic 等只认 system 的上游）
pub fn unify_roles(mut body: Value) -> Value {
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for m in msgs.iter_mut() {
            if m.get("role").and_then(|r| r.as_str()) == Some("developer") {
                m["role"] = Value::String("system".into());
            }
        }
    }
    body
}

/// ZCode off-peak 定时器：到窗口且队列非空时执行（清队列）
#[tauri::command]
pub fn zcode_tick() -> Result<Value, String> {
    let v = load();
    let z = v.get("zcode").cloned().unwrap_or_else(|| json!({}));
    let queue = z
        .get("queue")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default();
    if queue.is_empty() {
        return Ok(json!({ "ran": 0, "reason": "队列为空" }));
    }
    let now = chrono::Local::now().format("%H:%M").to_string();
    let window = z
        .get("window")
        .and_then(|w| w.as_str())
        .unwrap_or("00:00-08:00");
    let (start, end) = window.split_once('-').unwrap_or(("00:00", "08:00"));
    let n = now.as_str();
    let in_window = if start <= end {
        n >= start && n < end
    } else {
        n >= start || n < end
    };
    if !in_window {
        return Ok(json!({ "ran": 0, "reason": "不在 off-peak 窗口", "now": now, "window": window }));
    }
    let mut v2 = v.clone();
    if let Some(obj) = v2.get_mut("zcode").and_then(|x| x.as_object_mut()) {
        obj.insert("queue".into(), json!([]));
        obj.insert("last_run".into(), json!(now));
    }
    save(&v2)?;
    Ok(json!({ "ran": queue.len(), "at": now, "window": window }))
}

/// 控制面：Key 白名单 / 模型白名单
pub fn control_settings() -> (Vec<String>, Vec<String>) {
    let v = load();
    let c = v.get("control");
    let read = |name: &str| -> Vec<String> {
        c.and_then(|x| x.get(name))
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };
    (read("keys"), read("whitelist"))
}

/// 上下文压缩：读取开关与上限
pub fn compress_settings() -> (bool, usize) {
    let v = load();
    let c = v.get("compress");
    (
        c.and_then(|x| x.get("enabled")).and_then(|x| x.as_bool()).unwrap_or(false),
        c.and_then(|x| x.get("maxChars"))
            .and_then(|x| x.as_u64())
            .unwrap_or(12000) as usize,
    )
}

/// 压缩 messages：保留 system + 最近 6 条，单条超长截断
pub fn compress_body(mut body: Value, max_chars: usize) -> Value {
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        if msgs.len() > 8 {
            let mut kept: Vec<Value> = Vec::new();
            if let Some(first) = msgs.first() {
                if first.get("role").and_then(|r| r.as_str()) == Some("system") {
                    kept.push(first.clone());
                }
            }
            let start = msgs.len().saturating_sub(6);
            for m in &msgs[start..] {
                kept.push(m.clone());
            }
            *msgs = kept;
        }
        for m in msgs.iter_mut() {
            if let Some(c) = m.get("content").and_then(|v| v.as_str()) {
                if c.len() > max_chars {
                    let cut: String = c.chars().take(max_chars).collect();
                    m["content"] = Value::String(format!("{cut}…[compressed]"));
                }
            }
        }
    }
    body
}
