# Technical design

## 1. Authority and migration shape

This task is a directed architecture replacement on top of the existing NSIS
and release trust chain. It does not reopen or rewrite archived work. The
current task artifacts and updated active specs become authority for the
changed contracts; retained behavior continues to use existing owners.

The migration has two independent but ordered planes:

1. Remove FyAgent-specific Trellis/mise enforcement while retaining upstream
   Trellis files as optional assistance.
2. Replace the Windows machine-runtime/equal-user model with an immutable
   Shell-user context and a narrow Shell-user package helper.

Executable Windows installer lifecycle and HIL remain outside CI and Release.
The retained harness is an optional manual diagnostic that is not scheduled in
this delivery. The implementation is split into scoped commits so each plane
can be reviewed or reverted at its ownership boundary.

## 2. Tooling boundary

### Retained

- `mise` remains the repository's developer task runner for bootstrap,
  environment checks, development, formatting, tests, contracts, and release
  checks.
- `.trellis/**`, `AGENTS.md`, upstream agents/skills/hooks, archived tasks, and
  journals remain available.
- The upstream Codex hooks continue to inject workflow breadcrumbs and
  task-local context when their generic runtime state is available.

### Removed

- `.mise/tasks/trellis.toml` inclusion and `scripts/tasks/trellis.mjs`.
- Overlay manifest, transform assets, reconciliation and verification scripts,
  overlay-owned tests, and CI/check dependencies.
- Project-local `fyagent-trellis` skill, Codex hook runner/mise hook tasks, and
  bootstrap-time prompt preparation/injection.
- Tests and docs asserting that contribution, build, CI, or release require a
  Trellis task/spec/CLI or forbid direct bundled task scripts.

After the decoupling commit, this active task uses the retained upstream
scripts directly for lifecycle operations. No new wrapper or long-term project
API is introduced.

## 3. Frozen Windows interactive-user context

`WindowsInteractiveUserContext` is created once during process initialization
before any component derives a user path. It contains:

```text
process_session_id
shell_session_id
canonical_sid
user_profile
user_local_app_data
user_roaming_app_data
```

Resolution uses the Explorer Shell window/process/token for identity and
Windows token/profile-known-folder APIs for paths. The process token remains
relevant only for privilege and diagnostic facts. It does not select the user.

The context is immutable and internal. Services receive references/clones of
the frozen value rather than querying ambient environment on demand. A live
side-effect boundary may re-prove that the current Shell token still matches
the frozen session/SID, but it cannot replace the context with another user.

Initialization outcomes:

- same process/Shell user: continue with the Shell paths;
- elevated Bob with Shell Alice: continue with Alice's paths and SID;
- absent Shell, token/profile lookup failure, noncanonical SID, or known-folder
  failure: fail explicitly before reading/writing user state;
- non-Windows: preserve existing path and single-instance behavior.

Panic logging must have an explicit early-failure destination that does not
pretend an ambient process path is user state. Once the Shell context exists,
panic logs, configuration, databases, provider state, tray state, and all
other FyAgent per-user paths use it.

## 4. Windows single-instance boundary

The Windows custom runtime state/lease/capability pipe is deleted. The existing
Tauri single-instance plugin is registered uniformly. Its callback enters the
same bounded argument normalizer used by the existing deep-link/lightweight/
focus flow.

The callback is an untrusted request boundary because the plugin's predictable
local mutex/window transport is not an application capability. It may request
only:

- parse and surface an allowlisted deep link through the established guarded
  confirmation flow;
- open the lightweight window through existing rules;
- focus/show the current window when no actionable argument exists.

It cannot invoke the helper, PackageManager, elevated cleanup, or arbitrary
filesystem operations. Count, item-size, aggregate-size, control-character,
scheme/version/action, and deep-link DTO validation all precede behavior.

## 5. Helper process and IPC

### Binary boundary

`fyagent-user-helper.exe` is a separate Windows binary target with an
`asInvoker` manifest and minimal dependencies. It does not construct Tauri or
expose the renderer command table.

The CLI parser accepts one action and exactly two named values. `job-id` is a
canonical UUID. `pipe` is exactly 64 lowercase hexadecimal characters. Duplicate,
missing, unknown, positional, option-like, or oversized values are rejected.

The helper derives:

```text
install_root = parent(current_exe())
pipe_name = fixed_prefix + nonce
```

