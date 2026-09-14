# Native Host Task Execution

## 1. Scope / Trigger

Read this contract before changing host-native command resolution, the Windows
MSVC environment loader used by local Tauri/Cargo tasks, or the macOS signed
development app runner.

This owner is intentionally narrower than
[Repository Task Runner](./task-runner-contract.md): the task runner owns the
public `mise run` API, effects, validated argument transport and DAG; this file
owns the last matching-host process/environment boundary immediately before a
native command starts. Optional macOS-to-Windows compile diagnostics are owned
separately by
[Windows MSVC Cross Diagnostic](./windows-msvc-cross-diagnostic.md).

## 2. Signatures

```text
resolveTaskExecutable("pnpm", platform)
  win32 -> reviewed pnpm.exe from mise lock/runtime
  darwin/linux -> direct pnpm executable

resolveWindowsMsvcEnvironment({ processArch, inheritedEnv })
  -> additive child environment for VS 2022/2026 native tools

ownedCargoEnvironment(...)
  + resolved MSVC/SDK variables
  -> final Cargo/Tauri child env

mise run dev  # macOS
  -> host-native wrapper
  -> signed development app bundle runner

scripts/tasks/macos-signed-dev.mjs
  configure | machine-preflight [--keep-session] | restore-session | ...

scripts/tasks/macos-signed-dev-cargo.mjs <fixed Cargo runner protocol>

RAW_TASKS = dev | dev:renderer | test:unit:watch

executeTauriTask({ operation: "dev", runForegroundCommand })
  -> runForeground(pnpm, ["exec", "tauri", "dev", ...])

signalExitCode(SIGINT | SIGTERM | SIGHUP | SIGQUIT | SIGKILL | other)
  -> 130 | 143 | 129 | 131 | 137 | 1

killProcessTree(pid, platform)
  win32 -> taskkill.exe /pid <pid> /t /f
  darwin | linux -> kill process group -pid (TERM then KILL)
  other -> throw Unsupported task host
```

No renderer/user input becomes an executable path, shell command, signing
identity, target triple, compiler wrapper or environment injection.

## 3. Contracts

### Windows executable and batch boundary

- Local mise tasks resolve only the actually used `pnpm` command to
  `pnpm.exe` on Windows. This matches the reviewed x64/arm64 executable assets
  and SHA-256 values in `mise.lock`.
- The task runner does not synthesize `.cmd` names for pnpm, npm, npx or pnpx
  and does not introduce `shell:true`, generic command strings, or batch-shim
  quoting. Non-Windows hosts continue direct execution.
- GitHub Actions has a separate reviewed `pnpm.cmd` bridge in the CI toolchain
  verifier. That hosted boundary is not reused by local mise tasks.

### Foreground interactive process ownership

- `interactive=true` if and only if `raw=true`. The raw task owns the console
  and complete child process tree so Ctrl+C or terminal close cannot leave a
  hidden Tauri/watch process.
- The closed interactive task set is `dev`, `dev:renderer`, and
  `test:unit:watch`. Tauri `dev` uses `runForeground` with inherited stdio and
  visible Windows console behavior; noninteractive build/check operations keep
  the synchronous hidden runner.
- On the first `SIGINT` or `SIGTERM`, `runForeground` starts graceful tree
  shutdown and an unref force-kill timer (default 3000ms). A repeated signal
  immediately re-signals the tree and exits with the standard signal code.
- `signalExitCode` maps SIGINT/SIGTERM/SIGHUP/SIGQUIT/SIGKILL to
  130/143/129/131/137; unknown signals map to 1. The synchronous runner uses
  the same SIGINT/SIGTERM mapping rather than throwing an unhandled signal.
- Darwin/Linux children start a detached process group and are signalled by
  negative PID. Windows remains attached and uses
  `taskkill.exe /T /F`. Host dispatch is `win32`, then the closed POSIX helper,
  then throw; `platform !== "win32"` is prohibited.
- `scripts/tasks/platform.mjs` is the zero-dependency POSIX-host authority.
  Ordinary task helpers may re-export it; bootstrap/CI code that runs before
  dependency installation imports it directly.
- The macOS privileged-helper build script traps INT/TERM and exits promptly;
  the outer task runner remains owner of the final signal exit code.
- The `taskkill` helper belongs only to development task process control. NSIS
  installer scripts must not use it.

### Windows MSVC environment loader

- On native Windows only, the guarded wrapper resolves a bounded Visual Studio
  2022 or 2026 instance inside `[17.0,19.0)` with official `vswhere.exe` JSON.
  Build Tools is valid when the native component exists.
