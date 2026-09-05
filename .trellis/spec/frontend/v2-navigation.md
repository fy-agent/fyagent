# V2 Navigation and Persistent Route Contract

## 1. Scope / Trigger

Read this contract before changing V2 primary-route registration, sidebar
groups, hash-router redirects, lazy loading/prefetch, keep-alive page behavior,
hidden-page query isolation, route-leave blocking, keyboard navigation, or the
closed Agent return descriptor.

Primary owners are:

- `src/v2/shared/config/navigation.ts` for the route/group registry;
- `src/v2/app/router.tsx` and `src/v2/app/primaryPages.tsx` for routing and
  literal page loaders;
- `src/v2/app/PersistentPrimaryOutlet.tsx`,
  `src/v2/shared/ui/PersistentSurface.tsx`, and
  `src/v2/shared/ui/usePersistentSearchParams.ts` for visited-page lifetime and
  route-owned query state;
- `src/v2/widgets/app-shell/SideNavigation.tsx` for visible navigation;
- `src/v2/shared/features/agent-navigation.ts` for Agent return semantics;
- `src/v2/shared/ui/PrimaryBlocker.tsx` for one shell-level leave blocker.

Visual material, native title-bar behavior, shared selection-lens geometry and
motion are owned by [V2 Window Shell and Interaction](./v2-window-shell.md).
Feature-specific URL state remains in each feature contract.

## 2. Signatures

The primary registry is a closed literal union:

```ts
type NavigationItem = {
  id: "agents" | "auth" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path:
    | "/agents"
    | "/auth"
    | "/models"
    | "/skills"
    | "/mcp"
    | "/prompts"
    | "/memory";
  label: string;
};

type NavigationGroup = {
  id: "agent-configuration" | "configuration-management" | "memory";
  label: string;
  collapsible: boolean;
  items: readonly NavigationItem[];
};
```

Routing/lifetime boundaries are:

```ts
createAppRouter(): ReturnType<typeof createHashRouter>
initialPrimaryPageId(hash: string): NavigationItem["id"]
preloadInitialPrimaryRoute(hash: string): Promise<void>
prefetchPrimaryRoutes(): void
PersistentPrimaryOutlet(): JSX.Element
PersistentSurface({ active, children, className }): JSX.Element
usePersistentVisibility(): boolean
usePersistentSearchParams(): {
  visible: boolean;
  searchParams: URLSearchParams;
  setSearchParams: SetURLSearchParams;
}
useStickyVisibleValue<T>(visible, explicit, fallback): T
usePrimaryBlocker(rule): ReturnType<typeof useBlocker>
usePrimaryNavigationOrigin(): (element: HTMLElement, destination: string) => void
usePrimaryBlockerOrigin(): DialogOriginRef
```

The only cross-page Agent return value is:

```ts
type AgentReturnDescriptor = {
  agentId: AgentCatalogId;
  section: "models" | "skills" | "mcp" | "prompts";
};

agentReturnPath(descriptor): string
agentReturnDescriptorFromSearch(search): AgentReturnDescriptor | null
agentReturnDescriptorFromManagementSearch(search): AgentReturnDescriptor | null
appendAgentReturnToPath(path, descriptor): string
```

The descriptor is represented by closed query tuples (`target`/`section` on
the Agent page, `agentReturn`/`agentSection` on management pages). It is not an
arbitrary return URL, serialized history entry, or free-form navigation state.

## 3. Contracts

### Registry and router

- `navigationGroups` is the single production owner of primary IDs, paths,
  labels, grouping, and collapsibility. `navigationItems` is derived from it;
  the router and sidebar do not maintain separate route arrays.
- The root index and unknown production paths redirect with replacement to
  `/agents`. The seven primary paths remain hash-router paths so browser and
  Tauri startup share one routing model.
- `__dev/ui-lab` exists only when `import.meta.env.DEV` is true. It must not
  enter production navigation, production bundles as an eager route, or
  release acceptance as an end-user surface.
- Every primary page has one literal dynamic import in `primaryPages.tsx`.
  Literal loaders are required for reviewable chunk ownership and architecture
  tests; do not replace them with a computed import path or page-side registry.
