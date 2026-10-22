<#
.SYNOPSIS
  Единая точка сборки дистрибутива DocCode: иконки + Windows + Linux.

.DESCRIPTION
  - иконки (PNG/ICO/RGBA)      → packaging/icon/*
  - установщик Windows (NSIS)  → dist/windows/DocCode-<ver>-x64-setup.exe
  - deb и AppImage для Linux   → dist/linux/*
  macOS из-под Windows не собирается (нужен macOS SDK и подпись).

.EXAMPLE
  powershell -File packaging\build-release.ps1
  powershell -File packaging\build-release.ps1 -Target linux
  powershell -File packaging\build-release.ps1 -SkipBuild
#>
[CmdletBinding()]
param(
    [ValidateSet('all', 'windows', 'linux')]
    [string]$Target = 'all',

    # Версия (по умолчанию — из Cargo.toml).
    [string]$Version,

    # Не пересобирать exe перед созданием установщика.
    [switch]$SkipBuild,

    # Подписать exe и установщик сертификатом Code Signing.
    [switch]$Sign,

    # PFX-файл (иначе DOCCODE_PFX_PATH).
    [string]$PfxPath = $env:DOCCODE_PFX_PATH,

    # Отпечаток сертификата в хранилище Windows (иначе DOCCODE_CERT_THUMBPRINT).
    [string]$Thumbprint = $env:DOCCODE_CERT_THUMBPRINT,

    # Искать сертификат в LocalMachine\My вместо CurrentUser\My.
    [switch]$MachineStore
)

$ErrorActionPreference = 'Stop'

$packagingDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $packagingDir

# --- 1. иконки --------------------------------------------------------------
$ico = Join-Path $packagingDir 'icon\doccode.ico'
if (-not (Test-Path $ico)) {
    Write-Host '==> генерация иконок'
    & python (Join-Path $packagingDir 'icon\make_icon.py')
    if ($LASTEXITCODE -ne 0) { throw 'Генерация иконок завершилась ошибкой' }
}

$steps = @()
if ($Target -in @('all', 'windows')) { $steps += 'windows' }
if ($Target -in @('all', 'linux')) { $steps += 'linux' }

# --- 2. платформы -----------------------------------------------------------
foreach ($step in $steps) {
    switch ($step) {
        'windows' {
            Write-Host '==> Windows: установщик'
            $installerArgs = @()
            if ($Version) { $installerArgs += @('-Version', $Version) }
            if ($SkipBuild) { $installerArgs += '-SkipBuild' }
            if ($Sign) {
                $installerArgs += '-Sign'
                if ($PfxPath) { $installerArgs += @('-PfxPath', $PfxPath) }
                if ($Thumbprint) { $installerArgs += @('-Thumbprint', $Thumbprint) }
                if ($MachineStore) { $installerArgs += '-MachineStore' }
            }
            & (Join-Path $packagingDir 'windows\build-installer.ps1') @installerArgs
        }
        'linux' {
            Write-Host '==> Linux: deb + AppImage (Docker)'
            & (Join-Path $packagingDir 'linux\docker-build.ps1')
        }
    }
}

# --- 3. итог ---------------------------------------------------------------
Write-Host ''
Write-Host 'Собранные артефакты:'
Get-ChildItem -Path (Join-Path $root 'dist') -Recurse -File -ErrorAction SilentlyContinue |
    ForEach-Object { '  {0,10:N0}  {1}' -f $_.Length, $_.FullName.Substring($root.Length + 1) }
