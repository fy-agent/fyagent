# Final integration review

## Baseline

- Canonical candidate: PR #135 / `codex/pr-108-115-consolidation`.
- Original PR head: `d826bbf50a51efeb18629c474d44c74f31c0512d`.
- Latest-main integration baseline: `67f50b8ffdf4105b1e478f87fe60eca0af7dc9c2` (includes PR #140).
- Integration strategy: merge current `origin/main` into the public PR branch; do not force-rewrite public review history.

## Review A — architecture / persistence / security

### PASS

- Exactly one active Schema v20 Change Plan definition remains. Active source contains no
  #130/#134 `proof_id` or `process_epoch_id` schema.
- `change_plans`, `change_jobs`, `change_job_events` are both sync-skipped and locally preserved;
  the executable sync regression passes.
- `ProviderService` remains the only live configuration mutation owner. Change Plan injects the
  lock-held Provider writer once after atomic admission; reject paths call it zero times.
- Recovery APIs own readback/convergence only. They do not receive a writer closure and do not
  replay Provider writes.
- Plan/apply wire input remains narrow. Apply transports only `planId + planDigest`; renderer
  validation rejects widened objects and unsafe opaque IDs/digests.
- Change Plan credential-neutral projection and persistence tests exclude secret-bearing auth,
  headers, query values and local paths.

### BLOCKER FOUND AND FIXED

**Fixed Quick Setup projection parity after PR #140.** PR #140 changed the fixed Codex Quick
Setup writer from whole-snapshot replacement to a targeted patch over the existing live TOML.
PR #135 still calculated normal-mode Change Plan target projection from the stored Provider
snapshot. A successful switch that correctly preserved user comments, review model, custom
provider fields, MCP or feature tables could therefore be misclassified as readback drift and
`recovery_required`.

Resolution:

- Added one pure `build_codex_quick_setup_live_projection` owner.
- The real fixed Quick Setup writer consumes that projection.
- Change Plan target projection consumes the same function for the reserved Codex Quick Setup ID.
- Added a Change Plan regression with unrelated comment/review-model/provider/MCP/feature content;
  Apply must finish `succeeded` while preserving all unowned content.
- Preserved config-only official-auth behavior: untouched `auth.json` is not parsed merely to
  write `config.toml`; an opaque/non-JSON auth file remains byte-identical in preservation mode.

### FOLLOW-UP (non-blocking)

- Existing #135 review noted that repeated read-only observation of a job that remains
  `recovery_required` can append equivalent recovery events on repeated calls. Current UI does
  not high-frequency poll this state and the behavior cannot replay a writer. Keep as a later
  bounded-ledger/deduplication improvement rather than changing recovery semantics in this
  canonicalization task.

## Review B — product / V2 / current-main regression

### PASS

- PR #140 Codex targeted patch and exact rolling-backup behavior remains active after integration.
- Official-auth preservation still exposes config-only write targets and leaves auth bytes alone.
- WorkBuddy/OpenCode backup/path/revision contracts from current main remain covered by V2 and
  browser tests.
- V2 Apply has no fake/scenario/timer progress state. It consumes backend plan/job snapshots,
  uses one explicit confirmation, renders recovery/readback mismatch non-green, and keeps
  `usageEvidence=not_observed` without claiming real model use.
- Agent install readiness remains read-only; no generic installer/executor action was introduced.
- Grok wording/i18n scope remains independent from Change Plan mutation ownership.

## SPEC normalization

- The V2 Models spec merge conflict was resolved by retaining **both** current-main #140 contracts
  (write targets, backups, revision dirty state, stale-probe reset, model-probe protocol, shared
  SecretInput geometry) and #135 Change Plan/Agent-readiness contracts.
- Backend Codex Provider spec now states that fixed Quick Setup Change Plan preview and writer must
  share the same targeted final-state projection; preserved user TOML must not look like drift.
- Supported-platform structure hashes were recomputed from the final source, not selected from a
  merge side.

## Verification evidence

- `mise run rust:test -- change_plan`: 22/22 before the parity fix; the final full Rust suite covers
  the added regression and passes.
- Fixed Quick Setup focused suite: 21/21 before the final config-only regression; the new isolated
  config-only preservation test passes independently.
- `mise run test:v2`: 44 files / 315 tests passed.
- `mise run typecheck:v2`: passed.
- `mise run lint:v2`: passed.
- `mise run test:v2:browser`: 120/120 passed at all maintained viewports.
- `mise run rust:fmt:check`, `mise run rust:check`, `mise run rust:clippy`: passed.
- `mise run supported-platform:check`: passed, 2048 current files.
- `mise run check:contracts`: passed; release contract 510 passed / 1 skipped; native fetch 4/4.
- `mise run check`: passed; frontend unit 1489 passed / 1 skipped, Rust library 2847 passed /
  5 ignored plus all integration/doc/helper suites, repository/release/platform/desktop gates.

## Verdict

**PASS WITH FIX APPLIED.** No unresolved P0/P1/blocker remains in the canonical #135 integration.
The recovery-event deduplication item is a non-writing P2 follow-up. Merge remains blocked until
the carried Trellis task tree and this task are correctly archived, latest-main drift is rechecked,
the exact pushed PR head has green Required CI, and merge occurs through GitHub PR only.

## Trellis closeout

- The four completed consolidation child tasks and the historical parent review task were validated
  and archived under `.trellis/tasks/archive/2026-08/` before this task's own prearchive gate.
- The first prearchive attempt correctly rejected non-standard nested `evidence/` / `reviews/`
  archive payloads. Those historical Markdown receipts were flattened into the canonical archive
  root (with `research/` retained as the only allowed subdirectory), and the fresh direct-session
  `check:prearchive` then passed in full.
- This task completes at merge-ready/prearchive state so that its own archive is present in the PR
  head before GitHub merge. Exact-head Required CI and post-merge main CI are post-archive gates,
  not reasons to leave an active Trellis task in `main`.
