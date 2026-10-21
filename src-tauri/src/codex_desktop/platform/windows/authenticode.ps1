$ErrorActionPreference = 'Stop'
$PSModuleAutoLoadingPreference = 'None'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
    $signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH -ErrorAction Stop
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { exit 1 }
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
        $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::ExcludeRoot
        $chain.ChainPolicy.VerificationFlags = [System.Security.Cryptography.X509Certificates.X509VerificationFlags]::NoFlag
        $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(10)
        if (-not $chain.Build($signature.SignerCertificate)) { exit 1 }
        $root = $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate
        # The fields are certificate values, never executable text. JSON is emitted
        # through the fully qualified utility module, independent of PSModulePath.
        Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" -ErrorAction Stop
        $result = [ordered]@{
            status = [string]$signature.Status
            signer = $signature.SignerCertificate.Subject
            chain_valid = $true
            root_subject = $root.Subject
            root_thumbprint = $root.Thumbprint
        }
        Microsoft.PowerShell.Utility\ConvertTo-Json -InputObject $result -Compress
    } finally { $chain.Dispose() }
} catch { exit 1 }
