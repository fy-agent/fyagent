# Round-seven integrated acceptance

## Requirement evidence

| Requirement                   | Implementation and evidence                                                                                                                                                                                                                                                     |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1: reachable long content    | `fa2684d0` assigns required flow/workspace tab roles and restores the bounded height chain; `3cc34ad7` verifies physical scrolling across all seven routes. No wheel interception or alternate scrollbar was introduced.                                                        |
| R2: source continuity         | `a483d8d9` preserves entry under asynchronous previews, captures explicit transient/reflow sources and return anchors, and covers WorkBuddy, menus, chained installs and recovery wrappers. The existing 39-site inventory and AST guard complement real geometry/timing tests. |
| R3: root governance           | `e4c0c038` and `d7f753a2` move six real configurations, retaining only justified root discovery entries. Merge `9527d7a5` incorporates the whole isolated worktree without dropping the newer dialog test selections.                                                           |
| R4: prevention and boundaries | Native scroll/keyboard, live preferences, stale previews, independent focus/lifetime, typed/AST admission and exact test collection are covered. No native command, dependency lock or `src-tauri` change was introduced relative to the round-six baseline.                    |
| R5: delivery                  | Three children are completed and archived. This parent owns the final full gate, archive-reference repair, journal and clean-checkout verification.                                                                                                                             |

## Merged verification

The merged tree passed the complete root-task prearchive gate, then functional
browser and real-production tests sequentially. Logs:

- `/tmp/fyagent-round7-merged-root-gate.log`: strict type/lint/format, 175 unit
  files with 1,560 passing tests and one existing skip, native tests 3,495 passed /
  zero failures / six existing ignores, desktop mock 7, release contracts 611 /
  one existing skip, native Fetch 4, plus graph/configuration/toolchain checks.
- `/tmp/fyagent-round7-merged-browser.log`: 476 passed across four Chromium
  viewports and WebKit. Exact project/file/test-name collection is unchanged
  across configuration relocation: 476 before and after, no lost/added cases.
- `/tmp/fyagent-round7-merged-performance.log`: 25 passed on actual production
  assets, one worker, no raised budgets. This includes entry/async/cancellation
  and follow-up source tests, not only route loading or a final data attribute.

At 1232x700 with 42 revisits per CPU condition, navigation p95 is 29.0ms at 1x
and 50.2ms at 4x. Modal warm-frame p95 is 33.4ms; same-session step resize is
33.4/49.9ms. Theme reveal thirds remain 16.7–16.8ms. Stress results retain a
99ms navigation and a 52ms presentation long task. These measurements do not
claim universal 60fps, OS input latency, real credentials or native GPU evidence.

## Merge and cleanup safety

The isolated tree was verified clean before removal; its nontracked content
was limited to generated dependencies/build/Python state, an empty `.fyagent`
directory and the task session pointer. Both its work commit and final head
`d7f753a24b9dd7c57b1ff4ab7adc5f05c710a427` are ancestors of the merged main
checkout. The worktree directory and branch are now absent, the Git worktree
registry contains only `dev/laiyongjie`, and dry-run prune reports no residue.
The archived task keeps its original implementation branch as history; a
validation notice that this merged branch was deleted is expected.

No push, release, deployment, real account operation, minimum-version native
WebView test or signing operation was performed. Existing historical task prose
and unrelated legacy broken context references are not mass-rewritten.

## Parent gate

After all three child archives and the final quality-guideline update, the
parent's own complete `check:prearchive` also exited 0
(`/tmp/fyagent-round7-parent-delivery-gate.log`). No product source, dependency
or build configuration changed after the merged 476/25 browser/production runs.
No unexpected React/act diagnostic or failed check appeared in the final gate.
The remaining administrative steps are the parent work commit/archive, its five
effective context-link relocations, no-exclusion contracts, and journal recording.
