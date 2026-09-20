# Managed Account Proxy Binding and Overview Contract

## 1. Scope / Trigger

Read this contract before changing how an explicitly selected OpenAI/xAI
managed account becomes a local Agent Provider, how that Provider is handed to
the existing Proxy runtime, or how the `fyagent_proxy` connection is projected
into the Managed Auth overview.

Primary implementation boundaries are:

- `src-tauri/src/commands/provider.rs` for
  `bind_managed_proxy_provider` and the retained xAI compatibility command;
- `src-tauri/src/services/provider/managed_proxy.rs` for account admission,
  stable Provider construction and target-specific bind results;
- `src-tauri/src/commands/xai_oauth.rs` for the bounded xAI CLI model
  suggestions;
- the `fyagent_proxy` reconciliation and wire-summary helpers in
  `src-tauri/src/services/managed_auth/service.rs`.

[Managed Auth Core](./managed-auth.md) owns identities, credential metadata,
SecretRef admission, exact-account lookup, refresh ownership and access-token
resolution. [Proxy Runtime](./proxy-runtime.md) owns the loopback listener,
target mutation locks, Agent live-file projection, readback, compensation and
the read-only route observer. [Local Proxy Pipeline](./local-proxy-pipeline.md)
owns upstream request shaping, same-account 401 replay, streaming and usage.
This document composes those owners; it does not authorize a second token
store, refresh loop, listener, target writer or retry state machine.

The Renderer counterpart is
[Managed Account Subscriptions](../frontend/managed-account-subscriptions.md).
The strict account/connection DTO parser remains in
[Renderer Managed Accounts](../frontend/managed-auth.md).

## 2. Signatures

The generalized Tauri command accepts exactly one nested request:

```text
bind_managed_proxy_provider({ request: {
  app: "claude" | "codex" | "grokbuild",
  accountId: "ma1:" + 32 lowercase hex,
  modelId: string
}}) -> BindManagedProxyResult
```

OpenCode has a dedicated revisioned surface:

```text
bind_opencode_managed_proxy({ request: {
  accountId: "ma1:" + 32 lowercase hex,
  modelId: string,
  expectedRevision: string | null
}}) -> BindManagedProxyResult { app: "opencode", activated: true, ... }
```

It holds the OpenCode configuration lock from revision admission through the
existing Provider transaction and readback. A stale revision is a
`provider_conflict`. Generic quick setup and `bind_managed_proxy_provider`
still reject OpenCode; a second unrestricted writer is not admitted.

The retained compatibility façade is xAI-only and additionally admits the
existing Claude Desktop draft workflow:

```text
bind_xai_managed_provider({ request: {
  app: "claude" | "claude-desktop" | "codex",
  accountId: "ma1:" + 32 lowercase hex,
  modelId: string
}}) -> BindManagedProxyResult
```

`modelId` is 1–128 ASCII bytes, begins with an alphanumeric byte and otherwise
contains only alphanumeric, `.`, `_`, `-` or `:`. The native command repeats
that validation; the compatibility façade does not turn a non-xAI account into
an xAI binding.

All binding commands return the exact wire shape:

```text
BindManagedProxyResult {
  providerId: string,
  providerName: string,
  app: submitted app,
  alreadyBound: boolean,
  activated: boolean
}
```

The closed error body contains only `code`:

```text
invalid_request
account_unavailable
provider_conflict
apply_failed_rolled_back
rollback_partial_state_unknown
```

`activated` is true exactly for successful Claude Code, Grok Build or OpenCode binds.
Codex and compatibility Claude Desktop are saved drafts and return false.

The xAI suggestion command is:

```text
get_xai_oauth_models({ accountId }) -> Vec<FetchedModel>
```

It validates the same explicit overview identity and current vault credential.
It is a bounded list of documented CLI route suggestions, not a subscription
catalog or entitlement response.

The Proxy-owned observation consumed by overview reconciliation is:

```text
observe_managed_account_route(authKind, resolvedAccountKey, isDefault)
  -> Option<bool>
```

