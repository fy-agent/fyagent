# Repository Gap Analysis — macOS Agent lifecycle

Date: 2026-08-31
Scope: planning evidence only; no feature code changed.

## 0. Non-mutating evidence from the test Mac

A read-only inventory and signature check on 2026-08-31 found:

- `/Applications/OpenCode.app`
  - `CFBundleIdentifier`: `ai.opencode.desktop`
  - version: `1.18.19`
  - Developer ID Team ID: `5NZ4Q7NXJ4`
  - Gatekeeper result: accepted as a notarized Developer ID application
  - LaunchServices/Spotlight resolves this bundle identifier to the same application path
- `/Applications/ChatGPT.app`
  - `CFBundleIdentifier`: `com.openai.codex`
  - version: `26.825.51511`
  - Developer ID Team ID: `2DC432GLL2`
  - Gatekeeper result: accepted as a notarized Developer ID application

The evidence contains no user-home path or credentials. It confirms that the OpenCode scan failure is caused by FyAgent's current product/surface registry rather than macOS failing to register the app. It also confirms that this upgraded ChatGPT build still uses the existing Codex stable bundle identity; no speculative ChatGPT-wide alias should be added.

## 1. Existing architecture worth reusing

### Target authority

Archived tasks `08-29-agent-install-target-authority` and `08-29-macos-agent-in-place-update` established:

- installation inventory IDs;
- opaque target IDs and expected target revisions;
- multiple-install selection;
- macOS staging;
- exact-path update intent;
- backup/rollback;
- post-install authoritative readback.

This task should extend those contracts, not create a second target selector.

### Codex Desktop installer

The Codex installer already contains stronger lifecycle primitives than the generic Agent installer:

- monotonic job `sequence`;
- stable terminal snapshots;
- byte progress (`completedBytes`, `totalBytes`, `percent`);
- frontend download-speed sampling with a minimum one-second window;
- bytes/s display;
- source metadata and typed errors;
- package verification and platform adapters.

The generic path should extract/reuse these pieces where practical.

## 2. Confirmed gaps

### 2.1 `/Applications` is intentionally closed today

`src-tauri/src/agent_install/inventory.rs` exposes both macOS destinations, but:

- `~/Applications` is writable/eligible;
- `/Applications` is `requiresElevation`, not writable, not eligible;
- the system destination carries `authorization_required`.

`src-tauri/src/agent_install/macos.rs` mirrors this policy:

- fresh `MacUserApplications` deploys to `~/Applications`;
- fresh `MacSystemApplications` returns `authorization_required`;
- update of current-user app is supported;
- update of all-users app returns `authorization_required`.

Conclusion: installing into the user directory is not a frontend default bug. The privileged system-deployment executor is missing.

### 2.2 OpenCode Desktop cannot be discovered

`AgentCatalogId::OpenCode` is routed through CLI probing. The generic desktop registry currently contains WorkBuddy, Qoder Work and TRAE Work, but not OpenCode Desktop.

Conclusion: an installed `.app` is invisible by construction. Adding another path to CLI discovery would not solve the product-model problem; OpenCode needs separate CLI and Desktop surfaces.

### 2.3 Generic CLI lifecycle is synchronous and evidence-poor

`src-tauri/src/agent_install/mod.rs` delegates Claude Code, Grok Build and OpenCode CLI install/update to `run_cli_lifecycle` and returns an immediate result without a persistent job ID.

`src-tauri/src/services/tooling.rs` launches shell/package-manager commands and only receives bounded output after process exit. Generic failures are collapsed into a small reason set.

Conclusion: “检查来源 / 正在安装 / 然后没了” is structurally possible. The frontend has no intermediate snapshots and weak terminal evidence.

### 2.4 Generic Agent progress contract is incomplete

`AgentActionJobSnapshot` currently has job ID, action, stage, cancellability and reason code, but no:

