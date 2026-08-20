# Windows Shell-user runtime contract

## 1. Scope / Trigger

Read this note before changing Windows process startup, Explorer/Shell identity
resolution, per-user paths or registry access, WebView/window-state persistence,
single-instance callbacks, or the elevated command boundary. The Codex package
helper and installer staging boundary are covered separately by
[Codex Desktop Installer](./codex-desktop-installer.md); NSIS ownership and
legacy cleanup are covered by [Windows Installer](./windows-installer.md).

Windows release builds run elevated, so the account that approved UAC is not a
safe source of the interactive user's identity or directories. The runtime
therefore freezes the Explorer user's authority before any user state is read
or written. `%ProgramData%\FyAgent\runtime`, the former state/lease files, and
the authenticated activation pipe are retired and must not be recreated. The
one-operation
`%ProgramData%\FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}\v1`
package bridge is a separate Codex installer object with no state, lease, HMAC,
activation, or startup-admission role. Its application bridge module owns
normal settlement and next-elevated-creation orphan cleanup; neither this
runtime nor NSIS may reinterpret it as the former runtime tree.

## 2. Signatures

```rust
pub fn initialize_windows_user_context()
    -> Result<(), WindowsStartupErrorCode>;

pub(crate) fn require_interactive_user_context()
    -> &'static WindowsInteractiveUserContext;

pub(crate) fn user_home_dir() -> PathBuf;
pub(crate) fn user_local_app_data_dir() -> PathBuf;
pub(crate) fn user_roaming_app_data_dir() -> PathBuf;
pub(crate) fn shell_command_search_paths() -> Vec<PathBuf>;

pub(crate) fn revalidate_interactive_user_context(
    expected: &WindowsInteractiveUserContext,
) -> bool;

pub(crate) fn normalize_single_instance_args(
    args: Vec<String>,
) -> Option<Vec<String>>;

pub(crate) async fn open_http_url_as_user(
    app: tauri::AppHandle,
    raw_url: String,
) -> Result<(), String>;
```

The frozen internal value is deliberately not serializable:

```text
WindowsInteractiveUserContext {
  process_session_id,
  shell_session_id,
  canonical_sid,
  user_profile,
  user_local_app_data,
  user_roaming_app_data,
  shell_command_paths,
}
```

`RuntimePrivilegeStatus` exposes only process privilege telemetry. It does not
expose the Shell SID or paths and does not decide which user owns state.

## 3. Contracts

### Freeze the Explorer user before all user-path consumers

- Windows `main` calls `initialize_windows_user_context` before the panic hook,
  compatibility CLI parsing, Tauri construction, configuration, database,
  logging, tray, WebView, or any other user-path lookup. Failure is reported as
  one stable, non-sensitive startup code and terminates the process.
- The sole authority chain is `GetShellWindow` -> Explorer process/token ->
  Shell token session and canonical SID -> `SHGetKnownFolderPath` with that
  token for Profile, LocalAppData, and RoamingAppData plus
  `CreateEnvironmentBlock` for Alice's real `PATH`. The Explorer token carries
  `TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_IMPERSONATE`; failure to create or
  strictly parse a nonempty PATH fails startup. Command PATH entries must be
  drive-rooted on a local fixed volume; UNC, device, relative, WindowsApps,
  removable, and mapped/remote-drive entries are rejected before filesystem
  probing. There is no WTS session
  selection, process-environment, current-directory, process-profile, SYSTEM,
  or hard-coded drive fallback.
- The elevated process may belong to Bob while the Explorer Shell belongs to
  Alice. Process/Shell SID equality is telemetry, not admission. Alice's SID
  and directories own the runtime state in both the same-user and Bob/Alice
  cases.
- Missing Shell, token/session/SID failure, noncanonical SID, or any missing or
  non-absolute known folder fails closed. A later revalidation may stop a
  side-effect when the Shell session/SID drifts, but it never replaces the
  frozen context with a newly discovered account. Revalidation proves the
  session/SID/Profile/Local/Roaming identity projections only; it neither
  re-reads nor compares mutable PATH against the startup-frozen value.

### Route every Windows user path through the frozen context

- Default configuration, database, panic/log files, provider state, prompt
  files, transient database/provider/sync/skill data, Codex/Claude/Hermes/
  OpenCode data, renderer directory defaults, WebView2 data, and window state
  derive from the frozen Profile, LocalAppData, or RoamingAppData projections.
  A validated explicit application-data
  override may retain its existing product meaning, but the override itself is
  loaded from Alice's store and an ambient Bob path is never a fallback.
