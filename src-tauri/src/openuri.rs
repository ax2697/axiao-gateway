//! 打开外部链接的唯一入口。
//!
//! 为什么统一：Windows 上打开浏览器最容易想到 `cmd /c start`，但那意味着
//! 整条 URL 会先过一遍 cmd 的词法分析。`&` 会被当命令分隔符，`%PATH%` 会被
//! 展开成本机路径，`^` 是转义符 —— 而这些 URL 里有**远程数据**（面板下发的
//! `jump_url`、更新检查返回的下载地址），不是我们写死的常量。
//!
//! 所以这里直接调 `ShellExecuteW`：URL 作为参数传给外壳，不经过任何 shell
//! 解析，`%` / `&` / `^` / `|` 都只是普通字符。
//!
//! 仍然保留协议白名单：只允许 http/https，避免 `file:`、`javascript:` 之类
//! 被当成"链接"打开。

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut core::ffi::c_void,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> *mut core::ffi::c_void;
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn open(url: &str) -> Result<(), String> {
        let op = wide("open");
        let file = wide(url);
        let rc = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                op.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        } as isize;
        // 文档约定：返回值 > 32 才算成功
        if rc > 32 {
            Ok(())
        } else {
            Err(format!("系统打不开这个链接（ShellExecute 返回 {rc}）"))
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn open(url: &str) -> Result<(), String> {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

pub fn open_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("链接是空的".into());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("只允许打开 http/https 链接".into());
    }
    // 控制字符会让日志/显示错乱，直接拒掉（换行在 URL 里本来也不合法）
    if url.chars().any(|c| c.is_control()) {
        return Err("链接里含控制字符".into());
    }
    platform::open(url)
}
