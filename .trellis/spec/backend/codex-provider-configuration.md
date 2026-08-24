# Codex Provider Configuration Contract

## 1. Scope / Trigger

Read this contract before changing Codex Provider TOML analysis or mutation,
native capability controls, vendor-specific model projection, session-resume
command construction, provider warnings, the `liveConfigChanged` result, or
the Codex Provider Change Plan ledger/readback path.
It owns the Codex provider configuration domain only. Trusted Codex Desktop
discovery, installation, process restart, and launch are owned by
[Codex Desktop Installer](./codex-desktop-installer.md); application version and
release metadata are owned by their dedicated contracts.

## 2. Signatures

```text
add_provider_with_result(provider, app, addToLive?)
update_provider_with_result(provider, app, originalId?)
delete_provider_with_result(id, app)
switch_provider_with_result(id, app)
import_default_config_with_result(app)
  -> { value, liveConfigChanged, app, warningCodes? }

analyze_codex_provider_features(app: "codex", provider, isNew?)
  -> CodexProviderFeatureState

patch_codex_provider_features(app: "codex", provider, intent, isNew?)
  -> {
       tomlText,
       state,
       imageExtensionConfigured?,
       codexNativeCapabilitiesGeneratedProvider?
     }

get_provider_summary({ app: "codex" })
  -> {
       providers: Record<string, { id: string; name: string }>;
       currentId: string;
       writeTargets: Array<{ path: string; backupPath: string; exists: boolean }>;
     }
```

Feature commands reject every `app` other than Codex. No provider command may
accept a renderer-controlled filesystem path. Generic mutation results never
return a filesystem path, process identifier, launch command,
credential-bearing diagnostic, or generic application-version field. The V2
sanitized summary is the narrow exception: it may return user-visible
`writeTargets` path/backup metadata owned by native path resolution. Paths under
the frozen user home use `~`; the DTO contains no file bytes, digest, Provider
settings, credentials, or arbitrary renderer-supplied path.

Successful Codex add/update mutations may return these stable warning codes:

```text
CODEX_WEBSOCKET_NON_GPT_MODEL
CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED
```

## 3. Contracts

### Lossless TOML and native capabilities

- Every Codex Provider exposes image-extension and WebSocket controls in the
  existing, initially collapsed advanced region. Provider ID, `base_url`,
  credentials, official/managed classification, OAuth type, proxy takeover,
  `wire_api`, and `meta.apiFormat` do not make a valid TOML draft ineligible.
- A fixed official Provider is identified only by `category == "official"` or
  ID `codex-official`. Names, URLs, and `requires_openai_auth` are not
  classifiers.
- Analysis and patching use `toml_edit` and preserve comments, blank lines,
  table and field order, unrelated fields, and unrelated headers. An invalid
  complete TOML document keeps both controls visible but disabled and blocks
  capability writes; it is never reconstructed from parsed form state.
- An invalid `http_headers` or `supports_websockets` field is a non-blocking,
  non-sensitive diagnostic. Ordinary saves preserve the invalid field. Only an
  explicit operation on the corresponding control may repair it.
- The image capability owns only the case-insensitive
  `x-openai-actor-authorization` header whose value is exactly
  `local-image-extension`. Enabling removes every case variant and writes one
  canonical key. Disabling removes every variant and then removes an empty
  header table. Other valid header entries survive.
- If `http_headers` is not a string map, explicit image enable replaces that
  field with the managed map and explicit disable deletes it. No unrelated save
  performs this repair.
- WebSocket configuration is format-agnostic. Enabling always writes boolean
  `supports_websockets = true`; disabling removes the field rather than writing
  `false`. Responses, Chat, Anthropic, managed OAuth, official, and proxy
  Providers remain saveable with the field present.
