# Database Persistence Contract

## 1. Scope / Trigger

Read this contract before changing SQLite initialization, `SCHEMA_VERSION`,
schema creation or migration, a DAO write transaction, JSON-to-SQLite import,
SQL export/import, binary backup/restore, local-only sync tables, or database
maintenance. The implementation owners are `src-tauri/src/database/` and its
`dao/` modules.

This contract owns persistence mechanics. A domain Spec still owns the meaning
of its rows and fields; for example, Change Plan state is defined by
[Change Plan Typed Executor](./change-plan-executor.md), while this contract
defines how its schema and transactions remain durable.

## 2. Signatures

The current storage authority is:

```text
application database: get_app_config_dir()/fyagent.db
schema authority:      src-tauri/src/database/schema.rs
schema version:        Database::SCHEMA_VERSION (source-owned current value)
connection owner:      Database { conn: Mutex<rusqlite::Connection> }
```

Important entry points are:

```text
Database::init() -> Result<Database, AppError>
Database::memory() -> Result<Database, AppError>
Database::set_change_listener(listener: impl Fn(&str) + Send + 'static)
  -> Result<(), AppError> // crate-private; composition-root wiring
Database::stored_user_version_exceeds_supported(path)
  -> Result<Option<i32>, AppError>

Database::migrate_from_json(config) -> Result<(), AppError>
Database::migrate_from_json_dry_run(config) -> Result<(), AppError>

Database::export_sql_string() -> Result<String, AppError>
Database::export_sql_string_for_sync() -> Result<String, AppError>
Database::import_sql_string(sql) -> Result<String, AppError>
Database::import_sql_string_for_sync(sql) -> Result<String, AppError>

Database::backup_database_file() -> Result<Option<PathBuf>, AppError>
Database::list_backups() -> Result<Vec<BackupEntry>, AppError>
Database::restore_from_backup(filename) -> Result<String, AppError>
Database::rename_backup(oldFilename, newName) -> Result<String, AppError>
Database::delete_backup(filename) -> Result<(), AppError>
```

DAO methods are exposed through `impl Database`; callers do not take the raw
connection or create a second connection owner to bypass transactions,
constraints, hooks, or error mapping.

## 3. Contracts

### Initialization and connection ownership

- `Database::init` creates the application directory, opens only
  `fyagent.db`, enables `PRAGMA foreign_keys = ON`, creates the current table
  set, then applies ordered migrations. It does not install a global service
  listener. The production composition root injects its change listener before
  starting automatic sync workers; registration failure aborts before spawning.
- A brand-new file selects incremental auto-vacuum before tables are created.
  An existing non-incremental database may be backed up and rebuilt with
  `VACUUM`; failure to establish the requested mode is logged as maintenance
  degradation rather than reinterpreted as a successful rebuild.
- `Database::memory` provides the production schema and required seeds in an
  in-memory connection for tests. It must not silently omit constraints that
  production DAO code relies on. It has no process-global cloud notification
  side effects; tests that observe writes explicitly inject a listener.
- Mutex poisoning and SQLite failures become `AppError`; production code must
  not use `unwrap` to acquire the shared connection or serialize persisted
  JSON.

### Provider credential export boundary

[Provider Credential Persistence](./provider-credentials.md) owns schema v24,
reference-only Codex saves and migration. Ordinary SQL export is a portable
connection/model projection, not a lossless credential backup; it omits full
Provider snapshots and private fields. SQL import preserves local credential
bindings together with their full route. Private binary backups stay lossless.

### Schema and migration

- `SCHEMA_VERSION` and `PRAGMA user_version` move together. Every schema
  change adds an explicit forward migration and fixtures for both a fresh
  database and the oldest affected predecessor shape.
- The numeric current version lives only in the Rust authority. Specs, renderer
  code, task runners, and tests compare against that owner or assert the
  expected terminal behavior; they do not maintain a second “current version”
  constant that can drift.
- Migration is ordered and forward-only. A stored version newer than the
  binary supports fails closed with an upgrade-required database error; it is
  never downgraded, recreated, or opened as if current.
- Schema migration uses the migration savepoint/rollback path. An unknown
  predecessor version, failed DDL, failed data rewrite, or failed version bump
  leaves no partially accepted schema.
