# Planning review and implementation entry

## Review result

The requirement set is split into two deliverables with a parent combination
gate. No user-owned behavior is silently removed: brighter appearance,
frost/liquid character, visible press feedback, staged content, return path and
performance all have explicit acceptance criteria. Navigation and native
operations are out of scope. Final user approval of this plan is still required
before starting either implementation task.

The desktop timing comes from inspection of the actual implementation and its
regression-test source. The reference application was not run or modified;
no claim of side-by-side real-time playback or equal visual fidelity is made.
Its cloned-source/frozen-rectangle semantics are intentionally not adopted.

The material decision keeps a single adapter and a cross-engine CSS baseline.
No new runtime dependency is selected solely from a demo or a library name.
Enhanced optics must earn their cost in the production modal benchmark, not
only in route navigation tests. Static color contrast passing is not proof
that brightness and glass character meet the user's expectation.

## Checks completed during planning

| Check                                   | Actual result                                                                            |
| --------------------------------------- | ---------------------------------------------------------------------------------------- |
| Three task context manifests            | task.py validate passed for all three                                                    |
| Production startup/navigation baseline  | Existing3 tests passed;42 returns at1x and4x;44.3ms/57.8ms p95                           |
| Four-size material/readability baseline | Existing12 tests passed;exit0 recorded                                                   |
| New task Markdown and JSON formatting   | Passed via mise format:files                                                             |
| Repository contract gate                | mise run check:contracts exited0;log at /tmp/fyagent-round5-planning-contracts.log       |
| Product-change scope                    | Git status shows only the three new task directories; product and existingSPEC unchanged |

These are planning/baseline results, not fifth-round implementation acceptance.
No native minimum-WebView/GPU measurement or new modal benchmark has passed yet.

## Next entry

After the final summary is approved, start`frontend-luminous-materials`, first
capture the modal frame-cost baseline before modifying product code, and then
execute the parent/child checklists. Implementation must update the actual
owningSPEC before each archive. Keep all tasks planning until that approval.
