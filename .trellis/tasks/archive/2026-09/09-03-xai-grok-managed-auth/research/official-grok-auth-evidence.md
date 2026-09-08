# Research: Official grok-build auth evidence

- **Query**: first-party xAI/Grok Build Apache-2.0 OIDC, Device Code, auth.json schema, refresh rotation, locks, external auth command, GROK_HOME; license; Mainland install vs Auth
- **Scope**: mixed (GitHub `xai-org/grok-build` + FyAgent lifecycle specs)
- **Date**: 2026-09-03

## Findings

### Exact upstream pin

| Item | Value |
|---|---|
| Repo | https://github.com/xai-org/grok-build |
| Branch examined | `main` via GitHub API `commits/main` |
| Commit | `72a61251fcffb464bcc687aeb5a998e5a98ec0c9` |
| Commit date | 2026-09-01T22:20:33Z |
| Author | `grokkybara[bot]` (“Synced from monorepo”) |
| License file | `LICENSE` — Apache License 2.0, “Copyright 2023-2026 SpaceXAI” |
| Third-party | README points at `THIRD-PARTY-NOTICES` (not fully re-read this refresh) |
| Unchanged vs parent | Parent research already pinned this same SHA on 2026-09-03 |

No tokens or user home paths are recorded here. Public OAuth **client_id** `b1a00492-073a-47ea-816f-4c329264a828` is first-party source (obfuscated string in `config.rs`), also already used by FyAgent.

### License boundary

- **Usable as protocol authority**: `xai-org/grok-build` Apache-2.0.
- **Not usable as code source without written commercial license**: `jlcodes99/cockpit-tools` CC BY-NC-SA 4.0 (parent research). This child must not copy that tree.
- Prefer current FyAgent `xai_oauth_auth.rs` + this grok-build commit.

### Files Found (upstream)

| Path (in grok-build @ `72a61251`) | Description |
|---|---|
| `crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md` | User-facing auth methods, helper stdout contract, hot reload, precedence |
| `crates/codegen/xai-grok-pager/docs/user-guide/05-configuration.md` | `GROK_HOME`, `[auth] auth_provider_command`, OIDC env |
| `crates/codegen/xai-grok-shell/src/auth/config.rs` | Issuer, client_id, scopes, `auth_provider_command` env |
| `crates/codegen/xai-grok-shell/src/auth/model.rs` | `GrokAuth` / `AuthStore` / `AuthMode` / expiry |
| `crates/codegen/xai-grok-shell/src/auth/storage.rs` | Read/merge/write `auth.json`, 0600, atomic rename, corrupt backup |
| `crates/codegen/xai-grok-shell/src/auth/manager/lock.rs` | `auth.json.lock`, flock, never unlink, heartbeat |
| `crates/codegen/xai-grok-shell/src/auth/device_code.rs` | RFC 8628; hardcoded `{issuer}/oauth2/device/code` |
| `crates/codegen/xai-grok-shell/src/auth/oidc/protocol.rs` | Discovery `{issuer}/.well-known/openid-configuration` |
| `crates/codegen/xai-grok-shell/src/auth/oidc/refresh.rs` | Refresh grant; rotation; suspend/reuse detection |
| `crates/codegen/xai-grok-shell/src/auth/external_auth.rs` | Session helper: `GROK_AUTH_EXPIRED=1`, 7s refresh timeout |
| `crates/codegen/xai-grok-shell/src/auth/token_output.rs` | Shared stdout parser (bare token or JSON) |
| `crates/codegen/xai-grok-shell/src/auth/auth_provider.rs` | **Different** per-model helper; tokens stay in memory, never `auth.json` |
| `crates/codegen/xai-grok-shell/src/util/subprocess.rs` | `shell_c`: Unix `sh -c`, Windows `cmd /C` |
| `crates/codegen/xai-grok-config/src/paths.rs` | Re-exports `grok_home()` from `xai_dirs`; `$GROK_HOME/bin/grok` |

### Code Patterns

#### OIDC discovery

Login/refresh discovery (browser/OIDC path) fetches `{issuer}/.well-known/openid-configuration`, caches 1h, retries twice (`oidc/protocol.rs` `discover` / `discover_once`, ~lines 257–317). Discovery struct used there has `authorization_endpoint`, `token_endpoint`, `jwks_uri` — **not** `device_authorization_endpoint`.

