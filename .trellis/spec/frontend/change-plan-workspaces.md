# Renderer Change Plan Workspace Contract

## 1. Scope / Trigger

Read this contract before changing Renderer Change Plan preview/apply presentation,
Codex or WorkBuddy save workspaces, saved Codex source switching, automatic
Change Job observation, cache/cancellation lifetime, or terminal readback.

Primary Renderer owners are:

- `src/shared/features/change-plans-ui/ApplyWorkspace.tsx`,
  `ChangePlanWorkspace.tsx`, `useChangeJob.ts`, `changePlanErrors.ts`, and
  `view-model.ts` for shared presentation, source switching, job observation,
  error projection, and evidence-correct status;
- `src/pages/models/apply/SavePlanWorkspace.tsx` plus the Codex/WorkBuddy
  wrappers for Models-specific save admission;
- `src/pages/auth/CodexRequestSource.tsx` for the sole saved Codex
  Provider-switch entry;
- `src/shared/features/change-plans.ts` and `queries.ts` for strict DTO/Port
  contracts and Query keys.

Native plan creation, typed execution, idempotency, compensation, persistence,
and crash recovery belong to
[Change Plan Typed Executor](../backend/change-plan-executor.md). Model drafts,
validation, and product-specific readbacks remain in
[Renderer Models](./models.md); account/connection ownership remains in
[Renderer Managed Accounts](./managed-auth.md).

## 2. Signatures

Apply presentation accepts only the opaque plan identity:

```ts
type ConfirmChangePlanInput = {
  readonly planId: string;
  readonly planDigest: string;
};
```

The shared job observer is:

```text
useChangeJob(port: ChangePlansPort, active: boolean)
  -> {
       job: ChangeJobSnapshot | null,
       error: { code: ChangePlanErrorCode } | null,
       setJob(snapshot: ChangeJobSnapshot | null),
       refetch()
     }

featureKeys.changeJobs
  = ["v2", "change-plans", "job"]
featureKeys.changeJob(jobId)
  = ["v2", "change-plans", "job", jobId]
```

`SavePlanWorkspace<Request>` receives a typed request/create callback and
product copy from the Codex or WorkBuddy wrapper; that request may contain an
API key and therefore remains mounted/imperative state. `ChangePlanWorkspace`
receives only parsed Provider summaries/current ID and an optional terminal
readback callback. Shared apply/source-switch surfaces accept no native path,
command, credential, arbitrary operation body, or caller-defined Query key,
and `SavePlanWorkspace` never forwards its request to apply.

## 3. Contracts

### File-impact disclosure

`ApplyWorkspace` accepts native-owned `writeTargets` and composes the shared
`FileWriteDisclosure`. Saved-source selection freezes that source's targets
with the generated plan, so a later selection/readback does not relabel the
operation. Missing targets cannot become permission to apply a source switch.
Root Quick Setup and per-source target lists have distinct ownership: sources
with generated model catalogs disclose those files too. Paths are display-only;
apply still sends only the existing opaque plan identity/digest.

### Ownership and route boundaries

- Models owns adding, editing, testing, and saving configurations. Saving still
  activates the submitted configuration; labels must not imply a draft-only
  native operation that does not exist.
- Auth software-connections detail owns switching among already-saved Codex
  Providers. Models links to that closed destination and must not mount a
  second `ChangePlanWorkspace`.
- `ApplyWorkspace` is presentation over parsed plan/job state. It derives copy
  from the shared view model, uses `useId` for mounted-instance-safe labels,
  and never becomes model/account authority.
- Codex and WorkBuddy wrappers adapt typed creation and copy to one
  `SavePlanWorkspace`; they do not clone apply, polling, or error state.

### Write admission and secret lifetime

- Preview creation and apply are separate. Apply sends exactly
  `{planId, planDigest}` from the parsed preview; the Renderer never resends the
  write set, Provider document, path, overwrite token, or credential.
- API-key-bearing save requests stay in the mounted Models draft and imperative
  call stack. Do not place them in `useMutation` variables, Query keys/data,
  URL state, storage, analytics, or a second persistent workflow store. Follow
  [Renderer and Build Input Security](./security-boundaries.md).
- `ApplyWorkspace` and each owning controller use synchronous refs in addition
  to rendered disabled state, so two same-tick activations admit one operation.
- Closing or unmounting increments the owning request generation and rejects
  late UI replies. It removes that observer; it does not cancel an already
  admitted native job.
- Saved-source switching treats an apply exception before admission can be
  classified as unknown authority. It blocks later source/account writes until
  the user reopens and rereads; it never automatically repeats the write.
- An immediate `getChangeJob` after successful admission is part of the
  explicit busy operation. It is not a second polling scheduler.

### Query-owned job observation

- Only `jobId` is component state. Strictly parsed, redacted
  `ChangeJobSnapshot` values live in TanStack Query; raw native diagnostics and
  write requests never enter Query or Mutation caches.
- Automatic reads require both the caller's `active` flag and inherited
  `PersistentSurface` visibility. Hiding a visited route pauses observation but
  does not stop the backend job.
- A `planned` or `running` job refetches every 1,000 ms through Query's
  single-flight lifecycle. Retry, focus refetch, and reconnect refetch are
  disabled. Terminal state or a sanitized read error stops the interval while
  retaining the last authoritative snapshot.
- A lower native `revision` cannot replace a newer cached snapshot, whether it
  arrives from a late read or an older seed.
- The query consumes Query's AbortSignal after IPC resolves. IPC itself is not
  abortable; canceled/hidden observers reject late acceptance with Query's
  `CancelledError`. Use `signal.aborted`, not
  `AbortSignal.throwIfAborted`, so minimum native WebViews need no newer API.
