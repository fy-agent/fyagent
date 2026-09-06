# Hook Guidelines

## Placement and API

Route-only hooks live beside their route. Shared visual hooks live in
`shared/ui`; shared feature queries/workflows live in `shared/features`.
Native subscriptions cross a typed port, not a page import of Tauri. Pure
`domain` modules never contain hooks. Reuse an existing owner before creating
a second one; promote a hook when a concrete second consumer needs it.

Use a `use`-prefixed function and explicit typed arguments. Return named state
and actions rather than positional tuples for complex workflows. Preserve the
owning discriminated state machine; do not collapse unknown/error/stale into
booleans that imply success.

## Effects, visibility and cleanup

Timers, observers, event listeners and animations require cleanup. Async native
subscription setup must release the resolved unlisten function even when its
effect was already disposed. Guard late resolutions and errors against the
current session/operation identity; StrictMode and repeated open/close are
normal lifecycle cases, not reasons to suppress warnings.

Persistent routes stay mounted. Query/polling hooks therefore use
`usePersistentVisibility()` or an explicit active/allowed condition rather than
relying on unmount. Hidden surfaces cannot retain interactive portals. Native
jobs remain backend-owned; animation completion cannot authorize a mutation.
See [State Management](./state-management.md) and
[Change Plan Workspaces](./change-plan-workspaces.md).

Native startup readiness intentionally does not wait for RAF or document
visibility. `shared/platform/useFrontendReady.ts` signals only after usable/error
content commits; preserve the hidden-window bootstrap exception.

## Concrete owners

`pages/agents/useAgentAuthSession.ts` owns external-auth sessions;
`pages/auth/useManagedAuthLoginSession.ts` owns managed login observation;
`shared/features/change-plans-ui/useChangeJob.ts` owns Query-based change-job
observation. `shared/ui/useDialogState.ts` owns conditional dialog session
identity, while `usePersistentSearchParams.ts` owns route snapshots. Extend
these contracts instead of introducing duplicate timers/state in components.

## Verification

Run unified type/lint/unit gates and the affected browser interaction suite.
Cover delayed subscription resolution, unmount/hidden cleanup, stale results,
reopen, duplicated action attempts and reduced motion where relevant. Tests
must wait for real state transitions with `act`/async assertions or controlled
promises, not add arbitrary sleeps or blanket console suppression.

Wrong: a page interval continuously polls a hidden native job.
Correct: reuse the owning visibility-aware query/session hook and reconcile
authoritative state when the page becomes active.