Issuer constant: `XAI_OAUTH2_ISSUER = "https://auth.x.ai"` (`config.rs` line 122). Local-dev issuer `http://localhost:22255` when `GROK_LOCAL_AUTH` is set.

#### Device Code

`device_code.rs` POSTs to `{issuer}/oauth2/device/code` (not the discovery device endpoint) with `client_id`, space-joined `scopes`, `referrer=grok-build`, headers `x-grok-client-version` and `x-grok-client-surface` (`ui`/`cli`/`headless`). Poll POSTs `{issuer}/oauth2/token` with `urn:ietf:params:oauth:grant-type:device_code`. Handles `authorization_pending`, `slow_down` (+5s), `access_denied`, `expired_token`. HTTP 404 on device endpoint is typed `NotEnabled` (fallback to loopback).

FyAgent instead uses the discovery document’s `device_authorization_endpoint` and `token_endpoint` after host allowlist. That is a protocol-shape difference, not a second client_id.

#### Client id and scopes

Default OAuth2 config (`config.rs` `GrokComConfig::default`, ~241–256):

- `client_id`: `obfstr!("b1a00492-073a-47ea-816f-4c329264a828")`
- scopes: `openid profile email offline_access grok-cli:access api:access conversations:read conversations:write workspaces:read workspaces:write`

FyAgent Device Code scope string omits the four `conversations:*` / `workspaces:*` values. Customer IdP default scopes omit `grok-cli:access` (`default_oidc_scopes`).

auth.json **scope key** for first-party OAuth2: `{issuer}::{client_id}` e.g. `https://auth.x.ai::b1a00492-073a-47ea-816f-4c329264a828` (`config.rs` `auth_scope` / `base_auth_scope`). Legacy key `https://accounts.x.ai/sign-in`. API-key key `xai::api_key`.

#### auth.json schema (`GrokAuth`)

Top-level store is `BTreeMap<String, GrokAuth>` (`model.rs`). Per-entry fields include:

- `key` (access token)
- `auth_mode`: `oidc` / `external` / `api_key` / deprecated `web_login` (`grok`)
- `create_time`, `user_id`, optional email/name/team/org/ZDR fields
- optional `refresh_token`, `expires_at`, `oidc_issuer`, `oidc_client_id`
- `coding_data_retention_opt_out` defaults **true** if missing

Lookup skips `WebLogin` entries (forces re-OIDC). Missing `expires_at` falls back to `TOKEN_TTL = 30 days` from `create_time`. Early invalidation default 300s (`GROK_AUTH_EARLY_INVALIDATION_SECS`). User guide also says credentials without server expiry fall back to 30 days; shell README still says “tokens expire after 7 days” in one place — **docs disagree**; source TTL is 30 days when `expires_at` is absent.

Writes: owner-only (`open_secure_file` / `ensure_owner_only_permissions`, documented 0600 on Unix). Atomic temp `auth.json.{pid}.{seq}.tmp` then rename; **Windows deletes destination before rename**. Disk-full falls back to in-place truncate rewrite. Corrupt JSON is renamed `auth.json.corrupt.{millis}` then recovered as empty map. `store_api_key` **merges** one scope into the existing map (does not wipe siblings).

#### Refresh rotation

OIDC refresh (`oidc/refresh.rs`): requires `refresh_token` + `oidc_issuer` + `oidc_client_id`. Discovers token endpoint, exchanges refresh grant. Terminal IdP codes: `invalid_grant` → refresh rejected; `invalid_client` → client rejected. If IdP omits a new refresh token, the old one is kept. Comment: reuse detection can revoke a sibling’s successor if a refresh is retried after suspend past 60s grace.

Lock comments (`lock.rs`, `storage.rs` `still_live`): a refresh token may be used only once; lock file is **never deleted**; held flock is never broken. Unix revalidates lock inode; **non-Unix `still_live` is always true**; **non-Unix `is_process_alive` is always true**.

#### Two different “auth provider” mechanisms

1. **Session broker** (`[auth]` / `[grok_com_config]` `auth_provider_command`, env `GROK_AUTH_PROVIDER_COMMAND`): Grok runs the command via `shell_c`, parses stdout into `GrokAuth { auth_mode: External }`, **writes `auth.json`**. Headless refresh sets `GROK_AUTH_EXPIRED=1`, 7s timeout, stdin closed (`external_auth.rs`). Docs: interactive sign-in unset env, ~300s, stderr shown. Docs: Grok refreshes by **re-running the binary**, not an OAuth refresh grant; JSON `refresh_token` is stored “for reference”. Issuer `https://auth.x.ai` makes `is_xai_auth()` true for External mode.

