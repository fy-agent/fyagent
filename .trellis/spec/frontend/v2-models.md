# V2 Model Configuration Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Models route, target selection,
provider quick setup, model discovery/probing, write confirmation, Codex or
WorkBuddy Change Plans, OpenCode revisioned writes, TRAE observation, or model
credential handling.

Primary owners are:

- `src/v2/pages/models/Page.tsx` and `OpenCodeModelsPanel.tsx` for current
  product-specific page behavior;
- `src/v2/pages/models/QoderModelsPanel.tsx` and `TraeModelsPanel.tsx` for the
  unsupported/read-only targets;
- `src/v2/pages/models/quickSetup.ts`, `workBuddyModels.ts`, and the apply
  workspace modules for validation, drafts, preview, apply, and polling;
- `apply/SavePlanWorkspace.tsx` for the shared Codex/WorkBuddy save controller,
  and `apply/useChangeJob.ts` for automatic Query-owned job reads;
- `src/v2/shared/features/models.ts`, `change-plans.ts`, and `ports.ts` for the
  DTOs and five actual Port owners;
- `src/v2/shared/platform/tauri/feature-ports/models.ts`, `changePlans.ts`, and
  `qoderTrae.ts` for the desktop IPC boundaries.

Native ownership remains split by operation:

- [Codex Provider Configuration](../backend/codex-provider-configuration.md)
  owns Codex provider/auth projection and readback;
- [WorkBuddy Configuration](../backend/workbuddy-configuration.md) owns
  WorkBuddy revisioned model writes;
- [External Agent Model Integration](../backend/external-agent-models.md) owns
  TRAE observation/preflight and OpenCode model persistence;
- [Change Plan Typed Executor](../backend/change-plan-executor.md) owns Codex
  and WorkBuddy preview/apply/recovery semantics.

There is no aggregate `ModelPorts` or `ChangePlanPorts` type. New code uses the
focused Ports already present in `FeaturePorts`.

## 2. Signatures

### Closed route targets

`MODEL_TARGETS` is derived from `MODEL_DIRECTORY_IDS` and has this exact order:

```text
qoderwork | trae | workbuddy | grokbuild | codex | claude | opencode
```

The `target` search parameter accepts only those values. The persistent route
keeps the last valid visible target; absent/invalid input defaults to
`qoderwork`.

### Actual Port surface

```ts
interface ProvidersPort {
  getSummary(
    app: "claude" | "codex" | "grokbuild",
  ): Promise<ProviderSummaryQueryData>;
  applyQuickSetupWithResult(
    request: ProviderQuickSetupRequest,
    app: "claude" | "codex" | "grokbuild",
  ): Promise<ProviderMutationResult<ProviderSwitchResult>>;
  fetchModels(baseUrl: string, apiKey: string): Promise<FetchedModelRef[]>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
}

interface WorkBuddyPort {
  getStatus(): Promise<WorkBuddyStatus>;
  getModelIds(): Promise<WorkBuddyModelIdsResult>;
  fetchModels(
    request: WorkBuddyFetchModelsRequest,
  ): Promise<WorkBuddyFetchModelsResult>;
  saveModels(
    request: WorkBuddySaveModelsRequest,
  ): Promise<WorkBuddySaveModelsResult>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
}

interface OpenCodeModelsPort {
  getSnapshot(): Promise<OpenCodeModelSnapshot>;
  fetchProviderModels(
    request: OpenCodeFetchModelsRequest,
  ): Promise<FetchedModelList>;
  saveModels(
    request: OpenCodeSaveModelsRequest,
  ): Promise<OpenCodeSaveModelsResult>;
  checkReachability(baseUrl: string): Promise<ReachabilityResult>;
  checkModel(request: ModelProbeRequest): Promise<ModelProbeResult>;
}

interface TraeWorkPort {
  validateModelConfig(
    request: TraeWorkModelRequest,
  ): Promise<TraeModelValidationResult>;
  testModelEndpoint(
    requestId: string,
    request: TraeWorkModelRequest,
  ): Promise<TraeModelProbeResult>;
  cancelModelEndpoint(requestId: string): Promise<CancelTraeModelProbeResult>;
  getModelIds(): Promise<TraeWorkModelIdsResult>;
}

interface ChangePlansPort {
  createCodexProviderSwitchPlan(targetProviderId: string): Promise<ChangePlan>;
  createCodexProviderUpsertPlan(
    request: ProviderQuickSetupRequest,
  ): Promise<ChangePlan>;
  createWorkBuddySavePlan(
    request: WorkBuddySaveModelsRequest,
  ): Promise<ChangePlan>;
  applyChangePlan(input: {
    planId: string;
    planDigest: string;
  }): Promise<ApplyChangePlanOutcome>;
  cancelChangeJob(jobId: string): Promise<CancelChangeJobOutcome>;
  getChangeJob(jobId: string): Promise<ChangeJobSnapshot>;
  listRecoverableChangeJobs(): Promise<ChangeJobSnapshot[]>;
}
```

