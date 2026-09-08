# Planning decision record

## Decided

1. **One cross-platform product policy.** Six products are Desktop; only Grok Build is CLI.
2. **Retire every non-Grok public CLI installer on both OSes.** Backend actions, UI buttons, copied npm/Shell/PowerShell commands and remote-script instructions must be removed/rejected; read-only CLI/config consumers may remain.
3. **QoderWork, TRAE Work and WorkBuddy remain install-only.** This task cannot enable FyAgent update for them.
4. **Reuse the existing `fyagent-user-helper`.** Formal elevated Windows routes Grok observe/install/update through a closed product/action protocol; no new sidecar/service or elevated fallback.
5. **Package-type reuse.** Proven MSIX/AppX uses the existing Codex PackageManager capability; reviewed desktop EXE uses the existing signed Agent runner.
6. **No generic WinGet or shell installer.** WinGet is at most a product-level optional adapter after exact package/source/scope/architecture/HIL proof.
7. **Authoritative post-readback owns success.** Installer exit, download completion, Store handoff and file existence are insufficient.
8. **Codex/ChatGPT migration stays exact.** Clean install, historical Codex upgrade and ChatGPT Classic coexistence HIL determine any small migration set; never use display/process/window names.
9. **Codex deferred uses typed reason + separate retry/emission state.** Expected lag is not repeated WARN/INFO; true failures remain observable.
10. **Native Windows x64 HIL is an archive gate.** ARM64 claims require current native artifact plus native HIL.
11. **No new runtime dependency by default.** Existing owners and Windows APIs cover the baseline design.

## Phase 0 decisions still open

1. Claude Windows uses official MSIX or official user installer; choose by exact identity, functionality/scope, single update owner, security and native HIL.
2. Grok fresh-install owner is fixed from current official evidence; existing installations preserve their observed official owner.
3. WorkBuddy remains signed EXE unless exact Store package deployment is demonstrably safer and fully HIL-proven.
4. Product-level ARM64 support is decided independently from current release assets and native HIL.
5. Codex package identity changes only if current/historical installation evidence proves the existing exact policy stale.

## Explicitly rejected

- restoring Claude/OpenCode CLI install surfaces;
- leaving duplicate non-Grok Settings/Tooling installers because Agent policy is already desktop-only;
- enabling Qoder/TRAE/WorkBuddy update in this task;
- generic command/PowerShell/npm/WinGet IPC;
- Store page handoff reported as installation;
- automatic Windows optional-feature provisioning or reboot;
- broad signer/package/name matching;
- global WARN suppression or cursor advancement to hide log spam.
