//! 访问 Key：为「把网关给别人用」这个场景做的。
//!
//! 之前只有一个 `control.keys` 字符串数组，而且**前端没有任何入口**——
//! 想给朋友一个 Key，得自己去改 extras.json。更要紧的是没有任何约束：
//! 一旦发出去，对方能无限刷你的上游额度。
//!
//! 这里补三件事：
//! - **能起名字**：三个月后你还知道哪个 Key 是给谁的
//! - **能限次数**：每个 Key 每天最多多少次，超了回 429
//! - **能看到用量**：谁用得多，一眼看出来
//!
//! 兼容：`control.keys` 里如果还是老的纯字符串，读的时候自动当成
//! "没名字、不限次"的 Key，不破坏已有配置。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessKey {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub key: String,
    /// 每天最多多少次请求；null = 不限
    #[serde(default)]
    pub daily_limit: Option<u64>,
    /// 每分钟最多多少次请求；null = 不限。
    /// 日限挡的是"慢慢用超"，分钟限挡的是"短时间猛刷"——后者更容易把
    /// 上游的 RPM 打满，导致你自己也用不了。
    #[serde(default)]
    pub rpm_limit: Option<u64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyUsage {
    pub requests: u64,
    pub denied: u64,
    pub day: String,
}

#[derive(Default)]
struct UsageStore {
    map: HashMap<String, KeyUsage>,
}

#[derive(Default)]
struct RpmStore {
    /// key_id -> (分钟序号, 该分钟已用次数)
    map: HashMap<String, (i64, u64)>,
}

fn rpm_store() -> &'static Mutex<RpmStore> {
    static S: std::sync::OnceLock<Mutex<RpmStore>> = std::sync::OnceLock::new();
    S.get_or_init(|| Mutex::new(RpmStore::default()))
}

fn minute_now() -> i64 {
    chrono::Utc::now().timestamp() / 60
}

fn rpm_used(id: &str) -> u64 {
    let m = minute_now();
    let g = match rpm_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    match g.map.get(id) {
        Some((mm, c)) if *mm == m => *c,
        _ => 0,
    }
}

fn rpm_hit(id: &str) {
    let m = minute_now();
    let mut g = match rpm_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let e = g.map.entry(id.to_string()).or_insert((m, 0));
    if e.0 != m {
        *e = (m, 0);
    }
    e.1 += 1;
}

fn usage_store() -> &'static Mutex<UsageStore> {
    static S: std::sync::OnceLock<Mutex<UsageStore>> = std::sync::OnceLock::new();
    S.get_or_init(|| Mutex::new(UsageStore::default()))
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 生成一个 `sk-gw-` 开头的 Key。
///
/// 用 `RandomState` 取种子 —— 标准库保证每次构造的种子不同，
/// 本机场景足够；不需要密码学强度（真要防爆破，外面还有每日次数限制）。
fn gen_key() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::from("sk-gw-");
    while s.len() < 30 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        h.write_u64(t);
        h.write_u32(std::process::id());
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s.truncate(30);
    s
}

fn load_raw() -> Vec<Value> {
    crate::extras::get("control")
        .get("keys")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
}

/// 读出所有 Key，顺手把老格式（纯字符串）转成结构化的
pub fn list() -> Vec<AccessKey> {
    load_raw()
        .into_iter()
        .enumerate()
        .filter_map(|(i, v)| match v {
            Value::String(s) => {
                if s.trim().is_empty() {
                    None
                } else {
                    // 老格式：只有 Key 字符串，没有名字也不限次
                    Some(AccessKey {
                        id: format!("legacy_{i}"),
                        name: format!("未命名 {}", i + 1),
                        key: s,
                        daily_limit: None,
                        rpm_limit: None,
                        enabled: true,
                    })
                }
            }
            Value::Object(_) => serde_json::from_value(v).ok(),
            // 其它类型是脏数据，忽略掉比崩掉好
            _ => None,
        })
        .collect()
}

fn persist(keys: &[AccessKey]) -> Result<(), String> {
    let arr: Vec<Value> = keys
        .iter()
        .map(|k| serde_json::to_value(k).unwrap_or(Value::Null))
        .collect();
    crate::extras::put_nested("control", "keys", json!(arr))
}

/// 要不要校验（没有配置任何 Key 就是全开，保持便携版开箱即用的手感）
pub fn required() -> bool {
    !list().is_empty()
}

#[derive(Debug)]
pub enum Denied {
    /// 没有这个 Key
    Unknown,
    /// Key 被停用了
    Disabled,
    /// 今天次数用完了
    QuotaExceeded { limit: u64, used: u64 },
    /// 这一分钟刷太快
    RateLimited { limit: u64, used: u64 },
}

impl Denied {
    pub fn message(&self) -> String {
        match self {
            Denied::Unknown => "invalid api key".into(),
            Denied::Disabled => "这个 Key 已被停用".into(),
            Denied::QuotaExceeded { limit, used } => {
                format!("今天已用 {used} 次，达到上限 {limit} 次")
            }
            Denied::RateLimited { limit, used } => {
                format!("这一分钟已经请求 {used} 次，上限 {limit} 次/分")
            }
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            Denied::Unknown | Denied::Disabled => 401,
            Denied::QuotaExceeded { .. } | Denied::RateLimited { .. } => 429,
        }
    }
}

