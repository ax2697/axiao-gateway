# WorkBuddy / CodeBuddy 专用网关 · 生态全景与融合方案（最终版）

> 目标：把分散的 40+ 个网关项目合并成一份去重后的全景图，去掉"基础能力重复"和"衍生 fork 雷同"，只保留每个项目的独有价值，并给出"融合成一个统一网关"的蓝图 + Trae Code CN 互通接线法。
> 数据源：用户五轮贴表 + `workbuddy-gateways-open-source.md` 全集（含 F 节）。最后更新：2026-11-03（最终版）

---

## 0. 去重结论（先看这里）

1. **本地「WorkBuddy网关-便携版」不是独立项目** —— 它就是 `workbuddy2api-panel` 的 Windows 便携打包（v1.13.0 windows-amd64 官方 Release，`wb2api.exe` 即该仓库产物）。已并入 panel 行。
2. **WB 专用网关 37 个 + Trae 专用 5 个，真正独立技术路线约 20 条**，其余是 panel / workbuddy2api / hub 的衍生 fork、同思路轻量克隆、或运维/观测类旁支。
3. **基础网关能力高度重叠**（多账号池轮转、Token 续期/保活、OpenAI 兼容透传、熔断/冷却、会话粘性）——"网关"标配，合并成一条，不逐行重复。
4. 真实差异集中在六个维度：**上游覆盖面、协议、登录态来源、调度策略、管理平面、部署形态**。
5. **9 个通用聚合网关（D 类）+ 8 个广义生态（横向参考）不是 WorkBuddy 专用**——单独成节，不计入专用去重。
6. 雷同集群：**DSH 集群**（#17/#18/#19）、**Node 零依赖集群**（#14/#16）、**全家桶集群**（#21/#22/#23）、**Python codebuddy2api 链**（#20/#28 根 + #26 平行 + #29 衍生）、**panel fork 链**（#1 主 + #27 快照 + #33 增强）、**WB+Trae 互通集群**（#34/#35/#36/#37）、**Trae2api 链**（F2 中 J1aDong→linqiu919/chismi1017 归档 fork）。
7. **上游已删库 + 主分支曾停维**：必藏 `jiangwanying` 最终版快照（#27）与 `workbuddy2api-hub`（#3）作备份。
8. **饱和度分层**：「WB 单上游」方向已饱和（继续检索仅返回 fork/镜像站）；但「**WB + Trae Code CN 多上游互通**」（F 节）是仍活跃的独立维度，仍有大量项目。

---

## 1. 专用网关主清单（37 个 WB 相关，按路线归类，已去重）

