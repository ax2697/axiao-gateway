# workbuddy-switch.zip 逆向报告 + 与 REA / 签到助手 的融合契约

> 手段说明：REA 是 MCP 逆向工具，但本会话**未加载其 MCP 工具**，因此本次为**静态逆向**（PE 结构 + 字符串/存储/字段抽取），并以 REA 源码中的 `SetupClient` 规则作为"必须同步的规则 Json"。
> 逆向对象：`workbuddy-switch.zip`（`wb-switch-rust.exe` 27.5MB / `agent-studio-runtime.exe` 4.4MB / `uninstall.exe`）
> 日期：2026-11-03

---

## 1. 结论先行

| 项 | 结论 |
|---|---|
| 形态 | **Tauri 桌面应用**（Rust 后端 + WebView2 前端），非 Web 网站、非纯 CLI |
| 主程序 | `wb-switch-rust.exe`（Rust，27.5MB，内含前端资源） |
| 定位 | WorkBuddy / CodeBuddy / Trae **多账号切换器** + 积分过期监控 + 自动签到 + Token 用量统计 |
| 账号存储 | `%WB_SWITCH_HOME%`（默认 `~/.wb-switch`）下 `accounts.json` / `config.json` / `settings.json` |
| 凭证来源 | 直接读写 CodeBuddy 授权文件：`AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai` |
| 密钥保护 | `options/secret-storage.xml`（Windows 上通常 DPAPI）+ `xml.wb-switch-bak` 备份 |
| 钩子 | `hook.sh` → `~/.wb-switch/hook-events.jsonl`，`hook-backups/` |
| 上游接口 | `copilot.tencent.com`；`/v2/billing/meter`、`/v2/billing/meter/get`、`daily-checkin` |
| 结论 | **它就是"融合产品的 UI+exe 底座"**：Tauri 壳 + 账号池 + 积分/签到，正好承接全景清单里的账号池/签到/监控能力 |

---

## 2. 证据链（静态抽取）

### 2.1 框架
- `tauri`(195) / `Tauri`(57) / `tauri_runtime_wry`(14) / `webview2`(11) / `TAURI_INTERNALS__`(52) / `tauri_plugin_updater`(6)
- → 确认 Tauri v2 + WebView2 + 内置更新器。

### 2.2 中文 UI 功能词（出现次数）
`账号`153 · `登录`50 · `切换`34 · `签到`17 · `刷新`15 · `积分`9 · `设置`8 · `过期`6 · `导出`5 · `导入`4 · `面板`1 · `保活`1 · `设备`1

> 直接印证：账号管理、登录/切号、签到、积分与过期监控、导入/导出、设置面板。

### 2.3 账号数据字段（serde 字符串）
`accountId` · `account_ids` · `accessToken` / `access_token` · `refreshToken` / `refresh_token` · `expiresAt` / `expires_at` · `credits` · `credit_usage` · `credit_statistics` · `credit_expiry`

### 2.4 存储路径
```
WB_SWITCH_HOME            → 默认 ~/.wb-switch
  accounts.json           → 账号列表
  config.json             → 配置
  settings.json           → 设置
  hook-events.jsonl       → 钩子事件
  hook-backups/           → 钩子备份
  options/secret-storage.xml
  xml.wb-switch-bak
AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai  → 上游授权
```

### 2.5 接口与文案
- `copilot.tencent.com`
- `/v2/billing/meter`、`/v2/billing/meter/get`、`daily-checkin`
- 文案：`未读取到本地 WorkBuddy 登录信息`、`eds_relogin_reason`、`cnai`

---

## 3. REA 的「规则 Json」（必须同步的契约）

REA（`io.github.morluto/rea` v6.8.0）在 `src/application/SupportedClients.ts` 中把每个客户端定义为 `SetupClient`：

```ts
interface SetupClient {
  name: string;
  displayName?: string;
  configPath: string;
  configPaths?: readonly string[];
  markerPath?: string;
  skillPath?: string | readonly string[];
  discoveryPaths?: readonly string[];
  format?: "json" | "claude_code" | "toml" | "vscode" | ... | "dsh" | "unsupported";
  sharedRegistration?: boolean;
  quitBeforeSetup?: boolean;
  zcodeFallbackPath?: string;
  msixPackage?: {...};
  geminiSettings?: {...};
}
```

其内置的 WorkBuddy 规则为：

```json
{
  "name": "workbuddy",
  "displayName": "WorkBuddy",
  "skillPath": [".workbuddy", "skills"],
  "configPath": [".workbuddy", "mcp.json"],
  "markerPath": [".workbuddy"],
  "format": "json"
}
```

> **这就是"规则的 Json"**：融合产品的账号导入/导出，必须能无损映射到这套 `name/configPath/markerPath/skillPath/format` 结构，否则无法与 REA 生态同步。详见 `schema/workbuddy-account.schema.json`。

