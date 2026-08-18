# V2 Shell and Cross-Agent Prompt / Memory Contract

## 1. Scope / Trigger

Read this contract before changing `src/v2/**`, the V2-only test/configuration
files, or the renderer entry that selects `src/v2/main.tsx`. It is a narrow V2
exception to the legacy renderer conventions: the existing frontend specs
remain authoritative for every path outside the V2 boundary.

The current production slice has a deep-blue shell and two implemented business
pages at `#/prompts` and `#/memory`. The other four routes remain empty
placeholders. Both pages are interactive frontend prototypes: their state is
local and must not be represented as connected to native commands, Agent files,
or durable storage.

Production V2 code uses this structure:

```text
src/v2/
|- app/                  # composition root, router, errors, and styles
|- pages/<route>/        # one folder for each first-level route
|- widgets/app-shell/    # visible shell composition
|- shared/               # config, assets, UI, design system, and platform ports
`- dev/                  # development-only UI Lab
```

Do not create empty `entities`, `features`, store, or service layers ahead of a
real V2 data boundary.

## 2. Signatures

Navigation and native-window behavior use these exact internal contracts:

```ts
export type NavigationItem = {
  id: "agents" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path: "/agents" | "/models" | "/skills" | "/mcp" | "/prompts" | "/memory";
  label: string;
};

export interface WindowFramePort {
  isNative: boolean;
  platform: "browser" | "windows" | "macos" | "linux" | "unknown";
  prepareFrame(): Promise<void>;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
}
```

The lifecycle-ready operation returns `Promise<void>` and owns a module-level
promise/state guard. Its native side effect is the existing payload-free
`frontend-deeplink-ready` event.

Prompt and Memory prototype state uses these executable shapes:

```ts
export type AgentTargetId =
  | "codex-global"
  | "claude-global"
  | "gemini-global"
  | "opencode-global"
  | "openclaw-default"
  | "openclaw-group"
  | "hermes-global";

export interface PromptPrototypeItem {
  id: string;
  name: string;
  description: string;
  content: string;
  enabled: boolean;
  kind: "builtin" | "custom";
  category: PromptCategory;
  origin: "本机规则提炼" | "官方基础模板" | "用户创建";
  targetIds: AgentTargetId[];
  updatedAt: string;
}

export type MemoryCategory = "longTerm" | "daily" | "sessions";
export type PromptPathState = "exists" | "create-on-enable";
export type MemorySyncEligibility =
  | "source-only"
  | "verified-rule-bridge"
  | "verified-native";
export type MemoryResourceState = "exists" | "missing" | "frontend-draft";
export type MemoryLocalState =
  | "source"
  | "saved-preview"
  | "changes-pending"
  | "managed-by-prompts";

export interface MemoryPreviewTargetTask {
  targetId: AgentTargetId;
  sourceRevision: number;
  previewState: "pending";
  durableState: "not-run";
  createdAt: string;
  error: null;
}
```

The shared Agent catalog lives in `shared/config/agentTargets.ts`. Prompt and
Memory pages must reuse it instead of maintaining different lists or filenames.
Targets represent a concrete tool scope/workspace/path, not merely an app name.
Each target has explicit `promptPathState`, Prompt-only
`promptCanonicalResourceKey`, and `memorySyncEligibility`. The current seven
Prompt resources cover eight Agent instances because OpenClaw `main` and
`utility` share one workspace. Prompt grouping deduplicates that canonical
resource while retaining every covered instance ID. Memory derives its four
verified adapter/scope destinations from eligibility and must not use the Prompt
canonical key to infer `MEMORY.md` / `USER.md` identity.

## 3. Contracts

### Navigation and content

The navigation source contains exactly these entries in this order:

| ID        | Path       | Label        |
| --------- | ---------- | ------------ |
| `agents`  | `/agents`  | `Agent 目录` |
| `models`  | `/models`  | `模型`       |
| `skills`  | `/skills`  | `Skills`     |
| `mcp`     | `/mcp`     | `MCP`        |
| `prompts` | `/prompts` | `提示词`     |
| `memory`  | `/memory`  | `记忆`       |

- Use a hash data router. The index route and every unknown route redirect to
  `/models`; the stable default URL is `#/models`.
