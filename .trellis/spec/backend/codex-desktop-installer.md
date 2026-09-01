# One-click Executable Software Installer Contract

## 1. Scope / Trigger

This contract owns every current and future FyAgent one-click install or upgrade
flow for executable software, regardless of product or platform. Codex Desktop
is the first implementation, not a policy exception. QoderWork CN, TRAE Work CN,
WorkBuddy, OpenCode Desktop, and Claude Desktop reuse the same
source/download/job/cancel/temp/post-install orchestration policy through the
Agent install façade; they must not grow a second downloader. OpenCode and
Claude Agent lifecycle surfaces are desktop-only; public Tooling CLI
install/update/copy-command surfaces exist only for Grok Build and are not an
Agent directory one-click path. `/Applications` last write is owned by
[macOS Privileged System-Commit Helper](./macos-system-commit.md) and stays
disabled until that contract's production gate is flipped.
Bounded `Info.plist` reads go through the Codex `plutil -> JSON -> typed fields`
owner (binary and XML). Codex remains the golden MSIX/DMG regression fixture.
Windows EXE/NSIS artifacts in this iteration are not deployed from elevated
FyAgent directly. They reuse the authenticated ordinary-user helper through a
second closed action, while Codex remains the only PackageManager/MSIX action.

FyAgent MUST NOT admit or reject downloaded executable software by comparing
downloaded content or native package contents with mirror/upstream publication
fields or maintained constants. Prohibited admission comparisons include:

- remote or manifest SHA/checksum;
- remote, manifest, or `Content-Length` byte size;
- package/bundle identity, publisher, Team ID, or package-family suffix;
- release/package/bundle version or downgrade ordering;
- package architecture or minimum operating-system fields;
- upstream signature, notarization, or Gatekeeper fields.

Managed-Agent adapters have one narrower product-routing safety gate. After a
fixed first-party DMG is mounted, they require the closed local bundle identity
and reviewed local version source. Before a fixed first-party Windows EXE is
bridged to Alice, they require a regular no-reparse PE, supported architecture,
closed ProductName, `WinVerifyTrust`, exactly one top-level signer, and the
reviewed signer leaf subject for that product. These values are backend policy,
not renderer or remote-manifest input. They prevent a product action from
deploying another vendor's executable; they do not admit a remote checksum,
remote publisher field, manifest identity, version pin, silent switch, or
renderer bypass. Codex keeps its publication-agnostic downloaded-package
behavior.

The native operating-system installer remains authoritative for package format,
signature/trust, dependencies, compatibility, and deployment result. After a
native install, FyAgent still verifies that one operational result exists and
can be represented for subsequent existence/version/runnable checks.

This policy applies only to executable software installers and updaters. It
does not apply to Skills, plugins, MCP packages, configuration packs, or other
extension/configuration data; their independently owned validation rules remain
unchanged.

Existing Grok-only CLI install/update flows remain operational and MUST NOT gain
hash, identity, publication-field, or package-content admission validation as a
side effect of this contract. Non-Grok Tooling installers are retired.

### Preserved security and reliability boundaries

Removing publication/content admission MUST NOT remove:

- fixed product-owned source endpoints and bounded metadata parsing;
- HTTPS/redirect policy, retry, timeout, cancellation, and response/body caps;
- disk error handling and optional conservative preflight when a size hint exists;
- protected job directories, fixed local filenames, exclusive create, flush,
  sync, finalize, reopen, cleanup, and no-follow/reparse protections;
- a locally computed size used for progress, caps, and conservative disk
  preflight;
- Windows PackageBridge hash-while-copy on the same I/O as the copy, when a
  local digest from the download stream is retained for that copy;
- Windows frozen Shell SID, helper image pinning, authenticated
  action-bound pipe/control ordering, PackageBridge ACL/file-ID/no-follow
  protections, native `AddPackageByUriAsync` for Codex, closed
  `ShellExecuteExW` for reviewed Agent EXEs, process-handle observation,
  quarantine, and known-only cleanup;
- macOS controlled read-only mount, exactly one direct top-level `.app`, safe
  target selection, generated same-volume staging/backup paths, atomic replace,
  compensating rollback, and exact expected-replacement cleanup;
