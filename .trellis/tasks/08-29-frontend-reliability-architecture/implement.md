# Implement — Stage 5

## 1. Baseline and SPEC revision

- [ ] Record production build chunk graph, initial route requests, mounted page/query/observer counts and current React warnings.
- [ ] Reproduce or instrument the left-nav selected-state dimming after right-side interaction in browser and native WebView where possible.
- [ ] Update V2 Shell, State Management, Reuse and Quality specs before changing their implementation assumptions.
- [ ] Create a #141 current-main frontend finding matrix without inheriting historical verdicts.

## 2. Selected state and Lens

- [ ] Add host-owned selected CSS for SideNavigation, FeatureTabs and reviewed Catalog consumers.
- [ ] Add common selected tokens/recipe when at least two consumers share semantics.
- [ ] Make Lens decorative and non-blocking.
- [ ] Replace subtree observers with active-host/track observation.
- [ ] Add no-Lens, no-ResizeObserver, delayed observer, hidden reveal, reduced-motion and right-side-interaction tests.
- [ ] Verify focus-visible and selected treatment coexist without overlap or contrast loss.

## 3. Tabs

- [ ] Wrap Radix Tabs inside the existing `FeatureTabs` owner.
- [ ] Add stable trigger/panel IDs, keyboard activation policy and panel semantics.
- [ ] Migrate all current V2 FeatureTabs consumers.
- [ ] Remove page-local/parallel exclusive-tab behavior and update tests.

## 4. Honest controls and assignment writes

- [ ] Resolve Search/Settings/Account individually: wire a real owner, remove, or explicitly disable without a keyboard stop.
- [ ] Remove `noop` from production ToolCluster.
- [ ] Select and implement Skills/MCP mutation concurrency policy.
- [ ] Extract a shared authoritative-assignment controller only after verifying identical semantics.
- [ ] Show global/per-item busy or queue state; never drop an apparently valid click.
- [ ] Preserve authoritative reread, rollback UI and WorkBuddy trust-dialog behavior.

## 5. Route and state architecture

- [ ] Convert six primary routes and UI Lab to lazy module loaders.
- [ ] Remove render-phase state updates in `PersistentPrimaryOutlet`, ModelsPage and any additional findings.
- [ ] Classify local state for each route: URL/query/draft/transient/secret.
- [ ] Remove blanket visited-page keep-alive.
- [ ] Add route/domain draft controllers and blockers only where required.
- [ ] Pass `enabled/active` to queries and stop inactive polling/subscriptions/observers.
- [ ] Ensure backend jobs recover through authoritative query on remount.

## 6. Modular review

- [ ] Produce responsibility diagrams for Models, Skills, MCP, Memory, Prompts and MCP catalog.
- [ ] Extract only modules with clear props/domain/test/lazy-load boundaries.
- [ ] Keep route orchestration local; promote shared UI/features at the first real second consumer.
- [ ] Add architecture tests for FeatureTabs ownership, FeaturePort-only Tauri access, no render-phase visited state and no page-local duplicate selected recipes where mechanically enforceable.

## 7. Performance

- [ ] Add route chunk/build manifest contract.
- [ ] Eliminate current monolithic app main chunk warning through lazy routes and focused dependency splitting.
- [ ] Review any remaining large vendor chunk and document budget/source.
- [ ] Measure route switch, DOM/query/observer growth and interaction responsiveness after refactor.
- [ ] Do not raise warning thresholds or add broad memoization without evidence.

## 8. Test warning cleanup

- [ ] Fix all current targeted `act(...)` warnings in Codex installer, Prompt, Memory and Radix dialog tests encountered by V2 suites.
- [ ] Add focused unexpected-warning guards.
- [ ] Keep upstream exceptions exact, versioned and temporary if any are unavoidable.
- [ ] Run browser tests at all configured viewports and native macOS/Windows WebView UAT.

## 9. Same-domain defect loop

- [ ] For every new shell/selection/Tabs/route/query/assignment/responsive defect found, add a minimal failing regression test before or with the fix.
- [ ] Record the root owner and whether a shared extraction was made or rejected.
- [ ] Split backend/product-scope findings instead of expanding this task.

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
