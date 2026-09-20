# Repair the common-config preservation integration

Same Cursor desktop Debug conversation, existing model/account, same integration
tree and branch. The previous writer stopped. GPT-6 accepted the fixture
corrections and persisted-row upsert verification direction, but has not
accepted the new `persist_unowned_codex_common_tables` production path.

GPT-6 source review: `.trellis/spec/backend/codex-source-selection.md` gives
source selection a targeted patch over live that preserves MCP, features,
profiles and credential-store choice. The new helper copies every non-source
key from the whole effective Provider, not just explicitly enabled common
settings, so stale Provider tables can overwrite unrelated live tables. It
also performs a second file publication and treats every read failure as empty.
These are observable ownership/write-boundary regressions, even though the
new `[tui].notifications=false` test passes.

Grok is reviewing the original delta in parallel; its initial diff is preserved
at `artifacts/grok-credentials-reviewed-delta.patch`. Do not wait for that review
to begin fixing the root-confirmed defects below. Read
`grok-final-credentials-review.md` if it is available at your final handoff, and
report any finding it covers on old source as superseded by the actual fix.
Implement the smallest correction that keeps legal common settings working:

- Remove the broad post-snapshot whole-table copy. Only explicit enabled common
  snippet fields may extend the source writer's ownership; preserve unrelated
  live tables and sibling values. Do not restore old Provider snapshots wholesale.
- Compute the complete intended config before the existing atomic config write.
  Preview, writer and readback must share the same pure projection. Do not add
  a second independent publication or relax ownership/conflict/SecretRef checks.
- Failed/invalid current-config reads must reject, never become empty config.
- Add regressions for existing sibling `[tui]` settings, MCP/features/profiles
  and credential-store values surviving; disabled/absent common config must not
  import stale Provider-owned copies; an enabled valid common field applies;
  route/auth changes still require fresh authorization; preview/apply readback
  agrees. Cover any specific confirmed Grok finding with a narrow test.

Ownership: `services/provider/live.rs`, the exact existing pure Codex source
projection/writer helpers and their direct caller in `services/provider/mod.rs`
as necessary, focused credentials/source/ChangePlan tests, and corresponding
existing specs. No installer/helper/frontend/proxy/recovery edits. There is no
other writer on these paths. Preserve all prior task changes. Do not alter
global rules or unrelated supported-platform manifests; root handles the final
manifest review after code is stable.

Complete the coherent repair before rebuilding. Run relevant common-config,
source projection, credentials and ChangePlan filters/suites, with no Debug
instrumentation in final code. Root owns the full integrated gate. Report exact
changes, commands/results and residuals in
`research/cursor-common-projection-repair-result.md`; no commit/push/Issue
closure. Stop writer after delivery.

## Completed Grok review: one additional confirmed failure

`grok-final-credentials-review.md` is now saved. Findings 1-3 confirm the exact
old common-config defects this package is already replacing. Finding 4 is an
additional concrete regression in the previous upsert verification change:
create preview + writer returns Err before inserting its reserved row →
persisted-row verify returns TargetNotFound → generic ReadbackUnavailable /
RecoveryRequired, even when current/live remained exactly at baseline. Existing
compensation contract expects WriterFailedBaselineRestored / recovery Succeeded.

The narrow upsert adapter/classification repair and its regression are also in
your ownership (`change_plan/{adapter,service}.rs`). Preserve persisted-row
readback on success; distinguish an absent target after a failed create from a
present-but-unreadable target. Do not mask arbitrary readback/credential errors
or report restoration without actual baseline current/live evidence. Add a
failed-create/no-insert regression plus the corresponding successful upsert and
idempotence checks. Also verify an upsert whose retained binding is rotated after
preview is rejected before mutation; Grok listed this as a test gap, not a proven
defect. Do not expand into its speculative future scenarios.
