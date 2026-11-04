<!-- AXIAO-ECOSYSTEM:START -->
## 🔴 开工先读（总控投递 · 勿手改这一段）

> 🛑 **STOP：先做这 3 件，再动代码（缺一件就是没读铁律）**
>
> 1. **读完本段 + `.axiao/ECOSYSTEM.md` 开头三条**（logo / 铁律 / 三基底版本）
> 2. **写回执**：在 `.axiao/receipt.json` 加一行
>    `{"ironlaws": "<版本号，见下>", "readAt": "YYYY-MM-DD", "by": "AI"}`
>    —— 不写回执 = 视为没读，总控会在「铁律已读台账」点名
> 3. **只做任务队列里的当前那一条**，做完再写 `.axiao/receipt.json` 的 step 回执
>
> ⛔ 不许"按自己的节奏"另起一摊：本项目的下一步由总控按事实派，不由你自选。
> ⛔ 不许在本文件里另写一份「AI 铁律」：只有下面那份是正文。

**本项目属于「阿晓全生态」。动手前先读这两份，否则你会做错方向：**

1. `.axiao/ECOSYSTEM.md` —— 开头就是 **logo / 铁律 / 三基底版本** 三条硬答案，后面有 **「现在能不能上线」闸门表** 和 **「你的任务队列」**（当前只该做哪一条）
2. 若任务队列给了步骤：**一次只做当前那一条**，做完在项目根写 `.axiao/receipt.json`，总控核对后才放行下一条

**🎨 本项目的 logo 在哪 —— 直接看这里，⛔ 不许反问用户：**

本项目**没有专属软件 logo**，用全生态主品牌（Axiao-X 虎首 + 日轮）：

```
D:\360MoveData\Users\Administrator\Desktop\Axiao AI模型\deliver\brand\axiao-x-mark-currentcolor.svg
```

⛔ 不要自己画一枚、不要拿别的软件的 logo 顶上。

**📜 AI 铁律只有一份，⛔ 不许在这个文件里再写一份：**

版本 **v2.34.0** · 全生态唯一一份正文 · 分册 0 份

```
C:\Users\Administrator\.trae-cn\user_rules\ai-ironlaws.md
```

编辑器**每次会话自动加载**它，你不需要复制。
⛔ 本项目 AGENTS.md 里若还留着别的「AI 铁律 / 铁律 N 条」章节，那是历史复制件；
**冲突时一律以总纲为准**，条数不一样不代表你那份更新。要改规则走总纲，不许在这里加。

**🧰 三个基底 skill 是持续更新的，版本不固定：**

| 基底 | 管什么 | 现在是什么版本 |
|---|---|---|
| 晓之编程 | 软件工程流程底座（S0~S9） | **v15.45.36** |
| 晓之奥义 | AI 影视内容生产底座（P0~P5） | **v17.9.4** |
| 晓之前端 | 界面与前端质量底座（七步） | **v2.29.0** |

> 采信时间 2026-11-04 11:11:07。三个都是**持续更新**的付费产品，版本会变。

> ⛔ 版本**不许写死**进你的文档或代码；要最新版就读 `.axiao/ECOSYSTEM.md` 的开头（每次分发刷新）。
> 用哪个、怎么触发：`.axiao/CAPABILITIES.md`

**⛔ 做软件一律照抄 DeepSeek Harness，不许自我创作（用户 2026-10-23 明确指令）：**

基准目录：`C:\Users\Administrator\AppData\Local\Programs\DeepSeek Harness`
落地口径：`D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\参照基准-DeepSeek-Harness.md`
界面怎么拆：`D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\参照基准-DeepSeek-Harness-界面包.md`（54 个包逐个写清管什么/边界/对应我们哪个页面）
架构怎么分层：`D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\参照基准-架构中和.md`（DSH + LobeHub 中和后结论）
抄：目录分层 / 界面按职责拆小 / routes 薄壳 / 接口与实现分离 / 依赖自带 / 交互与信息层级。
不抄：它的品牌、logo、名称、闭源界面文案、打包的二进制、译文。
照它的「怎么组织」，不照它的「长得像它」。**动手前必须打开对应文件看过，不许凭印象。**

**这套划分不是 DeepSeek 一家的偏好**——已对照四家官方文档核实，同一个结构：
微软 VS Code（每个功能一个 `contrib/` + 单一注册文件 + 不许伸手进别人内部）、
Hugging Face Transformers（每个模型一个自包含目录）、
Obsidian（每个插件一个目录 + manifest/main/styles 三件套）、
LobeHub（`packages/builtin-tool-*/` 一个工具一个包）。
出处与原文引文见 `D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\参照基准-DeepSeek-Harness-界面包.md` 第五节。

**判断一个模块该不该独立，用这一条**（比看代码行数可靠）：
把这个模块整个删掉，产品其余部分还能正常跑吗？能 → 它就该独立（大小不论，14 MB 也算一个）；不能 → 先解耦，别先拆文件。

**📦 能用开源现成的，⛔ 不许自己重写（用户 2026-10-24 明确指令）：**

口径：`D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\开源复用优先.md`

