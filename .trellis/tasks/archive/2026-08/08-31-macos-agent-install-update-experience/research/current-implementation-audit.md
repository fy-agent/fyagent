# Current Implementation Audit — macOS Agent lifecycle

Date: 2026-08-31
Method: repository inspection plus non-mutating local probes. No feature code was changed.

## 1. Generic desktop artifact flow buffers the full DMG

`src-tauri/src/agent_install/fetch.rs` currently exposes `fetch_artifact_bytes(...) -> Result<Vec<u8>, ...>` and `collect_body(...) -> Result<Vec<u8>, ...>`. The response body is accumulated in memory up to the two-gigabyte artifact cap.

`src-tauri/src/agent_install/macos.rs` then receives those bytes, creates a second job-owned `installer.dmg`, and writes the full buffer with `write_all` before mounting.

Implication: the generic Agent path has memory proportional to artifact size and duplicates the artifact handoff shape. Codex already owns a streamed `.part`/finalized-file path with cancellation, bounded progress and cleanup. The task must extract or narrowly adapt that owner rather than implement another downloader.

## 2. macOS desktop bundle metadata is parsed as XML text

`src-tauri/src/agent_install/desktop.rs::plist_string` reads `Contents/Info.plist`, converts the bytes with `String::from_utf8_lossy`, searches for `<key>...</key>`, then expects a following `<string>` element.

Implication: binary plists and valid XML variations are not handled authoritatively. The generic adapter should reuse the Codex bounded `plutil -> JSON -> typed fields` owner rather than expand the handwritten parser or add another plist/native bridge.

## 3. OpenCode Desktop is absent from the managed desktop registry

The existing desktop registry covers QoderWork, TRAE Work and WorkBuddy, while `AgentCatalogId::OpenCode` is routed through CLI/tooling observation.

A read-only local probe found:

- `/Applications/OpenCode.app`
- `CFBundleIdentifier = ai.opencode.desktop`
- version `1.18.19`
- Developer ID Team ID `5NZ4Q7NXJ4`
- Gatekeeper accepted it as a notarized Developer ID application
- LaunchServices/Spotlight resolved `ai.opencode.desktop` to that app

Implication: macOS has registered the application correctly; FyAgent cannot detect it because its current product/installation model has no OpenCode desktop surface.

Team ID and Gatekeeper results are research/HIL evidence only. Under the current executable-installer specification they are not new downloaded-content admission comparisons.

## 4. System destination is visible but deliberately non-executable

`src-tauri/src/agent_install/inventory.rs` projects both macOS destination scopes, but `/Applications` is marked as requiring elevation and ineligible while `~/Applications` is the executable fresh-install destination.

`src-tauri/src/agent_install/macos.rs` supports current-user fresh install and current-user exact-path update, but returns `authorization_required` for fresh system install and all-users update.

Implication: the incorrect destination is not only frontend copy. A real macOS authorization/commit backend is missing and must be proven with a signed package before `/Applications` can become executable.

## 5. Codex update and launch are coupled

`src-tauri/src/services/codex_desktop/mod.rs::run_install_flow` re-inspects the local app before download. When the installed platform version is equal to or newer than the selected release, it invokes the platform launcher and returns `LaunchedExisting`.

The existing `platform::process_launch` macOS adapter currently delegates to the command-line `open` tool after bundle revalidation.

Implication: “update” has a concrete launch side effect. This is relevant to the reported red warning, but the warning text was not captured, so its exact cause remains unknown. The task must remove the implicit launch and add explicit result-bearing launch diagnostics rather than claim a specific root cause.

## 6. Generic action jobs cannot preserve progress or failure evidence

The generic `AgentActionJobSnapshot` currently lacks a monotonic sequence, timestamps, completed/total bytes, source attempt and bounded diagnostic summary. `useAgentLifecycleAction` exposes `percent` as `null` and clears transient lifecycle state after completion.

Implication: the observed Grok sequence “checking source → installing → nothing” can occur even when the backend command failed. A persistent terminal snapshot and shared progress contract are prerequisites for fixing the UX.

## 7. Grok installation owner on the test Mac

A read-only probe found:

- executable under `$HOME/.grok/bin/grok`;
- reported version `1.0.5` (`5115b46bc909`);
- `$HOME/.grok/config.toml` contains `[cli] installer = "internal"`.

Implication: an update failure must preserve the vendor internal owner and layout. FyAgent must not silently convert this installation to npm. The official installer/update path should remain the first owner; npm is not a transparent replacement for an existing internal installation.

## 8. Codex/new ChatGPT local identity

A read-only local probe found `/Applications/ChatGPT.app` with:

- `CFBundleIdentifier = com.openai.codex`;
- version `26.825.51511`;
- Developer ID Team ID `2DC432GLL2`;
- Gatekeeper acceptance as a notarized Developer ID application.

Implication: the observed upgraded app still uses the existing Codex stable bundle identity. No broad `ChatGPT` display-name alias is justified, and ChatGPT Classic coexistence must stay fail-closed.

## 9. Decisions supported by this audit

1. Preserve the exact seven-item Agent catalog and add a closed install-surface dimension beneath it.
2. Reuse Codex streaming/job/progress and managed-DMG owners.
3. Replace handwritten plist parsing with the shared Codex structured reader, and replace command-line `open` inside the existing process-launch owner with a native completion adapter.
4. Prove the `/Applications` authorization backend in a signed build before making the destination eligible.
5. Keep Grok’s detected distribution owner stable and retain terminal errors.
6. Keep install/update separate from explicit launch.
