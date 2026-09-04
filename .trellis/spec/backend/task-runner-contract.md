# Repository Task Runner Contract

## 1. Scope / Trigger

Read this reference before adding, renaming, removing, documenting, or
composing a `mise run` task or changing `scripts/tasks/`. The task API is the
stable local entrypoint for developers. Package scripts and Cargo commands
remain implementation leaves; GitHub Actions is an explicit non-mise boundary
and the sole executor for non-host platform work. Optional Trellis files and
prompt hooks do not extend the project task API.

## 2. Layout and Signatures

`mise.toml` explicitly includes these domain files:

```text
.mise/tasks/core.toml
.mise/tasks/frontend.toml
.mise/tasks/rust.toml
.mise/tasks/python.toml
.mise/tasks/upstream.toml
.mise/tasks/contracts.toml
.mise/tasks/release.toml
```

Included TOMLs use mise's task-file format (top-level task tables, no
`[tasks]` prefix). Simple leaves wrap pnpm/Cargo/uv directly. Complex,
parameterized, filesystem, Git, environment, lock, documentation, or
maintenance logic lives in cross-platform Node `.mjs` scripts; core task
behavior may not depend on Bash.

Every public task has:

- a non-empty `description`;
- `env.FYAGENT_TASK_EFFECT` from the approved effect vocabulary;
- a formal `usage` declaration whenever it accepts an argument or flag;
- `interactive = true` together with `raw = true`, or an explicit
  confirmation, only when its I/O contract requires that behavior.
  Interactive long-running tasks must set `raw` so the console owns Ctrl+C
  and terminal close. The host-native `dev` runner then kills the POSIX
  process group (`kill(-pid)`) or the Windows process tree
  (`taskkill.exe /T /F`). That Windows helper belongs to the development
  task runner; NSIS installers still must not use `taskkill`.

The canonical required subset is generated from live task metadata into
`docs/fyagent/development/mise-tasks.md`. Later requirements may add tasks when
they satisfy this contract; validation requires the named baseline as a subset
instead of freezing a task count.

## 3. Composition and Side Effects

`check` executes `env:check`, frontend, backend, and contracts. Its complete
task-reference closure must have effect `read-only`. Mutation, dependency
installation, build output, interactive tasks, temporary dependency tools,
Git ref writes, and preview-by-default maintenance tasks never enter that
closure.

`check:backend` uses structured sequential task references in this order:

```text
rust:fmt:check -> rust:check -> rust:clippy -> rust:test
```

Frontend checks may be extended by later contracts without replacing the
stable `check`/`check:frontend` entrypoints. A task must not reference a
nonexistent future test or claim a domain gate before that domain implements
it.

`dev`, `build`, `build:binary`, and `build:debug` are fixed current-host
operations and have no caller argument. `pnpm dev`/`pnpm build` and those mise
tasks route through one shared Node wrapper. `dev` is interactive and `raw`;
the wrapper spawns Tauri in the foreground (`windowsHide: false`, POSIX new
process group) and tears down the whole tree on SIGINT/SIGTERM/SIGHUP or
Windows SIGBREAK. The wrapper validates the exact
process OS/architecture against matching absolute `rustc`/`rustdoc` `-vV`
identities. Before probing or launching a toolchain it rejects caller target,
compiler, rustdoc, wrapper, or target-runner/linker controls case-insensitively and
rejects target-bearing ordinary/build/encoded flag sources plus every
target-specific Rust/rustdoc flag source. Process loader/runtime injection
controls are rejected before probing and cleared from toolchain children.
The child owns the absolute tools, empty wrapper/flag settings, and explicit
current-host target. Before starting the toolchain, the wrapper recursively
inspects every effective Cargo config and rejects build target/compiler/
rustdoc/wrapper/flags plus target runner/linker/flags, including config include
cycles and symlinks. The same protected-name classifier rejects corresponding
Cargo config `[env]` keys regardless of case or string/table value. Cargo test
receives its native direct runner through a
CLI TOML argv array built from the current Node process and same wrapper; no
shell quoting is involved. The runner validates target/path/file/native format
and exact PE/Mach-O machine identity, then directly spawns the test binary
with `shell: false`; filters remain argv and never enter a shell.
`rust:check`, `rust:clippy`, and `rust:test` use the same guard; rustfmt does not need a target. `rust:test` accepts at most one test-name
filter, passes it after Cargo's `--`, and rejects every option-like value; in
particular, a caller cannot smuggle `--target` through a variadic usage field.

The lower-level `pnpm tauri` package leaf is retained for reviewed Actions and
maintenance commands that deliberately do not use the local task API. It is
not a standard local entrypoint. This contract enforces canonical wrappers; it
does not claim to intercept an arbitrary hand-written Cargo/Tauri command, and
such a command cannot provide project acceptance evidence.

Portable policy tests can run on the current host, but their result remains a
portable contract result. Windows, macOS, ARM64, and any other non-host native
gate runs only on its matching GitHub Actions runner. Repository tasks never
install or activate a non-host Rust target as part of the standard local
execution path.

