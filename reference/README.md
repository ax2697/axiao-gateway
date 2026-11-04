# reference/ — 参考资料

这些不是运行时代码，是**维护时要用到的原始素材**。删掉程序照样能跑，但下次改 UI / 对接口时就没得比了。

| 文件 | 来源 | 用途 |
|---|---|---|
| `switch-assets/index-D0uigd0E.js` | 从 workbuddy-switch（Tauri）通过 CDP 抓的渲染后资源 | 界面视觉对标、命令表反查（1,247,899 B） |
| `switch-assets/index-BAujnMcC.css` | 同上 | 现在 `src/style.css` 的视觉语言就是从这份提炼的（104,417 B） |
| `switch-assets/rendered.html` | 同上 | 真实 DOM 结构，改布局时对照（45,502 B） |
| `panel-README.md` | 便携版 `wb2api` 内核自带 README | 46 个配置字段、`/panel/api/*` 语义的权威说明 |
| `REA-SYNC.md` | 当时和 REA（逆向工程智能体）同步的规则笔记 | `schema/` 里 `clientRules` 的出处 |

## 没有放进来的东西（重要）

| 东西 | 大小 | 为什么不在仓库里 | 怎么拿回来 |
|---|---|---|---|
| `fusion/vendor/` 15 个内核仓库 | 386 MB | 全是第三方 clone，进 Git 会把仓库撑爆，且版权不属于本项目 | `powershell -File scripts/fetch-vendor.ps1` |
| `fusion/tools/`（pyinstxtractor、rustup-init） | 15 MB | 公开工具，可随时下载 | 见 `docs/REVERSE-REPORT.md` |
| `app/vendor/wb-switch/` | 27 MB | 同上，是 switch 的 clone | `git clone` 原始仓库 |
| `wb2api.exe`（网关内核，8.7 MB） | 8.7 MB | 第三方二进制，跟着便携版发布，不重复分发 | 从便携版包里拿 |
| `auths/`、`config.json`、`data/` | — | **含账号凭证，永远不进仓库** | 用户自己登录生成 |

## switch 资源是怎么抓的

`workbuddy-switch` 是 Tauri 应用，前端资源被压缩进二进制，直接解压拿到的是 brotli 流。
当时的做法：

1. 启动 switch，带 `--remote-debugging-port=9222`
2. CDP 连上去，`Network` 域拿所有响应体
3. 对 `index-*.js` / `index-*.css` 做 brotli 解压后落盘
4. 再 `Runtime.evaluate` 取一次 `document.documentElement.outerHTML` 存成 `rendered.html`

完整过程记在 `docs/REVERSE-REPORT.md`，需要重抓时照做。
