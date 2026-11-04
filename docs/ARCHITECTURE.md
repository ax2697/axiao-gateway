# 架构说明

## 1. 分层

```
┌─────────────────────────────────────────────┐
│ 前端（WebView2）  src/index.html / app.js    │
│ 28 个视图：账号/会话/Token/积分/额度/网关/任务/系统 │
└───────────────┬─────────────────────────────┘
                │ Tauri invoke
┌───────────────▼─────────────────────────────┐
│ Rust 后端  src-tauri/src                     │
│ store/auth/gateway/panel/quota/switch/signin │
│ extras/config/update/hooks                   │
└───────┬───────────────┬─────────────────────┘
        │               │
┌───────▼──────┐  ┌─────▼──────────────────────┐
│ 便携版文件    │  │ 本地/远程服务               │
│ auths/       │  │ panel 127.0.0.1:7863        │
│ data/state   │  │ copilot.tencent.com（签到）  │
│ config.json  │  │ api.trae.cn / z.ai / …      │
└──────────────┘  └────────────────────────────┘
```

**原则**：控制台不重复实现内核。多账号池、熔断、三协议、429、会话粘性等由 7863 的 panel 内核执行，控制台负责配置、观测与补充能力（原生签到、免费额度、会话、更新）。

## 2. Rust 模块

| 模块 | 职责 | 关键命令 |
|---|---|---|
| `main.rs` | 注册全部命令、初始化状态 | — |
| `model.rs` | Account / ClientRule / ExportBundle / Upstream / Ledger / Hook | — |
| `state.rs` | `Mutex<Store>` + `Mutex<Gateway>` | — |
| `store.rs` | 账号存储、导入导出、REA 规则、合并 auths/state | `get_accounts` `save_account` `import_accounts` `export_accounts` |
| `auth.rs` | 授权文件读写、keyring | `read_auth` `switch_account` `store_secret` `load_secret` |
| `gateway.rs` | 多上游、三协议、冷却、粘性、账本、压缩 | `gateway_chat` `list_upstreams` `save_upstream` `ledger` |
| `panel.rs` | 面板实时接口 | `panel_packages` `panel_tasks` `panel_models` `panel_login_*` `panel_import_cockpit` |
| `quota.rs` | 免费额度抓取 | `quota_cards` `quota_refresh` |
| `switch.rs` | 会话/Token/积分/工具/会话组 | `switch_sessions` `switch_token_stats` `switch_tool_status` `switch_get_auto` |
| `signin.rs` | 原生签到/成长/保活/刷新 | `native_checkin` `native_growth` `native_keepalive` `native_refresh` |
| `extras.rs` | 移动端/控制面/压缩/ZCode/思考 | `extras_get` `extras_save` `mobile_info` |
| `config.rs` | 便携版 config 直读直写 | `local_config_get` `local_config_save` `portable_info` |
| `update.rs` | 组件更新检查 | `update_targets` `update_check` `update_save_proxy` |
| `hooks.rs` | 钩子事件 | `append_hook` `list_hooks` |

## 3. 关键数据流

### 签到

```
前端「立即签到」
  → native_checkin(uid)
    → store 取 accessToken
    → POST /v2/billing/meter/checkin-activity-status
    → POST /v2/billing/meter/daily-checkin
  → 返回 JSON → 前端日志 + 钩子
```

### 任务

```
前端 loadPanel
  → panel_packages → 取 uid
  → panel_tasks(uid) → GET /panel/api/accounts/{uid}/tasks
  → 渲染任务表
一键完成 → panel_task_auto_all(uid)
```

### 配置

```
前端「网关配置」
  → local_config_get（直接读 config.json，不依赖 panel）
  → 表单 46 字段
保存 → local_config_save（备份）→ 尝试 panel_save_config 热生效
```

### 网关请求

```
gateway_chat(req)
  → extras 压缩开关 → compress_body
  → 选上游（权重/冷却）
  → POST {base}/v1/(chat/completions|responses|messages)
  → 解析 usage → ledger.jsonl
```

## 4. 安全边界

- 授权文件写前备份 `*.axiao-bak-<ts>`。
- `config.json` 写前备份 `.json.axiao-bak`。
- 密钥存 Windows 凭据管理器（service=`axiao-gateway`）。
- 前端不落明文 token；导出默认脱敏。

## 5. 扩展点

- 新增上游：`save_upstream` + 预设数组。
- 新增平台检测：`switch_tool_status` 加路径。
- 新增免费额度源：`quota_cards` + `quota_refresh`。
- 新增命令：`main.rs` 注册 + 前端 `invoke`。
