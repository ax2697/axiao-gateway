use std::time::Duration;

/// 探测一个本地/远端地址是否活着（用于「本地桥」类上游）
/// 只发 GET，超时 3 秒，不跟随重定向。
#[tauri::command]
pub async fn probe_endpoint(url: String) -> Result<serde_json::Value, String> {
    let url = url.trim().trim_end_matches('/').to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("只支持 http/https".into());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    for path in ["/health", "/v1/models", ""] {
        let target = format!("{url}{path}");
        if let Ok(resp) = client.get(&target).send().await {
            let status = resp.status().as_u16();
            if status < 500 {
                return Ok(serde_json::json!({
                    "ok": status < 400,
                    "status": status,
                    "url": target,
                    "hint": if status < 400 { "服务在线" } else { "端口通了但接口返回错误" },
                }));
            }
        }
    }
    Ok(serde_json::json!({
        "ok": false,
        "status": 0,
        "url": url,
        "hint": "连不上——本机没有这个服务（端口未监听）",
    }))
}