- Windows must not use `dirs::home_dir`, Tauri's ambient user path resolver,
  `%HOME%`, `%USERPROFILE%`, `%APPDATA%`, `%LOCALAPPDATA%`, alternate-platform
  path variables, or elevated-process user-tool variables to select per-user paths. User tool
  discovery derives candidates from Alice's frozen directories plus
  OS-resolved system locations; it does not inherit Bob's `PATH`, NVM, pnpm,
  Volta, Scoop, Hermes, OpenCode, or Codex path variables.
- Keep Alice's real PATH and supplemental discovery locations distinct. PATH
  default resolution and child PATH construction use only the frozen real PATH;
  after a candidate is selected, its containing directory may be prepended.
  Supplemental directories may discover candidates but never silently become
  Alice's PATH default.
- Shell-user registry state uses `HKEY_USERS\<canonical Shell SID>`. `HKCU`
  would address the elevated process account and is forbidden for per-user
  FyAgent policy on Windows. The only supported locations are the fixed
  `Environment` and `Software\Microsoft\Windows\CurrentVersion\Run` keys.
  Each component is opened relative to an already pinned parent with
  `REG_OPTION_OPEN_LINK`; any `SymbolicLinkValue` marker is rejected. An
  existing key returned by create-or-open is discarded and reopened with the
  no-follow option before mutation, while a newly created key is verified on
  its returned handle. Callers receive only the minimum query/set rights and
  cannot supply an arbitrary root or relative registry path.
- Credentials and non-path process settings are separate contracts. This rule
  does not turn the Shell context into a general environment-variable ban.

### Keep Tauri's hidden paths on Alice

- Only the configured `main` window has automatic creation disabled on
  Windows. At the beginning of application setup it is built from the retained
  window configuration with an explicit absolute WebView2 data directory below
  Alice's LocalAppData. Lightweight-mode reconstruction uses the same builder.
- Windows does not register the Store or window-state plugins because their
  locked implementations resolve the ambient process user's directories.
  `app_paths.json` and `.window-state.json` are read and atomically written by
  the application directly below Alice's RoamingAppData. Existing window-state
  JSON fields, normal geometry, and pre-maximize coordinates remain compatible.
  Both files have fixed read limits and are streamed only up to one byte past
  those limits, so Alice-controlled input cannot force an unbounded elevated
  allocation.
- Frozen Alice path selection precedes panic-log setup. Main-window
  construction is the first operation in application setup and precedes
  configuration, database, tray, and other business initialization. Corrupt or
  oversized optional state may fall back to the configured geometry, but must
  not redirect the path or block Shell-context admission.

### Treat single-instance transport as untrusted input

- `tauri-plugin-single-instance` is registered on all desktop platforms.
  Windows no longer owns a pre-Tauri state file, lease, capability, HMAC, or
  custom activation pipe.
- The callback accepts at most 8 arguments, at most 64 KiB per argument, and at
  most 73,712 serialized JSON bytes for the complete
  `{"version":1,"args":[...]}` envelope. It rejects control characters before
  lightweight exit, deep-link emission, or focus behavior.
- A protocol-looking argument must pass the existing scheme/version/action and
  deep-link DTO validation before the renderer receives the parsed request.
  Raw arguments and URLs are not logged. With no actionable deep link, the
  callback may only restore/focus the existing window.
- Startup and lightweight reconstruction may register the native callback
  before the renderer listeners exist. Keep only a bounded queue of validated
  semantic requests (`focus`, parsed deep link, or invalid-deep-link notice),
  mark it ready only after both renderer listeners acknowledge registration,
  and drain it once. Never queue raw argv or raw URLs.
- Plugin input can never directly start an elevated file operation, helper,
  PackageManager call, cleanup, arbitrary command, or arbitrary path access.
- The pinned Windows plugin transport uses a predictable local mutex/window
  and decodes `WM_COPYDATA` before the application callback. Application-level
  validation cannot authenticate the peer or bound malformed bytes before that
  dependency decoder. This is an accepted residual dependency risk: keep the
  callback non-privileged, retain strict post-decode limits, and re-evaluate an
  upstream fix or dependency change separately rather than treating the
  transport as a capability.

### Preserve the narrow elevated-command boundary

`formal_windows_build` remains a compile-time manifest fact. In a formal
elevated build, user CLI probing/execution stops before a user tool is launched.
The Codex model-catalog CLI fallback is skipped completely. A non-formal build
may execute Alice's discovered Codex entry only after clearing the inherited
environment and rebuilding a narrow environment from the frozen Alice paths
and OS-resolved constants. The shared child-environment builder clears inherited
variables and supplies Alice Profile/Local/Roaming/TEMP, frozen PATH (with only
the selected entry directory optionally prepended), PATHEXT, and OS-resolved
ComSpec/SystemRoot. Windows CLI, version, and cmd-shim execution all use that
builder. Shared detected-tool execution helpers enforce the
same formal-build gate themselves so internal callers cannot bypass a public
command-level check.
It no longer selects a machine runtime or requires the process SID to equal the
Shell SID. Legacy Run-value cleanup is known-name-only, runs after primary
instance admission, and is best-effort; its failure must not block startup.

