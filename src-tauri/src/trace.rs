//! 请求追踪 ID。
//!
//! 为什么值得做：用户报「刚才那次调用失败了」时，没有 id 就只能按时间瞎猜。
//! 有了 id，日志、账本、响应头三处能对上，一眼看到这次请求走到了哪个渠道、
//! 跳过了谁、最后为什么失败。
//!
//! 两个使用面：
//! - **同步链路**（HTTP 收包、路由、写响应）：用 thread-local，日志函数自动带上，
//!   调用点不用改。
//! - **异步链路**（tokio 里跑的上游请求、流式 pump）：**不能靠 thread-local** ——
//!   任务会在 worker 线程之间迁移，读到的可能是别的请求的 id。这些地方必须
//!   把 id 显式传下去。
//!
//! 客户端自带 `X-Request-Id` 时优先用它，这样可以和自己系统里的日志串起来；
//! 没带就生成一个返回在响应头里。

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

thread_local! {
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// 生成一个短 id。不需要密码学强度 —— 它只是用来对齐同一台机器上的日志。
pub fn new_id() -> String {
    static N: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let seq = N.fetch_add(1, Ordering::Relaxed);
    // 混一个质数乘法打散，避免相邻请求的 id 看起来连号
    let mixed = nanos ^ seq.wrapping_mul(2_654_435_761);
    format!("req_{:08x}", mixed)
}

pub fn set(id: &str) {
    CURRENT.with(|c| {
        *c.borrow_mut() = Some(id.to_string());
    });
}

pub fn get() -> Option<String> {
    CURRENT.with(|c| c.borrow().clone())
}

pub fn clear() {
    CURRENT.with(|c| {
        *c.borrow_mut() = None;
    });
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        clear();
    }
}

/// 设置当前线程的请求 id，返回的 guard 在作用域结束时清掉。
pub fn begin(id: &str) -> Guard {
    set(id);
    Guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrip() {
        clear();
        assert!(get().is_none());
        {
            let _g = begin("req_test1234");
            assert_eq!(get().as_deref(), Some("req_test1234"));
        }
        // guard 离开作用域后自动清掉，不会把 id 泄漏给下一个请求
        assert!(get().is_none());
    }

    #[test]
    fn ids_are_distinct() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(a.starts_with("req_"));
        assert_eq!(a.len(), 12);
    }
}
