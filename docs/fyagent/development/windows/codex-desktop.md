# Windows runtime and Codex Desktop flow

Two security boundaries meet in the Windows Codex Desktop path:

- the host freezes the Explorer user's SID and Profile/LocalAppData/
  RoamingAppData before Tauri or any user-state consumer starts;
- Codex Desktop services and platform adapters own trusted-package discovery,
  installation/update, post-verification, restart, and launch for that frozen
  user.

Retained runtime-security and Codex Desktop installer notes under
`.trellis/spec/` are optional AI-assistance review material.

## Shell-user data flow

```text
process start
  -> current Explorer process and token
  -> freeze Shell session, canonical SID, Profile, LocalAppData, RoamingAppData
  -> initialize WebView/config/log/database/state on those paths
  -> explicit-SID Codex package discovery
  -> recheck the fixed downloaded MSIX and repeat signature admission on its held file
  -> parent seals one protected ProgramData copy by hash-while-copy
  -> launch the fixed current-user helper through Explorer
  -> `Hello`, authenticate helper, send bridge control, verify `Started`, admit
  -> helper serves the pinned leaf on an opaque ephemeral loopback capability
  -> PackageManager fetches those handle-bound bytes and reaches true terminal state
  -> clean pipe close permits application-owned known-only cleanup
  -> post-verification and launch for the same frozen context
```

An administrator named Bob may approve UAC while Alice owns the Explorer
Shell. FyAgent continues in that case and treats Alice as the user: it does not
fall back to Bob's profile, `HKCU`, process environment, SYSTEM, the current
directory, or a default drive. If the Shell user or one of the required known
folders cannot be resolved, startup fails before user data is read or written.

Windows supplies explicit Alice-owned paths for the WebView2 data directory,
application-path Store, window state, configuration, logs, and database. The
same LocalAppData projection owns transient database, provider hand-off, sync,
and skill-processing files instead of the elevated process temp directory. The
application builds only the configured `main` window manually so Tauri cannot
derive its WebView path from the elevated account; lightweight-mode recreation
uses the same path. A later Shell identity drift stops the next protected side
effect rather than selecting a different user.

The Windows single-instance plugin is coordination, not authentication. Its
callback bounds the complete argument envelope and validates deep links before
it can restore lightweight mode, emit a confirmation request, or focus the
window. Callback input never starts package installation, a helper, cleanup,
an elevated file operation, or an arbitrary command/path. The pinned plugin's
Windows transport still decodes local `WM_COPYDATA` before the callback and
does not authenticate the peer; keeping the callback non-privileged is the
required containment for that accepted residual dependency risk.

Official Codex Desktop packages on Windows are MSIX packages. That package
format belongs to the software FyAgent manages and is independent of the NSIS
format used to install FyAgent itself.

## Download staging versus final placement

Windows chooses the final MSIX location. FyAgent neither fakes nor exposes a
final-destination picker. The confirmation can instead change the separate
download-staging directory that FyAgent actually owns. The renderer asks the
backend to open the native folder picker and receives only a bounded label,
capacity, and random five-minute one-use capability; the selected path never
crosses renderer IPC.

Selection admission canonicalizes the existing directory and retains no-follow
handles and identities for its complete ancestor chain. In a real UUID probe
child it creates, writes, flushes, finalizes, reopens and reads an MSIX-shaped
artifact, removes that file and child, then checks the selected volume. Zero
capacity is rejected immediately and prepare recomputes capacity instead of
trusting a stale capability value. Prepare consumes the capability and binds the
same handle chain into its confirmation. Issuance schedules autonomous
five-minute expiry; dismissing or navigating away sends an identifier-only
discard so retained no-delete-share handles release immediately. Unsafe or
unavailable admission, write/cleanup permission denial, capacity-probe failure,
and insufficient space have distinct staging platform codes and offer native
directory re-selection. Download creation, partial-file finalization, signature
verification, install handoff, failure recovery, cancellation, and success
cleanup all use that bound staging root and fail closed on identity/reparse
drift. Directory sharing prevents an ancestor rename or deletion until the
operation releases the capability. The owned UUID job directory is removed at
terminal cleanup; the user-selected directory remains. Vendor EXE final-location
choices, if any, remain inside the vendor UI.

## Current-user helper boundary

FyAgent packages a Windows-only `fyagent-user-helper.exe` with its own
`asInvoker` manifest and no Tauri UI/runtime dependency. It accepts only this
fixed command shape:

```text
fyagent-user-helper.exe codex-msix-install --job-id <canonical-lowercase-uuid> --pipe <64-lowercase-hex>
```

The helper derives its install root from `current_exe()` only to bind its own
installed image. It does not derive the package source from that tree and does
not accept an executable, command, URI, package path, bridge root, operation ID,
installer scope, or validation bypass. After authenticated `Hello`, the parent
sends one fixed bridge control and the helper independently resolves the fixed
CommonApplicationData hierarchy. Its only A1 deployment operation is
current-user `PackageManager.AddPackageByUriAsync`; the retired headless/runas
control and job files, all-users DTOs, Stage, and Provision operations have no
replacement in the shipped path.