The package source is not derived from the install root and is not a CLI
value. The helper first sends a fieldless `Hello`; only after the parent binds
that frame to an authenticated peer does it send one fixed-width bridge control
containing a random canonical operation ID and the expected package file
identity. The helper independently resolves `FOLDERID_ProgramData` and the
fixed bridge layout from constants shared by the parent/helper library. No
input can replace the executable, install root, bridge root, package path, URI,
operation ID, or operation kind.

### Launch and identity

The elevated main process reuses the existing Explorer COM launch boundary to
start the fixed helper path as the frozen Shell user. It does not use headless
runas, a generic command line, or a parent-owned control file.

The parent owns one named-pipe server created before launch with:

- local-only and first-instance flags;
- a descriptor granting the frozen Shell SID plus SYSTEM/Administrators only;
- a fixed prefix and a cryptographically random 256-bit nonce;
- one accepted connection, bounded timeout, and handle destruction afterward.

After the bounded `Hello` read, the parent resolves the pipe client PID/session,
binds the read to the impersonated pipe-client token, verifies the canonical SID
equals the frozen Shell SID, and proves the pinned helper image. The helper also
verifies it connected to the expected BA-owned one-shot controls. No bridge
control or package operation is processed before this peer authentication.

### Protocol

The wire format is versioned and length-prefixed with a small absolute frame
cap. The only ordered conversation is:

```text
helper -> parent: Hello
parent authenticates PID/session/SID/image
parent -> helper: bridge control { operation ID, expected package identity }
helper -> parent: Started { package identity }
parent revalidates context/pins and signals admission
helper -> parent: progress { completed: 0..100 }
helper -> parent: success | error { bounded code and redacted message }
```

Before admission, an unknown version/variant, out-of-order `Hello`/control/
`Started`, trailing bytes, oversized length, malformed UTF-8, timeout, early
exit, or second client may terminate the operation through the existing
structured/redacted installer error surface; PackageManager has not run. After
admission, any invalid progress or terminal, duplicate/extra data, protocol or
transport error, timeout, early exit, or unclean close instead causes
best-effort cancellation followed by permanent process-lifetime quarantine.
The Job remains `Installing`, no terminal result is published to the renderer,
and the operation is not cleaned. Cleanup is allowed only after an
authenticated valid terminal status, its matching valid terminal frame, and a
clean pipe close.

The helper calls `PackageManager.AddPackageByUriAsync` for its own current user
only. There is no Stage/Provision/all-user API.

### Protected one-operation PackageManager source

The elevated parent creates a separate machine-data bridge for each install:

```text
<FOLDERID_ProgramData>/
  FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}/
    v1/
      <64-lowercase-hex-operation-id>/
        installer.msix.part
        installer.msix
```

The parent resolves the known folder through `SHGetKnownFolderPath`; it never
hard-codes `C:\ProgramData`, reads an environment override, or falls back to a
temporary/current directory. The fixed product root and `v1` are opened through
held-parent, no-follow capabilities and are either create-new or accepted only
after exact type, BA owner/group, protected allow-only DACL, and identity
verification. Their ACL is stable across users: BA gets lifecycle management,
SYSTEM gets the required read/traverse access, and Authenticated Users (`AU`)
gets stable directory `FILE_GENERIC_EXECUTE` semantics—traverse, read
attributes, `READ_CONTROL`, and synchronize—to reach an already-known random
child, never list/create/write/delete/delete-child. It is not rebound to the
first Alice. Each operation directory and `.part`/final file is create-new and
has a distinct protected ACL with BA management, minimum SYSTEM read/traverse,
and minimum read/traverse for the exact frozen Alice SID. No broad principal can
create, modify, delete, replace, hard-link, reparse, take ownership, or rewrite
the DACL. Effective access through the ProgramData parent is also checked; an
Alice route to `DELETE_CHILD` makes the root unsafe. The application never
repairs an incompatible preimage in place.

The parent streams bytes only from its already verified source `File` handle
into a create-new `.part` leaf, computing exact size and SHA-256 while copying.
After a complete write it flushes the file, handle-renames it without
replacement to the final fixed name, reopens the final leaf no-follow using
`GENERIC_READ + FILE_SHARE_READ`, and revalidates hash, size, volume/file ID,
link/reparse state, owner/group/DACL, and exact continuity with the already
validated source. Parent preflight owns release SHA/size and bounded ZIP/
manifest publisher/name/version/architecture/OS checks; PackageManager remains
the native MSIX signature-chain authority. The source pin, bridge ancestry, and
sealed bridge file all remain live through authenticated settlement, including
the terminal status/frame and clean-close proof below.

