# Windows Installer Contract

## 1. Scope / Trigger

Read this contract before changing the Windows Tauri bundle configuration,
NSIS template or hooks, install-directory selection, WebView2 bootstrapper,
legacy runtime cleanup, uninstall ownership, Windows signing adapter, or
native installer packaging. It owns installer mechanics and per-asset Windows
evidence. The GitHub job graph, frozen release identity, cross-platform asset
set, attestation, and publication transaction remain owned by
[GitHub Release Workflow](./github-release-workflow.md). Shell-user startup,
per-user paths, and untrusted single-instance activation are owned by
[Windows Runtime Security](./windows-runtime-security.md). The Codex
one-operation PackageBridge is owned by the running application's installer
module, not by NSIS; its ACL, settlement, and orphan-cleanup contract is owned by
[Codex Desktop Installer](./codex-desktop-installer.md).

Windows x64 and ARM64 installer claims require matching native hosted runners.
Local structure tests, cross-compilation, or inspection of the setup launcher
cannot replace matching native build and package evidence. The repository's
install/verify/uninstall lifecycle script is a manual diagnostic and is not a
Release workflow gate.

## 2. Signatures

```text
FYAGENT_WINDOWS_MANIFEST = release | test | dev

FyAgent-<version>-Windows-x64-setup.exe
FyAgent-<version>-Windows-arm64-setup.exe

FYAGENT_WINDOWS_SIGNING_MODE = unsigned | provider
FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER
FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256
```

```text
node scripts/release/verify-windows-nsis-contract.mjs

node scripts/release/windows-signing.mjs asset \
  --asset <setup.exe> \
  --architecture <x64|arm64> \
  --version <X.Y.Z> \
  --source-sha <sha> \
  --output <native-fragment.json>

node scripts/release/windows-signing.mjs transform \
  --asset <untrusted-formal-setup.exe> \
  --architecture <x64|arm64> \
  --version <X.Y.Z> \
  --source-sha <sha>

node scripts/release/windows-signing.mjs verify-sealed \
  --raw <frozen-raw-setup.exe> \
  --candidate <untrusted-formal-setup.exe> \
  --architecture <x64|arm64> \
  --version <X.Y.Z> \
  --source-sha <sha> \
  --mode <unsigned|provider> \
  [--expected-publisher <publisher> \
   --expected-certificate-sha256 <sha256>] \
  --output <trusted-native-fragment.json>

node scripts/release/windows-signing.mjs aggregate \
  --x64-status <x64-fragment.json> \
  --arm64-status <arm64-fragment.json> \
  --assets-directory <directory> \
  --version <X.Y.Z> \
  --source-sha <sha> \
  --output signing-status.json

./scripts/release/verify-windows-nsis-lifecycle.ps1 \
  -InstallerPath <setup.exe> \
  -Architecture <x64|arm64> \
  -AppVersion <X.Y.Z>
```

Public signing metadata uses `fyagent-windows-signing-status/v1`. Private
per-architecture fragments use `fyagent-windows-signing-asset/v1` and never
become a second public signing authority.

## 3. Contracts

### Build and bundle selection

- `build.rs` selects the embedded Windows manifest from
  `FYAGENT_WINDOWS_MANIFEST`. Only `release` enables
  `fyagent_windows_release`; `test` and `dev` select the ordinary-user
  manifest. A release-profile build without an explicit selection fails.
- `fyagent-release.manifest` uses `requireAdministrator`; test/dev use
  `asInvoker`. Packaging verifies the application executable's exact PE
  Machine, execution level, and `uiAccess=false` before accepting an asset.
- Cross-platform targets remain in the base Tauri configuration.
  `tauri.windows.conf.json` is the Windows-only merge layer and selects exactly
  `nsis`.
- The NSIS bundle is `perMachine`, defaults to
  `C:\Program Files\FyAgent`, uses English and Simplified Chinese selected from
  the OS language without a selector, and configures WebView2 as
  `downloadBootstrapper`. Silent `/S` and a final
  `/D=<install-directory>` are supported.