The current `/models` page uses only `traeWork.getModelIds()`. TRAE validation,
probe, and cancellation are real shared/native capabilities, but this route
does not currently expose them as model-management controls.

### Core write DTOs

Provider quick setup is:

```ts
interface ProviderQuickSetupRequest {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  codexFeatures?: { imageExtension?: boolean; websockets?: boolean };
}
```

WorkBuddy/OpenCode writes include the current authoritative revision and may
return one of:

```text
saved
overwrite_confirmation_required { token, existingIds }
concurrent_modification
```

Change Plan apply accepts only `{planId, planDigest}` from a previously parsed
plan. The UI never re-sends the preview's write set or credential material as
an apply instruction.

## 3. Contracts

### Routing, persistence, and query ownership

- The route selector order comes from the shared product directory. A page must
  not maintain a competing target list or introduce Gemini/Hermes merely
  because native provider code supports other products elsewhere.
- `usePersistentSearchParams` and `useStickyVisibleValue` own the visible
  target. Hidden persistent trees must not rewrite the Models query string.
- Provider, WorkBuddy, TRAE, and OpenCode observations have separate query keys
  and are enabled only when their panel is active. A cached result from one
  target is never projected into another target's UI.
- QoderWork renders the current explicit product statement that third-party
  model configuration is unsupported. It exposes no fetch, probe, save, path,
  or external-settings action.
- TRAE renders only `getModelIds()` observation and guidance that FyAgent does
  not write the local cache. It exposes no fetch/save action in this route.

### Claude, Grok Build, and Codex provider flows

- `ProviderPanel` is shared only by `claude`, `grokbuild`, and `codex`. It owns
  local `name`, `baseUrl`, API key, `modelId`, fetched model IDs, connectivity
  tests, write confirmation, and per-target warning state.
- Fetching models calls `providers.fetchModels(baseUrl, apiKey)` and keeps the
  API key in the current draft so the same credential can be used for probe or
  save. Fetch success is not a persisted configuration.
- The save confirmation shows the native `writeTargets` returned by
  `getSummary`; React never constructs target or backup paths.
- Claude and Grok Build call `applyQuickSetupWithResult`, then reread
  `getSummary`. They claim the new provider is current only when the reread
  `currentId` equals the closed quick-setup provider ID.
- `APPLY_FAILED_ROLLED_BACK` is the only direct-provider error currently treated
  as confirmed baseline restoration. An unclassified failure or
  `ROLLBACK_PARTIAL_STATE_UNKNOWN` blocks further writes for that target until
  the owning `/models` page is unmounted/remounted and authority can be reread.
  `blockedProviderWrites` lives on `ModelsPage`; switching targets or merely
  remounting a child Provider panel does not clear the block.
