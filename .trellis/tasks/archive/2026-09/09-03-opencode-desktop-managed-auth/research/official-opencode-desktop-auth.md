# Research: Official OpenCode Desktop Auth

- **Query**: Freeze official `Global.Path.data/auth.json` schema, Desktop Provider Connect, 0600, unknown-entry behavior, reload/restart, data-dir rules, sidecar non-publicness, MIT license.
- **Scope**: external (pinned `anomalyco/opencode@b578b7261fc9ec4917fe272df5cc4bd8a056cd5d`, 2026-09-03) plus official docs
- **Date**: 2026-09-03

## Findings

### Files Found

| File Path | Description |
|---|---|
| `packages/opencode/src/auth/index.ts` | Auth store: path, schema, `all`/`set`/`remove`, `0o600` |
| `packages/core/src/global.ts` | `Global.Path.data` via `xdg-basedir` `xdgData` + `"opencode"` |
| `packages/core/src/fs-util.ts` | `writeJson`: stringify + `writeFileString` + optional `chmod` |
| `packages/opencode/src/provider/auth.ts` | Provider Auth API: methods / authorize / callback → `Auth.set` |
| `packages/opencode/src/cli/cmd/providers.ts` | CLI `opencode auth` alias of `opencode providers` (list/login/logout) |
| `packages/desktop/src/main/index.ts` | Desktop sidecar: ephemeral port (`listen(0)`), `randomUUID()` password |
| `packages/desktop/src/main/sidecar.ts` | Sidecar env: `OPENCODE_SERVER_PASSWORD`, `XDG_STATE_HOME=userDataPath` |
| `packages/app/src/components/dialog-connect-provider.tsx` | Desktop/web Provider Connect UI |
| `packages/app/src/utils/server-compat.ts` | Legacy client: `auth.set` / oauth authorize+callback, then `instance.dispose()` |
| `LICENSE` | MIT, Copyright (c) 2025 opencode |

### Code Patterns

#### 1. Credential path: `Global.Path.data/auth.json`

Pinned `packages/opencode/src/auth/index.ts`:

```ts
const file = path.join(Global.Path.data, "auth.json")
```

Pinned `packages/core/src/global.ts`:

```ts
import { xdgData, xdgCache, xdgConfig, xdgState } from "xdg-basedir"
const app = "opencode"
const data = path.join(xdgData!, app)
```

`xdg-basedir` (`sindresorhus/xdg-basedir` `index.js`) has **no Windows LOCALAPPDATA branch**:

```js
export const xdgData = env.XDG_DATA_HOME ||
  (homeDirectory ? path.join(homeDirectory, '.local', 'share') : undefined);
```

