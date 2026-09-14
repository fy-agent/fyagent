# Claude Code CLI Lifecycle

## 1. Scope / Trigger

Read before changing Claude CLI detection, installation, updates, Agent
surface policy or the ordinary-user Windows helper. Main owners are
`services/tooling/claude.rs`, macOS `tooling/npm_runtime.rs`,
`user-helper/src/{claude,cli,windows}.rs`, and the Windows Claude adapter at
`user-helper/src/windows/claude.rs`.

[External Agent Lifecycle](./external-agent-lifecycle.md) owns inventory,
jobs and IPC. [External Agent Auth](./external-agent-auth.md) owns official
login/logout observation. [External Agent Product Sources and Desktop
Identity](./external-agent-sources.md) owns shared Claude/Grok live npm
metadata, mirror/integrity admission and compact exact-version plans. No new
OAuth implementation or package dependency is needed for this lifecycle.

## 2. Signatures

```text
Agent claude-code: surface=cli, action=install|update
run_claude_cli_lifecycle("install"|"update") -> closed lifecycle error/result
get_tool_versions(["claude"]) -> ToolVersion

OfficialNpmTool = Grok | Claude
GrokNpmInstallPlan::npm_argv_for(tool) -> closed exact-version argv
external source owner:
  grok_npm::resolve_published_manifest(Claude)
    -> live root/current-platform manifest
  grok_npm::registries_matching_manifest(manifest)
    -> admitted registries
fetch_npm_latest_for_tool(package, tool, local) -> dist-tags.latest or /latest version
build_tool_search_paths(tool) -> login PATH + process PATH + product env
default_install(installs) -> PATH default, else the sole entry

Windows: claude-tool --action observe|install|update --job-id <uuid> --pipe <nonce>
UserHelperAction::ClaudeTool { action } -> independent wire identities 15–17
```

The renderer supplies only Agent/action and existing opaque inventory fields.
It never supplies a package, registry, version, path, command or installer URL.
`tool_host_missing` and `tool_owner_unsupported` are closed Agent reason codes.
Compact readiness is `surface=cli`, `sourceKind=cli_tooling`, no `surfaces`
array. The renderer parser in `agent-install-readiness.ts` must admit that
shape; requiring `managed_desktop` or treating `cli` as illegal fails the
Agent directory scan as 「读取失败」 without reaching install. The existing
compact npm-plan control and authenticated helper session are reused; no
generic command execution capability is added.

## 3. Contracts

### CLI-only product boundary

- `claude-code` defaults to CLI. Install/update are legal; Desktop and launch
  are rejected before source lookup or side effects. The old Claude Desktop
  download resolver is removed. Existing user data and legacy stored Desktop
  configuration are not deleted or migrated by this change.
- Detection uses Tooling, not `Claude.app`, a configuration directory or
  a desktop source. Unknown/conflicting ownership does not mean absent.
- Fresh install requires absence. Update requires one confirmed official npm
  installation and its actual global prefix. Native, Homebrew and other
  package-manager ownership is not silently converted to npm. A same/newer
  installed version is not downgraded to an older published version.

### CLI discovery

- macOS search directories are the login-shell PATH, the current process
  PATH, `~/.local/bin`, and product env (`GROK_BIN_DIR` / `~/.grok/bin` for
  Grok). Do not walk mise, nvm, fnm or Volta internal install trees, and do
  not add per-manager adapters to make those trees visible. If the CLI is on
  the user's PATH, that is enough.
- `is_mise_dispatcher` stays in the health filesystem probe. Enumerate must
  not use it to drop PATH hits.
- `default_install` prefers the PATH-default entry. Several copies may exist;
  that is not `tool_owner_unsupported` when one of them is PATH default. No
  PATH default and more than one copy remains unsupported.
- Windows still uses the existing manager search plus the ordinary-user
  helper; it does not copy the macOS env-only rule onto Alice's PATH.

### Claude projection of the shared npm source

- Source resolution consumes the exact manifest and admitted registries from
  [External Agent Product Sources and Desktop Identity](./external-agent-sources.md).
  Claude lifecycle code must not reimplement mirror ordering, metadata bounds,
  current-platform package selection or root/platform integrity comparison.
- Directory `latest_version` uses `fetch_npm_latest_for_tool`: npmjs packument
  `dist-tags` first, then the shared bounded `/latest` fallback. Claude may
  consider `next` only when the local version is already newer than `latest`.
  Display and install may race; install resolves a fresh exact manifest.
- Claude npm argv contains `@anthropic-ai/claude-code@<exact-version>`, the
  admitted general registry and the matching `@anthropic-ai:registry` option
  for that invocation. Global/user npmrc and shell profiles are not rewritten.
- Claude retains optional dependencies. npm versions requiring script policy
  receive only `--allow-scripts=@anthropic-ai/claude-code`; arbitrary script
  allowance is forbidden. The root installer must produce the matching native
  optional package; process exit or a stub alone is not success.
- Require the reviewed Node major floor and Node architecture matching the
  product. Do not install Node, use sudo, switch architecture or edit PATH.
  macOS checks an already-discoverable global bin directory; update checks that
  npm's prefix matches the selected installation.
- Shared root/optional metadata comparison and npm package-integrity checks are
  distribution admission, not proof of registry-independent artifact signing,
  successful login or inference availability.

### Native execution boundaries

- macOS composes the existing bounded Tooling child-process owner and login
  PATH resolution, shared with Grok. Reobserve installation identity after
  network work. Bound execution/output and verify the final CLI version and
  npm owner. Do not replace this with an arbitrary shell command from IPC.
- Windows always uses the existing Explorer-user helper boundary, never
  elevated PATH/npm execution. Frozen user, authenticated pipe, job nonce,
  signed helper and admission ordering remain unchanged.
