# Codex Provider Configuration Contract

## 1. Scope / Trigger

Read this contract before changing Codex Provider TOML analysis or mutation,
native capability controls, vendor-specific model projection, session-resume
command construction, provider warnings, the `liveConfigChanged` result, or
the Codex Provider Change Plan ledger/readback path.
It owns the Codex provider configuration domain only. Trusted Codex Desktop
discovery, installation, process restart, and launch are owned by
[Codex Desktop Installer](./codex-desktop-installer.md); application version and
release metadata are owned by their dedicated contracts. Managed ChatGPT
OAuth accounts, `credential_id` vs `chatgpt_account_id` routing identity,
and native `auth.json` projection are owned with
[External Agent P0 Safety](./external-agent-p0.md) and the Codex OAuth store:
Provider rows store `ProviderMeta.authBinding.accountId = credential_id` only.
Native file projection is allowed only when live `cli_auth_credentials_store`
is explicitly `file`; `keyring`, `auto`, `ephemeral`, unset, and unknown fail
closed. Workspace/account IDs are never HashMap keys.

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
  `requires_openai_auth = false` in the stored TOML and sets
  `experimental_bearer_token` on the active `[model_providers.<id>]` table.
  Disabling image-extension restores stored `requires_openai_auth = true` and
  omits the stored bearer field. The stored Provider always keeps
  `auth.OPENAI_API_KEY`.
- Third-party live writes are always config-only: they never create, replace,
  or delete `auth.json`. `prepare_codex_provider_live_config` projects the
  stored API key onto `experimental_bearer_token` so Codex can authenticate
  without touching the ChatGPT login cache. This is a hard invariant, not the
  leftover `preserveCodexOfficialAuthOnSwitch` setting.
- Official live writes with ChatGPT login material may still write `auth.json`
  only when the live credential store is an explicit `file` store.
  `project_codex_live_config_when_openai_auth_disabled` injects a bearer token
  only when that official table sets `requires_openai_auth = false`.

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

- The reusable executor contract (typed adapter descriptor, five durable
  phases, idempotent replay, cancellation, partial truth, event ordering, and
  crash recovery) is owned by
  [Change Plan Typed Executor](./change-plan-executor.md). This section owns
  only the Codex-specific Provider/projection/security semantics layered on
  that executor.
- Schema 20 owns local-only `change_plans`, `change_jobs`, and append-only
  `change_job_events`. Fresh creation and v19 migration call the same
  idempotent table helper; WebDAV sync skips and locally preserves all three.
- `create_codex_provider_switch_plan` runs under the existing Provider mutation
  guard, reads DB/device/live baselines, and writes only the credential-free
  ledger. It performs no Provider mutation or network request. The plan expires
  after 15 minutes and stores separate DB/device current IDs.
- `create_codex_provider_upsert_plan` accepts the closed Codex Quick Setup
  request, converts it to the reserved Quick Setup Provider, and likewise
  writes only the credential-free ledger. The intended Provider is held in a
  process-private draft until apply; a lost process makes the plan `stale`.
  The public plan names save-then-set-current as the operation without a
  second confirm payload.
- Switch admission accepts only an existing saved Codex Provider whose
  already-saved material proves that no new credential is needed. Upsert
  admission proves the same capability from the process-private intended
  Provider and does not call SecretRef. Unknown or managed auth is
  `secret_dependency_unavailable`; API keys, auth objects, raw config, paths,
  SecretRef/Keychain values, and credential-derived values never enter DTOs,
  ledger rows, errors, or logs.