---

## 4. 三方融合契约

### 4.1 角色分工

| 来源 | 在融合产品中的角色 |
|---|---|
| `workbuddy-switch.zip` | **UI + exe 底座**（Tauri 壳、账号列表页、切换/监控/签到页、导入导出入口） |
| `网关生态全景与融合方案.md` | **功能清单**（37 个 WB 网关 + 9 通用 + Trae 互通维度，取 delta 决定要不要内置哪些能力） |
| `WorkBuddy签到助手.exe`（bworkbuddy / WorkBuddy-Daily） | **账号导入导出 + 签到引擎**（`wb_refresh_tokens.json` / `WORKBUDDY_ACCESS_TOKEN.txt` / `wb_login_result.json`） |
| REA `SetupClient` | **规则 Json 标准**（账号/客户端的路径与格式描述层） |

### 4.2 统一账号模型（导出结构概览）

```json
{
  "schemaVersion": "1.0.0",
  "exportedAt": "2026-11-03T19:45:00+08:00",
  "clientRules": [
    {
      "name": "workbuddy",
      "displayName": "WorkBuddy",
      "configPath": [".workbuddy", "mcp.json"],
      "markerPath": [".workbuddy"],
      "skillPath": [".workbuddy", "skills"],
      "format": "json",
      "authFile": "AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai"
    }
  ],
  "accounts": [
    {
      "id": "acc_01",
      "client": "workbuddy",
      "displayName": "主号",
      "accountId": "wb_xxx",
      "accessToken": "…",
      "refreshToken": "…",
      "expiresAt": "2026-12-01T00:00:00+08:00",
      "credits": 3000,
      "creditUsage": 1200,
      "creditExpiry": "2026-12-31",
      "enabled": true,
      "lastUsedAt": "2026-11-03T09:00:00+08:00",
      "source": "switch"
    }
  ]
}
```

完整约束见 `schema/workbuddy-account.schema.json`。

### 4.3 同步规则（不可违背）

1. `clientRules[]` 的字段名与语义必须与 REA `SetupClient` 一致（`name/configPath/configPaths/markerPath/skillPath/format/discoveryPaths/sharedRegistration/quitBeforeSetup`）。
2. `accounts[].client` 必须引用 `clientRules[].name`（外键约束）。
3. 导入时：若目标机已有 `~/.wb-switch/accounts.json`，按 `accountId` 去重合并；`accessToken/refreshToken` 以导入包为准（可配置"保留本地"）。
4. 导出时：默认**剔除** `accessToken/refreshToken` 明文（只导出 `refreshToken` 引用或加密串），除非用户显式勾选"含凭证"。
5. 签到助手的三类凭据文件（`wb_refresh_tokens.json` / `WORKBUDDY_ACCESS_TOKEN.txt` / `wb_login_result.json`）需能映射到 `accounts[]`，不得另造格式。

---

## 5. 融合产品的 UI / 页面结构（基于 switch 底座）

| 页面 | 来源能力 | 说明 |
|---|---|---|
| 账号列表 | switch `accounts.json` | 多账号卡片、积分、过期倒计时、启用/禁用 |
| 一键切换 | switch | 写回 `auth.workbuddy.workbuddy-ai`，重启 IDE |
| 导入 / 导出 | 签到助手 + REA 规则 | 导入 `accounts.json`/`wb_refresh_tokens.json`，导出统一 schema |
| 签到中心 | switch + bworkbuddy | 每日签到、成长任务、连登、旅行/盲盒（按全景清单取舍） |
| 网关接入 | 全景清单 | 本地 `/v1` 网关地址、模型列表、Key 管理（可选内置） |
| 设置 | switch | WS 端口、开机自启、钩子、密钥存储 |
| 逆向/诊断 | REA 规则 | 客户端路径检测、登录态读取诊断（复用 `SetupClient` 规则） |

---

## 6. 尚未完成 / 需要你确认

1. **REA MCP 未接入**：要跑 REA 的 frida/ghidra 级动态逆向，需要把 REA 配成 MCP（`npx rea-agents mcp`）或在本地构建它的 CLI；静态逆向已能支撑 UI/账号层设计。
2. **exe 是成品无源码**：`wb-switch-rust.exe` 不能直接改；融合有两条路：
   - A. **套壳**：新做一个 Tauri 外壳，复用 switch 的 `accounts.json`/auth 文件协议，UI 自己写（推荐，可控）。
   - B. **补丁/注入**：改前端资源或 hook 进程（风险高、易碎，不建议）。
3. **是否要我真的起一个 Tauri 工程骨架**（页面 + 导入导出 + schema 校验 + 签到引擎接口）？确认后我按 `schema/workbuddy-account.schema.json` 直接落地代码。
