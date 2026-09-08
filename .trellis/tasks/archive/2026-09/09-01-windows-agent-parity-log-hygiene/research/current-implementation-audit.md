# Current implementation audit

## Audit baseline

- Date: 2026-09-01
- Branch: `dev/laiyongjie`
- Baseline commit: `b1335f2f`
- Mode: repository read-only audit; no product implementation changed.

## 1. Unified lifecycle already exists

`src-tauri/src/agent_install/lifecycle_policy.rs` already owns the desired cross-platform matrix:

- QoderWork / TRAE Work / WorkBuddy: Desktop install + launch, no update;
- Codex / Claude / OpenCode: Desktop install + update + launch;
- Grok Build: CLI install + update.

The task must extend this owner, not create separate Windows/macOS product tables. Claude/OpenCode CLI and Qoder/TRAE/WorkBuddy update are already illegal surfaces/actions and must remain so.

## 2. Formal Windows ordinary-user boundary

- Windows release processes are elevated; `windows_runtime` explicitly notes that process-scoped HKCU/profile state can belong to the administrator who approved UAC rather than the Explorer user.
- The repository freezes Explorer SID, session, profile, LocalAppData, RoamingAppData and safe command search paths before user data initialization.
- `services/tooling.rs` rejects direct Windows CLI execution when the elevated/formal boundary is active.
- `src-tauri/user-helper/` already provides a closed parser, random job/nonce, one-shot named pipe, action binding, PID/SID/session/image checks, bounded frames/messages, ordered progress and timeout.
- Current helper actions cover Codex packaged deployment and selected Agent EXE installs; it is not a generic sidecar.

Conclusion: Grok formal-Windows support must extend the existing helper with a closed product/action DTO. It must not run user-profile CLI from the administrator process or add a generic command bridge.

## 3. Reusable Desktop owners

- `agent_install/inventory.rs`: inventory snapshots, opaque targets/destinations/revisions and readiness projection.
- `agent_install/desktop.rs`: desktop product descriptors, source routing and launch policy.
- `agent_install/windows.rs`: App Paths, Uninstall Registry 32/64, known paths, PE identity/version/architecture, WinVerifyTrust, actual signer and signed EXE execution.
- `agent_install/jobs.rs` and existing artifact owners: download, retry, cancellation, private job directories, progress and terminal state.
- `codex_desktop/platform/windows/*`: PackageManager inventory/deployment, PFN/AUMID, current-user binding, package bridge/helper and exact runtime control.
- macOS owners: managed DMG transactions and restricted privileged helper; these are regression targets, not rewrite targets.

No second downloader, PackageManager stack, EXE runner, helper, launcher or job state machine is justified.

## 4. Concrete Windows gaps

### QoderWork / TRAE Work / WorkBuddy

Windows product names/paths, sources, signer policy and EXE runner exist. However, prior work did not complete full native Windows x64 official-installer HIL. Likely classes to test—rather than assume—include current signer/product drift, scope/default directory, Explorer-vs-admin hive, bootstrapper child timing, custom paths and post-install polling.

### Claude Desktop

- Windows product names and relative executable hints are empty;
- fresh Windows destination is absent;
- source adapter supports macOS DMG only and returns unsupported on Windows;
- Windows package/PE identity and signer policy are not frozen.

### OpenCode Desktop

- Windows product names and relative executable hints are empty;
- fresh Windows destination is absent;
- source adapter supports macOS DMG only and returns unsupported on Windows;
- Windows signer/product identity is not frozen.

### Codex / ChatGPT Desktop

The current Windows owner uses an exact historical Codex identity. Current first-party product communication describes a new ChatGPT desktop app containing Codex and potential coexistence with ChatGPT Classic. Exact clean-install/upgrade/coexistence HIL is required before changing constants. The current exact identity boundary must not be replaced by display/process/window-name matching.

## 5. Duplicate non-Grok installer surface

`src/components/settings/AboutSection.tsx` still builds/displays install commands for Claude, Gemini, Grok, OpenCode, OpenClaw and Hermes using npm/Shell/PowerShell. `services/tooling/lifecycle.rs` still constructs corresponding non-Grok lifecycle actions, including remote-script flows.

This is a second public installer owner and directly conflicts with the user's required unified policy. The task must:

- retain Grok lifecycle only;
- remove/reject all other public install/update/manual-command actions on both OSes;
- preserve read-only CLI discovery/configuration only where another feature truly consumes it.

This is not optional UI cleanup: backend stale actions must fail before side effects.

## 6. Codex deferred log root cause

- Parent replay lookup returns a retryable condition when the parent timeline has not yet reached the child fork cutoff.
- `mark_deferred` tries to warn only when pending state changes.
- Before the next retry, the unchanged retryable pending entry is removed; the same condition is then reinserted as if new, so WARN repeats.
- The aggregate sync path also logs INFO whenever deferred files exist.
- The caller runs once at startup and every 60 seconds, amplifying an unchanged normal condition.

The fix must preserve retry/cursor/replay/dedup correctness while separating retry scheduling from diagnostic emission memory. See `research/codex-sync-log-root-cause.md`.

## 7. Dependency conclusion

Current Cargo dependencies already cover Windows APIs, Registry, PackageManager bindings, logging, HTTP and async runtime. Baseline implementation needs no new runtime dependency. WinGet may be researched per product but is not a new mandatory owner.

## 8. Audit conclusions

1. Keep one cross-platform lifecycle owner.
2. Retire non-Grok Tooling installers across macOS and Windows.
3. Route Grok formal Windows through the existing closed ordinary-user helper.
4. Use existing PackageManager for proven packaged apps and existing signed EXE runner for unpackaged apps.
5. Keep Qoder/TRAE/WorkBuddy install-only.
6. Freeze Claude/OpenCode/Codex exact identities from current artifacts and native HIL; do not guess.
7. Fix Codex deferred semantics, not merely the log level.
