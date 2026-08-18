# Current state and safety boundaries

## Purpose

This research captures the repository state that the replacement design must
either preserve or intentionally supersede. It is task-local evidence, not a
second product specification.

## Baseline identity

- Task base: `b6f60dfe0b4e815fdb9eb3ba446c827dc41e0527` on
  `dev/laiyongjie`.
- At task creation, local HEAD, the local upstream ref, and live remote branch
  all matched that SHA.
- The worktree held only upstream-managed changes to `.codex/hooks.json` and
  the two Python hook files, plus a missing final newline in
  `.trellis/.template-hashes.json`. The newline was restored without entering
  a commit.
- CI, Release, Label PRs, and Dependency Graph workflows were active. The
  authenticated account had `repo` and `workflow` scopes.
- No active Trellis task existed before this task. Archived 08-09 work remains
  historical evidence and must not be modified.

## Intentional contract reversals

The current checkout enforces two choices that this approved task replaces:

1. FyAgent-specific Trellis overlay/wrapper/managed-hook enforcement.
2. A protected ProgramData machine runtime plus process-SID-equals-Shell-SID
   startup admission and authenticated custom activation forwarding.

The release contract's exclusion of executable installer lifecycle smoke is
retained. The lifecycle harness remains an optional manual diagnostic and is not
scheduled by CI, Release, or this delivery.

The replacement retains the existing NSIS perMachine/directory UX, dual
manifest selection, exact-SHA release eligibility, native target set,
signer/fresh-sealer split, attested subject set, attachment set, and formal
single-publication transaction.

## Hook update evidence and accepted risk

The three dirty hook files match the reviewed upstream Trellis `0.6.14` bases
that the former overlay manifest expected as inputs. They remove project-only:

- repository/task realpath containment for context files;
- exact-path binding for dynamically imported Trellis modules;
- strict Codex event/session/input failure behavior;
- markup/control-character escaping in workflow breadcrumbs;
- the ambient-system-Python avoidance supplied by the project hook runner.

The task intentionally accepts those upstream bytes and deletes the project
overlay/runner instead of reconciling back to hardened output. This is a known
security regression in prompt-assistance hooks, not an equivalent migration.
Product runtime authority must not depend on these hooks.

## Windows runtime observations

- `src-tauri/src/main.rs` currently invokes the all-users headless entry and
  the pre-Tauri Windows startup gate.
- Windows single-instance registration is currently excluded while macOS and
  Linux use `tauri-plugin-single-instance` with existing deep-link,
  lightweight-window, and focus handling.
- `src-tauri/src/windows_runtime/` owns ProgramData runtime state/lease,
  equal-user proof, HMAC/capability activation, and the custom pipe.
- The current immutable interactive context contains only process session,
  Shell session, and canonical SID, and equality is an admission condition.
- User directory access is distributed across panic, configuration, database,
  logs, provider/runtime, tray, and installer code; replacement requires a
  semantic consumer audit, not only changing one path helper.

The locked Tauri single-instance implementation on Windows uses a predictable
named mutex, hidden window, and `WM_COPYDATA` without an application-layer
peer capability. Therefore callback argv are local untrusted input. Existing
deep-link envelope limits and validation should be reused; plugin input cannot
directly invoke a privileged side effect.

The same dependency decodes and forwards the raw process argv before the
application callback can apply its limits. A same-Shell-user process can also
pre-claim the predictable endpoint. In the elevated Bob/Shell Alice case this
means a provider deep link that embeds an API key has no transport-level peer
confidentiality from Alice-local processes. The approved replacement does not
restore the retired capability/HMAC transport or patch the pinned plugin, so
this remains an explicit dependency residual: never log raw argv, bound and
parse it immediately after callback entry, queue only validated semantic
requests, and never connect that callback to privileged work.

Alice-owned profile/configuration directories are deliberately treated as the
selected user's mutable data, not as trusted elevated input. Because the main
process remains elevated and the approved design keeps existing custom data
directory semantics, a hostile Alice can redirect/reparse paths that Bob later
opens. The task adds absolute-directory validation for the application-data
override and no ambient fallback, but does not claim that this is an
impersonating filesystem broker or a full lower-integrity path-containment
boundary. The install-candidate handoff is different: it crosses directly into
PackageManager admission and therefore gets the explicit no-write/no-delete
file pin described below.

