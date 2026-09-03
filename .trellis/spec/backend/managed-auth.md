# Managed Auth Core, SecretRef Vault, and Legacy Migration Contract

## 1. Scope / Trigger

Read this contract before changing Managed Auth metadata, Credential Session
lifecycle, OS-native secret bundles, legacy JSON migration, Proxy token
resolution, or the thin Tauri façade in `commands/managed_auth.rs`.

Primary owners:

- `src-tauri/src/services/managed_auth/`
- `src-tauri/src/database/dao/managed_auth.rs`
- `src-tauri/src/commands/managed_auth.rs`

Related owners:

- [SecretRef Native Backend](./secretref-backend.md) owns the OS vault leaf.
- [Database Persistence](./database-persistence.md) owns schema/migration
  mechanics and the WebDAV skip/preserve sets.
- [V2 Managed Accounts](../frontend/v2-managed-auth.md) owns renderer Ports.
- [External Agent Auth](./external-agent-auth.md) remains the Agent-owned
  Claude/desktop handoff façade; it must not grow a second OAuth store.
- OpenAI browser loopback PKCE and Device Code are owned by
  `services/managed_auth/providers/openai.rs` plus backend login sessions.
  Codex native file projection stays fail-closed until matching-host HIL.
  xAI/Grok and OpenCode Desktop adapters remain later slices.

This is the first production consumer of `services::secret`. Do not introduce a
second keyring, a plaintext JSON token authority, or a `shared` refresh owner.

## 2. Signatures

Composition root (`lib.rs` setup):

```text
SecretService::new(NativeSecretBackend::new())
ManagedAuthService::new(db, secrets, config_dir)
ManagedAuthService::startup()
app.manage(ManagedAuthState(Arc<NativeManagedAuthService>))
```

Startup failure is fail-closed overview (`secret_unavailable` /
`migration_blocked`). It must not crash the app and must not fall back to a
file or environment secret store.

Current V2 commands:

```text
managed_auth_get_overview() -> ManagedAuthOverview
managed_auth_set_default_account({ accountId, expectedRevision })
  -> ManagedAuthMutationResult
managed_auth_preview_account_removal({ accountId, expectedRevision })
  -> ManagedAuthAccountRemovalPreview
managed_auth_remove_account({ previewId, accountId, expectedRevision })
  -> ManagedAuthMutationResult

managed_auth_start_login / get_login_session / cancel_login /
reopen_login / switch_login_method
  -> OpenAI snapshots after request validation; xAI/Copilot stay
     provider_not_supported
managed_auth_apply_connection_action
  -> Codex refresh/disconnect/connect metadata; projection remains
     native_projection_unavailable until HIL
```

`operationId` on mutation results is a canonical UUID v4 string (hyphenated).
Account IDs are `ma1:` + 32 lowercase hex. Credential IDs are `mcred1:` + 32
lowercase hex. Connection IDs are `mc1:` + 32 lowercase hex. Revisions are
`mr1:` + 64 lowercase hex.

SQLite metadata tables (schema v21):

```text
managed_auth_identities
managed_auth_credentials     // opaque secret_ref + secret_version only
managed_auth_defaults
managed_auth_connections
managed_auth_migrations
```

`refresh_owner` CHECK is `fyagent | codex_native | grok_native | opencode |
unavailable`. There is no `shared` value.

Secret bundle (`ManagedOAuthSecretBundleV1`) is one versioned OS-vault payload
per Credential Session:

```text
schemaVersion, credentialId, provider, generation,
accessToken?, refreshToken?, idToken?, tokenType?,
grantedScopes, issuedAt?, expiresAt?
```

At least one of access/refresh/id is required. Encoded UTF-8 JSON must fit
`MAX_SECRET_BYTES` (2560). Oversized material fails closed; it is never
truncated or spilled to a second file.

Proxy resolution:

```text
ManagedAuthService::resolve_access_material(provider, legacy_account_id?)
  -> AccessMaterial { access_token (zeroizing), routing_subject? }
```

OpenAI/xAI migrated sessions use `purpose=proxy_upstream`. Copilot uses
`purpose=copilot`. Both require `refresh_owner=fyagent`. The resolver never
returns a refresh token or SecretRef.

## 3. Contracts

### Identity versus credential session

- `ManagedIdentity` is keyed by `(provider, provider_subject, provider_tenant)`,
  not by email.
- `CredentialSession` is one OAuth/API grant and one rotating refresh-token
  lineage. UI may aggregate sessions onto one account card; backend must not
  merge refresh tokens across consumers.
- Default refresh owner for migrated Proxy-purpose sessions is `fyagent`.

### Secret write admission

Create/migrate a session in this order:

```text
reserve SecretHandle
INSERT credentials status=provisioning with that handle
create_reserved on the OS backend
typed decode readback of ManagedOAuthSecretBundleV1
optional set_default
mark_ready (Ready or RequiresReauth only after readback)
```

Production code must not call `SecretService::create()` for Managed Auth: that
API generates a SecretRef that is not yet durable in SQLite. A native create
that succeeds without authoritative readback keeps the DB SecretRef
(`provisioning` / `secret_missing`) and does not blindly delete the native
item.

Refresh:

```text
per-credential lock
read metadata + bundle + generation + owner
reject unless refresh_owner=fyagent
HTTP refresh through the existing Codex/xAI/Copilot protocol helpers
re-check generation
replace_reserved + update_secret_cas
CAS false => discard the network result; do not overwrite
```

Delete order: clear connections → delete SecretRef → delete credential /
identity metadata. SQLite `ON DELETE CASCADE` is not a license to skip the
native delete.

### Legacy JSON migration

Sources, one journal row each:

```text
legacy-codex-oauth-v2   <- codex_oauth_auth.json
legacy-xai-oauth-v1     <- xai_oauth_auth.json
legacy-copilot-auth-v3  <- copilot_auth.json
```

- Parse is strict and bounded. Copilot v1 (token without stable identity)
  stays `blocked`; the source file is left in place.
- Parser output is never `Ready`. Ready happens only after vault readback.
- Sources are isolated: one blocked source must not prevent another source
  from preparing/finalizing.
- Empty JSON stores do not create a journal row and do not seal future writes.
- Finalize: recover + typed readback of every credential for that source →
  persist `completed` + `backup_name` → rename to
  `{filename}.managed-auth-v1.bak`. If the live file is already gone and the
  backup hash matches, treat as completed. Never delete the backup in this
  slice.
- `legacy_store_sealed(id)` is true only for that source's
  `prepared|completed` journal. Vault unavailable or a foreign source failure
  must not seal remaining JSON stores.
- After a source is sealed, the matching manager `seal_json_store()` and must
  not write new tokens to plaintext JSON. Unsealed sources remain readable
  JSON until they migrate.

### Proxy and compatibility commands

- `proxy/forwarder.rs` prefers `resolve_access_material` when a fyagent-owned
  vault session exists. Unmigrated / blocked sources may still use the old
  manager path.
- Native-owned sessions (`codex_native` / `grok_native` / `opencode`) are
  never refreshed by Proxy.
- V1 `commands/auth.rs` may list vault accounts only after that provider's
  JSON store is sealed. Otherwise JSON remains the live compatibility path.
- Renderer DTOs, logs, and overview JSON must not contain tokens, SecretRef,
  `device_code`, verifier, or authorization codes.
- `copilot_get_token*` still returns a token string to the leftover V1 Copilot
  UI. Do not add new token-returning commands. Later hardening should retire
  that IPC.

### Sync and export

`managed_auth_*` tables are local-only in the WebDAV skip/preserve sets.
Opaque SecretRef values are device-bound; syncing them to another machine
must not be treated as a portable login. Full SQL export may include
metadata and must never include token columns.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| unsupported host / locked / denied vault | fail-closed overview; old JSON not renamed |
| reserved SecretRef not yet in SQLite | reject; do not call native create |
| create/replace readback or typed decode fails | keep provisioning/secret_missing; no plaintext fallback |
| bundle exceeds 2560 bytes or contains NUL | reject before native write |
| refresh_owner is not `fyagent` | resolver conflict; no refresh |
| CAS generation/owner mismatch | discard late result; store unchanged |
| Copilot v1 JSON without identity | that source `blocked`; other sources continue |
| migration hash changes after prepare | stale/blocked; do not rename |
| finalize rename fails after DB completed | retry rename on next startup; JSON is not writable authority |
| login/session command for OpenAI | backend-owned snapshot; success only after SecretRef readback |
| login for xAI/Copilot | `provider_not_supported` |
| Codex connect without HIL file projection | `partial` + `native_projection_unavailable`; no auth.json write |
| mutation `operationId` is not UUID v4 | frontend parser rejects the result |
| DTO/log/debug contains token/secretRef | test failure / NO-GO |
| `shared` refresh owner in schema or enum | reject implementation |

## 5. Good / Base / Bad Cases

- **Good:** Codex JSON migrates to SecretRef, overview shows the account
  without tokens, Proxy resolves access through the vault, and the JSON file
  is renamed to a bounded `.bak`.
- **Good:** Copilot v1 is blocked while a valid Codex store still finalizes.
- **Base:** keychain/Credential Manager unavailable. Overview reports
  `secret_unavailable`; plaintext JSON remains the live store.
- **Base:** OpenAI login succeeds only after SecretRef + metadata readback.
  Connecting Codex without HIL-proven file projection ends `partial` with
  `native_projection_unavailable`. Migrated accounts remain visible.
- **Bad:** write refresh tokens back to JSON after that source is sealed.
- **Bad:** seal every JSON store because one source failed.
- **Bad:** pre-mark credentials `Ready` in the parser before vault readback.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test -- managed_auth
mise run rust:test -- --test secret_service_contract
mise run typecheck:v2
```

Required assertions:

- schema v20→v21 creates the five tables; future `SCHEMA_VERSION+1` fails
  closed; CHECK excludes `shared`; no token columns;
- WebDAV skip/preserve include all `managed_auth_*` tables together;
- bundle 7.2 round-trip, wrong schema rejected, oversized fail-closed,
  Debug redaction;
- admission: SecretRef row exists before native create; recover after
  create-without-mark_ready;
- migration idempotent prepare/finalize; failure leaves the source file;
  Copilot v1 does not block Codex finalize;
- vault unavailable does not seal JSON;
- stale generation cannot overwrite; resolver rejects native refresh owners;
- overview/DTO leak scan includes `access_token`, `refresh_token`, `id_token`,
  `authorization_code`, `device_code`, `secretRef` / `secret_ref`, `verifier`;
- unsigned `cargo test` DPK `errSecMissingEntitlement` is fail-closed
  evidence, not product acceptance. Matching-host HIL remains `#[ignore]`
  until a signed app with `HY446996QX.com.fyagent.desktop` access-group
  evidence runs.

## 7. Wrong vs Correct

Wrong:

```text
SecretService::create(material)
  -> native item exists
  -> Err(readback) drops the SecretRef
  -> unreachable keychain item
```

Correct:

```text
handle = reserve()
INSERT provisioning(handle)
create_reserved(handle, material)
typed decode readback
mark_ready
```

Wrong:

```text
any migration error -> seal_json_store() on Codex, xAI, and Copilot
parser status = Ready
rename JSON then write DB completed
```

Correct:

```text
per-source journal
Ready only after typed readback
DB completed + backup_name, then rename
seal only that source
```
