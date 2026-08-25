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
   a non-host OS/architecture locally. Linux is a development host for
   `mise run check` and current-host compile/test; it is not a shipped product
   platform. Native compile/test entrypoints must use
   their guarded mise task (or the guarded `pnpm dev`/`pnpm build` alias), not
   the low-level `pnpm tauri` maintenance/Actions leaf.
5. Preserve protocol and schema versions as protocol facts, but never infer the
   application version or current behavior from an archived design label.
6. For main-window restore, maximize, min-size, or `layout-mode-changed`, read
   the [Main Window Layout Contract](./main-window-layout.md). Host owns
   geometry: do not mutate it while maximized. V2 Overlay is renderer
   app-shell chrome only; do not treat that React chrome or CSS as the
   Windows overflow fix.

## Guidelines

| Guide                                                                     | Use it for                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Codex Desktop Installer](./codex-desktop-installer.md)                   | Fixed-source installer service without package-hash admission or multi-pass full-file SHA, shared renderer DTO/state core, V2 port/events, Shell-user package ownership, protected PackageBridge, trusted restart/launch, and Agent Catalog managed-desktop reuse of the same policy.            |
| [Rust Host Modular Boundaries](./modular-boundaries.md)                   | Modular-monolith visibility/dependency rules, command ownership, private Provider/Skill/Proxy/Codex subdomains, target-gated Tooling imports/tests, and executable architecture checks.                                                  |
| [Codex Provider Configuration](./codex-provider-configuration.md)         | Lossless Codex Provider TOML, targeted V2 Quick Setup live patches with native write-plan/rolling backups, Codex Change Plan admission/apply/readback, official-auth preservation, native capabilities, vendor/session projection, warnings, live-config change evidence, managed ChatGPT `credential_id` vs workspace routing, and file-only native `auth.json` projection. |
| [Change Plan Typed Executor](./change-plan-executor.md)                  | Canonical schema-v20 Change Plan execution: closed typed adapters, five durable phases, plan idempotency, pre-write cancellation, partial truth, event ordering, and readback-only crash recovery.                                      |
| [WorkBuddy Configuration](./workbuddy-configuration.md)                   | WorkBuddy model discovery, legacy-array/object-root shape-preserving persistence, Change Plan save with private overwrite capability, native main/backup path metadata, credential isolation, and renderer-domain isolation.             |
| [External Agent P0 Safety](./external-agent-p0.md)                        | Catalog v4 (Grok Build + TRAE Work CN URL), runtime/launch authority, Skills including disk observation, Qoder Hooks, TRAE GET observation/`state.vscdb`, OpenCode `opencode.json` persist, Agent install/action façade, narrow permissions, and secret boundaries.   |
| [Application Version and Installer Assets](./fyagent-version-contract.md) | Cargo version single source, version commands, frozen release values, exact cross-platform asset names, and evidence sets.                                                                                                               |
| [GitHub CI Workflow](./github-ci-workflow.md)                             | Repository-owned change classification, domain-aware PR/merge-group Required CI, lightweight branch-push commit policy, merge-queue event isolation, explicit Full CI diagnostics, and the stable `CI / Required` aggregate.              |
| [GitHub Merge Governance](./github-merge-governance.md)                  | Merge-commit mainline policy, first-parent PR boundaries, Trellis merge-ready lifecycle, exact-head Auto-merge handoff, Merge Queue latest-main validation, upstream ancestry preservation, and post-merge `dev/laiyongjie` sync.       |
| [SecretRef Native Backend](./secretref-backend.md)                       | Opaque SecretRef/version identities, zeroizing material, macOS Data Protection Keychain, Windows Credential Manager semantics, source-free errors, and matching-host native CRUD evidence.                                                |
| [GitHub Release Workflow](./github-release-workflow.md)                   | Tag-target formal identity (annotated or lightweight), no live-main freeze or prior push CI gate, registry-only Cargo cache, single DMG notarization with `notarytool info` polling, asset transaction, attestation, and public Release. |
| [macOS Styled DMG Layout](./macos-dmg-layout.md)                          | Finder-free UDRW `.DS_Store` layout, V2 DMG background, `dmg-layout` uv group, and left-to-right Applications drag-install.                                                                                                              |
| [Windows Installer](./windows-installer.md)                               | NSIS bundle, install-path behavior, bounded legacy/staging cleanup, explicit PackageBridge non-ownership, per-asset signing policy, x64/ARM64 native build/package, and lifecycle diagnostics.                                           |
| [Windows Runtime Security](./windows-runtime-security.md)                 | Frozen Explorer-user identity/paths, Alice-owned Tauri state, validated foreground external links through Explorer, untrusted single-instance input, elevated CLI boundary, and Agent Catalog CLI/auth fail-closed on formal elevated Windows.                                                          |
| [Development Environment](./development-environment.md)                   | Locked mise-first local tool versions and the supported host-native compiler/runner/linker boundary.                                                                                                                                     |
| [Repository Task Runner](./task-runner-contract.md)                       | Canonical mise metadata, argv transport, DAG effects, generic direct-session prearchive verification, maintenance safety, and generated task documentation.                                                                              |
| [Optional Codex Development Hooks](./development-hooks.md)                | Upstream prompt-assistance registration, behavior limits, and explicitly accepted hardening regressions.                                                                                                                                 |
| [Application Brand Assets](./application-brand-assets.md)                 | Cross-platform app icons, About reuse, macOS tray templates, and validation.                                                                                                                                                             |
| [Application Identity](./application-identity.md)                         | Cross-layer FyAgent identity, clean-break behavior, and provenance exceptions.                                                                                                                                                           |
| [CC Switch Upstream Synchronization](./upstream-sync.md)                  | Immutable upstream tag verification, two-parent merge ancestry, conflict precedence, and provenance boundaries.                                                                                                                          |
| [Deep-Link Import Security](./deeplink-import-security.md)                | Untrusted `fyagent://v1/import` request validation, explicit provider activation approval, and credential-safe confirmation.                                                                                                             |
| [Main Window Layout](./main-window-layout.md)                             | Work-area clamp, `layout-mode-changed`, and the Windows invariant that maximized windows must not receive `set_min_size` / `set_size` / `set_position`. V2 Overlay is renderer chrome; host owns geometry.                               |

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
