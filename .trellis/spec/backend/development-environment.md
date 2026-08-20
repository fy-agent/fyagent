# Development Environment Contract

## 1. Scope / Trigger

Read this reference before changing repository tool versions, `mise.toml`, any
lockfile, Python execution, local onboarding commands, the canonical task API,
or any local compile, package, test, or verification path. Local current-host
development, including `mise run check`, runs on macOS, Windows, and Linux.
The shipped desktop product and its CI/Release evidence remain Windows and
macOS only.
GitHub Actions deliberately installs tools with native setup actions instead
of installing mise.

## 2. Authoritative Version Sources

Each standard ecosystem file is the only human-maintained version source for
its tool:

```text
.node-version                    Node.js 24.19.0
package.json#packageManager      pnpm@10.12.3
rust-toolchain.toml              Rust 1.97.1, minimal + rustfmt + clippy
.python-version                  Python 3.14.7 (consumed only by uv)
mise.toml#[tools]                uv = "latest"
mise.lock                        approved uv resolution and tool artifacts
```

`mise.toml` must not repeat Node, pnpm, Rust, or Python versions. It enables the
Node, pnpm, and Rust idiomatic files, declares only `uv = "latest"`, includes
the domain task TOMLs, and disables automatic installation when ordinary tasks
start. The repository requires mise `>= 2026.8.6`, which is the
`mise.toml` `min_version`. Repository scripts never download or privately
install mise.

The audited repository aliases are:

```toml
[tool_alias]
pnpm = "github:pnpm/pnpm"
uv = "github:astral-sh/uv"
```

They are required because the default aqua resolution observed with mise
2026.8.1 selected x64 Windows assets under a `windows-arm64` key even though
both upstream releases publish native ARM64 assets. A lock regenerated from an
empty file through the aliases selects `pnpm-win-arm64.exe` and
`uv-aarch64-pc-windows-msvc.zip`; `lockfile-check.mjs` rejects a platform key
whose URL names another architecture.

`mise.lock` targets `macos-x64`, `macos-arm64`, `windows-x64`,
`windows-arm64`, `linux-x64`, and `linux-arm64`. Node, pnpm, and uv have a
generated HTTPS URL and SHA-256 checksum for every development host. The
`core:rust` backend currently
emits no platform artifact records: mise reports those four entries as skipped,
so the lock stores exact Rust version/options and native jobs must additionally
prove the selected rustup toolchain. Do not fabricate Rust checksums or present
the platform list alone as artifact evidence.

## 3. uv-owned Python Project

The repository is not a Python package. `pyproject.toml` defines an empty
development environment with `requires-python = ">=3.14,<3.15"`, an empty
`dev` dependency group, and:

```toml
[tool.uv]
package = false
python-preference = "only-managed"
python-downloads = "automatic"
```

uv exclusively owns Python selection, downloads, `.venv`, project dependencies,
and `uv.lock`. There is no system-Python fallback and mise does not inject
`.venv` into every task. Repeatable Python dependencies enter
`pyproject.toml`/`uv.lock`; one-off dependencies use `python:with` or
`python:tool`.

Repository Python tasks use locked uv commands and may prepare the locked
environment. Optional upstream Codex/Trellis prompt hooks are independent of
the project task API and are not an environment, build, check, CI, or release
prerequisite.

## 4. Setup and Execution Boundaries

After independently reviewing the repository config, a developer may trust it
once outside any repository task. The standard flow is:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

No task runs `mise trust` or `mise untrust`. `bootstrap` is the only high-level
environment preparation task. It may run locked mise installation, frozen pnpm
installation, `uv sync --locked`, strict environment checks, and task
validation. It must not install system packages, change trust, change Git
remotes, refresh locks, build, tag, sign, upload, or publish.

`env:check` is strict and read-only. It verifies the standard sources, actual
versions, executable ownership, generated lock structure,
uv-managed Python, `.venv`, offline locked Python execution, Rust components
and sysroot, and mise task metadata. `--json` emits one machine-readable report
and any failed check exits nonzero.

`system:check` is strict and read-only. It probes current-host Tauri
prerequisites and prints official package/tool hints; it never calls `sudo`, a
system package manager, or an installer.

All maintained local project operations use `mise run <task>`. `pnpm dev` and
`pnpm build` are supported package-level aliases for the same guarded native
wrapper. The lower-level `pnpm tauri` leaf remains available to reviewed
GitHub Actions and repository maintenance code, but it is not a standard local
development/build entrypoint and its direct output is not acceptance evidence.
Current developer instructions under `docs/fyagent/development/` and active
specs use this command boundary; new direct-execution entrypoints fail the
documentation contract. CI and Release remain the explicit non-mise execution
boundary. Optional Trellis files and prompt hooks remain outside the project
task contract.

