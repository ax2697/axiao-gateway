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

## v0.9.x 新增能力（2026-11-04 补录）

> 这一批是 0.9.0 ~ 0.9.5 新增的，上表未收录导致「38 项」这个数字失真。补录后**总数 38 → 59**。

| 能力 | 状态 | 落点 |
|:---|:---|:---|
| 生图（文生图 / 图生图） | 已实现 | `media::image_generate` + `media::media_save` |
| 音频生成 | 已实现 | `media::audio_generate` |
| 生视频 + 轮询取产物 | 已实现 | `media::video_generate` / `media::video_poll` |
| 本地媒体服务（产物托管） | 已实现 | `media_server::media_server_start/stop/status` |
| 访问 Key 管理（增删改查 / 明文查看） | 已实现 | `access::access_keys_get/save/reveal/delete` |
| 访问 Key 用量清零 | 已实现 | `access::access_usage_reset` |
| 熔断器状态与手动复位 | 已实现 | `breaker::breaker_status/reset` |
| 响应缓存统计 / 开关 / 清空 | 已实现 | `cache::cache_stats/set_enabled/clear` |
| 模型目录（读 / 列 / 加上游） | 已实现 | `catalog::catalog_get/catalog_models/catalog_add_upstream` + `catalog.json` |
| 本机凭证扫描（CLI / IDE 登录态） | 已实现 | `creds::scan_local_creds` |
| 端点探测（连通性 / 健康） | 已实现 | `probe::probe_endpoint` + `gateway::upstream_health` |
| 模型定价表（查 / 映射维护） | 已实现 | `pricing::price_lookup/model_map_get/model_map_set` |
| 协议转换工具链 | 已实现 | `convert.rs` + `openuri.rs` + `httpc.rs` |
| SSE 流式转发 | 已实现 | `chat_stream.rs` + `chat_proxy.rs` |
| 网关日志查看 / 清空 | 已实现 | `logs::gateway_logs/gateway_logs_clear` |
| 模型上下文（token 预算） | 已实现 | `modelctx.rs` |
| 请求分类 | 已实现 | `classify.rs` |
| 上游模型列表 | 已实现 | `gateway::upstream_models` |
| 更新日志读取 + 运行时版本号 | 已实现 | `changelog::changelog/app_version` |
| 上游删除 | 已实现 | `gateway::delete_upstream` |

## 统计

| 版本 | 已实现 | 内核已实现 | 不适用 | 总数 |
|:--|:--|:--|:--|:--|
| v0.1.0（原表） | 27 | 9 | 2 | 38 |
| **v0.9.5（含补录）** | **48** | **9** | **2** | **59** |

> 上表 38 项 + 本节 21 项 = **59 项**。另有 5 项（`catalog.json` 数据维护、定价表数据维护、
> `classify` 规则库、`modelctx` 策略配置、媒体服务端口配置）属**配置数据**而非独立能力，未单列。
