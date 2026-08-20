# Research: redundant contract tests

- **Query**: Tests that assert documentation file contents / markdown contracts / "docs contain string X"; overlap with generated-doc checkers
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

### Files Found

| File Path | Description |
|---|---|
| `tests/currentDocsContract.test.ts` | 812-line Vitest suite (`describe` "current FyAgent documentation authority", 14 `it`s). Reads Markdown/specs/YAML and asserts substrings, regexes, file inventories, and local-link existence. |
| `scripts/tasks/docs-contract-check.mjs` | Canonical Node checker: byte-compares `docs/fyagent/development/mise-tasks.md`, validates every `mise run <task>` in public READMEs / `CONTRIBUTING.md` / `.github` Markdown / `docs/fyagent/development/**`, validates standalone setup on `CONTRIBUTING.md`. |
| `scripts/tasks/task-docs.mjs` | Generator; `check` mode is a byte-for-byte compare of `mise-tasks.md`. |
| `tests/taskDocs.test.ts` | 219-line Vitest: re-implements byte-for-byte compare, re-executes `task-docs.mjs check`, plus fixture tests of `docs-contract-check.mjs` parsers. |
| `tests/localBuildBoundary.test.ts` | 466 lines; one `it` reads eight current documents and asserts they do not contain retired cross-build task/path strings. |
| `tests/desktopSecurityBoundary.test.ts` | 230 lines; one `it` reads the three public READMEs plus three locale `1.2-installation.md` files and asserts they do not contain `Windows-Portable`. |
| `.mise/tasks/contracts.toml` | `tasks:validate` runs `docs-contract-check.mjs`; `tasks:docs:check` runs `task-docs.mjs check`. |
| `scripts/tasks/release-check.mjs` | Always runs `task-docs.mjs check`; CI-safe Vitest list includes `currentDocsContract.test.ts`; local list adds `taskDocs.test.ts`. |
| `.github/workflows/ci.yml` | Frontend unit job excludes `tests/taskDocs.test.ts` (host-mise suite); contracts job still runs `release-check.mjs --ci` which includes `currentDocsContract` and `task-docs` check. |

### Code Patterns

#### 1. Template-doc / "docs contain string X" (primary)

`tests/currentDocsContract.test.ts` is the concentrated markdown-contract suite. Representative assertions:

- Public READMEs must contain architecture tokens (`React`/`Vite`, `Tauri IPC`, `Rust`, `SQLite`), `docs/fyagent/development/README.md`, `mise >= 2026.8.6`, onboarding commands in exact order (`mise trust` → `bootstrap` → `system:check` → `dev`), `mise run check`, `CI / Required`, `HIL` (lines 291–338).
- `CONTRIBUTING.md` must contain `fy-agent/fyagent` (≥2), maintainer/fork/origin prose, `CI / Required` (≥2), `squash` (≥2) (lines 341–358).
- READMEs must contain `src="assets/brand/github/for-you-gate.svg"` and `discussions/categories/q-a`, and must not contain `src="assets/fyagent.png"` (lines 367–375).
- GitHub discussion templates must contain `body:` (lines 379–385).
- `.github/workflows/release.yml` must contain `docs/release-notes/${RELEASE_TAG}-en.md`; each release-note file basename must appear in `docs/release-notes/README.md` (lines 418–429).
- Windows package-bridge docs/specs must contain identifier strings (`FOLDERID_ProgramData`, `FYABRIDG`, `UrlCreateFromPathW`, `Job remains \`Installing\``, etc.) and must not contain retired HTTP fallback strings (`FYAHHTTP`, `http://127.0.0.1`, …) (lines 473–588).
- Spec files must contain protocol/schema/API/toolchain strings: `fyagent://v1/import`, `fyagent-download-manifest/v3`, `canonical_sid`, `Node.js 24.19.0`, `Rust 1.97.1`, `get_workbuddy_model_ids() -> WorkBuddyModelIdsResult` (lines 609–638).
- READMEs + three locale installation guides must contain every `expectedInstallerNames("1.2.3")` template with `X.Y.Z`, plus `NSIS` / `Developer ID`; READMEs must contain `NotSigned` and `signing-status.json` (lines 641–676).
- Language-link HTML: `href="README_EN.md">English</a>` etc. (lines 679–689).
- READMEs must contain three screenshot paths; marketing samples must contain `status: superseded` / `status: concept_candidate` (lines 692–782).
- Audit doc `docs/fyagent/audits/user-manual-screenshots.md` must contain every user-manual PNG filename as a backtick token (lines 767–770).
- Inventory freeze: development docs list equals `CURRENT_DEVELOPMENT_DOCS` (11 files); each of 3 locales has exactly `EXPECTED_MANUAL_CHAPTERS` (26 chapters); shot-cards length 16; user-manual PNG count 40; image-reference count 84 (lines 431–765).

