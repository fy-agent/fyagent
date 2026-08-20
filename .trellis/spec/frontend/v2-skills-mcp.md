# V2 Skills and MCP Feature Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Skills or MCP pages, their shared
feature types, query state, controls, platform adapters, or feature tests. It
defines the renderer boundary over the native Skills/MCP commands. Native
target, persistence, and security rules are authoritative in
[External Agent P0 Safety](../backend/external-agent-p0.md); this page does not
authorize widening the outer V2 shell or unrelated feature domains.

Production V2 feature code is limited to these boundaries:

```text
pages/skills, pages/mcp -> shared/features, shared/ui
shared/features        -> shared/platform feature ports
shared/platform/tauri  -> @tauri-apps/api/core.invoke
```

Legacy renderer modules are not a compatibility layer for V2. Do not import
from `src/components`, `src/hooks`, `src/lib`, or `src/i18n`. Reuse is the
default: Skills and MCP must share `FeatureTabs`, `FeatureSearch`,
`FeatureList`, `FeaturePagination`, `AssignmentPanel`, `InstallTargetDialog`,
and `SplitPanes`. New
chrome that the other page will need goes in `src/v2/shared/ui` on the first
commit. See [Frontend Reuse](./reuse.md).

## 2. Signatures

Skills and direct MCP assignment use the same closed seven identities, in Agent
catalog order. Leftover Gemini / Hermes flags remain on backend rows and must
round-trip; they are not V2 assignment targets. Grok Build is a catalog-aligned
V2 assignment target. Do not merge these collections or add Claude Desktop or
OpenClaw to either list. WorkBuddy, QoderWork, and TRAE Work are Skills/MCP-domain
targets only and are never `AppType`.

```ts
type McpTargetId =
  | "qoderwork"
  | "trae-work"
  | "workbuddy"
  | "grokbuild"
  | "codex"
  | "claude"
  | "opencode";

type SkillTargetId = McpTargetId;

const MCP_TARGET_IDS: readonly McpTargetId[] = [
  "qoderwork",
  "trae-work",
  "workbuddy",
  "grokbuild",
  "codex",
  "claude",
  "opencode",
];

const SKILL_TARGET_IDS = MCP_TARGET_IDS;

const MCP_TARGETS: ReadonlyArray<{ id: McpTargetId; label: string }> = [
  { id: "qoderwork", label: "QoderWork CN" },
  { id: "trae-work", label: "TRAE Work CN" },
  { id: "workbuddy", label: "WorkBuddy" },
  { id: "grokbuild", label: "Grok Build" },
  { id: "codex", label: "Codex" },
  { id: "claude", label: "Claude Code" },
  { id: "opencode", label: "OpenCode" },
];

const SKILL_TARGETS = MCP_TARGETS;

const DEFAULT_NEW_APPS: readonly McpTargetId[] = MCP_TARGET_IDS;

const supportedAppIconById: Record<McpTargetId, string>;
const skillTargetIconById: Record<SkillTargetId, string>;

function getSupportedAppIcon(id: McpTargetId): string;
function getSkillTargetIcon(id: SkillTargetId): string;

function skillInstallDestination(
  target: SkillTargetId,
  directory?: string,
): string;
function mcpInstallDestination(
  target: SkillTargetId,
  platform?: McpLaunchPlatform,
): string;

interface SkillsPort {
  getInstalled(): Promise<InstalledSkill[]>;
  getBackups(): Promise<SkillBackupEntry[]>;
  deleteBackup(backupId: string): Promise<boolean>;
  install(
    skill: DiscoverableSkill,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill>;
  uninstall(id: string): Promise<{ backupPath?: string }>;
  restoreBackup(
    backupId: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill>;
  toggleApp(id: string, app: SkillTargetId, enabled: boolean): Promise<boolean>;
  scanUnmanaged(): Promise<UnmanagedSkill[]>;
  importFromApps(imports: ImportSkillSelection[]): Promise<InstalledSkill[]>;
  discoverPage(
    request: DiscoverSkillsPageRequest,
  ): Promise<DiscoverableSkillsPage>;
  checkUpdates(): Promise<SkillUpdateInfo[]>;
  update(id: string): Promise<InstalledSkill>;
  migrateStorage(target: "fyagent" | "unified"): Promise<SkillMigrationResult>;
  searchSkillHub(
    query: string,
    limit: number,
    offset: number,
    category?: string,
  ): Promise<SkillHubSearchResult>;
  installSkillHub(
    slug: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill[]>;
  getRepos(): Promise<SkillRepo[]>; // leftover V1 only; V2 pages must not call
  addRepo(repo: SkillRepo): Promise<boolean>; // leftover V1 only
  removeRepo(owner: string, name: string): Promise<boolean>; // leftover V1 only
  pickZip(): Promise<string | null>;
  installFromZip(
    filePath: string,
    currentApp: SkillTargetId,
  ): Promise<InstalledSkill[]>;
}

const SKILL_DISCOVERY_PAGE_SIZE = 21; // V2 Skill 市场 pageSize；3 列网格 7×3
const SKILL_DISCOVERY_MAX_PAGE_SIZE = 50; // leftover discoverPage and SkillHub page size clamp

const SKILLHUB_OFFICIAL_CATEGORIES: ReadonlyArray<{
  key: string;
  name: string;
}> = [
  { key: "office-efficiency", name: "办公效率" },
  { key: "content-creation", name: "内容创作" },
  { key: "dev-programming", name: "开发编程" },
  { key: "data-analysis", name: "数据分析" },
  { key: "design-media", name: "设计多媒体" },
  { key: "ai-agent", name: "AI Agent" },
  { key: "knowledge-management", name: "知识管理" },
  { key: "business-ops", name: "商业运营" },
  { key: "education", name: "教育学习" },
  { key: "professional", name: "行业专业" },
  { key: "it-ops-security", name: "IT 运维与安全" },
  { key: "life-service", name: "生活服务" },
];
// Keys come from SkillHub find-skill-skillhub categories.md. Secondary
// tags must not be sent as `?category=`.

type SkillHubCategoryFilter =
  | "all"
  | (typeof SKILLHUB_OFFICIAL_CATEGORIES)[number]["key"];

type SkillDiscoveryStatus = "all" | "installed" | "uninstalled";

interface DiscoverableSkillsPage {
  skills: DiscoverableSkill[];
  totalCount: number;
}

interface DiscoverSkillsPageRequest {
  query: string;
  repo?: string;
  status: SkillDiscoveryStatus;
  limit: number;
  offset: number;
}

function FeaturePagination(props: {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  ariaLabel: string;
}): JSX.Element | null;

function buildFeaturePaginationItems(
  page: number,
  totalPages: number,
): Array<
  { type: "page"; page: number } | { type: "ellipsis"; id: "start" | "end" }
>;

function AssignmentPanel<T extends SkillTargetId>(
  props:
    | {
        mode?: "switch";
        apps: Record<string, boolean | undefined>;
        onToggle: (app: T, enabled: boolean) => void;
        labelSuffix: string;
        targets: ReadonlyArray<{ id: T; label: string }>;
        disabled?: boolean;
      }
    | {
        mode: "radio";
        value: T;
        onChange: (value: T) => void;
        ariaLabel: string;
        targets: ReadonlyArray<{ id: T; label: string }>;
        disabled?: boolean;
      },
): JSX.Element;
```

