# Design: CI and Release runner boundaries

## Boundary model

Jobs are assigned by the evidence they must produce, not by historical
uniformity:

| Responsibility                                                        | Runner                  | Reason                                              |
| --------------------------------------------------------------------- | ----------------------- | --------------------------------------------------- |
| Git/Node policy, classification, mock tests, aggregation              | `ubuntu-24.04`          | Portable control-plane behavior                     |
| Release identity, artifact collation, attestation, GitHub publication | `ubuntu-24.04`          | Portable orchestration; no native product semantics |
| Windows x64 compile/runtime contracts                                 | `windows-2025`          | Matching native Windows evidence                    |
| Windows ARM64 compile/package/proof/sign/seal                         | `windows-11-vs2026-arm` | Explicit matching ARM64 + VS2026 image              |
| macOS compile/sign/notarize/DMG                                       | `macos-15`              | Apple-native build and security tooling             |

No job container is added. A fresh GitHub-hosted VM remains the isolation
boundary, avoiding a second base-image lifecycle while native jobs continue to
use platform tools unavailable from Linux containers.

## CI changes

Runner-only changes are applied to portable jobs. Their steps, permissions,
conditions, outputs, and aggregation remain byte-for-byte equivalent except
where Linux portability requires an executable contract correction.

The frontend job continues to run its existing full unit discovery. Rather
than exclude Release tests on Ubuntu, the two Bash fixtures admit Linux and the
NSIS verifier executes the same portable static checks before returning on
non-Windows hosts. Native PowerShell 5.1 parsing/execution stays Windows-only.

## Release changes

The Release dependency graph and trust zones remain unchanged. Only portable
orchestration jobs move to Ubuntu. Native build/sign/seal jobs retain their
existing permissions and secret boundaries.

All four Windows ARM64 matrix occurrences move together to the explicit
VS2026 label. `EXPECTED_TARGETS` and every workflow mutation test move in the
same change so published build metadata continues to bind the requested label.

## Visual Studio discovery

`windows-msvc-env.mjs` owns one exported range:

```text
[17.0,19.0)
```

The lower bound preserves VS2022 support; the exclusive upper bound admits
VS2026 while failing closed on an unreviewed VS19 major. `system-check.mjs`
uses the same exported constant rather than duplicating the range.

This task does not add Visual Studio or `cl.exe` fields to Release metadata.
The current metadata contract deliberately distinguishes a requested runner
label from observed runner context and removed undocumented hosted-image
variables. Recording a separately discovered compiler without binding the
actual Tauri/Cargo build process to that exact discovery would overstate the
evidence. Native runner execution plus exact requested label, architecture,
Rust/Node/pnpm versions, and artifact attestation remain the truthful current
contract.

## Compatibility and rollback

- No user-facing application behavior or artifact naming changes.
- A runner rollback is a single-label/workflow/spec/test reversal, but the old
  ARM64 label should not be restored after its announced migration window.
- Portable jobs can be returned to macOS independently if a genuine
  undocumented host dependency is discovered; tests must identify the missing
  contract rather than silently pinning macOS again.
- Release publication remains fail-closed if any migrated control-plane job
  lacks a required Linux utility or contract.

## Change boundary

Expected write set:

- `.github/workflows/{ci,commit-convention-push,labeler,release}.yml` — runner
  ownership and ARM64 label routing;
- `scripts/tasks/{windows-msvc-env,system-check}.mjs` — one VS admission range;
- `scripts/release/verify-windows-nsis-contract.mjs` — Linux static-contract
  admission;
- affected workflow, MSVC, release, and host-boundary tests;
- maintained backend Trellis specs and this task's evidence.

Explicitly not changing application source, release permissions/dependencies,
signing code, artifact formats, package managers, dependencies, or caches.
