param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Fa-f0-9]{40}$')]
    [string]$CertificateThumbprint,

    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$temporaryConfig = Join-Path $projectRoot 'src-tauri\tauri.signing.local.json'

try {
    $certificate = Get-ChildItem -Path Cert:\CurrentUser\My, Cert:\LocalMachine\My |
        Where-Object { $_.Thumbprint -eq $CertificateThumbprint } |
        Select-Object -First 1
    if (-not $certificate) {
        throw 'Сертификат с таким thumbprint не найден в Windows Certificate Store.'
    }
    if (-not $certificate.HasPrivateKey) {
        throw 'У сертификата нет доступного приватного ключа.'
    }
    if ($certificate.NotAfter -le (Get-Date)) {
        throw 'Срок действия сертификата закончился.'
    }

    $config = @{
        bundle = @{
            windows = @{
                certificateThumbprint = $CertificateThumbprint
                digestAlgorithm = 'sha256'
                timestampUrl = $TimestampUrl
            }
        }
    } | ConvertTo-Json -Depth 5
    Set-Content -LiteralPath $temporaryConfig -Value $config -Encoding utf8

    Push-Location $projectRoot
    try {
        npm run tauri build -- --config $temporaryConfig
        if ($LASTEXITCODE -ne 0) {
            throw "Tauri build завершился с кодом $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }

    $signTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Filter signtool.exe -Recurse |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $signTool) {
        throw 'SignTool не найден. Установите Windows SDK.'
    }

    $artifacts = @(
        Join-Path $projectRoot 'src-tauri\target\release\reqvault.exe'
        Join-Path $projectRoot 'src-tauri\target\release\bundle\msi\ReqVault_0.2.0_x64_en-US.msi'
        Join-Path $projectRoot 'src-tauri\target\release\bundle\nsis\ReqVault_0.2.0_x64-setup.exe'
    )
    foreach ($artifact in $artifacts) {
        if (-not (Test-Path -LiteralPath $artifact)) {
            throw "Артефакт не найден: $artifact"
        }
        & $signTool.FullName verify /pa /v $artifact
        if ($LASTEXITCODE -ne 0) {
            throw "Проверка подписи не прошла: $artifact"
        }
    }
}
finally {
    if (Test-Path -LiteralPath $temporaryConfig) {
        Remove-Item -LiteralPath $temporaryConfig -Force
    }
}
