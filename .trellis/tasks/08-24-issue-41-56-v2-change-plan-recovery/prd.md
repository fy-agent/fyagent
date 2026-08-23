# Implement Issues 41 and 56 V2 change plan UI recovery

## Goal

Stack a real Codex Provider switch plan preview, one confirmation, backend-driven phase snapshots, cancellation, polling, and honest outcome UI on PR #134 without restoring the old fake coordinator.

## Requirements

- Add one V2-owned, strictly parsed Change Plan contract and one bounded Tauri
  feature port. Browser preview must fail as native-only and never seed a fake
  successful plan or job.
- In the Codex Models detail, list only sanitized existing Providers from the
  current Provider summary. Current Provider is identified but cannot generate
  a switch plan; a different Provider can generate a side-effect-free plan.
- Present one immutable preview with four readable sections: semantic change,
  risk/restart, preconditions/read-write scope, and recovery mode. Display no
  digest, secret, full configuration, raw diff, absolute path, or free-form
  backend error.
- Require exactly one explicit confirmation of the concrete `planId` and
  `planDigest`. Plan creation performs no target write; apply is never invoked
  before confirmation.
- Subscribe to `change-job://updated` before apply. Treat event payloads as
  hints only, fetch authoritative job snapshots with `get_change_job`, ignore
  stale event sequences, and poll while a known job is nonterminal.
- Render the exact five backend phases and real backend statuses. Do not use
  timers to synthesize progress or claim a backup/refresh action that the
  adapter did not report.
- Allow cancellation through the backend only while the execution is still at
  its pre-write safe point. Preserve the backend outcome when the commit point
  has passed.
- Express success, warning, failure, cancelled, partial/recovery truth,
  restart requirement, and the fixed `not_observed` usage-evidence boundary.
- Keep quick-setup create/edit and #35 SecretRef integration outside this
  stacked UI slice. Do not import legacy `App.tsx`, hooks, fake fixtures, or
  call connectivity/model endpoints from the apply flow.

## Acceptance Criteria

- [x] Strict wire parsers reject unknown/excess/malformed plan, outcome, job,
      step, resource, event, cancel, and partial-result shapes.
- [x] A non-current existing Codex Provider produces one real preview; current
      Provider and browser-only state never produce a write.
- [x] The preview derives all visible facts from the safe plan DTO and requires
      one confirmation before `apply_change_plan`.
- [x] Backend event hints trigger authoritative `get_change_job` reads; polling
      continues from the last known job and stale `eventSeq` values cannot
      regress the UI.
- [x] The five real phases and all terminal statuses render without fake
      progress, secret/config/digest/path exposure, or proactive network calls.
- [x] Pre-write cancellation, commit-point rejection, partial/manual recovery,
      restart, and `not_observed` copy have focused component coverage.
- [x] V2 lint/typecheck/tests/browser/build and the full repository gate pass.
- [x] A native Tauri run demonstrates real plan creation, one confirmation,
      phase events/polling, write/readback, and terminal UI on an isolated
      reversible fixture before Issues #41/#56 can close.

## Governance Boundary

This UI can close the switch-only presentation slice only after final-head CI
and native evidence. It does not close #63 create/edit, #35 Windows HIL,
#58 second-adapter reuse, or #60 crash/restart evidence.