## 5. Host-Native Local Execution

Every canonical local development, build, test, package, and verification
entrypoint is restricted to the OS and architecture of the process actually
running it. The shared Node wrapper maps the six development-host
`process.platform`/`process.arch` pairs
(`darwin`/`win32`/`linux` × `x64`/`arm64`), resolves `rustc` and `rustdoc` from
PATH to absolute executable paths, parses both `-vV` identities, and requires
their host/release/commit to match the process host and each other. Before
starting either probe, Cargo, or Tauri, it rejects fixed-operation forwarded
arguments and case-insensitive caller controls for target, compiler, rustdoc,
compiler wrappers, or any `CARGO_TARGET_*_RUNNER`/`*_LINKER`. A `--target` token is also
rejected in ordinary, build-wide, encoded, or target-specific Rust/rustdoc flag
environment variables; every target-specific Rust/rustdoc flag variable is
rejected even without that token because it can select a linker. Loader/runtime
injection variables such as `DYLD_*` search/insertion paths and
`NODE_OPTIONS`/`NODE_PATH` are likewise rejected before the first probe and
cleared in toolchain children.

The child environment owns the verified absolute compiler/rustdoc paths,
clears both compiler-wrapper slots and every general Rust/rustdoc flag source,
and rejects every effective Cargo config source that declares build target,
compiler, rustdoc, wrapper, Rust/rustdoc flags, or target runner/linker/flags.
The scan covers repository/ancestor/Cargo-home files and recursive includes,
rejecting required-missing includes, symlinks, and cycles before a toolchain
starts. Protected names under Cargo config `[env]` are classified with the same
case-insensitive rules, including forced table values, so an include cannot
restore a cleared compiler, runner, linker, flag, or injection control. Cargo
test receives the exact current-target runner as a CLI TOML array
whose fixed argv is the current absolute Node executable plus the same
`host-native.mjs`; paths containing whitespace remain separate argv. Cargo
appends only the test binary and filter argv. The runner validates the process
host/target, repository target-directory boundary, regular non-symlink file,
native format, and exact PE `Machine`, thin Mach-O `cputype`, or 64-bit
little-endian object `e_machine` before direct
`spawnSync(..., shell: false)`. No shell, emulator,
subsystem bridge, user runner, or user linker participates. The wrapper also passes an explicit
`--target <verified-current-host>` to Tauri and Cargo check/Clippy/test. Caller
safe flags are intentionally not preserved by canonical tasks; reviewed
low-level maintenance commands own any such customization. `rust:fmt` and
`rust:fmt:check` remain the exceptions because rustfmt does not compile or run
a target executable.

On Windows only, `rust:check`, `rust:clippy`, and `rust:test` invoke the same
dependency-free `scripts/prepare-windows-user-helper.mjs` packaging input
preparer exactly once after caller, Cargo-config, runner, and absolute
rustc/rustdoc current-host validation, but before the main workspace Cargo
command. The wrapper supplies its canonical current-host target as
`TAURI_ENV_TARGET_TRIPLE` and fixes `TAURI_ENV_DEBUG=true`; a preparation
failure stops before workspace Cargo. macOS Rust tasks do not run this Windows
resource step. This preserves the Tauri `externalBin` resource
fail-closed contract for local Windows Rust compilation without turning a
local compile or test into PackageManager, ACL, setup, or other native-runtime
evidence.

On Windows only, before launching the final `pnpm tauri`/`cargo` compile child
the wrapper resolves the VS 2022 MSVC/SDK environment through
`scripts/tasks/windows-msvc-env.mjs` and merges it into that child's
environment. This keeps Windows native builds working without a Developer
PowerShell and without persisting MSVC/SDK variables into the system PATH. The
loader locates VS 2022 (Build Tools included) with the official `vswhere.exe`,
verifies the `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` component, and
runs `VsDevCmd.bat -no_logo -arch=<arch> -host_arch=<hostArch>` through a
`cmd.exe` child (`/d /s /c`, `shell: false`, `windowsVerbatimArguments: true`)
that dumps `process.env` as JSON. `-arch`/`-host_arch` derive from
`process.arch`. The parsed environment is validated for `INCLUDE`/`LIB`, loaded
only into the child env, and never mutates `process.env`, the user/system
environment, or the registry. The merge is additive and never overrides the
owned RUSTC/RUSTDOC/target/linker/runner controls. `rust:fmt`/`rust:fmt:check`
do not load MSVC because rustfmt does not compile. On Windows `system:check`
reports a static `vswhere` diagnosis instead of a bare `where.exe cl.exe` probe,
and a missing VS 2022 or VC tools component yields an actionable hint naming the
"Desktop development with C++" workload.

