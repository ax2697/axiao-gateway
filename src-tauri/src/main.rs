#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod access;
mod auth;
mod breaker;
mod cache;
mod catalog;
mod changelog;
mod chat_proxy;
mod chat_stream;
mod classify;
mod clients;
mod config;
mod convert;
mod creds;
mod device;
mod extras;
mod gateway;
mod hooks;
mod httpc;
mod logs;
mod media;
mod media_server;
mod model;
mod modelctx;
mod openuri;
mod panel;
mod pricing;
mod probe;
mod quota;
mod signin;
mod state;
mod switch;
mod trace;
mod update;
mod store;

use std::sync::Mutex;

use state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

/// 无界面模式：只起本地 HTTP 服务，不开窗口、不建托盘。
///
/// 两个用途：
/// 1. 把网关当常驻服务跑（服务器、开机自启、给别人远程用）；
/// 2. 排障时排除 GUI 干扰 —— 窗口/WebView 起不来的机器上照样能用。
fn serve_only(argv: &[String]) -> i32 {
    // 落地标记：无界面模式下没有控制台，出问题只能靠文件确认"到底进没进这个分支"
    let marker = std::env::temp_dir().join("axiao-serve-only.marker");
    let _ = std::fs::write(&marker, format!("{argv:?}\n"));

    let mut port: u16 = 7864;
    let mut i = 0;
    while i < argv.len() {
        if (argv[i] == "--port" || argv[i] == "-p") && i + 1 < argv.len() {
            if let Ok(p) = argv[i + 1].parse() {
                port = p;
            }
        }
        i += 1;
    }
    glog!("启动模式：无界面（--serve-only），端口 {port}");
    match media_server::media_server_start(Some(port)) {
        Ok(v) => glog!("就绪：{v}"),
        Err(e) => {
            gwarn!("启动失败：{e}");
            return 1;
        }
    }
    glog!("日志文件：{}", logs::log_path().to_string_lossy());
    loop {
        std::thread::sleep(std::time::Duration::from_secs(600));
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv
        .iter()
        .any(|a| a == "--serve-only" || a == "--headless" || a == "-s")
    {
        std::process::exit(serve_only(&argv));
    }

    tauri::Builder::default()
        .manage(AppState {
            store: Mutex::new(store::Store::load()),
            gateway: Mutex::new(gateway::Gateway::new()),
        })
        .setup(|app| {
            // 启动时自动开「本地创作 API」（端口被占就静默跳过）
            let _ = media_server::media_server_start(None);
            let show = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            let _ = tray.build(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 账号存储
            store::get_accounts,
            store::switch_accounts,
            store::save_account,
            store::delete_account,
            store::import_accounts,
            store::export_accounts,
            // 凭证 / 密钥
            auth::read_auth,
            auth::switch_account,
            auth::store_secret,
            auth::load_secret,
            // 钩子
            hooks::append_hook,
            hooks::list_hooks,
            // 多上游网关
            gateway::list_upstreams,
            gateway::save_upstream,
            gateway::delete_upstream,
            gateway::upstream_models,
            gateway::upstream_health,
            media::image_generate,
            media::audio_generate,
            media::video_generate,
            media::video_poll,
            media::media_save,
            media_server::media_server_start,
            media_server::media_server_stop,
            media_server::media_server_status,
            changelog::changelog,
            changelog::app_version,
            logs::gateway_logs,
            logs::gateway_logs_clear,
            // 免费模型目录（freellmapi catalog）
            // 访问 Key（把网关给别人用时用）
            access::access_keys_get,
            access::access_key_save,
            access::access_key_reveal,
            access::access_key_delete,
            access::access_usage_reset,
            // 上游熔断
            breaker::breaker_status,
            breaker::breaker_reset,
            // 响应缓存
            cache::cache_stats,
            cache::cache_set_enabled,
            cache::cache_clear,
            catalog::catalog_get,
            catalog::catalog_models,
            catalog::catalog_add_upstream,
            creds::scan_local_creds,
            probe::probe_endpoint,
            pricing::model_map_get,
            pricing::model_map_set,
            pricing::price_lookup,
            gateway::ledger,
            gateway::gateway_chat,
            // 面板实时抓取（任务/模型/套餐）
            panel::panel_packages,
            panel::panel_tasks,
            panel::panel_task_auto_all,
            panel::panel_task_queue,
            panel::panel_models,
            panel::panel_get_config,
            panel::panel_save_config,
            panel::panel_login_start,
            panel::panel_login_poll,
            panel::panel_import_cockpit,
            panel::panel_import_cockpit_text,
            panel::open_external,
            // 签到（原生实现，不启动 exe）
            signin::native_checkin,
            signin::native_growth,
            signin::native_keepalive,
            signin::native_refresh,
            signin::native_status,
            // 积分任务（面板「积分任务」弹窗的原生版）
            signin::growth_tasks,
            signin::growth_accept,
            signin::growth_claim,
            // switch 原生模块（会话/Token/积分/设置）
            switch::switch_sessions,
            switch::switch_token_stats,
            switch::switch_usage_cache,
            switch::switch_settings,
            switch::switch_tool_status,
            switch::switch_get_auto,
            switch::switch_save_auto,
            switch::switch_session_groups,
            switch::switch_create_group,
            switch::switch_delete_group,
            switch::switch_credit_expiry,
            // 免费额度
            quota::quota_cards,
            quota::quota_refresh,
            // 便携版预置配置 / 说明
            config::local_config_get,
            config::local_config_save,
            config::portable_info,
            // 设备指纹 / 客户端接入
            device::device_id,
            device::device_ids,
            clients::cli_detect,
            clients::cli_import,
            clients::export_client,
            // 扩展能力（移动端/控制面/压缩/ZCode/思考档位）
            extras::extras_get,
            extras::extras_save,
            extras::mobile_info,
            extras::zcode_tick,
            // 自动更新
            update::update_targets,
            update::update_get_proxy,
            update::update_save_proxy,
            update::update_check,
            update::update_open,
            // 组件（exe 仅作备用）
            signin::component_status,
            signin::launch_signin,
            signin::launch_component,
        ])
        .run(tauri::generate_context!())
        .expect("阿晓网关助手启动失败");
}
