# 交接清单（HANDOVER）

> 项目：阿晓网关助手（Axiao Gateway Assistant）
> 版本：v0.1.0（源码 + exe 均已完成，本机已构建验证）
> 交接日期：2026-11-03

---

## 1. 一句话现状

**源码 + exe 都已完成。** `axiao-gateway.exe`（11.2MB）已编译并启动验证；发布包在 `release/`。

- 可直接运行：`dist\axiao-gateway.exe`（需同目录 `WebView2Loader.dll`）
- 发布包：`release\阿晓网关助手-v0.1.0-win64.zip`
- 后续维护：`docs/MAINTENANCE.md`；推 tag 由 `.github/workflows/release.yml` 自动构建

---

## 2. 已完成（可直接用）

| 模块 | 状态 | 说明 |
|---|---|---|
| 账号管理 | ✅ | 读便携版 `auths/*.json` + `data/state.json`；CN/Global 分段；批量导入/导出 |
| 添加账号 | ✅ | 浏览器登录（CN/Global，`login/start` + `login/poll`）、导入 JSON（`import/cockpit`） |
| 账号切换 | ✅ | 写回 CodeBuddy 授权文件，写前备份 |
| 关联会话 | ✅ | 读 `hook-events.jsonl`；会话组新建/删除 |
| Token 统计 | ✅ | 解析 transcript `/message/usage`；输入/输出/缓存读/命中率；按日 |
| 积分统计 | ✅ | `official_usage_cache.json` + `state.json` 到期 |
| 免费额度 | ✅ | WorkBuddy 真实积分；Trae/Z.AI/BigModel/OpenCode Zen/Kimi/MiniMax/CommandCode 卡片 |
| 签到中心 | ✅ | 原生 `native_checkin/growth/keepalive/refresh/status`；批量签到 |
| 任务中心 | ✅ | 实时抓 `/panel/api/accounts/{uid}/tasks`；一键完成 |
| 模型与档位 | ✅ | 实时抓 `/panel/api/models` |
| 上游管理 | ✅ | 8 个预设（WorkBuddy/Trae/Z.AI/BigModel/MiniMax/豆包/DSH 等）+ 自定义 |
| 协议与路由 | ✅ | 三协议 + 接入信息（Base URL / Key / 模型列表 / 复制 / 重置密钥） |
| 成本账本 | ✅ | `ledger.jsonl` |
| 定时任务 | ✅ | 表单 + 本地保存（未接调度器） |
| 请求/运行日志 | ✅ | 账本 + 钩子事件 |
| 网关配置 | ✅ | 直读直写便携版 `config.json`（46 字段，5 区块） |
| 上下文压缩 | ✅ | 网关请求链路真实生效（system + 最近 6 条，单条截断） |
| 控制面 | ✅ | Key/白名单持久化 + 网关请求链路强制执行 |
| 扩展能力 | ✅ | 移动端地址、控制面、CLI 检测、压缩、24 家官方 API、ZCode 队列、思考档位 |
| 自动更新 | ✅ | 16 个组件，GitHub Releases API + 代理 |
| 使用说明 | ✅ | 便携版说明 + 预置配置 + FAQ |
| 能力覆盖 | ✅ | 38 项 5 态核对表 |
| 平台 Logo | ✅ | 10 个真实图标（官方资源/favicon） |

---

## 3. 代码完成度（38 项：27 已实现 + 9 内核已实现 + 2 不适用）

**控制台侧已无「待实现」项。** 其余为外部配置或不适用：

| 项 | 代码状态 | 你需要做 |
|---|---|---|
| 设备指纹隔离 | ✅ 网关按账号注入 `x-device-id` / `x-machine-id` | 无 |
| 多上游三合一 | ✅ 16 家上游预设 + 权重路由 | 填各平台 Key |
| 控制面强制执行 | ✅ 网关校验 Key / 模型白名单 | 在「扩展能力」填白名单 |
| 极简 CLI 会话复用 | ✅ 检测 + 导入为账号 | 安装并登录 CodeBuddy CLI |
| 原生 Claude Code / Codex / DSH | ✅ 各端 config 导出器 | 点「客户端配置导出」 |
| 全家桶 7+ | ✅ 16 家预设 | 填 Key |
| 24 家官方 API + MCP | ✅ 目录 + `scripts/mcp-server.mjs` + CF Worker | `node scripts/mcp-server.mjs` / `wrangler deploy` |
| ZCode off-peak | ✅ 队列 + 每分钟 `zcode_tick` | 在扩展能力里加任务 |
| Docker Web 控制台 | ⚪ 不适用 | 桌面已内嵌 panel |
| Go 零依赖 + 热更新 | ⚪ 不适用（wb2go 内核属性） | 如需请切换内核 |
| WB+Trae+Qoder + 托盘 | ✅ 聚合 + Tauri tray + 角色转换 | 编译后自动生效 |
| Deno+CF 10 上游 | ✅ Worker 代码已提供 | `wrangler deploy` + 配 Key |

