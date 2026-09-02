# Skill Management Contract

## 1. Scope / Trigger

Read this contract before changing Skill discovery, repository sources,
installation/update/uninstall, backup/restore, ZIP ingestion, unmanaged-skill
import, storage migration, target assignment, or the native Skill DTOs exposed
to the Renderer.

Primary owners are:

- `src-tauri/src/commands/skill.rs` for the Tauri command surface;
- `src-tauri/src/services/skill.rs` and `src-tauri/src/services/skill/**` for
  discovery, validation, filesystem effects, assignment, backup, and import;
- `src-tauri/src/database/dao/skills.rs` for durable rows, repositories, and
  the nine target flags;
- `src-tauri/src/app_config.rs` for `InstalledSkill`, `SkillApps`, and
  `SkillTargetId`.

Renderer behavior is owned by [V2 Skills](../frontend/v2-skills.md) and
[V2 Shared Assignment](../frontend/v2-assignments.md). SQLite lifecycle and
migration rules are owned by [Database Persistence](./database-persistence.md).

## 2. Signatures

The native target domain is closed and contains nine IDs:

```text
claude | codex | gemini | grokbuild | opencode | hermes |
qoderwork | trae-work | workbuddy
```

QoderWork, TRAE Work, and WorkBuddy are direct `SkillTargetId` values. They do
not convert to the general `AppType` enum. The V2 presentation subset is the
seven catalog-aligned targets documented by
[V2 Shared Assignment](../frontend/v2-assignments.md); Gemini and Hermes remain
native/compatibility targets.

The current unified Tauri commands are:

```text
get_installed_skills() -> Vec<InstalledSkill>
get_skill_backups() -> Vec<SkillBackupEntry>
delete_skill_backup(backup_id) -> bool

install_skill_unified(skill, current_app) -> InstalledSkill
uninstall_skill_unified(id) -> SkillUninstallResult
restore_skill_backup(backup_id, current_app) -> InstalledSkill
toggle_skill_app(id, app, enabled) -> bool

scan_unmanaged_skills() -> Vec<UnmanagedSkill>
import_skills_from_apps(imports) -> Vec<InstalledSkill>

discover_available_skills() -> Vec<DiscoverableSkill>
discover_available_skills_page(query, repo?, status, limit, offset)
  -> DiscoverableSkillsPage
check_skill_updates() -> Vec<SkillUpdateInfo>
update_skill(id) -> InstalledSkill
migrate_skill_storage(target) -> MigrationResult

search_skillhub(query, limit, offset, category?) -> SkillHubSearchResult
install_skillhub(slug, current_app) -> Vec<InstalledSkill>
install_skills_from_zip(file_path, current_app) -> Vec<InstalledSkill>

get_skill_repos() -> Vec<SkillRepo>
add_skill_repo(repo) -> bool
remove_skill_repo(owner, name) -> bool
```

`search_skills_sh`, `get_skills*`, `install_skill*`, and `uninstall_skill*`
remain compatibility/leftover commands. New V2 work uses the unified command
family through `SkillsPort`; it must not add another page-specific command set.

Key DTO contracts are:

```text
InstalledSkill {
  id, name, description?, directory,
  repoOwner?, repoName?, repoBranch?, readmeUrl?,
  apps: SkillApps, installedAt, contentHash?, updatedAt,
  path? // observed display path; not persisted
}

SkillUninstallResult { backupPath? }
MigrationResult { migratedCount, skippedCount, errors[] }
```

The service and command layers return errors rather than converting a partial
filesystem/database result into success.

## 3. Contracts

### SSOT, observation, and persistence

- The managed Skill source of truth is under `~/.fyagent/skills/`. Repository
  code resolves the real home/SSOT path; the Renderer never constructs it.
- `directory` is a validated single directory name, not a relative or absolute
  path. Traversal, path separators, and values that escape the managed root are
  rejected before copy, removal, backup, or restore.