- Codex image-extension mode on a third-party Provider writes
  `requires_openai_auth = false`. Current Codex then ignores
  `auth.json`'s `OPENAI_API_KEY`, so the same key must also be written to the
  active `[model_providers.<id>]` table as `experimental_bearer_token`.
  Disabling image-extension restores `requires_openai_auth = true` and keeps
  the key only in stored/live `auth.json`; do not add a bearer token in that
  case. Live writes with an explicit `requires_openai_auth = false` project
  `auth.OPENAI_API_KEY` onto the token even when the stored TOML omitted it.
  A missing or `true` `requires_openai_auth` field is not this trigger.
  Official ChatGPT-login preservation still uses its own config-only write
  path and is not this image-mode contract.

### Migration metadata and official-provider ownership

- `ProviderMeta.imageExtensionConfigured` is migration-only private metadata.
  For a non-official Provider, missing metadata plus no managed/conflicting
  header is a legacy pending-on draft; no bulk migration writes live TOML.
  The first successful new-provider save or explicit historical choice marks
  the row configured. Displayed state still derives from TOML.
- A fixed official Provider defaults both native capabilities off. Merely
  opening or saving it creates no Provider table.
- The first actual enable creates `model_provider = "custom"` and a minimal
  table with `name = "OpenAI"`, `requires_openai_auth = true`, and
  `wire_api = "responses"` when no suitable table exists.
- `ProviderMeta.codexNativeCapabilitiesGeneratedProvider` claims ownership only
  when the capability operation created that table. A pre-existing inactive
  `custom` table may be reused but is never claimed.
- When both controls are off, an owned table is removed only if it still has
  the exact managed shape and no user fields. Otherwise only capability-owned
  fields are removed. An explicit Provider table takes precedence over unified
  Codex session-history injection.

### Vendor projection and safe session resume

- A native Responses Provider receives a vendor model catalog only when the
  active `base_url` parses as HTTPS to a reviewed hostname. The DeepSeek rule
  permits exactly `deepseek.com` and its dot-delimited subdomains.
- Scheme, hostname, and authority are parsed structurally. Substrings, paths,
  or user information such as `deepseek.com.evil.example`,
  `notdeepseek.com`, or `deepseek.com@evil.example` retain the neutral native
  template and receive no vendor harness instructions.
- Session resume crosses a shell-command boundary. Every persisted session ID
  passes the shared fail-closed validator before command construction. It must
  be nonempty ASCII; its first character is alphanumeric or `_`, and every
  remaining character is alphanumeric, `_`, `-`, or `.`.
- An unsafe ID remains visible in session history but has no `resumeCommand`.
  Do not quote or escape it into a shell string. A wider future grammar requires
  typed argv plus platform-specific launch/copy handling.

### Warnings, proxy projection, and live mutation evidence

- Warning codes are computed from the final saved Provider only when
  WebSocket is `true`. Inspect nonempty top-level `model`, `review_model`, and
  `modelCatalog.models[].model`; use the segment after the final `/` and accept
  an ASCII case-insensitive `gpt-` prefix. Any recognizable non-GPT model emits
  the model warning; no recognizable models do not.
- Active Codex proxy takeover adds the proxy warning. Warnings are omitted for
  switches, failed saves, and empty-risk results. They communicate configuration
  risk, not a transport failure or a claim that the local HTTP/SSE proxy
  supports WebSocket Upgrade.
- Normal and official proxy projections preserve explicit WebSocket state and
  the managed image header while continuing to rewrite routing `base_url` and
  `wire_api` under the proxy contract.
- `liveConfigChanged` is `true` only when a successful operation changes the
  final bytes of the current interactive user's `~/.codex/config.toml`.
  It contains no bytes, digest, path, or credential. Non-Codex mutations return
  `false`. The renderer may use the flag to offer the trusted restart flow from
  [Codex Desktop Installer](./codex-desktop-installer.md), but saving and
  restarting remain separate outcomes.

### Change Plan admission, apply, and recovery

- Schema 20 owns local-only `change_plans`, `change_jobs`, and append-only
  `change_job_events`. Fresh creation and v19 migration call the same
  idempotent table helper; WebDAV sync skips and locally preserves all three.
