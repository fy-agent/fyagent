# Frontend Reuse Contract

## 1. Scope / Trigger

Read this contract before adding any frontend component, helper, hook, CSS
recipe, or page-local chrome. It applies to all renderer work under
`src/v2/**` and, when those trees still change, leftover
`src/components/**` / `src/lib/**`. It does not authorize importing leftover
UI into V2.

Reuse is the default frontend preference. It applies to every page, widget,
shared module, leftover surface, and follow-up change — not only to a
cleanup pass after duplication already shipped.

### Development preference

1. Search existing shared owners first. Prefer reuse or a small extension over
   a new file.
2. Existing shared chrome that already matches the job is mandatory. Do not
   fork a page-local copy of `FeatureTabs`, `FeatureSearch`, `FeatureList`,
   `FeaturePagination`, `AssignmentPanel`, `SelectionLens` / `SelectionLensGroup`, `SplitPanes`,
   `CatalogMasterDetail`, `CatalogOfficialLinks`, `SecretInput`, `ExternalLinkButton`, FeaturePorts,
   or the TRAE/OpenCode `modelsShared` / `modelChips` helpers. Agent / Skills / MCP / Models / Prompts
   product order and display names come from `src/v2/shared/features/directory.ts`.
3. Before creating a **new** component, helper, hook, or CSS recipe, ask
   whether another current module, or a later sibling module, will use it.
   If yes or likely, put it in the shared layer on the **first commit**. Do
   not park it under `pages/<route>/` and wait for a second or third copy.
4. The old "extract after three copies" rule is not the frontend default for
   chrome that sibling routes already have or will need. Waiting for a third
   copy is a spec miss.
5. Keep a file page-local only when there is no plausible second consumer
   (a one-off form, a single dialog, a trivial `className` repeat).

Pre-V2 product UI (the tree at `f424ceff`, parent of
`82ea583a feat(frontend): add v2 visual shell`) is a **reference**, not a
runtime import. That snapshot already shared `ManagementListSearch`,
`AppToggleGroup`, `AppCountBar`, and `ListItemRow` across Skills, MCP, and
Prompts. V2 keeps the current glass / `SelectionLens` design, and ports those
behaviors into `src/v2/shared/ui`. Leftover `src/lib/api/**` remains the
command-name reference for FeaturePorts; V2 must not import `src/components`,
`src/hooks`, `src/lib`, or `src/i18n`.

## 2. Signatures

Shared feature chrome lives under `src/v2/shared/ui/`:

```ts
export function FeatureTabs<T extends string>({
  id,
  label,
  value,
  onChange,
  options,
  className,
}: {
  id: string;
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: ReadonlyArray<{ id: T; label: ReactNode }>;
  className?: string;
}): JSX.Element;

export function FeatureSearch({
  value,
  onValueChange,
  placeholder,
  ariaLabel,
  clearLabel = "清除搜索",
  className,
  disabled,
  id,
}: {
  value: string;
  onValueChange: (value: string) => void;
  placeholder: string;
  ariaLabel: string;
  clearLabel?: string;
  className?: string;
  disabled?: boolean;
  id?: string;
}): JSX.Element;

export function FeatureList({
  id,
  className,
  children,
  ...props
}: Omit<HTMLAttributes<HTMLDivElement>, "id"> & { id: string }): JSX.Element;

export function FeatureListItem({
  selected,
  onSelect,
  title,
  children,
  ariaLabel,
}: {
  selected: boolean;
  onSelect: () => void;
  title: ReactNode;
  children?: ReactNode;
  ariaLabel?: string;
}): JSX.Element;

export function FeaturePagination({
  page,
  totalPages,
  onPageChange,
  ariaLabel,
}: {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  ariaLabel: string;
}): JSX.Element | null;
```

Related shared owners already in place: `SelectionLens` /
`SelectionLensGroup` (nav, catalog, UI Lab), `AssignmentPanel` (Skills/MCP
app toggles), `SplitPanes`, `CatalogMasterDetail`, `CatalogOfficialLinks`,
`SecretInput`, `ExternalLinkButton`, `CopyablePath`, FeaturePorts, and
`PRODUCT_DIRECTORY` in `shared/features/directory.ts`.

Placement:

```text
src/v2/shared/ui/**          # chrome used by 2+ routes, or likely to be
src/v2/shared/features/**    # ports, types, helpers, queries
src/v2/pages/<route>/**      # route-owned copy, one-off forms, page CSS
src/components/common/**     # leftover-only shared chrome; not a V2 import
```

## 3. Contracts

### Search first

Before writing a new component, helper, hook, CSS class, or parser:

1. Search `src/v2/shared/ui`, `src/v2/shared/features`, and the nearest page.
2. Search leftover `src/components/common` and `src/lib/api` for the pre-V2
   behavior and command names.
3. Reuse or extend the existing owner. Do not copy the JSX.

If the job is an exclusive option track, management-list search, a
feature master list, or numbered feature pagination, the owner already exists:
`FeatureTabs`, `FeatureSearch`, `FeatureList`, `FeaturePagination`. Use them.
Do not add a second recipe.

### New component placement

Ask, before the file is created. "Later" and "expected next" count as a
second consumer:

| Will this be used by...                                      | Put it in                                   |
| ------------------------------------------------------------ | ------------------------------------------- |
| Two or more current V2 routes or shared widgets              | `src/v2/shared/ui` or `shared/features` now |
| One route today, but a sibling route/module is expected next | `shared/**` now; do not park it in `pages/` |
| Only this page, with no plausible second consumer            | `pages/<route>/`                            |
| Leftover V1 surfaces only                                    | `src/components/` or `src/lib/`; never V2   |