Host command and DTO (camelCase on the wire):

```rust
discover_available_skills_page(
    query: String,
    repo: Option<String>, // None / "" / "all" → no repo filter
    status: String,       // "all" | "installed" | "uninstalled"
    limit: usize,         // 0 → 20; >50 → 50
    offset: usize,
) -> Result<DiscoverableSkillsPage, String>

struct DiscoverableSkillsPage {
    skills: Vec<DiscoverableSkill>,
    total_count: usize, // serde rename_all = camelCase → totalCount
}

discover_available_skills() -> Result<Vec<DiscoverableSkill>, String> // leftover V1 only

search_skillhub(query, limit, offset, category) -> Result<SkillHubSearchResult, String>
// limit is page size (0 → 21, >50 → 50). Host maps offset to
// page = offset / pageSize + 1 and calls GET https://api.skillhub.cn/api/skills
// with keyword, optional category, page, pageSize, sortBy.
// sortBy=score when keyword is present or category is empty (全部);
// sortBy=downloads when browsing one official category with an empty keyword.
// totalCount is data.total from {"code":0,"data":{"total","skills"}}.
// Unknown category keys are dropped (treated as 全部). Do not use /api/v1/search.
install_skillhub(slug, current_app) -> Result<Vec<InstalledSkill>, String>
search_skills_sh(...) // leftover V1 only; V2 discovery must not call it

const SKILLHUB_MARKET_OWNER = "skillhub.cn"; // not a GitHub owner

struct SkillHubDiscoverableSkill {
    key, slug, name, description, directory,
    repo_owner, // always "skillhub.cn"
    repo_name,  // slug
    repo_branch, version, owner_name,
    installs, downloads, homepage_url, readme_url,
    category, // official key or None
}
struct SkillHubSearchResult {
    skills, total_count, query,
    categories, // the 12 official {key,name} rows
}
// homepage_url is always https://skillhub.cn/skills/{slug}, never api.skillhub.cn
// and never the API `homepage` field.
```

```ts
interface McpPort {
  getAll(): Promise<Record<string, McpServer>>;
  upsert(server: McpServer): Promise<void>;
  delete(id: string): Promise<boolean>;
  toggleApp(
    serverId: string,
    app: McpTargetId,
    enabled: boolean,
  ): Promise<void>;
  importFromApps(): Promise<number>;
}

interface SettingsPort {
  get(): Promise<FeatureSettings>;
  save(settings: FeatureSettings): Promise<boolean>;
  openExternal(url: string): Promise<void>;
}

function useOpenExternal(): {
  openExternal: (
    url: string,
    options?: { errorTitle?: string },
  ): Promise<void>;
  openingUrl: string | null;
};

function ExternalLinkButton(props: {
  url?: string;
  children: ReactNode;
  errorTitle?: string;
  busyLabel?: string;
}): JSX.Element;
```

## 3. Contracts

### Platform and command boundary

- Only `src/v2/shared/platform/tauri/**` imports `@tauri-apps/**`.
- The Tauri adapter maps the port methods to the existing snake-case command
  names and camel-case payload keys. It must not call deprecated per-app APIs.
- Skill ports accept all V2 `SkillTargetId` values. Native `SkillApps` still
  stores leftover Gemini / Hermes columns. MCP CRUD/import/direct assignment
  accepts the same seven V2 `McpTargetId` values as Skills (`qoderwork`,
  `trae-work`, `workbuddy`, `grokbuild`, `codex`, `claude`, `opencode`). Native
  `McpTargetId` also keeps leftover Gemini / Hermes for round-trip; those
  leftover IDs are not V2 assignment targets and never convert to `AppType`.
  QoderWork and TRAE Work `validate_external_mcp_config` remains a native
  command and does not replace live-file assignment; the Agent directory does
  not host that panel.
- `get_installed_skills` / `SkillService::get_all_installed` unions SQLite
  rows with `scan_unmanaged` across every native `SkillTargetId`. GET is
  read-only: it does not insert rows or copy into SSOT. Dot directories such as
  Codex `.system` are skipped. The first toggle or uninstall of an observed
  skill adopts through existing `import_from_apps`.
- Browser reads return empty authority snapshots. Browser writes reject with a
  clear native-only error and never report success.
- MCP presets have one source under `shared/features`: Windows uses
  `cmd /c npx`, and every other native platform uses direct `npx`. Time and
  Fetch use `uvx`. The legacy renderer adapter only re-exports this source.
- Feature tests inject ports or a page-load Tauri IPC fixture. Production code
  must not contain test routes, fixture switches, or synthetic data.
- V2 Skills discovery calls `search_skillhub` through
  `SkillsPort.searchSkillHub`. It must not invoke leftover
  `discover_available_skills`, `search_skills_sh`, `discover_available_skills_page`,
  `get_skill_repos`, `add_skill_repo`, or `remove_skill_repo`. Those host
  commands remain on the port for leftover V1 only. V2 must not render
  **管理仓库** / **仓库管理**, must not add GitHub skill-repo sources, and
  must not treat configured GitHub repos as a discovery or management
  surface. The Discover tab is Skill 市场 only. Default Skill
  市场 page size is 21 (`SKILL_DISCOVERY_PAGE_SIZE`) so the 3-column grid fills
  7 rows; leftover `discoverPage` still defaults to 20. Host SkillHub
  `limit == 0 → 21` and `limit > 50 → 50`; leftover `discoverPage` still uses
  `0 → 20`. The list API is official `GET /api/skills` (`keyword`, `category`,
  `page`, `pageSize`, `sortBy`). Do not call `/api/v1/search`, do not grow
  `limit = offset + pageSize`, and do not invent `totalCount`. `totalCount` is
  `data.total`. `page` is `offset / pageSize + 1`. Category tabs are the
  official 12 first-level keys plus **全部**; the selected key is the optional
  `category` argument (`""` for 全部). Changing search or category resets to
  page 1 in the handler. Search is debounced (~300ms) for the request, but the
  page index resets on the keystroke. If the requested page is past
  `totalCount`, `useSkillHubSearch` retries the last page; do not `setPage` in
  a `useEffect` (`react-hooks/set-state-in-effect`). Install matching is
  directory tail plus case-insensitive `repoOwner`/`repoName` (Skill 市场 uses
  owner `skillhub.cn` and slug as `repoName`). Empty query still loads the
  ranked feed (`sortBy=score`).

