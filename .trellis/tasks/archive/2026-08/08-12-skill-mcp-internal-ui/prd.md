# Skills and MCP V2 internal UI

## Goal

Deliver production-capable Skills and MCP management inside the existing V2 content viewport while preserving all backend commands, persistence semantics, the outer shell, and every non-Skills/MCP page. The result must use V2-only code and provide responsive, accessible, failure-aware workflows rather than a static or mock-only redesign.

## Confirmed facts and boundaries

- `src/v2/pages/skills/Page.tsx` and `src/v2/pages/mcp/Page.tsx` are currently empty Phase 1 pages.
- V2 cannot import Legacy renderer modules and only `src/v2/shared/platform/tauri/**` may import `@tauri-apps/**`.
- Existing Skills and MCP commands already provide the required behavior. Rust commands, parameters, data structures, database schema, and business logic are frozen.
- The TopBar, brand, primary navigation, window controls, ContentViewport shell, route order, light-only tokens, and the Agents, Models, Prompts, and Memory pages are frozen.
- User-visible changes are restricted to the Skills/MCP content areas and their dialogs, confirmations, notices, progress, and toasts. Necessary non-visual V2 adapters, providers, shared controls, local styles, specs, and tests are allowed.
- No external handoff files or images may be copied, linked, or referenced by repository content.
- No new dependency, OpenClaw/Claude Desktop target, backend endpoint, database migration, health probe, V2 localization migration, global Agent switcher, dark theme, or Legacy UI dependency is in scope.

## Requirements

### Shared V2 foundation

- Add V2-owned domain types and typed ports for all existing unified Skills/MCP commands, settings read/save, and safe external links. Browser reads return safe empty values; browser writes reject clearly and never claim success.
- Direct Tauri imports remain below the V2 Tauri adapter boundary. Native mode uses real commands; tests inject fake ports without production fixtures or debug routes.
- Provide the six supported targets only: Claude, Codex, Gemini, Grok Build, OpenCode, Hermes. A session-scoped install target defaults to Claude, survives route changes, resets on app restart, and is not persisted.
- Add only reusable controls needed by both modules: semantic buttons, tabs, inputs/search, select, switch, checkbox, badges, dialog/alert dialog, inline notice, pagination, spinner/progress, and empty/error states. Preserve refs, native props, focus behavior, ARIA, reduced motion, and `--fy-*` styling.
- Use resource caching and authoritative refresh after mutations. Keep the last valid result during refresh failures. A module-level write lock disables only conflicting writes; reading, search, and selection remain available. Bulk writes run sequentially and report progress/partial failure.
- MCP secrets in `env` and `headers` never enter search text, ordinary details, logs, notices, or toast descriptions.
- Keep MCP preset data single-sourced: move the pure preset definition to a V2-legal shared domain module and retain the Legacy path as a compatibility re-export without changing Legacy UI behavior.

### Skills installed management

- Permanent tabs are exactly `已安装` and `发现`. Installed actions are manual update check, conditional `更新全部 · N`, and a More menu containing local import, ZIP install, backup restore, and Skill settings.
- Empty collection uses a complete empty state. Otherwise use a responsive list/detail/assignment master-detail layout: three columns when content fits, two columns with assignment inside detail at medium widths, and no page-level horizontal scrolling.
- Search matches name, id, description, directory, repository owner/name, and `owner/repo`. Select the first filtered item by default; preserve a still-visible selection and otherwise converge to the new first item.
- Details show only available overview, source, installation, and advanced fields. README appears only when available. Technical values use readable code treatment.
- Update status comes only from the existing hash comparison; never invent versions. Support single update, sequential update-all with partial-failure reporting, and authoritative refresh.
- Current-item app switches and full-collection per-Agent bulk management use the six supported targets. Bulk scope is never narrowed by search.
- Uninstall requires danger confirmation explaining unified removal and existing backup behavior, then refreshes installed, backup, update, and unmanaged resources and converges selection.

### Skills discovery and auxiliary workflows

