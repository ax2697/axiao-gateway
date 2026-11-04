use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::store::portable_root;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaCard {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub remaining: Option<f64>,
    pub total: Option<f64>,
    pub detail: String,
    pub status: String,
    pub source: String,
}

fn read_json(path: std::path::PathBuf) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

fn workbuddy_totals() -> (f64, f64) {
    let state = read_json(portable_root().join("data").join("state.json"));
    let mut remain = 0.0;
    let mut total = 0.0;
    if let Some(m) = state.get("accounts").and_then(|v| v.as_object()) {
        for v in m.values() {
            remain += v.get("credits").and_then(|x| x.as_f64()).unwrap_or(0.0);
            total += v.get("credits_total").and_then(|x| x.as_f64()).unwrap_or(0.0);
        }
    }
    (remain, total)
}


/// 读上游列表（配额卡片要按实际配置补全，不能只列死 8 个）
fn upstreams() -> Vec<serde_json::Value> {
    let dir = crate::store::data_dir();
    std::fs::read_to_string(dir.join("upstreams.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<serde_json::Value>>(&t).ok())
        .unwrap_or_default()
}

fn host_of(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

#[tauri::command]
pub fn quota_cards() -> Result<Vec<QuotaCard>, String> {
    let (remain, total) = workbuddy_totals();
    let mut cards = vec![
        QuotaCard {
            id: "workbuddy".into(),
            name: "WorkBuddy / CodeBuddy".into(),
            kind: "签到 + 成长任务".into(),
            remaining: Some(remain),
            total: Some(total),
            detail: "每日签到、22 项成长任务、Buddy 旅行".into(),
            status: if total > 0.0 { "已接入".into() } else { "未读取到账号".into() },
            source: "data/state.json".into(),
        },
        QuotaCard {
            id: "trae".into(),
            name: "Trae CN".into(),
            kind: "每日签到".into(),
            remaining: None,
            total: None,
            detail: "checkin_credits/status + claim（需插件登录态）".into(),
            status: "需 Trae 登录态".into(),
            source: "api.trae.cn".into(),
        },
        QuotaCard {
            id: "opencode_zen".into(),
            name: "OpenCode Zen".into(),
            kind: "免费模型".into(),
            remaining: None,
            total: None,
            detail: "匿名免费额度，可直接探测".into(),
            status: "可探测".into(),
            source: "opencode.ai/zen/go/v1/usage".into(),
        },
        QuotaCard {
            id: "zai".into(),
            name: "Z.AI / Bigmodel".into(),
            kind: "coding-plan".into(),
            remaining: None,
            total: None,
            detail: "quota/limit 订阅窗口".into(),
            status: "需 ZAI_API_KEY".into(),
            source: "api.z.ai".into(),
        },
        QuotaCard {
            id: "bigmodel".into(),
            name: "BigModel".into(),
            kind: "coding-plan".into(),
            remaining: None,
            total: None,
            detail: "quota/limit 订阅窗口".into(),
            status: "需 BIGMODEL_API_KEY".into(),
            source: "open.bigmodel.cn".into(),
        },
        QuotaCard {
            id: "kimi".into(),
            name: "Kimi Code".into(),
            kind: "订阅".into(),
            remaining: None,
            total: None,
            detail: "coding/usages 窗口".into(),
            status: "需 Kimi Key".into(),
            source: "api.kimi.com".into(),
        },
        QuotaCard {
            id: "minimax".into(),
            name: "MiniMax".into(),
            kind: "订阅".into(),
            remaining: None,
            total: None,
            detail: "Coding Plan 5h / 周窗口".into(),
            status: "需 MiniMax Key".into(),
            source: "api.minimaxi.com".into(),
        },
        QuotaCard {
            id: "commandcode".into(),
            name: "CommandCode".into(),
            kind: "订阅".into(),
            remaining: None,
            total: None,
            detail: "/alpha/billing/credits".into(),
            status: "需 CommandCode Key".into(),
            source: "api.commandcode.ai".into(),
        },
    ];

    // 预设平台全量展示：不能只列 8 个，用户会以为不支持。
    // has_api = 该平台有公开余额/额度接口（能「抓取」）；否则只能去控制台看。
    let more: [(&str, &str, &str, &str, bool); 14] = [
        ("deepseek", "DeepSeek", "按量计费", "api.deepseek.com/user/balance", true),
        ("siliconflow", "硅基流动", "赠送额度", "api.siliconflow.cn/v1/user/info", true),
        ("openrouter", "OpenRouter", "credits", "openrouter.ai/api/v1/credits", true),
        ("trae_intl", "Trae Intl", "每日签到", "api.trae.ai（需插件登录态）", false),
        ("qoder", "Qoder", "订阅制", "qoder.com 控制台", false),
        ("codearts", "华为云 CodeArts", "按量计费", "华为云控制台", false),
        ("ark", "火山方舟（豆包）", "按 token 计费", "console.volcengine.com/ark", false),
        ("doubao_bridge", "豆包桥（本地）", "本地桥", "127.0.0.1:9090", false),
        ("spark", "讯飞星火", "赠送额度", "console.xfyun.cn", false),
        ("hunyuan", "混元", "按量计费", "console.cloud.tencent.com", false),
        ("baichuan", "百川", "赠送额度", "platform.baichuan-ai.com", false),
        ("lingyi", "零一万物", "赠送额度", "platform.lingyiwanwu.com", false),
        ("monkeycode", "MonkeyCode", "订阅制", "monkeycode-ai.com", false),
        ("dsh", "DSH", "本地内核", "127.0.0.1:8317", false),
    ];
    for (id, name, kind, source, has_api) in more {
        if cards.iter().any(|c| c.id == id || c.name == name) {
            continue;
        }
        cards.push(QuotaCard {
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            remaining: None,
            total: None,
            detail: if has_api {
                "有公开余额接口，配好 Key 后可点「抓取」".into()
            } else {
                "该平台未开放额度查询接口，余额请到控制台看".into()
            },
            status: if has_api { "需配 Key".into() } else { "未开放接口".into() },
            source: source.into(),
        });
    }

    // 补全：凡是「上游管理」里配过的平台，都在额度页出现一条，
    // 没有公开额度接口的就明说，而不是干脆不显示。
    let known = [
        "workbuddy", "trae", "opencode", "zai", "bigmodel", "kimi", "minimax", "commandcode",
    ];
    for u in upstreams() {
        let id = u.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let name = u.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        if id.is_empty() || name.is_empty() {
            continue;
        }
        let hay = format!("{id} {name}").to_lowercase();
        if known.iter().any(|k| hay.contains(k)) {
            continue;
        }
        if cards.iter().any(|c| c.id == id) {
            continue;
        }
        let has_key = u
            .get("apiKey")
            .and_then(|x| x.as_str())
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false);
        let base = u.get("baseUrl").and_then(|x| x.as_str()).unwrap_or("");
        let local = base.contains("127.0.0.1") || base.contains("localhost");
        cards.push(QuotaCard {
            id: id.clone(),
            name: name.clone(),
            kind: if local { "本地桥".into() } else { "上游渠道".into() },
            remaining: None,
            total: None,
            detail: if local {
                "本地服务，额度取决于它背后的账号".into()
            } else {
                "该平台未开放额度查询接口，余额请到控制台看".into()
            },
            status: if has_key || local {
                "未开放接口".into()
            } else {
                "需填 Key".into()
            },
            source: if base.is_empty() {
                "—".into()
            } else {
                host_of(base)
            },
        });
    }
    Ok(cards)
}


/// 按 id 或名字找到上游，拿它的 baseUrl / apiKey
fn find_upstream(id: &str) -> Option<(String, String)> {
    for u in upstreams() {
        let uid = u.get("id").and_then(|x| x.as_str()).unwrap_or("");
        let name = u.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if uid == id || name == id || name.to_lowercase().contains(&id.to_lowercase()) {
            let base = u.get("baseUrl").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let key = u
                .get("apiKey")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if !key.trim().is_empty() {
                return Some((base, key));
            }
        }
    }
    None
}

/// 有公开余额接口的平台：直接打到厂商端点，用上游里配的那把 Key
async fn balance_of(kind: &str, url: &str, key: &str) -> Result<Value, String> {
    let client = crate::httpc::client_for(url)?;
    let resp = client
        .get(url)
        .bearer_auth(key)
        .header("accept", "application/json")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if status >= 400 {
        return Err(format!("{kind} 返回 {status}: {v}"));
    }
    Ok(v)
}

#[tauri::command]
pub async fn quota_refresh(id: String) -> Result<Value, String> {
    match id.as_str() {
        "workbuddy" => {
            let (r, t) = workbuddy_totals();
            Ok(json!({"ok": true, "remaining": r, "total": t, "detail": "已从 data/state.json 重新读取"}))
        }
        "opencode_zen" => {
            let resp = reqwest::Client::new()
                .get("https://opencode.ai/zen/go/v1/usage")
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
            if status >= 400 {
                return Err(format!("Zen 返回 {status}: {v}"));
            }
            Ok(json!({"ok": true, "http": status, "data": v}))
        }
        "zai" | "bigmodel" => {
            let (env, url) = if id == "zai" {
                ("ZAI_API_KEY", "https://api.z.ai/api/monitor/usage/quota/limit")
            } else {
                ("BIGMODEL_API_KEY", "https://open.bigmodel.cn/api/monitor/usage/quota/limit")
            };
            let key = std::env::var(env)
                .ok()
                .filter(|k| !k.trim().is_empty())
                .ok_or_else(|| format!("未配置 {env}"))?;
            let resp = reqwest::Client::new()
                .get(url)
                .bearer_auth(key)
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
            if status >= 400 {
                return Err(format!("返回 {status}: {v}"));
            }
            Ok(json!({"ok": true, "http": status, "data": v}))
        }
        "trae" => Err("Trae 签到依赖 Trae CN 插件会话（magpie 走插件 fetch），裸 Token 不可用".into()),
        // 下面这些厂商有公开的余额接口，直接用「上游管理」里配的 Key 查
        "deepseek" | "dsh" | "DSH" => {
            let (_, key) = find_upstream("deepseek")
                .or_else(|| find_upstream("dsh"))
                .ok_or("未在「上游管理」里配 DeepSeek Key")?;
            let v = balance_of("DeepSeek", "https://api.deepseek.com/user/balance", &key).await?;
            Ok(json!({"ok": true, "platform": "DeepSeek", "data": v}))
        }
        "kimi" | "moonshot" | "Kimi" => {
            let (_, key) = find_upstream("kimi")
                .or_else(|| find_upstream("moonshot"))
                .ok_or("未在「上游管理」里配 Kimi Key")?;
            let v = balance_of("Kimi", "https://api.moonshot.cn/v1/users/me/balance", &key).await?;
            Ok(json!({"ok": true, "platform": "Kimi", "data": v}))
        }
        "siliconflow" | "硅基流动" => {
            let (_, key) = find_upstream("siliconflow")
                .or_else(|| find_upstream("硅基流动"))
                .ok_or("未在「上游管理」里配硅基流动 Key")?;
            let v = balance_of("硅基流动", "https://api.siliconflow.cn/v1/user/info", &key).await?;
            Ok(json!({"ok": true, "platform": "硅基流动", "data": v}))
        }
        "openrouter" => {
            let (_, key) = find_upstream("openrouter").ok_or("未在「上游管理」里配 OpenRouter Key")?;
            let v = balance_of("OpenRouter", "https://openrouter.ai/api/v1/credits", &key).await?;
            Ok(json!({"ok": true, "platform": "OpenRouter", "data": v}))
        }
        _ => Err("该平台需要配置上游 Key 后才能抓取".into()),
    }
}
