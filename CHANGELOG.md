# 更新日志

## 0.9.5 — 2026-11-04

### 修复：界面上显示的版本号一直是 0.1.0

exe 的版本信息已经改成 0.9.3 了，但界面左下角和「关于」页仍然显示 **0.1.0** ——
因为这两处是**写死在 HTML 里的**：

```
src/index.html:59    v0.1.0 · 本地运行
src/index.html:893   <code>0.1.0</code>
src-tauri/update.rs:39  current: "0.1.0"
```

版本改到 0.9.x 之后没人回来改这三处，用户看到的就是"怎么还是旧版"。

**版本号只该有一个来源**。现在统一走编译时的 `Cargo.toml`：

- 新增 `app_version()` 命令，返回 `env!("CARGO_PKG_VERSION")`
- 左下角 / 关于页改成占位，启动时由前端填充
- 更新检查的 `current` 也改成 `env!(...)`，否则它会以为自己还是 0.1.0，
  每次都报"有新版本"

验证：左下角已显示 `v0.9.3 · 本地运行`。

## 0.9.4 — 2026-11-04

### 修复：三处「功能做了但交付物没跟上」

功能写得再多，用户手上的包不更新等于没做。这次补三个断点：

**1. `dist/` 里的 exe 停在旧版本**

打包脚本从 `D:\axiao-build\release\` 复制，但没人跑完整流程，
dist 里躺着的是几小时前那个旧 exe。**用户下到的包缺后面所有功能。**

**2. 给用户的说明永远是旧的**

`build.ps1` 第 68/69 行写的是从 `docs/USAGE.txt` 和
`docs/GETTING-STARTED.md` 复制——**这两个源文件根本不存在**，
`Test-Path` 为假，每次构建都静默跳过。

结果：`dist/使用说明.txt` 里没有访问 Key、没有失败降级、没有响应缓存、
没有文生音乐。用户拿到手根本不知道有这些功能，等于白做。

现在把源补进 `docs/`，`使用说明.txt` 加了第十节「0.9.x 加的功能怎么用」，
按「在哪个页面 → 点哪个按钮 → 留空是什么」写，不堆术语。

**3. exe 右键属性一片空白**

`resource.rc` 只有图标和 manifest，**没有 VERSIONINFO**。
属性里版本、产品名、公司全是空的——用户看到一个空白属性的程序，
第一反应是"这东西来路不明"。

补上后读到：

```
文件版本: [0.9.3]        产品名: [Axiao Gateway]
公司:     [Axiao AI]     描述:   [Axiao Gateway]
版权:     [MIT License]
```

### 新增：一键发布脚本

`scripts/release.ps1` 一条命令串起：编译 → 组装 dist → 打源码包 → preflight。

以前这四步是手工的，漏一步就出上面那三个问题（实际发生过两次）。
`-SkipBuild` 可以只重打包不重编译。

> 踩坑记录：Windows PowerShell 5.1 读**无 BOM** 的 UTF-8 脚本会把中文
> 读成乱码，`Copy-Item "docs\新手教程.md"` 会生成一个叫
> `鏂版墜鏁欑▼.md` 的文件。脚本必须存成 UTF-8 **with BOM**。

### 版本号四处对齐 0.9.3

`Cargo.toml` / `tauri.conf.json` / `resource.rc` / `CHANGELOG` 一致。

## 0.9.3 — 2026-11-04

### 新增：音乐生成（对齐豆包桥的多模态能力）

翻聊天记录发现「豆包桥」这件事被问过四次（控制台在哪 / 豆包分本地端 /
看看这个目录 / 我压缩给用户怎么用），但一直没落实到"网关能用上它的全部能力"。

查下来上游预设、9090 端口探测、视频路径适配（`/videos/generations`）其实
早就做了，**缺的是音乐** —— 豆包桥有 `/v1/audio/generations`，我们这边没有对应通道。

现在补上：

- `media::audio_generate` 命令 + 7864 的 `POST /v1/audio/generations`
- 创作页多一个「文生音乐」模式，出结果直接 `<audio>` 试听 + 一键保存

**参数整体透传，不猜字段**：各家音乐接口差异很大（时长、风格、歌词、
是否带人声），写死几个参数反而会挡住上游本来就有的能力。
实测（mock 上游）传 `{prompt, model, duration:30, style:"pop"}`，
上游完整收到这四项，一个不丢。

上游没有这个接口时明确提示「该上游没有 /audio 接口——音乐生成目前主要靠
豆包桥（本地）」，而不是丢一个看不懂的 400。

## 0.9.2 — 2026-11-04

### 修复：源码包一直是从旧副本打的

`make_source_zip.py` 里的路径指向 `Desktop/开源/阿晓网关助手`，那是**另一个目录**，
缺 `access.rs` / `classify.rs` / `breaker.rs`，CHANGELOG 也停在很早的版本。
也就是说之前给出的源码包**跟实际发出去的 exe 不是同一份代码**。

现在 `scripts/make-source-zip.py` 从**当前工作副本**打，并且：

- 版本号从 `Cargo.toml` 读，不手写（手写必然忘记改）
- 打完**自检关键文件**：`resource.o` / `catalog.json` / 前端 / 变更日志，
  缺任何一个直接报失败
- `.o` 不再被扩展名规则一刀切掉——`resource.o` 是编译必需品，
  本机 windres 坏了，没有它连 manifest 都没有

**验证过能编译**：解压到干净目录 → `cargo check` → 44.75s 通过（只有无害警告）。
源码包唯一的验收标准就是"别人拿到能编译"，这条现在过了。

### 统一版本号

`Cargo.toml` 和 `tauri.conf.json` 一直是 `0.1.0`，而 CHANGELOG 已经到 0.9.x。
现在统一成 `0.9.1`（源码包文件名也跟着对上了）。

> 注意：exe 内部嵌的 VERSIONINFO 来自 `resource.o`，改 Cargo 版本号不会更新它
> （那要重新用 windres 编一次资源）。不影响功能，只是右键属性里看到的版本偏旧。

## 0.9.1 — 2026-11-04

### 新增：页面完整性检查（E2E 的第一层）

用户报过好几次「某个页面是空的」。这类问题的根因几乎都能枚举：

1. 侧栏 `data-view="x"` 指向的 `#view-x` 不存在 → 点了白屏
2. JS 里 `$("#foo")` 拼成了 HTML 里没有的 id → 静默拿到 null，后续赋值报错，
   **整块渲染中断**（比白屏更难查）
3. HTML 重复 id → 监听器绑错元素
4. PAGE_META / GUIDE 有条目但没对应视图

`scripts/check-views.py` 一次覆盖全部 31 个视图：视图 ↔ 导航 ↔ PAGE_META ↔
教程 ↔ id 引用，五方对齐。**比截图看一遍可靠**——截图只能看到当前那一页。

已接进 `preflight.ps1`，每次打包自动跑。

**它第一次跑就抓到 3 处死代码**：`#stats2`、`#credit-expiry`、`#guide-api-key`
这三处的容器早就从 HTML 里删了，但 JS 还在查。因为有 `if (el)` 保护所以没崩，
属于"白跑"——尤其 `renderCreditExpiry()` 每次渲染都调一次、每次都直接 return。

### 清理

- 删掉 `renderCreditExpiry` 整个函数及其调用（容器已不存在）
- 删掉 `#stats2` / `#guide-api-key` 两处查询