- post-install existence, actual version reporting, launch target shape, and
  runnable/executable-path verification;
- privacy-safe structured errors and the process-lifecycle single-flight claim.

No renderer IPC, ordinary CLI, helper CLI, bridge protocol, or future installer
API may accept an arbitrary URL, filesystem path, identity, publisher, hash,
install scope, or validation-bypass switch.

## 2. Signatures

Renderer input remains only:

```ts
type StartInstallRequest = {
  expectedReleaseId: string;
};
```

The release status exposes operational display data and an optional hint:

```ts
type RemoteReleaseStatus = {
  releaseId: string;
  platform: "windows" | "macos";
  architecture: "x86_64" | "aarch64";
  displayVersion: string;
  platformVersion: PlatformVersion;
  downloadSizeHint: number | null;
  checkedAt: string;
};
```

`downloadSizeHint` is never an integrity or equality requirement. It may be
absent. When present it may drive UI progress and conservative disk preflight;
actual write failures remain authoritative.

The job stages are:

```text
checking -> preflight -> downloading -> installing
         -> verifying_installation -> succeeded
```

Terminal alternatives are `failed` and `cancelled`. There is no
`verifying_download` stage or renderer view. `ProgressPhase::Verification` is
reserved for post-install/runtime verification, not remote checksum work.

The release ID binds the platform, architecture, platform version, and fixed
download endpoint selected during the checked release. It MUST NOT include a
remote hash, remote byte size, manifest filename, publisher, or other upstream
publication field.

## 3. Contracts

### Source and metadata

Codex Desktop requests only the fixed AgentsMirror manifest endpoint and the
fixed architecture artifact endpoint selected in Rust. Manifest-provided URLs,
redirect targets, filenames, delta artifacts, and checksum endpoints never
become download capabilities.

The manifest parser keeps only flow-selection fields needed to choose the fixed
endpoint and display the release: schema/platform/architecture presence,
availability/status, version, and optional size hint. Unknown publication
fields are ignored. Hash, package moniker, identity, publisher/team, minimum OS,
signature, architecture details inside a package, and remote URL changes MUST
NOT block resolution.

Manifest body size, schema shape, platform branch, requested architecture
presence, status, and version syntax stay bounded and fail closed. Cache,
single-flight, retry, cancellation, redirect, and timeout behavior stays intact.

### Download and local handoff

The downloader writes only `installer.msix`, `installer.dmg`, or
`agent-installer.exe` beneath the protected current-job directory. A matching
fixed `.part` file is exclusively created, written
with bounded streaming, flushed/synced, atomically finalized without replacement,
and reopened through the held directory capability.

`Content-Length` and metadata size are progress hints. A mismatch with either
does not fail the download. A nonempty absolute artifact cap still applies, and
empty or over-cap bodies fail. HTTP status, redirect, timeout, cancellation,
retry exhaustion, disk, write, flush, finalize, reopen, and cleanup failures keep
their structured errors.

During the current download, FyAgent may compute the actual byte count and a
streaming SHA-256. Those local values may bind `DownloadedArtifact` /
`PreparedInstallPackage` and may be consumed by a same-I/O PackageBridge copy.
They MUST NOT be used as package-hash admission, and installers MUST NOT add a
second full-file SHA-256 pass after download, before pin, before `hdiutil`, or
after a PackageBridge copy. Future one-click installers inherit this rule: do
not re-read the whole artifact to compare SHA-256.

### Windows PackageManager current-user installation (Codex)

The Windows adapter accepts only `PreparedInstallPackage`; there is no package
path parameter. It does not parse the downloaded MSIX manifest or compare Name,
Publisher, Version, ProcessorArchitecture, MinVersion, signature, or remote
metadata before deployment.

Immediately before helper launch it:

1. revalidates the frozen interactive-user context;
2. captures the exact SID/Main `PackageManager` inventory;
3. opens a no-follow file pin through the downloader-owned capability without a
   full-file SHA-256 reread;
4. passes locally computed actual size and the download-stream digest into the
   protected bridge for hash-while-copy only;
