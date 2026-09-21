# Cursor managed-auth aggregate flake result

- Writer: Cursor desktop Debug mode, Cursor Grok 4.6 / High / Fast (as assigned).
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`
- Remote `origin`: `https://github.com/fy-agent/fyagent.git`
- Branch: `codex/next-iteration-engineering-20260920`
- Code baseline: `fac051ea`; root committed independent demo documentation while this task ran. Delivery includes this package's dirty edits. No reset/stash/branch/worktree/commit/push.
- Writer stopped.

## Cause

Aggregate-only fixture isolation, not a production guard bug.

`apply_connection_action` binds preview write + preserved paths and rejects a changed override with `TargetChanged`. OpenCode connect preserves `get_opencode_config_path()`, which follows live `FYAGENT_TEST_HOME` / `get_home_dir()`. The failing test used a private `config_dir` for `auth.json` but did not pin home and did not take `serial_test::serial`.

In the full lib run, a concurrent environment-modifying test (including the newly locked common-config fixture-home tests) can change `FYAGENT_TEST_HOME` between preview and apply. Isolated `integration-native-serial.log` never hit that window; `artifacts/integration-prearchive.log` did:

```
project: ManagedAuthErrorDto { contract_version: 1, reason_code: TargetChanged }
test result: FAILED. 3552 passed; 1 failed
```

`ExternalChangeDetected` / `OpencodeAuthError::Stale` are a different mapping. This failure is the preview path-CAS `stale()` path.

Reproduction (session `f6f5be`): flip `FYAGENT_TEST_HOME` after a successful preview. Apply returned `TargetChanged` with

- prepared preserved: `~/.config/opencode/opencode.json`
- current preserved: `<temp>/concurrent-home-mutation/.config/opencode/opencode.json`
- write `auth.json` unchanged

That is the intended “changed override cannot redirect a confirmed operation” check.

## Diff

`src-tauri/src/services/managed_auth/service.rs` only:

- `#[serial_test::serial]` on `opencode_connect_projects_independent_session_and_rejects_proxy_lineage`
- local `TestHome` guard pins `FYAGENT_TEST_HOME` to the fixture tempdir for the whole test and restores it on drop

No production snapshot/path-CAS change. `TargetChanged` is not remapped or removed. The suite is not globally serialized. Timeouts were not increased. Installer/helper/CLI were not touched.

After the pin, preview and apply both used the fixture home (`bound paths stable` / `match`). Replay of the consumed preview still fails closed (`preview missing`), which the existing test already expects.

## Focused evidence (no Debug instrumentation)

```
CARGO_TARGET_DIR=~/.../fyagent/src-tauri/target
rtk mise run rust:test -- <filter>
```

| Filter                                          | Result   | exit |
| ----------------------------------------------- | -------- | ---- |
| `opencode_connect_projects_independent_session` | 1 passed | 0    |
| `opencode_overview_observes_auth_json`          | 1 passed | 0    |

No full aggregate / `check:prearchive`. Root owns the final gate.

## Residuals

- Fixture/Memory checks do not prove OS keychain HIL, Windows UAT, or release.
- Other writers may still hold unrelated dirty files.
- No commit/push/Issue closure.