### Open validated links through the interactive Explorer shell

- `open_http_url_as_user` accepts only the shared normalized HTTP(S) launch
  request. An invalid scheme, missing authority, command, executable, path, or
  arbitrary argument never reaches a platform launcher; user-facing failures
  are stable public error codes rather than the raw URL or HRESULT.
- Windows performs the launch on an STA initialized with
  `COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE`. It obtains the Explorer-
  owned `IShellDispatch2` through the desktop shell view, following the exact
  chain `SWC_DESKTOP -> IUnknown_QueryService(SID_STopLevelBrowser) ->
QueryActiveShellView -> GetItemObject(SVGIO_BACKGROUND as IDispatch) ->
IShellFolderViewDual.Application -> IShellDispatch2`.
- `GetItemObject` must first request `IDispatch` and then cast to
  `IShellFolderViewDual`; directly requesting the dual interface is not the
  supported automation boundary. `SWC_EXPLORER` is forbidden because it depends
  on an open File Explorer folder window and makes an otherwise valid link fail.
- Ordinary external-link launch passes `SW_SHOWNORMAL` to
  `IShellDispatch2::ShellExecute`, so the system browser receives a foreground-
  eligible normal-show request. The fixed Codex user-helper launch retains its
  separate empty show argument and fixed target/arguments contract.
- There is no `ShellExecuteW`, `Command::new`, `cmd`, PowerShell, arbitrary
  executable, current-process browser launch, or `window.open` fallback. If the
  Explorer COM chain is unavailable, fail closed instead of launching as the
  elevated process account or broadening the renderer input.

## 4. Validation & Error Matrix

| Condition                                                                                        | Required result                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell window/process/token/session/SID is unavailable                                            | Exit before panic logging, CLI parsing, Tauri, or user-path access with `WIN_INTERACTIVE_USER_UNAVAILABLE`.                                                                |
| Shell Profile, LocalAppData, or RoamingAppData is unavailable/non-absolute                       | Exit with the matching stable path code; never use Bob, SYSTEM, cwd, environment, or a drive fallback.                                                                     |
| Alice environment block is unavailable, PATH is absent, or strict PATH filtering leaves no entry | Exit with `WIN_INTERACTIVE_ENVIRONMENT_UNAVAILABLE`; never read or merge Bob's process PATH.                                                                               |
| Process is Bob and Shell is Alice                                                                | Continue with Alice SID and all three Alice directory projections; report process/Shell mismatch only as telemetry.                                                        |
| Frozen Shell session/SID drifts before a protected side effect                                   | Stop that side effect; do not mutate the context or select another user.                                                                                                   |
| Alice Store/window-state JSON is missing, corrupt, or oversized                                  | Use safe defaults at the same Alice path; do not consult or create Bob's app-data directories or allocate beyond the fixed read limit.                                     |
| Any fixed Alice HKU path component is a registry symbolic link                                   | Reject that operation before reading, deleting, or writing a value; never reopen the key by an unverified full string path.                                                |
| Legacy Alice Run value is absent, inaccessible, or cleanup fails                                 | Continue startup and emit only a bounded diagnostic after first-instance admission.                                                                                        |
| A protected Codex PackageBridge orphan exists                                                    | Do not use it for startup, activation, identity, or user-path selection; only the Codex application bridge module may inspect it during the next elevated bridge creation. |
| Single-instance envelope is oversized, contains controls, or has an invalid deep link            | Reject before lightweight/focus/event behavior; never log the raw payload.                                                                                                 |
| Valid envelope has no actionable deep link                                                       | Restore/focus the existing window only.                                                                                                                                    |
| No File Explorer folder window is open                                                           | Desktop-view Explorer chain still launches the validated URL; never enumerate only `SWC_EXPLORER`.                                                                         |
| Desktop background object is requested directly as `IShellFolderViewDual`                        | Treat as a contract regression; request `IDispatch` first and cast explicitly.                                                                                             |
| External link is accepted but browser would remain backgrounded                                  | Pass fixed `SW_SHOWNORMAL` for ordinary external links; helper show semantics remain unchanged.                                                                            |
| Explorer COM acquisition or `ShellExecute` fails                                                 | Return controlled `INTERACTIVE_USER_UNAVAILABLE`; do not try a command, direct shell, renderer, or elevated-user fallback.                                                 |
| Non-Windows platform                                                                             | Preserve its existing path resolver, Store/window-state plugin, and single-instance behavior.                                                                              |

