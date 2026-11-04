//! 上游熔断：连续失败的渠道暂时摘掉，别把请求往死里送。
//!
//! 参考的是 LiteLLM Router 的 cooling 机制（`allowed_fails` + `cooldown_time`），
//! 参数按免费额度场景调过：
//!
//! | 情况 | 处理 | 为什么 |
//! |---|---|---|
//! | 429 限流 | 立刻冷却 5 秒 | 免费额度打满很常见，换个渠道马上能用 |
//! | 401 / 403 | 冷却 60 秒 | Key 不对或没权限，短时间内自己好不了 |
//! | 404 | 冷却 60 秒 | 地址填错了，重试同一个还是 404 |
//! | 5xx / 连接失败 | 连续 3 次才冷却 30 秒 | 偶发抖动不该直接摘掉，那是误杀 |
//!
//! 关键点：**冷却只针对单个渠道，不影响同组其它渠道**。
//! 三条同平台的 Key 挂了一条，另外两条照样服务。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::classify::FailKind;

/// 连续失败多少次才冷却（5xx / 连接类）
const FAIL_THRESHOLD: u32 = 3;
/// 上面那种情况的冷却时长
const FAIL_COOLDOWN: Duration = Duration::from_secs(30);
/// 429 的冷却时长：限流是暂时的，短一点，尽快回到池子里
const RATE_LIMIT_COOLDOWN: Duration = Duration::from_secs(5);
/// 认证/地址类的冷却：自己不会好，冷久一点
const HARD_COOLDOWN: Duration = Duration::from_secs(60);

#[derive(Default)]
struct State {
    fails: u32,
    cool_until: Option<Instant>,
    /// 累计失败次数（不因成功而清零），给界面展示用
    total_fails: u64,
    last_status: Option<u16>,
    last_error: String,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, State>,
}

fn store() -> &'static Mutex<Inner> {
    static S: std::sync::OnceLock<Mutex<Inner>> = std::sync::OnceLock::new();
    S.get_or_init(|| Mutex::new(Inner::default()))
}

fn with<R>(f: impl FnOnce(&mut Inner) -> R) -> R {
    let mut g = match store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    f(&mut g)
}

/// 这个渠道现在能不能用
pub fn available(id: &str) -> bool {
    with(|g| match g.map.get_mut(id) {
        None => true,
        Some(s) => match s.cool_until {
            None => true,
            Some(t) if t <= Instant::now() => {
                // 冷却到期，自动恢复
                s.cool_until = None;
                s.fails = 0;
                true
            }
            Some(_) => false,
        },
    })
}

fn cool_down(g: &mut Inner, id: &str, dur: Duration, status: Option<u16>, why: &str) {
    let s = g.map.entry(id.to_string()).or_default();
    s.cool_until = Some(Instant::now() + dur);
    s.total_fails += 1;
    s.last_status = status;
    s.last_error = why.to_string();
    crate::gwarn!(
        "上游 {id} 进入冷却 {}s（{why}），期间会自动跳过",
        dur.as_secs()
    );
}

pub fn record_success(id: &str) {
    with(|g| {
        let s = g.map.entry(id.to_string()).or_default();
        s.fails = 0;
        s.cool_until = None;
    });
}

/// 记一次 HTTP 失败。**按失败类型**决定冷却策略，不是只看状态码。
pub fn record_failure(id: &str, kind: FailKind, status: u16) {
    with(|g| match kind {
        FailKind::RateLimit => cool_down(g, id, RATE_LIMIT_COOLDOWN, Some(status), "429 限流"),
        FailKind::Auth | FailKind::NotFound => {
            cool_down(g, id, HARD_COOLDOWN, Some(status), kind.hint())
        }
        FailKind::ServerError => {
            let st = g.map.entry(id.to_string()).or_default();
            st.fails += 1;
            st.total_fails += 1;
            st.last_status = Some(status);
            st.last_error = format!("上游 {status}");
            if st.fails >= FAIL_THRESHOLD {
                cool_down(
                    g,
                    id,
                    FAIL_COOLDOWN,
                    Some(status),
                    &format!("连续 {FAIL_THRESHOLD} 次 {status}"),
                );
            }
        }
        // 上下文超长**不冷却渠道**：不是渠道坏了，是这个请求太大。
        // 冷却它会让紧接着的正常请求也被无谓跳过 30 秒。
        FailKind::ContextTooLong => {
            let st = g.map.entry(id.to_string()).or_default();
            st.total_fails += 1;
            st.last_status = Some(status);
            st.last_error = kind.hint().to_string();
        }
        // 内容策略 / 参数错误：都不是渠道的问题，不计入
        FailKind::ContentPolicy | FailKind::BadRequest | FailKind::Network => {}
    });
}

