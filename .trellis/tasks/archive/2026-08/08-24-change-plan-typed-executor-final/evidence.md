# Canonical typed executor evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Baseline and source decision

- Canonical base: `main@bda0ffe74901dee53bacefb73a93484d428c44c3`
  after SecretRef replacement PR #145 merged through Merge Queue.
- Draft PR #134 was inspected as a source of executor requirements only. Its
  old #130 process-epoch/HMAC/private-proof stack and incompatible schema-v20
  definition were not imported.
- Replacement implementation commit:
  `faac9b93e13d87d701b82ee9abc576dbc6a764d8`.
- Final-main synchronization commit:
  `60bd8548eba5da223b27675f3849f734142968e8`.
- Test-fixture isolation blocker fix:
  `4186fb925840b5beefc84353d345680f14f35ae8`.
- Final adapter-contract metadata correction:
  `35ea4634` (`adapterVersion=1`; wire version remains v2).
- Integration-fixture isolation commit:
  `4186fb92` (`test(rust): isolate integration fixture homes`).

## Compatibility findings fixed during salvage

1. Canonical schema-v20 constrains `change_jobs.status` to
   `planned/running/succeeded/warning/failed`. Draft #134 attempted to persist
   `cancelled` without a migration. The replacement keeps the v20 row legal as
   `failed + result_code=cancelled_before_write` and derives public
   `status=cancelled` on read.
2. Draft #134 depended on the superseded process-epoch/HMAC/private-proof
   design. The replacement instead binds the closed adapter descriptor into
   the current canonical plan digest/contract and preserves #135 baseline,
   secret-capability, targeted projection, single-writer, and readback rules.
3. Existing v1 job rows may contain `apply/reconcile` phases. The replacement
   retains them only as decode variants and normalizes reads to the five-phase
   v2 projection without rewriting stored JSON/event rows.
4. Wire and adapter versions are independent axes. The wire contract advances
   to `fyagent-change-plan/v2`, while the first registered Codex adapter is
   `adapterVersion=1`; a wire revision is not treated as an adapter
   implementation revision.
5. Draft error classes could imply retryability that the existing Provider
   writer does not expose. The final contract uses factual `writer_failed`,
   `verify_failed`, and `unknown_outcome` categories and does not invent
   `transient`/`permanent` from a collapsed writer error.

## Focused verification

- `mise run rust:fmt:check` — PASS.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml services::change_plan -- --nocapture`
  — PASS: 24/24 focused Change Plan tests, 0 failures.
- `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
  — PASS.
- Focused V2 Change Plan/ACL tests — PASS: 6 files / 47 tests.
- `mise run typecheck:v2` — PASS.
- `mise run lint:v2` — PASS.

Focused coverage explicitly proves:

- adapter descriptor is bound to the plan contract;
- sequential/concurrent same-plan replay calls the writer no additional times;
- pre-write cancellation wins only before managed-write claim and persists a
  v20-compatible terminal row;
- observer hints resolve only after the corresponding snapshot/event commits;
- both fault points recover by readback without replaying the writer;
- partial projection distinguishes compensated and unverified effects;
- legacy v1 `apply/reconcile` rows normalize without migration;
- existing stale/TTL/secret/single-writer/Quick-Setup/takeover projection
  contracts remain green.

## Validation hardening discovered during full gate

The first aggregate Rust runs exposed intermittent `disk I/O error`,
`readonly database`, and `Directory not empty` failures across otherwise
unrelated integration binaries. Each affected target passed independently, and
one standalone canonical `mise run rust:test` also passed. Investigation found
that `src-tauri/tests/support.rs` gave every integration-test process and every
successive `cargo test` invocation the same global temporary directory
`fyagent-test-home`; suite-local mutexes cannot serialize distinct processes.

The test fixture now includes `std::process::id()` in that temporary home. This
does not change product behavior or reduce test concurrency: tests inside one
integration binary still share their existing mutex/home, while independent
test processes cannot delete or rewrite one another's SQLite/config fixture.
The reviewed supported-platform structure digest was updated with the exact
final helper bytes.