### 关于「抓平台额度」（原 01-Scrapling）

**不做**，理由写清楚：

- 需要先在各家网页登录，登录态过期就得重来；用户不会为此反复登录
- 每家一个解析规则，页面改一次就坏一次，维护成本高于收益
- 有公开 API 的四家（DeepSeek / Kimi / 硅基流动 / OpenRouter）已经接了真实余额
- 免费额度真正的"数量"在「免费模型目录」里已经有了（35 家 / 410 个模型的
  速率限制和月度预算是**实数**，不是抓来的估算）

替代做法：剩下那些没接口的平台，界面直接标「未开放接口」并给出控制台入口，
用户点一下就能自己去看——**这比抓一个随时会坏的数更诚实**。

## 0.9.0 — 2026-11-04

### 新增：按 Key 的每分钟限速

0.8.0 的访问 Key 只有「每天多少次」，挡的是"慢慢用超"，挡不住"短时间猛刷"。
后者危害更大：几秒钟把上游的 RPM 打满，**你自己也用不了了**。

现在每个 Key 可以单独设「次/分」。超了回 429，提示写清「这一分钟已经请求
N 次，上限 M 次/分」。

实现上是**预占**语义：校验通过就立刻记一次，避免并发时同一个 Key 一起冲过
上限（先查后记的写法在并发下会漏）。

### 新增：健康度感知路由

之前是无脑轮转，一个成功率 30% 的渠道和 99% 的渠道被平均对待。

「本地网关 API」多了一个选路策略下拉：

- **轮转（默认）**：按顺序轮流，多个 Key 平均分摊
- **按健康度**：按最近 300 次调用的成功率排序，最健康的先用；成功率相同时
  平均耗时短的优先

**新渠道排最前面**——它没有任何调用记录，如果永远排在老渠道后面，
一旦有历史数据就再也轮不到它，等于白配。这个细节是从 LiteLLM Router 的
"新部署要能拿到流量"学来的。

### 新增

- `test_access.py` 加一条每分钟限速用例（前 2 次通过、第 3 次 429 且提示按分钟限）
- 单测仍 16 条，八套端到端回归全过

## 0.8.1 — 2026-11-04

### 新增：查看 / 复制完整 Key

0.8.0 里界面只回显 Key 的后 4 位，问题是很明显——**用户没法把 Key 复制给对方**。

现在点一下 `…4b5c` 这一格：

- 展开显示完整 Key（30 个字符，自动折行，不会被省略号吃掉）
- 顺手尝试复制到剪贴板，成功就提示「已复制」
- 剪贴板权限被关时降级为「已显示，手动选中复制」，不打断流程
- 再点一下收起

**完整 Key 是按需单独取的**（`access_key_reveal`），不放进列表接口。
否则整个界面的 JSON 里都是明文密钥，一次截图、一次日志打印就漏了。

## 0.8.0 — 2026-11-04

### 新增：访问 Key 管理（「把网关给别人用」的前置条件）

之前访问 Key 只是 `extras.json` 里的一个字符串数组，而且**前端连入口都没有**——
想给朋友一个 Key，得自己去改 JSON。更要紧的是没有任何约束：发出去之后
对方能无限刷你的上游额度。

现在补三件事：

- **能起名字**：三个月后你还知道哪个 Key 是给谁的
- **能限次数**：每个 Key 每天最多多少次，超了回 429 且提示里说清用量
- **能看到用量**：今日请求数、被拒次数，一眼看出谁用得多

界面在「图像与视频 → 本地网关 API」的缓存条下面：生成 Key、改名、改限次、
停用、删除，都是行内直接改，不用弹窗。**只回显 Key 的后 4 位**，整个界面
不会铺满明文密钥。

兼容：`control.keys` 里如果还是老的纯字符串数组，读的时候自动当成
「没名字、不限次」的 Key，已有配置不用改。

### 修复（测试抓出来的）

- **`/health` 被访问 Key 拦住**：配了 Key 之后健康检查返回 401，监控和探活
  全废。健康检查不含敏感操作，现在明确放行。这个问题是写完测试才暴露的——
  测试脚本用 `/health` 判断服务就绪，配 Key 后永远不就绪，直接把后面三条用例
  全跳过了。

### 新增

- `scripts/test_access.py`：六条验证 —— 没配 Key 全开、带错/不带都 401、
  限次生效（第 3 次 429 并说明用量）、流式通道同样校验、停用的 Key 拒绝、
  老格式配置仍能识别
- 单测 15 → 16（`access::tests`：Key 格式与老格式兼容）

## 0.7.0 — 2026-11-04

### 修复：把所有 400 当成"请求有问题不降级"是错的

之前降级只看状态码：`429 / 5xx / 连接失败` 换渠道，其余不换。但 **400 里混着
三种完全相反的情况**（这是 LiteLLM 把 400 细分成 `ContextWindowExceededError` /
`ContentPolicyViolationError` / `BadRequestError` 的原因）：

| 400 的真实原因 | 该怎么做 | 之前的行为 |
|---|---|---|
| **上下文超长** | 换个窗口更大的渠道**一定能成功** | ❌ 直接失败 |
| 内容策略拒绝 | 换同厂商没用 | ✅ 不降级（碰巧对） |
| 参数不支持 / JSON 写错 | 换十个渠道都一样 | ✅ 不降级 |

上下文超长是**唯一"换个渠道就一定成功"的失败**，之前被我们当成不可降级处理了。
长文档问答、合同审阅、RAG 拼接、多轮客服这些场景会稳定失败。

现在按**错误内容**分类（匹配各家真实措辞：OpenAI 的
`maximum context length is N tokens`、Anthropic 的 `prompt is too long`、
Gemini 的 `exceeds the maximum number of tokens`），上下文超长走降级。

**配套的关键细节**：上下文超长**不让渠道进冷却**——它不代表渠道坏了，
冷却它会让紧接着的正常请求被无谓跳过 30 秒。

### 新增：调用前上下文预检

参考 LiteLLM 的 `enable_pre_call_checks`：**发之前就跳过窗口装不下的渠道**。

之前会把 5 万 token 的 prompt 发给一个 8K 窗口的模型，等它返回 400，再换下一个——
白花一次往返的延迟，部分厂商还按输入计费。

现在内置 50+ 常见模型的上下文窗口（最长前缀匹配，跟 `pricing.rs` 一个套路），
先估请求长度，装不下的直接跳过并在日志里说明。

**估算刻意偏保守（低估）**：真实 tokenizer 拿不到（各家不同、要额外依赖），
高估会误杀本来能成功的请求，那比多试一次更糟。所以中文按 1 字 1 token、
其它按 4 字符 1 token，且**只有明确超出才跳过**，不设收紧系数。

### 新增

- `scripts/test_errors.py`：三条精确验证 —— 上下文超长会降级且不冷却渠道、
  5 万 token 请求一个字节都没发给上游、参数错误如实返回不降级
- 单测 13 → 15（`classify::tests` 用三家真实报错措辞验证分类，
  `modelctx::tests` 验证窗口最长前缀与估算）

## 0.6.0 — 2026-11-04

### 新增：请求追踪 ID

排障时最烦的是"用户说刚才失败了"，而你只有时间可以猜。现在每个请求有一个
`req_xxxxxxxx`，**贯穿三处**：

