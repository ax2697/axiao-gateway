# 数据模型

## 1. 统一账号（导出/导入）

Schema：`schema/workbuddy-account.schema.json`

```json
{
  "schemaVersion": "1.0.0",
  "clientRules": [
    { "name": "workbuddy", "displayName": "WorkBuddy",
      "configPath": [".workbuddy", "mcp.json"],
      "markerPath": [".workbuddy"],
      "skillPath": [".workbuddy", "skills"],
      "format": "json",
      "authFile": "AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai" }
  ],
  "accounts": [
    { "id": "wb_<uid>", "client": "workbuddy", "displayName": "主号",
      "accountId": "<uid>", "accessToken": null, "refreshToken": null,
      "expiresAt": "2026-12-01T00:00:00+08:00",
      "credits": 3000, "creditUsage": 1200, "creditExpiry": "2026-12-31",
      "enabled": true, "lastUsedAt": null, "region": "cn",
      "source": "portable", "notes": "最近签到 2026-11-03" }
  ]
}
```

- `clientRules` 与 REA `SetupClient` 对齐：`name / configPath / markerPath / skillPath / format / discoveryPaths / sharedRegistration / quitBeforeSetup`
- `accounts[].client` 外键引用 `clientRules[].name`
- 导出默认脱敏 token；`include_tokens` 才含明文

## 2. 便携版账号文件

`auths/workbuddy-<uid>.json`

```json
{
  "account": { "enterpriseId": "", "nickname": "130****", "uid": "..." },
  "auth": { "accessToken": "...", "refreshToken": "...", "expiresAt": 1796295919, "realm": "cn", "domain": "..." }
}
```

## 3. 运行状态

`data/state.json`

```json
{ "accounts": { "<uid>": {
  "credits": 47012, "credits_total": 52189, "disabled": false,
  "last_checkin_day": "2026-11-03",
  "credits_earliest_expiry": "2026-11-30T11:11:47+08:00",
  "credits_earliest_remaining": 45723,
  "soft_streak": 0, "consecutive_fails": 0
}}}
```

## 4. 网关配置

`config.json`（主要字段）

| 字段 | 含义 |
|---|---|
| `listen` | 监听地址，默认 `127.0.0.1:7863` |
| `api_key` | 网关鉴权密钥 |
| `schedule` | 签到/旅行/活跃/成长/保活/余额刷新 |
| `pool` | 并发、熔断、冷却、切号权重、积分保底 |
| `cooldown` | 软冷却基础/上限 |
| `upstream` | 超时、User-Agent、指纹 |
| `prompt` | 系统提示词模式/文件 |
| `logging` | 日志开关/保留/容量 |

## 5. 应用自有数据（`~/.wb-switch/axiao/`）

| 文件 | 内容 |
|---|---|
| `accounts.json` | 手动/导入账号（不含 portable 账号） |
| `extras.json` | 控制面、压缩、ZCode、思考档位、工具开关 |
| `switch-settings.json` | 自动签到/旅行/轮换 |
| `session-groups.json` | 会话组 |
| `update.json` | 更新代理 |
| `ledger.jsonl` | 网关调用账本 |
| `hook-events.jsonl` | 钩子事件 |

## 6. 钩子事件

`~/.wb-switch/hook-events.jsonl` 每行：

```json
{ "ts": "...", "kind": "signin", "detail": "..." }
```

来源：switch 的 `hook.sh` 与阿晓自己的操作。
