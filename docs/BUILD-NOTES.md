# 构建排错

## 推荐：MSVC 工具链

```powershell
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools
rustup default stable-x86_64-pc-windows-msvc
npm install
npm run build
```

## GNU 工具链（不装 VS）

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
rustup default stable-x86_64-pc-windows-gnu
```

项目已带 `.cargo/config.toml`：

```toml
[target.x86_64-pc-windows-gnu]
linker = "rust-lld"
rustflags = ["-C", "link-self-contained=yes"]
```

### 坑 1：`dlltool.exe: program not found`

把工具链的 `bin` 和 `self-contained` 加进 PATH：

```powershell
$tc = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu"
$env:PATH = "$tc\lib\rustlib\x86_64-pc-windows-gnu\bin;$tc\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;$env:PATH"
```

### 坑 2：`dlltool ... CreateProcess`

Rust 自带的精简 dlltool 可能缺 `as`。装完整 MinGW（WinLibs）：

```
https://github.com/brechtsanders/winlibs_mingw/releases
选 winlibs-x86_64-posix-seh-gcc-*-mingw-w64ucrt-*.zip
解压后把 mingw64\bin 加进 PATH（或设 MINGW_BIN）
```

同时把产物目录指向纯 ASCII 路径（见下）。

GNU binutils 生成 import lib 时无法处理中文路径。
**把构建产物目录指向纯 ASCII 路径**：

```powershell
$env:CARGO_TARGET_DIR = "D:\axiao-build"
cargo build --release
```

产物在 `D:\axiao-build\release\axiao-gateway.exe`。

或把整个项目复制到 `D:\build\axiao-gateway` 再构建。

### 坑 3：`keyring` 报 feature `windows-native` 不存在

`keyring 2.x` 的 Windows 后端是默认的，不需要该 feature。`Cargo.toml` 应为：

```toml
keyring = "2"
```

### 坑 4：`tauri build` 卡在 NSIS/WiX

只要 exe 不打安装包：

```bash
npx tauri build --no-bundle
```

### 坑 5：`windres: preprocessing failed`（本机 GNU 构建真实踩过）

`tauri-build` 会无条件调用 `windres` 编译 `resource.rc`。如果本机 MinGW 的
`cc1.exe` 缺 DLL（`mpfr` / `mpc` / `isl` / `gmp` / `zstd` / `libiconv` 之类），
这一步会挂，错误只显示 `preprocessing failed`，手动跑同样的命令却可能成功
（因为手动跑时 PATH 不同）。

#### 5.0 先定位缺哪个 DLL（别猜）

windres 的报错只有一句 `preprocessing failed`，真正原因要自己查。两个手段：

```powershell
# ① 看 windres.exe 的导入表里缺哪些 DLL
$pe = "D:\axiao-build\msys2\mingw64\bin\windres.exe"
$t  = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($pe))
[regex]::Matches($t,'[A-Za-z0-9_\-\.]+\.dll') | ForEach-Object { $_.Value } | Sort-Object -Unique
```

```powershell
# ② 官方验证法：cc1 能不能跑
"" | & "D:\axiao-build\msys2\mingw64\bin\gcc.exe" -E -x c -
# 报错 0xC0000135 = 缺 DLL；报 0xC0000005 = ABI 不兼容（见 5.1）
```

#### 5.1 ⚠️ ABI 混用陷阱（本机真实踩过，坑中坑）

把**别处拷来的 `libintl-8.dll`** 丢进 `mingw64\bin` 后，windres 能启动了
（`--version` 有输出），但一编译 `.rc` 就崩：

```
windres.exe: ... exited with code: 0xc0000005
```

原因：**Git 自带的 `ucrt64\bin\libintl-8.dll` 是 UCRT ABI，而 MSYS2 mingw64 是
msvcrt ABI** —— 两者混用会内存访问违例。

> ⛔ **规则**：只往 `mingw64\bin` 放**同一个 ABI** 的 DLL。
> Windows 自带的 `C:\Program Files\Git\ucrt64\bin` 是 UCRT，**不能**给 MSYS2 mingw64 用。
> 同理，rustup 的 `libwinpthread-1.dll`（GNU/mingw ABI）也不要往 MSYS2 目录里塞。

正确做法是**从 msys2 官方源取同 ABI 的包**（`repo.msys2.org` 可达）：

```powershell
# gettext 提供 libintl-8.dll（msvcrt ABI）
Invoke-WebRequest "https://repo.msys2.org/mingw/mingw64/mingw-w64-x86_64-gettext-runtime-1.0-1-any.pkg.tar.zst" -OutFile rt.pkg.tar.zst

# 解包（Windows 自带 tar 不支持 zstd，用 python）
python -m pip install zstandard
python -c "import zstandard,tarfile
d=zstandard.ZstdDecompressor()
with open(r'rt.pkg.tar.zst','rb') as f, open(r'rt.tar','wb') as o: o.write(d.stream_reader(f).read())
tarfile.open(r'rt.tar').extractall(r'rt', filter='data')"