- **响应头** `X-Request-Id`（客户端能看到，也能塞进它自己的日志）
- **网关日志** 每条前面带 `[req_xxxxxxxx]`，`grep` 一下就是完整链路
- **账本** `ledger.jsonl` 每行带 `requestId`，能和 token/成本/降级链对上

细节：

- **客户端自带 `X-Request-Id` 时优先沿用**——这样能和自己系统的日志串起来；
  没带就生成一个返回在响应头里
- 非法的 id（超长、含控制字符）会被忽略并换成新生成的，避免有人往日志里塞垃圾
- 流式请求同样带头
- 同步链路（收包、路由、写响应）靠 thread-local 自动带上，调用点不用改；
  **异步链路（tokio 里跑的上游请求、流式 pump）显式传**——tokio 会在 worker
  线程之间迁移任务，在那里读 thread-local 会读到别的请求的 id

### 修复

- **错误响应可能被 RST 冲掉正文**：服务端写完 503/413 立刻关闭连接，内核发现
  还有未读数据会发 RST，某些客户端看到的是"连接被中止"而不是我们的错误说明。
  现在写完先 `shutdown(Write)` 再关闭，客户端能读到完整正文和 EOF。
  顺带把「连接数打满后的恢复时间」从 4.1 秒降到 2.2 秒

### 新增

- `scripts/test_trace.py`：六条验证 —— 自动生成、沿用客户端 id、账本对得上、
  日志搜得到、流式带头、非法 id 被忽略
- 单测 11 → 13（`trace::tests` 两条：id 格式与 guard 自动清理）
- `scripts/test_security.py` 容错 RST（这是正确行为，不是缺陷）

## 0.5.0 — 2026-11-04

### 新增：失败自动降级 + 熔断

这是网关最该有、之前却完全没有的能力：**一个渠道挂了，请求就直接把错甩给客户端**。
现在会按顺序把候选渠道都试一遍，谁先成功用谁。

取舍参考 LiteLLM Router 与 Portkey 的做法，并且刻意做了"不降级"的判断：

- **只对 429 / 5xx / 连接失败换渠道**。400、401、403、404、413、422 是请求本身
  的问题，换十个渠道也是同样的错，只会白烧配额 —— 这是 Portkey 的
  `on_status_codes: [429, 503]` 想表达的意思
- **冷却按错误类型区分**（参数按免费额度场景调过，不照抄 LiteLLM 默认值）：

  | 情况 | 处理 | 为什么 |
  |---|---|---|
  | 429 限流 | 立刻冷却 5 秒 | 免费额度打满很常见，换个渠道马上能用 |
  | 401 / 403 | 冷却 60 秒 | Key 不对或没权限，自己好不了 |
  | 404 | 冷却 60 秒 | 地址填错了，重试同一个还是 404 |
  | 5xx / 连接失败 | 连续 3 次才冷却 30 秒 | 偶发抖动不该直接摘掉，那是误杀 |

- **冷却只影响单个渠道**：三条同平台的 Key 挂了一条，另外两条照常服务
- **轮转起点也在动**：降级时"先试谁"不会永远是同一条，多 Key 分摊仍然成立
- 账本新增 `skipped` 字段，记录这次请求跳过了哪些渠道 —— 排障时一眼看出
  是主渠道挂了还是整体都不行

### 新增：熔断状态可见

`GET /health` 返回 `breaker` 数组（冷却中/剩余秒数/连续失败次数/最后错误）；
上游管理页在健康度那一列优先显示「冷却中 29s」+ 原因，并带「立即恢复」按钮。
冷却中的渠道比历史成功率更该被看到——它现在就是不可用的。

### 新增

- `scripts/test_fallback.py`：四条端到端验证 —— 500 自动换渠道、连接被拒自动换、
  连续失败后进冷却、400 不降级、全挂时 2 秒返回错误而不是挂住
- 单测 10 → 11（`breaker::tests::cooldown_rules`）
- `docs/GATEWAY-COMPARISON.md`：与 LiteLLM / Portkey / new-api / one-api /
  Envoy AI Gateway / Bifrost 的能力矩阵，以及每项的取舍理由

## 0.4.0 — 2026-11-04

### 新增：响应缓存

相同请求 10 分钟内直接复用上一次的回答，省 token 也省延迟。接入点在
`chat_proxy::complete`（7864 本地 API 的非流式路径），命中时**不选上游、
不发请求**。

设计上刻意做了几个"宁可少缓存"的取舍，每条都是为了不帮倒忙：

- **温度 > 0 的请求不缓存** —— 用户调高温度就是要每次不一样，
  缓存会让"重新生成"按钮失效，会被当成 bug 报上来
- **带 `tools` / `tool_choice` 的不缓存** —— 工具调用往往伴随外部状态变化，
  返回十分钟前的旧结果比不缓存更糟
- **`n > 1` 不缓存** —— 一次要多个候选，缓存一个没意义
- **流式不缓存** —— 流式的价值就在逐字出，缓存完再拼 SSE 是假流式，是体验退步
- **错误响应不进缓存** —— 否则用户重试十次都是同一个错
- **只在内存，不落盘** —— 缓存里是对话原文，写磁盘等于把聊天记录留在硬盘上
- **键里剔除 `stream`/`stream_options`/`user`/`upstream` 并按键名排序** ——
  客户端的字段顺序一变就全 miss，那等于白做；剔掉 upstream 后同一问题
  在多个上游之间也能复用

实现：LRU + TTL(10min) + 200 条上限（不设上限的内存在长期运行的服务里就是
慢性泄漏）。界面「图像与视频 → 本地网关 API」多了开关、命中统计、清空按钮，
`GET /health` 也返回 `cache` 字段方便脚本观测。

### 新增

- `scripts/test_cache.py`：让假上游数请求次数，直接证明"同一请求真的没打到上游"
  ——相同请求 2 次只打到上游 1 次、温度 0.9 打 2 次、带 tools 打 2 次、
  不同问题各打 1 次且答案不串、字段顺序不同仍算同一请求
- 单测 9 → 10（`cache::tests::hit_miss_and_rules`）

## 0.3.2 — 2026-11-04

### 修复（协议转换里最容易被踩的链路：工具调用）

请求/响应双向转换从 0.1.0 就有了（`chat_proxy.rs` 与 `gateway.rs` 两条路径
都调用 `convert`），但**工具调用这条路是断的** —— 而它是官方 API 一定会拒的地方：

- **`role:"tool"` 没转成 `tool_result`**：Anthropic 只接受 user / assistant
  两种角色，OpenAI 的工具返回消息原样发过去就是 `400 invalid role`。
  现在转成 `{"type":"tool_result","tool_use_id":...}`，并合并进 user 消息
- **assistant 的 `tool_calls` 没转成 `tool_use` block**：多轮工具调用到这里就断了
  （模型看不到自己上次调了什么）。现在把 `arguments` 字符串解析成对象塞进 `input`
- **`tool_choice` 没转**：OpenAI 的 `"auto"` / `"required"` / 指定函数，
  Anthropic 是 `{"type":"auto"}` / `{"type":"any"}` / `{"type":"tool","name":...}`
- **`stop` 是字符串时没转数组**：OpenAI 允许 `"stop":"END"`，Anthropic 只认数组
- **Gemini 请求整个丢了 `tools`**：上游不知道有哪些函数可调。
  现在转成 `tools[].functionDeclarations`
