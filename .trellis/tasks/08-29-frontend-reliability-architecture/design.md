# Design — V2 Semantic State, Route Lifecycle, and Shared UI

## 1. Principles

1. Semantic state must be visible without animation, observers or backdrop filters.
2. Router/resource/draft/secret state have different owners; keep-alive is not a state-management substitute.
3. Existing adopted primitives own mature interaction behavior; FyAgent wrappers own product semantics and tokens.
4. Shared extraction follows one reason to change together and at least two real consumers.
5. Performance changes are measured and independently reversible.

## 2. Selected-state architecture

The selected host is authoritative:

```text
Router / controlled value
  -> aria-current / aria-selected / data-state
  -> host CSS selected surface (always visible)
  -> optional SelectionLens decoration (best effort)
```

`SelectionLens` no longer registers the only painted state. It receives an active host, measures the host relative to one track, and paints a non-interactive decorative overlay. If no box is available, it renders nothing while host CSS remains correct.

Preferred shared recipe:

```text
--fy-selected-bg
--fy-selected-border
--fy-selected-shadow
--fy-selected-text
data-selected / aria-current / data-state selectors
```

Navigation, Tabs and Catalog can vary radius/layout but derive from one token/recipe owner when their visual contract is the same.

Observer policy:

- observe active host and track/container only;
- one ResizeObserver per active lens group at most;
- optional window resize/scroll hook where geometry actually changes;
- no recursive subtree observation;
- no MutationObserver for unrelated descendants;
- batch measurement in layout effect/animation frame and skip identical geometry.

## 3. FeatureTabs adapter

`FeatureTabs` remains the stable V2 API but wraps Radix:

```text
FeatureTabs.Root   controlled value/onValueChange
FeatureTabs.List   shared selected treatment + optional Lens
FeatureTabs.Trigger
FeatureTabs.Content (or caller-provided panel mapping)
```

The exact public wrapper can preserve current `options` input while adding panel IDs/activation mode. Pages must not know Radix package details. This centralizes keyboard, focus and ARIA behavior and makes visual changes one-owner.

## 4. Route composition

Replace static page imports with route module loaders:

```text
HashRouter
  -> lazy route element
  -> route-level Suspense/Error boundary
  -> active page only
```

Unknown/index redirects remain unchanged. UI Lab remains development-only and lazy.

`PersistentPrimaryOutlet` should be removed or reduced to a composition helper that does not own visited state. A route may request an explicit `keepAlivePolicy` only after a reviewed need; default is unmount.

## 5. State ownership matrix

| State | Owner | Persistence |
| --- | --- | --- |
| Current route/target/tab when shareable | Router or controlled parent | URL/session navigation |
| Backend data | TanStack Query + FeaturePort | cache/backend authority |
| In-flight install/Auth/change-plan job | Backend job + query subscription | survives route renderer lifecycle |
| Unsaved business draft | Domain route hook/controller | explicit until save/discard/navigation decision |
| Search/filter/transient panel | Local component | discarded unless product requirement says otherwise |
| Secret input | Narrow local state | cleared on success/failure/navigation according to security contract |
| Shell selected state | Router | never duplicated in local state |

No render-phase synchronization. URL-to-state reconciliation uses derived values or effects only when a local draft intentionally differs from route authority.

## 6. Query/effect lifecycle

Every feature query accepts an `enabled` parameter or is mounted only in the active route. Query keys remain domain-owned. Hidden surfaces do not poll merely because their component was visited once.

Subscriptions/timers use one cleanup owner. Backend jobs continue independently and are reread on remount; page components do not act as job daemons.

## 7. Draft strategy

Before removing keep-alive from a route, classify its current local state:

- safe to discard;
- reconstructable from URL/query cache;
- unsaved draft requiring navigation blocker;
- secret requiring cleanup.

Create a route-local `use...Draft` controller when one page owns the workflow. Promote a shared draft boundary only if two routes share validation, dirty, save/discard and restoration semantics. Do not introduce a generic form store.

## 8. Authoritative assignment controller

Skills and MCP currently duplicate:

```text
start pending -> call closed mutation -> refetch authority
-> compare requested value -> success feedback
-> on failure refetch and show warning -> clear pending
```

After verifying identical concurrency semantics, place a typed controller in `shared/features`, conceptually:

```text
useAuthoritativeAssignmentMutation({
  mutate,
  reread,
  readValue,
  concurrencyPolicy,
})
```

It returns per-item/global pending state and a typed outcome. Domain components provide copy and trust-dialog behavior. If backend operations cannot safely run in parallel, use global disabled/busy rather than dropping clicks.

## 9. Honest ToolCluster

`ToolCluster` is a composition widget, not a roadmap placeholder. Its inputs should be real action descriptors or it should render only implemented controls. An empty/no-op action is invalid at the type/API level where practical.

Search/settings/account product surfaces remain separate tasks unless already implemented. This task removes the misleading interaction and updates keyboard/geometry tests.

## 10. Large-module decomposition

Perform dependency sketches before moving files. Candidate decompositions:

- `ModelsPage`: route target selection, panel registry/loaders, shared apply workspace, individual product panels.
- Skills/MCP pages: route orchestration vs list/discovery/editor dialogs; retain shared Feature UI.
- MCP catalog: static recipe data/validation separate from UI controller.
- Memory/Prompts: long-term/daily or library/editor panels only where props and tests are stable.
- CSS: shared selected/control/list/dialog recipes vs route layout.

No barrel files with wildcard exports. Route-local modules stay below their page folder unless a true sibling consumer exists.

## 11. Bundle and measurement

Build output should show independent route chunks. Add a lightweight build contract that inspects manifest/chunks rather than relying only on warning text. Suggested evidence:

- initial entry/app chunk sizes;
- route chunk names and dependencies;
- route module request log in Playwright;
- mounted DOM/query/observer count after route switching;
- interaction timing for navigation and target switches.

Budgets are reviewed constants in build/test policy, not a raised `chunkSizeWarningLimit` escape hatch.

## 12. Warning policy

Targeted tests install a console/error guard that fails on unexpected React warnings. Tests must await user-visible settlement using `act`, Testing Library async utilities and explicit fake timers where needed. Do not mock `console.error` globally to silence Radix/React lifecycle messages.

Where an upstream dependency emits a confirmed warning, isolate one exact message/version with an upstream reference and removal condition; broad regex suppression is prohibited.

## 13. Rollout slices

1. Spec correction + CSS selected fallback.
2. Lens observer simplification and robustness tests.
3. FeatureTabs Radix migration.
4. ToolCluster/assignment honesty fixes.
5. Route lazy loading and render-purity cleanup.
6. Explicit draft/query lifecycle per route.
7. Large-module decomposition and bundle budget.
8. Current-main UAT and cross-platform WebView validation.

Each slice remains separately reviewable and revertible.
