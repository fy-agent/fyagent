# V2 Prompts and Memory Native Business Contract

> **Frontend Interaction V3 integration note — 2026-08-26**
> `/agents` may expose a selected-Agent prompt library/editor entry and link to
> `/prompts`, but the supported `PromptAppId` set and native prompt port in this
> contract remain authoritative. Unsupported targets render truthful unavailable
> or read-only guidance rather than a local-only successful toggle. `/memory`
> receives the V3 left shell and visual hierarchy only; its data ownership,
> persistence and native directory behavior do not change.

## 1. Scope / Trigger

Read this contract before changing the V2 Prompts or Memory pages, their
`FeaturePorts`, query hooks, Tauri/browser adapters, tests, or standalone
preview assertions.

This is a bounded renderer integration over existing native commands. It does
not authorize new Tauri commands, ACL entries, arbitrary filesystem access,
database migrations, automatic imports, automatic file writes, or cross-tool
memory synchronization. V2 pages must not import legacy hooks or Tauri APIs
directly; all effects cross `FeaturePorts` and the existing
`src/v2/shared/platform/**` boundary. Reuse is the default: Prompts and
Memory share `FeatureSearch`, `FeatureList`, and Memory type `FeatureTabs`.
New chrome that the other page will need goes in `src/v2/shared/ui` on the
first commit. See [Frontend Reuse](./reuse.md).

## 2. Signatures

Prompts support exactly the native applications whose prompt backends already
exist:

```ts
export type PromptAppId =
  | "claude"
  | "codex"
  | "gemini"
  | "grokbuild"
  | "opencode"
  | "openclaw"
  | "hermes";

export type ManagedPrompt = {
  id: string;
  name: string;
  content: string;
  description?: string;
  enabled: boolean;
  createdAt?: number;
  updatedAt?: number;
};

export interface PromptsPort {
  getAll(app: PromptAppId): Promise<ManagedPrompt[]>;
  getCurrentFileContent(app: PromptAppId): Promise<string | null>;
  upsert(app: PromptAppId, prompt: ManagedPrompt): Promise<void>;
  delete(app: PromptAppId, id: string): Promise<void>;
  enable(app: PromptAppId, id: string): Promise<void>;
  importFromFile(app: PromptAppId): Promise<string>;
}
```

Long-term Memory supports only four stable resource identifiers:

```ts
export type MemoryDocumentId =
  | "openclaw-memory"
  | "openclaw-user"
  | "hermes-memory"
  | "hermes-user";

export interface MemoryPort {
  readDocument(id: MemoryDocumentId): Promise<string | null>;
  writeDocument(id: MemoryDocumentId, content: string): Promise<void>;
  getHermesLimits(): Promise<HermesMemoryLimits>;
  setHermesEnabled(kind: "memory" | "user", enabled: boolean): Promise<void>;
  listDailyFiles(): Promise<DailyMemoryFileInfo[]>;
  readDailyFile(filename: string): Promise<string | null>;
  writeDailyFile(filename: string, content: string): Promise<void>;
  deleteDailyFile(filename: string): Promise<void>;
  searchDailyFiles(query: string): Promise<DailyMemorySearchResult[]>;
  openOpenClawDirectory(subdir: "workspace" | "memory"): Promise<void>;
}
```

Concrete response types may carry native metadata such as existence, modified
time, character limit, or enabled state, but may not widen the identifiers or
paths above.

## 3. Contracts

### Prompt behavior

- The page defaults to Claude. Application selection is page-local state and is
  not written to preferences. The application rail follows
  `PRODUCT_DIRECTORY` prompt members (Grok Build, Codex, Claude Code,
  OpenCode) then `PROMPT_ONLY_DIRECTORY` (Gemini, OpenClaw, Hermes). Claude's
  display name is Claude Code. The rail shows each application's authoritative
  enabled count, not a generic “提示词库” placeholder.
- Search filters the library list only. It must not replace the selected
  prompt with `filtered[0]`. A selected prompt stays in the editor even
  when the current query hides that row or matches nothing. Do not replace
  the editor with the empty-search state while a prompt is selected.
- Each application has an independent prompt collection and live-file query.
  Enabling one prompt uses the backend's single-enabled invariant; the result
  shown in the UI comes from the authoritative reread.
