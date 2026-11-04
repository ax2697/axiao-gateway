# UI 对齐记录：参考来源与取舍

> 目标：把界面从「能用」做到「像成品」。
> 原则：**能从参考里取值的就去取值，不能用的写清楚为什么不用**，不编造"已集成"。

## 一、基准：workbuddy-switch 的真实样式

`reference/switch-assets/index-BAujnMcC.css`（switch 打包后的完整样式表，104 KB）是 shadcn light 主题，
本项目的设计令牌全部从这里取，不是凭感觉调的：

| 令牌 | 参考原值 | 换算 | 用在哪 |
|---|---|---|---|
| background | `oklch(100% 0 0)` | `#ffffff` | 页面底色 |
| card | `oklch(100% 0 0)` | `#ffffff` | 卡片 |
| border | `oklch(91% .006 264)` | `#e4e6eb` | 卡片/表格/输入框边框 |
| foreground | `oklch(21% .01 264)` | `#1f2430` | 正文 |
| muted-foreground | `oklch(52% .012 264)` | `#6b7280` | 次级文字 |
| muted | `oklch(96% .004 264)` | `#f5f6f8` | 表头底、hover 底 |
| primary | `oklch(69.6% .17 162.48)` | `#10b981` | 主色（按钮/选中态/图表） |
| destructive | `oklch(57.7% .245 27.325)` | `#ef4444` | 危险操作 |
| radius | `.75rem` | `12px`（按钮 8px） | 卡片/按钮/输入框 |
| text-2xl | `1.5rem` | `24px` | 页头标题 |
| 侧栏 | `w-64` | `256px` | 左导航 |

字体栈也照抄参考的 `--font-sans`：`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, …`。

## 二、七个参考包，逐个评估

| 包 | 体积 | 是什么 | 用了吗 | 结论 |
|---|---|---|---|---|
| `20-开源UI组件图库-Galaxy` | 20 MB | 3800+ 个独立 HTML 组件样例（按钮/卡片/输入/通知/加载） | ✅ 部分 | 样例风格差异太大，不适合整包引入；取其共性：**按下 `scale(.97)`、`transition .16s`、输入框 focus 3px 光晕、卡片 hover 轻抬**，已写进 `style.css` |
| `14-图表设计-DiagramDesign` | 31 MB | 45 种图表的设计规范（自包含 HTML+SVG） | ✅ 原则 | 采用它的「No shadows · 扁平 · 一图一义」：仪表盘两张图是**纯 SVG、无阴影、无网格线**，只保留基线 |
| `23-设计技能合集-AwesomeDesignSkills` | 12 MB | 数十种设计风格 skill（bento / clean / corporate…） | ✅ 布局 | 仪表盘改用 **bento 思路**：`1.35fr / 1fr` 两栏不等宽，小屏降为单列 |
| `10-三维UI组件库-ThreeUI` | 89 MB | 3D 组件站（Next.js 应用，带 login-free 目录） | ❌ 未用 | 是完整应用不是 CSS 片段；它的 3D 依赖 canvas/WebGL，引入等于给工具站塞一个渲染器。**工具类界面用 3D 只会拖慢** |
| `26-免费模型插件-DSHOurFreeModel` | 127 MB | DSH 插件：免 Key 用 DeepSeek/Kimi | ❌ 未用 | 它跑在 **DSH 内核**里（`127.0.0.1` 本地服务），不是公开 API 地址；没有可填进「上游管理」的 Base URL。已在对话中说明 |
| `31-DeepSeek插件框架-DeepSeekHarness` | 178 MB | DSH 插件框架 + 品牌规范 | ⚠️ 参考 | 读了它的 BRAND_GUIDELINES 排版规则（字号阶梯/行高/留白），与我们采用的 shadcn 令牌一致，未引入其代码 |
| `35-开发者路线图-DeveloperRoadmap` | 42 MB | roadmap.sh 的路线图数据（JSON） | ❌ 未用 | 内容是「学习路线」，与本工具的功能无关；做成页面只会增加噪音 |

## 三、这次实打实改了什么

1. **设计令牌**全部换成参考值（见上表），底色从灰改白、品牌绿从 iOS 绿改翡翠绿。
2. **布局**：侧栏 210→256px 带右边框；页头 30→24px；内容区 1120→1180px、上边距 46→36px。
3. **卡片**：补 `1px` 边框 + `shadow-sm`，圆角统一 12px，`overflow:hidden` 让表头不溢出圆角。
4. **表格**：表头浅灰底 40px、数据行 46px，hover 仅极浅变化（原来是 #fafafa 一片）。
5. **微交互**（Galaxy 取共性）：按钮按下缩放、卡片 hover 轻抬、输入框 focus 环。
6. **仪表盘新增两张图**（纯 SVG，无依赖）：
   - 近 7 天 Token 面积图（渐变填充 + 数据点 + 日期轴）
   - 积分构成环形图（`stroke-dasharray` 分段 + 图例 + 中心合计）

## 四、没有做的事，以及为什么

- **没引入组件库**：Galaxy 是样例集不是库；ThreeUI 是应用。引入任何一个都要带构建链，而本项目是「单 exe 免安装」，多一个依赖就多一份坏掉的可能。
- **没做 3D**：3D 组件适合展示型站点，工具类界面里 3D 只增加 GPU 开销。
- **没做图表库**：ECharts/Chart.js 会带来 200KB+ 体积；两张图用 60 行 SVG 就够，且能跟着主题色走。

## 五、验收方式

```powershell
# 1. 起应用，看仪表盘两张图有没有数据
# 2. 鼠标按住任一按钮 —— 应有轻微下压反馈
# 3. 鼠标划过卡片 —— 阴影应加深，不是位移
# 4. 聚焦任一输入框 —— 应出现 3px 绿色光晕
```

颜色/尺寸可逐项对照第一节表格核对；若某处仍不对，去 `reference/switch-assets/rendered.html`
量真实 DOM 的 computed style，而不是继续猜。
