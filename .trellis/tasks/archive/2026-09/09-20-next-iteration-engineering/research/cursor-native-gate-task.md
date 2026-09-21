# Cursor native gate follow-up

Continue in the same root integration worktree and branch, HEAD 35848bef with
the existing task-owned changes. Prior main-integration result is source-reviewed
and accepted for combined checks. Keep those test changes. No other root native
writer is active. Antigravity has separate CLI and UI worktrees.

The full root backend gate stopped in Clippy. Read
`artifacts/integration-backend-main.log`: `admit_usage_test` in
`services/provider/credentials.rs` has 9 arguments and test helper `run_test_with`
in `services/provider/usage.rs` has 8. Group the already-related usage-test input
into a small typed parameter structure or reuse the existing input owner; keep
behavior and all target-binding/security assertions. Avoid broad lint suppression.

Own the two usage/credentials files, their focused tests, and this result file.
Run canonical `mise run check:backend` after the narrow repair, with the verified
root native CARGO_TARGET_DIR. Root transfers the root-tree native check window
to you for this run. All terminal commands start with rtk. Save the full log to
`artifacts/cursor-native-gate.log`. This checks fmt, check, clippy and all native
tests and exposes remaining integration failures. Do not run frontend/browser.

If more failures are only stale public-Codex plaintext assertions, apply the
same redaction plus native-resolver pattern in the exact failing test files,
preserving the original contract. This grants those failing test files only.
Any production failure outside usage/credentials: report precise evidence and
stop before expanding writes; root will assign it. Do not alter unrelated config,
installer, helper, proxy or model behavior to get tests green.

Remove temporary Debug instrumentation before formal checks. Record actual final
checks, counts, paths and failures in `research/cursor-native-gate-result.md`.
No commit, push, Issue closure or real credential/service access. Stop writer
after delivery; execution completed and final root acceptance remain separate.
