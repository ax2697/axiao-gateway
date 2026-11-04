# workbuddy-switch 逆向结果（动态 CDP 提取）

## 提取物

素材已归档到 `reference/switch-assets/`：

- `index-D0uigd0E.js` — 1,247,899 字节（完整前端逻辑）
- `index-BAujnMcC.css` — 104,417 字节（完整样式）
- `rendered.html` — 45,502 字节（渲染后 DOM）

## 完整 Tauri 命令表（93 个）

### 账号

- `delete_account`
- `export_accounts`
- `export_accounts_to_path`
- `get_accounts`
- `import_accounts`
- `list_account_sessions`
- `refresh_account_token`
- `switch_account`
- `switch_codebuddy_cli_account`
- `switch_codebuddy_cn_ide_account`
- `switch_codebuddy_ide_account`
- `switch_jetbrains_account`
- `switch_vscode_ext_account`
- `update_account_display`

### 会话

- `add_session_group_member`
- `delete_session_group`
- `get_session_group`
- `list_account_sessions`
- `list_codebuddy_ide_sessions`
- `list_codebuddy_intl_ide_sessions`
- `list_session_groups`
- `list_sessions`
- `list_vscode_sessions`
- `sync_session_group_pair`
- `sync_session_group_safe_batch`
- `sync_session_group_unify`

### Token/积分

- `credits`
- `get_credit_expiry`
- `get_credit_statistics`
- `get_rate_limit_config`
- `get_rate_limit_hook_status`
- `get_rate_limits`
- `get_token_statistics`
- `refresh_account_token`
- `save_rate_limit_config`
- `tokens`

### 客户端状态

- `get_codebuddy_cli_status`
- `get_codebuddy_cn_ide_status`
- `get_codebuddy_ide_status`
- `get_jetbrains_status`
- `get_vscode_ext_status`
- `list_codebuddy_ide_sessions`
- `list_codebuddy_intl_ide_sessions`
- `list_vscode_sessions`
- `switch_codebuddy_cli_account`
- `switch_codebuddy_cn_ide_account`
- `switch_codebuddy_ide_account`
- `switch_jetbrains_account`
- `switch_vscode_ext_account`

### 自动/设置

- `check_auth_permission`
- `check_update`
- `checkin`
- `checkin_all`
- `checkin_end`
- `checkin_start`
- `checking`
- `clear_notifications`
- `get_auto_checkin_config`
- `get_auto_rotate_config`
- `get_auto_travel_config`
- `get_checkin_logs`
- `get_checkin_status`
- `get_companion_enabled`
- `get_github_config`
- `get_launch_at_login_enabled`
- `get_rotate_logs`
- `get_travel_status`
- `list_notifications`
- `open_companion_settings`
- `open_permission_settings`
- `save_auto_checkin_config`
- `save_auto_rotate_config`
- `save_auto_travel_config`
- `save_github_config`
- `set_companion_enabled`
- `set_launch_at_login_enabled`
- `update`
- `update_account_display`
- `update_download`
- `update_restart`
- `update_state`

## 数据文件

- `~/.wb-switch/hook-events.jsonl` — 会话/客户端/模型
- `~/.wb-switch/official_usage_cache.json` — 积分用量 daily
- `~/.wb-switch/rate_limit_state.json` — 限流状态
- `~/.wb-switch/workbuddy_exe.json` — 工具路径
- `~/.wb-switch/hook.sh` — hook 注入脚本