- Creating and editing happen inline in the detail pane. The prompt body is
  the first readable surface after the compact title row. Name and
  description stay on a secondary identity row and must not sit above a
  min-height textarea that hides the body. The library list uses
  `FeatureList`; the workspace search uses `FeatureSearch`. Deletion and
  dirty-discard use
  the shared ConfirmDialog. An enabled prompt must be disabled before
  deletion. Do not open a Dialog to read or edit prompt content.
- Import is explicit. Initial load only reads; it never imports, enables, or
  writes a prompt.
- The live-file inspector is collapsed by default under the editor. It
  reports the current native file content and is not an editable second
  source of truth. Do not keep it as a third always-open column that steals
  width from the prompt body.
- Claude Desktop is intentionally absent because the native prompt backend does
  not support it.

### Memory behavior

The fixed resource mapping is:

| Resource ID       | Native resource                 | Editable title/path |
| ----------------- | ------------------------------- | ------------------- |
| `openclaw-memory` | `workspace/MEMORY.md`           | No                  |
| `openclaw-user`   | `workspace/USER.md`             | No                  |
| `hermes-memory`   | `memories/MEMORY.md` / `memory` | No                  |
| `hermes-user`     | `memories/USER.md` / `user`     | No                  |

- Missing OpenClaw long-term files render as not yet created and are created
  only by an explicit Save.
- Only Hermes documents expose enabled switches and native character limits.
  Over-limit content is warned about but remains saveable because the native
  runtime may truncate it.
- Long-term source groups show `MEMORY.md` / `USER.md` in the list. The
  accessible name and editor title keep `OpenClaw ·` / `Hermes ·`. Pass that
  accessible name through `FeatureListItem` `ariaLabel`; do not fork a
  page-local list button to get a different label than the visible title.
- Daily memory is restricted to OpenClaw `workspace/memory/YYYY-MM-DD.md`.
  The adapter validates the filename before invoke, and the backend validates
  it again. Long-term vs daily uses `FeatureTabs`; document and daily files
  use `FeatureList`; daily search uses `FeatureSearch`.
- Open-today creates no file until Save. Search is debounced by 300 ms. Daily
  deletion always requires shared confirmation.
- Opening the OpenClaw workspace or memory folder uses
  `openOpenClawDirectory`. That is not an HTTP(S) jump; do not route it
  through `ExternalLinkButton`.
- No session inventory, tool scan, refinement draft, cross-tool target, or
  simulated synchronization task belongs in this page.

### Query, write, and navigation behavior

- Query keys are partitioned by app, document, daily file, search string, and
  Hermes limits. A mutation invalidates only resources it can affect.
- Each page owns a mutual-exclusion write lock. Repeated clicks while a write is
  pending do not send a second invoke.
- After a successful native write, reread the authoritative resource. If the
  reread fails, retain prior cached data and warn that the write may have
  completed but refresh failed; do not announce synchronized state.
- Application, document, tab, daily-file, and route transitions share the same
  dirty-discard confirmation flow. Do not use `window.confirm`.
- Prompts select the application with `CatalogMasterDetail`, not a
  `<select>`. The workspace is a two-pane list + inline editor. Memory is
  a two-pane source/file list + inline editor. Do not keep a third
  “记忆信息” or “使用说明” column. Path, character count, Hermes toggle,
  and directory actions stay in a compact editor head. The body textarea
  fills the remaining pane height with `min-height: 0` and pane overflow;
  do not use a 220/330/450px editor min-height that forces users to drag
  a splitter before they can read. Users must be able to read the body
  without dragging a splitter.
- Prompts and Memory reuse the V2 `fy-feature-*`, `fy-control-*`, shared UI
  primitives, `CatalogMasterDetail` where a source rail is needed, and the
  shared `SplitPanes` chassis (independent scroll, 14px gutter,
  pointer/keyboard resize). Split-pane children fill the pane and scroll
  inside it; page CSS must not set `height: 100%` without overflow or let
  editor/assignment rows paint past the panel. Page CSS is limited to
  namespaced editor height, scrolling, and responsive arrangement; it must
  not create an independent dark-blue theme.

### Platform boundary