5. launches the fixed sibling `fyagent-user-helper.exe` through Explorer.

The parent-created helper pipe DACL remains Alice-only for
`FILE_GENERIC_READ|FILE_WRITE_DATA` (`0x0012008b`). Connecting to a named pipe
requires `FILE_READ_ATTRIBUTES`; `FILE_GENERIC_WRITE` remains withheld because
it aliases `FILE_CREATE_PIPE_INSTANCE`.

The Codex helper command remains exactly:

```text
fyagent-user-helper.exe codex-msix-install --job-id <uuid> --pipe <nonce>
```

It accepts no URL, path, operation ID, identity, publisher, hash, scope, command,
or bypass argument. Pipe ownership/authentication, frozen SID/session binding,
`Hello(action) -> control -> Started -> admission -> progress -> terminal`, fixed
deadlines, native terminal-state observation, cancellation, quarantine, and
clean-close requirements remain unchanged.

Before admission, an invalid frame, timeout, transport failure, or early exit
returns a structured error because PackageManager has not run. After admission,
invalid progress or terminal data, duplicate or extra data, protocol/transport
failure, timeout, early exit, or an unclean close triggers best-effort
cancellation and permanent process-lifetime quarantine. The Job remains
`Installing`, and no terminal result is published to the renderer.

Only an authenticated valid terminal status, its matching valid terminal frame,
and a clean pipe close permit cleanup.

PackageBridge continues to copy from the duplicated pinned source handle into a
fixed ProgramData hierarchy. Protected ACLs, local-fixed-NTFS checks, exact
frozen-SID access, file identity/link/reparse/placeholder checks, hash-while-copy
against the download-stream digest, flush/finalize/reopen, stable ancestor
handles, and known-only cleanup remain required. PackageBridge MUST NOT add a
second full-file SHA pass after copy. ProgramData-parent effective-access
fail-closed applies to a non-administrator Shell token; an Administrators-enabled
Explorer token is not rejected for OS-owned ancestor rights it already holds.

After a successful helper terminal result, the adapter captures the exact
SID/Main inventory again. The current job result is selected as follows:

- exactly one record differs from the pre-install inventory: select it;
- no changed record but exactly one compatible existing Stable record: select
  it for idempotent native deployment;
- otherwise fail with a structured verification/ambiguity error.

The selected dynamic record must have nonempty operational identity and
publisher values, a Windows package version, and exactly one safe application
ID/AUMID. It is not required to match a maintained identity/publisher/version
constant. If the inventory delta is non-unique, FyAgent MUST fail rather than
guess.

Fixed Stable identity remains allowed only for discovery and lifecycle actions
against an already installed known product. Launch/restart still re-enumerates
the same frozen context and proves the selected local record/AUMID did not drift.

New ChatGPT Desktop (Codex-in-ChatGPT) and ChatGPT Classic may coexist. Any
migration set must be a small first-party exact package name/publisher/family/
application ID/AUMID list proven by native HIL of clean install, official Codex
upgrade, and Classic coexistence. Display names, window titles, and process
names are not identity. Until that HIL exists, keep the current exact Codex
owner and fail closed on ambiguity.

### Windows vendor EXE installation (Agent Catalog)

QoderWork CN, TRAE Work CN, and WorkBuddy reuse the same downloader-owned job
directory, retained artifact, pin factory, PackageBridge, fixed helper image,
authenticated named pipe, admission/cancel events, and settlement/quarantine
rules as Codex. The Agent job slot remains separate from the Codex installer
job slot; reuse is through crate-private infrastructure, not through Codex IPC.

The only additional helper command shape is:

```text
fyagent-user-helper.exe agent-exe-install
  --product qoderwork|trae-work|workbuddy
  --job-id <uuid>
  --pipe <nonce>
```

Argument order is exact. The helper accepts no package path, URL, verb,
working directory, arbitrary product, scope, installer arguments, silent
switches, hash, identity, or bypass. Protocol version 3 binds the selected
`UserHelperAction` into `Hello(action)`; the parent rejects a different action
or product before sending bridge control or signaling admission.

Before helper launch, the parent:

1. force-refreshes the product source/release binding;
2. captures the pre-install Agent inventory;
3. downloads into fixed `agent-installer.exe` through the shared streaming
   downloader and retained job capability;
4. reopens the retained file, rejects reparse/identity drift, reads bounded
   Win32 version resources, verifies supported architecture, runs
   `WinVerifyTrust`, requires exactly one signer, resolves that signer through
   `CryptMsgGetAndVerifySigner`, and compares its bounded subject with the
   reviewed product policy;
5. revalidates Alice's frozen context and bridges only the pinned file.

The helper rechecks the bridge/action, then invokes `ShellExecuteExW` with
`SEE_MASK_NOCLOSEPROCESS`, fixed `open`, no arguments, and the bridge-owned
path. Windows owns UAC. `ERROR_CANCELLED` is user cancellation. Missing
`hProcess` is `installer_process_unobservable`; it is never immediate success.
When a handle exists, the helper waits within the bounded operation deadline,
does not kill the vendor installer, reads `GetExitCodeProcess`, and returns one
closed terminal hint. A nonzero exit is a failure hint, not installation
authority.

Agent job stages are:

```text
checking -> downloading -> staging -> launching_installer
         -> awaiting_user -> verifying_installation
         -> succeeded | failed | cancelled | incomplete
```

`launching_installer` is the non-cancellable side-effect boundary.
`awaiting_user` means the vendor UI/UAC owns interaction. `incomplete` is a
terminal, non-green outcome used when the installer may still be running or
the result cannot be observed uniquely. “Cancel” before launch cancels waiting
and download; after launch FyAgent offers no false “cancel installation”.

Every helper outcome is followed by a fresh Agent inventory readback. Fresh
Qoder install requires one new current-user trusted candidate. TRAE/WorkBuddy
vendor-choice install requires exactly one new trusted candidate in any
observed scope. Update requires the selected candidate to remain at the same
canonical path/scope and change authoritative identity/version. An absent,
unchanged, duplicate, scope-drifted, or version-incompatible result cannot
succeed even when the helper reports exit 0. EXE vendor UI is assisted and has
no rollback claim.

The three current products have no reviewed MSIX/PFN/AUMID contract, so Agent
inventory does not query or guess PackageManager identities for them. Codex is
the sole MSIX consumer. Qoder's reviewed artifact is the User x64 installer;
TRAE and WorkBuddy remain vendor-choice/unknown-scope. Windows ARM64 remains
unsupported until a product-owned source and HIL evidence exist.

### macOS current-user installation

The macOS adapter accepts only `PreparedInstallPackage`; it does not compare a
downloaded DMG/app with remote hash/size, bundle ID, Team ID, version,
architecture, minimum OS, codesign, notarization, or Gatekeeper publication
fields.

Installation still:

1. reopens the fixed local DMG through the retained job capability without a
   full-file SHA-256 reread;
2. mounts it read-only through fixed `hdiutil` arguments and a bounded mount plist;
3. requires exactly one direct top-level regular `.app` bundle;
4. reads bounded `Info.plist` operational fields and verifies the executable is
   a safe regular path contained by the bundle;
5. copies with fixed `ditto` arguments into a generated same-volume staging path;
6. re-reads the staged bundle, atomically replaces/moves the target, verifies
   the installed bundle equals the staged local identity/version, and returns
   that actual installed application;
7. rolls back through a generated backup on update failure and deletes only an
   exact expected replacement or generated transaction path;
8. detaches the image on every path and surfaces detach failure after success.

Fixed Stable bundle ID remains allowed only to discover and safely manage an
already installed Stable application. It is not a downloaded-content gate.
System/user Applications targeting, permission fallback, running-app checks,
path containment, atomic replacement, and rollback remain unchanged.

The shared transaction also exposes a crate-private managed-Agent entry point.
It reuses the same controlled mount, direct-child discovery, executable
containment, generated same-volume staging/backup paths, exact-copy checks,
replacement, rollback, cleanup, and bounded detach logic; it does not fork a
second DMG deployer. The product adapter supplies only:

- the closed expected bundle ID;
- `Info.plist` or bounded non-symlink TRAE `product.json.tronBuildVersion` as
  the local comparable version source;
- exact or reviewed WorkBuddy dotted-prefix comparison against the resolved
  release display version;
- an exact existing target path or one backend-projected fresh parent;
- commit and post-install readback callbacks.

For managed-Agent updates, source/staging/installed copies must match exactly,
the selected existing canonical path and basename are retained, and no
permission failure may redirect an update to another Applications scope. A
system `/Applications` target is disabled with `authorization_required` while
`macos_system_commit::production_enabled()` is false. The privileged helper
exists as code and nested packaging, but it is not a production commit owner
until signed/notarized HIL flips that gate. Fresh user-scope install
may target `~/Applications`; it is never an implicit fallback from a selected
system target and must never be labeled as a system Applications install.
See [macOS Privileged System-Commit Helper](./macos-system-commit.md).

macOS Agent DMG download reuses the Codex streaming persist path
(`prepare_transport_download` / `persist_transport_response`, `.part`,
job-local `installer.dmg`). It must not buffer the full artifact as `Vec<u8>`
or write a second complete DMG. Bounded `Info.plist` reads use the Codex
`plutil → JSON → typed fields` owner (binary and XML).

### Lifecycle and post-install verification

Before install, the service force-refreshes metadata and requires the checked
release ID to remain current. This protects UI/job coherence; it does not compare
remote content fields. A remote hash/size/identity/version/minimum-OS/signature
change that leaves the flow descriptor current does not trigger reanchoring or
block installation.

After native install, the service uses the current job's dynamic Windows/macOS
result when available. It verifies a nonempty operational identity, platform
shape, actual installed version representation, existence, and runnable launch
target. It MUST NOT require identity/version/architecture equality with remote
publication fields.

Cancellation remains allowed before the non-cancellable `installing` boundary.
The process-lifecycle claim and restart capability rules remain unchanged.
Managed-Agent jobs expose `staging` before that boundary. Their post-commit
callback re-enumerates the closed product identity and proves exact target
path, scope, product-comparable version, and no newly introduced cross-scope
copy. A failed update verification restores and re-verifies the old bundle;
an unproven restore is recovery-required rather than success.

Equal-or-newer Codex Desktop (local ≥ selected release) is `AlreadyCurrent`:
inventory readback only. Install, update, check, and already-current **must
not** call `platform.launch`. Explicit launch and restart remain on
`CodexDesktopService::launch` / `platform::process_launch`. macOS application
open uses NSWorkspace completion inside that owner, not `/usr/bin/open`.
Managed-Agent desktop launch calls `launch_trusted_macos_application_as_user`
with a backend-validated `.app` path.

## 4. Validation & Error Matrix

Obsolete remote-content errors such as missing checksum, Team-ID mismatch, and
FyAgent Gatekeeper admission are not part of the DTO. `CHECKSUM_MISMATCH` remains
only for Windows PackageBridge hash-while-copy mismatch, not for a second
full-file SHA reread before consume. `PACKAGE_IDENTITY_MISMATCH` remains for
interactive-user/context/file/lifecycle drift, not upstream package admission.
Native deployment may still return signature, dependency, OS compatibility, or
deployment errors through its structured native-result mapping.

Renderer wording must describe the actual remaining condition and must not say
that an upstream hash, identity, Team ID, or package publication field was
required or rejected.

