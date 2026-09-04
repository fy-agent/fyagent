# Managed Auth Consumer Projection Contract

## 1. Scope / Trigger

Read this contract before changing managed account connections to Codex, Grok
Build, or OpenCode Desktop; connection observation; native auth-file
projection; refresh-owner transfer; pending-restart behavior; or
`managed_auth_apply_connection_action`.

Primary owners:

- `src-tauri/src/services/managed_auth/consumers/codex/mod.rs`
- `src-tauri/src/services/managed_auth/consumers/codex/{auth_document,delta,observation,project,swap}.rs`
- `src-tauri/src/services/managed_auth/consumers/grok.rs`
- `src-tauri/src/services/managed_auth/consumers/opencode.rs`
- consumer orchestration in
  `src-tauri/src/services/managed_auth/{login,service,providers/xai}.rs`
- the connection action in `src-tauri/src/commands/managed_auth.rs`

[Managed Auth Core](./managed-auth.md) owns account/credential metadata,
SecretRef material and refresh CAS. [Managed Auth Login](./managed-auth-login.md)
owns provider grants and backend login sessions. Codex Provider TOML and its
config-only third-party writer are owned by
[Codex Provider Configuration](./codex-provider-configuration.md). Agent-card
observation and handoff semantics remain in
[External Agent Auth](./external-agent-auth.md).

## 2. Signatures

```text
async managed_auth_apply_connection_action({
  connectionId: mc1:<32-lowercase-hex>,
  expectedRevision: mr1:<64-lowercase-hex>,
  action: connect_account | switch_account | disconnect | refresh |
          restart | open_consumer | switch_to_official,
  accountId?: ma1:<32-lowercase-hex>
}) -> ManagedAuthMutationResult | ManagedAuthErrorDto
```

`accountId` is required only for `connect_account` and `switch_account`.
`switch_to_official` requires an explicit connection-bound credential; it must
not pick an implicit default account. Every positive result contains a freshly
reread overview; the renderer does not patch a connection optimistically.
The command validates the closed request before offloading the synchronous
service call through `tauri::async_runtime::spawn_blocking`. The Tauri command
thread must not directly run the blocking credential locks, filesystem work,
or nested runtime bridge used by the consumer coordinator. A blocking-task
join failure maps to source-free `invalid_response`.

Consumer boundaries:

```text
codex::observe_codex_home(path) -> CodexManagedAuthObservation
codex::plan_codex_managed_auth_delta(live, target) -> Noop|AuthOnly|ProviderOnly|AuthThenProvider
codex::project_codex_official_account(app_state, home, subject, doc, expected_rev)
  -> CodexProjectionOutcome
codex::file_projection_enabled() -> true when capability is generally available;
  unsupported effective stores still fail closed at plan time

grok::project_grok_native(home, store) -> Result<(), GrokStoreError>
grok::auth_provider_command_enabled() -> false until matching-host HIL
grok::file_projection_enabled() -> false until matching-host HIL

opencode::observe_auth_store(path) -> OpencodeAuthObservation
opencode::upsert_projection(path, provider, entry, expectedRevision?)
  -> AuthJsonWriteReceipt | OpencodeAuthError
opencode::remove_file_key(path, officialKey, expectedRevision?)
  -> AuthJsonWriteReceipt | OpencodeAuthError
opencode::remove_capability(path, capabilityId, expectedRevision?)
  -> AuthJsonWriteReceipt | OpencodeAuthError
opencode::connection_summaries(
  observation, credentialRows, connectionRows, checkedAt
) -> ManagedAuthConnectionSummary[]
OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN = false
CODEX_EXTERNAL_WRITE_HOT_RELOAD_PROVEN = false
```

## 3. Contracts

### Shared connection boundary

- A managed account and a software connection are separate resources. A ready
  credential does not prove that the consumer has accepted or is using it.
- The connection-action IPC boundary is asynchronous, but the service method
  remains synchronous. Validate before `spawn_blocking`; run the complete
  service call inside that worker; and never move secret material or native
  paths into an async/renderer payload merely to avoid blocking the command
  thread.
- Every connection request carries a syntactically valid `expectedRevision`,
  but enforcement is consumer-specific. OpenCode connect/switch/disconnect
  compares the current `auth.json` revision under its process-wide writer lock,
  and restart acknowledgement compares the latest observation before updating
  metadata. Codex official projection compares live auth revision under the
  Codex auth writer lock and serializes with the existing Provider mutation
  guard.
- Consumer adapters receive secret material only inside native code. Tokens,
  SecretRef, raw auth-file bytes, paths, helper output and provider sidecar
  details never cross IPC.
- Credential purpose and refresh owner are explicit. Never copy one refresh
  lineage among Proxy, Codex, Grok, OpenCode, or Copilot to simulate a
  connection.
