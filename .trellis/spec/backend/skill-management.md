# Skill Management and Target Synchronization Contract

## 1. Scope / Trigger

Read this contract before changing Skill persistence, repository discovery,
marketplace search/install, ZIP import, backup/restore, update detection,
target assignment, vendor destination paths, or first-start migration.

Primary owners:

- `src-tauri/src/services/skill.rs` — stable service façade and shared
  filesystem/transaction safety;
- `src-tauri/src/services/skill/{assignment,discovery,marketplace,migration,repository}.rs`
  — focused policy and orchestration;
- `src-tauri/src/commands/skill.rs` — Tauri transport;
- `src-tauri/src/database/dao/skills.rs` and schema migrations — persisted
  Skill metadata and target flags.

Frontend behavior is in [V2 Skills](../frontend/v2-skills.md) and
[V2 Assignment](../frontend/v2-assignments.md).

## 2. Signatures

The existing command family remains resource-specific rather than per-Agent:

```text
get_all_skills() -> Skill[]
get_all_installed() -> InstalledSkill[]
install_skill(request) -> Skill
uninstall_skill(skillId) -> ()
toggle_skill_app(skillId, targetId, enabled) -> authoritative Skill
import_from_apps(targetIds?) -> ImportResult
install_from_zip(request) -> Skill
restore_from_backup_for_target(request) -> Skill
check_skill_updates(request?) -> SkillUpdate[]
update_skill(request) -> Skill
install_skillhub(request) -> Skill
search_*_skills(request) -> paginated discovery result
```

Repository add/remove/list helpers remain in the same transport module and
delegate to `skill/repository.rs`.

Native `SkillTargetId` contains the supported legacy application targets plus
the direct external targets `qoderwork`, `trae-work`, and `workbuddy`.
V2 presentation uses catalog order:

```text
qoderwork | trae-work | workbuddy | grokbuild |
codex | claude | opencode
```

QoderWork, TRAE Work and WorkBuddy are target IDs, not `AppType` conversions.

## 3. Contracts

### Persistence and one Skill identity

- SQLite stores one Skill row per canonical Skill ID, repository metadata,
  installed/content/update metadata and explicit target-enable flags. Current
  schema/migration sequencing is owned by
  [SQLite Persistence](./persistence-and-migrations.md).
- DAO reads/writes every supported target flag. A new target requires fresh
  table creation, ordered migration, DAO round trip, native enum, frontend
  parser and assignment test; adding only a UI checkbox loses state.
- Disk discovery may report an installed Skill not yet represented in SQLite.
  Observation does not silently mutate the database or claim FyAgent ownership.
- Repository coordinates and lock metadata are validated/sanitized before
  persistence. A repository/branch string is not a filesystem path or shell
  command.

### Filesystem and archive safety

- The shared Skill service owns path confinement, archive budgets, ZIP entry
  validation, symlink/reparse rejection, same-root temporary staging,
  hash/content checks, backup-before-destructive change and atomic/bounded
  materialization.
- `marketplace.rs` owns remote DTO/slug/category/URL validation and response
  mapping, then delegates install to the existing bounded archive owner. It
  must not implement a second ZIP extraction path.
- `assignment.rs` and `migration.rs` orchestrate through the shared safety
  primitives. They do not copy vendor trees with ad-hoc `copy_dir_all`, skip
  hash checks, or widen allowed roots.
- Treat source and destination directory identity as stable capabilities during
  an operation. Detect symlink/reparse, volume/inode/file-identity swaps and
  path escape before commit.
- Unknown files outside the managed Skill directory are not deleted. Backup
  and rollback results describe only bytes the transaction actually restored.

### Target destinations and sync

- Destinations derive from trusted home/application configuration, never from
  renderer paths.
- External direct-copy destinations include:

```text
QoderWork CN  -> ~/.qoderworkcn/skills
TRAE Work CN  -> ~/.trae-cn/skills
WorkBuddy     -> ~/.workbuddy/skills
```

Qoder Hooks deliberately remains `.qoderwork/settings.json`; do not reuse the
Hooks path as the QoderWork CN Skill root.

- `sync_to_app_dir` and `remove_from_target` are the materialization owners for
  direct-copy targets. Assignment updates are successful only after the target
  operation and authoritative database reread agree.
