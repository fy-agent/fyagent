# OpenAI first-party login constants

> Captured 2026-09-03 from `openai/codex` commit
> `36984da4424cb91b6bc88c6af8d73207930ac729` (Apache-2.0).
> Used to freeze FyAgent's OpenAI managed-auth adapter. No tokens, codes,
> verifiers, or user paths.

## Browser loopback PKCE

Source: `codex-rs/login/src/server.rs`, `codex-rs/login/src/pkce.rs`.

| Item | First-party value |
| --- | --- |
| Client ID | `app_EMoamEEZ73f0CkXaXp7hrann` (same as current FyAgent Device Code) |
| Issuer | `https://auth.openai.com` |
| Authorize | `{issuer}/oauth/authorize` |
| Token | `{issuer}/oauth/token` |
| Preferred port | `1455` |
| Fallback port | `1457` (Hydra allow-list) |
| Bind address | `127.0.0.1` |
| Redirect URI | `http://localhost:{port}/auth/callback` |
| Callback path | `/auth/callback` |
| PKCE | S256; 64-byte verifier; SHA-256 then base64url |
| State | 32 random bytes, base64url |
| Scope | `openid profile email offline_access api.connectors.read api.connectors.invoke` |
| Extra query | `id_token_add_organizations=true`, `codex_cli_simplified_flow=true`, `originator=codex_cli_rs` |

FyAgent differences (intentional):

- Unknown processes occupying `1455`/`1457` are never cancelled or killed.
  First-party may send `GET /cancel` to a previous login server of the same
  process; FyAgent only cancels its own in-process session.
- Authorization URL, code, state, and verifier never enter ordinary logs.
- Token exchange and SecretRef admission happen after the one-shot callback,
  not inside the HTTP handler.

## Device Code

Source: current FyAgent `CodexOAuthManager` (already aligned with OpenAI
`/api/accounts/deviceauth/usercode` and `/api/accounts/deviceauth/token`).
The HTTP helpers move into `services/managed_auth/providers/openai.rs`;
the JSON store manager calls them and does not keep a second protocol.
