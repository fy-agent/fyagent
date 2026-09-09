# Verification evidence

## Starting point

Original PR #185 head: `2956da243d1e79f401d946852dadba8f30d28043`.
CI run `34229637275`, Frontend Checks job `102072461491`: one WebKit dark-theme
readability failure, 535 browser cases passed. Grok Build text sampled white
over RGB(111,141,164), contrast 3.487260310188897 versus the existing 4.5 limit.
The original two backend jobs passed. No required check policy was changed.

## Browser evidence

- Unchanged original dark-theme WebKit case: 3/3 passed on this macOS host.
  Therefore the specific hosted compositor/font cause is not established.
- A deterministic real-card regression replays the observed backing brightness.
  Before the CSS change it fails: title 3.4523378936933087 and supporting text
  3.160268536459908. The initial selector-typo timeout is not baseline evidence.
- Existing `--fy-surface-inset` replaces ambient soft backing on directory cards.
  No color token, sampler, original assertion, retry or contrast limit changed.
- Original dark-theme case plus the new backing regression: 6/6 passed, each
  repeated three times on WebKit.
- Affected browser suites: 64/64 passed across four Chromium viewports and
  WebKit, covering theme transitions, text/material readability, directory
  behavior and synthetic Grok binding/source-plan flows.

Local tests use an ignored configuration derived from the checked-in Playwright
configuration: only port (4197) and artifact directory differ to avoid another
session's occupied 4173 listener. Production CI configuration is unchanged.

## Additional full-gate finding

Full frontend testing initially exposed a test comment rejected by the existing
supported-platform scanner. The comment was narrowed to the browser/run ID;
the scanner and its admission policy were not changed.

A subsequent run passed all 181 files and 1614 tests (one existing skip) but
failed with an unhandled `window is not defined` at the scan hook after jsdom
teardown. This is recorded as a failed run, not a pass. Four deterministic
regressions fail on the original scan owner: late success, late rejection,
retained callback and duplicate StrictMode admission. The local fix guards
unmounted owners and does not replace newer synchronous request IDs with old
render state; hidden-owner buffering and native job ownership stay intact.

The repaired scan and router suites passed 38/38. A full run then hit one
5-second hdiutil fixture timeout while another contract gate was also running.
The unchanged hdiutil suite passed 7/7 when run separately (its affected case
took 709ms). A subsequent serial complete `mise run check:frontend` exited 0:
181 files, 1618 passed and one existing skip; desktop mock checks and read-only
visual preflight also passed. No timeout, concurrency setting or retry policy
was changed to achieve that result. Prearchive repository contracts exited 0.
The earlier timed-out run remains a failure in this record.

## Scope of evidence

Browser/renderer tests use synthetic IPC/credentials. They do not demonstrate
real Grok account entitlement, paid quota consumption, actual installed CLI
requests or native Windows human acceptance. Historical WorkBuddy/Desktop and
two-machine checklists stay unchecked and are explicitly superseded, not
misrepresented as implemented by administrative task completion.

## Delivery record

At the time of this work record, GitHub delivery is pending. The replacement
pull request's checks, merge queue receipt and merge commit are the authority
for remote delivery. Cleanup must follow actual main inclusion, not a local
commit or an auto-merge request. Do not infer live-account acceptance from CI.