- **Gemini 响应的 `functionCall` 被丢弃**：`gemini_to_openai` 之前只读 `text`，
  模型返回的工具调用直接扔掉、`finish_reason` 恒为 `stop`。
  另外 Gemini 的 `functionResponse` 认的是**函数名**，而 OpenAI 的 tool 消息只有
  `call_id` —— 现在从 assistant 的 `tool_calls` 里建 id→name 映射再回填

### 新增

- `scripts/test_convert.py`：起一个**严格校验**的假上游，行为对齐官方 API
  （role 白名单、`max_tokens` 必填、content 必须是 block 数组、
  `tool_result` 必须有 `tool_use_id`、Gemini 必须有 `contents` 且 role 只能是
  user/model、`functionResponse` 必须有 name）。请求从网关走一圈，
  转换漏东西这里一定红
- 单测 6 → 9：`tool_calls_to_anthropic`、`tool_calls_to_gemini_and_back`、
  `stop_string_normalized_for_anthropic`

## 0.3.1 — 2026-11-04

### 安全加固（本地网关的攻击面）

参考「24-安全审计技能」的 Desktop/Local-IPC、Protocols、Resource-Exhaustion
三篇，逐条对照后修掉的真实问题：

- **请求体大小无上限（严重）**：`serve()` 按客户端声明的 `Content-Length`
  直接 `vec![0u8; length]` 分配缓冲区。本机任何进程发一个
  `Content-Length: 4294967295` 就能让我们去申请 4GB 内存，一个请求打崩进程。
  现在上限 64MB，超了回 `413`，且**在分配之前**判断
- **请求头无长度上限**：`read_line` 本身不封顶，客户端发一条不带换行的
  2MB "头"就能一直往 String 里堆。现在单行 8KB、合计 64KB
- **连接数无上限**：每个连接一个线程，几千个空连接就能吃光线程和内存。
  现在上限 128，超了立刻回 `503` 并关闭
- **排队时间过长导致"服务卡住"**：并发闸门的等待从 20 秒降到 **3 秒**。
  排队期间连接是被占住的，一堆请求一起排队会把连接名额和线程全占满；
  实测 200 个连接压进来后恢复时间从 **18 秒降到 4 秒**
- **读超时 20 秒 → 8 秒**：它决定"被空连接占满后多久恢复"，本机回环请求是
  毫秒级，8 秒已经非常宽松。只约束读，不影响流式响应
- **CSP 从 `null` 改成严格策略**：原来等于不限制，加上 `withGlobalTauri: true`，
  任何注入的脚本都能调用全部 Tauri 命令。现在
  `script-src 'self'`、`object-src 'none'`、`frame-ancestors 'none'`
- **外链打开改走 `ShellExecuteW`**：原来用 `cmd /c start`，URL 会先过一遍 cmd
  词法分析 —— `%PATH%` 会被展开、`&` 会被当命令分隔符。而这些 URL 里有远程
  数据（面板下发的 `jump_url`、更新检查返回的地址）。改完不经过任何 shell 解析
- **删掉 4 个没被编译的僵尸 `.rs`**（`src-tauri/` 根目录下的 `main.rs` /
  `panel.rs` / `media_server.rs` / `chat_stream.rs`）：`mod panel` 实际解析到
  `src/panel.rs`，根目录那几份是旧副本 —— 其中一份还留着直接拼 `cmd` 的旧实现，
  读源码的人会以为那就是在用的代码
- 启动时如果没设访问 Key，日志里明确警告"本机任何程序都能调用这个接口"

### 新增

- `scripts/test_security.py`：把上面这些边界变成回归用例 —— 超大
  `Content-Length` 回 413 且进程不崩、超长请求头不断服务、200 连接压顶回 503
  且 4 秒内恢复、正常请求不被误伤
- `preflight.ps1` 增加 **7 项安全边界检查**：监听地址必须 loopback、
  `vec![0u8; length]` 必须有上限、CSP 不能为空、前端不许 `eval`/`new Function`、
  Rust 不许直接拼 `cmd`/`powershell`、产物不许带 sourcemap、release 必须 strip

## 0.3.0 — 2026-11-04

### 新增

- **免费模型目录（35 家 / 410 个模型）**：从 freellmapi 的 `server/data/freeapi.db`
  与 `server/src/providers/*.ts` 提取，编译期嵌进 exe。每家的**模型数、RPM / RPD /
  TPM / TPD 限额、月度 token 预算、上下文窗口、是否支持视觉与工具、付费单价**
  都是目录里的实数，不是估的。
  - 免费额度页新板块，3 列卡片，可按「官方直营 / 聚合站」筛、按厂商名搜
  - 「加为上游」一键写进 `upstreams.json`：地址、协议、名称自动填好，只差一个 Key
  - 「看模型」展开该家全部模型，带视觉/工具/上下文标记
  - 幂等：同一家重复点只更新那一条，不会堆出一串重复上游；已填的 Key 不会被空值冲掉
  - 带占位符的地址（如 Cloudflare 要 account id）**直接拒绝**并说明原因，
    而不是加一个用不了的死上游
  - 数据来源脚本 `scripts/build-catalog.py`，重跑即可刷新
- **免费额度页不再全是「未开放接口」**：原来那 24 张 `remaining: null` 的空壳卡
  是硬编码追加的噪音，已移除；真实数字改由上面的目录提供

### 修复

- **构建时资源重复链接**：`tauri-build` 本就会用 windres 生成 `libresource.a`
  （含 manifest + 图标），`build.rs` 又链了 `resource.o`，链接器报
  `duplicate resource: type ICON / type MANIFEST`。改成显式二选一：
  默认交给 tauri-build；设 `TAURI_SKIP_WINRES=1` 时 build.rs 自动补链 `resource.o`

## 0.2.0 — 2026-11-04

### 新增

- **真增量流式对话**（`chat_stream.rs`）：以前是「等上游把整段生成完 → 切成 40 字
  一帧发出去」，首字延迟＝整段生成时间，跟不流式没区别。现在上游吐一个分片就解析、
  就转发，首字延迟＝上游第一个 token 的时间。实测（mock 上游每 350ms 吐一片）：
  **首字 7ms / 总耗时 2100ms，帧间隔 350ms 与上游完全一致**。
  - OpenAI 协议：帧原样透传，`reasoning_content` 这类私有字段不丢
  - Anthropic 协议：`content_block_delta` → OpenAI `delta`，工具参数碎片按 index
    攒齐后补一帧
  - Gemini 协议：走 `:streamGenerateContent?alt=sse`，读 `candidates[].content.parts`
  - 超时策略改成「90 秒无新数据才断」，而不是整请求总超时——长文生成不会再被
    180 秒的总超时从中间掐断
- **网关服务日志 + 界面展示**：日志双写控制台与 `~/.wb-switch/axiao/gateway.log`
  （5MB 自动滚动）。记录服务启动、每个请求（方法/路径/状态/耗时）、选了哪个上游、
  流式转发了多少帧 / 多少字符 / 首字耗时 / 结束原因。界面「运行日志」页拆成两块：
  上半「网关服务日志」（客户端请求进来后发生了什么），下半「操作事件」（你点了什么），
  带级别筛选、关键字搜索、8 秒自动刷新、一键清空
- **无界面常驻模式**：`阿晓网关助手.exe --serve-only --port 7864` 只起 HTTP 服务、
  不开窗口不建托盘，适合开机自启或部署到服务器
