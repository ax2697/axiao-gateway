# 拉取参考实现（不提交到仓库）
$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot "..\vendor"
New-Item -ItemType Directory -Force -Path $root | Out-Null

$repos = @(
  "ardeyouxipianyi/workbuddy2api-hub",
  "HanawaBanana/workbuddy2api",
  "jjjane233/workbuddy-3in1",
  "icebears111/workbuddy2api",
  "wangct233-source/wb2go",
  "zouzhenglu/modelwharf",
  "yanghuan/wild-work",
  "practice019/multi2api",
  "aimod-cc/Agent2API",
  "CangShui/workbuddy-gateway",
  "ikunops/free2api",
  "yetone/magpie",
  "HitMargin/ai-proxy",
  "xu20060420fan-cell/workbuddy2api-trae-qoder-minimax-panel",
  "L0NE-6/WorkBuddy-Daily"
)

foreach ($r in $repos) {
  $name = $r -replace "/", "_"
  $dest = Join-Path $root $name
  if (Test-Path $dest) {
    Write-Host "skip $name"
    continue
  }
  Write-Host "clone $r"
  git clone --depth 1 --single-branch "https://github.com/$r.git" $dest
}

Write-Host "done: $root"