Proxy Runtime owns how this value is established. This contract owns its
mapping into the complete Managed Auth wire snapshot:

```text
Some(true)  -> connected / official_subscription
Some(false) -> disconnected / none / connection_unavailable
None        -> checking / unknown / observer_unavailable
```

## 3. Contracts

### Exact account and credential admission

- `accountId` is the public identity from the current Managed Auth overview.
  Native code resolves that exact identity; it never selects the default
  account or a similarly labelled account for the caller.
- The resolved credential must be `Ready`, use
  `purpose=proxy_upstream`, `consumer=fyagent_proxy` and
  `refresh_owner=fyagent`. Native Codex/Grok/OpenCode credentials are not
  eligible even when they aggregate onto the same account card.
- Admission includes reading the matching SecretRef bundle. A removed,
  revoked, stale-generation or unavailable vault session returns
  `account_unavailable`; a new vault-created binding never falls back to an
  in-memory legacy JSON account.
- Binding never returns a token, SecretRef, credential ID or refresh lineage.
  Managed Auth Core remains the only owner that resolves or refreshes upstream
  access material at request time.

### Stable Provider identity and compatibility

- Native code constructs the Provider ID from the resolved account/model
  binding and target; caller-supplied display text never becomes an ID. OpenAI
  and xAI identities remain distinct even when their labels or model IDs match.
- OpenAI Codex Providers use the custom `fyagent_chatgpt` slot and
  `codex_oauth`; they must not overwrite or impersonate the reserved native
  `openai` Provider. xAI keeps its existing `xai` / `xai_oauth` compatibility.
- An existing Provider with the same stable ID must match the complete
  persisted binding definition. Its saved presentation name is retained so a
  public account rename or user rename does not break idempotency. A mismatched
  definition returns `provider_conflict`; it is never overwritten in place.
- `alreadyBound` reports whether that matching Provider row existed before the
  operation. It does not claim that the listener is running, the Agent live
  file adopted it, or an upstream request succeeded.

### Target-specific application

- Claude Code and Grok Build enter the existing Provider transaction and
  Proxy managed-activation path. Positive return requires the target-specific
  write/readback and runtime commit to finish; `activated=true` is not emitted
  after a merely saved row.
- OpenCode enters that same transaction through its dedicated binder. Its
  `@ai-sdk/openai` Provider uses the isolated `/opencode/v1` loopback base URL,
  one explicit model and the local proxy marker. The OpenCode writer preserves
  unrelated Providers, MCP and unknown fields and selects `providerId/modelId`.
  It does not write OpenCode `auth.json` or export any OAuth material. Only an
  owned, intact prior managed projection can be replaced. Native OpenCode auth
  remains a separate consumer with its own refresh lineage.
- Codex saves or reuses the Provider draft without changing the current
  Provider marker. The user continues through the existing Auth source
  workspace and typed Change Plan before any live switch. This command does
  not mount a second Codex apply flow.
- The old xAI command retains Claude Desktop draft compatibility. The generic
  command rejects `claude-desktop`; the Models route exposes no Desktop action
  until its own authoritative draft readback and application path exist.
- Agent-facing configuration receives the loopback endpoint, selected model
  and existing local proxy marker/placeholder. Upstream OAuth access/refresh
  tokens never enter Provider JSON returned to the Renderer, a Change Plan,
  Claude/Codex/Grok/OpenCode live files or native Agent auth files.
- Existing native auth material, MCP configuration, permissions and unrelated
  target fields survive application and restoration. Automatic failover is
  disabled for the managed target so an expired subscription cannot silently
  spend against an API-key Provider.
- The shared listener/target lock order, snapshots, backup ownership,
  compensation and state-unknown result are owned by Proxy Runtime. Binding
  cannot publish a Provider success before that owner returns an authoritative
  result.

### Model suggestions and upstream evidence

- xAI suggestion lookup uses the exact selected account and current SecretRef
  admission, then returns reviewed CLI route IDs. It never sends the session
  token to the API-key `/models` endpoint.
