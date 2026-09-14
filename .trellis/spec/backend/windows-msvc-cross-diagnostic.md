# Optional macOS Windows-MSVC Compile Diagnostic

## 1. Scope / Trigger

Read this contract before changing the optional macOS-only Windows-MSVC
preflight or Clippy diagnostic exposed through `mise run`.

The owner is `scripts/tasks/windows-msvc-cross.mjs` plus the matching task
definitions and executable tests. This diagnostic may help a macOS developer
find Windows compile errors earlier, but it is never Windows runtime,
installer, registry, signing, packaging or HIL evidence. Native host task
execution remains in [Native Host Task Execution](./native-task-runner.md);
actual Windows evidence remains in CI/Release and the Windows runtime specs.

## 2. Signatures

```text
mise run system:check:windows-msvc-cross:advisory
  effect = read-only
  -> node scripts/tasks/windows-msvc-cross.mjs advisory
  -> bootstrap DAG only; never check/CI/Release

mise run system:check:windows-msvc-cross [--json]
  effect = read-only
  -> node scripts/tasks/windows-msvc-cross.mjs check

mise run rust:clippy:windows-msvc-cross
  effect = dependency-environment
  confirmation.default = no
  -> node scripts/tasks/windows-msvc-cross.mjs clippy

WINDOWS_MSVC_CROSS_TARGET = x86_64-pc-windows-msvc
WINDOWS_MSVC_CROSS_HOST_TARGETS = darwin-x64 | darwin-arm64
```

The exact `cargo-xwin` version is owned once by the script. Generic docs/specs
must not duplicate its literal.

JSON report:

```text
{
  ok: boolean,
  platform: Node process.platform,
  target: "x86_64-pc-windows-msvc",
  checks: [{
    id: "supported-host" | "caller-environment" | "cargo-xwin" | "clippy"
      | "rust-target" | "clang-cl" | "lld-link" | "llvm-lib"
      | "cmake" | "ninja",
    name: string,
    ok: boolean,
    hint?: string,
    detail?: string
  }]
}
```

Frozen Clippy plan (`shell:false`):

```text
cargo xwin clippy
  --cross-compiler clang-cl
  --xwin-version 17
  --target x86_64-pc-windows-msvc
  --workspace --all-targets --locked
  --manifest-path src-tauri/Cargo.toml
  -- -D warnings
```

## 3. Contracts

### Advisory

- Advisory is read-only bootstrap reporting. On unsupported hosts it prints
  `SKIP`, exits zero and performs no tool probe.
- Supported-host detection calls the reviewed host-map owner and handles its
  failure. Do not write a negated Darwin predicate such as
  `process.platform !== "darwin"`; the supported-platform scanner treats that
  as an implicit non-macOS target branch.
- On macOS, advisory runs the complete prerequisite report. Missing tools emit
  `ADVISORY` and still exit zero so bootstrap remains useful.
- Advisory never starts Clippy, downloads/caches Windows CRT/SDK, accepts a
  license or mutates repository/system state.

### Strict preflight and Clippy

- `check` is explicit and strict. It probes every bounded prerequisite, prints
  the complete report and exits nonzero for an unsupported/incomplete setup.
  It never installs, downloads or starts Clippy.
- `clippy` starts only after the same preflight is fully green. Mise owns the
  default-no confirmation because cargo-xwin may download/cache Microsoft
  CRT/SDK. The script prints the license note and does not prompt again.
- Only Darwin x64/arm64 is admitted. Windows developers use native builds; all
  other hosts fail strict preflight before probing tools.
- No forwarded argv is accepted. The target and Cargo plan are closed.
- Caller overrides are rejected before any child. Reuse the host-native Rust
  target/compiler/wrapper/runner/linker/Cargo-config checks and additionally
  reject exact `AR`, `CC`, `CXX`, `CFLAGS`, `CXXFLAGS`, `LDFLAGS`, `CMAKE`,
  `CMAKE_GENERATOR`, `CMAKE_PREFIX_PATH`, `CMAKE_TOOLCHAIN_FILE`, `RUSTFLAGS`,
  `RUSTDOCFLAGS`, plus prefixes `CARGO_XWIN_`, `XWIN_`, `CMAKE_`, `CC_`,
  `CXX_`, `AWS_LC_`, and `RING_`.