2. **Per-model tables** (`[auth_provider.<name>]`, `auth_provider.rs`): minted bearer stays **in memory only, never `auth.json`**. Comment: helper owns its own durable storage. This is not the §10.2 session broker.

Stdout contract (`token_output.rs`): UTF-8; exit 0; either a single-line bare token or JSON `{access_token, refresh_token?, expires_in?, issuer?}`. JSON starting with `{` that is not a token payload fails closed (e.g. `{"error":"expired"}`). Control characters rejected.

Windows shell: `cmd /C` not `sh -c` (`subprocess.rs` lines 48–63). Test comment: hardcoded `sh` previously made `auth_provider_command` silently fall back to built-in login on Windows.

Trusted config: README: provider commands honored only from `~/.grok/config.toml`, managed config, requirements — **not** project `.grok/config.toml`.

Hot reload: user guide “Grok picks up changes to `~/.grok/auth.json` automatically” — **source-documented; not HIL-proven** on shipped macOS/Windows binaries.

#### GROK_HOME

Docs (`05-configuration.md`): `GROK_HOME` overrides config directory (default `~/.grok`). `paths.rs` re-exports `xai_dirs::{default_grok_home, grok_home, user_grok_home}` and places the app at `$GROK_HOME/bin/grok` (Unix) or `grok.exe` (Windows). Tests set process-global `GROK_HOME` via `OnceLock`. Separate file `managed_config.lock` exists for team managed-config; it is **not** `auth.json.lock`.

### Mainland China vs this Auth task

Install evidence (do not treat as Auth evidence):

- Child must not break Grok official npm / Mainland **install** owned by Tooling.
- Feature commit for npm+manifest: `a189ff40c37540c13befd2a86e40a438cff23ab1` (2026-09-03 13:27:25 +0800) `feat: install Grok via official npm and add OpenCode Windows source`.
- Follow-ups: `0f766831` keep npm plans on product hosts; `c7cd6906` split platform package lookup by product OS.
- User-cited `907c0a63c5ec50fe574ae5a8a841a2cc2d7cdce9` (2026-09-03 14:36:13 +0800) message is **`fix: admit OpenCode Windows install and scan the installed target`** — OpenCode Desktop install, not Grok Auth and not the Grok npm feature itself.
- Spec: `.trellis/spec/backend/external-agent-lifecycle.md` — default fresh Grok install is `@xai-official/grok` + bundled exact-version manifest + mainland-first registry chain; **bad**: claim mainland sign-in/inference because CLI installed.
- Spec: `.trellis/spec/frontend/user-facing-copy.md` — Grok install copy must not claim sign-in/inference on mainland networks.
- Archived install research: `.trellis/tasks/archive/2026-09/09-03-remove-grok-install-opencode-windows/research/grok-mainland-npm-install.md` — no official mainland **auth** mirror; `auth.x.ai` reachability is a separate HIL item (parent research §7.5).

Auth and install remain separate owners.

### External References

- [xai-org/grok-build](https://github.com/xai-org/grok-build) — Apache-2.0 source
- [02-authentication.md @ 72a61251](https://github.com/xai-org/grok-build/blob/72a61251fcffb464bcc687aeb5a998e5a98ec0c9/crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md)
- [docs.x.ai/build/enterprise](https://docs.x.ai/build/enterprise) — four session methods table (browser OIDC, device code, external provider, API key)
- Parent: `.trellis/tasks/09-03-unified-agent-auth-management/research/auth-source-and-reuse-review.md`

## Caveats / Not Found

- Shipped npm/CLI version string on user machines (e.g. 1.0.13 vs this source snapshot) was not re-verified against npm this refresh.
- `xai_dirs::grok_home()` implementation lives outside the files fully inlined here; behavior is documented as env override + default `~/.grok`.
- Windows ACL for `auth.json` (beyond “owner-only” docs / Unix 0600) is not quoted from `open_secure_file`.
- Live `auth.x.ai` discovery JSON was not fetched (would be network/HIL).
- Mid-session hot-reload, helper identity checks, and lock contention on real macOS/Windows Grok builds remain **unproven** (parent HIL list item 3).
