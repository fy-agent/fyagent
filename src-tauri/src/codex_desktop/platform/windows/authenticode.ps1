$ErrorActionPreference = 'Stop'
$PSModuleAutoLoadingPreference = 'None'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
    $signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH -ErrorAction Stop
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { exit 1 }
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        # PowerShell's Signature exposes the signer but not WinVerifyTrust's chain.
        # Rebuild from that signer for explicit revocation and root pinning. Keep
        # Authenticode Status=Valid authoritative for trusted timestamp semantics;
        # timestamp presence alone never grants trust or changes verification time.
        $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
        # Exclude the pinned trust anchor: roots are trusted by the system store
        # and the Rust subject/thumbprint pin, not by their own revocation service.
        # The leaf and every intermediate still require known revocation status.
        $chain.ChainPolicy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::ExcludeRoot
        $chain.ChainPolicy.VerificationFlags = [System.Security.Cryptography.X509Certificates.X509VerificationFlags]::IgnoreNotTimeValid
        $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(10)
        $built = $chain.Build($signature.SignerCertificate)
        # ChainStatus.Status is a flags enum, so split combined flags as well.
        $chainStatus = @(
            foreach ($entry in $chain.ChainStatus) {
                foreach ($flag in ([string]$entry.Status -split ', ')) {
                    $flag
                }
            }
        )
        foreach ($status in $chainStatus) {
            if ($status -cne 'NotTimeValid' -and $status -cne 'NotTimeNested') { exit 1 }
        }
        # A failed build is tolerable only with explicit time-only diagnostics.
        # Unknown, offline, revoked and every other non-time status fail above,
        # even if Build returned true. An unexplained failure must also fail shut.
        if ((-not $built -and $chainStatus.Count -eq 0) -or $chain.ChainElements.Count -eq 0) { exit 1 }
        $root = $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate
        # The fields are certificate values, never executable text. JSON is emitted
        # through the fully qualified utility module, independent of PSModulePath.
        Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" -ErrorAction Stop
        $result = [ordered]@{
            status = [string]$signature.Status
            signer = $signature.SignerCertificate.Subject
            # The explicit revocation/time-only policy above passed.
            chain_valid = $true
            chain_status = @($chainStatus)
            timestamped = ($null -ne $signature.TimeStamperCertificate)
            root_subject = $root.Subject
            root_thumbprint = $root.Thumbprint
        }
        Microsoft.PowerShell.Utility\ConvertTo-Json -InputObject $result -Compress
    } finally { $chain.Dispose() }
} catch { exit 1 }
