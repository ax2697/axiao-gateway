/// 更新日志：把仓库根目录的 CHANGELOG.md 在编译期嵌进 exe，
/// 这样单文件分发也能看到改了什么，不依赖外部文件。
const RAW: &str = include_str!("../../CHANGELOG.md");

#[tauri::command]
pub fn changelog() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "text": RAW,
    }))
}
