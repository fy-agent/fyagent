# Agent lifecycle reliability program — final integration report

> Review date: 2026-08-30  
> Integration branch: `dev/laiyongjie`  
> Scope: installation target authority, macOS/Windows deployment, Agent Auth verification, and V2 frontend reliability.

## Verdict

The five engineering stages are implemented and archived in dependency order. The integrated branch passed the repository's full current-host prearchive gate, the four-viewport V2 browser matrix, and an exact-head GitHub Full CI run at Stage 5 archive commit `1f308459cf5782afeefafccfeff1e8fc092d3e7c` (run `33310186138`).

The program is therefore complete as an **engineering implementation and integration review**. It is not represented as native release certification. Real Windows UAC/signing/Explorer-user installation, real macOS application replacement/rollback, installed WebView behavior, and real vendor-account Auth remain HIL evidence gaps and are listed explicitly below.

## Dependency and archive audit

| Stage | Archived task | Integration result |
|---|---|---|
| 1 | `.trellis/tasks/archive/2026-08/08-29-agent-install-target-authority` | Candidate identity, scope, target revision, ambiguity handling, and closed IPC are the authority used by later desktop actions. |
| 2 | `.trellis/tasks/archive/2026-08/08-29-macos-agent-in-place-update` | Managed updates preserve the selected bundle path/scope, use staged replacement, verify readback, and restore the old bundle on post-commit failure. |
| 3 | `.trellis/tasks/archive/2026-08/08-29-windows-agent-discovery-install` | Registry/App Paths/MSIX/known-path evidence is normalized into inventory; installer execution is verified, bounded, elevation-aware, and followed by authoritative readback. |
| 4 | `.trellis/tasks/archive/2026-08/08-29-agent-auth-verification-state-machine` | Login handoff is no longer success. Claude/OpenCode use authoritative before/after observations; Grok/desktop products remain handoff-only; Codex remains Auth Center owned. Active sessions can be rediscovered after renderer reload without persisting commands, paths, URLs, or credentials. |
| 5 | `.trellis/tasks/archive/2026-08/08-29-frontend-reliability-architecture` | Navigation selection, shared tabs, route lazy-loading, query ownership, keep-alive behavior, shared assignment surfaces, noop controls, warning gates, and route-chunk budgets are governed and tested. |

The audit verified that each child `task.json` is `completed` with a completion timestamp, and that no child remains in the active task directory.

## Cross-stage acceptance matrix

| Acceptance concern | Evidence and disposition |
|---|---|
| Candidate selection is authoritative | **Pass.** Desktop actions consume the Stage 1 inventory/target revision contract rather than accepting renderer paths or choosing the first installation. |
| macOS update does not silently change scope | **Pass in current-host tests/contracts.** The selected bundle path remains the update target; permission fallback applies only to a fresh install, not an in-place update. Native signed-app HIL remains pending. |
| macOS rollback is verified | **Pass in current-host tests/contracts.** Post-commit failures restore and re-observe the previous bundle before reporting recovery. Native Finder/running-app HIL remains pending. |
| Windows discovery combines supported authorities | **Pass in portable and Windows CI contracts.** Registry, App Paths, MSIX and bounded known paths contribute typed evidence; stale/untrusted evidence remains visible but non-actionable. |
| Windows install is not inferred from process exit | **Pass in implementation/contracts.** Source identity, signer/product checks, helper boundary, elevation outcome and fresh post-install inventory readback are required. Real UAC and signed vendor-installer HIL remain pending. |
| Auth handoff is not login success | **Pass.** Verified terminal states require authoritative account/provider change; unsupported observers are visibly `handoff_only` or `unavailable`. |
| Auth survives renderer reload safely | **Pass for process-local recovery.** The renderer asks the backend for the active session by canonical Agent ID and resumes polling. No path, command, URL, credential, account identifier or session token is persisted in renderer storage. Full backend-process restart recovery is intentionally not claimed. |
| Codex does not gain a second OAuth implementation | **Pass.** Codex Auth continues to delegate to the existing Auth Center. |
| Frontend selected state remains truthful | **Pass.** Route, hash, selected link, `aria-current`, SelectionLens, expanded groups and keyboard focus share one route-derived authority. |
| Tabs and assignment UI are shared components | **Pass.** Radix-based shared tabs and common assignment surfaces replace page-local state/markup duplication. |
| Routes are lazy and bounded | **Pass.** Six routes are lazy-loaded; the route-chunk verifier is owned by CI classification and enforces the reviewed initial/chunk budgets. |
| Query lifecycle is owned and bounded | **Pass.** Query keys, invalidation and enabled lifecycles live in shared feature/query boundaries rather than component-local fetch effects. |
| No visible noop actions | **Pass.** Unsupported actions are absent/disabled with explicit state instead of shipping clickable controls that do nothing. |
| Warning regressions are observable | **Pass.** The focused React warning gate, full frontend tests and browser matrix passed. Existing deliberately exercised error/warning output is not reclassified as product success. |

