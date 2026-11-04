# 一条命令出全部交付物。
#
# 为什么要有这个：以前编译、复制 dist、打源码包、跑 preflight 是四步手工操作，
# 漏一步就会出问题 —— 实际发生过两次：
#   1) dist 里的 exe 停在旧版本，用户下到的包缺新功能
#   2) 源码包从另一个旧副本打，跟 exe 不是同一份代码
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File scripts/release.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -SkipBuild

param(
    [switch]$SkipBuild,
    [switch]$SkipSource
)

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

# 让 python 的 UTF-8 输出在控制台里别变成乱码（5.1 默认按 ANSI 解析）
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}
$OutputEncoding = [System.Text.Encoding]::UTF8

$env:CARGO_TARGET_DIR = "D:\axiao-build"
$env:TAURI_SKIP_WINRES = "1"
# 本机 windres 在 cargo 里跑不起来，必须用它自带的 msys2 工具链
$env:PATH = "D:\axiao-build\msys2\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin;$env:PATH"

$exe = "D:\axiao-build\release\axiao-gateway.exe"

if (-not $SkipBuild) {
    Write-Host "`n=== 1/4 编译 ===" -ForegroundColor Cyan
    Push-Location src-tauri
    cargo build --release
    Pop-Location
}

if (-not (Test-Path $exe)) { throw "找不到 $exe" }

Write-Host "`n=== 2/4 组装 dist ===" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path "dist" | Out-Null
Copy-Item $exe "dist\axiao-gateway.exe" -Force
Copy-Item "config.example.json" "dist\config.example.json" -Force
# 这两份是给用户看的说明，源在 docs/ 下。以前 build.ps1 引用的源文件不存在，
# 导致每次构建都静默跳过，dist 里的说明永远是旧的。
if (Test-Path "docs\USAGE.txt") { Copy-Item "docs\USAGE.txt" "dist\使用说明.txt" -Force }
if (Test-Path "docs\GETTING-STARTED.md") { Copy-Item "docs\GETTING-STARTED.md" "dist\新手教程.md" -Force }

$dll = Get-ChildItem -Recurse -Filter "WebView2Loader.dll" -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -notmatch '\\dist\\' } | Select-Object -First 1
if ($dll) { Copy-Item $dll.FullName "dist\WebView2Loader.dll" -Force }

Get-ChildItem "dist" | Format-Table Name, Length -AutoSize

if (-not $SkipSource) {
    Write-Host "`n=== 3/5 打成品包（给用户用的）===" -ForegroundColor Cyan
    # 这一步以前没有，导致桌面上只有源码包 —— 用户看到 137 个源码文件
    # 根本不知道该点哪个。成品包才是发给普通人的东西。
    python scripts/make-dist-zip.py

    Write-Host "`n=== 4/5 打源码包（给开发的）===" -ForegroundColor Cyan
    python scripts/make-source-zip.py
}

Write-Host "`n=== 5/5 打包前置检查 ===" -ForegroundColor Cyan
$check = & powershell -ExecutionPolicy Bypass -File "scripts/preflight.ps1" 2>&1
$check | Select-Object -Last 6
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n!!! 检查未通过，不要发布 !!!" -ForegroundColor Red
    exit 1
}

Write-Host "`n全部完成。产物：" -ForegroundColor Green
Write-Host "  dist\axiao-gateway.exe        （发布包）"
Write-Host "  桌面\阿晓网关助手-v*-source.zip（源码包）"
