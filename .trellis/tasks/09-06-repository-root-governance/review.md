# Root governance review

Implemented in an isolated worktree from91b37c8c to avoid concurrent Dialog
changes. No product source, native commands, versions or dependency locks change.

Six real configurations move toconfig/ without root shims. Vite source/envDir,
alias/dist, Vitest's two projects, Playwright testDir/server cwd, PostCSS plugin
and actual TS dependency graph retain their owners. Standard ESLint/tsconfig,
package/lock/version, Git metadata and project/licensing documents stay at root.
CI and labels recognize the new paths; retired names classify deletion sides.
The deprecation scanner now scansconfig/. No effective JSONL rows reference the
six old root files, so no historical text mass replacement is needed.

Actual-loader regression first exposed a test environment error: loading
esbuild directly inside jsdom crosses TextEncoder/Uint8Array realms. The test
now runs the real Vite loader in a Node child process, as normal tooling does;
there is no global constructor override, fake success or console suppression.
Focused architecture/classification/task/deprecation tests:126 passed,1 existing
skip. Strict types/lint/format and175 collected unit files pass1554 tests with
1 existing skip. Renderer build and seven-route chunk verification pass; the
functional browser configuration collects401 cases from18 files. Collection
is not claimed as execution; final merged browser verification is still required.

Secondary caller scan also found CI's independently collected desktop-mock
step and its verifier had a direct Vitest invocation. Both now select the
same config; the two diagnostics remain separate, with no workflow trigger,
permission or Required-CI policy change.

The first contract aggregate correctly rejected the remaining exact desktop
command assertion in githubWorkflowTriggers.test.ts. It now checks the new
explicit config while retaining all automatic trigger/independent-diagnostic
and mock-only conditions. Desktop mock7 and focused CI15 tests also pass.

Official editor discovery was checked against vitest-dev/vscode: its recursive
config search includes config/vitest.config.ts, and vitest.rootConfig supports
explicit selection. No speculative workspace/editor settings file is added.

Full native/prearchive verification will run on the merged main checkout,
whose unchanged Rust artifacts are already available; it is not replaced by
this worktree's targeted tests. Archive only after that gate. Supplemental
browser tests keep ports isolated or wait for the other owned run; no existing
server is silently reused or killed. Root Finder metadata is checked/removed
individually during integration, not via git clean.

## Merged-checkout verification

Merge `9527d7a5` contains both the origin work `a483d8d9` and the complete
isolated branch through `d7f753a2`; ancestry was checked explicitly. Both browser
configs retained the scroll, dialog-origin and MCP follow-up tests after their
move. Exact project/file/test-name collection before versus after relocation is
476 versus 476, with no lost or added case, rather than only matching totals.

The merged full prearchive gate exited 0
(`/tmp/fyagent-round7-merged-root-gate.log`): 175 unit files, 1,560 passing cases
and one existing skip; 3,495 Rust cases passed with zero failures and six
existing ignores. Desktop mock, native Fetch, graph, actual configuration
loader, CSS prefixer and release/toolchain contracts passed. Current changed
SPEC files have 97 valid relative links and no broken relative reference.
The root code-file inventory now contains only the deliberately retained
`eslint.config.mjs`; no root TS config implementation or Finder metadata remains.
Final merged functional-browser and production-performance execution follows
this full gate and is recorded with the parent delivery evidence.

Those final sequential runs completed successfully: 476 functional browser
cases and 25 real-production cases, with process exit 0. Evidence is retained in
`/tmp/fyagent-round7-merged-browser.log` and
`/tmp/fyagent-round7-merged-performance.log`. The normal/4x navigation p95 values
are 29.0/50.2ms, modal warm-frame p95 is 33.4ms in both conditions, and step
resize p95 is 33.4/49.9ms. Theme segment p95 values remain 16.7–16.8ms.
Stress runs retain their recorded long tasks (99ms navigation, 52ms presentation);
these are bounded browser measurements, not a native GPU or universal 60fps claim.
The linked worktree is clean and its full head is an ancestor of the merged
checkout. Its final safe removal is owned by parent delivery, after archival.

Final reference review also aligns the Type Safety compiler-scope description
with `config/**/*.ts`. An experimental config under ignored node_modules/.cache
was rejected by Node's TypeScript-loader boundary; it is not a product-config
failure and is not used for acceptance. Verification uses the checked-in
configurations directly, waiting for occupied test ports rather than adding
an alternative config, loader flag or relaxed server reuse.

## Verified worktree cleanup

The isolated branch head `d7f753a24b9dd7c57b1ff4ab7adc5f05c710a427` was verified
as an ancestor of main checkout merge `9527d7a5`. Its tracked and ordinary
untracked status was empty, and no process held that worktree as its working
directory. The only ignored contents were this checkout's generated caches,
dependencies, virtual environment and build output.

Normal `git worktree remove` completed without force, followed by normal
`git branch -d refactor/round7-root-governance`. The path no longer exists,
the branch no longer exists, `git worktree list --porcelain` contains only
the primary `dev/laiyongjie` checkout, and `git worktree prune --dry-run`
reports no stale administration. No unrelated worktree, branch, remote or
uncommitted source was removed; no reset, force-clean, history rewrite or push.
