# Windows vendor installer handoff

## Goal

On Windows, QoderWork CN / TRAE Work CN / WorkBuddy one-click install downloads the reviewed official EXE, verifies it, and opens the vendor installer for the user. FyAgent must not wait for the wizard to finish or treat inventory readback as install success. macOS DMG install stays the managed mount/stage/replace/rollback path.

## Requirements

1. Windows install for the three EXE products still resolves the closed first-party source, streams the artifact, admits it with WinVerifyTrust / closed ProductName / signer leaf, and launches it through the existing `agent-exe-install` helper as Alice. No silent switch, URL, verb, or free argument vector is added.
2. After `ShellExecute` starts the official installer, the helper returns success without waiting for process exit, timeout, or exit code. UAC / launch cancellation remains a cancelled or failed job.
3. The Windows job must not enter `verifying_installation` or require a trusted inventory candidate to appear. Helper success is terminal `succeeded`. The directory card may still show not-installed until a later scan actually observes the closed identity.
4. macOS QoderWork / TRAE / WorkBuddy install is unchanged: DMG transaction, post-commit identity proof, rollback on verification failure.
5. Discovery / ProductName matching is out of scope. Existing installs that the closed path does not admit stay not-installed and keep the install action.

## Acceptance Criteria

- [x] Windows helper `agent-exe-install` succeeds once the official EXE is launched; it does not wait 30 minutes or map a later nonzero exit into job failure.
- [x] Windows `run_windows_desktop_install_job` marks `succeeded` after helper success and never calls post-install inventory proof.
- [x] UAC cancel and ShellExecute launch failure still fail closed with the existing reason codes.
- [x] macOS desktop install job still verifies the deployed bundle before `succeeded`.
- [x] Frontend still shows Windows vendor-UI stages (`launching_installer` / `awaiting_user`) and does not paint those as installed.
- [x] Specs in `external-agent-p0.md` and `v2-agent-models.md` describe Windows handoff vs macOS managed verify.

## Notes

Approved in session: Windows-only; keep official package download rather than a browser page; do not wait or fail because Qoder/WorkBuddy/TRAE was not rediscovered.
