# mihomo-cli Windows 一键安装脚本
#
# 用法（PowerShell 5.1+）：
#   irm https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.ps1 | iex
#
# 环境变量：
#   MIHOMO_CLI_VERSION       指定版本（默认 latest，如 v0.2.0）
#   MIHOMO_CLI_INSTALL_DIR   安装目录（默认 $env:LOCALAPPDATA\mihomo-cli\bin）
#   MIHOMO_CLI_NO_PATH       设任意非空值可跳过 PATH 写入（CI 用）
#
# 安全：下载后强制校验发布资产对应的 SHA256SUMS。

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5.1 默认 TLS 1.0/1.1，GitHub API 要求 TLS 1.2+
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = 'lennan747/mihomo-cli'
$Project = 'mihomo-cli'
$Target = 'x86_64-pc-windows-msvc'

function Log($msg) { Write-Host "[$Project] $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "[$Project] $msg" -ForegroundColor Yellow }
function Die($msg) { Write-Host "[$Project] $msg" -ForegroundColor Red; exit 1 }

# --- 1. 架构检查 -------------------------------------------------------------
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64') {
    Die "暂不支持的 Windows 架构: $arch（当前仅提供 x86_64）"
}

# --- 2. 解析版本 -------------------------------------------------------------
$Version = $env:MIHOMO_CLI_VERSION
if (-not $Version) {
    Log '查询最新版本 ...'
    $Version = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name
    if (-not $Version) { Die '无法获取最新版本号' }
}
Log "版本: $Version"

# --- 3. 下载并校验 -----------------------------------------------------------
$Asset = "$Project-$Target.zip"
$Base = "https://github.com/$Repo/releases/download/$Version"
$Tmp = Join-Path $env:TEMP ("mihomo-cli-install-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Tmp | Out-Null

try {
    $zip = Join-Path $Tmp $Asset
    Log "下载 $Asset ..."
    Invoke-WebRequest -Uri "$Base/$Asset" -OutFile $zip

    $sums = Join-Path $Tmp 'SHA256SUMS'
    Invoke-WebRequest -Uri "$Base/SHA256SUMS" -OutFile $sums

    Log '下载并校验 SHA256SUMS ...'
    $line = Get-Content $sums | Where-Object { $_ -match '^\s*([0-9a-fA-F]{64})\s+\*?' + [regex]::Escape($Asset) + '\s*$' } | Select-Object -First 1
    if (-not $line) { Die "SHA256SUMS 中未找到 $Asset 的校验值" }
    $expected = ($line -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { Die 'SHA256 校验失败，已中止安装（请勿绕过校验）' }
    Log '校验通过'

    # --- 4. 解压并安装 -------------------------------------------------------
    $InstallDir = if ($env:MIHOMO_CLI_INSTALL_DIR) { $env:MIHOMO_CLI_INSTALL_DIR }
                  else { Join-Path $env:LOCALAPPDATA "$Project\bin" }
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Expand-Archive -Path $zip -DestinationPath $Tmp -Force
    # 兼容 zip 内为扁平或带路径两种布局
    $exe = Get-ChildItem -Path $Tmp -Recurse -Filter "$Project.exe" | Select-Object -First 1
    if (-not $exe) { Die "压缩包内未找到 $Project.exe" }
    Copy-Item -Path $exe.FullName -Destination $InstallDir -Force

    # --- 5. 写入用户 PATH（新终端生效） --------------------------------------
    if (-not $env:MIHOMO_CLI_NO_PATH) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $userPath) { $userPath = '' }
        if (($userPath -split ';') -notcontains $InstallDir) {
            $newPath = if ($userPath.TrimEnd(';')) { $userPath.TrimEnd(';') + ';' + $InstallDir } else { $InstallDir }
            [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Log "已将 $InstallDir 加入用户 PATH（新开终端生效）"
        }
    }

    Log "已安装: $InstallDir\$Project.exe ($Version)"
    & (Join-Path $InstallDir "$Project.exe") --version
}
finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
