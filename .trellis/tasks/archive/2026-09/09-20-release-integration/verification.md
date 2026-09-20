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