- `prefetchPrimaryRoutes` warms those same cached loaders. Prefetch must not
  create a second module map, render a page, start native queries, or turn a
  failed optional preload into a false successful route transition.
- Startup awaits only the initial hash route's cached module before mounting
  the router, then prefetches the other literal loaders. Unknown paths warm
  Agents, leaving the router's existing redirect authority unchanged. Query
  parameters never become module paths. No new eager bundle or fixed delay.
- Optional prefetch catches rejection and clears only its failed cached
  promise so actual navigation may try loading the module. Actual route or
  initial-load failure renders `RootError` with a reload action; it is not
  silently replaced with a successful page. Readiness comes from the committed
  surface, never the outer shell/Suspense fallback (see Window Shell).

### Persistent primary pages

- `CommittedPrimaryPage` is memoized at the stable route-ID boundary. A parent
  route change must not re-render an unrelated hidden page solely by creating
  a fresh JSX element. Visibility, actual route-context consumers, Query state
  and local state still update normally; this is not a frozen router context.

- A primary page is lazy until its first visit. Once visited, it remains
  mounted behind `PersistentSurface` so draft/UI state can survive switching
  among primary routes.
- An unvisited route is not mounted merely because it exists in navigation.
  Persistent mounting is not permission for hidden pages to poll, mutate URL
  state, retain focus, or expose interactive accessibility descendants.
- An inactive surface is `hidden`, `aria-hidden`, and `inert`, and it blurs a
  focused descendant when becoming hidden. Visibility propagates through
  `usePersistentVisibility` so nested persistent owners cannot treat an
  inactive ancestor as active.
- `usePersistentSearchParams` snapshots the route's last visible query. While
  hidden, it returns that snapshot and its setter is a no-op; a hidden page must
  never read or overwrite the active route's query string.
- `useStickyVisibleValue` may preserve the last explicit visible selection when
  a keep-alive route returns without that query field. It must not invent an
  unsupported ID or override a new valid explicit value.
- Query/data hooks that can poll, launch work, or refetch on focus must consume
  visibility deliberately. Component mount alone is not an active-page signal.

### Sidebar and route blocking

The shared blocker may carry one explicit sidebar control/destination intent
for confirmation animation. It consumes that intent on the next transition,
matches the exact destination and otherwise exposes no source. This does not
alter blocker rules or router authority. Native/history/programmatic changes
without a matching intent use neutral presentation; see
[Motion and Dialog Presence](./motion-system.md).

- `SideNavigation` renders semantic links from the registry and derives active
  state from the router. Exactly one primary link is `aria-current="page"` for
  a valid primary path.
- The AI software group exposes `/agents` and `/auth` as direct controls. The configuration group is the only collapsible primary group. Collapsing it
  may retain a visually active group trigger, but must not leave hidden child
  links keyboard-focusable or produce a second current page.
- Arrow Up/Down wrap through currently available navigation controls; Home and
  End move to the first/last available control. Hidden collapsed children are
  excluded from the focus sequence.
- `PrimaryBlockerProvider` owns one router blocker for the shell. Feature pages
  register bounded rules through `usePrimaryBlocker`; they do not mount
  independent global blockers or bypass confirmation after a blocked
  transition.

### Closed Agent return flow

- Returning from Models/Skills/MCP/Prompts to an Agent preserves only a closed
  catalog ID and section. The parser rejects unknown IDs/sections, duplicates,
  missing members, and extra Agent-page query keys.
- Management routes may keep their own query fields when
  `appendAgentReturnToPath` adds the closed tuple. The helper owns encoding and
  replacement of the two return keys.
- The Agent page accepts `target` plus `section` only for this return shape.
  No caller supplies a pathname, hash, external URL, command, secret, or opaque
  JSON blob as return state.
- Sidebar transitions preserve a valid return tuple while moving among the
  management pages and map the Agents link back to the corresponding closed
  Agent path. Invalid tuples are ignored instead of partially repaired.

## 4. Validation & Error Matrix