- `apply_change_plan` accepts only `planId + planDigest`, reacquires the same
  Provider guard, rechecks contract/digest/TTL/consumption/baselines/secret
  capability, atomically consumes the plan, and invokes the lock-held Provider
  writer at most once. Invalid, expired, stale, secret-blocked, or changed-
  digest requests invoke it zero times. A same-digest replay of a consumed v2
  plan returns the existing execution snapshot as an idempotent replay and
  likewise invokes the writer zero additional times.
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
  a confirmed original baseline is failed/restored. If the executor proves the
  interruption occurred before managed write, it reports
  `interrupted_before_write`; if an unknown post-write outcome is later proven
  at the target, it reports `recovered_target_reached`.

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
| Change Plan admission is invalid, expired, stale, secret-blocked, or uses a changed digest          | Return a closed error code and invoke the Provider writer zero times.                                                   |
| A consumed v2 Change Plan is reapplied with the exact same digest                                   | Return the already-created execution as `idempotent_replay`; invoke the Provider writer zero additional times.          |
| Change Plan readback is mixed/unavailable                                                            | Persist `recovery_required`; later recovery performs readback only and never replays the writer.                        |
| Change Plan targets the fixed Quick Setup row while live TOML contains unrelated user content        | Preview and writer use the same targeted projection; preserved content does not create a false readback mismatch.       |
| Codex image-extension is enabled (`requires_openai_auth = false`) and the Provider has an API key    | Stored and live `[model_providers.<id>]` contain `experimental_bearer_token` equal to `auth.OPENAI_API_KEY`.            |
| Codex image-extension is disabled (`requires_openai_auth = true`)                                    | Stored TOML has no image-mode bearer token; the stored Provider still keeps `auth.OPENAI_API_KEY`.                      |
| Third-party Codex live write (any leftover preserve setting)                                         | Config-only; live `auth.json` bytes unchanged; API key projected to `experimental_bearer_token`.                        |
| Official live write, `requires_openai_auth` missing or `true`                                        | Do not inject a bearer token via the official helper; file-store official writes may still update `auth.json`.          |
| Two ChatGPT users share one workspace/account routing ID                                             | Store two `credential_id` rows; never use the workspace ID as the HashMap key.                                          |
| Provider `authBinding.accountId` still holds a v1 workspace ID that maps to exactly one credential   | Remap that binding to the new `credential_id`.                                                                          |
| Provider `authBinding.accountId` is missing, already a credential, or maps to multiple credentials   | Unbind; never guess the default or another account.                                                                     |
| Bound managed credential is missing/expired during proxy forwarding                                  | Fail closed; do not send another account's token.                                                                       |
| Live `cli_auth_credentials_store` is `keyring`, `auto`, `ephemeral`, unset, invalid, or unknown      | `native_projection_available=false`; do not write `auth.json` to fake a switch.                                         |
| Native projection writes `auth.json` because the file already exists                                 | Contract regression; file existence is not a store hint.                                                                |
| Auth DTO/log/Debug serializes access/refresh tokens                                                  | Security regression.                                                                                                    |

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
- Good: two managed ChatGPT logins that share one Team workspace keep distinct
  `credential_id` values; Provider binding stores only that ID.
- Bad: key the OAuth store by `chatgpt_account_id`, copy a token package onto
  the Provider row, or overwrite `auth.json` while the live store is `keyring`.

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
- Auth-projection tests cover image-on stored shape writing both
  `experimental_bearer_token` and stored `auth.OPENAI_API_KEY`, image-off
  stored TOML omitting the bearer field, third-party live switches never
  rewriting `auth.json` while projecting the key to
  `experimental_bearer_token`, the official helper injecting the token only
  when `requires_openai_auth = false`, and leftover preserve=false not
  restoring overwrite.
- Change Plan tests cover 0/v19 to schema 20, sync skip/local preserve,
  zero-side-effect planning, 15-minute expiry, concurrent single admission,
  writer exactly once/zero on rejection, same-plan idempotent replay,
  pre-write cancellation, five-phase durable event ordering,
  normal/backup-only/live-takeover
  projection parity, fixed-Quick-Setup targeted-patch projection parity,
  credential-negative persistence/serialization, and recovery-required
  readback convergence/fault recovery without replay. Generic executor tests
  and the shared v2 DTO fixture are specified in
  [Change Plan Typed Executor](./change-plan-executor.md).
- Codex OAuth store tests cover v2 `credential_id` keys, same-workspace two
  users, v1 backup + idempotent migrate, unique vs ambiguous Provider binding
  remap, bound-missing fail-closed forwarding, `auth_cancel_login` actually
  dropping the pending device flow, Debug/DTO token redaction, explicit `file`
  native projection, and fail-closed `keyring`/`auto`/`ephemeral`/unset/unknown
  without consulting `auth.json` existence.

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
  + stored auth.OPENAI_API_KEY = apiKey
imageExtension false -> requires_openai_auth = true
  + stored auth.OPENAI_API_KEY = apiKey
  + no stored experimental_bearer_token
third-party live write -> config-only + live experimental_bearer_token
  + auth.json unchanged