---

## 4. 构建（接手第一件事）

### 环境

- Windows 10/11 x64
- Node.js ≥ 20
- Rust：`rustup`（MSVC 推荐；GNU 需自带 gcc/ld）
- WebView2 Runtime（Win11 自带）

### 首次

```powershell
npm install
npm run dev        # 开发
# 推荐：一键构建（处理 GNU dlltool / ASCII 产物目录）
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
# 或手动：
npx tauri build --no-bundle
```

> GNU 工具链必须装完整 MinGW（WinLibs），Rust 自带的精简 dlltool 缺 `as` 会失败。
> 产物目录必须是纯 ASCII（如 `D:\axiao-build`），中文路径会让 dlltool 的 `CreateProcess` 失败。
> 详见 `docs/BUILD-NOTES.md`。

### 产物

```
src-tauri/target/release/axiao-gateway.exe
src-tauri/target/release/bundle/   # 安装包（如启用）
```

### 已知问题（本机实测）

1. **GNU 工具链 windres 会挂**：本机 `cc1.exe` 缺 `mpfr/mpc/isl/gmp` 等 DLL，
   `tauri-build` 调 `windres` 预处理 `.rc` 时报 `preprocessing failed`。
   本机走的是「跳过 windres + 手动链 `resource.o`」的兜底路线，具体步骤见
   `docs/BUILD-NOTES.md` 坑 5。**换到 MSVC 或装齐运行库的 MinGW 上，这条路不需要走。**
2. **manifest 必须嵌**：跳过 windres 后若不链 `resource.o`，exe 会弹
   `无法定位程序输入点 TaskDialogIndirect`。见坑 6。
3. **中文路径**：GNU binutils 生成 import lib 时无法处理中文路径，`CARGO_TARGET_DIR`
   必须指向纯 ASCII（默认 `D:\axiao-build`）。
4. **不要开 LTO**：`lto=true + codegen-units=1` 会让 rustc 空转 20 分钟。见坑 8。
5. **中文 productName**：exe 文件名由 Cargo 决定（`axiao-gateway.exe`），中文只影响
   窗口标题/安装包。

---

## 5. 数据与路径

| 数据 | 路径 | 说明 |
|---|---|---|
| 便携版根 | `D:\...\WorkBuddy网关-便携版` | 可用 `WB_GATEWAY_HOME` 覆盖 |
| 账号凭证 | `<根>/auths/workbuddy-<uid>.json` | 明文 token，**禁止提交/外发** |
| 运行状态 | `<根>/data/state.json` | credits / disabled / 签到日 |
| 网关配置 | `<根>/config.json` | `api_key` / `listen` / `schedule` / `pool` |
| 应用数据 | `~/.wb-switch/axiao/` | `accounts.json` / `extras.json` / `ledger.jsonl` / `hook-events.jsonl` / `update.json` |
| 密钥 | Windows 凭据管理器 | service=`axiao-gateway` |

---

## 6. 关键接口（已实测）

### panel（7863）

- `GET /panel/api/packages` → 账号 + 积分
- `GET /panel/api/accounts/{uid}/tasks` → 任务
- `POST /panel/api/accounts/{uid}/tasks/auto_all` → 一键完成
- `GET /panel/api/models` → 模型
- `GET/POST /panel/api/config` → 配置
- `POST /panel/api/login/start` → `{realm}` → `{state,url}`
- `GET /panel/api/login/poll?state=` → 登录结果
- `POST /panel/api/import/cockpit` → 导入 JSON（multipart）

### 上游（签到）

- `POST https://copilot.tencent.com/v2/billing/meter/checkin-activity-status`
- `POST https://copilot.tencent.com/v2/billing/meter/daily-checkin`
- `GET /v2/activity/growth/tasks`、`POST /v2/activity/growth/tasks/accept`
- `POST /activity/growth/tasks/{code}/claim`
- `GET/POST /v2/activity/growth/buddy/travel/{status,config,depart,claim}`
- `GET /v2/activity/growth/lottery/chances`、`POST .../draw`
- `POST /v2/report`（活跃保活）、`POST /v2/plugin/auth/token/refresh`

### 免费额度

- Trae：`https://api.trae.cn/trae/api/v2/ug/checkin_credits/status`
- Z.AI：`https://api.z.ai/api/monitor/usage/quota/limit`
- BigModel：`https://open.bigmodel.cn/api/monitor/usage/quota/limit`
- OpenCode Zen：`https://opencode.ai/zen/go/v1/usage`

---

## 7. 开源发布流程

1. `git init` + 首次提交（`.gitignore` 已排除 `auths/`、`config.json`、`target/`、
   `node_modules/`、`dist/*`、`release/`、`*.exe`）。
2. GitHub 建仓 `axiao-gateway`。
3. 打 tag `v0.1.0` 并推送 → `.github/workflows/release.yml` 自动在 `windows-latest`
   上构建并把 `axiao-gateway.exe`、`WebView2Loader.dll`、`config.example.json`、
   `使用说明.txt`、`新手教程.md` 挂到 GitHub Release。