- `bundle.windows.nsis.installerIcon` is exactly `icons/icon.ico`. The checked-in
  template applies that canonical FyAgent icon to both `MUI_ICON` and
  `MUI_UNICON`, so setup and uninstall surfaces share the product identity.
  Configuration text alone is not acceptance: each raw native setup and each
  final sealed x64/ARM64 setup must contain exactly one `RT_GROUP_ICON`, whose
  referenced `RT_ICON` frames match `icons/icon.ico` in order, metadata, and
  raw bytes. Extra/default groups or unreferenced frames fail packaging or
  final asset verification.
- The checked-in template is a minimal derivative of the template embedded by
  the locked Tauri CLI. The source verifier pins its upstream tag, commit, and
  SHA-256. A template change must retain the documented Tauri merge boundary,
  standard NSIS directory page, bounded legacy cleanup, and bounded uninstall
  behavior.
- The Windows override bundles exactly the fixed
  `binaries/fyagent-user-helper` external binary. The template's reviewed
  external-binary loops copy it beside the main executable and delete that
  exact installed binary during uninstall. The formal main executable retains
  `requireAdministrator`; test/development and helper manifests retain
  `asInvoker` with `uiAccess=false`.

### Install-directory selection and legacy runtime cleanup

- The GUI uses the standard NSIS directory page. Silent installation forwards
  its final `/D=` value through the normal NSIS path without a repository-owned
  leave callback or pre-write path gate.
- FyAgent does not impose a custom absolute-path, drive-type, local/fixed-volume,
  UNC/network, removable-media, reparse-point, existing-ancestor, ACL, owner,
  protected-folder, or write-right admission policy on the selected install
  directory. NSIS and Windows retain their own parsing and filesystem behavior;
  an actual create/copy/registry/shortcut failure remains an installation
  failure rather than a FyAgent pre-validation error.
- Maintenance/reinstall uses the path recorded by the existing NSIS
  installation without reintroducing the retired custom path policy.
- Before an install, update, maintenance uninstall, or ordinary uninstall can
  run a hook, migrate an old package, clean a directory, or change a payload,
  the local NSIS gate checks both `${MAINBINARYNAME}.exe` and the fixed
  `fyagent-user-helper.exe`. It never calls Tauri's force-terminating
  `CheckIfAppIsRunning` macro, `KillProcess`, `TerminateProcess`, or `taskkill`.
  Interactive callers must close the process normally and may Retry or Cancel;
  passive and silent callers abort immediately while either process is alive.
  This is a fail-closed check at the time of observation, not an atomic promise
  that a user cannot launch an old executable afterward. The residual
  check-to-launch race is a maintenance availability concern: Windows must
  preserve a subsequently running image/pinned handle and setup may fail for a
  later retry, but setup never terminates that process or treats its pin as
  released. Closing this race would require a cross-version launch interlock or
  system policy outside the NSIS-only contract.
- The only retired package migration is the public v0.3.0 WiX MSI. The
  architecture selects one frozen ProductCode: x64
  `{D50D8CE2-B49A-41DE-839D-6574FB69ADC1}` or ARM64
  `{78F69296-A73D-40CA-A2BA-11D117AA2C9B}`. The installer queries that exact
  product with `MsiQueryProductStateW` loaded from the explicit
  `$SYSDIR\msi.dll`; the elevated setup never relies on a bare DLL search. It
  never enumerates products, reads an `UninstallString`, or executes a
  registry-supplied command. A registered product is synchronously removed with the fixed
  `msiexec /x <ProductCode> /qn /norestart` command before any new payload
  mutation. Only exit codes 0, 1605, and 1614 are accepted. Code 3010 requires
  a restart and aborts, and launch failure or any other code aborts without
  installing the new payload.
- `.onInit` captures only the legacy named `InstallDir` value from
  `HKLM\Software\fyagent\FyAgent`. An explicit final `/D=` remains
  authoritative; otherwise the current NSIS registration wins, and the legacy
  marker is only a fallback. After the fixed product is absent or its
  synchronous removal is accepted, that named legacy marker is removed and the
  NSIS install writes its normal default install-location value. The process
  gates precede this migration in `EarlyChecks`, and migration precedes the
  WebView2 section, `SetOutPath`, hooks, cleanup, or payload writes. The Install
  section repeats both process checks immediately before its own mutations.
