# Managed Auth Consumer Projection Contract

## 1. Scope / Trigger

Read this contract before changing managed account connections to Codex, Grok
Build, or OpenCode Desktop; connection observation; native auth-file
projection; refresh-owner transfer; pending-restart behavior; or
`managed_auth_apply_connection_action`.

Primary owners:

- `src-tauri/src/services/managed_auth/consumers/codex.rs`
- `src-tauri/src/services/managed_auth/consumers/grok.rs`
- `src-tauri/src/services/managed_auth/consumers/opencode.rs`
- consumer orchestration in `services/managed_auth/{login,service,providers/xai}.rs`
- the connection action in `commands/managed_auth.rs`

[Managed Auth Core](./managed-auth.md) owns account/credential metadata,
SecretRef material and refresh CAS. [Managed Auth Login](./managed-auth-login.md)
owns provider grants and backend login sessions. Codex Provider TOML and its
config-only third-party writer are owned by
[Codex Provider Configuration](./codex-provider-configuration.md). Agent-card
observation and handoff semantics remain in
[External Agent Auth](./external-agent-auth.md).

## 2. Signatures

```text
managed_auth_apply_connection_action({
  connectionId: mc1:<32-lowercase-hex>,
  expectedRevision: mr1:<64-lowercase-hex>,
  action: connect_account | switch_account | disconnect | refresh |
          restart | open_consumer | switch_to_official,
  accountId?: ma1:<32-lowercase-hex>
}) -> ManagedAuthMutationResult | ManagedAuthErrorDto
```

`accountId` is required only for `connect_account` and `switch_account`.
Every positive result contains a freshly reread overview; the renderer does not
patch a connection optimistically.

Consumer boundaries:

```text
codex::observe_codex_home(path) -> CodexObservation
codex::file_projection_enabled() -> false until matching-host HIL

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
```

## 3. Contracts

### Shared connection boundary

- A managed account and a software connection are separate resources. A ready
  credential does not prove that the consumer has accepted or is using it.
- Every connection request carries a syntactically valid `expectedRevision`,
  but enforcement is consumer-specific. OpenCode connect/switch/disconnect
  compares the current `auth.json` revision under its process-wide writer lock,
  and restart acknowledgement compares the latest observation before updating
  metadata. Codex/Grok metadata-only refresh/connect/disconnect paths do not yet
  independently compare the current connection revision. Their production
  vendor-file gates remain closed, so this residual cannot authorize a native
  file write, but it must not be described as full cross-consumer CAS.
- Consumer adapters receive secret material only inside native code. Tokens,
  SecretRef, raw auth-file bytes, paths, helper output and provider sidecar
  details never cross IPC.
- Credential purpose and refresh owner are explicit. Never copy one refresh
  lineage among Proxy, Codex, Grok, OpenCode, or Copilot to simulate a
  connection.
- A successful file API call is not enough. Readback, owner transfer, external
  pickup evidence and recovery state jointly determine connection status and
  mutation outcome. A completed mutation may still truthfully require
  `pending_restart`; a partial mutation is not a connected state.
- Codex, Grok, and OpenCode summaries currently emit `target_id: None`. That is
  not lifecycle install discovery and is not evidence the software is missing.
  `requestMode` is observation of the consumer's current model source (for
  Codex, `config.toml`); it does not prove that managed auth rewrote
  `auth.json` or `model_provider`.

### Known Codex/Grok summary residual

- `service.rs::build_connection_summaries` currently passes the first ready
  `codex_native` / `grok_native` credential to the slot summary independently
  of whether a connection row names that credential.
- `consumers/codex.rs::connection_summary` currently maps credential presence
  to `authStatus=connected` even while the production projection gate is false;
  it also emits `native_projection_unavailable`. Grok maps the equivalent
  credential-only state to `unavailable`. Neither shape proves native consumer
  pickup, and the Codex shape is an explicit evidence mismatch.
- Do not cite these summaries as proof that account/connection separation is
  fully enforced. Before strengthening the contract, derive the selected
  account from the explicit connection row, require projection/pickup evidence
  for `connected`, and add regression coverage for the credential-only state.

### Codex managed connection

- `CODEX_FILE_PROJECTION_PRODUCTION_ENABLED` remains `false` until signed,
  matching-host file-store HIL proves write/readback and Codex pickup.
- Without that evidence, connect or login-to-connect stores the managed
  credential but ends `partial` with `native_projection_unavailable` and writes
  no vendor `auth.json`. Production Codex also does not advertise
  `switch_to_official`. The current summary may nevertheless report
  `connected` from credential presence as described above; that value is not
  live Codex evidence and must not be used to rewrite `~/.codex/auth.json` or
  switch `model_provider`.
- Codex Provider third-party API-key switches remain config-only under the
  Codex Provider contract. They do not become evidence for a managed ChatGPT
  connection.
- A future projection may run only when the live Codex credential store is
  explicitly `file`; `keyring`, `auto`, `ephemeral`, unset, invalid, unknown,
  or mere `auth.json` existence are not admission evidence.

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
  changed. This is a known recovery residual, not an atomic rollback: do not
  claim that this branch proved FyAgent ownership or live Desktop pickup, and
  require compensation/reconciliation before strengthening the contract.
- Copilot login is not provided by Managed Auth. A legacy Copilot row without
  stable identity remains blocked and must not be projected.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| connection/account/revision is malformed | reject before dispatch; no file or metadata mutation |
