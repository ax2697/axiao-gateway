use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::model::{Account, ComponentStatus};
use crate::state::AppState;
use crate::store::home;

const BASE: &str = "https://copilot.tencent.com";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36";

fn gen_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", n, std::process::id())
}

fn account_of(state: &AppState, uid: &str) -> Result<Account, String> {
    let s = state.store.lock().map_err(|e| e.to_string())?;
    s.data
        .accounts
        .iter()
        .find(|a| a.account_id == uid || a.id == uid)
        .cloned()
        .ok_or_else(|| format!("账号不存在：{uid}"))
}

async fn call(
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> Result<(u16, Value), String> {
    let url = format!("{BASE}{path}");
    let client = reqwest::Client::new();
    let mut rb = match method {
        "POST" => client.post(&url),
        _ => client.get(&url),
    }
    .bearer_auth(token)
    .header("accept", "application/json")
    .header("content-type", "application/json")
    .header("user-agent", UA)
    .timeout(Duration::from_secs(25));
    if let Some(b) = body {
        rb = rb.json(&b);
    } else if method == "POST" {
        rb = rb.json(&json!({}));
    }
    let resp = rb.send().await.map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    Ok((status, v))
}

/// 原生签到：活动状态 → 领取
#[tauri::command]
pub async fn native_checkin(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let acc = account_of(&state, &uid)?;
    let token = acc.access_token.clone().filter(|t| !t.is_empty()).ok_or("缺少 accessToken")?;
    let (s1, st) =
        call("POST", "/v2/billing/meter/checkin-activity-status", &token, Some(json!({}))).await?;
    let (s2, cl) =
        call("POST", "/v2/billing/meter/daily-checkin", &token, Some(json!({}))).await?;
    Ok(json!({ "statusHttp": s1, "status": st, "claimHttp": s2, "claim": cl }))
}

/// 原生成长任务：批量接受 → 领奖 → 旅行 → 抽奖
#[tauri::command]
pub async fn native_growth(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let acc = account_of(&state, &uid)?;
    let token = acc.access_token.clone().filter(|t| !t.is_empty()).ok_or("缺少 accessToken")?;
    let mut log: Vec<String> = Vec::new();

    let (_, tasks) = call("GET", "/v2/activity/growth/tasks", &token, None).await?;
    let rows = tasks
        .get("data")
        .and_then(|d| d.get("tasks"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    log.push(format!("读取任务 {} 条", rows.len()));

    let todo: Vec<String> = rows
        .iter()
        .filter(|t| t.get("accept_status").and_then(|v| v.as_str()) == Some("not_accepted"))
        .filter_map(|t| t.get("task_code").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    if !todo.is_empty() {
        let (_, r) =
            call("POST", "/v2/activity/growth/tasks/accept", &token, Some(json!({"task_codes": todo}))).await?;
        log.push(format!("接受任务 {} 项：{}", todo.len(), r.get("msg").and_then(|v| v.as_str()).unwrap_or("ok")));
    }

    // 领取已完成任务（web 域）
    let mut claimed = 0;
    for t in &rows {
        let code = t.get("task_code").and_then(|v| v.as_str()).unwrap_or("");
        let cur = t.get("progress").and_then(|p| p.get("current")).and_then(|v| v.as_i64()).unwrap_or(0);
        let tar = t.get("progress").and_then(|p| p.get("target")).and_then(|v| v.as_i64()).unwrap_or(0);
        let accepted = t.get("accept_status").and_then(|v| v.as_str()).unwrap_or("");
        if code.is_empty() || tar <= 0 || cur < tar || accepted == "claimed" {
            continue;
        }
        let path = format!("/activity/growth/tasks/{code}/claim");
        if let Ok((s, _)) = call("POST", &path, &token, Some(json!({}))).await {
            if (200..300).contains(&s) {
                claimed += 1;
            }
        }
    }
    log.push(format!("领取完成奖励 {} 项", claimed));

    // 旅行
    let (_, st) = call("GET", "/v2/activity/growth/buddy/travel/status", &token, None).await?;
    let state_str = st.get("data").and_then(|d| d.get("state")).and_then(|v| v.as_str()).unwrap_or("idle");
    if state_str == "arrived" {
        if let Some(rid) = st.get("data").and_then(|d| d.get("record_id")).and_then(|v| v.as_i64()) {
            let (_, r) = call("POST", "/v2/activity/growth/buddy/travel/claim", &token, Some(json!({"record_id": rid}))).await?;
            log.push(format!("旅行礼物：{}", r.get("msg").and_then(|v| v.as_str()).unwrap_or("已领取")));
        }
    } else {
        let (_, cfg) = call("GET", "/v2/activity/growth/buddy/travel/config", &token, None).await?;
        if let Some(loc) = cfg.get("data").and_then(|d| d.get("locations")).and_then(|l| l.get(0)).and_then(|l| l.get("id")).and_then(|v| v.as_i64()) {
            let _ = call("POST", "/v2/activity/growth/buddy/travel/depart", &token, Some(json!({"location_id": loc}))).await;
            log.push("已派出 Buddy 旅行".into());
        }
    }

    // 抽奖
    let (_, ch) = call("GET", "/v2/activity/growth/lottery/chances", &token, None).await?;
    let chances = ch
        .get("data")
        .and_then(|d| d.get("balance").or_else(|| d.get("chances")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .min(10);
    let mut won = 0;
    for _ in 0..chances {
        let token_body = format!("draw-{}", gen_id());
        if let Ok((_, r)) = call("POST", "/v2/activity/growth/lottery/draw", &token, Some(json!({"client_token": token_body}))).await {
            if r.get("code").and_then(|v| v.as_i64()) == Some(0) {
                won += 1;
            }
        }
    }
    if chances > 0 {
        log.push(format!("抽奖 {chances} 次，中奖 {won} 次"));
    }

    Ok(json!({ "log": log }))
}

fn token_of(state: &AppState, uid: &str) -> Result<String, String> {
    let acc = account_of(state, uid)?;
    acc.access_token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "账号缺少 accessToken，请先重新授权".to_string())
}

fn upstream_err(status: u16, v: &Value) -> String {
    let msg = v
        .get("msg")
        .or_else(|| v.get("message"))
        .and_then(|x| x.as_str())
        .unwrap_or("上游返回异常");
    format!("上游 {status}：{msg}")
}

/// 积分任务列表（对应面板「积分任务」弹窗）：只读，不改状态
#[tauri::command]
pub async fn growth_tasks(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let token = token_of(&state, &uid)?;
    let (status, v) = call("GET", "/v2/activity/growth/tasks", &token, None).await?;
    if !(200..300).contains(&status) {
        return Err(upstream_err(status, &v));
    }
    let tasks = v
        .get("data")
        .and_then(|d| d.get("tasks"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let streak = call("GET", "/v2/activity/growth/streak", &token, None)
        .await
        .ok()
        .and_then(|(_, s)| s.get("data").cloned())
        .unwrap_or_else(|| json!({}));
    Ok(json!({ "tasks": tasks, "streak": streak }))
}

/// 接受任务（面板上的「全部接受」）
#[tauri::command]
pub async fn growth_accept(
    state: tauri::State<'_, AppState>,
    uid: String,
    codes: Vec<String>,
) -> Result<Value, String> {
    if codes.is_empty() {
        return Ok(json!({ "count": 0, "msg": "没有待接受的任务" }));
    }
    let token = token_of(&state, &uid)?;
    let (status, v) = call(
        "POST",
        "/v2/activity/growth/tasks/accept",
        &token,
        Some(json!({ "task_codes": codes })),
    )
    .await?;
    if !(200..300).contains(&status) {
        return Err(upstream_err(status, &v));
    }
    Ok(json!({ "count": codes.len(), "msg": v.get("msg").and_then(|x| x.as_str()).unwrap_or("已接受") }))
}

/// 领取单个任务的奖励
#[tauri::command]
pub async fn growth_claim(
    state: tauri::State<'_, AppState>,
    uid: String,
    code: String,
) -> Result<Value, String> {
    if code.trim().is_empty() {
        return Err("缺少 task_code".into());
    }
    let token = token_of(&state, &uid)?;
    let path = format!("/activity/growth/tasks/{code}/claim");
    let (status, v) = call("POST", &path, &token, Some(json!({}))).await?;
    if !(200..300).contains(&status) {
        return Err(upstream_err(status, &v));
    }
    Ok(json!({ "code": code, "resp": v }))
}

/// 活跃保活：上报 chat_request_send
#[tauri::command]
pub async fn native_keepalive(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let acc = account_of(&state, &uid)?;
    let token = acc.access_token.clone().filter(|t| !t.is_empty()).ok_or("缺少 accessToken")?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let event = json!({
        "eventCode": "chat_request_send",
        "timestamp": now,
        "reportDelay": 0,
        "mode": "craft",
        "conversationId": gen_id(),
        "requestId": gen_id(),
        "inputLength": 12,
        "requestModelId": "deepseek-v4-flash",
        "requestModelName": "DeepSeek V4 Flash",
        "isPlan": false,
        "isAutoExecuteTerminal": false,
        "isAutoModify": false,
        "codebaseEnable": false,
        "maxToken": 0,
        "maxSteps": 0,
        "temperature": 0,
        "maxRetries": 0,
        "mentionContexts": [],
        "knowledgeId": [],
        "knowledgeName": [],
        "codebaseId": "",
        "mentionContextCount": 0,
    });
    let (s, r) = call("POST", "/v2/report", &token, Some(event)).await?;
    Ok(json!({ "http": s, "report": r }))
}

/// 刷新 Token
#[tauri::command]
pub async fn native_refresh(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let acc = account_of(&state, &uid)?;
    let token = acc.access_token.clone().filter(|t| !t.is_empty()).ok_or("缺少 accessToken")?;
    let (s, r) = call("POST", "/v2/plugin/auth/token/refresh", &token, Some(json!({}))).await?;
    Ok(json!({ "http": s, "result": r }))
}

/// 签到状态（只读）
#[tauri::command]
pub async fn native_status(state: tauri::State<'_, AppState>, uid: String) -> Result<Value, String> {
    let acc = account_of(&state, &uid)?;
    let token = acc.access_token.clone().filter(|t| !t.is_empty()).ok_or("缺少 accessToken")?;
    let (s1, st) =
        call("POST", "/v2/billing/meter/checkin-activity-status", &token, Some(json!({}))).await?;
    let (_, streak) = call("GET", "/v2/activity/growth/streak", &token, None).await?;
    Ok(json!({ "http": s1, "checkin": st, "streak": streak }))
}

/* ---------- 组件状态（保留 exe 作为备用，不是主路径） ---------- */

pub fn signin_path() -> PathBuf {
    std::env::var("WB_SIGNIN_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("Desktop").join("WorkBuddy签到助手.exe"))
}

pub fn switch_path() -> PathBuf {
    std::env::var("WB_SWITCH_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            home()
                .join("Desktop")
                .join("WorkBuddy网关-便携版")
                .join("app")
                .join("vendor")
                .join("wb-switch")
                .join("wb-switch-rust.exe")
        })
}

/// 探测本机端口是否在监听（组件是服务型时用它判断"就绪"）
fn port_open(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(300),
    )
    .is_ok()
}

#[tauri::command]
pub fn component_status() -> Result<Vec<ComponentStatus>, String> {
    let signin = signin_path();
    let switch = switch_path();
    Ok(vec![
        ComponentStatus {
            id: "panel".into(),
            name: "workbuddy2api-panel".into(),
            role: "网关内核 · 账号池/任务/模型/配置/日志".into(),
            path: "http://127.0.0.1:7863/panel/".into(),
            exists: true,
            detail: "由本应用实时抓取".into(),
            builtin: false,
        },
        ComponentStatus {
            id: "switch".into(),
            name: "workbuddy-switch".into(),
            role: "多账号切换 · 积分监控".into(),
            path: switch.to_string_lossy().to_string(),
            exists: switch.exists(),
            detail: if switch.exists() {
                "外部 exe 存在；本应用已原生实现同样能力".into()
            } else {
                "能力已内置（读 auths/state.json、写授权文件），不需要这个 exe".into()
            },
            builtin: true,
        },
        ComponentStatus {
            id: "signin".into(),
            name: "WorkBuddy签到助手".into(),
            role: "签到 · 成长任务".into(),
            path: signin.to_string_lossy().to_string(),
            exists: signin.exists(),
            detail: if signin.exists() {
                "外部 exe 存在；本应用已原生实现同样能力".into()
            } else {
                "能力已内置（直连上游签到接口），不需要这个 exe".into()
            },
            builtin: true,
        },
        ComponentStatus {
            id: "media".into(),
            name: "本地创作 API".into(),
            role: "图像 / 视频生成 · OpenAI 兼容".into(),
            path: "http://127.0.0.1:7864/v1".into(),
            exists: crate::media_server::is_running(),
            detail: "随本应用启动，任何客户端填这个地址即可调用".into(),
            builtin: true,
        },
        ComponentStatus {
            id: "dsh".into(),
            name: "DSH 内核".into(),
            role: "可选 · DeepSeek Harness".into(),
            path: "http://127.0.0.1:8317/v1".into(),
            exists: port_open(8317),
            detail: "可选组件，装了才通；没装不影响其它功能".into(),
            builtin: false,
        },
        ComponentStatus {
            id: "freellmapi".into(),
            name: "FreeLLMAPI".into(),
            role: "可选 · 34 家免费模型聚合".into(),
            path: "http://127.0.0.1:3001/v1".into(),
            exists: port_open(3001),
            detail: "可选组件：装了就有 600+ 免费端点，没装不影响其它功能".into(),
            builtin: false,
        },
        ComponentStatus {
            id: "doubao_bridge".into(),
            name: "豆包桥".into(),
            role: "可选 · 把豆包账号变 API".into(),
            path: "http://127.0.0.1:9090/v1".into(),
            exists: port_open(9090),
            detail: "可选组件，需自己启动；在「其他平台」里选「豆包桥（本地）」接入".into(),
            builtin: false,
        },
    ])
}

/// 备用：启动 exe（非主路径）
#[tauri::command]
pub fn launch_signin(confirm: bool) -> Result<String, String> {
    if !confirm {
        return Err("needConfirm".into());
    }
    let p = signin_path();
    if !p.exists() {
        return Err(format!("签到助手不存在：{}", p.to_string_lossy()));
    }
    Command::new(&p).spawn().map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn launch_component(id: String, confirm: bool) -> Result<String, String> {
    if !confirm {
        return Err("needConfirm".into());
    }
    let p = match id.as_str() {
        "signin" => signin_path(),
        "switch" => switch_path(),
        _ => return Err("不支持启动该组件".into()),
    };
    if !p.exists() {
        return Err(format!("组件不存在：{}", p.to_string_lossy()));
    }
    Command::new(&p).spawn().map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().to_string())
}
