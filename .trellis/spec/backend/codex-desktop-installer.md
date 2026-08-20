# One-click Executable Software Installer Contract

## 1. Scope / Trigger

This contract owns every current and future FyAgent one-click install or upgrade
flow for executable software, regardless of product or platform. Codex Desktop
is the first implementation, not a policy exception.

FyAgent MUST NOT admit or reject downloaded executable software by comparing
downloaded content or native package contents with mirror/upstream publication
fields or maintained constants. Prohibited admission comparisons include:

- remote or manifest SHA/checksum;
- remote, manifest, or `Content-Length` byte size;
- package/bundle identity, publisher, Team ID, or package-family suffix;
- release/package/bundle version or downgrade ordering;
- package architecture or minimum operating-system fields;
- upstream signature, notarization, or Gatekeeper fields.

The native operating-system installer remains authoritative for package format,
signature/trust, dependencies, compatibility, and deployment result. After a
native install, FyAgent still verifies that one operational result exists and
can be represented for subsequent existence/version/runnable checks.

This policy applies only to executable software installers and updaters. It
does not apply to Skills, plugins, MCP packages, configuration packs, or other
extension/configuration data; their independently owned validation rules remain
unchanged.

Existing generic CLI install/update flows remain operational and MUST NOT gain
hash, identity, publication-field, or package-content admission validation as a
side effect of this contract.

### Preserved security and reliability boundaries

Removing publication/content admission MUST NOT remove:

- fixed product-owned source endpoints and bounded metadata parsing;
- HTTPS/redirect policy, retry, timeout, cancellation, and response/body caps;
- disk error handling and optional conservative preflight when a size hint exists;
- protected job directories, fixed local filenames, exclusive create, flush,
  sync, finalize, reopen, cleanup, and no-follow/reparse protections;
- a locally computed size/hash used only to prove that the current job hands the
  same local file across process/privilege boundaries;
- Windows frozen Shell SID, exact-SID/Main inventory, helper image pinning,
  authenticated pipe/control ordering, PackageBridge ACL/file-ID/no-follow
  protections, native `AddPackageByUriAsync`, quarantine, and known-only cleanup;
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

The downloader writes only `installer.msix` or `installer.dmg` beneath the
protected current-job directory. A `.part` file is exclusively created, written
with bounded streaming, flushed/synced, atomically finalized without replacement,
and reopened through the held directory capability.

`Content-Length` and metadata size are progress hints. A mismatch with either
does not fail the download. A nonempty absolute artifact cap still applies, and
empty or over-cap bodies fail. HTTP status, redirect, timeout, cancellation,
retry exhaustion, disk, write, flush, finalize, reopen, and cleanup failures keep
their structured errors.

During the current download, FyAgent computes the actual byte count and SHA-256.
Those local values bind `DownloadedArtifact`/`PreparedInstallPackage` and may be
rechecked before platform consumption and Windows bridge transfer. This check
detects mutation or replacement of the same local file; it is not an assertion
about what upstream intended to publish.

### Windows current-user installation

The Windows adapter accepts only `PreparedInstallPackage`; there is no package
path parameter. It does not parse the downloaded MSIX manifest or compare Name,
Publisher, Version, ProcessorArchitecture, MinVersion, signature, or remote
metadata before deployment.

Immediately before helper launch it:

1. revalidates the frozen interactive-user context;
2. captures the exact SID/Main `PackageManager` inventory;
3. revalidates the downloader-owned artifact and opens a no-follow file pin;
4. passes only locally computed actual size/hash into the protected bridge;
5. launches the fixed sibling `fyagent-user-helper.exe` through Explorer.

The parent-created helper pipe DACL remains Alice-only for
`FILE_GENERIC_READ|FILE_WRITE_DATA` (`0x0012008b`). Connecting to a named pipe
requires `FILE_READ_ATTRIBUTES`; `FILE_GENERIC_WRITE` remains withheld because
it aliases `FILE_CREATE_PIPE_INSTANCE`.

The helper command remains exactly:

```text
fyagent-user-helper.exe codex-msix-install --job-id <uuid> --pipe <nonce>
```

It accepts no URL, path, operation ID, identity, publisher, hash, scope, command,
or bypass argument. Pipe ownership/authentication, frozen SID/session binding,
`Hello -> control -> Started -> admission -> progress -> terminal`, fixed
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
frozen-SID access, file identity/link/reparse/placeholder checks, same-file
actual size/hash checks, flush/finalize/reopen, stable ancestor handles, and
known-only cleanup remain required. ProgramData-parent effective-access
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

