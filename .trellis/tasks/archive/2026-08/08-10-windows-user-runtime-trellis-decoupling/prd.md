# Rebuild Windows user-scoped runtime and decouple Trellis

## Goal

Replace the Windows machine-runtime and all-users Codex installation model with
a fail-closed Shell-user model that works when an elevated FyAgent process and
the interactive Explorer Shell belong to different users. At the same time,
remove FyAgent's project-specific Trellis/mise coupling while retaining the
upstream-managed Trellis workspace as optional contributor assistance.

The accepted delivery candidate remains version `0.3.1`. One exact work SHA,
called `H1`, must be pushed to `dev/laiyongjie`, pass its full push CI, and pass
the same-SHA release dispatch preflight. The task is archived and journaled
locally only after those remote gates succeed.

## Product requirements

### Windows interactive user

- Create one immutable `WindowsInteractiveUserContext` before panic logging,
  configuration, database, tray initialization, or any user-path lookup.
- Resolve the interactive Explorer Shell's session, canonical SID, profile,
  LocalAppData, and RoamingAppData. A formal build fails explicitly when any
  required Shell identity or path cannot be proven.
- Support `process user = Bob` and `Shell user = Alice`. FyAgent user data,
  Codex inventory, Codex installation, restart, and launch belong to Alice.
- Never fall back to Bob, SYSTEM, the current directory, `%USERPROFILE%`, or
  another process-environment-derived user when the Shell context is absent or
  invalid.
- Remove the pre-Tauri process-SID-equals-Shell-SID gate, ProgramData runtime
  bootstrap/state/lease, capability/HMAC activation pipe, and experimental
  all-users command/deployment surface.
- Keep only bounded, known-name, best-effort cleanup for legacy
  `%ProgramData%\FyAgent\runtime` state. Unknown content and cleanup failures
  must not cause recursive deletion or block uninstall.

### Single-instance and untrusted activation input

- Register `tauri-plugin-single-instance` on Windows as well as macOS/Linux.
- Reuse the existing deep-link, lightweight-window, and focus behavior.
- Preserve bounded activation argument count, per-item size, and aggregate
  size. Invalid or oversized input is ignored/rejected before business logic.
- Treat the plugin's local mutex/window/`WM_COPYDATA` input as untrusted. A
  second-instance argument must never directly trigger elevated filesystem,
  helper, or PackageManager side effects.

### Current-user Codex helper

- Keep the existing renderer semantics: the Install/Update control means
  current-user installation only. Do not add an installer scope choice.
- Bundle a Windows-only `fyagent-user-helper.exe` with an independent
  `asInvoker` manifest and no Tauri UI/runtime linkage.
- The helper accepts exactly:

  ```text
  fyagent-user-helper.exe codex-msix-install --job-id <uuid> --pipe <256-bit-hex-nonce>
  ```

- It derives its installation root from `current_exe()` only to bind its own
  installed image. It neither derives nor accepts a package source from that
  tree. After sending `Hello` and passing parent authentication, it receives
  only one bounded bridge-operation control record and independently resolves
  that record through the fixed Windows CommonApplicationData bridge layout.
  It accepts no arbitrary executable, command, URI, package path, bridge root,
  or operation ID on the command line.
- The elevated parent launches the helper through the existing Explorer COM
  boundary so it runs as the frozen Shell user.
- Use a one-shot local pipe with a fixed prefix and high-entropy nonce,
  first-instance semantics, an Alice SID + SYSTEM/Administrators DACL, client
  PID/session/token-SID/image verification, bounded versioned messages, a
  timeout, one connection, and destruction after completion.
- The authenticated ordering is exactly `Hello` -> parent bridge control ->
  `Started { package identity }` -> parent admission -> bounded `progress` and
  one `success` or structured `error`. The helper must not resolve a bridge
  operation before control, and must not call current-user
  `PackageManager.AddPackageByUriAsync` before admission.
