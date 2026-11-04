# 贡献指南

## 开发环境

```bash
npm install
npm run dev
```

Rust 后端在 `src-tauri/`，前端在 `src/`。Tauri 命令改动需同时注册到 `main.rs` 的 `invoke_handler!`。

## 代码风格

- Rust：`cargo fmt`，`cargo clippy`（不强制零警告，但不要新增明显问题）
- 前端：原生 JS，无框架；缩进 2 空格；函数保持短小
- 提交前跑 `node --check src/app.js`
- 不要提交 `auths/`、`config.json`、`target/`、`node_modules/`

## 分支与提交

- 分支：`feat/xxx`、`fix/xxx`、`docs/xxx`
- 提交信息：`feat: ...` / `fix: ...` / `docs: ...` / `refactor: ...`
- 一个 PR 只做一件事；UI 改动附截图

## 新增能力

1. Rust 侧加模块/命令 → `main.rs` 注册
2. 前端加视图 → `index.html` 的 `#nav` + `.view` + `PAGE_META`
3. 需要持久化 → `extras.json` 或 `store.rs`
4. 更新 `docs/ARCHITECTURE.md` 与覆盖表

## 安全

- 不要在任何代码/文档里写真实 token、api_key、账号
- 示例配置统一用 `sk-REPLACE_ME`
- 涉及凭证的功能必须默认脱敏

## 许可证

贡献即同意以 MIT 协议发布。
