# V2 Skills Management UI Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Skills installed/discovery views,
SkillHub search, install-target selection, ZIP installation, unmanaged import,
update, uninstall, backups, storage migration, sync-method settings, or
management-page target assignment.

Primary owners are:

- `src/v2/pages/skills/Page.tsx` for the current route and dialogs;
- `src/v2/shared/features/skills.ts` for renderer Skill DTOs, category/status
  unions, and page-size constants;
- `src/v2/shared/features/queries.ts` for installed, SkillHub, backup, update,
  and unmanaged queries;
- `src/v2/shared/features/ports.ts` and
  `src/v2/shared/platform/tauri/feature-ports/simple.ts` for `SkillsPort` IPC;
- the Skill/path/search helpers in `src/v2/shared/features/helpers.ts`.

Shared target presentation and strict Agent-page readback are owned by
[V2 Shared Assignment](./v2-assignments.md). Native archive, filesystem,
database, backup, migration, and assignment ordering are owned by
[Skill Management](../backend/skill-management.md).

## 2. Signatures

The current Port is exactly:

```ts
interface SkillsPort {
  getInstalled(): Promise<InstalledSkill[]>;
  getBackups(): Promise<SkillBackupEntry[]>;
  deleteBackup(backupId: string): Promise<boolean>;
  install(skill: DiscoverableSkill, currentApp: SkillTargetId): Promise<InstalledSkill>;
  uninstall(id: string): Promise<{ backupPath?: string }>;
  restoreBackup(backupId: string, currentApp: SkillTargetId): Promise<InstalledSkill>;
  toggleApp(id: string, app: SkillTargetId, enabled: boolean): Promise<boolean>;
  scanUnmanaged(): Promise<UnmanagedSkill[]>;
  importFromApps(imports: ImportSkillSelection[]): Promise<InstalledSkill[]>;
  discoverPage(request: DiscoverSkillsPageRequest): Promise<DiscoverableSkillsPage>;
  checkUpdates(): Promise<SkillUpdateInfo[]>;
  update(id: string): Promise<InstalledSkill>;
  migrateStorage(target: "fyagent" | "unified"): Promise<SkillMigrationResult>;
  searchSkillHub(
    query: string,
    limit: number,
    offset: number,
    category?: string,
  ): Promise<SkillHubSearchResult>;
  installSkillHub(slug: string, currentApp: SkillTargetId): Promise<InstalledSkill[]>;
  getRepos(): Promise<SkillRepo[]>;
  addRepo(repo: SkillRepo): Promise<boolean>;
  removeRepo(owner: string, name: string): Promise<boolean>;
  pickZip(): Promise<string | null>;
  installFromZip(filePath: string, currentApp: SkillTargetId): Promise<InstalledSkill[]>;
}
```

`createSimpleFeaturePorts().skills` maps these methods to the unified Tauri
commands named by [Skill Management](../backend/skill-management.md). It is a
thin compile-time-typed adapter and currently does not runtime-parse/version
the returned Skill DTOs. Do not claim a strict renderer parser already exists;
a future versioned/untrusted response must add one at this boundary.

The current route actively uses SkillHub search/install, installed Skills,
updates, unmanaged import, backups, ZIP, migration, settings, and assignment.
`install`, `discoverPage`, and repository CRUD remain available through the
Port/query layer but are not the current Skill discovery UI path.

## 3. Contracts

### Queries, filters, and selection

- `useInstalledSkills()` owns installed data under `featureKeys.skills`.
  Search is local over ID, name, description, directory, and repository
  metadata; it does not inspect Skill file contents.
- The current discovery tab uses `useSkillHubSearch`, not
  `useSkillDiscoveryPage`. Search text is debounced by 300 ms; category and page
  reset/clamp explicitly.
- The SkillHub query key contains `skillhub`, query, category, and page. It uses
  `keepPreviousData`, and an out-of-range page performs one request for the last
  valid page after total count is known.
- `SKILL_DISCOVERY_PAGE_SIZE` is 21. The current category tabs are the closed
  `SKILLHUB_OFFICIAL_CATEGORIES` plus `all`; the page does not accept a free-form
  category from route state.
- Unmanaged, backup, and settings queries are enabled only while their owning
  dialog is visible. Installed/update queries remain independent.