- Import from applications copies only sources that pass the same validation
  and are not already present. It does not overwrite a managed Skill merely
  because the vendor copy has a newer timestamp.
- Target directory absence is handled according to the target adapter. Do not
  create arbitrary vendor roots when the contract says the application/user
  data root must already exist.

### Discovery, update and repository ownership

- `discovery.rs` owns filtering, pagination and cache behavior. Invalid status,
  source or page tokens fail closed rather than widening to “all.” A poisoned
  cache mutex recovers its inner value instead of panicking.
- `repository.rs` owns `.agents` lock parsing, repository coordinate/branch
  derivation, metadata persistence and repository-list CRUD.
- Update compares canonical source/repository/content evidence, stages new
  bytes, backs up the existing managed Skill and rematerializes enabled targets
  through the same assignment owner.
- Network/discovery success is not install success. Return installed only after
  local validation, persistence and required target readback complete.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown Skill/target ID or extra request field | Reject; no DB/filesystem mutation. |
| Renderer supplies destination path, shell command or arbitrary archive URL | Reject; use closed source/target contracts. |
| ZIP entry is absolute, traversing, linked, oversized or exceeds count/budget | Reject archive and clean staging. |
| Source/destination identity changes during operation | Fail closed; do not commit or claim rollback beyond known bytes. |
| Existing managed Skill is removed/updated | Backup before destructive mutation; restore or report recovery-required on failure. |
| Target flag changes but target materialization fails | Preserve/restore prior assignment state; authoritative reread must not claim enabled. |
| Disk Skill exists without DB row | Report observation separately; do not silently take ownership. |
| Import finds an already-present managed destination | Skip/conflict according to owner policy; do not overwrite by mtime. |
| Marketplace code starts its own extraction implementation | Reject in review; delegate to shared archive owner. |
| Invalid discovery status/page/source | Typed validation error; never silently widen. |
| New target lacks schema migration/DAO/parser coverage | Contract regression. |

## 5. Good / Base / Bad Cases

- **Good:** validate one marketplace result, download to bounded staging,
  extract through the shared archive owner, persist one Skill, materialize its
  enabled targets and reread authoritative state.
- **Good:** assignment to WorkBuddy uses the trusted-home derived
  `.workbuddy/skills` destination and rolls back the flag if materialization
  fails.
- **Base:** discovery sees a vendor-installed Skill absent from SQLite; show it
  as observed/importable without claiming FyAgent management.
- **Base:** a target application directory is absent and the adapter returns a
  skipped/unavailable result; do not manufacture the vendor installation.
- **Bad:** add per-Agent install commands, trust ZIP paths, copy QoderWork CN
  Skills into `.qoderwork/skills`, compare only mtime, or set the DB flag before
  an unverified copy and leave it enabled after failure.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
```

Required assertion points:

- `src-tauri/tests/skill_sync.rs` and Skill service unit tests cover install,
  uninstall, assign/unassign, import, ZIP, backup/restore, update and
  repository round trips;
- archive traversal, absolute path, symlink/reparse, hard-link, file-identity
  race, entry-count, per-file and aggregate-size rejection;
- exact external target paths and catalog order, with QoderWork/Trae/WorkBuddy
  kept outside `AppType`;
- current schema/DAO preserves every target flag across migration and fresh DB;
- marketplace delegates to the shared extraction owner; no duplicate archive
  budget implementation;
- assignment failure restores previous state and authoritative reread drives
  the DTO;
- disk observation does not write SQLite; import is explicit and skips existing
  managed destinations;
- frontend tests cover loading/error/empty states, closed target parsing,
  confirmation, assignment rollback and no secret/path leakage.

## 7. Wrong vs Correct

Wrong:

```rust
#[tauri::command]
fn install_skill_for_agent(agent: String, zip: String, dest: String) {
    unzip(zip, dest);
}
```

Correct:

```rust
// One bounded install owner persists the canonical Skill.
let skill = service.install_from_zip(request).await?;

// One assignment owner derives and validates the closed target destination.
let updated = service.toggle_skill_app(&skill.id, target_id, true).await?;
```

The renderer chooses only a closed target ID. Native code owns archive,
destination, transaction and readback authority.
