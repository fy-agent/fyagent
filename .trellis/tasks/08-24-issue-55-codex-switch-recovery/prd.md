# Recover Unified Change Plan Codex switch

## Goal

Rebuild the proven Codex provider-switch vertical slice from current main as a narrow, reviewable replacement for PR #114; preserve zero-write create, single apply, stale-plan rejection, readback reconciliation, and cross-platform behavior.

## Requirements

- Rebuild the Codex existing-Provider switch backend from `origin/main@e94307cd`
  instead of rebasing the 71-commit-stale PR #114.
- Creating a plan is side-effect free: no Provider write, live-file write,
  external request, job creation, or secret-bearing persistence.
- Persist immutable plan identity, bounded non-sensitive presentation fields,
  semantic baseline digests, expiry, one-time admission state, durable job
  snapshots, and monotonic events.
- Applying `planId + planDigest` is the only admission boundary. Expired,
  mismatched, stale, consumed, or replayed plans must call the Provider writer
  zero times.
- A valid apply reuses the existing `ProviderService::switch` owner exactly
  once, then derives its result from independent database/current-setting/
  target-definition/Codex-live readback.
- Interrupted or uncertain jobs are reconciled by fresh readback and are never
  automatically replayed.
- Add the tables through an explicit schema-v20 migration and the new-database
  creation path; do not retain the old branch's schema-v16 exception.
- Register typed Tauri commands for plan creation, apply, job lookup, and
  recoverable-job reconciliation without exposing raw config, Provider
  settings, absolute paths, unrestricted backend errors, or secret values.
- Preserve every non-Codex and non-switch Provider path unchanged.

## Out of Scope

- Full #35 SecretBackend/keyring lifecycle and secret CRUD.
- #41 production Apply workspace and V2 Change Plan UI.
- Provider create/edit/delete/import, WorkBuddy, Prompt/Memory, Workspace Pack,
  network probes, and real Agent usage evidence.
- Merging `main`; this task produces a reviewable replacement PR only.

## Acceptance Criteria

- [ ] The cumulative branch diff is based on current `main` and contains no
      obsolete V1 renderer or historical task payload from PR #114.
- [ ] Schema v19 upgrades transactionally to v20 with all three Change Plan
      tables/indexes; fresh databases start at the same shape.
- [ ] Repeated create requests have unique plan IDs but a stable semantic plan
      digest on an unchanged baseline.
- [ ] Plan creation proves zero writes/network access and serialized plan/job
      payloads contain no raw settings, config, secret, or absolute path.
- [ ] Digest mismatch, expiry, stale current/target/live baseline, and replay
      are rejected before the Provider writer and create no second job.
- [ ] A valid apply invokes the existing Codex Provider switch once and only
      reports success when all required readback authorities match.
- [ ] Writer failure, baseline restored, target reached, mixed/unknown state,
      and interrupted-job reconciliation are distinct and tested; reconcile
      performs no write replay.
- [ ] The path-like display-name sanitizer behaves identically on macOS,
      Linux, and Windows path separators.
- [ ] Focused Rust tests, schema migration tests, format, clippy, full Rust
      tests, repository contracts, `git diff --check`, and Trellis validation
      pass from the final commit.
- [ ] The branch is pushed and a replacement PR referencing #55 and superseding
      #114 is opened; #114 is closed only after the replacement is reviewable.

## Notes

- User explicitly reprioritized this mainline above the install/distribution
  chain on 2026-08-24 and authorized cross-thread collaboration and PR repair.