- OpenAI has no equivalent subscription catalog in this feature. The Renderer
  requires explicit manual model input; native code does not guess a default.
- A suggestion, saved Provider, adopted listener or successful local readback
  is not evidence of account entitlement, quota, every model's availability or
  a successful upstream generation.

### Route observation and overview projection

- Proxy Runtime observes the same effective current Provider authority used by
  forwarding, not only a database current marker. `Some(true)` requires an
  owned running loopback listener and at least one matching effective Provider
  whose target live configuration has adopted that listener.
- A stopped listener, unadopted target or nonmatching Provider yields
  `Some(false)`. Lock contention yields `None`. Target-local unreadable or
  stale selection is accumulated while the other targets are observed: a
  matching account's confirmed route can still yield `Some(true)`, otherwise
  that unreadable state yields `None`. Observation is read-only and never
  repairs selections or probes the upstream service.
- Reconciliation may retain `accountId` on a named `fyagent_proxy` slot even
  when observation is false. That represents the selected saved credential,
  not current request routing.
- OpenAI/xAI proxy connections are keyed by the admitted proxy credential, not
  just the provider's default account. Each eligible credential has a stable
  private provider slot with an empty target ID; the public connection ID stays
  opaque. Observe that exact credential, including non-default accounts.
  Reconcile before account-removal impact preview, and prune only obsolete
  `fyagent_proxy` slots with an empty target ID. Native consumers and Copilot
  slots keep their existing owners. Multiple target routes for one credential
  still count as one unique `fyagent_proxy` consumer.
- `requestMode=none` always serializes
  `requestProviderLabel=null`. A stale label must be cleared before the
  complete overview crosses IPC.
- Each account's `connectedConsumerCount` is the number of unique
  `connections[].consumer` values whose `accountId` names that account. It
  includes a disconnected/non-routing `fyagent_proxy` slot and does not count
  duplicate rows for the same consumer twice.
- The complete overview must satisfy the strict Renderer cross-reference and
  count parser. Backend reconciliation never hides a named connection merely
  to make the count smaller and never fabricates connected state from
  credential readiness alone.

## 4. Validation & Error Matrix

| Condition                                                                                                       | Required result                                                                           |
| --------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Generic request names an unsupported app, malformed account ID/model ID, or extra field                         | Reject before Provider, listener, file or credential mutation.                            |
| Compatibility request selects a non-xAI account                                                                 | `account_unavailable`; do not reinterpret it as a generic bind.                           |
| Explicit account is absent, not Ready, wrong-purpose, wrong-consumer, native-owned or unreadable from the vault | `account_unavailable`; no default/legacy fallback.                                        |
| Stable Provider ID exists with a different account/model/config definition                                      | `provider_conflict`; preserve the existing row and live state.                            |
| Claude/Grok application fails and every snapshot is restored/read back                                          | `apply_failed_rolled_back`; no activation claim.                                          |
| Provider/target/listener compensation cannot establish the baseline                                             | `rollback_partial_state_unknown`; preserve recovery evidence and block optimistic retry.  |
| Codex draft save/readback/current-marker verification fails but restoration is confirmed                        | `apply_failed_rolled_back`; current source remains unchanged.                             |
| Codex draft restoration cannot be confirmed                                                                     | `rollback_partial_state_unknown`; do not hand off as a usable draft.                      |
| xAI suggestions are empty or unavailable                                                                        | Return failure/empty evidence; do not manufacture a model or query an API-key catalog.    |
| Proxy observation is `Some(true)`                                                                               | Project connected + official subscription only for the named account slot.                |
| Proxy observation is `Some(false)`                                                                              | Keep the named account if present, project disconnected + none, clear the provider label. |
| Proxy observation is `None`                                                                                     | Project checking + unknown with observer-unavailable evidence; no connected claim.        |
| `requestMode=none` still has a provider label, or account count omits a named proxy slot                        | Backend/Renderer contract test fails; the complete snapshot is not accepted.              |

