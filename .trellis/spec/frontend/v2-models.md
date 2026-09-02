# V2 Models and Change Plan UI Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Models route, model/provider
selection, product-specific model panels, endpoint preflight, OpenCode model
edits, WorkBuddy/Codex model configuration, quick setup, or Change Plan
preview/confirmation/execution UI.

Primary owners:

- `src/v2/pages/models/**`
- `src/v2/shared/features/models.ts`
- `src/v2/shared/features/change-plans.ts`
- `src/v2/shared/features/ports.ts`
- desktop `ModelPorts` and `ChangePlanPorts` adapters

Native product owners include:

- [External Agent Model Integration](../backend/external-agent-models.md)
- [WorkBuddy Configuration](../backend/workbuddy-configuration.md)
- [Codex Provider Configuration](../backend/codex-provider-configuration.md)
- [Change Plan Executor](../backend/change-plan-executor.md)

## 2. Signatures

Route:

```text
/models
```

`ModelPorts` is the only renderer access to model/provider read, validation,
probe/fetch and save operations. `ChangePlanPorts` is the only access to plan
preview, confirmation and execution. Components do not call Tauri `invoke`
directly and do not read/write vendor files, SQLite, environment variables or
secret stores.

Product selection uses the canonical Agent/catalog identity. Feature rows may
expose only the product-specific operations admitted by the parsed capability
contract; page code does not create a second model-support matrix.

Representative native surfaces are:

```text
WorkBuddy revisioned config/model snapshot and save
Codex provider/auth/model read + revisioned transaction
TRAE endpoint validate/test/cancel + observed model IDs
OpenCode model snapshot/fetch/save
Change Plan create/read/confirm/execute with compensation ledger
```

Secret-bearing edit requests remain local DTOs and never become query data,
URL state, local storage or serializable diagnostics.

## 3. Contracts

### Selection and query ownership

- The Models page derives product order/identity from the same strict catalog
  owner used by Agents. Unknown/unsupported products show a closed unavailable
  state and issue no product mutation.
- URL state may select a canonical product/model subsection when useful, but
  parsed server snapshots stay in query cache and secret drafts stay in local
  state.
- Every query key contains the canonical product plus resource/revision scope
  required to prevent data from one product/provider/model appearing in
  another panel.
- Switching product/provider/model clears validation output, overwrite
  capability, endpoint request ID and secret draft that belong to the previous
  selection.

### Strict DTO and capability projection

- Parse all native model/provider/config/change-plan DTOs in
  `shared/features/**` before rendering. Unknown fields, duplicate IDs, invalid
  enum values, malformed revisions or impossible tagged unions fail closed.
- Unsupported/read-only operations are absent/disabled from the parsed
  capability state. The renderer must not expose a save button merely because
  it has form fields.
- `unverified` and `handoff_only` are visible evidence states. A successful
  endpoint probe or application launch cannot upgrade a catalog write mode.
- Raw paths, provider documents, CLI output, database rows, request/response
  bodies and secrets never enter component props or error copy.

### Product-specific behavior

#### TRAE Work CN

- Endpoint preflight uses one backend-generated request ID: validate, then
  test/cancel using the same ID. The page never performs fetch/DNS itself.
- API key lives only in the current draft/request and is cleared on every
  terminal/cancel/error/product-change/unmount path.
- Results display only the closed terminal state/reason/duration/status class.
  Do not expose URL, key, body, headers or raw transport diagnostics.
- Model IDs are observation-only from the native TRAE cache. There is no
  renderer save/fetch-to-TRAE action, and observed presence is not proof that
  the vendor cloud accepted a new model.

#### OpenCode

- Snapshot and save preserve the native revision/overwrite-capability
  transaction. The page patches the selected provider/model fields and does
  not rebuild the full `opencode.json` document.
- GET snapshots contain no `apiKey`. A key may exist only in the edit draft and
  current fetch/save request, then is cleared.
- Fetch-models is a bounded native action. Its success does not write the
  document until the user reviews and saves.
- Revision drift shows an explicit compare/overwrite decision; do not retry
  automatically with an old snapshot.

#### WorkBuddy

- Use the one revisioned WorkBuddy config owner and its model catalog/storage
  modes. Do not write guessed `model_providers.json` or a second renderer copy.
- Unsupported existing storage shapes, revision drift and post-write readback
  disagreement remain explicit blocking/uncertain states.

#### Codex

- Codex provider/auth/model changes use the existing ordered provider
  transaction. Model choice is not an isolated renderer write when config and
  auth documents must remain consistent.
- Local `/v1/models` probing is optional/unverified evidence; failure never
  destroys stored fallback IDs or a valid existing config.

#### Products without reviewed model writes

