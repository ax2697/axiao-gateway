//! 免费模型目录。
//!
//! 数据来自 freellmapi 的 `server/data/freeapi.db`（models / media_models /
//! embedding_models 三张表）与 `server/src/providers/*.ts`（每家厂商的
//! platform、显示名、baseUrl、协议），由 `scripts/build-catalog.py` 在构建前
//! 提取成 `catalog.json`，这里 `include_str!` 编译进 exe。
//!
//! 为什么不用联网拉：这 410 个模型的配额数字是静态快照，跟本应用同一个数据源，
//! 联网只会多一个失败点。要更新就重跑脚本再编译。
//!
//! 这解决了什么问题：以前「免费额度」页里那 24 张卡是硬编码的空壳，
//! 全部显示「未开放接口」——用户根本不知道每家能免费用多少。现在每张卡
//! 都有真实的模型数、RPM/RPD 限额、月度预算描述，还能一键加成上游。

use std::sync::OnceLock;

use serde_json::{json, Value};

const RAW: &str = include_str!("catalog.json");

static DATA: OnceLock<Value> = OnceLock::new();

fn data() -> &'static Value {
    DATA.get_or_init(|| serde_json::from_str(RAW).unwrap_or_else(|_| json!({})))
}

fn find(platform: &str) -> Option<Value> {
    data()
        .get("providers")
        .and_then(|x| x.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|p| p.get("id").and_then(|x| x.as_str()) == Some(platform))
                .cloned()
        })
}

#[tauri::command]
pub fn catalog_get() -> Result<Value, String> {
    Ok(data().clone())
}

/// 某家厂商的模型明细。models 是紧凑数组：
/// [platform, model_id, display_name, intel_rank, speed_rank, ctx,
///  vision, tools, rpm, rpd, budget]
#[tauri::command]
pub fn catalog_models(platform: String) -> Result<Value, String> {
    let rows: Vec<Value> = data()
        .get("models")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|m| m.get(0).and_then(|x| x.as_str()) == Some(platform.as_str()))
                .map(|m| {
                    json!({
                        "id": m.get(1),
                        "name": m.get(2),
                        "intelRank": m.get(3),
                        "speedRank": m.get(4),
                        "context": m.get(5),
                        "vision": m.get(6).and_then(|x| x.as_i64()).unwrap_or(0) == 1,
                        "tools": m.get(7).and_then(|x| x.as_i64()).unwrap_or(0) == 1,
                        "rpm": m.get(8),
                        "rpd": m.get(9),
                        "budget": m.get(10),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({ "platform": platform, "count": rows.len(), "models": rows }))
}

/// 把目录里的某家一键加成上游。
///
/// 幂等：同一个平台重复点只会更新那一条，不会堆出一串重复上游。
/// 已有的 Key 会保留（用户填过就不该被空值覆盖）。
#[tauri::command]
pub fn catalog_add_upstream(
    platform: String,
    api_key: Option<String>,
) -> Result<Value, String> {
    let p = find(&platform).ok_or_else(|| format!("目录里没有 {platform} 这家"))?;
    let base = p.get("baseUrl").and_then(|x| x.as_str()).unwrap_or("");
    if base.is_empty() {
        return Err(format!("{} 没解析到 Base URL，只能手动去「上游管理」加", platform));
    }
    if base.contains('{') {
        return Err(format!(
            "{platform} 的地址里带占位符（{base}），需要先替换成你自己的账户 ID"
        ));
    }

    let id = format!("cat_{platform}");
    let name = p.get("name").and_then(|x| x.as_str()).unwrap_or(&platform);
    let protocol = p
        .get("protocol")
        .and_then(|x| x.as_str())
        .unwrap_or("openai");

    let dir = crate::store::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("upstreams.json");
    let mut list: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Value>>(&t).ok())
        .unwrap_or_default();

    let key = api_key.unwrap_or_default();
    let existed = list
        .iter()
        .position(|u| u.get("id").and_then(|x| x.as_str()) == Some(id.as_str()));

    let entry = json!({
        "id": id,
        "name": name,
        "baseUrl": base,
        "apiKey": key.clone(),
        "protocol": protocol,
        "enabled": true,
        "weight": 1,
        "model": Value::Null,
        "note": format!("来自免费模型目录（{}）", platform),
    });

    match existed {
        Some(i) => {
            // 保留原来的 Key：用户没填新的时候别把旧 Key 抹了
            let old_key = list[i]
                .get("apiKey")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let mut e = entry.clone();
            if key.is_empty() && !old_key.is_empty() {
                if let Some(o) = e.as_object_mut() {
                    o.insert("apiKey".into(), json!(old_key));
                }
            }
            list[i] = e;
        }
        None => list.push(entry),
    }

    let text = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("写 upstreams.json 失败：{e}"))?;

    crate::glog!(
        "免费目录：{} 已{}为上游（{}，协议 {}）",
        name,
        if existed.is_some() { "更新" } else { "新增" },
        base,
        protocol
    );

    Ok(json!({
        "ok": true,
        "created": existed.is_none(),
        "id": format!("cat_{platform}"),
        "name": name,
        "baseUrl": base,
        "needsKey": key.is_empty(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个测试里跑完所有断言：这些 case 共用 WB_AXIAO_DATA 环境变量，
    /// 拆成多个 #[test] 并行执行会互相改环境变量导致随机失败。
    #[test]
    fn catalog_and_add_upstream() {
        // 目录本身：35 家、每家都有 baseUrl
        let d = data();
        let ps = d.get("providers").and_then(|x| x.as_array()).unwrap();
        assert_eq!(ps.len(), 35, "厂商数变了？");
        for p in ps {
            let base = p.get("baseUrl").and_then(|x| x.as_str()).unwrap_or("");
            assert!(!base.is_empty(), "{} 没有 baseUrl", p.get("id").unwrap());
        }
        assert!(super::catalog_models("google".into()).unwrap()["count"].as_u64().unwrap() > 10);

        // 一键加为上游：幂等 + 保留已填的 Key
        let dir = std::env::temp_dir().join(format!("axiao-cat-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("WB_AXIAO_DATA", &dir);

        let read = || -> Vec<Value> {
            serde_json::from_str(
                &std::fs::read_to_string(dir.join("upstreams.json")).unwrap_or_default(),
            )
            .unwrap_or_default()
        };

        let r = catalog_add_upstream("google".into(), None).unwrap();
        assert_eq!(r["created"], true);
        let l = read();
        assert_eq!(l.len(), 1);
        assert_eq!(
            l[0]["baseUrl"].as_str().unwrap(),
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(l[0]["protocol"].as_str().unwrap(), "gemini");

        // 第二次带 Key：更新，不是新增，不能出现两条
        let r2 = catalog_add_upstream("google".into(), Some("k-123".into())).unwrap();
        assert_eq!(r2["created"], false);
        let l2 = read();
        assert_eq!(l2.len(), 1, "重复加成上游了");
        assert_eq!(l2[0]["apiKey"].as_str().unwrap(), "k-123");

        // 第三次不带 Key：旧的 Key 要留着，不能被空值冲掉
        let _ = catalog_add_upstream("google".into(), None).unwrap();
        let l3 = read();
        assert_eq!(l3[0]["apiKey"].as_str().unwrap(), "k-123", "Key 被空值覆盖了");

        // 带占位符地址的（cloudflare 要 account id）应该拒绝，而不是加个用不了的
        let e = catalog_add_upstream("cloudflare".into(), None).unwrap_err();
        assert!(e.contains("占位符"), "实际错误：{e}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