- `create_codex_provider_switch_plan` runs under the existing Provider mutation
  guard, reads DB/device/live baselines, and writes only the credential-free
  ledger. It performs no Provider mutation or network request. The plan expires
  after 15 minutes and stores separate DB/device current IDs.
- Admission accepts only an existing saved Codex Provider whose already-saved
  material proves that no new credential is needed. Unknown or managed auth is
  `secret_dependency_unavailable`; API keys, auth objects, raw config, paths,
  SecretRef/Keychain values, and credential-derived values never enter DTOs,
  ledger rows, errors, or logs.
- `apply_change_plan` accepts only `planId + planDigest`, reacquires the same
  Provider guard, rechecks contract/digest/TTL/consumption/baselines/secret
  capability, atomically consumes the plan, and invokes the lock-held Provider
  writer at most once. Invalid, expired, replayed, stale, or secret-blocked
  requests invoke it zero times.
- When the target is the fixed V2 Codex Quick Setup Provider, Change Plan must
  derive `target_projection_digest` from the **same pure targeted-patch
  projection** consumed by the real Quick Setup writer. The current live
  document is part of that projection so unrelated user comments, fields,
  providers, MCP and feature tables that the writer preserves are also
  expected by readback; they must never be misclassified as post-write drift.
- Readback covers DB current, device current, target definition, and the
  credential-neutral live projection. Mixed/unavailable authority becomes
  `recovery_required`. `get_change_job` and
  `list_recoverable_change_jobs` may converge that ledger state by readback,
  including a prior failed/recovery-required snapshot, but never replay the
  writer. A target reached after uncertain execution is warning, not success;
  a confirmed original baseline is failed/restored.

## 4. Validation & Error Matrix

| Condition                                                                                            | Required result                                                                                                         |
| ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| A non-Codex app calls a native-feature command                                                       | Reject before TOML analysis or mutation.                                                                                |
| The complete Codex TOML document is invalid                                                          | Keep controls visible but disabled; never reconstruct or overwrite the document.                                        |
| A managed header or WebSocket field has an invalid shape                                             | Preserve it on unrelated saves; show a non-sensitive diagnostic; repair only on an explicit matching control operation. |
| Chat, Anthropic, official, managed, or proxied Provider saves with WebSocket enabled                 | Save successfully and preserve the explicit choice; add applicable warning codes without rewriting it.                  |
| An official Provider has empty TOML and both capabilities remain off                                 | Preserve empty TOML and create no table or ownership metadata.                                                          |
| A persisted session ID fails the conservative ASCII grammar                                          | Omit `resumeCommand`; never interpolate the raw ID into a shell command.                                                |
| A DeepSeek-looking URL has HTTP, user information, a suffix-confusion hostname, or only a path match | Use the neutral template; grant no vendor behavior.                                                                     |
| A mutation succeeds but final live Codex bytes do not change                                         | Return `liveConfigChanged: false`; do not offer an automatic restart.                                                   |
| A mutation fails                                                                                     | Preserve prior live bytes and omit risk/restart success signals.                                                        |
| Change Plan admission is invalid, expired, consumed, stale, or secret-blocked                       | Return a closed error code and invoke the Provider writer zero times.                                                   |
| Change Plan readback is mixed/unavailable                                                            | Persist `recovery_required`; later recovery performs readback only and never replays the writer.                        |
| Change Plan targets the fixed Quick Setup row while live TOML contains unrelated user content        | Preview and writer use the same targeted projection; preserved content does not create a false readback mismatch.       |
| Codex image-extension is enabled (`requires_openai_auth = false`) and the Provider has an API key    | Stored and live `[model_providers.<id>]` contain `experimental_bearer_token` equal to `auth.OPENAI_API_KEY`.            |
| Codex image-extension is disabled (`requires_openai_auth = true`)                                    | Write `OPENAI_API_KEY` to `auth.json`; do not add `experimental_bearer_token` for this reason.                          |
| `requires_openai_auth` is missing or `true` during a default (preservation-off) live write           | Leave stored TOML bearer fields unchanged; authenticate via `auth.json`.                                                |

