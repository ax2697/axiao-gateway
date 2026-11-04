use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Value};

use crate::model::{Account, ClientRule, ExportBundle};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: usize,
    pub updated: usize,
    pub total: usize,
}

pub struct Store {
    pub dir: PathBuf,
    pub file: PathBuf,
    pub portable: PathBuf,
    pub data: ExportBundle,
}

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    // 环境变量覆盖：多实例并跑、自动化测试、把数据放 U 盘，都用它隔离，
    // 免得测试一跑就把用户真账号/真上游冲掉。
    if let Ok(p) = std::env::var("WB_AXIAO_DATA") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    home().join(".wb-switch").join("axiao")
}

/// 定位便携版网关根目录（含 config.json / auths / data）
///
/// 查找顺序：
/// 1. 环境变量 `WB_GATEWAY_HOME`
/// 2. exe 同级的 `WorkBuddy网关-便携版\`（绿色版最常见的摆法）
/// 3. exe 上一级的 `WorkBuddy网关-便携版\`
/// 4. exe 所在目录本身（把 exe 直接丢进便携版目录的摆法）
/// 5. 桌面 `WorkBuddy网关-便携版\`
/// 6. 当前工作目录的上两级 / 上一级
pub fn portable_root() -> PathBuf {
    if let Ok(p) = std::env::var("WB_GATEWAY_HOME") {
        let p = PathBuf::from(p);
        if p.join("config.json").exists() {
            return p;
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        candidates.push(dir.join("WorkBuddy网关-便携版"));
        candidates.push(dir.join("..").join("WorkBuddy网关-便携版"));
        candidates.push(dir.clone());
    }
    candidates.push(home().join("Desktop").join("WorkBuddy网关-便携版"));
    candidates.push(PathBuf::from("..").join(".."));
    candidates.push(PathBuf::from(".."));

    for p in candidates {
        if p.join("config.json").exists() {
            return p;
        }
    }
    home().join("Desktop").join("WorkBuddy网关-便携版")
}

pub fn default_rules() -> Vec<ClientRule> {
    vec![
        ClientRule {
            name: "workbuddy".into(),
            display_name: Some("WorkBuddy".into()),
            config_path: Some(json!([".workbuddy", "mcp.json"])),
            marker_path: Some(json!([".workbuddy"])),
            skill_path: Some(json!([".workbuddy", "skills"])),
            format: "json".into(),
            auth_file: Some("auths/workbuddy-<uid>.json".into()),
            api_base: Some("https://copilot.tencent.com".into()),
        },
        ClientRule {
            name: "codebuddy".into(),
            display_name: Some("CodeBuddy".into()),
            config_path: Some(json!([".codebuddy", "mcp.json"])),
            marker_path: Some(json!([".codebuddy"])),
            skill_path: Some(json!([".codebuddy", "skills"])),
            format: "json".into(),
            auth_file: None,
            api_base: None,
        },
        ClientRule {
            name: "trae".into(),
            display_name: Some("Trae".into()),
            config_path: Some(json!(["AppData/Roaming/Trae/User/mcp.json"])),
            marker_path: Some(json!(["AppData/Roaming/Trae/User"])),
            skill_path: Some(json!([".trae", "skills"])),
            format: "json".into(),
            auth_file: None,
            api_base: None,
        },
    ]
}

impl Store {
    pub fn load() -> Self {
        let dir = data_dir();
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("accounts.json");
        let portable = portable_root();

        let mut data = if file.exists() {
            fs::read_to_string(&file)
                .ok()
                .and_then(|s| serde_json::from_str::<ExportBundle>(&s).ok())
                .unwrap_or_else(Self::seed)
        } else {
            Self::seed()
        };

        // 真实账号以便携版 auths + data/state.json 为准
        merge_portable(&portable, &mut data);

        Self {
            dir,
            file,
            portable,
            data,
        }
    }

    fn seed() -> ExportBundle {
        ExportBundle {
            schema_version: "1.0.0".into(),
            exported_at: Some(chrono::Utc::now().to_rfc3339()),
            generator: Some("axiao-gateway".into()),
            client_rules: default_rules(),
            accounts: Vec::new(),
        }
    }

    /// 只持久化手动/导入账号；真实账号每次从便携版实时读取
    pub fn save(&mut self) -> Result<(), String> {
        self.data.exported_at = Some(chrono::Utc::now().to_rfc3339());
        self.data.generator = Some("axiao-gateway".into());
        let mut out = self.data.clone();
        out.accounts = out
            .accounts
            .into_iter()
            .filter(|a| a.source.as_deref() != Some("portable"))
            .collect();
        let text = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
        fs::write(&self.file, text).map_err(|e| e.to_string())
    }

    pub fn switch_accounts(&self) -> Vec<Value> {
        let p = self.portable.join("auths");
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(p) {
            for e in rd.flatten() {
                if let Ok(s) = fs::read_to_string(e.path()) {
                    if let Ok(v) = serde_json::from_str::<Value>(&s) {
                        out.push(v);
                    }
                }
            }
        }
        out
    }
}

fn merge_portable(root: &std::path::Path, bundle: &mut ExportBundle) {
    let state: Value = fs::read_to_string(root.join("data").join("state.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({ "accounts": {} }));
    let states = state.get("accounts").and_then(|v| v.as_object());

    let auths_dir = root.join("auths");
    let Ok(rd) = fs::read_dir(&auths_dir) else {
        return;
    };

    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let account = v.get("account").cloned().unwrap_or_else(|| json!({}));
        let auth = v.get("auth").cloned().unwrap_or_else(|| json!({}));
        let uid = account
            .get("uid")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        if uid.is_empty() {
            continue;
        }
        let nickname = account
            .get("nickname")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let st = states
            .and_then(|m| m.get(uid.as_str()))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let credits = st.get("credits").and_then(|x| x.as_f64());
        let total = st.get("credits_total").and_then(|x| x.as_f64());
        let disabled = st.get("disabled").and_then(|x| x.as_bool()).unwrap_or(false);
        let credit_expiry = st
            .get("credits_earliest_expiry")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let last_success = st
            .get("last_success")
            .and_then(|x| x.as_str())
            .filter(|s| !s.starts_with("0001"))
            .map(|s| s.to_string());

        let expires = auth
            .get("expiresAt")
            .and_then(|x| x.as_i64())
            .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
            .map(|d| d.to_rfc3339());

        let acct = Account {
            id: format!("wb_{uid}"),
            client: "workbuddy".into(),
            display_name: if nickname.is_empty() {
                uid.clone()
            } else {
                nickname
            },
            account_id: uid.clone(),
            access_token: auth
                .get("accessToken")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            refresh_token: auth
                .get("refreshToken")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            expires_at: expires,
            credits,
            credit_usage: total.zip(credits).map(|(t, c)| (t - c).max(0.0)),
            credit_expiry,
            enabled: !disabled,
            last_used_at: last_success,
            region: auth
                .get("realm")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            source: Some("portable".into()),
            notes: Some(
                st.get("last_checkin_day")
                    .and_then(|x| x.as_str())
                    .map(|d| format!("最近签到 {d}"))
                    .unwrap_or_default(),
            ),
        };

        if let Some(existing) = bundle
            .accounts
            .iter_mut()
            .find(|a| a.account_id == uid || a.id == acct.id)
        {
            *existing = acct;
        } else {
            bundle.accounts.push(acct);
        }
    }
}

#[tauri::command]
pub fn get_accounts(state: tauri::State<'_, AppState>) -> Result<ExportBundle, String> {
    let s = state.store.lock().map_err(|e| e.to_string())?;
    Ok(s.data.clone())
}

#[tauri::command]
pub fn switch_accounts(state: tauri::State<'_, AppState>) -> Result<Vec<Value>, String> {
    let s = state.store.lock().map_err(|e| e.to_string())?;
    Ok(s.switch_accounts())
}

#[tauri::command]
pub fn save_account(
    state: tauri::State<'_, AppState>,
    account: Account,
) -> Result<ExportBundle, String> {
    let mut s = state.store.lock().map_err(|e| e.to_string())?;
    if account.source.as_deref() == Some("portable") {
        // 真实账号：只允许切换启用状态，写回 data/state.json
        write_disabled(&s.portable, account.account_id.as_str(), !account.enabled)?;
    }
    if let Some(existing) = s.data.accounts.iter_mut().find(|a| a.id == account.id) {
        *existing = account;
    } else {
        s.data.accounts.push(account);
    }
    s.save()?;
    Ok(s.data.clone())
}

fn write_disabled(root: &std::path::Path, uid: &str, disabled: bool) -> Result<(), String> {
    let path = root.join("data").join("state.json");
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(acc) = v
        .get_mut("accounts")
        .and_then(|a| a.get_mut(uid))
        .and_then(|a| a.as_object_mut())
    {
        acc.insert("disabled".into(), Value::Bool(disabled));
        let _ = fs::copy(&path, path.with_extension("json.axiao-bak"));
        fs::write(&path, serde_json::to_string_pretty(&v).unwrap())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn delete_account(state: tauri::State<'_, AppState>, id: String) -> Result<ExportBundle, String> {
    let mut s = state.store.lock().map_err(|e| e.to_string())?;
    if let Some(a) = s.data.accounts.iter().find(|a| a.id == id) {
        if a.source.as_deref() == Some("portable") {
            return Err("真实账号不能在此删除，请在便携版面板移除".into());
        }
    }
    s.data.accounts.retain(|a| a.id != id);
    s.save()?;
    Ok(s.data.clone())
}

#[tauri::command]
pub fn import_accounts(
    state: tauri::State<'_, AppState>,
    bundle: ExportBundle,
) -> Result<ImportResult, String> {
    if bundle.schema_version != "1.0.0" {
        return Err("schemaVersion 必须为 1.0.0".into());
    }
    let names: std::collections::HashSet<String> =
        bundle.client_rules.iter().map(|r| r.name.clone()).collect();
    for (i, a) in bundle.accounts.iter().enumerate() {
        if !names.contains(&a.client) {
            return Err(format!("accounts[{i}].client 引用了不存在的规则 {}", a.client));
        }
    }
    let mut s = state.store.lock().map_err(|e| e.to_string())?;
    let mut added = 0usize;
    let mut updated = 0usize;
    for a in bundle.accounts {
        if let Some(existing) = s.data.accounts.iter_mut().find(|x| x.id == a.id) {
            *existing = a;
            updated += 1;
        } else {
            s.data.accounts.push(a);
            added += 1;
        }
    }
    let local: std::collections::HashSet<String> =
        s.data.client_rules.iter().map(|r| r.name.clone()).collect();
    for r in bundle.client_rules {
        if !local.contains(&r.name) {
            s.data.client_rules.push(r);
        }
    }
    s.save()?;
    Ok(ImportResult {
        added,
        updated,
        total: s.data.accounts.len(),
    })
}

#[tauri::command]
pub fn export_accounts(
    state: tauri::State<'_, AppState>,
    include_tokens: bool,
) -> Result<ExportBundle, String> {
    let s = state.store.lock().map_err(|e| e.to_string())?;
    let mut out = s.data.clone();
    out.exported_at = Some(chrono::Utc::now().to_rfc3339());
    if !include_tokens {
        for a in out.accounts.iter_mut() {
            a.access_token = None;
            a.refresh_token = None;
        }
    }
    Ok(out)
}
