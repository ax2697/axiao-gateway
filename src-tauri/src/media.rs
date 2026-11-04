use std::time::Duration;

use serde_json::{json, Value};

use crate::state::AppState;

/// 极简 base64 解码（支持 data URL 前缀），避免为这一个功能引入新依赖。
fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.split(",").last().unwrap_or(s).trim();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("图片数据不是合法的 base64".into()),
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

fn target(state: &AppState, id: &str) -> Result<(String, String, String, String), String> {
    let g = state.gateway.lock().map_err(|e| e.to_string())?;
    let u = g
        .upstreams
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| format!("上游不存在：{id}"))?;
    Ok((
        u.base_url.trim_end_matches('/').to_string(),
        u.api_key.clone().unwrap_or_default(),
        u.protocol.clone(),
        u.name.clone(),
    ))
}

fn apply_auth(
    rb: reqwest::RequestBuilder,
    protocol: &str,
    key: &str,
) -> reqwest::RequestBuilder {
    if key.is_empty() {
        return rb;
    }
    if protocol == "anthropic" {
        rb.header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    } else {
        rb.bearer_auth(key)
    }
}

/// 文生图 / 图生图（OpenAI 兼容：/images/generations 与 /images/edits）
#[tauri::command]
pub async fn image_generate(
    state: tauri::State<'_, AppState>,
    upstream: String,
    prompt: String,
    model: Option<String>,
    size: Option<String>,
    image: Option<String>,
) -> Result<Value, String> {
    let (base, key, protocol, name) = target(&state, &upstream)?;
    let client = crate::httpc::client_for(&base)?;
    let upstream_model = {
        let g = state.gateway.lock().map_err(|e| e.to_string())?;
        g.upstreams
            .iter()
            .find(|x| x.id == upstream)
            .and_then(|x| x.model.clone())
            .filter(|m| !m.trim().is_empty())
    };
    let model = model
        .or(upstream_model)
        .unwrap_or_else(|| "gpt-image-1".into());
    let size = size.unwrap_or_else(|| "1024x1024".into());

    let rb = match image.filter(|s| !s.trim().is_empty()) {
        // 图生图：multipart 上传原图
        Some(img) => {
            let bytes = b64_decode(&img)?;
            let part = reqwest::multipart::Part::bytes(bytes)
                .file_name("input.png")
                .mime_str("image/png")
                .map_err(|e| e.to_string())?;
            let form = reqwest::multipart::Form::new()
                .text("prompt", prompt.clone())
                .text("model", model.clone())
                .text("size", size.clone())
                .part("image", part);
            let path = if name.contains("智谱") || name.to_lowercase().contains("bigmodel") {
                "/images/generations"
            } else {
                "/images/edits"
            };
            client.post(format!("{base}{path}")).multipart(form)
        }
        None => client
            .post(format!("{base}/images/generations"))
            .json(&json!({ "model": model, "prompt": prompt, "size": size, "n": 1 })),
    };

    let resp = apply_auth(rb, &protocol, &key)
        .timeout(Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}（检查该上游的 Base URL 与 Key）"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    Ok(json!({
        "status": status,
        "upstream": upstream,
        "data": data,
        "path": if status == 404 { "该上游没有 /images 接口，换一家支持画图的上游" } else { "" },
    }))
}

/// 文生视频 / 图生视频：智谱走 /videos/generations + /async-result 轮询；
/// 其它平台按 OpenAI 风格试 /videos。
#[tauri::command]
pub async fn video_generate(
    state: tauri::State<'_, AppState>,
    upstream: String,
    prompt: String,
    model: Option<String>,
    image: Option<String>,
) -> Result<Value, String> {
    let (base, key, protocol, name) = target(&state, &upstream)?;
    let client = crate::httpc::client_for(&base)?;
    let zhipu = name.contains("智谱") || name.to_lowercase().contains("bigmodel");
    // 豆包桥的路径是 /videos/generations，用 GET /videos/generations/{id} 轮询
    let bridge = name.contains("豆包桥") || base.contains(":9090");
    let upstream_model = {
        let g = state.gateway.lock().map_err(|e| e.to_string())?;
        g.upstreams
            .iter()
            .find(|x| x.id == upstream)
            .and_then(|x| x.model.clone())
            .filter(|m| !m.trim().is_empty())
    };
    let model = model.or(upstream_model).unwrap_or_else(|| {
        if zhipu { "cogvideox-flash".to_string() } else { "sora-2".to_string() }
    });

    let mut body = json!({ "model": model, "prompt": prompt });
    if let Some(img) = image.filter(|s| !s.trim().is_empty()) {
        // 主流实现都接受 image 字段（data URL 或 base64）
        body["image"] = json!(img);
    }
    let path = if zhipu || bridge { "/videos/generations" } else { "/videos" };
    let resp = apply_auth(client.post(format!("{base}{path}")), &protocol, &key)
        .json(&body)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    Ok(json!({ "status": status, "upstream": upstream, "zhipu": zhipu, "data": data }))
}