| 你要做的 | 先别写，先找 |
|---|---|
| 日期格式化 / 校验 / JWT / 表单 / 图片处理这类基础件 | **先搜 npm / PyPI**；搜不到再写 |
| 抄某开源项目的一段实现 | **先查许可证**；合规则保留原版权头并在 CHANGELOG 注明来源 |

**四步分诊**（每个关键选择落笔前按序走）：**1 找**（功能覆盖 ≥70%，至少试 3 个）
→ **2 验**（近 12 个月有提交 · 有测试 · 有文档）→ **3 法**（许可证合规）
→ **4 用**（改造量 < 自写成本）。

⛔ **许可证红线**：GPL 强传染、AGPL 网络服务也算分发、**无许可证 = 不许用**（最常见误区）。
⛔ **3 个候选都不通过才允许自研**，且必须在 `开发状态.md` 写明「已检索 N 个候选、均因 X 不适用」。
⛔ 交付消息必须带一行：`开源复用：用了 A(许可证)/B(许可证)；自研 C（原因见 开发状态.md）`。**不写 = 没做这条。**

> 两份文件不冲突：**DeepSeek 基准管「怎么组织」，开源复用优先管「不要自己重写」。**
> 先用现成件解决「有没有」，再按 DeepSeek 结构决定「放哪、怎么切」。

**🔑 要登录官网 / 要接统一账号 —— 先读台账，⛔ 不许反问用户账号（用户 2026-10-24 明确批评过）：**

台账：`D:\360MoveData\Users\Administrator\Desktop\电脑总控\docs\账号台账.md`

| 你要的 | 答案在台账里 |
|---|---|
| 演示号手机号 | `13800000001`（`demo_creator`，演示·创作者） |
| 访客演示号 | `13800000002`（`demo_viewer`，演示·访客） |
| 只读管理员号 | 同 `demo_creator`，后台 6 级观察员（只能看，一切写操作都没有） |
| 有密码吗 | **没有**。只走手机号 + 验证码；虚拟号收不到真短信，线上由服务器管理员临时插码，用完即删 |
| 子站要不要自己发账号 | ⛔ **不要**。注册登录只在官网一处，子站只消费身份 |
| 怎么接 | 后端透传 Cookie `axiaoai_user` → 调 `GET https://aixiao-ai.com/api/account/me`；401 跳 `/account?return=…`，403 提示已停用 |

⛔ **这三件事一律不许问用户**：演示号是多少、密码是多少、官网怎么登录。台账里都有。
⛔ **也不许自己造账号、自己写登录页**——那等于长出第二套账号体系。
⛔ 台账是本地私有的，**不许写进代码、commit、日志、交付物**。

**五条最容易搞错的：**

- ⛔ **完工 ≠ 能上线**。写完 `.axiao/online.json` 只进清单。反复询问部署 = 违反本约定
- ⛔ **素材不是依赖**。C/D/E 三盘的约 24000 份 skill 是**查阅参考**，不是要装进你项目的内容
- ⛔ **不改别的项目的代码**。需要别人配合，写「交接需求」交给对方
- ⛔ **越界即停**。只改本目录（含子目录）；要动别的项目先问「确认切换过去吗？」
- ⛔ **不许自我创作**。做软件先照基准，偏离必须在交付说明里写理由
<!-- AXIAO-ECOSYSTEM:END -->
<!-- AXIAO-ECOSYSTEM:END -->
<!-- AXIAO-CAPABILITIES:START -->
## 🧰 三个常驻能力（用前先读能力档案 · 勿手改这一段）

**开工前读 `.axiao/CAPABILITIES.md`**，才知道该用哪个底座、怎么触发、它现在是什么版本。

| 你在做什么 | 用哪个底座 |
|---|---|
| 写界面 / 落地页 / 后台 / 报告 | 晓之前端 |
| 写软件 / 修 bug / 拆任务 / 验收发版 | 晓之编程 |
| 做短剧 / 分镜 / 投喂词 / 成片 | 晓之奥义 |

⛔ **它们是"装"，不是"抄"**：不要把它们的文件复制进你的项目。
⛔ **版本不是固定的**：能力档案里的版本带采信时间；你手上的更新就以本机为准，并报给总控。
<!-- AXIAO-CAPABILITIES:END -->
# AGENTS.md · 阿晓网关助手（axiao-gateway）

> 写给所有在本仓库工作的 AI。**开工前必读。**
> **进度真相源**：[开发状态.md](开发状态.md)（同一事实只在那里定义一次，此处不重复）

## 这是什么项目

**统一网关控制台**（Tauri v2 桌面应用）：把 WorkBuddy / CodeBuddy 网关、账号切换、签到引擎、免费额度抓取、26 家上游预设、Token 与成本统计，熔进一个窗口。

| 项 | 值 |
|---|---|
| 形态 | 桌面应用（Rust 后端 + 原生 HTML/CSS/JS 前端，WebView2） |
| 当前版本 | **v0.9.5**（见 `开发状态.md` frontmatter） |
| 远端仓库 | https://github.com/ax2697/axiao-gateway （Public，main） |
| 协议 | MIT（`LICENSE`） |
| 视图数 | 31 视图 / 102 命令 / 36 Rust 模块 / 59 项能力核对（`docs/COVERAGE.md`） |

