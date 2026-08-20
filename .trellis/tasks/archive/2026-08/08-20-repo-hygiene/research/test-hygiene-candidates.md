# Research: test hygiene candidates

- **Query**: Tautological, duplicate, or template-doc contract tests; obvious dumped-together modules this hygiene task might split **if already touching them**. List candidates with reason; do not delete yet.
- **Scope**: internal
- **Date**: 2026-08-20

## Findings

### Files Found — candidate classes

This list is an inventory with reasons. It is not a deletion plan.

### A. Template-doc / markdown-content contracts

See also `research/redundant-contract-tests.md`.

| File | Reason |
|---|---|
| `tests/currentDocsContract.test.ts` (812 lines, 14 `it`s) | Reads live Markdown/specs and asserts they contain architecture tokens, installer filename templates, status strings, screenshot paths, protocol identifiers, and exact file inventories (40 PNGs, 84 references, 26 chapters × 3 locales). Several `it`s are independent product concerns stacked in one `describe`. |
| `tests/taskDocs.test.ts` first `it` | Byte-for-byte `mise-tasks.md` compare that `docs-contract-check.mjs` and `task-docs.mjs check` already perform; additionally shells out to the same `check` CLI. |
| `tests/taskDocs.test.ts` "standalone setup contract" | Fixture tests of `validateStandaloneSetup`; production scan of `CONTRIBUTING.md` already lives in `docs-contract-check.mjs`. |
| `tests/localBuildBoundary.test.ts` `it("keeps current documents free of retired cross-build interfaces")` | Docs-must-not-contain retired task names; overlaps README set with `currentDocsContract`. Rest of the file tests `host-native.mjs` behavior (not a doc contract). |
| `tests/desktopSecurityBoundary.test.ts` `it("does not advertise or classify Windows Portable downloads")` | Same `Windows-Portable` substring as `currentDocsContract` on the same six install docs. Other `it`s in this file assert CSP/capability/Rust sources. |

### B. Duplicate execution of the same checker

| Candidate | Reason |
|---|---|
| `tests/taskDocs.test.ts` + `.mise/tasks/contracts.toml` `tasks:docs:check` + `docs-contract-check.mjs` + `release-check.mjs` `task-docs` step | Four invocations of generated-doc byte compare / mise-reference validation on the contributor path. CI excludes `taskDocs.test.ts` from the frontend unit job (`.github/workflows/ci.yml` lines 286–290) but `release-check.mjs --ci` still runs `task-docs.mjs check` and `currentDocsContract.test.ts`. |
| `tests/versionConsistency.test.ts` (3 `it`s) | Wraps `scripts/version.mjs get/check`: second test asserts `check` stdout equals `"FyAgent version contract OK: " + get()`. `release-check.mjs` already runs `pnpm run version:check` as its first plan step. |

### C. Source-string / CSS-string contracts (not markdown, same "file contains token" pattern)

| File | Reason |
|---|---|
| `tests/v2/pages/agents/Page.styles.test.ts` | `readFileSync` on CSS; regex-asserts exact custom properties (`--fy-catalog-rail-width: clamp(220px, 24vw, 268px)`), overflow, and "page CSS must not mention workbuddy". |
| `tests/v2/pages/skills/page.styles.test.ts` | Single `it`: CSS must match two overflow rules. |
| `tests/v2/pages/prompts-memory.styles.test.ts` | CSS must not contain `min-height: 220\|330\|450px` and must contain `flex: 1 1 auto`. |
| `tests/components/ProviderCardLayout.test.ts` | Asserts `ProviderCard.tsx` source contains/omits specific Tailwind class strings (`max-w-[280px]`, `min-w-0 flex-1 space-y-1`). |
| `tests/components/SelectItemIndicator.test.ts` | Asserts `select.tsx` source character order of `ItemIndicator` vs `ItemText`. |
| `tests/codexWindowsUserScopeContract.test.ts` (482 lines) | Reads many `.rs` files and `expect(source).toContain("GetShellWindow")` / Win32 symbol names. Adjacent behavioral coverage exists in `src-tauri/tests/codex_desktop_domain.rs`. |
| `tests/ciWorkflow.test.ts` (22 706 bytes) | Reads `.github/workflows/ci.yml` and asserts job names, `runs-on`, and substring presence/absence (including "must not contain `24.19.0`"). Spec `.trellis/spec/backend/github-ci-workflow.md` is the prose contract; this test freezes YAML text. |
| `tests/v2/app/architecture.test.ts` (526 lines) | TypeScript AST walk of `src/v2` import graph; architectural freeze, not a runtime test. |
| `tests/remainingPlatformSurface.test.ts` (1707 lines, 21 `it`s) | Mixed: scanner unit tests, frozen raster SHA-256 inventory, frozen "platform-sensitive source" path/mode/digest, active-task exclusion rules. One file owns both the checker behavior and the whole-repo asset seal. |

### D. Locale-key template family

Same shape: import four locale JSON trees, `it.each` that required key paths exist and are non-empty strings.

| File | Extra beyond `localeKeyParity.test.ts` |
|---|---|
| `tests/config/localeKeyParity.test.ts` | Full leaf-key set equality vs `zh.json` baseline. CI also runs `pnpm test:i18n` as a dedicated job (`.github/workflows/ci.yml` lines 292–295). |
| `tests/config/xaiOauthLocales.test.ts` | Subset of keys under `xaiOauth.*` / `managedAuth.*`. |
| `tests/config/managementListLocales.test.ts` | Subset of search/bulk keys; also interpolation-variable parity vs `en`. |
| `tests/config/toolManagementLocales.test.ts` | Subset of `settings.*` tool-management keys; interpolation parity vs `en`. |