This spec is the active execution boundary. Historical build decisions may
explain provenance, but they do not override the current host-native contract.

The enforced current-host boundary covers `pnpm dev`, `pnpm build`, and the
canonical `mise run dev`, `build`, `build:binary`, `build:debug`, `check`,
`rust:check`, `rust:clippy`, and `rust:test` paths. `rust:test` additionally
accepts at most one non-option test-name filter through mise usage metadata.
The test plan alone enables `fyagent/test-hooks`, allowing integration-test
fixtures to route Windows user paths through their explicit
`FYAGENT_TEST_HOME`; check, Clippy, Tauri builds, and production binaries do not
enable that feature and retain the frozen Explorer-user fail-closed boundary.
It also uses `--no-fail-fast` before the test-harness separator so every
independent test executable runs even when an earlier executable fails; an
optional test-name filter remains after `--` and is passed only to the harness.
The aggregate `check` task runs a pure host-native guard before `env:check`, so
caller compiler, wrapper, runner, target environment, or target-bearing flags
cannot reach even the initial rustc toolchain probe. The guard launches no
subprocess; the later fixed Rust tasks still verify the absolute
rustc/rustdoc identities and pin their Cargo environment independently.
This contract does not claim to intercept arbitrary hand-written low-level
`cargo`, `rustc`, or `pnpm tauri` commands; contributors must not use those as
standard local project entrypoints or cite their output as acceptance. A pure
portable test may exercise platform-neutral policy, but it does not become
native evidence for another OS or architecture.

Matching native GitHub Actions runners are the only project path for non-host
compilation, packaging, and verification. Every supported non-host OS or
architecture gate remains remote. Windows installer execution and locally
copied or staged Windows artifacts are diagnostic experiments at most and
never acceptance evidence. The current native setup contract is owned by
[Windows Installer](./windows-installer.md).

Repository tasks do not install non-host Rust targets or provision a
cross-compilation environment. Adding a new shipped product platform requires a
matching native Actions job and its evidence, not a local target flag or
compatibility script. Adding a development host is not product support: it must
not add Linux packaging, distribution, or CI/Release surfaces, and it must not
`compile_error!` or otherwise refuse `mise run check` on that host.

## 6. Lock and Update Governance

Normal bootstrap/install consumes existing locks and never bumps them. An
intentional full lock regeneration is:

```bash
mise lock --platform macos-x64,macos-arm64,windows-x64,windows-arm64,linux-x64,linux-arm64
mise run tasks:validate
```

Generate the lock from an empty-file state when changing a backend alias, then
run it a second time and require byte stability. Do not hand-edit a checksum,
URL, backend, platform key, or generated marker.

Toolchain and dependency update tasks are ecosystem-specific and preview by
default. They require `--apply` before writing; no task commits, tags, pushes,
changes remotes, opens a PR, or publishes. A failed toolchain update restores
the standard version file and `mise.lock` captured before the attempt.

## 7. Validation / Error Matrix

| Condition                                                             | Required result                                                                         |
| --------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| mise is missing or older than 2026.8.6                                | Stop before dependency preparation                                                      |
| Ordinary task is started with a missing tool                          | Fail and direct the developer to `bootstrap`; never auto-trust                          |
| A standard version differs from the actual executable                 | `env:check` fails                                                                       |
| `mise.toml` repeats Node/pnpm/Rust/Python                             | Lock and environment contracts fail                                                     |
| Python resolves outside uv management or `.venv` is absent            | Python/environment checks fail                                                          |
| Lock platform URL names another architecture                          | Fail, even when checksum and URL are otherwise valid                                    |
| Rust lock has no platform assets                                      | Record exact version/options plus native rustup evidence; never invent an asset claim   |
| A script changes mise trust or private mise/Cargo/rustup homes        | Reject the change                                                                       |
| A fixed native operation receives any forwarded argument              | Reject before probing rustc or starting Cargo/Tauri                                     |
| Caller sets either target environment variable                        | Reject before probing rustc or starting Cargo/Tauri                                     |
| Caller sets compiler/rustdoc/wrapper or any target runner env         | Reject case-insensitively before probing rustc/rustdoc                                  |
| Any supported Rust/rustdoc flag env contains `--target`               | Reject before probing rustc/rustdoc or starting Cargo/Tauri                             |
| `rustc`/`rustdoc` identity differs from host or each other            | Reject before Cargo/Tauri execution                                                     |
| User Cargo config selects target/compiler/wrapper/flags/runner/linker | Reject the effective config before rustc/rustdoc/Cargo/Tauri starts                     |
| A Windows helper preparation fails or selects another target/profile  | Stop before the main workspace Cargo command                                            |
| A local command selects another OS/architecture by any route          | Reject before compilation, packaging, or verification                                   |
| A Linux development host is refused by task or toolchain wrappers     | Fail the environment contract; `mise run check` must admit the current host             |
| `src-tauri` uses `compile_error!` to reject a non-shipping OS         | Fail; compile through the existing unsupported adapter instead                          |
| A non-host result is offered as native acceptance evidence            | Keep the gate pending and require the matching native Actions runner                    |
| Host native libraries are missing                                     | `system:check` fails with a non-elevating installation hint                             |
| Windows VS 2022 / VC tools component is missing                       | `system:check` reports a `vswhere` FAIL naming "Desktop development with C++"; never elevate |
| MSVC environment parse fails or lacks `INCLUDE`/`LIB`                 | Fail before the compile child; never fall back to a bare PATH                           |
| A prerequisite command is absent or cannot be launched                | Record a failed check with its installation hint and finish the machine-readable report |

