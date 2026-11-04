# 打包前置检查：防止把个人数据 / 密钥 / 账号打进发布包
# 用法：
#   powershell -ExecutionPolicy Bypass -File scripts/preflight.ps1
# 退出码：0 = 干净；1 = 发现问题（此时禁止打包）

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$problems = @()

# 不扫描的目录：构建产物、依赖、以及 AI 工作区（.axiao / AI工作区 都是过程产物，
# 已在 .gitignore 排除，不随仓库发布）
$SKIP = '\\(node_modules|target|release|\.git|AI工作区|\.axiao)\\'

# 内部协作文档：里面的绝对路径是"这台机器上的约定"（基准目录、素材位置），
# 对拿到源码包的人没有意义。不当作泄露处理，但下面会提示交付前清掉。
$INTERNAL_DOCS = 'AGENTS\.md$'

# 便携版出厂预置 key：这是功能的一部分（面板/接口登录用），不算泄露，白名单放行。
# 注意：它是所有便携包共用的默认值，开公网前必须改掉，这一点由文档提醒。
$KNOWN_DEFAULTS = @("sk-QxgVEFVZM6Pg8Z6MfeiCA6TS", "sk-REPLACE_ME")

# 1) 明文密钥：sk- 加 20 位以上
#    负向回顾 (?<![A-Za-z0-9]) 是为了避开压缩后的 CSS-in-JS 里
#    形如 "mask-linear-from-pos" 的假阳性（里面有 "sk-linear-from-pos"）。
$keyHits = Get-ChildItem -Recurse -File -Include *.md,*.txt,*.json,*.js,*.mjs,*.rs,*.ps1,*.yml |
  Where-Object { $_.FullName -notmatch $SKIP } |
  Select-String -Pattern '(?<![A-Za-z0-9])sk-[A-Za-z0-9_-]{20,}' -ErrorAction SilentlyContinue |
  Where-Object {
    $line = $_.Line
    -not ($KNOWN_DEFAULTS | Where-Object { $line -like "*$_*" })
  }
if ($keyHits) {
  $problems += "发现疑似明文 API Key（非出厂预置值）："
  $problems += ($keyHits | ForEach-Object { "  $($_.Path):$($_.LineNumber)" })
}

# 2) 真实 token 字段被写进示例/文档
$tokHits = Get-ChildItem -Recurse -File -Include *.md,*.txt,*.json |
  Where-Object { $_.FullName -notmatch $SKIP } |
  Select-String -Pattern '"(accessToken|refreshToken)"\s*:\s*"[^"]{20,}"' -ErrorAction SilentlyContinue
if ($tokHits) {
  $problems += "发现疑似真实 token："
  $problems += ($tokHits | ForEach-Object { "  $($_.Path):$($_.LineNumber)" })
}

# 3) 个人绝对路径（含用户名 / 盘符家目录）
$pathHits = @(Get-ChildItem -Recurse -File -Include *.md,*.txt,*.json,*.rs,*.js,*.ps1,*.yml |
  Where-Object { $_.FullName -notmatch $SKIP } |
  Select-String -Pattern 'C:\\Users\\[A-Za-z0-9_]+|D:\\360MoveData\\Users' -ErrorAction SilentlyContinue |
  Where-Object { $_.Path -notmatch $INTERNAL_DOCS })
if ($pathHits.Count -gt 0) {
  $problems += "发现个人绝对路径（建议改为相对路径或占位符）："
  $problems += ($pathHits | ForEach-Object { "  $($_.Path):$($_.LineNumber)" })
}
if (Test-Path "AGENTS.md") {
  Write-Host "  提示：AGENTS.md 含本机路径（内部协作约定）。要把源码包发给外部，先删掉它和 .axiao/。" -ForegroundColor DarkGray
}

# 4) 账号数据文件绝不能被跟踪或打包
foreach ($p in @("auths", "data/state.json", "config.json")) {
  if (Test-Path $p) { $problems += "工作区存在敏感文件：$p（打包前请确认不进包）" }
}
$acc = Get-ChildItem -Recurse -File -Filter "accounts.json" -ErrorAction SilentlyContinue |
  Where-Object { $_.FullName -notmatch $SKIP }
if ($acc) {
  $problems += "发现 accounts.json（可能含账号）："
  $problems += ($acc | ForEach-Object { "  $($_.FullName)" })
}

# 5) 若已建 git 仓：核「被跟踪的文件」里没有敏感物与产物
if (Test-Path ".git") {
  $tracked = git ls-files 2>$null
  if ($tracked) {
    $bad = $tracked | Where-Object {
      $_ -match '^(auths|data)/|config\.json$|accounts\.json$|\.axiao-bak$|node_modules/|^src-tauri/target/|^release/|^AI工作区/' -or $_ -match '\.exe$'
    }
    if ($bad) {
      $problems += "以下文件已被 git 跟踪，禁止入库："
      $problems += ($bad | ForEach-Object { "  $_" })
    }
  }
}

# ---------- 7) 安全边界（攻击面）----------
# 这一组来自「24-安全审计技能」里 Desktop/Local-IPC、Protocols、Resource-Exhaustion
# 三篇中可机械化的部分。跟前面几项的区别：前面查"有没有把密钥漏出去"，
# 这里查"别人能不能借这个程序干别的、或者一条请求把它打崩"。
Write-Host "`n=== 安全边界 ===" -ForegroundColor Cyan

