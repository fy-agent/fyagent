param(
  [Parameter(Mandatory = $true)]
  [string]$ArtifactPath,
  [switch]$RequireSignTool
)

$env:PSModulePath = "$PSHOME\Modules"
$PSModuleAutoLoadingPreference = 'None'
Microsoft.PowerShell.Core\Import-Module `
  -Name "$PSHOME\Modules\Microsoft.PowerShell.Management\Microsoft.PowerShell.Management.psd1" `
  -Force `
  -ErrorAction Stop
Microsoft.PowerShell.Core\Import-Module `
  -Name "$PSHOME\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1" `
  -Force `
  -ErrorAction Stop
Microsoft.PowerShell.Core\Import-Module `
  -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" `
  -Force `
  -ErrorAction Stop
Microsoft.PowerShell.Core\Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'Stop'

function Get-CertificateSha256 {
  param(
    [Parameter(Mandatory = $true)]
    [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
  )

  $hasher = [System.Security.Cryptography.SHA256]::Create()
  try {
    return ([System.BitConverter]::ToString($hasher.ComputeHash($Certificate.RawData))).Replace('-', '').ToLowerInvariant()
  }
  finally {
    $hasher.Dispose()
  }
}

function Convert-CertificateEvidence {
  param(
    [Parameter(Mandatory = $true)]
    [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
  )

  $enhancedKeyUsageOids = @()
  foreach ($extension in $Certificate.Extensions) {
    if ($extension.Oid.Value -eq '2.5.29.37') {
      $enhancedKeyUsageExtension = if ($extension -is [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]) {
        $extension
      }
      else {
        [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new(
          $extension.RawData,
          $extension.Critical
        )
      }
      foreach ($usage in $enhancedKeyUsageExtension.EnhancedKeyUsages) {
        if ([string]::IsNullOrWhiteSpace($usage.Value)) {
          throw 'Authenticode certificate contains an empty enhanced-key-usage OID'
        }
        $enhancedKeyUsageOids += $usage.Value
      }
    }
  }

  return [ordered]@{
    subject = $Certificate.Subject
    simpleName = $Certificate.GetNameInfo(
      [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
      $false
    )
    sha256 = Get-CertificateSha256 -Certificate $Certificate
    notBefore = $Certificate.NotBefore.ToUniversalTime().ToString(
      'o',
      [System.Globalization.CultureInfo]::InvariantCulture
    )
    notAfter = $Certificate.NotAfter.ToUniversalTime().ToString(
      'o',
      [System.Globalization.CultureInfo]::InvariantCulture
    )
    enhancedKeyUsageOids = @(
      $enhancedKeyUsageOids | Microsoft.PowerShell.Utility\Sort-Object -Unique
    )
  }
}

$resolvedArtifact = (
  Microsoft.PowerShell.Management\Resolve-Path -LiteralPath $ArtifactPath
).Path
$artifact = Microsoft.PowerShell.Management\Get-Item `
  -LiteralPath $resolvedArtifact `
  -Force
if (-not ($artifact -is [System.IO.FileInfo])) {
  throw 'Authenticode evidence input must be a regular file'
}
if (($artifact.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
  throw 'Authenticode evidence input must not be a reparse point'
}
if ($artifact.Length -le 0) {
  throw 'Authenticode evidence input must not be empty'
}

if ($RequireSignTool) {
  $sdkBin = [IO.Path]::Combine(
    [Environment]::GetEnvironmentVariable('ProgramFiles(x86)'),
    'Windows Kits\10\bin'
  )
  $nativeArchitecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
  $sdkDirectories = Microsoft.PowerShell.Management\Get-ChildItem -LiteralPath $sdkBin -Directory |
    Microsoft.PowerShell.Core\Where-Object { $_.Name -match '^10\.0\.\d+\.\d+$' } |
    Microsoft.PowerShell.Utility\Sort-Object -Property { [version]$_.Name } -Descending
  $signtool = $null
  foreach ($sdk in $sdkDirectories) {
    $path = [IO.Path]::Combine($sdk.FullName, $nativeArchitecture, 'signtool.exe')
    if ([IO.File]::Exists($path)) {
      $signtool = Microsoft.PowerShell.Management\Get-Item -LiteralPath $path -Force
      break
    }
  }
  if ($null -eq $signtool -or
      ($signtool.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw 'Native Windows SDK SignTool is required to verify the Authenticode signature'
  }
  $PSNativeCommandUseErrorActionPreference = $false
  $signToolOutput = & $signtool.FullName verify /pa /all /v /tw $resolvedArtifact 2>&1
  $verificationExitCode = $LASTEXITCODE
  [Console]::Error.WriteLine(($signToolOutput -join [Environment]::NewLine))
  if ($verificationExitCode -ne 0) {
    throw "SignTool verify /pa /all /v /tw failed with exit code $verificationExitCode"
  }
}

$signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature `
  -LiteralPath $resolvedArtifact
$signerCertificate = if ($null -eq $signature.SignerCertificate) {
  $null
}
else {
  Convert-CertificateEvidence -Certificate $signature.SignerCertificate
}
$timestampCertificate = if ($null -eq $signature.TimeStamperCertificate) {
  $null
}
else {
  Convert-CertificateEvidence -Certificate $signature.TimeStamperCertificate
}
$publisher = if ($null -eq $signature.SignerCertificate) {
  $null
}
else {
  $signature.SignerCertificate.GetNameInfo(
    [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
    $false
  )
}

$evidence = [ordered]@{
  schema = 'fyagent-authenticode-evidence/v1'
  status = $signature.Status.ToString()
  publisher = $publisher
  signerCertificate = $signerCertificate
  timestampCertificate = $timestampCertificate
}

$evidence | Microsoft.PowerShell.Utility\ConvertTo-Json -Depth 6 -Compress