- The installer never creates, repairs, admits, or requires
  `%ProgramData%\FyAgent\runtime`. On install and uninstall it may only attempt
  to retire `business-*.state` and `business-*.lock` from that fixed legacy
  directory, then remove the two known directories if empty.
- The protected Codex bridge at
  `$COMMONPROGRAMDATA\FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}\v1`
  is strictly distinct from that legacy runtime and from
  `$INSTDIR\cache\codex-installer` staging. NSIS never enumerates, opens,
  repairs, changes ACLs on, or removes the bridge or its orphans. Normal bridge
  settlement and next-elevated-creation known-only orphan cleanup remain owned
  by the application bridge module; an orphan may intentionally outlive
  uninstall.
- Legacy cleanup opens the fixed `$COMMONPROGRAMDATA\FyAgent` parent once as a
  no-follow anchor, then opens `runtime` relative to that parent handle. Its
  single-level enumeration supplies candidate names only: a candidate must be a
  complete `business-*.state` or `business-*.lock` direct-child name with the
  fixed prefix and suffix in lowercase,
  and the file is reopened relative to the held runtime handle with
  `OBJ_DONT_REPARSE` and `FILE_OPEN_REPARSE_POINT`. Only a regular non-reparse
  file is marked for deletion through that same leaf handle. Empty `runtime`
  and parent directories are likewise marked through their already-held
  handles before close; no wildcard/path `Delete` or path `RMDir` is allowed. A
  missing, inaccessible, malformed, reparse, nonempty, or concurrently changing
  object is preserved. Cleanup never creates a directory, changes a descriptor,
  recurses, broadens the filename families, sets an installer error, or aborts.
- The fixed legacy path uses the context-independent
  `$COMMONPROGRAMDATA\FyAgent` alias. Unknown-variable warning 6000 remains a
  packaging error. Across the repo-owned executable NSIS closure, the template
  carries the only pragma, `!pragma warning error 6000`, as a top-level
  directive before the unique top-level installer-hook include; no literal,
  dynamically expanded, conditional-decoy, per-code/all override, or warning
  stack operation may weaken it. Repo-owned executable sources reject dynamic
  preprocessor directive names and allow line-start `${NAME}` only for the
  reviewed NSIS/LogicLib macro inventory.
- Installer and uninstaller registry access uses the same 64-bit machine view
  on supported x64 and ARM64 systems. Shortcuts and protocol/uninstall records
  are machine-scoped.

### Install-root staging cleanup

- Install/upgrade and uninstall may clean only the fixed
  `$INSTDIR\cache\codex-installer` staging root. Cleanup opens `cache` once as
  the fixed no-follow anchor, then opens `codex-installer` relative to the held
  cache handle and each admitted job directory relative to the held staging
  handle. Every directory capability is proven to be a directory without the
  reparse attribute before use.
- The root enumeration is one level only. A job directory is admitted only
  when its complete direct-child name is the 36-character lowercase
  hyphenated UUID form. Uppercase, braced, malformed, nested, non-directory,
  or reparse entries are unknown and remain untouched.
- The full-path `FindFirst`/`FindNext` walk only supplies candidate names; no
  object reached through that enumeration path is deleted. Inside an admitted
  job directory, cleanup considers only exact `installer.msix` and
  `installer.msix.part` names. `NtCreateFile` resolves each leaf name relative
  to the already-open admitted job-directory handle through
  `OBJECT_ATTRIBUTES.RootDirectory`; after the cache anchor is held, no
  descendant capability reparses the mutable full staging path. The relative
  open uses `OBJ_DONT_REPARSE`,
  `FILE_OPEN_REPARSE_POINT`, `FILE_NON_DIRECTORY_FILE`, and
  delete/read-attributes access. The returned leaf handle is then proven to be
  a regular non-directory, non-reparse object, and deletion is marked through
  `FileDispositionInfo` on that same handle before close. A full-path
  `CreateFileW`, path-based `Delete`/`DeleteFileW`, validate-then-delete lookup,
  wildcard, or recursive removal is forbidden. The job directory, fixed
  staging root, and fixed `cache` ancestor are marked for deletion only through
  their held handles and only succeed if each is empty.