Switching ambient `HKCU` access to explicit `HKEY_USERS\<Alice SID>` adds a
separate object-redirection risk: registry symbolic links are followed by the
default `RegOpenKeyEx` behavior. The runtime therefore exposes only two fixed
registry locations, traverses every component relative to a verified parent
with `REG_OPTION_OPEN_LINK`, rejects link markers, and never mutates the handle
returned when create-or-open reports an existing key until it has been reopened
no-follow. This is required even though Alice legitimately owns the ordinary
values, because the elevated Bob token must not be redirected to another
registry object.

Alice can also replace optional `app_paths.json` or `.window-state.json` with a
very large file. These files are not authority inputs, but an unbounded
`fs::read` in Bob would create an avoidable startup denial of service. The
application streams them only through small fixed limits and treats overflow as
invalid optional state; writes remain atomic at the frozen Alice path.

## Codex installer observations

- The current experimental all-users path uses headless/runas job-control
  files, `StagePackageByUriAsync`, and
  `ProvisionPackageForAllUsersAsync` outside the ordinary renderer commands.
- The current-user PackageManager path still executes inside the elevated main
  process, so it targets the wrong user in a Bob-admin/Alice-Shell scenario.
- Windows staging currently derives from the process temporary directory,
  which does not bind it to the selected install root or its volume.
- Existing package validation already covers descriptor, size, checksum,
  bounded ZIP/manifest, publisher/identity/architecture/version, and
  post-verification continuity checks. The replacement should reuse those
  validators and change the final consumption boundary.
- The existing Explorer COM launcher is the correct privilege boundary to
  reuse for a fixed helper executable.

## Verified-file handoff

The selected design relies on documented Windows sharing and identity facts:

- A `CreateFileW` handle opened with only `FILE_SHARE_READ` permits concurrent
  reads but prevents later write and delete sharing until the handle closes.
  Windows rename/delete operations require compatible delete sharing.
- `GetFileInformationByHandle` exposes volume serial number, file index, and
  size for identity continuity checks.
- `PackageManager.AddPackageByUriAsync` installs a package for the calling
  user's PackageManager context.

Primary references:

- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle
- https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.addpackagebyuriasync?view=winrt-26100

The main process must hold the share-restricting read handle from the final
identity recheck through the helper's terminal PackageManager result. A fixed
path without this pin is insufficient when the selected install directory is
user-writable.

## 2026-08-11 PackageManager source decisions

### Rejected representations

The first handle-continuity design attempted to turn the pinned file's
Volume-GUID path into a `file://` URI. A bounded native Windows 10 probe found
no representation that satisfied both sides of the contract:

- the encoded exact `\\?\Volume{GUID}\...` forms were rejected by
  `Windows.Foundation.Uri` with `E_INVALIDARG`;
- forms accepted by the URI parser round-tripped with a missing leading
  backslash or treated `?` as the query delimiter;
- Unicode/custom-directory forms retained the same ambiguity, and URI
  percent-decoding is not a reliable UTF-8 identity binding for this API.

An intermediate design served the parent's pinned handle through an exclusive
numeric-loopback HTTP endpoint. Its local parser, handle ownership, and
quarantine model could be bounded, but the Windows 10 public contract has no
equivalent of `ExpectedDigests` and does not promise that direct
`AddPackageByUriAsync` bypasses the interactive user's static proxy or PAC.
Delivery Optimization documentation describes current-user automatic proxy
selection for HTTP payloads. A proxy could therefore become the response
authority without touching the parent's listener, defeating the exact-byte
claim even if one normal-environment probe happened to go direct. The product does
not accept that undocumented platform behavior as a durable security boundary,
so runtime HTTP and every HTTP fallback were rejected.

`AddPackageOptions.ExpectedDigests` would bind fetched contents, but it is not
available on the retained Windows 10 19041 baseline. Raising the minimum
Windows version was explicitly rejected. PackageManager also exposes no public
Windows 10 overload that deploys directly from an existing `HANDLE`, `IStream`,
`StorageFile`, or random-access stream.

