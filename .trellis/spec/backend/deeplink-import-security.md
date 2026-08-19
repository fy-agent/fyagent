# Deep-Link Import Security Contract

## 1. Scope / trigger

Read this contract whenever a `fyagent://v1/import` payload crosses the parser,
renderer confirmation, Tauri commands, or a resource service. The DTO is
untrusted and may carry credentials, code, remote URLs, or a request to mutate
live configuration.

A protocol value is intent, never approval. For a Codex Provider, import is a
protected create operation owned by Unified Change Plan. Deep-link parsing and
navigation cannot persist a Provider/endpoint/Plan, stage a secret, select a
Provider, or write live configuration.

## 2. Wire request and Codex safe draft

The legacy wire request remains camelCase and supports provider/prompt/MCP/skill
resources. `activationApproved` can only originate in the current renderer and
is never parsed from the URL. It does not authorize a Codex writer.

Codex Provider routing produces exactly this closed DTO:

```text
CodexDeepLinkPlanDraftV1 {
  schemaVersion: 1,
  draftId: UUID,                       // generated locally, not supplied by link
  app: "codex",
  operation: "create_only" | "create_and_select",
  name: String,
  homepage: CanonicalHttpUrl,
  primaryEndpoint: CanonicalHttpUrl,
  additionalEndpoints: CanonicalHttpUrl[],
  iconCode: BuiltInIconCode?,
  model: String?,
  credentialStatus: "secure_entry_required",
  source: "deeplink"
}
```

No unknown fields are retained. The exact input mapping is:

| `DeepLinkImportRequest` field | Codex v1 handling |
| --- | --- |
| `version` | require exact `v1` |
| `resource` | require exact `provider` |
| `app` | require exact `codex` |
| `name` | trim; 1–120 Unicode scalars; reject controls |
| `enabled` | `true -> create_and_select`; absent/false -> `create_only`; never approval |
| `activationApproved` | absent/false ignored; true rejected for Codex |
| `homepage` | require HTTP(S), no userinfo/query/fragment, <=2048 bytes; canonical URL serialization |
| `endpoint` | split comma list, trim, 1–8 HTTP(S) URLs, no userinfo/query/fragment, trim trailing slash, preserve order, reject canonical duplicates |
| `icon` | accept only a built-in icon code; otherwise reject |
| `model` | optional trim, <=200 Unicode scalars, reject controls |
| `apiKey` | reject with generic `secure_credential_entry_required`; never forward/stage/log |
| `config`, `configFormat`, `configUrl` | reject; never decode or fetch for Codex Plan routing |
| every `usage*` field | reject; scripts/tokens/URLs/intervals never enter the draft |
| `notes`, Claude model fields, prompt/MCP/skill fields | reject as wrong shape for Codex Provider draft |

Credential entry happens later through #35's explicit typed UI/port. Deep-link
v1 never auto-converts a raw API key into a secretRef. A future conversion needs
a new versioned contract and explicit user action.

## 3. Commands and routing

```text
parse_deeplink(url) -> DeepLinkImportRequest
merge_deeplink_config(request) -> DeepLinkImportRequest
import_from_deeplink_unified(request) -> DeepLinkRouteOutcome

DeepLinkRouteOutcome::CodexPlanDraftRequested(CodexDeepLinkPlanDraftV1)
```

- Every command validates the bounded envelope before merge/routing. Duplicate
  fields, controls, double encoding, invalid activation combinations, unknown
  fields for the selected resource, and over-limit values fail generically.
- Parser constructors always set `activationApproved=None`; a URL cannot create
  renderer approval.
- For Codex Provider, the backend performs the closed mapping, emits the safe
  draft request, and focuses/opens the add/edit-to-Plan UI. It never calls
  `ProviderService::add_draft`, endpoint writers, public add/update/switch, or
  the private effect-permit commit.
- If navigation/renderer delivery fails, return typed `change_plan_required`
  with zero Provider/endpoint/Plan/current/live/proxy/menu/cache/tray writes.
- Renderer sequence fencing remains mandatory: an older merge/import completion
  cannot replace, close, or lend intent to the latest dialog.
- Non-Codex Provider imports retain their reviewed legacy `add_draft` plus
  explicit activation-approval behavior until their own cutover. Prompt/MCP/
  skill semantics are unchanged. None of those paths is authority for Codex.
- Errors and logs are generic. The raw deep-link URL, rejected URL, API key,
  config, script/token, raw parser error, and forbidden fields never cross into
  UI/log/diagnostic output. Only canonical safe homepage/endpoint fields from the
  closed DTO may be displayed.

## 4. Validation / result matrix

| Condition | Required result |
| --- | --- |
| Wrong scheme/version/action, duplicate field, control, double encoding, or size overflow | generic reject; zero writes; no raw input in error/log |
| Direct IPC supplies invalid resource fields or `activationApproved=true` for Codex | generic typed reject; zero writes |
| Valid safe Codex request with `enabled=false/absent` | exact `create_only` safe DTO; focus/open Plan draft UI; zero persistence |
| Valid safe Codex request with `enabled=true` | exact `create_and_select` safe DTO; still requires later Plan preview/confirmation; zero persistence |
| Codex request contains API key/config/configUrl/any usage field | reject before navigation/persistence; secure-entry guidance only |
| Renderer/navigation unavailable | `change_plan_required`; preserve zero-write invariant |
| Older async result completes after newer link | ignore UI transition and retain latest draft/intent only |
| Error event includes URL/credential from an older host | show translated generic error; do not inspect/log payload |

## 5. Required tests

- One shared full-input fixture proves the exact safe DTO field-for-field,
  including endpoint normalization/order and `enabled` intent mapping.
- Table-driven Rust/TypeScript tests independently exercise every rejected
  field, bound, URL userinfo/query/fragment/control/duplicate case,
  `activationApproved=true`, and unknown field. Encoded and plain credential
  sentinels in query/fragment plus unique secret/script/config sentinels are
  absent from DTO, UI, event, log, diagnostics, and persistence.
- Side-effect spies prove Codex routing calls no `add_draft`, endpoint writer,
  add/update/switch, Plan insert, secret staging, process, or network adapter.
- Renderer tests prove exact draft ID/safe-field preservation, correct target
  UI/focus, subsequent explicit preview/confirmation, navigation failure
  zero-write, and stale async fencing.
- Non-Codex legacy and prompt/MCP/skill regressions remain covered separately.

## 6. Wrong vs correct

Wrong:

```text
Codex link -> add_draft -> activationApproved ? switch : done
```

Correct:

```text
Codex link -> validate/strip to closed safe DTO -> open draft-to-Plan UI
           -> explicit secure credential entry -> preview -> one confirmation
```
