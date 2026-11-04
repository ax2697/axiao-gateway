use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::store::home;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub client: String,
    pub model: String,
    pub cwd: String,
    pub transcript_path: String,
    pub last_event: String,
    pub last_at: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DayBucket {
    pub date: String,
    pub input: u64,
    pub output: u64,
    pub cached: u64,
    pub total: u64,
    pub calls: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cached: u64,
    pub calls: u64,
    pub hit_rate: f64,
    pub normalized: u64,
    /// 本次窗口内实际有数据的天数，以及最早/最晚日期
    pub covered_days: u64,
    pub first_day: String,
    pub last_day: String,
    pub days: Vec<DayBucket>,
}

pub fn wb_switch_dir() -> PathBuf {
    std::env::var("WB_SWITCH_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".wb-switch"))
}

fn read_json(path: PathBuf) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// 关联会话：按 hook-events 的 session_id 聚合，取最后一次事件
#[tauri::command]
pub fn switch_sessions() -> Result<Vec<SessionInfo>, String> {
    let path = wb_switch_dir().join("hook-events.jsonl");
    let file = File::open(&path).map_err(|e| format!("未找到 hook-events.jsonl：{e}"))?;
    let mut map: HashMap<String, SessionInfo> = HashMap::new();

    for line in BufReader::new(file).lines().flatten() {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let sid = v.get("session_id").and_then(|x| x.as_str()).unwrap_or("");
        if sid.is_empty() {
            continue;
        }
        let info = SessionInfo {
            session_id: sid.to_string(),
            client: v.get("client").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            model: v.get("model").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            cwd: v.get("cwd").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            transcript_path: v
                .get("transcript_path")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            last_event: v
                .get("hook_event_name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            last_at: v
                .get("timestamp")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        };
        map.insert(sid.to_string(), info);
    }

    let mut out: Vec<SessionInfo> = map.into_values().collect();
    out.sort_by(|a, b| b.last_at.cmp(&a.last_at));
    Ok(out)
}

/// Token 统计：解析 transcript 的 /message/usage，按天聚合
#[tauri::command]
pub fn switch_token_stats(days: i64) -> Result<TokenStats, String> {
    let sessions = switch_sessions()?;
    // 自然日窗口：今日 = 今天 00:00 起；N 天 = 含今天共 N 天。
    // 原来用 now-24h，选「今日」会把昨天也算进来。
    let today = chrono::Local::now().date_naive();
    let start = today - chrono::Duration::days((days.max(1) - 1) as i64);
    let mut paths: Vec<String> = sessions
        .iter()
        .map(|s| s.transcript_path.clone())
        .filter(|p| !p.is_empty())
        .collect();
    paths.sort();
    paths.dedup();

    let mut stats = TokenStats::default();
    let mut buckets: BTreeMap<String, DayBucket> = BTreeMap::new();
    // 记录被修正的条数（total_tokens < input+output），便于排查上游口径
    let mut fixed: u64 = 0;

    for path in paths {
        let Ok(file) = File::open(&path) else {
            continue;
        };
        for line in BufReader::new(file).lines().flatten() {
            if !line.contains("\"usage\"") {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(u) = v.pointer("/message/usage") else {
                continue;
            };
            let input = u.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
            let output = u.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
            let cached = u
                .get("cache_read_input_tokens")
                .and_then(|x| x.as_u64())
                .unwrap_or(0);
            // 口径统一：**总 Token = 输入 + 输出**。
            // 上游的 total_tokens 经常和 input+output 对不上（有的不含缓存、有的是估算），
            // 直接采信会出现「总比输入还小」这种不可能的行。取两者较大值兜底。
            let api_total = u.get("total_tokens").and_then(|x| x.as_u64());
            let sum = input.saturating_add(output);
            let total = match api_total {
                Some(t) if t >= sum => t,
                _ => sum,
            };
            if api_total.map(|t| t < sum).unwrap_or(false) {
                fixed += 1;
            }

            // timestamp 有两种形态：
            //   · ISO 字符串 "2026-11-03T12:34:56Z"（早期 CLI 写的）
            //   · **Unix 秒数字** 1793704318（现在 WorkBuddy/CodeBuddy 的 transcript 就是这种）
            // 只按字符串取前 10 位会把数字当成 "1793704318" 再解析失败，然后 continue，
            // 结果整份 transcript 的 usage 全被丢掉 —— 表现就是「明明是实时读的却全是 0」。
            let date = match v.get("timestamp") {
                Some(Value::String(s)) if s.len() >= 10 => s.get(0..10).unwrap_or("").to_string(),
                Some(Value::Number(n)) => n
                    .as_i64()
                    .and_then(|secs| {
                        // 容错：毫秒级时间戳
                        let secs = if secs > 100_000_000_000 { secs / 1000 } else { secs };
                        chrono::DateTime::from_timestamp(secs, 0)
                    })
                    .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            if date.is_empty() {
                continue;
            }
            if let Ok(d) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                if d < start || d > today {
                    continue;
                }
            }

            stats.input += input;
            stats.output += output;
            stats.cached += cached;
            stats.total += total;
            stats.calls += 1;

            let b = buckets.entry(date.clone()).or_insert_with(|| DayBucket {
                date,
                ..Default::default()
            });
            b.input += input;
            b.output += output;
            b.cached += cached;
            b.total += total;
            b.calls += 1;
        }
    }

    // switch 口径：缓存命中率 = 缓存读 / 输入 Token（input 已含缓存读）
    let denom = stats.input;
    stats.hit_rate = if denom > 0 {
        (stats.cached as f64 / denom as f64) * 100.0
    } else {
        0.0
    };
    stats.normalized = fixed;
    stats.covered_days = buckets.len() as u64;
    if let (Some(f), Some(l)) = (buckets.keys().next(), buckets.keys().next_back()) {
        stats.first_day = f.clone();
        stats.last_day = l.clone();
    }
    stats.days = buckets.into_values().collect();
    Ok(stats)
}

/// 积分统计：官方用量缓存
#[tauri::command]
pub fn switch_usage_cache() -> Result<Value, String> {
    Ok(read_json(wb_switch_dir().join("official_usage_cache.json")))
}

/// 设置：工具路径、限流状态、设置文件
#[tauri::command]
pub fn switch_settings() -> Result<Value, String> {
    Ok(serde_json::json!({
        "dir": wb_switch_dir().to_string_lossy(),
        "workbuddyExe": read_json(wb_switch_dir().join("workbuddy_exe.json")),
        "rateLimit": read_json(wb_switch_dir().join("rate_limit_state.json")),
        "settings": read_json(wb_switch_dir().join("settings.json")),
    }))
}

/* ---------- 非重复能力：客户端状态 / 自动设置 / 会话组 / 积分到期 ---------- */

fn axiao_dir() -> PathBuf {
    wb_switch_dir().join("axiao")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub path: String,
    pub detail: String,
}

fn exists_any(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

fn scan_contains(dir: &std::path::Path, needle: &str) -> Option<PathBuf> {
    let rd = fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        if name.contains(needle) {
            return Some(e.path());
        }
    }
    None
}

/// 客户端状态：WorkBuddy / CodeBuddy IDE CN·Intl / CLI / VS Code / JetBrains
#[tauri::command]
pub fn switch_tool_status() -> Result<Vec<ToolStatus>, String> {
    let local = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("AppData").join("Local"));
    let roaming = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("AppData").join("Roaming"));

    let wb_json = read_json(wb_switch_dir().join("workbuddy_exe.json"));
    let wb_cn = wb_json
        .get("cn")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let mut out = Vec::new();

    let wb = wb_cn
        .filter(|p| p.exists())
        .or_else(|| {
            exists_any(&[
                local.join("Programs").join("WorkBuddy").join("WorkBuddy.exe"),
                local.join("Programs").join("WorkBuddy CN").join("WorkBuddy.exe"),
            ])
        });
    out.push(ToolStatus {
        id: "workbuddy".into(),
        name: "WorkBuddy 桌面端".into(),
        installed: wb.is_some(),
        path: wb.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "账号管理 / 会话复制".into(),
    });

    let cb_ide = exists_any(&[
        local.join("Programs").join("CodeBuddy").join("CodeBuddy.exe"),
        local.join("Programs").join("CodeBuddy CN").join("CodeBuddy.exe"),
        local.join("CodeBuddy").join("CodeBuddy.exe"),
    ]);
    out.push(ToolStatus {
        id: "codebuddy_ide".into(),
        name: "CodeBuddy IDE".into(),
        installed: cb_ide.is_some(),
        path: cb_ide.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "国内版 / 国际版".into(),
    });

    let cli_cfg = exists_any(&[
        roaming.join("CodeBuddy").join("settings.json"),
        home().join(".codebuddy").join("settings.json"),
        home().join(".config").join("codebuddy").join("settings.json"),
    ]);
    out.push(ToolStatus {
        id: "codebuddy_cli".into(),
        name: "CodeBuddy CLI".into(),
        installed: cli_cfg.is_some(),
        path: cli_cfg.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "settings.json 认证 Token".into(),
    });

    let vscode_ext = scan_contains(&home().join(".vscode").join("extensions"), "codebuddy");
    out.push(ToolStatus {
        id: "vscode_ext".into(),
        name: "VS Code CodeBuddy 插件".into(),
        installed: vscode_ext.is_some(),
        path: vscode_ext.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "插件账号切换 / 会话复制".into(),
    });

    let jb = scan_contains(&roaming.join("JetBrains"), "codebuddy");
    out.push(ToolStatus {
        id: "jetbrains".into(),
        name: "JetBrains IDE 插件".into(),
        installed: jb.is_some(),
        path: jb.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "IDEA / PyCharm 插件".into(),
    });

    let trae = exists_any(&[
        local.join("Programs").join("Trae").join("Trae.exe"),
        local.join("Programs").join("Trae CN").join("Trae.exe"),
        roaming.join("Trae"),
    ]);
    out.push(ToolStatus {
        id: "trae".into(),
        name: "Trae".into(),
        installed: trae.is_some(),
        path: trae.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "CN / 国际版 · OpenAI 兼容端点".into(),
    });

    let qoder = exists_any(&[
        local.join("Programs").join("Qoder").join("Qoder.exe"),
        home().join(".qoder"),
    ]);
    out.push(ToolStatus {
        id: "qoder".into(),
        name: "Qoder".into(),
        installed: qoder.is_some(),
        path: qoder.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "国内 / 国际版".into(),
    });

    let zcode = exists_any(&[home().join(".zcode").join("cli").join("config.json")]);
    out.push(ToolStatus {
        id: "zcode".into(),
        name: "ZCode / Z.AI".into(),
        installed: zcode.is_some(),
        path: zcode.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "coding-plan · off-peak".into(),
    });

    let dsh = exists_any(&[home().join(".dsh")]);
    out.push(ToolStatus {
        id: "dsh".into(),
        name: "DeepSeek Harness (DSH)".into(),
        installed: dsh.is_some(),
        path: dsh.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "插件 / OpenAI 兼容".into(),
    });

    let codearts = exists_any(&[
        local.join("Programs").join("CodeArts").join("CodeArts.exe"),
        local.join("Programs").join("Huawei CodeArts").join("CodeArts.exe"),
    ]);
    out.push(ToolStatus {
        id: "codearts".into(),
        name: "华为云 CodeArts".into(),
        installed: codearts.is_some(),
        path: codearts.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "云端编程智能体".into(),
    });

    // OpenCode CLI（本机可检测）
    let opencode_cli = exists_any(&[
        home().join(".opencode").join("config.json"),
        home().join(".opencode"),
        home().join(".config").join("opencode"),
        roaming.join("opencode"),
    ]);
    out.push(ToolStatus {
        id: "opencode_cli".into(),
        name: "OpenCode CLI".into(),
        installed: opencode_cli.is_some(),
        path: opencode_cli.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        detail: "本地 CLI · 可接 Zen / 自定义上游".into(),
    });

    // 纯云端平台：无本地可执行文件，标注为云端
    for (id, name, detail) in [
        ("minimax", "MiniMax", "云端 · 需在网关添加上游"),
        ("doubao", "豆包", "云端 · 需在网关添加上游"),
        ("monkeycode", "MonkeyCode", "云端 · 需在网关添加上游"),
        ("opencode_zen", "OpenCode Zen", "云端免费额度"),
        ("kimi", "Kimi（Moonshot）", "云端 · 有免费额度"),
        ("bigmodel", "智谱 BigModel", "云端 · GLM / 有免费额度"),
        ("commandcode", "CommandCode", "云端 · 有免费额度"),
        ("baichuan", "百川", "官方 API · 需在网关添加上游"),
        ("lingyi", "零一万物", "官方 API · 需在网关添加上游"),
        ("spark", "讯飞星火", "官方 API · 需在网关添加上游"),
        ("hunyuan", "混元", "官方 API · 需在网关添加上游"),
        ("siliconflow", "硅基流动", "官方 API · 聚合多模型"),
    ] {
        out.push(ToolStatus {
            id: id.into(),
            name: name.into(),
            installed: false,
            path: String::new(),
            detail: detail.into(),
        });
    }

    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutoSettings {
    pub auto_checkin: bool,
    pub auto_travel: bool,
    pub auto_rotate: bool,
    pub checkin_hours: String,
    pub rotate_minutes: u32,
    pub exclude_accounts: Vec<String>,
    pub launch_at_login: bool,
    pub companion: bool,
}

fn auto_file() -> PathBuf {
    axiao_dir().join("switch-settings.json")
}

#[tauri::command]
pub fn switch_get_auto() -> Result<AutoSettings, String> {
    if let Ok(s) = fs::read_to_string(auto_file()) {
        if let Ok(v) = serde_json::from_str::<AutoSettings>(&s) {
            return Ok(v);
        }
    }
    Ok(AutoSettings {
        auto_checkin: false,
        auto_travel: false,
        auto_rotate: false,
        checkin_hours: "9,21".into(),
        rotate_minutes: 30,
        exclude_accounts: Vec::new(),
        launch_at_login: false,
        companion: true,
    })
}

#[tauri::command]
pub fn switch_save_auto(settings: AutoSettings) -> Result<AutoSettings, String> {
    let _ = fs::create_dir_all(axiao_dir());
    fs::write(
        auto_file(),
        serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(settings)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionGroup {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
    pub created_at: String,
}

fn groups_file() -> PathBuf {
    axiao_dir().join("session-groups.json")
}

fn load_groups() -> Vec<SessionGroup> {
    fs::read_to_string(groups_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn switch_session_groups() -> Result<Vec<SessionGroup>, String> {
    Ok(load_groups())
}

#[tauri::command]
pub fn switch_create_group(name: String, members: Vec<String>) -> Result<SessionGroup, String> {
    let mut groups = load_groups();
    let g = SessionGroup {
        id: format!("grp_{}", chrono::Utc::now().timestamp_millis()),
        name,
        members,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    groups.push(g.clone());
    let _ = fs::create_dir_all(axiao_dir());
    fs::write(
        groups_file(),
        serde_json::to_string_pretty(&groups).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(g)
}

#[tauri::command]
pub fn switch_delete_group(id: String) -> Result<(), String> {
    let mut groups = load_groups();
    groups.retain(|g| g.id != id);
    let _ = fs::create_dir_all(axiao_dir());
    fs::write(
        groups_file(),
        serde_json::to_string_pretty(&groups).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 积分到期：读便携版 data/state.json
#[tauri::command]
pub fn switch_credit_expiry() -> Result<Value, String> {
    let state = read_json(crate::store::portable_root().join("data").join("state.json"));
    Ok(state.get("accounts").cloned().unwrap_or_else(|| serde_json::json!({})))
}
