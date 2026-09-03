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
  xAI Device Code is owned by `providers/xai.rs`. Codex native file
  projection and Grok helper/`auth.json` writes stay fail-closed until
  matching-host HIL. OpenCode Desktop observation and `auth.json`
  projection live in `consumers/opencode.rs`; live Desktop pickup stays
  `pending_restart` until HIL.

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
  -> OpenAI (loopback + device) and xAI (device) snapshots after
     request validation; Copilot stays provider_not_supported
managed_auth_apply_connection_action
  -> Codex/Grok metadata with native projection fail-closed until HIL;
     OpenCode closed slots observe/project and may return pending_restart
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
  never refreshed by Proxy. OpenCode Path B projection writes an
  independent `purpose=opencode_provider` session into official
  `auth.json`, then sets `refresh_owner=opencode`. Codex/Proxy refresh
  lineages are never copied. Live Desktop hot-reload of an external
  write is unproven; successful FyAgent writes stay `pending_restart`.
- V1 `commands/auth.rs` may list vault accounts only after that provider's
  JSON store is sealed. Otherwise JSON remains the live compatibility path.
- Renderer DTOs, logs, and overview JSON must not contain tokens, SecretRef,
  `device_code`, verifier, or authorization codes.
- `copilot_get_token*` remains registered for leftover clients but always
  returns `copilot_token_not_exposed`. Do not add new token-returning
  commands. Proxy must resolve Copilot material natively, never through
  renderer IPC.

### Login sessions

- OpenAI browser loopback PKCE is the default. Ports are only the first-party
  registered `1455` then `1457`; unknown processes are never cancelled. Both
  busy falls back to Device Code.
- Device Code polling is backend-owned and uses the server interval. Cancel
  bumps generation so a late poll cannot save a credential.
- `reopen_login` re-opens the process-private official URL for a non-terminal
  session. The snapshot never includes the authorization URL, callback URL,
  code, state, or verifier.
- Success is published only after SecretRef + metadata readback. Codex native
  file projection stays closed without matching-host HIL (`partial` +
  `native_projection_unavailable`).

### xAI Device Code and Grok fail-closed consumer

- xAI login is Device Code only. Browser loopback is rejected at request
  validation. Discovery and token/device endpoints must be HTTPS
  `auth.x.ai:443` with empty userinfo. Polling is backend-owned and classifies
  `authorization_pending`, `slow_down` (interval +5s, capped),
  `access_denied`, and `expired_token`. Cancel bumps session generation so a
  late poll cannot save a credential.
- Migrated / Proxy xAI sessions use `purpose=proxy_upstream` and
  `refresh_owner=fyagent`. A Grok-purpose login creates a separate
  `purpose=grok_native` Credential Session with its own credential ID. While
  helper and file projection are disabled that session still uses
  `refresh_owner=fyagent` and stays in the OS vault; it is never Proxy-resolved
  and never copied into Grok `auth.json`.
- Production gates in `consumers/grok.rs` stay false until matching-host HIL:

```text
GROK_AUTH_PROVIDER_COMMAND_ENABLED = false
GROK_FILE_PROJECTION_PRODUCTION_ENABLED = false
```

- `project_grok_native` returns `Unsupported` and writes no vendor file.
  Connect or login-to-connect finishes `partial` +
  `native_projection_unavailable`. Overview Grok cards must not report
  `connected`.
- Windows `auth.json.lock` identity and `GROK_HOME` / multi-home targeting
  remain unproven. Do not enable file writes. External Grok refresh versus
  FyAgent generation reconcile is not a live path until a native write is
  HIL-proven; vault CAS still discards stale generations for fyagent-owned
  sessions.
- Agent Auth observation for Grok Build remains `handoff_only`. Vault
  `grok_native` metadata must not be painted as a verified Grok CLI/Desktop
  login. CLI install success is not login evidence.

### OpenCode Desktop `auth.json` consumer

Path and schema follow official OpenCode `Global.Path.data/auth.json`
(`consumers/opencode.rs` via `opencode_config::get_opencode_auth_json_path`).
This consumer is Desktop-first:

- Observation and Path B projection never require a PATH `opencode` CLI.
- The private Desktop sidecar (ephemeral loopback port and
  `OPENCODE_SERVER_PASSWORD`) is not a control plane. Do not probe, guess, or
  persist sidecar ports or passwords.
- Closed file keys are `openai`, `xai`, and `github-copilot`. Other keys stay
  in the raw object. Read-modify-write must preserve unknown providers,
  undecodable values, `wellknown` entries, and extra official fields on keys
  FyAgent is not replacing.
- Environment-variable / `OPENCODE_AUTH_CONTENT` providers are not `auth.json`
  rows. Projection must not invent or delete them.