/// 查询视频任务进度
#[tauri::command]
pub async fn video_poll(
    state: tauri::State<'_, AppState>,
    upstream: String,
    task_id: String,
    zhipu: Option<bool>,
) -> Result<Value, String> {
    let (base, key, protocol, _name) = target(&state, &upstream)?;
    let client = crate::httpc::client_for(&base)?;
    let bridge = base.contains(":9090");
    let path = if zhipu.unwrap_or(false) {
        format!("/async-result/{task_id}")
    } else if bridge {
        format!("/videos/generations/{task_id}")
    } else {
        format!("/videos/{task_id}")
    };
    let resp = apply_auth(client.get(format!("{base}{path}")), &protocol, &key)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    Ok(json!({ "status": status, "data": data }))
}

/// 把生成结果（url 或 b64）落盘到用户选的路径
#[tauri::command]
pub async fn media_save(url_or_b64: String, path: String) -> Result<String, String> {
    if url_or_b64.starts_with("http") {
        let resp = reqwest::get(&url_or_b64)
            .await
            .map_err(|e| format!("下载失败：{e}"))?;
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    } else {
        let bytes = b64_decode(&url_or_b64)?;
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

/// 文生音乐 / 文生音频。
///
/// 目前主要给「豆包桥」（本机 9090）用——它的 `/v1/audio/generations`
/// 能出音乐。别的平台如果有同名接口也能直接透传（失败会明确说"没有 /audio 接口"，
/// 而不是丢一个看不懂的 400）。
///
/// 参数走 `extra` 整体透传：各家音乐接口的字段差异很大（时长、风格、歌词、
/// 是否带人声…），写死几个参数反而会挡住人家本来就有的能力。
#[tauri::command]
pub async fn audio_generate(
    state: tauri::State<'_, AppState>,
    upstream: String,
    prompt: String,
    model: Option<String>,
    extra: Option<Value>,
) -> Result<Value, String> {
    let (base, key, protocol, name) = target(&state, &upstream)?;
    let client = crate::httpc::client_for(&base)?;
    let upstream_model = {
        let g = state.gateway.lock().map_err(|e| e.to_string())?;
        g.upstreams
            .iter()
            .find(|x| x.id == upstream)
            .and_then(|x| x.model.clone())
            .filter(|m| !m.trim().is_empty())
    };
    let model = model.or(upstream_model).unwrap_or_else(|| "doubao-music".into());

    let mut payload = extra.unwrap_or_else(|| json!({}));
    match payload.as_object_mut() {
        Some(o) => {
            o.entry("model".to_string()).or_insert(json!(model));
            o.entry("prompt".to_string()).or_insert(json!(prompt));
        }
        None => payload = json!({ "model": model, "prompt": prompt }),
    }

    let resp = apply_auth(client.post(format!("{base}/audio/generations")), &protocol, &key)
        .json(&payload)
        .timeout(Duration::from_secs(180))
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}（检查该上游的 Base URL 与 Key）"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
    crate::glog!("音乐生成 → 上游={name} 状态 {status}");
    Ok(json!({
        "status": status,
        "upstream": upstream,
        "data": data,
        "path": if status == 404 {
            "该上游没有 /audio 接口——音乐生成目前主要靠「豆包桥（本地）」"
        } else { "" },
    }))
}
