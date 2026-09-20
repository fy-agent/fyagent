# External Agent Model Integration Contract

## 1. Scope / Trigger

Read this contract before changing TRAE Work CN model endpoint validation,
TRAE cached model-ID observation, OpenCode model snapshots/fetch/save, their
network and secret boundaries, or the related native commands.

Primary owners:

- `src-tauri/src/services/traework_models.rs`
- `src-tauri/src/commands/traework.rs`
- `src-tauri/src/commands/opencode_models.rs`
- the OpenCode model service/config writer

WorkBuddy model persistence is defined by
[WorkBuddy Configuration](./workbuddy-configuration.md). Claude/Codex/Grok
Provider quick setup is defined by
[Codex/Provider Configuration](./codex-provider-configuration.md). Renderer
composition is defined by [Models](../frontend/models.md).

## 2. Signatures

TRAE Work CN exposes validation/test/cancellation plus a read-only model-ID
observation:

```text
validate_traework_model_config(request)
test_traework_model_endpoint(requestId, request)
cancel_traework_model_endpoint(requestId)
get_traework_model_ids()
  -> { modelIds, revision, truncated }
```

There is deliberately no `fetch_traework_models` or
`save_traework_models` command.

OpenCode owns a dedicated model configuration port:

```text
get_opencode_model_snapshot()
  -> { providers, selectedModel, revision, path, backupPath, exists }

fetch_opencode_provider_models(request)
  -> { models, truncated }

save_opencode_models(request)
  -> revision/overwrite-aware save result
```

GET snapshots contain only sanitized provider/model IDs, revision, and
user-visible write-target metadata. `apiKey` is a mutation/fetch argument only;
it is never query data or a persisted public DTO field.
`selectedModel` is a safe `providerId/modelId` reference matching the sanitized
provider/model list, or null. Unknown and secret-bearing config references are
not projected merely because they occur in the native `model` field.

## 3. Contracts

### TRAE endpoint validation is a bounded network probe

The endpoint parser accepts only the reviewed HTTP(S) model-service shape and
rejects credentials in URL authority, illegal schemes, malformed hosts, and
other unsupported components before network activity. Host resolution is
checked against unsafe/local/reserved IP classes and bound to the validated
address for the request. Proxy fallback, ambient credential forwarding, and
redirect-based host escape are not allowed.

The request has bounded connect/overall deadlines, response-byte limits, no
automatic decompression expansion, and cancellation keyed by the opaque
request ID. Errors are truncated and redact API keys, authorization values,
and credential-bearing URLs. A successful HTTP exchange validates only the
requested endpoint/model response; it is not proof that TRAE accepted a local
configuration.

### TRAE model IDs are observation-only

TRAE Work CN model listing is cloud-owned. FyAgent may read the reviewed TRAE
SOLO CN `state.vscdb` cache to project secret-free custom model IDs. The
colon-key form `{userId}:AI.agent.model.model_list_map` has priority over the
older underscore form when both exist. Reads are bounded, deduplicated,
revisioned, and never return account/credential/configuration fields.

FyAgent must not insert/update/delete TRAE SQLite rows or expose a save/fetch
command. Vendor launch may refresh the cache and remove local-only rows; local
SQLite success would therefore be a false product contract.

### OpenCode model writer is dedicated and revision checked

OpenCode models do not use the generic Provider quick-setup command. Native
code owns the fixed user config path, parser, per-config critical section,
revision, rolling backup, atomic write, overwrite capability, and
authoritative reread.

Managed subscriptions use the dedicated revisioned `bind_opencode_managed_proxy`
command specified by [Managed Account Proxy](./managed-account-proxy.md). It
reuses the config lock/snapshot owner and the internal Provider transaction,
not the public generic quick-setup surface. Ordinary model writes and managed
binding serialize on the same lock. Subscription conflicts have no automatic
overwrite/rebase path. Proxy lifecycle/recovery owns its loopback projection;
native OpenCode `auth.json` and independent native login remain untouched.
The ordinary model writer refuses reserved managed Provider IDs and any active
managed projection. Explicit target restore uses the existing Proxy takeover
owner; after authoritative snapshot/overview reread the ordinary writer can
resume. It cannot silently replace the sole managed provider via its legacy
single-provider convenience rule.

Unknown providers, models, and extension fields outside the managed mutation
survive. Invalid existing configuration fails closed instead of being replaced
with a minimal template. A revision mismatch requires the same bounded,
single-use overwrite capability pattern as other native configuration writers;
the renderer never sends `force`.

Fetch/save requests reject credentials in public provider/model IDs and URLs.
Responses, logs, errors, query keys, and snapshots never include `apiKey`,
authorization headers, or private config fragments. User-visible `path` /
`backupPath` are backend-projected metadata, not writable renderer inputs.

### Native provider import preserves vendor semantics

