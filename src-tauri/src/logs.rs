//! 网关运行时日志。
//!
//! 为什么要有这个：本地 API 是「接进别人客户端」的中间层，出问题的时候
//! 用户只能看到客户端报错，完全不知道我们这边发生了什么（是不是没选上游、
//! 是不是上游 401、是不是流断了）。所以从服务启动、选上游、发请求、每帧
//! 大小、结束原因，全部落到一个文件里，界面「运行日志」页直接读这个文件。

use std::io::Write;
use std::sync::Mutex;

use crate::store::data_dir;

/// 单文件上限 5MB，超了滚成 .1（只留一份，避免无限占盘）
const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// 多线程同时写同一文件会互相穿插成半行，加把锁
static LOCK: Mutex<()> = Mutex::new(());

pub fn log_path() -> std::path::PathBuf {
    data_dir().join("gateway.log")
}

/// 写一行日志：控制台 + 文件双写。文件写失败就静默（日志不能反过来搞崩服务）。
pub fn write(level: &str, msg: &str) {
    // 当前线程如果有请求 id 就带上，这样按 id 搜日志能串起整条链路。
    // 异步任务里读不到（tokio 会切线程），那些地方由调用方自己拼进 msg。
    let tag = crate::trace::get()
        .map(|i| format!("[{i}] "))
        .unwrap_or_default();
    let line = format!(
        "{} [{}] {}{}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        level,
        tag,
        msg
    );

    // ⚠️ 绝对不要往 stdout / stderr 写。
    //
    // 本程序是 `windows_subsystem = "windows"`（GUI 子系统），正常运行没有控制台，
    // 输出本来也没人看得到。更要命的是：在 GNU 目标上写 stdio 会把
    // `fprintf` / `fwrite` / `vfprintf` / `__iob_func` 拉进导入表，
    // 而这几个符号在本机 msvcrt.dll 上解析不到 —— 进程还没进 main 就
    // 以 0xC0000139（STATUS_ENTRYPOINT_NOT_FOUND）退出，表现是"双击没反应"。
    // 已实测：去掉 stdout 后导入表从 43 项回到 36 项，与能跑的旧版本一致。
    //
    // 日志只落文件：界面「运行日志」页读它，--serve-only 模式也用 tail 看它。

    let _guard = match LOCK.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let _ = std::fs::create_dir_all(data_dir());
    let p = log_path();
    if let Ok(md) = std::fs::metadata(&p) {
        if md.len() > MAX_BYTES {
            let _ = std::fs::rename(&p, data_dir().join("gateway.log.1"));
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// 读最后 N 行，供界面展示
pub fn tail(n: usize) -> Vec<String> {
    let text = std::fs::read_to_string(log_path()).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(n);
    all[start..].iter().map(|s| s.to_string()).collect()
}

pub fn clear() {
    let _ = std::fs::remove_file(log_path());
}

#[tauri::command]
pub fn gateway_logs(tail_lines: Option<usize>) -> Result<serde_json::Value, String> {
    let n = tail_lines.unwrap_or(300).clamp(1, 5000);
    let lines = tail(n);
    let size = std::fs::metadata(log_path()).map(|m| m.len()).unwrap_or(0);
    Ok(serde_json::json!({
        "path": log_path().to_string_lossy(),
        "size": size,
        "lines": lines,
    }))
}

#[tauri::command]
pub fn gateway_logs_clear() -> Result<bool, String> {
    clear();
    Ok(true)
}

/// `glog!("上游 {} 返回 {}", id, status)`
#[macro_export]
macro_rules! glog {
    ($($arg:tt)*) => { $crate::logs::write("INFO", &format!($($arg)*)) };
}

/// `gwarn!("上游 {} 超时", id)`
#[macro_export]
macro_rules! gwarn {
    ($($arg:tt)*) => { $crate::logs::write("WARN", &format!($($arg)*)) };
}