- QoderWork, Claude, Grok Build or any product whose parsed capability does not
  admit a model write remains read-only, unavailable or handoff-only as
  reported. Do not add a generic editor or infer a write path from vendor files.

### Change Plan preview and execution

- Multi-document/provider changes use `ChangePlanPorts`. The renderer presents
  the native plan's ordered steps, owned fields, expected revisions, evidence
  and rollback scope; it does not generate executable filesystem/SQL commands.
- Destructive/overwrite/secret-affecting execution requires explicit
  confirmation bound to the current plan/revision. A changed plan requires a
  new preview and confirmation.
- Execution status comes from the native ledger. Partial apply, compensated,
  rollback failed and recovery required are not rendered as success.
- After a terminal result, invalidate/reread every affected model/provider
  query. Optimistic form state cannot become authoritative configuration.

### UX and copy

- Preserve a stable three-state screen for loading, parsed content and error;
  empty/unsupported is distinct from loading and parse failure.
- Validation errors stay next to the owning field and focus the first invalid
  control. Native transaction errors render an alert with evidence-correct,
  localized copy and retry only when safe.
- Secret inputs do not prefill from native reads, do not expose copy buttons by
  default and clear deterministically.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| Unknown product or model DTO version/shape | Fail closed; no partial editor/mutation. |
| Capability is read-only/unverified/handoff-only | Render exact mode; do not expose reviewed write as green. |
| Product/provider selection changes | Clear product-scoped drafts, request IDs, overwrite tokens and errors. |
| TRAE request ID mismatch/expired/cancelled | Stop current probe, clear secret, show closed result; no renderer fallback fetch. |
| TRAE observed IDs are empty/unavailable | Show observation state; never offer local SQLite save. |
| OpenCode/WorkBuddy/Codex revision drift | Require fresh snapshot/explicit overwrite flow; do not silently retry. |
| Fetch/probe succeeds | Show reviewed result only; do not persist without explicit save/plan. |
| Change Plan changed after confirmation | Invalidate confirmation and require new preview. |
| Change Plan is partially applied/compensated/recovery-required | Render exact native terminal state and reread affected resources. |
| GET or query data contains plaintext secret | Security regression. |
| Native error includes raw document/path/body | Map/redact at adapter; do not render raw value. |
| Save reports success but authoritative reread differs | Show uncertain/failure; do not keep optimistic saved state. |

## 5. Good / Base / Bad Cases

- **Good:** TRAE validation returns a request ID; one probe runs through the
  port; terminal state renders sanitized evidence and the key is cleared.
- **Good:** OpenCode snapshot revision drifts; the page asks for a fresh review
  instead of overwriting unknown provider fields.
- **Good:** a multi-document Codex change is previewed as a native Change Plan,
  confirmed and rendered from its compensation ledger.
- **Base:** a product is handoff-only or read-only; the page provides guidance
  without manufacturing editable fields.
- **Bad:** fetch from React, store API key in query/local storage, write TRAE
  SQLite, rebuild `opencode.json`, treat endpoint reachability as vendor
  acceptance, or paint compensated/recovery-required execution green.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- product/catalog identity and all model/provider/change-plan DTOs parse
  strictly with closed enums/tags/revisions;
- query keys isolate product/provider/model; selection changes clear every
  product-scoped draft/request/overwrite capability;
- TRAE validate→test/cancel request-ID flow, no renderer networking, no local
  write and complete API-key cleanup;
- OpenCode unknown-field preservation, revision conflict/overwrite review,
  fetch-vs-save separation and no `apiKey` in GET/query/DOM;
- WorkBuddy delegates to the revisioned native owner; Codex delegates to the
  ordered provider transaction;
- unsupported products do not expose a generic write path;
- Change Plan preview/confirmation binding, exact terminal/compensation states,
  affected-query reread and no client-generated executable steps;
- loading/empty/error, field focus, keyboard behavior and secret input cleanup;
- browser fixtures demonstrate UI only and do not count as endpoint/vendor or
  native-write HIL evidence.

## 7. Wrong vs Correct

Wrong:

```tsx
const save = async () => {
  localStorage.setItem("model-api-key", apiKey);
  await fetch(baseUrl + "/models", { headers: { Authorization: apiKey } });
  await invoke("write_vendor_model_file", { product, form });
};
```

Correct:

```tsx
try {
  const validated = await ports.models.validate(product, draft);
  const preview = await ports.changePlans.preview(validated.change);
  await confirmAndExecute(preview);
  await invalidateAffectedModelQueries(preview);
} finally {
  clearSecretDrafts();
}
```

The exact port method names remain those exported by `ModelPorts` and
`ChangePlanPorts`; page components coordinate typed results and never create a
parallel native transaction.
