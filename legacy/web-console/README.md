# WorkBuddy 账号壳（套壳 A 方案）

复用 `workbuddy-switch` 的 `~/.wb-switch` + CodeBuddy 授权文件协议，提供账号池、切换、导入导出、签到、网关入口的本地控制台。

## 运行

双击 `start.bat`，或：

```bash
node server.mjs
```

浏览器打开 http://127.0.0.1:7870

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `PORT` | `7870` | 本地端口 |
| `WB_SWITCH_HOME` | `~/.wb-switch` | switch 数据根目录 |
| `WB_AUTH_FILE` | `%LOCALAPPDATA%/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai` | 授权文件 |
| `WB_SIGNIN_EXE` | `D:\...\WorkBuddy签到助手.exe` | 签到助手路径 |
| `WB_GATEWAY_BASE` | `http://127.0.0.1:7863/v1` | 本地网关基址 |

## 数据

- 统一账号库：`~/.wb-switch/axiao/accounts.json`（本应用读写）
- switch 原文件：`~/.wb-switch/accounts.json`（只读展示，不覆盖）
- 授权文件：写入前自动备份为 `auth.workbuddy.workbuddy-ai.axiao-bak-<时间戳>`

## API

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/status` | 路径与服务状态 |
| GET | `/api/clients` | REA 兼容客户端规则 |
| GET/POST | `/api/accounts` | 读/合并写账号 |
| POST | `/api/delete` | 按 id 删除账号 |
| POST | `/api/validate` | 校验统一 JSON |
| POST | `/api/import` | 导入合并（按 id 去重） |
| GET | `/api/export?tokens=1` | 导出（默认脱敏） |
| POST | `/api/switch` | 写回授权文件（先备份） |
| POST | `/api/signin` | 启动签到助手（需 `confirm:true`） |
| GET | `/api/gateway/models` | 透传网关 `/v1/models` |

## 说明

- 本机无 Rust/cargo，暂用 Node 本地应用；前端与 API 可原样套进 Tauri 壳。
- 账号导入导出格式见 `../schema/workbuddy-account.schema.json`，`clientRules` 与 REA `SetupClient` 对齐。
