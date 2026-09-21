# Proxy recovery compatibility after full native gate

Continue in the same root integration tree/branch, HEAD 35848bef plus task-owned
changes. Your earlier recovery writer stopped. The other Cursor conversation
owns usage/credentials and change-plan fixtures; root owns the catalog ACL test.
Do not touch those paths or any installer/frontend files. Preserve all edits.

Full native gate `artifacts/cursor-native-gate.log` has two recovery failures:

- `services::proxy::tests::codex_set_takeover_rebuilds_stale_enabled_state_without_overwriting_backup`
  at proxy.rs:5285: new owned-proof recovery reports conflict.
- `recover_from_crash_without_backup_cleans_placeholder_instead_of_writing_it_back`
  in tests/provider_service.rs:3150: new owned-proof recovery reports conflict.

Own the exact two failing tests plus existing proxy recovery implementation and
focused proxy specs only if evidence proves a behavior bug. Read current tests,
new recovery contracts and your prior result. Determine whether the fixtures
encode retired unsafe fallback behavior or whether a legitimate owned recovery
is broken. Preserve strict foreign-edit/ownership proof, recovery record and
user configuration. Do not make arbitrary/unowned fallback writes just to meet
old assertions. If safe success is no longer authorized, assert the actual
conflict and byte-for-byte preservation with a clear test name; cover the
corresponding legitimately owned success separately using existing tests.

Reproduce with the exact two filters, fix minimally, then run the affected
recovery_tests suite and the two filters. No full backend gate, branch/commit/
push/Issue updates, real user config or credentials. All terminal commands rtk,
tests canonical mise and root's existing native target directory. Different
test processes share that Cargo target and will serialize; do not change it.
Remove all debug instrumentation before final checks. Write
`research/cursor-recovery-integration-result.md`, then stop writer.