> ⛔ `AGENTS.md` 与 `开发状态.md` 是**本机总纲与台账**，随仓库公开，用于让贡献者看懂取舍。
> 与 `xiaozhi-will` 不同——那个项目的 `AGENTS.md` 不上传，本项目**上传**（含版本规则本身）。

## 纪律

- **唯一真相源**：`开发状态.md`。版本号、验收回执、台账只在那里定义一次。
- **版本号四处一致**（发布前逐项核）：① `开发状态.md` frontmatter ② `src-tauri/tauri.conf.json` ③ `src-tauri/Cargo.toml` ④ `package.json`。
  另有三处**展示位**需同步：`src/index.html`（侧边栏 `v0.1.0` + 关于页版本号）、`scripts/mcp-server.mjs`（`serverInfo.version`）、`src-tauri/Cargo.lock`（`axiao-gateway` 条目）。
- **⛔ 禁止提交**：`auths/`、`config.json`、`data/`、`*.axiao-bak`、`*.exe`、`node_modules/`、`src-tauri/target/`、`release/`、`dist/*`（除 `dist/README.md`）、`AI工作区/`。
  `.gitignore` 已覆盖，**但提交前仍要用 `git status` 核一遍**。
- **⛔ 不改别人的仓库**（`third_party/` 与 `reference/` 里的第三方源码一律只读；本机专属配置文件不得外传）。
- **⛔ 一次任务只升一次版本号**：中途可提交留痕，但版本号在任务收尾一次性定。

### 什么时候升版本

| 触发 | 升哪一位 | 例 |
|:--|:--|:--|
| 旧用法失效（改名 / 字段重命名 / 目录重构 / 协议不兼容） | 主 +1 | 三协议改名 |
| 新增能力且向后兼容（加文档 / 加视图 / 加命令） | 次 +1 | 新增「免费额度」页 |
| 只修 bug，不改能力 | 修 +1 | 修签到 401 |
| 改错别字 / 调格式 / 内部重构 / 改用户走不到的内部引用 | **不升** | 调 README 表格对齐 |

> 判据要诚实：**「用户的做法变了」才算感知得到**。过程里的来回不算版本。

## 常用命令

```powershell
# 开发
npm install
npm run dev

# 构建（MSVC 推荐；GNU 需完整 MinGW，产物目录必须纯 ASCII）
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
# 或：npx tauri build --no-bundle

# 发版前泄露自检（退出码非 0 = 禁止打包）
powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1

# 前端语法自检
node --check src/app.js
```

> 详细构建坑见 `docs/BUILD-NOTES.md`；长期维护流程见 `docs/MAINTENANCE.md`。

## 加新能力的顺序

1. Rust 侧加模块 / 命令 → 在 `src-tauri/src/main.rs` 的 `invoke_handler!` **注册**
2. 前端加视图 → `src/index.html` 的 `#nav` 加 `.nav-item[data-view]` + `<div id="view-xxx">` + `src/app.js` 的 `PAGE_META`
3. 需要持久化 → `src-tauri/src/store.rs`（账号）/ `extras.rs`（扩展能力）
4. 改协议字段 → `schema/workbuddy-account.schema.json` 升 `schemaVersion` + 在 `store.rs` 做迁移
5. 更新 `docs/COVERAGE.md`（能力 5 态）与 `docs/ARCHITECTURE.md`
6. 跑 `scripts/preflight.ps1` + `node --check src/app.js`

## 安全红线

- **任何真实 `accessToken` / `refreshToken` / 个人 `api_key` / 本机绝对路径都不许进代码与文档**。示例统一 `sk-REPLACE_ME`。
- `auths/` 是明文凭证，**永远不进仓库、不进发布包**。
- 便携版出厂预置 `api_key`（`sk-Qxg…`）是**所有便携包共用的默认值**：文档里保留是为了让新手照着填能连上，
  但**对外监听（`0.0.0.0`）前必须换掉**。`scripts/preflight.ps1` 已把它列入白名单，不算泄露。
- 平台 Logo 仅用于识别，商标归各公司；收到通知即换成文字标签（`src/index.html` 的 `.logo-img`）。

## 已知边界（别假装没有）

- 网关内核（账号池轮转、熔断冷却、粘性、OpenAI 透传）**由便携版 `wb2api` 提供**，控制台只做配置与观测 → 见 `docs/COVERAGE.md` 的「内核已实现」9 项。
- 非官方社区逆向实现，可能违反上游服务条款，存在限流 / 封禁风险。
- 定时任务目前**只存表不调度**（`docs/ROADMAP.md` v0.2/v0.3）。
- 打包机的应用数据目录 `%USERPROFILE%\.wb-switch\axiao\` 会残留测试账号，新装用户会看到「多出来的号」 → 发版前清空。

## 改版前必看

- 版本规则全文：`AGENTS.md` 上文「什么时候升版本」+ `开发状态.md` §〇
- 发版流程：`docs/MAINTENANCE.md` §1
- 生态调研（为什么做统一）：`docs/PANORAMA.md`
