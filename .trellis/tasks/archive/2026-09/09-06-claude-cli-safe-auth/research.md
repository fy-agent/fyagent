# Validation bundle boundary

The new file-impact/recovery schemas reuse `zod/mini` from the already locked
Zod package. Official reference: https://zod.dev/packages/mini . A read-only
probe against the installed package verified strict objects, nullable values,
bounds and refinements. This keeps the same strict validation while avoiding
the classic schema method surface in the initial bundle. A separate dynamic
recovery import was rejected because it changed the existing seven-page
bootstrap graph. No dependency/version, startup-budget or route-contract
relaxation is introduced.

# Research and evidence

Observed 2026-09-06 from the isolated baseline `590e845e`.

## Local findings

- `config.rs:64-103` already owns adjacent rolling backups, but `write_json_file_with_contents`, `write_text_file` and `atomic_write` do not require them. Reuse this owner, not a competing writer.
- `managed_auth/consumers/codex/project.rs:90-162` couples credential projection to the complete Provider switch. `managed_auth/login.rs:273` supplies the reread revision instead of the displayed request's revision. Both are safety gaps.
- `pages/auth/MutationDialogs.tsx` already owns confirmation but lacks concrete filesystem impact disclosure.
- `agent_install/cli.rs` already maps Claude; `auth_actions.rs:134-139,185-213` owns fixed official login/logout and status. Lifecycle policy rejects CLI and exposes Desktop instead.
- `tooling/grok_npm.rs` and `user-helper/src/grok_npm.rs` own the reviewed registry chain, exact-version control, npm argv and host-package matching. Reuse policy/mechanisms; do not copy a second ad hoc installer.

## Primary sources

- https://code.claude.com/docs/en/setup — npm remains an official install path; modern npm packages use per-platform native optional dependencies and a postinstall link step. Node requirement must follow reviewed package metadata, not an old assumption.
- https://code.claude.com/docs/en/cli-reference — official `claude auth login`, `auth logout`, JSON `auth status` and exit status 0/1. Existing adapter agrees.
- https://code.claude.com/docs/en/authentication — official login is vendor-owned; credential storage is platform dependent. Do not promise a universal auth JSON file.
- https://registry.npmjs.org/@anthropic-ai%2fclaude-code/latest — read-only metadata observed version `2.1.261`, Node `>=22.0.0`, exact-version Darwin/Windows x64/arm64 optional packages, `node install.cjs` postinstall. Root integrity: `sha512-j6+AkfCl6/UJBcx66nlZUmWc4XGK3TscvW19Tiat+oDwkz3WqQfKzjvHO5FhR+shXTtktqs6vqSBrJmeSWpU3Q==`. This investigation lookup is not permission to use `@latest` during product installation.
- OpenAI Codex auth/config documentation distinguishes `model_provider`, provider `base_url` and `requires_openai_auth`. Official credentials can coexist with a custom request route; replacing `auth.json` does not prove a switch to official model service. Preserve that distinction in UI and readback.

## Limits

Package metadata availability is not a mainland runtime connectivity test. Current-host macOS checks are not Windows installer/helper/HIL evidence. No real user credential files or login sessions are to be changed during development tests.
