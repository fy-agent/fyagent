# Development Environment Contract

## 1. Scope / Trigger

Read this contract before changing repository tool versions, `mise.toml`, a
lockfile, Python ownership, onboarding commands, development-host admission, or
the boundary between local checks and native CI/Release evidence.

The canonical local task API is owned in detail by
[Repository Task Runner](./task-runner-contract.md). Local current-host
development supports macOS, Windows, and Linux on x64/ARM64. The shipped
desktop product and native release evidence remain Windows and macOS only.
GitHub Actions installs reviewed tools through workflow setup steps; it does
not install or execute mise.

## 2. Authorities and signatures

Each ecosystem has one human-maintained version authority:

| Authority | Owns |
| --- | --- |
| `.node-version` | Node.js version. |
| `package.json#packageManager` | pnpm version. |
| `rust-toolchain.toml` | Rust channel, profile, and components. |
| `.python-version` | Managed Python version requested through uv. |
| `mise.toml#min_version` | Minimum supported mise version. |
| `mise.toml#[tools]` | Repository-selected mise tools that do not have an ecosystem authority file. |
| `mise.lock` | Generated, reviewed multi-host tool resolutions, URLs, and checksums. |
| `pyproject.toml` / `uv.lock` | Python environment policy and exact dependencies. |
| `pnpm-lock.yaml` / Cargo lockfiles | Exact JavaScript/Rust dependency graphs. |

Generic specs and workflows must not introduce a second literal version source.
Validation reads the files above and requires actual tools, workflow setup,
generated locks, and metadata to agree exactly.

`mise.toml` enables the ecosystem idiomatic files, declares only tools without
another authority, includes the domain task TOMLs, disables ordinary-task
auto-install, and declares the six development-host lock platforms. Its
reviewed aliases for pnpm and uv are part of lock resolution because they must
select matching native assets, including Windows ARM64. `mise.lock` must have a
generated HTTPS URL and SHA-256 for every applicable host artifact. A backend
that does not emit platform artifacts must be represented honestly through its
version/options plus native toolchain evidence; never fabricate checksums.

Repository scripts do not download or privately install mise. A developer
chooses and installs mise outside the repository trust boundary.

## 3. Contracts

### uv-owned Python environment

The repository is not a Python package. `pyproject.toml` defines a managed,
non-package environment. uv exclusively owns Python selection/download,
`.venv`, project dependency resolution, and `uv.lock`; there is no system
Python fallback.

The default locked sync installs only default development groups. Optional
macOS DMG layout dependencies are installed only by the explicit Release group
that owns them. Do not move a release-only dependency into the default group
for convenience.

Repeatable Python dependencies enter `pyproject.toml` and `uv.lock`. One-off
tools use the repository task API's reviewed uv execution modes. mise does not
globally inject `.venv` into every task.

Optional Trellis/Codex prompt hooks are independent of the project Python
environment and are not an onboarding, build, check, CI, or Release
prerequisite. Their boundary is [Optional Codex Development Hooks](./development-hooks.md).

### Onboarding and execution boundaries

After independently reviewing repository configuration, a developer may trust
the checkout once outside repository tasks:

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

- No repository task runs `mise trust` or `mise untrust`.
- `bootstrap` is the only high-level environment-preparation task. It consumes
  existing locks, installs repository tools/dependencies, synchronizes the
  locked uv environment, and validates task metadata. It must not install
  system packages, change Git/remotes, refresh locks, build, sign, tag, upload,
  or publish.
- `env:check` is strict and read-only. It compares every authority with the
  actual executable/environment/lock state and emits a complete machine-readable
  report when requested.
- `system:check` is strict and read-only. It diagnoses current-host native
  prerequisites and prints official, non-elevating installation guidance; it
  never invokes `sudo`, a package manager, or an installer.
- Maintained local project operations use `mise run <task>`. Supported package
  aliases route through the same guarded implementation. A hand-written
  low-level Cargo/Tauri command is not a canonical entrypoint or acceptance
  evidence.
- CI and Release are explicit non-mise boundaries and must still consume the
  same repository version authorities.

### Current-host execution and evidence boundary

Canonical local compile, test, build, package, and verification paths may
target only the OS/architecture of the process running them. The task runner
owns the exact host mapping, absolute `rustc`/`rustdoc` identity checks, caller
environment rejection, Cargo-config scan, no-shell runner, Windows helper
preparation, and bounded MSVC environment loading. See
[Repository Task Runner](./task-runner-contract.md).

This environment contract requires those guards to establish all of the
following before a native toolchain child runs:

- process OS/architecture, selected Rust target, compiler, rustdoc, and test
  executable identity agree;
- caller target/compiler/wrapper/runner/linker/flag and loader-injection controls
  cannot redirect the operation;
- effective Cargo configuration cannot restore a rejected control;
- fixed local operations do not accept arbitrary forwarded target arguments;
- Windows native prerequisites are prepared/loaded only for Windows children
  and never persisted to the user/system environment;
