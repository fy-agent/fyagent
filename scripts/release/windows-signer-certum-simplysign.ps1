# Certum / SimplySign Authenticode adapter for FyAgent Windows installers.
# Invoked by scripts/release/windows-signing.mjs as:
#   pwsh -NoLogo -NoProfile -NonInteractive -File <this-file> -ArtifactPath <absolute> -Architecture x64|arm64
# The private key stays in the SimplySign virtual smart card. This file selects
# that certificate and applies an RFC 3161 timestamp. It does not contain a
# password, PIN, OTP, token, or PFX. A missing credential fails closed and does
# not leave the installer unsigned-as-success.
# A signature produced on a local AIMaster machine is not CI evidence.

param(
  [Parameter(Mandatory = $true)]
  [string]$ArtifactPath,
  [Parameter(Mandatory = $true)]
  [ValidateSet('x64', 'arm64')]
  [string]$Architecture
)

$env:PSModulePath = "$PSHOME\Modules"
$PSModuleAutoLoadingPreference = 'None'
Microsoft.PowerShell.Core\Import-Module `
  -Name "$PSHOME\Modules\Microsoft.PowerShell.Management\Microsoft.PowerShell.Management.psd1" `
  -Force `
  -ErrorAction Stop
Microsoft.PowerShell.Core\Import-Module `
  -Name "$PSHOME\Modules\Microsoft.PowerShell.Utility\Microsoft.PowerShell.Utility.psd1" `
  -Force `
  -ErrorAction Stop
Microsoft.PowerShell.Core\Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'Stop'

$CertumProvider = 'certum-simplysign'
$CertumTimestampUrl = 'http://time.certum.pl'
$CredentialMaxLength = 8192

function Test-CredentialText {
  param(
    [AllowNull()]
    [AllowEmptyString()]
    [string]$Value
  )

  if ([string]::IsNullOrEmpty($Value)) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL is required; refusing to leave the artifact unsigned'
  }
  if ($Value.Length -gt $CredentialMaxLength) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL is too long'
  }
  if ($Value -cne $Value.Trim()) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL must not have outer whitespace'
  }
  foreach ($char in $Value.ToCharArray()) {
    $code = [int]$char
    if (($code -ge 0 -and $code -le 31) -or $code -eq 127) {
      throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL contains control data'
    }
  }
}

function Test-SigntoolPathText {
  param(
    [AllowNull()]
    [string]$Path
  )

  $drive = $Path -match '^[A-Za-z]:[\\/]'
  $unc = $Path -match '^\\\\[^\\]+\\'
  $leaf = [IO.Path]::GetFileName([string]$Path)
  $hasParentSegment = [string]$Path -match '(^|[\\/])\.\.([\\/]|$)'
  if (
    -not ($Path -is [string]) -or
    -not ($drive -or $unc) -or
    $leaf.ToLowerInvariant() -cne 'signtool.exe' -or
    $hasParentSegment
  ) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL signtoolPath must be an absolute path to signtool.exe'
  }
}

function Read-CertumSimplySignCredential {
  $credentialRaw = [Environment]::GetEnvironmentVariable(
    'FYAGENT_WINDOWS_SIGNER_CREDENTIAL',
    'Process'
  )
  Test-CredentialText -Value $credentialRaw

  try {
    $parsed = $credentialRaw | Microsoft.PowerShell.Utility\ConvertFrom-Json -ErrorAction Stop
  }
  catch {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL must be a JSON object'
  }
  finally {
    $credentialRaw = $null
    [Environment]::SetEnvironmentVariable(
      'FYAGENT_WINDOWS_SIGNER_CREDENTIAL',
      $null,
      'Process'
    )
  }

  if ($parsed -isnot [System.Management.Automation.PSCustomObject]) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL must be a JSON object'
  }

  $allowed = @('provider', 'timestampUrl', 'certificateSha1', 'signtoolPath')
  foreach ($property in @($parsed.PSObject.Properties)) {
    if ($allowed -notcontains $property.Name) {
      throw "FYAGENT_WINDOWS_SIGNER_CREDENTIAL contains an unexpected field: $($property.Name)"
    }
  }
  foreach ($key in @('provider', 'timestampUrl', 'certificateSha1')) {
    if ($null -eq $parsed.PSObject.Properties[$key]) {
      throw "FYAGENT_WINDOWS_SIGNER_CREDENTIAL is missing $key"
    }
  }
  if (
    $parsed.provider -isnot [string] -or
    $parsed.provider -cne $CertumProvider
  ) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL provider must be certum-simplysign'
  }
  if (
    $parsed.timestampUrl -isnot [string] -or
    $parsed.timestampUrl -cne $CertumTimestampUrl
  ) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL timestampUrl must be the Certum RFC 3161 endpoint http://time.certum.pl'
  }
  if (
    $parsed.certificateSha1 -isnot [string] -or
    $parsed.certificateSha1 -cnotmatch '^[0-9a-f]{40}$'
  ) {
    throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL certificateSha1 must be a 40-character SHA-1 thumbprint'
  }

  $signtoolPath = $null
  $signtoolProperty = $parsed.PSObject.Properties['signtoolPath']
  if ($null -ne $signtoolProperty) {
    Test-SigntoolPathText -Path $signtoolProperty.Value
    $signtoolPath = [string]$signtoolProperty.Value
  }

  return [ordered]@{
    provider = $CertumProvider
    timestampUrl = $CertumTimestampUrl
    certificateSha1 = [string]$parsed.certificateSha1
    signtoolPath = $signtoolPath
    fileDigest = 'sha256'
    timestampDigest = 'sha256'
  }
}