| Condition | Required result |
| --- | --- |
| Remote/manifest hash, size, identity, publisher/team, version, architecture, minimum OS, signature, or Gatekeeper field drifts | Do not reject based on that remote field; fixed local Agent EXE product/signer routing gates still apply. |
| Manifest body/schema/platform branch/status or fixed-endpoint selection is unusable | Fail with a bounded metadata/source error; never accept a remote URL or caller-provided locator. |
| Download is empty, exceeds the absolute cap, is cancelled, or hits HTTP/redirect/timeout/disk/write/finalize/reopen failure | Fail with the existing structured transport/storage error and clean up only known task-owned artifacts. |
| Metadata or `Content-Length` size hint differs from actual bytes | Keep the actual byte count; do not fail content admission or discard a valid progress snapshot. |
| Locally finalized artifact changes size, path, capability, or file identity before platform or bridge handoff | Fail closed with the local identity error; do not invoke the native installer with a different object. Same-size content drift is not a package-hash admission gate. |
| PackageBridge hash-while-copy digest does not match the download-stream digest | Fail closed with `CHECKSUM_MISMATCH`; do not add a second full-file SHA pass after copy. |
| Windows post-install inventory has exactly one changed record | Select that dynamic record after operational shape validation, without comparing it to maintained publication constants. |
| Windows post-install inventory has multiple changed records or no unique usable result | Fail with structured ambiguity/installation verification error; never guess. |
| Agent EXE local ProductName, architecture, WinVerifyTrust, signer count, or signer leaf subject fails | `source_not_verified` / `platform_unsupported`; do not launch the helper. |
| Helper Hello action/product differs from the parent request | Fail before bridge control/admission; zero installer launch. |
| User cancels UAC/vendor launch | `installer_user_cancelled`; inventory reread still determines whether an install actually appeared. |
| ShellExecuteEx returns no process handle or the wait deadline expires | `installer_process_unobservable` / `installer_timed_out`; stop waiting without killing the installer, then reread inventory. |
| Vendor EXE exits zero/nonzero | Treat only as a hint; authoritative Agent inventory readback decides success. |
| macOS mount has no unique direct top-level `.app`, escapes containment, or staged/installed local identity changes | Fail with the corresponding local mount/path/transaction error and run the bounded detach/rollback path. |
| Managed-Agent DMG resolves another product identity or the reviewed local version does not match the selected release | `source_not_verified`; do not move the selected target. |
| Managed-Agent update targets `/Applications` without a reviewed authorization adapter | `authorization_required`; do not fall back to `~/Applications`. |
| Managed-Agent app is running or the selected target drifts before commit | `application_running` / `target_changed`; no replacement write. |
| Managed-Agent post-install path/scope/version/duplicate readback fails | restore and reverify the prior bundle; report `rollback_restored` or `recovery_required`. |
| Native installer rejects signature, dependencies, OS compatibility, or deployment | Surface the native structured result; FyAgent must not reinterpret it as an upstream-field mismatch. |
| Renderer/helper request contains URL, path, hash, identity, scope, or bypass input | Contract/static test fails; no installation side effect is allowed. |

## 5. Good / Base / Bad Cases

- Good: the fixed Windows artifact endpoint returns a package whose publisher,
  identity, version, architecture metadata, hash, or byte length differs from
  the mirror manifest. FyAgent safely downloads it, the native installer accepts
  it, exactly one same-user package record changes, and the actual result is
  used for post-install verification.
- Good: a macOS DMG contains one safe direct top-level app with different
  bundle/team/version/signing publication fields. FyAgent uses the controlled
  read-only mount and local transaction, then reports the actual installed app.
- Base: no executable install is requested. Skills, plugins, MCP, configuration
  packs, generic CLI management, and release/CI asset verification continue
  under their own unchanged contracts.
- Bad: accept a renderer/helper URL, compare the package with a maintained
  publisher/hash/version allowlist, treat a size hint as an equality gate,
  remove Windows ACL/file-ID/SID checks, guess among multiple inventory deltas,
  or delete a macOS target without exact transaction ownership.

## 6. Tests Required

Tests must prove:

- fixed endpoints only; remote URL/filename/checksum endpoints are ignored;
- bounded manifest/body/redirect/retry/timeout/cancellation/cache behavior;
- remote hash, size, identity, publisher/team, version, architecture, minimum OS,
  and signature field drift does not block executable installation by itself;
  local Agent EXE ProductName/architecture/WinVerifyTrust/signer routing remains
  mandatory;
- empty/absolute-over-cap downloads still fail, while metadata/Content-Length
  mismatch does not;
- installers do not full-file SHA-256 reread a downloaded artifact before pin,
  helper, or `hdiutil`; PackageBridge may hash-while-copy and must not add a
  second full-file pass after copy;
- no URL/path/hash/identity/scope/bypass IPC or helper CLI exists; Helper CLI
  admits only exact Codex MSIX or Agent EXE product-enum shapes;