- The NSIS control executable and bundled System plug-in are PE32 i386 even
  when the installed payload targets x64 or ARM64; ARM64 executes that 32-bit
  control process through Windows compatibility. Therefore the System
  plug-in's lowercase packed layouts are pinned to pointer size 4,
  `UNICODE_STRING` size 8 with `Buffer` at offset 4,
  `OBJECT_ATTRIBUTES` size 24 with `RootDirectory` at offset 4, and
  `IO_STATUS_BLOCK` size 8. The macro measures each allocated structure and
  verifies both pointer-field offsets before calling `NtCreateFile`; ABI drift
  preserves the directory or leaf instead of issuing a native call with a
  malformed layout.
- Missing/inaccessible entries, enumeration races, files held by another
  process, leaf directories/reparse points, unknown files/directories, and any
  failed handle disposition are preserved. All branches converge on
  one final error clear; cleanup contains no early exit and never blocks
  payload installation, later hooks, or uninstall.

### WebView2 bootstrapper

- The repository-owned helper is embedded byte-for-byte through a bounded,
  deterministic encoded command. It invokes the absolute system Windows
  PowerShell with profiles disabled, restricts module lookup to `$PSHOME`, and
  uses module-qualified commands.
- Repository PowerShell and NSIS text inputs are LF-pinned. The embedded helper
  is level-9 gzip over its exact UTF-16LE source with the RFC 1952 OS header
  normalized to `255` (unknown); verification retains the full byte comparison
  and a separate decode-to-source comparison on every host.
- HTTPS redirects, elapsed time, and response size are bounded. Download bytes
  are created in one GUID-named, descriptor-protected ephemeral directory
  directly below Windows CommonApplicationData, independent of the retired
  `ProgramData\FyAgent` parent, and remain pinned against replacement from
  signature verification through process exit.
- Execution requires Authenticode `Valid`, Microsoft Corporation subject
  identity, Code Signing EKU, a LocalMachine whole-chain build with online
  revocation, the reviewed leaf allowlist, and the reviewed Microsoft Code
  Signing PCA public-key pin. CurrentUser trust injection cannot satisfy the
  policy. Pin rotation requires a reviewed source change and native live-link
  verification on both architectures.

### Uninstall ownership and user data

- Uninstall removes only installer-owned payload/external binaries/resources
  (including the fixed user helper), matching shortcuts,
  product/protocol/uninstall registration, the uninstaller, the known-only
  staging cleanup, and the bounded known-name legacy cleanup described above.
- `$INSTDIR` is never removed recursively. Known children are removed first;
  ancestors are removed only when empty. An unrelated file beside the
  installation survives.
- User-owned data survives uninstall, including `~/.fyagent` databases,
  settings, configuration, skills, OAuth state, logs, backups and storage;
  Tauri per-user roaming/local stores for `com.fyagent.desktop`; and Codex,
  Claude, Gemini, WorkBuddy, Bun, mise, or other external-tool homes.
- The installer exposes no user-data deletion checkbox. Manual lifecycle
  diagnostics create unique sentinels in independent test homes and can prove
  preservation for both default and custom-directory uninstall.

### Signing and sealed asset evidence

- Signer configuration is all-or-none. Absent selector/provider inputs, or
  explicit `unsigned` with absent provider inputs, selects unsigned mode.
  `provider` requires every public policy input and the adapter; an optional
  opaque credential is inherited only by that provider. Partial, blank,
  malformed, relative, failed, or mismatched configuration never falls back to
  unsigned.
- Native build runners receive no signer configuration or credential. Each raw
  setup must be `NotSigned`, have null publisher/signer/timestamp evidence, and
  have an empty PE security directory before it leaves the build runner.
