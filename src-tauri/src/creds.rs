use std::path::PathBuf;

use serde_json::{json, Value};

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn appdata(roaming: bool) -> PathBuf {
    let key = if roaming { "APPDATA" } else { "LOCALAPPDATA" };
    std::env::var(key)
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("AppData").join(if roaming { "Roaming" } else { "Local" }))
}

/// 候选清单：(平台名, 建议填到哪个上游, 文件路径)
fn candidates() -> Vec<(String, String, PathBuf)> {
    let h = home();
    let local = appdata(false);
    let roaming = appdata(true);
    let mut v: Vec<(String, String, PathBuf)> = vec![
        ("CodeBuddy CLI".into(), "workbuddy".into(), h.join(".codebuddy").join("settings.json")),
        ("ZCode / Z.AI".into(), "z_ai".into(), h.join(".zcode").join("cli").join("config.json")),
        ("DSH".into(), "dsh".into(), h.join(".dsh").join(".credentials.yaml")),
        ("DSH (config)".into(), "dsh".into(), h.join(".dsh").join("config.json")),
        ("OpenCode".into(), "opencode_zen".into(), h.join(".opencode").join("config.json")),
        ("OpenCode (config)".into(), "opencode_zen".into(), h.join(".config").join("opencode").join("config.json")),
        ("Kimi CLI".into(), "kimi".into(), h.join(".kimi").join("config.json")),
        ("Qoder".into(), "qoder".into(), h.join(".qoder").join("config.json")),
        ("Claude Code".into(), "anthropic".into(), h.join(".claude").join("settings.json")),
        ("WorkBuddy 授权".into(), "workbuddy".into(), local.join("CodeBuddyExtension").join("Data").join("Public").join("auth.workbuddy.workbuddy-ai")),
        ("Trae".into(), "trae_cn".into(), roaming.join("Trae").join("User").join("settings.json")),
    ];
    // 便携版 auths 目录（不在 home 下的也算上）
    if let Ok(root) = std::env::var("WB_GATEWAY_HOME") {
        v.push(("便携版账号池".into(), "workbuddy".into(), PathBuf::from(root).join("auths")));
    }
    v.retain(|(_, _, p)| p.exists());
    v
}

/// 从 JSON / 纯文本里捞出像 Key 的东西
fn extract_keys(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    // 1) JSON 里的常见字段名
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        fn walk(v: &Value, out: &mut Vec<(String, String)>) {
            match v {
                Value::Object(m) => {
                    for (k, val) in m {
                        let lk = k.to_lowercase();
                        let is_key = lk.contains("apikey")
                            || lk.contains("api_key")
                            || lk == "key"
                            || lk.contains("token")
                            || lk.contains("secret");
                        if is_key {
                            if let Some(s) = val.as_str() {
                                if s.len() >= 16 && !s.contains(' ') {
                                    out.push((k.clone(), s.to_string()));
                                }
                            }
                        }
                        walk(val, out);
                    }
                }
                Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                _ => {}
            }
        }
        walk(&v, &mut out);
    }
    // 2) 纯文本里的 sk-xxx / 长 token
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with("//") {
            continue;
        }
        // sk-xxx 裸值
        if l.starts_with("sk-") && l.len() > 20 && !l.contains(' ') {
            out.push(("sk".into(), l.to_string()));
            continue;
        }
        // KEY: value / KEY = value（YAML、.env、properties 都能覆盖）
        let sep = l.find(':').or_else(|| l.find('='));
        let Some(pos) = sep else { continue };
        let (raw_k, raw_v) = l.split_at(pos);
        let k = raw_k.trim().trim_start_matches('-').trim().to_string();
        let v = raw_v[1..]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_string();
        let lk = k.to_lowercase();
        let sensitive = lk.contains("key")
            || lk.contains("token")
            || lk.contains("secret")
            || lk.contains("password");
        if sensitive && v.len() >= 16 && !v.contains(' ') {
            out.push((k, v));
        }
    }
    out.sort();
    out.dedup_by(|a, b| a.1 == b.1);
    out
}

fn mask(s: &str) -> String {
    if s.len() <= 10 {
        return "****".into();
    }
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

#[tauri::command]
pub fn scan_local_creds() -> Result<Value, String> {
    let mut found = Vec::new();
    for (platform, upstream, path) in candidates() {
        if path.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&path) {
                for e in rd.flatten().take(5) {
                    let p = e.path();
                    let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
                    let ok = matches!(ext, "json" | "yaml" | "yml" | "toml" | "env")
                        || p.file_name().and_then(|x| x.to_str()) == Some(".credentials.yaml");
                    if !ok {
                        continue;
                    }
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        for (field, key) in extract_keys(&text) {
                            found.push(json!({
                                "platform": platform, "upstream": upstream,
                                "path": p.to_string_lossy(), "field": field,
                                "masked": mask(&key), "raw": key, "size": text.len()
                            }));
                        }
                    }
                }
            }
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (field, key) in extract_keys(&text) {
            found.push(json!({
                "platform": platform, "upstream": upstream,
                "path": path.to_string_lossy(), "field": field,
                "masked": mask(&key), "raw": key, "size": text.len()
            }));
        }
    }
    Ok(json!({
        "scanned": candidates().len(),
        "found": found,
        "note": if found.is_empty() {
            "本机没找到可复用的登录态。如果你用过某个客户端并登录过，它一般会在自己的配置目录里留 token；都没装过就只能去平台控制台建 Key。"
        } else {
            "这些是从本机客户端配置里读到的凭据，点「填入」就用它当上游 Key。"
        },
    }))
}
