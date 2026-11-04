# 文件清单（移交对照）

## 根目录

| 文件 | 说明 |
|---|---|
| `README.md` | 项目介绍、功能、构建、协议 |
| `HANDOVER.md` | **交接清单**（状态、构建、数据、接口、发布、checklist） |
| `BUILD-STATUS.md` | 本机构建记录与踩坑 |
| `CHANGELOG.md` | 版本日志 |
| `CONTRIBUTING.md` | 贡献指南 |
| `SECURITY.md` | 安全策略 / 凭证处理 |
| `LICENSE` | MIT |
| `THIRD_PARTY_NOTICES.md` | 第三方源码/商标声明 |
| `config.example.json` | 网关配置模板（密钥已脱敏） |
| `.gitignore` | 排除凭证、产物、vendor |
| `package.json` | Tauri CLI 依赖与脚本 |

## 源码

| 路径 | 说明 |
|---|---|
| `src/index.html` | 28 个视图 |
| `src/app.js` | 前端逻辑（Tauri invoke） |
| `src/style.css` | 样式（switch 同款视觉） |
| `src/assets/logos/` | 10 个平台真实图标 |
| `src-tauri/Cargo.toml` | Rust 依赖 / release 配置（LTO 关闭） |
| `src-tauri/Cargo.lock` | 依赖锁文件（可复现构建，**必须提交**） |
| `src-tauri/tauri.conf.json` | 窗口 / 标识 / 前端目录 |
| `src-tauri/build.rs` | tauri-build 入口 |
| `src-tauri/app.manifest` | Common Controls v6 + DPI（GNU 构建用） |
| `src-tauri/resource.rc` | manifest + icon 资源（GNU 构建用） |
| `src-tauri/icons/` | 应用图标 |
| `src-tauri/src/main.rs` | 命令注册 / 托盘 |
| `src-tauri/src/model.rs` | 数据模型 |
| `src-tauri/src/state.rs` | 全局状态 |
| `src-tauri/src/store.rs` | 账号存储 / 导入导出 |
| `src-tauri/src/auth.rs` | 授权文件 / keyring |
| `src-tauri/src/gateway.rs` | 多上游 / 三协议 / 冷却 / 账本 |
| `src-tauri/src/panel.rs` | 面板实时接口 |
| `src-tauri/src/quota.rs` | 免费额度 |
| `src-tauri/src/switch.rs` | 会话 / Token / 积分 / 工具 |
| `src-tauri/src/signin.rs` | 原生签到 |
| `src-tauri/src/extras.rs` | 移动端 / 控制面 / 压缩 / ZCode |
| `src-tauri/src/config.rs` | 便携版 config 直读写 |
| `src-tauri/src/update.rs` | 自动更新 |
| `src-tauri/src/clients.rs` | CLI 导入 / 客户端导出 |
| `src-tauri/src/device.rs` | 设备指纹 |
| `src-tauri/src/hooks.rs` | 钩子 |

## 文档

| 文件 | 说明 |
|---|---|
| `docs/GETTING-STARTED.md` | **新手教程**（从零到能用） |
| `docs/USAGE.txt` | 便携版原始使用说明 |
| `docs/ARCHITECTURE.md` | 分层 / 模块 / 数据流 |
| `docs/DATA-MODEL.md` | 账号 / 状态 / 配置 / extras |
| `docs/COVERAGE.md` | 38 项能力 5 态说明 |
| `docs/PANORAMA.md` | 生态全景（37+ 项目） |
| `docs/REVERSE-REPORT.md` | switch/签到/REA 逆向报告 |
| `docs/SWITCH-COMMANDS.md` | switch 93 条命令表 |
| `docs/FUSION-MANIFEST.md` | 14 个仓库 → 能力映射 |
| `docs/FUSION-APP-README.md` | 旧版工程说明（历史） |
| `docs/BUILD-NOTES.md` | 构建排错 |
| `docs/MAINTENANCE.md` | 长期维护 / 发版 / 巡检 |
| `docs/ROADMAP.md` | 路线图 |
| `docs/FILE-INVENTORY.md` | 本文件 |

## 脚本 / 部署

| 文件 | 说明 |
|---|---|
| `scripts/build.ps1` | 一键构建（GNU/MSVC 通用，自动复制 DLL 和文档） |
| `scripts/preflight.ps1` | **发版前泄露自检**（密钥 / token / 个人路径 / 账号数据） |
| `scripts/mcp-server.mjs` | MCP stdio 服务端 |
| `deploy/cloudflare-worker.js` | 10 上游 Cloudflare Worker |
| `.github/workflows/release.yml` | tag 触发自动构建发布 |
| `.cargo/config.toml` | GNU 链接配置 |

## 产物

| 路径 | 说明 |
|---|---|
| `dist/axiao-gateway.exe` | 可直接运行（11.2MB） |
| `dist/WebView2Loader.dll` | 必须与 exe 同目录 |
| `dist/config.example.json` | 配置模板 |
| `dist/使用说明.txt` | 便携版说明 |
| `dist/新手教程.md` | 新手教程 |
| `release/阿晓网关助手-v0.1.0-win64.zip` | 发布包（4.6MB，直接发人） |
| `release/阿晓网关助手-v0.1.0-source.zip` | 源码包（0.5MB，71 个文件） |

> `release/` 已在 `.gitignore` 里排除。给别人的包用 `release/*-win64.zip`，
> 仓库里不放二进制，正式发布走 GitHub Release（`.github/workflows/release.yml`）。

## 第三方

| 路径 | 说明 |
|---|---|
| `third_party/WorkBuddy-Daily/` | 签到源码 + LICENSE |
| `schema/` | 统一账号 JSON Schema + 示例 |

## 参考资料 / 历史版本

| 路径 | 说明 |
|---|---|
| `reference/README.md` | 这些素材干嘛用的、哪些东西**没**放进来、怎么重新抓 |
| `reference/switch-assets/` | switch 的前端 JS/CSS/渲染后 DOM（界面与命令表反查） |
| `reference/panel-README.md` | 便携版内核原始说明（46 个配置字段的出处） |
| `reference/REA-SYNC.md` | 与 REA 同步的 `clientRules` 规则 |
| `legacy/README.md` | 历史版本说明 |
| `legacy/web-console/` | 早期 Node Web 控制台（已被 Tauri 版取代，留作参考） |

### 故意没有放进仓库的

| 东西 | 大小 | 原因 | 怎么拿回来 |
|---|---|---|---|
| `fusion/vendor/` 15 个内核仓库 | 386 MB | 第三方 clone，版权不属于本项目 | `scripts/fetch-vendor.ps1` |
| `fusion/tools/`（pyinstxtractor 等） | 15 MB | 公开工具 | 见 `docs/REVERSE-REPORT.md` |
| `wb2api.exe`（网关内核） | 8.7 MB | 第三方二进制，随便携版分发 | 从便携版包里拿 |
| `auths/`、`config.json`、`data/` | — | 含账号凭证 | 用户自己登录生成 |