"Expected next" includes the other five product routes, catalog vs feature
lists, Skills vs MCP, Prompts vs Memory, and TRAE vs OpenCode model panels.

### Feature chrome

- Exclusive in-page option tracks (installed/discovery, memory types, MCP
  editor mode, Skills discovery filters/targets) use `FeatureTabs`. Do not
  hand-roll `SelectionLensTrack` + `fy-feature-tab` on those pages.
- Management-list search uses `FeatureSearch` (`role="search"`, Escape and
  clear button, Phosphor icons). That is the V2 port of pre-V2
  `ManagementListSearch`. Do not add a second raw `type="search"` Input for
  the same job.
- Feature master lists use `FeatureList` / `FeatureListItem`. Catalog agent
  rails stay on `CatalogList` / `CatalogListItem`. Primary nav stays on
  `SelectionLensGroup` with `inset={1}`.
- Skills discovery (repos and skills.sh) uses `FeaturePagination`. Do not
  hand-roll a second page-number window.
- Skills/MCP assignment stays on `AssignmentPanel` (V2 switch rows), not a
  second AppToggleGroup clone.

### Pre-V2 and leftover business

- Do not import leftover UI into V2. Architecture tests fail if `src/v2`
  imports `src/components`, `src/hooks`, `src/lib`, or `src/i18n`.
- Do reuse leftover **command names, DTO fields, and parsers** through
  FeaturePorts / `shared/platform/tauri`. Pages must not invent a second
  invoke wrapper.
- When pre-V2 UX is still the better interaction (search clear/Escape, bulk
  assignment, confirm), port it into V2 shared chrome and restyle with
  `--fy-*` tokens. Current V2 visual language wins over leftover Tailwind /
  lucide / i18n.

### Do not abstract

Do not create a shared component for a one-off form, a single dialog, or a
trivial `className` repeat. Do not merge TRAE and OpenCode model panels: they
already share `modelsShared`, `modelChips`, and `feedback`.

## 4. Validation & Error Matrix

| Condition                                                                                            | Required result                                                          |
| ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| A feature page hand-rolls `fy-feature-tab` instead of `FeatureTabs`                                  | Unit/architecture test fails; use `FeatureTabs`                          |
| Management search is a raw `Input type="search"` on Skills/MCP/Memory/Prompts/Discovery              | Use `FeatureSearch`; leftover `ManagementListSearch` stays leftover-only |
| V2 imports leftover `src/components` or `src/lib`                                                    | Architecture test fails                                                  |
| A second copy of AssignmentPanel / SecretInput / ExternalLinkButton / CatalogOfficialLinks           | Reject; extend the existing shared owner                                 |
| A page-local Agent/Skills/MCP/Models/Prompts order table                                             | Reject; extend `PRODUCT_DIRECTORY`                                       |
| New chrome used by two routes is added under `pages/<route>/`                                        | Move it to `shared/ui` before merge                                      |
| New chrome is parked in `pages/` because "only one consumer today" while a sibling route is expected | Put it in `shared/` on the first commit; do not wait for a third copy    |
| A page forks FeatureTabs / FeatureSearch / FeatureList "just for this screen"                        | Reject; pass options/copy into the shared owner                          |
| Page invents a second Tauri invoke for an existing FeaturePort command                               | Use the port; leftover `src/lib/api` is the name reference only          |

## 5. Good / Base / Bad Cases

- **Good:** Skills, MCP, Prompts, and Memory all import `FeatureSearch` /
  `FeatureList`. Skills/MCP/Memory import `FeatureTabs`. Skills discovery
  sources share `FeaturePagination`. A later filter track
  adds one `FeatureTabs` options array, not a new tab component.
- **Base:** Primary nav and catalog rails keep `SelectionLensGroup` /
  `CatalogListItem` because their geometry differs (`inset={1}`, brand frames).
- **Bad:** A page copies 20 lines of `SelectionLensTrack` + tab buttons, adds
  `pages/skills/SearchField.tsx` that MCP will need next week, or waits for
  three copies before moving chrome that Prompts/Memory already need.

## 6. Tests Required

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
```

- Unit tests cover `FeatureTabs` selection, `FeatureSearch` change / clear /
  Escape (same assertions as leftover `ManagementListSearch`),
  `FeatureListItem` `aria-current`, and `FeaturePagination` current-page
  `aria-current` plus the discovery-scroll CSS contract.
- Architecture tests prove Skills/MCP/Memory/Prompts/Discovery/model search
  import `FeatureTabs` / `FeatureSearch` / `FeatureList` as required, that
  Skills/MCP/Memory do not contain `className="fy-feature-tab"` literals, and
  that V2 still cannot import leftover UI.

## 7. Wrong vs Correct

Wrong: park reusable chrome in a page, wait for a third copy, or import leftover UI.

```tsx
// pages/skills/SearchField.tsx  — MCP will copy this next
// import { ManagementListSearch } from "@/components/common/ManagementListSearch";
<SelectionLensTrack className="fy-feature-tabs" role="tablist">
  <button className="fy-feature-tab" role="tab">
    ...
  </button>
</SelectionLensTrack>
```

Correct: shared V2 chrome; leftover is a behavior reference.

```tsx
<FeatureTabs id="skills-view-tabs" label="Skills 视图" value={tab} onChange={setTab} options={...} />
<FeatureSearch ariaLabel="搜索已安装 Skills" placeholder="..." value={search} onValueChange={setSearch} />
<FeatureList id="skills-installed-list">{items}</FeatureList>
<FeaturePagination page={page} totalPages={totalPages} ariaLabel="仓库 Skills 分页" onPageChange={setPage} />
```