- exact frozen SID/Main inventory is captured before and after Windows install;
  one dynamic delta succeeds and multiple deltas fail;
- helper authentication, action-bound Hello v3, ACL, no-follow, file-ID,
  PackageBridge, terminal, quarantine, and cleanup protections remain covered;
- Agent EXE tests cover signer-leaf resolution, wrong signer/product/arch,
  Qoder current-user versus TRAE/WorkBuddy vendor-choice policy, UAC cancel,
  missing process handle, timeout, nonzero exit, no-kill behavior, and
  post-install unique-candidate readback;
- macOS mount discovery, executable containment, generated path safety, atomic
  replacement, exact expected cleanup, rollback, and detach remain covered;
- managed-Agent exact selected-path update, no scope fallback, bundle identity,
  Qoder exact/WorkBuddy dotted-prefix/TRAE `tronBuildVersion` release checks,
  `staging` cancellation boundary, running-app zero-write, authoritative
  path/scope/version/duplicate readback, and restored-backup reread remain
  covered by the same macOS transaction tests;
- DTO/parser/fixture/UI/i18n contracts omit download verification stage and
  obsolete content-admission errors;
- generic CLI install/update other than Grok Build are retired and contain no
  new validator. Grok Build remains the only writable Tooling lifecycle.
- Agent Catalog desktop adapters reuse this policy without a second downloader
  and without occupying the Codex job slot. Windows EXE uses the closed Agent
  helper action. Formal elevated Claude/OpenCode CLI/auth remains
  `interactive_user_unavailable` / `executor_not_implemented` and is not
  routed through that helper. Formal elevated Grok Build uses the closed
  `grok-tool` helper action on the same executable, without PackageBridge.

Portable tests and Windows-host compilation do not establish real Windows or
macOS native compatibility. Unless native HIL is actually run, report
PackageManager, ACL/effective access, WinVerifyTrust/signer lookup, UAC/Shell-user,
vendor EXE process semantics, `hdiutil`, `ditto`, launch, rollback, Gatekeeper,
and real runnable behavior as unverified residual risk.

This task does not run native HIL locally or in GitHub Actions. Its evidence is
limited to static contract tests, scoped Windows-target compilation checks, and
code/security review. Real Windows 10/11, x64/ARM64, elevated-Bob/standard-Alice
Shell identity, PackageManager, protected file URI, ACL, and cleanup behavior
remain an explicit, unverified residual risk. These gaps prohibit claiming
native compatibility or native runtime verification.

## 7. Wrong vs Correct

Wrong: restore upstream-field admission in a future executable installer or
expose a bypassable/raw locator input.

```rust
if downloaded_sha256 != manifest.sha256 || package.publisher != EXPECTED_PUBLISHER {
    return Err(InstallerErrorCode::PackageIdentityMismatch.into());
}
install_from_renderer_path(request.path)?;
```

Correct: keep the endpoint and locator product-owned, skip package-hash
admission and multi-pass full-file SHA, delegate package acceptance to the native
installer, and fail closed when the actual result cannot be selected uniquely.

```rust
let artifact = download_from_fixed_endpoint(release.endpoint()).await?;
native_install(&artifact)?;
let installed = select_unique_dynamic_install_result(before, after)?;
verify_operational_shape(&installed)?;
```

## Scenario: Agent Catalog managed-desktop reuse

### 1. Scope / Trigger

- Trigger: QoderWork CN, TRAE Work CN, and WorkBuddy now consume this
  contract's source/download/job/cancel/temp/post-install policy through the
  Agent install façade. Codex remains the golden MSIX/DMG fixture. This is
  a cross-product installer boundary, not a second downloader.

### 2. Signatures

Codex renderer input is unchanged:

```ts
type StartInstallRequest = { expectedReleaseId: string };
```

Agent Catalog desktop install does **not** call Codex job commands. It uses
`start_agent_action` from
[External Agent P0 Safety](./external-agent-p0.md). Package format is a
Rust-only field and is never on the Agent DTO.

### 3. Contracts

