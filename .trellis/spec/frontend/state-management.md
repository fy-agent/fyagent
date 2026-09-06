# State Management

## 1. Scope / Trigger

Read before adding route state, caches, drafts, persistence or polling. The hash
router is the sole navigation state owner; `FeatureProvider` owns the product
QueryClient and session feature context. There is no parallel application store.

## 2. Signatures and owners

`app/PersistentPrimaryOutlet.tsx` keeps visited primary routes behind
`shared/ui/PersistentSurface.tsx`. `usePersistentVisibility()` exposes actual
surface activity; `usePersistentSearchParams` freezes a hidden route's last URL.
`shared/features/queries.ts` owns `featureKeys`, including prefix invalidations
such as `featureKeys.dailyMemorySearches`.

| State                        | Owner                       | Route-leave behavior                                                                       |
| ---------------------------- | --------------------------- | ------------------------------------------------------------------------------------------ |
| Route, shareable target/tab  | Hash URL                    | Hidden pages freeze their own search; active route remains authoritative.                  |
| Backend resource             | Query + FeaturePort/backend | Cache/reread according to the owner; automatic queries stop while hidden.                  |
| Install/Auth/Change Plan job | Native job/session          | Native operation may continue without a visible page.                                      |
| Non-secret unsaved draft     | Route-local controller      | Visited keep-alive may retain it; target/session change follows its explicit draft policy. |
| Transient visual state       | Component                   | Never becomes business authority or delays revocation.                                     |
| Secret input                 | Narrow local lifetime       | Clear immediately under the owning security contract.                                      |

## 3. Contracts

Unvisited routes create no DOM/query/observer effects. Preloading a module is
not mounting it. A visited hidden route is not a background job daemon: its
automatic queries, scan dispatch and polling derive `enabled` from persistent
visibility. Manual refetch requires an active owning surface. Freeze hidden URL
observations so hidden pages cannot rewrite the current route's query string.

Native `FeatureProvider` intentionally pins `focusManager.setFocused(true)`:
catalog/Auth queries gating frontend-ready must settle while the native WebView
is still `document.hidden`. Do not gate startup authority on document visibility
or RAF; this would reintroduce a hidden-window startup deadlock. Route visibility
is a different lifecycle boundary and continues to gate nonvisible pages.

Use shared key factories for reads and mutation invalidation, including prefixes.
The current in-memory namespace is `renderer`. Persisted provider identifiers,
native jobs and wire versions are not renamed with the cache namespace.
Wait for authoritative rereads where the feature requires them; a successful
mutation promise alone does not prove the effective configuration changed.

Change Plan's Query-owned observer has its own zero-retention, revision ordering,
multi-observer cancellation and secret-write rules in
[Change Plan Workspaces](./change-plan-workspaces.md). Do not generalize that
resource-specific cache policy to every query or add component interval owners.

Avoid render-phase state synchronization except the reviewed keep-alive route
registration/hidden-search snapshot owners. No second `currentView` store.
The host updater remains removed: do not add an update provider, automatic
download, migration bypass or renderer-written native settings during cleanup.

## 4. Validation & Error Matrix

| Condition                              | Required result                                                                   |
| -------------------------------------- | --------------------------------------------------------------------------------- |
| A route is preloaded but never visited | No mounted page effects or native observation.                                    |
| A visited route becomes hidden         | Preserve permitted draft state; stop automatic work and revoke active UI/portals. |
| Hidden page sees another route's query | Keep its previous search; do not write the active URL.                            |
| Native startup WebView is hidden       | Required authority queries can settle and acknowledge readiness.                  |
| Save succeeds but reread fails         | Show unknown/refresh failure, not an invented successful effective state.         |
| Key prefix changes                     | Update readers, mutations and prefix consumers together with behavior tests.      |

## 5. Good / Base / Bad Cases

Good: invalidate `featureKeys.dailyMemorySearches` after a native memory write.
Base: a native job continues while the route stops polling and reconciles on
return. Bad: use `document.hidden` for startup or retain credentials in a global
Query/store for animation convenience.

## 6. Tests Required

Run unit and browser gates. `tests/renderer/app/route-render-isolation.test.tsx`,
router-shell, PersistentSurface and persistent-search tests prove hidden-route
isolation. Frontend-ready tests retain native-hidden startup coverage. Feature
tests cover dirty navigation, secret clearing, failed rereads and exact keys.

## 7. Wrong vs Correct

Wrong: `queryKey: ["v2", "memory", "daily", "search"]` in a page.
Correct: `queryKey: featureKeys.dailyMemorySearches`, shared with the resource
queries. Read [Navigation](./navigation.md) for the exact route lifetime.