$rsFiles = @(Get-ChildItem -Recurse -File -Include *.rs | Where-Object { $_.FullName -notmatch $SKIP })

# 7.1 本地监听地址只允许 loopback（写成 0.0.0.0 就是把服务开到局域网）
$binds = @($rsFiles | Select-String -Pattern 'TcpListener::bind' -ErrorAction SilentlyContinue)
foreach ($b in $binds) {
  if ($b.Line -notmatch '127\.0\.0\.1|localhost|\[::1\]') {
    $problems += "监听地址不是 loopback（会暴露给局域网）：$($b.Path):$($b.LineNumber)"
  }
}

# 7.2 按客户端声明的 Content-Length 分配缓冲区，必须有硬上限
$rawAlloc = @($rsFiles | Select-String -Pattern 'vec!\[0u8;\s*length\]' -ErrorAction SilentlyContinue)
if ($rawAlloc.Count -gt 0) {
  $hasCap = @($rsFiles | Select-String -Pattern 'MAX_BODY_BYTES' -ErrorAction SilentlyContinue)
  if ($hasCap.Count -eq 0) {
    $problems += "按 Content-Length 直接分配缓冲区但没有 MAX_BODY_BYTES 上限（一个请求就能吃满内存）"
  }
}

# 7.3 CSP 不能为空
$confPath = "src-tauri/tauri.conf.json"
if (Test-Path $confPath) {
  $conf = Get-Content $confPath -Raw | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($conf.app.security.csp)) {
    $problems += "tauri.conf.json 的 CSP 为空：任何注入的脚本都能调用全部 Tauri 命令"
  }
}

# 7.4 前端不许 eval / new Function
$ev = @(Get-ChildItem "src" -Recurse -File -Include *.js -ErrorAction SilentlyContinue |
  Select-String -Pattern '\beval\s*\(|new\s+Function\s*\(' -ErrorAction SilentlyContinue)
if ($ev.Count -gt 0) {
  $problems += "前端出现 eval / new Function："
  $problems += ($ev | ForEach-Object { "  $($_.Path):$($_.LineNumber)" })
}

# 7.5 Rust 不许直接拼 shell（参数注入的老窝）
$shell = @($rsFiles | Select-String -Pattern 'Command::new\(\s*"(cmd|cmd\.exe|powershell|pwsh|sh|bash)"' -ErrorAction SilentlyContinue)
if ($shell.Count -gt 0) {
  $problems += "发现直接调 shell 的代码（容易被参数注入）："
  $problems += ($shell | ForEach-Object { "  $($_.Path):$($_.LineNumber)  $($_.Line.Trim())" })
}

# 7.6 发布产物不许带 sourcemap
if (Test-Path "dist") {
  $maps = @(Get-ChildItem "dist" -Recurse -File -Include *.map -ErrorAction SilentlyContinue)
  if ($maps.Count -gt 0) {
    $problems += "dist 里有 sourcemap（源码结构会一起发出去）："
    $problems += ($maps | ForEach-Object { "  $($_.FullName)" })
  }
}

# 7.7 release 必须 strip，别把符号表打进 exe
$cargoToml = "src-tauri/Cargo.toml"
if ((Test-Path $cargoToml) -and ((Get-Content $cargoToml -Raw) -notmatch 'strip\s*=\s*true')) {
  $problems += "Cargo.toml 的 [profile.release] 没有 strip = true（符号表会带进 exe）"
}

# 7.8 页面完整性：视图 / 导航 / PAGE_META / 教程 / id 引用是否对得上
#     「点进去白屏」几乎都是这几类问题，静态就能查，不用跑浏览器。
$pyCmd = Get-Command python -ErrorAction SilentlyContinue
if ($pyCmd) {
  $viewOut = & python "scripts/check-views.py" 2>&1
  if ($LASTEXITCODE -ne 0) {
    $problems += "页面完整性检查未通过："
    $problems += ($viewOut | Select-Object -Last 8 | ForEach-Object { "  $_" })
  } else {
    Write-Host "  页面完整性：视图 / 导航 / 教程 / id 引用 全部对得上" -ForegroundColor Gray
  }
} else {
  Write-Host "  跳过页面完整性检查（没找到 python）" -ForegroundColor DarkGray
}

Write-Host "  查过：监听地址 / 请求体上限 / CSP / eval / shell 拼接 / sourcemap / strip" -ForegroundColor Gray

# 6) 发布包里到底有什么（人工再确认一遍）
Write-Host "`n=== dist/ 内容 ===" -ForegroundColor Cyan
if (Test-Path "dist") { Get-ChildItem "dist" | Format-Table Name, Length -AutoSize }

if ($problems.Count -gt 0) {
  Write-Host "`n!!! 检查未通过 !!!" -ForegroundColor Red
  $problems | ForEach-Object { Write-Host $_ -ForegroundColor Yellow }
  exit 1
}

Write-Host "`n检查通过：没发现明文密钥 / token / 个人账号数据" -ForegroundColor Green
exit 0
