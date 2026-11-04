# 一键构建（Windows）
# 用法：
#   powershell -ExecutionPolicy Bypass -File scripts/build.ps1
#
# 可选环境变量：
#   MINGW_BIN              MinGW 的 bin 目录（GNU 工具链必需，例如 D:\axiao-build\winlibs\mingw64\bin）
#   TARGET_DIR             构建产物目录（默认 D:\axiao-build，GNU 工具链必须是纯 ASCII 路径）
#   TAURI_SKIP_WINRES=1    跳过 tauri-build 的 windres（本机 windres 坏掉时的兜底）。
#                          设了它 build.rs 会自动链 src-tauri/resource.o，不用再管别的
#   RESOURCE_OBJ           预编译资源对象，默认 D:\axiao-build\resource.o
#
# 正常情况（MSVC 或 windres 可用）：直接跑本脚本即可，manifest 由 tauri-build 自动嵌入。
# 本机 windres 不可用时：见 docs/BUILD-NOTES.md「坑 5」。

$ErrorActionPreference = "Stop"
$tc = Join-Path $env:USERPROFILE ".rustup\toolchains\stable-x86_64-pc-windows-gnu"

$paths = @()
if ($env:MINGW_BIN) { $paths += $env:MINGW_BIN }
$paths += (Join-Path $tc "lib\rustlib\x86_64-pc-windows-gnu\bin")
$paths += (Join-Path $tc "lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained")
$paths += (Join-Path $env:USERPROFILE ".cargo\bin")
$paths += $env:PATH
$env:PATH = $paths -join ";"

if (-not $env:TARGET_DIR) { $env:TARGET_DIR = "D:\axiao-build" }
$env:CARGO_TARGET_DIR = $env:TARGET_DIR

Set-Location (Join-Path $PSScriptRoot "..")

if (-not (Test-Path "node_modules")) {
  npm install
}

# 资源对象：GNU 兜底路线必需（exe 的 .rsrc 段，含 Common Controls v6 manifest + 图标 + 版本信息）。
#
# 为什么现场生成而不是入库：
#   resource.o 是**二进制**，入库会带来两个问题 ——
#   ① 换机器 / 换版本号时它不会跟着变（曾出现过 exe 显示 0.9.3 而源码是 0.9.5 的漂移）；
#   ② 别人 clone 拿到的是别人机器编译的产物，图标/manifest 可能对不上。
#   真正的**事实源**是同目录的 resource.rc（文本），它才该入库。
#   所以这里每次构建都从 rc 重新编一次，版本号永远跟着 rc 走。
#
# 没它 exe 会缺 manifest，双击以 0xC0000139 秒退（见 docs/BUILD-NOTES.md 坑 9）。
$rcPath = "src-tauri\resource.rc"
$resObj = "src-tauri\resource.o"
if ($env:TAURI_SKIP_WINRES -eq "1") {
  if (-not (Test-Path $rcPath)) { throw "缺少 $rcPath（资源事实源）" }
  $windres = Get-Command "windres.exe" -ErrorAction SilentlyContinue
  if (-not $windres) {
    throw "找不到 windres.exe —— 设 MINGW_BIN 指向 MinGW 的 mingw64\bin（见 docs/BUILD-NOTES.md 坑 5）"
  }
  Write-Host "生成资源对象：$($windres.Source) -i $rcPath -o $resObj"
  & $windres.Source --input $rcPath --output $resObj --include-dir "src-tauri" --output-format=coff
  if ($LASTEXITCODE -ne 0) { throw "windres 失败（exit $LASTEXITCODE）" }
  Write-Host "OK -> $resObj"
} elseif (-not (Test-Path $resObj)) {
  Write-Warning "缺少 $resObj（正常路线由 tauri-build 自己编，不需要它）"
}

npx tauri build --no-bundle
if ($LASTEXITCODE -ne 0) { throw "tauri build 失败（exit $LASTEXITCODE）" }

$exe = Join-Path $env:TARGET_DIR "release\axiao-gateway.exe"
if (-not (Test-Path $exe)) { throw "构建失败：未找到 $exe" }

New-Item -ItemType Directory -Force -Path "dist" | Out-Null
Copy-Item $exe "dist\axiao-gateway.exe" -Force

# WebView2Loader.dll：GNU 构建需要，MSVC 不需要。从 webview2-com-sys 的输出目录里捞。
$dll = $null
foreach ($root in @($env:TARGET_DIR, "target")) {
  if ($dll) { break }
  if (-not (Test-Path $root)) { continue }
  $dll = Get-ChildItem -Path $root -Recurse -Filter "WebView2Loader.dll" -ErrorAction SilentlyContinue |
         Select-Object -First 1
}
if ($dll) {
  Copy-Item $dll.FullName "dist\WebView2Loader.dll" -Force
  Write-Host "OK -> dist\WebView2Loader.dll"
} else {
  Write-Host "提示：未找到 WebView2Loader.dll（MSVC 构建不需要，GNU 构建必须手动放一份）"
}

Copy-Item "config.example.json" "dist\config.example.json" -Force
if (Test-Path "docs\USAGE.txt")        { Copy-Item "docs\USAGE.txt" "dist\使用说明.txt" -Force }
if (Test-Path "docs\GETTING-STARTED.md") { Copy-Item "docs\GETTING-STARTED.md" "dist\新手教程.md" -Force }

Write-Host ""
Write-Host "构建完成，产物在 dist\"
Get-ChildItem "dist" | Format-Table Name, Length -AutoSize