managed ChatGPT account map key = credential_id
  + chatgpt_account_id is routing metadata only
  + Provider.authBinding.accountId = credential_id
native projection only when cli_auth_credentials_store = "file"
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
- Quick Setup is always config-only. `auth.json` is not written and is
  absent from `writeTargets`. The submitted key is projected through the
  provider-scoped bearer-token path in `config.toml`. The writer must not
  parse an untouched `auth.json` merely to perform this config-only write;
  official ChatGPT login bytes are not a write prerequisite.
- The leftover `preserveCodexOfficialAuthOnSwitch` setting is ignored. There
  is no compatibility mode that writes third-party keys into `auth.json`.
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
| Quick Setup would need `auth.json` to be a JSON object                      | Do not parse or write it; continue the config-only path                |
| Required backup cannot be created or source permissions cannot be preserved | Reject; primary file remains byte-for-byte unchanged                   |
| Config exists for a third-party Quick Setup write                           | Back up/write config only; leave auth bytes untouched                  |
| Untouched `auth.json` is missing or not parseable                           | Config-only Quick Setup still does not parse/write auth; preserve bytes exactly |
| Existing unrelated provider/header/MCP/feature fields are present           | Preserve them while changing only owned Quick Setup fields             |
| Fixed historical row omits a modern optional owned field                    | Preserve the corresponding current live field; do not invent a default |

### 5. Good / Base / Bad Cases

- Good: a large hand-written config keeps comments, other providers, custom
  headers, MCP, features, and `disable_response_storage = false`; only the
  selected Quick Setup route changes and the backup equals the exact old file.
- Base: first creation has no config preimage, so no backup file is fabricated;
  the new config is still created atomically.
- Good: official ChatGPT login stays byte-identical in `auth.json` and the
  third-party key is placed in the active provider bearer field only.
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
  and live config contains the required provider-scoped bearer token. The
  leftover preserve setting being false must not change this outcome.
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
  stored `experimental_bearer_token` equals the same `apiKey`.
- Image off: stored `requires_openai_auth = true`; no stored image-mode
  bearer token.
- Third-party live write: never write `auth.json`. Always run
  `prepare_codex_provider_live_config` so the stored API key is projected
  onto live `experimental_bearer_token`.
- Official live write: if the active table's `requires_openai_auth` is
  explicitly `false`, `project_codex_live_config_when_openai_auth_disabled`
  injects `auth.OPENAI_API_KEY` onto `experimental_bearer_token`. Missing or
  `true` leaves that official TOML unchanged. File-store official writes may
  still update `auth.json`.
- Environment: live files remain `~/.codex/auth.json` and
  `~/.codex/config.toml`. No new env key.

### 4. Validation & Error Matrix

- Empty `apiKey` -> quick setup rejects before TOML derivation.
- Invalid TOML at live write -> existing parse error; do not synthesize a
  bearer token onto a document that cannot be parsed.
- No API key and `requires_openai_auth = false` -> do not invent a token;
  `prepare_codex_provider_live_config` leaves the text unchanged.

### 5. Good/Base/Bad Cases

- Good: image-on quick setup stores the API key on the Provider and lives it
  as `experimental_bearer_token` without rewriting `auth.json`.
- Base: image-off quick setup is still config-only; live `auth.json` stays
  byte-identical and the live provider table receives the bearer projection.
- Bad: write third-party `OPENAI_API_KEY` into live `auth.json`, or inject a
  bearer token onto an official file-store write merely because an image
  header is present while `requires_openai_auth` remains `true`.

### 6. Tests Required

- `quick_setup_request_writes_image_extension_and_websocket_features`
- `quick_setup_request_disabling_image_keeps_requires_openai_auth_true`
- `quick_setup_request_derives_the_fixed_provider_shape` (no bearer token)
- `active_provider_disables_openai_auth_only_for_explicit_false`
- `project_live_config_injects_bearer_token_only_when_openai_auth_is_disabled`
- `provider_service_switch_codex_projects_bearer_token_when_openai_auth_disabled`
- `provider_service_switch_codex_default_preserves_official_auth`

### 7. Wrong vs Correct

#### Wrong

```text
imageExtension true -> requires_openai_auth = false
live auth.json OPENAI_API_KEY = apiKey
live config.toml has no experimental_bearer_token
```

#### Correct

