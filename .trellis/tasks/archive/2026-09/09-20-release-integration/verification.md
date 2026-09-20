# Same-delivery PR integration

## Scope and provenance

The delivery consists of FDE PR #193 (branch `codex/fde-delivery-integration`,
source head `590f6f96`, including its CI repairs) and subscription PR #192, which incorporates that FDE
history. Review #193 first, then the incremental #192 diff against its FDE base.
After #193 reaches main through the repository merge queue, retarget #192 to
main and require checks on that comparison before queueing it. This task does
not merge either PR or publish a release.

The old #192 head `6a4af16d` failed the workstation-path privacy test because the
UAT guide used two concrete machine paths. The guide now uses named placeholders;
the privacy check remains enabled. The failed run is `35452775621`. Its 636
browser passes do not cancel the failed unit-contract result.

FDE's first PR run, `35493454323`, exposed Windows test initialization and
platform-expectation failures. Its frontend failure was the WebKit Agents
contrast case: glyph positions were sampled without waiting for the initial
directory scan to remove its progress block and commit the final card order.
The contrast test now requires scan completion, retaining the 4.5:1 threshold
and bright-background regression. Matching hosted evidence is still required
for the new commit; the old failed run is not acceptance.

The combined local browser run also exposed an independent guide sizing bug.
The guide lacked the inherited brand-size variables: a focused regression
measured a 512 px frame where 64 px was intended. Local guide variables restore
64 px frames and 48 px artwork. All 30 Chromium/WebKit guide cases then passed,
with the original viewport, keyboard and persistence assertions preserved.

## Combined behavior under review

- Both schema-22 histories must migrate to the combined schema with original
  proxy settings, project data and verification records preserved.
- OpenCode keeps builtin-provider structure, exact provider-ID selection,
  subscription binding/restoration and unknown-state write blocking.
- Model adapters remain lazy. Six adapters are direct bootstrap dependencies;
  the subscription adapter is a separately checked dynamic child of Models.
  The existing startup and per-chunk limits remain unchanged.
- Startup import must not replace a managed OpenCode provider's canonical
  upstream definition with its live loopback projection.
- Health observes the OpenCode subscription route without mutating data or
  reporting a valid projected endpoint as drift.
- FDE's saved-model HTTP probe rejects managed subscriptions before extracting
  credentials or sending a request. It reports the check as unsupported. Using
  the subscription from the target Agent remains a separate capability.

## Evidence ownership

The research reports in this task record review findings and their closure.
The final combined local prearchive check passed on 2026-09-20: 200 frontend
test files with 1,869 tests passing and one skipped; 3,700 Rust tests passing
with six intentionally ignored; type checking, lint, formatting, Rust checking
and Clippy, renderer build budgets, source/platform contracts and task contracts
all passed. The separate desktop mock suite passed seven tests, and the contract
and native-fetch suites passed 652 and four tests respectively. The complete
browser command passed all 640 cases after the final browser fixes.

Canonical local gate logs are kept under `.trellis/.runtime/verification/`:
`integration-prearchive-accepted.log` and `integration-browser-complete.log`.
Earlier failing logs remain historical diagnostics. In particular, the initial
parallel browser/unit run exposed a stale exact TestHome guard and exceeded
several existing unit timeouts under load. The guard now includes the two
settings-cache reloads already used by TestHome; the full gate passed when run
serially, without increasing timeouts or dropping assertions.

GitHub check results belong to each PR's exact delivered head; the PR bodies
record the final run links after completion. Pending runs are not acceptance.

The earlier signed subscription app/ZIP and installed FDE app are unchanged
historical candidates. Neither is a combined binary. This integration does not
run a migration against real user data, prove real subscription entitlement,
perform Windows device UAT, contact Apple notarization, or publish a release.
FDE project context files and Codex project-directory preparation currently have
a macOS implementation; Windows explicitly reports the operation as unavailable.
Its native test asserts that this failure publishes no context or directory and
does not advance the project revision. This is a product capability boundary,
not merely a missing Windows UAT receipt.

## 0.4.6 acceptance follow-up — 2026-09-20

The later release coordination includes workflow PR #194 as the final
integration boundary, prepares version 0.4.6 and plain-language release notes,
and owns ordered protected merges of #193, #192 and #194. Earlier numerical
results above describe their historical candidate, not this final candidate.

