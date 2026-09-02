# Execution context

## Mission

```text
Desktop: QoderWork, TRAE Work, WorkBuddy, Codex, Claude Desktop, OpenCode Desktop
CLI:     Grok Build only
```

Complete real Windows lifecycle parity with one cross-platform policy. macOS scope is removal of stale non-Grok installer surfaces plus regression of the working Desktop chain.

## Highest-priority findings

1. The lifecycle matrix already exists; do not create a Windows product table.
2. Windows release is elevated, while user-scoped state belongs to the frozen Explorer user. Grok formal support must use the existing closed user-helper.
3. Qoder/TRAE/WorkBuddy have code but incomplete native HIL; make evidence-driven minimal fixes and keep update disabled.
4. Claude/OpenCode Windows descriptors, sources and destinations are absent/unsupported.
5. Settings/Tooling still exposes non-Grok npm/Shell/PowerShell installers; these must retire on both OSes while required read-only CLI/config support remains.
6. Current Codex exact identity may be transitional because Codex moves into the new ChatGPT desktop app; verify, never name-match.
7. Codex log spam comes from deleting retryable pending state before reinsertion and from steady deferred INFO summaries.

## Reuse decisions

- Packaged apps -> existing Codex PackageManager/windows-rs capability, narrow extraction/delegation only.
- Signed EXEs -> existing Agent verifier + Explorer user helper + post-readback.
- Grok CLI -> existing hardened Grok owner + closed ordinary-user helper.
- Download/job/progress -> current shared owners.
- WinGet -> optional per-product review, never baseline/generic.
- Store link -> manual fallback unless exact PackageManager deployment is proven.
- New runtime dependency -> default no.

## Execution order

1. Re-audit live baseline, official artifacts and formal-vs-dev Windows failures.
2. Lock the policy and failing tests.
3. Retire all non-Grok Tooling install/update/manual-command surfaces.
4. Route Grok through the existing formal-Windows user-helper.
5. Consolidate only the minimum packaged-app capability.
6. Complete authoritative Windows inventory.
7. HIL/fix Qoder, TRAE and WorkBuddy without enabling update.
8. Add Claude Windows through one evidence-proven owner.
9. Add OpenCode Windows through the signed EXE owner.
10. Verify/migrate exact ChatGPT/Codex/Classic identities only when proven.
11. Fix deferred retry/log state.
12. Frontend/macOS regression, full native HIL, reviews and specs.

## Stop conditions

- Renderer-controlled URL/path/command/package ID appears.
- Generic PowerShell/cmd/WinGet/npm installer or second helper/downloader/job owner appears.
- Elevated process runs user-profile CLI directly or helper failure falls back to admin execution.
- Product identity depends on display/process/window name.
- Signer/package matching is broadened without current artifact evidence.
- ARM64 is claimed without native artifact and native HIL.
- Log suppression advances cursors, disables retry or hides parse/DB/invariant failures.

## Acceptance shorthand

- Only Grok is installable as CLI anywhere in FyAgent.
- Six Desktop products share one Agent lifecycle owner.
- Qoder/TRAE/WorkBuddy remain install-only.
- Real Windows x64 discover/install/allowed-update/launch succeeds only after authoritative readback.
- Claude/OpenCode no longer return Windows unsupported after their evidence gates pass.
- Current/historical Codex/ChatGPT identities are exact and safe.
- Unchanged expected deferred has no repeated WARN/INFO and recovers usage exactly once when parent catches up.