- Derive selected state only from router location. The active link has
  `aria-current="page"`; do not maintain a second `currentView` state.
- Put each production page element below its matching `pages/<route>/` folder.
  `prompts` and `memory` render the workspaces described below; `agents`,
  `models`, `skills`, and `mcp` render no business content yet.
- Register the UI Lab only when `import.meta.env.DEV` is true. Production must
  not expose `#/__dev/ui-lab`.

### Cross-Agent Prompt prototype

- The supplied `1586 x 992` prototype remains the shell and three-pane visual
  authority. Its former Codex-only labels and one-enabled rule are superseded
  by this product contract.
- Prompt is a global best-practice instruction library, not a current-App
  settings page. Do not render a Codex context pill or a single current-App
  inspector.
- Seed the library from anonymized rule domains observed in the local Agent
  tools, then calibrate them against official instruction-file contracts. The
  current scenarios are communication style, goal/boundary/evidence, context
  loading, planning, implementation discipline, review, memory continuity,
  troubleshooting, and heartbeat boundaries. Never copy private local rule or
  memory text into prototype data.
- Every prompt owns an independent `enabled` state and zero or more
  `targetIds`. Multiple prompts may be enabled simultaneously. Toggling one
  prompt must never disable another.
- Toggling a saved prompt that is not currently selected must not select it,
  replace the current editor, or disturb the current dirty draft. The toggle
  commits only that saved item's `enabled` field.
- An enabled prompt must retain at least one target. A disabled draft may have
  no target. Saving or enabling an item without a target shows local validation
  feedback and does not mutate the saved state.
- The right pane is `注入目标`: it shows tool, scope/instances, actual path,
  file-existence state, target-file count, and covered-instance count. Target
  rows are multi-select controls. Missing Gemini/OpenCode instruction files are
  labelled `启用时创建`, never presented as already configured.
- Current data comes only from `pages/prompts/prototype.ts`. The page root has
  `data-data-source="prototype"`; search, selection, creation, editing, save,
  enable, and assignment remain local React interactions.
- Future file synchronization must compose all enabled prompts applicable to a
  concrete resource in stable order. It must deduplicate shared canonical paths,
  own a managed block, and preserve content outside that block; overwriting an
  entire Agent file is forbidden.

### Cross-Agent Memory prototype

- Memory uses three concrete lifecycle categories grounded in the detected local
  stores: `长期记忆`, `每日记录`, and `会话记录`. Cross-Agent sharing is a
  long-term-memory sync action, not a fourth top-level concept.
- Every item exposes tool, source scope/instances, purpose, path, storage kind,
  read/write/search capabilities, item count, update time, owner, and sync state.
  Markdown, JSON/JSONL, and SQLite are distinct capabilities; do not infer edit
  permission from a filename alone.
- `长期记忆` includes user profiles, stable memory files, and derived/indexed
  memory. Writable items support edit/save, target multi-select, and a local
  sync-task preview. The current target list contains four verified destinations:
  Claude's locally referenced memory directory plus the native memory files for
  two OpenClaw workspaces and Hermes. Codex-derived memory, Gemini sessions, and
  OpenCode's unreferenced maintenance file are sources only; database-derived or
  adapter-read-only items stay read-only.
- `每日记录` and `会话记录` preserve their original source. A user may promote
  the selected source into a new long-term-memory draft while keeping
  provenance; promotion never edits or deletes the original.
- Instruction, identity, tools, and heartbeat resources discovered in a memory
  scan use `owner="prompts"`, remain read-only here, and explicitly route the
  user to Prompt management to avoid two writers for one resource.
- Current data comes only from `pages/memory/prototype.ts`. The page root has
  `data-data-source="prototype"`; all writes and sync feedback stay in local
  React state and must not imply native persistence.
