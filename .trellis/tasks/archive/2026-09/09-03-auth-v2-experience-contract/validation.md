# Validation Evidence

## Scope delivered

- Added the V2 `/auth` primary route and the central Accounts / Software Connections experience.
- Added the strict `ManagedAuthPort` v1 contract, unknown-response parsers, request admission and Tauri/browser adapters.
- Separated account identity, software connection, current request source and renewal owner in the UI.
- Added backend-owned login-session recovery/polling UI, device-code interaction, account-removal impact preview and connection confirmations.
- Routed Codex, Grok Build and OpenCode Agent panels to the central page while retaining Claude and desktop handoff sessions.
- Registered a credential-free native command façade that remains fail-closed until later child tasks attach the production service.
- Added the owning frontend Spec and updated the seven-route navigation and Agent Auth contracts.

## Checks passed

```text
mise run env:check
mise run typecheck:v2
mise run lint:v2
mise run test:v2                     # 490 tests
mise run test:v2:browser             # 160 tests across three viewports
mise run build:renderer
mise run check:frontend              # 1,573 unit tests + i18n + desktop mock/preflight
mise run rust:fmt
mise run rust:check
cargo test --locked --manifest-path src-tauri/Cargo.toml services::managed_auth -- --nocapture
mise run supported-platform:check
mise run check:contracts
```

The full frontend/browser/build/Rust/contract sequence above was rerun from the
isolated worktree after the final review. The rerun completed with 490 V2 unit
tests, 160 browser tests, and the repository release-contract suite passing.

## Evidence boundary

This child deliberately does not claim production OAuth, OS credential storage, token refresh or vendor-file projection. The native façade returns the closed `native_projection_unavailable` state until the backend child tasks replace that implementation behind the same contract. Browser fixtures are synthetic and credential-free.
