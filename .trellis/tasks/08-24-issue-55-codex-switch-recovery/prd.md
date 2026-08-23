# Recover Unified Change Plan Codex switch

## Goal

Rebuild the proven Codex provider-switch vertical slice from current main as a narrow, reviewable replacement for PR #114; preserve target-side-effect-free create, single apply, stale-plan rejection, readback reconciliation, and cross-platform behavior.

## Requirements

- Rebuild the Codex existing-Provider switch backend from `origin/main@e94307cd`
  instead of rebasing the 71-commit-stale PR #114.
- Creating a plan may insert one immutable UCP control-plane row, but performs
  no Provider/business-state write, live/external-target write, network
  request, job/event creation, or secret-bearing persistence.
- Persist immutable plan identity, bounded non-sensitive presentation fields,
  per-plan non-secret approval bindings, expiry, one-time admission state,
  durable job snapshots, and monotonic events. Full Provider/live proofs remain
  process-private and memory-only.
- The public plan carries `actor.type=direct_user`, Cargo `sourceVersion`, and
  `revision=1`; the approval binding covers these identity fields and expiry.
- Applying `planId + planDigest` is the only admission boundary. Expired,
  mismatched, stale, consumed, or replayed plans must call the Provider writer
  zero times.
- A valid apply reuses the existing `ProviderService::switch` owner exactly
  once while holding the existing Provider mutation guard across baseline,
  admission, writer, and independent database/current-setting/
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
- [ ] Repeated create requests have unique plan/proof IDs and approval bindings;
      secret-bearing equality cannot be derived across plans.
- [ ] Plan creation proves exactly one plan-ledger insert, zero Provider/live/
      job/event writes, and zero network access; serialized payloads and UCP
      rows contain no raw settings, secret, private proof, or absolute path.
- [ ] Digest mismatch, expiry, stale current/target/live baseline, and replay
      are rejected before the Provider writer and create no second job.
- [ ] API-Key-only drift and process-private-proof loss reject an unapplied
      plan as stale with zero writer calls; a nonterminal job without its proof
      becomes recovery-required and is never replayed.
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
