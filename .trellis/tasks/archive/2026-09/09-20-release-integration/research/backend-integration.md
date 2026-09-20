# Backend integration evidence

Scope: merge FDE `6f38c2136bfea551225a6c207400e88ad94683e5` into subscription
`6a4af16d15e8c29f3588eb77e478d4bc995d0705`, preserving both implementations.
Backend owner did not stage, commit, push, modify frontend, or touch real user data.

## Decisions and regression boundaries

- The source-owned schema advances to 23. Its forward migration completes both
  historical 22 shapes: FDE's project/generation/evidence tables plus four proxy
  targets, and subscription's five-target proxy constraint without FDE tables.
  Existing proxy columns (including `live_takeover_active`), settings, providers,
  recovery records, FDE rows and tombstone generations are preserved. Migration
  does not bump existing generations; binary restore intentionally does, using
  the FDE owner's existing evidence invalidation rule.
- Fixtures cover FDE-only 22, subscription-only 22, legacy 21, fresh initialization,
  repeat migration, a working resource generation trigger, a denied final version
  update with complete rollback, and both binary backup shapes. Binary restore
  leaves each source backup file unchanged. Existing generation-exhaustion restore
  coverage remains in the same test module.
- Managed Auth Health remains read-only (no reconciliation, vault access or native
  repair), while projecting all OpenAI/xAI account slots. Only owned empty-target
  slots are replaced in memory; arbitrary and nonempty-target slots remain. A
  stale target cannot hide another independently proven active route; absent such
  proof the result remains unknown.
- OpenCode preserves FDE exact `providerId` selection, builtin `editable` behavior,
  opaque provider fields and subscription `selectedModel` readback. Generic saves
  and writers reject managed projections. Startup import skips reserved/placeholder
  live entries and existing subscription rows, retaining canonical upstream
  settings, metadata and FDE resource generation. Bind → import → rebind → restore
  uses the existing subscription integration test with both account sources.
- FDE `saved_model_probe` rejects subscription providers before extracting API-key
  credentials or entering transport. Both sources × four targets record
  `Unsupported` / `saved_model_unavailable`, with zero listener connections and no
  marker/endpoint leakage. This checker has no new subscription transport; existing
  target-Agent subscription forwarding remains supported.
- OpenCode Health reads its independent `/opencode/v1` route and exact persisted
  binding. Correct loopback projection is in sync; wrong path, port, model or
  provider remains drift, and stopped listeners remain blocked. Observing preserves
  native bytes and database rows. The read-only DAO allows the fifth target.
- Combined native handler inventory is 407. Database and verification ownership
  stays with the existing modules; no new persistence owner or secret copy exists.

## Checks

Logs are local ignored artifacts under `.trellis/.runtime/verification/`.
All shell invocations use `rtk`; Rust commands use the canonical `mise` tasks.

| Command | Result | Log |
| --- | --- | --- |
| `mise run rust:fmt` | Passed | Tool exit 0 |
| `mise run rust:test -- release_integration` | 7 passed, 0 failed | `backend-release-integration-final.log` |
| `mise run rust:test -- subscription` | 51 passed, 0 failed, after the last observer change | `backend-release-subscription-verified.log` |
| `mise run rust:test -- health` | 43 passed, 0 failed | `backend-release-health-final.log` |
| `mise run rust:test -- config_reliability` | 7 passed, 0 failed | `backend-release-config-reliability.log` |
| `mise run rust:test -- database` | 117 library + 1 integration passed; 2 existing ignored | `backend-release-database.log` |
| `git diff --check -- src-tauri ...` | Passed | Tool exit 0 |

Initial integration runs exposed one test-expression compile error, the fresh
fixture's omitted migration call, and the Health DAO's missing OpenCode allowance;
all were fixed before the passing seven-test run. The first subscription run
exposed cached settings crossing temporary-home fixtures after the FDE observer
stopped repairing settings while reading; fixture lifecycle now reloads settings,
without restoring any mutation in production observation.

The first Health-filter run also corrected a new test assumption: an absent
OpenCode row returns `None` without being initialized by observation. Its final
fixture explicitly deletes both missing-target rows and asserts zero writes.

## Backend freeze

All owned backend edits are frozen after the checks above; no owner Cargo process
remains. The unstaged merge resolutions are deliberately left for the parent.
Beyond the automatically merged FDE files, the integration edits are:

- `commands/agent_catalog.rs`.
- `database/{mod.rs,schema.rs,fde_restore_tests.rs}` and
  `database/dao/{health.rs,projects.rs}` (the latter changes migration comments).
- `services/fde_workspace.rs`, `services/fde_workspace/tests.rs` and
  `services/health/{configuration.rs,proxy.rs,tests.rs}`.
- `services/managed_auth/{service.rs,proxy_overview_tests.rs,subscription_tests.rs,subscription_opencode_tests.rs}`.
- `services/opencode_models.rs`, `services/provider/{live.rs,mod.rs}` and
  `services/proxy.rs`.
- `.trellis/spec/backend/database-persistence.md`.

The registered structure assets among those owner edits include
`services/provider/mod.rs` and `services/managed_auth/subscription_tests.rs`;
the parent owns the complete merged-source inventory/seal reconciliation.

Final full prearchive, remote CI and native/real-account UAT are parent-owned and
are not implied by these local fixture checks.