## 5. Good / Base / Bad Cases

- Good: elevated Bob starts FyAgent from Alice's Explorer. One immutable
  context contains Alice's SID/Profile/Local/Roaming paths; WebView, settings,
  database, logs, registry state, and later package work all consume it.
- Base: Bob and Alice are the same account. The same Shell-token algorithm and
  path projections apply; no special current-process shortcut is introduced.
- Good: a valid second launch contains one allowlisted deep link. The callback
  validates the complete envelope and DTO, emits the parsed confirmation
  request, then focuses the existing window.
- Good: elevated FyAgent opens an HTTPS catalog link through Explorer's desktop
  automation object. The interactive user's default browser receives a normal-
  show request even when no File Explorer folder window is open.
- Bad: read `%APPDATA%`, `dirs::home_dir`, Tauri `app_data_dir`, `HKCU`, process
  `PATH`, or a user-tool home variable on Windows and call it Alice's state.
- Bad: restore `%ProgramData%\FyAgent\runtime`, treat PackageBridge as runtime
  state or an activation channel, infer a user from an active WTS session, or
  let a second-instance argument invoke helper/package/filesystem side effects.

## 6. Tests Required

- Portable Rust tests cover same-user, Bob/Alice, missing Shell/session/SID,
  noncanonical SID, each missing/non-absolute folder, immutable revalidation,
  redacted debug output, and stable error codes.
- Contract tests assert initialization precedes panic/CLI/Tauri, the formal
  process/Shell equality gate and machine-runtime implementation are absent,
  the fixed PackageBridge is referenced only from the Codex installer boundary,
  and Windows production paths do not consult ambient home/app-data or
  user-tool path environment variables.
- Renderer tests prove directory defaults and resets use the backend Shell home
  rather than the frontend Tauri path API.
- Window-state tests round-trip the existing JSON shape and preserve normal and
  pre-maximize geometry. Shared bounded-read tests cover the exact limit and
  one-byte overflow. Static/native tests prove only `main` is manually created
  and both initial/lightweight WebViews use Alice's explicit data path.
- Registry tests cover regular/missing/link components, an intermediate link,
  a final link, newly created keys, and the required no-follow reopen after an
  existing create result. Native registry-link HIL remains unexecuted. Any
  future, separately authorized runtime validation must use only disposable
  HKCU test keys when checking intermediate and final link rejection.
- Single-instance tests cover count, item, aggregate, control-character,
  malformed/unsupported deep-link, valid deep-link, no-link focus,
  renderer-readiness queuing/drain, and no privileged callback action. Never
  construct a test that treats callback validation as pre-decode peer
  authentication.
- Process-launch tests cover HTTP(S) normalization/rejection, stable public
  errors, the complete desktop-view COM source chain, the `IDispatch`
  intermediate cast, exact OLE/DDE initialization, `SW_SHOWNORMAL` only on the
  ordinary link path, and negative scans for `SWC_EXPLORER`, command
  interpreters, direct `ShellExecuteW`, and arbitrary executable fallback.
  Native acceptance must click a real Tauri catalog action and observe the
  target in the interactive user's foreground browser; process creation or a
  successful HRESULT alone is insufficient.
- A real current-host Tauri click may prove only the external-link path it
  exercises. It does not establish Windows 10/11 coverage, ARM64, elevated
  Bob/Alice, startup admission, WebView path ownership, Shell-token freezing,
  HKU/UAC behavior, or plugin-transport compatibility. Those behaviors remain
  explicit unverified residual risks and must not be described as broadly
  native-compatible or native-runtime-verified.

## 7. Wrong vs Correct

Wrong:

```text
elevated process user -> ambient profile/HKCU -> user state
second-instance argv -> arbitrary URL/path -> privileged action
missing Shell path -> process environment or C: fallback
```

Correct:

```text
Explorer token -> freeze Alice SID/Profile/Local/Roaming before Tauri
frozen context -> explicit user paths + HKEY_USERS\Alice
bounded untrusted argv -> validated deep-link confirmation or focus only
missing/drifted Shell authority -> explicit failure before side effect
```

Wrong:

```text
SWC_EXPLORER -> first open folder -> ShellExecute with default show
failure -> cmd /c start <renderer URL>
```

Correct:

```text
validated HTTP(S) -> SWC_DESKTOP automation chain -> IDispatch cast
                  -> interactive Explorer ShellExecute(SW_SHOWNORMAL)
COM failure -> controlled error with no fallback
```