The scope decision for this follow-up is to repair confirmed compatibility
defects through existing FyAgent owners. It does not import another complete
upstream release or change the retained CC Switch v3.19.2 ancestry baseline.
The immutable upstream references and licensing are recorded in
[Selected CC Switch compatibility fixes](../../../../../docs/upstream/cc-switch-compatibility-2026-09-20.md).
This is the separately recorded narrow-fix decision required by the upstream
contract; it does not authorize unrelated upstream UI, application additions,
remote changes or release publication.

Focused evidence before the final combined gate:

- Legacy 0.4.5 subscription backups: seven native regression cases and an
  independent review close the upgrade recovery gap. Historical local backup
  authority remains explicit; absent historical signatures are not invented.
- Proxy settings and universal-provider metadata: the original source failed
  three of four new cases after correcting a fixture initialization issue.
  The repaired source passes all four plus the existing ephemeral-port
  takeover/restoration case, formatting and Clippy. An independent reviewer
  found no remaining P1/P2 issue in these two files.
- Commentary/tool turns and empty reasoning: the original source failed four
  of five cases. Independent review then caught paragraph indentation loss;
  that regression also failed before correction. The final two modules pass
  85 and 14 cases, formatting and Clippy, with the review finding closed.
- xAI native Responses: an actual schema regression fails on the old source;
  the initial adapted path passes 72 focused cases. Independent review then
  reproduces original decimal precision loss and dropped SSE id/retry/comment
  fields. The repair in `a26723dd3c93c7343653ec222c140c95c73bc964`
  rewrites only provably lossless original numeric tokens and replaces only
  SSE data lines, retaining other fields and line endings. The revised path
  passes 84 focused cases, formatting and Clippy. Its independent re-review
  and full-candidate gates remain required before merge; the earlier 72-case
  result does not close either finding. The replacement production stream
  wrapper retains the prior namespace regression coverage.
- Kimi Quick Setup already uses native Responses on the actual user path.
  Only regression coverage is added; no production preset or user interface is
  changed, and Kimi is not advertised as a newly introduced capability. The
  frontend subset passes 66 cases. Its new native saved-provider readback test
  remains part of the final combined Rust gate.
- Navigation coverage now checks actual current-link/frame geometry after
  real clicks in all five browser environments. The five new executions and
  24 existing shell executions pass without a fake clock, longer waits or
  weakened assertions. The initial intermittent native visual observation
  did not reproduce in the later four native checks; no speculative product
  animation change is made or claimed as a fix.

PR #193 entered main at `d07eaef982c7ede3a84c0dd4e14a67c7b34af826` after
merge-group run `35517573737` succeeded. Its tree exactly matches the previous
#192 base `590f6f965e0fb431bb473e288e0ca977d0c27b3a`, so the already-passing
#192 exact-head comparison has the same source content after retargeting.
An early overlapping queue attempt was removed by GitHub with
`invalid_merge_commit`; the sequential retry after #193 merged generated
merge group `243236e719486606a6337910d12147049e4ce1f5`.
That group passed all required checks in run `35519051677`, and PR #192 merged
at that exact SHA on 2026-09-20 at 15:45:51 UTC. Its tree exactly matches the
reviewed PR head `6af07c82107e399a7606673cb1941f0d3d95914a`.
The final #194 PR and merge-group results remain owned by their exact SHA;
pending hosted checks are not local acceptance or completed merges.

The coordinator runs the final combined local gates after integration, then
reads back the final protected mainline merges. Fixture tests and local native
smoke checks do not establish real subscription entitlement, upstream quotas,
Windows device UAT, notarization or publication of installation assets.

The separate native smoke initially suffered a test-tool relaunch without its
environment-only isolation setting, transiently upgrading the installed
application's database structure. The coordinator stopped only that test
process, backed up the live database, and restored only the proven proxy-table
schema/user-version delta in a reviewed transaction. The other 31 business
tables stayed identical within that transaction and integrity checks passed;
no old full database was restored over newer data. The installed application
was not replaced or stopped. A later test build hardcodes its synthetic home
at process entry; a launch without the environment variable proved that it
opens only that test database and leaves the real schema at 22. This local
test overlay is not part of the product commits or release artifacts.

Jev recommended a final-source isolated native rebuild and bounded smoke in
addition to complete local gates. The actual final run links, source bindings,
remaining ignored-test boundaries and protected merge results are recorded in
PR #194; the recommendation itself is not acceptance evidence.