- Reuse the orchestration policy (fixed product source, HTTPS/redirect,
  cancel, temp ownership, post-install reread). QoderWork CN, TRAE Work CN,
  and WorkBuddy DMGs reuse the same macOS transaction through a narrow product
  policy adapter; do not duplicate mount, staging, replacement, rollback, or
  generated-path cleanup. Their Windows EXEs reuse the same downloader,
  retained artifact, pin, PackageBridge, fixed helper image, pipe/admission,
  settlement, and cleanup owners through a closed Agent product action.
- A managed update binds the inventory-selected existing path and keeps that
  exact path/basename. A fresh install binds one backend-projected destination.
  The transaction never guesses another scope. The reviewed helper is present
  as packaging and a crate-private port, but production system commits stay
  blocked with `authorization_required` until
  [macOS Privileged System-Commit Helper](./macos-system-commit.md) enables
  `production_enabled()`.
- The shared transaction invokes the Agent commit gate after staging validation
  and before the old target moves. It invokes the Agent inventory readback
  after the new target is locally verified and before the backup is deleted.
  Readback failure restores/reverifies the backup or reports recovery required.
- Windows EXE/NSIS for Catalog desktop agents is a closed recognized format.
  It reuses the PackageBridge/helper infrastructure but never the Codex
  PackageManager operation or Codex job slot. The helper product is exactly
  `qoderwork | trae-work | workbuddy`; no generic executable/path runner or
  renderer-provided installer argument exists. Qoder is current-user User-x64;
  TRAE/WorkBuddy remain vendor-choice and may show UAC.
- Codex `install`/`update` stay on this service. The Agent façade must not
  occupy the Codex job slot or start a parallel Codex download.
- TRAE/WorkBuddy still require opaque release-id coherence before creating
  an Agent job. Qoder's versionless `/latest/` alias is the documented
  exception in the Agent contract, not a license to skip Codex
  `expectedReleaseId` checks.
- Publication-field admission remains forbidden for every product that
  inherits this contract.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Agent façade starts Codex install/update | `managed_by_codex_desktop`; Codex job unchanged |
| Catalog desktop EXE uses a reviewed product action | Protected bridge + Alice helper + vendor UI + authoritative inventory reread |
| Catalog desktop EXE lacks matching ProductName/trusted signer/architecture | Zero helper launch; fail closed |
| Renderer supplies a download URL to either installer | Contract/static test fails |
| A second downloader module is added for Qoder/TRAE/WorkBuddy | Architecture regression |
| Managed macOS update writes a different path/scope than the selected candidate | Transaction/readback failure; restore the original target |
| System target is selected while `production_enabled()` is false | `authorization_required`; zero write, no user-scope fallback |

### 5. Good/Base/Bad Cases

- Good: Codex domain tests (109+) stay green while Catalog desktop adapters
  use first-party sources.
- Base: no Catalog desktop action is requested; Codex one-click flow is
  unchanged.
- Bad: pass WorkBuddy's package path to the helper, invoke PackageManager for
  an EXE, omit the product from Hello, or hardcode a researched TRAE version
  URL as a fallback artifact.

### 6. Tests Required

- Existing Codex desktop domain suite remains authoritative and unforked.
- Agent source/job tests live under `agent_install` and must not weaken
  Codex `expectedReleaseId` or helper-CLI contracts.
- Helper tests prove exact CLI/action codes, Hello-action binding, bridge
  artifact kind, process-handle/exit mapping, and no arbitrary arguments.
- Negative scan: no renderer/helper URL/path/hash/bypass input on either
  surface.

### 7. Wrong vs Correct

#### Wrong

```rust
install_msix_via_codex_helper(workbuddy_exe_path)?;
start_codex_desktop_job_from_agent_action()?;
```

#### Correct

```rust
if agent_id == AgentCatalogId::Codex && matches!(action, Install | Update) {
    return Err(AgentReasonCode::ManagedByCodexDesktop);
}
let product = AgentInstallerProduct::try_from(agent_id)?;
run_verified_agent_exe_installer(product, prepared_package, progress)?;
verify_agent_inventory_readback(agent_id, selected_target)?;
```
