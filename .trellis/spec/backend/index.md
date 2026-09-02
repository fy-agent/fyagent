# Backend Development Guidelines

This directory contains the current Rust host, persistence, native-platform,
delivery, and repository-governance contracts. This index is a discovery
surface, not a second source of behavior. Exact signatures, state machines,
error matrices, tests, paths, and security rules belong in the linked owner.

## Reading order

1. Start with [Rust Host Modular Boundaries](./modular-boundaries.md) and
   [Backend Reuse](./reuse.md).
2. Read [Development Environment](./development-environment.md),
   [Repository Task Runner](./task-runner-contract.md), and
   [Database Persistence](./database-persistence.md) when changing shared
   runtime or repository infrastructure.
3. Read the focused feature contract that owns the data, filesystem, process,
   network, secret, or IPC behavior being changed.
4. For platform or delivery work, also read the matching Windows/macOS and
   CI/release governance contracts.

## Core architecture and repository infrastructure

| Contract | Owns |
| --- | --- |
| [Rust Host Modular Boundaries](./modular-boundaries.md) | Module responsibilities, dependency direction, command/service/platform separation, and top-level composition. |
| [Backend Reuse](./reuse.md) | Existing-owner, adopted-dependency, open-source, adapter, and bespoke implementation order. |
| [Development Environment](./development-environment.md) | Toolchain authority, bootstrap, host support, locks, optional macOS Windows-MSVC diagnostics, and environment verification. |
| [Optional Codex Development Hooks](./development-hooks.md) | Optional Codex hook files, timeout/failure behavior, and Trellis-version ownership. |
| [Repository Task Runner](./task-runner-contract.md) | Public `mise run` API, effects, parameter transport, host guards, mutation policy, and platform diagnostics. |
| [Database Persistence](./database-persistence.md) | SQLite path, schema version, startup lifecycle, migrations, import/backup/restore, DAO placement, and transactional boundaries. |
| [Application Identity](./application-identity.md) | Product names, identifiers, license/provenance identity, and migration boundaries. |
| [Application Brand Assets](./application-brand-assets.md) | Canonical icons, asset derivation, platform packaging, and byte-level validation. |
| [Application Version and Installer Assets](./fyagent-version-contract.md) | Canonical version source, package versions, and installer filename contract. |
| [Main Window Layout](./main-window-layout.md) | Native geometry, maximize/work-area behavior, and renderer chrome boundary. |

## Product, configuration, and runtime security

| Contract | Owns |
| --- | --- |
| [SecretRef Native Backend](./secretref-backend.md) | Secret storage, opaque references, DTO redaction, and native evidence. |
| [Deep-Link Import Security](./deeplink-import-security.md) | Untrusted deep-link parsing, confirmation, import capabilities, and side-effect limits. |
| [Change Plan Typed Executor](./change-plan-executor.md) | Typed plans, idempotency, execution phases, compensation, and partial results. |
| [Codex Provider Configuration](./codex-provider-configuration.md) | Codex provider/auth projection, writer serialization, backup, rollback, and readback. |
| [One-click Executable Software Installer](./codex-desktop-installer.md) | Codex desktop discovery/install/update, PackageBridge/helper, signing, and transaction safety. |
| [Codex Session Usage Sync](./codex-session-usage.md) | Codex JSONL usage import, typed deferred reasons, retry/fingerprint separation, and bounded logging. |
| [WorkBuddy Configuration](./workbuddy-configuration.md) | Revisioned WorkBuddy model/config writes, overwrite capabilities, backup, and reread. |
| [External Agent Catalog and Runtime](./external-agent-catalog-runtime.md) | Static Agent catalog, capability/evidence projection, runtime observation, trusted launch, and ACL. |
| [External Agent Lifecycle](./external-agent-lifecycle.md) | Readiness, inventory, opaque targets, install/update/launch jobs, source verification, and recovery. |
| [External Agent Auth](./external-agent-auth.md) | Login/logout/provider observation, Auth sessions, desktop target binding, and handoff semantics. |
| [QoderWork Hooks Configuration](./qoderwork-hooks.md) | QoderWork Hooks snapshot, revisioned writes, allowed hooks, backup, and reread. |
| [External Agent Model Integration](./external-agent-models.md) | TRAE model preflight/observation and OpenCode model persistence. |
| [Skill Management](./skill-management.md) | Skill SSOT, discovery, install/update/uninstall, backups, imports, archive safety, and target assignment. |
| [MCP Management](./mcp-management.md) | MCP CRUD, validation, import, assignment, database state, and vendor live-file projection. |
| [Proxy Runtime and Control](./proxy-runtime.md) | Tauri commands, listener lifecycle, takeover, provider switching, crash recovery, and breaker administration. |
| [Local Proxy Request Pipeline](./local-proxy-pipeline.md) | Listener admission, request routing, permits, provider transforms, retry/streaming, and usage recording. |

## Native platforms and distribution

| Contract | Owns |
| --- | --- |
| [Windows Shell-user Runtime](./windows-runtime-security.md) | Explorer-user authority, per-user paths/HKU, registry masks, single-instance input, COM launch, and helper boundary. |
| [Windows Installer](./windows-installer.md) | NSIS mechanics, bounded cleanup, signing evidence, uninstall ownership, and native diagnostics. |
| [macOS Privileged System-Commit Helper](./macos-system-commit.md) | Blessed helper, C ABI, product/slot integers, `MacSystemCommitPort`, and production enablement gates. |
| [macOS Styled DMG Layout](./macos-dmg-layout.md) | DMG contents, Finder metadata, retries, byte preservation, and layout verification. |
| [GitHub CI Workflow](./github-ci-workflow.md) | Change classification, domain jobs, required aggregation, runner/toolchain evidence, and failure semantics. |
| [GitHub Release Workflow](./github-release-workflow.md) | Release identity, native builds, signing/notarization, assets, attestation, draft recovery, and publication. |
| [GitHub Merge Governance](./github-merge-governance.md) | Merge Queue, merge method, task/spec lifecycle, and merge-readiness governance. |
| [CC Switch Upstream Synchronization](./upstream-sync.md) | Immutable upstream identity, ancestry-preserving merge, conflict precedence, and provenance handoff. |

## Compatibility entry points

- [External Agent P0](./external-agent-p0.md) routes archived broad Agent,
  Skill, and MCP references to focused owners.
- [External Agent Configuration](./external-agent-configuration.md) routes
  archived combined Qoder/TRAE/OpenCode configuration references.

New task context must cite the smallest focused contract above.

## Maintenance rules

- Keep stable current contracts here. Put investigation logs, migration dates,
  run IDs, reviewed commit hashes, and one-time evidence in Trellis tasks,
  provenance ledgers, CHANGELOG, or Git history.
- Do not copy tool versions from repository authority files into generic specs.
  Contracts may require equality with those sources without duplicating them.
- When two specs touch one operation, name one semantic owner and make the
  other describe only its boundary or orchestration role.
- High-risk native/security/release contracts may remain long when splitting
  would hide fail-closed ordering, rollback, or evidence requirements.

## Quality Check

Use every affected contract's **Tests Required** section. The standard local
implementation gate is `mise run check`; focused backend work may start with
`mise run check:backend`, while docs/task/version/release contract changes use
`mise run check:contracts`. Before archiving an active Trellis task, run the
matching prearchive gate with its exact `--exclude-active-task` path. Portable
checks never replace native runtime, installer, signing, notarization, or
publication evidence required by the owning contract.