- sequence;
- timestamps;
- completed/total bytes;
- percent;
- source attempt;
- diagnostic summary.

`useAgentLifecycleAction` declares `percent` but always returns `null`, then clears busy/stage/job after the operation.

Conclusion: the generic path cannot restore download speed until its contract carries progress and terminal snapshots.

### 2.5 Codex progress exists but is not consistently projected

`src/shared/codex-desktop/snapshots.ts` already computes bytes/s from accepted snapshots and rejects stale or invalid samples.

`CodexDesktopInstallerPanel.tsx` displays bytes and speed, but its progress bar rounds percent to an integer. `AgentDirectory.tsx` uses a reduced projection and can expose raw percent formatting.

Conclusion: extract one formatter/projection and use it in all surfaces; do not add another rate algorithm.

### 2.6 Codex install/update can trigger launch

`CodexDesktopService::run_install_flow` checks the local version before download. If it is equal to or newer than the selected release, it calls the platform launcher and settles the install job as “launched existing”.

Conclusion: install/update and launch are coupled at the service layer. This is concrete evidence relevant to the reported red warning, although the original warning text was not captured and its exact cause remains unproven.

### 2.7 Codex launch diagnostics are limited on macOS

The current macOS launch adapter revalidates the bundle and then runs the command-line `open` command. A zero process exit does not necessarily provide the same completion/error evidence as an `NSWorkspace` callback.

Conclusion: an explicit launch action should use a native adapter with asynchronous result and retain error evidence.

### 2.8 Codex filename migration is partly anticipated

The macOS bundle module explicitly notes that a stable package may be named `ChatGPT.app` while an older installation may be `Codex.app`; discovery uses `com.openai.codex` rather than filename.

Conclusion: preserve identity-based discovery. Verify the current official upgraded artifact before changing the allowlist, and do not match ChatGPT Classic by display name.

## 3. Risk assessment

### Highest risk: privileged installation

A helper able to write `/Applications` is security-sensitive. The safe boundary is not “run this command as admin”; it is “install this already verified product bundle into this compiled destination under this immutable job”.

### High risk: distribution-owner conversion

Updating a Homebrew/npm/native CLI with the wrong mechanism can create duplicate commands, PATH shadowing or permission problems. Owner must be discovered and bound to the update target.

### High risk: product/surface flattening

Treating OpenCode CLI and Desktop as one install state will keep producing false detection and ambiguous actions.

### Medium risk: source fallback

Fallback must not downgrade versions or replace official provenance with an unaudited mirror.

### Medium risk: progress UX

Incorrectly carrying speed across retries or presenting guessed totals is worse than an indeterminate state. Only measured bytes should be shown.

## 4. Recommended reuse map

| Need | Reuse |
| --- | --- |
| Target selection/revision | existing Agent installation inventory |
| macOS staging and rollback | existing `agent_install::macos` flow |
| Job sequencing | Codex `JobStore` semantics |
| Byte progress | Codex download progress DTO/bridge |
| Download speed | `src/shared/codex-desktop/snapshots.ts` |
| Source descriptors | Codex source/release descriptor pattern |
| Target picker | existing `LifecycleTargetPicker` |
| Error copy | existing reason-code projection |
| Desktop bundle validation | current Codex macOS bundle discipline, generalized per product |

## 5. Decisions carried into the task

1. One serial full-stack task.
2. macOS only.
3. OpenCode split into CLI/Desktop.
4. New managed desktop installs go to `/Applications`; existing installs update in place.
5. Use one system-commit port: prove Apple native authorization first, and adopt a narrow Blessed/SecureXPC helper only if the native route cannot satisfy fresh create and rollback.
6. Preserve Grok's detected distribution owner: native uses xAI/GCS; official npm is an explicit alternative, never an automatic native failure fallback.
7. Install/update and launch become separate actions.
8. Generalize existing progress/job capabilities rather than rebuilding them.
