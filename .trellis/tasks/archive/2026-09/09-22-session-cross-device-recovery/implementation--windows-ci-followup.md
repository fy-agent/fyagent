# Windows CI follow-up

PR #197, inspected head `2335438e58dadc828d863473249faa40a9419f16`, [CI run 35654321621](https://github.com/fy-agent/fyagent/actions/runs/35654321621), Windows backend job `106514785768`.

## Failure and correction

- Clippy rejected the Hermes test helper `expectation` as unused on Windows. Its only caller already requires macOS; the helper now has the same condition. No warning suppression or runnable Windows test was removed.
- Three Codex extraction cases in `session_migration_model` reached the production Windows home resolver before Shell-context initialization. The integration binary now uses the existing shared test mutex and isolated-home reset before extraction. The production identity resolver and elevated execution restrictions are unchanged.
- The Windows library suite had already passed 3,748 tests; eight migration integration cases passed and three failed. The separate Credential Manager check also passed. These results do not turn the failed backend job into a pass.

The correction adds seven lines across two Rust files. The Hermes source fingerprint in the supported-platform registry is updated to match the reviewed source; structural rules and CI gates are unchanged.

## Validation and evidence boundary

After the correction, canonical macOS `rust:fmt:check`, `rust:clippy`, and the targeted `rust:test -- session_migration_` all exited 0; migration integration executed 11 tests with 11 passes, no failures, and no ignored cases. Coordinator `check:contracts` also exited 0: 664 passed / 1 existing skipped, native Fetch 4/4, and platform coverage 3,142 files. The new archived report was staged before this check so the archive validator could verify its Git mode.

The initial hosted run finished its frontend job successfully: production boot 3/3, full browser regressions 667/667, and CI-selected unit tests 2,037 passed / 1 skipped. The full macOS backend and both Windows native-contract jobs also succeeded. The failed Windows backend and aggregate results remain recorded as failures until the corrected head is checked.

A subsequent Windows hosted-CI result is required to establish Windows success. Follow the latest PR check result; this document records the failure and local correction rather than predicting the rerun. Windows real-machine UAT remains waived by the user and does not waive CI failures or alter product capability limits.

The native executor delivered this correction and released its write scope before coordinator integration. Exact job logs, failure excerpts, patch, source hashes, and local command records are retained outside the repository in `work/executor-logs/ci-windows-fix/`. This is a new follow-up document, not a transformed original in the historical archive path map.