## 8. Tests Required

- Parse every standard source and assert Node 24.19.0, pnpm 10.12.3, Rust
  1.97.1, Python 3.14.7, and `mise.toml` `min_version = "2026.8.6"` without
  duplicate mise tool-version declarations.
- Regenerate `mise.lock` from no prior lock, target all six development hosts, and
  require an identical second generation.
- Structurally validate backend identity, URLs, SHA-256 checksums, platform
  architecture, native Windows ARM64 pnpm/uv assets, Rust options, and absence
  of mise-managed Python, release targets, and `llvm-tools`.
- Run `uv lock --check --offline`, `uv sync --locked`, and locked/no-sync/offline
  Python 3.14.7 through the created `.venv`.
- Run `mise config ls --json`, `mise tasks ls --json`, `env:check --json`, and
  current-host `system:check`; path comparisons must work with native Windows
  separators, and an empty PATH probe must still return the complete JSON
  failure report with a hint for every missing prerequisite.
- Verify Node/pnpm/uv resolve to `mise which`, and prove Rust with
  `mise which rustc`, the exact rustup active toolchain, components, and sysroot.
- Exercise a parameter plus flag through real `mise run`, and prove filters
  cannot smuggle `--target` into Rust tests.
- Unit-test the exact six-entry process-host mapping, absolute rustc/rustdoc
  resolution and matching `-vV` identities, case-insensitive compiler/wrapper/
  runner/target rejection, target-bearing flag rejection, and fixed
  Tauri/Cargo argv plus owned child environment.
- Unit-test Windows Rust task ordering and environment: one helper preparation
  after all current-host validations and before workspace Cargo, the exact
  canonical target, `TAURI_ENV_DEBUG=true`, and no workspace Cargo after
  preparation failure. Prove macOS and Linux Rust tasks never invoke the helper
  preparer. Prove `src-tauri` compiles on a non-shipping OS through the
  unsupported adapter rather than `compile_error!`, and that
  `supported-platform:check` still rejects product packaging surfaces.
- Unit-test the MSVC loader on a non-Windows host with injected spawn: x64/arm64
  architecture mapping, vswhere candidate paths, non-Windows returning `null`
  without probing, successful `VsDevCmd` environment parsing, missing
  `INCLUDE`/`LIB`, and unparseable JSON. Prove the merge is additive, macOS never
  invokes the loader, and the loader never mutates `process.env`.
- Smoke the real `pnpm dev`/`pnpm build` and canonical mise wrappers with
  rejected arguments/environment, proving the error occurs before rustc,
  rustdoc, Cargo, Tauri, or a frontend build command can start. Fake native
  executables must also prove the normal path receives only the absolute tools,
  empty wrappers/flags, no-shell Node native runner, and verified current-host
  target. Runner tests must include whitespace-containing encoded paths, an
  accepted current-host native binary inside the target boundary, and rejected
  out-of-bound/symlink/wrong-signature cases without building a foreign target.
- Run `developmentEnvironment.test.ts`, `miseTaskContract.test.ts`,
  `taskDocs.test.ts`, `systemCheck.test.ts`, `windowsMsvcEnv.test.ts`, and
  `localBuildBoundary.test.ts`.
- In Required CI, run the locked uv/Python preparation on both `windows-2025`
  x64 and `windows-11-arm` ARM64. Require an explicit uv full managed-Python
  request for each matrix architecture and Python `sysconfig.get_platform()`
  to match `win-amd64`/`win-arm64`; a version-only request can select
  Windows-on-ARM x64 emulation and therefore does not prove a native
  interpreter.
