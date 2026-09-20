# #64 / proxy portion of #61 delivery

Execution complete; native acceptance is pending root integration. Branch
`codex/next-proxy-recovery-64`, base
`2c09c4be2c5f50b4060fd6f7a13a7be31c364c54`, origin
`https://github.com/fy-agent/fyagent.git`. No push, merge, Issue closure, real
Agent configuration mutation, elevation or listener operation was performed.

## Changes and ownership

- Both ordinary API-key DB-backup exit branches now use `proxy/legacy_recovery.rs`.
  Admission checks the complete projection, the path-bound writer receipt and
  its original; the actual writer rechecks the owned file. Exact original bytes
  are restored when retained by the receipt. Already-restored targets are no-ops.
- Valid legacy receipts remain compatible, including masked-key-only hot switches,
  source switches retaining independently explained inactive Codex tables, and
  a missing DB backup with a verified real original receipt. Arbitrary receipt
  postimages, external edits and unverified historical files are not adopted.
- SSOT fallback uses the same guard. An ownership failure cannot fall through
  into unchecked placeholder cleanup. No source is fabricated by deleting keys.
- Managed exit accepts each owned postimage OR its exact original preimage,
  allowing partial-restore retry. Rebinding remains strictly postimage checked.
- Native Codex auth remains outside proxy restore ownership, including login
  refresh while the proxy is active. Existing format writers/catalog generation
  remain authoritative; three existing pure catalog-field functions are exposed
  within the crate for read-only admission, with no change to their algorithms.
- The root authorized a minimal `config/recovery.rs` extension. Its synchronous,
  non-Send `file_restore_scope` constrains paths and hashes under the existing
  WRITER mutex. It holds no lock while calling format writers, rejects nested
  restore scopes, tracks internal writes, and resets on drop. Normal writes are
  unchanged. This is per-file protection; it does not make a multi-file operation
  atomic. #35 should retain this guard when integrating any shared-writer changes.
- Existing FileRecoveryButton already discloses backup paths, rejects conflicts
  and advises reopening clients. OpenCode subscription restore now retains
  actionable failure guidance and comparison paths, and discloses the need to
  reopen OpenCode or start a new session. The current renderer has no generic
  legacy `set_takeover_for_app` control; no artificial settings surface was added.

## Anomaly matrix and filesystem evidence

The cases below are executable Rust tests using real temporary directories and
ordinary/atomic file writes. They are **written but not executed in this package**
because root requested one integrated Rust build. Source inspection established
the old unchecked writes; do not label these fixtures a passed native reproduction
until root runs them.

| Case | Required assertion in added fixture |
| --- | --- |
| External edit after Claude takeover | Both DB restore branches return error; exact external bytes and DB backup unchanged |
| External edits in Gemini / Grok / Codex | Exact changed bytes retained; explicitly restoring owned fixture bytes permits clean recovery |
| Native Codex login refreshed during proxy use | Login bytes unchanged after configuration restoration |
| Exact restore repeated | Original bytes restored, repeat is no-op and rolling backup retained |
| Newer FyAgent writer changes unrelated permissions | A valid newer receipt alone does not authorize restore |
| External edit after admission | Guarded publication refuses overwrite and retains recovery evidence |
| Corrupt / missing receipt | Live bytes and original DB backup remain unchanged |
| Missing DB backup, valid first-write receipt | Exact real original restored without inventing a source |
| Hot switch whose key is masked in projection | Latest intended source restores with unrelated settings retained |
| Managed interruption after first file | First file stays restored; rebinding rejects partial state; exit retry restores remaining file |
| Unplanned native login path inside restore scope | No file creation or write |
| Scoped continuation and later-file conflict | No lock recursion; first preimage preserved, completed first file retained, external second file preserved |

Files: `src-tauri/src/services/proxy/recovery_tests.rs` (9 tests) and
`src-tauri/src/config/recovery.rs` (3 new scope tests). Existing catalog, SSOT and
serialization tests now seed actual original/takeover writes and receipts instead
of treating an arbitrary backup plus absent/unrelated live file as overwrite
permission. The old cleanup-only test now asserts that missing original evidence
does not manufacture a credential-less configuration.

## Executed checks

- `rtk proxy mise run bootstrap`: pass. Frozen pnpm/uv dependencies installed;
  optional Windows cross-toolchain advisory only. No build.
- `rtk proxy mise run test:unit -- tests/renderer/pages/models/OpenCodeSubscriptionRestore.test.tsx tests/renderer/features/FileRecoveryButton.test.tsx tests/architecture/rustModuleBoundaries.test.ts`:
  **3 files / 22 tests passed**, final run at 2026-09-21 00:18 local.
- `rtk proxy mise run typecheck`: pass.
- `rtk proxy mise exec -- pnpm exec eslint src/pages/models/OpenCodeSubscriptionRestore.tsx tests/renderer/pages/models/OpenCodeSubscriptionRestore.test.tsx`: pass.
- Focused Prettier check of both TS files and both modified backend specs: pass.
- `rtk proxy mise run rust:fmt` and `rust:fmt:check`: pass (syntax/format only).
- `git diff --check` and Trellis task context validation: pass.

## Pending integrated acceptance

Run on the integrated tree after merging upstream proxy stop/port changes:

- `mise run rust:check`
- `mise run rust:test -- services::proxy::recovery_tests`
- `mise run rust:test -- config::recovery::tests`
- `mise run rust:test -- services::proxy::tests`
- Managed subscription restoration/activation tests, then required root native
  suite and contracts. No native test or compiler success is claimed here.

The proxy module overlaps upstream release changes in lifecycle/fallback context;
retain its stop/dynamic-port fix and protocol mappings. This package deliberately
does not edit listener stop ordering or Grok protocol mapping. Keep the new restore
guards on both normal and SSOT branches during merge. #35's later shared-writer
changes must preserve normal behavior outside `file_restore_scope`.

Native Windows atomic replacement, permissions/reparse behavior, real crash or
power-loss recovery, and actual external Agent pickup after restart remain native
runtime / HIL evidence, distinct from temporary filesystem tests. Existing legacy
DB backups cannot prove DB-only changes to original fields masked by successive
projections; the DB remains authoritative for those source fields. An unprovable
historical receipt requires manual comparison rather than automatic overwrite.