- **数据目录可用环境变量覆盖**：`WB_AXIAO_DATA=<路径>`，多实例并跑与自动化测试
  互不干扰

### 修复

- **release 版双击闪退 / 启动即退出 `0xC0000139`**：exe 缺 `.rsrc` 段 → 没有
  Common Controls v6 manifest → Windows 按 comctl32 v5 加载 → Tauri 用到的
  `TaskDialogIndirect` 找不到 → **加载期**就失败（`main()` 第一行都不执行）。
  本机 windres 坏、构建必须 `TAURI_SKIP_WINRES=1`，资源得自己链，
  而 `.cargo/config.toml` 里那条 `link-arg` 会静默失效。改用
  `build.rs` 的 `cargo:rustc-link-arg` 链入 `src-tauri/resource.o`
  （只作用于最终 exe，不受依赖包 build script 的工作目录影响）
- **manifest 内容是旧的、格式也不完整**：原来嵌进去的只有一条 Common-Controls
  依赖，没有 XML 声明、没有自身 `assemblyIdentity`、没有 DPI 与 `supportedOS`
  声明——这种情况 Windows 可能直接忽略整份 manifest，于是虽然资源段里"有"
  manifest，`TaskDialogIndirect` 依然找不到。改成标准完整写法后重新生成
  `resource.o`（1548 字节，含 `<?xml?>` 声明、`assemblyIdentity`、
  `requestedExecutionLevel=asInvoker`、`dpiAware`/`dpiAwareness`/`longPathAware`、
  `supportedOS`），实测启动 20 秒无弹框
- **`print!` 会给导入表塞 7 个 C stdio 符号**：`fprintf` / `fwrite` /
  `vfprintf` / `__iob_func` 等，msvcrt 导入从 36 项涨到 43 项，
  `__iob_func` 还是老版 MSVCRT 的兼容符号，等于给启动期埋雷。
  日志改为只写文件，全项目不再使用 `print!` / `println!`
- **流式请求零输出**：原来 SSE 响应发的是 `Content-Length`，长度又是事先算出来的，
  客户端会一直等「收满这么多字节」才渲染。改成 `Transfer-Encoding: chunked`，
  每个分片一到就能被消费
- **Anthropic 流式一帧都收不到**：`convert::to_anthropic` 末尾会主动去掉 `stream`
  字段（避免非流式场景被上游 400），流式通道拿到的是「去掉 stream 的请求」，
  上游按一次性 JSON 回，解析 SSE 时全部跳过。现在 `chat_stream::prepare` 在转换后
  把 `stream` 补回去
- **Anthropic 流式 usage 字段名不对**：上游回 `input_tokens` / `output_tokens`，
  客户端只认 `prompt_tokens` / `completion_tokens`，读出来全是 0。现在统一规范化，
  `total_tokens` 缺失时自动补算
- 并发闸门补上 `429` 的 `Retry-After` 语义说明，错误体带 `type: rate_limit_error`

### 测试

- 新增 `scripts/test_stream.py`：三种上游协议各跑一遍，断言「首字延迟 < 总时长
  一半」「帧间隔贴近上游节奏」「内容拼接正确」「usage 已识别」「有结束帧和
  [DONE]」，并支持 `--concurrency N` 多路并发（8 路实测全部 2.1s 完成，
  墙钟 2142ms，证明是真并行而不是排队）
- 脚本自建临时数据目录（`WB_AXIAO_DATA`），跑测试不会碰用户真实配置

## 0.1.0 — 2026-11-03

### 新增

- Tauri v2 桌面壳：28 个视图
- 账号管理：读便携版 `auths/` + `state.json`，CN/Global 分段
- 添加账号：浏览器登录（`login/start` + `login/poll`）、导入 JSON（`import/cockpit`）
- 账号切换：写回 CodeBuddy 授权文件（自动备份）
- 关联会话 + 会话组
- Token 统计：输入/输出/缓存读/命中率，按日
- 积分统计 + 到期提醒
- 免费额度：WorkBuddy / Trae / Z.AI / BigModel / OpenCode Zen / Kimi / MiniMax / CommandCode
- 签到中心：原生签到 / 成长任务 / 旅行 / 抽奖 / 保活 / 批量签到
- **积分任务页**：面板「积分任务」弹窗的原生版——任务列表（进度 / 奖励 / 状态）、
  逐条「领取」、「全部接受」、「一键完成可领任务」；接口 `growth_tasks` /
  `growth_accept` / `growth_claim`
- **添加账号 → 其他平台**：第三个页签，选平台（Trae / Z.AI / BigModel / MiniMax /
  豆包 / Kimi / Qoder / OpenCode Zen / 硅基流动 等 16 家 + 自定义）→ 自动带出
  Base URL → 填 API Key → 选协议，一步存成网关上游
- **协议与路由 → 对话测试**：选上游 + 模型，真实发一次 `gateway_chat`，
  验证 Key 是否可用，并把这次调用写入「用量 / 成本账本」（此前账本没有任何写入入口，
  两页永远是 0）
- **用量 / 成本的空态**改成解释性文案：说明这两页只统计「经网关转发的请求」，
  并给「复制接入地址」「看本机实际用量（Token 统计）」两个出口
- **WorkBuddy 上游端点改成官方地址**：原来写 `https://copilot.tencent.com` +
  `/v1/chat/completions`，实测 **404**；官方对话端点是
  `https://www.workbuddy.cn/v2/chat/completions`。现在 `endpoint()` 会看
  `base_url` 末尾是否已带版本段（`/v1` `/v2` `/v4` `/api/paas/v4`），带了就只补
  `/chat/completions`，没带才补 `/v1`；老配置里的 `copilot.tencent.com` 启动时
  自动迁移到新地址
- **自动处理「只支持流式」**：WorkBuddy 对非流式请求返回
  `400 Non-stream chat request is currently not supported`，现在会自动带
  `stream:true` 重发一次，并把 SSE 的 `delta.content` 拼回普通
  `chat.completion` 结构，调用方无感（已用真实 token 验证：`HTTP 200`，
  拼回「好」）
- **思考档位从「只存不用」变成真生效**：补上 `minimal / xhigh / max` 三档；
  可全局设默认、也能按上游单独设（key 用真实 `upstream.name`）；转发时
  `anthropic` 协议写 `thinking:{type:enabled,budget_tokens}`（1K/4K/16K/32K/64K），
  其它协议写 `reasoning_effort`（`xhigh`/`max` 降级成 `high`，避免上游 400）；
  调用方自己带了同名字段则不覆盖；默认值改为 `default`＝不干预
- **积分任务加「去完成」**：行为类任务（关注公众号、体验某个专家）进度由官方服务端记录，
  脚本代做不了；现在每行给一个「去完成」，直接打开任务自带的 `jump_url`
  （`workbuddy://experts?expertId=…` 或 `www.workbuddy.cn/work/launch/?deeplink=…`），
  `open_external` 白名单放行 `workbuddy://` 协议
- **模型与档位改为「先平台、再模型」**：WorkBuddy / CodeBuddy 一张卡（面板实时目录，
  带倍率 / 能力 / 思考档位），其余每个上游一张卡，各自按 `{base_url}/models`
  拉取（新命令 `upstream_models`，支持 bearer 与 `x-api-key` 两种鉴权）；
  顶部「全部刷新」一次拉完。拉不到时显示 HTTP 状态与原始响应片段，不假装有数据
