<#
.SYNOPSIS
  Собирает DocCode под Linux (deb + AppImage) внутри Docker-контейнера.

.DESCRIPTION
  Образ — packaging/linux/Dockerfile (Ubuntu 22.04 + Rust + зависимости winit/wgpu).
  Кеш cargo живёт в именованном томе, поэтому повторные сборки инкрементальные.

.EXAMPLE
  powershell -File packaging\linux\docker-build.ps1
  powershell -File packaging\linux\docker-build.ps1 -RebuildImage
#>
[CmdletBinding()]
param(
    # Пересобрать образ с нуля (apt/rustup не будут пропущены кешем).
    [switch]$RebuildImage,
    # Имя образа и тома с кешем cargo.
    [string]$ImageName = 'doccode-linux-builder',
    [string]$CacheVolume = 'doccode-build-cache',

    # После сборки прогнать verify-linux.sh (состав deb, ldd, .desktop).
    [switch]$Verify
)

$ErrorActionPreference = 'Stop'

$linuxDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$packagingDir = Split-Path -Parent $linuxDir
$root = Split-Path -Parent $packagingDir

# --- docker доступен? -------------------------------------------------------
& docker info --format '{{.ServerVersion}}' | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw 'Docker не запущен. Запустите Docker Desktop и повторите.'
}

# --- образ ------------------------------------------------------------------
$buildArgs = @('build', '-t', $ImageName)
if ($RebuildImage) { $buildArgs += '--no-cache' }
$buildArgs += $linuxDir
Write-Host "==> docker build $ImageName"
& docker @buildArgs
if ($LASTEXITCODE -ne 0) { throw 'Сборка docker-образа завершилась ошибкой' }

# --- сборка в контейнере ----------------------------------------------------
# Репозиторий монтируется в /src (результат — dist/ появится на хосте),
# кеш тулчейна лежит в именованном томе /build.
$mount = $root -replace '\\', '/'
Write-Host "==> сборка под linux (deb + AppImage) из $mount"

& docker run --rm `
    --platform linux/amd64 `
    -v "${mount}:/src" `
    -v "${CacheVolume}:/build" `
    -e CARGO_HOME=/build/cargo `
    -e CARGO_TARGET_DIR=/build/target `
    -w /src `
    $ImageName `
    bash /src/packaging/linux/build-linux.sh

if ($LASTEXITCODE -ne 0) { throw 'Linux-сборка завершилась ошибкой' }
Write-Host 'Linux-пакеты готовы: dist\linux'

# --- проверка ---------------------------------------------------------------
if ($Verify) {
    Write-Host '==> проверка пакетов (verify-linux.sh)'
    & docker run --rm `
        --platform linux/amd64 `
        -v "${mount}:/src" `
        -w /src `
        $ImageName `
        bash /src/packaging/linux/verify-linux.sh
    if ($LASTEXITCODE -ne 0) { throw 'Проверка пакетов завершилась ошибкой' }
}
