# Deep-Link Import Security Contract

## 1. Scope / Trigger

This contract applies whenever a `fyagent://v1/import` payload crosses the
custom-protocol parser, renderer confirmation UI, Tauri commands, or provider
configuration service. It is required because `DeepLinkImportRequest` is an
untrusted cross-layer DTO that can carry credentials and request a change to a
live provider configuration.

The native parser, import commands and activation inbox remain registered.
The current single renderer has no deep-link import Port, event consumer or
`DeepLinkImportDialog`; only portable preview/redaction utilities remain under
`src/domain`. A received protocol event is therefore not evidence of a current
end-user import workflow. Reintroducing one requires explicit product scope and
the confirmation tests below; do not restore the retired offline HTML tool.

The protocol may request an outcome; it must never be treated as evidence that
the user approved the outcome. In particular, a provider link with
`enabled=true` is a request to activate, not authority to switch the current
provider.

## 2. Signatures

The wire shape is camelCase in TypeScript and Rust serde output:

```ts
type DeepLinkImportRequest = {
  version: "v1";
  resource: "provider" | "prompt" | "mcp" | "skill";
  enabled?: boolean;
  // Caller approval field; never populated by parsing a URL.
  activationApproved?: boolean;
  // Resource-specific fields, including endpoint, apiKey, config, and content.
};
```

```rust
#[tauri::command]
fn parse_deeplink(url: String) -> Result<DeepLinkImportRequest, String>;

#[tauri::command]
fn merge_deeplink_config(
    request: DeepLinkImportRequest,
) -> Result<DeepLinkImportRequest, String>;

#[tauri::command]
async fn import_from_deeplink_unified(
    state: State<'_, AppState>,
    request: DeepLinkImportRequest,
) -> Result<serde_json::Value, String>;

ProviderService::add_draft(
    state: &AppState,
    app_type: AppType,
    provider: Provider,
) -> Result<bool, AppError>;

ProviderService::switch(
    state: &AppState,
    app_type: AppType,
    id: &str,
) -> Result<SwitchResult, AppError>;
```

`DeepLinkImportRequest.activation_approved` serializes as
`activationApproved`. It is optional, is valid only for the `provider`
resource, and is meaningful only together with `enabled == Some(true)`.

## 3. Contracts

- `parse_deeplink_url` accepts the bounded `fyagent://v1/import` envelope and
  **always** returns `activation_approved: None`; an `activationApproved`
  query parameter is never a protocol capability.
- Every IPC command that receives a `DeepLinkImportRequest` calls
  `validate_deeplink_request` before merging or importing. Direct renderer IPC
  must receive the same envelope, control-character, double-percent-encoding,
  resource, and activation-field validation as a protocol invocation.
- A provider import first stores the provider through `add_draft`. It calls
  `ProviderService::switch` only when both `enabled == Some(true)` and
  `activation_approved == Some(true)`. Without that conjunction, an import
  may create/update the draft record but must not select it or write the live
  provider configuration.
- Parser, merge, and import failures are renderer-safe generic strings. They
  must not include the source URL, API key, nested configuration, or raw parser
  error. The native approval boolean is a validated caller assertion, not a
  native UI, signed receipt or proof that a person saw a confirmation.
- Prompt `enabled` and imported usage-script options retain their own native
  semantics. The provider-only `activationApproved` check is not a universal
  activation guard for every resource.

### Admission requirements for a future renderer consumer

No such consumer is currently shipped. Before adding it, require a fresh
unchecked activation choice for each provider link, complete bounded prompt
review, and explicit submission through one typed Port. Link generations must
isolate pending merge/import results and consent. Errors display generic copy
without inspecting or logging old `deeplink-error` payloads. Portable preview
helpers and native tests alone do not satisfy these UI requirements.

## 4. Validation & Error Matrix

| Condition                                                                                                                             | Required result                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| URL has a wrong scheme/version/action, duplicate parameter, control character, second percent encoding, or exceeds a documented bound | Parse is rejected with a generic parse error; no credential or URL is returned/logged to the renderer.    |
| Renderer IPC sends an oversized or otherwise invalid DTO                                                                              | `merge_deeplink_config` and import commands reject it with the generic operation error.                   |
| `activationApproved` is present for `prompt`, `mcp`, or `skill`                                                                       | Reject the request.                                                                                       |
| `activationApproved=true` but `enabled` is absent or false                                                                            | Reject the request.                                                                                       |
| Direct provider import has `enabled=true` but no positive caller approval                                                             | Store only a draft; leave current provider and live configuration unchanged.                              |
| Direct provider import has both activation fields true                                                                                | Store the draft and switch to that exact provider; native validation does not prove a UI confirmation.    |
| Current renderer receives an import event                                                                                             | No import consumer exists; do not claim a displayed confirmation or completed import.                     |
| A future consumer receives stale merge/import completion or an old unsafe error payload                                               | Discard obsolete state, never inherit consent, and render generic errors; add actual UI regression tests. |

## 5. Good / Base / Bad Cases

- Good: the native parser strips URL-supplied approval; a direct import without
  separate approval stores a draft and leaves live configuration unchanged.
- Base: A provider link without `enabled=true` imports as a draft without
  native switching. Prompt, MCP, and skill imports retain their resource
  semantics and cannot carry an activation approval bit.
- Bad: Parsing `activationApproved=true` from the URL, preserving the previous
  dialog's checked state for the next link, or calling `switch` directly after
  `add_draft` merely because `enabled=true` was supplied.

## 6. Tests Required

- Rust parser/unit tests must assert that every parser constructor sets
  `activation_approved` to `None`, and that direct DTO validation rejects
  invalid activation combinations without leaking secrets.
- Provider-service tests must assert that `add_draft` preserves an existing
  current provider and its live configuration, while the explicit approved
  path is the only path that calls `switch`.
- Current evidence lives in `src-tauri/src/deeplink/{tests,provider}.rs`,
  `src-tauri/src/commands/deeplink.rs`, native Provider tests, and
  `tests/domain/serialization/deepLinkConfigPreview.test.ts`. The domain test
  proves redaction/preview only, not a mounted confirmation or native import.
- A future renderer consumer must add real component/browser tests for full
  writable prompt review, fresh approval, generic errors, stale merge/import
  results and zero writes before explicit consent. Deleted legacy component
  tests cannot count as current coverage.
- Run the declared fake/static checks through mise: `mise run test:unit`,
  `mise run typecheck`, and `mise run format:check`. No real custom-protocol
  launch or desktop application operation is a substitute for these assertions.

## 7. Wrong vs Correct

### Wrong

```rust
if request.enabled == Some(true) {
    ProviderService::switch(state, app_type, &provider_id)?;
}
```

This lets a URL select the active provider and write its live configuration.

### Correct

```rust
ProviderService::add_draft(state, app_type.clone(), provider)?;
if request.enabled == Some(true) && request.activation_approved == Some(true) {
    ProviderService::switch(state, app_type, &provider_id)?;
}
```

The second condition must be supplied separately by an explicitly confirmed
caller; a URL cannot set it. The current renderer has no such import caller.