- Windows discovers `npm.cmd` then `npm.exe`. `.cmd` / `.bat` shims are
  launched as `cmd.exe /D /S /C call "{quoted-program}" …` via `CommandExt::raw_arg`.
  Keep this explicit shared dispatch as project policy, not a claim that
  standard-library `Command` can never start batch files. The
  [Rust process contract](https://doc.rust-lang.org/std/process/index.html)
  documents special batch handling and warns against depending on it.
  `raw_arg` bypasses standard escaping: it receives only the native-owned
  program and closed npm plan, never an arbitrary renderer command or argument.
  Static routing assertions do not prove arbitrary batch quoting is safe.
  macOS keeps the existing Tooling child-process owner and does not use cmd
  shims.
- The Windows action carries independent Claude identity, while sharing npm
  plan decoding and execution. It discovers closed Claude executable names,
  inspects npm's actual prefix, checks ownership/Node/architecture, rechecks
  before the write and verifies the installed native binary. No native-owner
  conversion or helper failure fallback is permitted.
- Claude and Grok lifecycle operations share a process-wide single-flight
  guard. An uncertain helper result does not authorize an automatic second
  install. Ordinary-user filesystem permissions remain authoritative.

## 4. Validation & Error Matrix

| Condition                                                       | Required result                                                        |
| --------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Claude Desktop surface or generic launch requested              | `surface_not_supported` / `action_not_supported`, no source or process |
| Root/platform metadata differs from the resolved latest                  | Skip source; source failure if none match                              |
| `/latest` document version is the tag `latest`                         | Reject; resolve a concrete semver first                              |
| Multiple PATH-visible copies, one is PATH default                         | Use PATH default; not `tool_owner_unsupported`                          |
| macOS discovery walks mise/nvm/fnm/Volta install trees                     | Contract regression; env/PATH only                                     |
| Node/npm missing, too old or wrong architecture                 | `tool_host_missing`; no installer                                      |
| Multiple installs, unrecognized owner or prefix mismatch        | `tool_owner_unsupported`; no conversion/write                          |
| Already same/newer npm install on update                        | No-op; never downgrade                                                 |
| npm exits successfully but CLI is stub/wrong version/unrunnable | Verification failure, not installed success                            |
| Windows helper identity/admission/result is uncertain           | Fail closed; no elevated fallback                                      |
| Windows npm is a `.cmd` / `.bat` shim launched via `Command::new(path)` | Contract regression; use quoted `cmd /C call` through `raw_arg`     |
| Any request supplies a command/package/path/registry            | Reject at the closed boundary                                          |
| CLI install succeeds                                            | Installation only; no claim of successful login or inference           |
| Renderer requires `managed_desktop` or treats `cli` as illegal  | Directory scan fails; host still emitted CLI not_installed + install   |

## 5. Good / Base / Bad Cases

Good: a verified mirror installs the exact official optional package and the
actual CLI reports the expected version. Base: a native installation remains
usable/readable but must update through its original owner. Bad: `npm @latest`,
a compiled reviewed version JSON, walking mise/nvm trees, treating
`len>1` as unsupported when PATH default exists, global mirror changes,
treating an npm success exit as runnable proof, executing a user-writable
npm from the elevated Windows parent, or a renderer parser that still expects
Claude Desktop/`managed_desktop`.

## 6. Tests Required

Run `mise run rust:test`, `mise run rust:clippy`, `mise run typecheck`,
`mise run test:unit`, and the existing user-helper/ACL/architecture suites.
Assert product-specific wire identity, closed arguments, exact package and
scope registry, narrow script allowance, platform/SRI admission, no downgrade,
owner/prefix rejection, CLI-only policy and post-install observation. Renderer
tests must parse compact `claude-code` readiness as `cli` / `cli_tooling` and
reject Desktop/`managed_desktop`. Mirror smoke uses an isolated temporary
home/prefix/cache and no login or inference.
The shared source-owner tests must parse a `/latest` document version, reject
`version=latest`, keep executable argv free of `@latest`, match root and
current-platform integrity and prove no compiled version/hash JSON exists.
Claude tests additionally require its exact scoped-registry option and narrow
script allowance. `default_install` tests prefer the PATH default over a
second copy.
Windows native helper execution and real vendor login require their own
matching-host evidence; macOS and portable tests do not establish it.
Helper contract tests must require `npm.cmd` discovery plus
`.raw_arg(&command_line)` / `call {quoted_program}` and must not accept
`Command::new(npm.cmd)`.

Host-side npm command-string adapters used only by Windows are compiled under
`#[cfg(any(target_os = "windows", test))]`, with matching conditional imports.
Pure tests still run on the development host; the unused Windows adapters do
not enter a normal macOS build. Do not add `allow(dead_code)` to silence this
ownership mismatch. Check both the normal library and all test targets with
`mise run check:backend`; portable tests do not prove Windows execution.

## 7. Wrong vs Correct

```text
wrong: install `@latest` from any mirror; npm exit 0 -> installed
wrong: compile a reviewed version/hash JSON and treat it as latest
wrong: walk ~/.mise / nvm / volta trees; treat any second copy as unsupported
wrong: run user npm from the elevated desktop process
wrong: Command::new("npm.cmd") as the helper application name
wrong: renderer surfacesForAgent(claude-code)=desktop; sourceKind=managed_desktop
correct: login/process PATH + product env -> PATH-default owner
correct: registry /latest -> exact version + integrity -> matching registry
         -> closed plan -> ordinary-user execution -> actual CLI version/owner
correct: Windows .cmd shim -> cmd /D /S /C call "{quoted}" via raw_arg
correct: renderer admits compact CLI readiness (cli_tooling, no surfaces array)
correct: Claude npm script policy allows only @anthropic-ai/claude-code
```
