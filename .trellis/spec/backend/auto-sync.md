# Automatic Cloud Sync Scheduling

## 1. Scope / Trigger

Read before changing database dirty notifications, S3/WebDAV automatic upload,
debouncing, suppression, or worker startup. `services/auto_sync.rs` privately
owns scheduling; `s3_auto_sync.rs` and `webdav_auto_sync.rs` own their transport
settings, upload lock, status persistence, and events. SQLite connection
ownership remains in [Database Persistence](./database-persistence.md).

## 2. Signatures

```rust
Database::set_change_listener(
    &self, listener: impl Fn(&str) + Send + 'static,
) -> Result<(), AppError>;

// Existing backend-specific facades; each owns an independent controller.
s3_auto_sync::notify_db_changed(table: &str);
s3_auto_sync::start_worker(db: Arc<Database>, app: AppHandle);
webdav_auto_sync::notify_db_changed(table: &str);
webdav_auto_sync::start_worker(db: Arc<Database>, app: AppHandle);
```

Each backend retains `AutoSyncSuppressionGuard::new()` for its command-side
import boundary. `AutoSyncController` and its guard are private implementation
details, not a public event bus, Tauri command, or general job scheduler.

## 3. Contracts

### Composition and isolation

The `lib.rs` composition root installs the database listener before starting
either worker, then registers application state. Registration failure aborts
before spawning workers. The database knows only the injected callback, not
S3/WebDAV modules. `Database::memory()` has no process-global sync side effects.
The callback executes synchronously under the connection lock: only a bounded,
nonblocking notification is permitted, never database reentry or network work.

Each backend has its own controller, capacity-one Tokio mpsc channel, nested
suppression counter, upload lock, and settings. Starting a controller twice
does not create a second worker. A notification before startup or after channel
closure is ignored. Suppression is checked at notification time so an imported
change cannot be queued and replayed after the guard drops. One backend's guard
does not suppress the other backend.

### Dirty hints and batching

Only `providers`, `provider_endpoints`, `mcp_servers`, `prompts`, `skills`,
`skill_repos`, `settings`, and `proxy_config` trigger scheduling; table matching
trims whitespace and is ASCII-case-insensitive. SQLite update hooks are dirty
hints, not commit acknowledgements: even a later rollback can produce a hint.
There is no exactly-once, durable queue, or remote-success guarantee.

The worker waits for one hint, merges further hints until one second of quiet,
and flushes no later than ten seconds from the first consumed hint. These are
Tokio-time scheduling bounds, not a network completion SLA. Uploads execute
serially within each worker. Changes during an upload coalesce into one pending
hint and produce a later batch. A failed upload is logged and does not kill the
worker; without a new hint it does not retry itself. Channel closure terminates
the worker without starting a pending upload.

Before upload the backend rereads settings and requires both `enabled` and
`auto_sync`. Its existing sync lock coordinates manual and automatic work.
Errors retain the backend's `last_error`/`last_error_source = "auto"` persistence
and status event; extraction must not merge the two transports or error stores.

## 4. Validation & Error Matrix

| Condition                                       | Required result                                                                         |
| ----------------------------------------------- | --------------------------------------------------------------------------------------- |
| Listener registration fails                     | Propagate `AppError`; no worker has been spawned.                                       |
| Listener is called during an uncommitted change | Queue only a dirty hint; no claim of commit or remote success.                          |
| Backend suppression is nested                   | Remain suppressed until that backend's final guard drops.                               |
| Table is not in the allowlist                   | Ignore without scheduling upload.                                                       |
| Queue is full                                   | Coalesce; never block the SQLite writer.                                                |
| Continuous notifications                        | Flush at the ten-second batching deadline.                                              |
| Upload is still running                         | Keep at most one pending hint; do not overlap automatic uploads.                        |
| Upload fails                                    | Preserve backend error reporting; process a later dirty hint, without autonomous retry. |
| Backend settings disable automatic upload       | Consume the batch without uploading.                                                    |
| Channel closes                                  | Exit without starting another upload.                                                   |

## 5. Good / Base / Bad Cases

- Good: many writes during an unsuccessful upload produce one later serialized
  upload, without retaining a per-row event backlog.
- Base: a memory database used by a unit test does not notify production cloud
  workers unless a test explicitly injects a listener.
- Bad: enqueue first and check suppression later, run upload inside the SQLite
  callback, or share one suppression counter across both transports.

## 6. Tests Required

`services/auto_sync.rs` tests use Tokio's development-only `test-util` feature
and paused time for one-second quiet, ten-second maximum, and changes during
failed upload. They also assert nested/backend-isolated suppression, all table
rules, bounded/coalesced notifications, closed channels, and one receiver.
Each backend tests all settings-flag combinations. Database tests assert
connection-local injection, INSERT/UPDATE/DELETE and rollback hints, and
listener replacement. `tests/architecture/rustModuleBoundaries.test.ts` guards
the private owner, non-duplicated scheduler, and database dependency direction.
Run `mise run check:backend` and the repository architecture tests; no live
cloud endpoint is needed to validate these scheduling contracts.

## 7. Wrong vs Correct

Wrong: `database -> services::s3_auto_sync`, or clone a worker loop whenever a
new cloud transport is added.

Correct: `composition root -> Database::set_change_listener -> nonblocking
backend facade -> private shared Tokio scheduler -> backend upload lock`.
Keep backend-specific transport and error policy outside the scheduler.