### macOS current-user installation

The macOS adapter accepts only `PreparedInstallPackage`; it does not compare a
downloaded DMG/app with remote hash/size, bundle ID, Team ID, version,
architecture, minimum OS, codesign, notarization, or Gatekeeper publication
fields.

Installation still:

1. revalidates the fixed local DMG;
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

## 4. Validation & Error Matrix

Obsolete remote-content errors such as missing checksum, Team-ID mismatch, and
FyAgent Gatekeeper admission are not part of the DTO. `CHECKSUM_MISMATCH` remains
only for local same-file handoff mutation. `PACKAGE_IDENTITY_MISMATCH` remains for
interactive-user/context/file/lifecycle drift, not upstream package admission.
Native deployment may still return signature, dependency, OS compatibility, or
deployment errors through its structured native-result mapping.

Renderer wording must describe the actual remaining condition and must not say
that an upstream hash, identity, Team ID, or package publication field was
required or rejected.

| Condition | Required result |
| --- | --- |
| Remote/manifest hash, size, identity, publisher/team, version, architecture, minimum OS, signature, or Gatekeeper field drifts | Do not reject the executable; continue through the fixed endpoint and native installer flow. |
| Manifest body/schema/platform branch/status or fixed-endpoint selection is unusable | Fail with a bounded metadata/source error; never accept a remote URL or caller-provided locator. |
| Download is empty, exceeds the absolute cap, is cancelled, or hits HTTP/redirect/timeout/disk/write/finalize/reopen failure | Fail with the existing structured transport/storage error and clean up only known task-owned artifacts. |
| Metadata or `Content-Length` size hint differs from actual bytes | Keep the actual byte count; do not fail content admission or discard a valid progress snapshot. |
| Locally finalized artifact changes size/hash/identity before platform or bridge handoff | Fail closed with the local same-file mutation error; do not invoke the native installer with a different object. |
| Windows post-install inventory has exactly one changed record | Select that dynamic record after operational shape validation, without comparing it to maintained publication constants. |
| Windows post-install inventory has multiple changed records or no unique usable result | Fail with structured ambiguity/installation verification error; never guess. |
| macOS mount has no unique direct top-level `.app`, escapes containment, or staged/installed local identity changes | Fail with the corresponding local mount/path/transaction error and run the bounded detach/rollback path. |
| Native installer rejects signature, dependencies, OS compatibility, or deployment | Surface the native structured result; FyAgent must not reinterpret it as an upstream-field mismatch. |
| Renderer/helper request contains URL, path, hash, identity, scope, or bypass input | Contract/static test fails; no installation side effect is allowed. |

## 5. Good / Base / Bad Cases

- Good: the fixed Windows artifact endpoint returns a package whose publisher,
  identity, version, architecture metadata, hash, or byte length differs from
  the mirror manifest. FyAgent safely downloads and locally fingerprints the
  current file, the native installer accepts it, exactly one same-user package
  record changes, and the actual result is used for post-install verification.
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
  and signature field drift does not block executable installation;
- empty/absolute-over-cap downloads still fail, while metadata/Content-Length
  mismatch does not;
- local post-download mutation still fails same-file handoff;
- no URL/path/hash/identity/scope/bypass IPC or helper CLI exists;
- exact frozen SID/Main inventory is captured before and after Windows install;
  one dynamic delta succeeds and multiple deltas fail;
- helper authentication, ACL, no-follow, file-ID, PackageBridge, terminal,
  quarantine, and cleanup protections remain covered;
- macOS mount discovery, executable containment, generated path safety, atomic
  replacement, exact expected cleanup, rollback, and detach remain covered;
- DTO/parser/fixture/UI/i18n contracts omit download verification stage and
  obsolete content-admission errors;
- generic CLI install/update flows are unchanged and contain no new validator.

Portable tests and Windows-host compilation do not establish real Windows or
macOS native compatibility. Unless native HIL is actually run, report
PackageManager, ACL/effective access, UAC/Shell-user, `hdiutil`, `ditto`, launch,
rollback, Gatekeeper, and real runnable behavior as unverified residual risk.

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

Correct: keep the endpoint and locator product-owned, use the local fingerprint
only to prove same-file handoff, delegate package acceptance to the native
installer, and fail closed when the actual result cannot be selected uniquely.

```rust
let artifact = download_from_fixed_endpoint(release.endpoint()).await?;
artifact.revalidate_local_fingerprint()?;
native_install(&artifact)?;
let installed = select_unique_dynamic_install_result(before, after)?;
verify_operational_shape(&installed)?;
```