There is one explicit non-acceptance exception for early diagnostics on macOS:
`system:check:windows-msvc-cross`,
`system:check:windows-msvc-cross:advisory`, and
`rust:clippy:windows-msvc-cross`. The strict preflight is read-only and reports
the complete bounded prerequisite set. The advisory task is also read-only and
is the only one allowed in `bootstrap`: missing tools print `ADVISORY` and
exit 0, so onboarding never fails. The Clippy task is
`FYAGENT_TASK_EFFECT=dependency-environment`, requires a default-no
confirmation because cargo-xwin may download/cache Microsoft CRT/SDK content,
and runs only after the same preflight passes. All three tasks fix cargo-xwin
to the reviewed version, target only `x86_64-pc-windows-msvc`, use the clang-cl
backend and reviewed xwin toolset, accept no forwarded argument, reject caller
Rust/C/CMake/xwin controls and effective Cargo target/toolchain config, and
invoke a fixed workspace/all-targets/locked Clippy argv with `-D warnings`.
They never install the Rust target, LLVM, CMake, Ninja, a system package, or
accept a license on the developer's behalf. Strict preflight and Clippy are
never reachable from `bootstrap`, `check`, `check:backend`, CI release gates,
or a standard dev/build/test alias. Advisory is bootstrap-only and is also
absent from `check`. The result is cross-compilation diagnostics only; native
Windows CI/HIL remains the authority for registry, PackageManager, WebView2,
installer, UAC, launch, runtime, signing, packaging, and release behavior.
The executable signatures, JSON report, frozen argv, and override matrix for
this exception live in **Scenario: Optional macOS Windows-MSVC Clippy
diagnostic** below.
Linux x64/arm64 is a development host for `check` and other current-host
tasks; it is not a shipped product platform and does not add a local
cross-compile or Actions job.

### Foreground interactive process ownership

#### 1. Scope / Trigger

`mise run dev` (and other `FYAGENT_TASK_EFFECT=interactive` tasks) must own
Ctrl+C and terminal close on every development host. This is a cross-host
process-control contract: macOS/Linux and Windows must be specified
together, not inferred as “not Windows means POSIX”.

#### 2. Signatures

```text
RAW_TASKS = dev | dev:renderer | test:unit:watch | test:v2:watch

executeTauriTask({ operation: "dev", runForegroundCommand })
  -> runForeground(pnpm, ["exec", "tauri", "dev", ...])

signalExitCode(signal)
  SIGINT  -> 130
  SIGTERM -> 143
  SIGHUP  -> 129
  SIGQUIT -> 131
  SIGKILL -> 137
  other   -> 1

killProcessTree(pid, platform)
  win32  -> spawnSync("taskkill.exe", ["/pid", pid, "/t", "/f"])
  darwin | linux -> process.kill(-pid, SIGTERM then SIGKILL)
  other  -> throw Unsupported task host
```

#### 3. Contracts

- `interactive = true` if and only if `raw = true`. `raw` makes the console
  the process-group leader so Ctrl+C and closing the terminal reach the
  child.
- `dev` uses `runForeground` (`spawn`, `stdio: "inherit"`,
  `windowsHide: false`). Other Tauri operations keep `run` /
  `spawnSync` / `windowsHide: true`.
- On first interrupt signal (`SIGINT` / `SIGTERM`), `runForeground` initiates
  graceful shutdown via `killProcessTree` and starts an unref fallback force-kill
  timer (default 3000ms).
- Repeated interrupt signals (such as a second `Ctrl+C`) during shutdown
  immediately re-signal the tree and force-exit the process with standard exit
  code (130 for `SIGINT`).
- Normal signal exits assign standard exit codes (`signalExitCode`, e.g. 130
  for `SIGINT`, 143 for `SIGTERM`) instead of generic failure exit code 1.
- Synchronous task runner `run()` terminates with `signalExitCode` on `SIGINT`
  (130) and `SIGTERM` (143) instead of throwing unhandled signal errors.
- The macOS development helper build script
  (`build-macos-privileged-helper.sh`) traps `INT` and `TERM` and exits
  promptly, preventing orphaned build children during `mise run dev`
  preflight. The outer task runner still owns the user-visible standard signal
  exit code.
- POSIX hosts (`darwin`, `linux`) start a new process group with
  `detached: true` and kill `-pid`. Windows stays attached
  (`detached: false`) and
  uses `taskkill.exe /T /F`. That helper belongs to the development task
  runner only.
- NSIS installers must not use `taskkill`. JavaScript host branches must
  be `win32` then `isPosixTaskHost` then throw; `platform !== "win32"` is
  forbidden.
- `scripts/tasks/platform.mjs` is the zero-dependency owner of the closed
  POSIX-host predicate. `scripts/tasks/lib.mjs` re-exports it for ordinary task
  callers, while bootstrap/CI helpers that run before dependency installation
  import the owner directly.

#### 4. Validation & Error Matrix

| Condition                                            | Required result                                |
| ---------------------------------------------------- | ---------------------------------------------- |
| `FYAGENT_TASK_EFFECT=interactive` without `raw=true` | `tasks:validate` fails                         |
| `dev` uses `run` / `spawnSync`                       | `miseTaskContract` fails                       |
| Windows tree kill uses POSIX `kill(-pid)`            | Child GUI survives; contract test fails        |
| POSIX group kill uses `taskkill.exe`                 | Contract test fails                            |
| Repeated Ctrl+C during dev task shutdown             | Immediate termination with exit code 130       |
| Child process terminated by SIGINT                   | Process exitCode set to 130                    |
| `platform !== "win32"` fallback                      | `supported-platform:check` fails               |
| NSIS script contains `taskkill`                      | Windows installer contract fails               |
| Unsupported `process.platform`                       | Throw `Unsupported task host`; no silent POSIX |

#### 5. Good/Base/Bad Cases

