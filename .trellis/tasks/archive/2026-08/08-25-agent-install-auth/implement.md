# Implement

## Phase 0 — Freeze evidence and contracts

1. Re-verify current official install/auth surfaces and CC Switch upstream references recorded in `research/upstream-and-official-evidence.md`.
2. Re-verify the now-resolved first-party desktop source contracts:
   - QoderWork: official fixed `/releases/latest/` User-x64 / macOS ARM64 / macOS x64 aliases; remote semver remains unknown unless a newer first-party version endpoint is found.
   - TRAE Work: official latest API -> `data.solo` -> `region=cn`; current fixture `2.3.76922`.
   - WorkBuddy: official `/v2/update` closed platform IDs; current fixture `5.3.14.36279234`; macOS exact `.zip -> .dmg` transform matches the official frontend.
   Do not turn the observed current version values into production constants.
3. Update the planned contract deltas for `external-agent-p0`, executable installer, Windows runtime and Codex provider auth ownership before production writes are introduced.

Exit gate: every enabled action has a source/ownership authority; unknown remains disabled.

## Phase 1 — P0 Agent action façade and CLI reuse

1. Extend the existing `agent_install` DTO/commands with closed install/update/auth action states while preserving canonical Agent Catalog IDs.
2. Add catalog→Tooling adapters for Claude Code, Grok Build and OpenCode; reuse existing discovery/version/lifecycle functions.
3. Refresh Claude Windows install strategy from the current official native/WinGet contract without weakening formal elevated-Windows restrictions.
4. Add auth-ownership metadata and actions:
   - Claude official login/logout/status;
   - Grok official login/logout, status unknown unless a stable structured status command is verified;
   - OpenCode provider-connect action, never a global auth bool;
   - desktop app-owned login launch for Qoder/TRAE/WorkBuddy.
5. Reuse the current Auth Center for FyAgent-managed OAuth; add cancellation/provider-neutral DTO cleanup only where required.

Exit gate: no new generic command runner, no credentials parsed for agent-owned auth, existing Tooling tests remain green.

Regression gate: Gemini CLI / OpenClaw / Hermes and other existing Tooling consumers retain their current install/update behavior; no Catalog refactor is allowed to retire or silently reroute those product surfaces.

## Phase 2 — P0 Managed package generalization

1. Extract product-neutral seams from the Codex Desktop one-click installer while keeping Codex as golden regression fixture.
2. Introduce the three first-party source descriptors. Each owns fixed HTTPS hosts/redirect allowlist and explicit platform/architecture/package-format branches. TRAE/WorkBuddy emit a backend-generated opaque release/source revision derived from validated versioned resolver state. Qoder emits a versionless latest-source state and does not fabricate a remote version/revision from Last-Modified/ETag.
3. Reuse common job/single-flight/cancel/download/temp/platform-install/post-install verification.
4. Before creating a managed-package job, force-refresh/revalidate source authority. TRAE/WorkBuddy require the renderer-provided opaque `expectedReleaseId`/revision to match the current backend release; source drift returns refresh-required. Qoder revalidates the fixed latest alias and explicitly installs the object currently behind it; it never claims exact remote-version coherence that the vendor does not expose.
5. Add trusted runtime detection and launch adapter needed to prove installed/runnable after native installation.
6. Reuse the existing DMG/MSIX concrete installer only when the resolved artifact format matches. If an evidenced vendor uses EXE/NSIS/MSI/PKG, add a closed format adapter under the same core; do not route it through an arbitrary process API.
7. On API/alias/schema/allowlist/package-probe failure, surface source unavailable and keep the official product-page fallback. Never use the research-time TRAE/WorkBuddy concrete version URLs as stale fallback packages.

Exit gate: no arbitrary URL/path IPC; Codex installer regression suite unchanged; each enabled desktop action authoritatively rereads installed state.

## Phase 3 — P1 Codex managed-account schema and migration

1. Version the current Codex OAuth store and separate `credential_id` from ChatGPT workspace/account routing identity.
2. Add backup + idempotent v1→v2 migration and deterministic Provider `authBinding` remap.
3. Keep `CodexOAuthManager` as the only managed credential SSOT; provider rows store ID only.
4. Add same-workspace/multi-user collision fixtures based on CC Switch #5885.

Exit gate: multiple credentials from one workspace coexist; no provider token duplication.

## Phase 4 — P1 binding, concurrency and refresh correctness

1. Implement/default/test explicit bound and unbound/follow-native Provider semantics using existing `ProviderMeta.authBinding`.
2. Separate credential source from upstream destination in Provider/proxy decisions; add regression fixtures for custom endpoint + official/managed auth combinations.
3. Add login cancellation, bounded timeout and operation serialization; terminal paths release handles.
4. Before file-mode native projection, reconcile Codex-rotated refresh token only for the same positively identified credential.
5. Fail closed when a bound credential disappears/expires; never silently use another default account or failover queue entry.

Exit gate: concurrency/property tests prove no cross-account request or lost refresh update.

## Phase 5 — P1 Codex credential-store interoperability

1. Parse/evaluate `cli_auth_credentials_store` through a dedicated pure resolver; cover documented file/keyring/auto plus source-visible/future non-file, invalid, unset and current-version semantics with fixtures.
2. File mode: reuse existing atomic `auth.json + config.toml` live write/rollback and current provider/change-plan locks.
3. Any non-file/unknown store: keep native projection disabled unless an official stable Codex API/command was verified during Phase 0; do not implement OpenAI's private credential-store format.
4. Preserve stale third-party auth cleanup, official-auth preservation, Quick Setup targeted write and takeover regression suites.

Exit gate: no path assumes `auth.json` is authoritative when effective store is not file.

## Phase 6 — UI, documentation and verification

1. Make Agent detail actions render from backend capability/readiness state; never reconstruct URL/command policy in TS.
2. Auth Center shows managed credential identity, default state, bound Providers, re-auth state and native-projection availability without exposing secrets.
3. Distinguish “managed account available for FyAgent routing” from “currently active in native Codex”.
4. Update Trellis specs and relevant user docs/i18n.
5. Run focused and repository gates; record native HIL separately and do not upgrade unexecuted platforms to verified.

## Suggested parallelism

After Phase 0/contract freeze:

- Stream A: CLI Tooling + auth ownership adapters.
- Stream B: managed-package core + desktop source adapters.
- Stream C: Codex managed-account schema/migration.

Merge A/B/C before Phase 4 because Provider/account concurrency and UI consume all three contracts. Phase 5 follows the finalized Codex account identity model.

## Verification commands

Exact focused test names are finalized during implementation. Minimum gates:

```bash
mise run typecheck
mise run test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
mise run check
```

Do not treat these as Windows/macOS installer HIL evidence.

