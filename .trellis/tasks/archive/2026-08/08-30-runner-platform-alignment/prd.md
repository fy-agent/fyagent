# Align CI and release runners with platform boundaries

## Goal

Assign platform-neutral CI and Release work to a pinned Ubuntu hosted runner
while preserving native Windows and macOS build, runtime, signing,
notarization, and packaging evidence. Complete the explicit Windows ARM64
Visual Studio 2026 migration before GitHub changes the `windows-11-arm` alias.

## Background

- Portable CI and Release control-plane jobs currently run predominantly on
  `macos-15`.
- GitHub announced that `windows-11-arm` will move to Visual Studio 2026 from
  2026-09-21 through 2026-09-30; the explicit GA label is
  `windows-11-vs2026-arm`.
- Local Windows MSVC discovery accepts only Visual Studio 2022
  (`[17.0,18.0)`) and always probes the x64/x86 component.
- Repository history deliberately removed `ImageOS` and `ImageVersion` from
  Release metadata because they are not a stable documented workflow
  contract. This task must not restore them.

## Requirements

1. Use `ubuntu-24.04` for platform-neutral CI jobs: commit policy, change
   classification, repository contracts, frontend checks, mock-only desktop
   acceptance, and the Required aggregate.
2. Use `ubuntu-24.04` for branch-push commit policy and trusted Labeler.
3. Use `ubuntu-24.04` for Release eligibility, artifact pinning, exact asset
   aggregation, attestation, and publication.
4. Keep Windows backend/native checks, Windows build/sign/seal work, macOS
   backend checks, and macOS build/sign/notarization/DMG work native.
5. Replace every Windows ARM64 CI and Release request with
   `windows-11-vs2026-arm`; retain `windows-2025` x64 and `macos-15`.
6. Support bounded Visual Studio 2022 and 2026 discovery (`[17.0,19.0)`) with
   the architecture-appropriate VC tools component.
7. Make fake-tool/static repository and Release contract tests portable to
   Linux without claiming native Windows or macOS evidence.
8. Record explicit Visual Studio installation and MSVC toolset versions in
   Windows Release platform metadata without restoring hosted-image variables.
9. Update executable tests and maintained Trellis specs with the workflows and
   schema.

## Acceptance Criteria

- [ ] Portable CI and Release jobs request `ubuntu-24.04`; native evidence
      stays on matching Windows/macOS runners.
- [ ] No active workflow, executable contract, maintained spec, or release
      target metadata requests `windows-11-arm`.
- [ ] MSVC discovery accepts only Visual Studio major versions 17 or 18 and
      selects `VC.Tools.x86.x64` for x64 or `VC.Tools.ARM64` for ARM64.
- [ ] Linux executes portable contract suites without bypassing native gates.
- [ ] Windows platform metadata includes non-empty Visual Studio and MSVC
      versions; macOS has an explicit non-Windows shape; malformed or extra
      fields fail closed.
- [ ] Ambient `RUNNER_OS`, `RUNNER_ARCH`, `ImageOS`, and `ImageVersion` remain
      excluded from the trusted metadata contract.
- [ ] Relevant unit/static, type, formatting, task, and supported-platform
      validation passes.
- [ ] The branch is pushed and a PR targeting `main` is created.

## Out of Scope

- Job containers, self-hosted/larger runners, or Linux product artifacts.
- Cross-compiling Windows/macOS Release artifacts.
- Changing Windows x64 away from `windows-2025`.
- Cargo target caches, sccache, Release trust-topology changes, or signer and
  publication transaction changes.
- CI job consolidation or other performance restructuring needing measured
  follow-up evidence.

## Open Questions

None. The user approved implementation after the prior runner analysis.