- **上游管理补齐增删改**：每行加「编辑 / 停用·启用 / 删除」；编辑复用同一个弹窗、
  ID 锁定不可改；新增 `delete_upstream` 命令；无 Key 的上游标「无 Key」提示
- **其他平台保存后自动联动**：同时写入一条 `client=apikey` 的账号条目，
  「账号管理」立刻能看到（状态标「API Key」，不显示积分/到期，不给「切换」按钮），
  并自动跳到「上游管理」；删除任一侧都会一起清理，避免孤儿数据
- 任务中心：实时任务 + 一键完成
- 模型与档位：实时抓取
- 上游管理：16 家预设 + 自定义
- 协议与路由：三协议 + 接入信息 + 密钥重置
- 成本账本 / 请求日志 / 运行日志 / 钩子
- 网关配置：直读直写 `config.json`（46 字段）
- 上下文压缩（真实生效）、角色统一（developer→system）
- 控制面：Key / 模型白名单（网关强制执行）
- 设备指纹：按账号注入 `x-device-id`
- ZCode off-peak 队列 + 每分钟 tick
- 客户端配置导出：Claude Code / Codex / DSH / 通用 JSON
- MCP 服务端脚本（stdio，无依赖）
- Cloudflare Worker（10 上游）
- 自动更新：16 个组件 + GitHub 代理
- 10 个平台真实 Logo
- Tauri 托盘 + 角色转换

- **修正「豆包」的误导**：豆包 App / 豆包工作 / 网页版都是产品端，**没有 API Key**，
  预设改名为「火山方舟（豆包）」，并按官方流程写清四步（实名 → API 密钥管理建
  `ek-` Key → 开通模型 → 在线推理创建接入点拿 `ep-`）
- **上游支持配置默认模型**：新增 `model` 字段，请求里没带 model 时自动用它；
  火山方舟必须填 `ep-xxxx`，填 `doubao-pro` 会 404。表单、表格、编辑弹窗都支持
- **本机凭据扫描**：「其他平台」页加「扫描本机已登录的客户端」，从
  CodeBuddy CLI / ZCode / DSH / OpenCode / Claude Code / Trae 等配置文件里提取
  token（支持 json / yaml / .env），点「填入」直接用

- **接入「阿晓豆包桥」**：新增预设「豆包桥（本地）」→ `http://127.0.0.1:9090/v1`，
  **不用 API Key**（先跑桥、扫一次码登录豆包账号）。视频端点按桥的
  `/videos/generations` + `GET /videos/generations/{id}` 走，和智谱/OpenAI 的路径分流。
  豆包两条路都保留：**豆包桥**（自用账号、不按 token 付费、易碎）/
  **火山方舟**（官方 API、按量付费、稳）

- **修复「创作页上游下拉是空的」**：并行会话执行 `git stash` 把前端改动收走，
  导致 `renderCreate()` 不存在、`loadAll` 中途抛错，下拉填不上选项。
  已从 `stash@{0}` 取回前端并补回后续改动（扫描本机凭据 / 平台说明 / 豆包桥 /
  模型字段），并给 `main.rs` 补回 `media` / `media_server` / `changelog` / `creds`
  四个模块与命令注册

- **修「添加账号」页签叠加**：`openAdd()` 重置时漏了 `#add-tab-platform`，
  导致「浏览器登录」和「其他平台」两块表单同时显示，关掉重开还会残留
- **账号管理区分两类账号**：WorkBuddy/CodeBuddy（有积分/签到/可切换）与
  API Key / 本地桥（无签到无积分，能力是对话+画图+视频）；后者状态列显示
  「Key 已配 / 缺 Key / 本地桥」，操作列给「创作」直达创作页并自动选中该上游

- **`loadAll` 改为逐块容错**：原来是一串 `await`，任何一处抛错（比如读账号失败）
  后面全部不执行，于是「免费额度」「模型」等页面一片空白且没有报错。
  现在每块独立 try/catch，一处坏了只影响那一块
- **免费额度页补用法**：说明这页是「记账本」+ 三个动作（单卡抓取 / 刷新全部 /
  去上游管理把它加成上游），每张卡多一行「怎么解锁」，抓不到时显示原因而不是空白

- **全功能教程**：「使用说明」页重做成 25 个功能的教程（每个含「这是干啥的 / 三步怎么做 / 注意什么」），
  支持关键字搜；每个页面右上角新增「怎么用」按钮，点了直接跳到该页教程段落

- **并发稳定性（压测过）**：两处真问题——
  ① 本地创作 API 每个请求都新建一个 tokio runtime，并发一高就耗资源；
  ② **本机地址走系统代理**：装了 Clash/v2ray 的用户，`127.0.0.1:9090` 的请求被
  丢给代理，直接 502 / 掉线。现在统一走 `httpc::client_for()`，本机地址强制
  `no_proxy()`；runtime 改成全局共享；并发上限 12，超了先排队、20 秒拿不到返回 429。
  实测（mock 上游，单次 150–600ms，5% 随机 500）：
  并发 1/5 → 95%、并发 20 → 100%、并发 50 → 93%（失败全是上游那 5%），无超时、无连接拒绝

- **UI 对齐参考设计（workbuddy-switch）**：设计令牌直接取自
  `reference/switch-assets/index-BAujnMcC.css`（shadcn light）——
  背景纯白、边框 `oklch(91% .006 264)`→`#e4e6eb`、主色
  `oklch(69.6% .17 162.48)`→`#10b981`、圆角 `.75rem`→12px（按钮 8px）；
  侧栏 210→256px（w-64）加右边框、选中项改浅绿底+品牌绿字；
  页头标题 30→24px（text-2xl）；卡片补 1px 边框 + shadow-sm；
  表头改浅灰底 40px 行高、数据行 46px

- **仪表盘加两张图（纯 SVG，无依赖）**：近 7 天 Token 面积图（渐变填充 + 数据点 + 日期轴）、
  积分构成环形图（`stroke-dasharray` 分段 + 图例 + 中心合计）；参考
  `14-图表设计-DiagramDesign` 的「无阴影、扁平、一图一义」原则
- **微交互**（取自 `20-开源UI组件图库-Galaxy` 样例共性）：按钮按下 `scale(.97)`、
  卡片 hover 轻抬阴影、输入框 focus 3px 绿环；仪表盘用 bento 不等宽两栏
- **新增 `docs/UI-ALIGNMENT.md`**：七个参考包逐个评估——用了什么、为什么不用，
  以及设计令牌与参考 CSS 的逐项对照表

- **同平台多 Key / 多账号**：修复「第二把 Key 覆盖第一把」——原来新增上游的 id 由名称
  生成，同名会直接覆盖；现在自动加序号（`kimi`、`kimi_2`…）。上游列表标出 `#1/2`，
  每行新增「再加一条」按钮（复制平台与地址、Key 留空）。
  选择逻辑从「永远取权重最高」改成**加权轮转**，同一平台多条会轮流分摊；
  本地创作 API 也按同样规则轮转。实测两条 mock 上游交替命中，6 次请求 →
  18080 / 18081 / 18080 / 18081 / 18080 / 18081