| # | 项目 | 形态 / 便携度 | 路线归类 | 独有 delta（去重后保留） | 仓库 |
|---|---|---|---|---|---|
| 1 | **workbuddy2api-panel** ★含本地便携包 | Go 单文件免 Docker | **本体**（本地=此仓库打包） | 最完整 Web 面板 + 成长任务一键完成(17/18) + 任务中心 + 用量统计 + global 国际域 | linguo2625469/workbuddy2api-panel |
| 2 | workbuddy-gateway | 纯 Go 单二进制，零 CGO，跨平台 | 独立 | 国内站/国际站**双站**原生；可选 WebUI | CangShui/workbuddy-gateway |
| 3 | workbuddy2api-hub | 绿色包 + 精简 Python，双击即启 | 独立 | **三协议** Chat/Responses/Anthropic；设备指纹隔离；国际/国内双区域路由 | ardeyouxipianyi/workbuddy2api-hub |
| 4 | workbuddy2api (HanawaBanana) | Go 自托管，OAuth 设备授权 | 独立（多 fork 基座） | **成本账本**（无 UI，功能最硬核）；会话粘性/熔断冷却最强 | HanawaBanana/workbuddy2api |
| 5 | Free2API | Go 单二进制 + 内置控制台 | 独立 | **读本地客户端登录态**（免 OAuth） | ikunops/free2api |
| 6 | Agent2API | Rust + Tauri 桌面 / Docker | 独立 | **聚合 15+ AI 编程桌面端**成单网关；全局优先级队列；429 自动降级 | aimod-cc/Agent2API |
| 7 | WorkBuddy2API-Android | Android APK（网关+面板+看门狗） | 独立 | **手机常驻**、移动管理面板、账号轮询 | adeahejshshsh/workbuddy2api-android |
| 8 | workbuddy-3in1 | Go 单二进制 + 面板 | **panel 衍生** | **多上游三合一**：WorkBuddy + Trae Code + ZCode 共用 /v1 | jjjane233/workbuddy-3in1 |
| 9 | icebears111/workbuddy2api | Go 单二进制，零外部依赖 | **workbuddy2api 衍生** | **配额感知切号** + 冷却状态机 + API Key/Token 混挂；Web 后台 :7865 | icebears111/workbuddy2api |
| 10 | workbuddy-manager | Next.js+FastAPI+SQLite | **正交（控制面）** | 账号池**扫码批量纳管** + 密钥分组分发 + IP/模型白名单 + 签名校验一键更新 | ithtelab/workbuddy-manager |
| 11 | codebuddy2openai | 单文件 converter.py，极简 | 独立（≈Free2API 思路） | **复用本地已登录 CodeBuddy CLI 会话**，零鉴权，仅 127.0.0.1 | HanHan666666/codebuddy2openai |
| 12 | codebuddy-openai-proxy | Python，OAuth2 | 独立 | **CodeBuddy CN** → OpenAI 兼容，支持手动 Token，接 Cherry Studio/Cursor | gitee.com/jevil961/codebuddy-openai-proxy |
| 13 | workbuddy2api-ui | Docker + Web 控制台 | **panel 的 Docker 形态** | 自托管 OpenAI 反代 + Web 控制台，浏览器 OAuth 授权即加号（:7863），一条 docker compose 部署 | cnblogs 同名仓库 |
| 14 | yuan240324/workbuddy-openai-proxy | Node.js 零依赖 | 独立（Node 零依赖集群） | WorkBuddy/CodeBuddy→OpenAI+Anthropic；多账号轮换 + **上下文压缩** | yuan240324/workbuddy-openai-proxy |
| 15 | hawklithm/workbuddy2api | Python 轻量 | 独立 | 国内/国际 CodeBuddy；OpenAI/Anthropic/Responses；**原生支持 Claude Code/Codex CLI/DSH** | hawklithm/workbuddy2api |
| 16 | doone-skser/wb-proxy | Node.js 单文件零依赖 | 独立（Node 零依赖集群） | WorkBuddy→OpenAI，Chat/Responses；token 自动续期 + **429 退避** | doone-skser/wb-proxy |
| 17 | zhuang-HE/workbuddy-harness | JS | **DSH 集群** | **复用本机桌面端登录态**→本地 OpenAI/Anthropic；附面板 + **DSH 原生插件** | zhuang-HE/workbuddy-harness |
| 18 | Ianzhyh/workbuddy-to-dsh | JS | **DSH 集群**（≈#17） | 本机桌面端模型→本地 OpenAI/Anthropic + DSH 插件 | Ianzhyh/workbuddy-to-dsh |
| 19 | workbuddy-gateway-dsh | DSH 插件 | **DSH 集群**（≈#17） | DSH 一键安装版网关，127.0.0.1:8317 | ethanweave/workbuddy-gateway-dsh |
| 20 | Sliverkiss/CodeBuddy2api | Python | **Python 链根**（已停更） | 原 Python 项目，CodeBuddy **API Key 直连模式**（溯源参考） | Sliverkiss/CodeBuddy2api |
| 21 | workbuddy2api-trae-qoder-minimax-panel | Go + Web 面板 | **全家桶集群** | 最多上游：WorkBuddy/CodeBuddy/Trae/Qoder/MiniMax/豆包/华为云 CodeArts | xu20060420fan-cell/...-panel |
| 22 | unified2api | Python + 控制台 (Docker) | **全家桶集群** | TRAE/CodeBuddy 国内/WorkBuddy 国际/MonkeyCode/Qoder + **自定义 OpenAI** | rycar1/unified2api |
| 23 | modelwharf（模型码头） | Go/TS + 面板 | **全家桶集群**（最强） | workbuddy/trae/qwen/qoder + **24 家官方 API** → OpenAI/Anthropic/**MCP** | zouzhenglu/modelwharf |
| 24 | zcode-proxy / zcode-api | Python | **关联上游专用** | Z.AI/Bigmodel.cn coding-plan → OpenAI+Anthropic；**off-peak 异步队列** | mucsbr/zcode-api |
| 25 | **wb2go** | Go 零依赖单文件 | **独立（最贴便携版新秀）** | 纯 Go 零依赖单二进制 + **内嵌 Web 控制台** + **热更新** + 国内/国际双区域 + 多账号池 | wangct233-source/wb2go |
| 26 | maiphucgiang/codebuddy2api | Python + WebUI | **独立（≈hub 思路平行）** | OpenAI Chat/Responses + Anthropic Messages；浏览器登录、国内/国际路由（~247★） | maiphucgiang/codebuddy2api |
| 27 | jiangwanying/workbuddy2api-panel | Go（预编译/Docker） | **panel fork（最终版快照）** | 上游停维后的**最终版 v1.13.0 快照** + 预编译二进制（上游死后的保险） | jiangwanying/workbuddy2api-panel |
| 28 | FFFFFFpy/codebuddy2api | Python | **Python 链源头** | CodeBuddy/WorkBuddy 订阅 → OpenAI+Anthropic 兼容（tonny0812 等多个 fork 的源头） | FFFFFFpy/codebuddy2api |
| 29 | tonny0812/buddy-tools | Python | **正交（运维工具，非网关）** | workbuddy/codebuddy **批量签到** + apikey 代理（辅助运维） | tonny0812/buddy-tools |
| 30 | YuLaiZ/token-usage | Go CLI + 仪表盘 | **正交（观测分析）** | 本地 LLM **用量分析**仪表盘，支持 DSH/WorkBuddy/ZCode/CodeBuddy | YuLaiZ/token-usage |
| 31 | Tom6814/WorkBuddy2API | Go（推测） | **独立（轻量开箱）** | 微信技术文推荐"最简单"三选一之一（另为 panel/hub），定位轻量开箱 | Tom6814/WorkBuddy2API |
| 32 | linbeize/workbuddy2api | Go | 独立（CodeBuddy CN 专用） | CodeBuddy CN OpenAI 兼容反代；OAuth 登录、多账号轮转、**工具调用**、流式 | linbeize/workbuddy2api |
| 33 | zhizhishu/workbuddy2api-panel | Go | **panel 增强 fork** | linguo2625469 panel 的**另一增强 fork**（与 #27 快照定位不同） | zhizhishu/workbuddy2api-panel |
| 34 | **yanghuan/wild-work** | Go 桌面（托盘+浏览器面板） | **WB+Trae 互通集群** | **WB(CodeBuddy)+TraeWork+Qoder** 聚合一 `/v1`；双击启动、模型前缀路由、粘性路由、自动签到/保活、**developer→system 角色转换** | yanghuan/wild-work |
| 35 | **practice019/multi2api** | Go + /ui 控制台 | **WB+Trae 互通集群** | WB/WorkBuddy AI/CodeArts/Loomy/**TRAE** 统一抽象到账号池；权重选号/会话粘性/熔断冷却/续期；**核心代码可扩展上游** | practice019/multi2api |
| 36 | **yetone/magpie** | 菜单栏 App（mac/win/linux） | **WB+Trae 互通集群** | 聚合 **WB+Trae CN+OpenCode Zen Free 免费额度**；自动协议转换+多账号 failover；**一键接 Claude Code/Codex/Gemini CLI/DSH** | yetone/magpie |
| 37 | **HitMargin/ai-proxy** | Deno 本地+Cloudflared+CF Worker | **WB+Trae 互通集群** | **10 上游聚合**；`trae.ts`/`workbuddy.ts` 协议翻译、工具调用、**思考档位**、每日签到、积分余额；附 DSH 插件面板 | HitMargin/ai-proxy |

> 注释：#8=#1 加多上游；#9=#4 加切号；#13≈#1 Docker 版；#11 同 #5；#17/18/19 同属"复用桌面端态+DSH"；#14/16 同属"Node 零依赖"；#21/22/23 同属"多上游全家桶"；#27=panel 快照、#33=panel 增强 fork；#28 是 Python codebuddy2api 链源头（#26 平行、#29 衍生）；#29/#30 是运维/观测旁支；#31 是微信推荐轻量档；#34-37 同属"WB+Trae 互通"，各自 delta 不同（见第 9 节）。

---

## 2. 通用聚合网关（9 个，D 类——非 WorkBuddy 专用，接任意上游官方 Key）

| 项目 | Stars | 一句话定位 | 仓库 |
|---|---|---|---|
| OmniRoute | 84K | 359 家/1200+ 模型单端点，配额感知自动 fallback | cheaper-inference/OmniRoute |
| headroom | 78.6K | 上下文压缩代理/库/MCP，省 token | headroomlabs-ai/headroom |
| 9router | 31.9K | 本地路由+省 token，40+ provider/100+ 模型，订阅→免费模型自动降级 | decolua/9router |
| Portkey-AI/gateway | 13.3K | 1600+ 模型/45+ provider 单 API，含 guardrails/缓存/重试 | Portkey-AI/gateway |
| bifrost | 9.3K | 高性能网关，23+ provider 统一 OpenAI 兼容，自动 failover | maximhq/bifrost |
| One API | 30K+ | 最经典单文件，20+ 模型，负载均衡/令牌管理 | songquanpeng/one-api |
| New API | 7K+ | One API 二次开发，功能最全，Windows 单 exe | QuantumNous/new-api |
| VoAPI | 1K+ | 基于 New/One API，高颜值 UI、规则引擎 | VoAPI/VoAPI |
| 2papi | — | Go 单二进制，订阅账号池化、虚拟 key、预算、MCP 网关 | Retinger/2papi |

---

## 3. 能力矩阵（共享基础 + 独有）

### 3.1 共享基础能力（一次列出，不重复计）
多账号池轮转 · Token 续期/保活 · OpenAI 兼容模型透传 · 熔断/冷却（部分）· 会话粘性（部分）· 自动签到/活跃保活（多数）· 工具调用/流式（多数已支持）。

### 3.2 独有 / 特色能力（真实去重价值）

| 能力维度 | 仅谁提供 |
|---|---|
| 双站 / 多区域原生路由 | gateway（双站）、hub（双区域）、panel（global 域）、wb2go（双区域） |
| 三协议适配（Responses / Anthropic Messages） | hub、maiphucgiang（#26）、hawklithm（#15） |
| 设备指纹隔离 | hub（#3）独有 |
| 成本账本 / 选号最硬核 | workbuddy2api（#4）独有 |
| 读本地客户端登录态（免 OAuth） | Free2API（#5）独有 |
| 多桌面端统一聚合 + 全局优先级队列 + 429 降级 | Agent2API（#6）独有 |
| 成长任务自动完成 + 最完整 Web 面板 | panel（#1，=本地）独有 |
| 移动端常驻 | Android（#7）独有 |
| 多上游三合一（WB+Trae+ZCode） | workbuddy-3in1（#8）独有 |
| 配额感知切号 + 冷却状态机 + Key/Token 混挂 | icebears111（#9）独有 |
| 控制面：批量纳管 / 密钥分发 / 白名单 / 签名更新 | workbuddy-manager（#10）独有（正交层） |
| 极简本地 CLI 会话复用（零鉴权） | codebuddy2openai（#11）独有 |
| CodeBuddy CN 专用 OAuth2 + 手动 Token | codebuddy-openai-proxy（#12）独有 |
| Docker Web 控制台（浏览器 OAuth 即加号） | workbuddy2api-ui（#13，panel 的 Docker 形态） |
| Node.js 零依赖 + 上下文压缩 | yuan240324（#14）独有 |
| Python 轻量 + 原生支持 Claude Code/Codex CLI/DSH | hawklithm（#15）独有 |
| Node.js 零依赖 + 429 退避 | doone-skser/wb-proxy（#16）独有 |
| 复用桌面端登录态 + DSH 原生插件 | zhuang-HE（#17）/ Ianzhyh（#18）/ workbuddy-gateway-dsh（#19）集群 |
| 历史 Python 根 / Python codebuddy2api 链源头 | Sliverkiss（#20）/ FFFFFFpy（#28）溯源参考 |
| 全家桶：7+ 编程上游覆盖 | trae-qoder-minimax-panel（#21）独有 |
| 全家桶 + 自定义 OpenAI 上游 | unified2api（#22）独有 |
| 全家桶 + 24 家官方 API + MCP 协议 | modelwharf（#23）独有 |
| ZCode 上游专用 + off-peak 异步队列 | zcode-proxy（#24）独有 |
| Go 零依赖 + 内嵌 Web 控制台 + 热更新 | wb2go（#25）独有 |
| Python + WebUI + 浏览器登录（三协议） | maiphucgiang（#26）独有 |
| panel 最终版快照（上游死后的保险） | jiangwanying（#27，panel fork） |
| 批量签到运维工具（非网关本体） | tonny0812/buddy-tools（#29）旁支 |
| 本地 LLM 用量分析仪表盘（观测面） | YuLaiZ/token-usage（#30）旁支 |
| 微信推荐"最简单"轻量开箱（与 panel/hub 同档） | Tom6814（#31）轻量档 |
| CodeBuddy CN 专用 + 工具调用 + 流式 | linbeize（#32）独有 |
| panel 另一增强 fork | zhizhishu（#33，panel fork） |
| **WB+Trae+Qoder 聚合 + 托盘/双击 + 角色转换** | wild-work（#34）独有 |
| 多上游抽象到账号池 + **可扩展上游** | multi2api（#35）独有 |
| **免费额度聚合** + 菜单栏 + 一键接 CLI/DSH | magpie（#36）独有 |
| **Deno+Cloudflare 10 上游 + 协议翻译 + 思考档位** | ai-proxy（#37）独有 |

---

## 4. 主分支家族关系（避免踩已删库 / 套壳）

```
Sliverkiss/workbuddy2api  ← 原 Go 项目，已删库（根）
├─ linguo2625469/workbuddy2api-panel  ★活跃主分支 ~4.4k stars（= 本地便携包本体，最值得跟）
│   ├─ jackspeng / catboy98 / misakano7545  (fork/mirror)
│   ├─ jjjane233/workbuddy-3in1             (fork + Trae/ZCode 上游, #8)
│   ├─ jiangwanying/workbuddy2api-panel     (最终版 v1.13.0 快照, #27，保险)
│   └─ zhizhishu/workbuddy2api-panel        (另一增强 fork, #33)
└─ 独立新实现（非 fork）：
    HanawaBanana(#4) · ikunops/free2api(#5) · ardeyouxipianyi/workbuddy2api-hub(#3)
    icebears111(#9) · CangShui/workbuddy-gateway(#2) · hawklithm(#15)
    wangct233-source/wb2go(#25) · linbeize/workbuddy2api(#32) · Tom6814(#31)
    WB+Trae 互通：yanghuan/wild-work(#34) · practice019/multi2api(#35)
                 yetone/magpie(#36) · HitMargin/ai-proxy(#37)

Python codebuddy2api 链：Sliverkiss/CodeBuddy2api(#20) → FFFFFFpy(#28) → tonny0812(#29) / maiphucgiang(#26)
Trae2api 链（Trae 专用，见第 9 节 F2）：J1aDong → linqiu919 / chismi1017（归档 fork）
```

> 选型建议：跟 **panel（#1/#27/#33）** 主分支 + 快照最稳；多上游看 **3in1(#8)/modelwharf(#23)**；WB+Trae 互通看 **wild-work(#34)/multi2api(#35)/magpie(#36)/ai-proxy(#37)**；硬核调度看 **workbuddy2api(#4)/icebears111(#9)**；便携新秀看 **wb2go(#25)**。上游删库 → 必藏 **jiangwanying 快照(#27)** + **hub(#3)**。

---

## 5. 融合蓝图（统一网关构成）

- **内核 / 调度层**：`workbuddy2api`（#4）为底，套 `panel`（#1）运维层；便携新秀 `wb2go`（#25）可作轻量替代底座。
- **上游覆盖**：`workbuddy-3in1`（#8）+ `gateway`（#2）双站；全家桶叠加 `modelwharf`（#23）。**WB+Trae 互通**用 `wild-work`（#34，托盘双击+角色转换）或 `multi2api`（#35，可扩展上游）或 `magpie`（#36，免费额度聚合）。
- **协议层**：`hub`（#3）Responses/Anthropic + 指纹隔离；`hawklithm`（#15）/ `maiphucgiang`（#26）CLI/浏览器登录；`linbeize`（#32）补 CodeBuddy CN + 工具调用；`ai-proxy`（#37）的协议翻译+思考档位。
- **登录态来源**：panel OAuth + Free2API（#5）本地态 + codebuddy2openai（#11）CLI 复用 + icebears111（#9）混挂；桌面端用 DSH 集群（#17-19）。
- **调度策略**：icebears111（#9）配额感知切号 + 冷却状态机；Agent2API（#6）优先级队列 + 429 降级；doone-skser（#16）429 退避、yuan240324（#14）上下文压缩。
- **管理平面**：workbuddy-manager（#10）+ tonny0812（#29）批量签到 + YuLaiZ（#30）用量仪表盘。
- **部署形态**：gateway（#2）/ wb2go（#25）零依赖单二进制 + panel/workbuddy2api-ui（#13）Docker/Web + Android（#7）。
- **官方 Key 兜底**：接 OmniRoute/New API（D 类）降级。

**最小可行融合**：panel（本体）+ 3in1 多上游 + hub 三协议 + icebears111 配额感知切号；**要做 WB+Trae 互通则加 wild-work(#34) 或 multi2api(#35)**。同步备份 jiangwanying 快照(#27)。

---

## 6. 重复项 / 雷同清单（融合时只取 delta）

| 重复组 | 关系 | 取谁的 delta |
|---|---|---|
| 本地便携包 ↔ panel | 同一项目 | 已是 panel |
| workbuddy-3in1 ↔ panel | 衍生（加多上游） | 「多上游三合一」 |
| workbuddy2api-ui ↔ panel | Docker 形态 | 「Docker 部署形态」 |
| jiangwanying/zhizhishu ↔ panel | fork（快照/增强） | #27 快照保险 + #33 增强 |
| icebears111 ↔ workbuddy2api | 衍生（加切号） | 「配额感知切号+冷却状态机+混挂」 |
| codebuddy2openai ↔ Free2API | 同思路 | 「极简+CodeBuddy CLI 会话」 |
| maiphucgiang ↔ hub | 平行（Py+WebUI 三协议） | 「Python+WebUI+浏览器登录」 |
| Tom6814 ↔ panel/hub | 同档轻量 | 「微信推荐最简单档」 |
| linbeize ↔ codebuddy-openai-proxy | 同思路（CN 专用） | 「工具调用+流式+Go」 |
| DSH 集群 #17/#18/#19 | 同思路 | #17 为代表 |
| Node 零依赖 #14/#16 | 同形态 | #14 上下文压缩 + #16 的 429 退避 |
| 全家桶 #21/#22/#23 | 同思路 | #23（覆盖最广+MCP）为旗舰 |
| **WB+Trae 互通 #34/#35/#36/#37** | 同维度不同实现 | #34 托盘双击+角色转换 / #35 可扩展上游 / #36 免费额度+菜单栏 / #37 Deno+CF+思考档位 |
| Python 链 #20/#28/#26/#29 | 同源 | #28 源头，#26 平行，#29 运维 |
| Trae2api 链（F2） | 归档 fork | 取 trae2api-cn / trae2api-web 为现役，J1aDong/linqiu919/chismi1017 仅溯源 |

---

## 7. 活跃度参考与饱和度结论（2026-11 快照）

- linguo2625469/workbuddy2api-panel：~4.4k stars，最活跃主分支
- ardeyouxipianyi/workbuddy2api-hub：~3.3k stars（便携绿色包首选）
- aimod-cc/Agent2API：~1.6k–1.8k stars；wb2go、wild-work、magpie、multi2api、modelwharf 等互通/全家桶新秀活跃
- **风险预警**：上游 Sliverkiss/workbuddy2api 已删库；linguo 主分支曾停维 → 必藏 jiangwanying 快照(#27)+hub(#3)

**饱和度分层结论**：
- 「**WB 单上游**」方向已饱和：真正独立实现约 25–30 个、高度同质，继续检索主要返回 fork/镜像站（github.locker / git.vaiox / codekk / githubhelp / repositorystats / kaiyuanbang 等），无实质新项目。
- 「**WB + Trae Code CN 多上游互通**」（F 节）是仍活跃的独立维度，项目仍在增长（wild-work / multi2api / magpie / ai-proxy 等）。
- 最终推荐档位：主分支 panel｜便携首选 hub｜纯 Go 零依赖 wb2go/workbuddy-gateway｜跨客户端全家桶 Agent2API/modelwharf/unified2api｜WB+Trae 互通 wild-work/multi2api/magpie｜保险存档 jiangwanying 快照。

---

## 8. 广义 AI 网关生态（横向参考，非专用，仅作同源背景）

| 项目 | Stars | 说明 | 仓库 |
|---|---|---|---|
| opencodex | 18.2K | OpenAI Codex / Claude Code 跑 40+ provider，universal provider proxy | lidge-jun/opencodex |
| gpt-load | 7.2K | Go，API key / 订阅账号路由到单入口，含调度/故障转移/日志 | tbphp/gpt-load |
| monid | 6.1K | "OpenRouter for agent tools"，1 base URL 跨 72+ provider、2000+ tools | monid-ai/monid |
| openclaw-zero-token | 5.2K | 登录官方 web UI 后免费用 Claude/ChatGPT/Gemini 等，统一网关 | linuxhsj/openclaw-zero-token |
| weave-os/router | 5.6K | drop-in proxy 经 OpenRouter 路由到最佳模型，省 40–70% 成本 | weave-os/router |
| ccx | 4K | Go，44 vendor 统一入口 + 协议转换 | BenedictKing/ccx |
| archestra | 4.4K | 企业 AI 平台（含 LLM 网关 / MCP / A2A） | archestra-ai/archestra |
| my-free-code / ThinkWatch-Lite / freebucks-proxy | — | 同类轻量路由/代理（Py/TS/Go 各一） | hkqr · ThinkWatchProject · trefeon |

---

## 9. WorkBuddy ↔ Trae Code CN 互通维度（F 节融合，最终版新增）

> 本质：**多上游聚合网关**——把两边账号池化到同一 `/v1` 入口。这是独立于"WB 单上游"的活跃维度。

### 9.1 直接互通：WB + Trae 同网关（WB 相关，已并入主清单 #34–#37）

| # | 项目 | 形态 | 说明 | 仓库 |
|---|------|------|------|------|
| 34 | yanghuan/wild-work | Go 桌面（托盘+浏览器面板） | WB(CodeBuddy)+TraeWork+Qoder 聚合一 `/v1`；双击启动、模型前缀路由、粘性路由、自动签到/保活、developer→system 角色转换 | yanghuan/wild-work |
| 35 | practice019/multi2api | Go + /ui 控制台 | WB/WorkBuddy AI/CodeArts/Loomy/TRAE 统一抽象到账号池；权重选号/粘性/熔断/续期；核心代码可扩展上游 | practice019/multi2api |
| 36 | yetone/magpie | 菜单栏 App（mac/win/linux） | 聚合 WB+Trae CN+OpenCode Zen Free 免费额度；自动协议转换+多账号 failover；一键接 Claude Code/Codex/Gemini CLI/DSH | yetone/magpie |
| 37 | HitMargin/ai-proxy | Deno 本地+Cloudflared+CF Worker | 10 上游聚合；trae.ts/workbuddy.ts 协议翻译、工具调用、思考档位、每日签到、积分余额；附 DSH 插件面板 | HitMargin/ai-proxy |

### 9.2 Trae Code CN 专用网关（关联上游，非 WB 本体，单独列）

| 项目 | 形态 | 说明 | 仓库 |
|------|------|------|------|
| autumnsentiment/trae2api-cn | Python | Trae CN / Trae Solo CN 反代+协议适配；多账号 round-robin、并发控制、空闲会话回收；auto/env/manual/cli/web-login 认证；Chat/Responses | autumnsentiment/trae2api-cn |
| connectedgraph/trae2api-web | Go 标准库 | TRAE SOLO 多账号反代 + `/admin` Web 面板(:7864)；按积分选号、异常冷却、每日签到、Token 预刷新 | connectedgraph/trae2api-web |
| J1aDong/trae2api | Go/Docker | 早期 Trae→OpenAI 封装（**已停更**，不支持 Trae v1.3.0+ 模型）｜溯源 | J1aDong/trae2api |
| linqiu919/trae2api | Go | 同上 fork（2025-08 归档，39★）｜溯源 | linqiu919/trae2api |
| chismi1017/trae2api | Go | 同上 fork｜溯源 | chismi1017/trae2api |

### 9.3 选型速查（针对「WB 引用 Trae CN」）
- **最轻量、专做 WB+Trae+Qoder**：`yanghuan/wild-work`（双击即跑，托盘+面板）
- **最「白嫖免费额度」友好**：`yetone/magpie`（菜单栏，聚合 WB+Trae CN+OpenCode Zen Free）
- **想自己扩展上游渠道**：`practice019/multi2api`（Go，目录式上游扩展）/ `HitMargin/ai-proxy`（Deno，10 上游）
- **只要 Trae CN 单上游**：`autumnsentiment/trae2api-cn`（Python）/ `connectedgraph/trae2api-web`（Go+面板）

### 9.4 接法：Trae Code CN 如何「引用」网关里的 WorkBuddy（已核实）

> Trae Code CN **桌面端原生支持自定义 OpenAI 兼容端点**（官方文档 docs.trae.cn / docs.trae.ai 确认）。

1. 部署任一聚合网关（wild-work / magpie / multi2api / ai-proxy），把 **WorkBuddy 作为上游之一**，记下它暴露的 `/v1` 地址与 Key。
2. Trae Code CN 桌面端：`设置 → 模型 → 添加模型 → 自定义模型`
   - API 格式：`OpenAI Chat Completions`
   - 自定义请求地址：网关 base_url（如 `http://127.0.0.1:7863/v1`；可开「完整 URL」填 `…/v1/chat/completions`）
   - API 密钥：网关 Key
   - 模型 ID：网关 `/v1/models` 里某个 WB 后端模型（如 `deepseek-v4.1-flash`）
3. Trae 对该自定义模型的请求 → 网关 → 路由到 WorkBuddy 额度。即完成「Trae 引用 WorkBuddy」。

- **限制**：仅 **Trae Code CN 桌面端**支持自定义模型；TraeWork 网页/移动端**不支持**（产品设计限制）。
- **注意**：关闭 Trae 的「Auto 模式」，否则它只调内置模型不走你的 Key。

---

## 10. 融合落地：switch 底座 + 签到助手 + REA 规则 Json（最终交付层）

> 本节把"清单"落到"可交付的产品形态"。详见 `逆向-workbuddy-switch-REA同步.md` 与 `schema/workbuddy-account.schema.json`。

### 10.1 三方分工（同步前提）

| 来源 | 角色 | 关键产物 |
|---|---|---|
| **workbuddy-switch.zip** | **UI + exe 底座**（Tauri/Rust + WebView2 桌面应用） | `wb-switch-rust.exe`；`.wb-switch/accounts.json` / `config.json` / `settings.json` |
| **WorkBuddy签到助手.exe** | **账号导入导出 + 签到引擎** | `wb_refresh_tokens.json` / `WORKBUDDY_ACCESS_TOKEN.txt` / `wb_login_result.json` |
| **REA（MCP）** | **规则 Json 标准**（`SetupClient`） | `name/configPath/configPaths/markerPath/skillPath/format/discoveryPaths/sharedRegistration/quitBeforeSetup` |
| **本清单（第 1–9 节）** | **功能取舍表** | 账号池/网关/签到/监控/互通 的 delta 清单 |

### 10.2 switch 逆向结论（静态）

- 形态：**Tauri 桌面应用**（`tauri`/`webview2`/`tauri_plugin_updater`/`TAURI_INTERNALS` 证据充分）。
- 存储：`WB_SWITCH_HOME`（默认 `~/.wb-switch`）下 `accounts.json`、`config.json`、`settings.json`、`hook-events.jsonl`。
- 凭证：读写 `AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai`；密钥存 `options/secret-storage.xml`。
- 字段：`accountId`、`accessToken/access_token`、`refreshToken/refresh_token`、`expiresAt`、`credits`、`credit_usage`、`credit_expiry`。
- 接口：`copilot.tencent.com`、`/v2/billing/meter`、`/v2/billing/meter/get`、`daily-checkin`。
- UI 词频：账号 153 / 登录 50 / 切换 34 / 签到 17 / 刷新 15 / 积分 9 / 过期 6 / 导出 5 / 导入 4。

### 10.3 统一账号 JSON（同步 REA 规则）

导出结构 = `schemaVersion + clientRules[]（REA SetupClient 兼容）+ accounts[]（switch 字段）`，示例见 `schema/accounts.example.json`。硬约束：

1. `clientRules[]` 字段与 REA `SetupClient` 同名同义；REA 内置的 `workbuddy` 规则可直接复用。
2. `accounts[].client` 必须引用 `clientRules[].name`。
3. 导入按 `accountId` 去重合并；导出默认脱敏 token（显式勾选才含明文）。
4. 签到助手的三类 token 文件必须映射进 `accounts[]`，不得另造格式。

### 10.4 落地路线（二选一）

- **A. 套壳（推荐）**：新建 Tauri 外壳，复用 switch 的 `accounts.json` + `auth.workbuddy.workbuddy-ai` 协议，页面按 10.5 实现，导入导出按 schema 校验。可控、可维护。
- **B. 改成品（不建议）**：patch `wb-switch-rust.exe` 前端资源/hook 进程，脆弱且易崩。

### 10.5 页面结构（承接第 1–9 节能力）

账号列表（积分/过期倒计时）· 一键切换（写回 auth 文件）· 导入导出（schema 校验）· 签到中心（签到/成长任务/连登）· 网关接入（`/v1` 地址+模型+Key，取第 5 节最小可行融合）· 设置（端口/自启/钩子/密钥存储）· 诊断（按 REA 规则检测客户端路径与登录态）。

---

> ⚠️ **合规提示**：以上均为非官方社区逆向实现，违反上游服务条款，存在账号封禁风险，仅供本人授权账号、本机/私有环境学习自用。切勿购买"收费版/卡密版/公益中转版"（多为套壳窃取凭证）。