- Good: `mise run dev` on macOS or Windows; Ctrl+C or closing the terminal
  stops Tauri and its children. A second Ctrl+C immediately force-quits with code 130.
- Base: `mise run build` still uses the hidden `run()` helper.
- Bad: only fix Darwin because the author is on a Mac; Windows keeps
  `spawnSync` / `windowsHide: true`.

#### 6. Tests Required

- `RAW_TASKS` equals the four interactive tasks; every interactive task has
  `raw=true`.
- `executeTauriTask({ operation: "dev" })` calls `runForegroundCommand`,
  not `run`.
- `signalExitCode` maps signals to standard exit codes (130 for SIGINT, 143 for SIGTERM).
- `runForeground` force-kills and exits with 130 on repeated SIGINT.
- Child exit on signal yields standard signal exit code.
- the macOS helper source traps INT/TERM, while the outer runner remains the
  owner of final `signalExitCode` mapping;
- `killProcessTree` on `win32` records `taskkill.exe /pid /t /f`; on
  `darwin` and `linux` signals `-pid`; on any other host throws.
- `supported-platform:check` rejects implicit non-Windows branches in
  `scripts/tasks/lib.mjs`.

#### 7. Wrong vs Correct

#### Wrong

```js
if (platform !== "win32") process.kill(-pid, "SIGKILL");
```

#### Correct

```js
if (platform === "win32") {
  runner("taskkill.exe", ["/pid", String(pid), "/t", "/f"], {
    windowsHide: true,
    stdio: "ignore",
  });
} else if (isPosixTaskHost(platform)) {
  posixKill(-pid, "SIGTERM");
  posixKill(-pid, "SIGKILL");
} else {
  throw new Error(`Unsupported task host: ${platform}`);
}
```

## 4. Parameter Transport

mise parses each `usage` spec and exports `usage_<name>` values. Node wrappers
read those values, parse variadic shell-escaped lists into argv arrays, validate
SemVer/package/tag/enum/path inputs, and spawn a command without a shell.
Arguments must never be concatenated into a command string.

### Prearchive active-task verification

**Scope / trigger.** This lifecycle bridge allows one directly active,
in-progress Trellis task to be excluded while its own tracked planning markers
are still present before archival. It is reusable across task names only
because identity is derived and re-proved on every invocation; it never means
"skip active tasks" generally. Ordinary local checks, CI, and post-archive
verification use canonical tasks without an exclusion.

**Signatures.** `check:prearchive` and `check:contracts:prearchive` each require
`--exclude-active-task <path>`. They delegate to
`scripts/tasks/prearchive-check.mjs`, which selects only `check` or
`check:contracts` and never forwards the usage argument to unrelated leaves.

**Contracts.** The accepted path is exactly one repository-relative direct
child matching `.trellis/tasks/MM-DD-<id>`. The checker resolves it below the
canonical tasks root, rejects traversal, backslashes, nesting, archive paths,
wildcards, symlinks, non-direct realpaths, a non-directory task, and a missing,
symlinked, or non-regular `task.json`. It derives `<id>` from the path and
requires `task.json.id` and `task.json.name` to equal it, with
`status === "in_progress"`.

The same canonical task path must be Trellis's current pointer with
`stale === false` and a direct `source: "session:<id>"`; `session-fallback`, a
different task, or any stale pointer fails closed. Validation transports the
path through the private `FYAGENT_SUPPORTED_PLATFORM_ACTIVE_TASK` entry only
after proving that identity. The leaf accepts exactly one input channel: direct
CLI, mise usage, or private environment. Caller-preseeded, conflicting, or
duplicate channels fail. Default `check`, `check:contracts`, and
`supported-platform:check` never infer or apply an exclusion.

**Error matrix.** Missing usage, unknown wrapper mode, malformed/noncanonical
path, path/realpath/file-type escape, metadata ID/name/status mismatch,
missing/stale/fallback/wrong session pointer, caller-preseeded internal state,
multiple input channels, or nested nonzero status stops the wrapper. No failure
may retry with a broader path or omit the platform check.

**Good/base/bad cases.** Good: two differently named canonical fixture tasks
both validate when each is the direct current in-progress task. Base: canonical
`check` runs with no internal entry. Bad: accepting a hard-coded historical ID,
`session-fallback`, an archived/nested/symlinked task, or a second input source.

**Tests required.** Pure and integration tests cover two valid task identities;
path/date/ID/traversal/backslash/archive/nesting failures; task-directory and
metadata-file symlinks; ID/name/status mismatch; stale, fallback, and mismatched
current pointers; and CLI/usage/private-environment duplication. Acceptance
records a real prearchive composite from the directly bound session and a
post-archive canonical run without an exclusion. The private environment entry
is lifecycle evidence and is never provided to CI.

**Wrong vs correct.** Wrong: freeze a historical task constant, broadcast the
raw flag through every task, add a task glob, or teach canonical checks to skip
`.trellis/tasks/**`. Correct: derive one canonical path/ID, prove exact direct
session ownership and metadata, transport it privately to one leaf, archive,
then rerun canonical checks with no exclusion.

### Supported-platform identity seals

The durable surface checker keeps two reviewed identity inventories in
`scripts/tasks/`: one for platform-sensitive first-party source and one for
tracked raster assets. These inventories are fail-closed review authorities,
not content exclusions. Every listed file still passes the normal path, text,
and structure scanners.