- The provider-neutral transform receives only the absolute candidate path and
  architecture. Repository code never reads or serializes provider-specific
  secret material. Workflow-private secret identifiers and provider syntax are
  not long-term interfaces in this spec. Temporary adapter bytes use a
  create-new random path and are deleted after the attempt.
- A fresh matching-architecture sealer reopens the pinned raw and untrusted
  candidate bytes. Unsigned mode requires byte identity and strict `NotSigned`.
  Provider mode permits only PE checksum/security-directory changes, alignment
  padding, and an appended `WIN_CERTIFICATE`; it requires system status `Valid`,
  exact expected publisher/certificate SHA-256, Code Signing EKU, a timestamp
  certificate, and Time Stamping EKU.
- Link/real-path drift, PE Machine drift, an out-of-policy byte mutation,
  provider-produced evidence, or a verification-time replacement is rejected.
  The setup launcher's Machine does not establish product architecture; the
  installed `fyagent.exe` does.
- x64 and ARM64 fragments must agree on mode and, in provider mode, public
  publisher/certificate policy. Aggregation reopens final bytes and binds name,
  size, SHA-256, source SHA, signature evidence, and attestation subject into
  `signing-status.json`. Release disclosure is generated from this record.

### Manual lifecycle diagnostic

`verify-windows-nsis-lifecycle.ps1` remains available for an operator who wants
to run an install/verify/uninstall diagnostic against one operator-prevalidated
architecture-specific setup on a matching native Windows machine. The caller
must establish the setup's provenance and byte identity before invoking the
script; the diagnostic does not do that. It is not invoked by
`.github/workflows/release.yml`, does not gate `verify-assets`, attestation, or
publication, and does not define Release acceptance.

When invoked manually, the diagnostic proves:

- runner OS/process architecture match the logical target;
- default silent installation succeeds under Program Files;
- a custom path with spaces and Unicode is passed through `/S` with final
  `/D=` and succeeds when NSIS/Windows can write it;
- installed `fyagent.exe` is `0x8664` for x64 or `0xAA64` for ARM64;
- version, HKLM registration, protocol registration, all-users shortcuts, and
  bounded legacy cleanup match their contracts;
- known staging leaves are removed while unknown content and directory/leaf
  reparse objects survive; defensive replacement fixtures prove that cache,
  staging, job, and leaf access remains bound to the already-held capability
  chain and that unrelated sentinels survive;
- on both native x64 and ARM64 runners, the NSIS setup/control process is
  observed as PE32 i386 with a 4-byte System pointer ABI, ordinary exact leaves
  are actually deleted through a successful `NtCreateFile`, and the structure
  size/offset guards above remain satisfied;
- silent uninstall removes bounded installer-owned state while preserving every
  user-data sentinel.

The script retains bounded process waits so a manual diagnostic cannot hang
indefinitely. Each installer waits on its direct `Process` for at most 10
minutes. An ordinary uninstall first copies the installed uninstaller into a
GUID-named directory
under that case's test root, then starts the copy with exact raw
`/S _?=<install-directory>` arguments. The final, unquoted `_?=` disables the
NSIS self-copy handoff, so the bounded direct `Process` is the real uninstall
execution and supplies its real exit code before state assertions run. The
copied executable is deleted exactly and its now-empty directory is removed
without recursive cleanup. Best-effort uninstall cleanup uses the same path
with a 2-minute limit. The lifecycle-only Windows PowerShell signature verifier
and native `signtool` fixture wait at most 3 and 2 minutes respectively, drain
redirected stdout and stderr asynchronously with a bounded completion wait,
retain any completed stream in failure diagnostics, and dispose every owned
process handle. A timed-out helper issues `Kill(true)` only for the process tree
rooted at that case's PID, waits a further bounded interval for the direct root
process to exit, makes no stronger claim about descendant exit, and then fails.
Every launched case emits UTC start/end markers with its PID, elapsed
milliseconds, exit code (or an explicit unavailable marker), and outcome so a
remote failure can be localized without weakening any later state assertion.