- Codex does not call the direct apply path. It creates a parsed Change Plan
  through `createCodexProviderUpsertPlan`, shows the closed preview, and applies
  only its `planId` and `planDigest` through the Change Plan workspace.
- Codex image-extension and WebSocket choices exist only in the Codex request.
  The page sanitizes returned warning codes against the closed
  `CodexProviderMutationWarning` union.

### WorkBuddy flow

- WorkBuddy reads `getStatus()` and `getModelIds()` separately. A read is
  authoritative only when both queries succeed; refreshing one cannot confirm
  the other.
- Model fetch accepts `allowNoApiKey`, keeps the submitted key in the draft,
  rejects a returned model ID containing that key, and records whether the
  native list was truncated.
- Draft IDs preserve order and uniqueness; fetched and manual IDs are split in
  the write request. A model ID containing the submitted API key is rejected
  before save.
- Normal save captures the current draft revision and native expected revision,
  shows `ModelsWriteConfirmDialog`, creates a WorkBuddy Change Plan, and applies
  through `ChangePlansPort`. A stale plan is regenerated rather than retried
  with an old digest.
- Terminal Change Plan handling rereads both WorkBuddy queries. An unconfirmed
  job keeps writes blocked; success/warning commits the captured draft revision
  only when the job is not an unconfirmed authority state.
- Deleting an existing model is a distinct confirmed action that calls
  `workbuddy.saveModels` directly with `removedModelIds`. If that direct call
  returns an overwrite token, the already confirmed delete may resubmit once
  with the token; this is not the normal add/save flow.

### Change Plan workspace lifecycle

Codex and WorkBuddy save wrappers supply typed request/create callbacks and
product copy to `SavePlanWorkspace`. Their one-shot writes stay local and
imperative: do not put API-key-bearing requests in `useMutation` variables,
Query keys/data, or a second persistent workflow store. Synchronous admission
prevents same-tick duplicate submission; closing/unmounting invalidates pending
UI replies. Terminal callbacks are delivered at most once per job in the
mounted save workspace. This does not cancel the native operation.

Both saves and the Codex switch workspace use:

```text
useChangeJob(port: ChangePlansPort, active: boolean)
  // -> { job: ChangeJobSnapshot | null, error: {code} | null, setJob }
featureKeys.changeJob(jobId) // ["v2", "change-plans", "job", jobId]
```

Only the job ID is local state; parsed/redacted snapshots live in Query.
Automatic reads are enabled only while the caller is active and its persistent
surface is visible. A running/planned job polls every second through Query's
single-flight lifecycle, with retry/focus/reconnect disabled. Terminal state or
a sanitized read error stops polling; an error retains the last snapshot and
must not manufacture success. A lower native `revision` cannot replace a newer
cached snapshot. The immediate authoritative reread after apply remains part of
the explicit operation while `busy`; it is not a second automatic timer.

The read consumes Query's abort signal so a late IPC response is not accepted
after its observer is canceled. IPC itself is not abortable by this signal.
Use `signal.aborted` and Query's exported `CancelledError`, not newer
`AbortSignal.throwIfAborted`, to avoid raising the minimum native WebView API.
Hiding a workspace cancels only inactive queries; closing one observer cannot
cancel a read still owned by another visible observer. No `setInterval` or
custom promise/cache scheduler belongs in these workspaces.

Set a single `featureKeys.changeJobs` family default with `gcTime: 0` **before**
the first `setQueryData` seed, and use the same lifetime in the observer.
Otherwise the seed creates a default-lived query and its longest configured GC
time survives a later shorter setting. After all observers are removed the job
cache is eligible for immediate collection. Do not add per-job defaults or a
custom eviction timer, and never cache raw native error diagnostics.

### OpenCode flow

- OpenCode reads a strict `OpenCodeModelSnapshot` containing providers,
  revision, path/backupPath, and existence. Current UI edits the first provider
  snapshot.
- Fetch uses `fetchProviderModels`, preserves the key for later save, and keeps
  ordered unique model IDs plus `ownedBy` metadata for local icons/grouping.
