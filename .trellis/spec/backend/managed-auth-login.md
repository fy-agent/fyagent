# Managed Auth Login Session and Provider Protocol Contract

## 1. Scope / Trigger

Read this contract before changing `managed_auth_start_login`, login-session
recovery/cancellation/reopen/switch behavior, OpenAI browser loopback PKCE,
OpenAI Device Code, xAI Device Code, or the point at which a provider grant is
allowed to become a Managed Auth credential.

Primary owners:

- `src-tauri/src/services/managed_auth/login.rs`
- `src-tauri/src/services/managed_auth/login_sessions.rs`
- `src-tauri/src/services/managed_auth/providers/openai.rs`
- `src-tauri/src/services/managed_auth/providers/xai.rs`
- the login-session methods in `src-tauri/src/commands/managed_auth.rs`

[Managed Auth Core](./managed-auth.md) owns SecretRef admission, credential
metadata, refresh CAS, and legacy migration. [Managed Auth Consumers](./managed-auth-consumers.md)
owns Codex/Grok/OpenCode connection projection after a login grant has been
stored. [V2 Managed Accounts](../frontend/v2-managed-auth.md) owns renderer
polling and presentation. Do not duplicate provider protocol state in either
consumer adapters or the renderer.

## 2. Signatures

```text
managed_auth_start_login({ request })
  -> ManagedAuthLoginSessionSnapshot | ManagedAuthErrorDto
managed_auth_get_login_session({ sessionId })
  -> ManagedAuthLoginSessionSnapshot | ManagedAuthErrorDto
managed_auth_cancel_login({ sessionId })
  -> ManagedAuthLoginSessionSnapshot | ManagedAuthErrorDto
managed_auth_reopen_login({ sessionId })
  -> ManagedAuthLoginSessionSnapshot | ManagedAuthErrorDto
managed_auth_switch_login_method({ sessionId, method })
  -> ManagedAuthLoginSessionSnapshot | ManagedAuthErrorDto
```

```text
StartManagedAuthLoginRequest {
  provider: openai | xai | github_copilot,
  purpose: save_only | connect_consumer | reauthenticate,
  consumer?: codex | grokbuild | opencode | fyagent_proxy,
  method: browser_loopback | device_code,
  accountId?: ma1:<32-lowercase-hex>
}
```

Request shape is closed:

- `save_only` has no consumer;
- `connect_consumer` requires a consumer and has no account ID;
- `reauthenticate` requires a valid account ID;
- browser loopback is accepted only for OpenAI;
- GitHub Copilot with the only otherwise admissible method (`device_code`) is
  currently `provider_not_supported`; browser loopback is rejected earlier by
  the method/provider validator.

The advertised `connect_consumer` pairs are also closed:

| Provider | Admitted consumers |
| --- | --- |
| `openai` | `codex`, `opencode`, `fyagent_proxy` |
| `xai` | `grokbuild`, `opencode`, `fyagent_proxy` |
| `github_copilot` | none; provider login is unavailable |

The renderer derives these choices from `ManagedAuthProviderSummary`; it must
not synthesize a cross-provider pair. xAI repeats this compatibility check in
native admission. OpenAI's fallback purpose mapping is reserved for the
advertised `fyagent_proxy` pair, not a generic authorization for every consumer
enum. Do not widen this matrix without adding provider-side admission, purpose
mapping, projection behavior, and negative tests together.

The backend-generated snapshot contains only:

```text
contractVersion, sessionId, provider, purpose, consumer?, method, stage,
canCancel, canRetry, canSwitchToDeviceCode, officialHost,
userCode?, verificationUri?, expiresAt?, accountId?, connectionId?,
reasonCode?, terminal
```

It never contains an authorization URL, callback URL, verifier, state,
`device_code`, authorization code, token, SecretRef, native path, or raw HTTP
body. Session IDs are backend-generated UUIDs; command admission rejects a
malformed or zero-version UUID.

## 3. Contracts

### Backend-owned session lifecycle

- `LoginSessionStore` is process-private, holds at most eight retained
  snapshots, and admits at most one non-terminal session per provider. OpenAI
  and xAI sessions may coexist; a second session for the same provider returns
  `operation_conflict`. The renderer receives opaque snapshots and never runs
  OAuth polling.
- Cancel and browser-to-device switching bump the session generation. A late
  callback or poll result from an older generation must not create or replace a
  credential.
- Terminal snapshots are stable. `reopen_login` on a terminal session is a
  read-only no-op. For a non-terminal session it may reopen only the
  process-private official URL already owned by that session.
- Login success is published only after Managed Auth reserves metadata, writes
  the versioned bundle to the OS vault, performs typed readback, and marks the
  credential ready. Opening a browser or receiving a provider grant is not
  success by itself.
- After account storage, an unavailable Codex/Grok native projection ends
  `partial` with its consumer-owned reason. A successful OpenCode file write
  and readback may instead end `completed` with `pending_restart`, because the
  write is proven while live Desktop pickup is not.

### OpenAI browser loopback and Device Code

- Browser loopback is the default OpenAI method. Bind only the first-party
  registered ports `1455`, then `1457`; do not terminate an unknown listener.
  When both are busy, start Device Code instead of choosing an arbitrary port.
