# Research — Upstream and official evidence

Reviewed: 2026-08-25

This file records product/source facts used for planning. Runtime adapters must revalidate mutable web contracts during implementation; a URL recorded here is not automatically an executable capability.

## Claude Code

Official evidence:

- https://code.claude.com/docs/en/setup
- https://code.claude.com/docs/en/cli-reference

Current documented behavior:

- Native install is recommended.
- macOS/Linux/WSL: `https://claude.ai/install.sh`.
- Windows PowerShell: `https://claude.ai/install.ps1`; CMD: `https://claude.ai/install.cmd`; WinGet package `Anthropic.ClaudeCode`.
- Native Windows does not require Administrator.
- `claude update` is the official manual update surface.
- `claude auth login`, `claude auth logout`, and `claude auth status` are documented; status emits JSON and exit 0/1.

Reuse decision: refresh Tooling's stale Windows source knowledge and use Claude's own auth CLI. Do not read credential files.

## Grok Build

Official evidence:

- https://docs.x.ai/build/overview
- https://docs.x.ai/build/cli/reference
- https://x.ai/news/grok-build-cli

Current documented behavior:

- Official installer is `https://x.ai/cli/install.sh` for POSIX; docs expose a Windows PowerShell install path as well.
- First launch opens browser authentication.
- CLI reference documents `grok login`, optional `--device-auth`, and `grok logout`.

Reuse decision: keep existing Tooling native-install/self-update detection and invoke Grok's own login/logout surface. Authentication state remains unknown unless a stable structured status command is verified.

## OpenCode

Official evidence:

- https://opencode.ai/docs
- https://opencode.ai/docs/providers
- https://opencode.ai/

Current documented behavior:

- Installer `https://opencode.ai/install`; npm package `opencode-ai`; Windows also documents Chocolatey/Scoop.
- `/connect` manages **Provider credentials**, not a single OpenCode identity.
- Official provider docs include browser OAuth for OpenAI ChatGPT Plus/Pro and device-code OAuth for xAI SuperGrok, alongside API-key providers.
- `opencode auth list` is the troubleshooting surface for provider credentials; credentials are stored by OpenCode under its own data root.

Reuse decision: install/update through Tooling; UI action is “connect provider”, not “log into OpenCode”. Do not make FyAgent the credential store.

## QoderWork CN

Official evidence:

- https://qoder.com.cn/download

Current page explicitly lists QoderWork CN separately from Qoder CN IDE and Qoder CN CLI, with macOS 14+ and Windows 10+ support.

The user supplied the concrete QoderWork CN package URLs. Read-only inspection of the current official Next.js download bundle (`module 43399`) independently confirms that the first-party page itself maps QoderWork to these stable versionless endpoints:

- Windows x64 user/recommended: `https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe`
- macOS Apple Silicon: `https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg`
- macOS Intel: `https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-x64.dmg`
- The site also exposes a Windows system-wide x64 installer, but FyAgent should prefer the official **User** installer because the product page marks it recommended and the task owns a current-user installation experience.

Read-only `HEAD` requests on the three selected User/macOS endpoints returned HTTP 200 on 2026-08-25. All three objects were last modified on 2026-08-24. This establishes a practical first-party **latest artifact alias** without putting a version literal in FyAgent.

The separate official QoderWork CN update-log page currently tops out at `0.9.12` dated 2026-07-15, while the `/latest/` artifacts were modified in late August. The docs therefore cannot be treated as a sufficiently fresh machine version oracle. Common Electron-builder metadata candidates under the same `/latest/` prefix were also not publicly readable in this review (HTTP 403), so no stable remote semver endpoint is claimed.

Reuse decision:

- Enable QoderWork package automation from the three fixed first-party `/releases/latest/` aliases above.
- Treat the remote semantic version as `unknown` unless implementation discovers a newer documented machine-readable contract. Do not infer a version from `Last-Modified`, ETag, byte size, stale docs, or a third-party package index.
- If QoderWork is installed, FyAgent may still show the authoritatively detected **local** version and offer `更新到最新版`; it must not claim “有新版本 X” without remote version evidence.
- If the fixed latest alias is unavailable or its HTTPS/host/format contract drifts, fall back to the official QoderWork download page rather than a pinned historical package.

## TRAE Work CN

Official/community evidence:

- https://www.trae.cn/sem-work / https://www.trae.cn/work
- https://work.trae.cn/
- TRAE official Chinese community support threads direct users to the Work web app / official page to download the desktop client.

The user supplied current first-party package URLs whose path contains build `2.3.76922`. Further read-only inspection found the official machine latest endpoint used by the current TRAE website:

`https://api.trae.cn/icube/api/v1/native/version/trae/cn/latest`

The site code also carries `https://api.trae.ai` as a first-party fallback host. The endpoint requires no query parameters. Its current JSON contains separate `data.manifest` (TRAE Code) and `data.solo` (TRAE Work CN / former SOLO) branches. `data.solo` currently resolves region `cn` to exactly the three user-supplied artifacts:

- Windows x64: `.../stable/2.3.76922/win32/TraeWork_CN-Setup-x64.exe`
- macOS Apple Silicon: `.../stable/2.3.76922/darwin/TraeWork_CN-darwin-arm64.dmg`
- macOS Intel: `.../stable/2.3.76922/darwin/TraeWork_CN-darwin-x64.dmg`