- Schema 25 retires the former customer-project tables. Fresh databases do
  not create `fde_*` or `verification_*` tables and do not create
  `retired-customer-projects/`. Upgrades from schema 24 drop
  `fde_resource_*` triggers so shared Provider/Skill/MCP/prompt writes no
  longer update leftover generation counters; existing leftover rows stay in
  place as unconsumed historical data until a later SQL import, sync import,
  or binary restore would replace the live database. Before that replace, if
  the live database still has retired tables, the host writes one durable
  `retired-customer-projects/historical-fde-*.db` archive, verifies it, and
  only then continues. Ordinary rotating `backups/` retention must not delete
  that archive; archive failure aborts the replace and leaves the live
  database unchanged. The product does not read leftover rows or revive the
  retired module. The historical schema-22 merge still accepts both
  predecessor shapes (OpenCode proxy with or without leftover FDE tables) and
  retains proxy settings, takeover state and recovery records. SQL dumps omit
  retired tables so an import cannot revive the removed module. Binary restore
  accepts only the closed historical `fde_resource_*` trigger definitions;
  unknown trigger SQL is rejected before DML, confirmed retired triggers are
  dropped on the candidate before create/migrate, and restore finishes with
  no remaining executable triggers. SQL import continues to deny
  `CREATE TRIGGER` in the authorizer. Fixtures cover both schema-22 shapes,
  schema 21, fresh initialization, late-failure rollback, binary restore,
  keyword/forged-trigger rejection, genuine-trigger restore without migration
  side effects, and archive-before-replace; never repair a real database by
  manually rewriting its version.
- A pre-migration binary backup is attempted for an existing older database.
  The current implementation logs and continues when that safety copy fails;
  do not strengthen or weaken that behavior accidentally inside an unrelated
  migration.
- JSON migration is one transaction across Providers, MCP, Prompts, Skills,
  and common configuration. Dry-run uses an in-memory database with current
  schema compatibility checks and performs no application-file write.

### DAO and change notification

- Multi-row or cross-table invariants are committed in one transaction. A
  caller must not reproduce DAO SQL in a command/service to gain a second
  mutation path.
- Insert, update, and delete hooks invoke the connection-local listener injected
  by `set_change_listener`. The callback runs under the connection lock and must
  not block or reenter the database. The composition root alone fans hints out
  to WebDAV/S3; database modules must not import these service consumers.
- Notifications are SQLite change hints, not commit or remote-sync guarantees.
  A write later rolled back can still notify. Replacing a listener replaces the
  prior connection callback. Batching, allowlisted tables, and import suppression
  are owned by [Automatic Cloud Sync Scheduling](./auto-sync.md).
- Tables, indexes, foreign keys, uniqueness constraints, and CHECK clauses are
  part of the public persistence contract. A Rust enum/DTO change is incomplete
  until stored legacy values and schema constraints have a deliberate decode or
  migration rule.

### Export, import, backup, and sync

- The FyAgent SQL header identifies the supported wire format but is not a
  trust boundary. SQL import remains untrusted input and is executed only
  through the authorizer, temporary database, schema/trigger validation, and
  SQLite backup transaction path.
- Import rejects cross-database attachment and unsupported persistent side
  effects. SQL import authorizer denial of `CREATE TRIGGER` is not weakened
  for retired customer-project triggers. It creates a safety backup before
  replacing the live database; a failed validation, retired-data archive, or
  import leaves the main database unchanged.
- Binary restore validates executable schema with a closed allowlist of known
  historical retired triggers, copies to a candidate, disables and drops those
  triggers before create/migrate, archives leftover live retired tables when
  present, then creates the safety backup and restores through SQLite's backup
  API. Restore finishes with no remaining executable triggers. The selected
  backup file stays immutable.
- Backup filenames are leaf names owned by the backup directory. Path
  traversal, arbitrary paths, replacement collisions, and non-owned deletion
  are rejected.
- Sync export omits local-only operational tables, and sync import restores
  the corresponding local snapshot. The exact skip/preserve sets in
  `backup.rs` are one contract and must be updated and tested together.
- Managed Auth metadata (`managed_auth_identities`,
  `managed_auth_credentials`, `managed_auth_defaults`,
  `managed_auth_connections`, `managed_auth_migrations`) is local-only.
  Those rows hold opaque SecretRef handles that are meaningless on another
  device; they must be skipped on sync export and preserved on sync import
  together. Token material never has a SQLite column. Domain meaning of the
  rows is owned by [Managed Auth Core](./managed-auth.md).
- Session restore receipts (`session_restore_attempts`) are device-local and
  contain no conversation bodies. The forward migration after customer-project
  retirement creates their table. SQL exports omit receipts; SQL/sync imports
  and binary restores preserve the live device's receipts instead of replaying
  imported or backed-up target mappings. Historical project archiving must
  complete before live replacement even when a receipt snapshot is preserved.
  Domain meaning is owned by [Session Migration](./session-migration.md).
- Periodic pruning, rollup, backup retention, and incremental vacuum are
  maintenance. Disabling automatic backup must not disable unrelated pruning
  or rollup work.

## 4. Validation & Error Matrix

