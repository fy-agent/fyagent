# Local Proxy Service and Takeover Contract

## 1. Scope / Trigger

Read this contract before changing `src-tauri/src/commands/proxy.rs`,
`src-tauri/src/services/proxy.rs`, or
`src-tauri/src/services/proxy/takeover.rs`: proxy start/stop/configuration,
application takeover, Provider switching, live-file backup/restore, crash
recovery, or circuit-breaker administration.

The HTTP listener, route aliases, request context, Provider-attempt admission,
retry/failover, streaming, response processing, and usage attribution are owned
by [Local Proxy Pipeline](./local-proxy-pipeline.md). Provider document
mutation remains owned by the relevant Provider/configuration contract, and
persistence mechanics remain owned by
[Database Persistence](./database-persistence.md). Upstream OAuth access
tokens for Codex/xAI/Copilot come from
[Managed Auth Core](./managed-auth.md) when a fyagent-owned vault session
exists; the forwarder must not grow a second token store.

## 2. Signatures

The reviewed Tauri command families in `commands/proxy.rs` are:

```text
start_proxy_server / stop_proxy_server / stop_proxy_with_restore
get_proxy_status / is_proxy_running
get_proxy_config / update_proxy_config

get_global_proxy_config / update_global_proxy_config
get_proxy_config_for_app / update_proxy_config_for_app
get_default_cost_multiplier / set_default_cost_multiplier
get_pricing_model_source / set_pricing_model_source

get_proxy_takeover_status / set_proxy_takeover_for_app
is_live_takeover_active / switch_proxy_provider

get_provider_health / reset_circuit_breaker
get_circuit_breaker_config / update_circuit_breaker_config
get_circuit_breaker_stats
```

`ProxyService` owns the corresponding native state and orchestration:

```text
start() / start_with_takeover()
stop() / stop_with_restore() / stop_with_restore_keep_state()
get_status() / get_config() / update_config()
get_takeover_status() / set_takeover_for_app(appType, enabled)
hot_switch_provider(appType, providerId)
recover_from_crash()
update_circuit_breaker_configs(...)
reset_provider_circuit_breaker(...)
```

Stable proxy, takeover, health, and configuration DTOs are owned by
`src-tauri/src/proxy/types.rs` and their command parsers. Renderer adapters
parse those wire values and submit closed application/Provider IDs; they never
submit an executable, filesystem path, arbitrary upstream URL, credential,
backup body, or replacement routing implementation.

## 3. Contracts

### Command and state ownership

- `commands/proxy.rs` is transport only: parse bounded wire input, acquire
  `AppState`, delegate, and map one authoritative result. It does not edit live
  files, open a listener, manipulate a breaker map, or reproduce service
  rollback logic.
- One `ProxyService` owns one in-process `ProxyServer`, the application handle,
  persisted/effective configuration, switch locks, active-target state, and
  live backup/restore sequencing. A second service or command-local server is
  not a recovery mechanism.
- Start succeeds only after the internal server has bound and reported its
  address. A second start is an already-running error; it must not create an
  orphan listener or silently replace the active configuration.
- Configuration updates validate and persist through their owning service/DAO,
  then update the running engine only through the reviewed hot-update path.
  Database success alone is not proof that a running listener adopted a value.
- Pricing source and cost multiplier are configuration inputs to usage
  attribution. They do not authorize a Provider switch or rewrite historical
  usage records.

### Stop, takeover, and restoration

- `stop` stops the server only. `stop_with_restore` is the explicit operation
  that also attempts to restore taken-over Agent configuration. Generic process
  shutdown must not silently choose the stronger restore transaction.
- Takeover is application-specific. Claude, Codex, and Grok Build projections
  retain their own authentication, model, base-URL, and MCP rules; they are not
  normalized through one generic JSON rewrite.
- Before a live mutation, inspect the current file/state and retain the
  authoritative backup or prove the existing backup still applies. Write
  through the application-specific owner, reread/validate the result, and only
  then publish active takeover state.
- A switch lock serializes conflicting transitions for the same application.
  A concurrent writer must wait or fail with a conflict; it must not race the
  backup, Provider selection, live write, or active-target update.
- If a live write, later switch step, or readback fails, run the existing
  compensation/restore path and report whether recovery completed. Never claim
  activation from a Provider database row when the live Agent state is unknown.
- Takeover matching helpers in `services/proxy/takeover.rs` are pure URL/config
  recognition. Stateful reads, writes, locks, backup ownership, and transition
  order stay in `ProxyService`.

### Provider switching and crash recovery