- The helper must not hand PackageManager a name that Alice can rebind away
  from the verified object. The elevated parent copies only from its already
  pinned MSIX handle into a create-new, BA-owned, protected-DACL, one-operation
  package bridge below `FOLDERID_ProgramData`. The helper receives only the
  random operation identifier plus expected file identity, reopens that fixed
  bridge leaf no-follow, and constructs a canonical DOS `file://` URI only
  after an exact identity/ACL/owner round trip. There is no runtime HTTP,
  proxy, network, user-temp, current-directory, or install-root path fallback.

### Install-root staging and verified-byte continuity

- Stage Windows downloads at
  `<install-root>\cache\codex-installer\<uuid>\installer.msix` using direct-child
  UUID directories, `.part` to final rename, a fixed filename, reparse
  rejection, and known-only cleanup.
- Probe free space on the actual install-root volume. An unresolvable volume,
  unwritable root, or insufficient space fails; there is no `C:` fallback.
- After all size, checksum, ZIP/manifest, publisher, identity, architecture,
  version, and OS checks succeed, reopen the exact file using
  `GENERIC_READ + FILE_SHARE_READ`, record and recheck volume serial/file
  index/size, and hold the source handle until the protected bridge copy is
  sealed and PackageManager has completed.
- Resolve `FOLDERID_ProgramData` through the known-folder API and use only
  `<ProgramData>/FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}/v1/<64-hex-id>/`
  with handle-relative, no-follow operations. Create the fixed root/`v1` when
  absent or accept them only after exact verification; every operation
  directory and leaf is create-new. Require local NTFS, BA ownership/group, a
  protected allow-only DACL, and BA management rights.
  The fixed product root and `v1` have one exact stable ACL independent of any
  user: SYSTEM gets only the required read/traverse access and Authenticated
  Users (`AU`) get stable directory `FILE_GENERIC_EXECUTE` semantics—traverse,
  read attributes, `READ_CONTROL`, and synchronize, never list/create/write/
  delete/delete-child. Each create-new operation directory and file has a
  separate exact ACL granting only SYSTEM and the frozen Alice SID the minimum
  read/traverse rights in addition to BA management. Do not change the
  user-selected install-root staging ACL or bind the fixed roots to the first
  Alice who installs.
- Stream `.part` only from the verified source handle while computing exact
  length and SHA-256, flush it, rename it without replacement to
  `installer.msix`, reopen no-follow with `GENERIC_READ + FILE_SHARE_READ`,
  and recheck hash, size, file identity, owner/group, DACL, reparse state, and
  exact continuity with the already validated source. Parent preflight retains
  release SHA/size plus bounded ZIP/manifest publisher/name/version/
  architecture/OS checks; the native MSIX signature chain remains
  PackageManager's sink authority. The parent and helper hold the bridge
  ancestry/file capabilities through authenticated settlement, including the
  terminal status/frame and clean-close proof below.
- Alice may read/traverse the sealed bridge but must be unable to create,
  append, write, rename, POSIX-replace, delete, hard-link, reparse, change the
  DACL/owner, or mutate an ancestor namespace. Any incompatible preimage,
  parent `DELETE_CHILD` exposure, unsupported filesystem/volume, disk-space
  failure, URI round-trip ambiguity, or identity drift fails closed without a
  Temp/current-directory/HTTP fallback.
- The application bridge module owns cleanup. Normal cleanup is allowed only
  after an authenticated non-`Started` WinRT terminal status, its matching valid
  terminal frame, and clean pipe close. A failed or indeterminate operation
  leaves one immutable random bridge directory. The next elevated bridge
  creation may perform known-only orphan cleanup through held handles, admitting
  only the fixed hierarchy, canonical operation IDs, and exact known leaves;
  unknown, reparse, ACL-drifted, nonempty, inaccessible, or changing objects
  survive. Operation IDs are never reused, and NSIS never owns, repairs, or
  removes this bridge.
- Keep the existing minimum supported Windows version. If the host or Codex
  package is below the applicable API/package `MinVersion`, existing OS/package
  preflight fails before helper launch; no compatibility branch or support-floor
  increase is introduced here.

### Installer and lifecycle

- Preserve NSIS `perMachine`, the standard directory page, the formal
  `requireAdministrator` application manifest, and test/development
  `asInvoker` manifests.