Read-only `HEAD` checks returned HTTP 200 for all three packages on 2026-08-25. This independently confirms the provided URLs and removes the need to hardcode `2.3.76922`.

Reuse decision:

- Use the first-party latest API as the TRAE Work CN source resolver; select only `data.solo`, the current platform/architecture, and `region == "cn"`.
- Validate returned download URLs against an exact HTTPS allowlist and path/filename grammar for the expected TRAE Work product, platform and architecture. A successful API response is not permission to fetch an arbitrary URL.
- Derive the display/source version only from the validated `releases/stable/<version>/...` path (or a future explicit version field) and require the selected artifact to remain within the expected `TraeWork_CN-*` family. Never read `data.manifest` as Work CN.
- If `api.trae.cn` fails, the source resolver may try the observed first-party `api.trae.ai` fallback under the same bounded schema. If both endpoints/schema/allowlist fail, disable automated install/update and expose the official TRAE Work page; never fall back to a hardcoded old build URL.

## WorkBuddy

Official evidence:

- https://www.workbuddy.cn/home
- https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Installation-Windows-Guide
- https://www.workbuddy.cn/docs/workbuddy/From-Beginner-to-Expert-Guide/Installation-Mac-Guide

Official material describes Windows/macOS packages and browser/WeChat login returning to the client. The user supplied direct first-party packages for version `5.3.14.36279234`.

Read-only inspection of the official WorkBuddy website bundle found the machine update API it actually calls:

`GET https://www.workbuddy.cn/v2/update?platform=<closed-platform-id>`

Current closed platform IDs used by the site are:

- `workbuddy-darwin-x64`
- `workbuddy-darwin-arm64`
- `workbuddy-win32-x64-user`

On 2026-08-25 each request returned `version` / `productVersion = 5.3.14.36279234` plus a first-party `download.codebuddy.cn/workbuddy/saas/...` URL. The Windows URL is exactly the user-supplied `.exe`. For macOS the API returns a `.zip`; the official website then replaces the exact `.zip` suffix with `.dmg`, producing exactly the two user-supplied `.dmg` URLs. All three supplied final package URLs returned HTTP 200 in read-only `HEAD` checks. The public changelog independently records the 5.3.14 release on 2026-08-17.

Reuse decision:

- Use `/v2/update` as the WorkBuddy latest-version/source resolver with a closed platform enum.
- Require bounded JSON, a valid version, and a returned URL under the exact HTTPS `download.codebuddy.cn/workbuddy/saas/<expected-platform>/` prefix and expected WorkBuddy filename grammar.
- For macOS, mirror the official website's narrowly defined `.zip -> .dmg` transformation only after the returned URL passes the platform/path grammar; do not invent other suffix/path rewrites.
- The API's remote `sha256hash` is publication metadata and does not become FyAgent executable-admission authority under the existing one-click installer contract.
- If the update API is unavailable, malformed, returns an unapproved host/path, or the derived DMG does not exist, disable automated install/update and expose the official WorkBuddy download page. Do not pin the last observed 5.3.14 URL as a stale fallback.

Login remains WorkBuddy-owned; FyAgent launches the app/login surface and never stores its credentials.

## OpenAI Codex credential storage

Official/source evidence:

- https://developers.openai.com/codex/auth
- https://github.com/openai/codex/blob/main/codex-rs/config/src/types.rs
- https://github.com/openai/codex/blob/main/codex-rs/login/src/auth/storage.rs

Current public Codex authentication documentation defines `file`, `keyring`, and `auto`. Current source additionally contains an `ephemeral` enum/backend. The login implementation has separate file/keyring/auto storage and `auto` may fall back; source-visible `ephemeral` must be treated as non-file defensive evidence rather than a documented user-facing mode. This is materially wider than FyAgent's current `auth.json`-only projection.

OpenAI Codex is Apache-2.0, but the login/keyring implementation is an internal monorepo subsystem with its own storage abstractions and evolving behavior.

Reuse decision: use the upstream implementation as semantic reference, not as a copied private protocol. Support FyAgent projection only where a stable storage contract/API exists; file mode can reuse FyAgent's existing atomic writer, other modes fail closed unless a public integration surface appears.

## CC Switch v3.20.0 selective reuse

Evidence:

- https://github.com/farion1231/cc-switch/releases/tag/v3.20.0
- https://github.com/farion1231/cc-switch/issues/5885
- https://github.com/farion1231/cc-switch/issues/6668
- repository history/CHANGELOG for v3.20.0

Valuable behavior:

- multiple managed ChatGPT/Codex OAuth accounts;
- per-Provider account binding/unbound native behavior;
- operation serialization, login cancellation and shorter OAuth timeout;
- refresh-token reconciliation before live account switches;
- bound-account mismatch fail-closed.

Known design risk:

- Issue #5885 documents Team/Business members overwriting each other when a workspace/account identifier is used as the credential key. Current FyAgent has the same structural risk.
- Recent CC Switch work also shows that credential source and upstream destination must not be conflated.
- v3.20.0's bare Codex switch writes a full token package into `~/.codex/auth.json`; this must not be copied unconditionally because current Codex supports non-file stores.

Reuse decision: port state-machine/concurrency semantics and tests selectively under FyAgent identity; do not wholesale merge Pi or blindly copy file-storage assumptions.