- Provider switching admits only a closed application ID and an existing
  Provider eligible for that application. The service resolves secrets and
  application-specific projection at the final native boundary.
- While takeover is active, a Provider change is one transaction across the
  selected Provider, live configuration, effective proxy target, readback, and
  compensation. Partial mutation is a failed or recovery-required outcome, not
  success.
- On startup/recovery, `recover_from_crash` reconciles persisted state, live
  takeover evidence, and backups through the existing application-specific
  rules. It does not delete an uncertain backup or overwrite an unrecognized
  live file merely to make state look consistent.
- Detection is evidence, not permission. A live file resembling the local
  proxy can report takeover evidence, but mutation still passes normal locks,
  ownership, validation, and backup rules.

### Breaker administration and diagnostics

- Breaker configuration and reset commands operate on the shared runtime owner
  used by the HTTP pipeline. Updating persisted thresholds must deliberately
  refresh the in-memory router; resetting one Provider/application key must not
  clear unrelated health.
- Health and stats are bounded observations. Reading them does not consume a
  half-open permit, probe an arbitrary URL, or mutate Provider selection.
- Errors and events may identify the application, Provider ID, transition
  stage, and stable reason code. They do not expose API keys, authorization
  headers, raw live configuration, complete backup contents, or secret-bearing
  upstream URLs.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| listener bind/start fails | Return failure and keep running/takeover state honest; do not publish a server target. |
| start is called while running | Return the reviewed already-running error; retain the one listener. |
| plain stop is called | Stop the server only; do not select restoration implicitly. |
| restore was requested but no authoritative backup applies | Return the bounded no-backup/recovery result; do not synthesize a default live file. |
| application or Provider ID is unsupported/ineligible | Reject before live-file, secret, or breaker mutation. |
| conflicting switch/takeover transition exists | Serialize or return conflict; never run a parallel backup/write transaction. |
| Provider persistence succeeds but live projection or readback fails | Compensate and report failure/recovery-required; do not claim active switch. |
| compensation also fails | Preserve recovery evidence/backups and return the incomplete recovery state. |
| crash recovery finds uncertain or unrecognized live state | Preserve it and surface bounded recovery evidence; do not overwrite destructively. |
| breaker config persistence succeeds but runtime refresh fails | Report the real incomplete update; do not claim the running router adopted it. |
| health/stats are requested | Return bounded observation without consuming permits or changing selection. |

## 5. Good / Base / Bad Cases

- Good: a Provider switch acquires the application lock, validates the selected
  Provider, retains the applicable backup, writes and rereads the Agent config,
  updates active runtime state, then releases the lock.
- Base: the server is stopped without restore; backups and live Agent files
  remain untouched because the caller selected only the listener operation.
- Base: startup detects possible takeover after an interrupted process and
  reports recoverable evidence without deleting an uncertain file.
- Bad: update the selected Provider row first and toast success before live
  readback, start a second listener for a new port, or restore from a renderer-
  supplied JSON/path.

## 6. Tests Required

- Command tests assert exact Tauri names/payload casing, closed IDs, thin
  delegation, ACL registration, and secret-negative serialization.
- Service lifecycle tests cover bind failure, duplicate start, status after
  successful bind, plain stop, explicit stop-with-restore, and repeated stop.
- Takeover tests cover each supported application projection, backup reuse and
  mismatch, lock conflicts, write failure, readback mismatch, compensation
  success/failure, and active-state publication only after verification.
- Provider-switch tests cover ineligible/missing Providers, same-Provider
  behavior, persisted/live/runtime ordering, concurrent switches, and rollback.
- Recovery tests cover clean shutdown, interrupted takeover, missing/uncertain
  backup, recognized local-proxy state, unrecognized user state, and idempotent
  rerun.
- Breaker administration tests cover application-scoped refresh/reset and prove
  health reads do not consume permits; HTTP attempt behavior remains covered by
  [Local Proxy Pipeline](./local-proxy-pipeline.md).
- Run `mise run rust:test` and `mise run check:contracts`; Renderer adapter
  changes also run their focused type/parser tests.

## 7. Wrong vs Correct

Wrong:

```text
command -> edit live file -> update Provider row -> report success
renderer -> { backupPath, liveConfig, apiKey } -> restore/switch
stop -> always restore every Agent
```

Correct:

```text
closed command -> ProxyService -> per-app lock
  -> validate Provider/backup -> app-specific write -> authoritative reread
  -> publish runtime state, or compensate and report recovery truthfully

plain stop and explicit stop-with-restore remain separate operations
```