## 5. Good / Base / Bad Cases

- Good: explicit image enable normalizes only the managed header while
  preserving comments, custom headers, table order, and unrelated Provider
  fields.
- Base: a valid Provider contains no recognizable models. WebSocket remains
  enabled, the save succeeds, and no non-GPT warning is invented.
- Good: `https://api.deepseek.com/v1` matches the reviewed hostname rule;
  `https://deepseek.com.evil.example/v1` does not.
- Bad: derive official-provider identity from display name, rewrite invalid TOML
  from form state, use proxy preservation as proof of WebSocket transport, or
  quote an unsafe persisted session ID into a command string.
- Good: V2 Codex quick setup with `codexFeatures.imageExtension = true` writes
  `requires_openai_auth = false`, the managed image header, and
  `experimental_bearer_token` equal to the request `apiKey`, while still
  storing `auth.OPENAI_API_KEY`.
- Good: the same request with `imageExtension = false` writes
  `requires_openai_auth = true` and `auth.OPENAI_API_KEY` only.
- Bad: enable image-extension, set `requires_openai_auth = false`, and leave
  the API key only in `auth.json`. Current Codex will not send that key.

## 6. Tests Required

- Rust/TOML fixtures cover lossless unrelated edits, complete-document failure,
  invalid field shapes, case-variant header normalization, empty-table cleanup,
  WebSocket enable/remove, and official minimal-table ownership/cleanup.
- Migration fixtures cover pending legacy rows, explicit choices, newly created
  Providers, reused unowned tables, and exact owned-shape retirement.
- Hostname fixtures cover the approved HTTPS host and subdomains plus scheme,
  user-info, substring, suffix, and path-confusion rejections.
- Session fixtures cover ordinary UUID/provider-prefixed IDs and every rejected
  empty, leading-hyphen, non-ASCII, whitespace, quote, separator, and control
  character class.
- Result tests cover byte-exact `liveConfigChanged`, non-Codex false results,
  warning ordering/deduplication, GPT/non-GPT catalogs, proxy warnings, switches,
  and failed saves. Renderer tests prove only successful changed Codex saves can
  offer the separate trusted restart flow.
- Auth-projection tests cover image-on quick setup writing both
  `experimental_bearer_token` and `auth.OPENAI_API_KEY`, image-off keeping the
  key in `auth.json` only, live switch injecting the token when
  `requires_openai_auth = false`, and no injection when the field is `true` or
  missing.
- Change Plan tests cover 0/v19 to schema 20, sync skip/local preserve,
  zero-side-effect planning, 15-minute expiry, concurrent single admission,
  writer exactly once/zero on rejection, normal/backup-only/live-takeover
  projection parity, fixed-Quick-Setup targeted-patch projection parity,
  credential-negative persistence/serialization, and recovery-required
  readback convergence without replay.

## 7. Wrong vs Correct

Wrong:

```text
provider URL contains "deepseek.com" -> enable vendor behavior
session resume = "codex resume '" + persistedId + "'"
save succeeded -> liveConfigChanged = true
```

Correct:

```text
parsed HTTPS hostname matches reviewed host rule -> vendor behavior
persisted ID passes conservative ASCII grammar -> construct established command
successful final live bytes differ -> liveConfigChanged = true
imageExtension true -> requires_openai_auth = false
  + experimental_bearer_token = apiKey
  + auth.OPENAI_API_KEY = apiKey
imageExtension false -> requires_openai_auth = true
  + auth.OPENAI_API_KEY = apiKey
  + no experimental_bearer_token
```

## Scenario: V2 Codex Quick Setup targeted live write

### 1. Scope / Trigger

- Trigger: the fixed V2 Quick Setup Provider ID is written or switched to live.
- The stored Provider remains a minimum snapshot. It is **not** the authority
  for unrelated user-owned `config.toml` or `auth.json` fields.
- Generic imported Providers keep their existing snapshot semantics; do not
  broaden this patch mode to every Codex Provider.

### 2. Signatures