The elevated parent creates one local first-instance duplex pipe before asking
Explorer to launch the fixed sibling helper as Alice. Its session-local
`LOCAL\` name combines a fixed versioned prefix with a random 256-bit nonce.
The BA-owned descriptor gives Alice `FILE_GENERIC_READ` plus `FILE_WRITE_DATA`
(`FILE_READ_ATTRIBUTES` is required to connect to a named pipe);
SYSTEM and Administrators retain only `READ_CONTROL`. No generic-write alias
grants pipe-instance creation.

The parent also first-creates BA-owned admission and cancellation events. Alice
can synchronize and inspect their owner but cannot signal them. The helper
opens, never creates, the two events and pipe once and verifies the actual
owner of all three handles is Builtin Administrators. A late helper therefore
rejects absent objects or Alice-created replacements before `Hello` or
AddPackage; the nonce is not treated as sufficient authority. Cancellation is
checked before admission when both are signaled.

The parent reads one bounded raw fieldless `Hello` only to enable pipe
impersonation. It gets PID/session from the pipe, opens that process only for
exact pinned-image/synchronization proof, and gets SID/session from the
impersonated pipe-client token; it never opens a separate process token. After
explicitly reverting impersonation and authenticating Alice's helper, the
parent writes one exact 80-byte `FYABRIDG` version-2 control. It contains only
the expected volume serial/file index/size and a parent-generated 256-bit bridge
operation ID—never a host, URI, filename, path, SID, or mode.

The ordering is fixed: `Hello` → peer authentication → bridge control → helper
bridge/URI proof → `Started { package identity }` → parent context/pin recheck →
admission → progress → one terminal result. The helper independently resolves
the fixed bridge and compares its no-follow pin with the control before
`Started`; the parent compares `Started` with its own pin before signaling
admission. The helper cannot call PackageManager before admission.

Messages are versioned and length-prefixed under a small absolute cap. The only
helper frames are `Hello`, `Started { package identity }`, strictly increasing
bounded `progress`, one `success`, or one structured bounded `error`. Before
admission, unknown/trailing/oversized data, invalid UTF-8, a missing or
out-of-order step, timeout, transport failure, or early exit may terminate with
a structured error because PackageManager has not run. After admission, any
invalid progress or terminal, duplicate/extra data, protocol/transport error,
timeout, early exit, or unclean close causes best-effort cancellation followed
by permanent process-lifetime quarantine. The Job remains `Installing`, no
terminal result is published to the renderer, and the operation is not cleaned.
Only an authenticated valid terminal status, its matching valid terminal frame,
and a clean pipe close permit cleanup.

## Protected package bridge and process lifetime

The production path stages under the install-root cache and opens the current
job's fixed MSIX with `GENERIC_READ + FILE_SHARE_READ`. The elevated parent
repeats local signature admission against that protected source handle, then
copies and hashes the same object into:

```text
<FOLDERID_ProgramData>\FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}\v1\<64-lowercase-hex-operation-id>\installer.msix
```

The fixed root and `v1` use one stable BA-owned/grouped protected ACL independent
of Alice. Every create-new operation directory and `.part`/final file has a
separate protected ACL granting only BA management, minimum SYSTEM read/traverse,
and minimum read/traverse for the exact frozen Alice SID. Exact bridge objects
reject creation, content/metadata mutation, deletion, and DACL/owner changes.
Incompatible existing objects are rejected, not repaired.

For Codex MSIX, OS-owned ancestors from the volume root through ProgramData
reject effective `DELETE_CHILD`, `DELETE`, `WRITE_DAC`, and `WRITE_OWNER` for a
non-administrator Alice. Standard ProgramData ACLs may grant `FILE_WRITE_EA` or
`FILE_WRITE_ATTRIBUTES`; those metadata rights are accepted only because no
asynchronous consumer later reopens the MSIX through that namespace. Vendor EXE
actions still use `ShellExecuteExW` and therefore retain the stricter ancestor
metadata rejection.

The helper resolves every fixed bridge component relative to retained no-follow
handles, verifies the final volume serial/file index/size from the authenticated
control, and keeps that exact file open. It then binds an ephemeral IPv4
loopback listener and derives one unguessable request path from the 256-bit
operation ID. A fixed four-worker set prevents slow-drip local clients from
occupying PackageManager's stream indefinitely. Each accepted socket has one
absolute, monotonic 500 ms deadline for the complete header, checks shutdown
between reads, and has a bounded write deadline. Each worker reuses fixed
request and transfer buffers. Only bounded `HEAD`, `GET`, and one-range requests
for the capability path are served, using positional reads from the pinned
handle. PackageManager receives this fixed
`http://127.0.0.1` capability instead of a ProgramData file URI. A namespace
race can make initial helper admission fail, but cannot change the bytes
consumed after admission.

