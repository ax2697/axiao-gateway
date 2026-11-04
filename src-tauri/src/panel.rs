use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

const PANEL_BASE: &str = "http://127.0.0.1:7863/panel/api";

/// 读取本地网关 api_key：环境变量 > 便携版 config.json > 相对路径
pub fn gateway_key() -> String {
    if let Ok(k) = std::env::var("WB_GATEWAY_KEY") {
        if !k.trim().is_empty() {
            return k;
        }
    }
    let candidates: Vec<PathBuf> = vec![
        crate::store::portable_root().join("config.json"),
        PathBuf::from("../config.json"),
        PathBuf::from("../../config.json"),
        crate::store::home()
            .join("Desktop")
            .join("WorkBuddy网关-便携版")
            .join("config.json"),
    ];
    for p in candidates {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<Value>(&s) {
                if let Some(k) = v.get("api_key").and_then(|x| x.as_str()) {
                    if !k.is_empty() {
                        return k.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

async fn request_body(method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let url = format!("{PANEL_BASE}/{path}");
    let client = crate::httpc::client_for("http://127.0.0.1").unwrap_or_default();
    let mut rb = match method {
        "POST" => client.post(&url),
        _ => client.get(&url),
    }
    .timeout(Duration::from_secs(25));
    if let Some(b) = body {
        rb = rb.json(&b);
    } else if method == "POST" {
        rb = rb.json(&serde_json::json!({}));
    }
    let key = gateway_key();
    if !key.is_empty() {
        rb = rb.bearer_auth(key);
    }
    let resp = rb
        .send()
        .await
        .map_err(|e| format!("面板不可达（先启动便携版网关）：{e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let text = v.to_string();
        // 上游限流：面板把 429 包成 502 透传，这里统一识别成 RATE_LIMITED，
        // 便于上层重试与前端给出人话提示（而不是甩一坨 JSON）。
        let limited = status.as_u16() == 429
            || text.contains("429")
            || text.to_lowercase().contains("too many requests");
        if limited {
            return Err(format!("RATE_LIMITED: 上游限流（{status}）"));
        }
        return Err(format!("面板返回 {status}: {v}"));
    }
    Ok(v)
}

async fn request(method: &str, path: &str) -> Result<Value, String> {
    request_body(method, path, None).await
}

/// 读取面板完整配置
#[tauri::command]
pub async fn panel_get_config() -> Result<Value, String> {
    request("GET", "config").await
}

/// 保存面板配置（整对象回写）
#[tauri::command]
pub async fn panel_save_config(config: Value) -> Result<Value, String> {
    request_body("POST", "config", Some(config)).await
}

/// 浏览器登录：获取授权链接（realm = cn / global）
#[tauri::command]
pub async fn panel_login_start(realm: String) -> Result<Value, String> {
    request_body(
        "POST",
        "login/start",
        Some(serde_json::json!({ "realm": realm })),
    )
    .await
}

/// 浏览器登录：轮询授权状态
#[tauri::command]
pub async fn panel_login_poll(state: String) -> Result<Value, String> {
    let path = format!("login/poll?state={}", urlencoding(&state));
    request("GET", &path).await
}

/// 导入 JSON（cockpit 格式）：按本地路径上传
#[tauri::command]
pub async fn panel_import_cockpit(path: String) -> Result<Value, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败：{e}"))?;
    let name = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("import.json")
        .to_string();
    upload_cockpit(name, bytes).await
}

/// 导入 JSON：前端读到的文本直接上传（不依赖本地路径）
#[tauri::command]
pub async fn panel_import_cockpit_text(name: String, text: String) -> Result<Value, String> {
    upload_cockpit(name, text.into_bytes()).await
}

async fn upload_cockpit(name: String, bytes: Vec<u8>) -> Result<Value, String> {
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(name)
        .mime_str("application/json")
        .map_err(|e| e.to_string())?;
    let form = reqwest::multipart::Form::new().part("file", part);
    let url = format!("{PANEL_BASE}/import/cockpit");
    let client = crate::httpc::client_for("http://127.0.0.1").unwrap_or_default();
    let mut rb = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(60));
    let key = gateway_key();
    if !key.is_empty() {
        rb = rb.bearer_auth(key);
    }
    let resp = rb
        .send()
        .await
        .map_err(|e| format!("面板不可达（先启动便携版网关）：{e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("导入失败 {status}: {v}"));
    }
    Ok(v)
}

/// 用系统默认浏览器打开授权链接
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    // 允许 http/https（普通网页）和 workbuddy://（客户端的专家/技能 deeplink）。
    // 后者是任务页「去完成」按钮要用的，必须先由 WorkBuddy 注册协议。
    let ok = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("workbuddy://");
    if !ok {
        return Err("只允许打开 http/https/workbuddy:// 链接".into());
    }
    // 授权链接形如 .../login?platform=CLI&state=xxx，里面的 & 会被 cmd 当成命令分隔符，
    // 导致 state 被截掉、登录页报「登录链接不完整」(100034A)。
    // 所以这里必须走 raw_arg，自己把 URL 用双引号整段包住。
    crate::openuri::open_url(&url)
}

/// 账号 + 积分套餐（实时查上游）
#[tauri::command]
pub async fn panel_packages() -> Result<Value, String> {
    request("GET", "packages").await
}

/// 指定账号的任务列表（实时抓取）
#[tauri::command]
pub async fn panel_tasks(uid: String) -> Result<Value, String> {
    let path = format!("accounts/{}/tasks", urlencoding(&uid));
    request("GET", &path).await
}

/// 一键完成该账号可自动任务
#[tauri::command]
pub async fn panel_task_auto_all(uid: String) -> Result<Value, String> {
    let path = format!("accounts/{}/tasks/auto_all", urlencoding(&uid));
    // 上游是「软限流」，429 时这批请求根本没执行，重试是安全的。
    // 3 次退避：3s → 8s → 15s，共约 26 秒；仍失败就把人话错误交给前端。
    let waits = [3u64, 8, 15];
    let mut last = String::new();
    for (i, w) in waits.iter().enumerate() {
        match request("POST", &path).await {
            Ok(v) => return Ok(v),
            Err(e) if e.starts_with("RATE_LIMITED") => {
                last = e;
                if i + 1 < waits.len() {
                    tokio::time::sleep(std::time::Duration::from_secs(*w)).await;
                }
            }
            Err(e) => return Err(e),
        }
    }
    let _ = last;
    Err(format!(
        "RATE_LIMITED: 已重试 {} 次仍被上游限流，等 5~10 分钟再点（限流来自上游，不是本应用）",
        waits.len()
    ))
}

/// 任务队列状态
#[tauri::command]
pub async fn panel_task_queue() -> Result<Value, String> {
    request("GET", "tasks/queue").await
}

/// 模型与档位（实时查上游）
#[tauri::command]
pub async fn panel_models() -> Result<Value, String> {
    request("GET", "models").await
}

/// 极简 percent-encoding（uid 为 UUID，安全字符足够）
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                c.to_string()
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf)
                    .bytes()
                    .map(|b| format!("%{b:02X}"))
                    .collect()
            }
        })
        .collect()
}
