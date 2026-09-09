# Managed Grok Subscriptions

## 1. Scope / Trigger

Read before changing the Grok account/model picker or managed subscription
binding. `pages/models/XaiSubscriptionSection.tsx` is the single picker; it
composes the current Models panels and Managed Auth overview, not a second
authentication page. Agent Grok model entries link to the Claude/Codex Models
target with the existing validated Agent-return tuple. Login and account
recovery use `/auth?view=accounts`.

General write/draft/secret rules remain in [Models](./models.md); native
authority stays in [Managed Auth](../backend/managed-auth.md),
[Proxy Runtime](../backend/proxy-runtime.md) and
[Change Plan Executor](../backend/change-plan-executor.md).

## 2. Signatures

`ProvidersPort.bindXaiManaged` accepts the request and returns the result below.
`ProvidersPort.fetchXaiManagedModels(accountId)` uses the existing
`WorkBuddyFetchModelsResult` shape for discovery; sharing that shape does not
admit a WorkBuddy subscription workflow.

```ts
interface BindXaiManagedRequest {
  app: "claude" | "claude-desktop" | "codex";
  accountId: string; // Explicit overview ma1 identity, never legacy/default ID.
  modelId: string;
}
interface BindXaiManagedResult {
  providerId: string;
  providerName: string;
  app: "claude" | "claude-desktop" | "codex";
  alreadyBound: boolean;
  activated: boolean;
}
```

## 3. Contracts

- The overview is Query-owned under `featureKeys.managedAuthOverview` and
  pauses automatic reads while hidden. The user explicitly selects an xAI
  account. Only `health=ready` is selectable; health does not prove the native
  proxy-purpose credential exists, so native revalidates it during discovery
  and binding. No default-account fallback or renderer-held credential exists.
- Account selection reads `get_xai_oauth_models` with that exact overview
  identity and renders the existing selectable model chips. Native validates
  the vault credential and returns documented CLI route suggestions; it does
  not send the subscription token to the API-key `/models` catalog. The UI
  labels these as official-example options, not an account entitlement list,
  and labels this integration experimental. Changing accounts clears the old
  selection/list; no model is hardcoded in the renderer or selected implicitly.
  A failed/empty options read exposes manual input. Model IDs are 1–128 ASCII
  characters, start with an alphanumeric character and otherwise admit only
  alphanumeric, `.`, `_`, `-` and `:`. Discovery is not entitlement/use proof.
- The bind request has exactly the three keys above. Native response parsing
  checks exact keys, the submitted target identity, bounded provider ID/name,
  boolean fields and `activated === (app === "claude")`. Invalid or unknown
  responses fail closed; no raw native diagnostic enters product copy.
- Claude Code confirmation discloses the native write targets and shares the
  Provider panel's synchronous write guard. A positive result requires native
  application plus provider-summary/current-ID and managed-auth rereads.
  Any failed reread or unknown write result blocks further writes to that
  target through the existing Models parent block. Other targets remain usable.
  This target block still applies when switching targets unmounts the picker
  while a bind is pending; mounted guards may suppress only local UI updates.
- Codex binding saves a draft only. The result links to the existing Auth
  `consumer=codex&view=connections` source workspace, where the user selects
  the named saved source, previews and confirms the existing Change Plan.
  Models never mounts another source-switch workspace. This picker exposes
  only Claude Code and Codex CLI binding. The native contract retains Desktop
  draft compatibility, but Models provides no Desktop action until its own
  authoritative saved-source readback and application path are integrated.
- Bind errors are the closed `{code}` values `invalid_request`,
  `account_unavailable`, `provider_conflict`, `apply_failed_rolled_back`, and
  `rollback_partial_state_unknown`. The last value and malformed failures
  block writes; known preflight/confirmed-restoration failures retain a safe
  retry path. Once binding returned, subsequent owner-read errors are always
  unconfirmed regardless of their error shape.
- Successful binding invalidates/rereads the affected Provider summary and
  managed-auth overview. Account/login changes reuse the same overview key.
  The UI states that using the subscription requires FyAgent running in the
  background, and separates saved config from actual calls/quota use.
- WorkBuddy has no subscription picker: CLI route suggestions do not describe
  models supported by its configured API-key service. Its existing service/key
  input, model discovery and save-plan behavior stay intact.

## 4. Validation & Error Matrix

These focused cases extend the general write, secret and lifecycle matrix in
[Models](./models.md).

| Condition | Required renderer behavior |
| --- | --- |
| No explicit account/model, or selected account is no longer ready | Disable confirmation; never select the default account or another model. |
| Discovery fails or returns no usable IDs | Offer explicit manual input; do not manufacture a model or change the upstream source. |
| Bind result names another target, has excess fields, or contradicts activation semantics | Reject the result and mark target state unconfirmed. |
| Native rejects unavailable account/conflict, or confirms restoration | Show the closed failure and retain an explicit retry/recovery entry. |
| Native cannot confirm restoration, or either post-bind owner read fails | Block further target writes; no optimistic success or automatic write retry. |
| Codex draft is saved | State that it is a draft and expose the existing Auth source-plan continuation. |
| Claude Code picker is open | Offer only its own target; do not save a Desktop source then read the Claude Code summary. |
| WorkBuddy is selected | Keep its service/key workflow; do not offer subscription route suggestions. |

## 5. Good / Base / Bad Cases

Good: a user chooses an existing xAI account and a suggested model, confirms
Claude's native file disclosure, and sees applied copy only after native and
owner readback. Base: suggestions are unavailable, so the user enters a model ID
and native still revalidates the selected account. Bad: use the first/default
account, resurrect `auth_get_status`, or call a draft a working subscription.

## 6. Tests Required

`XaiSubscriptionSection.test.tsx` covers explicit account/model selection,
changed/expired accounts, native failure/readback, per-target draft/application
copy, navigation and hidden reads. Models Page coverage verifies that WorkBuddy
has no unsupported subscription picker. `xaiSubscriptionPort.test.ts` covers
exact vault identity payloads, invalid fields, target/result agreement, closed
errors and browser native-only behavior.

`tests/browser/xai-subscription.spec.ts` covers the real renderer's Claude
confirmation and Codex source preview/apply using synthetic IPC fixtures.
These are not native-file, live-subscription or Windows evidence.

## 7. Wrong vs Correct

Wrong: `bindXaiManaged({ app: "codex", accountId: defaultAccountId })` followed
by immediate "已切换" copy. Correct: pass the exact selected
`{app, accountId, modelId}`, reread the saved source, then hand off to Auth's
existing preview/apply workspace. A model fetch or saved draft is never live
usage evidence.