- Strict preflight and Clippy are absent from `bootstrap`, `check`, all scoped
  check aliases, dev/build, CI and Release. Advisory is required in bootstrap
  and forbidden in the canonical check closure.

### Evidence boundary

A green compile diagnostic proves only that the frozen source can pass the
selected cross Clippy plan in that macOS environment. It does not prove
Windows PackageManager, registry, Credential Manager, WebView2, installer,
UAC, ordinary-user launch, signing, packaging, runtime or HIL behavior.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Advisory on non-macOS | Print `SKIP`, exit zero, no tool probe. |
| Advisory uses a negated Darwin fallback branch | Supported-platform check fails. |
| Advisory on macOS misses prerequisites | Complete report + `ADVISORY`, exit zero, no Clippy. |
| Strict preflight on unsupported host | One failed `supported-host` check, exit nonzero, no tool probe. |
| Caller env/Cargo config override is present | Failed `caller-environment`, no tool child. |
| Prerequisite missing or cargo-xwin version differs from owner | Report all bounded checks, exit nonzero, no Clippy. |
| Clippy invoked without mise confirmation | Task does not start; no download/cache. |
| Extra argv or target override is supplied | Reject before Cargo. |
| Strict/Clippy enters bootstrap, canonical check, CI or Release | Task-contract failure. |
| Advisory leaves bootstrap or enters canonical check | Task-contract failure. |
| Cross result is cited as Windows native acceptance | Keep native evidence pending. |

## 5. Good / Base / Bad Cases

- **Good:** bootstrap on macOS prints a non-blocking advisory. A developer who
  wants more evidence explicitly runs strict preflight, then explicitly accepts
  the dependency-environment confirmation for the frozen Clippy plan.
- **Base:** bootstrap on Windows/Linux prints `SKIP`; strict preflight there
  fails with only `supported-host`. Native Windows CI remains authority.
- **Base:** one macOS prerequisite is missing; the advisory reports all checks
  and exits zero while strict mode returns the same facts nonzero.
- **Bad:** let advisory fail onboarding, put strict/Clippy in canonical checks,
  accept forwarded `--target`, duplicate the cargo-xwin version, install tools,
  or claim Windows installer/runtime acceptance.

## 6. Tests Required

- `tests/windowsMsvcCross.test.ts` covers exact owner-version parsing,
  complete missing-tool reports, supported/unsupported hosts, override
  rejection before spawn, frozen argv, default-no metadata, no installer/
  elevation command, live JSON shape and advisory zero-exit behavior.
- Task graph tests require advisory in bootstrap and prove strict/Clippy are
  absent from bootstrap, canonical checks, dev/build, CI and Release.
- `supported-platform:check` and
  `tests/remainingPlatformSurface.test.ts` reject negated Darwin/implicit-target
  branches and require the reviewed host-map exception path.
- `tests/localBuildBoundary.test.ts` and `miseTaskContract` prove the three
  task signatures/effects and that standard entry points reject other
  cross-target markers.
- `tests/classifyChanges.test.ts` classifies this script as task-runner source,
  not native Windows acceptance.
- Matching-host Windows jobs/HIL remain independently required.

## 7. Wrong vs Correct

### Wrong

```text
bootstrap -> strict Windows-MSVC preflight -> nonzero blocks onboarding
check -> cargo xwin clippy
if (process.platform !== "darwin") return
green cross Clippy -> Windows accepted
```

### Correct

```text
bootstrap -> advisory                    # always non-blocking
explicit strict preflight --json         # may fail
explicit default-no cross Clippy         # closed target/argv
native Windows CI/HIL                    # runtime acceptance

try { expectedWindowsMsvcCrossTarget(platform, arch) }
catch { print SKIP; return }
```