- `convergeSelection` keeps the selected canonical Skill ID when it remains in
  the filtered list and otherwise selects the first result. Display name,
  directory, slug, and repository label are not list/detail identity.

### Page-wide mutation and readback behavior

- `SkillsPage.write` owns one page-wide `writeLock`, busy state, success/error
  notification, and cleanup. A second management-page write is ignored while
  the lock is held.
- Every terminal write invalidates installed, backup, discovery, and unmanaged
  query keys. It refetches updates only when update data has already been
  loaded. The page does not optimistically edit installed query data.
- The management page calls `toggleApp` and then invalidates/refetches; it does
  not use `useAuthoritativeAssignmentMutation`. The current native command
  resolves `true` after success and throws on failure, while this page only
  awaits the Promise and does not inspect the returned boolean. Consequently,
  any resolved value—including a forced/test-double `false`—currently follows
  the success-toast and invalidation path. Agent-bound Skill assignment uses
  the stricter shared helper documented in
  [V2 Shared Assignment](./v2-assignments.md).
- `runSequentialBulk` executes update/assignment items in order, records all
  thrown failures, and continues. A resolved `false` is currently counted as a
  success. If native/Port semantics ever make `false` meaningful, update the
  single and bulk page paths plus their tests in the same change. If any item
  throws, the final notification reports partial success counts; earlier
  successful native writes remain applied.
- Non-`UserFacingError` messages are collapsed by `errorMessage` to a generic
  retry message. The page does not render raw native errors.

### SkillHub, ZIP, and unmanaged import

- A SkillHub card is discoverable until native installation completes. The
  installed badge is derived from canonical market ID/repository identity plus
  a normalized directory tail, not name alone.
- Installing a SkillHub item opens `InstallTargetDialog`, persists the chosen
  target in feature context, and calls `installSkillHub(slug, target)`. React
  does not download or extract the archive.
- ZIP flow calls the native `pickZip()` first. Cancellation (`null`) performs no
  install. A selected path is held only until target confirmation, then passed
  unchanged to `installFromZip(path, target)`; native code owns validation,
  extraction, and destination confinement.
- `skillInstallDestination(target, directory?)` is a renderer display preview
  for the confirmation dialog. It must never replace the closed target ID as
  native authority or be sent as the extraction destination.
- Unmanaged import starts from `scanUnmanaged()`, selects rows by `directory`,
  derives default seven-target flags from `foundIn`, allows local adjustment,
  and submits only `{directory, apps}`. The observed absolute `path` is not sent
  back as import authority.

### Updates, uninstall, backups, settings, and migration

- Update checking is disabled by default and runs only after the explicit
  action. The returned native `SkillUpdateInfo` decides whether a badge/action
  appears; the renderer does not compare mtimes or remote files itself.
- Individual update executes immediately from its button; update-all is
  sequential and may partially succeed. Do not document an update confirmation
  dialog that does not exist.
- Uninstall requires the confirmation dialog and calls `uninstall(id)`. Its
  result has optional `backupPath`; UI/spec logic must not treat backup creation
  as guaranteed evidence. Current confirmation wording is not authority for
  whether a backup was actually created.
- Backup deletion requires confirmation. Restore selects one closed target via
  the shared radio panel and calls `restoreBackup(backupId, target)`.
- Sync method is read from current settings, saved through `SettingsPort`, then
  the settings query is invalidated. The three closed values are `auto`,
  `symlink`, and `copy`.
- Storage migration requires confirmation and returns migrated/skipped counts
  plus errors. The dialog shows partial failure as warning; it does not turn a
  non-empty error list into full success.

### Paths, links, copy, and evidence

- Installed detail intentionally exposes `skill.path` when observed, otherwise
  `directory`, through `CopyablePath(revealValue=false)`. This is explicit
  user-initiated path UI; do not claim paths never enter the renderer.
- Target destination previews are static renderer labels only. Native code
  resolves the actual user/home/platform paths and enforces confinement.
- GitHub links are built only when owner/name match `^[\w.-]+$`; SkillHub
  entries use their supplied reviewed homepage/readme metadata. All external
  opening goes through `ExternalLinkButton`/the HTTP(S)-only settings Port.
- A successful install/update/assignment means the FyAgent operation returned
  and its queries were invalidated. It does not prove a vendor process reloaded
  or executed the Skill.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| Installed query fails before data | Render load failure and retry; do not fabricate an empty list. |
