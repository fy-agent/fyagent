# Design — Windows vendor installer handoff

## Boundary

Change only the Windows EXE install **settlement** for QoderWork / TRAE Work / WorkBuddy.

Keep: source resolve, download, signature/product admission, PackageBridge, helper CLI, Alice `ShellExecute`, job single-flight, macOS DMG verify.

Do not change: Windows discovery/ProductName roots, Claude/OpenCode, Grok CLI, Codex desktop installer, silent switches.

## Current vs target

Today the helper `ShellExecuteExW`s the bridged EXE, then `WaitForSingleObject` up to 30 minutes. The parent then polls inventory for up to 90 seconds. Missing closed-path identity becomes `incomplete` / verification failure even when the vendor wizard ran.

Target: `ShellExecute` success is the handoff. Helper sends progress (parent `awaiting_user`) and returns `Success`. Parent maps helper `Ok` to job `succeeded` and skips `wait_for_windows_deployment`. Bridge cleanup may leave an in-use orphan; that must not replace a successful launch.

## Contracts

- Helper: launch fail / `ERROR_CANCELLED` unchanged. Missing process handle after a successful `ShellExecute` is still handoff success (do not wait). Do not kill the vendor process.
- Parent: `Invoked(Ok)` → `Succeeded`. `InstallerUserCancelled` → `Cancelled`. Other helper errors keep existing mappings. Do not transition to `VerifyingInstallation` on Windows.
- Parent helper deadline can shrink from 31 minutes to a launch/UAC bound (5 minutes); it no longer covers the full wizard.
- macOS `run_desktop_install_job` path is untouched.

## Tests

- Helper: document/assert the launch function no longer waits (source/contract test if the wait loop cannot be HIL-tested).
- Parent: unit-test the Windows settle mapping (`Ok` → succeeded, cancel → cancelled, launch mapped failures unchanged).
- Keep `verify_windows_deployment*` tests; they remain the comparison helper, just unused by the Windows job.
- Frontend copy tests stay green unless Windows success copy changes; do not claim the product is installed.

## Rollback

Revert helper wait removal and restore `wait_for_windows_deployment` after helper success.
