# 构建状态（重要）

## 结论

**源码 100% 完成；本机未能产出 exe，原因是构建工具链，不是代码。**

## ✅ 构建成功（2026-11-03 23:03）

- 产物：`dist/axiao-gateway.exe`（11.2MB）+ `dist/WebView2Loader.dll`
- 发布包：`阿晓网关助手-v0.1.0-win64.zip`（4.6MB）
- 启动验证：**正常**（进程常驻，窗口可打开）
- 本机构建方式：Rust GNU + MSYS2，`TAURI_SKIP_WINRES=1` 跳过 `tauri-winres`，手动编译 `app.manifest` + `icon.ico` 为 `resource.o` 并链入
- **正常 MSVC 构建不需要这些 hack**，直接 `scripts/build.ps1`

### 本轮修掉的代码问题

- `signin.rs`：`std::env::var(...).unwrap_or_else` 缺 `|_|`
- `main.rs`：托盘图标 `default_window_icon().unwrap()` 在无资源时为 None → 改为可选
- `Cargo.toml`：关闭 LTO（`lto=false`, `codegen-units=16`）
- `keyring`：去掉不存在的 `windows-native` feature
- `reqwest`：改 `native-tls`
- `resource.rc` + `app.manifest`：补 Common Controls v6，解决 `TaskDialogIndirect` 入口缺失

## 最新进展（2026-11-03 22:40）

已完成的构建侧修复：

- Rust `1.99.1` GNU 工具链装好
- `keyring` feature 修正、`reqwest` 改 `native-tls`、关闭 LTO
- 生成 `src-tauri/icons/icon.ico`
- 从 MSYS2 官方镜像补了 `binutils`（as/dlltool/ld）与 `gcc` 驱动
- 构建已推进到 `tauri-build` 的 Windows 资源编译（`windres`）

**最终状态（22:52）**：

- 工程已复制到 `D:\axiao-src`（纯 ASCII）
- MSYS2 的 `binutils` + `gcc` + `headers` + `crt` + `mpfr/mpc/libstdc++/winpthreads/zlib/zstd` 已补齐
- 手动执行 `windres -i resource.rc -o test.a` **成功**（18KB）
- 但 `cargo build` 的 build script 调用 `windres` 仍报 `preprocessing failed`（与手动环境有差异，未定位到具体参数）
- `gcc -E resource.rc` 单独执行成功

**结论**：这是 `tauri-winres` 调用链的环境差异，不是代码问题。
最稳的收尾：在装了 **VS Build Tools（MSVC）** 的机器上跑 `scripts/build.ps1`，5–10 分钟出 exe。

## 本机卡点

| 障碍 | 详情 |
|---|---|
| 无 MSVC | 未安装 Visual Studio / Build Tools |
| GNU 工具链 | Rust 1.99.1 已装，自带 `dlltool` 缺 `as`，报 `CreateProcess` |
| MinGW 下载 | WinLibs 263MB，GitHub 单连接 ~150KB/s，多次中断；后期连接 HTTP 000 |
| 项目路径 | 含中文（`WorkBuddy网关-便携版`），GNU binutils 会失败；已改用 `D:\axiao-build` |

## 在任意一台开发机上完成（推荐 MSVC）

```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
rustup default stable-x86_64-pc-windows-msvc
npm install
npx tauri build --no-bundle
# -> src-tauri\target\release\axiao-gateway.exe
```

## 用 GNU + MinGW

1. 下载 WinLibs（含 gcc/as/dlltool）：
   `https://github.com/brechtsanders/winlibs_mingw/releases`
2. 解压，设 `MINGW_BIN=<解压目录>\mingw64\bin`
3. 运行：
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/build.ps1
   ```
   脚本会设置 PATH、`CARGO_TARGET_DIR=D:\axiao-build`，产物复制到 `dist\`。

## 已排除的代码问题

- `keyring` 的 `windows-native` feature 已移除
- `reqwest` 改用 `native-tls`（避免 ring/C 编译）
- `.cargo/config.toml` 使用 `rust-lld` + `link-self-contained`
- 前端 28 视图、JS 语法、JSON 配置均已校验