- Category, item, rescan, and promotion actions protect dirty drafts with a
  discard confirmation. Read-only resources and sessions must never appear
  persistently editable.
- Title normalization happens before dirty comparison. A title that differs
  only by surrounding whitespace is a clean no-op and must not increment the
  saved revision or invalidate existing preview tasks.
- Future synchronization must retain provenance and expose per-target status;
  it must not silently treat a successful local save as successful Agent sync.

### Standalone preview

- `pnpm build:renderer` generates `FyAgent-前端交互预览.html` with the production
  CSS, JavaScript, and image assets inlined. It opens at `#/prompts` under
  `file://` and must navigate to `#/memory` without a local server.
- Directly opening source or built `index.html` under `file://` must route to
  the standalone preview without blocked module requests or console errors.
  HTTP and Tauri launches keep the normal Vite entry behavior.
- The standalone file is generated output; behavior changes belong in V2
  source code and `scripts/build-v2-preview.mjs`, never hand-edited in the
  generated artifact.

### Styling and text

- V2 owns its deep-blue globals, motion, primitives, and semantic CSS custom
  properties. Every V2 semantic token starts with `--fy-`; the base is
  `#172d43`, primary text is `#f5f8fc`, and the selected blue is `#1967b5`.
- Do not import legacy `src/index.css`, legacy theme tokens, UI wrappers, or
  `src/i18n/**`. Current production labels are fixed Simplified Chinese
  literals; multilingual stress strings belong only in the UI Lab.
- Namespace V2 selectors. Do not add blanket positioning, globally hide
  scrollbars, use `transition: all`, animate layout/backdrop blur, or ignore
  `prefers-reduced-motion`.
- At `1586 x 992`, the content shell is approximately `x=23..1563`. Both pages
  use the approved three-pane developer-tool layout and preserve usable pane
  scrolling at narrower supported sizes.

### Layer and platform boundaries

Dependencies point downward only:

```text
app -> pages, widgets, shared, dev (DEV-only)
pages -> shared
widgets -> shared
shared -> third-party packages or other shared modules
dev -> shared
```

No V2 module may import legacy `src/App.tsx`, `src/main.tsx`,
`src/components/**`, `src/hooks/**`, `src/lib/**`, `src/i18n/**`, or
`src/index.css`. `pages`, `widgets`, and `app` must not import
`@tauri-apps/**` directly. All direct Tauri imports live below
`src/v2/shared/platform/tauri/**`; consumers depend on `WindowFramePort` or a
future V2 domain data source.

Browser window methods resolve safely without side effects while the preview
still renders Windows controls. The Windows adapter prepares the frame with
`setDecorations(false)` and delegates minimize, toggle-maximize, and close to
the current Tauri window. Dragging is enabled only on explicit empty header
regions, never on navigation or controls.

The ready lifecycle emits at most once per renderer lifetime, including under
React StrictMode or repeated calls, and is a browser no-op. It preserves only
the minimum host activation handshake: this renderer does not yet restore the
complete startup contract or Prompt/Memory persistence, so it is not
Release-ready.

## 4. Validation & Error Matrix

| Condition                                         | Required result                                                         |
| ------------------------------------------------- | ----------------------------------------------------------------------- |
| Empty hash, root route, or unknown route          | Redirect to `#/models`; Models link alone has `aria-current="page"`     |
| Browser calls any `WindowFramePort` method        | Resolve without throwing and without a native side effect               |
| React StrictMode or callers repeat ready          | One native `frontend-deeplink-ready` emission for the renderer lifetime |
| Production requests the UI Lab path               | Route is absent and the wildcard fallback selects `#/models`            |
| V2 imports a legacy module                        | ESLint and the executable architecture test fail                        |
| Non-Tauri-boundary code imports `@tauri-apps/`    | ESLint and the executable architecture test fail                        |
| A route other than Prompt/Memory renders copy     | Shell/content test fails                                                |
| Prompt/Memory calls native persistence            | Architecture/review gate fails; prototypes must remain local            |
| Enabling a prompt disables another prompt         | Prompt interaction test fails                                           |
| Enabled prompt has no target                      | Action is rejected with local validation feedback                       |
| Prompt target choice is limited to one Agent      | Prompt interaction test fails                                           |
| Memory uses Agent names or abstract shared/native tabs | Memory interaction/content test fails                               |
| Daily/session or adapter-read-only memory is edited | Edit/save remains unavailable                                         |
| Shared workspace is rendered/written once per instance | Target-count and path-deduplication tests fail                      |
| Prompt-owned context is editable from Memory      | Ownership/content test fails                                            |
| Category/item change discards a dirty draft       | Dirty-guard interaction test fails                                      |
| A direct-open HTML stays blank or logs errors     | Standalone Playwright regression test fails                             |
| A supported viewport overflows or overlaps        | Playwright geometry gate fails                                          |

