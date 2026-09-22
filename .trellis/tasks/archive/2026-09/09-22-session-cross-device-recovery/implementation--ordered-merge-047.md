# Ordered integration for 0.4.7 — 2026-09-22

The user authorized merging #196 followed by #197 into `fy-agent/fyagent` main and setting the application version to 0.4.7. This follow-up integrates the customer-project retirement into Session recovery; it does not create a tag or publish installers. The previous Session implementation and review evidence remain historical; the final combined head and merge-group checks are the merge authority.

## Integration decisions

- Keep retirement migration25 and add Session receipts in migration26. Fresh installs have no retired tables; older databases retain retired rows and disable their triggers before proceeding.
- Preserve durable historical-project archiving before SQL/sync/binary replacement. Preserve the current Session receipt ledger under the final live-database lock for all three replacement paths. Restoring a database cannot roll back provider history, so old or foreign binary-backup receipts must not erase current claims or unresolved writes. This binary-restore and final-lock protection is a new integration correction, not a claim about the earlier #197 implementation.
- Keep the retired customer-project routes, ports, commands and permissions removed. Keep Session as a primary entry and Memory as auxiliary: nine registered routes, eight primary routes. Registered command set is396; renderer invokes141 (13 new migration commands plus three existing native calls account for16 additions against #196).
- Set workspace application and local Cargo package versions to0.4.7; synchronize the app's macOS helper requirement and helper bundle version. Preserve the independent minimum-client compatibility floor. CHANGELOG covers both PRs and real capability limits.
- Retain #196's natural keyboard-focus precondition and #197's committed-route transition checks; do not loosen motion, draft, cleanup or zero-write assertions.

## Merge governance and residual scope

Live GitHub rules require `CI / Required` and Merge Queue with merge commits. There are no unresolved review threads or mandatory approval-count requirements. Do not use admin bypass, force push, squash/rebase, direct main pushes or branch-rule changes. #196 must be merged before #197 is queued.

Dependabot28 remains an existing open glib0.18.5 upstream compatibility warning. Neither PR changes that dependency graph; the only lock changes in this integration are the two local package versions. Existing triage is in the archived security review; this task does not close or suppress the alert.

Windows real-device UAT was explicitly waived while unavailable; hosted Windows CI remains required. Elevated Session operations still fail closed where the ordinary-user CLI bridge is unavailable. Native readback, loopback request evidence and real continued model replies remain distinct.

## Validation

Frontend integration:13 targeted files,123 tests passed; typecheck and scoped formatting passed. Exact AST command sets were compared against #196. Database integration:152 passed,2 existing ignored in the focused database suite; fresh/24/25 migrations, rollback, SQL/sync/binary receipt preservation, archive failures and final publication locking have explicit cases.

Final canonical gates, Grok4.7 integration review and hosted checks are recorded by the coordinator after the combined source and archived task state are finalized. Raw logs stay outside the repository under the coordinator's `work/merge-047/`; GitHub PR checks and merge-group runs provide remote evidence.