- Package and uninstall both the main executable and helper, while deleting
  only known install-root staging content and empty owned ancestors. NSIS does
  not enumerate or delete the separate PackageBridge tree.
- Preserve the installed path across reinstall/upgrade, including a previous
  install on `D:`.
- The retained x64/ARM64 lifecycle harness is an optional manual diagnostic for
  default and `D:\FyAgent-Acceptance` fresh install/start/uninstall,
  same-version reinstall, immutable `v0.3.0` MSI upgrade, D-drive upgrade,
  bounded legacy ProgramData preimages, cleanup failures, unknown-file
  retention, main/helper presence, shortcuts, registry values, and final
  installation location. It is not scheduled or required in this delivery.
- If a future operator elects to run the harness and the machine has no `D:`,
  create a task-owned temporary VHD/VHDX, format and assign only that image, and
  best-effort detach/delete it on every exit path. Never alter existing disks.
- The old MSI baseline is repository-pinned by tag, asset ID/name, size, and
  SHA-256; no `latest` URL or unchecked download is permitted.

### Tooling and documentation

- Commit the upstream-managed Trellis `0.6.14` hook registration and Python
  hook bytes as received.
- Remove the FyAgent Trellis overlay/reconcile/verify implementation, Trellis
  mise task include and wrapper, project-local `fyagent-trellis` skill, Codex
  hook runner, and automatic bootstrap prompt injection.
- Remove contracts that make `.trellis/**`, Trellis tasks/specs, overlay state,
  or Trellis CLI use a prerequisite for contribution, build, check, CI, or
  release. Do not create a replacement wrapper.
- Retain upstream `.trellis/**`, `AGENTS.md`, upstream skills/agents/hooks, and
  every archived task and journal as optional assistance and history.
- Keep standard mise task, locked toolchain, release, and developer-document
  consistency checks that do not depend on Trellis.
- Establish the standalone flow
  `mise trust -> mise run bootstrap -> mise run system:check -> mise run dev`,
  with `mise run check` as the pre-commit full gate.
- Remove obsolete project-contract wording from active specs precisely; retain
  durable AI-assistance knowledge that remains true.

### CI and release preflight

- A push to `dev/laiyongjie` remains a full CI run summarized by the unique
  `CI / Required` job.
- CI and release use native `windows-2025` x64 and `windows-11-arm` ARM64
  runners only for the build/package/manifest/icon/signing/sealing scope they
  actually execute. They do not run final NSIS setup/uninstall, PackageBridge
  A1, Bob/Alice, or other HIL in this delivery.
- Do not add release lifecycle smoke jobs or gate preflight on HIL. Exact-SHA
  eligibility, signer isolation, fresh sealing, thirteen attested subjects,
  fourteen release attachments, and the non-publishing dispatch topology remain
  intact.
- In dispatch mode, formal signing/sealing and publication remain skipped;
  unsigned preflight proof, verification, attestation, and exact fourteen-file
  attachment inventory must succeed.

## Delivery requirements

- Produce intentional domain commits in the approved order. A discovered
  defect may add a narrow `fix(...)` commit; do not amend or hide fixes in a
  catch-all commit.
- Before the only push, fetch and require remote `dev/laiyongjie` to remain the
  original task baseline ancestor. Never force-push or silently rebase/merge a
  remote move.
- Push the complete implementation once. Capture that exact SHA as `H1` and
  wait in the foreground for its unique push CI run to complete.
- After successful exact-`H1` CI, dispatch `release.yml` with
  `source_sha=H1`, wait in the foreground, verify the job topology and download
  and verify the exact fourteen release attachments for version `0.3.1`.
- Do not create, move, or delete a tag. Do not create a draft, prerelease, or
  formal GitHub Release. The existing unrelated `v0.3.1` tag is untouched.
- Archive this task and record the session only after preflight succeeds. Keep
  those two closeout commits local; the remote remains at `H1` and the local
  branch is exactly two commits ahead with a clean worktree.

## Explicit non-goals