## 5. Good / Base / Bad Cases

- **Good:** Opening `#/prompts` shows nine grounded rule scenarios. `中文与回复风格`
  and `目标、边界与完成证据` stay enabled while their concrete resource targets
  are edited independently.
- **Good:** Opening `#/memory` defaults to `长期记忆`; a daily or session source
  can be promoted into a long-term draft with provenance preserved.
- **Base:** Opening the renderer without a route lands on `#/models`, with all
  six links, three tools, and three Windows controls visible and focusable.
- **Bad:** A prompt switch enforces global exclusivity, Prompt renders a fixed
  Codex application card, Memory uses abstract shared/native categories, a
  shared workspace is written once per instance, or local feedback is described
  as durable Agent synchronization.

## 6. Tests Required

Run the V2-specific project tasks:

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
```

- Prompt unit tests assert grounded local-target wording, simultaneous enabled
  items, target multi-selection, required targets for enabled items, local
  create/save, and dirty-draft protection.
- Memory unit tests assert long-term/daily/session categories, real source
  metadata, long-term save/sync-task preview, daily/session promotion,
  Prompt-owned and adapter-read-only behavior, required sync targets, and
  dirty-draft protection.
- Browser acceptance carries a promoted Daily draft through save, target
  selection, and per-target `pending / not-run` task generation, and separately
  confirms Session sources remain read-only.
- Platform tests assert browser no-ops, Windows decoration/action delegation,
  and one ready emission under repeated calls and StrictMode.
- Architecture tests parse V2 imports and reject legacy dependencies, upward
  layer imports, and direct Tauri imports outside `shared/platform/tauri`.
- Playwright runs at `900x600`, `1152x640`, `1232x700`, and `1440x900`. It
  asserts shell geometry, primary-control visibility, Prompt/Memory workspace
  interaction, standalone navigation, keyboard access, and no relevant console,
  page, or framework-overlay error.
- Capture separate `1586x992` Prompt and Memory runtime screenshots for review.
  Treat them as `runtime_screenshot`; without automated image comparison they
  are not `pixel_diff` evidence.
- The production renderer build must succeed with the UI Lab route omitted.

Real Windows Tauri/WebView2 behavior at the current host scale remains a
separate native acceptance gate. Native 125% and 150% display scaling remain
human acceptance and must not be represented by browser emulation alone.

## 7. Wrong vs Correct

Wrong: encode the retired one-App, one-enabled-prompt model.

```ts
function enablePrompt(id: string) {
  setItems((items) => items.map((item) => ({ ...item, enabled: item.id === id })));
}
```

Correct: enabled state and Agent scope belong to each item independently.

```ts
function togglePrompt(id: string) {
  setItems((items) =>
    items.map((item) =>
      item.id === id ? { ...item, enabled: !item.enabled } : item,
    ),
  );
}
```

Wrong: imply that a frontend-only action wrote an Agent file.

```tsx
<p>已写入所有 Agent</p>
```

Correct: keep the prototype boundary explicit until a real V2 data source and
per-target result contract are implemented.

```tsx
<section data-data-source="prototype">
  <span aria-live="polite">本地交互预览已更新</span>
</section>
```