- Normal save includes `expectedRevision`, shows the native write target, and
  calls `saveModels`. `concurrent_modification` requires reread. An initial
  `overwrite_confirmation_required` opens an explicit overwrite confirmation;
  only its matching token may be resubmitted.
- Delete is separately confirmed and may use one native overwrite token after
  that confirmation. An expired/invalid token becomes a failed operation and
  requires reread; the UI does not manufacture success.
- Every terminal direct write performs authoritative snapshot reread when the
  operation semantics permit it. The saved result alone is not a replacement
  for current revision/path/provider state.

### Runtime parsing boundary

- `changePlans.ts` strictly validates request identity, plan/job exact keys,
  closed enums, plan digest, resource sets, and result state before UI use.
- `qoderTrae.ts` strictly validates TRAE requests/results, canonical UUID v4
  request IDs, closed reason/state combinations, and the model-ID snapshot.
- `models.ts` strictly parses provider summaries, fetched provider refs,
  reachability/model-probe results, and OpenCode snapshot/fetch/save results.
- `parseModelProbeResult` validates the result shape and closed status, but it
  does not currently bind `modelUsed` back to `request.modelId`. Do not claim
  cross-request identity protection at this Port boundary; adding it requires
  an adapter regression test and a deliberate native-alias policy.
- The current adapter still forwards these responses with compile-time typing
  only: `providers.applyQuickSetupWithResult`, `workbuddy.getStatus`,
  `workbuddy.getModelIds`, `workbuddy.fetchModels`, and
  `workbuddy.saveModels`. Do not document them as runtime-validated until a
  parser is actually added at the Port boundary.
- Components do not cast arbitrary IPC values or call `invoke()` directly.

### API key and sensitive-state boundary

- API keys live in controlled input state plus a ref for the mounted panel. They
  are never placed in route/search params, query keys, local/session storage,
  analytics, visible error text, or model IDs.
- Fetch/probe intentionally retain the key for the remaining draft. A terminal
  save clears it only when the submitted draft revision is still current;
  unmount also clears the ref/state by destroying the panel.
- A base URL must be HTTP(S), have a hostname, and contain no username,
  password, query, or fragment.
- Provider Quick Setup additionally rejects the API key when it appears in the
  normalized URL host/path, configuration name, selected model ID, or reserved
  quick-setup provider ID. WorkBuddy rejects key collisions in its normalized
  URL and model IDs. OpenCode rejects fetched/selected model IDs that equal or
  contain the key, but the current renderer/native save path does not apply the
  same text-collision rule to OpenCode `providerName` or `baseUrl`; do not claim
  that broader guard exists.
- Port requests currently carry the plaintext API key to trusted native code.
  This renderer boundary is not a `SecretRef` contract and must not be described
  as one.
- Fetch/probe/write failures render closed generic copy. Native diagnostics,
  response bodies, credentials, and raw provider errors do not enter the DOM.

### Connectivity tests and preview evidence

- Reachability and model probes are separate operations. Model probe is offered
  only after candidate IDs exist and uses the selected ID plus the current
  draft revision; changing the owning draft invalidates a stale result.
- A successful fetch/probe proves only the native request result. It does not
  prove the configuration was saved or that a vendor process reloaded it.
- `ModelsWriteConfirmDialog` is a target/path confirmation, while a Change Plan
  preview is a neutral, parsed plan. Neither is an apply result.
- Change Plan job UI derives status, compensation, recovery, live-config, and
  usage evidence only from the parsed job snapshot. It never infers provider
  use from a successful write.

## 4. Validation & Error Matrix