- commands run without an emulator, subsystem bridge, foreign executable,
  shell-string transport, or cross-target fallback.

Local Linux x64/ARM64 is a **development host**, not a shipped platform. The
Rust crate must remain checkable there through the existing unsupported product
adapter; a crate-level `compile_error!` for non-Windows/macOS is forbidden.
Linux admission does not add Linux packaging, runtime support, CI product jobs,
or Release assets.

Matching native GitHub Actions runners are the only project acceptance path
for another OS/architecture. Portable tests may validate policy and parsing,
but they do not prove native installer execution, platform APIs, signing,
notarization, packaging, or shipped runtime behavior.

### Lock and update governance

Ordinary bootstrap/install consumes committed locks and never updates them.
Intentional lock regeneration must:

1. use the platform set declared by `mise.toml`;
2. regenerate rather than hand-edit generated URLs/checksums/backend markers;
3. start from an empty lock when changing a resolution backend/alias;
4. run a second generation and require byte stability;
5. run task, environment, architecture, checksum, and offline lock validation.

Toolchain/dependency update tasks are ecosystem-specific and preview by
default. `--apply` is required to write. A failed update restores captured
authority and lock files. Update tasks never commit, tag, push, change remotes,
open a PR, or publish.

## 4. Validation / Error Matrix

| Condition | Required result |
| --- | --- |
| Installed mise is absent or below `mise.toml#min_version` | Stop before dependency preparation. |
| Ordinary task starts with a missing managed tool | Fail with `bootstrap` guidance; never auto-trust. |
| Actual Node/pnpm/Rust/Python differs from its authority | `env:check` fails. |
| `mise.toml` duplicates an ecosystem-owned version | Environment/lock contract fails. |
| Python is outside uv management or the locked `.venv` is unavailable | Python/environment checks fail; no system fallback. |
| A lock URL/platform selects another architecture | Fail even when the URL and checksum are syntactically valid. |
| Generated lock data is manually edited or unstable on regeneration | Reject the lock update. |
| A repository task changes trust, private tool homes, system packages, or Git | Reject the task/change. |
| Caller or Cargo config redirects target/compiler/wrapper/runner/linker/flags | Reject before the first toolchain process. |
| Compiler/rustdoc/native test identity differs from the current host | Reject before Cargo/Tauri execution. |
| Windows helper/MSVC preparation fails or selects another host target | Stop before the compile child; do not fall back. |
| Linux x64/ARM64 is refused as a development host | Fail the environment contract. |
| A local/portable result is presented as another platform's native evidence | Keep that native gate pending. |
| Host native libraries are missing | `system:check` fails with non-elevating official guidance. |

## 5. Good / Base / Bad Cases

- **Good:** one authority per ecosystem; locked bootstrap; exact executable,
  lock, and workflow agreement; current-host-only local execution; Linux
  development-host admission without a Linux product claim; matching native
  Actions evidence for shipped platforms.
- **Base:** a portable policy/parser test runs on the current host and reports
  only that bounded result. Missing native platform evidence remains pending.
- **Bad:** duplicate literal versions, automatic trust/system installation,
  system-Python fallback, hand-edited lock artifacts, a foreign `--target`, or
  portable/local output presented as another platform's native acceptance.

## 6. Tests Required

- Parse every authority file and prove actual executables, workflow setup, task
  metadata, and generated locks agree without hard-coded duplicate values in
  generic specs/configuration.
- Regenerate the mise lock for the declared host set from an empty state when
  relevant and require an identical second generation; validate URL,
  checksum, backend, and architecture identity.
- Run uv lock/sync/offline checks and prove the selected interpreter and
  `.venv` are uv-managed and match `.python-version`/`pyproject.toml`.
- Run mise config/task metadata, `env:check --json`, and current-host
  `system:check`, including complete failure reporting when prerequisites are
  absent.
- Exercise real task argument/flag transport and the host-native guard's
  positive and negative OS/architecture mappings, including Linux x64/ARM64.
- Test absolute compiler/rustdoc identity, environment/Cargo-config rejection,
  native test-binary format, no-shell argv transport, Windows helper ordering,
  and Windows MSVC child-only environment behavior through the task-runner
  suites.
- Scan active local entrypoints for cross-target flags, emulators, subsystem
  bridges, foreign build tools, and retired cross-build paths. Native workflow
  targets are outside that negative local-entrypoint scan.
- Require matching Windows/macOS native runner evidence before claiming shipped
  product platform verification. Linux check success remains development-host
  evidence only.

## 7. Wrong vs Correct

Wrong:

```text
copy exact versions into mise, specs, and workflows
fall back to system Python
let ordinary tasks install/trust/update automatically
run cargo/tauri with a foreign --target and cite it as acceptance
treat Linux development-host admission as a Linux release target
```

Correct:

```text
one ecosystem authority -> exact executable/workflow/lock comparison
mise orchestration -> committed multi-host lock -> uv-owned Python
canonical task -> verified current host -> direct native child
matching native Actions runner -> shipped-platform evidence
Linux local check -> development evidence only
```