function Resolve-SigningTool {
  param(
    [AllowNull()]
    [string]$ConfiguredPath
  )

  if (-not [string]::IsNullOrEmpty($ConfiguredPath)) {
    try {
      $tool = Microsoft.PowerShell.Management\Get-Item -LiteralPath $ConfiguredPath -Force -ErrorAction Stop
    }
    catch {
      throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL signtoolPath must be an absolute path to signtool.exe'
    }
    $toolIsInvalid = (
      -not ($tool -is [IO.FileInfo]) -or
      ($tool.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
      $tool.Name.ToLowerInvariant() -cne 'signtool.exe'
    )
    if ($toolIsInvalid) {
      throw 'FYAGENT_WINDOWS_SIGNER_CREDENTIAL signtoolPath must be an absolute path to signtool.exe'
    }
    return $tool.FullName
  }

  $found = @(
    Microsoft.PowerShell.Core\Get-Command `
      -Name 'signtool.exe' `
      -CommandType Application `
      -ErrorAction SilentlyContinue
  )
  if ($found.Count -eq 0) {
    throw 'signtool.exe was not found; refusing to leave the artifact unsigned'
  }
  if ($found.Count -ne 1) {
    throw 'Expected exactly one signtool.exe on PATH; refusing to leave the artifact unsigned'
  }
  return [string]$found[0].Source
}

function Resolve-InstallerFile {
  if (-not [IO.Path]::IsPathRooted($ArtifactPath)) {
    throw "ArtifactPath for $Architecture must be an absolute path"
  }
  $resolvedArtifact = (
    Microsoft.PowerShell.Management\Resolve-Path -LiteralPath $ArtifactPath
  ).Path
  $canonicalInput = [IO.Path]::GetFullPath($ArtifactPath)
  if ($resolvedArtifact -cne $canonicalInput) {
    throw "ArtifactPath for $Architecture must already be a canonical absolute path"
  }
  $artifact = Microsoft.PowerShell.Management\Get-Item -LiteralPath $resolvedArtifact -Force
  $artifactIsInvalid = (
    -not ($artifact -is [IO.FileInfo]) -or
    ($artifact.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
    $artifact.Length -le 0 -or
    $artifact.Extension.ToLowerInvariant() -cne '.exe'
  )
  if ($artifactIsInvalid) {
    throw "Windows $Architecture installer must be a non-empty regular .exe file"
  }
  return $resolvedArtifact
}

# Fail closed before the installer is opened or signtool starts.
$credential = Read-CertumSimplySignCredential
$resolvedArtifact = Resolve-InstallerFile
$signtool = Resolve-SigningTool -ConfiguredPath $credential.signtoolPath

# /sha1 selects the SimplySign certificate. /fd and /td are the file and
# RFC 3161 digest. /tr is required; the legacy /t Authenticode timestamp
# protocol is intentionally absent. signtool may update only the PE checksum
# and the security directory.
$signArguments = @(
  'sign',
  '/sha1', $credential.certificateSha1,
  '/fd', $credential.fileDigest,
  '/tr', $credential.timestampUrl,
  '/td', $credential.timestampDigest,
  '/v',
  $resolvedArtifact
)
& $signtool @signArguments
if ($LASTEXITCODE -ne 0) {
  throw "signtool sign failed with exit code $LASTEXITCODE; refusing to accept an unsigned artifact"
}

$verifyArguments = @(
  'verify',
  '/pa',
  '/all',
  '/v',
  '/tw',
  $resolvedArtifact
)
& $signtool @verifyArguments
if ($LASTEXITCODE -ne 0) {
  throw "signtool verify /pa /all /v /tw failed with exit code $LASTEXITCODE"
}
