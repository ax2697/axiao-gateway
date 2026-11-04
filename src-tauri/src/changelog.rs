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

/// 当前版本。给界面显示的**唯一来源**。
///
/// 以前左下角和关于页各自写死了 "0.1.0"，版本改到 0.9.x 了界面上还是 0.1.0，
/// 用户看到的就是"怎么还是旧版"。版本号只该有一个来源：编译时的 Cargo.toml。
#[tauri::command]
pub fn app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}