The source inventory is recomputed bidirectionally from all tracked Cargo
manifests and build scripts plus executable/configuration files containing
platform selectors. The candidate set, canonical paths, reviewed Git index
mode, regular non-symlink file type, and SHA-256 digest must match exactly.
Platform-sensitive source is `100644` except the single Tauri Cargo runner
`scripts/tasks/macos-signed-dev-cargo.mjs`, which is deliberately `100755`
because Tauri invokes `--runner` directly as `<runner> run ...`; making that
file non-executable is a runtime failure, not a hardening improvement.
Adding, removing, renaming, moving, changing, or changing the mode of a
candidate fails until the source diff is reviewed and the identity inventory
is deliberately updated. A digest-only update is not evidence that a platform
dispatch remains safe.

The checker and both inventories must remain runnable from a clean checkout
using only Node built-ins. The always-running CI Changes job invokes this path
before dependency installation, so importing a package or a helper with a
package dependency is a contract violation.

`format:files` accepts one or more reviewed files and first validates every
operand. It routes validated `.jsonl` names
case-insensitively through record formatting: before any write or Prettier
invocation, it reads every such input, normalizes CRLF to LF, preserves blank
rows, validates each nonblank record as JSON, and removes only insignificant
JSON whitespace outside strings. It does not reserialize parsed values, so
large-number spellings, duplicate members, negative zero, and string escapes
remain byte-identical. A JSON parse failure identifies its file and line,
aborts the whole operation, and leaves every JSONL input untouched without
starting Prettier. Only after all JSONL inputs parse does the task forward the
remaining reviewed paths as distinct argv entries to the repository-locked
Prettier. Immediately before committing, it compares every changed JSONL
target with the bytes read during preflight; drift observed by that precommit
check fails without overwriting the newer content. It stages the complete
JSONL output set and uses the shared
rollback-capable writer for per-file replacement. It rejects empty input,
option-like values, parent traversal, repository-external paths, directories,
symlinks, and realpath escapes. Repository-relative and
absolute-inside-repository paths may contain whitespace or Unicode. JSONL
formatting is syntactic record normalization only. A consumer-specific JSONL
schema, if one exists, must be validated by that consumer's executable tests or
tooling.

On native Windows, local mise tasks resolve only the actually used `pnpm`
command to `pnpm.exe`. This matches the audited `mise.lock` assets
`pnpm-win-x64.exe` and `pnpm-win-arm64.exe`; both carry required SHA-256
checksums. The task runner does not synthesize `.cmd` names for pnpm, npm, npx,
or pnpx and does not introduce `cmd.exe`, `shell: true`, or command-string
quoting. Non-Windows commands remain direct. This local mise boundary is
distinct from GitHub Actions, which does not install mise and uses its own
reviewed `pnpm.cmd` batch-shim bridge in the CI toolchain verifier.

On Windows only, the guarded native wrapper resolves a bounded Visual Studio
2022 or Visual Studio 2026
MSVC/SDK environment for the child process immediately before the final
`cargo`/`pnpm tauri` compile. This is the single controlled exception to the
"no `cmd.exe`" rule: Visual Studio's only supported loading mechanism is
`cmd.exe` + `VsDevCmd.bat`. `scripts/tasks/windows-msvc-env.mjs` locates the
latest complete instance inside `[17.0,19.0)` (including Build Tools) through
the official `vswhere.exe`, requests UTF-8 JSON, and verifies the native-host
component: `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` on x64 or
`Microsoft.VisualStudio.Component.VC.Tools.ARM64` on ARM64. It then spawns
`cmd.exe` directly (not `shell: true`) with the argv array
`["/d", "/s", "/c", "<command>"]` and `windowsVerbatimArguments: true`, where
`<command>` is manually built as
`call "<VsDevCmd>" -no_logo -arch=<arch> -host_arch=<hostArch> >nul && "<node>" -e "process.stdout.write(JSON.stringify(process.env))"`.
The child dumps `process.env` as JSON to avoid `set` text encoding/quoting
ambiguity; the result is parsed and validated for `INCLUDE`/`LIB`, a numeric
`VCToolsVersion`, and a Visual Studio environment major matching the selected
17.x/18.x installation. The loaded
environment merges only into the child env and never mutates `process.env`,
writes the system/user environment, or touches the registry.
`-arch`/`-host_arch` derive from `process.arch` (x64/x64 or arm64/arm64) and are
never hard-coded. The merge is additive: it only adds MSVC/SDK variables and
never overrides the owned RUSTC/RUSTDOC/target/linker/runner controls from
`ownedCargoEnvironment`. macOS never invokes the loader.

Contract tests execute real `mise run` calls for a positional value, a flag,
and a filtered test. Metadata inspection alone is not sufficient proof that
values reach the wrapper.

## 5. Mutation Policies

- `bootstrap` may install locked repository tools/dependencies and run the
  read-only cross-MSVC advisory, but may not install system packages, execute
  strict cross Clippy, accept licenses, trust, change Git, refresh locks,
  build, or publish.
- Formatting is an explicit source-modifying leaf and does not prompt. The
  full `format` task retains its frontend-wide behavior; `format:files` is the
  safe reviewed-subset entrypoint. Its JSONL record normalization does not
  replace a consumer-specific schema check.
- Version, dependency, toolchain, Python lock/dependency, icon, task-doc, and
  clean tasks preview by default; `--apply` is required to write.
- `version:set` and `version:bump` delegate to the canonical atomic version
  tool and remain dry-run by default.
- Clean tasks select only an internal allowlist, resolve every target below the
  repository root, and never delete locks, `.git`, `.trellis`, baselines, or
  end-user data.