```text
imageExtension true -> stored requires_openai_auth = false
stored auth.OPENAI_API_KEY = apiKey
live config.toml experimental_bearer_token = apiKey
live auth.json unchanged
```

## Scenario: Codex managed credential identity and native projection

### 1. Scope / Trigger

- Trigger: the managed ChatGPT OAuth store, Provider binding IDs, proxy
  routing headers, Auth Center DTO, and native `auth.json` projection all
  changed together. This is a persisted-schema plus cross-layer command
  contract, so code-spec depth is mandatory.
- Owner: `proxy/providers/codex_oauth_auth.rs` is the only token SSOT.
  `ProviderMeta.authBinding` stores IDs only. Native file projection is
  `codex_config/credential_store.rs` plus existing
  `codex_config/{auth,storage}.rs` writers. Agent Catalog install actions
  are owned by [External Agent P0 Safety](./external-agent-p0.md).

### 2. Signatures

```text
CodexOAuthStore v2 {
  version: 2,
  accounts: HashMap<credential_id, CodexAccountData>,
  default_account_id?: credential_id
}

CodexAccountData {
  credential_id,            // FyAgent UUID; map key; never workspace id
  chatgpt_account_id,       // upstream routing/workspace identity only
  email?,
  refresh_token,            // disk only; never DTO/log/Debug
  authenticated_at
}

ManagedAuthAccount {
  id,                       // credential_id
  provider, login, avatar_url?, authenticated_at,
  is_default, github_domain, requires_reauth,
  chatgpt_account_id?       // routing metadata for display; not a key
}

ManagedAuthStatus {
  provider, authenticated, default_account_id?,
  migration_error?, accounts,
  native_projection_available?   // Codex only; true only for explicit file
}

auth_start_login(authProvider, githubDomain?)
auth_poll_for_account(authProvider, deviceCode, githubDomain?)
auth_list_accounts(authProvider)
auth_get_status(authProvider)
auth_remove_account(authProvider, accountId)   // accountId = credential_id
auth_set_default_account(authProvider, accountId)
auth_logout(authProvider)
auth_cancel_login(authProvider, deviceCode?)   // Codex: abort pending device flow

parse_cli_auth_credentials_store(config_toml)
  -> File | Keyring | Auto | Ephemeral | Unset | Unknown | ConfigInvalid

native_file_projection_allowed(config_toml) -> bool
overlay_cli_auth_credentials_store(outgoing, current_live) -> toml
```

`authProvider` remains `github_copilot | codex_oauth | xai_oauth`. No new
Auth Center. No environment key.

### 3. Contracts

- Credential identity and ChatGPT workspace/account identity are different
  fields. Login completion generates a random `credential_id`. Duplicate
  rows from repeated logins are accepted; merging people by workspace ID is
  forbidden until a verified stable user claim exists.
- Provider rows keep `authBinding.authProvider = "codex_oauth"` and
  `authBinding.accountId = credential_id`. They never copy access/refresh
  tokens.
- Proxy `ChatGPT-Account-Id` is the routing id looked up from the bound
  credential. Missing/empty routing id fails closed. A bound credential that
  disappears does not fall back to default or another account.
- v1→v2 migration: backup `codex_oauth_auth.json.v1.bak` once (keep it),
  assign a new UUID per row, preserve the old workspace id as
  `chatgpt_account_id`, remap a Provider binding only when the old id maps
  to exactly one new credential, otherwise unbind. Lost collision data is
  not reconstructed. Re-running on an already-v2 store is a no-op.
  Remap runs only after a successful store load (`store_loaded=true`); a
  parse/IO failure must not treat the empty in-memory map as “no accounts”
  and clear every Codex `authBinding`.
- Credential source and upstream destination stay independent. Official vs
  custom endpoint, proxy takeover, and managed-account binding must not be
  collapsed into one `is_official` boolean.
- Native Codex projection writes `auth.json` only when live
  `cli_auth_credentials_store` is the string `file`. `keyring`, `auto`,
  source-visible `ephemeral`, unset, non-string, and future values set
  `native_projection_available=false` and tell the user to use Codex login.
  Existence of `auth.json` is never a store selector.
- Empty official snapshots that omit `cli_auth_credentials_store` must
  overlay the current live value so a later switch cannot silently drop
  file-mode projection.
- `auth_cancel_login` for `codex_oauth` must drop the pending device-code
  poll. GitHub Copilot / xAI currently no-op rather than invent a second
  cancel protocol.
