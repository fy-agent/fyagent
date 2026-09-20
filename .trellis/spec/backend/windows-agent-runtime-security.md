# Windows Agent Runtime Security

## 1. Scope / Trigger

Read this contract before changing Windows Agent desktop launch, elevated
Claude/Grok lifecycle routing, ordinary-user helper actions, or Uninstall/App
Paths enumeration rights.

This owner assumes the frozen Explorer-user, hidden-path and elevation
fundamentals in [Windows Shell-user Runtime](./windows-runtime-security.md).
[External Agent Lifecycle](./external-agent-lifecycle.md) owns action legality,
inventory normalization and jobs; [External Agent Product Sources and Desktop
Identity](./external-agent-sources.md) owns release and installed-product
identity plus shared exact npm plans; [Claude Code CLI](./claude-code-cli.md)
owns Claude-specific discovery, owner and post-install verification.
This file owns the Windows execution/access boundary after those authorities
have admitted an action or observation.

## 2. Signatures

```text
InteractiveUserLaunch::trusted_windows_exe(path)
  -> TrustedWindowsExe | InvalidWindowsExe

InteractiveUserLauncher::open_trusted_windows_exe(path)
launch_trusted_windows_exe_as_user(path)
  -> Explorer ShellExecute(SW_SHOWNORMAL) | INTERACTIVE_USER_UNAVAILABLE

machine_program_files_directories()
  -> [ProgramFiles, ProgramFilesX86]

formal Windows Claude install|update
  -> claude-tool helper

formal Windows Grok observe|preflight|install|update
  -> grok-tool helper

development Windows Grok npm
  -> LocalProcess using the same live exact plan

fyagent-user-helper.exe
  codex-msix-install --job-id <uuid> --pipe <nonce>
  agent-exe-install --product qoderwork|trae-work|workbuddy|opencode
                    --job-id <uuid> --pipe <nonce>
  grok-tool --action observe|preflight|install|update [--owner native|npm]
            --job-id <uuid> --pipe <nonce>
  claude-tool --action observe|preflight|install|update --job-id <uuid> --pipe <nonce>

RegistryRights { query_value, enumerate_subkeys, create_subkey, set_value }
READ_VALUES           = query
UPDATE_VALUES         = query + set
TRAVERSE              = query + enumerate
INVENTORY_PARENT_READ = query + enumerate, no create/set

open_shell_user(Uninstall | AppPaths) -> INVENTORY_PARENT_READ
open_machine(Uninstall | AppPaths, Registry32 | Registry64)
  -> INVENTORY_PARENT_READ
open_child(validatedName) -> READ_VALUES
```

No generic command/path helper exists. Public invalid-EXE errors map to
`external_launch_invalid_windows_exe`.

## 3. Contracts

### Trusted desktop EXE launch as Alice

- Desktop identity is proven before this boundary. The launcher validates only
  a nonempty host-absolute `.exe` path (case-insensitive), with no NUL,
  `ParentDir` component or arguments.
- Launch reuses the Explorer COM adapter and `SW_SHOWNORMAL`, so the process
  starts as Alice. COM failure returns `INTERACTIVE_USER_UNAVAILABLE`; it never
  falls back to elevated `CreateProcess`, direct `Command`, Bob's
  `ShellExecuteW`, or a renderer path.
- Keep trusted EXE, trusted AUMID and directory/URL request types separate.
  A downloaded installer EXE is not eligible for ordinary launch; install uses
  the retained package and closed helper product action.
- Observation roots are frozen Alice `LocalAppData\\Programs` plus machine
  Program Files roots resolved with `SHGetKnownFolderPath(..., token=None)`.
  Tests may substitute `FYAGENT_TEST_HOME` only through the reviewed fixture.
- Non-Windows implementations fail as unavailable/unsupported. A macOS unit
  test must not pretend a `C:\\...` string is host-absolute.

### Closed CLI/helper boundary on formal elevated Windows

