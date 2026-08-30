# Engineer CI and release runner boundaries

## Goal

Align each GitHub Actions job with the narrowest runner class that can prove
its behavior, while preserving FyAgent's native Windows/macOS acceptance and
Release trust boundaries.

The change must remove avoidable macOS coupling from portable control-plane
work, move Windows ARM64 jobs to the explicit Visual Studio 2026 image before
the legacy label's announced migration window, and admit both Visual Studio
2022 and 2026 in the local Windows toolchain resolver.

## Background

- Portable CI and Release orchestration currently run broadly on `macos-15`
  even when they use only Git, Node.js, shell utilities, GitHub APIs, or mock
  tests.
- Windows ARM64 CI and Release matrices currently request `windows-11-arm`.
  GitHub has published the explicit `windows-11-vs2026-arm` label and announced
  that the legacy label will migrate gradually from 2026-09-21 through
  2026-09-30.
- `scripts/tasks/windows-msvc-env.mjs` and `system-check.mjs` admit only
  Visual Studio installation versions in `[17.0,18.0)`, which excludes Visual
  Studio 2026 (`18.x`).
- Repository contract tests intentionally removed `ImageOS` and
  `ImageVersion`: those hosted-image implementation variables are not a
  portable provenance contract and must not be reintroduced.

## Requirements

### R1 — Portable CI control plane

Run the following jobs on explicit `ubuntu-24.04` runners:

- Required CI `commit-convention`, `changes`, `contracts`, `frontend`,
  `desktop-acceptance-contract`, and `required`;
- branch-push Conventional Commit validation;
- trusted-base Labeler automation.

The change must preserve all existing triggers, permissions, change
classification, diagnostic aggregation, and stable check names.

### R2 — Native CI acceptance

Keep native product evidence on native runners:

- Windows x64 backend checks remain on `windows-2025`;
- macOS backend checks remain on `macos-15`;
- Windows native x64 remains on `windows-2025`;
- Windows native ARM64 moves atomically to `windows-11-vs2026-arm`.

No Linux cross-build, emulator, reduced-target fallback, container, or
self-hosted runner may replace native evidence.

### R3 — Portable Release control plane

Run Release `eligibility`, `pin-release-build-inputs`, `verify-assets`,
`attest`, and `publish` on `ubuntu-24.04` without changing their permissions,
dependency topology, frozen identity, signing isolation, attestation subjects,
or one-time publication transaction.

Keep Windows build/proof/sign/seal jobs on matching native Windows runners and
keep universal app build, Developer ID signing, notarization, and DMG creation
on `macos-15`.

### R4 — Explicit Windows ARM64 VS2026 routing

Replace every Windows ARM64 CI/Release runner request and every release target
contract with `windows-11-vs2026-arm`. Preserve native `ARM64`,
`aarch64-pc-windows-msvc`, and managed Python `win-arm64` gates.

### R5 — Visual Studio 2022/2026 compatibility

Use one shared reviewed Visual Studio version range for `vswhere` discovery.
It must admit VS 2022 (`17.x`) and VS 2026 (`18.x`) while rejecting future
unreviewed major versions. User-facing prerequisite guidance and executable
tests must describe both admitted versions.

### R6 — Portable contract execution

Make only the existing platform-neutral portions of Release contract tests
runnable on Linux:

- Bash fixture tests may use the native Linux `bash` executable;
- the Windows NSIS verifier must retain its static checks on Linux while
  reserving native PowerShell 5.1 execution for Windows.

Tests must not be skipped merely to make Ubuntu jobs green.

### R7 — Maintained contracts

Update executable tests and maintained Trellis specs atomically so runner
ownership, ARM64 routing, Linux development-host semantics, and Visual Studio
compatibility do not drift from the workflows.

## Acceptance Criteria

- [x] Every portable job named in R1 and R3 requests `ubuntu-24.04`.
- [x] Only native Windows/macOS jobs retain native runner labels.
- [x] All Windows ARM64 CI and Release matrices request
      `windows-11-vs2026-arm`, with exact native architecture/target checks
      unchanged.
- [x] The MSVC resolver and system-check contract share one `[17.0,19.0)`
      admission range and tests cover it.
- [x] Linux runs the existing Bash/static contract coverage instead of
      excluding those suites.
- [x] Release permissions, signer-secret isolation, immutable build-input
      pinning, fresh sealing, attestation, and publication topology remain
      unchanged.
- [x] `ImageOS`, `ImageVersion`, `imageOs`, and `imageVersion` remain outside
      release metadata and its types/contracts.
- [x] Targeted workflow/toolchain tests, repository Release checks, typecheck,
      formatting, and Trellis validation pass locally.
- [ ] The branch is pushed and a pull request targeting `main` is created.

## Out of Scope

- Job-level Docker containers, service containers, self-hosted runners, larger
  runners, or a new runner provider.
- Linux installer production or Windows/macOS cross-compilation.
- Cargo `target` caching, sccache, workflow topology consolidation, or merging
  Windows CI jobs for performance.
- Reintroducing hosted image implementation variables into release metadata.
- Claiming hosted/native execution before GitHub Actions runs the PR workflow.

## Deferred Evidence

The PR's native Windows ARM64 workflow run is the first authoritative evidence
that the current codebase completes on `windows-11-vs2026-arm`. Local macOS
tests can prove workflow structure and pure contracts, but not that hosted
runner's installed toolchain or native execution.
