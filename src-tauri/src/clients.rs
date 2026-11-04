use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::store::home;

fn candidates() -> Vec<PathBuf> {
    let roaming = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("AppData").join("Roaming"));
    vec![
        roaming.join("CodeBuddy").join("settings.json"),
        home().join(".codebuddy").join("settings.json"),
        home().join(".config").join("codebuddy").join("settings.json"),
        home().join(".codebuddy").join("config.json"),
    ]
}

/// 检测 CodeBuddy CLI 配置
#[tauri::command]
pub fn cli_detect() -> Result<Value, String> {
    let found = candidates().into_iter().find(|p| p.exists());
    Ok(json!({
        "found": found.is_some(),
        "path": found.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "candidates": candidates().iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
    }))
}

fn find_token(v: &Value) -> Option<String> {
    for k in ["accessToken", "access_token", "token", "apiKey", "api_key"] {
        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    if let Some(auth) = v.get("auth").and_then(|x| x.as_object()) {
        for k in ["accessToken", "access_token", "token"] {
            if let Some(s) = auth.get(k).and_then(|x| x.as_str()) {
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

/// 读取 CodeBuddy CLI 的登录 Token
#[tauri::command]
pub fn cli_import() -> Result<Value, String> {
    let path = candidates()
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| "未找到 CodeBuddy CLI 配置（settings.json）".to_string())?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let token = find_token(&v).ok_or_else(|| "配置文件里没有找到 Token".to_string())?;
    Ok(json!({
        "path": path.to_string_lossy(),
        "token": token,
        "tokenMasked": format!("{}…{}", &token[..token.len().min(6)], &token[token.len().saturating_sub(4)..]),
    }))
}

fn backup_write(path: &PathBuf, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if path.exists() {
        let _ = fs::copy(path, path.with_extension("axiao-bak"));
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

/// 给原生客户端导出网关配置（Claude Code / Codex / DSH / 通用 JSON）
#[tauri::command]
pub fn export_client(client: String, base_url: String, api_key: String) -> Result<Value, String> {
    let base = base_url.trim_end_matches('/').to_string();
    match client.as_str() {
        "claude_code" => {
            let path = home().join(".claude").join("settings.json");
            let mut v: Value = fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| json!({}));
            let env = v
                .as_object_mut()
                .ok_or("配置根节点不是对象")?
                .entry("env")
                .or_insert_with(|| json!({}));
            if let Some(o) = env.as_object_mut() {
                o.insert("ANTHROPIC_BASE_URL".into(), json!(base));
                o.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(api_key));
            }
            backup_write(&path, &serde_json::to_string_pretty(&v).unwrap())?;
            Ok(json!({ "client": client, "path": path.to_string_lossy(), "written": true }))
        }
        "codex" => {
            let path = home().join(".codex").join("config.toml");
            let mut text = fs::read_to_string(&path).unwrap_or_default();
            if !text.contains("[model_providers.axiao]") {
                text.push_str(&format!(
                    "\n[model_providers.axiao]\nname = \"Axiao Gateway\"\nbase_url = \"{base}\"\nenv_key = \"AXIAO_API_KEY\"\n"
                ));
            }
            backup_write(&path, &text)?;
            Ok(json!({ "client": client, "path": path.to_string_lossy(), "written": true, "env": "AXIAO_API_KEY" }))
        }
        "dsh" => {
            let path = home().join(".dsh").join("axiao.json");
            let body = json!({ "baseUrl": base, "apiKey": api_key, "provider": "openai" });
            backup_write(&path, &serde_json::to_string_pretty(&body).unwrap())?;
            Ok(json!({ "client": client, "path": path.to_string_lossy(), "written": true }))
        }
        "generic" => Ok(json!({
            "client": client,
            "baseUrl": base,
            "apiKey": api_key,
            "snippet": { "base_url": base, "api_key": api_key }
        })),
        other => Err(format!("不支持的客户端：{other}")),
    }
}
