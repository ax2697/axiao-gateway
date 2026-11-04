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

# 资源对象必须存在：没有它 exe 就没有 .rsrc 段，等于没嵌 manifest，
# 双击会以 0xC0000139 秒退（见 docs/BUILD-NOTES.md 坑 9）。
# build.rs 会自动把这个 .o 链进 exe，这里只负责在缺失时喊一声。
if (-not (Test-Path "src-tauri\resource.o")) {
  Write-Warning "缺少 src-tauri/resource.o —— 产出的 exe 会缺 manifest，双击可能闪退（坑 9）"
  Write-Warning "生成：windres --input src-tauri/resource.rc --output src-tauri/resource.o --include-dir src-tauri --output-format=coff"
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