- SQLite stores Skill identity, repository metadata, content hash/timestamps,
  and all nine assignment flags. `path` is current observation data and is not
  written to the row.
- `get_installed_skills` merges durable records with observed target
  directories. Observation may surface an unmanaged/adoptable Skill, but the
  read path does not silently write a new database row.
- The same directory observed in several targets is one logical Skill with
  merged assignment flags, not several independent installations.

### Discovery and installation

- Repository coordinates (`owner`, `name`, `branch`) are validated before
  persistence and again before download. The resolved archive must remain on
  the expected GitHub host/path; a branch cannot inject a second URL.
- Discovery pagination validates the closed status value and applies query,
  repository, status, limit, and offset in the service owner. Adding/removing a
  repository invalidates the discovery cache.
- Remote and local ZIP extraction is bounded by entry/size budgets, rejects
  traversal, and owns temporary directories until the extracted tree is no
  longer needed. A failed extraction must not leave a partial managed Skill.
- Symlink entries are never followed outside the extracted tree. Local ZIP
  installation accepts the path selected by the trusted desktop picker; the
  product must not expose an arbitrary web/path text box as equivalent trust.
- `install_skillhub` downloads the reviewed archive itself; it does not launch
  a SkillHub CLI. Installation succeeds only after the service has created the
  managed Skill, persisted its metadata, and attempted the requested target
  projection according to the service transaction.

### Target assignment and non-atomic boundaries

- `SkillService::toggle_target` first adopts a safely observed Skill when
  needed, changes the in-memory flag, performs the target copy/link or removal,
  and only then updates the SQLite flags.
- Therefore a live-target failure leaves the database flag unchanged. A rarer
  database failure after a successful live-target effect can leave filesystem
  and SQLite state divergent. The command returns an error; callers must
  reread/reconcile and must not describe the operation as atomically rolled
  back.
- QoderWork, TRAE Work, and WorkBuddy require direct copies to their native
  Skill roots. Other targets use the platform/service-selected safe projection
  method. Pages do not select copy versus symlink or construct target paths.
- Bulk target synchronization is best-effort per Skill: one malformed/stale
  row is logged and skipped rather than preventing every other Skill from
  reconciling. The aggregate operation still must not report a skipped row as
  successfully synchronized.
- A successful FyAgent projection proves only that FyAgent wrote/read its
  managed/native target state. It does not prove that the vendor application
  reloaded or executed the Skill.

### Uninstall, backup, restore, and migration

- For a normal stored row whose `directory` passes the native identity check,
  uninstall removes owned target projections, creates a recoverable backup
  when a safe source exists, removes the managed/source directory it owns, and
  returns `backupPath` only when a backup was actually created.
- A legacy/corrupt stored row whose `directory` fails validation is an explicit
  recovery exception. Uninstall skips every managed/source/target filesystem
  read, removal, and backup, deletes only the SQLite row, and returns
  `backupPath = None`. This cleanup path does not authorize a new install,
  import, restore, or caller-supplied directory to bypass validation.
- Backup IDs are resolved inside the owned backup root. Metadata containing a
  traversal directory is rejected; restore never trusts a caller path.
- Restore recreates the managed directory, recomputes the content hash,
  persists the record, and synchronizes the requested target. If persistence
  or target synchronization fails, the restore path performs the cleanup
  encoded in `SkillService` and returns an error.
- Storage migration returns migrated/skipped counts plus individual errors.
  Partial migration is explicit; an empty `errors` array is the only evidence
  that no item failed.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown target ID | Reject before any filesystem/database mutation. |
