//! 响应缓存：相同的请求直接命中，省 token 也省延迟。
//!
//! 设计取舍（每一条都是为了不帮倒忙）：
//!
//! - **只缓存非流式**。流式的价值就在"逐字出"，缓存完再拼 SSE 是假流式，
//!   是体验退步，不是优化。
//! - **只在内存，不落盘**。缓存里是对话原文，写磁盘等于把聊天记录留在硬盘上。
//!   进程退出即失效，最省心。
//! - **LRU + TTL + 条数上限**。不设上限的内存缓存，在长期运行的服务里
//!   就是慢性内存泄漏。
//! - **温度 > 0 的请求不缓存**。用户把温度调高就是想每次不一样，
//!   缓存会让"重新生成"按钮失效，会被当成 bug 报上来。
//! - **带 tools 的请求不缓存**。工具调用往往伴随外部状态变化，
//!   返回十分钟前的旧结果比不缓存更糟。
//! - **键里剔除不影响结果的字段**（stream / stream_options / user / upstream），
//!   这样同一个问题在不同上游之间也能复用，省的是真金白银。

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// 默认 10 分钟。太短没有命中率，太长会让用户觉得"模型回答变旧了"。
const TTL_SEC: u64 = 600;
/// 默认最多 200 条。按每条平均 4KB 估算，约 800KB 内存，很轻。
const MAX_ENTRIES: usize = 200;

static ENABLED: AtomicBool = AtomicBool::new(true);
static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);

struct Entry {
    value: Value,
    expires: Instant,
    last_used: Instant,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, Entry>,
}

fn store() -> &'static Mutex<Inner> {
    static S: OnceLock<Mutex<Inner>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Inner::default()))
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on {
        clear();
    }
    crate::glog!("响应缓存{}", if on { "已开启" } else { "已关闭并清空" });
}

/// 这个请求该不该走缓存
pub fn cacheable(body: &Value) -> bool {
    // 流式不缓存
    if body.get("stream").and_then(|x| x.as_bool()).unwrap_or(false) {
        return false;
    }
    // 温度 > 0：用户就是要每次不一样
    if let Some(t) = body.get("temperature").and_then(|x| x.as_f64()) {
        if t > 0.0 {
            return false;
        }
    }
    // 多候选：一次要 n 个结果，缓存一个没意义
    if body.get("n").and_then(|x| x.as_u64()).unwrap_or(1) > 1 {
        return false;
    }
    // 工具调用：结果可能依赖外部状态，缓存会返回过期事实
    if body.get("tools").is_some() || body.get("tool_choice").is_some() {
        return false;
    }
    true
}