/// 记一次连不上（超时、连接被拒、DNS 失败）
pub fn record_network_error(id: &str, err: &str) {
    with(|g| {
        let st = g.map.entry(id.to_string()).or_default();
        st.fails += 1;
        st.total_fails += 1;
        st.last_status = None;
        st.last_error = err.chars().take(160).collect();
        if st.fails >= FAIL_THRESHOLD {
            cool_down(g, id, FAIL_COOLDOWN, None, "连续连接失败");
        }
    });
}

pub fn snapshot() -> Value {
    with(|g| {
        let now = Instant::now();
        let items: Vec<Value> = g
            .map
            .iter()
            .map(|(id, s)| {
                let remain = s
                    .cool_until
                    .map(|t| t.saturating_duration_since(now).as_secs())
                    .unwrap_or(0);
                json!({
                    "id": id,
                    "cooling": s.cool_until.map(|t| t > now).unwrap_or(false),
                    "coolRemainSec": remain,
                    "consecutiveFails": s.fails,
                    "totalFails": s.total_fails,
                    "lastStatus": s.last_status,
                    "lastError": s.last_error,
                })
            })
            .collect();
        json!(items)
    })
}

pub fn reset(id: Option<&str>) {
    with(|g| match id {
        Some(x) => {
            g.map.remove(x);
        }
        None => g.map.clear(),
    });
}

#[tauri::command]
pub fn breaker_status() -> Result<Value, String> {
    Ok(snapshot())
}

#[tauri::command]
pub fn breaker_reset(id: Option<String>) -> Result<Value, String> {
    reset(id.as_deref());
    crate::glog!("熔断状态已重置");
    Ok(snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_rules() {
        reset(None);

        // 新渠道默认可用
        assert!(available("a"));

        // 429 立刻冷却
        record_failure("a", FailKind::RateLimit, 429);
        assert!(!available("a"), "429 应该立刻冷却");

        // 别的渠道不受影响
        assert!(available("b"), "冷却应该只影响单个渠道");

        // 401 冷却
        record_failure("b", FailKind::Auth, 401);
        assert!(!available("b"));

        // 5xx 要连续 3 次才冷却
        reset(None);
        record_failure("c", FailKind::ServerError, 500);
        assert!(available("c"), "第 1 次 5xx 不该冷却");
        record_failure("c", FailKind::ServerError, 500);
        assert!(available("c"), "第 2 次 5xx 不该冷却");
        record_failure("c", FailKind::ServerError, 500);
        assert!(!available("c"), "第 3 次 5xx 应该冷却");

        // 成功会清零计数
        reset(None);
        record_failure("d", FailKind::ServerError, 500);
        record_failure("d", FailKind::ServerError, 500);
        record_success("d");
        record_failure("d", FailKind::ServerError, 500);
        assert!(available("d"), "成功之后计数应清零，第 3 次才该冷却");

        // 400 不计入
        reset(None);
        for _ in 0..10 {
            record_failure("e", FailKind::BadRequest, 400);
        }
        assert!(available("e"), "400 是请求本身的问题，不该触发熔断");

        // 上下文超长不该冷却渠道（不是渠道的错）
        reset(None);
        record_failure("f", FailKind::ContextTooLong, 400);
        assert!(available("f"), "上下文超长不该让渠道进入冷却");

        reset(None);
    }
}
