# Unified Change Plan: Codex Provider Switch Contract

## 1. Scope / Trigger

Read this contract before changing the first Unified Change Plan vertical
slice: switching Codex to an existing Provider. It covers plan identity,
schema-v20 persistence, one-time admission, Provider mutation ownership,
readback/reconciliation, and the Rust/Tauri wire DTOs. It does not make the
generic adapter engine, V2 Apply UI, SecretBackend, or create/edit Provider
flows complete.

Plan creation may insert one immutable UCP control-plane row. It must perform
zero Provider/business-state writes, zero live/external-target writes, zero
job/event writes, and zero network requests.

## 2. Signatures

```text
create_codex_provider_switch_plan(targetProviderId: String)
  -> Result<ChangePlan, ChangePlanErrorCode>

apply_change_plan(planId: String, planDigest: String)
  -> Result<ApplyChangePlanOutcome, ChangePlanErrorCode>

get_change_job(jobId: String)
  -> Result<ChangeJobSnapshot, ChangePlanErrorCode>

list_recoverable_change_jobs()
  -> Result<Vec<ChangeJobSnapshot>, ChangePlanErrorCode>

event change-job://updated
  -> { jobId: String, eventSeq: i64 }
```

The existing `ProviderService::switch` remains the public writer. UCP holds
`ProviderService::lock_provider_mutation(..., Codex)` across baseline check,
SQLite admission, one call to `switch_with_lock_held`, and fresh readback.
No second Provider writer is allowed.

Schema v20 adds:

```text
change_plans(plan_id, operation, target_provider_id, target_provider_name,
  plan_digest, baseline_digest, actor_code, source_version, plan_revision,
  proof_id, process_epoch_id, current_provider_id, current_provider_code,
  target_provider_code, contract_digest, created_at, expires_at, status,
  consumed_at)
change_jobs(... one UNIQUE plan_id ...)
change_job_events(PRIMARY KEY(job_id, event_seq), ...)
```

## 3. Contracts

- `ChangePlan` uses camelCase and includes `planId`, `operation`, target ID and
  bounded display name, `planDigest`, `baselineDigest`,
  `actor.type=direct_user`, `sourceVersion`, `revision=1`, timestamps, closed
  status/codes, restart expectation, risks, and evidence note.
- `planDigest` and `baselineDigest` are per-plan opaque approval bindings over
  non-secret fields. They start with `mac1:` and are not stable content hashes.
- Full current/target Provider definitions and Codex live/target projections
  are bound only by process-private HMAC proofs keyed by random `proofId`.
  Private proof bytes never enter SQLite, IPC, logs, exports, events, or Debug.
- SQLite stores only the random `proofId`, random `processEpochId`, bounded
  non-sensitive metadata, and non-secret approval bindings. A process restart
  loses the private proof by design.
- `apply` accepts only the exact stored `planId + planDigest`, rechecks the
  non-secret binding and private proof under the Provider mutation lock, then
  atomically consumes the plan and creates one job before invoking the writer.
- Writer return is not success evidence. DB current, device current, target
  definition, and Codex live projection must pass fresh readback.
- Reconciliation reads and classifies only. It never calls the writer. If the
  private proof is unavailable after restart, the job is
  `recovery_required`; current IDs may still be displayed, but secret equality
  is never guessed.
- `usageEvidence=not_observed` is the only claim in this slice.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Target missing | `target_not_found`; no plan row |
| Target already effective current | `target_already_current`; no plan row |
| Unknown plan | rejected `plan_not_found`; writer zero |
| Wrong plan digest | rejected `invalid_digest`; writer zero |
| `now >= expiresAt` | rejected `expired`; writer zero |
| Consumed/replayed plan | rejected `consumed`; no second job; writer zero |
| IDs, target definition, common config, live projection, or API key drift | rejected `stale`; writer zero |
| Process epoch/private proof missing | unapplied plan `stale`; nonterminal job `recovery_required`; no replay |
| Writer error and baseline restored | failed `writer_failed_baseline_restored`, recovery succeeded |
| Writer error but complete target readback | warning `writer_error_target_reached` |
| Mixed state or definition drift after write | failed `post_write_mismatch`, recovery required |
| Readback unavailable | failed `readback_unavailable`, recovery required |

## 5. Good / Base / Bad Cases

- Good: two previews over unchanged secret-bearing state produce distinct
  plan IDs, proof IDs, and approval bindings; only two immutable plan rows are
  added and no target state changes.
- Good: API-Key-only drift between preview and apply is detected by the
  memory-only proof before admission, so writer calls remain zero.
- Base: a valid apply calls the existing Provider writer once, reports
  `not_observed`, and uses readback to decide success/restart truth.
- Bad: hash or HMAC full Provider/live projections and persist the result.
  Even keyed output is secret-derived durable state and conflicts with #35.
- Bad: release the Provider mutation guard between baseline validation and
  writer/readback, or call public `ProviderService::switch` while already
  holding that guard.

## 6. Tests Required

Focused Rust assertions must cover:

- closed camelCase DTOs and the shared fixture;
- v19-to-v20 migration plus identical fresh-database shape;
- exactly one plan-ledger insert and zero Provider/live/job/event changes on
  preview;
- no raw secret or private HMAC in persisted UCP fields;
- digest mismatch, expiry, stale ID/config/live/API-key state, proof loss, and
  replay all calling the writer zero times;
- existing writer exactly once, independent four-authority readback, writer
  failure classifications, terminal-race reload, and no-replay reconcile;
- process-proof loss yielding `stale` or `recovery_required` as appropriate;
- both Unix and Windows path separators in display-name sanitization.

Final gates:

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run test:unit -- tests/remainingPlatformSurface.test.ts
```

GitHub Windows/Linux/macOS required jobs must pass before calling the PR
reviewable. Local macOS evidence is not Windows runtime evidence.

## 7. Wrong vs Correct

Wrong:

```text
SHA256(full Provider/live JSON) -> change_plans.*_digest
check baseline -> release lock -> ProviderService::switch -> readback
restart + missing secret proof -> infer success from current provider ID
```

Correct:

```text
full secret-bearing projections -> memory-only per-plan HMAC proof
non-secret metadata + random proofId -> persisted approval binding
hold existing Provider mutation lock:
  baseline + private proof -> atomic admission -> existing writer once -> readback
restart + missing private proof -> stale/recovery_required, never replay
```