4. 自动更新：把 `src-tauri/src/update.rs` 里的 `repo` 改成新仓地址；发版后
   应用内「自动更新」即可检查。
5. 如需签名更新：配置 Tauri updater 公私钥 + `latest.json`。
6. 本地已有构建好的包，也可以手动 `gh release create v0.1.0 release/*.zip --notes-from-tag`
   直接上传（不依赖 CI）。

---

## 8. 接手 checklist

- [x] 装完 Rust（`rustc --version` 有输出）
- [x] `npm install` 成功
- [x] `npm run dev` 能起窗口
- [x] 启动便携版网关，账号池有账号
- [x] 账号管理能读到账号与积分
- [x] 任务/模型能实时抓取
- [x] 签到中心能执行一次签到
- [x] `npx tauri build --no-bundle` 出 exe
- [x] `scripts/preflight.ps1` 返回 0（密钥 / 个人路径 / 账号数据 全部干净）
- [ ] 按第 7 节发首个 Release
- [ ] 把 `docs/ROADMAP.md` 里 v0.2 的 8 项排期

---

## 9. 风险与合规

- 全部为社区逆向实现，可能违反上游服务条款，存在限流/封禁风险。
- `auths/` 是明文凭证，仓库/压缩包绝不能包含。
- 平台 Logo 仅用于识别，商标归各公司；如收到通知需替换为文字。
- 不要提供“收费版/卡密版/公益中转”，避免被套壳窃取凭证。

---

## 10. 交付物与校验

| 交付物 | 路径 | 大小 | SHA256 前 16 位 |
|---|---|---|---|
| 可直接运行的 exe | `dist\axiao-gateway.exe` | 11,585,024 B | `93e89f9d61cafec4` |
| 必须同目录的 DLL | `dist\WebView2Loader.dll` | 160,880 B | `86545b66cdb0603b` |
| **发人用的包** | `release\阿晓网关助手-v0.1.0-win64.zip` | 4,898,299 B | `0a6abedf1a85b80f` |
| 源码包 | `release\阿晓网关助手-v0.1.0-source.zip` | 1.01 MB / 90 文件 | 见 `SHA256SUMS.txt` |
| 完整校验和 | `release\SHA256SUMS.txt` | — | — |

> 源码包的哈希不写进本文件（它就在源码包里，写死会自相矛盾），以
> `release\SHA256SUMS.txt` 为准。

### 包里到底有没有「我的东西」

| 项目 | 在包里吗 | 说明 |
|---|---|---|
| 你的账号 / token | ❌ 没有 | 账号是应用**运行时**从本机便携版 `auths/` 读的，不进包 |
| 便携版 `config.json`（含账号池配置） | ❌ 没有 | 包里只有 `config.example.json`，`api_key` 是 `sk-REPLACE_ME` |
| 便携版出厂 `api_key` | ⚠️ 文档里有 | `sk-QxgVEFVZM6Pg8Z6MfeiCA6TS` 是便携包**出厂预置值**（登录面板用），不是个人凭证；所有便携包共用，对外监听前必须换掉 |
| 应用数据 `accounts.json` / `ledger.jsonl` / `hook-events.jsonl` | ❌ 没有 | 在 `%USERPROFILE%\.wb-switch\axiao\`，删掉即清空 |
| 本机绝对路径 | ❌ 没有 | 已改为「exe 同级 / 上级 / 桌面」自动探测 + `WB_GATEWAY_HOME` 覆盖 |

**「我只装了 2 个号，打开却有 4 个」的原因**：不是包带进去的，是打包这台机器的
`%USERPROFILE%\.wb-switch\axiao\accounts.json` 里留了测试账号，和新装的应用共用同一个
用户目录，于是「便携版 2 个真实号 + 应用数据里的测试号」一起显示。
清空该目录（或删掉 `accounts.json`）重启就恢复成 2 个。
`scripts/preflight.ps1` 已经把这步做成检查项。

`win64.zip` 内容：`axiao-gateway.exe`、`WebView2Loader.dll`、`config.example.json`、
`使用说明.txt`、`新手教程.md`、`README.md`。解压双击即用，无需装 Rust / Node。

校验：

```powershell
Get-FileHash .\阿晓网关助手-v0.1.0-win64.zip -Algorithm SHA256
```

### 发给别人时怎么说

> 解压后先双击 `axiao-gateway.exe`。第一次打开请在「设置 → 便携版目录」指向你本机的
> `WorkBuddy网关-便携版` 目录，账号会自动读出来。详细步骤看 `新手教程.md`。

### 迁移说明

本目录（`开源\阿晓网关助手`）是**唯一权威副本**。原先的
`Desktop\WorkBuddy网关-便携版` 只保留了便携版本体运行所需的文件，源码与文档以本目录为准。

