# 第三方声明

本项目自身代码为 MIT（见 `LICENSE`）。仓库内含以下第三方源码与参考产物，各自协议见对应条目。

## 仓库内随附的第三方源码

| 组件 | 来源 | 协议 | 用途 | 位置 |
|---|---|---|---|---|
| WorkBuddy-Daily | https://github.com/L0NE-6/WorkBuddy-Daily | MIT（LICENSE 已随附） | 签到 / 成长任务逻辑参考与源码 | `third_party/WorkBuddy-Daily/`（`workbuddy_daily.py` / `workbuddy_login.py` + LICENSE） |

## 仓库内随附的第三方参考产物

| 产物 | 来源 | 协议 | 用途 | 位置 |
|---|---|---|---|---|
| switch 前端资源（渲染后 JS / CSS / DOM） | https://github.com/changexbc/workbuddy-switch | MIT | 界面视觉对标、93 条命令表反查 | `reference/switch-assets/` |
| 便携版内核 README | https://github.com/linguo2625469/workbuddy2api-panel | MIT | 46 个 `config.json` 字段与 `/panel/api/*` 语义的权威出处 | `reference/panel-README.md` |

> `reference/switch-assets/` 是从 workbuddy-switch（Tauri 应用）通过 CDP 抓取的**渲染后**资源，
> 用途是维护时对照界面与命令表，**不是本项目的运行时代码**。原仓 MIT 协议，允许再分发；
> 若权利人要求，可按 `docs/REVERSE-REPORT.md` 记录的流程删除并改为重新抓取。

## 商标与图标

- WorkBuddy、CodeBuddy、Trae、Qoder、Z.AI、MiniMax、豆包、华为云 CodeArts、VS Code、JetBrains 等名称与 Logo 归各自公司所有。
- 本项目仅用于识别，不代表任何官方合作或背书。
- 如权利人要求，可替换为文字标签（`src/index.html` 的 `.logo-img`）。

## 仅作参考、未再分发

以下项目通过 `scripts/fetch-vendor.ps1` 拉取到本机 `vendor/`，**不包含在本仓库**：

- workbuddy2api-panel（linguo2625469）
- workbuddy2api-hub（ardeyouxipianyi）
- workbuddy-switch（changexbc）
- workbuddy-3in1、wb2go、modelwharf、wild-work、multi2api、magpie、ai-proxy、Agent2API、Free2API、workbuddy-gateway、workbuddy2api（HanawaBanana）

各自协议以仓库为准。

## 免责

本项目不提供账号、额度或破解能力；所有凭证来自用户本机已登录客户端。使用风险由使用者承担。