- **修「弹窗压满屏幕」**：「添加账号」弹窗没有高度约束，扫描结果 + 平台说明 + 表单全堆一起，
  小窗口下保存按钮被顶出可视区。现在弹窗 `max-height:88vh`、头尾固定中间滚动、加宽到 600px；
  「扫描本机」改成默认收起的折叠块、结果区限高 168px、「平台说明」限高 112px；
  保存按钮吸底

- **修 Token 统计口径不一致**：原来直接采信上游的 `total_tokens`，但它常与
  `input + output` 不符，导致出现「总 271M 比输入 272M 还小」这种不可能的行，
  且卡片合计与明细相加对不上。现在统一为 **总 = 输入 + 输出**（上游给得更大才用上游），
  并把被修正的条数暴露出来；表格底部加**合计行**，卡片数字可直接与它核对；
  表格下方写明口径（缓存读已含在输入内、数据来自 transcript）

- **修「今日/7 天/30 天都一样」**：两个原因——① 窗口原来按 `now - 24h` 算，
  选「今日」会把昨天也算进来；② 本机 transcript 只有 2 天数据，7 天/30 天/全年
  自然同值。现在窗口改**自然日**（今日=今天 0 点起，N 天=含今天共 N 天），
  并在表格下方显示「当前窗口 N 天，实际有数据 M 天（首末日）」，数据不够时明说
  窗口再选大也不会变多。实测本机：今日 306.91M / 7 天及以上 642.87M（仅 11-03、11-04 两天）

- **修「积分统计」整页空白**：导航指向 `#view-creditstats`，但 HTML 里从来没建这个容器
  （只有「积分构成」的 `#view-credits`），点进去所有视图都被取消选中 → 白屏。
  已补容器（明细表 + 累计已用兜底 + 口径说明），并给导航加兜底：
  目标视图缺失时显示「这一页的容器还没建」而不是留白

- **免费额度从 8 个平台扩到 22 个**：原来只列死 8 家，看起来像不支持。现在预设平台全量展示
  （DeepSeek / 硅基流动 / OpenRouter / Trae Intl / Qoder / CodeArts / 火山方舟 / 豆包桥 /
  讯飞星火 / 混元 / 百川 / 零一 / MonkeyCode / DSH），并且**按「上游管理」里的实际配置动态补全**
  ——你加的每一家都会出现在这一页。状态分三档：可抓取（绿）/ 需配 Key（橙）/ 未开放接口（灰，
  不给「抓取」按钮，因为抓也抓不到）
- **接上 4 家真实余额接口**：DeepSeek `/user/balance`、Kimi `/v1/users/me/balance`、
  硅基流动 `/v1/user/info`、OpenRouter `/api/v1/credits`，用「上游管理」里配的那把 Key 直接查

- **创作页重排**：原来所有字段从上往下堆成一条，主按钮被挤到底部。改成左写右调——
  左侧提示词大框（带字数统计）+ 参考素材拖放区（选完列出文件名与类型），
  右侧灰底参数区（模型 / 尺寸 / 时长 / 高级 JSON）+ 整宽主按钮；
  模式从下拉改成**分段控件**（文生图 / 图生图 / 文生视频 / 图生视频 一眼可选）；
  结果区独立成块，生成前隐藏、有结果才展开

- **修「一键完成任务 429 失败」**：上游对批量任务有软限流，面板把它包成 502 透传，
  原来直接把 JSON 甩给用户（`{"error":"Testt tasks upstream soft rate (http429)"...}`）。
  现在：① `request_body` 统一识别 429/Too Many Requests → `RATE_LIMITED`；
  ② 一键完成自动退避重试 3 次（3s → 8s → 15s，共约 26 秒，429 表示未执行故重试安全）；
  ③ 前端提示改成「被上游限流了，等 5~10 分钟再点」，不再贴原始 JSON；
  ④ 任务中心加说明，建议任务多时隔几分钟分几次点

- **修「融合组件」显示缺失**：workbuddy-switch / 签到助手这两个外部 exe 本应用早已原生实现，
  界面却打「缺失」，像是坏了。现在状态分三态——**就绪 / 已内置 / 未安装**，
  内置项说明写「不需要这个 exe」；组件从 3 个扩到 6 个，
  补上「本地创作 API（7864，随应用启动）」「DSH 内核（8317，可选）」「豆包桥（9090，可选）」，
  后两个按端口是否监听判断，页面顶部给出三态含义

- **修「设置页右侧空空荡荡」**：内容区 `max-width:1180` 在小窗口下留出大片空白，
  且「支持工具」表里的长路径把表格撑出横向滚动。现在内容区 `width:100%/max-width:1280`
  + `.view/.card` 显式 100% 宽；路径列超长自动省略号（原来整串铺开）；
  **设置页改成「主表 + 右侧参数栏」双栏**——左边 20 个支持工具，右边自动设置 / CLI 轮换 / 本机信息，
  右侧不再留白；窄屏（<1240）自动回落单列

- **修「图标碎图 + 名字重复」**：`commandcode.png` 其实是 SVG 源码、`deepseek.png` 其实是 ICO，
  扩展名错了浏览器解不出来，就显示成碎图 + alt 文字（于是出现「CommandCode CommandCode」）。
  已按真实格式改名为 `commandcode.svg` / `deepseek.ico` 并更新引用；
  另加全局兜底——任何 logo 加载失败自动降级成首字母色块，不再出现碎图

- **Token 页补趋势图（参考实现用的是 recharts）**：核过 `reference/switch-assets/index-D0uigd0E.js`，
  里面有 110 处 recharts 引用，Token 页本来就有图表。现在补一张纯 SVG 图——
  **柱=总 Token、折线=缓存读、渐变面积**，带入场动画（折线描边 `stroke-dashoffset`、
  柱子逐根 `scaleY`、点按序弹出），悬停显示当天「总/输入/输出/缓存读/次数」。
  切时间范围时图表与表格同步重绘

- **用量页补图表 + 修表格排版**：时间列原来被撑到极宽、其余挤在右边；改成 `table-layout:fixed`
  并给每列固定宽度。上方新增「按上游汇总」横向条形图（按 Prompt+Completion 占比，逐条动画长出），
  空态提示去「对话测试」发一条

- **积分构成重做（更复杂的动态图）**：原来只有两条朴素长条。现在——
  顶部三个**滚动计数**汇总卡（剩余 / 已用 / 使用率）；左侧**大环形**（已用 vs 剩余双渐变，
  `stroke-dasharray` 描边动画 + 整体旋转入场）；右侧每个账号一条**分段堆叠条**
  （橙=已用、绿=剩余，两段错开 60ms 依次长出），显示到期日与使用率，悬停看完整数字

- **全站补动态图表（8 页）**：抽了一套小图表组件（横条 `miniBars` / 环形 `miniDonut` /
  迷你柱 `miniColumns`，均带入场动画），接到——成本账本（按上游）、请求日志（近 24 小时分布）、
  运行日志（类型环形）、积分任务（完成度环形）、账号管理（积分横条）、关联会话（客户端分布）、
  积分统计（按账号消耗）、免费额度（剩余对比）。各处刷新时图表同步重绘
- **页面文案统一重写**：31 个页面的副标题从「名词堆叠」改成「一句话说清这页干什么 + 一个注意点」，
  例如 Token 页写明「不是网关账本」、免费额度写明「抓不到的标未开放接口，不装样子」