- SkillService caches a successful enabled-repo scan in process for 5 minutes
  under a fingerprint of sorted `owner/name/branch`. Cache misses still scan
  via the existing zip path. `add_skill_repo` and `remove_skill_repo` must
  invalidate that cache. IPC still returns only the requested page.

### State and writes

- A FeatureProvider owns one stable QueryClient and a session-only install
  target. The default target remains `claude` (label Claude Code); navigation
  preserves it, while a full application restart resets it. Discovery does not
  show header install-target tabs. Clicking **安装** opens a Dialog to pick
  the target before `installSkillHub`. That picker is the shared
  `InstallTargetDialog`, which wraps `AssignmentPanel`
  `mode="radio"` (`aria-label="安装目标"`), not a page-local
  `InstallTargetPicker`. After the user picks a target, **下一步** shows the
  official destination from `skillInstallDestination` /
  `mcpInstallDestination` (`InstallPathPreview`); **确认安装** (overwrite:
  **确认覆盖安装**) is the only control that writes. **返回** goes back to
  the radio without calling the host. Do not use `ConfirmDialog` for this
  path step. MCP discovery **安装** and **重新配置** (zero-config)
  reuse that same dialog and write a single chosen `McpTargetId`; they do not
  install onto `DEFAULT_NEW_APPS`. MCP **配置并安装** keeps credential fields
  in `InstallDialog` and uses the same `AssignmentPanel mode="radio"` for the
  Agent target, then the same path preview before `upsert`. ZIP install
  shows the target skills root plus a note that the folder name comes from
  the archive. Backup restore reuses the same radio
  panel (`aria-label="恢复目标"` on restore). Unmanaged import uses the same
  component in switch mode, not `fy-feature-check-grid`. Installed assignment
  (narrow and wide panes) is the same component in switch mode. New-MCP editor
  initial assignment is also `AssignmentPanel` switch rows. Confirming
  install, ZIP, or restore updates the session target. Assignment, bulk
  全开/全关, that radio picker,
  and new-MCP `DEFAULT_NEW_APPS` must render in Agent catalog order, not
  alphabetical or Claude-first order.
- Skill assignment authority on V2 pages contains seven booleans. Native rows
  still persist leftover Gemini / Hermes plus Qoder / TRAE / WorkBuddy flags.
  Missing `qoderwork`, `trae-work`, `workbuddy`, or `grokbuild` values parse as
  false; leftover flags are preserved. All seven V2 Skill targets install,
  assign, and unassign through one host path: `install_skillhub` /
  `install_from_zip` / `restore_from_backup_for_target` / `toggle_skill_app`
  → `SkillService::sync_to_app_dir` or `remove_from_target`.
  `import_from_apps` writes the selected flags, then syncs only **missing**
  dests for those flags (do not rewrite a live vendor copy that already
  exists). Do not add per-agent commands or page branches.
  QoderWork / TRAE Work / WorkBuddy remain copy-only destinations
  (`~/.qoderworkcn/skills`, `~/.trae-cn/skills`, `~/.workbuddy/skills`) inside
  that shared path. Successful UI copy claims directory synchronization, not
  vendor recognition or loading. Claude / Codex / Grok Build / OpenCode keep
  the configured symlink-or-copy method. Directory-swap checks compare volume
  - inode only; parent mtime changes from creating a tempdir must not fail
    copy or remove.
- Server data is authoritative. Successful writes and partial failures both
  invalidate and reread the affected resources before the UI settles.
- Disabling or deleting an MCP assignment removes it from that application's
  live configuration before clearing the authoritative flag. Multi-application
  cleanup commits each successful removal so a later failure remains exactly
  retryable without a false disabled claim.
- Direct MCP live files: QoderWork CN writes `{trusted-home}/.qoderworkcn/mcp.json`
  (`mcpServers` map); TRAE Work CN writes TRAE SOLO CN `User/mcp.json`
  (macOS `Library/Application Support/TRAE SOLO CN/User`, Windows roaming same
  product folder); WorkBuddy writes `{trusted-home}/.workbuddy/mcp.json`.
  Import may read hidden `{trusted-home}/.workbuddy/.mcp.json` when the
  official file is absent. First official write may seed `mcpServers` from
  that hidden file. Do not treat `.mcp.json` as the live write target.
  All three skip when neither the home/User directory nor the file exists.
  Do not write Qoder `userData/mcp.json` (builtin table) or TRAE `state.vscdb`
  for MCP. Import may normalize Qoder `type: "streamable-http"` to `http`
  before `validate_server_spec`.
- After a successful WorkBuddy install or newly enabled assignment, the MCP
  page opens the shared `Dialog` (`需要在 WorkBuddy 中信任 MCP`) telling the
  user to trust it under **连接器 → 自定义连接器**, with **知道了** to
  dismiss. Body copy states that WorkBuddy officially requires a manual trust
  step for third-party MCP after install. Do not use `ConfirmDialog` for this
  notice. Other targets and
  failed writes do not show it. Re-saving an already-assigned WorkBuddy MCP
  does not repeat the notice.
- Cross-application MCP imports merge assignments only when normalized server
  specifications are equivalent. A conflicting shared ID is preflighted before
  any server from that source application is persisted.
- OpenCode and Hermes imports preserve explicit source disablement: disabled
  commands clear an existing assignment and never create a new managed row.
- One write lock disables only conflicting writes. Reads, search, selection,
  and details remain available. Batch writes run sequentially and report
  progress and a final success/failure summary.
- Skill storage migration calls only `migrate_skill_storage`. Sync-method
  saves first read the complete settings object and merge the changed field.

### MCP configuration and secrets

- List search uses explicit public-field allow-lists. It never recursively
  stringifies an MCP server, never indexes `env` or `headers`, never indexes
  URL query values, and never indexes argument values that follow sensitive
  flags such as `-s` or `--token`.
- Ordinary details show only secret-field item counts for `env` and `headers`.
  Those values may appear only in the explicit editor or a catalog install
  dialog. Ordinary details must redact sensitive URL query values and
  sensitive command arguments.