- Hiding or clearing one observer cancels only an inactive query. A second
  visible observer of the same job keeps the shared read alive.
- Set one `featureKeys.changeJobs` family default with `gcTime: 0` before the
  first `setQueryData` seed, and use the same observer lifetime. When the final
  observer leaves, the job cache is eligible for immediate collection. Do not
  add per-job defaults, custom eviction timers, `setInterval`, or a parallel
  promise cache.

### Terminal reconciliation and evidence

- Shared presentation never infers success from an admitted apply or a consumed
  plan. Product owners reconcile terminal jobs against their authoritative
  resources before enabling another write or committing a draft.
- Auth rereads both Codex Provider summary and managed-auth overview. Either
  failure retains the job/blocking state and exposes a read-only retry; retrying
  reconciliation must not rewrite the plan.
- Models callbacks perform the readbacks required by the owning Codex or
  WorkBuddy contract. An unconfirmed/partial authority state remains blocked.
- The admitted job remains visible even when readback changes `currentId`.
  Mounted workspaces suppress duplicate delivery of the same observed terminal
  identity; a later higher native revision may require reconciliation again.
- Recoverable-job discovery may show bounded guidance; it never automatically
  resumes or repeats a configuration write.

## 4. Validation & Error Matrix

| Condition                                                                                                                                                    | Required result                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| Apply receives anything beyond parsed `planId + planDigest`                                                                                                  | Reject the design; native paths, secrets, commands, and write bodies stay out of Renderer apply. |
| Confirm is activated twice before React commits disabled state                                                                                               | Admit one operation through the synchronous plan/controller lock.                                |
| Workspace is hidden or inactive                                                                                                                              | Pause automatic reads; keep the native job and last parsed snapshot.                             |
| A slow automatic read exceeds 1,000 ms                                                                                                                       | Share the in-flight Query request; do not overlap interval reads.                                |
| One of several observers closes                                                                                                                              | Preserve a read still owned by another visible observer.                                         |
| Canceled observer's IPC result arrives late                                                                                                                  | Reject acceptance with `CancelledError`; do not overwrite current authority.                     |
| Snapshot revision is lower than cached authority                                                                                                             | Retain the newer snapshot.                                                                       |
| Job read fails                                                                                                                                               | Cache only a closed error code, retain the last snapshot, and stop automatic polling.            |
| Final observer leaves                                                                                                                                        | Cancel obsolete acceptance and make the zero-retention job query collectible.                    |
| Saved-source apply admission is unknown                                                                                                                      | Block subsequent source/account writes; require reopen/reread, never automatic apply retry.      |
| Terminal owner readback fails                                                                                                                                | Keep the operation visible and blocked; offer read-only reconciliation retry.                    |
| Raw native error, path, write body, or token reaches Query/Mutation cache, route, DOM, or log; or a credential escapes its owning secret input/mounted draft | Security regression.                                                                             |

## 5. Good / Base / Bad Cases

- **Good:** Models creates a typed Codex/WorkBuddy plan from the mounted draft,
  renders the neutral preview, applies only the opaque identity, observes the
  parsed job through `useChangeJob`, and commits state only after owner readback.
- **Good:** Auth switches a saved Codex source, keeps the admitted job visible
  after `currentId` changes, then rereads both Provider and account authority.
- **Base:** a persistent route is hidden while a native job continues. Query
  observation pauses, then resumes from the same opaque job ID when visible.
- **Base:** recoverable jobs exist. The UI warns and rereads; it does not infer
  that replay is safe.
- **Bad:** put a secret-bearing request in `useMutation`, clone an async
  `setInterval`, let Models mount a second source switcher, retry an unknown
  admission automatically, or paint success from apply admission alone.

## 6. Tests Required

```bash
mise run lint
mise run typecheck
mise run test:unit
mise run test:browser
mise run build:renderer
```

Required assertion owners:

- `tests/renderer/pages/models/apply/useChangeJob.test.tsx`: slow-read single flight,
  terminal/error stop, hidden cancellation/resume, concurrent observers,
  revision ordering, stale reply rejection, sanitized cache, and collection;
- `tests/renderer/pages/models/apply/architecture.test.ts`: one save controller,
  Query-owned automatic reads, Auth-only source switching, and no local timer
  or Mutation-cache orchestration;
- `tests/renderer/pages/models/apply/ApplyWorkspace.test.tsx`,
  `ChangePlanWorkspace.test.tsx`, `CodexSavePlanWorkspace.test.tsx`, and
  `WorkBuddySavePlanWorkspace.test.tsx`: preview/apply identity, duplicate
  admission, unique mounted IDs, terminal delivery, close/unmount, and product
  adapters;
- `tests/renderer/pages/auth/CodexRequestSource.test.tsx`: one apply, both readbacks,
  failure/retry without rewrite, unknown admission, hidden reads, and closed
  return context;
- `tests/renderer/pages/models/Page.test.tsx` and
  `tests/renderer/pages/auth/Page.test.tsx`: owner placement, reverse invalidation,
  write blocking, and authoritative feature readback.

Renderer/browser tests do not prove native file writes, compensation, consumer
pickup, or crash recovery; those remain in the backend executor/writer tests.

## 7. Wrong vs Correct

Wrong:

```ts
useMutation({ mutationFn: () => applyChangePlan({ ...plan, apiKey }) });
setInterval(() => getChangeJob(jobId), 1_000);
```

Correct:

```ts
apiKeyRef.current = apiKey; // Mounted feature draft only.
await ports.changePlans.applyChangePlan({
  planId: plan.planId,
  planDigest: plan.planDigest,
});

// Shared Query lifecycle owns redacted job observation.
const { job, error, setJob, refetch } = useChangeJob(ports.changePlans, active);
```
