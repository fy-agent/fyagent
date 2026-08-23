# Issues #58/#59/#60 executor recovery evidence

Evidence date: 2026-08-24 (Asia/Shanghai)

## Source baseline and boundary

- Branch: `codex/issue-58-60-executor-recovery`
- Implementation commit: `52b61e894efdd55393f418f31218c105a13cf05c`.
- Replacement Draft PR: <https://github.com/fy-agent/fyagent/pull/134>.
- Stacked base: PR #130 head
  `a725b121642b82ad2eb19ae728f5772ced5b4a96`.
- Schema remains v20; no migration or second Provider writer was added.
- Scope is backend-only: one registered Codex Provider switch adapter, durable
  five-phase execution, event hints, polling snapshots, idempotency,
  pre-write cancellation, partial truth, and readback-only reconciliation.
- SecretRef, V2 UI, WorkBuddy, generic Undo, arbitrary scripts/commands, and
  network work during apply remain outside this slice.

## Fresh local evidence

| Gate | Result | Evidence level |
| --- | --- | --- |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib change_plan` | PASS: 28 passed, 2812 filtered | focused executor/legacy-regression contract |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --lib --tests -- -D warnings` | PASS: no issues | focused lint |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib commands::agent_catalog::tests::application_acl_covers_every_registered_command_without_remote_access` | PASS: 1 passed, 2839 filtered | Tauri registration/ACL closure |
| `mise run supported-platform:check` | PASS: 2022 current files | platform governance |
| `mise run check:contracts` | PASS | task/docs/lock/version/release contracts |
| `mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts tests/remainingPlatformSurface.test.ts` | PASS: 27 tests | architecture and platform structure |
| `mise run check` | PASS after the ACL snapshot correction below | complete current-host repository gate |
| `git diff --check` | PASS | patch hygiene |

The first full `mise run check` correctly failed one Rust lib test because the
new `cancel_change_job` Tauri registration had not yet been added to the local
application ACL or the intentional handler-count snapshot. The permission was
added exactly once, the expected registered-command count moved from 340 to
341, the exact failing test passed, and the complete gate then passed.

## Behavioral evidence

- The public descriptor is closed over one adapter/version/operation and the
  exact ordered phases `precheck -> snapshot -> managed_write -> readback ->
  finalize`; serialized descriptors contain no shell, script, command, argv,
  or dynamic settings payload.
- Eight concurrent apply attempts produce one admitted execution, seven
  idempotent replays, one job id, and exactly one Provider writer call.
- Cancellation that wins the atomic gate persists
  `cancelled_before_write`; a cancellation after `managed_write` is claimed is
  rejected as `commit_point_passed`.
- Every observer callback reads the same committed `eventSeq` back from SQLite;
  the full sequence is monotonic (`1..7`) and `get_change_job` returns the
  authoritative snapshot.
- Fault injection before the first write recovers as
  `interrupted_before_write` with writer zero. Fault injection after the side
  effect but before result recording reconciles by real readback as
  `recovered_target_reached`; repeated polling does not replay the writer.
- Restart/private-proof loss remains `recovery_required`; no recovery branch
  guesses secret equality or invokes the writer.
- Structured partial results distinguish succeeded, compensated, unverified,
  remaining-effect, and manual-action codes without exposing paths, Provider
  definitions, raw errors, or secrets.

## Not yet established

- Final-head GitHub Required CI, especially Windows Backend, is pending until
  the stacked replacement PR is pushed.
- #58 is not closed until the WorkBuddy second real adapter (#66) proves the
  seam reusable.
- #59/#60 still require the stacked V2/native-runtime and crash/restart evidence
  from the approved recovery plan.
- This evidence is partial mitigation and does not close Issues #58/#59/#60.