- A successful file API call is not enough. Readback, owner transfer, external
  pickup evidence and recovery state jointly determine connection status and
  mutation outcome. When file write/readback succeeds but live pickup is not
  proven, the exact positive wire shape is `outcome=completed` with
  `reasonCode=pending_restart`. The returned overview remains authoritative
  about which connection is pending; a partial mutation is not a connected
  state.
- Codex, Grok, and OpenCode summaries currently emit `target_id: None`. That is
  not lifecycle install discovery and is not evidence the software is missing.
  `requestMode` is observation of the consumer's current model source (for
  Codex, `config.toml`); it does not alone prove that managed auth rewrote
  `auth.json`.

### Codex managed connection

- Codex file-store projection is capability-gated by machine-checkable facts:
  effective store is unset/default-file or explicit file; complete identity-
  matched ChatGPT auth material; revision CAS; atomic write + `0600`; auth and
  (when needed) Provider route readback. Matching-host HIL is optional smoke
  evidence and does not control a production boolean gate.
- Effective defaults follow the pinned OpenAI Codex contract: unset
  `cli_auth_credentials_store` → file; missing `model_provider` → openai.
  Explicit `auto` / `keyring` / `ephemeral` / unknown values fail closed with
  zero auth writes and are not silently rewritten to file.
- Connected status requires live ChatGPT identity to match the connection-
  bound credential. A ready SecretRef alone is saved-not-projected /
  disconnected, never connected.
- Minimum write delta:
  - target account + official route → no-op
  - other/missing account + official route → auth.json only
  - target account + third-party route → Provider official switch only
  - other/missing account + third-party route → auth then Provider switch
- Provider route changes reuse
  `ProviderService::switch_with_lock_held_skipping_backfill` under the existing
  Codex mutation guard. Auth-only paths take the same guard but do not call the
  Provider writer. No new Change Plan operation is added.
- Legacy API-key-only live auth must be recoverable via Provider current
  backfill before auth overwrite. Official→third-party continues the existing
  Provider/Change Plan path and must keep `auth.json` bytes equal.
- After a successful auth write while hot reload is unproven, return
  `completed` + `pending_restart`, and persist the Codex connection itself as
  pending in the authoritative overview. Do not emit
  `native_projection_unavailable` for a successful write, and do not translate
  this positive state into a generic retry failure.
- `switch_to_official` is advertised only when live identity already matches
  the bound credential and the route is third-party under a supported store.

### Grok fail-closed consumer

- `GROK_AUTH_PROVIDER_COMMAND_ENABLED` and
  `GROK_FILE_PROJECTION_PRODUCTION_ENABLED` remain `false` until matching-host
  helper/file-lock/home-selection HIL exists.
- A Grok connection uses a separate `purpose=grok_native` credential. It is
  never Proxy-resolved and is not copied from `purpose=proxy_upstream`.
- `project_grok_native` returns `Unsupported` while the gate is closed and
  writes no `auth.json`. Connect or login-to-connect remains `partial` with
  `native_projection_unavailable`.
- When Grok tooling is available, Agent Auth observation stays
  `handoff_only`; unavailable tooling yields an unavailable observation. In
  neither case are CLI installation, a vault row, or opening a vendor page
  verified Grok login evidence.

### OpenCode Desktop `auth.json`

- Resolve the official Desktop data path through
  `opencode_config::get_opencode_auth_json_path`; observation and Path B writes
  do not require a PATH `opencode` CLI.
- Closed file keys are `openai`, `xai`, and `github-copilot`. Preserve every
  unrelated provider, undecodable value, `wellknown` entry, and extra official
  field not owned by the replacement operation.
- Environment/`OPENCODE_AUTH_CONTENT` providers are not file rows. Do not
  invent, delete, or claim to observe them through `auth.json`.
- The private Desktop sidecar and `OPENCODE_SERVER_PASSWORD` are not a control
  plane. Do not scan, guess, persist, or probe loopback ports/passwords.
- Writes use the existing atomic file owner, apply `0600` on Unix, and require
  byte/semantic readback. A mismatch restores the exact preimage or deletes a
  newly created file; success is never inferred from `atomic_write` alone.
- `consumer=opencode` creates an independent
  `purpose=opencode_provider` credential. Proxy/Codex/Grok/Copilot purposes are
  rejected rather than copied. After successful file readback, the service
  attempts to transfer refresh ownership to `opencode`.
- External-write hot reload is not proven. A successful FyAgent write remains
  `pending_restart`; do not also emit `native_projection_unavailable`, which
  means the write itself was unavailable.
- Owner transfer occurs after file readback. A CAS miss (`Ok(false)`) records
  the connection with `pending_restart` evidence and returns `partial` /
  `partial_completion`. A hard repository error currently returns before that
  connection metadata upsert even though the official file may already have
  changed. This is a known recovery residual, not an atomic rollback.
- Copilot login is not provided by Managed Auth. A legacy Copilot row without
  stable identity remains blocked and must not be projected.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| connection/account/revision is malformed | reject before dispatch; no file or metadata mutation |
