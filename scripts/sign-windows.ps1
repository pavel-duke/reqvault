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
        throw 'The certificate thumbprint was not found in Windows Certificate Store.'
    }
    if (-not $certificate.HasPrivateKey) {
        throw 'The certificate does not have an accessible private key.'
    }
    if ($certificate.NotAfter -le (Get-Date)) {
        throw 'The certificate has expired.'
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
            throw "Tauri build failed with exit code $LASTEXITCODE."
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
        throw 'SignTool was not found. Install Windows SDK.'
    }

    $artifacts = @(
        Join-Path $projectRoot 'src-tauri\target\release\reqvault.exe'
        Join-Path $projectRoot 'src-tauri\target\release\bundle\msi\ReqVault_0.2.0_x64_en-US.msi'
        Join-Path $projectRoot 'src-tauri\target\release\bundle\nsis\ReqVault_0.2.0_x64-setup.exe'
    )
    foreach ($artifact in $artifacts) {
        if (-not (Test-Path -LiteralPath $artifact)) {
            throw "Artifact was not found: $artifact"
        }
        & $signTool.FullName verify /pa /v $artifact
        if ($LASTEXITCODE -ne 0) {
            throw "Signature verification failed: $artifact"
        }
    }
}
finally {
    if (Test-Path -LiteralPath $temporaryConfig) {
        Remove-Item -LiteralPath $temporaryConfig -Force
    }
}
