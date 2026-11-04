# 融合清单（已下载 → 能力映射）

> 下载位置：`fusion/vendor/`（14 个仓库，浅克隆，合计 384MB）
> 映射目标：`fusion-app/`（阿晓网关助手 Tauri 工程）
> 口径：**不进程并列**，把每个仓库的独有能力模块移植/重写进同一套 Rust 代码。

## 1. 已下载仓库

| 仓库 | 语言 | 文件数 | 体积 | 检测到的能力 |
|---|---|---:|---:|---|
| CangShui/workbuddy-gateway | Go | 148 | 2.8M | cooldown/sticky/3protocol/zcode/credit/checkin/panel |
| HanawaBanana/workbuddy2api | Go | 369 | 5.6M | cooldown/sticky/3protocol/zcode/credit/checkin/panel |
| HitMargin/ai-proxy | Deno/TS | 18 | 61K | 3protocol/checkin |
| aimod-cc/Agent2API | Node/TS | 1026 | 37M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |
| ardeyouxipianyi/workbuddy2api-hub | Python | 405 | 12M | cooldown/sticky/3protocol/zcode/credit/checkin/panel |
| icebears111/workbuddy2api | Go | 71 | 1.7M | cooldown/sticky/3protocol/zcode/qoder/credit/checkin/panel |
| ikunops/free2api | Go | 487 | 8.8M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |
| jjjane233/workbuddy-3in1 | Go | 257 | 4.1M | cooldown/sticky/3protocol/trae/zcode/credit/checkin/panel |
| practice019/multi2api | Go | 1289 | 27M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |
| wangct233-source/wb2go | Go | 77 | 1.2M | cooldown/sticky/3protocol/trae/credit/checkin/panel |
| xu20060420fan-cell/workbuddy2api-trae-qoder-minimax-panel | Go | 1522 | 52M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |
| yanghuan/wild-work | Go | 92 | 2.8M | cooldown/sticky/3protocol/trae/qoder/credit/checkin/panel |
| yetone/magpie | Go | 7792 | 230M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |
| zouzhenglu/modelwharf | Node/TS | 88 | 1.9M | cooldown/sticky/3protocol/trae/zcode/qoder/credit/checkin/panel |

## 2. 能力 → 模块映射

| 能力 | 参考仓库（关键实现） | 阿晓网关助手落点 |
|---|---|---|
| 多账号池 / 权重选号 | HanawaBanana、icebears111、free2api | `src-tauri/src/store.rs` + `gateway.rs::select` |
| 配额感知切号 | icebears111、Agent2API | `gateway.rs::select`（按 credits/weight） |
| 冷却状态机 / 熔断 | CangShui、HanawaBanana、free2api | `gateway.rs::cool`（429/5xx → 60s 冷却） |
| 会话粘性 | HanawaBanana、free2api | `gateway.rs::sticky` |
| 三协议（Chat/Responses/Anthropic） | hub、ai-proxy、Agent2API | `gateway.rs::protocol_path` |
| 多上游（WB/CodeBuddy/Trae/ZCode/Qoder） | 3in1、multi2api、wild-work、modelwharf | `gateway.rs` 上游表 + `upstreams.json` |
| 成本账本 | HanawaBanana | `gateway.rs::append_ledger` + `ledger.jsonl` |
| 签到 / 成长任务 | panel、bworkbuddy | `signin.rs::launch_signin` |
| 积分/过期监控 | switch、panel | `store.rs` 账号字段 + 前端积分构成 |
| Web 面板 / 日志 | panel、wb2go | 前端 20 模块 + `hooks.rs` |
| 导入导出（REA 规则） | panel + switch + REA | `store.rs::import/export_accounts` |
| 密钥保护 | switch（secret-storage） | `auth.rs::store_secret/load_secret`（keyring） |
| 钩子 | switch（hook-events.jsonl） | `hooks.rs` |
| 多桌面端聚合 | Agent2API | 上游表 + 协议路由（后续可扩客户端适配） |
| Trae 互通 | wild-work、multi2api、magpie | 上游表增加 trae 项 + 协议翻译 |
| ZCode 专用 | zcode-api、3in1 | 上游表增加 zcode 项 + off-peak 队列（规划） |

## 3. 为什么不直接编译这些仓库

- 语言不一（Go/Python/Node/Deno/Rust），无法链接进同一进程。
- 多数是同一底座（workbuddy2api-panel）的 fork，代码互相冲突。
- 正确融合方式：**保留一个内核（Rust 重写的能力并集）+ 上游可配置**，而不是把 14 个二进制并列。

## 5. 便携版真实数据源（已逐个读）

| 文件 | 结构 |
|---|---|
| `config.json` | `api_key` / `listen` / `pool` / `schedule` / `session_sticky` |
| `auths/workbuddy-<uid>.json` | `account{uid,nickname,enterpriseId}` + `auth{accessToken,refreshToken,expiresAt,realm,domain}` |
| `data/state.json` | 按 uid：`credits`、`credits_total`、`disabled`、`last_success`、`last_checkin_day`、`token_usage`、`soft_streak`、`consecutive_fails`、`credits_earliest_expiry`、`credits_earliest_remaining` |
| `data/usage.json` | `version` / `saved` / `buckets[]` |
| `data/request-logs/` | 请求日志归档 |
| `wb2api.exe` | 主程序（v1.13.0，go:embed 前端） |

> 任务与模型不是固定数据：`GET /panel/api/accounts/{uid}/tasks` 返回 21 条真实任务；`GET /panel/api/models` 返回真实模型档位；`POST /panel/api/accounts/{uid}/tasks/auto_all` 一键完成。

## 4. 下一步

1. 补齐前端 20 模块（面板 7 + switch 3 + 签到 3 + 网关 4 + 系统 3）。
2. 把 `vendor/` 里各仓库的协议细节（端点、请求头、签到接口）逐项移植进 `gateway.rs` / `signin.rs`。
3. 安装 Rust 工具链后 `npm run build` 出 `axiao-gateway.exe`。