- Installed Skills and MCP use the same three-column workspace: list, detail,
  and assignment, laid out with the shared `SplitPanes` chassis (14px
  gutter, pointer/keyboard resize, independent pane scroll). Installed lists
  use `FeatureList` / `FeatureListItem`; installed/discovery and MCP editor
  tracks use `FeatureTabs`; management search uses `FeatureSearch`. Skills
  discovery (Skill 市场) and any later paged feature list use
  `FeaturePagination`; do not clone the page-number window. The shared pager
  shows `第 x / n 页`, 上一页 / 下一页, numbered pages, and ellipsis when
  `totalPages > 7`. Do not add a pagination npm package. Do not
  add a page-local tabs, search, list, or pagination clone. `.fy-feature-list` is a
  column flex track so `SelectionLens` (absolute overlay) is not a grid item
  and list rows do not collapse onto one another. Do not restore
  `display: grid` on that class. Each column
  scrolls independently; the content viewport must
  not grow with the left-hand list. Split-pane children fill the pane height
  (`min-height: 100%` and `height: 100%`) and scroll inside the pane
  (`overflow: auto`), matching catalog rails. Do not leave `height: 100%`
  on a feature panel without overflow, or assignment rows and cards paint
  past the panel chrome. Assignment rows wrap (`flex-wrap: wrap`,
  `min-width: 0`) so “全开 / 全关” stay inside the pane. The Discover tab
  stays a card grid and must not use this master-detail chassis. Discovery
  chrome is search (`FeatureSearch`), then official first-level category
  `FeatureTabs` (`aria-label="分类筛选"`: **全部** plus the 12 SkillHub names),
  then the card grid, then `FeaturePagination`. Do not add source tabs
  (**Skill 市场** / **仓库**), install-status tabs, repo-filter chips, or a
  summary line such as `Skill 市场 · 21 / n · 将安装到 Claude Code`. Do not add
  skills.sh, SkillsMP, ClawHub, or configured GitHub repos as a V2 discovery
  source. Do not add second-level SkillHub tags as tabs. Do not keep
  install-target `FeatureTabs` or **将安装到 …** in the discovery header.
  Do not render **管理仓库** anywhere on the V2 Skills page (header, Discover,
  Installed **更多**, or a 仓库管理 dialog). Leftover GitHub repo CRUD stays
  in leftover V1 only. Discovery **安装** opens a Dialog (`安装 {name}`) that
  reuses shared `InstallTargetDialog` → `AssignmentPanel`
  `mode="radio"` (`aria-label="安装目标"`) for the
  seven catalog targets with decorative icons (`alt=""`, `aria-hidden="true"`)
  and labels. **下一步** then shows the destination path; confirm with
  **确认安装**. MCP discovery one-click
  **安装** and overwrite **重新配置** use that same dialog (**确认覆盖安装**
  on overwrite) and persist only the chosen target. MCP
  **配置并安装** keeps field checkboxes for recipe options, but Agent target
  selection is the same radio panel, not `fy-feature-check-grid`, and must
  show `mcpInstallDestination` before `upsert`. ZIP install
  opens the same Dialog chrome (`从 ZIP 安装`) after a file is chosen and
  previews the skills root. Backup
  restore keeps
  `AssignmentPanel mode="radio"` (`aria-label="恢复目标"`) in the backups
  Dialog. Unmanaged import uses switch mode on each selected Skill. Do not add
  a page-local picker, a `<select>`, a checkbox grid, or a page-local tab clone.
  Confirm copy uses the catalog label (`Claude Code`, not `Claude`).
  Skill Discover
  cards show the name and
  install state in the header, a 3-line clamped description, then an optional
  one-line note of category, version, and author. Do not print a
  GitHub `owner/repo` line or a “来自 …” fallback on the card. Do not group
  cards under repository headings. `.fy-feature-card` is a column flex so
  footer actions align across a
  stretched grid row. Full copy is not on the card: **详情** opens the shared
  `Dialog` with the complete description and source meta (Skill 市场 category /
  slug / author / version / installs).
  Skill 市场 cards use **主页** (`https://skillhub.cn/skills/{slug}`) as
  `ExternalLinkButton`. Skills discovery search chrome
  and `FeaturePagination` stay with the page that scrolls as a whole; the
  inner `.fy-feature-discovery-scroll` is in-flow. MCP discovery still uses
  that shared class as an independent scroller. Do not add overflow to
  `.fy-feature-detail-scroll`. Do not group cards by wrapping a second
  card around `DiscoveryCard`. Skill uninstall and MCP edit/delete stay
  in the detail header above source, assignment, and install cards so they
  remain reachable without scrolling the middle pane. MCP details must show
  install provenance and current assignment chips, matching Skills.
- Installed Skill details copy the resolved SSOT install path; the path is
  not shown beside **安装目录**, only the copy action. MCP details copy a
  local install directory when `cwd` or an absolute stdio command path is
  available, also without displaying the path; npx, uvx, and remote
  transports show that no local directory exists. Skill 市场 installs use
  source **从 Skill 市场安装**, not GitHub. `.fy-feature-code` and
  `.fy-feature-path-value` have no dark pill background.
- MCP has permanent Installed and Discover tabs. Discover is a static curated
  catalog of about 20–30 installable items: each card is either one-click or a
  credential/config form. Discover classification is only “直接安装” versus
  “配置安装”, plus an “全部” default. Prefer popular no-credential stdio/HTTP
  recipes for the remaining slots. It does not add a market API, persist catalog
  metadata, or widen the V2 MCP assignment set beyond the seven catalog-aligned
  targets. Entries that need OAuth,
  post-start login, SSE-only transport, or unverified high-privilege cloud
  control stay out of the catalog. New remote recipes use Streamable HTTP
  only.
- Discover card “文档” / “主页” and installed-detail homepage/docs, plus
  installed Skill “打开仓库” / “查看说明”, render `ExternalLinkButton`.
  That control is the only HTTP(S) jump: it calls `useOpenExternal`, which
  owns one FeatureProvider lock and `settings.openExternal`. Discover shows
  docs when present, otherwise homepage, never both. Do not add
  `.fy-mcp-card-link`, `<a href>`, `window.open`, or a page-local
  `openExternal` wrapper. Failures toast and never echo the URL.
- Quick and advanced modes share one canonical `McpServerSpec`. Quick edits
  replace known fields while preserving unknown extension fields, unknown
  top-level fields, and hidden application flags.
- Advanced JSON accepts one non-array object and rejects an `mcpServers`
  container. Invalid JSON cannot be saved or replaced by a mode switch.