| A new request or backup metadata contains a directory that escapes an owned root | Reject as invalid input; do not inspect/remove the escaped path. |
| An existing installed row has an invalid `directory` during uninstall | Treat it as database-only recovery: touch no filesystem target/source, create no backup, delete only the row, and return no backup path. |
| Repository owner/name/branch can alter the expected archive host/path | Reject before saving/downloading and leave the discovery cache/state unchanged. |
| Archive exceeds entry/size budget or contains traversal | Abort extraction, remove temporary/partial output, and persist nothing. |
| Installed read observes the same directory in several targets | Return one Skill with merged flags; do not duplicate rows during observation. |
| Target projection/removal fails during toggle | Return error and leave the SQLite flag unchanged. |
| SQLite flag update fails after live target effect | Return error and treat state as divergent/unconfirmed; require reread/reconciliation. |
| Uninstall has no safe backup source | Return `backupPath = None`; do not invent a recovery location. |
| Restore backup ID/metadata is invalid | Reject without writing the managed or target directory. |
| One migration item fails | Preserve per-item error and accurate migrated/skipped counts; do not report full success. |
| Vendor app reload is unobserved | Say assigned/synchronized by FyAgent, not loaded/executed by the vendor. |

## 5. Good / Base / Bad Cases

- **Good:** install a reviewed SkillHub archive into the managed root, persist
  one `InstalledSkill`, project it to WorkBuddy by native target ID, then show
  the reread assignment state.
- **Good:** a Skill observed in Claude and Codex is returned once with both
  flags; observation itself does not create a database row.
- **Base:** a repository discovery returns no matching page; return an empty
  page with correct pagination metadata rather than treating it as failure.
- **Base:** uninstall cannot create a safe backup; return a successful removal
  with no backup path only when the removal transaction itself completed.
- **Base:** uninstall a legacy row whose stored directory is invalid; remove
  only the discoverable SQLite record and do not resolve or touch any path
  derived from that row.
- **Bad:** accept `../../skill`, derive `~/.qoderworkcn/skills` in React, follow
  archive symlinks, update a checkbox before native reread, or claim target
  writes and SQLite are one atomic transaction.

## 6. Tests Required

Run the focused backend/V2 gates named by the repository task runner. Required
assertion owners include:

- `src-tauri/src/services/skill.rs`: repository-coordinate and archive-URL
  validation, entry/size/traversal budgets, temporary cleanup, observed Skill
  merging, every V2 target toggle, direct-copy targets, uninstall/restore path
  confinement, invalid-stored-directory database-only uninstall, discovery
  filtering, and migration result semantics;
- `src-tauri/src/services/skill/assignment.rs`: live effect before SQLite flag
  update and best-effort per-row target reconciliation;
- `src-tauri/src/database/dao/skills.rs`: all nine flags round-trip and metadata
  updates do not resurrect an uninstalled generation;
- `tests/v2/features/authoritativeAssignment.test.tsx`: serialized toggle,
  explicit-false/error rejection, reread authority, and pending cleanup for the
  Agent-bound shared helper;
- Skill page/Port tests: current native `true`/throw mapping, page-wide query
  invalidation, exact seven-target mapping, pagination, backup/restore, and no
  direct Tauri call. A future meaningful `false` result needs a dedicated page
  regression because the current management page does not inspect it.

Portable tests cannot prove a real vendor application reloaded a projected
Skill; that claim requires separate native/HIL evidence.

## 7. Wrong vs Correct

Wrong:

```rust
db.update_skill_apps(id, &apps)?;
copy_to_renderer_supplied_path(path)?;
// UI now claims the assignment is atomically committed.
```

Correct:

```rust
SkillService::toggle_target(&state.db, id, &target, enabled)?;
// Caller rereads. Any error is unconfirmed; current filesystem/DB ordering is
// explicit and is not described as an atomic cross-resource transaction.
```

Wrong:

```ts
await invoke("toggle_skill_app", { id, app, enabled }); // feature page
setChecked(enabled);
```

Correct:

```ts
await ports.skills.toggleApp(id, app, enabled);
await installedSkillsQuery.refetch();
// Render the parsed reread value, not the click intent.
```
