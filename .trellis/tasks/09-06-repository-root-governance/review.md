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

Final reference review also aligns the Type Safety compiler-scope description
with `config/**/*.ts`. An experimental config under ignored node_modules/.cache
was rejected by Node's TypeScript-loader boundary; it is not a product-config
failure and is not used for acceptance. Verification uses the checked-in
configurations directly, waiting for occupied test ports rather than adding
an alternative config, loader flag or relaxed server reuse.