| Installed/SkillHub refresh fails with cached data | Preserve the last successful data and show a warning. |
| Search/category changes | Reset page to 1; key the new request independently. |
| Requested SkillHub page exceeds new total | Fetch the last valid page; do not display an impossible page. |
| Skill already matches installed identity | Disable install and label installed. |
| Native picker returns `null` | Close the attempt with no install command. |
| ZIP/SkillHub install fails | Show generic/safe failure, invalidate relevant queries, and retain no optimistic install. |
| A second page mutation starts while busy | Do not issue a second native write. |
| One bulk item fails | Continue remaining items, report partial counts, and refresh all affected queries. |
| Current native `toggleApp` throws | Treat the management-page write as failed, refresh, and do not force the requested flag. |
| A Port/test double resolves `toggleApp` as `false` | Current management page treats it as resolved success and refreshes; do not document false-rejection until page code and tests implement it. |
| Uninstall returns no `backupPath` | Do not claim a recoverable backup exists. |
| Migration `errors` is non-empty | Show partial warning with exact counts, not full success. |
| Unmanaged observation has a path outside managed storage | Submit only the validated directory/target selection; native import remains authority. |
| Vendor reload/execution is unobserved | Use FyAgent install/assignment wording only. |

## 5. Good / Base / Bad Cases

- **Good:** debounce a SkillHub query, install its slug to a selected closed
  target, then render the invalidated installed query result.
- **Good:** import an unmanaged Skill by directory with adjusted target flags;
  do not pass its observed absolute path as destination authority.
- **Base:** a ZIP picker is cancelled; no target dialog or install call occurs.
- **Base:** update-all succeeds for two Skills and fails for one; report both
  counts and keep successful updates.
- **Base:** a forced Port returns `false` without throwing. The current page
  still reports the operation as resolved and relies on invalidated query data;
  this is a characterized limitation, not an authoritative-rejection path.
- **Bad:** call a nonexistent `SkillPorts`, claim runtime DTO parsing in
  `simple.ts`, unzip in React, send the display destination string to native
  code, guarantee a backup from `backupPath?`, claim the management page checks
  `false`, or optimistically keep a target switch after an error.

## 6. Tests Required

Run focused V2 checks through the repository task runner. Required assertion
owners include:

- `tests/v2/platform/featurePorts.test.ts`: every `SkillsPort` command/payload,
  target ID, picker cancellation, and returned-value mapping;
- `tests/v2/features/featurePages.test.tsx`: installed/discovery states,
  debounced SkillHub pagination, install target, ZIP cancellation/install,
  unmanaged import, update/bulk partial outcomes, assignment invalidation,
  current resolved-value/throw assignment behavior, backups, restore target,
  settings, and migration;
- `tests/v2/features/helpers.test.ts`: canonical installed matching, search,
  display path/destination helpers, selection convergence, and sequential bulk;
- `tests/v2/shared/AssignmentPanel.test.tsx`: switch/radio semantics, disabled
  controls, labels, and closed seven-target order;
- `tests/v2/app/architecture.test.ts`: shared component ownership and no
  page-level Tauri/archive/vendor-path implementation.

Browser fixtures prove renderer behavior only. Archive confinement, backup,
database ordering, native target projection, and vendor reload require the
backend/native evidence named by [Skill Management](../backend/skill-management.md).

## 7. Wrong vs Correct

Wrong:

```ts
const destination = skillInstallDestination(target, directory);
await unzipInRenderer(file, destination);
setInstalled(true);
```

Correct:

```ts
const filePath = await ports.skills.pickZip();
if (!filePath) return;
await ports.skills.installFromZip(filePath, target);
await queryClient.invalidateQueries({ queryKey: featureKeys.skills });
// The destination preview is UI copy; native code owns extraction and paths.
```

Wrong:

```ts
const accepted = await ports.skills.toggleApp(skillId, target, enabled);
if (!accepted) showRejected();
// The current management page does not implement this false-result branch.
```

Correct:

```ts
await write("分配已更新", async () => {
  await ports.skills.toggleApp(skillId, target, enabled);
});
// `write` invalidates/refetches in finally. Today native success is true and
// failure throws; any future meaningful false result requires a page change.
```
