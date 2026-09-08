# Research: Grok Auth capability matrix (macOS / Windows)

- **Query**: what is proven from first-party source vs what stays fail-closed pending HIL; helper vs native-owner fallback; refresh-owner isolation
- **Scope**: mixed
- **Date**: 2026-09-03
- **Upstream**: grok-build `72a61251fcffb464bcc687aeb5a998e5a98ec0c9` (2026-09-01)
- **FyAgent**: `ea2ecdc9efb681760fbd9b581465162384d08a25`

## Findings

Parent design §10.2 (`.trellis/tasks/09-03-unified-agent-auth-management/design.md` lines 679–703):

```text
Grok auth_provider_command
  -> shipped narrow FyAgent credential helper
  -> request valid access token for one opaque connection
  -> FyAgent remains refresh owner
```

Gate: current official Grok version, config schema, stdout contract, refresh env, signature/path, hot reload on **macOS and Windows HIL**. Helper accepts only opaque connection ID; stdout is token; stderr limited/redacted; consumer checks helper identity.

If that gate fails, fallback is **not** `grok login` handoff: independent native session, merge registry entry, **Grok** is refresh owner, `auth.json.lock` + generation reconcile; unproven platforms stay disabled.

### Matrix

Legend: **Source** = visible in grok-build or FyAgent tree. **HIL** = proven on a shipped Grok + FyAgent build on that OS. **Product now** = what FyAgent currently does.

| Capability | Source (both OS unless noted) | macOS HIL | Windows HIL | FyAgent product now | Fail-closed if unproven |
|---|---|---|---|---|---|
| OIDC discovery `https://auth.x.ai/.well-known/openid-configuration` | Yes (FyAgent + grok-build) | Not in this research | Not in this research | Used by `XaiOAuthManager` for Proxy login | Invalid issuer/host already rejected in FyAgent |
| Device Code RFC 8628 | Yes; grok-build uses `{issuer}/oauth2/device/code`; FyAgent uses discovery device endpoint | Not in this research | Not in this research | Auth Center / `xai_oauth` Device Code | Pending/slow_down/deny/expiry classified |
| Same public client_id as Grok CLI | Yes (`b1a00492-073a-47ea-816f-4c329264a828`) | n/a | n/a | Hardcoded in `xai_oauth_auth.rs` | n/a |
| Session `auth_provider_command` / `GROK_AUTH_PROVIDER_COMMAND` | Yes; Unix `sh -c`, Windows `cmd /C` | **No** | **No** | **Absent** (no Grok Auth helper) | Keep disabled; do not enable production helper |
| Helper stdout JSON/bare token parser | Yes (`token_output.rs`) | No | No | Not wired to Grok | Fail closed on non-token JSON |
| `GROK_AUTH_EXPIRED=1` headless refresh, 7s timeout | Yes (`external_auth.rs`) | No | No | Not implemented | Stay disabled |
| Interactive helper ~300s + stderr URL | Docs + flow comments | No | No | Not implemented | Stay disabled |
| Per-model `[auth_provider.<name>]` (in-memory only) | Yes; **not** session registry | No | No | Not used | Do not confuse with §10.2 |
| `auth.json` registry merge (scope keys) | Yes `BTreeMap` + `store_api_key` merge | No | No | **Read-only** quota parser; **no write** | Native write stays disabled until HIL + lock |
| `auth.json.lock` flock, never unlink | Yes `fs2`; Unix inode check; Windows `still_live`/`is_process_alive` are no-ops | No | **Weaker in source** | No lock usage | Native writer disabled; Windows cannot prove inode identity from source |
| Atomic `auth.json` write + Windows pre-delete rename | Yes | No | No | Not used for Grok home | Disabled |
| OIDC refresh rotation + generation | Yes; lock comments “RT used once” | No | No | FyAgent rotates **its own** `xai_oauth_auth.json` / vault only | Do not copy RT into Grok store |
| Hot reload of external `auth.json` writes | Documented in 02-authentication.md | No | No | n/a | Do not assume running Grok picks up writes |
| `GROK_HOME` | Official env; default `~/.grok` | No mapping test | No mapping test | FyAgent uses `settings.grok_config_dir`, **not** `GROK_HOME` | Multi-home targeting unproven |
| Agent observation of Grok login state | Spec: no structured status | n/a | n/a | `HandoffOnly` | Unverified; not an account observation |
| FyAgent user-helper `grok-tool` | Observe/Install/Update only | Install HIL is lifecycle task | Install HIL is lifecycle task | Install owner | **Not** an Auth helper |
| Mainland npm install | Spec + `a189ff40` / `0f766831` / `c7cd6906` | Lifecycle | Lifecycle | Tooling owner | Auth must not own; CLI install ≠ login |
| `auth.x.ai` reachability on mainland networks | Parent listed as HIL item | No | No | Device Code uses outbound proxy if configured | Do not invent mirrors |

