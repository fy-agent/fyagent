# Verification evidence

Evidence is recorded only from fresh command output on product digest
`cd38c07602e747179444641b6b77da9db1d70876f5507d2ccca3e21be421d018`.

| Gate | Result | SHA/digest | Evidence |
|---|---|---|---|
| V2 lint | pass | `cd38c076…` | `mise run lint:v2`, exit 0 |
| V2 typecheck | pass | `cd38c076…` | `mise run typecheck:v2`, exit 0 |
| V2 unit | pass | `cd38c076…` | `mise run test:v2`: 44 files, 314/314 tests |
| V2 browser | pass | `cd38c076…` | final canonical rerun `mise run test:v2:browser`: 116/116 |
| renderer build | pass | `cd38c076…` | 726 modules; standalone preview built; existing >500 KB chunk warning only |
| Rust fmt/check/clippy/test | pass | `cd38c076…` | aggregate `mise run check`: fmt/check/clippy pass; lib 2838 passed, 5 ignored; integration suites pass |
| Repository Contracts | pass | `cd38c076…` | `mise run check:contracts`, exit 0; contract tests 510 passed, 1 skipped |
| GitHub CI / Required initial | pass | `5b2d904b03549621f2caea9497ac6c7dcbcf23a5` | push run `32663818280` and pull-request run `32663856167`; both Required aggregators passed after Linux contracts/frontend, macOS backend, Windows backend, X64 and ARM64 contracts passed |
| GitHub CI / Required governance head | pass | `3c27acaf97f2e1a052552a7f5e392debf64fc549` | push run `32664954837` and pull-request run `32664957087`; both Required aggregators and all hosted jobs passed |
| GitHub CI / Required latest finalization head | pending | pending | authoritative live check on PR #135 after the task-state-only amend |

## Local gate notes

- `mise run check` completed successfully after the final Trellis code fixes.
- One browser run started under concurrent local load with one `top-bar` readiness timeout (`115/116`). The exact failed case immediately passed `1/1`, and a fresh canonical full run then passed `116/116`; no product change was made between runs.
- `origin/main` was fetched immediately before commit preparation and remained `e94307cd810d7c5157b3791da2a8d7ef6a01b8a7` with Schema v19.
- Grok final static review: PASS, no P0/P1. Independent Trellis review: PASS_WITH_FIXES, all P1 fixed and reverified.
- Gemini review did not execute because both available Google routes were externally blocked; its receipt is BLOCKED/INCONCLUSIVE and is not counted as PASS.
- After recording the initial CI and source-PR governance readback, `mise run check:contracts` passed again: all 80 task cards valid; 510 contract tests passed and 1 skipped.

## Unverified by local host

Hosted Windows/macOS evidence passed for both `5b2d904b` and governance head `3c27acaf`; a fresh Required run remains mandatory after the final task-state-only amend. Real Provider UAT, WebDAV device round-trip, merge and Release are unverified. No local, static, or earlier-head CI result may be promoted to those evidence levels.
