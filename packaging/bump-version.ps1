<#
.SYNOPSIS
  Меняет версию DocCode в одном месте — `version` в Cargo.toml.

.DESCRIPTION
  Версия хранится только в Cargo.toml. Её читают:
    * NSIS-установщик (packaging/windows/build-installer.ps1);
    * Linux-пакеты (packaging/linux/build-linux.sh);
    * workflow релиза (.github/workflows/release.yml);
    * код приложения (doccode::VERSION = env!("CARGO_PKG_VERSION")).

  Скрипт проверяет semver, правит Cargo.toml и запись `doccode` в Cargo.lock
  (чтобы `cargo build --locked` в CI не ругался) и проверяет результат
  через `cargo metadata --locked`.

.EXAMPLE
  powershell -File packaging\bump-version.ps1 0.2.0
  git commit -am "release: 0.2.0"
  git tag v0.2.0
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'

if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
    throw "Ожидается semver вида 1.2.3 или 1.2.3-rc.1, получено: $Version"
}

$packagingDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $packagingDir
$cargoToml = Join-Path $root 'Cargo.toml'
$cargoLock = Join-Path $root 'Cargo.lock'

function Set-FirstMatchVersion {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$NewVersion,
        [string]$Label
    )

    $text = [IO.File]::ReadAllText($Path)
    $match = [regex]::Match($text, $Pattern)
    if (-not $match.Success) {
        throw "Не нашёл $Label в $Path"
    }

    $prefix = $match.Groups[1].Value
    $suffix = $match.Groups[3].Value
    $updated = $text.Substring(0, $match.Index) + $prefix + $NewVersion + $suffix +
        $text.Substring($match.Index + $match.Length)

    [IO.File]::WriteAllText($Path, $updated, (New-Object System.Text.UTF8Encoding($false)))
    return $match.Groups[2].Value
}

# Первый version = "..." в Cargo.toml — это версия пакета ([package] идёт первым).
$oldVersion = Set-FirstMatchVersion -Path $cargoToml `
    -Pattern '(?m)^(\s*version\s*=\s*")([^"]+)(")' `
    -NewVersion $Version `
    -Label 'version в [package]'

# Cargo.lock: запись локального пакета должна совпадать, иначе --locked упадёт.
Set-FirstMatchVersion -Path $cargoLock `
    -Pattern '(?ms)(\[\[package\]\]\r?\nname = "doccode"\r?\nversion = ")([^"]+)(")' `
    -NewVersion $Version `
    -Label 'пакет doccode' | Out-Null

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargo) {
    Push-Location $root
    try {
        & $cargo.Source metadata --locked --no-deps --format-version 1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'cargo metadata --locked не подтвердил Cargo.lock' }
    } finally {
        Pop-Location
    }
} else {
    Write-Warning 'cargo не найден в PATH — Cargo.lock не проверен.'
}

Write-Host "Версия: $oldVersion -> $Version"
Write-Host "Дальше: git commit -am `"release: $Version`" и git tag v$Version"
