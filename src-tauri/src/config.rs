use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::store::portable_root;

fn config_path() -> PathBuf {
    portable_root().join("config.json")
}

/// 直接读便携版 config.json（预置配置，不依赖面板运行）
#[tauri::command]
pub fn local_config_get() -> Result<Value, String> {
    let p = config_path();
    let text =
        fs::read_to_string(&p).map_err(|e| format!("读取 config.json 失败：{e}（{}）", p.to_string_lossy()))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(json!({ "config": v, "path": p.to_string_lossy() }))
}

/// 写回便携版 config.json（先备份）
#[tauri::command]
pub fn local_config_save(config: Value) -> Result<Value, String> {
    let p = config_path();
    if p.exists() {
        let _ = fs::copy(&p, p.with_extension("json.axiao-bak"));
    }
    let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&p, text).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "path": p.to_string_lossy() }))
}

/// 便携版概况：路径、监听、密钥、账号数
#[tauri::command]
pub fn portable_info() -> Result<Value, String> {
    let root = portable_root();
    let cfg = fs::read_to_string(root.join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));
    let auth_count = fs::read_dir(root.join("auths"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    e.path().extension().and_then(|x| x.to_str()) == Some("json")
                })
                .count()
        })
        .unwrap_or(0);
    Ok(json!({
        "root": root.to_string_lossy(),
        "configPath": root.join("config.json").to_string_lossy(),
        "authsPath": root.join("auths").to_string_lossy(),
        "listen": cfg.get("listen").and_then(|x| x.as_str()).unwrap_or("127.0.0.1:7863"),
        "apiKey": cfg.get("api_key").and_then(|x| x.as_str()).unwrap_or(""),
        "authCount": auth_count,
        "version": "v1.13.0",
    }))
}
