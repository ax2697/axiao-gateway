//! 模型单价表（参考价，USD / 1M tokens）。
//!
//! 用途：把账本里的 token 换算成钱，并在「成本账本」显示。
//! 价格会变，这里只是内置默认值；可以在 extras.json 的 "prices" 里覆盖：
//!   { "prices": { "gpt-4o": { "in": 2.5, "out": 10 } } }
//! 单位统一 USD/1M tokens。取不到价的模型按 0 计，不猜。

use serde_json::Value;

use crate::extras;

/// 内置参考价：(模型前缀, 输入单价, 输出单价)
const TABLE: &[(&str, f64, f64)] = &[
    // OpenAI
    ("gpt-4o-mini", 0.15, 0.60),
    ("gpt-4o", 2.50, 10.00),
    ("gpt-4.1-mini", 0.40, 1.60),
    ("gpt-4.1", 2.00, 8.00),
    ("o4-mini", 1.10, 4.40),
    // Anthropic
    ("claude-3-5-haiku", 0.80, 4.00),
    ("claude-3-5-sonnet", 3.00, 15.00),
    ("claude-sonnet-4", 3.00, 15.00),
    ("claude-opus-4", 15.00, 75.00),
    // Google
    ("gemini-2.5-flash", 0.30, 2.50),
    ("gemini-2.5-pro", 1.25, 10.00),
    ("gemini-2.0-flash", 0.10, 0.40),
    // 国内
    ("deepseek-chat", 0.27, 1.10),
    ("deepseek-reasoner", 0.55, 2.19),
    ("qwen-max", 1.60, 6.40),
    ("qwen-plus", 0.40, 1.20),
    ("qwen-turbo", 0.05, 0.20),
    ("glm-4-plus", 0.70, 0.70),
    ("glm-4-flash", 0.00, 0.00),
    ("moonshot-v1-8k", 0.17, 0.17),
    ("kimi-k2", 0.60, 2.50),
    ("doubao-pro", 0.11, 0.28),
    ("doubao-lite", 0.04, 0.08),
    ("abab6.5s", 0.14, 0.14),
    ("yi-large", 0.43, 0.43),
    ("spark-4.0", 0.07, 0.07),
    ("hunyuan-turbo", 0.21, 0.21),
    // 免费/自建
    ("llama", 0.0, 0.0),
    ("qwen2.5", 0.0, 0.0),
];

fn overrides() -> Value {
    extras::get("prices")
}

/// 按模型名匹配单价（大小写不敏感、前缀匹配，取最长匹配）
pub fn price_of(model: &str) -> (f64, f64) {
    let m = model.to_lowercase();

    // 1) 用户覆盖优先
    if let Some(o) = overrides().as_object() {
        for (k, v) in o {
            if m.contains(&k.to_lowercase()) {
                let i = v.get("in").and_then(|x| x.as_f64()).unwrap_or(0.0);
                let out = v.get("out").and_then(|x| x.as_f64()).unwrap_or(0.0);
                return (i, out);
            }
        }
    }

    // 2) 内置表：最长前缀优先，避免 gpt-4o 抢掉 gpt-4o-mini
    let mut best: Option<(&str, f64, f64)> = None;
    for (name, i, o) in TABLE {
        if m.contains(name) {
            if best.map(|(b, _, _)| name.len() > b.len()).unwrap_or(true) {
                best = Some((name, *i, *o));
            }
        }
    }
    best.map(|(_, i, o)| (i, o)).unwrap_or((0.0, 0.0))
}

/// 算钱：返回 USD
pub fn cost_usd(model: &str, prompt: u64, completion: u64) -> f64 {
    let (i, o) = price_of(model);
    (prompt as f64 / 1_000_000.0) * i + (completion as f64 / 1_000_000.0) * o
}

/// 读模型映射（前端渲染成一行一条）
#[tauri::command]
pub fn model_map_get() -> Result<Value, String> {
    Ok(extras::get("modelMap"))
}

/// 保存模型映射：{"gpt-4o":"ep-xxxx"}
#[tauri::command]
pub fn model_map_set(map: Value) -> Result<Value, String> {
    if !map.is_object() && !map.is_null() {
        return Err("modelMap 必须是对象，如 {\"gpt-4o\":\"ep-xxx\"}".into());
    }
    extras::set("modelMap", if map.is_null() { serde_json::json!({}) } else { map })?;
    Ok(serde_json::json!({"ok": true}))
}

/// 查一个模型的参考单价（前端悬停提示用）
#[tauri::command]
pub fn price_lookup(model: String) -> Result<Value, String> {
    let (i, o) = price_of(&model);
    Ok(serde_json::json!({"model": model, "in": i, "out": o, "unit": "USD / 1M tokens"}))
}
