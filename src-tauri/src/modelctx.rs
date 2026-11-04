//! 模型上下文窗口 + 请求长度预检。
//!
//! 为什么值得做：上下文超长是**唯一一种"发出去注定失败"的请求**。
//! 如果不预检，我们会把 20 万 token 的 prompt 发给一个 8K 窗口的模型，
//! 等它返回 400，再换下一个 —— 白花一次往返的延迟和（部分厂商按输入计费的）钱。
//!
//! 参考 LiteLLM 的 `enable_pre_call_checks`：调用前就把窗口装不下的渠道过滤掉。
//!
//! **估算刻意偏保守（低估）**：真实 tokenizer 拿不到（各家不同、要额外依赖）。
//! 高估会误杀本来能成功的请求，那比多试一次更糟。所以：
//! - 中文按 1 字 ≈ 1 token
//! - 其它按 4 字符 ≈ 1 token
//! - 只有**明确超出**（估算 > 窗口）才跳过，不设 95% 之类的收紧系数

use serde_json::Value;

/// 常见模型的上下文窗口（tokens）。
///
/// 匹配用**最长前缀**，跟 `pricing.rs` 一个套路 —— 这样 `gpt-4o-mini` 不会被
/// `gpt-4o` 抢先匹配。数据来自各家官方文档，认不出的返回 None（不预检，照常发）。
const WINDOWS: &[(&str, u32)] = &[
    // OpenAI
    ("gpt-4o-mini", 128_000),
    ("gpt-4o", 128_000),
    ("gpt-4.1-mini", 1_047_576),
    ("gpt-4.1", 1_047_576),
    ("gpt-4-turbo", 128_000),
    ("gpt-4-32k", 32_768),
    ("gpt-4", 8_192),
    ("gpt-3.5-turbo", 16_385),
    ("o1-mini", 128_000),
    ("o1", 200_000),
    ("o3-mini", 200_000),
    // Anthropic
    ("claude-3-5-sonnet", 200_000),
    ("claude-3-5-haiku", 200_000),
    ("claude-3-opus", 200_000),
    ("claude-3-sonnet", 200_000),
    ("claude-3-haiku", 200_000),
    ("claude-sonnet-4", 200_000),
    ("claude-opus-4", 200_000),
    ("claude", 200_000),
    // Google
    ("gemini-2.5-pro", 1_048_576),
    ("gemini-2.5-flash", 1_048_576),
    ("gemini-2.0-flash", 1_048_576),
    ("gemini-1.5-pro", 2_097_152),
    ("gemini-1.5-flash", 1_048_576),
    ("gemini", 1_048_576),
    // 国内
    ("deepseek-chat", 65_536),
    ("deepseek-reasoner", 65_536),
    ("deepseek", 65_536),
    ("qwen-max", 32_768),
    ("qwen-plus", 131_072),
    ("qwen-long", 1_000_000),
    ("qwen", 32_768),
    ("glm-4-plus", 131_072),
    ("glm-4", 131_072),
    ("moonshot-v1-128k", 131_072),
    ("moonshot-v1-32k", 32_768),
    ("moonshot", 131_072),
    ("kimi", 131_072),
    ("doubao-pro-128k", 131_072),
    ("doubao-pro-32k", 32_768),
    ("doubao", 32_768),
    ("ernie", 8_192),
    ("hunyuan", 32_768),
    ("baichuan", 32_768),
    ("yi-large", 32_768),
    ("spark", 8_192),
    // 常见本地 / 开源
    ("llama-3.1-70b", 131_072),
    ("llama-3.1-8b", 131_072),
    ("llama3", 8_192),
    ("mistral-large", 131_072),
    ("mixtral", 32_768),
];

/// 按最长前缀查窗口大小。认不出来返回 None（上层不预检，照常发）
pub fn context_window(model: &str) -> Option<u32> {
    let m = model.to_lowercase();
    WINDOWS
        .iter()
        .filter(|(k, _)| m.starts_with(k) || m.contains(k))
        .max_by_key(|(k, _)| k.len())
        .map(|(_, w)| *w)
}

/// 估算一段文本的 token 数（保守偏低）
pub fn estimate_text(s: &str) -> u64 {
    let mut cjk = 0u64;
    let mut other = 0u64;
    for ch in s.chars() {
        let c = ch as u32;
        // CJK 统一表意文字 + 中日韩标点
        if (0x4E00..=0x9FFF).contains(&c)
            || (0x3000..=0x303F).contains(&c)
            || (0xFF00..=0xFFEF).contains(&c)
        {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other / 4
}

/// 估算整个请求的输入 token（messages + tools 定义）
pub fn estimate_request(body: &Value) -> u64 {
    let mut total = 0u64;
    if let Some(msgs) = body.get("messages").and_then(|x| x.as_array()) {
        for m in msgs {
            total += 4; // 每条消息的角色/分隔开销，跟各家实现取个近似
            match m.get("content") {
                Some(Value::String(t)) => total += estimate_text(t),
                Some(Value::Array(parts)) => {
                    for p in parts {
                        if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                            total += estimate_text(t);
                        } else if p.get("image_url").is_some() {
                            // 一张图大致按 1000 token 算（各家差异大，取中间值）
                            total += 1_000;
                        }
                    }
                }
                _ => {}
            }
            // assistant 的工具调用参数也算输入
            if let Some(tcs) = m.get("tool_calls").and_then(|x| x.as_array()) {
                for tc in tcs {
                    if let Some(a) = tc.pointer("/function/arguments").and_then(|x| x.as_str()) {
                        total += estimate_text(a);
                    }
                }
            }
        }
    }
    if let Some(tools) = body.get("tools") {
        total += estimate_text(&tools.to_string());
    }
    total
}

/// 这个模型装不装得下。`None` 表示认不出模型 → 不拦截
pub fn fits(model: &str, tokens: u64) -> Option<bool> {
    context_window(model).map(|w| tokens <= w as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn window_lookup_and_estimate() {
        // 最长前缀：mini 不能被 4o 抢走
        assert_eq!(context_window("gpt-4o-mini"), Some(128_000));
        assert_eq!(context_window("gpt-4"), Some(8_192));
        assert_eq!(context_window("gemini-2.5-flash"), Some(1_048_576));
        assert_eq!(context_window("claude-3-5-sonnet-20241022"), Some(200_000));
        // 认不出来的不拦
        assert_eq!(context_window("some-unknown-model"), None);
        assert_eq!(fits("some-unknown-model", 999_999_999), None);

        // 中文按 1 字 1 token
        assert_eq!(estimate_text("你好世界"), 4);
        // 英文 4 字符约 1 token
        assert_eq!(estimate_text("abcdefgh"), 2);

        // 整请求估算：能粗略反映长度
        let short = json!({"messages":[{"role":"user","content":"hi"}]});
        let long_text = "长".repeat(50_000);
        let long = json!({"messages":[{"role":"user","content": long_text}]});
        assert!(estimate_request(&long) > estimate_request(&short) + 40_000);

        // 8K 窗口装不下 50K 中文
        assert_eq!(fits("gpt-4", estimate_request(&long)), Some(false));
        // 1M 窗口装得下
        assert_eq!(fits("gemini-2.5-flash", estimate_request(&long)), Some(true));
    }
}