/// 缓存键：剔除不影响结果的字段，剩下的按键名排序后哈希。
///
/// 排序是必须的 —— 客户端每次把 JSON 字段顺序打乱一点，不排序就全 miss。
fn key_of(body: &Value) -> String {
    let mut b = body.clone();
    if let Some(o) = b.as_object_mut() {
        for k in ["stream", "stream_options", "user", "upstream"] {
            o.remove(k);
        }
    }
    let mut canon = String::new();
    if let Value::Object(map) = &b {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for k in keys {
            canon.push_str(k);
            canon.push('=');
            canon.push_str(&serde_json::to_string(&map[k]).unwrap_or_default());
            canon.push('\u{1}');
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    canon.hash(&mut h);
    format!("{:016x}", h.finish())
}

pub fn get(body: &Value) -> Option<Value> {
    if !enabled() || !cacheable(body) {
        return None;
    }
    let key = key_of(body);
    let mut g = match store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let now = Instant::now();
    match g.map.get_mut(&key) {
        Some(e) if e.expires > now => {
            e.last_used = now;
            HITS.fetch_add(1, Ordering::Relaxed);
            Some(e.value.clone())
        }
        Some(_) => {
            // 过期了，顺手清掉
            g.map.remove(&key);
            MISSES.fetch_add(1, Ordering::Relaxed);
            None
        }
        None => {
            MISSES.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

pub fn put(body: &Value, value: &Value) {
    if !enabled() || !cacheable(body) {
        return;
    }
    // 只缓存成功响应：错误也缓存的话，用户重试十次都是同一个错
    let has_choices = value
        .get("choices")
        .and_then(|x| x.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if !has_choices {
        return;
    }

    let key = key_of(body);
    let mut g = match store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let now = Instant::now();
    let ttl = Duration::from_secs(TTL_SEC);

    // 满了就淘汰最久没被用过的
    if g.map.len() >= MAX_ENTRIES && !g.map.contains_key(&key) {
        if let Some(victim) = g
            .map
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
        {
            g.map.remove(&victim);
        }
    }

    g.map.insert(
        key,
        Entry {
            value: value.clone(),
            expires: now + ttl,
            last_used: now,
        },
    );
}

pub fn clear() {
    let mut g = match store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    g.map.clear();
}

pub fn stats() -> Value {
    let g = match store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let hits = HITS.load(Ordering::Relaxed);
    let misses = MISSES.load(Ordering::Relaxed);
    let total = hits + misses;
    json!({
        "enabled": enabled(),
        "entries": g.map.len(),
        "maxEntries": MAX_ENTRIES,
        "ttlSec": TTL_SEC,
        "hits": hits,
        "misses": misses,
        "hitRate": if total == 0 { 0.0 } else { (hits as f64 / total as f64 * 100.0).round() / 100.0 },
    })
}

#[tauri::command]
pub fn cache_stats() -> Result<Value, String> {
    Ok(stats())
}

#[tauri::command]
pub fn cache_set_enabled(on: bool) -> Result<Value, String> {
    set_enabled(on);
    Ok(stats())
}

#[tauri::command]
pub fn cache_clear() -> Result<Value, String> {
    clear();
    crate::glog!("响应缓存已手动清空");
    Ok(stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(temp: Option<f64>) -> Value {
        let mut v = json!({"model":"m","messages":[{"role":"user","content":"你好"}]});
        if let Some(t) = temp {
            v.as_object_mut().unwrap().insert("temperature".into(), json!(t));
        }
        v
    }

    #[test]
    fn hit_miss_and_rules() {
        set_enabled(true);
        clear();
        HITS.store(0, Ordering::Relaxed);
        MISSES.store(0, Ordering::Relaxed);

        let body = req(None);
        let resp = json!({"choices":[{"message":{"role":"assistant","content":"在"}}]});

        // 空缓存 → 未命中
        assert!(get(&body).is_none(), "空缓存不该命中");
        put(&body, &resp);
        // 存过 → 命中，且内容一致
        let got = get(&body).expect("应该命中");
        assert_eq!(got["choices"][0]["message"]["content"], "在");

        // 字段顺序不同的等价请求，应该命中同一个键
        let reordered = json!({"messages":[{"content":"你好","role":"user"}],"model":"m"});
        assert!(get(&reordered).is_some(), "字段顺序不同不该 miss");

        // 温度 > 0 不缓存
        let hot = req(Some(0.9));
        put(&hot, &resp);
        assert!(get(&hot).is_none(), "温度>0 不该缓存");

        // 温度 = 0 缓存
        let cold = req(Some(0.0));
        put(&cold, &resp);
        assert!(get(&cold).is_some(), "温度=0 应该缓存");

        // 带 tools 不缓存
        let mut tool = body.clone();
        tool.as_object_mut()
            .unwrap()
            .insert("tools".into(), json!([{"type":"function","function":{"name":"f"}}]));
        put(&tool, &resp);
        assert!(get(&tool).is_none(), "带 tools 不该缓存");

        // 流式不缓存
        let mut st = body.clone();
        st.as_object_mut().unwrap().insert("stream".into(), json!(true));
        assert!(!cacheable(&st));

        // 错误响应不进缓存
        let err = json!({"error":{"message":"boom"}});
        let b2 = json!({"model":"m","messages":[{"role":"user","content":"另一句"}]});
        put(&b2, &err);
        assert!(get(&b2).is_none(), "错误响应不该缓存");

        // 关掉开关就全 miss
        set_enabled(false);
        assert!(get(&body).is_none(), "关闭后不该命中");
        set_enabled(true);
        clear();
    }
}