- User-facing errors and logs must not interpolate MCP configuration objects,
  environment variables, headers, tokens, or secret-bearing URLs.

### Presentation boundary

- User-visible CSS is namespaced under `.fy-feature-*` or `.fy-control-*`.
  Skills and MCP own only the page wrappers `.fy-skills-page` and
  `.fy-mcp-page`; do not invent a parallel `.fy-skills-*` / `.fy-mcp-*` theme.
  Consume only `--fy-*` tokens. MCP discovery card grids scroll with shared
  `.fy-feature-discovery-scroll` (`overflow: auto`). Skills discovery scrolls
  the whole `.fy-skills-page-discovery` feature page; its inner
  `.fy-feature-discovery-scroll` is in-flow (`overflow: visible`).
  `.fy-feature-detail-scroll` remains the installed-detail column and must not
  gain overflow for this job.
- The shared assignment panel resolves all V2 Skill and MCP targets through
  `skillTargetIconById` / `getSkillTargetIcon`. MCP passes `MCP_TARGETS`
  explicitly and still goes through that map. `supportedAppIconById`
  / `getSupportedAppIcon` cover the same seven catalog identities. Runtime code must
  not import a legacy asset path or a remote URL. A reviewed byte-for-byte
  local asset copy is acceptable when V2 owns the resulting path and the asset
  inventory is updated. WorkBuddy uses `../agents/workbuddy.png`; QoderWork CN
  uses `../agents/qoderwork.png`; TRAE Work CN uses `../agents/trae-work.png`.
- Assignment icons are decorative beside the existing text:
  `alt=""` and `aria-hidden="true"`. The switch keeps the sole accessible name
  `${app.label} ${labelSuffix}`; an icon must not create a duplicate label.
- The installed Skills/MCP workspace renders exactly one switch
  `AssignmentPanel` in the DOM and accessibility tree. Responsive layout
  changes whether it is the third column or a details section; CSS must not
  hide a duplicate semantic panel. Modal dialogs may mount another
  `AssignmentPanel` for install, ZIP, restore, or import.
- Changes must not alter the TopBar, brand, primary navigation, window chrome
  ownership, ContentViewport shell, route order, existing shell-owned Blue
  Ambient token values or appearance, or the Agents, Models, Prompts, and
  Memory page contents.
- Feature controls may add the minimum required semantic tokens under the
  `--fy-*` namespace without changing the shell-owned appearance.

## 4. Validation & Error Matrix

| Condition                                                        | Required result                                                           |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Browser performs a feature read                                  | Return an empty collection or settings snapshot without a side effect     |
| Browser performs a feature write                                 | Reject with the native-only error; never show a success toast             |
| Initial authority read fails                                     | Show an error/retry state, not an empty-state success                     |
| Refresh fails after old data exists                              | Keep old data and show an inline error                                    |
| Batch write partially fails                                      | Report counts, keep no stale optimistic claim, and reread authority       |
| MCP search term matches only an env/header value                 | Return no match                                                           |
| MCP search term matches only a URL query secret or sensitive arg | Return no match                                                           |
| env/header line has no delimiter or an empty key                 | Show a line error and block save                                          |
| Advanced JSON is invalid, an array, or an `mcpServers` container | Stay in advanced mode and block save                                      |
| New MCP ID is blank or duplicates an authoritative ID            | Block save before invoking Tauri                                          |
| A backend error may contain MCP configuration                    | Show a fixed secret-safe message                                          |
| Imported shared ID has a different executable specification      | Reject that application's import without partial persistence              |
| OpenCode/Hermes source entry has `enabled: false`                | Keep it disabled; do not create or activate a managed assignment          |
| MCP live cleanup fails while disabling or deleting               | Retain the failed assignment and retryable authoritative record           |
| A Skill response omits either new external target                | Default that target to false without changing any legacy assignment       |
| QoderWork or TRAE Work MCP assignment is enabled                 | Write the vendor live `mcp.json`; skip if home/User and file are absent   |
| WorkBuddy MCP assignment is enabled                              | Write `{trusted-home}/.workbuddy/mcp.json`; skip if home and file absent  |
| WorkBuddy MCP install/assignment succeeds                        | Shared `Dialog` tells the user to trust it under 连接器 → 自定义连接器   |
| WorkBuddy MCP writes `.mcp.json` as canonical                    | Host test fails; official live file is `mcp.json`                         |
| QoderWork CN Skill dest is `.qoderwork/skills`                   | Host test fails; CN product dest is `.qoderworkcn/skills`                 |
| MCP discovery one-click installs `DEFAULT_NEW_APPS` with no dialog | Page test fails; reuse shared `InstallTargetDialog` for one target      |
| Skill/MCP install writes after picking a target, before path confirm | Page test fails; **下一步** must show the destination; host waits for **确认安装** |
| MCP/Skills assignment order is alphabetical or Claude-first      | Page/component test fails; order must match Agent catalog                 |
| `.fy-feature-list` is restored to CSS Grid                       | List rows overlap because `SelectionLens` occupies a grid track           |
| A supported app is missing from the local icon map               | Type/asset test fails; never render a remote fallback or broken image     |
| An assignment icon contributes an accessible name                | Component accessibility test fails; switch text remains the sole name     |
| Viewport changes between two- and three-column layouts           | Render exactly one panel: seven unique Skill or seven unique MCP switches |
| `.fy-feature-discovery-scroll` loses `overflow: auto` on MCP     | MCP discovery cards cannot scroll independently                           |
| Skills discovery only scrolls the inner card strip               | Page CSS test fails; `.fy-skills-page-discovery` must scroll the page     |
| Discovery `.fy-feature-card-body` drops `-webkit-line-clamp`     | CSS test fails; preview stays 3 lines; full copy is the 详情 dialog       |
| Discovery card has no 详情 control                               | Page test fails; full description must not live only on the card          |
| FeaturePagination is a 5-number slice without prev/next          | Shared owner must include 上一页/下一页, ellipsis, and `第 x / n 页`      |
| A page adds a pagination npm package or a second pager           | Reject; extend `FeaturePagination`; Radix has no Pagination primitive     |
| WorkBuddy is converted to `AppType` or added as a Provider app   | Type/runtime test fails; WorkBuddy stays Skills/MCP-domain only           |
| Discover/docs or Skill repo is opened without ExternalLinkButton | Component test fails; the click must hit `settings.openExternal`          |
| A second HTTP(S) jump starts while one is in flight              | Ignored; only the in-flight control shows pending copy                    |
| V2 Skills discovery calls leftover `discover_available_skills`   | Adapter/page test fails; use `searchSkillHub`                             |
| Browser `discoverPage` read                                      | `{ skills: [], totalCount: 0 }` with no zip/scan side effect              |
| `limit == 0` / `limit > 50`                                      | SkillHub page size 21 / 50; leftover `discoverPage` still 20 / 50         |
| Discovery `status` is not `all\|installed\|uninstalled`          | Leftover `discoverPage` command error; V2 discovery does not send it      |
| `offset` is past the filtered total                              | Leftover `discoverPage`: `skills: []` and unchanged filtered `totalCount` |
| Search or category change on Skills discovery                    | `search_skillhub` uses official page/pageSize; UI returns to page 1       |
| V2 Skills discovery calls `search_skills_sh` or `discoverPage`   | Adapter/page test fails; Discover uses `search_skillhub` only             |
| SkillHub list called as `/api/v1/search`                         | Reject; list is `GET /api/skills` with `page` / `pageSize` / `category`   |
| SkillHub catalog frozen at 50                                    | Reject; `SKILL_DISCOVERY_MAX_PAGE_SIZE` clamps page size only             |
| SkillHub `category` is not an official first-level key           | Host omits `category` (treat as 全部); do not send secondary tags         |
| SkillHub card shows `owner/repo` or a grouping heading           | Page test fails; source meta lives in 详情 only                           |
| Skill 市场 install spawns `skillhub` CLI                         | Host must download `api.skillhub.cn/api/v1/download?slug=` ZIP            |
| SkillHub slug contains `/`, space, `@`, or `..`                  | Command error `INVALID_SKILLHUB_SLUG`; do not build the download URL      |
| Skill 市场 homepage is `api.skillhub.cn/...`                     | Reject; construct `https://skillhub.cn/skills/{slug}` only                |
| `.fy-feature-detail-scroll` is given overflow for discovery      | Skills toolbars scroll away; Skills discovery scrolls the page wrapper    |
| Discover shows **Skill 市场** / **仓库** source tabs             | Page test fails; Discover is Skill 市场 only, no source tabs              |
| V2 Skills shows **管理仓库** / **仓库管理** / add-repo form      | Page test fails; GitHub skill-repo CRUD is leftover V1 only               |
| V2 Skills page calls `getRepos` / `addRepo` / `removeRepo`       | Page/query test fails; leftover commands must not run in V2 UI            |
| Discover shows header **将安装到 …** / 安装目标 tabs             | Page test fails; pick the target in the install Dialog                    |
| Skills install picker is not `AssignmentPanel mode="radio"`      | Reuse test fails; do not add `InstallTargetPicker` on the page            |
| ZIP / restore / import uses a second target list                 | Page test fails; reuse `AssignmentPanel` radio or switch                  |
| Assign/unassign/install uses a per-agent host command            | Reject; all seven V2 targets use `toggle_skill_app` / `sync_to_app_dir`   |
| Vendor parent mtime change fails WorkBuddy/Qoder/TRAE copy       | Host test fails; directory identity is volume + inode only                |
| Discover omits 办公效率 / 开发编程 / IT 运维与安全               | Page test fails; official 12 first-level names plus 全部                  |
| Discover shows `Skill 市场 · n / m · 将安装到 …`                 | Page test fails; counts live on `FeaturePagination` only                  |