- Scan active local task/package entrypoints for cross-target flags, retired
  cross-build scripts, foreign build tools, subsystem bridges, and emulators;
  exclude GitHub workflow definitions from that negative scan because they own
  the required native platform targets.
- Obtain native Windows ARM64, Windows x64, macOS x64, and macOS ARM64 runner
  evidence before claiming all shipped product platforms verified. Local Linux
  `mise run check` success is development-host evidence, not product support.

## 9. Wrong vs Correct

Wrong: duplicate versions in mise, accept an x64 URL under an ARM64 key, use a
system Python fallback, run a repository trust task, silently install system
packages, bypass the canonical wrapper with a low-level target command, bridge
into a foreign executable, treat a locally staged non-host package as
acceptance, or refuse `mise run check` on a Linux development host by collapsing
product support and development-host admission into one Windows/macOS-only
allowlist or a crate-level `compile_error!`.

Correct: standard ecosystem files select exact versions, mise orchestrates and
locks audited assets for every development host, uv owns Python, canonical local
wrappers verify and pin the current host before toolchain execution, matching
native Actions runners close every shipped-product evidence gate, and Linux
hosts compile through the existing unsupported adapter without claiming product
support.

## Scenario: Development-host admission

### 1. Scope / Trigger

- Trigger: current-host task wrappers, lockfile platforms, `src-tauri` compile
  gates, or `supported-platform:check` would refuse Linux as a development host
  or would reintroduce Linux as a shipped product/CI/Release surface.

### 2. Signatures

- `lib.mjs`: `PRODUCT_PLATFORMS`, `DEVELOPMENT_HOSTS`/`SUPPORTED_PLATFORMS`,
  `isPosixTaskHost(platform)`, `resolveTaskExecutable(command, platform)`
- `host-native.mjs`: `HOST_RUST_TARGETS` includes
  `linux-x64` → `x86_64-unknown-linux-gnu` and
  `linux-arm64` → `aarch64-unknown-linux-gnu`
- `src-tauri/build.rs`: `CARGO_CFG_TARGET_OS` match arms `macos` / `windows` /
  `_` (the catch-all calls `tauri_build::build()` and returns)
- `supported-platform-check.mjs`: `DEVELOPMENT_HOST_ADMISSION_PATHS`

### 3. Contracts

- Request: `mise run check` / `env:check` / `rust:check` on
  `process.platform === "linux"` with `x64` or `arm64`
- Response: host admission succeeds; later missing Tauri native libraries may
  still fail `system:check` or Cargo with installation hints
- Environment: `mise.toml` `settings.lockfile_platforms` equals the six
  development hosts; Node/pnpm/uv lock artifacts exist for `linux-x64` and
  `linux-arm64`
- Product CI/Release workflows remain Windows and macOS runners only

### 4. Validation & Error Matrix

- `freebsd` or another unknown `process.platform` → `Unsupported task host` /
  `Unsupported local host OS/architecture`
- Linux `x64`/`arm64` → admit and pin `x86_64-unknown-linux-gnu` /
  `aarch64-unknown-linux-gnu`
- `compile_error!` on `not(windows|macos)` → contract failure
- Development-host file names the kernel marker → allowed
- Same file names AppImage/Flatpak/GTK product packaging → still rejected
- Product source (`src/**`, unlisted scripts) names the kernel marker → rejected

### 5. Good/Base/Bad Cases

- Good: Linux x64/arm64 `expectedRustTarget` mapping, posix `pnpm` resolution,
  no Windows helper, ELF-style current-host test-binary identity
- Base: macOS and Windows mappings and wrappers unchanged
- Bad: crate-level `compile_error!`, `build.rs` panic on unknown OS, lockfile
  without Linux artifacts, collapsing product and development hosts

### 6. Tests Required

- `developmentEnvironment.test.ts` lockfile platform set
- `localBuildBoundary.test.ts` six-entry mapping
- `miseTaskContract.test.ts` posix executable resolution, Linux helper skip,
  native test-binary identity
- `systemCheck.test.ts` `--describe-platform linux`
- `remainingPlatformSurface.test.ts` admission-path skip vs product packaging

### 7. Wrong vs Correct

#### Wrong

```rust
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
compile_error!("FyAgent desktop supports only Windows and macOS.");
```

#### Correct

Keep Windows/macOS product adapters. On any other OS, compile
`UnsupportedPlatformAdapter` so `cargo check` can run, and admit Linux only in
the development-host wrappers and lockfile.
