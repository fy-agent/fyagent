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
  `fyagent.db`, enables `PRAGMA foreign_keys = ON`, registers the database
  change hook, creates the current table set, then applies ordered migrations.
- A brand-new file selects incremental auto-vacuum before tables are created.
  An existing non-incremental database may be backed up and rebuilt with
  `VACUUM`; failure to establish the requested mode is logged as maintenance
  degradation rather than reinterpreted as a successful rebuild.
- `Database::memory` provides the production schema and required seeds in an
  in-memory connection for tests. It must not silently omit constraints that
  production DAO code relies on.
- Mutex poisoning and SQLite failures become `AppError`; production code must
  not use `unwrap` to acquire the shared connection or serialize persisted
  JSON.

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
- Insert, update, and delete hooks notify both WebDAV and S3 auto-sync owners.
  Notifications are post-SQLite change hints, not a remote-commit guarantee.
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
  effects. It creates a safety backup before replacing the live database; a
  failed validation or import leaves the main database unchanged.
- Binary restore validates the candidate schema before creating the safety
  backup or mutating the main database, restores through SQLite's backup API,
  then applies supported forward migrations.
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
- Periodic pruning, rollup, backup retention, and incremental vacuum are
  maintenance. Disabling automatic backup must not disable unrelated pruning
  or rollup work.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| stored `user_version` is newer than `SCHEMA_VERSION` | Fail closed and report the stored version; no downgrade or destructive reset. |
| a migration step or version update fails | Roll back the migration boundary; do not expose a partially current schema. |
| JSON migration fails after some domain rows | Roll back the whole JSON migration transaction. |
| dry-run is requested | Validate against an in-memory current schema; write no application database or backup. |
| imported SQL has the wrong header, unsafe authorization action, unsupported trigger, or invalid schema | Reject before replacing the main database. |
| SQL/binary restore fails after safety preparation | Keep or restore the prior main database as defined by the SQLite backup transaction; surface an error, never success. |
| backup filename contains path components or resolves outside the backup directory | Reject the request. |
| sync payload contains rows for local-only tables | Omit them on export and preserve the local snapshot on import. |
| a DAO write succeeds | Emit database-change hints for the changed table; do not claim remote sync has completed. |
| cleanup, pricing-file sync, or incremental vacuum fails during otherwise valid startup | Log the bounded maintenance failure without fabricating completion; preserve the authoritative database. |

## 5. Good / Base / Bad Cases

- Good: a fixture at the immediately preceding supported version is backed up,
  migrates once to the source-owned current version, preserves domain data,
  satisfies the current constraints, and reopens idempotently.
- Good: malicious SQL carries the FyAgent header but attempts a persistent
  trigger; the authorizer/schema validation rejects it and the live database
  remains unchanged.
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
  observable.
- SQL import/export tests cover genuine and legacy supported exports, wrong
  product header, ATTACH/cross-file statements, persistent triggers, malformed
  late statements, exact main-database preservation, and sync skip/preserve
  symmetry.
- Binary backup tests cover validation-before-mutation, safety backup, older
  schema forward migration, filename containment, collision, retention,
  rename, and delete.
- Run `mise run rust:test` and `mise run check:contracts`; a schema change also
  requires the affected domain and sync tests.

## 7. Wrong vs Correct

Wrong:

```text
open fyagent.db directly in a service
execute imported SQL against the live connection
advance PRAGMA user_version without the matching predecessor fixture/migration
```

Correct:

```text
service -> Database DAO -> one checked transaction
untrusted SQL -> validate/authorize temp DB -> safety backup -> SQLite copy
schema change -> fresh schema + ordered forward migration + rollback fixtures
```
