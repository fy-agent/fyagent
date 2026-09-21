# Grok review: final credential gate repairs

Read-only terminal Grok review, existing grok-4.6 account/model, no subagents.
All commands start with rtk. Do not edit files, run builds/full suites, install,
commit, push, or inspect user credentials. Return concise findings to stdout;
GPT-6 persists the reviewed report. Stop after this bounded review.

Target: current integration tree
`~/.codex/worktrees/fyagent-next-night-20260920/fyagent`,
branch `codex/next-iteration-engineering-20260920`, HEAD a0a170b2 with the
completed Cursor credential gate-repair delta still dirty. Cursor writer stopped.
Another external writer operates only in a different installer worktree.

Inspect the exact uncommitted production changes in
`services/provider/live.rs`, `services/change_plan/{adapter,service}.rs` under
src-tauri/src, their adjacent helpers/contracts and focused tests only as needed.
The result is `research/cursor-credential-gate-repair-result.md` under this task.
Do not repeat a whole-repository audit or previously accepted other packages.

Review two concrete questions:

1. Does the new post-snapshot `persist_unowned_codex_common_tables` safely retain
   legal common configuration without overwriting unrelated user-owned settings,
   introducing a read/write race, partial commit, or treating an I/O failure as
   an empty file? Determine the existing transaction/ownership contract before
   concluding. Recommend the smallest correct placement if a bug is present.
2. Does persisted-row upsert verification plus the first-SecretRef digest
   comparison preserve preview target identity, stale/rotation rejection,
   compensation, and one-write semantics? Flag concrete mismatches with a
   falsifiable regression; do not demand identical opaque refs where first
   persistence necessarily assigns one.

Findings need severity, file/line, trigger, observed or source-proven result,
and a narrow repair/test. Separate confirmed defects from unanswered questions.
Existing 35 credential tests and six focused filters passed after instrumentation
removal; that does not settle untested paths. No test rerun is necessary here.