## 5. Good / Base / Bad Cases

- **Good:** Skills discovery is Skill 市场 only (`search_skillhub` →
  `GET /api/skills`). Empty query still loads the ranked feed (`sortBy=score`).
  Category tabs are **全部** plus the 12 official first-level names; **办公效率**
  sends `category=office-efficiency`. Cards are a flat grid with a 3-line
  Chinese `description_zh` preview and an optional category/version/author
  note; **详情** shows category / slug / author / version; **主页** opens
  `https://skillhub.cn/skills/{slug}`. Install calls `install_skillhub` (ZIP
  download + `install_from_zip`), never GitHub archive and never the
  `skillhub` CLI. There are no source tabs, no **管理仓库**, and no
  `Skill 市场 · n / m` summary. Clicking **安装** opens a Dialog that reuses
  `AssignmentPanel mode="radio"` (icon + name), then a path preview, before
  `installSkillHub`.
  MCP discovery one-click **安装** uses the same shared `InstallTargetDialog`
  and writes only the chosen target after **确认安装**. ZIP install and backup
  restore reuse that
  radio panel. Unmanaged import
  uses switch mode. Installed assignment uses the same component in switch
  mode. New-MCP editor assignment uses switch mode, not a checkbox grid. All
  seven V2
  targets assign and unassign through `toggle_skill_app` → `sync_to_app_dir`
  / `remove_from_target`. Import syncs missing dests only. `FeaturePagination` (`ariaLabel="Skill 市场分页"`)
  pages through `data.total` at 21 items per page. Search or category changes
  return to page 1. MCP discovery still scrolls through
  `.fy-feature-discovery-scroll`. Skills discovery scrolls the whole feature
  page.
- **Good:** A user toggles Codex for one Skill. The UI invokes
  `toggle_skill_app` with `{ id, app: "codex", enabled }`, locks only
  conflicting writes, then rereads installed Skills before settling. The row
  shows the V2-owned Codex icon decoratively without changing the switch name.
- **Good:** An old installed-Skill row has leftover Gemini / Hermes flags.
  The adapter preserves those values, supplies false for missing
  Qoder / TRAE / WorkBuddy / Grok Build flags, and a later WorkBuddy sync copies
  only to `~/.workbuddy/skills`. MCP writes `~/.workbuddy/mcp.json` as
  `mcpServers` and skips when neither the home nor the file exists.
  Hidden `~/.workbuddy/.mcp.json` is import/seed fallback only.
- **Good:** A QoderWork CN Skill assign copies only to `~/.qoderworkcn/skills`.
  Qoder Hooks remain `~/.qoderwork/settings.json` and are not this dest.
- **Good:** V2 assignment shows seven Skill targets and seven MCP targets in
  Agent catalog order: QoderWork CN, TRAE Work CN, WorkBuddy, Grok Build,
  Codex, Claude Code, OpenCode. Gemini / Hermes do not appear as chips.
- **Good:** Enabling QoderWork MCP writes `~/.qoderworkcn/mcp.json`; enabling
  TRAE writes TRAE SOLO CN `User/mcp.json`. Missing home and file skips write.
- **Base:** A browser preview has no fixture. Both pages show their native-safe
  empty states; `discoverPage` returns an empty page; attempts to mutate reject
  instead of simulating persistence.
