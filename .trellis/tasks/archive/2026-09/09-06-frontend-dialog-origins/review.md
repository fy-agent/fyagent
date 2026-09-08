# Dialog-origin integration review

## Scope / actual causes

The original 39 direct/wrapped Dialog call sites were reviewed, not counted as
39 independent business operations. Existing origin refs were already present
for account removal; the defect was cancellation by intrinsic preview sizing.
Before the fix all six immediate/120/300ms Chromium/WebKit account tests failed
with cancelled entry geometry (`/tmp/fyagent-round7-origin-before-2.log`).

The shared adapter now retargets the same geometry track in viewport coordinates,
keeping its remaining deadline and other tracks. No native request is delayed,
no minimum-loading timer is added, and cancel/apply authorization is unchanged.
Cancelled account previews have a request generation so a late result cannot
overwrite a later account/session. The browser test never confirms a real delete.

## Integration coverage

| Source family                                        | Ownership and checks                                                                                                                                                                                                                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Auth add/remove/connection                           | Existing explicit page refs retained; rapid removal result, stale cancellation and return focus are new physical tests.                                                                                                                              |
| Agent Auth, file recovery                            | Existing exact initiating refs retained; baseline browser and port contract coverage rerun.                                                                                                                                                          |
| WorkBuddy trust from Agent/MCP assignment/bulk       | Switch asChild and AssignmentPanel forward the real initiating ref before async handlers; required wrapper origin; both page paths tested.                                                                                                           |
| Skills More menu/settings/backups/local/ZIP          | One-use finite geometry/radii/non-resource material, explicit persistent More return anchor; real menu dialogs tested through complete teardown. ZIP retains the same capture across the native picker boundary, without claiming native picker HIL. |
| Skills uninstall/discovery/migration/backup deletion | Existing individual triggers/nested origin refs reviewed; generic dialog/frame suites and wrapper coverage retain these chains.                                                                                                                      |
| MCP edit/delete/install/reconfigure                  | Existing refs retained; confirm-to-target-picker captures the actual transient confirmation before unmount, with catalog return; no upsert is admitted during cancellation test.                                                                     |
| Model save/delete/overwrite/probe                    | Existing refs and write/secret contracts retained; shared confirmation captures its own button for a later native overwrite result.                                                                                                                  |
| Prompts/Memory discard/delete/navigation             | Existing matched navigation intent and local refs retained; deterministic deferred-close test prevents stealing focus from an already chosen editor.                                                                                                 |

Every production Dialog and wrapper call now has an explicit origin attribute,
checked through the existing TypeScript AST inventory (including import aliases).
The base Dialog prop is required; intentionally programmatic test roots pass
undefined. This guard prevents omission, but does not pretend static syntax
alone proves every runtime path: physical timing/geometry tests remain required.

## Additional defects found

Skills settings has a never-opened sibling confirmation. An empty propagating
AnimatePresence registered a parent barrier without an exit child, leaving an
invisible dialog/scroll lock after all native tracks completed. Propagation now
participates only while a real layer is open/exiting. Existing Radix and Motion
remain lifecycle owners; no library patch or alternative focus trap was added.
The deterministic sibling test and real menu open/close both verify cleanup.

Independent review found captured geometry could become invalid after a window
resize while an async intent waited. Consumption now validates finite positive
dimensions and current viewport bounds, then uses a live or neutral fallback.
The new deterministic test rejects the previously admitted offscreen capture.

Memory's warning probe exposed nested async userEvent inside act with unrelated
zero-delay waits. The test now awaits the real interaction and existing dialog
helpers, with an explicit pass-through diagnostic assertion. No console output
is hidden and no animation or focus rule is disabled.

The full browser sweep also caught a compact-window async source: at900x600,
Agent WorkBuddy feedback shifted the clicked switch from y530.95 toy623.95
before the trust dialog opened. Live-source validation correctly rejected the
now-offscreen control. A modal-producing Switch therefore explicitly captures
its valid click geometry and same-element return anchor before checked-change
effects. It reuses the transient-origin adapter; bounds checks remain intact,
and closing still remeasures the real control rather than flying offscreen.
The identical compact regression failed before this change and now passes.

## Verification

Before/after logs are retained under `/tmp/fyagent-round7-origin-*`. Focused
account/menu/WorkBuddy/stale-preview browser cases passed14; the corrected
two-stage MCP target-picker check passed in Chromium and WebKit. Unit suite
passed1555 plus one existing skip before final snapshot validation was added.
Final all-scope unit/browser/production/full-gate results are recorded below.
The origin interaction file is also selected by the existing serial production
configuration, reusing the exact same fixture/assertions against compiled CSS
and chunks rather than substituting a development-server timing result.

No real account writes, native picker, minimum-version WebView/GPU, signing,
publication or deployment has been performed. Intermediate passing subsets
are not the final integration claim.

## Final prearchive evidence

The complete current diff was re-reviewed against the task and the shared
motion/ownership contracts. The final functional browser run passed 474 cases
across four Chromium viewports and WebKit (`origin-final-browser.log`). The
follow-up MCP submit-origin verification independently passed four cases.
The full `check:prearchive` for this exact task exited 0
(`/tmp/fyagent-round7-origin-delivery-gate.log`), with 1,556 unit tests passed,
one existing skip, and unchanged native/contract gates; no act diagnostic or
suppressed warning was reported in that final run.

The serial production suite then passed all 25 cases
(`/tmp/fyagent-round7-origin-delivery-performance.log`). It includes the same
account preview timing/cancellation, transient menu, assignment and follow-up
origin assertions against production assets, as well as the existing navigation,
presentation, step-resize and theme budgets. No thresholds were loosened.
Root-config integration and the parent acceptance run still follow this child.

## Additional completion review

Two further callback gaps were corrected: catalog installation had a private
origin ref that was never forwarded to the page's trust notice; editor save
retained the editor-opening button instead of the actual submit control.
MCP owners now capture the submit event before the write and pass one source
record through the result. Shared installer callbacks expose that native event,
and both installation step dialogs identify their same-session presentationKey.
Four added Chromium900x600/WebKit follow-up tests passed with stateful synthetic
upsert, one write per action, sourced notice and complete teardown.

These are existing user-visible branches in the approved all-entry audit,
not new MCP capabilities or an additional animation framework. Final merged
acceptance must include these additions as well as the earlier origin suite.
