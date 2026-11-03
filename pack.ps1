<#
.SYNOPSIS
    iClick-gpui 一键打包：改版本号 → release 编译 → 拷进 dist → 打 zip。

.DESCRIPTION
    版本号只有 Cargo.toml 里一个地方，程序标题、设置页、更新比对全都读它，
    所以这里改完就等于全项目都改了。编译失败会自动把 Cargo.toml 还原，
    不会留下一个改了一半的版本号。

.EXAMPLE
    .\pack.ps1 1.5.0
        版本改成 1.5.0，编译，产出 dist\iClick-gpui.exe 和 dist\iClick-gpui_v1.5.0.zip

.EXAMPLE
    .\pack.ps1 1.5.0 -NoZip
        只要 exe，不打压缩包

.EXAMPLE
    .\pack.ps1 1.5.0 -DryRun
        只检查版本号能不能改，不编译（改了会立刻还原）

.EXAMPLE
    .\pack.ps1 -Open
        版本号不动，用当前版本重新打包，打完打开 dist 目录
#>
[CmdletBinding()]
param(
    # 目标版本号，写 1.5.0 或 v1.5.0 都行；不写就用 Cargo.toml 里现有的
    [Parameter(Position = 0)]
    [string]$Version,

    # 不生成 zip
    [switch]$NoZip,

    # 只验证版本号改写，不编译
    [switch]$DryRun,

    # 打包完成后打开 dist 目录
    [switch]$Open
)

$ErrorActionPreference = 'Stop'

# ---------- 基础路径 ----------
$root = $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($root)) { $root = (Get-Location).Path }
$cargoToml = Join-Path $root 'Cargo.toml'
$distDir = Join-Path $root 'dist'
$targetExe = Join-Path $root 'target\release\iClick-gpui.exe'
$distExe = Join-Path $distDir 'iClick-gpui.exe'

# Cargo.toml 里有中文注释，必须按 UTF-8 无 BOM 读写，否则会写坏
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Write-Step([string]$text) {
    Write-Host ''
    Write-Host ">>> $text" -ForegroundColor Cyan
}

function Write-Ok([string]$text) {
    Write-Host "    $text" -ForegroundColor Green
}

function Write-Warn2([string]$text) {
    Write-Host "    $text" -ForegroundColor Yellow
}

# ---------- 版本号读写（只动 [package] 段里的 version） ----------
function Get-PackageVersion([string]$text) {
    $inPackage = $false
    foreach ($line in ($text -split "`r?`n")) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[(.+)\]$') {
            $inPackage = ($matches[1].Trim() -eq 'package')
            continue
        }
        if ($inPackage -and $trimmed -match '^version\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
    }
    return $null
}

function Set-PackageVersion([string]$text, [string]$newVersion) {
    # 第 1 组要把 [package] 到 version 键之间的所有内容原样带上，
    # 否则替换会把 [package]、name 这些行一起吃掉（只保留到 version 前缀）。
    # 依赖段里的 version 不在这个范围里，不会被碰。
    $pattern = '(?ms)^(\[package\][\s\S]*?^\s*version\s*=\s*")[^"]*(")'
    if ($text -notmatch $pattern) {
        throw "Cargo.toml 里找不到 [package] 段的 version 字段"
    }
    $evaluator = { param($m) $m.Groups[1].Value + $newVersion + $m.Groups[2].Value }
    return [regex]::Replace($text, $pattern, $evaluator)
}

# ---------- 开始 ----------
if (-not (Test-Path $cargoToml)) {
    Write-Host "找不到 Cargo.toml：$cargoToml" -ForegroundColor Red
    Write-Host "把 pack.ps1 放在项目根目录再运行。" -ForegroundColor Red
    exit 1
}

$originalText = [System.IO.File]::ReadAllText($cargoToml, $utf8NoBom)
$currentVersion = Get-PackageVersion $originalText
if (-not $currentVersion) {
    Write-Host "Cargo.toml 里读不到版本号，先检查文件格式。" -ForegroundColor Red
    exit 1
}

# 没给版本号就用现有的（等于「用当前版本重新打包」）
if ([string]::IsNullOrWhiteSpace($Version)) {
    $targetVersion = $currentVersion
    Write-Warn2 "没指定版本号，用 Cargo.toml 里的 v$currentVersion 重新打包"
} else {
    $targetVersion = $Version.Trim().TrimStart('v', 'V')
    if ($targetVersion -notmatch '^\d+\.\d+\.\d+$') {
        Write-Host "版本号格式不对：「$Version」" -ForegroundColor Red
        Write-Host "要写成 1.5.0 这种三段式（可以带 v 前缀，比如 v1.5.0）。" -ForegroundColor Red
        exit 1
    }
}

