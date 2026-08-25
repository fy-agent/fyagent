# Research — Local backend reuse inventory

Reviewed: 2026-08-25

## Conclusion

FyAgent already contains nearly every heavy mechanism required by this iteration. The task should be implemented as adapters and contract evolution, not a new installer/auth subsystem.

## Installation reuse

### Agent Catalog and readiness

- `src-tauri/src/services/external_agents/mod.rs` owns the closed Agent IDs and runtime capability vocabulary.
- `src-tauri/src/agent_install/mod.rs` is deliberately read-only today: no installer/executor/registry/network/filesystem. This is the correct façade to evolve because it already binds install readiness to canonical Agent IDs.
- `src-tauri/src/commands/agent_install_readiness.rs` exposes the narrow IPC.
- V2 `AgentInstallReadinessSection` already renders backend readiness.

Decision: evolve this façade; do not create another Agent install registry.

### CLI Tooling

`src-tauri/src/services/tooling.rs` and `services/tooling/{discovery,lifecycle,terminal,versions}.rs` already provide:

- bounded supported-tool selection;
- executable candidate discovery and canonicalization;
- multi-install reporting/default selection;
- version probing and latest-version lookup;
- package-manager-aware anchored updates;
- official installer + package-manager fallback construction;
- install/update lifecycle action execution;
- Windows shim handling and extensive unit tests.

Existing Tooling already knows Claude, Grok and OpenCode. The main stale fact is Claude's Windows native installation support. Codex CLI lifecycle is intentionally disabled and must not be enabled incidentally.

Decision: Agent Catalog maps to Tooling IDs; no copied detection/lifecycle code.

### Managed executable installer

`src-tauri/src/codex_desktop/` already contains source resolution, streaming download, cancellation, temp ownership, jobs, verify, Windows deployment/PackageBridge and macOS DMG transaction code. `services/codex_desktop` adds restart/lifecycle policy.

The current Trellis contract explicitly says this is the **repository-wide one-click executable software installer contract**, with Codex Desktop only the first implementation.

Decision: extract reusable seams from this implementation for Qoder/TRAE/WorkBuddy; never build a second desktop downloader/installer.

### Windows elevated boundary

`services/tooling.rs` deliberately fails closed in formal elevated Windows releases before inspecting/executing user CLI tools. `.trellis/spec/backend/windows-runtime-security.md` freezes the Explorer user's SID/Profile/PATH and forbids a generic command bridge.

Decision: current Tooling can be reused directly on macOS/non-formal Windows. Formal Windows automation needs a closed ordinary-user helper with authenticated control, or remains unavailable. Updating Claude's official Windows install command is not permission to remove this boundary.

## Auth reuse

### Unified Auth Center already exists

`src-tauri/src/commands/auth.rs` already normalizes three FyAgent-managed auth providers:

- `github_copilot`
- `codex_oauth`
- `xai_oauth`

It already exposes start/poll/list/status/remove/default/logout. Frontend API and `AuthCenterPanel` already exist.

Decision: extend/clean this seam; no second Auth Center.

### CodexOAuthManager is already multi-account

`src-tauri/src/proxy/providers/codex_oauth_auth.rs` already has:

- versioned on-disk store;
- `accounts: HashMap`;
- default account;
- refresh-token persistence;
- access-token memory cache;
- per-account refresh locks;
- add/remove/default/clear/status methods.

The defect is semantic: the current HashMap key is the ChatGPT account/workspace routing ID. Two users in the same Team/Business workspace can collide.

Decision: P1 is a schema/identity/concurrency correction, not a new manager.

### Generic Provider auth binding already exists

`src-tauri/src/provider.rs` already owns:

- `AuthBindingSource::{ProviderConfig, ManagedAccount}`
- `AuthBinding { authProvider, accountId }`
- `ProviderMeta.authBinding`
- `managed_account_id_for()`

Proxy/Provider code already consumes `codex_oauth` bindings.

Decision: keep this as the only Provider→credential reference; do not add `codexAccountId` fields or token-bearing provider state.

### Codex live write helpers already exist

`src-tauri/src/codex_config/storage.rs` owns Codex config/auth paths and atomic `auth.json + config.toml` projection/rollback. `codex_config/auth.rs` already understands login material, stale third-party API-key residue and official-switch cleanup.

Decision: preserve these paths for verified `file` credential-store mode and regression-test them. The missing functionality is effective credential-store awareness, not another auth-file writer.

## Frontend reuse

- Agent detail/readiness already exists in V2.
- Codex has a dedicated installer panel that can be retained while backend managed-package core is generalized.
- Auth Center already has managed-account interaction patterns.

Decision: UI should consume backend capabilities and jobs, not contain source/command policy.