- The Tauri adapter calls only registered, already-authorized native commands.
  It validates app/document/file inputs and parses unknown IPC output before
  returning typed state.
- The browser adapter rejects every operation with a recognizable native-only
  error. Browser UI distinguishes that state from a real empty collection.
- Browser and standalone preview data contains no seeded prompts, memories,
  private counts, or simulated successful operations.

## 4. Validation & Error Matrix

| Condition                                     | Required result                                             |
| --------------------------------------------- | ----------------------------------------------------------- |
| Unsupported prompt app                        | Reject before native invoke                                 |
| Unknown long-term resource                    | Reject before native invoke                                 |
| Daily filename outside `YYYY-MM-DD.md`        | Reject before native invoke                                 |
| Malformed prompt, limit, file, or search IPC  | Fail closed; render an error, never typed fake state        |
| Browser operation                             | Native-only state, not empty success or sample data         |
| Initial page load                             | Reads only; no import, enable, save, or file creation       |
| Empty collection                              | Application-specific empty state                            |
| Search matches nothing                        | No-results state distinct from empty collection             |
| Missing OpenClaw long-term file               | Not-created state; file remains absent until Save           |
| Enabled prompt deletion                       | Block deletion and require disable first                    |
| Mutation is already pending                   | Disable/ignore duplicate action; one native invoke          |
| Native mutation fails                         | Preserve baseline; report failure; no success claim         |
| Mutation succeeds, authoritative reread fails | Preserve cached baseline and show refresh warning           |
| Dirty transition requested                    | Confirm discard before changing app/resource/tab/file/route |
| Hermes content exceeds native character limit | Warn visibly but allow explicit Save                        |
| Prompt/Memory page introduces private theme   | Static/style review and browser acceptance fail             |

## 5. Good / Base / Bad Cases

- **Good:** A user enables a Codex prompt, the page rereads Codex prompts and
  its live file, then renders the native single-enabled result without changing
  Claude's cache.
- **Base:** Browser preview opens Prompts or Memory and renders the shared V2
  layout with an explicit desktop-capability notice and no business records.
- **Good:** A missing OpenClaw `USER.md` opens as not yet created, stays absent
  during navigation, and is written only after the user edits and saves it.
- **Bad:** The page imports a legacy hook, accepts a typed path, seeds example
  memories, assumes a successful mutation updated the live file, or uses an
  independent gradient/card/button theme.

## 6. Tests Required

Focused Vitest coverage must prove:

- every Prompt command name/payload and all seven app IDs in Agent-catalog
  order (`grokbuild`, `codex`, `claude`, `opencode`, then Gemini / OpenClaw /
  Hermes);
- all four long-term resource mappings, Hermes limits/toggles, and daily
  CRUD/search/open-directory payloads;
- rejection of invalid identifiers, filenames, and malformed IPC output;
- browser native-only errors with no prototype records;
- injected stateful page ports for app/resource isolation, search, CRUD,
  import, enable/disable, authoritative reread, write locks, error states, and
  dirty guards;
- negative assertions for retired prototype/sync/session language and
  page-specific theme material;
- existing ACL registration/permission coverage, with no new capability entry.

Run the complete V2 and desktop gates listed by
[V2 Shell Contract](./v2-shell.md), then run `mise run check`. Generating the
gitignored standalone preview is a local optional check, not Required CI
evidence. A real native smoke is read-only on
the current profile; actual write HIL is not required without an isolated
test-hook profile.

## 7. Wrong vs Correct

Wrong: import native or legacy effects into a page and trust an asserted DTO.

```ts
import { invoke } from "@tauri-apps/api/core";
const prompts = (await invoke("get_all_prompts")) as ManagedPrompt[];
```

Correct: request the closed V2 port and let the platform adapter validate
unknown data.

```ts
const prompts = await ports.prompts.getAll("codex");
```

Wrong: treat a successful write as a synchronized local state transition.

```ts
await ports.prompts.enable(app, id);
setPrompts((items) => items.map(markEnabledLocally));
```

Correct: invalidate only the affected resources and render the authoritative
reread, with a distinct warning if refresh fails.

```ts
await ports.prompts.enable(app, id);
await Promise.all([refetchPrompts(), refetchLiveFile()]);
```
