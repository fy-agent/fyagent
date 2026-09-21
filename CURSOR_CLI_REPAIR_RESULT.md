# Cursor CLI repair result

Writer stopped after the scoped local commit below. No push, no PR, no Issue closure. Seed `e0cb6f69` was not cherry-picked.

## Location

- Directory: `/Users/serendipity/.codex/worktrees/fyagent-next-cli-space/fyagent` (`pwd -P` same)
- Branch: `codex/next-cli-space-27`
- Baseline HEAD: `be21f65337bd0fa5769c010c03166a4571b809ce`
- Remote: `origin` → `https://github.com/fy-agent/fyagent.git` (not pushed)

## What landed

1. Shared bounded `@iarna/toml@3.0.0` admission via `serde_json` + `File.take(limit+1)` in `src-tauri/user-helper/src/closed_dep.rs`. Malformed / trailing / duplicate / wrong-type / non-exact version fail closed.
2. Exact npm dest (`prefix` / `cache` / `temp` / `npm_identity`) lives on `PreparedInstallTarget` and plan-control v3. Equality ignores `available_bytes`. Spawn uses `pinned_npm_install_invocation` (confirmed program + `--prefix` / `--cache`; no second `config` scan). Windows helper observes TEMP/TMP with `GetEnvironmentVariableW`, not `std::env::temp_dir()`.
3. Claude and Grok share `npm_install_argv_or_reject_for`. Helper protocol is v4 (`ToolTargetChanged = 30`).
4. Host closed-graph admission in `src-tauri/src/services/tooling/grok_npm.rs` only: recognized platform suffix sets per product; nonempty/malformed root `peerDependencies` and child graphs rejected; absent/`{}` remain valid. **Every recognized sibling spec** is run through `GrokNpmInstallPlan::for_execution` (the existing exact-version validator). Distinct exact sibling versions remain allowed. Linux suffixes are vendor metadata, not first-party Linux support. Root source-manifest digest guards were not changed. No new resolver. Metadata tests stay platform-neutral: sibling keys are `tool.package()` plus the closed suffix table, the chosen sibling is never `current_platform`, prefix-only rejection uses an invented suffix, and there is no current-platform==foreign-platform branch. Scanner files were not edited; root owns the narrow contract that admits only the two complete suffix arrays.

## Tests

Commands used `unset CARGO_TARGET_DIR` in the CLI tree.

```text
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib -- \
  published_root_admits_vendor published_child_graph published_root_reads_optional
# ok. 3 passed (non-current sibling from closed suffix table;
# npm:left-pad@1.3.0 / ^1.0.25 / latest / @latest rejection;
# distinct exact sibling 1.0.26; invented-suffix prefix-only)

cargo test --locked --manifest-path src-tauri/Cargo.toml --lib -- \
  helper_platform_errors_keep_space preflight_is_bound_to_exact \
  windows_installer_errors_never_mislabel
# ok. 3 passed

cargo test --locked --manifest-path src-tauri/user-helper/Cargo.toml --lib -- \
  pinned_invocation confirmed_npm_target control_v3 closed_dep admit_closed_iarna
# ok. 7 passed

pnpm exec vitest run tests/codexUserHelperContract.test.ts
# first run failed: production windows.rs used std::env::temp_dir()
# after GetEnvironmentVariableW TEMP/TMP: 21 passed
```

Earlier host dest/graph compile of the pre-sibling-spec objects (12 passed) is superseded by the reruns above.

## Fixtures

No on-disk npm registry fixture was added. Closed-graph cases use inline `serde_json` documents in `grok_npm::tests`. `@iarna/toml` admission uses temp files inside `closed_dep` unit tests only. Helper Cargo.lock change is only `serde` + `serde_json` on `fyagent-user-helper`.

## Limitations

- Windows helper compile and Windows npm execution were not run on this Mac.
- Full aggregate / serial integration gate remains with root.
- Antigravity handoff files and `CURSOR_CLI_REPAIR_TASK.md` were left untracked.

## Commit

See git log on this branch after `be21f653`. Writer stopped.
