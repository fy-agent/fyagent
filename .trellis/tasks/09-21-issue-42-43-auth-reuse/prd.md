# Issues #42 / #43: account reuse and official login

## Goal

Expose the existing account reuse and official login capabilities honestly. Each
connection remains an independent native preview, confirmed save, readback and
recovery operation. Grok official CLI handoff must be reachable from `/auth`.

## Acceptance

- Only actually supported provider/consumer pairs offer managed login/connect.
- Grok CLI login/logout remains handoff-only. Device-code account storage and
  API-key model configuration have distinct names and outcomes.
- Gated Grok projection stays disabled; summaries do not infer native login,
  current model source or preserved session from a saved account.
- Target A completion survives target B failure; retry requests a fresh preview
  for B. Recovery controls are scoped to the affected target's existing files.
- Shared Agent session controller binds identity, recovers active sessions,
  rejects stale responses and pauses reads while its persistent surface is hidden.
- No real account login, vendor network call or user configuration is used in tests.

## Delivery

Local commit in `codex/next-auth-reuse-42-43`, base `2c09c4be`. Root integrates and
accepts. Native runtime/HIL/release evidence remains separate. Heavy native and
full renderer checks are deferred to root's integrated gate during release quiet
window. Root-provided NEXT_ISSUES.json / VENDOR_SOURCES.md remain untracked input.