| Condition                                                                                | Required result                                                                                        |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| root index or unknown production route is opened                                         | Replace with `/agents`; do not leave a blank shell or add the path to persistence.                     |
| a new primary item lacks a literal page loader or route match                            | Architecture tests fail; do not ship a navigation-only or loader-only entry.                           |
| a primary page has never been visited                                                    | Keep it unmounted even after global prefetch.                                                          |
| a visited primary page becomes inactive                                                  | Keep it mounted but hidden, inert, aria-hidden, and without retained descendant focus.                 |
| a hidden page calls its persistent search setter                                         | Ignore the write; preserve the active route's query.                                                   |
| a hidden page receives a global focus/refetch signal                                     | Its visibility-aware owner must suppress active-only work.                                             |
| configuration navigation is collapsed                                                    | Remove child links from the available keyboard sequence while preserving one honest active indication. |
| Arrow/Home/End is pressed in sidebar navigation                                          | Move among currently available navigation controls with the reviewed wrap/boundary behavior.           |
| a feature has an active unsaved-change blocker                                           | Hold the transition until the owning confirmation proceeds or resets it.                               |
| Agent return tuple is incomplete, duplicated, unknown, or contains extra Agent-page keys | Return `null`/ignore it; never construct a partial or arbitrary return path.                           |
| a valid Agent return tuple crosses management pages                                      | Preserve the two closed fields and each destination's unrelated route-owned query fields.              |

## 5. Good / Base / Bad Cases

- Good: visit Models, edit an unsaved form, switch to Skills, then return. The
  Models tree remains mounted but inert while hidden, its query snapshot stays
  isolated, and its draft is still present.
- Good: an Agent detail links to MCP with `{ agentId: "workbuddy", section:
"mcp" }`; sidebar navigation among management pages retains the closed tuple,
  and the Agents link reconstructs only the reviewed Agent path.
- Base: route chunks are prefetched at startup, but no unvisited page component
  mounts and no feature query starts until the route is visited.
- Base: an unknown hash route redirects to `/agents` with exactly one current
  sidebar link.
- Bad: derive imports from arbitrary route strings, render all seven pages at
  startup, let a hidden page call the live `setSearchParams`, put a full return
  URL in query state, or leave collapsed child links focusable.

## 6. Tests Required

- `tests/v2/app/router-shell.test.tsx` covers route/active-link behavior,
  persistence, lazy/prefetch boundaries, redirects, and sidebar integration.
- `tests/v2/widgets/app-shell/SideNavigation.test.tsx` covers grouping,
  collapsed availability, active semantics, Agent return preservation, and
  Arrow/Home/End focus behavior.
- `tests/v2/shared/usePersistentSearchParams.test.tsx` covers visible snapshots,
  hidden reads, hidden setter suppression, reactivation, and sticky values.
- `tests/v2/features/agentNavigation.test.ts` covers exact accepted tuples,
  duplicate/unknown/extra-key rejection, destination query preservation, and
  deterministic return paths.
- Feature tests that use `usePrimaryBlocker` cover blocked transition,
  proceed/reset, cleanup, and multiple-feature registration through the one
  provider.
- `tests/v2/app/architecture.test.ts` asserts the closed route registry,
  literal imports, V2 boundaries, and the development-only UI Lab rule.
- Browser coverage in `tests/v2-browser/shell.spec.ts` and affected feature
  flows proves actual hash navigation and persisted page behavior. Run the
  focused tests plus `mise run check:contracts`; browser mocks do not prove
  native window behavior.

## 7. Wrong vs Correct

Wrong:

```ts
const Page = lazy(() => import(`../pages/${route}/Page`));
const returnTo = new URLSearchParams(location.search).get("returnTo");
navigate(returnTo ?? "/agents");
```

Correct:

```ts
const Page = primaryPages[item.id]; // closed literal loader registry
const descriptor = agentReturnDescriptorFromManagementSearch(location.search);
navigate(descriptor ? agentReturnPath(descriptor) : "/agents");
```

Wrong:

```ts
useEffect(() => setSearchParams({ target: selected }), [selected]);
// runs while the keep-alive page is hidden and rewrites another route
```

Correct:

```ts
const { visible, searchParams, setSearchParams } = usePersistentSearchParams();
// hidden pages see their snapshot; the setter is inert until visible
```
