# legacy/ — 历史版本（保留，不再维护）

这里是 Tauri 桌面版**之前**的形态，留作参考，不作为交付物。

## `web-console/` — 早期 Node Web 控制台（v0.0.x）

- 纯 Node，无依赖，`node server.mjs` 就跑，端口 7870
- 自带一个小前端（`public/`），能看账号、切号、触发签到
- 后来功能全部搬进 Tauri 桌面版（`src/` + `src-tauri/`），这份就停更了

启动：

```bash
set WB_GATEWAY_KEY=你的api_key
set WB_SIGNIN_EXE=D:\path\to\WorkBuddy签到助手.exe   # 可选，不设就用不了签到按钮
node server.mjs
# http://127.0.0.1:7870
```

> 已经被 `src-tauri/src/signin.rs` 的原生签到取代——**不需要那个 exe 也能签到**，
> 这正是当初重写的原因。

**为什么还留着**：`server.mjs` 里有一份「怎么从面板 API 拉数据、怎么改写
CodeBuddy 授权文件」的最简实现，比 Rust 版好读，排查问题时适合先在这里验证思路。