- The callback path is `/auth/callback`. Validate loopback host, bound port,
  method/path, OAuth state, PKCE exchange, body bounds, and callback deadline
  before accepting a code.
- Device Code polling is backend-owned and follows the server interval and
  expiry. The public snapshot may expose the bounded user code and the
  query-free HTTPS verification URI for `auth.openai.com`, but not the provider
  `device_code`.
- Browser reopen uses the process-private official authorize URL. That URL
  never crosses IPC or enters Query/route state.

### xAI Device Code

- xAI accepts Device Code only. A browser-loopback request is rejected before
  a session is created.
- Discovery, device, token, and refresh endpoints must remain HTTPS on
  `auth.x.ai:443` with empty user information. Redirected or discovered
  endpoints outside that origin are rejected.
- Polling classifies `authorization_pending`, `slow_down`, `access_denied`, and
  `expired_token`. `slow_down` increases the interval by five seconds within
  the existing cap; it does not spin or reset expiry.
- `purpose=grok_native` and `purpose=proxy_upstream` are distinct credential
  sessions. A Grok login must not copy or retag a Proxy refresh lineage.

### Error and redaction boundary

- Provider HTTP errors may expose a bounded status classification, never the
  response body, token payload, callback query, or authorization code.
- Vault unavailable and migration blocked fail before a provider worker starts.
- Unknown enum values, malformed request shapes, and invalid provider/method
  combinations fail before browser open, device request, session worker, or
  credential write. Consumer compatibility follows the closed advertised
  matrix above; do not claim a wider pair from a fallback mapping.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| vault unavailable | `secret_unavailable`; no provider worker |
| migration blocks admission | `migration_blocked`; no provider worker |
| OpenAI loopback ports `1455` and `1457` are busy | switch to Device Code; do not kill listeners or bind an arbitrary port |
| callback host/path/state/PKCE/body/deadline is invalid | fail the session; store no grant |
| user cancels while callback/poll is in flight | generation changes; late result is discarded |
| switch method is not OpenAI browser-loopback → Device Code | `invalid_response`; session unchanged |
| xAI requests browser loopback | reject before session creation |
| xAI discovery or token endpoint leaves `auth.x.ai:443` | fail closed; send no credential request to that endpoint |
| GitHub Copilot + Device Code | `provider_not_supported` |
| renderer is offered a consumer outside the provider summary | contract regression; do not submit the request |
| second non-terminal session for the same provider | `operation_conflict`; return no new session |
| OpenAI and xAI each have one non-terminal session | both may coexist; retain unique backend UUIDs |
| grant is received but SecretRef readback fails | never publish `completed`; retain the core recovery state |
| Codex/Grok native projection is unavailable after storage | `partial` with the consumer-owned reason |
| OpenCode file write/readback succeeds but live pickup is unproven | `completed` + `pending_restart`; do not relabel the write unavailable |
| snapshot/error/log contains URL secrets, codes, tokens, verifier, SecretRef, or HTTP body | security regression |

## 5. Good / Base / Bad Cases

- **Good:** an OpenAI `save_only` login binds `1455`, validates state and PKCE,
  stores the bundle, rereads it, and only then publishes a completed account
  ID.
- **Good:** both registered loopback ports are occupied, so the same request
  becomes a Device Code session without touching either process.
- **Base:** a valid xAI Device Code flow stores a separate `grok_native`
  credential, then finishes partial because Grok projection is not HIL-proven.
- **Base:** cancel succeeds while an HTTP poll is outstanding; the later grant
  is ignored because its generation is stale.
- **Bad:** return the authorize URL to React, let React poll the token endpoint,
  reuse a Proxy credential for Grok, or mark completion before vault readback.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test -- managed_auth
mise run typecheck:v2
mise run test:v2 -- tests/v2/features/managed-auth.test.ts \
  tests/v2/platform/managedAuthPort.test.ts
```

Required assertions:

- exact request combinations, the advertised provider/consumer matrix, and
  closed DTO keys/enums; xAI rejects a cross-provider pair and renderer tests
  prove that options come only from the provider summary;
- maximum-eight retention, per-provider single-flight, cross-provider
  coexistence, UUID/session lookup rejection and terminal-session stability;
- OpenAI callback host/path/state/PKCE/body/deadline validation;
- `1455` → `1457` → Device Code fallback without process cancellation;
- Device Code interval/expiry plus cancel/switch generation races;
- reopen uses the process-private official URL and the snapshot has no URL;
- xAI origin allowlist and pending/slow-down/deny/expiry classification;
- `grok_native` versus `proxy_upstream` isolation;
- SecretRef readback gates success and a failed consumer projection remains
  partial rather than completed;
- DTO/Debug/log leak scans cover callback/code/state/verifier/device/token and
  raw HTTP body fields.

Provider protocol mocks prove parser/state behavior only. Real browser,
keychain/Credential Manager, callback, and consumer pickup claims require the
matching-host HIL named by the owning contracts.

## 7. Wrong vs Correct

Wrong:

```text
renderer receives authorizeUrl + verifier
renderer polls token endpoint
grant received -> completed
cancel only changes visible UI state
```

Correct:

```text
backend owns URL, callback, polling, verifier, and generation
grant -> reserve metadata -> native vault write -> typed readback
consumer projection/readback -> completed or partial
cancel/switch bumps generation so late work cannot commit
```