Diagnostic cleanup is best effort only for installations created by that test.
It does not use Store access, a real Codex installation, or application UI.
Preview ARM64 runner unavailability blocks matching native ARM64 build/package
evidence; it does not authorize a cross-build or a reduced asset set. It does
not make the optional manual lifecycle diagnostic a Release gate.

## 4. Validation & Error Matrix

| Condition                                                                                                                  | Required result                                                                                                  |
| -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Release-profile build has no explicit manifest mode                                                                        | Fail before compiling/bundling; do not guess.                                                                    |
| GUI or `/D=` selects a relative, UNC/network, removable, reparse, non-existing, or otherwise unusual path                  | Apply no FyAgent path-admission rule; let standard NSIS/Windows handling and actual writes determine the result. |
| Install directory is user-writable or has an unusual owner/ACL                                                             | Do not add an ACL/owner warning, repair, or rejection.                                                           |
| Legacy ProgramData parent/leaf is absent, unsafe, inaccessible, nonempty, or cannot be cleaned                             | Preserve it and continue; never repair, recurse, or make cleanup an admission gate.                              |
| A fixed PackageBridge root/version or operation orphan exists                                                              | Leave it untouched; NSIS owns neither normal settlement nor orphan cleanup.                                      |
| Staging root/child is a reparse point, child name is not a canonical lowercase UUID, or unknown content remains            | Preserve it and continue; never traverse it, broaden deletion, or make cleanup fatal.                            |
| A canonical staging child contains an exact `installer.msix` or `.part` plus unknown content                               | Attempt only the two exact files; preserve the unknown content and therefore the nonempty directory/ancestors.   |
| An exact staging leaf is a directory/reparse point, changes before open, cannot be opened for delete, or disposition fails | Preserve it and continue; never delete it by a second path lookup or follow the reparse target.                  |
| A held staging ancestor or admitted job name is concurrently replaced before a descendant open                             | Continue from the held parent capability; never resolve a deletable descendant through the changed full path.    |
| Main executable or fixed user helper is running during install/update/uninstall                                            | Interactive: require normal exit and Retry or Abort; passive/silent: Abort before migration or any mutation.     |
| Main executable or fixed user helper starts after the final process check                                                  | Never terminate it; preserve the live image/pin and allow a conflicting mutation to fail for a later retry.      |
| Frozen v0.3.0 MSI is absent                                                                                                | Do not start Windows Installer; continue after removing any stale named legacy path marker.                      |
| Frozen v0.3.0 MSI removal returns 0, 1605, or 1614                                                                         | Accept the synchronized result, remove the named legacy marker, and continue with NSIS installation.             |
| Frozen v0.3.0 MSI removal returns 3010, another code, or cannot launch                                                     | Abort before new payload mutation; 3010 explicitly requires restart and a later setup retry.                     |
| Raw candidate contains any signature/security-directory evidence                                                           | Reject before signer or preflight sealing.                                                                       |
| Provider configuration is partial, blank-active, malformed, or fails                                                       | Hard fail; never emit unsigned evidence as fallback.                                                             |
| Signature is `HashMismatch`, `UnknownError`, wrong publisher/certificate/EKU/timestamp, or mutates non-Authenticode bytes  | Reject the architecture and block release.                                                                       |
| x64 and ARM64 signing modes or signed identities differ                                                                    | Aggregation fails; emit no public status.                                                                        |
| Uninstall encounters unrelated install/user data                                                                           | Preserve it; remove only known children and empty owned ancestors.                                               |
| Matching native ARM64 runner is unavailable                                                                                | Acceptance remains blocked.                                                                                      |

## 5. Good / Base / Bad Cases

- Good: both GUI selection and `/S ... /D=C:\Program\FilesFyAgent` use standard
  NSIS path handling without a custom FyAgent rejection; any legacy runtime
  preimage is handled only by bounded best-effort cleanup.
- Base: signer configuration is completely absent. Both final setups remain
  byte-identical to raw, strict `NotSigned` evidence is aggregated, and public
  notes explicitly disclose the unsigned state with digests and attestation.
