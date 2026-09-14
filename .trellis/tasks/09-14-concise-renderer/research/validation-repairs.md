# Completion-gate repair review

Reviewed 2026-09-14 after the user's request to resolve every reported blocker.
This supersedes the earlier proposed baseline-failure exception. No remote write,
real login, CLI installation, live inference or visual-baseline approval is part
of this work.

## Test lifecycle: reproduced failure and fix

The renderer configuration has `restoreMocks: true`; its console guard was
installed in `beforeAll`. Two new guard tests both failed before any repair:
Vitest restored the spy before the tests and the warning was merely printed.
Moving installation to `beforeEach` retains the same exact warning rejection,
while reinstalling it after the runner's mock restore. Other console messages
still go to the original console. The install-target test also ended immediately
after resolving an action-job Promise. It now awaits the next installation-target
button after readiness/inventory readback and verifies configuration stays
disabled because the fixture still reports not-installed. No product timing or
state machine was modified.

Primary references (consulted in the adopted tool's version where available):

- https://v3.vitest.dev/config/#restoremocks — restores spies before each test;
  this repository uses Vitest 3.2.7 rather than inferring behavior from newer docs.
- https://react.dev/reference/react/act — flush pending updates; prefer awaited
  asynchronous interactions. React Testing Library wraps its helpers with act.
- https://testing-library.com/docs/dom-testing-library/api-async/ — await the
  actual DOM condition with findBy/waitFor rather than assuming Promise completion.

The guard's negative run is `node_modules/.cache/concise-renderer/guard-red.log`;
the corrected complete renderer run is `renderer-guard-first.log` (680 tests,
no unexpected warning). These are local generated logs, not published artifacts.

## Reviewed platform source identity

`git diff 38b4a986..b2cdc098` proves the five stale inventory entries came from
the two already-committed CLI changes, not the UI patch. Each changed source
was reviewed before changing its individual digest:

| File under src-tauri/src/services | Reviewed change and preserved boundary |
| --- | --- |
| tooling.rs | macOS login/process PATH reads moved into one macOS cfg block; Windows still uses the frozen Shell-user paths and local-path filtering. Development Grok npm commands use live resolved plans; formal builds still return through the ordinary-user helper before that branch. |
| tooling/claude.rs | npm metadata replaces bundled version data; selected installation follows PATH default; Windows execution still uses the established helper, closed plan and reobservation. |
| tooling/grok.rs | PATH-default owner determines distribution; resolved exact npm manifests replace compiled versions; Windows command formatting consumes reviewed plans rather than renderer strings. |
| tooling/lifecycle.rs | Windows-only batch wrapper preserves call and nonzero exit propagation; no new renderer command or scope was introduced. |
| tooling/versions.rs | existing macOS/native versus non-macOS dispatch remains; registry fallback replaces compiled versions. The unnecessary let-and-return introduced by that commit is removed without changing the expression. |

The stale Codex Windows test required a macOS cfg directly adjacent to a helper
call. The actual helper calls now live inside the same macOS-only block. The
replacement assertion checks that complete formatted function/block, both PATH
sources and no matching call/read in other branches. Mutating the guard to admit
Windows, moving or copying the process PATH read outside must fail. The scanner itself,
all candidate/mode/digest checks and its negative fixtures remain unchanged.

## Additional native build findings

The first full backend run failed Clippy on three Windows-only npm command
adapters compiled into the ordinary macOS library, plus a redundant let-return.
The adapters and their two exclusive imports now have matching explicit
`cfg(any(target_os = "windows", test))`. Their existing pure unit tests remain
available on the host; no warning allowance was added. This makes grok_npm.rs a
new platform-sensitive candidate, so its fully reviewed source is added to the
identity inventory. Entries use the existing English locale comparator and
SHA-256 of final rustfmt-checked bytes. No bulk refresh or checker exemption.
The modified Codex Windows contract test is itself a sealed source; its reviewed
negative-case additions are included with the final formatted test digest.

Owning contracts: frontend/quality-guidelines.md, backend/claude-code-cli.md,
and backend/task-runner-contract.md (identity seals and exact active-task gate).
The latter is read directly; it is not injected as an oversized context file.

The later production-frame measurement failure and controlled trace-recorder
experiments are documented in `performance-instrumentation.md` alongside this
file. No product motion or benchmark threshold was changed to resolve it.

## Native fixture port conflict

The next complete aggregate repeat (`prearchive-final.log`) exposed another
pre-existing fixture defect: `update_current_claude_desktop_provider_syncs_profile_when_proxy_takeover_is_active`
failed with `Address already in use` while starting the default 15721 listener.
The specific competing listener was no longer present when inspected; its origin
is not inferred. Tests immediately beside it already use `listen_port: 0`.

Only that existing test now uses the same `ProxyConfig` ephemeral-port input,
asserts a nonzero port from the real start result, compares the complete native
profile gateway URL to that result, and explicitly stops its listener afterward.
Backup sentinel, auth scheme and model-routing assertions are unchanged. This
removes the fixed-port assumption rather than retrying, probing/releasing a free
port, skipping the test or terminating an unrelated process. The source file's
reviewed identity seal is updated for these test-only bytes; production Provider
behavior and the scanner are unchanged.

Primary semantics: https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html
and https://doc.rust-lang.org/std/net/struct.TcpListener.html specify that binding
port 0 lets the OS allocate the bound port, retrievable via `local_addr`. FyAgent's
existing proxy owner already exposes it in its start result; no new helper is
needed. The prevention rule is in backend/proxy-runtime.md, Tests Required.