- Required component is `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`
  on x64 or `Microsoft.VisualStudio.Component.VC.Tools.ARM64` on ARM64.
- Visual Studio's supported environment mechanism is the one narrow exception
  to the local no-`cmd.exe` rule. Spawn `cmd.exe` directly—not `shell:true`—with
  `[/d, /s, /c, <closed command>]` and
  `windowsVerbatimArguments:true`.
- The closed command calls the validated `VsDevCmd.bat` with `-no_logo`,
  architecture/host architecture derived from `process.arch`, then invokes the
  current Node executable to serialize `process.env` as JSON. Do not parse
  localized `set` output or hard-code x64 on ARM64.
- Validate `INCLUDE`, `LIB`, numeric `VCToolsVersion`, and a Visual Studio
  environment major matching the selected 17.x/18.x instance.
- Merge the result only into the child environment. Never mutate
  `process.env`, user/system environment or registry.
- The merge is additive and may not override owned Rust/Cargo controls,
  including RUSTC/RUSTDOC, target, linker, runner, wrappers and flags already
  established by `ownedCargoEnvironment`.
- Missing prerequisites fail with bounded guidance naming the Visual Studio
  “Desktop development with C++” workload. The wrapper never elevates or
  installs components.

### macOS signed development runner

- The canonical macOS `mise run dev` remains current-host-only, interactive
  and raw. It keeps Tauri HMR while wrapping the emitted debug executable in a
  real development app bundle before launch.
- Preflight uses full Xcode and user-local Developer ID PKCS#12 configuration
  to create/reuse one mode-0700 cache keychain. It extracts the certificate and
  private key into a temporary mode-0700 directory, imports the leaf,
  traditional RSA key and pinned Apple Root/Developer ID G2 public
  certificates, and smoke-signs a copy of `/usr/bin/true`.
- `machine-preflight --keep-session` keeps that keychain as the user default
  through the detached Tauri process and nested signing. App-runner or
  `restore-session` restores the original default/search list after success,
  setup failure or process exit. Standalone preflight restores immediately;
  restore is idempotent when no session exists.
- Never delete a keychain after it has signed with the identity. Delete only
  temporary extracted PEM files after import.
- Build a development-flavor universal privileged helper/client, verify
  embedded plists, compile Tauri with the privileged-client feature, assemble
  the app bundle, embed client/helper, sign inside-out with the frozen reviewed
  Developer ID identity, verify signature/link/rpath, then launch the bundle
  executable. Development does not notarize or staple.
- The Cargo runner accepts only Tauri/Cargo's fixed protocol and rejects
  forwarded application arguments. It owns/sanitizes `DEVELOPER_DIR`,
  Cargo/rustc runner settings, privileged artifact variables, `DYLD_*`,
  `RUSTFLAGS`, `NODE_OPTIONS` and related injection surfaces.
- Ctrl+C terminates the complete child process group. Linux and Windows retain
  their matching-host native behavior; no macOS wrapper becomes a cross-host
  acceptance path.
- The repository never stores the developer PKCS#12 path or password.
  `macos-signed-dev.mjs configure` writes a mode-0600 local configuration under
  FyAgent Application Support, referencing local PKCS#12 and credentials
  files. `mise run dev` accepts no env/argv override for those secrets.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Windows local task resolves `pnpm.cmd` or uses `shell:true` | Reject; use reviewed `pnpm.exe` direct execution. |
| Interactive task lacks `raw=true` | Task validation fails. |
| Tauri dev uses the synchronous/hidden runner | Task-contract failure. |
| Windows tree kill uses POSIX negative PID | Reject; use `taskkill.exe /T /F`. |
| Darwin/Linux tree kill uses `taskkill.exe` | Reject; signal the detached group. |
| Repeated Ctrl+C during shutdown | Immediate tree termination and exit 130. |
| Unsupported host falls through a negated Windows branch | Supported-platform failure; throw explicitly. |
| NSIS script contains `taskkill` | Installer-contract failure. |
| Windows x64/ARM64 mise executable or lock digest is absent/drifted | Fail tool admission before task child. |
| `vswhere` finds no complete 17.x/18.x instance/native component | Bounded workload hint; no elevation/install. |
| MSVC loader hard-codes architecture or accepts unsupported `process.arch` | Reject before `VsDevCmd`. |
| Closed `cmd.exe` command/path is caller-controlled | Reject; no shell execution. |
| Parsed environment lacks INCLUDE/LIB/version match | Fail before Cargo/Tauri. |
| MSVC env overrides owned Rust/Cargo controls or mutates parent/system state | Contract regression; abort. |
| macOS signing configuration is missing/unsafe | Fail preflight before build/sign/launch. |
| Keychain session restores before detached/nested signing completes | Fail lifecycle; signing authority must remain available until owner cleanup. |
| App/helper/plist/signature/link/rpath verification fails | Do not launch the development app. |
| Cargo runner receives application args or injected target/wrapper/env | Reject before compile/launch. |
| Development flow claims notarization, Release trust or foreign-host acceptance | Evidence regression; keep those gates separate. |