- Good: complete provider configuration transforms only Authenticode-owned PE
  fields; a fresh no-secret runner independently proves publisher, certificate,
  EKUs, timestamp, and final bytes before aggregation.
- Bad: add a custom absolute/fixed/UNC/reparse path classifier, inspect an
  install-directory owner, allow a partial signer to downgrade to unsigned,
  trust the provider's own fragment, or recursively delete `$INSTDIR`/the
  user's profile.

## 6. Tests Required

- `tests/windowsNsisContract.test.ts` and
  `scripts/release/verify-windows-nsis-contract.mjs` pin configuration, template
  provenance, canonical setup/uninstaller icon, absence of a custom
  installation-path gate, fixed helper packaging, absence of machine-runtime
  provisioning, no-follow known-only legacy cleanup, canonical direct-child
  staging cleanup with two exact artifact names, bounded uninstall, explicit
  absence of any PackageBridge enumeration/ACL-repair/removal surface, and
  absence of retired package surfaces. Mutation cases must reject uppercase or
  malformed UUID admission, a moved/nested staging root, wildcard or recursive
  deletion, either leaf falling back to full-path open or path deletion,
  changing/removing any parent-handle `RootDirectory`, missing relative-open
  no-follow/type checks, a different handle for leaf or empty-directory
  disposition, full-path legacy deletion, broadened legacy filename admission,
  fatal/early-exit cleanup, changed `FindFirst`/`FindNext` close targets or
  cleanup fallthrough, helper omission, any force-termination surface, a
  running-process gate moved after migration/hook/cleanup, a bare/non-system
  MSI library lookup, a changed fixed MSI
  ProductCode/command/accepted-code set, a dynamic MSI lookup, accepting 3010,
  loss of `/D=` precedence, and loss of known shortcut/registration cleanup.
- `tests/windowsSetupIcon.test.ts` and
  `scripts/release/verify-windows-setup-icon.mjs` parse canonical ICO and PE
  resources and reject missing, extra, default, unreferenced, metadata-drifted,
  or byte-different setup icon frames. They also require canonically ordered
  named/ID resource entries and enforce whole-file, resource-tree, leaf, name,
  and cumulative-payload budgets without copying every resource payload or
  accepting reused data entries and aliased/overlapping payload ranges. PNG
  chunk names are admitted only after their original bytes are proven to be
  ASCII alphabetic, and cumulative-budget tests use multiple individually
  admissible payloads.
- `tests/windowsSigningAdapter.test.ts` covers the complete signer matrix,
  strict unsigned state, provider simulation, publisher/certificate/timestamp/
  EKU policy, Authenticode-only mutation, launcher-architecture independence,
  path replacement, fragment drift, cross-architecture mismatch, and final
  size/SHA binding.
- `tests/releaseWorkflow.test.ts` and `tests/releaseAssets.test.ts` bind native
  runner selection, secret isolation, one-file handoffs, two setup names,
  build/package admission into exact asset verification, disclosure, subject
  count, and attachment count without becoming the owner of installer
  internals. They also assert that Release defines no lifecycle job and does
  not invoke `verify-windows-nsis-lifecycle.ps1`.
- Matching native x64 and ARM64 workflow jobs must build and package the setup
  executables successfully. Static tests keep the manual lifecycle script
  internally coherent, but neither those tests nor a manual diagnostic are a
  Release gate.

## 7. Wrong vs Correct

Wrong:

```text
reject install path unless it passes a FyAgent absolute/fixed-volume classifier
if signer fails then publish as unsigned
uninstall: recursively delete $INSTDIR and user configuration
```

Correct:

```text
standard NSIS path selection -> actual Windows write result
legacy ProgramData runtime -> no-follow known-name best-effort cleanup only
Executable PackageBridge -> application-owned settlement/orphan cleanup; NSIS untouched
raw strict unsigned -> optional provider transform -> fresh independent seal
delete allowlisted installer-owned children -> remove only empty owned ancestors
```