- **Bad:** MCP search uses `JSON.stringify(server)`, assignment lists are
  sorted A–Z, a toast prints an invoke
  error containing headers, quick mode reconstructs the whole server object,
  both responsive assignment panels remain mounted, or V2 discovery pulls the
  full leftover `discover_available_skills` list into the renderer. Each
  violates a security, compatibility, or accessibility contract.

## 6. Tests Required

Run the V2 gates from the repository task API:

```powershell
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
git diff --check
```

- Adapter tests assert every command name, exact camel-case payload, return,
  and error propagation across Skills, MCP, Settings, and external links,
  including V2 seven-value Skill and seven-value MCP identity plus leftover
  backend Gemini / Hermes flag round-trip, disk-observed installed Skills, and
  leftover `discover_available_skills_page` kept for leftover V1 rather than
  the leftover full-list command.
  Adding `search_skillhub` / `install_skillhub` increments the host
  invoke-handler freeze
  in `application_acl_covers_every_registered_command_without_remote_access`
  (currently 334). Rust unit tests cover SkillHub slug/URL pinning, Chinese
  description mapping, official `/api/skills` query (`keyword` / `category` /
  `page` / `pageSize` / `sortBy`), dropping illegal category keys, mapping
  `data.total`, and page-size clamp `0 → 21` / `>50 → 50`.
- Pure tests cover public-field search, secret exclusion, URL/args redaction,
  selection convergence, repository parsing, installed-key matching,
  pagination, env/header/args parsing, advanced JSON validation, extension
  retention, and each MCP catalog builder. Catalog `apps` JSON key order and
  `DEFAULT_NEW_APPS` must match Agent catalog order. Rust unit tests cover
  discovery limit clamp, query/repo/status filters, directory-tail install
  match, and out-of-range offset without hitting the network. Adding
  `discover_available_skills_page` increments the host invoke-handler freeze
  in `application_acl_covers_every_registered_command_without_remote_access`.
- Component tests cover empty, loading, error, pending, write/refetch, dialogs,
  assignment, destructive confirmation, secret-safe presentation, an exhaustive
  seven-ID Skill/MCP icon map, decodable local assets, decorative
  icon semantics, seven unique Skill switches, seven unique MCP switches in
  catalog order, Discover/docs and
  Skill repo clicks through `ExternalLinkButton` → `settings.openExternal`,
  discovery install Dialog targets in that same catalog order (icon + name)
  via shared `InstallTargetDialog` / `AssignmentPanel mode="radio"`, path
  preview from `skillInstallDestination` / `mcpInstallDestination` before the
  host write, MCP
  discovery one-click persisting only the chosen target, WorkBuddy install or
  new assignment showing the connector-trust `Dialog`, MCP config-install
  Agent radio (not `fy-feature-check-grid`), ZIP and restore radio pickers, unmanaged
  import switch rows, host round-trip assign then unassign
  for every V2 Skill target, import sync of missing selected dests, no **管理仓库** in Discover or Installed **更多**, flex list overlay, one
  shared in-flight lock, Skills discovery page-3 offset 42 (21×2), search
  resetting to page 1, clicking **办公效率** sending `office-efficiency`,
  `FeaturePagination` selection plus prev/next and ellipsis,
  discovery card 3-line clamp plus 详情 dialog, Skill 市场 as the only V2
  discovery source without grouping, source tabs, skills.sh, or a `n / m`
  summary line, Skills page-level discovery
  scroll, MCP `.fy-feature-discovery-scroll` overflow, and no overflow on
  `.fy-feature-detail-scroll`.
- Browser tests cover `900x600`, `1152x640`, `1232x700`, and `1440x900`, with
  populated two-/three-column layouts, a single correctly-sized assignment
  panel whose switch accessible names match catalog order, visible split
  separators above 760px, assignment rows contained
  inside their pane, no overlapping list rows, no overflow, no secret
  rendering, exact invoke payloads, and authoritative refetch.
- Browser tests do not replace native Windows Tauri/WebView2 acceptance,
  actual filesystem/config writes, or 125%/150% display-scale review.

## 7. Wrong vs Correct

Wrong: expose every field and replace the server with the quick-form subset.

```ts
const matches = JSON.stringify(server).includes(query);
const next = { type, command, args, env };
throw new Error(JSON.stringify(server));
```

Correct: search only public fields, merge known edits into the canonical
draft, and keep user-visible failure text secret-safe.

```ts
const matches = searchMcpServers([server], query).length > 0;
const next = { ...canonicalSpec, type, command, args, env };
throw new Error("MCP 配置保存失败，请检查配置后重试");
```

Wrong: derive an icon URL dynamically or make its alt text repeat the app
label.

```tsx
<img src={`https://icons.example/${app.id}.svg`} alt={app.label} />
```

Correct: use the exhaustive local V2 map and keep the image decorative.

```tsx
<img src={getSkillTargetIcon(app.id)} alt="" aria-hidden="true" />
```

Wrong: add `AppType::WorkBuddy` so MCP can reuse Provider/session writers.

```rust
AppType::from_str("workbuddy")
```

Correct: keep WorkBuddy, QoderWork, and TRAE Work on `SkillTargetId` /
`McpTargetId` only; `TryFrom` to `AppType` fails.

Wrong: sort Skills/MCP assignment alphabetically or Claude-first.

```ts
const MCP_TARGETS = [...apps].sort((a, b) => a.label.localeCompare(b.label));
```

Correct: keep the Agent catalog sequence as the single list order.

```ts
const MCP_TARGET_IDS = [
  "qoderwork",
  "trae-work",
  "workbuddy",
  "codex",
  "claude",
  "opencode",
] as const;
```

Wrong: make `.fy-feature-list` a CSS Grid so the overlay lens is a grid item.

```css
.fy-feature-list {
  display: grid;
}
```

Correct: use a column flex track; the lens stays out of flow.

```css
.fy-feature-list {
  display: flex;
  flex-direction: column;
}
```

Wrong: fill a split pane with `height: 100%` and leave the feature panel
overflow visible, so bulk-assign buttons paint past the card.

```css
.fy-split-pane > * {
  height: 100%;
}
.fy-feature-assignment {
  display: flex;
  white-space: nowrap;
}
```

Correct: reuse `SplitPanes` child overflow from the catalog rail, and let
assignment rows wrap inside the pane.

```css
.fy-split-pane > * {
  min-height: 100%;
  height: 100%;
  overflow: auto;
}
.fy-feature-assignment {
  flex-wrap: wrap;
  min-width: 0;
}
```

Wrong: MCP Discover opens docs through a page-owned callback and a custom
underline button.

```tsx
<button className="fy-mcp-card-link" onClick={() => onOpen(item.docs!)}>
  文档