# 拷进 mingw64\bin
Copy-Item rt\mingw64\bin\libintl-8.dll  D:\axiao-build\msys2\mingw64\bin\
```

`libwinpthread-1.dll` / `libgmp-10.dll` 同理，从 `libwinpthread-git-*` / `gmp-*` 包取。
**别从 rustup 拷**（虽然也是 GNU ABI，但版本可能与 MSYS2 的 gcc 不匹配）。

#### 5.2 修好之后：cargo 里仍然失败，手工却成功

即使 `gcc -E` 通过、windres 手工跑通，`cargo build` 里仍可能报
`windres: preprocessing failed` —— 因为 **build script 的环境与你手工的 shell 不同**。

`embed-resource` crate 支持用环境变量指定资源编译器（**变量名里的 TARGET 带连字符**）：

```powershell
# ✅ 正确：TARGET = x86_64-pc-windows-gnu
$env:RC_x86_64_pc_windows_gnu = "D:\axiao-build\msys2\mingw64\bin\windres.exe"
$env:RC = $env:RC_x86_64_pc_windows_gnu
```

> ⛔ 写成 `RC_x86_64_pc_windows_gnu`（全下划线）**不生效** —— 源码里查的是
> `RC_$TARGET` 与 `RC_${TARGET//-/_}` 两个名字，后者才是下划线版，但**优先匹配带连字符的那个**。

#### 5.3 首选解法（推荐，一劳永逸）

装**完整 WinLibs MinGW**，确保 `mingw64\bin` 下有全部运行库 DLL。
验证方式：

```bash
printf '' | gcc -E -x c - > /dev/null && echo "cc1 OK"
```

#### 5.4 兜底解法（本机当前出货方式）

跳过 windres，改为**手工编资源对象 + 链进链接器**：

1. 让 tauri 生成 rc（跑一次失败的 build，rc 会留在 OUT_DIR），再手工编译：
   ```powershell
   $out = "D:\axiao-build\release\build\axiao-gateway-XXXX\out"
   & "D:\axiao-build\msys2\mingw64\bin\windres.exe" `
       --input "$out\resource.rc" --output "D:/axiao-build/resource_full.o" `
       --include-dir "$out" --output-format=coff
   ```
2. 给 `tauri-build` 打个本地补丁（**只在你的 registry 里，不进仓库**）：
   `~/.cargo/registry/src/*/tauri-build-2.x.x/src/lib.rs`，找到**调用 winres 的那处**
   `if target_triple.contains("windows") {`，改成
   `... && std::env::var_os("TAURI_SKIP_WINRES").is_none()`
   > ⚠️ 该文件里有**多处** `target_triple.contains("windows")`，只改**调用 winres 的那一处**
   > （`res.compile()` 附近）。改错地方会连带影响 webview runtime 的资源复制。
3. 清掉该 crate 的指纹后重建：
   ```bash
   rm -rf $CARGO_TARGET_DIR/release/.fingerprint/tauri-build-*
   TAURI_SKIP_WINRES=1 npx tauri build --no-bundle
   ```
4. 因为跳过了 windres，资源（含 manifest）得自己链进去，在根 `.cargo/config.toml` 加：
   ```toml
   [target.x86_64-pc-windows-gnu]
   linker = "rust-lld"
   rustflags = ["-C","link-self-contained=yes","-C","link-arg=D:/axiao-build/resource_full.o"]
   ```

**注意**：跳 windres 后如果不链 `resource.o`，exe 双击会弹
`无法定位程序输入点 TaskDialogIndirect`（缺 Common Controls v6 manifest）。
这就是坑 6。

### 坑 6：`TaskDialogIndirect` 入口点错误 / 双击闪退

症状：进程启动即弹框 "无法定位程序输入点 TaskDialogIndirect 于动态链接库
comctl32.dll 上"，或直接闪退。

原因：exe 没有嵌入 manifest，Windows 用 comctl32 v5 加载，Tauri 的对话框 API 找不到符号。

修法二选一：

- **正常路线**：别跳 windres，`tauri-build` 默认会嵌入带 Common Controls v6 +
  per-monitor DPI 的 manifest，什么都不用做。
- **GNU 兜底路线**：用仓库里的 `src-tauri/app.manifest` + `src-tauri/resource.rc`
  手动 `windres` 出 `resource.o`，再按坑 5 第 4 步链进去。

### 坑 7：`WebView2Loader.dll` 缺失

GNU 构建的 exe 依赖 `WebView2Loader.dll`，必须和 exe **同目录**。从构建目录里捞：

```bash
find $CARGO_TARGET_DIR -name WebView2Loader.dll
# 一般在 target/release/build/webview2-com-sys-*/out/x64/
```

`scripts/build.ps1` 会自动找并复制到 `dist\`。MSVC 构建静态链接，不需要这个 DLL。

### 坑 8：release 构建卡死（LTO）

`Cargo.toml` 里 `lto = true` + `codegen-units = 1` 在本机会出现 rustc 空转
（20 分钟 CPU 0.015s，没有任何输出）。已经改成：

```toml
[profile.release]
panic = "abort"
codegen-units = 16
lto = false
opt-level = "s"
strip = true
```

体积 11.2MB，够小，不要为了再省 1MB 去开 LTO。

### 坑 9：release 版双击没反应 / 进程秒退 `0xC0000139`

**症状**：debug 版跑得好好的，`cargo build --release` 出来的 exe 双击没反应；
命令行启动立即退出，退出码 `3221225785`（`0xC0000139`，STATUS_ENTRYPOINT_NOT_FOUND）。
在 `main()` 第一行写标记文件也不会生成 —— 说明根本没执行到 `main`。

**原因**：exe **没有 `.rsrc` 段**。没有资源段 = 没嵌 manifest，Windows 按
comctl32 v5 加载，Tauri 用的 `TaskDialogIndirect` 在新版才有的导出找不到，
**加载期**直接失败。

**诊断三步（别猜）**：

```bash
# ① 退出码 0xC0000139 = 加载期找不到入口点，不是运行时错误
# ② 看有没有资源段
objdump -h app.exe | grep rsrc        # 空 = 没有 .rsrc
# ③ 拿一个"能跑"的旧版本对比
objdump -h old.exe | grep rsrc        # 正常应有 .rsrc，约 0x4800 字节
```

**为什么会踩**：本机 windres 坏，构建必须 `TAURI_SKIP_WINRES=1`（坑 5）。
跳过后资源得自己链，`.cargo/config.toml` 里那条 `link-arg` 一旦漏了或路径写错，
就会**静默产出"能编译、不能运行"的 exe**，报错只体现在退出码上。

**修法（仓库已落地）**：在 `src-tauri/build.rs` 里用 `cargo:rustc-link-arg`：

```rust
let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
let res = std::path::Path::new(&manifest_dir).join("resource.o");
if res.exists() {
    println!("cargo:rustc-link-arg={}", res.display());
}
```

> **为什么不能写在 `.cargo/config.toml` 的 `rustflags` 里**：
> rustflags 对依赖 crate 的 build script 编译也生效，那些编译单元的工作目录
> 不是 `src-tauri/`，相对路径找不到文件（实测报 `rust-lld: could not open
> 'resource.o'`）；改用绝对路径又会把某台机器的路径写死进仓库。
> `rustc-link-arg` 只作用于本 package 的可执行文件，正好。

`src-tauri/resource.o` 必须入库（`.gitignore` 已加 `!src-tauri/resource.o`）。
重新生成：

```bash
windres --input src-tauri/resource.rc --output src-tauri/resource.o \
        --include-dir src-tauri --output-format=coff
```

#### 资源只能链一次（两条路二选一）

`tauri-build` 自己也会用 windres 生成 `libresource.a`（里面已经有 manifest 和图标）。
如果同时又在 `build.rs` 里链 `resource.o`，链接器会报：

```
rust-lld: error: duplicate resource: type ICON (ID 3)/name ID 1/language 1033,
        in src-tauri/resource.o and in .../out/libresource.a
rust-lld: error: duplicate resource: type MANIFEST (ID 24)/name ID 1/...
```

所以现在是显式二选一（逻辑在 `src-tauri/build.rs`）：

| 场景 | 怎么构建 | 资源从哪来 |
|---|---|---|
| windres 正常 | `cargo build --release` | tauri-build 生成的 `libresource.a` |
| windres 坏（本机） | `TAURI_SKIP_WINRES=1 cargo build --release` | `build.rs` 自动链 `src-tauri/resource.o` |

`TAURI_SKIP_WINRES` 存在时**自动联动**，不需要再记第二个变量。

**怎么判断本机属于哪种**：直接跑一次不带变量的 `cargo build --release`，
报 `windres: preprocessing failed` 就是坏的那一类。注意这个失败只在
cargo 调 build script 时出现——你手动在 shell 里跑 windres 可能是成功的，
因为 PATH 上下文不一样，别被这个假象带偏。

### 坑 10：`print!` / `println!` 会给导入表塞 7 个 C stdio 符号

`windows_subsystem = "windows"` 的程序本来就没有控制台，往 stdout 写没人看得到；
但 GNU 目标上会把这一组符号拉进导入表：

```
fprintf   fwrite   vfprintf   __iob_func   signal   ...
```

实测 msvcrt 导入从 **36 项涨到 43 项**。`__iob_func` 是老版 MSVCRT 的兼容符号，
在新系统上解析行为不确定，等于给启动期埋雷（排查时极易被误导成"就是这么崩的"）。

**规矩：这个项目里不用 `print!` / `println!` / `eprintln!`。**
日志一律走 `crate::logs::write()` / `glog!` / `gwarn!`，写
`~/.wb-switch/axiao/gateway.log`，界面「运行日志 → 网关服务日志」直接读。

## 一键脚本

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

脚本已处理 PATH、`CARGO_TARGET_DIR`（默认 `D:\axiao-build`）、
自动复制 `WebView2Loader.dll` 和说明文档到 `dist\`。