## 5. Good / Base / Bad Cases

- **Good:** native Windows resolves the reviewed `pnpm.exe`, loads one complete
  matching-architecture VS environment into the child only, then starts the
  fixed current-host Tauri plan.
- **Good:** `mise run dev` on macOS/Linux/Windows owns the process group/tree;
  first Ctrl+C begins graceful shutdown and a second exits immediately with
  code 130.
- **Good:** macOS preflight keeps its cache-keychain session through detached
  compilation and nested bundle signing, verifies the assembled app, restores
  the prior keychain state and terminates the whole group on Ctrl+C.
- **Base:** Linux executes ordinary direct pnpm/current-host tasks and never
  invokes Windows MSVC or macOS signing owners.
- **Base:** a macOS developer runs standalone preflight; it restores keychain
  state immediately because no detached dev session follows.
- **Bad:** choose `pnpm.cmd`, set `shell:true`, parse localized `set`, merge VS
  env into `process.env`, hard-code x64, store signing secrets in Git, or
  launch an unsigned/unverified debug binary outside the app bundle.
- **Bad:** fix only Darwin process teardown, use `platform !== "win32"` as an
  implicit POSIX branch, or let a Windows GUI child survive terminal close.

## 6. Tests Required

- Executable-resolution tests require `pnpm.exe` only on Win32, bind both
  reviewed mise lock assets/digests, preserve direct non-Windows execution and
  reject `.cmd`, shell and command-string fallbacks.
- Task metadata tests require exactly the three raw interactive tasks.
  `executeTauriTask({operation:"dev"})` must call the foreground runner.
- Process tests cover every signal-code mapping, first/repeated interrupt,
  child signal exits, Windows `taskkill.exe /pid /t /f`, Darwin/Linux `-pid`
  signalling and unsupported-host throw. Supported-platform tests reject
  implicit non-Windows branches; installer tests reject NSIS `taskkill`.
- The macOS helper-build source must trap INT/TERM while the outer runner owns
  final status mapping.
- Windows MSVC tests cover VS 2022/2026 selection, x64/ARM64 component mapping,
  exact closed `cmd.exe` argv, architecture derivation, JSON environment parse,
  required variables/version matching, parent-env immutability and additive
  merge protection for every owned Rust/Cargo control.
- Missing-tool fixtures prove bounded guidance and zero installer/elevation
  children.
- macOS signed-dev tests cover configuration mode/path confinement, temporary
  extraction cleanup, cache-keychain permissions, keep/restore lifecycle,
  idempotent restore, smoke signing, helper/client/plist assembly, inside-out
  signing, signature/link/rpath verification and no notarization.
- Cargo runner tests reject forwarded application args and every target,
  wrapper, runner, linker, flag, DYLD/NODE/privileged-artifact injection.
- Foreground lifecycle tests prove Ctrl+C terminates the full POSIX group and
  Windows process tree through the task-runner owner.
- Matching-host execution remains required. Portable unit tests do not prove
  Visual Studio, Keychain, signing identity or native launch behavior.

## 7. Wrong vs Correct

### Wrong

```js
spawn("pnpm.cmd", args, { shell: true });
Object.assign(process.env, parsedVsEnvironment);
if (platform !== "win32") process.kill(-pid, "SIGKILL");
```

```text
mise run dev -> launch target/debug/fyagent directly
repo -> developer-signing.p12 + password
```

### Correct

```text
win32 -> reviewed pnpm.exe -> fixed argv
  -> validated VS instance/component
  -> closed cmd /d /s /c VsDevCmd + Node JSON env
  -> additive child-only merge
  -> fixed current-host Cargo/Tauri command

interactive dev -> foreground child
  -> win32 taskkill tree | reviewed POSIX process-group signals | throw
  -> standard signal exit code
```

```text
local mode-0600 signing configuration
  -> keep-session cache keychain
  -> helper/client + Tauri bundle
  -> inside-out sign and verify
  -> launch bundle executable
  -> restore original keychain/session state
```
