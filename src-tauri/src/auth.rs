use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Value};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthWriteResult {
    pub auth_file: String,
    pub merged: bool,
    pub backup: Option<String>,
}

pub fn auth_file_path() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::store::home().join("AppData").join("Local"));
    base.join("CodeBuddyExtension")
        .join("Data")
        .join("Public")
        .join("auth.workbuddy.workbuddy-ai")
}

fn set_key(obj: &mut serde_json::Map<String, Value>, camel: &str, snake: &str, value: Value) {
    if obj.contains_key(camel) {
        obj.insert(camel.to_string(), value);
    } else if obj.contains_key(snake) {
        obj.insert(snake.to_string(), value);
    } else {
        obj.insert(camel.to_string(), value);
    }
}

#[tauri::command]
pub fn read_auth() -> Result<Value, String> {
    let p = auth_file_path();
    if !p.exists() {
        return Ok(json!({ "exists": false, "path": p.to_string_lossy() }));
    }
    let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(json!({ "exists": true, "path": p.to_string_lossy(), "data": v }))
}

#[tauri::command]
pub fn switch_account(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<AuthWriteResult, String> {
    let account = {
        let s = state.store.lock().map_err(|e| e.to_string())?;
        s.data
            .accounts
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or_else(|| "账号不存在".to_string())?
    };
    let access = account
        .access_token
        .clone()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "该账号没有 accessToken，无法切换".to_string())?;

    let path = auth_file_path();
    let mut current: Value = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|_| {
            format!("授权文件不是合法 JSON，已拒绝写入：{}", path.to_string_lossy())
        })?
    } else {
        json!({})
    };

    let backup = if path.exists() {
        let bak = path.with_extension(format!(
            "workbuddy-ai.axiao-bak-{}",
            chrono::Utc::now().timestamp()
        ));
        fs::copy(&path, &bak).ok();
        Some(bak.to_string_lossy().to_string())
    } else {
        None
    };

    let obj = current
        .as_object_mut()
        .ok_or_else(|| "授权文件根节点不是对象".to_string())?;
    set_key(obj, "accessToken", "access_token", Value::String(access));
    if let Some(r) = account.refresh_token.clone().filter(|t| !t.is_empty()) {
        set_key(obj, "refreshToken", "refresh_token", Value::String(r));
    }
    if let Some(e) = account.expires_at.clone().filter(|t| !t.is_empty()) {
        set_key(obj, "expiresAt", "expires_at", Value::String(e));
    }

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, serde_json::to_string_pretty(&current).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(AuthWriteResult {
        auth_file: path.to_string_lossy().to_string(),
        merged: true,
        backup,
    })
}

/// 把敏感值存入 Windows 凭据管理器，返回引用串
#[tauri::command]
pub fn store_secret(id: String, secret: String) -> Result<String, String> {
    let entry =
        keyring::Entry::new("axiao-gateway", &id).map_err(|e| e.to_string())?;
    entry.set_password(&secret).map_err(|e| e.to_string())?;
    Ok(format!("keyring:axiao-gateway:{id}"))
}

/// 从凭据管理器取出敏感值
#[tauri::command]
pub fn load_secret(id: String) -> Result<String, String> {
    let entry =
        keyring::Entry::new("axiao-gateway", &id).map_err(|e| e.to_string())?;
    entry.get_password().map_err(|e| e.to_string())
}
