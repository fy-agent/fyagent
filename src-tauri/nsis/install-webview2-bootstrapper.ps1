param(
  [ValidateSet('Install', 'VerifyOnly')]
  [string]$Mode = 'Install',

  [string]$VerifyPath
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
Microsoft.PowerShell.Core\Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$strictDirectorySddl = 'O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)'
$strictFileSddl = 'O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)'
$allowedDirectorySddl = @(
  'O:SYD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)',
  'O:SYD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)',
  'O:SYD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)',
  'O:SYD:PAI(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)',
  'O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)',
  'O:BAD:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)',
  'O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)',
  'O:BAD:PAI(A;OICI;FA;;;BA)(A;OICI;FA;;;SY)'
)
$allowedFileSddl = @(
  'O:SYD:P(A;;FA;;;SY)(A;;FA;;;BA)',
  'O:SYD:PAI(A;;FA;;;SY)(A;;FA;;;BA)',
  'O:SYD:P(A;;FA;;;BA)(A;;FA;;;SY)',
  'O:SYD:PAI(A;;FA;;;BA)(A;;FA;;;SY)',
  'O:BAD:P(A;;FA;;;SY)(A;;FA;;;BA)',
  'O:BAD:PAI(A;;FA;;;SY)(A;;FA;;;BA)',
  'O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)',
  'O:BAD:PAI(A;;FA;;;BA)(A;;FA;;;SY)'
)

# Leaf certificates reviewed from the fixed Microsoft WebView2 fwlink below.
# Both chain to the pinned Microsoft Code Signing PCA 2024 key.
$allowedSignerCertificateSha256 = @(
  'CB97E8E85E8E9321FB2646E9574EFD17669B3B0581D24262AC7C8A227433A244', # 2026-08-09
  'E0113ED8F5AEBC48D48E53F699129D14DDEDCFCAE489ABA7DB411ED0838A14C0'  # 2026-11-20
)
$allowedCodeSigningPcaSpkiSha256 = @(
  '50E824592CAA59C7DB9615D676738C7E4EEE522622440C4C2152D0668D68C6D9'
)

function Get-Sha256Hex {
  param(
    [Parameter(Mandatory = $true)]
    [byte[]]$Bytes
  )

  $sha256 = [Security.Cryptography.SHA256]::Create()
  try {
    return [BitConverter]::ToString(
      $sha256.ComputeHash([byte[]]$Bytes)
    ).Replace('-', '')
  } finally {
    $sha256.Dispose()
  }
}

function ConvertTo-DerLength {
  param(
    [Parameter(Mandatory = $true)]
    [int]$Length
  )

  if ($Length -lt 0 -or $Length -gt 0x7fffffff) {
    throw 'DER length is outside the supported range.'
  }
  if ($Length -lt 0x80) {
    return [byte[]]@([byte]$Length)
  }
  $encoded = [Collections.Generic.List[byte]]::new()
  while ($Length -gt 0) {
    $encoded.Insert(0, [byte]($Length -band 0xff))
    $Length = $Length -shr 8
  }
  return [byte[]]@([byte](0x80 -bor $encoded.Count)) + $encoded.ToArray()
}

function Get-RsaSubjectPublicKeyInfoSha256 {
  param(
    [Parameter(Mandatory = $true)]
    [Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
  )

  if ($Certificate.PublicKey.Oid.Value -cne '1.2.840.113549.1.1.1') {
    throw "Reviewed Microsoft PCA key algorithm is not RSA: $($Certificate.PublicKey.Oid.Value)"
  }
  [byte[]]$algorithmIdentifier = @(
    0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86,
    0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00
  )
  [byte[]]$bitStringBody = @(
    [byte]0x00
  ) + [byte[]]$Certificate.PublicKey.EncodedKeyValue.RawData
  [byte[]]$bitString = @([byte]0x03) + @(
    ConvertTo-DerLength -Length $bitStringBody.Length
  ) + $bitStringBody
  [byte[]]$body = $algorithmIdentifier + $bitString
  [byte[]]$subjectPublicKeyInfo = @([byte]0x30) + @(
    ConvertTo-DerLength -Length $body.Length
  ) + $body
  return Get-Sha256Hex -Bytes $subjectPublicKeyInfo
}

function Get-OwnerDaclSddl {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $item = Microsoft.PowerShell.Management\Get-Item -LiteralPath $Path -Force
  if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "A secure WebView2 path is a reparse point: $Path"
  }
  $sections =
    [Security.AccessControl.AccessControlSections]::Owner -bor
    [Security.AccessControl.AccessControlSections]::Access
  return (
    Microsoft.PowerShell.Management\Get-Acl -LiteralPath $Path
  ).GetSecurityDescriptorSddlForm($sections)
}

function Assert-StrictSecurity {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path,

    [Parameter(Mandatory = $true)]
    [string[]]$AllowedSddl
  )

  $sddl = Get-OwnerDaclSddl -Path $Path
  if ($AllowedSddl -cnotcontains $sddl) {
    throw "A secure WebView2 path has an unexpected owner or DACL: $Path ($sddl)"
  }
}

