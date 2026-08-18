# Backend Development Guidelines

This layer records evidence-based backend review guidance for the Rust/Tauri
host. It is optional AI-assistance material: code, configuration, tests, and
workflows remain executable authority, while maintained explanations live
under `docs/fyagent/development/`. These notes, Trellis tasks, and Trellis CLI
state are not prerequisites for contribution, build, check, CI, or release.

Archived tasks and Git history are historical evidence only. When behavior
changes, update its implementation, executable tests, and maintained
developer-facing explanation. Refresh the relevant note when it remains useful
to future automated review, but never use a note to override observed behavior.

## Pre-Development Checklist

When using these notes before changing Rust/Tauri host code:

1. Locate the nearest topic below; do not duplicate a rule already enforced by
   an existing installer, version, security, release, or platform test.
2. For a Tauri command, serialized DTO, event, or persisted-data change, also
   read the [Frontend Development Guidelines](../frontend/index.md) and its
   [Type Safety](../frontend/type-safety.md) boundary before changing either
   side.
3. For user files, credentials, deep links, process control, installers, or
   release artifacts, identify the validation/error case and the executable
   test or matching native evidence that will prove it before editing code.
4. Run local commands through the shared
   [Development Environment Contract](./development-environment.md); do not
   substitute a machine-global Node, Rust, or pnpm toolchain, and never select
   a non-host OS/architecture locally. Native compile/test entrypoints must use
   their guarded mise task (or the guarded `pnpm dev`/`pnpm build` alias), not
   the low-level `pnpm tauri` maintenance/Actions leaf.
5. Preserve protocol and schema versions as protocol facts, but never infer the
   application version or current behavior from an archived design label.

## Guidelines

| Guide                                                                     | Use it for                                                                                                                                                                                                                         |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md)                   | Fixed-source installer service, IPC DTOs/job events, frozen Shell-user package ownership, protected ProgramData PackageBridge, current-user helper admission, A1's unverified native-runtime boundary, and trusted restart/launch. |
| [Agent Install](./agent-install.md)                                       | Six-agent G1 install chain: registry, four-layer contract, snapshot-only start, helper second verb, and post-install probe. Distinct from Codex Desktop MSIX.                                                                      |
| [Codex Provider Configuration](./codex-provider-configuration.md)         | Lossless Codex Provider TOML, native capabilities, vendor/session projection, warnings, and live-config change evidence.                                                                                                           |
| [WorkBuddy Configuration](./workbuddy-configuration.md)                   | WorkBuddy model discovery, restricted third-party `/v1` access, credential-safe persistence, and renderer-domain isolation.                                                                                                        |
| [Application Version and Installer Assets](./fyagent-version-contract.md) | Cargo version single source, version commands, frozen release values, exact cross-platform asset names, and evidence sets.                                                                                                         |
| [GitHub CI Workflow](./github-ci-workflow.md)                             | Repository-owned change classification, domain-aware PR/merge-group jobs, full dev/main pushes, and the stable `CI / Required` aggregate.                                                                                          |
| [GitHub Release Workflow](./github-release-workflow.md)                   | Exact dev-HEAD/tag/successful-push-CI identity, full preflight/formal topology, asset transaction, attestation, and public Release.                                                                                                |
| [Windows Installer](./windows-installer.md)                               | NSIS bundle, install-path behavior, bounded legacy/staging cleanup, explicit PackageBridge non-ownership, per-asset signing policy, x64/ARM64 native build/package, and lifecycle diagnostics.                                     |
| [Windows Runtime Security](./windows-runtime-security.md)                 | Frozen Explorer-user identity and paths, Alice-owned Tauri state, untrusted single-instance input, elevated CLI boundary, and separation from the retired ProgramData runtime.                                                     |
| [Development Environment](./development-environment.md)                   | Locked mise-first local tool versions, host-native compiler/runner/linker boundary, and WSL PATH isolation.                                                                                                                        |
| [Repository Task Runner](./task-runner-contract.md)                       | Canonical mise task metadata, argv transport, DAG effects, maintenance safety, and generated task documentation.                                                                                                                   |
| [Optional Codex Development Hooks](./development-hooks.md)                | Upstream prompt-assistance registration, behavior limits, and explicitly accepted hardening regressions.                                                                                                                           |
| [Application Brand Assets](./application-brand-assets.md)                 | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                                                                                                                       |
| [Application Identity](./application-identity.md)                         | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                                                                                                                                     |
| [CC Switch Upstream Synchronization](./upstream-sync.md)                  | Immutable upstream tag verification, two-parent merge ancestry, conflict precedence, and provenance boundaries.                                                                                                                    |
| [Deep-Link Import Security](./deeplink-import-security.md)                | Untrusted `fyagent://v1/import` request validation, explicit provider activation approval, and credential-safe confirmation.                                                                                                       |

## Quality Check

Use each affected note's **Tests Required** section as a checklist, then verify
the current task metadata and implementation because the note is not itself a
gate. For ordinary Rust/Tauri changes, the baseline local checks are:

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
```

Add the applicable renderer, version, Windows, macOS, or release contract
checks rather than reporting an unrelated local command as platform or release
evidence. Canonical local native commands verify and pin only the exact host,
architecture, and behavior they execute; matching Actions jobs likewise prove
only their executed scope. A changed durable behavior must have executable
evidence; successful static checks or Windows-target compilation checks never
prove native runtime compatibility, a native package, artifact attestation, or
a remotely published Release. This delivery does not run Windows HIL locally or
in Actions, so its Windows 10/11, x64/ARM64, Bob/Alice, PackageManager, file-URI,
ACL, and cleanup behavior remains an explicit residual risk.