### Helper vs native-owner (what source proves)

**Proven from source**

- Grok will execute a configured command string through the **platform shell**, capture stdout as a session credential, persist `AuthMode::External` into the **registry map**, and on headless refresh re-run with `GROK_AUTH_EXPIRED=1` instead of presenting the stored `refresh_token` to the IdP.
- A FyAgent-shipped helper that only prints an access token (and optional `expires_in`) can keep the **OAuth refresh token inside FyAgent** while Grok treats the helper as the mint source — **if** Grok is configured to use that command and **if** HIL shows Grok actually calls it for login and mid-session refresh without also doing OIDC refresh on a copied RT.
- Native fallback (write a full `GrokAuth` OIDC entry with `refresh_token` + `oidc_client_id` + `oidc_issuer` under the correct scope key, under `auth.json.lock`) would make **Grok** the refresh owner of that lineage. Source shows Grok’s OIDC refresher persists rotated tokens into the same map.
- Windows helper spawn is `cmd /C`, not Unix `sh`.

**Not proven (must stay disabled pending HIL)**

- That current packaged Grok on macOS/Windows honors `auth_provider_command` from user `config.toml` vs managed/requirements layers the same way as this snapshot.
- That a codesigned FyAgent helper path is accepted (identity, Gatekeeper, SmartScreen, argv0).
- That Grok will not also refresh an OIDC entry if both helper and OIDC material exist (precedence: helper > enterprise OIDC > browser OAuth — documented, not HIL).
- That hot reload works without restart after FyAgent writes `auth.json`.
- That Windows flock + always-true `still_live` is safe for FyAgent as a concurrent writer.
- That FyAgent’s existing **install** helper can be reused as the Auth helper (source: it cannot; actions are observe/install/update only). Parent says reuse trusted helper **packaging**, not the grok-tool install protocol.

Current Agent path remains `HandoffOnly` (`auth_actions.rs` + `external-agent-auth.md`). That is the live product behavior, not the §10.2 target.

### Refresh-owner isolation (Grok vs FyAgent Proxy)

**Proven from current FyAgent source**

- Proxy xAI sessions migrate and resolve as `purpose=proxy_upstream`, `refresh_owner=fyagent`, consumer FyAgent Proxy (`migration.rs` `parse_xai_store`; `service.rs` `purpose_for_provider` / `resolve_credential_access`).
- Resolver returns `Conflict` if purpose is not proxy/copilot or owner is not `fyagent`. Native owners including `grok_native` cannot be refreshed by Proxy (spec `managed-auth.md`; test `resolver_rejects_native_refresh_owner` uses CodexNative as the stand-in).
- Grok CLI `auth.json` is a **different file** from `xai_oauth_auth.json`. Quota reader does not call refresh. Manager/vault refresh only the FyAgent store.
- Schema already allows a **separate** credential with `purpose=grok_native` / `refresh_owner=grok_native`; nothing in this tree writes that row yet.

**Proven from grok-build source**

- One rotating refresh token is serialized by `auth.json.lock` because reuse revokes siblings.
- External helper JSON may store a `refresh_token` “for reference” while Grok still refreshes by re-executing the command. Copying FyAgent’s RT into that field would create a second refresh actor.

**Must remain true for this child (parent PRD out of scope)**

- Do not copy one refresh-token lineage into both Proxy vault and Grok `auth.json`.
- UI may show one identity with two connections; backend sessions stay separate.
- Helper path: Grok must not receive the Proxy refresh token; it receives short-lived access (or helper mint) only.
- Native-owner path: Grok’s session is a **new** grant (or an explicit lease transfer), not the Proxy RT.

Same **client_id** as Grok CLI does not imply a shared lineage. Each Device Code grant mints its own refresh token. Lineage is shared only if the **same refresh token bytes** are written to two stores.

### Related Specs

- `.trellis/tasks/09-03-unified-agent-auth-management/design.md` §10.2, §10.4
- `.trellis/spec/backend/managed-auth.md`
- `.trellis/spec/backend/external-agent-auth.md`
- `.trellis/spec/backend/external-agent-lifecycle.md`

## Caveats / Not Found

- No macOS or Windows HIL log for Grok Auth exists in this child directory.
- Official Grok desktop app (if any) vs CLI TUI behavior was not separated; evidence is grok-build CLI/TUI source.
- Whether `conversations:*` / `workspaces:*` extra scopes on official default vs FyAgent’s shorter scope affect Device Code success is not live-tested.
- `907c0a63` is OpenCode Windows install, not Grok Auth; Grok npm install is `a189ff40` and follow-ups.