/// 校验一个 Key，通过就返回它（用于后续记用量）
pub fn verify(got: &str) -> Result<AccessKey, Denied> {
    let keys = list();
    let found = keys
        .iter()
        .find(|k| !k.key.is_empty() && k.key == got)
        .ok_or(Denied::Unknown)?
        .clone();
    if !found.enabled {
        return Err(Denied::Disabled);
    }
    if let Some(limit) = found.daily_limit {
        let used = usage_of(&found.id).requests;
        if used >= limit {
            return Err(Denied::QuotaExceeded { limit, used });
        }
    }
    if let Some(limit) = found.rpm_limit {
        let used = rpm_used(&found.id);
        if used >= limit {
            return Err(Denied::RateLimited { limit, used });
        }
        // 通过就立刻记一次：这是"预占"，避免并发时同一个 Key 一起冲过上限
        rpm_hit(&found.id);
    }
    Ok(found)
}

fn usage_of(id: &str) -> KeyUsage {
    let mut g = match usage_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let day = today();
    let e = g.map.entry(id.to_string()).or_default();
    // 跨天自动清零
    if e.day != day {
        e.requests = 0;
        e.denied = 0;
        e.day = day;
    }
    e.clone()
}

pub fn record_ok(id: &str) {
    let mut g = match usage_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let day = today();
    let e = g.map.entry(id.to_string()).or_default();
    if e.day != day {
        e.requests = 0;
        e.denied = 0;
        e.day = day;
    }
    e.requests += 1;
}

pub fn record_denied(id: &str) {
    let mut g = match usage_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let e = g.map.entry(id.to_string()).or_default();
    if e.day != today() {
        e.day = today();
    }
    e.denied += 1;
}

/// 给界面看的：Key 列表 + 今日用量
pub fn snapshot() -> Value {
    let keys = list();
    let items: Vec<Value> = keys
        .iter()
        .map(|k| {
            let u = usage_of(&k.id);
            json!({
                "id": k.id,
                "name": k.name,
                "key": k.key,
                // 只回显后 4 位，避免整个界面都是明文 Key
                "keyTail": k.key.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>(),
                "dailyLimit": k.daily_limit,
                "rpmLimit": k.rpm_limit,
                "enabled": k.enabled,
                "usedToday": u.requests,
                "usedThisMinute": rpm_used(&k.id),
                "deniedToday": u.denied,
            })
        })
        .collect();
    json!({ "required": !keys.is_empty(), "keys": items })
}

// ---------------- 命令 ----------------

#[tauri::command]
pub fn access_keys_get() -> Result<Value, String> {
    Ok(snapshot())
}

/// 保存（新增或修改）。不传 key 就自动生成一个。
#[tauri::command]
pub fn access_key_save(
    id: Option<String>,
    name: String,
    daily_limit: Option<u64>,
    rpm_limit: Option<u64>,
    enabled: Option<bool>,
) -> Result<Value, String> {
    let mut keys = list();
    let enabled = enabled.unwrap_or(true);
    let limit = daily_limit.filter(|v| *v > 0);
    let rpm = rpm_limit.filter(|v| *v > 0);

    match id.filter(|x| !x.is_empty()) {
        Some(edit_id) => {
            let k = keys
                .iter_mut()
                .find(|k| k.id == edit_id)
                .ok_or_else(|| format!("找不到 {edit_id}"))?;
            k.name = name;
            k.daily_limit = limit;
            k.rpm_limit = rpm;
            k.enabled = enabled;
        }
        None => {
            keys.push(AccessKey {
                id: format!("k_{}", &gen_key()[6..14]),
                name,
                key: gen_key(),
                daily_limit: limit,
                rpm_limit: rpm,
                enabled,
            });
        }
    }
    persist(&keys)?;
    crate::glog!("访问 Key 已保存，当前共 {} 个", keys.len());
    Ok(snapshot())
}

/// 按需取完整 Key。
///
/// 为什么不放进 `snapshot()` 一起返回：那会让整个界面的 JSON 里都是明文密钥，
/// 一次截图、一次日志打印就漏了。只在用户主动点"显示"时取这一条。
#[tauri::command]
pub fn access_key_reveal(id: String) -> Result<String, String> {
    list()
        .into_iter()
        .find(|k| k.id == id)
        .map(|k| k.key)
        .ok_or_else(|| format!("找不到 Key {id}"))
}

#[tauri::command]
pub fn access_key_delete(id: String) -> Result<Value, String> {
    let mut keys = list();
    keys.retain(|k| k.id != id);
    persist(&keys)?;
    crate::glog!("访问 Key 已删除，剩余 {} 个", keys.len());
    Ok(snapshot())
}

#[tauri::command]
pub fn access_usage_reset(id: Option<String>) -> Result<Value, String> {
    let mut g = match usage_store().lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    match id {
        Some(x) => {
            g.map.remove(&x);
        }
        None => g.map.clear(),
    }
    Ok(snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_format_and_compat() {
        let k = gen_key();
        assert!(k.starts_with("sk-gw-"), "前缀不对：{k}");
        assert_eq!(k.len(), 30);
        assert_ne!(k, gen_key(), "两次生成不该一样");

        // 老格式（纯字符串数组）能读成结构化
        let legacy = vec![Value::String("sk-old-123".into())];
        let parsed: Vec<AccessKey> = legacy
            .into_iter()
            .enumerate()
            .filter_map(|(i, v)| match v {
                Value::String(s) => Some(AccessKey {
                    id: format!("legacy_{i}"),
                    name: format!("未命名 {}", i + 1),
                    key: s,
                    daily_limit: None,
                    rpm_limit: None,
                    enabled: true,
                }),
                _ => None,
            })
            .collect();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].key, "sk-old-123");
        assert!(parsed[0].daily_limit.is_none());
    }
}
