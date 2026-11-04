use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use crate::store::data_dir;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTarget {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub current: String,
    pub repo: String,
    pub note: String,
}

fn proxy_file() -> PathBuf {
    data_dir().join("update.json")
}

fn load_proxy() -> String {
    std::fs::read_to_string(proxy_file())
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.get("proxy").and_then(|x| x.as_str()).map(|s| s.to_string()))
        .unwrap_or_default()
}

#[tauri::command]
pub fn update_targets() -> Result<Vec<UpdateTarget>, String> {
    let mut out = vec![
        UpdateTarget {
            id: "app".into(),
            name: "阿晓网关助手".into(),
            kind: "self".into(),
            current: "0.1.0".into(),
            repo: "ax2697/axiao-gateway".into(),
            note: "本应用自身（GitHub Releases 自更新）".into(),
        },
        UpdateTarget {
            id: "panel".into(),
            name: "workbuddy2api-panel".into(),
            kind: "kernel".into(),
            current: "v1.13.0".into(),
            repo: "linguo2625469/workbuddy2api-panel".into(),
            note: "网关内核 / 主分支".into(),
        },
        UpdateTarget {
            id: "switch".into(),
            name: "workbuddy-switch".into(),
            kind: "desktop".into(),
            current: "v0.1.62".into(),
            repo: "changexbc/workbuddy-switch".into(),
            note: "账号切换 / 会话 / Token 统计".into(),
        },
        UpdateTarget {
            id: "signin".into(),
            name: "WorkBuddy-Daily".into(),
            kind: "engine".into(),
            current: "v3.35".into(),
            repo: "L0NE-6/WorkBuddy-Daily".into(),
            note: "签到 / 成长任务引擎".into(),
        },
    ];
    for (id, name, repo) in [
        ("hub", "workbuddy2api-hub", "ardeyouxipianyi/workbuddy2api-hub"),
        ("3in1", "workbuddy-3in1", "jjjane233/workbuddy-3in1"),
        ("wb2go", "wb2go", "wangct233-source/wb2go"),
        ("modelwharf", "modelwharf", "zouzhenglu/modelwharf"),
        ("wildwork", "wild-work", "yanghuan/wild-work"),
        ("multi2api", "multi2api", "practice019/multi2api"),
        ("magpie", "magpie", "yetone/magpie"),
        ("aiproxy", "ai-proxy", "HitMargin/ai-proxy"),
        ("agent2api", "Agent2API", "aimod-cc/Agent2API"),
        ("free2api", "Free2API", "ikunops/free2api"),
        ("gateway", "workbuddy-gateway", "CangShui/workbuddy-gateway"),
        ("workbuddy2api", "workbuddy2api", "HanawaBanana/workbuddy2api"),
    ] {
        out.push(UpdateTarget {
            id: id.into(),
            name: name.into(),
            kind: "kernel".into(),
            current: "cloned".into(),
            repo: repo.into(),
            note: "已下载到 fusion/vendor".into(),
        });
    }
    Ok(out)
}

#[tauri::command]
pub fn update_get_proxy() -> Result<String, String> {
    Ok(load_proxy())
}

#[tauri::command]
pub fn update_save_proxy(proxy: String) -> Result<String, String> {
    let _ = std::fs::create_dir_all(data_dir());
    let body = json!({ "proxy": proxy });
    std::fs::write(proxy_file(), serde_json::to_string_pretty(&body).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(proxy)
}

#[tauri::command]
pub async fn update_check(id: String) -> Result<Value, String> {
    let targets = update_targets()?;
    let t = targets
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| "未知组件".to_string())?;
    if t.repo.is_empty() {
        return Err("该组件暂无公开更新源".into());
    }
    let proxy = load_proxy();
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("axiao-gateway-updater");
    if !proxy.trim().is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(proxy.trim()).map_err(|e| e.to_string())?);
    }
    let client = builder.build().map_err(|e| e.to_string())?;
    let api = "https://api.github.com";

    // 1) 仓库是否存在：404 说明改名/删库/slug 写错，直接说清楚，别甩一坨 JSON
    let repo_resp = client
        .get(format!("{api}/repos/{}", t.repo))
        .send()
        .await
        .map_err(|e| format!("连不上 GitHub（可在上面配代理）：{e}"))?;
    let repo_status = repo_resp.status().as_u16();
    let repo_v: Value = repo_resp.json().await.unwrap_or_else(|_| json!({}));
    if repo_status == 404 {
        return Err(format!(
            "仓库 {} 不存在（可能已改名 / 删库 / 转为私有），请点「更新源」换成新地址",
            t.repo
        ));
    }
    if repo_status == 403 || repo_status == 429 {
        return Err(format!("GitHub 限流了（{repo_status}），等几分钟或配代理再试"));
    }
    if repo_status >= 400 {
        return Err(format!("GitHub 返回 {repo_status}：{}", api_msg(&repo_v)));
    }

    // 2) 有 Release 就用 Release
    let rel = client
        .get(format!("{api}/repos/{}/releases/latest", t.repo))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let rel_status = rel.status().as_u16();
    let v: Value = rel.json().await.unwrap_or_else(|_| json!({}));
    if (200..300).contains(&rel_status) {
        return Ok(json!({
            "ok": true,
            "id": t.id,
            "current": t.current,
            "latest": v.get("tag_name").and_then(|x| x.as_str()).unwrap_or(""),
            "name": v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
            "publishedAt": v.get("published_at").and_then(|x| x.as_str()).unwrap_or(""),
            "url": v.get("html_url").and_then(|x| x.as_str()).unwrap_or(""),
            "noRelease": false,
        }));
    }

    // 3) 仓库存在但没有 Release（GitHub 对 latest 会返回 404）→ 退回看最新 tag
    if rel_status == 404 {
        let tags = client
            .get(format!("{api}/repos/{}/tags?per_page=1", t.repo))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let tags_v: Value = tags.json().await.unwrap_or_else(|_| json!([]));
        let tag = tags_v
            .get(0)
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        return Ok(json!({
            "ok": true,
            "id": t.id,
            "current": t.current,
            "latest": tag,
            "name": if tag.is_empty() { "该仓库没有发过 Release，也没有 tag".to_string() } else { format!("最新 tag：{tag}（该仓库未发 Release）") },
            "publishedAt": repo_v.get("pushed_at").and_then(|x| x.as_str()).unwrap_or(""),
            "url": repo_v.get("html_url").and_then(|x| x.as_str()).unwrap_or(""),
            "noRelease": true,
            "note": "仓库存在，只是作者没发 Release；上面的时间是最新提交时间",
        }));
    }

    Err(format!("GitHub 返回 {rel_status}：{}", api_msg(&v)))
}

fn api_msg(v: &Value) -> String {
    v.get("message")
        .and_then(|x| x.as_str())
        .unwrap_or("未知错误")
        .to_string()
}

#[tauri::command]
pub fn update_open(url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("只允许打开 http/https 链接".into());
    }
    crate::openuri::open_url(&url)
}
