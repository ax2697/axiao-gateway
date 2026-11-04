# 能力覆盖说明（38 项）

> 对应应用内「能力覆盖」页。状态分 5 类，避免把「内核已实现」误算成缺。

| 状态 | 含义 |
|---|---|
| 已实现 | 控制台自有代码完成 |
| 内核已实现 | 由 7863 的 panel 内核执行，控制台只做配置/观测 |
| 控制台部分 | 控制台只做入口/持久化，执行依赖外部 |
| 不适用 | 桌面形态不需要，或属于其它内核属性 |
| 需外部设施 | 需要发布源 / Cloudflare 账号 |

## 清单

| 能力 | 状态 | 落点 |
|---|---|---|
| 多账号池轮转 | 内核 | panel pool |
| Token 续期 / 保活 | 已实现 | `native_refresh` / `native_keepalive` |
| OpenAI 兼容透传 | 内核 | panel / gateway |
| 熔断 / 冷却 | 内核 | panel pool |
| 会话粘性 | 内核 | panel pool |
| 自动签到 / 活跃保活 | 已实现 | `native_checkin` / `native_keepalive` |
| 工具调用 / 流式 | 内核 | panel |
| 双站 / 多区域路由 | 内核 | panel global |
| 三协议 | 已实现 | `gateway.rs` + 协议页 |
| 设备指纹隔离 | 已实现 | `device.rs` + 网关注入 |
| 成本账本 | 已实现 | `ledger.jsonl` |
| 读本地客户端登录态 | 已实现 | `store.rs` 读 auths |
| 多桌面端聚合 + 队列 + 429 | 内核 | panel pool |
| 成长任务 + Web 面板 | 已实现 | 任务中心 + panel |
| 移动端常驻 | 已实现 | `mobile_info` + LAN 地址 |
| 多上游三合一 | 已实现 | 16 家上游预设 |
| 配额感知切号 + 混挂 | 内核 | panel pool |
| 控制面 | 已实现 | 网关校验 Key/模型白名单 |
| 极简 CLI 复用 | 已实现 | `cli_import` |
| CodeBuddy CN OAuth2 | 已实现 | 添加账号 |
| Docker Web 控制台 | 不适用 | 桌面内嵌 panel |
| 上下文压缩 | 已实现 | `compress_body` |
| 原生 Claude Code / Codex / DSH | 已实现 | `export_client` |
| 429 退避 | 内核 | panel |
| 复用桌面端登录态 + DSH | 已实现 | 账号管理 |
| 全家桶 7+ | 已实现 | 16 家预设 |
| 全家桶 + 自定义 OpenAI | 已实现 | 上游管理 |
| 24 家官方 API + MCP | 已实现 | 目录 + `scripts/mcp-server.mjs` + CF Worker |
| ZCode off-peak | 已实现 | `zcode_tick` + 队列 |
| Go 零依赖 + 热更新 | 不适用 | wb2go 内核属性 |
| Python + WebUI + 浏览器登录 | 已实现 | 添加账号 |
| panel 快照 | 已实现 | 融合组件 |
| 批量签到 | 已实现 | 签到中心 |
| 用量分析仪表盘 | 已实现 | Token 统计 |
| WB+Trae+Qoder + 托盘 | 已实现 | 聚合 + Tauri tray |
| 多上游抽象 + 可扩展 | 已实现 | 上游管理 |
| 免费额度聚合 | 已实现 | 免费额度 |
| Deno+CF 10 上游 | 已实现 | `deploy/cloudflare-worker.js`（部署需 CF 账号） |

## 统计

- 已实现：27
- 内核已实现：9
- 不适用：2
- 控制台部分 / 需外部设施：0
