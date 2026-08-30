# Current frontend architecture gap review

## Required full-contract read

Before implementation and final review, read the complete `.trellis/spec/frontend/v2-shell.md`. It is intentionally omitted from automatic JSONL injection because it exceeds the configured context-file size limit; this research note is not a replacement for the authoritative shell contract that this task must deliberately revise.

## Selected-state dependency

`src/v2/app/styles/shell.css` currently makes the selected navigation host transparent and removes its shadow/border. `src/v2/app/styles/features.css` similarly leaves selected FeatureTabs transparent. The visible selected surface is therefore supplied by `SelectionLens`.

`src/v2/shared/ui/SelectionLens.tsx` recursively registers a ResizeObserver on the track subtree, maintains a MutationObserver for added/removed descendants, reads host/track geometry and computed border radius, then drives four Motion values. This creates a timing window in which a delayed or hidden-surface measurement can leave the semantic host without a visible selected surface.

The user's reported behavior—left selected control becoming dark/unclear after interacting on the right—is consistent with this architecture. Native WebView reproduction is still required; the code-level invariant is already insufficient because selection must remain visible when decoration fails.

## Tabs

`FeatureTabs.tsx` hand-writes `role=tablist`, `role=tab`, `aria-selected` and click selection. It does not own complete roving focus, Arrow/Home/End or tab-panel relationships. `@radix-ui/react-tabs` is already installed, so this is a direct reuse opportunity behind the existing FyAgent wrapper.

## Route lifecycle

`PersistentPrimaryOutlet.tsx` statically imports all six pages and stores a visited set. It calls `setVisited` while rendering and keeps every visited page mounted through `PersistentSurface`.

`ModelsPage` repeats the pattern for model targets and also calls `setSessionTarget` / `setVisitedTargets` during render. These patterns make state persistence implicit, load all page code up front and retain hidden DOM/hooks/queries/observers.

The current V2 Shell SPEC explicitly requires this behavior, so implementation repair must revise the contract rather than creating a code/SPEC mismatch.

## Misleading controls

`ToolCluster.tsx` binds Search, Settings and Account buttons to one `noop` function. They are visible, focusable and tested for reachability despite producing no user-visible result.

`AgentAssignmentSections.tsx` uses one global `pendingId`; when any assignment is pending, another click returns immediately. Only the current row is disabled, so other switches appear actionable while their clicks are discarded.

## Size and performance

Current large V2 modules include approximately:

- Models Page: 1,750 lines;
- Skills Page: 1,274 lines;
- MCP catalog: 1,136 lines;
- Memory Page: 996 lines;
- MCP Page: 983 lines;
- Agent Page CSS: 957 lines;
- Prompts Page: 838 lines;
- OpenCode Models Panel: 799 lines.

Line count is not itself a defect, but it identifies modules that need responsibility diagrams before route lazy loading and state extraction.

The reviewed production build emitted one app main JavaScript chunk around 855.76 KB (271.34 KB gzip) and triggered Vite's >500 KB warning. All primary pages are static imports, so route-level code splitting is currently absent.

## Test reliability

Focused V2 lint/type/unit/browser gates pass, but unit output contains repeated React `act(...)` warnings around Codex installer and Radix dialog interactions in Prompt/Memory tests. Passing assertions do not make those timing warnings harmless; they must be fixed or exactly attributed.

## Reuse conclusion

Do not rewrite V2. The concrete shared improvements are:

- host-owned selected-state tokens/recipe;
- existing FeatureTabs as the sole Radix adapter;
- one authoritative assignment controller if Skills/MCP semantics match;
- typed visual action-status primitives without merging domain state machines;
- explicit route/domain draft ownership instead of blanket keep-alive.