```text
patch_codex_quick_setup_live_config(currentConfig, desiredQuickSetupConfig)
  -> patchedConfig

build_codex_quick_setup_live_projection(currentLive, fixedQuickSetupProvider)
  -> { auth, config }

ProviderService::quick_setup_write_targets(Codex)
  -> [{ path, backupPath, exists }, ...]

write_live_with_common_config(Codex, fixedQuickSetupProvider)
  -> consume the same targeted projection and write its owned physical targets
```

### 3. Contracts

- Parse the current live TOML with `toml_edit::DocumentMut`. Invalid current
  TOML fails closed; never rebuild a minimal document from form state.
- The fixed Quick Setup patch may own top-level `model_provider` and `model`,
  plus its active provider's `name`, `base_url`, `wire_api`,
  `requires_openai_auth`, managed image header, managed WebSocket field, and
  managed bearer-token field. A historical fixed row that does not yet carry a
  modern owned field preserves the current live value rather than guessing a
  default.
- Existing `disable_response_storage`, `review_model`, other providers,
  comments, ordering, unrelated provider fields/headers, MCP, features,
  projects, hooks, desktop, memory/history, sandbox/approval, and user tables
  survive.
- When the current compatibility setting allows third-party auth writes,
  `auth.json` is read-modify-written: only `OPENAI_API_KEY` changes; login,
  account, token, and future unknown fields survive.
- When official-auth preservation is enabled, `auth.json` is not written and
  is absent from `writeTargets`; the submitted key is projected through the
  established provider-scoped bearer-token path in `config.toml`. The writer
  must not parse an untouched `auth.json` merely to perform this config-only
  write; preservation means its bytes are not a write prerequisite.
- The pure final-state projection is shared with Change Plan. Do not duplicate
  Quick Setup patch logic in the planner/readback owner: otherwise unrelated
  preserved TOML can make a successful targeted write look like drift.
- Before each existing physical file is mutated, replace exactly one adjacent
  `.fyagent.backup` with that file's exact preimage. No source file means no
  fake empty backup. Backup creation/permission failure aborts before the
  primary write. Backups inherit the source permission boundary.
- `writeTargets` must list exactly the physical files the current mode can
  mutate. Renderer code displays it but never supplies a path back to Rust.

### 4. Validation & Error Matrix

| Condition                                                                   | Required result                                                        |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Current `config.toml` is invalid or `model_providers` is not editable       | Reject; no backup/primary rewrite from the minimum snapshot            |
| Current `auth.json` must be written but its root is not an object           | Reject before primary mutation                                         |
| Required backup cannot be created or source permissions cannot be preserved | Reject; primary file remains byte-for-byte unchanged                   |
| Config exists but auth preservation mode is enabled                         | Back up/write config only; leave auth bytes untouched                  |
| Auth preservation is enabled and untouched `auth.json` is not parseable     | Config-only Quick Setup still does not parse/write auth; preserve bytes exactly |
| Existing unrelated provider/header/MCP/feature fields are present           | Preserve them while changing only owned Quick Setup fields             |
| Fixed historical row omits a modern optional owned field                    | Preserve the corresponding current live field; do not invent a default |

### 5. Good / Base / Bad Cases

- Good: a large hand-written config keeps comments, other providers, custom
  headers, MCP, features, and `disable_response_storage = false`; only the
  selected Quick Setup route changes and the backup equals the exact old file.
- Base: first creation has no config preimage, so no backup file is fabricated;
  the new config is still created atomically.
- Good: official-auth preservation keeps `auth.json` byte-identical and places
  the third-party key in the active provider bearer field only.
- Bad: serialize the minimum Quick Setup Provider as the complete
  `config.toml`, or overwrite all of `auth.json` with `{ OPENAI_API_KEY }`.

### 6. Tests Required

- Large TOML fixture: assert comments, unrelated top-level keys, other provider,
  custom provider field/header, MCP and features survive while owned route
  fields change.