- Two consecutive canonical `mise run rust:test` executions in one `&&`
  command chain — PASS/PASS.
- Subsequent full `mise run check` — PASS, including the previously affected
  `import_export_sync`, `mcp_commands`, `provider_commands`, and
  `provider_service` targets under the default runner/concurrency.

## Full local verification

- `mise run test:v2:browser` — PASS: 120/120.
- `mise run test:v2` — PASS: 44 files / 317 tests.
- `mise run check` — PASS:
  - frontend: 171 files, 1491 passed / 1 skipped;
  - Rust main library: 2853 passed / 5 ignored;
  - Rust integration/helper suites: PASS;
  - SecretRef contract: 7 passed / 1 matching-host HIL ignored locally;
  - task/docs/platform/release contracts: PASS;
  - native-fetch: 4/4 PASS.
- `supported-platform` final implementation shape: 2135 current files.
- `git diff --check origin/main..HEAD` — PASS before Trellis closeout.

The final aggregate run above was executed after the adapter-version and
factual error-class corrections, so earlier verification is not being used to
grandfather the final metadata state.

## Validation-infrastructure blocker resolved

- Two prearchive attempts exposed a real cross-process integration-test race:
  separate Cargo integration-test binaries shared one fixed
  `fyagent-test-home`, while the existing `RecoveringTestMutex` could only
  serialize threads inside one process. During the aggregate gate another
  binary could reset that shared fixture and create nondeterministic repository
  validation timing.
- `4186fb925840b5beefc84353d345680f14f35ae8 test(rust): isolate integration
  fixture homes` isolates the shared test HOME by process ID and refreshes only
  its supported-platform digest. This is test infrastructure only; no runtime
  FyAgent behavior or Change Plan contract changes.
- After that commit, `mise run supported-platform:check` passes against the
  stable final source tree. The full prearchive gate is rerun from scratch; no
  earlier failed/aborted attempt is counted as acceptance evidence.

## Final metadata-correction verification

- `cargo test --locked --manifest-path src-tauri/Cargo.toml services::change_plan -- --nocapture`
  — PASS: 24/24 after the adapter-version correction.
- focused V2 Change Plan tests — PASS: 4 files / 28 tests.
- `mise run typecheck:v2` — PASS.
- The final full direct-session prearchive was rerun after this correction and
  passed; earlier full runs remain supporting process evidence only.

## Remaining merge-ready evidence

- replacement PR #146 exact-head `CI / Required` and Merge Queue `merge_group` must
  pass before merge;
- old Draft #134 is closed only after the replacement PR exists.

## Replacement PR binding

- Replacement PR: https://github.com/fy-agent/fyagent/pull/146
- Base: `main`
- Head: `dev/change-plan-typed-executor-final`
- The old Draft #134 is now eligible to be closed as superseded after this
  binding is committed and pushed.

## Final direct-session prearchive

- `TRELLIS_CONTEXT_ID=chatgpt-change-plan-executor mise run check:prearchive --exclude-active-task .trellis/tasks/08-24-change-plan-typed-executor-final`
  — PASS, exit 0 on the final code/spec/workspace-handoff state.
- The passing run included frontend 1491 passed / 1 skipped, Rust 2853 passed /
  5 ignored, supported-platform 2135 current files, release contracts, task/docs
  contracts, and native-fetch 4/4.
- This final prearchive supersedes all earlier aborted or pre-correction runs.

## Final archive verification

- Task status is `completed` with `completedAt=2026-08-24` and is stored at
  `.trellis/tasks/archive/2026-08/08-24-change-plan-typed-executor-final`.
- `task.py list` reports zero active tasks after archival.
- `mise run check:contracts` on the staged canonical archive shape — PASS,
  exit 0.
- The post-archive run reported supported-platform 2142 current files and all
  task/docs/release/native-fetch contracts green.
