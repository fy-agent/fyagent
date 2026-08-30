# Implement — Agent lifecycle reliability program

## 1. Baseline and orchestration

- [x] Re-read issues `#31`, `#47`, `#101`, and `#141` at final integration time rather than relying on stale planning assumptions.
- [x] Verify all five child tasks are `completed`, have completion timestamps, and exist under `.trellis/tasks/archive/2026-08/`.
- [x] Preserve the actual single-branch serial integration model on `dev/laiyongjie`; no fictional child-branch merge history is recorded.
- [x] Confirm Stage 1 preceded and supplied the hard authority dependency for Stages 2, 3, and the desktop portion of Stage 4.
- [x] Confirm Stage 5 final installation/Auth wiring landed only after the corresponding backend contracts were available.

## 2. Cross-stage contract review

- [x] Recheck that renderer input never selects installation targets by path or first-match heuristics.
- [x] Recheck that macOS updates preserve the selected bundle path/scope and perform verified rollback after post-commit failure.
- [x] Recheck that Windows discovery aggregates Registry, App Paths, MSIX and bounded known-path evidence without promoting stale/untrusted evidence.
- [x] Recheck that Windows installer success requires verified package/executor boundaries and authoritative post-install inventory readback rather than process exit alone.
- [x] Recheck that Agent Auth launch/handoff is never serialized as verified login success.
- [x] Recheck Claude/OpenCode before/after verification, Grok/desktop handoff-only behavior, and Codex Auth Center ownership.
- [x] Add process-local active-session discovery so a renderer reload can recover the canonical in-flight Auth session without persisting commands, paths, URLs, credentials, or account identifiers.
- [x] Recheck selected navigation, Radix Tabs, lazy routes, query ownership, keep-alive, shared assignment surfaces, noop controls, warning gates and route chunk budgets.

## 3. Integrated validation

- [x] Run the full current-host repository gate with only the active parent Trellis record excluded.
- [x] Run Rust format/check/Clippy/test gates and retain ignored native fixtures as explicit HIL, not pass evidence.
- [x] Run the complete frontend and V2 suites.
- [x] Run the four-viewport Chromium V2 matrix.
- [x] Run desktop mock/visual preflight, supported-platform, ACL, task, lock, release and repository-governance gates.
- [x] Verify the Stage 5 archive commit with exact-head GitHub Full CI (`33310186138` at `1f308459cf5782afeefafccfeff1e8fc092d3e7c`).
- [x] Produce `final-report.md` with a cross-child acceptance matrix, exact automated evidence and conservative release disposition.

## 4. Archival

- [x] Keep native Windows/macOS/Auth HIL gaps visible in the final report rather than weakening acceptance language or inventing success.
- [ ] Execute signed-candidate macOS replacement/rollback/launch HIL.
- [ ] Execute signed-candidate Windows x64/ARM64 discovery/UAC/signing/install/readback HIL.
- [ ] Execute disposable-account Claude/OpenCode/Grok/desktop/Codex Auth HIL on both supported desktop systems.
- [ ] Execute a full backend-process restart during an active Auth operation; current recovery is intentionally process-local and renderer-reload scoped.
- [x] Define archival as: pass the parent prearchive gate, archive and push, then verify the immutable archive head with an external exact-head Full CI run. The run is not embedded into the commit it verifies.

## Validation commands

```bash
TRELLIS_CONTEXT_ID=chatgpt-parent-20260830 \
  mise run check:prearchive \
  --exclude-active-task .trellis/tasks/08-29-agent-lifecycle-reliability-program

mise run test:v2:browser
```

Native HIL must use exact signed candidates and disposable/test accounts where permitted. Evidence may include bounded state transitions, stable codes and redacted screenshots/logs, but never credentials, account identifiers, raw auth files, private paths or unbounded command output.
