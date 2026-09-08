# Validation Evidence

## Scope delivered

- Activated `ManagedAuthService` as the runtime owner for identities,
  Credential Sessions, SecretRef admission/recovery, refresh CAS, and
  fyagent-owned Proxy token resolution.
- Registered `services::secret` with `NativeSecretBackend` in the composition
  root. SQLite stores only metadata and opaque SecretRef/version.
- Migrated `codex_oauth_auth.json`, `xai_oauth_auth.json`, and Copilot v3 JSON
  into versioned OS-vault bundles with per-source journals. Finalize writes DB
  `completed` before renaming to `{filename}.managed-auth-v1.bak`.
- Sealed plaintext JSON writers only for sources that actually prepared or
  completed. Vault unavailable or Copilot v1 identity-less stores fail closed
  without destroying other sources.
- V2 overview/default/remove talk to the service. Login/session/connection
  actions remain closed `unavailable` for later children.
- Restored `tests/v2/pages/auth/Page.test.tsx` to the real ManagedAuthPort
  (WIP had rewritten it against a stale spec).
- Added `.trellis/spec/backend/managed-auth.md` and updated SecretRef,
  database persistence, proxy, Agent Auth, and V2 managed-auth contracts.

## Checks passed

```text
mise run env:check
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test -- managed_auth
mise run rust:test -- --test secret_service_contract   # non-ignored
mise run rust:test -- schema_zero_and_v19
mise run typecheck:v2
mise run test:v2 -- tests/v2/pages/auth/Page.test.tsx \
  tests/v2/features/managed-auth.test.ts \
  tests/v2/platform/managedAuthPort.test.ts
```

Native ignored HIL (`native_os_backend_crud_readback`) was not executed.
Unsigned `cargo test` missing Data Protection Keychain entitlement remains
expected fail-closed evidence, not product acceptance.

`mise run check:contracts` currently fails on an unrelated
`src-tauri/src/agent_install/desktop.rs` supported-platform drift that this
child did not change.

## Evidence boundary

This child does not claim OpenAI browser PKCE, Device Code login, Codex native
store projection, Grok managed auth, OpenCode Desktop provider auth, or
signed macOS/Windows SecretRef HIL. `copilot_get_token*` still returns a
token to leftover V1 Copilot UI. After a JSON store is sealed, new V1 Device
Code logins stay in manager memory until the OpenAI child writes them through
the vault.