- Auth Center distinguishes “managed account usable for FyAgent routing”
  from “native Codex projection available”. Refresh-token remaining source
  of truth is the managed store. Reconciling a Codex-rotated refresh token
  into that store before file-mode projection is still residual: identity
  must match first; this iteration does not implement the full live
  readback path. OAuth HTTP errors may include the status code and must
  not echo the response body.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Store keyed by `chatgpt_account_id` | Schema/test fails |
| Bound credential missing during forward | Auth error; no other account |
| Ambiguous v1 workspace binding | Unbind; do not pick default |
| Non-file credential store | No `auth.json` write; UI shows native projection unavailable |
| Invalid `config.toml` while resolving store | Treat as not-file; do not guess |
| Token in DTO, log, Debug, ledger, DOM | Security regression |
| OAuth HTTP failure embeds response body | Security regression; errors may include status only |
| Store load failed (`store_loaded=false`) then remap bindings | Forbidden; empty in-memory map must not unbind every Provider |
| `auth_cancel_login` leaves Codex device poll running | Contract regression |

### 5. Good/Base/Bad Cases

- Good: Alice and Bob in workspace `ws-shared` persist as two UUID keys
  with the same `chatgpt_account_id`.
- Base: a file-store user can project the selected managed credential
  through the existing atomic `auth.json` + `config.toml` writer.
- Bad: CC Switch-style unconditional `auth.json` overwrite under `keyring`,
  or infer file mode because `auth.json` exists.

### 6. Tests Required

- `same_workspace_two_users_coexist_under_distinct_credential_ids`
- v1 fixture migrates, writes `.v1.bak` once, and is idempotent on v2
- unique vs many-to-one Provider `authBinding` remap
- forwarder fail-closed when routing id is missing
- `only_explicit_file_allows_native_projection` plus
  `auth_json_existence_is_not_consulted`
- overlay preserves live `cli_auth_credentials_store` when outgoing omits it
- Debug redacts `refresh_token`
- Auth Center/status DTO has `id` + optional `chatgpt_account_id` and no
  token fields

### 7. Wrong vs Correct

#### Wrong

```rust
accounts.insert(chatgpt_account_id, account);
if Path::new("~/.codex/auth.json").exists() {
    write_auth_json(token_package)?;
}
```

#### Correct

```rust
accounts.insert(credential_id, CodexAccountData { credential_id, chatgpt_account_id, .. });
if parse_cli_auth_credentials_store(&live_toml)? == CodexCredentialStore::File {
    project_auth_json_for_credential(&credential_id)?;
} else {
    return native_projection_unavailable();
}
```

### Design Decision: credential UUID instead of workspace map key

**Context**: ChatGPT Team/Business `chatgpt_account_id` is a workspace routing
id. Two people in one workspace collided when it was the store key
(CC Switch #5885 class). OpenAI tokens do not currently prove a stable user
claim we can use as identity.

**Options Considered**:

1. Keep workspace id as the map key
2. Deduplicate on email
3. Generate a FyAgent `credential_id` UUID at login

**Decision**: Option 3. Duplicate rows from repeated logins are safer than
merging distinct people. Provider bindings store that UUID only.

**Example**:

```text
accounts[credential_uuid] = { credential_id, chatgpt_account_id: "ws-shared", refresh_token }
ProviderMeta.authBinding.accountId = credential_uuid
```

**Extensibility**: Later dedup requires positive identity evidence, not
workspace or email heuristics.

### Common Mistake: 存储没加载成功就 remap binding

**Symptom**: Codex OAuth JSON is unreadable, then every Provider
`authBinding` disappears.

**Cause**: remap saw an empty in-memory map and treated every old workspace
id as ambiguous/missing.

**Fix**: set `store_loaded` only after a successful read; skip remap otherwise.

**Prevention**: tests must cover corrupt/missing store without mutating
Provider bindings.

### Common Mistake: 用 auth.json 是否存在判断 Codex store

**Symptom**: `keyring`/`auto` users appear to switch native Codex accounts,
then Codex ignores `auth.json`.

**Cause**: File existence is not `cli_auth_credentials_store`.

**Fix**: Parse the live TOML field. Only explicit `file` may project.

**Prevention**: `native_file_projection_allowed` must not take a filesystem
path to `auth.json`.