- OpenCode remains Desktop-only. Claude uses its dedicated closed lifecycle;
  Grok remains the sole writable generic Tooling CLI. Other generic tools fail
  before side effects.
- Formal elevated Windows never launches a user CLI or CLI-backed Auth from
  the elevated parent. Claude and Grok delegate to distinct authenticated
  ordinary-user helper actions; direct Auth remains unavailable. There is no
  generic tool/Auth/helper verb.
- Helper argv contains only the fixed action/product identity, job UUID and
  pipe nonce. After Hello, Grok npm receives the compact reviewed plan. Paths,
  URLs, shell strings, free tool names, environment, stdout/stderr, browser
  URLs, device codes and command lines never cross back to the elevated parent
  or renderer.
- The signed product host selects the current x64/arm64 Grok optional package,
  validates root and platform SHA-512 at an allowed registry, and sends only
  exact version, registry index and the narrow allow-scripts bit. The helper
  does not fetch metadata, choose Darwin/Linux packages or invent `@latest`.
- Native Grok install is an explicit closed action and consumes no npm plan.
  An absent/malformed/`@latest`/unknown-registry npm plan produces no child.
- Development Windows LocalProcess is not an alternate product policy. It
  resolves the same live exact plan, finds the npm major from the sibling or
  PATH-default `npm.cmd` that actually runs, adds only
  `--allow-scripts=@xai-official/grok` for npm 12+, treats blocked scripts as
  failure, and requires PATH-default Grok version readback. It never executes
  the `1.2.3` command-shape fixture.
- Closed desktop Agent EXE install uses its own protected package bridge.
  Trusted desktop launch is not an install bypass.

### Read-only installation preflight

The closed Grok/Claude `preflight` action uses the same pinned helper, frozen
Shell identity and authenticated action-bound controls as the existing lifecycle
operations. It accepts no caller path, URL, package, argv or executable. Its
native/npm version and fixed runtime probes plus directory access-right queries
must return before any mutation path. Prefix access is checked with the ordinary
helper token. Wire action IDs 18–20 are Grok preflight with the closed owner
choices, and 21 is Claude preflight; `ToolPermissionDenied` is bounded error 27.
A permission failure is not reported as a missing runtime. The parent preserves
identity/admission/quarantine behavior and returns actionable redacted errors.

### Inventory parent registry rights

- Intermediate fixed components use `TRAVERSE`. Uninstall/App Paths parent
  leaves use the distinct `INVENTORY_PARENT_READ` constant even when its
  current bit mask equals traverse. Validated child names open query-only with
  `READ_VALUES`.
- Win32 parent mask is `KEY_QUERY_VALUE | KEY_ENUMERATE_SUB_KEYS` plus the
  selected WOW64 view. It never includes create/set rights or the broad
  `KEY_READ` convenience mask.
- Optional parent `NotFound` is absence. A rejected registry-link on an
  optional parent is also absence: shared WOW64 keys such as machine App Paths
  may expose a `SymbolicLinkValue` in the 32-bit view for a location already
  enumerated in the 64-bit view. The link is never followed.
- Raw OS access denied, enumeration/bounds failure, or frozen Shell-context
  drift makes discovery incomplete. It must not become a false complete-empty
  scan or `not_installed`.
- Child names are length/charset validated before open. Registry values remain
  hints and are never executed. This access does not authorize WinGet,
  PowerShell, a second scanner, or writable Uninstall/App Paths.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Trusted EXE path is relative, contains `..`/NUL, has arguments or non-EXE suffix | `external_launch_invalid_windows_exe`; no Explorer call. |