Helper `read()` at lines 150–154 is `fs.readFileSync` + CRLF normalize. `currentAuthorityMarkdownFiles()` walks `docs/fyagent/development`, `docs/user-manual`, `.trellis/spec` plus root READMEs.

#### 2. Generated-doc byte-for-byte (stacked three to four times)

The same generated file is compared in overlapping layers:

| Layer | What it does |
|---|---|
| `mise run tasks:docs:check` | `node scripts/tasks/task-docs.mjs check` |
| `docs-contract-check.mjs` `validateDocsContract()` | `committed !== generateTaskDocs()` throws "Generated task documentation is stale" (lines 270–280) |
| `tests/taskDocs.test.ts` first `it` | `expect(document).toBe(generator.generateTaskDocs())` then `execFileSync(..., [GENERATOR, "check"])` expecting `"byte-for-byte current"` (lines 50–63) |
| `release-check.mjs` | always appends `["task-docs", "node", ["scripts/tasks/task-docs.mjs", "check"]]` |

`taskDocs.test.ts` also asserts the generated doc contains the generator banner and does not contain `## Trellis and Codex Hooks` (lines 53–57), and that every `mise tasks ls --json` name appears as a Markdown table cell (lines 66–81).

#### 3. Standalone setup / onboarding order (split across three surfaces)

| Surface | Target files | Mechanism |
|---|---|---|
| `docs-contract-check.mjs` `validateStandaloneSetup` | `CONTRIBUTING.md` only | exact fenced block of four commands + "never run automatically" prose + `mise run check` |
| `tests/taskDocs.test.ts` "standalone setup contract" | fixtures, not repo files | calls the same exported `validateStandaloneSetup` |
| `tests/currentDocsContract.test.ts` README onboarding | `README.md`, `README_EN.md`, `README_JA.md` | `indexOf` order of the same four commands plus `mise run check` |

`.trellis/spec/backend/task-runner-contract.md` §6 names `docs-contract-check.mjs` as the maintained-docs owner for mise references and standalone setup.

#### 4. Cross-suite duplicate "docs must not contain X"

| String / class | `currentDocsContract` | Other suite |
|---|---|---|
| `Windows-Portable` / `FyAgent-X.Y.Z-Windows-Portable.zip` | READMEs + locale install guides (lines 652–655) | `desktopSecurityBoundary.test.ts` lines 168–174: same six docs plus `scripts/generate-download-manifest.mjs` |
| Retired cross-build task names (`macos:preflight`, `build:cross-windows`, paths `scripts/macos-cross`) | not this file | `localBuildBoundary.test.ts` `CURRENT_DOCUMENTS` (8 files including the three READMEs, `CONTRIBUTING.md`, four development docs) lines 434–443 |
| Retired backend spec filenames `windows-release-boundary.md`, `fyagent-v1-0-1-config-domains.md` | all "current authority" markdown (lines 591–602) | existence check that those spec files are absent (lines 440–442) |
| `docs/fyagent/dev/` | current authority markdown | existence check `docs/fyagent/dev` directory absent (line 405) |

#### 5. Spec-as-documentation string contracts