- Assert config/auth rolling backups equal their exact immediate preimages.
- Assert backup failure leaves the primary unchanged.
- Assert Claude/Codex/Grok fixed Quick Setup rows all use targeted projection,
  so switching back to a saved reserved row cannot reintroduce full-file
  clobbering.
- Auth-preservation test: summary lists config only, auth bytes remain exact,
  and live config contains the required provider-scoped bearer token.
- Change Plan parity fixture: seed unrelated comment/review-model/provider/MCP/
  feature content, apply the fixed Quick Setup row through Change Plan, assert
  terminal success and byte/semantic preservation of every unowned field.

### 7. Wrong vs Correct

#### Wrong

```text
stored quick-setup Provider.config -> replace ~/.codex/config.toml
stored quick-setup Provider.auth   -> replace ~/.codex/auth.json
```

#### Correct

```text
current live preimage
  -> validate
  -> patch only Quick Setup-owned fields
  -> exact single rolling backup of each existing target
  -> atomic primary write
```

## Scenario: Codex image-mode API key projection

### 1. Scope / Trigger

- Trigger: current Codex does not attach `auth.json`'s `OPENAI_API_KEY` when
  the active provider sets `requires_openai_auth = false`. FyAgent image
  extension writes that field to `false`, so the API key must also live on
  the provider table as `experimental_bearer_token`.

### 2. Signatures

```text
ProviderQuickSetupRequest.codexFeatures.imageExtension: Option<bool>
into_provider(AppType::Codex) -> Provider.settings_config { auth, config }
write_codex_live_for_provider(category, auth, config_text)
project_codex_live_config_when_openai_auth_disabled(auth, config_text) -> config_text
```

### 3. Contracts

- Request: V2 `apiKey` plus optional `codexFeatures.imageExtension`.
- Stored Codex shape always keeps `auth.OPENAI_API_KEY`.
- Image on: `[model_providers.custom].requires_openai_auth = false` and
  `experimental_bearer_token` equals the same `apiKey`.
- Image off: `requires_openai_auth = true`; no image-mode bearer token.
- Live default write (preservation off): if the active table's
  `requires_openai_auth` is explicitly `false`, project `auth.OPENAI_API_KEY`
  onto `experimental_bearer_token` before writing `config.toml`, and still
  write `auth.json`. Missing or `true` leaves TOML unchanged.
- Environment: live files remain `~/.codex/auth.json` and
  `~/.codex/config.toml`. No new env key.

### 4. Validation & Error Matrix

- Empty `apiKey` -> quick setup rejects before TOML derivation.
- Invalid TOML at live write -> existing parse error; do not synthesize a
  bearer token onto a document that cannot be parsed.
- No API key and `requires_openai_auth = false` -> do not invent a token;
  `prepare_codex_provider_live_config` leaves the text unchanged.

### 5. Good/Base/Bad Cases

- Good: image-on quick setup stores and lives the same key in auth and the
  provider bearer token.
- Base: image-off quick setup writes only `auth.json`.
- Bad: treat preservation-mode config-only writes as this image-mode path, or
  inject a bearer token merely because the image header is present while
  `requires_openai_auth` remains `true`.

### 6. Tests Required

- `quick_setup_request_writes_image_extension_and_websocket_features`
- `quick_setup_request_disabling_image_keeps_requires_openai_auth_true`
- `quick_setup_request_derives_the_fixed_provider_shape` (no bearer token)
- `active_provider_disables_openai_auth_only_for_explicit_false`
- `project_live_config_injects_bearer_token_only_when_openai_auth_is_disabled`
- `provider_service_switch_codex_projects_bearer_token_when_openai_auth_disabled`

### 7. Wrong vs Correct

#### Wrong

```text
imageExtension true -> requires_openai_auth = false
live auth.json OPENAI_API_KEY = apiKey
live config.toml has no experimental_bearer_token
```

#### Correct

```text
imageExtension true -> requires_openai_auth = false
live auth.json OPENAI_API_KEY = apiKey
[model_providers.<id>].experimental_bearer_token = apiKey
```
