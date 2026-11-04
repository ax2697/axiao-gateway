# 维护指南（长期更新用）

## 0. 发版前必跑：泄露自检

**这一步不能跳。** 个人密钥、真实 token、本机绝对路径、测试账号，只要混进
`docs/` 就会被公开发出去。

```powershell
powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1
```

脚本会扫：

| 检查项 | 说明 |
|---|---|
| `sk-` + 20 位以上 | 明文 API Key（**便携版出厂值已白名单放行**，其他一律拦截） |
| `"accessToken": "..."` 长串 | 真实 token |
| `C:\Users\<名字>` / 本机盘符家目录 | 个人绝对路径 |
| 工作区里的 `auths/`、`config.json`、`accounts.json` | 账号数据 |

> 白名单在脚本顶部的 `$KNOWN_DEFAULTS`。`sk-QxgVEFVZM6Pg8Z6MfeiCA6TS` 是便携包
> 出厂预置的 `api_key`，属于功能配置不是个人凭证，所以保留在文档里。它的风险是
> 「所有便携包共用一个默认值」——只在 `127.0.0.1` 用没问题，**改成对外监听前必须换**。

退出码 0 才允许打包；非 0 会列出文件与行号。

> 另外，**打包机的应用数据目录**也要清：`%USERPROFILE%\.wb-switch\axiao\`
> 里的 `accounts.json`、`ledger.jsonl`、`hook-events.jsonl` 是测试残留，
> 会被新装的应用读到，表现成「我没这么多账号，怎么多出来几个」。
> 清法：整个 `axiao` 子目录删掉，应用重启会自己重建空目录。

## 1. 发版流程

```bash
# 1. 改版本号
#    src-tauri/tauri.conf.json -> version
#    src-tauri/Cargo.toml      -> version
#    package.json              -> version

# 2. 更新 CHANGELOG.md

# 3. 泄露自检（必须）
powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1

# 4. 本地验证
npm run dev

# 5. 提交并打 tag
git add -A
git commit -m "release: v0.1.1"
git tag v0.1.1
git push origin main --tags
```

推送 tag 后，GitHub Actions（`.github/workflows/release.yml`）会在 windows-latest 上自动构建并挂到 Release。

## 2. 每月例行

- [ ] `cargo update`（在 `src-tauri`），跑 `cargo check`
- [ ] `npm update`，跑 `npm run dev`
- [ ] 检查 GitHub Issues
- [ ] 检查上游接口是否变化（见第 3 节）
- [ ] 发一个 patch 版本

## 3. 上游接口巡检

| 接口 | 用途 | 变化时的表现 |
|---|---|---|
| `copilot.tencent.com/v2/billing/meter/checkin-activity-status` | 签到状态 | 签到返回 404/401 |
| `copilot.tencent.com/v2/billing/meter/daily-checkin` | 签到 | 同上 |
| `copilot.tencent.com/v2/activity/growth/tasks` | 成长任务 | 任务列表为空 |
| `copilot.tencent.com/v2/activity/growth/buddy/travel/*` | 旅行 | 旅行报错 |
| `copilot.tencent.com/v2/report` | 活跃保活 | 连登不涨 |
| `127.0.0.1:7863/panel/api/*` | 面板 | 任务/模型/配置空白 |
| `api.trae.cn/.../checkin_credits` | Trae 签到 | 需插件会话 |
| `api.z.ai` / `open.bigmodel.cn` / `opencode.ai` | 额度 | 额度卡抓取失败 |

巡检方式：打开「签到中心」「任务中心」「免费额度」，各点一次，看返回。

## 4. 数据兼容

- 账号 Schema 版本：`schemaVersion: "1.0.0"`（`schema/workbuddy-account.schema.json`）
- 改字段必须升版本，并在 `store.rs` 的 `import_accounts` 做迁移
- `extras.json` 用默认值合并，新增字段不破坏旧文件

## 5. 依赖升级注意

- `tauri` / `tauri-build` 大版本升级：同时升 `@tauri-apps/cli`
- `keyring`：Windows 后端默认，不要再加 `windows-native`（2.x 不存在）
- `reqwest`：保持 `native-tls`，换 `rustls` 会引入 ring/C 编译
- `windows` 系列 crate 由 tauri 传递，不要手动锁版本

## 6. 安全

- 永远不要提交 `auths/`、`config.json`、`*.axiao-bak`
- 用户报告凭证泄露 → 建议其轮换 `api_key` + 重新授权账号
- Logo 如收到商标通知 → 替换为文字标签（`src/index.html` 的 `.logo-img`）

## 7. 发布包的「干净」标准

一个可以发出去的包，里面**只能有**：

```
axiao-gateway.exe        程序本体
WebView2Loader.dll       运行时依赖（GNU 构建需要）
config.example.json      配置模板（api_key 必须是 sk-REPLACE_ME）
使用说明.txt / 新手教程.md   文档
README.md
```

绝对不能有：`auths/`、`config.json`（真配置）、`data/`、`accounts.json`、
`ledger.jsonl`、`*.axiao-bak`、任何 `sk-` 真密钥。

验收命令（在解压目录里跑）：

```powershell
Get-ChildItem -Recurse | Where-Object { $_.Name -match '^(auths|data)$' -or $_.Name -eq 'config.json' }
Select-String -Path *.* -Pattern 'sk-[A-Za-z0-9_-]{20,}'
```

两条都应该没有任何输出。