- Writes use the existing atomic file owner, then `0600` on Unix, then
  byte-equal readback. Readback mismatch restores the exact preimage or
  deletes a newly created file. Success is never inferred from `atomic_write`
  returning Ok.
- Live Desktop pickup of an **external** write is unproven.
  `OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN` stays `false`. After a successful
  FyAgent write, connection `authStatus` is `pending_restart` with reason
  `pending_restart`. Do not also emit `native_projection_unavailable` (that
  reason means FyAgent could not write). Do not paint `connected` as hot.
- `connect_consumer` / reauthenticate with `consumer=opencode` creates a
  separate `purpose=opencode_provider` Credential Session. Proxy, Codex,
  Grok, and Copilot purposes are rejected as lineage copies. After a
  successful file readback, `refresh_owner` becomes `opencode`. A failed
  owner transfer still records the connection as `pending_restart` so the
  file write cannot look like a live Path A login.
- Copilot login remains `provider_not_supported`. Copilot v1 JSON without a
  stable identity stays `blocked`. Do not project that token into
  `github-copilot`.
- Matching-host Desktop HIL for connect, refresh, disconnect, external
  change, and restart is still required before flipping the hot-reload gate
  or claiming production live pickup.

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
| login for xAI Device Code | backend-owned snapshot; Grok native projection stays fail-closed |
| login for Copilot | `provider_not_supported` |
| Codex/Grok connect without HIL file projection | `partial` + `native_projection_unavailable`; no vendor auth.json write |
| Grok helper or `auth.json` write while production gates are false | `Unsupported` / `partial` + `native_projection_unavailable`; no vendor file |
| Proxy resolve of `purpose=grok_native` | conflict; no refresh |
| OpenCode Path B write while live Desktop hot-reload is unproven | file write + readback may succeed; status stays `pending_restart`; do not emit `native_projection_unavailable` |
| OpenCode login/connect with `consumer=opencode` | new `purpose=opencode_provider` session; never copy Proxy/Codex/Grok/Copilot refresh lineage |
| OpenCode connect using only `purpose=proxy_upstream`/`codex_native`/`copilot` | `provider_not_supported`; no `auth.json` write |
| OpenCode observe with Desktop data dir and no PATH CLI | sanitized provider list from `auth.json`; not `AuthObserverUnavailable` |
| OpenCode RMW of one closed key | unknown/undecodable/other provider keys unchanged |
| sidecar port/password supplied or scanned | reject; no network probe |
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
- **Good:** OpenCode `connect_consumer` login writes an independent
  `purpose=opencode_provider` oauth/api entry, transfers `refresh_owner` to
  `opencode`, and overview stays `pending_restart` until Desktop HIL proves
  live pickup.
- **Base:** missing `auth.json` is empty providers, not observer failure.
- **Bad:** write refresh tokens back to JSON after that source is sealed.
- **Bad:** seal every JSON store because one source failed.
- **Bad:** pre-mark credentials `Ready` in the parser before vault readback.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test -- managed_auth
mise run rust:test -- opencode
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
- OpenAI loopback callback host/path/state/PKCE, `1455`→`1457`→Device Code,
  cancel drops a late result, and reopen opens the official page without
  putting the authorize URL on the snapshot;
- xAI Device Code discovery allowlist, pending/`slow_down`/expiry/deny
  classification, cancel drops a late grant, and grok_native vs
  proxy_upstream isolation; Proxy resolver rejects grok purpose even with
  `refresh_owner=fyagent`;
- Grok helper and file-projection constants remain false;
  `project_grok_native` writes nothing;
- Codex file projection remains closed without HIL; connect/login-to-connect
  finishes `partial` with `native_projection_unavailable`;
- third-party Codex writers never overwrite official `auth.json`;
- OpenCode observation does not call PATH CLI; missing CLI is not
  `AuthObserverUnavailable`;
- OpenCode RMW preserves unknown/undecodable keys and `0600`; stale CAS
  leaves the file unchanged; readback mismatch restores the preimage;
- OpenAI/xAI `consumer=opencode` login creates `purpose=opencode_provider`
  and does not copy Proxy/Codex/Grok/Copilot lineage; successful writes stay
  `pending_restart` while `OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN` is
  false;
- Copilot v1 without identity stays blocked and is not projected;
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

Wrong:

```text
missing PATH opencode -> AuthObserverUnavailable
OpenCode connect uses proxy_upstream refresh token
auth.json write Ok -> authStatus=connected
```

Correct:

```text
read Global.Path.data/auth.json; missing file is empty providers
new purpose=opencode_provider session, then RMW + readback
OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN=false -> pending_restart
```