## Integrated automated evidence

- The final parent-task prearchive gate passed on 2026-08-30 after excluding only the exact active parent task record; its supported-platform scan covered 2,332 current files.
- Rust workspace: formatting, check, Clippy with warnings denied, and tests passed; the integrated Stage 5 run recorded 2,945 passed tests and 5 explicitly ignored platform/HIL fixtures.
- Frontend baseline: 172 test files and 1,536 tests passed, with one intentional skip.
- V2 focused suite: 58 files and 417 tests passed.
- V2 Chromium geometry/interaction matrix: 140/140 passed across 900×600, 1152×640, 1232×700, and 1440×900.
- Repository/release contracts: 575 tests passed with one intentional skip.
- Desktop mock contract: 7/7 passed; visual preflight reported ready for candidate capture.
- The earlier Stage 5 archive-head gate covered 2,321 current files; the final parent-task gate covered 2,332 after adding the integration artifacts.
- Stage 5 exact-head Full CI: `33310186138`, head `1f308459cf5782afeefafccfeff1e8fc092d3e7c`, completed successfully before this parent review began.

The parent review also re-read issues `#31`, `#47`, `#101`, and `#141` from GitHub on 2026-08-30. This report does not mechanically close or reinterpret those issues; issue closure remains contingent on their own current acceptance and HIL evidence.

## Evidence gaps and release gates

The following are **not executed** by this program session and must not be inferred from mock, portable, cross-compile, or browser results:

1. A signed macOS candidate replacing a real `/Applications` bundle, handling a running app, exercising permission denial, restoring after a forced post-commit fault, and passing launch/readback on the installed WebView.
2. Windows x64 and ARM64 signed candidates exercising Registry/App Paths/MSIX discovery, user/system scope, UAC consent/cancel/timeout, signer/product mismatch, installer reboot/exit codes, Explorer-user authority, and fresh post-install readback.
3. Disposable-account Auth HIL for Claude and OpenCode before/after verification, Grok handoff behavior, at least one desktop Agent handoff, and Codex Auth Center delegation on both supported desktop operating systems.
4. A complete renderer/backend process restart during an active Auth operation. The implemented recovery contract covers renderer reload while the backend process and process-local session store remain alive.

These gaps are release-candidate validation work, not hidden implementation success. A release decision should remain conservative until the relevant HIL rows are recorded against an exact signed candidate.

## Final integration decision

- Engineering stages implemented: **5/5**
- Child tasks archived: **5/5**
- Dependency order respected: **yes**
- Integrated portable/current-host gates: **pass**
- Exact Stage 5 archive-head Full CI: **pass**
- Native Windows/macOS/Auth HIL: **not complete**
- Program disposition: **archive the engineering program with explicit HIL residuals; do not label it native release-certified**