- Discovery is a responsive 3/2/1-card grid with local sources `仓库` and `skills.sh`; an empty repository list stays on the repository source and offers explicit source-changing actions.
- The shared session install target applies to repository, skills.sh, ZIP, and restore installs. Local unmanaged import retains per-item multi-Agent selection initialized from the supported intersection of `foundIn`.
- Repository discovery supports search, repository filter, and installed-state filter. Installed matching uses normalized directory tail plus repository owner/name. Successful install stays on discovery and offers navigation to the newly installed item.
- skills.sh sends no request below two query characters, uses page size 20, numeric pagination from `totalCount`, preserves source/query while paging, scrolls results to top, and does not invent descriptions.
- Repository management accepts `https://github.com/owner/repo(.git)` or `owner/repo`, defaults branch to `main`, and has no subdirectory or enable switch. Removing a repository requires confirmation and does not uninstall Skills.
- ZIP selection, unmanaged scan/import, lazy backup restore/delete, storage migration, and sync-method settings provide pending/error/empty states and refresh every affected authoritative resource.
- Storage migration calls only `migrate_skill_storage`; the backend owns the move-then-persist sequence. Sync method updates merge into a freshly read complete settings object before `save_settings`.

### MCP management

- MCP has no duplicate service/assignment tabs. Header actions are `导入现有` and `添加 MCP`.
- Empty collection uses a complete empty state; populated state uses the same responsive master-detail-assignment grammar as installed Skills.
- Search is an explicit allow-list of id, name, description, tags, type, command, args, cwd, url, homepage, docs, and source. It must not recursively stringify or include env/headers.
- Lists show name, description/tags, transport, and enabled target count. Details show available basics, connection/launch fields, metadata, and danger actions. Secret-bearing maps show counts only with an edit affordance.
- Current-server and full-collection bulk assignment use the same six targets. Do not infer client installation state; surface backend errors and refresh authority.
- Import directly calls the existing no-argument unified command and distinguishes 0, N, and error outcomes.
- Add/edit uses one responsive large dialog without changing the route. It supports Custom plus the five existing presets, stdio/http/sse quick forms, six initial app choices, advanced JSON, metadata, validation, sticky/reachable actions, and duplicate-submit prevention.
- New server defaults enable Claude, Codex, Gemini, and Grok Build; OpenCode and Hermes are off. Hidden app fields remain false unless preserved from an edited object.
- stdio requires command; args are one per line; env uses the first `=`. HTTP/SSE require URL; headers use the earliest valid `:` or `=`. Non-empty malformed lines block saving with line-level errors.
- Advanced JSON must be one non-array server object and rejects a `{mcpServers: ...}` container. Quick-form updates replace known fields but preserve unknown server extensions; invalid JSON cannot switch/save and errors do not echo secret content.
- New IDs are trimmed, required, and checked exactly against the current authoritative list. Edit IDs are immutable. Editing preserves unknown top-level/server fields and hidden application flags. Delete requires confirmation and authoritative refresh.

### Contract migration

- Update the V2 shell contract so Skills/MCP may render business content while Agents, Models, Prompts, and Memory remain empty.
- Preserve route-owned selection, V2 layer and Tauri boundaries, fixed navigation, light-only design, DEV-only UI Lab, shell accessibility, and geometry gates.
- Replace only the obsolete all-six-pages-empty assertions; do not remove or weaken architecture tests.

## Acceptance criteria

- [ ] Native V2 Skills and MCP invoke only the documented existing unified commands through typed V2 Tauri ports; browser writes reject and browser reads are harmless.
- [ ] The frozen outer shell and four unrelated pages have no user-visible or structural change.
- [ ] Skills installed/discovery and every auxiliary workflow above operate with loading, empty, retained-data error, pending, success, confirmation, and partial-failure behavior.
- [ ] MCP listing, secret-safe search/details, assignment, import, preset/custom add, edit, validation, advanced JSON, and deletion operate against the real port.
- [ ] No MCP env/header secret is observable outside an explicitly opened edit form.
- [ ] Keyboard/ARIA/focus behavior and reduced-motion behavior meet the V2 contracts.
- [ ] At 900x600, 1152x640, 1232x700, and 1440x900, both pages and dialogs have no document horizontal overflow and degrade from three to two columns without shell overlap.
- [ ] V2 architecture tests retain the Legacy/Tauri/layer restrictions; production exposes no test fixture or debug route.
- [ ] `mise run lint:v2`, `mise run typecheck:v2`, `mise run test:v2`, `mise run test:v2:browser`, `mise run build:renderer`, `mise run format:check`, and `git diff --check` pass.

## Deferred validation and residual risk

- Browser tests establish deterministic frontend behavior and geometry, not real Windows Tauri/WebView2 integration or filesystem writes.
- Real native behavior, 125%/150% Windows display scaling, and writes against representative local CLI configurations remain a separately reported human/native acceptance gate.