- `upstream:fetch` fetches one validated tag. Merge preparation requires a
  clean worktree and `--apply`, and may only enter
  `git merge --no-ff --no-commit`. Upstream tasks never change remotes, resolve
  conflicts, commit, tag, or push.
- `release:check` is read-only; no local task signs, uploads, creates, edits, or
  deletes a GitHub Release.
- No standard local task compiles, packages, or verifies a non-host
  OS/architecture. The exact macOS Windows-MSVC exception above is
  diagnostics-only, cannot package/run/accept Windows, keeps strict
  preflight/Clippy outside the default DAG, and allows only the non-failing
  advisory inside `bootstrap`. Release helpers may be referenced by matching
  native Actions jobs, but no local alias or wrapper turns them into a
  cross-platform acceptance path.

## 6. Generated Documentation

`task-docs.mjs` reads the actual included TOML metadata. It escapes Markdown
pipe characters, emits every loaded task, and writes only when
`tasks:docs:generate --apply` is used. `tasks:docs:check` regenerates in memory
and byte-compares with the committed document.

Maintained repository docs use the live `mise run <task>` API for ordinary
project operations. `docs-contract-check.mjs` scans the public READMEs,
`CONTRIBUTING.md`, `.github` Markdown, and
`docs/fyagent/development/**`; every concrete mise task reference must resolve
through the loaded task metadata. Retired local cross-build tasks have no alias
or deprecation forwarder. Generated-document identity and maintained-doc
`mise run` membership are owned by `task-docs.mjs check` and
`docs-contract-check.mjs`. Vitest must not freeze README or spec prose by
requiring protocol names, toolchain versions, or other documentation
substrings.

Every concrete `mise run <task>` reference must resolve through the live
task-definition loader. The parser accepts the current documented boolean
flags, short flags, value-taking `--jobs`/`--cd` forms (separate or `=` where
supported), and the `--` option boundary. An unknown option fails closed, and
task membership uses an own-property check so inherited object keys are not
treated as task definitions.

`CONTRIBUTING.md` contains the standalone checkout sequence in one exact fenced
block: `mise trust`, `mise run bootstrap`, `mise run system:check`, and
`mise run dev`. Nearby prose states that trust is a manual developer security
decision outside repository tasks, and the document names `mise run check` as
the complete current-host gate.

Optional `.trellis/**` tasks, specs, scripts, skills, hooks, archives, and
journals remain outside this contributor command contract. The docs checker
does not turn them into contribution, build, CI, or release prerequisites.

## 7. Validation / Error Matrix

| Condition                                                              | Required result                                                                         |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Missing description/effect/usage                                       | `tasks:validate` fails                                                                  |
| Interactive task lacks `raw=true`                                      | `tasks:validate` fails                                                                  |
| `dev` does not spawn foreground / kill the host process tree           | `miseTaskContract` fails                                                                |
| Missing task reference or DAG cycle                                    | mise/task contract fails                                                                |
| `check` reaches a non-read-only effect                                 | Fail closed                                                                             |
| A parameter is interpolated into a shell command                       | Reject; spawn validated argv instead                                                    |
| A Windows task forces a pnpm batch shim instead of locked `pnpm.exe`   | Task-runner and DEP0040 contracts fail                                                  |
| Supported Windows VS 2022/2026 or native VC tools component is missing | Fail with a bounded `vswhere` hint naming "Desktop development with C++"; never elevate |
| MSVC env load mutates `process.env` or the user/system environment     | Reject; the loader is child-env-only and additive only                                  |
| `-arch`/`-host_arch` is hard-coded or an unsupported architecture      | Reject; derive from `process.arch` (x64/arm64 only)                                     |
| A Rust filter begins with `-` or contains `--target`                   | Reject before rustc or Cargo starts                                                     |
| A fixed native operation receives forwarded argv                       | Reject before rustc or Tauri starts                                                     |
| Caller compiler/wrapper/runner/linker/target env redirects a task      | Reject before rustc/rustdoc starts                                                      |
| Any Rust/rustdoc flag env contains a target token                      | Reject before rustc/rustdoc starts                                                      |
| Target-specific flags or process-loader/runtime injection are set      | Reject before rustc/rustdoc starts                                                      |
| Absolute rustc/rustdoc identity and process host disagree              | Reject before Cargo/Tauri starts                                                        |
| User Cargo config selects target/compiler/wrapper/flags/runner/linker  | Reject before the toolchain starts                                                      |
| A standard task selects a non-host OS/architecture                     | Reject before any toolchain starts                                                      |
| Optional Windows-MSVC preflight is run off macOS                       | Strict `check` fails before probing; `advisory` prints SKIP and exits 0                 |
| Optional cross prerequisite/version is missing                         | Strict preflight reports every bounded failure and exits 1; advisory prints ADVISORY and exits 0; start no Clippy |
| Optional cross Clippy receives argv/env/Cargo-config override          | Reject before Cargo/cargo-xwin starts                                                   |
| Strict preflight or Clippy becomes reachable from `bootstrap` or `check` | Task-contract failure                                                                 |
| Advisory missing from `bootstrap` or present in `check`                | Task-contract failure                                                                   |
| Optional cross result is cited as native Windows acceptance            | Keep the native gate pending                                                            |
| A local wrapper bridges to a foreign executable/emulator               | Reject; require a native Actions job                                                    |
| Mutation task has neither preview default nor explicit confirmation    | Reject                                                                                  |
| Clean path resolves outside the repository                             | Reject without deletion                                                                 |
| Upstream safety/remotes/worktree do not match                          | Reject before fetch/merge                                                               |
| Generated task reference differs by one byte                           | `tasks:docs:check` fails                                                                |
| New active doc uses a legacy entrypoint                                | `docs-contract-check.mjs` fails                                                         |
| Standalone setup order or manual trust guidance disappears             | `docs-contract-check.mjs` fails                                                         |
| `format:files` receives an option, directory, symlink, or escape       | Reject before Prettier or JSONL writes                                                  |
| A reviewed `.jsonl` target is not valid UTF-8                          | Identify the file; no Prettier or JSONL write                                           |
| A nonblank reviewed `.jsonl` record is invalid JSON                    | Identify file and line; no Prettier or JSONL write                                      |
| A changed JSONL target no longer matches its preflight bytes           | Preserve the newer bytes and fail                                                       |
| A formatted JSONL file violates a consumer-specific schema             | The consumer's executable validation still fails                                        |