`currentDocsContract` treats `.trellis/spec/backend/*.md` as documents that must contain live protocol/API/toolchain tokens. Adjacent suites assert the same tokens against **code**, not docs:

- `canonical_sid` / not `canonical_user_sid`: docs in `currentDocsContract` line 632–633; Rust sources in `tests/codexWindowsUserScopeContract.test.ts` lines 91–94.
- `Node.js 24.19.0` / `Rust 1.97.1`: docs in `currentDocsContract` lines 637–638; `tests/ciWorkflow.test.ts` asserts CI YAML does **not** contain those literals (workflow must not freeze versions). Opposite polarity, same strings.

#### 6. Visual / marketing inventory frozen in two places

`currentDocsContract` `VISUAL_DELIVERABLES` (lines 139–148) requires eight markdown files to exist and three sample docs to contain status strings. Independently, `scripts/tasks/supported-platform-raster-assets.json` freezes SHA-256 of the three ~1.5–1.7 MB marketing sample PNGs those docs embed. `tests/remainingPlatformSurface.test.ts` `it("freezes the decoded and visually reviewed raster inventory by path and digest")` (line 1267) loads that JSON.

Untracking the sample PNGs would fail both the docs-existence/status tests and the raster digest contract.

### Overlap matrix (same generated or public docs)

| Assertion class | `docs-contract-check.mjs` | `task-docs.mjs check` | `taskDocs.test.ts` | `currentDocsContract.test.ts` | `localBuildBoundary` | `desktopSecurityBoundary` |
|---|---|---|---|---|---|---|
| `mise-tasks.md` bytes | yes | yes | yes (+ exec check) | file listed in `CURRENT_DEVELOPMENT_DOCS` only | no | no |
| `mise run <task>` on READMEs / development docs | yes | no | parser fixtures only | README onboarding command **order**, not live task names | no | no |
| CONTRIBUTING standalone setup | yes | no | fixtures of checker | CODEOWNERS/fork prose, not the fenced setup block | retired-task scan includes CONTRIBUTING | no |
| README installer names | no | no | no | yes | no | no |
| README `Windows-Portable` | no | no | no | yes | no | yes |
| README retired cross-build | no | no | no | no | yes | no |
| spec protocol strings | no | no | no | yes | no | no |
| screenshot/marketing inventory | no | no | no | yes | no | no |

CI/local execution also stacks: `release-check.mjs` runs `task-docs` check **and** Vitest `currentDocsContract`; local non-`--ci` additionally runs `taskDocs.test.ts`. Spec `.trellis/spec/backend/github-ci-workflow.md` line 234 states the frontend suite excludes `taskDocs` and the contracts job owns the pure/static contracts.

### Related Specs

- `.trellis/spec/backend/task-runner-contract.md` §6 Generated Documentation, §7 validation matrix rows for stale generated docs / legacy entrypoints / standalone setup, §8 Tests Required (`taskDocs.test.ts`)
- `.trellis/spec/backend/github-ci-workflow.md` — contracts job vs excluded host-mise suites
- `.trellis/spec/backend/development-environment.md` — same four excluded suites
- `.trellis/spec/backend/application-brand-assets.md` — canonical `assets/fyagent.png`; README img-src prohibition lives in `currentDocsContract`, not this spec's tests (`tests/applicationBrandAssets.test.ts` hashes the PNG itself)

## Caveats / Not Found

- No separate `*docsContract*` files besides `currentDocsContract.test.ts`. Markdown-content asserts also appear as **single tests** inside `localBuildBoundary.test.ts` and `desktopSecurityBoundary.test.ts`.
- `tests/classifyChanges.test.ts` mentions README/docs paths only as **fixtures** for the change classifier, not as live-repo content contracts.
- `tests/v2/platform/featurePorts.test.ts` uses `MEMORY.md` / `2026-08-14.md` as **runtime filenames**, not documentation contracts.
- This note lists overlap as it exists; it does not delete or rank "keep vs drop".
- `prd.md` for this task is still TBD; item 5 mapping is from the research prompt, not from a written AC list.
