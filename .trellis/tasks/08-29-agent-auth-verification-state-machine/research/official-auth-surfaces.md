# Official Auth surface review

## Claude Code

Official CLI reference:

https://docs.anthropic.com/en/docs/claude-code/cli-reference

- `claude auth login` signs in.
- `claude auth logout` signs out.
- `claude auth status` returns JSON; exit 0 means logged in and exit 1 means not logged in.

This is a suitable authoritative observer when output is bounded and only safe status fields are projected.

## OpenCode

Official CLI reference:

https://opencode.ai/docs/cli/

- `opencode auth login` configures a provider.
- `opencode auth list` lists authenticated providers.
- `opencode auth logout` removes a provider credential through the official CLI flow.

OpenCode authentication is provider-owned. The CLI list is preferable to reading `~/.local/share/opencode/auth.json`, which may contain API keys/OAuth tokens.

## Grok Build

Official CLI reference:

https://docs.x.ai/build/cli/reference

- `grok login` signs in.
- `grok logout` signs out and clears cached credentials.
- The reviewed reference documents `grok inspect --json`, but that command reports project configuration, not an authentication-status contract.

Therefore current Grok integration is handoff/command-only. Do not infer verified login from credential-file existence or a configuration command.

## Desktop applications

Official product guides describe browser/QR/application-owned login flows, but no stable machine-readable external status API was identified for QoderWork, TRAE Work or WorkBuddy in this review. Launching the application remains a handoff, not verification.

## Decision

Each adapter advertises its real capabilities. One product's status command cannot be generalized to another, and unsupported verification remains explicit.
