$ErrorActionPreference = 'Stop'
$PSModuleAutoLoadingPreference = 'None'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

function Set-ChainPolicy($policy) {
    # Authenticode Status=Valid owns trusted timestamp semantics. Rebuilding
    # the signer chain still requires online leaf/intermediate revocation checks.
    $policy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $policy.RevocationFlag = [System.Security.Cryptography.X509Certificates.X509RevocationFlag]::ExcludeRoot
    $policy.VerificationFlags = [System.Security.Cryptography.X509Certificates.X509VerificationFlags]::IgnoreNotTimeValid
    $policy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(10)
}

function Get-ChainStatus($entries) {
    foreach ($entry in $entries) {
        # ChainStatus.Status is a flags enum, including combined flags.
        foreach ($flag in ([string]$entry.Status -split ', ')) { $flag }
    }
}

function Test-ChainResult([bool]$built, [string[]]$statuses, [int]$elementCount) {
    foreach ($status in $statuses) {
        if ($status -cne 'NotTimeValid') { return $false }
    }
    # Never accept an unexplained failed build or an empty chain.
    return (($built -or $statuses.Count -gt 0) -and $elementCount -gt 0)
}

function Write-SignatureEvidence($signature, $chain, [bool]$built) {
    $chainStatus = @(Get-ChainStatus $chain.ChainStatus)
    $valid = Test-ChainResult $built $chainStatus $chain.ChainElements.Count
    $root = $chain.ChainElements[$chain.ChainElements.Count - 1].Certificate
    Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" -ErrorAction Stop
    $result = [ordered]@{
        status = [string]$signature.Status
        signer = $signature.SignerCertificate.Subject
        chain_valid = $valid
        chain_status = @($chainStatus)
        timestamped = ($null -ne $signature.TimeStamperCertificate)
        root_subject = [string]$root.Subject
        root_thumbprint = [string]$root.Thumbprint
        policy = [ordered]@{
            revocation_mode = [string]$chain.ChainPolicy.RevocationMode
            revocation_flag = [string]$chain.ChainPolicy.RevocationFlag
            verification_flags = [string]$chain.ChainPolicy.VerificationFlags
        }
    }
    Microsoft.PowerShell.Utility\ConvertTo-Json -InputObject $result -Depth 3 -Compress
}

# Production entry point: no fixture switch or fixture environment is read.
try {
    Import-Module -Name "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" -ErrorAction Stop
    $signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature -LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH -ErrorAction Stop
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { exit 1 }
    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        Set-ChainPolicy $chain.ChainPolicy
        $built = $chain.Build($signature.SignerCertificate)
        Write-SignatureEvidence $signature $chain $built
    } finally { $chain.Dispose() }
} catch { exit 1 }
