# Stage 5 implementation baseline — 2026-08-30

## Change boundary

Smallest behavior gaps:

- semantic selected hosts become visually weak when the decorative Lens is
  absent, delayed, hidden or retargeted;
- `FeatureTabs` hand-writes incomplete keyboard/tabpanel behavior despite the
  repository already adopting Radix Tabs;
- `PersistentPrimaryOutlet` statically imports all six routes, mutates visited
  state during render and keeps every visited page alive;
- ToolCluster exposes three focusable no-op controls;
- Agent Skills/MCP assignment writes use a global lock while leaving unrelated
  switches apparently actionable.

The behavior lives in the shared V2 shell/UI/query owners, not in page-level
workarounds. Expected implementation owners are:

- `shared/ui/SelectionLens`, shared selected tokens and host CSS;
- existing `shared/ui/FeatureTabs` as the only Radix adapter;
- `app/router` / `PersistentPrimaryOutlet` for route loading and mounting;
- `widgets/app-shell/ToolCluster` for honest controls;
- one narrow `shared/features` authoritative-assignment controller after the
  Skills/MCP semantics are proven identical.

Explicit non-goals: no backend product redesign, no new global store, no
second design system, no warning suppression, no raised chunk warning limit,
and no claim that browser/mock evidence proves installed macOS/Windows WebView
behavior.

## Production build baseline

`mise run build:renderer` at `0dc448be`/Stage 5 start transformed 745 modules
and emitted:

| Asset | Raw | Gzip |
| --- | ---: | ---: |
| `main-D8daCyll.js` | 881.21 kB | 278.51 kB |
| `index-B5_2o2T_.js` | 2.16 kB | 1.10 kB |
| `main-eXvVzKn5.css` | 77.54 kB | 13.05 kB |

Vite emitted the default `>500 kB` warning. No primary route chunks existed;
`PersistentPrimaryOutlet` statically imported all six page modules.

## Lifecycle baseline

- one primary route was visible, but every previously visited route remained
  mounted under `hidden`/`inert`;
- Models repeated a visited-target keep-alive pattern;
- `SelectionLens` recursively observed the complete track subtree and used a
  child-list `MutationObserver`;
- Search, Settings and Account were keyboard stops with a shared `noop`;
- Agent assignment controls silently returned while another row was pending.

## Evidence boundary

Browser/component tests can prove routing, semantics, query creation,
observer ownership and keyboard behavior. Installed-app UAT on macOS Tauri and
Windows WebView2 remains separate residual evidence.