| Shape-valid observer-proven absolute EXE | Explorer ShellExecute as Alice. |
| Explorer COM unavailable | `INTERACTIVE_USER_UNAVAILABLE`; no elevated fallback. |
| Downloaded installer is submitted to trusted launch | Reject; use closed install helper. |
| Formal elevated parent attempts direct CLI or CLI-backed Auth | Fail before user process. |
| Formal Grok/Claude lifecycle | Matching closed helper; no generic/elevated fallback. |
| Helper plan missing, `@latest`, malformed or registry unknown | Fail closed; no npm child. |
| Helper argv contains path/URL/shell/free tool or returns raw process evidence | Contract failure. |
| Product host lacks matching Grok platform/integrity | Produce no plan/helper npm child. |
| Development LocalProcess uses `1.2.3`/`@latest` | Contract regression; resolve live exact version. |
| npm 12+ omits narrow allow-scripts or install scripts are blocked | Fail even if npm exits zero. |
| PATH-default Grok remains below planned version | Fail the attempt; no Agent success. |
| Inventory parent opens query-only then enumerates | Discovery incomplete on real hives. |
| Optional parent missing/rejected shared-view link | Absence; continue other views. |
| Parent access/enumeration/bound/Shell context fails | Incomplete aggregate; no false absence. |
| Parent or child receives create/set rights | Contract failure. |

## 5. Good / Base / Bad Cases

- **Good:** inventory proves a closed WorkBuddy/Qoder/TRAE EXE, then Explorer
  opens it as Alice with no arguments from the elevated app.
- **Good:** formal Windows routes Grok through `grok-tool` and Claude through
  `claude-tool`; development Windows applies the same exact plan and verifies
  the PATH-default Grok version.
- **Good:** Alice/machine Uninstall and App Paths parents enumerate with
  query+enumerate while every validated child remains query-only.
- **Base:** CLI-backed Auth stays unavailable on formal elevated Windows while
  closed install/update helper actions remain usable.
- **Base:** an optional registry parent is absent or a rejected shared-view
  link; the remaining complete views can still establish absence.
- **Bad:** `Command::new(exe)`, generic `helper run --cmd`, helper metadata
  resolution, `npm @latest`, `KEY_READ`, WinGet, or treating access denied as
  “not installed”.

## 6. Tests Required

- Trusted-launch tests reject relative, `.bat`, NUL and `nested/../*.exe`
  inputs before a fake launcher runs, preserve AUMID/request separation and
  prove no installer path uses this API.
- Matching-host launch tests prove Explorer-user execution and controlled COM
  failure; portable shape tests do not claim Windows process identity.
- Formal boundary tests keep non-Grok generic tools fail-closed, route Grok to
  ordinary-user helper and Claude to its dedicated helper, and reject direct
  CLI/Auth execution.
- Static/helper protocol tests prove there is no generic verb/path/URL/raw
  stdout DTO and bind the compact plan framing to exact version, registry index
  and allow-scripts bit only.
- Product-host tests admit only current `grok-win32-x64`/`arm64`, require both
  integrities and never move metadata resolution into the helper.
- LocalProcess tests reject fixture/`@latest` argv, add allow-scripts only for
  npm 12+, preserve an existing flag, detect blocked-install-script output and
  require PATH-default version readback.
- Registry tests prove parent leaves use `INVENTORY_PARENT_READ`, child opens
  use `READ_VALUES`, masks exclude create/set, and shared-link rejection is
  distinct from raw access denied.
- Inventory projection tests distinguish complete/no-candidate from incomplete
  discovery. Alice HKU/Wow6432Node/UAC HIL remains explicit residual evidence.

## 7. Wrong vs Correct

### Wrong

```text
elevated FyAgent -> CreateProcess(observerPath)
helper run --cmd <renderer string>
development Windows -> npm i -g @xai-official/grok@1.2.3 -> exit 0 = success
open(UninstallParent, READ_VALUES) -> enum_keys -> treat access denied as empty
```

### Correct

```text
observer-proven closed EXE
  -> validate absolute no-arg .exe shape
  -> Explorer ShellExecute as Alice

formal Claude/Grok
  -> distinct closed helper action + authenticated pipe
  -> host-selected exact npm plan where applicable

development Grok
  -> live exact plan + npm-major script policy
  -> PATH-default version reaches planned version

open parent(INVENTORY_PARENT_READ)
  -> enumerate
  -> validate child name
  -> open child(READ_VALUES)
```
