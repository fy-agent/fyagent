# Backend Development Guidelines

This directory contains the current Rust host, native-platform, delivery, and
repository-governance contracts. This index is a router, not a second source of
behavior. Exact signatures, error matrices, tests, paths, and security rules
belong in the linked owning spec.

## Reading order

1. Start with [Rust Host Modular Boundaries](./modular-boundaries.md) and
   [Backend Reuse](./reuse.md).
2. Read [Development Environment](./development-environment.md) and
   [Repository Task Runner](./task-runner-contract.md) before changing local
   tooling or acceptance commands.
3. Read the feature contract that owns the data, filesystem, process, network,
   secret, or IPC behavior being changed.
4. For platform or delivery work, also read the matching Windows/macOS and
   CI/Release governance contracts.

## Core architecture and repository tooling

| Contract | Owns |
| --- | --- |
| [Rust Host Modular Boundaries](./modular-boundaries.md) | Module responsibilities, dependency direction, command/service/platform separation. |
| [Backend Reuse](./reuse.md) | Existing-owner, adopted-dependency, open-source, adapter, and bespoke implementation order. |
| [Development Environment](./development-environment.md) | Toolchain authority, bootstrap, host support, locks, and environment verification. |
| [Optional Codex Development Hooks](./development-hooks.md) | Optional Codex hook files and Trellis-version ownership. |
| [Repository Task Runner](./task-runner-contract.md) | Public `mise run` API, effects, parameter transport, host guards, and mutation policy. |
| [Application Identity](./application-identity.md) | Product names, identifiers, license/provenance identity, and migration boundaries. |
| [Application Brand Assets](./application-brand-assets.md) | Canonical icons, asset derivation, platform packaging, and byte-level validation. |
| [Application Version and Installer Assets](./fyagent-version-contract.md) | Canonical version source, package versions, and installer filename contract. |
| [Main Window Layout](./main-window-layout.md) | Native geometry, maximize/work-area behavior, and renderer chrome boundary. |

## Product, configuration, and security

| Contract | Owns |
| --- | --- |
| [SecretRef Native Backend](./secretref-backend.md) | Secret storage, opaque references, DTO redaction, and native evidence. |
| [Deep-Link Import Security](./deeplink-import-security.md) | Untrusted deep-link parsing, confirmation, import capabilities, and side-effect limits. |
| [Change Plan Typed Executor](./change-plan-executor.md) | Typed plans, idempotency, execution phases, compensation, and partial results. |
| [Codex Provider Configuration](./codex-provider-configuration.md) | Codex provider/auth projection, writer serialization, backup, rollback, and readback. |
| [One-click Executable Software Installer](./codex-desktop-installer.md) | Codex desktop discovery/install/update, PackageBridge/helper, signing, and transaction safety. |
| [WorkBuddy Configuration](./workbuddy-configuration.md) | Revisioned WorkBuddy model/config writes, overwrite capabilities, backup, and reread. |
| [External Agent P0 Safety](./external-agent-p0.md) | Agent catalog, runtime observation, install/inventory/actions, Auth, Skills/MCP, and vendor boundaries. |

## Native platforms and distribution

| Contract | Owns |
| --- | --- |
| [Windows Shell-user Runtime](./windows-runtime-security.md) | Explorer-user authority, per-user paths/HKU, single-instance input, COM launch, and helper boundary. |
| [Windows Installer](./windows-installer.md) | NSIS mechanics, bounded cleanup, signing evidence, uninstall ownership, and native diagnostics. |
| [macOS Styled DMG Layout](./macos-dmg-layout.md) | DMG contents, Finder metadata, retries, byte preservation, and layout verification. |
| [GitHub CI Workflow](./github-ci-workflow.md) | Change classification, domain jobs, required aggregation, runner/toolchain evidence, and failure semantics. |
| [GitHub Release Workflow](./github-release-workflow.md) | Release identity, native builds, signing/notarization, assets, attestation, draft recovery, and publication. |
| [GitHub Merge Governance](./github-merge-governance.md) | Merge Queue, merge method, task/spec lifecycle, and merge-readiness governance. |
| [CC Switch Upstream Synchronization](./upstream-sync.md) | Immutable upstream identity, ancestry-preserving merge, conflict precedence, and provenance handoff. |

## Maintenance rules

- Keep stable current contracts here. Put one-time investigation results,
  migration dates, run IDs, and reviewed commit hashes in Trellis tasks,
  provenance ledgers, CHANGELOG, or Git history.
- Do not copy tool versions from their repository authority files into generic
  specs. Contracts may require exact equality with those sources.
- When two specs touch the same operation, name one semantic owner and make the
  other describe only its boundary or orchestration role.
- High-risk native/security/release contracts may be long. Prefer explicit
  fail-closed conditions over a short narrative that loses authority or error
  semantics.

## Quality Check

Use every affected contract's **Tests Required** section. The standard local
implementation gate is `mise run check`; focused backend work may start with
`mise run check:backend`, while docs/task/version/release contract changes use
`mise run check:contracts`. Before archiving an active Trellis task, run the
matching prearchive gate with its exact `--exclude-active-task` path. Portable
checks never replace matching native runtime, installer, signing, notarization,
or publication evidence required by the owning contract.