- No web/API/database-schema change and no renderer-facing all-users option.
- No unrelated dependency upgrade, long-lived Windows service/broker, revival
  of the retired `%ProgramData%\FyAgent\runtime` state/lease/HMAC/control
  model, install-root staging ACL change, helper-side checksum pass, history
  rewrite, historical task/journal edit, CHANGELOG edit, or version bump. The
  isolated one-shot ProgramData package bridge and its explicit descriptor are
  the only newly approved machine-data exception.
- No claim that local Linux checks prove Windows setup, UAC, VHD,
  PackageManager, or native x64/ARM64 behavior.
- No formal `v0.3.1` release closure claim.

## Accepted residual risk

The upstream Trellis `0.6.14` Codex hooks are intentionally retained without
FyAgent's former project overlay. This removes the project's realpath
containment, exact-source import binding, strict Codex session/input checks,
and breadcrumb escaping. These hooks remain prompt-assistance code, but the
change is an explicit security regression acceptance, not an equivalent
security migration. It must remain visible in task evidence and the final
report.

The user-scoped helper is hardened against untrusted input, path/object races,
late launch, and termination without terminal evidence, but it is not a
protected process. Same-SID code injection or memory/handle manipulation of
the Alice-owned helper remains inside Alice's existing current-user
PackageManager authority. Resisting that attacker would require the explicitly
excluded trusted broker/service design. Likewise, administrator force-kill,
process crash, and operating-system shutdown are not durable-lifetime proof;
normal renderer, restart, installer, and uninstaller paths must still preserve
the in-process source/pin gate.

NSIS process discovery is a point-in-time guard, not an atomic prevention of a
new privileged launch between the last check and filesystem mutation. The
installer must never force-kill a discovered process and silent/passive flows
must fail before mutation, but a persistent cross-process launch-interlock or
installer handoff marker is intentionally not introduced in this task.

## Acceptance criteria

- [ ] All primary commits that remain in scope exist in order and are
      individually scoped; no Actions lifecycle/HIL commit is added.
- [ ] Active product/tooling code has zero all-users and retired ProgramData
      runtime execution paths; the only active ProgramData product path is the
      fixed one-shot package bridge plus bounded legacy/bridge cleanup, and
      historical archive/journal evidence is unchanged.
- [ ] Windows Shell-user context, untrusted single-instance input, helper IPC,
      install-root staging, protected bridge, DOS file-URI round trip, and
      pinned-file contracts have unit/static tests.
- [ ] Installer contracts and lifecycle harness cover x64/ARM64, default/D
      paths, reinstall, immutable v0.3.0 upgrades, legacy state, bounded
      cleanup, and VHD teardown.
- [ ] Current active specs and developer docs describe the replacement model
      and standalone mise workflow without making Trellis a project contract.
- [ ] Targeted frontend/Rust/contract checks and final `mise run check` pass.
- [ ] Record that this delivery runs no local or Actions HIL. Present evidence
      is limited to static contracts, scoped Windows-target compilation checks,
      and code/security review. Real Windows 10/11, x64/ARM64,
      Bob-elevated/Alice-standard-Explorer-Shell, PackageManager/protected DOS
      file-URI, effective ACL/mutation denial, and terminal/orphan/cleanup
      behavior remain explicit unverified residual risks and cannot support a
      native-compatibility or native-runtime-verification claim. A2 is never a
      shipped runtime branch or fallback; only future independent native
      validation plus an explicit, separately authorized design decision may
      start its implementation and review.
- [ ] Remote `H1` has one successful full push CI with one successful
      `CI / Required`.
- [ ] Same-SHA dispatch preflight succeeds with native build/package evidence,
      unsigned proof, verification, attestation, skipped formal/publish jobs,
      and exactly fourteen verified attachments. It schedules no lifecycle/HIL
      job and supplies no native-runtime compatibility evidence.
- [ ] No tag or GitHub Release changed; no temporary VHD/download remains.
- [ ] Task evidence records `H1`, run IDs/URLs, conclusions, build/package jobs,
      attachment verification, and explicit unverified native-runtime risks.
- [ ] Remote stays at `H1`; local has exactly the archive and journal commits
      above it; index and worktree are clean.