## 8. Tests Required

- `mise tasks validate --errors-only` and `task-contract-check.mjs`.
- Required-task subset, metadata/effect/usage, reference closure, check DAG,
  Rust order, retired task, and forbidden command scans.
- Real parameter/flag transport smoke tests, including dry-run `version:set`,
  a test filter, Python preview input, and upstream tag validation.
- `format:files` tests for empty input, option injection, parent/outside paths,
  directories, symlinks, realpath escape, and successful multi-file whitespace,
  Unicode, and absolute-inside-repository argv transport. Cover mixed JSONL
  and Prettier inputs, CRLF/blank-row-preserving compact JSONL records, token
  preservation for large numbers, duplicate members, escapes, and negative
  zero, a later JSONL parse failure that leaves every JSONL byte-identical and
  never invokes Prettier, Prettier failure before JSONL commit, and JSONL
  preimage drift observed by the precommit check that preserves the newer
  bytes.
- Pure executable-resolution tests must require `pnpm.exe` only on Win32,
  preserve direct non-Windows commands, bind both native Windows pnpm lock
  assets and checksums, and prove the DEP0040 checker uses the shared resolver
  without a `pnpm.cmd` fallback.
- Pure tests for all six development-host process mappings, strict absolute
  rustc/rustdoc identity, case-insensitive caller compiler/wrapper/runner/linker/
  target and target-bearing flag rejection, plus fixed current-host
  Tauri/Cargo argv and owned child environment.
- Encode Node runner argv as a Cargo CLI TOML array without a shell, including
  absolute paths containing whitespace; validate its current-target repository
  boundary, non-symlink native format and exact machine identity, and direct
  argv execution with current-host-only fixtures and metacharacter filters.
- Recursively scan effective Cargo config sources and includes; reject
  compiler/wrapper/flags/runner/linker controls, symlinks, and include cycles
  before any toolchain process starts.
- Negative Rust `--target` smuggling tests through normal and double-dash
  invocation paths, plus real `pnpm`/mise wrapper smoke proving rejection
  occurs before rustc, Cargo, or Tauri starts.
- Require the aggregate `check` task to begin with the subprocess-free
  host-native guard before `env:check`, so caller compiler/runner/target
  controls cannot reach its rustc toolchain probe.
- Active standard-entrypoint scans must cover package scripts and the exact
  dev/build/check/current-host Rust tasks, reject cross-target/cross-tool
  execution markers there, and validate the three named optional cross tasks
  separately for fixed metadata, argv, confirmation, host and DAG isolation.
  `.github/workflows/**` stays outside the negative local set so native runner
  targets remain required and testable.
- Cross-diagnostic tests cover exact cargo-xwin version parsing, complete
  prerequisite reporting, advisory success/host skip, strict unsupported-host
  and override rejection before child process launch, fixed x64 clang-cl argv,
  default-no metadata, bootstrap/check DAG membership, no package
  manager/elevation command, and real JSON strict-preflight output.
- Clean preview tests proving canonical repository-only targets and zero writes.
- Docs generation/check tests including a description containing `|` to prove
  table escaping. Live committed-file identity belongs to `tasks:docs:check`
  and `docs-contract-check.mjs`, not a second Vitest byte-compare of
  `mise-tasks.md`.
- Maintained-document fixtures covering mise options, continuations,
  own-property task lookup, the exact standalone checkout sequence, manual
  trust guidance, the full local `check` gate, and unknown task rejection.
- `developmentEnvironment.test.ts`, `miseTaskContract.test.ts`,
  `taskDocs.test.ts`, `systemCheck.test.ts`, `windowsMsvcEnv.test.ts`, and
  `localBuildBoundary.test.ts`.
- `miseTaskContract` must cover `dev` → `runForegroundCommand`,
  `killProcessTree` on `win32` / `darwin` / `linux`, and unsupported-host
  throw. Do not treat Darwin-only shutdown as sufficient Windows evidence.

## 9. Wrong vs Correct

Wrong: put every command back into one `mise.toml`, rely on Bash, infer safety
from a task name, concatenate usage input, let check install/update, bypass the
wrapper with a low-level local target command, hand-edit generated task rows,
or add a Vitest that freezes README/spec prose by substring (the retired
`currentDocsContract` pattern). Interactive `dev` without `raw`, or
`platform !== "win32"` process teardown, is the same class of drift: macOS
appears fixed while Windows (or Linux) keeps a hidden unsunk child.

