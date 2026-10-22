<#
.SYNOPSIS
  Собирает установщик DocCode для Windows (NSIS) и кладёт его в dist/windows.

.DESCRIPTION
  1. Генерирует иконки, если их ещё нет (packaging/icon/make_icon.py).
  2. Собирает release-бинарь (cargo build --release), если не просили пропустить.
  3. Вызывает makensis с версией из Cargo.toml и размером файла.
  4. Переносит готовый setup.exe в dist/windows.

.EXAMPLE
  powershell -File packaging\windows\build-installer.ps1
  powershell -File packaging\windows\build-installer.ps1 -SkipBuild
#>
[CmdletBinding()]
param(
    # Версия установщика. По умолчанию читается из Cargo.toml.
    [string]$Version,
    # Не пересобирать target/release/doccode.exe.
    [switch]$SkipBuild,
    # Подписать exe и установщик сертификатом Code Signing.
    [switch]$Sign,
    # PFX-файл (иначе DOCCODE_PFX_PATH).
    [string]$PfxPath = $env:DOCCODE_PFX_PATH,
    # Отпечаток сертификата в хранилище Windows (иначе DOCCODE_CERT_THUMBPRINT).
    [string]$Thumbprint = $env:DOCCODE_CERT_THUMBPRINT,
    # Искать сертификат в LocalMachine\My вместо CurrentUser\My.
    [switch]$MachineStore,
    # RFC 3161 timestamp-сервер.
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'

$windowsDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$packagingDir = Split-Path -Parent $windowsDir
$root = Split-Path -Parent $packagingDir

# --- версия из Cargo.toml --------------------------------------------------
if (-not $Version) {
    $line = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^\s*version\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if (-not $line) { throw 'Не удалось прочитать version из Cargo.toml' }
    $Version = $line.Matches[0].Groups[1].Value
}
Write-Host "DocCode $Version — сборка установщика Windows"

# NSIS VIProductVersion требует четыре числа, Cargo.toml даёт три.
# Prerelease-суффикс (1.2.3-rc.1) допустим в имени файла, но не в VIProductVersion.
$NumericVersion = $Version -replace '[-+].*$', ''
if ($NumericVersion -match '^\d+\.\d+\.\d+$') {
    $Version4 = "$NumericVersion.0"
} elseif ($NumericVersion -match '^\d+\.\d+\.\d+\.\d+$') {
    $Version4 = $NumericVersion
} else {
    throw "Версия должна начинаться с x.y.z или x.y.z.w: $Version"
}

if ($Sign -and -not $PfxPath -and -not $Thumbprint) {
    throw 'Для -Sign нужен PFX (-PfxPath / DOCCODE_PFX_PATH) или отпечаток сертификата (-Thumbprint / DOCCODE_CERT_THUMBPRINT).'
}
if ($PfxPath -and $Thumbprint) {
    throw 'Укажите только один источник ключа: -PfxPath или -Thumbprint.'
}

# --- иконки ----------------------------------------------------------------
$ico = Join-Path $packagingDir 'icon\doccode.ico'
if (-not (Test-Path $ico)) {
    Write-Host 'Иконки нет — генерируем (packaging/icon/make_icon.py)'
    & python (Join-Path $packagingDir 'icon\make_icon.py')
    if ($LASTEXITCODE -ne 0) { throw 'Генерация иконок завершилась ошибкой' }
}

# --- бинарь ----------------------------------------------------------------
$exe = Join-Path $root 'target\release\doccode.exe'
if (-not $SkipBuild -or -not (Test-Path $exe)) {
    Push-Location $root
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'cargo build --release завершился ошибкой' }
    } finally { Pop-Location }
}
if (-not (Test-Path $exe)) { throw "Бинарь не найден: $exe" }

# --- подпись exe до упаковки в установщик ----------------------------------
if ($Sign) {
    $signArgs = @{
        Path         = @($exe)
        TimestampUrl = $TimestampUrl
    }
    if ($PfxPath) {
        $signArgs.PfxPath = $PfxPath
    } else {
        $signArgs.Thumbprint = $Thumbprint
        if ($MachineStore) { $signArgs.MachineStore = $true }
    }
    & (Join-Path $windowsDir 'sign.ps1') @signArgs
}

# --- makensis --------------------------------------------------------------
$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if (-not $makensis) {
    throw 'makensis не найден. Установите: scoop install nsis'
}

$sizeKB = [int][math]::Ceiling((Get-Item $exe).Length / 1KB)
Write-Host "Размер doccode.exe: $sizeKB KB"

Push-Location $windowsDir
try {
    & $makensis.Source "/DPRODUCT_VERSION=$Version" "/DPRODUCT_VERSION4=$Version4" "/DESTIMATED_SIZE=$sizeKB" 'installer.nsi'
    if ($LASTEXITCODE -ne 0) { throw "makensis завершился с кодом $LASTEXITCODE" }
} finally { Pop-Location }

# --- в dist ---------------------------------------------------------------
$built = Join-Path $windowsDir "DocCode-$Version-x64-setup.exe"
if (-not (Test-Path $built)) { throw "makensis не создал $built" }

$outDir = Join-Path $root 'dist\windows'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$dest = Join-Path $outDir (Split-Path -Leaf $built)
Move-Item -Force $built $dest

# --- подпись готового установщика ------------------------------------------
if ($Sign) {
    $signArgs = @{
        Path         = @($dest)
        TimestampUrl = $TimestampUrl
    }
    if ($PfxPath) {
        $signArgs.PfxPath = $PfxPath
    } else {
        $signArgs.Thumbprint = $Thumbprint
        if ($MachineStore) { $signArgs.MachineStore = $true }
    }
    & (Join-Path $windowsDir 'sign.ps1') @signArgs
}

Write-Host "Готово: $dest"