The helper first emits a fieldless `Hello` frame. The parent raw-reads that
frame, binds pipe impersonation to the read, proves PID/image/SID/session, then
sends the fixed bridge control; no operation ID crosses the CLI or an
unauthenticated connection. The record contains no arbitrary path or URI. The
helper resolves the same known folder and fixed direct-child names, validates
every ancestor and the final file no-follow, checks the parent-supplied identity,
and constructs an ordinary DOS `file://` URI with `UrlCreateFromPathW`. It
round-trips that canonical URI with `PathCreateFromUrlW`, rejects UNC/host,
query/fragment, extended-path, overlength, or encoding ambiguity, reopens the
result no-follow, and requires the same object identity before it sends
`Started { package identity }`. The parent compares that identity, rechecks its
bridge pin and frozen Shell context, and only then signals admission. The helper
must wait for that signal before calling PackageManager. PackageManager
therefore consumes a name in a namespace Alice can read but cannot rebind. The
helper does not hash the package a second time and accepts no package path or URI
from CLI/renderer input.

There is no HTTP or network source fallback; the rejected design evidence is
retained in task research. A1 is the only implementation in this change. This
delivery intentionally runs no HIL locally or in GitHub Actions; present
evidence is limited to static contracts, scoped Windows-target compilation
checks, and code/security review. Real Windows 10/11, x64/ARM64,
Bob-elevated/Alice-standard-Explorer-Shell, PackageManager/protected DOS file-
URI, effective ACL/mutation-denial, and terminal/orphan/cleanup behavior remains
explicitly unverified and cannot support a native-compatibility or native-
runtime-verification claim. Only future independent native validation plus an
explicit, separately authorized design decision may start a separate A2
implementation/review: the elevated parent would Stage only this same protected
file to a true terminal result, and the Alice helper would Register only the
exact verified PackageFullName. A2 is not compiled as a runtime branch in A1,
is never selected by an HRESULT/ACL/disk/timeout/missing-evidence condition,
provides no dependency/optional/related URI, and may neither Provision nor
blindly remove an existing/staged package or another user's registration.

The product minimum Windows version does not change. Existing OS and package
`MinVersion` preflight rejects an unsupported host/package before helper launch;
it never routes around A1.

## 6. Staging and byte-identity continuity

The main process derives the install root from its own verified executable,
then creates only this hierarchy:

```text
<install-root>/cache/codex-installer/<uuid>/installer.msix.part
<install-root>/cache/codex-installer/<uuid>/installer.msix
```

Every ancestor/leaf is checked as a normal non-reparse object. Cleanup accepts
only canonical UUID children and known fixed names; unknown entries are left
alone and reported diagnostically. No recursive delete owns the install root
or an unknown cache subtree.

The free-space probe resolves the volume containing the real install root.
Failure to resolve/query/write that root is terminal; it never substitutes a
drive letter.

After download and full package validation, the parent opens the final
install-root MSIX with `GENERIC_READ` and only `FILE_SHARE_READ`, captures
volume/file identity and size, and copies from that handle into the protected
bridge. No install-root ACL is added. The bridge copy is a deliberately
separate trusted namespace rather than an assertion that a share-restricting
handle alone freezes an Alice-writable pathname; POSIX replacement semantics
make the latter insufficient.

The failure matrix covers an incompatible source handle, source drift during
copy, short read/write, hash or size mismatch, flush/rename failure, insufficient
space on the actual ProgramData volume, unsupported filesystem/drive type,
unsafe ProgramData/root preimages, ancestor `DELETE_CHILD`, file/ACL/owner/link
drift, DOS URI round-trip ambiguity, helper timeout, parent cancellation, and
all Alice write/replace/delete/hardlink/reparse attempts. None may fall back to
the old install-root pathname, HTTP, a temporary directory, or current cwd.

The application bridge module owns both cleanup paths. Normal cleanup removes
only the exact operation leaves and empty directory through held handles, and
only after an authenticated non-`Started` WinRT terminal status, its matching
valid terminal frame, and clean pipe close. An indeterminate or interrupted
operation leaves an immutable orphan. The next elevated bridge creation may
perform bounded opportunistic cleanup and admits only the fixed bridge
hierarchy, canonical 64-hex operation IDs, exact expected owner/group/DACL, and
the two known leaf names. Unknown, reparse, ACL-drifted, inaccessible, nonempty,
or concurrently changing objects survive, and operation IDs are never reused.
NSIS never enumerates, repairs, or removes PackageBridge; an orphan that outlives
application uninstall is an explicitly retained immutable diagnostic rather
than a reason to weaken cleanup validation.

## 7. NSIS lifecycle and legacy cleanup

