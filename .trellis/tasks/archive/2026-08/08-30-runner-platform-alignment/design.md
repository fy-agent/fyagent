# Design: runner platform alignment

## Runner ownership

Use three explicit execution classes:

1. `ubuntu-24.04` for portable Git/Node/Python/Rust contracts, frontend and
   mock checks, CI aggregation, and Release control-plane work.
2. `windows-2025` x64 and `windows-11-vs2026-arm` ARM64 for Windows-native
   Rust, WinRT/Credential Manager, NSIS, signing, and sealing evidence.
3. `macos-15` for macOS-native Rust plus universal build, Developer ID,
   notarization, and DMG evidence.

Runner movement must not alter job dependencies, permissions, frozen Release
identity, artifact IDs, signer boundaries, or the one-time publish transaction.

## Linux contract portability

The Linux blockers wrap portable mechanisms: fake `hdiutil` tests, fake
Release command fixtures, and static NSIS parsing. Extend those non-native
paths to Linux. Native PowerShell 5.1 validation remains Windows-only and
native macOS behavior remains owned by macOS jobs.

## Visual Studio discovery

Keep `vswhere.exe` as the sole installation owner and bound selection to
`[17.0,19.0)`. Map host architecture to both VsDevCmd arguments and the
required component:

| Host | VsDevCmd | Required component |
| --- | --- | --- |
| x64 | `-arch=x64 -host_arch=x64` | `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` |
| ARM64 | `-arch=arm64 -host_arch=arm64` | `Microsoft.VisualStudio.Component.VC.Tools.ARM64` |

The newest installation inside the bounded range is selected. A future major
version fails closed pending review.

## Windows Release toolchain evidence

Do not consume ambient `ImageOS` or `ImageVersion`. The native Windows builder
queries the bounded Visual Studio installation and the selected MSVC toolset,
then passes owned `ACTUAL_VISUAL_STUDIO_VERSION` and `ACTUAL_MSVC_VERSION`
values to the metadata writer.

Advance target records to `fyagent-platform-build/v3` with an exact
`nativeToolchain` field:

- Windows: `{ visualStudio, msvc }`
- macOS: `null`

The aggregate `fyagent-build-metadata/v2` envelope remains unchanged because
it already embeds and strictly validates each target record.

## Rollout and rollback

Required CI on the PR exercises Ubuntu portable jobs and the explicit ARM64
VS2026 runner. A formal Release is not triggered. Rollback is a PR revert; no
persistent product data or public runtime API changes.

## Explicit non-decisions

- No job container, self-hosted runner, `*-latest` label, or cross-compilation.
- No hosted-image implementation variable becomes provenance.
- No native runner is replaced by portable/static evidence.
