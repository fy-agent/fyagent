param(
  [Parameter(Mandatory)]
  [ValidateSet('Prepare','CleanInstall','Uninstall','InstallPrevious','Upgrade')]
  [string]$Step
)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_REPOSITORY -cne 'fy-agent/fyagent' -or $env:QA_RUNNER_KIND -cne 'github-hosted' -or
    $env:GITHUB_REF -cne 'refs/heads/qa/0417-native-install-82d6204') {
  throw 'Use this diagnostic only on the named disposable GitHub-hosted QA runners.'
}
$arch = $env:PACKAGE_ARCH
if ($arch -cnotin @('x64','arm64') -or [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant() -cne $arch) {
  throw 'Native runner and installer architectures disagree.'
}
$candidate = Join-Path $env:GITHUB_WORKSPACE "candidate\FyAgent-0.4.17-Windows-$arch-setup.exe"
$previousName = "FyAgent-0.4.16-Windows-$arch-setup.exe"
$previous = Join-Path $env:RUNNER_TEMP $previousName
$install = Join-Path $env:ProgramW6432 'FyAgent'
$registry = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\FyAgent'
$exe = Join-Path $install 'fyagent.exe'
$remover = Join-Path $install 'uninstall.exe'
$profileDirectory = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.fyagent'
$marker = "qa-$env:GITHUB_RUN_ID-$env:GITHUB_RUN_ATTEMPT-$arch"
$profileMarker = Join-Path $profileDirectory "$marker.txt"
$unrelatedFile = Join-Path $install "$marker.txt"

function Assert-Preserved {
  foreach ($file in @($profileMarker,$unrelatedFile)) {
    if ([IO.File]::ReadAllText($file) -cne $marker) { throw "Marker changed: $file" }
  }
}

function Assert-Installed([string]$Version) {
  $registered = Get-ItemProperty -LiteralPath $registry
  if ($registered.DisplayVersion -cne $Version -or $registered.InstallLocation.Trim('"') -ne $install -or
      -not (Test-Path -LiteralPath $remover -PathType Leaf)) { throw "Missing or incorrect installation of $Version" }
  $bytes = [IO.File]::ReadAllBytes($exe)
  $peOffset = [BitConverter]::ToInt32($bytes,0x3c)
  $machine = [BitConverter]::ToUInt16($bytes,$peOffset + 4)
  $expected = if ($arch -eq 'arm64') { 0xaa64 } else { 0x8664 }
  if ($machine -ne $expected) { throw "Installed application is the wrong architecture: $machine" }
  Write-Host "Verified installed version $Version and native $arch application."
}

function Invoke-Installer([string]$File,[string]$Arguments) {
  $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "$(Split-Path -Leaf $File) exited $($process.ExitCode)" }
}

switch ($Step) {
  'Prepare' {
    if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $install)) { throw 'Runner already contains FyAgent' }
    $url = "https://github.com/fy-agent/fyagent/releases/download/v0.4.16/$previousName"
    Invoke-WebRequest -Uri $url -OutFile $previous
    if ((Get-FileHash -LiteralPath $previous).Hash.ToLowerInvariant() -cne $env:PREVIOUS_SHA256 -or
        (Get-FileHash -LiteralPath $candidate).Hash.ToLowerInvariant() -cne $env:CANDIDATE_SHA256) { throw 'Installer digest mismatch' }
    [void][IO.Directory]::CreateDirectory($profileDirectory)
    [IO.File]::WriteAllText($profileMarker,$marker)
    Write-Host "Release preflight 44733606638/1, source 82d6204908ea28d2dfb8a8af61cc7b67b65ea161, candidate $env:CANDIDATE_SHA256."
  }
  'CleanInstall' {
    Invoke-Installer $candidate '/S'
    Assert-Installed '0.4.17'
    [IO.File]::WriteAllText($unrelatedFile,$marker)
    Assert-Preserved
  }
  'Uninstall' {
    $external = Join-Path $env:RUNNER_TEMP ("qa-uninstall-" + [guid]::NewGuid().ToString('N') + ".exe")
    Copy-Item -LiteralPath $remover -Destination $external
    Invoke-Installer $external "/S _?=$install"
    if ((Test-Path -LiteralPath $exe) -or (Test-Path -LiteralPath $registry)) { throw 'FyAgent is still installed' }
    Assert-Preserved
    Write-Host 'Verified the application and registration were removed, and both marker files survived.'
  }
  'InstallPrevious' {
    Invoke-Installer $previous '/S'
    Assert-Installed '0.4.16'
    Assert-Preserved
  }
  'Upgrade' {
    Assert-Installed '0.4.16'
    Invoke-Installer $candidate '/S'
    Assert-Installed '0.4.17'
    Assert-Preserved
  }
}