## 5. Good / Base / Bad Cases

- **Good:** the user chooses one ready xAI account and model for Claude Code;
  native admission resolves that exact proxy credential, reuses or creates the
  stable Provider, the existing transaction adopts one loopback listener, and
  the returned overview confirms the named route without exposing a token.
- **Good:** an OpenAI account creates a Codex `fyagent_chatgpt` draft. The
  current Codex Provider is unchanged, and Auth presents the existing
  preview/apply workspace for explicit confirmation.
- **Base:** the account remains saved while FyAgent's listener is stopped. The
  `fyagent_proxy` connection still names the account, is disconnected with
  `requestMode=none`, has no provider label and still contributes one unique
  consumer to the account count.
- **Base:** route observation is temporarily locked. The overview says
  checking/unknown rather than connected or disconnected, and no upstream
  availability claim is made.
- **Bad:** bind the default account when another account ID was submitted,
  store an OAuth token in Provider/live configuration, overwrite the native
  Codex `openai` slot, call a Codex draft active, or drop a non-routing proxy
  connection so the summary count appears cleaner.

## 6. Tests Required

- `services/managed_auth/subscription_tests.rs` and
  `subscription_opencode_tests.rs` cover both OpenAI/xAI
  accounts, all four CLI targets, compatibility Desktop draft behavior,
  wrong-purpose admission, provider conflicts, stable/distinct identities,
  saved-name idempotency, Codex current-marker preservation, target
  application and compensation.
- Managed activation integration tests cover port conflict, concurrent targets,
  listener ownership, preimages/backups, native auth/MCP preservation and
  partial rollback. Those assertions follow
  [Proxy Runtime](./proxy-runtime.md); they must not weaken target readback to
  accommodate an ephemeral test port.
- `subscription_transport_tests.rs` covers stopped, adopted and unknown route
  observation against the effective forwarding owner. Request replay/SSE
  assertions remain owned by
  [Local Proxy Pipeline](./local-proxy-pipeline.md).
- Managed Auth overview tests prove a stopped proxy clears its provider label,
  preserves the named account slot and recomputes unique consumer counts after
  observation. Renderer parser tests accept that exact snapshot and reject a
  leftover label or under-count.
- Multi-account overview tests bind a non-default and a default account to
  different targets, exercise the complete overview/removal preview, verify
  unique counts and clean obsolete legacy slots without removing other owners.
- `xaiSubscriptionPort.test.ts` asserts exact nested command payloads, strict
  request/result keys, public `ma1` identity, target/result agreement, closed
  errors, old/new command compatibility, Grok activation and browser
  native-only behavior.
- `XaiSubscriptionSection.test.tsx` and browser subscription cases cover
  explicit account/model selection, changed account invalidation, target-local
  confirmation/readback, Codex handoff and OpenAI manual input. Synthetic IPC
  and local loopback fixtures are not live subscription or quota evidence.
- Secret-negative assertions cover Provider rows, Change Plans, Agent files,
  overview DTOs, errors and logs. Native Agent auth bytes must remain unchanged
  through bind, switch and restore.

## 7. Wrong vs Correct

Wrong:

```text
bind(accountId?)
  -> missing account means use default
  -> save OAuth token in Agent config
  -> set Codex current Provider
  -> return activated=true
```

Correct:

```text
validate exact {app, accountId, modelId}
  -> resolve the exact ready proxy-purpose credential + vault bundle
  -> construct/verify one stable Provider with no upstream secret
  -> Claude/Grok: existing transaction + listener/config readback
  -> Codex: draft only, current marker unchanged, existing Change Plan handoff
```

Wrong:

```text
listener stopped
  -> remove fyagent_proxy from the account count
  -> requestMode=none, requestProviderLabel="openai"
  -> Renderer rejects the complete overview
```

Correct:

```text
listener stopped
  -> keep the named fyagent_proxy account binding
  -> disconnected, requestMode=none, requestProviderLabel=null
  -> count unique named consumers, including fyagent_proxy
```
