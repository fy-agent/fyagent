# Design

## Architecture

```text
V2 Apply/Agents UI
  -> FeaturePorts
  -> strict Tauri adapters
  -> four Change Plan commands / one Install readiness command
  -> service contracts
  -> SQLite v20 local-only ledgers / existing Provider mutation writer
```

Change Plan owns orchestration. ProviderService remains the only configuration writer. Agent Install readiness is a pure projection over the canonical Agent Catalog and does not own an executor.

## Change Plan contract

- Contract version: `fyagent-change-plan/v1`, independent of DB schema.
- Plan TTL: 15 minutes.
- Apply input: only `planId + planDigest`.
- Native commands: `create_codex_provider_switch_plan`, `apply_change_plan`, `get_change_job`, `list_recoverable_change_jobs`.
- Frontend port methods use camelCase equivalents.
- Plan rows store separate `baseline_db_current_provider_id` and `baseline_device_current_provider_id`.
- Apply acquires the existing Provider mutation lock, reloads and validates the Plan, atomically consumes it and creates the Job/Event, calls the internal lock-held writer at most once, then performs DB/device/target/live readback.
- Reconcile only reruns readback and terminal-state convergence。它从不调用 writer。
- Mixed or unknown readback becomes `recovery_required`; success/warning never claims real usage.

## Secret boundary

Only a saved Codex Provider proven to require no new credential material may be planned. Failure to prove that property yields `secret_dependency_unavailable`. No raw setting, API key, live config, absolute path, secret digest, SecretRef or Keychain value may enter Plan/Job DTOs, database rows, errors or logs.

## Install readiness contract

- Canonical IDs: `qoderwork`, `trae-work`, `workbuddy`, `grokbuild`, `codex`, `claude-code`, `opencode`.
- One payload-bounded, read-only command: `get_agent_install_readiness(agentId)`.
- DTO contains only contract version, ID, states, reason codes, review/check timestamps and sanitized summaries.
- It excludes URL, path, hash, script, secret, package path and signer fingerprint.
- No plan snapshot is created; `plan.state` remains non-positive with `plan_not_created`.
- Unknown/unconfirmed stays non-green. Generic automation remains unavailable; Codex remains managed by the existing Codex Desktop installer.

## Renderer contract

- New DTO/parser owners live under `src/v2/shared/features/**` and `src/v2/shared/platform/tauri/feature-ports/**`.
- Browser adapters return native-only errors and never seed authoritative-looking data.
- Apply UI is route-local under `src/v2/pages/models/apply/**`; Agent readiness UI is a compact read-only section in the existing Agent detail.
- Existing V2 shared primitives and Agent directory order remain authoritative.
- Grok copy changes outside V2 must update all four locale files with parity tests.

## Schema and sync

- `SCHEMA_VERSION = 20`.
- One idempotent table/index helper is called both by initial database creation and v19→v20 migration.
- Migration ordering remains `<=18 -> 19 -> 20`; versions >20 fail closed.
- Change Plan tables are in both sync skip and local-preserve sets.

## Failure policy

- Invalid/expired/consumed/stale/digest/secret failures: writer=0.
- Concurrent apply: at most one caller consumes the Plan.
- Writer failure with confirmed original baseline: failed/restored result.
- Writer failure or readback ambiguity without authoritative restoration: `recovery_required`.
- Every positive UI state is derived from a validated terminal snapshot, never from an optimistic click result.

## Governance

Workers own disjoint files and do not use Git. A later integration worker owns shared registration, schema entrypoints, bridge composition and cross-layer tests. Root owns all Git/GitHub operations. External reviewers are read-only and do not substitute for runtime or CI evidence.
