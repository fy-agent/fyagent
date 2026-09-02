# V2 Skills UI Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Skills page, installed/discovered
Skill lists, search/filter/pagination, install/import/ZIP/update/uninstall,
repository/SkillHub interaction, or Skill target assignment.

Primary owners:

- `src/v2/pages/skills/**`
- `src/v2/shared/features/skills.ts`
- desktop `SkillPorts` adapter
- shared [V2 Assignment](./v2-assignments.md)

Native authority is [Skill Management](../backend/skill-management.md).

## 2. Signatures

The page accesses Skills only through `SkillPorts`. The port owns typed methods
for:

```text
list installed/known Skills
search approved discovery sources with typed filters/page state
install from a reviewed source or ZIP request
import from supported targets
check/apply updates
uninstall or restore from backup
list/add/remove repositories where exposed
toggle one closed target assignment and return authoritative Skill state
```

All dynamic payloads are parsed in `src/v2/shared/features/skills.ts`. Page
components never call `invoke`, open filesystem paths, extract archives, clone
repositories or assemble vendor destination paths.

The shared target order is defined by
`src/v2/shared/features/assignments.ts`, not by the Skills page.

## 3. Contracts

### Query and view state

- Query cache owns installed/known/discovered Skill results. Local state owns
  search text, selected source/category/status/page, current modal and
  confirmation only.
- Query keys include every server-side filter and page token. A response for an
  old query must not replace the visible new filter state.
- Distinguish loading, error, empty installed list, empty search result, and
  “observed on disk but not managed by FyAgent.” Do not collapse them into one
  blank screen.
- Keep one canonical Skill ID across list/detail/assignment/update. Display
  name/repository slug is not identity.

### Discovery and install

- Discovery filters and source/page tokens remain closed parsed values. The UI
  does not widen an invalid filter to “all” or build arbitrary archive URLs.
- Search/discovery success is not install success. Installed state updates only
  after the native install transaction returns/rereads an authoritative Skill.
- ZIP import accepts only the native file-selection/reference flow already
  exposed by the desktop adapter. The renderer never supplies an extraction
  destination and never inspects archive entries itself.
- Repository/SkillHub actions use typed coordinates/results. Do not shell out
  to Git or infer source safety from a repository URL rendered in the card.
- Import from applications is explicit, shows per-source results and does not
  optimistically overwrite an existing managed Skill.

### Update, uninstall and recovery

- Update availability is native evidence based on canonical source/content
  metadata. Timestamp differences alone are not rendered as an update.
- Destructive update/uninstall/restore requires the existing confirmation and
  preserves native backup/rollback/recovery outcomes. “Rollback failed” or
  “recovery required” is never painted green.
- After terminal mutation, replace/invalidate installed, discovery result,
  detail, update and assignment queries affected by the canonical Skill ID.
- Disable duplicate non-idempotent mutations for the same Skill. Do not enable
  automatic mutation retry through the query library.

### Assignment

- Render the shared `AssignmentPanel` and call the Skill-specific target port.
  The page does not duplicate target order, path mapping or rollback logic.
- A target switch commits only from the native returned/reread Skill. Failed
  materialization restores the previous state and shows the closed error.
- A successful assignment means FyAgent wrote/reread its target representation;
  it does not prove vendor reload or execution.

### Security and copy

- Absolute paths, archive internals, shell commands, repository credentials and
  raw native errors do not appear in props, query keys, DOM, logs or clipboard.
- Render repository/source provenance without turning it into a clickable
  executable/download authority.
- All visible labels, validation, mutation results and recovery guidance use
  localization keys and evidence-correct copy.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| Skill/discovery DTO has unknown version/field/enum | Strict parse failure; no partial mutation UI. |
| Search filter/page changes while request is active | New query key; stale response cannot overwrite current view. |
| Discovery returns result | Show discoverable only; do not mark installed. |
| Install/import/update is pending | Disable duplicate mutation for same Skill and show bounded progress/state. |
| Native install/update fails or requires recovery | Render exact terminal outcome; keep/restore authoritative list state. |
| Disk-observed Skill lacks DB ownership | Label observed/importable; do not expose managed uninstall as if owned. |
| Assignment write fails | Shared authoritative rollback; previous switch state restored. |
| Uninstall/update confirmation becomes stale | Require fresh resource state/confirmation. |
| ZIP/repository validation fails | Show sanitized closed reason; no renderer fallback extraction/clone. |
| Raw path/command/secret leaks into error | Security regression. |

## 5. Good / Base / Bad Cases

- **Good:** search with a typed source/category/page key, install through the
  port, then replace installed state with the authoritative returned Skill.
- **Good:** update reports recovery required; the page keeps the prior Skill
  visible and presents recovery guidance rather than optimistic success.
- **Base:** discovery finds a Skill already present on disk but unmanaged; show
  import/observation state separately.
- **Bad:** fetch GitHub from the component, unzip in JavaScript, choose a target
  path, compare only mtime, retry install automatically, or implement a
  page-local assignment target list.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- strict Skill/discovery/repository/update parsing and canonical ID use;
- query-key isolation across search/source/category/status/page and stale
  response protection;
- loading/error/empty/observed-vs-managed states;
- install/import/ZIP/repository/update/uninstall/restore delegate only through
  `SkillPorts`, with no direct invoke/filesystem/network/shell path;
- success/error/recovery invalidates or restores every affected query;
- no automatic retry of non-idempotent mutations;
- shared assignment target order and authoritative rollback/readback;
- keyboard/focus/modal/confirmation behavior and localized copy;
- browser tests demonstrate page interaction only, not native archive,
  filesystem or vendor reload evidence.

## 7. Wrong vs Correct

Wrong:

```tsx
const install = async (url: string) => {
  const zip = await fetch(url).then((r) => r.arrayBuffer());
  await extractZip(zip, `~/.agents/skills/${slug}`);
  setInstalled(true);
};
```

Correct:

```tsx
const install = useMutation({
  mutationFn: (request: InstallSkillRequest) => ports.skills.install(request),
  retry: false,
  onSuccess: (skill) => commitAuthoritativeSkill(skill),
  onError: () => restoreOrInvalidateSkillQueries(),
});
```