Primary references:

- https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.packagemanager.addpackagebyuriasync
- https://learn.microsoft.com/en-us/uwp/api/windows.management.deployment.addpackageoptions.expecteddigests
- https://learn.microsoft.com/en-us/windows/deployment/do/delivery-optimization-proxy
- https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-urlcreatefrompathw
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew

### Approved protected package bridge

The approved design preserves the existing Windows 10 API support floor by
using a one-operation protected copy below `FOLDERID_ProgramData`, independent
of the retired `%ProgramData%\FyAgent\runtime` state/lease/HMAC/control tree and
independent of the user-selected install-root staging ACL:

```text
<ProgramData>/FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}/v1/
  <64-lowercase-hex-operation-id>/installer.msix
```

The elevated parent creates or exactly verifies the fixed product root and `v1`
through held-parent, no-follow operations on a local fixed NTFS volume. Those
two stable objects are BA-owned/grouped with one protected allow-only DACL
independent of any particular user: BA has lifecycle management, SYSTEM has the
necessary read/traverse access, and Authenticated Users (`AU`) has stable
directory `FILE_GENERIC_EXECUTE` semantics—traverse, read attributes,
`READ_CONTROL`, and synchronize—to reach an already-known child, never list,
create, write, delete, or delete-child. The root must not be bound to the first
Alice. Each random operation directory and file is create-new with its own
protected DACL: BA management, minimum SYSTEM read/traverse, and minimum
read/traverse for the exact frozen Alice SID. Alice must have no create, append,
write-attributes, delete, delete-child, hardlink, reparse, write-DACL, or
write-owner route through the object or ProgramData ancestors. Existing fixed
root/version preimages are verified exactly or rejected; operation IDs and
leaves are never reused or repaired in place.

The parent copies only from the already verified source handle into a
create-new `.part`, computes exact size and SHA-256 while streaming, flushes,
handle-renames without replacement, reopens the final leaf with only read
sharing, and revalidates hash, size, volume/file identity, link/reparse state,
owner/group/DACL, and exact continuity with the already validated source. The
parent's pre-deployment validation remains release SHA/size plus bounded ZIP/
manifest publisher/name/version/architecture/OS checks; it does not claim to
replace the native MSIX signature chain, which remains PackageManager's sink
authority. This extra full copy is an accepted disk cost, and free-space
admission queries the actual ProgramData volume rather than guessing `C:`.

The authenticated conversation is `Hello` -> parent peer authentication ->
fixed bridge control -> helper `Started` identity -> parent revalidation and
admission. The control carries only the random operation ID and expected file
identity; no path or URI crosses the CLI or unauthenticated pipe. The helper
resolves the same known-folder and fixed names, reopens every object no-follow,
rechecks owner/group/DACL/identity, and uses `UrlCreateFromPathW` only for the
ordinary DOS path of this protected object. It round-trips the canonical
`file://` URI back with `PathCreateFromUrlW`, rejects UNC/host, query/fragment,
extended-path, length, or encoding ambiguity, and proves the same file identity
before sending `Started`. Only after the parent matches that identity and
signals admission may it call current-user PackageManager. The security boundary
is the immutable protected namespace; the URI is only its consumer-compatible
name, not a substitute for the object and ACL proofs.

Source and bridge ancestor/file handles remain live through authenticated
settlement. The application bridge module performs normal cleanup only
after an authenticated non-`Started` WinRT terminal status, its matching valid
terminal frame, and clean pipe close. A crash or indeterminate deployment leaves
an immutable orphan, which is safer than releasing or reusing a name while
PackageManager may still consume it. The next elevated bridge creation may run
known-only orphan cleanup through held handles and admits only the fixed
hierarchy, canonical operation IDs, exact owner/group/DACL, and exact leaf names;
unknown, reparse, ACL-drifted, inaccessible, nonempty, or changing content
survives. NSIS never owns or cleans this bridge. No operation ID is reused, and
there is no Temp/current-directory/install-root/HTTP fallback.

### Open-source design evidence

The selected shape follows mature components rather than claiming any one
project proves this exact Bob/Alice PackageManager bridge:

- PowerToys opens a user-writable installer with restrictive sharing, performs
  handle-bound Authenticode plus product-identity verification, and keeps that
  handle through the installer process lifetime.
- WinGet downloads under a hash-derived temporary name, recomputes full
  SHA-256 before rename, and uses content digests for remote PackageManager
  sources only where the OS API supports them.
- VS Code disables the user updater while running as administrator, and
  Squirrel re-enters through Explorer as the ordinary user; both avoid a
  cross-user mutable pathname bridge.
- Mozilla and Chromium use scope-aligned trusted updater/service designs with
  signed product-specific payloads and protected runners rather than trusting
  a low-integrity pathname.

These precedents justify restrictive handles, independent product identity,
scope alignment, protected machine namespaces, and fail-closed behavior. They
do not prove FyAgent's behavior on real Windows 10/11, x64/ARM64, or a
Bob-elevated/Alice-Shell boundary.

### A1/A2 evidence boundary

A1 uses the protected local file URI with Alice's current-user
`AddPackageByUriAsync`. This delivery intentionally runs no HIL locally or in
GitHub Actions. Its present evidence is limited to static contracts, scoped
Windows-target compilation checks, and code/security review. The package's
readability under the minimum Alice/SYSTEM ACL, exact consumed/installed
identity, mutation denial, and terminal/orphan cleanup remain unverified on
real Windows 10/11, x64/ARM64, and Bob-elevated/Alice-standard-Explorer-Shell
systems. These are explicit residual risks and the present evidence cannot be
reported as native compatibility or native runtime verification.

Only future independent native validation plus an explicit, separately
authorized design decision may enter the A2 boundary: the elevated parent may call
`StagePackageAsync` on the same protected file and wait for a true terminal
result, then the Alice helper may call only
`RegisterPackagesByFullNameAsync` with the exact PackageFullName. That branch
must reject or independently verify dirty pre-staged state, provide no extra
dependency/optional/related URI, never Provision, and never blindly use
`RemoveForAllUsers` to clean an orphan. A2 is not implemented or silently
selected, and no runtime HRESULT, ACL, disk, timeout, or missing-validation
condition may select it.

The helper remains Alice-owned rather than PPL/brokered. Verified image,
BA-owned controls, authenticated pipe, and protected namespace prove the
reviewed data path and make late replacement fail closed; they do not claim to
resist same-SID memory injection or arbitrary manipulation inside Alice's own
current-user PackageManager authority. Closing that stronger boundary requires
the still-excluded durable broker/service.

## Installer/release observations

- Formal app builds use `requireAdministrator`; test/development builds use
  `asInvoker`. NSIS is perMachine and exposes the standard directory page.
- The replacement NSIS gate requests normal shutdown and otherwise aborts; it
  never calls the upstream force-kill path. Its final process lookup and first
  mutation are not atomic. A real file-share probe showed that holding a
  no-read-share executable handle blocks a later launch but can still be
  acquired after an image is already mapped, so a complete maintenance handoff
  would need persistent installer/application state. That wider protocol is
  excluded here and the launch race remains an explicit point-in-time residual.
- The lifecycle harness currently exercises installer mechanics but treats
  unsafe ProgramData preimages as failure and is not a Release gate.
- CI has matching `windows-2025` x64 and `windows-11-arm` ARM64 native paths,
  but release currently relies on build/package proof rather than executing
  final setup/uninstall.
- Release dispatch is a five-target same-SHA preflight. It attests candidate
  bytes and skips publish; tag push is the only formal publication path.
- The current public release baseline is `v0.3.0`; immutable Windows MSI asset
  IDs/names/sizes and repository-pinned SHA-256 values must be captured by the
  lifecycle baseline contract before download.
- Version remains `0.3.1`, but an existing annotated `v0.3.1` tag already
  points to another historical SHA. This task must not move/reuse it and does
  not establish a future formal `v0.3.1` closure.

## Remote closure invariant

Release eligibility binds dispatch to the current remote
`dev/laiyongjie` HEAD and the exact successful full push CI attempt for the
same SHA. Therefore all in-scope work commits must be pushed together as H1 before
dispatch. Archiving/journaling after preflight would move the branch and make
that preflight no longer describe remote HEAD, so those two commits stay local
only by design.