Subset tests still catch empty-string values that key-parity would not; they duplicate the "key exists in all locales" check that parity already covers for those paths.

### E. Split / extra test files for one unit

| File | Reason |
|---|---|
| `tests/hooks/useImportExport.test.tsx` vs `tests/hooks/useImportExport.extra.test.tsx` | Same hook, same API mocks, second file labeled "edge cases". |
| `tests/config/*ProviderPresets.test.ts` (9 files: claude, longcat, mimo, doubao, therouter, subrouter, opencode, xaiOauth, codexChat, plus `therouterOpenCodeOpenClawPresets.test.ts`) | Each file `find`s a named preset and asserts pinned env literals. Pattern is repeated; `providerPresetPromotionBoundary.test.ts` is a separate boundary. |

### F. Raster digest tests overlapping asset copies

| File | Reason |
|---|---|
| `tests/v2/shared/agentAssets.test.ts` | Hard-coded SHA-256 for `qoderwork.png` / `trae-work.png` (same digests as `supported-platform-raster-assets.json`). |
| `tests/v2/shared/appAssets.test.ts` | Byte-identical copy check extracted → V2; `SKILL_TARGET_IDS` listed twice in one `it`. |
| `tests/applicationBrandAssets.test.ts` | PNG/SVG/ICO digest and geometry; overlaps raster JSON for `assets/fyagent.png` and tray PNGs. |
| `tests/remainingPlatformSurface.test.ts` raster `it` | Whole-inventory digest freeze including marketing samples and android/ios icons. |

### G. Tautological or wrapper-only

| File | Reason |
|---|---|
| `tests/versionConsistency.test.ts` | `check` message is a concatenation of a fixed prefix and `get()`; third `it` re-runs `check --tag v$version`. Canonical logic is `scripts/version.mjs`. |
| `tests/currentDocsContract.test.ts` `it("keeps every local link in current authority resolvable")` | Existence of link targets; does not test document meaning. |
| `tests/currentDocsContract.test.ts` discussion templates `toContain("body:")` | YAML form files almost necessarily have `body:`. |
| `tests/v2/shared/appAssets.test.ts` `getSupportedAppIcon(id) === supportedAppIconById[id]` | Getter vs map identity. |

### H. Large dumped-together **test** files this task might split **if already touching**

| File | Lines / bytes | What is packed together |
|---|---|---|
| `tests/currentDocsContract.test.ts` | 812 / 30 587 | README prose, CODEOWNERS, GitHub brand, release-note inventory, Windows bridge doc strings, spec protocol strings, installer names, locale trust regexes, screenshot census, link checker, retired-slug scan. Item 5 of this task already names this file. |
| `tests/remainingPlatformSurface.test.ts` | 1707 / 56 879 | Checker unit tests + current-repo raster/source digest seal. Item 7 (untrack binaries) collides with the digest seal in the same file. |
| `tests/releaseWorkflow.test.ts` | 2466 / 91 140 | Release YAML/process contracts; not docs, but the largest Vitest file. Split only if this task edits it. |
| `tests/windowsNsisContract.test.ts` | 2233 / 85 731 | NSIS installer contract; same caveat. |
| `tests/miseTaskContract.test.ts` | 1374 / 47 363 | Live mise CLI vs task metadata; overlaps `taskDocs` conceptually (task names in docs vs task runner). Host-mise suite, excluded from CI unit job. |
| `tests/v2/features/featurePages.test.tsx` | 1481 / 53 889 | MCP + Skills page RTL coverage in one file. Not a doc contract; split only if the file is already in the write set. |

### I. Large dumped-together **product** modules — out of this task unless already in the write set

These exist and are "everything in one file", but they are not implied by items 5/7 (tests + tracked binaries):

| File | Lines |
|---|---|
| `src-tauri/src/services/proxy.rs` | 7401 |
| `src-tauri/src/services/skill.rs` | 6909 |
| `src-tauri/src/services/provider/mod.rs` | 6704 |
| `src-tauri/src/codex_config.rs` | 6109 |
| `src-tauri/src/commands/misc.rs` | 5554 |

`provider/mod.rs` already has `mod endpoints/gemini_auth/live/usage` but still holds thousands of lines in the parent.

### Related Specs

- `.trellis/spec/backend/task-runner-contract.md` — names `docs-contract-check.mjs` and `taskDocs.test.ts` as required
- `.trellis/spec/backend/github-ci-workflow.md` — frontend excludes four host-mise suites; contracts job owns static contracts
- `.trellis/spec/frontend/v2-skills-mcp.md` — mentions reviewed byte-for-byte assets (relates to `appAssets` / raster JSON)

## Caveats / Not Found

- Candidate ≠ unused. Several "duplicate" layers are specified as defense-in-depth (`github-ci-workflow.md` says contracts job re-runs the supported-platform checker "as defense in depth").
- `tests/repositoryGovernanceScan.test.ts` uses **synthetic** git fixtures; it does not scan this repo's docs. Not a docs-contract candidate.
- `tests/utils/deepClone.test.ts` is a small behavioral unit test, not a template contract.
- Preset tests often pin **product** env values (context windows); they are repetitive in shape but not empty tautologies.
- No test files were deleted or edited in this research pass.
