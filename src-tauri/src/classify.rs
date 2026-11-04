//! 失败分类：光看状态码不够。
//!
//! 为什么必须细分：`400` 里混着三种完全不同的情况，处理方式正相反 ——
//!
//! | 400 的真实原因 | 该怎么做 |
//! |---|---|
//! | 上下文超长 | **换个窗口更大的渠道一定能成功**，值得降级 |
//! | 内容策略拒绝 | 换同厂商没用，换异厂商可能行；默认不降级 |
//! | 参数不支持 / JSON 写错 | 换十个渠道都是同样的错，绝不能降级 |
//!
//! 如果不细分，只有两个选择，都是错的：要么全降级（白烧配额），
//! 要么全不降级（长文档问答永远失败）。
//!
//! 关键词表参考各家真实报错措辞。宁可漏判（当成 BadRequest 不降级），
//! 也不要把"参数错误"误判成"上下文超长"——后者会白跑一遍降级链。

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailKind {
    /// 429：限流。换渠道 + 短冷却
    RateLimit,
    /// 5xx：上游故障。换渠道 + 连续 3 次才冷却
    ServerError,
    /// 连不上：超时 / 被拒 / DNS
    Network,
    /// 401 / 403：Key 不对或没权限。不降级（换渠道也没用）
    Auth,
    /// 404：地址填错。不降级
    NotFound,
    /// 400 且是上下文超长 —— **唯一"换个大窗口就一定成功"的失败**
    ContextTooLong,
    /// 400 且是内容策略拒绝
    ContentPolicy,
    /// 其它 400 / 413 / 422：请求本身的问题，不降级
    BadRequest,
}

impl FailKind {
    /// 值不值得换个渠道再试
    pub fn fallback_worthy(self) -> bool {
        matches!(
            self,
            FailKind::RateLimit | FailKind::ServerError | FailKind::Network | FailKind::ContextTooLong
        )
    }

    /// 给用户看的短说明（拼进最终错误里）
    pub fn hint(self) -> &'static str {
        match self {
            FailKind::RateLimit => "上游限流",
            FailKind::ServerError => "上游服务故障",
            FailKind::Network => "连不上上游",
            FailKind::Auth => "Key 不对或没有该模型的权限",
            FailKind::NotFound => "接口地址不对（404）",
            FailKind::ContextTooLong => "输入超出该模型的上下文窗口",
            FailKind::ContentPolicy => "内容被上游的安全策略拒绝",
            FailKind::BadRequest => "请求参数不被上游接受",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FailKind::RateLimit => "rate_limit",
            FailKind::ServerError => "server_error",
            FailKind::Network => "network",
            FailKind::Auth => "auth",
            FailKind::NotFound => "not_found",
            FailKind::ContextTooLong => "context_too_long",
            FailKind::ContentPolicy => "content_policy",
            FailKind::BadRequest => "bad_request",
        }
    }
}

/// 上下文超长的关键词。各家措辞不一样，这里是实际见过的写法。
const CTX_HINTS: &[&str] = &[
    "context length",
    "context_length",
    "context window",
    "maximum context",
    "max context",
    "prompt is too long",
    "too many tokens",
    "too long",
    "exceeds the maximum",
    "maximum number of tokens",
    "input token count",
    "input length",
    "reduce the length",
    "string too long",
    "range of input length",
];

/// 内容策略拒绝的关键词
const POLICY_HINTS: &[&str] = &[
    "content policy",
    "content_policy",
    "content management policy",
    "safety",
    "flagged",
    "moderation",
    "prohibited",
    "violates",
];

fn any(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| hay.contains(n))
}

/// 从响应体里取错误文本（各家结构不同，能取多少取多少）
fn error_text(body: &Value) -> String {
    // OpenAI / Anthropic: {"error":{"message":"..."}}
    // Gemini: {"error":{"message":"...","status":"..."}}
    // 有的直接是纯字符串
    let mut parts: Vec<String> = Vec::new();
    if let Some(m) = body.pointer("/error/message").and_then(|x| x.as_str()) {
        parts.push(m.to_string());
    }
    if let Some(t) = body.pointer("/error/type").and_then(|x| x.as_str()) {
        parts.push(t.to_string());
    }
    if let Some(c) = body.pointer("/error/code").and_then(|x| x.as_str()) {
        parts.push(c.to_string());
    }
    if let Some(r) = body.get("raw").and_then(|x| x.as_str()) {
        parts.push(r.to_string());
    }
    if parts.is_empty() {
        body.to_string()
    } else {
        parts.join(" ").to_lowercase()
    }
}

/// 结合状态码和响应体判断失败类型
pub fn classify(status: u16, body: &Value) -> FailKind {
    match status {
        429 => FailKind::RateLimit,
        401 | 403 => FailKind::Auth,
        404 => FailKind::NotFound,
        400 | 413 | 422 => {
            let text = error_text(body);
            // 先判上下文超长：它是唯一值得降级的 400
            if any(&text, CTX_HINTS) {
                FailKind::ContextTooLong
            } else if any(&text, POLICY_HINTS) {
                FailKind::ContentPolicy
            } else {
                FailKind::BadRequest
            }
        }
        s if s >= 500 => FailKind::ServerError,
        _ => FailKind::BadRequest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_400_variants() {
        // 上下文超长：OpenAI 的措辞
        let openai_ctx = json!({"error":{"message":"This model's maximum context length is 8192 tokens. However, your messages resulted in 9123 tokens.","type":"invalid_request_error","code":"context_length_exceeded"}});
        assert_eq!(classify(400, &openai_ctx), FailKind::ContextTooLong);
        assert!(FailKind::ContextTooLong.fallback_worthy(), "上下文超长必须可降级");

        // 上下文超长：Anthropic 的措辞
        let anthropic_ctx = json!({"error":{"message":"prompt is too long: 201234 tokens > 200000 maximum"}});
        assert_eq!(classify(400, &anthropic_ctx), FailKind::ContextTooLong);

        // 上下文超长：Gemini 的措辞
        let gemini_ctx = json!({"error":{"message":"The input token count (1048576) exceeds the maximum number of tokens allowed"}});
        assert_eq!(classify(400, &gemini_ctx), FailKind::ContextTooLong);

        // 内容策略
        let policy = json!({"error":{"message":"Your request was rejected as a result of our safety system."}});
        assert_eq!(classify(400, &policy), FailKind::ContentPolicy);
        assert!(!FailKind::ContentPolicy.fallback_worthy());

        // 普通参数错误：绝不能判成上下文超长
        let plain = json!({"error":{"message":"Unsupported value: 'temperature' does not support 3.0"}});
        assert_eq!(classify(400, &plain), FailKind::BadRequest);
        assert!(!FailKind::BadRequest.fallback_worthy());

        // 状态码大类
        assert_eq!(classify(429, &json!({})), FailKind::RateLimit);
        assert_eq!(classify(503, &json!({})), FailKind::ServerError);
        assert_eq!(classify(401, &json!({})), FailKind::Auth);
        assert_eq!(classify(404, &json!({})), FailKind::NotFound);

        // 可降级集合
        assert!(FailKind::RateLimit.fallback_worthy());
        assert!(FailKind::ServerError.fallback_worthy());
        assert!(!FailKind::Auth.fallback_worthy());
        assert!(!FailKind::NotFound.fallback_worthy());
    }
}