| synchronous connection service runs directly on the Tauri command thread | contract regression; validate first, then use `spawn_blocking`; map join failure to `invalid_response` |
| OpenCode write/delete/restart sees a stale revision | reject; leave the official file and connection metadata unchanged |
| Codex auth swap sees a stale auth revision | reject; zero auth write |
| OpenCode is offered Proxy/Codex/Grok/Copilot lineage | `provider_not_supported`; do not copy lineage |
| Codex has no purpose-compatible ready credential | `target_selection_required` / unavailable; no vendor file write |
| Codex effective store is explicit auto/keyring/ephemeral/unknown | store unsupported; zero auth write |
| Codex/Grok/OpenCode summary has `target_id: None` | slot is unbound to a lifecycle install; not missing-install evidence |
| ready CodexNative credential while live identity differs | disconnected / saved-not-projected; not connected |
| live Codex identity matches bound credential | connected; may still be third-party route with session preserved |
| Grok has a ready `grok_native` credential while projection is unavailable | current summary is `unavailable` + `native_projection_unavailable`; not native pickup |
| Grok helper/file gate is false | `Unsupported` / `partial`; no vendor file write |
| Proxy tries to resolve `purpose=grok_native` | conflict; no refresh |
| OpenCode data dir exists but PATH CLI does not | observe `auth.json`; not `AuthObserverUnavailable` |
| OpenCode `auth.json` is missing | empty provider set; not observer failure |
| OpenCode readback differs | restore exact preimage or remove new file; report failure/recovery |
| OpenCode/Codex write and readback succeed while hot reload is unproven | `completed` + `pending_restart`; returned overview marks the connection pending; not `native_projection_unavailable` |
| token, SecretRef, auth bytes, native path, or raw helper output reaches DTO/log/DOM | security regression |

## 5. Good / Base / Bad Cases

- **Good:** OpenCode replaces only the `openai` entry, preserves unknown keys,
  writes atomically with `0600`, rereads equal bytes, transfers ownership, and
  reports `pending_restart` until Desktop pickup is HIL-proven.
- **Good:** Codex A→B on official route swaps only `auth.json`; third-party→
  same official account switches only Provider route; auth bytes stay equal.
- **Base:** OpenCode has no `auth.json`; observation returns an empty provider
  set without requiring a CLI.
- **Base:** Codex unset store and missing `model_provider` are effective file
  and openai; explicit keyring remains unavailable with zero write.
- **Bad:** infer Codex connected from credential presence alone, copy a Proxy
  refresh token into OpenCode/Grok, treat CLI installation as auth evidence, or
  paint a file write as a live connection without readback evidence.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test -- managed_auth
mise run rust:test -- opencode
mise run typecheck:v2
mise run test:v2 -- tests/v2/features/managed-auth.test.ts \
  tests/v2/pages/agents/AgentAuthStatusPanel.test.tsx
```

Required assertions:

- the native command validates before `spawn_blocking`, does not run the
  synchronous service on the IPC command thread, and maps blocking-task join
  failure to `invalid_response`;
- Codex effective file defaults, delta matrix, auth swap 0600/readback/CAS,
  Provider-only auth byte-equality, and saved-not-projected overview status;
- Grok production gates remain false and write zero vendor bytes;
- Codex/Grok/OpenCode `target_id` is currently `None`;
- OpenCode missing-file and no-PATH observation; closed-key read/modify/write
  preserves unrelated, undecodable, `wellknown`, and extra official fields;
- OpenCode stale revisions, readback mismatch, exact-preimage recovery, Unix
  `0600`, purpose isolation, and owner transfer only after file readback;
- OpenCode owner-transfer CAS miss returns partial with pending evidence, while
  a hard repository error keeps its documented recovery residual explicit;
- Codex and OpenCode positive writes use `completed + pending_restart`, and
  strict V2 parsing rejects every other non-null reason on a completed result;
- native sidecar/password discovery stays absent, and DTO/log/DOM leak tests
  cover tokens, SecretRef, raw auth bytes, paths, and helper output;
- Codex HIL remains optional smoke evidence, not a runtime production gate.

## 7. Wrong vs Correct

Wrong:

```text
select any ready account -> copy its refresh token into consumer auth.json
atomic_write Ok -> connected
credential present -> Codex connected
CODEX_FILE_PROJECTION_PRODUCTION_ENABLED=false forever
sync Tauri command -> blocking credential/file coordinator
```

Correct:

```text
OpenCode selects a purpose-compatible credential under auth.json revision
consumer-specific write -> readback -> refresh-owner transfer
external pickup unproven -> pending_restart
Codex live identity match -> connected; otherwise saved-not-projected
Codex capability from effective store + complete material + readback
async Tauri command -> validate -> spawn_blocking(sync service) -> strict result
```