| Condition                                                                           | Required result                                                                                                                                                            |
| ----------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Unknown/absent route target                                                         | Use the closed default `qoderwork`; do not mount an arbitrary panel.                                                                                                       |
| QoderWork selected                                                                  | Show unsupported guidance only; issue no model IPC.                                                                                                                        |
| TRAE selected                                                                       | Read/display model IDs only; do not expose local save controls.                                                                                                            |
| Base URL contains credentials/query/fragment or is not HTTP(S)                      | Reject before fetch, probe, plan, or write.                                                                                                                                |
| API key is empty where `allowNoApiKey` is false/absent                              | Reject before fetch, probe, plan, or write and focus the key field.                                                                                                        |
| Provider Quick Setup key collides with name/URL/model/reserved provider ID          | Reject locally before the request.                                                                                                                                         |
| WorkBuddy key collides with normalized URL or any returned/manual/selected model ID | Reject in the renderer/native owner before display or persistence.                                                                                                         |
| OpenCode key collides with a fetched/selected model ID                              | Reject in the native fetch/save owner. Do not infer an equivalent `providerName`/`baseUrl` text-collision check.                                                           |
| Fetch/probe fails                                                                   | Show generic safe failure and keep the current draft/key for correction.                                                                                                   |
| Model probe returns a structurally valid result with a different `modelUsed`        | The current adapter accepts the shape; do not present this as an implemented request-binding guard. A hardening change must define alias policy and add a regression test. |
| Direct provider reread does not confirm `currentId`                                 | Report saved/pending confirmation; do not claim current provider.                                                                                                          |
| Direct provider error is not confirmed rollback                                     | Mark authority unknown and block further writes for that target.                                                                                                           |
| Codex/WorkBuddy plan is stale                                                       | Require regenerate; never apply the old digest.                                                                                                                            |
| Change Plan is preview-only                                                         | Do not show success or mutate until `applyChangePlan` admits a job.                                                                                                        |
| Change Plan terminal state is unconfirmed                                           | Reread and keep writes blocked; do not commit the draft.                                                                                                                   |
| Automatic job read exceeds its polling interval                                     | Share the in-flight read; do not start overlapping interval requests.                                                                                                      |
| Workspace is hidden or inactive                                                     | Pause automatic reads; ignore its canceled late result without canceling the native job or another active observer.                                                        |
| Job read returns an error                                                           | Cache only a closed error code, retain the last snapshot, stop automatic polling.                                                                                          |
| A snapshot has a lower native revision                                              | Retain the newer cached authority.                                                                                                                                         |
| Save is clicked twice before React commits state                                    | Admit one write using the synchronous operation guard.                                                                                                                     |
| Last job observer is removed                                                        | Cancel acceptance of its obsolete read and collect the zero-retention query.                                                                                               |
| WorkBuddy one of status/model-ID rereads fails                                      | Treat authoritative reread as failed.                                                                                                                                      |
| Returned WorkBuddy/manual model ID contains the submitted key                       | Reject and never persist/display it as a model.                                                                                                                            |
| OpenCode/WorkBuddy revision changed                                                 | Return/show concurrent modification and reread before retry.                                                                                                               |
| Initial OpenCode overwrite is required                                              | Show explicit confirmation and reuse only the issued token.                                                                                                                |
| Overwrite token is expired/invalid                                                  | Fail, reread, and require a new confirmation.                                                                                                                              |
| Panel unmounts                                                                      | Destroy unsaved panel-local key and draft state.                                                                                                                           |
| Raw native error/body/key reaches DOM, URL, storage, or logs                        | Security regression.                                                                                                                                                       |

## 5. Good / Base / Bad Cases

- **Good:** fetch Claude model refs, select one, confirm native write targets,
  apply quick setup, then claim current only after the provider summary reread
  returns the expected provider ID.
- **Good:** create a Codex or WorkBuddy Change Plan, render its neutral preview,
  apply only `{planId, planDigest}`, poll the parsed job, and reread authority
  before committing the draft.
- **Good:** OpenCode reports concurrent modification; keep the draft, reread the
  snapshot, and require a fresh save/overwrite decision.
- **Base:** QoderWork shows no configurable controls; TRAE displays a truncated
  or empty observed list with honest guidance.
