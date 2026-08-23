# Unified Change Plan Codex Switch Recovery — Design

## Recovery strategy

PR #114 contains a proven Codex-switch state machine, but its branch is 71
commits behind `main`, mixes 17,000+ lines of historical design and obsolete V1
renderer work, and fails the Windows test job. Rebuild from current `main` and
transplant only the backend contract, persistence, orchestration, commands, and
focused tests. The replacement is intentionally backend-first so the later V2
UI can consume one stable IPC contract without reviving `src/App.tsx`.

## Data flow

```text
create command
  -> ChangePlanService::create_codex_provider_switch_plan
  -> inspect current Provider + target Provider + Codex live projection
  -> canonical semantic digests
  -> immutable change_plans row
  -> redacted public plan DTO

apply command(plan_id, plan_digest)
  -> process-local apply lock
  -> fresh baseline inspection
  -> transactional one-time admission + change_jobs row
  -> existing ProviderService::switch exactly once
  -> independent fresh readback
  -> bounded resource/result classification
  -> durable snapshot + monotonic change_job_events

startup/query recovery
  -> load nonterminal jobs
  -> fresh readback classification
  -> persist terminal/warning truth
  -> never replay the writer
```

## Persistence

Schema v20 owns `change_plans`, `change_jobs`, and `change_job_events` plus the
lookup indexes used by admission and recovery. The same helper creates the
tables for fresh databases and for the v19-to-v20 migration. Admission is a
single SQLite transaction: validate status/digest/expiry/baseline, consume the
plan, and create one job. Rejected admission leaves no job.

Persisted JSON is limited to closed enums, stable codes, bounded presentation
fields, and domain-separated digests. Provider settings, raw live config,
filesystem paths, secrets, and unrestricted error strings are excluded.

## Ownership and compatibility

- `change_plan.rs` owns domain types, inspection, digesting, apply sequencing,
  readback classification, and reconciliation.
- `database/dao/change_plan.rs` owns transactional persistence.
- `commands/change_plan.rs` is a thin Tauri transport facade.
- `ProviderService::switch` remains the only Codex switch writer.
- Existing Provider commands and non-Codex paths are unchanged.
- No production renderer is added in this PR; #41/V2 integration follows on
  top of the stable backend contract.

## Platform correction

The old safe-name helper used the host OS path parser. A Windows-looking path
therefore leaked unchanged on Unix and a Unix-looking path leaked on Windows.
The recovered helper treats both `/` and `\\` as untrusted separators and
returns only the last non-empty segment, with a stable fallback.

## Failure truth

- Before-write drift/replay/expiry/digest mismatch: typed rejection, zero writer
  calls, no replacement job.
- Writer error or failed post-write readback: inspect actual state before
  classification.
- Baseline restored: failed-and-recovered.
- Target reached despite writer error: warning, not a false failure.
- Mixed/third/unavailable state: recovery required.
- Interrupted job: same readback classifier; no automatic apply retry.

## Verification boundary

Local macOS checks prove current-host compile/test and cross-platform pure
logic. GitHub Windows/Linux/macOS jobs are required before the PR is called
reviewable. Neither local checks nor CI prove Windows/macOS keyring HIL, real
Provider use, signed release, or production acceptance.
