# 路线图

## v0.1.0（当前，源码完成）

- 28 个视图，38 项能力核对（27 已实现 + 9 内核已实现 + 2 不适用）
- 原生签到 / 成长 / 旅行 / 抽奖 / 保活
- 关联会话 / Token 统计 / 积分统计 / 免费额度
- 多上游（16 家预设）/ 三协议 / 成本账本
- 设备指纹、控制面、上下文压缩、ZCode 定时器、MCP 脚本、CF Worker
- 自动更新检查（16 个组件）

## v0.2.0（打包与发布）

- [ ] 完成 Rust 构建，产出 `axiao-gateway.exe`
- [ ] GitHub Releases 首个版本
- [ ] 安装包（NSIS）
- [ ] 截图 + 演示 GIF
- [ ] 新手教程（图文版）

## v0.3.0（稳定性）

- [ ] Tauri updater 签名（公私钥 + `latest.json`）
- [ ] 托盘开机自启
- [ ] 控制面 Key 分发给下游客户端（生成子 Key）
- [ ] 会话组同步（复制 transcript 到目标客户端）
- [ ] 错误日志查看/清空（对齐 switch 设置页）
- [ ] 通知历史（hook 事件 100 条）

## v0.4.0（协议与生态）

- [ ] MCP server 内置到 exe（免 Node）
- [ ] Claude Code / Codex / DSH 配置一键写
- [ ] 24 家官方 API 全量预设
- [ ] ZCode off-peak 真正执行（调 gateway_chat）
- [ ] 多语言（EN）

## v1.0.0

- [ ] 稳定 API / 数据迁移
- [ ] 插件机制（上游适配器）
- [ ] 文档站