The ProgramData volume must be local fixed NTFS and have enough space for the
extra full copy. The parent handles short reads/writes while hashing, flushes
`.part`, renames without replacement, reopens the final leaf no-follow, and
proves exact SHA/size/source-object, file-ID, link/reparse/placeholder,
owner/group, and DACL continuity. These SHA/size values come from the current
download and prove same-file handoff only; they are never compared with mirror
or upstream publication fields. Windows remains authoritative for native MSIX
deployment and trust.

The parent keeps the source pin, bridge ancestors/file, helper-image pin,
control events, duplex pipe, and admitted process in one lifetime. The helper
keeps its package handle and loopback server until a non-`Started` WinRT
terminal status. Normal cleanup requires the matching authenticated terminal
frame and clean pipe close. Timeout, protocol loss, progress-write failure,
handler-registration failure, ambiguous synchronous AddPackage error, crash,
or termination leaves an immutable operation orphan. The next elevated bridge
creation may perform bounded known-only cleanup through held handles; unknown,
reparse, ACL-drifted, inaccessible, nonempty, or changing objects survive.
NSIS never owns, repairs, enumerates, or removes PackageBridge.

Normal renderer exit/restart paths use the fixed `exit_app`/`restart_app`
commands. Their process-lifetime claim shares one mutex with installer start,
so exactly one wins and a running, cancellation-pending, admitted, or
quarantined job blocks normal termination. The default renderer capability
contains no `process:allow-exit`, `process:allow-restart`, or `process:default`.

Three process limits remain explicit. The Alice helper is not protected from
same-SID code injection, memory writes, or handle manipulation; resisting that
attacker would require the excluded trusted broker/service model, and Alice
already owns current-user PackageManager authority. Administrator force-kill,
process crash, and OS shutdown can destroy in-process source/pin ownership and
are not durable terminal evidence; the protected orphan remains fail-closed.
Finally, NSIS asks a running FyAgent/helper to close normally and never
force-terminates it, but the last process lookup is only a point-in-time check
rather than an atomic launch interlock.

The helper tests exercise the real Windows loopback/file-handle path, including
range reads and rejection of the wrong capability path. A real PackageManager
deployment still requires a disposable signed Codex package and authorization
to change the interactive user's installed apps. Unless that HIL path is
actually run, Bob-elevated/Alice-standard identity, PackageManager consumption,
UAC, post-install inventory, and native compatibility remain explicitly
unverified; the socket smoke test does not substitute for installation proof.
Native deployment and post-install checks still surface unsupported operating
systems or packages. FyAgent does not maintain a duplicate package `MinVersion`
allowlist before helper launch. A2 requires future independent native validation
plus an explicit, separately authorized design decision; it is never a runtime fallback.
The minimum supported Windows version does not change under this policy.

Installation failures and rejected installer requests are recorded in the
application log with the error code, stage when available, platform error code,
and redacted diagnostic message. A rejected ProgramData ancestor also logs its
directory path and effective access mask and the specific rejected rights. This
local diagnostic can reveal a customized ProgramData location; review it before
sharing logs.

## Testing boundary

Portable Rust and frontend fixtures cover same-user and Bob/Alice identity,
missing Shell/folder failures, immutable context propagation, backend-owned
directory defaults, single-instance input limits, and context-bound package
inventory. Helper/source tests cover the exact CLI/layout, duplex protocol and
80-byte bridge control, `Hello`/control/`Started`/admit ordering, stable-root and
exact-Alice operation ACLs, copy/URI/object continuity, mutation denial,
application-owned cleanup/orphans, NSIS non-ownership, AddPackage-only runtime,
independent `asInvoker` manifest, terminal/quarantine paths, and the atomic
lifecycle claim. Static
contracts ensure Windows production paths do not derive user state from ambient
profile/app-data/tool-home variables or `HKCU`.

## One-click executable software policy

All FyAgent one-click executable software install and upgrade flows, including
future products and platforms, use fixed product-owned source endpoints but do
not admit or reject a download by comparing upstream hash, byte size, package
identity, version, minimum-OS, publisher/team, architecture, or signature
publication fields. Metadata size may be used only as a nullable progress and
disk-space hint. URL, path, scope, identity, hash, or validation-bypass inputs
must not be added to renderer IPC, CLI, or helpers.

This policy does not apply to Skills, plugins, MCP packages, configuration
packs, or other extension/configuration data. Their existing validation rules
remain independently owned. Transport bounds, protected temporary files,
same-file handoff checks, native installer behavior, atomic replacement and
rollback, and post-install existence/version/runnable verification remain in
force.

This delivery runs no Windows runtime HIL in Actions or on a local machine.
Static contracts, portable fixtures, scoped Windows-target compilation checks,
and review are the available evidence; they do not verify Explorer tokens, UAC,
WebView2 paths, Windows registry behavior, the protected bridge/ACL/file URI,
PackageManager terminal behavior, cleanup, or setup/uninstall lifecycle on real
Windows 10/11, x64/ARM64, or Bob/Alice systems. Those are explicit residual
risks and prohibit a native-compatibility or native-runtime-verified claim.
