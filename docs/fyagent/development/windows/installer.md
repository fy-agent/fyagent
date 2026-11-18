# FyAgent Windows installer flow

FyAgent's Windows distribution is built as architecture-specific NSIS setup
executables on matching native Windows runners. Current Tauri/NSIS
configuration, release scripts, workflow jobs, and executable tests define the
path, privilege, signing, uninstall, and packaging behavior. Retained Windows
installer notes under `.trellis/spec/` are optional AI-assistance review
material.

## Build and evidence flow

```text
canonical application version
  -> Windows-only Tauri configuration
  -> standard NSIS directory selection
  -> architecture-specific setup executable
  -> signing-policy transform or unsigned proof
  -> post-transform digest and signature verification
  -> aggregate release metadata and attestation
```

Interactive directory selection and a final silent `/D=` value use standard
NSIS handling. FyAgent does not add an absolute/fixed/local/UNC/reparse/
drive-type, ACL, directory-owner, or protected-folder admission rule for the
chosen installation directory. NSIS/Windows may still reject malformed input,
and an actual filesystem or registration write can still fail.

Before install/update or uninstall changes anything, the template checks both
`fyagent.exe` and the fixed `fyagent-user-helper.exe`. It does not force either
process to terminate. An interactive user must close it normally and can Retry
or Cancel; `/S` and passive mode abort immediately while either process is
running. The same gate protects the maintenance path that launches the existing
uninstaller, and it always runs before installer hooks, cleanup, package
migration, or payload changes.

The process check is deliberately described as fail-closed at observation
time, not as an atomic ban on a later user launch. If an old main/helper image
starts after the final check, setup never kills it or assumes its pinned handle
was released; Windows preserves the live image and a conflicting replacement
may fail so the user can close the application normally and retry. A true
cross-version launch interlock would require application or system-policy
cooperation outside this NSIS-only installer contract.

The only retired-package bridge is the public v0.3.0 WiX MSI. Setup selects the
frozen architecture-specific ProductCode (x64
`{D50D8CE2-B49A-41DE-839D-6574FB69ADC1}` or ARM64
`{78F69296-A73D-40CA-A2BA-11D117AA2C9B}`), queries only that product, and—when
registered—synchronously runs the fixed quiet, no-restart uninstall command.
The query loads the Windows Installer API through the explicit
`$SYSDIR\msi.dll` path; elevated setup does not use a bare DLL search.
It accepts only Windows Installer results 0, 1605, and 1614. Result 3010 tells
the user to restart and aborts; process-launch failure and every other result
also abort before a new payload is written. No product enumeration,
`UninstallString`, or registry-supplied executable command is used.

Setup captures the old MSI's named
`HKLM\Software\fyagent\FyAgent\InstallDir` marker before removal. A final
explicit `/D=` always wins; otherwise an existing NSIS path wins, and the old
MSI path is only the fallback. Once the fixed product is confirmed absent or
successfully removed, the old named marker is deleted and normal NSIS
registration takes over. The first running-process gate and migration run in
`EarlyChecks`, before the WebView2 section; the Install section then repeats
both process checks before `SetOutPath`, hooks, cleanup, and payload copying.

The former `%ProgramData%\FyAgent\runtime` is not provisioned or admitted. On
install and uninstall, the template opens the fixed FyAgent parent as a
no-follow anchor and opens `runtime` relative to that held parent. Enumeration
produces names only; a complete `business-*.state` or `business-*.lock` name
with the fixed prefix and suffix in lowercase must then be opened relative to the held runtime handle
and proven to be a regular non-reparse file before same-handle deletion. Empty
known directories are also retired through their held handles. There is no
wildcard/path delete or path-based directory removal. Missing, reparse,
inaccessible, nonempty, malformed, concurrently changing, or unknown content
is preserved and never blocks setup.

That legacy path is not the Codex package bridge.
A distinct PackageBridge is independent of the retired ProgramData runtime.
It is fixed at
`$COMMONPROGRAMDATA\FyAgent.PackageBridge-{A244DC78-3F27-4FA8-90F9-BE5B39404D63}\v1`.
NSIS never owns or removes PackageBridge. It also never enumerates, opens,
repairs, or changes ACLs on the bridge or its immutable operation orphans. The
bridge module owns both cleanup paths and all orphan handling. Normal cleanup
settles a proven operation; the next elevated bridge creation owns known-only
orphan cleanup, so a valid orphan may outlive uninstall.

Codex installer staging has the same best-effort ownership boundary at the
fixed `$INSTDIR\cache\codex-installer` root. Setup enumerates only direct
children whose complete name is a lowercase canonical UUID and whose directory
is not a reparse point. `cache` is the single fixed full-path anchor;
`codex-installer` is opened relative to its handle, and every admitted UUID job
is opened relative to the staging handle. The full-path enumeration supplies
candidate names only and never supplies a deletion capability. Within an
admitted job, cleanup considers only exact `installer.msix` and
`installer.msix.part` files. Each leaf is opened without following reparse
points by resolving only that fixed filename relative to the already-open job
handle. After the cache anchor is held, no descendant is reopened through the
mutable full staging path. A returned leaf handle must describe a regular
non-reparse file and is marked for deletion through that same handle; there is
no validation followed by a second path-based delete. The job, staging, and
`cache` directories are likewise marked through their held handles and only
disappear when empty. Unknown files/directories, malformed job names, leaf or
directory reparse points, handle/disposition failures, and other cleanup
failures survive install/upgrade and uninstall.

The NSIS setup and its System plug-in remain a 32-bit PE control process for
both x64 and ARM64 payloads. Its handle-relative native call therefore uses the
packed 32-bit `UNICODE_STRING` (8 bytes, `Buffer` at offset 4),
`OBJECT_ATTRIBUTES` (24 bytes, `RootDirectory` at offset 4), and
`IO_STATUS_BLOCK` (8 bytes) layouts. Runtime size/offset guards fail closed for
that relative directory or leaf if this ABI ever drifts; they do not fall back
to a full-path open.

The WebView2 bootstrapper uses a separate GUID-named, descriptor-protected
ephemeral directory directly below Windows CommonApplicationData. It does not
depend on or recreate the retired FyAgent runtime parent.

`icons/icon.ico` is the canonical Windows package icon. The Windows Tauri
configuration supplies it to the checked-in template, which uses it for both
the setup and uninstaller UI.

The installer owns its known main executable, fixed
`fyagent-user-helper.exe`, shortcuts, registration, uninstaller, and
empty-directory cleanup below the install root. It owns neither PackageBridge
settlement nor PackageBridge cleanup and does not recursively own the
installation directory: unrelated files beside the known payload survive.
Application database, configuration, OAuth, backup, and other user state have
separate runtime owners and must survive uninstall.

## Where to inspect

- `src-tauri/tauri.windows.conf.json` selects the Windows bundle surface.
- `src-tauri/nsis/` contains the repository-owned NSIS integration.
- `scripts/release/` contains signing and asset evidence helpers plus an
  optional manual lifecycle diagnostic.
- `.github/workflows/ci.yml` and `.github/workflows/release.yml` run the
  matching-architecture build/package and release-evidence gates. Release does
  not run install/verify/uninstall lifecycle jobs.
- `tests/windowsNsisContract.test.ts` and the release-contract suites provide
  portable policy coverage; native runner jobs provide actual build/package
  evidence. `verify-windows-nsis-lifecycle.ps1` can be run manually for bounded
  install/uninstall diagnostics but is not a publication gate.

Use the [validation guide](../validation.md) to distinguish local contract
checks from native package acceptance.