| Condition                                                                                              | Required result                                                                                                       |
| ------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| stored `user_version` is newer than `SCHEMA_VERSION`                                                   | Fail closed and report the stored version; no downgrade or destructive reset.                                         |
| a migration step or version update fails                                                               | Roll back the migration boundary; do not expose a partially current schema.                                           |
| JSON migration fails after some domain rows                                                            | Roll back the whole JSON migration transaction.                                                                       |
| dry-run is requested                                                                                   | Validate against an in-memory current schema; write no application database or backup.                                |
| imported SQL has the wrong header, unsafe authorization action, unsupported trigger, or invalid schema | Reject before replacing the main database.                                                                            |
| binary backup trigger SQL is not an exact known historical retired definition                          | Reject before DML; leave the live database and the original backup file unchanged.                                    |
| live database still has retired customer-project tables and a SQL/sync/binary replace would drop them  | Write and verify `retired-customer-projects/historical-fde-*.db` first; failure aborts the replace.                   |
| SQL/binary restore fails after safety preparation                                                      | Keep or restore the prior main database as defined by the SQLite backup transaction; surface an error, never success. |
| backup filename contains path components or resolves outside the backup directory                      | Reject the request.                                                                                                   |
| sync payload contains rows for local-only tables                                                       | Omit them on export and preserve the local snapshot on import.                                                        |
| a DAO write succeeds                                                                                   | Emit database-change hints for the changed table; do not claim remote sync has completed.                             |
| a transaction changes rows then rolls back                                                             | A listener may already have received dirty hints; do not treat those as commit events.                                |
| a memory database has no injected listener                                                             | No global cloud notification is emitted.                                                                              |
| cleanup, pricing-file sync, or incremental vacuum fails during otherwise valid startup                 | Log the bounded maintenance failure without fabricating completion; preserve the authoritative database.              |

## 5. Good / Base / Bad Cases

- Good: a fixture at the immediately preceding supported version is backed up,
  migrates once to the source-owned current version, preserves domain data,
  satisfies the current constraints, and reopens idempotently.
- Good: malicious SQL carries the FyAgent header but attempts a persistent
  trigger; the authorizer/schema validation rejects it and the live database
  remains unchanged.
- Good: a live database still holding leftover `fde_*` rows is archived to
  `retired-customer-projects/` before SQL, sync, or binary replace; shared
  configuration imports succeed, the archive reopens with the leftover rows,
  and a failed archive leaves the live database unchanged.
- Base: a fresh install creates the current schema directly and seeds required
  built-in pricing without replaying historical migrations.
- Bad: increment `SCHEMA_VERSION` without a predecessor fixture, execute import
  SQL on the live connection, accept an arbitrary restore path, or duplicate a
  DAO write in a Tauri command.

## 6. Tests Required

- Fresh-schema tests assert every required table/index/constraint and exact
  current `user_version`.
- Migration fixtures cover the oldest affected schema, missing columns,
  incompatible defaults/types, idempotent reopen, rollback on late failure,
  and rejection of a future version.
- JSON migration tests prove all-domain atomicity and disk-free dry-run.
- DAO tests cover uniqueness/foreign-key/CHECK failures, transaction rollback,
  concurrent access through the shared owner, and database-change hints where
  observable. Listener tests assert connection isolation, replacement,
  INSERT/UPDATE/DELETE, and hints for rolled-back writes.
- SQL import/export tests cover genuine and legacy supported exports, wrong
  product header, ATTACH/cross-file statements, persistent triggers, malformed
  late statements, exact main-database preservation, and sync skip/preserve
  symmetry. SQL import continues to deny `CREATE TRIGGER` in the authorizer,
  including genuine retired `fde_resource_*` definitions.
- Binary backup tests cover validation-before-mutation, safety backup, older
  schema forward migration, filename containment, collision, retention,
  rename, and delete. They also cover exact-match retired-trigger allowlisting,
  rejection of keyword-only and forged-prefix trigger SQL, genuine historical
  trigger restore without migration side effects, leftover-data archive before
  replace, archive-failure abort, and new installs not creating the archive
  directory.
- Run `mise run rust:test` and `mise run check:contracts`; a schema change also
  requires the affected domain and sync tests.

## 7. Wrong vs Correct

Wrong:

```text
open fyagent.db directly in a service
execute imported SQL against the live connection
advance PRAGMA user_version without the matching predecessor fixture/migration
import cloud service consumers into the SQLite update hook
```

Correct:

```text
service -> Database DAO -> one checked transaction
untrusted SQL -> validate/authorize temp DB -> safety backup -> SQLite copy
schema change -> fresh schema + ordered forward migration + rollback fixtures
composition root -> inject connection-local nonblocking dirty-hint listener
```