- **接入 FreeLLMAPI（来自「中心汇总」盘点）**：它是把 34 家免费厂商 / 635 个端点聚合成
  单个 `/v1` 的开源服务，默认 `127.0.0.1:3001`。已加成上游预设「FreeLLMAPI（本地）」，
  「融合组件」页按 3001 端口探测（三态：就绪 / 未安装），并写了「没启动这条不通」的说明
- **新增 `docs/中心汇总可用清单.md`**：把 39 个目录逐类判过——已用 5 个、可用待做 5 个、
  需大改架构 5 个、明确不引入 10+ 个，每条都写了理由，不做"全都接进来"的假动作

- **导入 FreeLLMAPI 的厂商目录**：从 `server/src/providers/*.ts` 抽出 32 家厂商，剔除搜索/抓取/语音类
  （Brave / Exa / Firecrawl / Linkup / Parallel / SerpSearch / Tavily / Speechify），
  剩 **24 家有对话接口**——写进 `FREEL_API_CHAT`。上游管理加折叠区「更多免费厂商」，
  点一下就能加成上游；免费额度页也补上这些卡片（标「未开放接口」，不假装能抓到数字）

- **对标开源网关补两个真能力**（参考 LiteLLM / New API 的做法）：
  ① **模型别名映射**：客户端写 `gpt-4o`，实际发给上游 `ep-xxxx` / `doubao-pro`。
  「协议与路由」页新增编辑区（一行 `客户端名=上游模型`），保存进 `extras.json.modelMap`，
  网关转发前替换 `body.model`；不填就原样传。
  ② **成本真实计费**：新增 `pricing.rs` 内置 29 个常见模型的参考单价（USD/1M tokens，
  可在 extras.json 的 `prices` 覆盖）。账本按 token × 单价算钱，成本列显示 `$0.0123`，
  汇总里带总花费——之前那列永远是 0。

- **协议真转换（OpenAI ↔ Claude / Gemini）**：之前「协议」下拉只换了 URL 和鉴权头，
  body 还是 OpenAI 形状，接官方 Anthropic / Gemini 会 400。新增 `convert.rs`：
  - OpenAI → Anthropic：system 提到顶层、连续同角色合并、`max_tokens` 缺省 4096、
    tools → `input_schema`、去掉 `stream`
  - Anthropic → OpenAI：`content[].text`/`tool_use` → message + tool_calls、
    `stop_reason` → `finish_reason`、usage 字段对齐
  - OpenAI ↔ Gemini `generateContent`（user/model 角色映射、systemInstruction）
  - 协议下拉新增「Google Gemini（原生 generateContent）」
  **写了 5 个单元测试，`cargo test` 全过**

- **上游健康度**（学 LiteLLM / new-api 的健康检查）：账本新增 `status` 与 `latency_ms` 两个字段，
  每次请求都记真实状态码与耗时；「上游管理」表新增「健康度」列，从最近 300 条账本聚合出
  **成功率 + 成功/总次数 + 平均耗时**（≥90% 绿、≥60% 橙、其余红）。没走过网关的上游显示「无记录」，
  不编造数字

- **接入「安全审计技能」的规则**（24-SecurityAuditSkill）：`preflight.ps1` 新增 5 项检查——
  前端 `eval`/`new Function`、页面远程脚本引用、Rust 侧 shell 调用、CSP 状态、capabilities 是否存在。
  **第一次跑就抓到两处真实问题**：`panel.rs` 与 `update.rs` 各自拼了一次系统外壳命令。
  已合并成唯一入口 `openuri.rs`（强制 http/https、转义引号、去换行），并把它从 shell 检查里单独放行
- **说明：freellmapi catalog 没能导入**——它的 635 个端点存在服务端 SQLite（`data/freeapi.db`），
  仓库里没有；`api.freellmapi.co` 也不开放匿名目录。所以额度页对这些厂商仍是「未开放接口」，
  不编数字

- **本地 API 支持对话转发**（本轮最重要）：7864 端口新增 `POST /v1/chat/completions`，
  客户端把 Base URL 指过来就能用。链路复用已有能力：模型映射 → 协议转换（Claude/Gemini）
  → 上游请求 → 转回 OpenAI 形状 → 按单价表记账。`stream:true` 时先取整段再合成 SSE
  （role 帧 + 40 字切片 + finish + [DONE]，协议兼容但不是真增量）。
  **已用 mock 上游实测**：非流式返回标准 `chat.completion`（含 usage），流式返回
  `text/event-stream` 且分帧正确

### 修复（发布前安全审查）

- **积分统计全是 0.00**：上游没有开放「按日积分消耗」接口（试过
  `/v2/billing/meter/usage` 等 5 个全 404），switch 写的那份
  `official_usage_cache.json` 31 天也全是 0。改为显示每个账号的**累计已用**
  （总额 − 剩余，来自 `data/state.json`）+ 剩余 + 到期，并在表尾说明口径；
  缓存将来有非零数据时自动切回按日显示。
- **Token 统计恒为 0**：transcript 里的 `timestamp` 现在是 **Unix 秒数字**，
  旧代码只按字符串截前 10 位取日期，`1793704318` 解析失败后 `continue`，
  整份 transcript 的 `/message/usage` 全被丢掉。改为数字/ISO 字符串双兼容
  （含毫秒容错、本地时区换算），并把「全部区间」的过滤条件修正为
  `days > 0` 才生效。
- **浏览器登录链接被截断（错误码 100034A）**：授权链接是
  `.../login?platform=CLI&state=xxx`，`open_external` 用
  `cmd /C start "" <url>` 打开，`&` 被 cmd 当成命令分隔符，导致 `state`
  整段丢失、登录页报「登录链接不完整」。改成 `raw_arg` 手工加引号整段包裹，
  并加了 http/https 白名单与换行过滤。同时「获取授权链接」后改为自动拉起浏览器。
- **便携版目录改为自动探测**：原来 `panel.rs` / `store.rs` 里硬编码了开发机的
  `D:\360MoveData\...` 路径，换台机器就找不到便携版。现在按
  `WB_GATEWAY_HOME` → exe 同级 `WorkBuddy网关-便携版` → exe 上级 → exe 所在目录
  → 桌面 的顺序查找。
- **新增 `scripts/preflight.ps1`**：发版前扫明文密钥 / 真实 token / 个人路径 /
  账号数据，非 0 退出即禁止打包。

### 说明：文档里的 `sk-QxgVEFVZM6Pg8Z6MfeiCA6TS`

这是便携版**出厂预置**的 `api_key`（面板和 `/v1` 接口共用），写在便携包的
`config.json` 和 `使用说明.txt` 里，属于开箱即用的功能配置，不是个人凭证，
所以文档里保留它，用户照着填就能进面板。

注意：它是所有便携包共用的同一个默认值。**只在 `127.0.0.1` 本机用是安全的；
一旦改成 `0.0.0.0` 对外，必须先换成自己的随机串**，否则别人拿这份说明就能用你的网关。
`scripts/preflight.ps1` 已把该值列入白名单，不会误报；换过的其他 `sk-` 仍会被拦。

### 已知

- 需 Rust 工具链编译才能自行出包；`release/` 里的二进制不进 Git
- 部分能力依赖外部 Key / 发布源 / Cloudflare 账号
- 应用数据目录 `%USERPROFILE%\.wb-switch\axiao\` 与开发机共用时会读到测试账号，
  打包/换机前先清空该目录（preflight 会提示）
