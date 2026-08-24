# Implementation Plan

## 1. Baseline and Conflict Ledger

- Fetch exact latest `origin/main` and PR #135 head; record SHAs/ahead-behind.
- Capture merge-tree conflicts and overlapping files since the old #135 base.
- Classify conflicts as mechanical metadata, SPEC contract, or product semantics.

## 2. Integrate Latest Main

- Bring latest main into the PR branch without force-rewriting public history by default.
- Resolve #140 overlaps by preserving current targeted config-write and backup contracts.
- Normalize overlapping tests rather than accepting an auto-merge solely because it compiles.

## 3. Persistence Safety Review

- Verify canonical schema v20 and migration helper.
- Verify local-only sync skip/preserve behavior with focused tests.
- Search for competing v20 Change Plan schemas/DAO owners and reject reintroduction.
- Verify Provider writer call-count invariants and current main writer ownership.

## 4. Transaction and Recovery Review

- Verify zero-side-effect preview, TTL/digest/baseline stale admission and concurrency.
- Exercise writer failure, restored baseline, ambiguous readback and interrupted recovery.
- Prove reconcile/get/list recovery never replay writer.

## 5. IPC and V2 Review

- Verify strict DTO/ACL and native-only browser ports.
- Verify one confirmation, real snapshot-driven progress, failure-not-green and
  `usageEvidence=not_observed`.
- Re-run #140 Models/Codex/WorkBuddy/OpenCode focused regressions.

## 6. Two Review Passes

- Architecture/security review: schema, sync, single writer, recovery, secret boundary, IPC.
- Product/regression review: Apply UI, Models behavior, #140 compatibility, Agent readiness/Grok
  scope. Classify findings as BLOCKER / MUST FIX / FOLLOW-UP / ACCEPTED.

## 7. SPEC Normalization

- Update backend/frontend SPEC only after executable behavior stabilizes.
- Cross-check every durable contract against code/tests; remove stale #135 text and retain #140
  current behavior.
- Run repository contract/SPEC drift checks and supported-platform manifest validation.

## 8. Trellis Closeout

- Validate #135 original task tree; archive completed child tasks.
- Resolve the original consolidation parent from `review` to truthful completion and archive it.
- Record this task's check evidence, then archive this canonicalization task before merge.
- Re-check that no task belonging to this integration remains completed/review under active
  `.trellis/tasks/`.

## 9. Full Validation and Delivery

- Focused V2 + Rust Change Plan/Provider tests during iteration.
- Final: V2 typecheck/lint/unit/browser; Rust fmt/check/clippy/test; Repository Contracts;
  supported-platform; `mise run check`; direct-session prearchive checks where required.
- Commit conventional changes, push only the PR branch, wait for exact-head GitHub Required CI.
- Merge only through PR #135 (or documented replacement if GitHub branch mechanics require it).
- Verify post-merge `origin/main` SHA and main push `CI / Required` success.

## Stop / Rollback Conditions

- Stop merge if schema semantics are ambiguous, a second writer appears, recovery can replay a
  write, #140 targeted-patch behavior regresses, SPEC does not match executable contracts, any
  owned Trellis task remains improperly active, or Required CI is not green at the exact head.
- Do not weaken contracts/tests simply to make the integration pass.
