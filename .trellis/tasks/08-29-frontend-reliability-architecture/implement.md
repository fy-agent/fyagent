# Implement — Stage 5

## 1. Baseline and SPEC revision

- [x] Record production build chunk graph, initial route requests, mounted page/query/observer counts and current React warnings.
- [x] Reproduce/instrument left-nav selected-state behavior in browser; record installed native WebView confirmation as an explicit residual gap.
- [x] Update V2 Shell, State Management, Reuse and Quality specs before changing their implementation assumptions.
- [x] Create a #141 current-main frontend finding matrix without inheriting historical verdicts.

## 2. Selected state and Lens

- [x] Add host-owned selected CSS for SideNavigation, FeatureTabs and reviewed Catalog consumers.
- [x] Add common selected tokens/recipe when at least two consumers share semantics.
- [x] Make Lens decorative and non-blocking.
- [x] Replace subtree observers with active-host/track observation.
- [x] Add no-Lens, no-ResizeObserver, delayed observer, hidden reveal, reduced-motion and right-side-interaction tests.
- [x] Verify focus-visible and selected treatment coexist without overlap or contrast loss.

## 3. Tabs

- [x] Wrap Radix Tabs inside the existing `FeatureTabs` owner.
- [x] Add stable trigger/panel IDs, keyboard activation policy and panel semantics.
- [x] Migrate all current V2 FeatureTabs consumers.
- [x] Remove page-local/parallel exclusive-tab behavior and update tests.

## 4. Honest controls and assignment writes

- [x] Resolve Search/Settings/Account individually: remove unowned placeholders without a keyboard stop.
- [x] Remove `noop` from production ToolCluster.
- [x] Select and implement Skills/MCP mutation concurrency policy.
- [x] Extract a shared authoritative-assignment controller after verifying identical semantics.
- [x] Show busy/disabled state; never drop an apparently valid click.
- [x] Preserve authoritative reread, rollback UI and WorkBuddy trust-dialog behavior.

## 5. Route and state architecture

- [x] Convert six primary routes and UI Lab to lazy module loaders.
- [x] Remove render-phase state updates in `PersistentPrimaryOutlet`, ModelsPage and additional findings.
- [x] Classify local state for each route: URL/query/draft/transient/secret.
- [x] Remove blanket visited-page keep-alive.
- [x] Add route/domain draft controllers and blockers only where required.
- [x] Stop inactive polling/subscriptions/observers through active-route mounting and query ownership.
- [x] Ensure backend jobs recover through authoritative query on remount.

## 6. Modular review

- [x] Produce responsibility diagrams for Models, Skills, MCP, Memory, Prompts and MCP catalog.
- [x] Extract only modules with clear props/domain/test/lazy-load boundaries; record rejected line-count-only splits.
- [x] Keep route orchestration local; promote shared UI/features at the first real second consumer.
- [x] Add architecture tests for FeatureTabs ownership, FeaturePort-only Tauri access, no render-phase visited state and no page-local duplicate selected recipes where mechanically enforceable.

## 7. Performance

- [x] Add route chunk/build manifest contract.
- [x] Eliminate the production monolithic app main chunk warning through lazy routes and focused dependency splitting.
- [x] Review remaining vendor chunks and document budget/source.
- [x] Measure route switching and bound DOM/query/observer lifecycle after refactor.
- [x] Do not raise warning thresholds or add broad memoization without evidence.

## 8. Test warning cleanup

- [x] Fix all current targeted `act(...)` warnings in Codex installer, Prompt, Memory and Radix dialog tests encountered by V2 suites.
- [x] Add an exact unexpected React act-warning fail-fast guard without suppressing other errors.
- [x] Keep upstream exceptions exact, versioned and temporary; no exception was required.
- [x] Run browser tests at all configured viewports.
- [ ] Run installed macOS Tauri and Windows WebView2 UAT; not available in this execution and retained as residual evidence.

## 9. Same-domain defect loop

- [x] For every new shell/selection/Tabs/route/query/assignment/responsive defect found, add a minimal regression test before or with the fix.
- [x] Record the root owner and whether a shared extraction was made or rejected.
- [x] Keep backend/product-scope findings outside this task and record residual owners.

## Validation

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run test:desktop:mock
mise run test:desktop:visual:preflight
```

Also run the renderer/V2 architecture suites and `mise run check:contracts` after SPEC/task changes. Native installed-app UAT remains separate evidence.

## Stop conditions

- Semantic selected state disappears without Lens.
- A route loses required unsaved state without an explicit product decision.
- Hidden routes continue polling or retain unbounded observers/DOM.
- A new global store or second design system is introduced without reviewed necessity.
- Test warnings are suppressed rather than fixed.
- Chunk warnings are hidden by increasing limits.

If any condition occurs, revert the affected slice while retaining earlier independent reliability fixes.
