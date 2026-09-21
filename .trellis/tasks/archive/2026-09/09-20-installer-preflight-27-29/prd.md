# Installer preflight and user-scoped recovery (#27/#29)

## Goal

Before downloading supported Agent software, show the selected software, version or channel, destination, relevant host requirements and intended change. Confirmation executes that same target. Preserve already completed work and actionable recovery when installation fails.

## Authorization and baseline

The user authorized implementation and the root delegated this package. This task starts from `2c09c4be2c5f50b4060fd6f7a13a7be31c364c54` on `codex/next-installer-27-29` in the isolated installer checkout. Parent task: `09-20-next-iteration-engineering` in the integration checkout. No additional approval gate applies.

## Requirements

- Reuse the inventory capability/revision and existing lifecycle/Codex job owners.
- Check only platform, architecture, disk capacity, required write locations and runtime needed by the selected installation.
- A changed/expired target or release requires a refreshed confirmation, never an automatic target substitution.
- Keep Windows Explorer-user/helper, native UAC, closed command/product/target and macOS production helper gates intact.
- Retain final job outcome and recovery guidance. A possibly active/partially completed installation cannot be silently retried.
- Use fixture or temporary-directory tests; do not install, elevate or alter real user Agent state.

## Acceptance

Focused native/renderer tests prove blocked preflight performs no download, confirmation carries the selected binding unchanged, cancellation/retry reuse current jobs, and recovery-required outcomes preserve evidence. Document the audit of supported user/helper routes and exact native Windows/macOS checks still outstanding.

## Exclusions

Provider/SecretRef/schema/proxy, recommendation logic, release/version workflows, navigation state, real installation/UAT and issue closure are owned elsewhere.
