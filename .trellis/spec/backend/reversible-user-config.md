# Reversible User Configuration Writes

## 1. Scope / Trigger

Read before adding or changing a write/delete of a user's configuration or
credential file, before bypassing the common file writer, or before changing
file-impact disclosure and recovery IPC. Mechanism owners are
`src-tauri/src/config.rs` and `config/recovery.rs`; the closed recovery facade
is `services/config/recovery.rs`. Domain locks, parsers, permissions and
multi-resource compensation remain with the originating service.

## 2. Signatures

```text
config::atomic_write(path, bytes) -> Result<(), AppError>
config::atomic_write_private(path, bytes) -> Result<(), AppError>
config::delete_file(path) -> Result<(), AppError>
config::file_mutation_scope() -> synchronous, non-Send RAII guard
config::file_write_target(path) -> { path, backupPath, exists }
config::file_recovery(path) -> optional native recovery receipt
config::restore_file_recovery(path, receiptId) -> Result<(), AppError>

get_config_file_recoveries({ targets }) -> ConfigFileRecoverySnapshot[]
restore_config_file_recovery({ request: { target, receiptId } })
  -> ConfigFileRecoverySnapshot | ConfigFileRecoveryError
```

Both commands run blocking work off the Tauri command thread. The recovery
permission is `allow-user-config-recovery`. Targets are the closed enum
`claude_settings | claude_mcp | codex_auth | codex_config | codex_catalog |
grok_config | opencode_config | opencode_auth`; paths are resolved natively.
Lists contain 1–8 distinct targets. Receipts are canonical lowercase UUID v4.
Requests reject unknown fields and never accept a path, bytes or hash.

Snapshot v1 has exactly `contractVersion`, `target`, `writeTarget`, `state`,
`receiptId`, `restoresExistingFile`. States are `available`, `none`,
`manual_backup`, `conflict`, `unavailable`. Only `available` carries a receipt
and boolean restoration kind. Error v1 is `{ contractVersion, code }` with
`invalid_request | unavailable | external_change | recovery_required`.

## 3. Contracts

### Backup is the default write boundary

- `atomic_write`, private writes, JSON/text writers and `delete_file` route
  through the same serialized recovery owner. A caller cannot opt out via IPC.
- A changed existing file receives one adjacent `<filename>.fyagent.backup`
  containing its exact preimage. A no-op does not rotate an earlier useful
  backup. A first creation has no fabricated empty backup; undo removes the
  new file. A deletion also retains its preimage.
- The adjacent `<filename>.fyagent.undo.json` contains only schema version,
  UUID, path binding and pre/postimage digests. It is a private local recovery
  record, not a credential payload or renderer API. Backup and receipt must be
  persisted before the primary mutation. Backup/permission/receipt failure
  authorizes no primary write.
- Same-directory exclusive temporary creation, flush/sync and native atomic
  replacement avoid half-written primary files. Unix private writes/backups
  are `0600` before publication. Windows uses `ReplaceFileW`, never a
  delete-primary fallback. Refuse symlink/non-regular leaves and unreadable or
  oversized inputs. Recovery reads are bounded to 64 MiB and markers to 4 KiB.
- Recheck the source before committing. On a failed primary write, compensate
  only while the live bytes are still this operation's expected state; retain
  the backup and report uncertainty when ownership cannot be established.
  This is a per-file atomicity/compensation contract, not filesystem-wide ACID,
  universal cross-process locking, or a guarantee against every external race.

### One user operation may perform multiple internal writes

Provider mutation/result, source switch and Quick Setup entrypoints create a
`file_mutation_scope`. A nested synchronous scope shares the outer operation.
For repeated writes to one path, the first preimage is retained and the receipt
tracks the final postimage. This prevents a later MCP reconciliation from
replacing the useful backup with an intermediate model configuration. A
receipt/source mismatch aborts rather than treating an external write as an
internal continuation. The guard is non-Send and must not cross an await or
thread handoff. This grouping does not make several files or database rows one
atomic transaction.

### Explicit exceptions are not alternate normal writers

`atomic_write_unbacked` is private. Crate-scoped `write_backup_file` writes a
backup/export artifact without recursively backing up the backup;
`restore_file_preimage` is for domain compensation without rotating recovery
history to failed intermediate bytes. Architecture tests enumerate their
owners. A new bypass caller requires review and a corresponding negative
boundary test; ordinary config writes never use these helpers.

WorkBuddy and Qoder protected native writers retain their existing focused
transaction/backup contracts. This mechanism is not permission to replace
their handle/ACL/path protections with a weaker generic writer.

### Disclosure and recovery

- Before a user-confirmed mutation, expose the actual write/create paths,
  backup paths, unchanged paths where relevant, and how recovery works.
  Native-owned display metadata is the narrow path-disclosure exception:
  paths under the frozen home use `~`; overrides outside it remain truthful.
  No tokens, contents, secret locators or digests cross the boundary.
- Recovery validates receipt/path/current postimage and backup preimage, then
  restores exact bytes or removes a newly-created file. It survives process
  restart. Stale receipts, changed primaries or changed backups fail closed.
  The existing Codex/OpenCode auth locks and Provider lock are reused.
- Historical backups without receipts remain `manual_backup`; the UI must
  not turn them into an unverified automatic overwrite capability.
- File recovery does not remove saved accounts/Provider rows or revoke server
  grants. Reread affected domain state after restoration and explain that the
  user may need to reopen the external software. A successful disk write is
  not proof of external software pickup.

## 4. Validation & Error Matrix

| Condition                                                        | Required result                                                      |
| ---------------------------------------------------------------- | -------------------------------------------------------------------- |
| Backup or marker cannot be written                               | Primary remains unchanged; no success                                |
| Source bytes already equal desired bytes                         | No backup/receipt rotation                                           |
| First creation                                                   | No fake original backup; receipt authorizes guarded deletion on undo |
| Later internal rewrite in one scope                              | Preserve the operation's first preimage                              |
| User/external write after the operation                          | `conflict`; do not overwrite it                                      |
| Receipt expired by a newer write, malformed or from another path | Reject                                                               |
| Backup changed, symlinked or unreadable                          | No automatic restore                                                 |
| Domain compensation cannot confirm ownership                     | Keep recovery evidence and surface uncertainty                       |
| User provides arbitrary path through IPC                         | Reject before filesystem access                                      |
| Restoring a file succeeds but metadata/readback fails            | Do not claim the entire application transaction rolled back          |

## 5. Good / Base / Bad Cases

Good: source switch plus MCP synchronization leaves one exact pre-operation
config backup. Base: first creation can be undone by guarded deletion. Bad:
backing up after truncation, ignoring backup failure, unconditionally restoring
over newer external bytes, or presenting OAuth logout as reversible file undo.

## 6. Tests Required

Run `mise run rust:test`, `mise run rust:clippy`, `mise run test:unit --
tests/architecture/rustModuleBoundaries.test.ts`, `mise run typecheck:v2` and
`mise run test:v2`. Core tests cover exact bytes, backup failure, no-op history,
creation/deletion, stale receipts, corruption, symlink/permissions, failed-write
compensation and scoped rewrites. Facade/parser/UI tests cover closed targets,
path metadata without credentials, cancel-before-write, one-shot confirmation,
conflict refusal and explicit deletion copy. Native Windows evidence remains
separate from portable tests.

## 7. Wrong vs Correct

```text
wrong: fs::write(user_config, replacement); backup_later()
wrong: reset live file from any existing *.backup without revision checks
correct: validate -> exact preimage backup + receipt -> atomic replacement
         -> readback -> guarded restore using the matching receipt
```
