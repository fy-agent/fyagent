# Provider Credential Persistence

## 1. Scope / Trigger

Read before changing Codex API-key Provider persistence, migration, native
resolution, export/import or deletion. This owner reuses [SecretRef](./secretref-backend.md)
and the [Codex writer](./codex-provider-configuration.md); it does not implement
another vault, change proxy recovery, or implement hardware support.

## 2. Signatures

- `Database::save_provider(app, &Provider)` is a compatibility facade implemented
  in `services/provider/credentials.rs`. `save_provider_record` is crate-private
  SQL only; production callers must use the facade.
- `Database::update_provider_settings_config` is the same facade for backfill.
- `ProviderCredentials::{merge_edit,resolve,migrate_legacy,cleanup,settle}` are
  internal native operations. `resolve` is only for writers/proxy/usage consumers.
- Schema v24 adds local-only `provider_credentials(credential_id PRIMARY KEY,
provider_id, secret_ref UNIQUE, secret_version, status)`. Status is constrained
  to `pending|ready|revoked|deleted`. There is no Provider FK because durable
  admission precedes Provider creation and cleanup can outlive Provider deletion.
- Provider JSON stores only `settingsConfig.credentialRef = pc_<random UUID>`.
  Each replacement reserves a new SecretRef/version and credential ID.

## 3. Contracts

- Database composition injects the existing NativeSecretBackend. Explicit
  `Database::memory()` fixtures inject MemorySecretBackend; a native failure
  never causes a backend fallback.
- Native I/O never runs under the SQLite connection mutex. A separate credential
  mutation guard serializes save/cleanup/SQL import. DAO modules remain SQL-only.
- Save: admit pending handle, create native value, verify through constant-time
  readback, mark ready, persist reference-only Provider and read back the row.
  Persistence failure restores the previous row without erasing its old key.
- Pending creates are not blindly deleted. Retry with freshly captured matching
  material proves ownership and resumes the admitted handle. Missing pending
  items become deleted. Uncertain ownership remains pending for later recovery.
- Migration is per Provider and never writes external files. Failure preserves
  original plaintext exactly; legacy unbound rows remain readable. Conflicting
  or extra unsupported inline credentials defer migration rather than guessing.
- Blank, `[REDACTED]` or all-star/bullet edits retain the same Provider binding.
  Fresh input can repair a missing backend item. Locked/denied/unavailable remains
  fail closed. A bound read never falls back to inline/env/live values.
- Resolve checks exact current Provider binding, ready status and ownership.
  Rotation/deletion invalidates old bindings immediately; backend deletion is
  deferred until enclosing rollback can no longer restore the old Provider.
- Change Plan provider-definition digest binds the opaque credential ID, never
  value-derived hashes. Rotation makes an existing plan stale before the writer.
- Codex create/edit/switch resolve at the existing config-only writer seam;
  `auth.json` remains consumer-owned. External `config.toml` requires plaintext;
  save disclosure names the file and explains rolling backups may contain keys.
- Provider Debug and QuickSetup errors are source-free. Renderer Codex list
  projections omit credentials/references. The ordinary SQL exporter uses a
  pure connection/model whitelist for every Provider app, drops free-form
  auth/headers/scripts/unknown scalar extensions, endpoint history and full
  profile/universal/common snapshots, and resets exported current/failover flags.
  Private binary backup remains the lossless recovery format.
- SQL import preserves device-local credential ledger and full local Provider
  routes which contain credentials; it never attaches a local key to imported
  remote endpoint data. Explicit rotation and removal remain Provider operations.

## 4. Validation & Error Matrix

| Condition                                       | Result                                                    |
| ----------------------------------------------- | --------------------------------------------------------- |
| Foreign/malformed binding                       | `provider_secret_invalid`; no native read/cutover         |
| Backend missing/locked/denied/unavailable       | bounded `provider_secret_*` code                          |
| Reference no longer current or revoked          | `provider_secret_revoked`                                 |
| Native verification mismatch                    | `provider_secret_operation_failed`; old row retained      |
| DB save fails, previous row restored            | `provider_secret_persistence_failed`                      |
| DB compensation cannot be verified              | `provider_secret_recovery_required`                       |
| Backend delete fails                            | reference revoked locally; durable cleanup retry retained |
| Ordinary export cannot safely preserve identity | bounded export error; no output                           |

## 5. Good / Base / Bad Cases

- Good: first Codex save leaves only a reference in SQLite, and temporary live
  config contains the exact fixture key without modifying login auth bytes.
- Base: leave key blank, change model, keep the credential ID and update live model.
- Bad: swap another Provider's credential ID or add a shadow plaintext value;
  no authority is gained and missing/locked resolution does not fall back.
- Bad: interrupted migration leaves the original usable row and a pending native
  admission; a matching retry resumes it instead of allocating another item.

## 6. Tests Required

`services::provider::credentials` covers reference save/read, masks, rotation,
rollback, pending-create retry, conflicts, missing/locked/foreign refs, deletion
retry, full temporary-file create/edit/switch/delete and export/import negatives.
Schema fixtures cover fresh/v23/failed migration. Change Plan rotation tests assert
stale and zero writer calls. Renderer tests cover existing-key blank edits and
plaintext disclosure. Fixture checks do not prove OS-keychain HIL, Windows UAT,
external Codex process adoption, or release acceptance.

## 7. Wrong vs Correct

Wrong: hold a SQLite mutex while querying keyring, overwrite legacy plaintext
before native readback, serialize a resolved Provider to IPC, or copy local keys
onto remotely imported routes.

Correct: prepare outside the DB lock, admit then verify then cut over, resolve
only for native consumers, project safe DTOs, and preserve exact local bindings.