function Assert-MicrosoftAuthenticode {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  $signature = Microsoft.PowerShell.Security\Get-AuthenticodeSignature `
    -LiteralPath $Path
  if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
    throw "WebView2 bootstrapper Authenticode status is $($signature.Status)."
  }
  $certificate = $signature.SignerCertificate
  if ($null -eq $certificate) {
    throw 'WebView2 bootstrapper has no signer certificate.'
  }
  if ($certificate.Subject -notmatch '(?:^|,\s*)O=Microsoft Corporation(?:,|$)') {
    throw "WebView2 bootstrapper signer is not Microsoft Corporation: $($certificate.Subject)"
  }

  $ekuExtension = $null
  foreach ($extension in $certificate.Extensions) {
    if ($extension.Oid.Value -eq '2.5.29.37') {
      $ekuExtension = $extension
      break
    }
  }
  if ($null -eq $ekuExtension) {
    throw 'WebView2 bootstrapper signer has no Enhanced Key Usage extension.'
  }
  $enhancedKeyUsage = [Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new(
    $ekuExtension.RawData,
    $false
  )
  if ($enhancedKeyUsage.EnhancedKeyUsages.Value -notcontains '1.3.6.1.5.5.7.3.3') {
    throw 'WebView2 bootstrapper signer is not authorized for code signing.'
  }

  $signerSha256 = Get-Sha256Hex -Bytes $certificate.RawData
  if ($allowedSignerCertificateSha256 -cnotcontains $signerSha256) {
    throw "WebView2 bootstrapper signer certificate is not reviewed: $signerSha256"
  }

  # The boolean constructor selects the LocalMachine chain engine. A user can
  # add a fake root to CurrentUser without consent from this elevated process;
  # that root must never establish trust for installer execution.
  $chain = [Security.Cryptography.X509Certificates.X509Chain]::new($true)
  try {
    $chain.ChainPolicy.RevocationMode =
      [Security.Cryptography.X509Certificates.X509RevocationMode]::Online
    $chain.ChainPolicy.RevocationFlag =
      [Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain
    $chain.ChainPolicy.VerificationFlags =
      [Security.Cryptography.X509Certificates.X509VerificationFlags]::NoFlag
    $chain.ChainPolicy.UrlRetrievalTimeout = [TimeSpan]::FromSeconds(30)
    if (-not $chain.Build($certificate)) {
      $chainStatuses = [Collections.Generic.List[string]]::new()
      foreach ($status in $chain.ChainStatus) {
        $chainStatuses.Add([string]$status.Status)
      }
      $chainErrors = [string]::Join(
        ', ',
        $chainStatuses.ToArray()
      )
      throw "WebView2 signer did not build in the LocalMachine chain engine: $chainErrors"
    }
    $pcaPinMatched = $false
    for ($index = 1; $index -lt $chain.ChainElements.Count; $index += 1) {
      $element = $chain.ChainElements[$index].Certificate
      $spkiSha256 = Get-RsaSubjectPublicKeyInfoSha256 -Certificate $element
      if ($allowedCodeSigningPcaSpkiSha256 -ccontains $spkiSha256) {
        $pcaPinMatched = $true
        break
      }
    }
    if (-not $pcaPinMatched) {
      throw 'WebView2 signer chain does not contain a reviewed Microsoft code-signing PCA key.'
    }
  } finally {
    $chain.Dispose()
  }
}

if ($Mode -eq 'VerifyOnly') {
  if ([string]::IsNullOrWhiteSpace($VerifyPath)) {
    throw 'VerifyOnly requires -VerifyPath.'
  }
  Assert-MicrosoftAuthenticode -Path $VerifyPath
  exit 0
}

# These production values are intentionally constants. Formal installer
# execution has no URL, publisher, launch-argument, or test-mode environment
# override.
$bootstrapperUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'
$bootstrapperArguments = @('/silent', '/install')
$maximumBootstrapperBytes = 64MB
$programDataRoot = (
  [Environment]::GetFolderPath([Environment+SpecialFolder]::CommonApplicationData)
)
if ([string]::IsNullOrWhiteSpace($programDataRoot) -or -not [IO.Path]::IsPathRooted($programDataRoot)) {
  throw 'Windows CommonApplicationData is unavailable.'
}
# This random, installer-ephemeral directory is deliberately not the retired
# `%ProgramData%\FyAgent\runtime` and does not depend on any legacy FyAgent
# parent. DirectoryInfo.Create with the final protected descriptor is atomic
# for this unguessable path; an unexpected collision or descriptor drift fails.
$stagePath = Join-Path $programDataRoot "FyAgent-WebView2-$([Guid]::NewGuid().ToString('N'))"
$bootstrapperPath = Join-Path $stagePath 'MicrosoftEdgeWebView2Setup.exe'
$writer = $null
$reader = $null
$response = $null
$responseStream = $null
$httpHandler = $null
$httpClient = $null
$cancellation = $null
$process = $null
$exitCode = 20

try {
  $directorySecurity = [Security.AccessControl.DirectorySecurity]::new()
  $directorySecurity.SetSecurityDescriptorSddlForm($strictDirectorySddl)
  $stage = [IO.DirectoryInfo]::new($stagePath)
  $stage.Create($directorySecurity)
  Assert-StrictSecurity -Path $stagePath -AllowedSddl $allowedDirectorySddl

  $fileSecurity = [Security.AccessControl.FileSecurity]::new()
  $fileSecurity.SetSecurityDescriptorSddlForm($strictFileSddl)
  $writer = [IO.FileStream]::new(
    $bootstrapperPath,
    [IO.FileMode]::CreateNew,
    [Security.AccessControl.FileSystemRights]::Write,
    [IO.FileShare]::None,
    65536,
    [IO.FileOptions]::WriteThrough,
    $fileSecurity
  )

  [void][Reflection.Assembly]::Load(
    'System.Net.Http, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a'
  )
  [Net.ServicePointManager]::SecurityProtocol =
    [Net.ServicePointManager]::SecurityProtocol -bor
    [Net.SecurityProtocolType]::Tls12
  $httpHandler = [Net.Http.HttpClientHandler]::new()
  $httpHandler.AllowAutoRedirect = $true
  $httpHandler.MaxAutomaticRedirections = 5
  $httpClient = [Net.Http.HttpClient]::new($httpHandler, $true)
  $httpClient.Timeout = [TimeSpan]::FromMinutes(2)
  $cancellation = [Threading.CancellationTokenSource]::new(
    [TimeSpan]::FromMinutes(2)
  )
  $response = $httpClient.GetAsync(
    $bootstrapperUrl,
    [Net.Http.HttpCompletionOption]::ResponseHeadersRead,
    $cancellation.Token
  ).GetAwaiter().GetResult()
  [void]$response.EnsureSuccessStatusCode()
  if (
    $null -eq $response.RequestMessage -or
    $null -eq $response.RequestMessage.RequestUri -or
    $response.RequestMessage.RequestUri.Scheme -cne [Uri]::UriSchemeHttps
  ) {
    throw 'WebView2 bootstrapper redirect resolved outside HTTPS.'
  }
  $contentLength = $response.Content.Headers.ContentLength
  if (
    $null -ne $contentLength -and
    ($contentLength -le 0 -or $contentLength -gt $maximumBootstrapperBytes)
  ) {
    throw 'WebView2 bootstrapper Content-Length is outside the allowed range.'
  }
  $responseStream = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()

  $buffer = [byte[]]::new(65536)
  [long]$totalBytes = 0
  while ((
    $read = $responseStream.ReadAsync(
      $buffer,
      0,
      $buffer.Length,
      $cancellation.Token
    ).GetAwaiter().GetResult()
  ) -gt 0) {
    $totalBytes += $read
    if ($totalBytes -gt $maximumBootstrapperBytes) {
      throw 'WebView2 bootstrapper exceeded the maximum expected size.'
    }
    $writer.Write($buffer, 0, $read)
  }
  if ($totalBytes -eq 0) {
    throw 'WebView2 bootstrapper download was empty.'
  }
  $writer.Flush($true)
  $writer.Dispose()
  $writer = $null
  Assert-StrictSecurity -Path $bootstrapperPath -AllowedSddl $allowedFileSddl

  # FileShare.Read prevents replacement or mutation while signature policy and
  # process execution consume the same fixed bytes.
  $reader = [IO.FileStream]::new(
    $bootstrapperPath,
    [IO.FileMode]::Open,
    [IO.FileAccess]::Read,
    [IO.FileShare]::Read
  )
  Assert-MicrosoftAuthenticode -Path $bootstrapperPath
  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $bootstrapperPath
  $startInfo.Arguments = $bootstrapperArguments -join ' '
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $process = [Diagnostics.Process]::new()
  $process.StartInfo = $startInfo
  if (-not $process.Start()) {
    throw 'WebView2 bootstrapper process did not start.'
  }
  $process.WaitForExit()
  $exitCode = $process.ExitCode
} catch {
  [Console]::Error.WriteLine(
    "Secure WebView2 bootstrapper installation failed: $($_.Exception.Message)"
  )
} finally {
  if ($null -ne $reader) {
    $reader.Dispose()
  }
  if ($null -ne $writer) {
    $writer.Dispose()
  }
  if ($null -ne $responseStream) {
    $responseStream.Dispose()
  }
  if ($null -ne $response) {
    $response.Dispose()
  }
  if ($null -ne $httpClient) {
    $httpClient.Dispose()
  }
  if ($null -ne $cancellation) {
    $cancellation.Dispose()
  }
  if ($null -eq $httpClient -and $null -ne $httpHandler) {
    $httpHandler.Dispose()
  }
  if ($null -ne $process) {
    $process.Dispose()
  }
  Microsoft.PowerShell.Management\Remove-Item `
    -LiteralPath $bootstrapperPath `
    -Force `
    -ErrorAction SilentlyContinue
  Microsoft.PowerShell.Management\Remove-Item `
    -LiteralPath $stagePath `
    -Force `
    -ErrorAction SilentlyContinue
}

exit $exitCode
