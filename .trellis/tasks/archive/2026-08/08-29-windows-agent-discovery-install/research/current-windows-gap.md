# Pre-implementation Windows gap review

This note records the baseline observed before Stage 3 implementation. The
completed design and current executable contracts supersede the gaps below;
do not use the former `windows_exe_unavailable` behavior as current authority.

## Required full-contract reads

Before implementation, read the complete `.trellis/spec/backend/external-agent-p0.md` and `.trellis/spec/frontend/v2-agent-models.md`. They are intentionally omitted from automatic JSONL injection because each exceeds the configured context-file size limit.

## Current discovery

`src-tauri/src/agent_install/desktop.rs` currently checks:

- `%LOCALAPPDATA%/Programs`;
- machine Program Files roots;
- a short product-specific list of relative EXE paths;
- bounded first/last PE byte windows for UTF-16 `ProductName`/`ProductVersion` strings.

It does not read Uninstall registry, App Paths or PackageManager for the generic desktop products. It returns only one bool/version and therefore cannot represent parallel or stale installations.

## Current execution

Windows EXE sources resolve, but `windows_exe_unavailable` causes download/action admission to return `InteractiveUserUnavailable`. Unit tests explicitly assert that EXE installation is not started.

The project already has stronger Windows infrastructure under Codex Desktop and platform/runtime owners:

- frozen Explorer interactive-user context;
- link-safe fixed registry traversal for Environment/Run;
- fixed FyAgent user helper with nonce/SID/image/pipe/bridge checks;
- retained package bridge/pin;
- PackageManager inventory/install and AUMID launch;
- trusted EXE launch for already verified applications.

Stage 3 should extend these owners, not create a second generic ShellExecute/helper stack.

## Product-source observations

- Qoder resolver currently pins `QoderWorkCN-Setup-User-x64.exe`; Windows arm64 is explicitly unsupported.
- WorkBuddy and TRAE resolve EXE sources, but the generic executor is disabled.
- WorkBuddy's official Windows guide presents an interactive destination chooser.

Conclusion: inventory can be shared immediately; execution must remain product/format/architecture gated and honest about vendor UI.