Correct: domain TOMLs describe a stable API, Node wrappers validate boundaries,
effects make composition auditable, guarded native wrappers verify and pin the
current host, and executable tests prove both metadata and real argument flow.
Interactive tasks set `raw` and tear down the POSIX group or Windows tree
explicitly. Generated `mise-tasks.md` identity is `tasks:docs:check` /
`docs-contract-check.mjs` only.

## Scenario: Optional macOS Windows-MSVC Clippy diagnostic

### 1. Scope / Trigger

- Trigger: new public `mise run` names, a foreign-target argv, a
  `dependency-environment` confirmation, and a JSON report that must not be
  confused with Windows native acceptance. Code-spec depth is mandatory.
- Owner: `scripts/tasks/windows-msvc-cross.mjs` plus the two mise task tables.
  Host-native override rejection is reused from `host-native.mjs`; this owner
  adds C/CMake/xwin/native-dependency prefixes. Semantic Windows runtime
  evidence stays in [Windows Shell-user Runtime](./windows-runtime-security.md)
  and native CI/Release.

### 2. Signatures

```text
mise run system:check:windows-msvc-cross:advisory
  env.FYAGENT_TASK_EFFECT = read-only
  run = node scripts/tasks/windows-msvc-cross.mjs advisory
  bootstrap DAG only; never check/CI/Release

mise run system:check:windows-msvc-cross [--json]
  env.FYAGENT_TASK_EFFECT = read-only
  run = node scripts/tasks/windows-msvc-cross.mjs check

mise run rust:clippy:windows-msvc-cross
  env.FYAGENT_TASK_EFFECT = dependency-environment
  confirm.default = no
  run = node scripts/tasks/windows-msvc-cross.mjs clippy

CARGO_XWIN_VERSION            # exact string in windows-msvc-cross.mjs
WINDOWS_MSVC_CROSS_TARGET     = x86_64-pc-windows-msvc
WINDOWS_MSVC_CROSS_HOST_TARGETS = darwin-x64 | darwin-arm64 -> that target
```

JSON report (`--json` or `usageBoolean("json")`):

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
    detail?: string   # first captured line, truncated to 240 chars
  }]
}
```

Frozen Clippy plan from `planWindowsMsvcCrossClippy` (`shell: false`):

```text
cargo xwin clippy
  --cross-compiler clang-cl
  --xwin-version 17
  --target x86_64-pc-windows-msvc
  --workspace --all-targets --locked
  --manifest-path src-tauri/Cargo.toml
  -- -D warnings