| OpenCode write/delete/restart sees a stale revision | reject; leave the official file and connection metadata unchanged |
| Codex/Grok metadata action carries a stale-but-well-formed revision | current path does not independently compare it; trust only the returned reread overview and retain this as hardening residual |
| OpenCode is offered Proxy/Codex/Grok/Copilot lineage | `provider_not_supported`; do not copy lineage |
| Codex/Grok has no purpose-compatible ready credential | `native_projection_unavailable`; no vendor file write |
| Codex projection gate is false | `partial` + `native_projection_unavailable`; no vendor file write; no `switch_to_official` |
| Codex/Grok/OpenCode summary has `target_id: None` | slot is unbound to a lifecycle install; not missing-install evidence |
| Codex has a ready `codex_native` credential while projection is unavailable | current summary may contain `connected` + `native_projection_unavailable`; known evidence mismatch, not native pickup |
| Grok has a ready `grok_native` credential while projection is unavailable | current summary is `unavailable` + `native_projection_unavailable`; not native pickup |
| credential exists but no connection row names it | current Codex/Grok overview may still expose that account; do not treat it as an explicit connection |
| Grok helper/file gate is false | `Unsupported` / `partial`; no vendor file write |
| Proxy tries to resolve `purpose=grok_native` | conflict; no refresh |
| OpenCode data dir exists but PATH CLI does not | observe `auth.json`; not `AuthObserverUnavailable` |
| OpenCode `auth.json` is missing | empty provider set; not observer failure |
| OpenCode readback differs | restore exact preimage or remove new file; report failure/recovery |
| OpenCode write succeeds while hot reload is unproven | `pending_restart`; not `connected` and not `native_projection_unavailable` |
| OpenCode connect is offered only a Proxy/Codex/Grok/Copilot credential | `provider_not_supported`; no write |
| OpenCode refresh-owner CAS returns false after readback | retain pending-restart connection evidence; return `partial_completion` |
| OpenCode refresh-owner repository call errors after readback | return error; file may already be changed and requires recovery/reconciliation |
| sidecar port/password is supplied, scanned, or logged | security regression; no probe |
| token, SecretRef, auth bytes, native path, or raw helper output reaches DTO/log/DOM | security regression |

## 5. Good / Base / Bad Cases

- **Good:** OpenCode replaces only the `openai` entry, preserves unknown keys,
  writes atomically with `0600`, rereads equal bytes, transfers ownership, and
  reports `pending_restart` until Desktop pickup is HIL-proven.
- **Base:** OpenCode has no `auth.json`; observation returns an empty provider
  set without requiring a CLI.
- **Base:** Codex or Grok login stores a valid credential while projection
  gates remain closed, so the mutation result is partial. Grok stays
  unavailable; Codex's current credential-presence `connected` summary remains
  a named evidence mismatch rather than a live native-login claim.
- **Bad:** infer Codex file mode from file existence, copy a Proxy refresh token
  into OpenCode/Grok, treat CLI installation as auth evidence, or paint a file
  write as a live connection without readback and pickup evidence.

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

- exact connection request shape and consumer-specific revision behavior:
  OpenCode stale file revisions reject under the writer lock; Codex/Grok tests
  must not claim full CAS until their metadata paths compare current revision;
- Codex and Grok production gates remain false and write zero vendor bytes;
  production Codex does not advertise `switch_to_official`;
- Codex/Grok/OpenCode `target_id` is currently `None` and tests must not treat
  that as missing-install evidence;
- credential-only Codex/Grok summaries are not used as HIL evidence; before
  resolving the named Codex mismatch, add a regression that derives account
  linkage from the connection row and gates `connected` on projection/pickup;
- Proxy rejects `grok_native` even while FyAgent temporarily owns refresh;
- OpenCode observation has no PATH CLI dependency and missing file is empty;
- closed-key RMW preserves unknown/undecodable/`wellknown` values and extra
  fields; Unix mode is `0600`;
- stale revision and readback mismatch leave/restore the authoritative file;
- OpenAI/xAI `consumer=opencode` creates `opencode_provider`, never copies
  another purpose, and transfers owner only after readback;
- owner-transfer CAS miss returns partial with retained pending-restart
  metadata; repository-error-after-write remains explicit recovery coverage;
- `OPENCODE_EXTERNAL_WRITE_HOT_RELOAD_PROVEN=false` yields
  `pending_restart` after a successful write;
- Agent Auth remains sanitized, OpenCode provider-scoped, and Grok handoff-only;
- matching-host connect/refresh/disconnect/external-change/restart HIL remains
  required before enabling a gate or claiming live pickup.

## 7. Wrong vs Correct

Wrong:

```text
select any ready account -> copy its refresh token into consumer auth.json
atomic_write Ok -> connected
missing PATH opencode -> observer unavailable
installed Grok CLI -> logged in
```

Correct:

```text
OpenCode selects a purpose-compatible credential under auth.json revision
consumer-specific write -> readback -> refresh-owner transfer
external pickup unproven -> pending_restart
closed production gate -> partial/native_projection_unavailable with zero write
Desktop auth.json observation is independent of PATH CLI
Codex/Grok metadata actions use the returned overview; full stale-revision CAS
is not claimed until their service paths enforce it
Codex connected + native_projection_unavailable -> known evidence mismatch,
not proof that native Codex accepted the credential
```