Write-Host ''
Write-Host "iClick-gpui 打包"
Write-Host "    项目目录：$root"
Write-Host "    当前版本：v$currentVersion"
Write-Host "    目标版本：v$targetVersion"

# ---------- 改版本号 ----------
$changed = $false
if ($targetVersion -ne $currentVersion) {
    Write-Step "改版本号 v$currentVersion -> v$targetVersion"
    $newText = Set-PackageVersion $originalText $targetVersion
    [System.IO.File]::WriteAllText($cargoToml, $newText, $utf8NoBom)
    $changed = $true
    Write-Ok "Cargo.toml 已更新"
} else {
    Write-Ok "版本号没变，跳过改写"
}

if ($DryRun) {
    if ($changed) {
        [System.IO.File]::WriteAllText($cargoToml, $originalText, $utf8NoBom)
        Write-Warn2 "DryRun：已把 Cargo.toml 还原成 v$currentVersion"
    }
    Write-Host ''
    Write-Host "DryRun 通过，没有编译。" -ForegroundColor Green
    exit 0
}

# ---------- 编译 ----------
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$buildFailed = $false
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "找不到 cargo，先把 Rust 工具链装好或检查 PATH"
    }

    Write-Step "编译 release 版本（opt-level=z + fat LTO，慢慢等）"
    Push-Location $root
    try {
        # cargo 的进度是写到 stderr 的；PS 5.1 在 Stop 策略下会把原生程序的
        # stderr 当成 ErrorRecord 直接抛异常，所以这一段临时放宽成 Continue，
        # 成败一律看退出码。
        $oldPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            & cargo build --release
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldPreference
        }
        if ($exitCode -ne 0) {
            throw "cargo build 失败，退出码 $exitCode"
        }
    } finally {
        Pop-Location
    }
    Write-Ok ("编译完成，用时 {0:N1} 秒" -f $sw.Elapsed.TotalSeconds)

    if (-not (Test-Path $targetExe)) {
        throw "编译说成功了，但没找到 $targetExe"
    }

    # ---------- 拷进 dist ----------
    Write-Step "拷到 dist"
    if (-not (Test-Path $distDir)) {
        New-Item -ItemType Directory -Path $distDir | Out-Null
    }
    Copy-Item -Path $targetExe -Destination $distExe -Force
    $exeSize = [math]::Round((Get-Item $distExe).Length / 1MB, 2)
    Write-Ok "dist\iClick-gpui.exe（$exeSize MB）"

    # ---------- 打 zip ----------
    $zipPath = $null
    if ($NoZip) {
        Write-Warn2 "按 -NoZip 跳过压缩包"
    } else {
        Write-Step "打压缩包"
        $zipPath = Join-Path $distDir ("iClick-gpui_v{0}.zip" -f $targetVersion)
        Compress-Archive -Path $distExe -DestinationPath $zipPath -CompressionLevel Optimal -Force
        $zipSize = [math]::Round((Get-Item $zipPath).Length / 1MB, 2)
        Write-Ok ("dist\iClick-gpui_v{0}.zip（{1} MB）" -f $targetVersion, $zipSize)
    }
} catch {
    $buildFailed = $true
    Write-Host ''
    Write-Host "打包失败：$($_.Exception.Message)" -ForegroundColor Red
} finally {
    $sw.Stop()
    # 失败就把版本号还原，别留个改了一半的 Cargo.toml
    if ($buildFailed -and $changed) {
        [System.IO.File]::WriteAllText($cargoToml, $originalText, $utf8NoBom)
        Write-Warn2 "已把 Cargo.toml 还原成 v$currentVersion"
    }
}

if ($buildFailed) { exit 1 }

# ---------- 收尾提示 ----------
Write-Host ''
Write-Host "打包完成：v$targetVersion" -ForegroundColor Green
Write-Host ("    用时：{0:N1} 秒" -f $sw.Elapsed.TotalSeconds)
Write-Host ''
Write-Host "发版还差两步（脚本不替你做，避免误提交）：" -ForegroundColor Yellow
Write-Host "    1. 把 dist\iClick-gpui_v$targetVersion.zip 传到 Release"
Write-Host "    2. 仓库里的 version.json 改成 {`"version`":`"v$targetVersion`"}，否则老版本检测不到更新"
Write-Host "       当前已检测到改动：Cargo.toml$(if ($changed) { ' + Cargo.lock' } else { '（版本号未变）' })"

if ($Open) {
    Start-Process explorer.exe $distDir
}
