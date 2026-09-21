# Cursor Issue 64 recovery repair

Coordinator GPT-6; executor Cursor desktop, existing local account/model. This is a separate bounded backend work package from the active Cursor #35/#47 task. User explicitly requested Cursor backend implementation/testing, terminal Grok review, Gemini frontend ownership. Root retains acceptance and all Git integration.

Directory `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`, branch `codex/next-iteration-engineering-20260920`, starting HEAD `07519b37` plus existing task-owned changes. Work directly here. You are not alone: do not revert existing changes, touch the original `~/fyagent`, reset/stash/clean/switch branches, commit/push, create PRs or close Issues.

Read `research/grok-recovery-review.md`, relevant proxy/recovery and database specs. Root inspected the identified code and accepts the three findings for repair. `research/jev-recovery-decision.json` is advisory; root selected the narrow atomic database finish below, based on the actual crash window. No further user confirmation is needed for this authorized work.

## Required repairs

1. Legacy restore plan `Unchanged` correctly performs zero file writes, but its preview returns no target paths with canRestore=true. Keep no-op write semantics and disclose the exact closed native target list just as a real restore does. Preserve the renderer's rejection of empty successful previews. Add native tests for already-restored bytes while enabled=true, then actual disable success.
2. After owned files have been restored and verified, clear only this app's enabled flag and delete its live backup in **one narrow SQL transaction**, rather than deleting the backup before disabling or merely swapping calls. Do not rewrite other app parameters. Existing per-app lock remains. Test SQL failure after the first statement rolls both changes back (SQLite trigger or existing fixture seam), leaving enabled=true and the recovery backup present; the now-restored files still yield an admissible retry preview. Successful retry completes with disabled+backup removed. Keep the DAO SQL-only and native I/O outside its mutex.
3. The pre-MARKER managed proof currently replaces the logical backup before final verify_proof. Verify ownership before replacing it. Preserve the original logical backup when the last check fails. Prefer persisting the upgrade only at a point with checked live hashes/owned restore proof; do not add a new recovery framework. Include a meaningful external-edit regression and end-to-end legacy logical backup preview -> exit fixture, not just a helper test. Inspect the whole existing unwrap path for stale in-memory proof/backup handling.
4. Native conflict previews should retain the closed target path/exists metadata when it can be obtained without trusting the conflicting backup or granting restore authority. canRestore stays false. Renderer already tells users to protect these files and should be able to display them. No file contents, auth data, backup paths, or caller-supplied paths in preview.

## Write ownership

- `src-tauri/src/services/proxy.rs`
- `src-tauri/src/services/proxy/{legacy_recovery.rs,managed_recovery.rs,restore_preview.rs,recovery_tests.rs}`
- `src-tauri/src/database/dao/proxy.rs` and its narrowly scoped tests
- `.trellis/spec/backend/proxy-runtime.md` and narrowly relevant existing recovery spec if necessary
- This task's `research/cursor-recovery-result.md` and focused test logs.

All provider credential, usage and live_summary files are owned by the other Cursor task. All frontend, installer and tooling files are out of scope. Grok is read-only. If an issue crosses your boundary, return the precise finding rather than editing it. Root owns the parent Page recovery wiring regression separately.

Use canonical `rtk proxy mise run rust:test recovery_tests` and targeted DAO tests; inspect available task arguments. Another Rust package is being edited/tested, so if its unrelated compile failure blocks you, record it and do not alter its files. Avoid full builds, real user configurations, network credential tests, installations and TLS/security changes. Final root checks run only after all writers stop. No debug instrumentation or absolute workstation paths in final source.

Deliver exact paths, changes, test names/exit codes, residual findings and writer-stopped status in `research/cursor-recovery-result.md`; do not merely return a plan. Fixture tests are not native user acceptance or Windows evidence.