```

Exact cargo-xwin version equality is against `CARGO_XWIN_VERSION` in the
script. Do not copy that literal into this spec or into generic docs.

### 3. Contracts

- `advisory` is read-only bootstrap reporting. Unsupported hosts print SKIP
  and exit 0 without probing. Detect that skip by catching
  `expectedWindowsMsvcCrossTarget(process.platform, process.arch)` against
  `WINDOWS_MSVC_CROSS_HOST_TARGETS`. Do not write
  `process.platform !== "darwin"` or `platform !== "darwin"`: the
  `js:implicit-target` scanner treats a negated Darwin predicate as an
  implicit non-macOS branch and fails `supported-platform:check`. On a
  reviewed macOS host it prints the same complete report as `check`; missing
  tools add an `ADVISORY` line and still exit 0. It never starts Clippy,
  downloads CRT/SDK, or fails `bootstrap`.
- `check` is the explicit strict preflight: probe every bounded prerequisite
  and print the complete report. Incomplete or unsupported-host results exit 1.
  It never installs, downloads, caches CRT/SDK, accepts a license, or starts
  Clippy, and it is not in `bootstrap`.
- `clippy` runs only after the same preflight is fully green. Mise owns the
  default-no confirmation because cargo-xwin may download/cache Microsoft
  CRT/SDK. The Node owner still prints the license note, then runs the frozen
  argv. It does not prompt a second time.
- Hosts other than `darwin-x64` / `darwin-arm64` fail the strict preflight
  before probing tools, with a single `supported-host` check. Windows
  developers use native CI/HIL.
- Caller overrides are rejected before any toolchain child: reuse the
  host-native Rust target/compiler/wrapper/runner/linker/Cargo-config scan,
  plus exact names `AR`, `CC`, `CXX`, `CFLAGS`, `CXXFLAGS`, `LDFLAGS`,
  `CMAKE`, `CMAKE_GENERATOR`, `CMAKE_PREFIX_PATH`, `CMAKE_TOOLCHAIN_FILE`,
  `RUSTFLAGS`, `RUSTDOCFLAGS`, and prefixes `CARGO_XWIN_`, `XWIN_`, `CMAKE_`,
  `CC_`, `CXX_`, `AWS_LC_`, `RING_`.
- No forwarded arguments. `clippy` with extra argv fails before Cargo.
- Strict preflight and Clippy are absent from `bootstrap`, `check`,
  `check:backend`, `check:frontend`, `check:contracts`, `dev`, `build`, and
  CI/Release. Advisory is required in the `bootstrap` closure and forbidden
  in the `check` closure.
- Passing Clippy is compile diagnostics only. It does not claim registry,
  PackageManager, WebView2, installer, UAC, launch, signing, packaging, or
  HIL.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Advisory on non-macOS | Print SKIP; exit 0; no tool probe |
| Advisory skip uses `process.platform !== "darwin"` | `js:implicit-target` / `supported-platform:check` fails |
| Advisory on macOS with missing tools | Print complete report + `ADVISORY`; exit 0; bootstrap continues |
| Strict preflight host is not macOS x64/arm64 | `ok=false`, `checks=[{id:supported-host}]`; exit 1; no tool probe |
| Caller env/Cargo-config override is set | `ok=false`, `checks=[{id:caller-environment}]`; no Cargo |
| Any bounded prerequisite missing or cargo-xwin version ≠ owner constant | Report every remaining check; strict preflight exit 1; no Clippy |
| `clippy` invoked without mise confirmation | Mise does not start the task; no download/cache |
| Forwarded Clippy argv | Throw before `cargo`; no child |
| Strict preflight or Clippy referenced from `bootstrap` / `check` / CI | Task-contract failure |
| Advisory missing from `bootstrap` or present in `check` | Task-contract failure |
| Result cited as native Windows acceptance | Keep the native gate pending |

### 5. Good / Base / Bad Cases

- **Good:** `bootstrap` prints the advisory without failing. A macOS developer
  who wants the diagnostic then runs `system:check:windows-msvc-cross --json`
  and explicitly confirms `rust:clippy:windows-msvc-cross`. Default `check` is
  unchanged.
- **Base:** Windows or Linux `bootstrap` prints SKIP for the advisory. The
  strict preflight on those hosts exits 1 with `supported-host`. Native
  Windows CI/HIL remains the authority.
- **Bad:** let advisory exit 1, put strict preflight or Clippy in bootstrap,
  skip hosts with `process.platform !== "darwin"`, accept forwarded
  `--target`, pin a second cargo-xwin version in a spec/workflow, or treat a
  green report as Windows installer/registry evidence.

### 6. Tests Required

- `tests/windowsMsvcCross.test.ts`: exact version parse, complete missing-
  tool reporting, unsupported-host and override rejection before spawn,
  frozen argv, default-no metadata, no package-manager/elevation command,
  live `--json` preflight shape, advisory in bootstrap with exit 0, and
  strict preflight/Clippy absent from bootstrap/check.
- `supported-platform:check` / `tests/remainingPlatformSurface.test.ts`: the
  owner has no negated Darwin/`!== "win32"` fallback; advisory skip is the
  reviewed host-map throw, not an implicit-target branch.
- `tests/localBuildBoundary.test.ts` and `miseTaskContract`: the three named
  tasks exist with the signatures above; standard entrypoints still reject
  other cross-target markers.
- `tests/classifyChanges.test.ts`: the new script is classified with other
  task-runner sources, not as a native Windows acceptance path.

### 7. Wrong vs Correct

#### Wrong

```text
bootstrap -> system:check:windows-msvc-cross   # exit 1 blocks onboarding
check -> rust:clippy:windows-msvc-cross
cite macOS cargo-xwin as Windows HIL
if (process.platform !== "darwin") { SKIP; return }
```

#### Correct

```text
bootstrap -> system:check:windows-msvc-cross:advisory   # never fails
mise run system:check:windows-msvc-cross --json         # explicit, may fail
mise run rust:clippy:windows-msvc-cross                 # default-no, frozen argv
native Windows CI/HIL                                   # remaining acceptance
try { expectedWindowsMsvcCrossTarget(process.platform, process.arch) }
catch { print SKIP; return }
```

## macOS signed development runner

On macOS, the canonical `dev` task remains current-host-only, interactive, and
raw. It keeps Tauri dev/HMR, but supplies a fixed Cargo runner chain that wraps
the emitted debug executable in a real development app bundle before launch.
The chain performs, in order:

1. fixed-path full-Xcode plus user-local Developer ID PKCS#12 preflight through
   a reusable 0700 cache keychain. The task extracts the same certificate and
   private key into a temporary 0700 directory, imports the leaf and traditional
   RSA private key alongside the pinned Apple Root and Developer ID G2 public
   certificates, never installs the release private key permanently in the
   login keychain, and smoke-signs a copy of `/usr/bin/true` before the Swift
   helper build. `machine-preflight --keep-session` keeps the cache keychain as
   the user default through the detached Tauri spawn and nested app-runner
   signing. App-runner or `restore-session` restores the original default and
   search list after nested signing, setup failure, or Tauri process exit. A
   standalone preflight restores immediately, and `restore-session` is
   idempotent when no session is active. Never restore early or delete a
   keychain after it has signed with this identity; delete only the temporary
   extracted PEM files after import.
2. development-flavor universal privileged helper/client build and embedded
   plist verification;
3. Tauri dev compilation with the privileged-client Cargo feature;
4. app bundle assembly, client/helper embedding, inside-out signing with the
   frozen `Developer ID Application: William Wang (HY446996QX)` identity,
   strict signature/link/rpath verification, and direct bundle executable
   launch. Development does not notarize or staple the app.

The runner accepts only Tauri/Cargo's fixed protocol arguments and rejects
forwarded application arguments. The task owns and sanitizes
`DEVELOPER_DIR`, Cargo/rustc runner settings, privileged artifact variables,
`DYLD_*`, `RUSTFLAGS`, `NODE_OPTIONS`, and related injection surfaces. Ctrl+C
continues to terminate the complete child process group. Linux and Windows keep
their previous native task behavior.

The repository does not contain a developer-machine PKCS#12 path or password.
`scripts/tasks/macos-signed-dev.mjs` subcommand `configure` writes a mode-0600 configuration
under the user's FyAgent Application Support directory that references a local
PKCS#12 and credentials file. `mise run dev` consumes only that fixed local
configuration; callers cannot override signing paths or credentials through
task arguments or environment variables.