- **Base:** a model fetch succeeds and keeps the API key so the user can probe or
  save; no persistence claim is made.
- **Bad:** introduce a fake `ModelPorts`, route every product through Change
  Plans, call TRAE write controls from this page, call WorkBuddy direct save for
  its normal add flow, retry a stale plan/token, or clear the key after fetch
  while claiming the draft is preserved.

## 6. Tests Required

Run the focused V2 checks through the repository task runner. Required
assertion owners include:

- `tests/v2/pages/models/Page.test.tsx`: exact target order/default,
  Qoder/TRAE non-write behavior, provider/WorkBuddy/OpenCode fetch-save-delete
  flows, key retention/clearing, write blocking, authoritative reread, partial
  rollback, truncation, warnings, target confirmation, and connectivity probes;
- `tests/v2/pages/models/quickSetup.test.ts`: closed target parsing, URL/key
  validation, exact minimal provider request, Codex feature payload, and manual
  model ID parsing;
- `tests/v2/pages/models/ModelConnectivityTest.test.tsx` and
  `workBuddyModels.test.ts`: draft-revision probe invalidation, search/grouping,
  ordered uniqueness, and fetched/manual split;
- `tests/v2/platform/featurePorts.test.ts`: exact Provider, WorkBuddy, OpenCode,
  and TRAE command/payload mappings plus every runtime parser currently owned by
  the adapter. Reachability payload tests pass URL only; a future model-probe
  request/response identity check needs an explicit mismatched-`modelUsed`
  regression;
- `tests/v2/features/change-plans.test.ts` and
  `tests/v2/platform/changePlansPort.test.ts`: exact plan/job parsing, request
  validation, digest/ID binding, and command names;
- `tests/v2/pages/models/apply/*.test.tsx`: neutral preview, one apply under
  repeated/StrictMode clicks, stale regeneration, job polling, recovery/
  compensation copy, and no secret/backend diagnostics;
- `tests/v2/pages/models/apply/useChangeJob.test.tsx`: slow-read single flight,
  terminal/error stop, hidden cancellation/resumption, concurrent observers,
  lower-revision rejection, stale replies after close, zero-retention collection,
  and sanitized cache; `apply/architecture.test.ts` guards shared orchestration
  and Query ownership;
- `tests/v2/app/router-shell.test.tsx`: persistent Models lifetime and hidden
  route/query isolation.

Native writer, rollback, file/readback, and real-provider behavior remain owned
by the linked backend contracts and native/HIL tests. Renderer tests do not
prove a vendor process reloaded or used a model.

## 7. Wrong vs Correct

Wrong:

```ts
const result = await ports.models.save(target, form);
setSaved(true); // No such Port; target protocols and evidence differ.
```

Correct:

```ts
if (target === "codex") {
  const plan = await ports.changePlans.createCodexProviderUpsertPlan(request);
  // Show parsed preview; apply later with plan.planId + plan.planDigest only.
} else if (target === "claude" || target === "grokbuild") {
  await ports.providers.applyQuickSetupWithResult(request, target);
  const reread = await summaryQuery.refetch();
  // Claim current only when reread.data.currentId is the expected closed ID.
}
```

Wrong:

```ts
localStorage.setItem("model-api-key", apiKey);
const plan = { ...preview, apiKey };
await ports.changePlans.applyChangePlan(plan);
```

Correct:

```ts
apiKeyRef.current = apiKey; // Mounted draft only; never URL/storage/query data.
const outcome = await ports.changePlans.applyChangePlan({
  planId: plan.planId,
  planDigest: plan.planDigest,
});
// Clear the key only at the owning terminal/current-revision boundary.
```

Wrong: clone an async `setInterval` for each save workflow or place its request
in the mutation cache merely to reuse a loading flag.

Correct: keep typed product write callbacks in `SavePlanWorkspace` and use
`useChangeJob` for redacted snapshot observation, visibility, and cancellation.
