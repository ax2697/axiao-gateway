<div align="center">

<img src="assets/banner.svg?v=1" alt="阿晓网关助手 · 统一网关控制台 / Axiao Gateway Assistant · Unified Gateway Console" width="1200" />

# 🛰️ 阿晓网关助手 · 统一网关控制台

### Axiao Gateway Assistant · Unified Gateway Console

**v0.9.5** · 中文 / English 双语

> 一个窗口，管完几十个网关。
> 账号、签到、积分、额度、Token、成本、上游、协议、日志、更新 —— 全在一个 Tauri 桌面应用里。
>
> *One window for dozens of AI gateways.*
> *Accounts, check-in, credits, quotas, tokens, cost, upstreams, protocols, logs and updates — in one Tauri desktop app.*

![Version](https://img.shields.io/badge/version-v0.9.5-6a5acd?style=flat-square)
![Views](https://img.shields.io/badge/views-31-2ec4b6?style=flat-square)
![Commands](https://img.shields.io/badge/commands-102-00b4d8?style=flat-square)
![Upstreams](https://img.shields.io/badge/upstreams-26-8e44ad?style=flat-square)
![Protocols](https://img.shields.io/badge/protocols-3%20%7C%20OpenAI%20%7C%20Anthropic-9d4edd?style=flat-square)
![Stack](https://img.shields.io/badge/stack-Tauri%20v2%20%7C%20Rust%20%7C%20WebView2-ff9f43?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-ffca3a?style=flat-square)
![Stars](https://img.shields.io/github/stars/ax2697/axiao-gateway?style=social)

[这是什么](#-这是在解决什么问题) · [统一在哪](#️-统一在哪) · [⚠️ 安全提醒](#️-安全提醒必读) · [核心能力](#-核心能力) · [快速开始](#-快速开始) · [目录结构](#-目录结构) · [赞助](#-赞助与合作) · [更新记录](#-更新记录) · [English](#english)

</div>

---

## 🎯 这是在解决什么问题

用 AI 编程的人，多半都装过不止一个「AI 网关」。然后就乱了。

### 坑一：每家一个客户端、一套配置、一份文档

WorkBuddy 一个网关，Trae 一个网关，CodeArts 又一个。想让 Claude Code 和 Cline 共用一个入口，得自己拼三四个项目。

### 坑二：协议不统一，换一次客户端重配一遍

OpenAI Chat / OpenAI Responses / Anthropic Messages 三种混用。想同时接 Claude Code、Codex、DSH、Cherry Studio，得分别适配。

### 坑三：账号池、签到、额度、统计，各写一遍

每个网关项目都自己实现一遍多账号轮转、Token 续期、熔断冷却、签到、额度抓取、用量统计 —— 重复造轮子，且各家的统计口径还不一样。

**网关不是缺项目，是缺「统一」。**

> 完整生态调研见 [`docs/PANORAMA.md`](docs/PANORAMA.md)：40+ 个开源网关项目去重对比，20 条独立技术路线，6 个真实差异维度（上游覆盖 / 协议 / 登录态来源 / 调度策略 / 管理平面 / 部署形态）。

---

## 🛰️ 统一在哪

阿晓网关助手**不是又一个网关内核**，而是一层统一控制面：

| 维度 | 统一前 | 统一后 |
|:---|:---|:---|
| **上游** | 一个网关 = 一家平台 | 26 家预设（WorkBuddy / Trae / Z.AI / BigModel / MiniMax / 豆包 / 混元 / 硅基流动 / Kimi / DSH / OpenCode Zen …）+ 自定义，全部收进一个 `/v1` |
| **协议** | 三种混用，换客户端重配 | OpenAI Chat / OpenAI Responses / Anthropic Messages 三协议并存，Claude Code / Codex / DSH / Cline / Cherry Studio 直接接 |
| **账号** | 各家账号各管各的 | 一处管理账号、积分、到期、签到状态，CN / Global 分段，批量导入导出 |
| **观测** | 每家统计口径不同 | 统一的 Token（含缓存命中率）/ 积分 / 成本账本 / 请求日志 / 钩子事件 |
| **控制面** | 没有或各写一套 | Key 分发、模型白名单、冷却熔断、上下文压缩、设备指纹隔离 |
| **运维** | 每个内核单独更新 | 16 个组件统一检查更新，GitHub 不通时可配代理 |

**规模**：31 视图 · 102 个 Tauri 命令 · 36 个 Rust 模块 · 26 家上游预设 · 3 种协议 · 59 项能力核对（48 已实现 + 9 内核已实现 + 2 不适用）。

> 哪些是本项目自有代码、哪些由便携版内核执行，见 [`docs/COVERAGE.md`](docs/COVERAGE.md)。不假装全是自己写的。

---

## ⚠️ 安全提醒（必读）

文档与便携包里出现的 `sk-QxgVEFVZM6Pg8Z6MfeiCA6TS` 是**便携版出厂预置的 `api_key`**，**所有便携包共用同一个值**。

| | 场景 | 是否安全 |
|:---|:---|:---:|
| ✅ | 只在本机 `127.0.0.1` 用 | 安全 |
| ❌ | 改成 `0.0.0.0` 对外监听 | **任何人拿这份文档就能用你的网关** |

对外监听前必须先换成你自己的随机串：记事本打开便携版 `config.json`，改 `api_key` 的值（热生效，不用重启；已填旧 Key 的客户端要同步改）。

其它红线：

- `auths/` 是**明文 accessToken / refreshToken** —— 拿到目录等于拿到账号，**永不入库、永不外发**。
- 应用数据在 `%USERPROFILE%\.wb-switch\axiao\`；换机前清掉，否则会看到「多出来的号」。
- 本项目是**社区逆向实现**，可能违反上游服务条款，请仅限本人授权账号、本机自用。

详见 [`SECURITY.md`](SECURITY.md) 与 [`docs/MAINTENANCE.md`](docs/MAINTENANCE.md) §0。

---

## 🚀 核心能力

### 账号 · 签到 · 额度

- **账号管理**：统一管理 WorkBuddy / CodeBuddy（IDE / CLI / VS Code / JetBrains）/ Trae / Qoder / ZCode / MiniMax / 豆包 / CodeArts / DSH / OpenCode Zen 的账号、积分、到期与签到状态，CN / Global 分段
- **添加账号**：① 浏览器登录（`login/start` + `login/poll`）② 其他平台 API Key 直连（26 家预设 + 自定义，自动带出 Base URL）③ 导入 cockpit JSON
- **签到中心**：原生直连上游 —— 签到 / 成长任务 / Buddy 旅行 / 抽奖 / 活跃保活 / 批量签到，**不启动外部 exe**
- **任务中心**：实时抓成长任务队列，一键完成 / 批量领取
- **免费额度**：WorkBuddy 积分 + Trae / Z.AI / BigModel / OpenCode Zen / Kimi / MiniMax / CommandCode 额度聚合

### 网关 · 协议 · 上游

- **三协议并存**：OpenAI Chat Completions / OpenAI Responses / Anthropic Messages
- **26 家上游预设 + 自定义**：权重路由、冷却熔断、会话粘性
- **控制面**：Key 与模型白名单，网关侧强制执行
- **上下文压缩**：system + 最近 N 条，单条截断，请求链路真实生效
- **设备指纹隔离**：按账号注入 `x-device-id` / `x-machine-id`
- **客户端配置导出**：Claude Code / Codex / DSH / 通用 JSON 一键写入

### 观测 · 运维

- **Token 统计**：按客户端与会话统计输入 / 输出 / 缓存读 / 命中率，按日
- **成本账本**：每次网关调用写入 `ledger.jsonl`，可在应用内查
- **积分统计与到期提醒**
- **请求 / 运行日志 + 钩子事件**
- **自动更新**：16 个组件检查 GitHub Releases，不通可配代理
- **MCP 服务端**：`scripts/mcp-server.mjs`（stdio，无依赖）；另有 10 上游 Cloudflare Worker
- **托盘常驻** + 角色统一（developer → system）

---

## 🛠️ 快速开始

### 前置

- Windows 10/11 64 位 · Node.js ≥ 20 · Rust（推荐 MSVC）· WebView2 Runtime（Win11 自带）

### 开发

```bash
npm install
npm run dev
```

### 打包

```bash
npm run build          # 产物：src-tauri/target/release/axiao-gateway.exe
npx tauri build --no-bundle   # 只出 exe，不打安装包
```

> GNU 工具链一键构建：`powershell -ExecutionPolicy Bypass -File scripts/build.ps1`
> 中文路径 / dlltool / windres 报错见 [构建排错](docs/BUILD-NOTES.md)。

### 运行前提

1. 便携版网关已启动，监听 `127.0.0.1:7863`
2. `auths/` 里至少有一个已授权账号
3. 客户端 Base URL：`http://127.0.0.1:7863/v1` · 密钥：便携版 `config.json` 的 `api_key`

### 发版前必跑

```bash
powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1   # 泄露自检，退出码非 0 禁止打包
node --check src/app.js                                          # 前端语法自检
```

---

## 📁 目录结构

```
阿晓网关助手/
├─ src/                     前端（31 个视图）
│  ├─ index.html  app.js  style.css
│  └─ assets/logos/         22 个平台真实图标
├─ src-tauri/               Rust 后端（36 个模块 / 102 个命令）
│  ├─ Cargo.toml  tauri.conf.json  Cargo.lock
│  └─ src/
│     ├─ main.rs            命令注册
│     ├─ store.rs           账号存储 / 导入导出
│     ├─ auth.rs            授权文件 / keyring
│     ├─ gateway.rs         多上游 / 三协议 / 冷却 / 粘性 / 账本
│     ├─ panel.rs           面板实时接口（任务/模型/登录/配置）
│     ├─ quota.rs           免费额度
│     ├─ switch.rs          会话 / Token / 积分 / 工具检测
│     ├─ signin.rs          原生签到 / 成长 / 保活 / 刷新
│     ├─ extras.rs          移动端 / 控制面 / 压缩 / ZCode
│     ├─ update.rs          自动更新
│     ├─ config.rs          便携版 config.json 直读直写
│     ├─ clients.rs         CLI 导入 / 客户端配置导出
│     ├─ device.rs  hooks.rs  model.rs  state.rs
├─ schema/                  账号 JSON Schema（REA 规则）
├─ docs/                    14 份文档（含生态全景 / 能力覆盖 / 逆向报告）
├─ scripts/                 构建 / 泄露自检 / MCP 服务端
├─ reference/               参考素材（switch 前端产物、panel README）
├─ third_party/             第三方源码（含 LICENSE）
├─ deploy/                  Cloudflare Worker（10 上游）
├─ assets/                  banner 等展示资源
├─ AGENTS.md                AI 工作约定 + 版本规则
├─ 开发状态.md               唯一真相源（版本 / 验收回执 / 台账）
└─ dist/                    构建产物（不提交）
```

---

## 📄 文档

- **新人先看**：[新手教程（从零到能用）](docs/GETTING-STARTED.md) · [便携版使用说明](docs/USAGE.txt)
- **为什么做统一**：[生态全景 PANORAMA](docs/PANORAMA.md) · [融合清单](docs/FUSION-MANIFEST.md)
- **技术**：[架构](docs/ARCHITECTURE.md) · [数据模型](docs/DATA-MODEL.md) · [能力覆盖 59 项](docs/COVERAGE.md) · [switch 命令表](docs/SWITCH-COMMANDS.md)
- **来源**：[逆向报告](docs/REVERSE-REPORT.md) · [第三方声明](THIRD_PARTY_NOTICES.md)
- **维护**：[维护指南](docs/MAINTENANCE.md) · [构建排错](docs/BUILD-NOTES.md) · [路线图](docs/ROADMAP.md)
- **真相源**：[开发状态.md](开发状态.md) · [AGENTS.md](AGENTS.md)

---

## 💖 赞助与合作

本项目持续维护中。如果你的产品能帮到多平台 AI 编程用户，欢迎联系。

| 档位 | 适合 | 权益 |
|:---:|:---|:---|
| 🥇 主赞助 | 模型厂商、GPU / API 平台 | 顶部 banner + 发布说明署名 |
| 🥈 合作方 | 工具、课程、发行渠道 | 表格位 + 链接 |
| 🥉 支持者 | 个人开发者 | 赞助者列表留名 |

联系方式：微信 **17344066178** · GitHub [@ax2697](https://github.com/ax2697)

---

## 📜 更新记录

完整更新记录见 [`CHANGELOG.md`](CHANGELOG.md)（v0.1.0 → v0.9.5 逐版本列出新增 / 修复 / 验收）。

### v0.9.5 — 2026-11-04 · 统一网关控制台首发（开源）

**规模**：31 视图 · 102 Tauri 命令 · 36 个 Rust 模块 · 26 家上游预设 · 3 种协议 · 22 个平台图标。

**新增**

- Tauri v2 桌面壳：31 个视图 / 102 个命令 / 36 个 Rust 模块
- 账号管理（CN/Global 分段、批量导入导出）、添加账号（浏览器登录 / API Key 直连 / 导入 JSON）
- 签到中心（原生签到 / 成长 / 旅行 / 抽奖 / 保活 / 批量）、任务中心、模型与档位
- Token 统计（含缓存命中率）、积分统计与到期、免费额度聚合、成本账本
- 26 家上游预设 + 自定义、三协议并存、控制面、上下文压缩、设备指纹隔离
- 客户端配置导出、MCP 服务端脚本、Cloudflare Worker、自动更新（16 组件）、托盘常驻
- 上游增删改、对话测试（真实走一次网关并写账本）、账本空态解释

**修复**

- 便携版目录改自动探测（原为硬编码本机路径）
- transcript 的 Unix 时间戳解析（此前整份 usage 被丢弃）
- 积分统计恒为 0
- `update.rs` 仓库指向修正（`changexcb` → `changexbc`；self 补 `ax2697/axiao-gateway`）

**验收**

- `scripts/preflight.ps1` 退出码 0 · `node --check src/app.js` 退出码 0
- 覆盖表 59 条（48+9+2）逐项核对一致
- 三个上游来源仓协议经 GitHub API 核对均为 MIT
- v0.1.0 首发记录见 [`CHANGELOG.md`](CHANGELOG.md)

---

## English

### What is this

**Axiāo Gateway Assistant** is a unified console for AI coding gateways — a Tauri v2 desktop app (Rust backend + WebView2 frontend).

It is **not** yet another gateway core. It is the missing unified layer on top of dozens of them:

| Layer | What it unifies |
|:---|:---|
| **Upstreams** | 16 presets (WorkBuddy / Trae / Z.AI / BigModel / MiniMax / Doubao / Hunyuan / SiliconFlow / Kimi / DSH / OpenCode Zen …) + custom, all behind one `/v1` |
| **Protocols** | OpenAI Chat Completions · OpenAI Responses · Anthropic Messages — Claude Code / Codex / DSH / Cline / Cherry Studio connect directly |
| **Accounts** | One place for accounts, credits, expiry and check-in state across platforms |
| **Observability** | Unified token usage (incl. cache hit rate), credit, cost ledger, request logs, hook events |
| **Control plane** | Key distribution, model allow-list, cooldown/breaker, context compression, device fingerprint isolation |
| **Operations** | 16 components checked for updates, proxy support, tray resident |

**Scale**: 31 views · 102 Tauri commands · 36 Rust modules · 26 upstream presets · 59 capability items audited (48 implemented + 9 kernel-provided + 2 not applicable).

### Security

The `api_key` shown in the docs (`sk-Qxg…`) is the **factory default shipped in every portable bundle** — the same value for everyone. It is safe on `127.0.0.1` only. **Replace it with your own random string before listening on `0.0.0.0`.**

`auths/` holds plaintext tokens. It never enters the repository, never enters a release package. See [SECURITY.md](SECURITY.md).

### Quick start

```bash
npm install
npm run dev
npx tauri build --no-bundle    # → src-tauri/target/release/axiao-gateway.exe
```

Pre-flight leak check (must exit 0 before packaging):

```bash
powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1
```

### Sponsor

Actively maintained. Get in touch if your product helps multi-platform AI coding users.

| Tier | Best for | Visibility |
|:---:|:---|:---|
| 🥇 Main sponsor | Model providers, GPU / API platforms | Top banner + release notes credit |
| 🥈 Partner | Tools, courses, distribution | Table slot + link |
| 🥉 Supporter | Individual developers | Name in the sponsor list |

Contact: WeChat **17344066178** · GitHub [@ax2697](https://github.com/ax2697)

---

## 📄 开源协议

- 本项目代码：[MIT](LICENSE)
- 第三方源码 / 图标 / 参考产物：遵循各自协议（见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)）——
  `third_party/WorkBuddy-Daily`（MIT）、`reference/switch-assets`（来自 `changexbc/workbuddy-switch`，MIT）均保留原始版权声明
- 平台名称与 Logo 归各自公司所有，仅用于识别，不代表官方合作；收到通知即替换为文字标签

---

## 网关能力一览（0.9.0）

一句话：**把几十家上游合成一个 OpenAI 兼容接口**，带协议互转、失败降级、
缓存、限流。本机一条命令就能跑，单 exe 免安装。

### 对着这张表找你需要的

| 你的情况 | 它怎么处理 |
|---|---|
| 客户端只认 OpenAI，上游是 Claude / Gemini | 「协议与路由」里把上游协议设成 Anthropic / Gemini，**请求和响应都自动互转**（含工具调用链路） |
| 火山方舟那类模型名对不上 | 模型映射：`gpt-4o=ep-20250101-abc123`，一行一条 |
| 一家渠道挂了，整个请求失败 | 自动换下一个渠道。**只对 429 / 5xx / 连接失败换**，参数错误直接如实返回 |
| 长文档报「超出上下文」 | 换窗口更大的渠道；装不下的**提前跳过不发送**（省一次往返和输入费） |
| 同一个问题反复问，重复花钱 | 响应缓存，10 分钟内直接复用。温度>0 / 带工具 / 流式**不缓存** |
| 想把网关给朋友用，怕他刷爆额度 | 生成访问 Key，设「每日次数」和「每分钟次数」，随时停掉 |
| 某一个渠道时好时坏 | 选路策略切「按健康度」，按最近成功率排序 |
| 用户说「刚才那次失败了」 | 每个请求有 `req_xxxxxxxx`，响应头、日志、账本三处能对上 |
| 客户端报错但看不出原因 | 「运行日志 → 网关服务日志」：请求、选了哪个上游、跳过了谁、流式多少帧 |

### 典型场景

**一个人用**：装好 → 上游管理填 Key → 客户端 base_url 指到 `127.0.0.1:7864/v1`。
不用设访问 Key（默认全开），不用管限流。

**分享给同事**：生成访问 Key → 设每日/每分钟上限 → 复制完整 Key 发过去 →
「运行日志」看他用得多不多 → 不给了就停用。

**接官方 Claude**：上游协议选 Anthropic，base 填 `https://api.anthropic.com`，
Key 填 Anthropic 的。客户端仍然按 OpenAI 格式调，工具调用也会正确转换。

### 明确不做

语义缓存（要 embedding 模型）、语义路由、SSO/多租户、MCP 网关、提示词版本化。
都需要额外运行时或属于团队产品能力，跟「单 exe、给个人和小团队用」冲突。
判断理由写在 [`docs/GATEWAY-COMPARISON.md`](docs/GATEWAY-COMPARISON.md)。


**免责声明**：本项目不提供任何账号、额度或破解能力；所有凭证来自用户本机已登录的客户端。使用产生的账号风险由使用者自行承担。

---

<div align="center">

**[MIT License](LICENSE)** · 用得好就点个 ⭐ · Star it if it helps

<sub>阿晓网关助手 · 让几十个网关，收进一个窗口</sub>

</div>