</button>
```

Correct: the same `ExternalLinkButton` used by Skills, Agents, and Models.

```tsx
<ExternalLinkButton url={item.docs}>文档</ExternalLinkButton>
```

Wrong: V2 discovery pulls the leftover full list, or discovery cards reuse the
detail column scroller.

```ts
discover: () => invoke("discover_available_skills");
```

```css
.fy-skills-page .fy-feature-workspace > .fy-feature-detail-scroll {
  overflow: auto;
}
```

Correct: page through `searchSkillHub`, keep leftover `discover_available_skills`
for V1, and scroll discovery cards with the shared class.

```ts
searchSkillHub: (query, limit, offset, category = "") =>
  invoke("search_skillhub", { query, limit, offset, category });
```

```css
.fy-feature-discovery-scroll {
  overflow: auto;
}
```

Wrong: dump the full Skill description on the discovery card, or omit 详情.

```css
.fy-feature-card-body {
  flex: 1 1 auto;
}
```

```tsx
<p className="fy-feature-card-body">{skill.description}</p>
```

Correct: clamp the card preview to three lines and open the shared `Dialog`
from **详情**. Skill 市场 cards use **主页** as `ExternalLinkButton`.

```css
.fy-feature-card-body {
  display: -webkit-box;
  overflow: hidden;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}
```

```tsx
<Button onClick={() => setDetailSkill(skill)}>详情</Button>
<Dialog open={Boolean(detailSkill)} title={detailSkill.name} onOpenChange={...}>
  <p className="fy-feature-intro">{skillDetailBody(detailSkill)}</p>
</Dialog>
```

Wrong: hand-roll a five-number page slice.

```tsx
{
  pages.slice(page - 3, page + 2).map((n) => <button key={n}>{n}</button>);
}
```

Correct: reuse `FeaturePagination` (status, 上一页 / 下一页, ellipsis).

```tsx
<FeaturePagination
  page={page}
  totalPages={totalPages}
  ariaLabel="Skill 市场分页"
  onPageChange={setPage}
/>
```

Wrong: V2 discovery still searches skills.sh, groups cards by GitHub repo,
shows **Skill 市场** / **仓库** source tabs, keeps **管理仓库**, labels the
market “中国 Skill 市场”, freezes the catalog at 50, or lists via
`/api/v1/search`.

```ts
searchSkillsSh: (query, limit, offset) =>
  invoke("search_skills_sh", { query, limit, offset });
searchSkillHub: (query, limit) =>
  invoke("search_skillhub", { query, limit: 50, offset: 0 });
```

```tsx
<FeatureTabs
  options={[
    { id: "market", label: "Skill 市场" },
    { id: "repos", label: "仓库" },
  ]}
/>
<Button>管理仓库</Button>
<p>Skill 市场 · 21 / 50 · 将安装到 Claude Code</p>
<h3>{repo} · {items.length}</h3>
```

Correct: Discover is Skill 市场 only. Copy is “Skill 市场”. Host
commands are `search_skillhub` / `install_skillhub`. Cards stay a flat grid.
Pagination uses official `page` / `pageSize=21` and `data.total`. Category
filter is one `FeatureTabs` of the 12 official first-level names.

```ts
searchSkillHub: (query, limit, offset, category = "") =>
  invoke("search_skillhub", { query, limit, offset, category });
installSkillHub: (slug, currentApp) =>
  invoke("install_skillhub", { slug, currentApp });
```

```tsx
<FeatureSearch ariaLabel="搜索 Skill 市场" />
<FeatureTabs
  label="分类筛选"
  value={category}
  options={SKILLHUB_CATEGORY_TABS}
/>
<FeaturePagination
  page={page}
  totalPages={totalPages}
  ariaLabel="Skill 市场分页"
  onPageChange={setPage}
/>
```

Wrong: a page-local Skill target picker, or a per-agent install/assign path.

```tsx
function InstallTargetPicker({ value, onChange }) {
  return SKILL_TARGETS.map((app) => (
    <button key={app.id} onClick={() => onChange(app.id)}>
      {app.label}
    </button>
  ));
}
```

Correct: shared `InstallTargetDialog` wrapping one `AssignmentPanel` for
install / ZIP (radio). Restore and assignment / import stay on
`AssignmentPanel` radio or switch. Host mutations stay `toggle_skill_app` /
`install_skillhub` / `install_from_zip` / `import_from_apps` for every V2
target. MCP discovery one-click uses the same dialog.

```tsx
<InstallTargetDialog
  title={`安装 ${name}`}
  busy={busy}
  defaultTarget={installTarget}
  pathForTarget={(target) => skillInstallDestination(target, directory)}
  onCancel={onCancel}
  onConfirm={onConfirm}
/>
<AssignmentPanel
  apps={skill.apps}
  onToggle={onToggle}
  labelSuffix="Skill 分配"
  targets={SKILL_TARGETS}
/>
```

## Design Decisions

- V2 discovery uses Tencent SkillHub (`skillhub.cn`) as the Skill 市场 API
  because listings include `description_zh`. Do not wire skills.sh, SkillsMP,
  ClawHub, or configured GitHub repos into the V2 discovery tab. Do not keep
  a V2 **管理仓库** surface: GitHub skill-repo CRUD is leftover V1 only and
  is not suitable as the China-facing catalog.
- List and category browse use the official SkillHub find-skill contract
  `GET https://api.skillhub.cn/api/skills`. Do not use `/api/v1/search` and do
  not grow `limit` to fake pagination. `page` / `pageSize` / `data.total` are
  real. `SKILL_DISCOVERY_PAGE_SIZE = 21` fills the 3-column discovery grid
  (7×3). `SKILL_DISCOVERY_MAX_PAGE_SIZE = 50` still clamps page size only.
  Leftover `discoverPage` keeps `0 → 20`.
- First-level categories are the 12 keys published in SkillHub
  `find-skill-skillhub` `categories.md` (办公效率 / 开发编程 / IT 运维与安全
  and the rest). Add a category tab only when that official list includes it.
  Do not invent keys by probing the search API. Secondary tags are not
  `?category=` values.
- Install downloads `GET https://api.skillhub.cn/api/v1/download?slug=`
  (HTTPS, pinned host/path, allowlisted slug) and reuses `install_from_zip`.
  Redirects to Tencent COS are expected. Do not spawn `npx skillhub` or the
  official CLI.
- User-facing copy is **Skill 市场**, not the vendor name and not
  “中国 Skill 市场”. Internal identifiers may still say `skillhub`.
- Card layout follows a marketplace feed (name, Chinese description, category,
  version, author, 安装 / 详情 / 主页), not GitHub repository groups.
