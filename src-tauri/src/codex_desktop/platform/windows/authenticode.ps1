$ErrorActionPreference = 'Stop'
$PSModuleAutoLoadingPreference = 'None'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
    $signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH -ErrorAction Stop
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { exit 1 }
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        # Status=Valid above is the trust decision (WinVerifyTrust handles trusted
        # timestamps). This chain only resolves the signer's root for pinning:
        # do not reject a historically valid signer for expiry or recheck current
        # revocation. Never derive trust from timestamp presence or package dates.
        $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
        $chain.ChainPolicy.VerificationFlags = [System.Security.Cryptography.X509Certificates.X509VerificationFlags]::IgnoreNotTimeValid
        $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(10)
        if (-not $chain.Build($signature.SignerCertificate)) { exit 1 }
        $root = $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate
        # The fields are certificate values, never executable text. JSON is emitted
        # through the fully qualified utility module, independent of PSModulePath.
        Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" -ErrorAction Stop
        $result = [ordered]@{
            status = [string]$signature.Status
            signer = $signature.SignerCertificate.Subject
            # This means root-chain resolution succeeded, not a second trust decision.
            chain_valid = $true
            timestamped = ($null -ne $signature.TimeStamperCertificate)
            root_subject = $root.Subject
            root_thumbprint = $root.Thumbprint
        }
        Microsoft.PowerShell.Utility\ConvertTo-Json -InputObject $result -Compress
    } finally { $chain.Dispose() }
} catch { exit 1 }