Official troubleshooting (https://opencode.ai/docs/troubleshooting/):

- macOS/Linux data: `~/.local/share/opencode/`
- Windows data: `%USERPROFILE%\.local\share\opencode`
- That directory contains `auth.json` (API keys, OAuth tokens), `log/`, `project/`
- Desktop **UI** state is separate: `opencode.settings.dat` / `opencode.global.dat` under Application Support / `%APPDATA%`

Desktop Electron `userData` at pinned `packages/desktop/src/main/index.ts` is `join(app.getPath("appData"), appId)` with `appId = ai.opencode.desktop` (prod). Sidecar `prepareSidecarEnv` sets `XDG_STATE_HOME` to that `userDataPath` (`sidecar.ts`). **Auth.json uses `xdgData`, not `xdgState`.** Overriding `XDG_STATE_HOME` does not move `auth.json`.

Desktop onboarding test mode **does** set `XDG_DATA_HOME` (`index.ts` OPENCODE_TEST_ONBOARDING block). Production Desktop does not set `XDG_DATA_HOME` in the cited snippet.

`Auth.all` also short-circuits to `process.env.OPENCODE_AUTH_CONTENT` if it parses as JSON. That env is a read overlay; at this commit `set`/`remove` still start from `all()`, so writing while the env is set can rewrite the file from the env snapshot (see GitHub issue #46128; fix PRs exist on later commits, **not** in `b578b726`).

#### 2. Schema (pinned Auth classes)

OAuth:

```text
type: "oauth"
refresh: string
access: string
expires: NonNegativeInt
accountId?: string
enterpriseUrl?: string
```

API key:

```text
type: "api"
key: string
metadata?: Record<string, string>
```

Well-known (CLI URL login to `/.well-known/opencode`):

```text
type: "wellknown"
key: string
token: string
```

`Info = Union(Oauth, Api, WellKnown)` discriminated on `type`. Store is a JSON object keyed by **provider ID** (or a wellknown URL after trailing-slash normalize).

`ProviderAuth.callback` (`provider/auth.ts`):

- success with `"key"` → `auth.set(providerID, { type: "api", key, metadata? })`
- success with `"refresh"` → `auth.set(providerID, { type: "oauth", access, refresh, expires, ...extra })` after stripping `type`/`provider`

`...extra` is how `accountId` / `enterpriseUrl` (and any other leftover success fields that match Oauth) land in the file.

#### 3. Write permissions and unknown-entry behavior at this commit

`Auth.set` / `Auth.remove` call `fsys.writeJson(file, data, 0o600)`.

Pinned `FSUtil.writeJson`:

```ts
const content = JSON.stringify(data, null, 2)
yield* fs.writeFileString(path, content)
if (mode) yield* fs.chmod(path, mode)
```

This is **in-place write then chmod**, not temp+rename. Cross-process lock is not in this commit’s Auth owner. Later upstream PRs (#45949, #46131) describe atomic rename + flock; those are **not** the pinned snapshot.

`Auth.all` decodes each value with `Schema.decodeUnknownOption(Info)` and **drops** undecodable entries (`Record.filterMap` → `undefined`). `set` then writes `{ ...data, [norm]: info }` where `data` is that filtered map. **Unknown / future / corrupt provider entries are omitted from the next write.** Env-sourced credentials (`OPENCODE_AUTH_CONTENT`) and environment-variable providers listed by the CLI under a separate “Environment” intro are not `auth.json` keys.

CLI `providers list` (`providers.ts`) prints `Credentials <displayPath>` then `Name type` rows then `N credentials`, then optionally a second “Environment” block for `process.env[provider.env]`. FyAgent’s current parser only consumes the Credentials block (`auth_actions.rs`).

CLI command at this commit: `ProvidersCommand` `command: "providers"` with `aliases: ["auth"]`. So `opencode auth list|login|logout` is the alias surface FyAgent currently launches. Login writes via `Auth.set`; logout via `Auth.remove`.

#### 4. Desktop Provider Connect (no PATH CLI)

UI: `packages/app/src/components/dialog-connect-provider.tsx` uses the **connected OpenCode server SDK**, not a spawned `opencode` binary:

- API key: `serverSDK().api.integration.connect.key({ integrationID, location, key })` then `refreshProviders()`
- OAuth code: `integration.oauth.complete({ integrationID, attemptID, location, code })`
- OAuth auto: poll `integration.oauth.status(...)`

Legacy compatibility (`server-compat.ts`) maps those calls onto:

- `legacy().provider.oauth.authorize` / `.callback`
- `legacy().auth.set({ providerID, auth: { type: "api", key } })`
- then `legacy().instance.dispose()` and `input.legacy().instance.dispose()`

That `instance.dispose()` is an **in-process server instance recycle** after credential write. It is not documented as “quit the Electron app”. Whether a running Desktop session picks up an **external** `auth.json` mutation without dispose/relaunch is **not proven** by this source read.

HTTP Provider Auth routes exist on the OpenCode server (`provider.oauth.authorize` / `callback`, `provider.auth` methods). Those routes are served by the Desktop sidecar, which is **not a public control plane**.

#### 5. Sidecar port/password are ephemeral and private

Pinned `packages/desktop/src/main/index.ts` (default sidecar v1):

- Port: `OPENCODE_PORT` if numeric, else `createServer().listen(0, "127.0.0.1")` (kernel-assigned).
- Password: `randomUUID()`.
- Username: `"opencode"`.
- Ready payload `{ url, username, password }` is given to the renderer via `serverReady` / IPC `awaitInitialization`.

Pinned `sidecar.ts` `Server.listen({ port, hostname, username: "opencode", password, cors: ["oc://renderer"] })` and env:

```ts
OPENCODE_SERVER_USERNAME: "opencode"
OPENCODE_SERVER_PASSWORD: password
XDG_STATE_HOME: process.env.XDG_STATE_HOME ?? userDataPath
```

Official troubleshooting: “OpenCode Desktop runs a local OpenCode server (the `opencode-cli` sidecar) in the background.” CORS is `oc://renderer`. Parent FyAgent research already records: FyAgent does not scan or hijack this sidecar.

#### 6. How FyAgent can resolve the data dir without port scanning

From official rules + current FyAgent user context (no new policy):

1. Take the **same frozen interactive user** already used for Desktop inventory and config:
   - macOS: `dirs::home_dir()` / `FYAGENT_TEST_HOME`
   - Windows: `windows_runtime::user_home_dir()` (Explorer Shell profile), not the elevated process home
2. Apply official `Global.Path.data`:
   - If `XDG_DATA_HOME` is set and non-empty in **that user’s** environment: `{XDG_DATA_HOME}/opencode`
   - Else: `{home}/.local/share/opencode`
3. Credential file: `{data}/auth.json`
4. Inventory `targetId` selects **which Desktop binary to launch** for official Connect handoff. It does **not** encode a per-install credential directory. Multiple OpenCode.app copies for one user share one `Global.Path.data` unless `XDG_DATA_HOME` / `OPENCODE_TEST_HOME` differs.
5. Do not use Electron `userData` (`~/Library/Application Support/ai.opencode.desktop` or `%APPDATA%\ai.opencode.desktop`) as `auth.json` location.
6. Do not probe `127.0.0.1:<random>` or `OPENCODE_SERVER_PASSWORD`.

Existing helper `opencode_config::get_opencode_data_dir()` already implements (2) on macOS (honors `XDG_DATA_HOME`) and the else-branch on Windows (ignores `XDG_DATA_HOME`). Alignment with a Desktop process that *does* set `XDG_DATA_HOME` on Windows is an HIL item.

#### 7. Reload / restart at source vs HIL

Observed in source:

- Connect UI calls `refreshProviders()` after success (`dialog-connect-provider.tsx` `complete()`).
- Legacy compat calls `instance.dispose()` after `auth.set` / oauth callback.
- Troubleshooting “Authentication issues” says re-auth with TUI `/connect`. Desktop “Quick checks” say fully quit and relaunch, or macOS Reload Webview (UI freeze, not specifically auth).
- `Auth.all` re-reads the file on each call (no in-module cache) unless `OPENCODE_AUTH_CONTENT` is set.

**Not proven from source:** whether Desktop’s live sidecar watches `auth.json`, whether an external FyAgent write is visible without dispose/relaunch, or whether Windows/macOS stable builds differ. Parent design records restart as HIL-gated (`pending_restart` until proven).

#### 8. License

Pinned `LICENSE` is MIT (“Copyright (c) 2025 opencode”). Schema/path facts may be implemented from this source with MIT copyright notice if code is copied. FyAgent itself is PolyForm Noncommercial + MIT CC Switch-derived (`LICENSE` in this repo). `jlcodes99/cockpit-tools` is CC BY-NC-SA 4.0 per parent research (`09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md`); it is **not** a copy source for this slice.

### External References

- OpenCode repo (pinned): https://github.com/anomalyco/opencode/tree/b578b7261fc9ec4917fe272df5cc4bd8a056cd5d
- Troubleshooting / data dir: https://opencode.ai/docs/troubleshooting/
- `xdg-basedir`: https://github.com/sindresorhus/xdg-basedir
- Auth write races (later than pin): https://github.com/anomalyco/opencode/issues/46128
- Sidecar password inheritance: https://github.com/anomalyco/opencode/issues/24747
- Parent freeze: `.trellis/tasks/09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md` section 4.4

### Related Specs

- `.trellis/spec/backend/managed-auth.md` — later OpenCode projection slice; `refresh_owner=opencode`
- `.trellis/spec/backend/external-agent-auth.md` — Agent Auth façade still CLI; must not grow a second OAuth store
- `.trellis/tasks/09-03-unified-agent-auth-management/design.md` §10.3 — two paths: official Connect vs independent session projection; no sidecar scan

## Caveats / Not Found

- Stable Desktop HIL for actual data-dir on macOS/Windows current release is **not** in this research. Community issue #27530 mentions `%LOCALAPPDATA%\opencode\auth.json` as a second path some Windows users hit; that is **not** what pinned `Global.Path.data` computes.
- `OPENCODE_AUTH_PATH` appears in later search hits; **not** present in pinned `auth/index.ts` (path is hard-coded to `Global.Path.data/auth.json`).
- Plugin auth methods (`Hooks["auth"]`) are a plugin surface. Task out of scope: copying arbitrary plugin auth into FyAgent.
- Atomic `writeJson` + flock are **later** than the pin; implementers must re-read the then-current first-party file before copying write mechanics.
- Restart-after-external-write remains **unknown until native HIL**.
