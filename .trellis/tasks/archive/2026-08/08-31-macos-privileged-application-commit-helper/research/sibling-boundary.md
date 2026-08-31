# Sibling task boundary — install policy vs privileged helper

Date: 2026-08-31
Baseline commit: `1d0aeecc5b4cff9dc914907f24a7ed321daff75b`

## Why this file exists

`08-31-macos-agent-directory-install-policy` is the next serial product task.
It owns Agent directory ordering, domestic install-only policy, Claude/OpenCode
lifecycle surfaces, and Claude Desktop source. This helper task must not
implement those product surfaces, or the next task will overwrite the work.

The predecessor `08-31-macos-agent-install-update-experience` is archived. Its
helper-facing contracts are settled: download, DMG, identity, inventory,
opaque targets, user-scope transactions, and `/Applications` remaining
`authorization_required`.

## This task owns

- Closed macOS privileged helper: Blessed/SMJobBless, SecureXPC, Authorized.
- Known-application root commit/rollback/recovery for existing desktop
  identities already in the backend (Codex, OpenCode Desktop, QoderWork,
  TRAE Work, WorkBuddy).
- Crate-private `MacSystemCommitPort` and the C ABI Swift bridge.
- Generated product/target-slot policy from the current backend identity
  owners. Claude Desktop is **not** added here; the generator must stay
  extensible so the install-policy task can add a row later.
- Helper status / ensure / remove backend APIs.
- Distinct helper reason codes on existing Agent/Codex job contracts.
- Nested helper/client inside-out signing and verifiers.
- Production `/Applications` actions stay disabled until formal signed
  notarized HIL. Missing HIL is an explicit blocker, not a silent enablement.

## This task does not own

- Agent directory scan-driven ordering or domestic priority metadata.
- QoderWork / TRAE Work / WorkBuddy install-only (no FyAgent update) policy.
- Removing OpenCode/Claude CLI Agent install surfaces.
- Claude Desktop managed source, mirror manifest, or `Claude.app` identity.
- Enabling production system destinations as “install complete” for those
  products.
- A new Agent Settings product page or directory UI for helper install.
- Windows helper, Windows installer, or Codex user-scope permission fallback
  behavior outside the disabled system-commit capability.

## Handoff contract for the install-policy task

```text
existing managed DMG / Codex transaction
  -> prepared source directory FD
  -> MacSystemCommitPort.commit_known_application(...)
  -> authoritative inventory readback
```

Until HIL passes, `MacSystemCommitPort` reports the helper as not production
ready and inventory continues to project `authorization_required` for
`MacSystemApplications`. The install-policy task may complete user-scope
install/update/order work against that disabled system target.