OpenCode/OpenClaw imports store raw provider objects as their native authority.
Display-name projection must not serialize a typed subset back into storage.
Live provider writes retain unknown nested fields, explicit nulls, and neighboring
providers through the existing locked/backup writers. Semantic preservation is
not a byte-for-byte formatting claim.

OpenCode `npm` is optional; omitted packages are never guessed. Typed models
allow omitted names and retain provider/model/limit extension fields. A Models
snapshot includes `editable: boolean` per provider. An existing provider without
an explicit nonempty npm, or with unsupported options/models shape, is read-only;
the native model writer rejects it before overwrite admission or backup/write.
Only newly created providers receive the dedicated writer's default package.

Model-save `providerId` is an existing exact ID or null for explicit creation.
An existing ID must still exist; display-name edits never change the target.
Creation rejects a derived-ID collision, including builtin providers. There is no
single-provider fallback. The overwrite HMAC includes the target ID, preventing
an issued confirmation from being replayed for another provider.

OpenClaw `apiKey` is native opaque JSON, including string/object/null. Parsing,
importing and copying do not stringify or resolve vendor references. Its typed
Debug representation is redacted. Public provider summaries remain id/name only
and reject credential-string collisions inside opaque objects. This does not add
an OpenClaw model editor or credential resolver to the renderer.

## 4. Validation & Error Matrix

| Condition                                                                       | Required result                                                  |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| TRAE endpoint is malformed, non-HTTP(S), credential-bearing, or resolves unsafe | Reject before request; no fallback address/proxy                 |
| DNS/address changes outside the validated binding                               | Request fails closed; do not reconnect to an unchecked address   |
| TRAE request is cancelled, times out, or exceeds body cap                       | Controlled non-success with secret-safe bounded error            |
| TRAE cache contains both colon and underscore keys                              | Use the reviewed colon key; do not merge stale duplicate records |
| TRAE cache contains credentials/private fields                                  | Project only sanitized model IDs; fail on credential collision   |
| Caller asks to save/fetch TRAE models                                           | No such native command; renderer stays guidance/observation only |
| OpenCode snapshot/config is malformed                                           | Controlled failure; never replace with a minimal file            |
| OpenCode expected revision is stale                                             | Conflict plus exact overwrite capability; no write               |
| OpenCode backup fails                                                           | Abort before primary mutation                                    |
| OpenCode reread differs                                                         | Non-success/recovery result; do not claim applied                |
| GET/query/log/error contains API key or auth header                             | Secret-safety test fails                                         |
| Renderer supplies an OpenCode config/backup path                                | API review fails; paths are native-owned output metadata only    |

## 5. Good / Base / Bad Cases

- **Good:** TRAE endpoint test validates and pins a safe address, sends one
  bounded request, redacts failures, and can be cancelled by request ID.
- **Base:** TRAE cache is absent or has no reviewed model-list key. Return an
  empty/unknown observation without creating or mutating SQLite.
- **Good:** OpenCode snapshot R1 is edited. Save with R1 backs up, atomically
  updates only managed provider/models, rereads, and returns a new revision.
- **Bad:** Add `save_traework_models`, write local-only TRAE rows, reuse Provider
  quick setup for OpenCode, or cache an API key in React Query.

## 6. Tests Required

- TRAE service tests cover URL parsing, unsafe IP classes, DNS/address binding,
  proxy/redirect rejection, deadlines, response caps, cancellation, redaction,
  and no decompression expansion.
- TRAE SQLite fixtures cover missing DB/key, colon-vs-underscore priority,
  malformed/bounded data, deduplication, truncation, revision stability, and
  secret-free projection. A negative test proves no TRAE model write command is
  registered.
- OpenCode tests cover sanitized snapshot, credential collision rejection,
  revision conflict, one-use overwrite capability, config lock, unknown-field
  preservation, rolling backup, atomic replacement, and authoritative reread.
- Command/ACL tests freeze the exact native command set and reject generic
  filesystem/network/process permission widening.
- Renderer Models port/page tests prove TRAE is observation/vendor-guidance only,
  OpenCode uses its dedicated port, and credentials never enter query cache or
  public snapshots.

## 7. Wrong vs Correct

Wrong: make a local TRAE SQLite write look like supported configuration.

```rust
sqlite.execute("INSERT INTO model_list (...) VALUES (...)", params)?;
Ok("saved")
```

Correct: project only the reviewed secret-free observation and direct users to
the vendor-owned UI for mutation.

```rust
let ids = read_trae_cached_model_ids(&trusted_state_db)?;
Ok(sanitize_model_id_snapshot(ids))
```

Wrong: send an OpenCode config path and force flag from React.

```ts
await invoke("save_opencode_models", { path, apiKey, models, force: true });
```

Correct: submit the bounded model mutation and expected revision; native code
owns target paths and conflict authorization.

```ts
await ports.opencodeModels.saveModels({
  providers: draft.providers,
  expectedRevision: snapshot.revision,
  apiKey,
});
```
