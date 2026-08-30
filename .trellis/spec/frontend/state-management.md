# State Management

This page describes the leftover V1 renderer (`src/App.tsx`, `src/hooks/`,
`src/lib/query/`). Production V2 owns selection in the hash router, keeps
session-only feature state in `FeatureProvider`, and uses a V2-owned
QueryClient; see [V2 Shell](./v2-shell.md) and the feature contracts.

The leftover renderer uses React local state and Context for UI state, plus
TanStack React Query for data read from or written to the Tauri backend. There
is no Zustand or Jotai dependency in `package.json`.

## V2 Route and Resource Lifecycle

Production V2 mounts only the active primary route by default. Route modules
are lazy loaded, so a route that has not been visited must not create its DOM,
queries, observers, timers, subscriptions, or effects. A cross-route native
job remains backend-owned and is recovered by an authoritative query/session
lookup when the route remounts; a hidden React tree is not a lifecycle owner.

V2 classifies state before choosing persistence:

| State | Owner | Route-leave behavior |
| --- | --- | --- |
| current route, target or shareable tab | hash router/query parameter | restored from URL |
| backend resource | TanStack Query + FeaturePort/backend | cached/reread authoritatively |
| install/Auth/change-plan job | backend job/session + query | continues without page mount |
| unsaved non-secret business draft | route/domain draft controller | explicit save/discard/block policy |
| transient visual state | local component | may reset on unmount |
| secret input | narrow local state | clear according to the owning security contract |

Do not synchronize a derived router value with `setState` during render. Use a
derived value directly, or an effect only when a deliberately separate draft
must reconcile with route authority. Blanket “visited page” keep-alive is not
a state-management mechanism.

Every V2 query or polling hook that can remain mounted behind a conditional
surface accepts/derives an `enabled`/`active` condition. Disabling a query must
also stop its automatic fetch/refetch/poll behavior; explicit user refetch may
remain available only when the owning surface is active.

## State Categories

- **Local UI state:** leftover components and feature hooks use `useState`,
  `useEffect`, and refs. `App.tsx` keeps the selected application and view
  locally, then persists those UI preferences in `localStorage`. V2 must not
  add a parallel `currentView` store.
- **Small cross-tree UI state:** Context providers own values used by unrelated
  descendants. `ThemeProvider` owns the selected theme and its persistence and
  is composed in `main.tsx`. The host updater is intentionally not renderer
  state: V1 has no `UpdateProvider`, updater query, or updater capability.
- **Backend/resource state:** TanStack Query owns results obtained through
  `src/lib/api/*`. The shared `queryClient` provides the renderer defaults;
  feature query and mutation hooks live under both `src/lib/query/` and
  `src/hooks/`.

## Host-Synchronized UI State

`src/lib/layout/useWindowLayoutMode.ts` keeps a renderer state value for
rendering, but accepts the validated native `layout-mode-changed` work-area mode
when available. The normal/constrained policy remains pure and testable in
`src/lib/layout/`; browser previews and fake tests fall back to renderer width.
Do not put this host-synchronized state in the Query cache or persist it as an
application preference.

## Query Key and Invalidation Pattern

When several hooks share a resource family, centralize their query keys in the
domain module and invalidate the affected keys from successful mutations.
`useOpenClaw.ts` is the clearest current example:

```tsx
export const openclawKeys = {
  env: ["openclaw", "env"] as const,
  health: ["openclaw", "health"] as const,
};

return useMutation({
  mutationFn: (env: OpenClawEnvConfig) => openclawApi.setEnv(env),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: openclawKeys.env });
    queryClient.invalidateQueries({ queryKey: openclawKeys.health });
  },
});
```

Not every existing query key is centralized: `src/lib/query/queries.ts` also
uses short array keys directly for resources such as providers and settings.
Match the local resource module instead of introducing a new application-wide
key factory.

## Persistence Boundary

`localStorage` is currently used for renderer preferences such as theme and
last view. Most feature data reaches native commands through typed Tauri API
facades and is frequently represented in the Query cache; `main.tsx` retains a
bootstrap-time direct `invoke` call. Keep the renderer-preference boundary
distinct from native configuration data when extending existing behavior.

## Host Updater Boundary

FyAgent V1 removes the host application's upstream updater end to end. Do not
add a global update context, update-check query, background download action, or
DatabaseUpgrade-to-updater bridge. `main.tsx` may render `DatabaseUpgrade` when
the native host reports `db_version_too_new`, but that recovery surface must
remain a local, no-network support or controlled-distribution prompt and must
not mutate the database.

## Evidence

- [src/v2/shared/features/provider.tsx](../../../src/v2/shared/features/provider.tsx)
  owns the production QueryClient and session install target.
- [src/main.tsx](../../../src/main.tsx) still composes leftover
  `QueryClientProvider` and `ThemeProvider` and renders the database-too-new
  recovery branch without an updater provider.
- [src/lib/query/queryClient.ts](../../../src/lib/query/queryClient.ts)
  defines the shared TanStack Query defaults.
- [src/hooks/useOpenClaw.ts](../../../src/hooks/useOpenClaw.ts) centralizes
  one resource family's keys, query hooks, mutations, and invalidation.
- [src/lib/layout/useWindowLayoutMode.ts](../../../src/lib/layout/useWindowLayoutMode.ts)
  accepts a validated native layout mode while retaining a renderer fallback.
