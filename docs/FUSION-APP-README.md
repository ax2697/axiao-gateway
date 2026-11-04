# 阿晓网关助手（Axiao Gateway Assistant）

> 定位：把 **WorkBuddy网关-便携版 + 网关生态全景与融合方案 + workbuddy-switch + WorkBuddy签到助手** 融合成**一个 Tauri 桌面应用**（Rust 后端 + WebView2 前端），不是 Web 网站、不是纯 CLI、不是启动菜单。

## 1. 形态

- 主程序：`axiao-gateway.exe`（Tauri v2，Rust 后端 + WebView2 前端）
- UI 底座：沿用 `workbuddy-switch` 的桌面壳形态（侧边栏 + 账号池 + 状态卡 + 表格）
- 单进程内完成：账号池、切换、签到、导入导出、网关、配置、日志

## 2. 四个来源如何"融成一个"

| 来源 | 吸收进本应用的部分 | 落地位置 |
|---|---|---|
| WorkBuddy网关-便携版（panel） | 账号池、积分/过期、任务、模型与档位、配置、运行日志 | 前端页面 + `gateway.rs` |
| 网关生态全景与融合方案（37+ 项目） | 多上游、三协议、配额感知切号、冷却状态机、成本账本、会话粘性、REA 规则 Schema | `gateway.rs` / `store.rs` |
| workbuddy-switch | Tauri UI+exe 底座、`.wb-switch` 账号协议、积分监控、导入导出 | `store.rs` / `auth.rs` |
| WorkBuddy签到助手（bworkbuddy） | **原生签到引擎**（直连 `copilot.tencent.com`，不启动 exe） | `signin.rs::native_*` |

> 说明：清单里 100+ 项目是**互相替代的内核**，不可能全部进程级并列。本应用采用「一个内核 + 能力并集」：把各项目**独有的能力模块**重新实现/移植进同一套 Rust 代码，而不是启动它们。

## 3. 覆盖点（对应你列的 1–9）

1. **Tauri 桌面应用**：`tauri.conf.json` + WebView2。
2. **自有主程序**：`src-tauri/src/main.rs`，不依赖 exe 外挂。
3. **定位**：阿晓网关助手，四源全覆盖。
4. **账号存储**：`~/.wb-switch/axiao/accounts.json`（统一 Schema），兼容读取 `~/.wb-switch/accounts.json`。
5. **凭证来源**：CodeBuddy 授权文件 `%LOCALAPPDATA%/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai` + 签到助手 token 文件。
6. **密钥保护**：Windows 凭据管理器（`keyring`）保存主密钥，敏感字段落盘前加密；授权文件写入前自动备份。
7. **钩子**：`~/.wb-switch/axiao/hook-events.jsonl`，支持注册/触发/查看。
8. **上游接口**：`copilot.tencent.com`、`/v2/billing/meter`、`daily-checkin`；多上游（WorkBuddy/CodeBuddy/Trae/ZCode）可配置。
9. **开源融合**：`src-tauri/src/gateway.rs` 内置多上游 + 三协议（Chat/Responses/Anthropic）+ 配额切号 + 冷却 + 成本账本 + 会话粘性。

## 4. 目录

```
fusion-app/
├─ package.json                 # Tauri CLI
├─ src/                         # 前端（WebView2）
│  ├─ index.html
│  ├─ app.js
│  └─ style.css
└─ src-tauri/
   ├─ Cargo.toml
   ├─ build.rs
   ├─ tauri.conf.json
   └─ src/
      ├─ main.rs                # 命令注册 / 状态
      ├─ model.rs               # 数据模型（Account/ClientRule/Upstream/Ledger/Hook）
      ├─ store.rs               # 账号存储 + 导入导出 + REA 规则对齐
      ├─ auth.rs                # 授权文件读写 + keyring 密钥保护
      ├─ hooks.rs               # 钩子事件
      ├─ gateway.rs             # 多上游 / 三协议 / 切号 / 冷却 / 账本
      └─ signin.rs              # 签到助手入口 + 组件状态
```

## 5. 构建（需要 Rust 工具链）

```bash
# 1) 安装 Rust（若未安装）
winget install Rustlang.Rustup
# 2) 安装 Tauri CLI
npm install
# 3) 开发运行
npm run dev
# 4) 打包 exe / 安装包
npm run build
```

产物：`src-tauri/target/release/axiao-gateway.exe`（及 `bundle/` 下的安装包）。

## 6. 数据与安全（已对齐真实文件）

| 数据 | 真实位置 | 字段 |
|---|---|---|
| 账号凭证 | `auths/workbuddy-<uid>.json` | `account{uid,nickname,enterpriseId}` + `auth{accessToken,refreshToken,expiresAt,realm,domain}` |
| 运行状态 | `data/state.json` | `credits`、`credits_total`、`disabled`、`last_checkin_day`、`credits_earliest_expiry`、`soft_streak`、`consecutive_fails` |
| 用量 | `data/usage.json` | `version`、`saved`、`buckets[]` |
| 上游/鉴权 | `config.json` | `api_key`、`listen`、`pool`、`schedule` |
| 请求日志 | `data/request-logs/` | 按天归档 |
| 应用账本/钩子 | `~/.wb-switch/axiao/` | `ledger.jsonl`、`hook-events.jsonl`、`accounts.json`（仅手动/导入账号） |
| 密钥保护 | Windows 凭据管理器 | service=`axiao-gateway` |

- 真实账号**只读**自 `auths/` + `data/state.json`，每次启动重新合并，不落应用自己的库。
- 切换前备份 `auth.workbuddy.workbuddy-ai`；禁用/启用写回 `data/state.json`（先备份 `.axiao-bak`）。
- 任务/模型不写死：实时抓 `GET /panel/api/accounts/{uid}/tasks`、`GET /panel/api/models`。
- 签到/成长/旅行/抽奖/保活：**原生 Rust 直连** `copilot.tencent.com`，接口取自 `fusion/vendor/L0NE-6_WorkBuddy-Daily`，不启动 `WorkBuddy签到助手.exe`。
- 切换：**原生**读写 `auths/workbuddy-<uid>.json` 与 CodeBuddy 授权文件，不启动 `wb-switch-rust.exe`。

## 7. 与旧 Node 壳的关系

`../app/` 是过渡期的 Node 版壳（可立即运行）；`fusion-app/` 是最终 Tauri 版，功能并集更全。安装 Rust 后即可 `npm run build` 出 exe。
