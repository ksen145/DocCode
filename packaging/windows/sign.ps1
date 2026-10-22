<#
.SYNOPSIS
  Подписывает exe/msi/installer сертификатом Code Signing.

.DESCRIPTION
  Поддерживает два источника ключа:
  - PFX-файл: -PfxPath + пароль из DOCCODE_PFX_PASSWORD или интерактивного ввода;
  - хранилище сертификатов Windows: -Thumbprint (CurrentUser\My или LocalMachine\My).

  Подпись: SHA-256 + RFC 3161 timestamp. После подписи выполняется проверка
  signtool verify /pa.

.EXAMPLE
  $env:DOCCODE_PFX_PASSWORD = 'secret'
  .\packaging\windows\sign.ps1 -Path .\dist\windows\DocCode-<ver>-x64-setup.exe `
      -PfxPath C:\secure\doccode.pfx

.EXAMPLE
  .\packaging\windows\sign.ps1 -Path .\target\release\doccode.exe `
      -Thumbprint 0123456789ABCDEF0123456789ABCDEF01234567
#>
[CmdletBinding(DefaultParameterSetName = 'Pfx')]
param(
    # Один или несколько файлов для подписи.
    [Parameter(Mandatory = $true, Position = 0)]
    [string[]]$Path,

    # PFX-файл с закрытым ключом.
    [Parameter(ParameterSetName = 'Pfx')]
    [string]$PfxPath = $env:DOCCODE_PFX_PATH,

    # Пароль PFX. По умолчанию берётся из DOCCODE_PFX_PASSWORD.
    [Parameter(ParameterSetName = 'Pfx')]
    [string]$Password = $env:DOCCODE_PFX_PASSWORD,

    # Отпечаток сертификата в хранилище Windows.
    [Parameter(ParameterSetName = 'Store', Mandatory = $true)]
    [string]$Thumbprint,

    # Искать сертификат в LocalMachine\My вместо CurrentUser\My.
    [Parameter(ParameterSetName = 'Store')]
    [switch]$MachineStore,

    # RFC 3161 timestamp-сервер.
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'

function Find-SignTool {
    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "$env:ProgramFiles\Windows Kits\10\bin"
    ) | Where-Object { $_ -and (Test-Path $_) }

    foreach ($root in $roots) {
        $found = Get-ChildItem $root -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

$signtool = Find-SignTool
if (-not $signtool) {
    throw 'signtool.exe не найден. Установите Windows SDK (Windows Kits\10\bin\x64).'
}

if ($PSCmdlet.ParameterSetName -eq 'Pfx') {
    if (-not $PfxPath) {
        throw 'Укажите -PfxPath или переменную окружения DOCCODE_PFX_PATH.'
    }
    $PfxPath = (Resolve-Path -LiteralPath $PfxPath).Path
    if (-not (Test-Path -LiteralPath $PfxPath)) {
        throw "PFX-файл не найден: $PfxPath"
    }
    if (-not $Password) {
        $secure = Read-Host 'Пароль PFX' -AsSecureString
        $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
        try {
            $Password = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
        } finally {
            [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
        }
    }
}

$common = @('sign', '/fd', 'SHA256', '/td', 'SHA256', '/tr', $TimestampUrl)
if ($PSCmdlet.ParameterSetName -eq 'Pfx') {
    $common += @('/f', $PfxPath, '/p', $Password)
} else {
    $common += @('/sha1', ($Thumbprint -replace '\s', '').ToUpperInvariant())
    if ($MachineStore) { $common += '/sm' }
}

foreach ($item in $Path) {
    $file = (Resolve-Path -LiteralPath $item).Path
    Write-Host "Подпись: $file"
    & $signtool @common $file
    if ($LASTEXITCODE -ne 0) {
        throw "signtool sign завершился с кодом $LASTEXITCODE для $file"
    }

    & $signtool verify /pa /v $file
    if ($LASTEXITCODE -ne 0) {
        throw "signtool verify не подтвердил подпись: $file"
    }
}

Write-Host 'Подпись и проверка завершены.'