The checked-in NSIS template retains standard `perMachine` directory selection
and the dual main-manifest model. It no longer calls or requires ProgramData
runtime bootstrap. It packages the helper and its manifest, registers the main
application/shortcuts/protocol as before, and performs allowlisted uninstall.

Legacy cleanup is isolated from admission:

- known old runtime state/lease files and empty known directories may be
  removed best-effort;
- reparse points, unknown names, access errors, and nonempty ancestors are
  preserved;
- cleanup failure never aborts installation/uninstallation and never triggers
  recursive repair.

Setup and uninstall never force-terminate the main process or helper. An
interactive caller must close them normally and retry; passive or silent mode
fails before migration, cleanup, or payload mutation. The NSIS process lookup
is intentionally not described as an atomic launch interlock: another
privileged launch can race the final lookup and the first mutation. Closing
that residual would require a persistent installer/application handoff
protocol that is outside this task. The important in-scope invariant is that
the installer does not itself terminate a process that may retain an admitted
package source or pin.

The retained harness is an optional manual diagnostic and is not run in this
delivery. If a future operator independently elects to run it, the harness owns
each installation case in a unique root. For D-drive cases it either uses a
pre-existing `D:` without changing it or creates one temporary VHD/VHDX with a
unique path, initializes only the new image, and registers finally-style
detach/delete cleanup. A mounted or image-identity drift fails without touching
another disk.

The upgrade source is frozen in repository data by public release tag, asset
ID, name, size, and SHA-256. The harness downloads by immutable asset identity,
checks every field and digest, then installs it before upgrading with H1's
setup.

## 8. CI and release evidence topology

CI and release keep native Windows x64/ARM64 build, package, manifest, icon,
signing/sealing, immutable pin, verification, and attestation evidence. They do
not execute final setup/uninstall, PackageBridge A1, or any other HIL. The
non-publishing preflight topology is:

```text
native build -> immutable input pin -> preflight proof/seal
             -> immutable sealed candidate -> verify-assets -> attest
```

Dispatch mode continues through unsigned proof, asset verification, and
attestation. Formal signer/sealer jobs and publish are skipped by existing mode
conditions. The subject allowlist stays thirteen and the attachment allowlist
stays fourteen. This evidence must not be interpreted as setup/uninstall,
PackageManager, file-URI, ACL, cleanup, or native compatibility verification.

## 9. Compatibility and rollback

- Web/API/database and renderer current-user semantics remain stable.
- macOS/Linux path and single-instance behavior remain stable.
- Reverting tooling commits restores only project enforcement, not product
  runtime behavior.
- Reverting Windows runtime/helper/staging commits must be done as an ordered
  group because their deleted machine-runtime assumptions are incompatible
  with the new Shell-user flow.
- The optional manual lifecycle harness is not a CI/Release gate and is not
  scheduled by this task. Its absence remains part of the explicit unverified
  native-runtime risk.
- No rollback moves a remote tag, rewrites history, or publishes an alternate
  asset.

## 10. Security decisions

- Official Trellis `0.6.14` hooks are accepted with their weaker path/import/
  input/escaping protections; do not describe them as hardened.
- Single-instance activation is untrusted and cannot cross a privileged
  side-effect boundary directly.
- Helper authority is reduced by a fixed executable/action/path plus Shell SID
  authentication and one-shot pipe semantics.
- Verified MSIX bytes are copied from an open source handle into a BA-owned,
  protected-DACL bridge; the source, bridge ancestry, and sealed bridge file
  remain pinned until PackageManager finishes.
- Process termination, late launch, malformed IPC, and path/object replacement
  are in scope. Same-SID code injection or memory/handle manipulation inside
  the Alice-owned `asInvoker` helper is not a separate trust boundary: Alice
  already owns current-user PackageManager authority, and defending that case
  would require the explicitly excluded protected broker/service model. The
  helper's authenticated terminal evidence must not be described as resistant
  to same-user process injection.
- Ordinary UI, restart, update, and uninstall paths cannot terminate a process
  that owns an admitted source/pin. Administrator force-termination, process
  crash, and operating-system shutdown may leave an immutable bridge orphan,
  but cannot make that object Alice-writable; later known-only elevated cleanup
  owns recovery. No long-lived broker/service is introduced.
- Install-root ACLs remain a user-selected/Windows concern and are unchanged.
  The explicitly approved protected descriptor applies only to the independent
  one-operation ProgramData package bridge; it does not restore the retired
  ProgramData runtime/state/lease/HMAC/control model.
- No Actions HIL or setup/uninstaller lifecycle job may be inferred from the
  native build/package/sealing topology.
