# Cursor backend repair package

- Coordinator: GPT-6 root. Executor: the open Cursor desktop app, Debug mode, observed Cursor Grok 4.6 / High / Fast. This is the user's requested product, using the existing account and settings.
- Work directory: `~/.codex/worktrees/fyagent-next-night-20260920/fyagent`. Remote must be `fy-agent/fyagent`; branch `codex/next-iteration-engineering-20260920`, initial HEAD `07519b377af57e6613050941995918804d0f2af5` plus intentional shared dirty changes. Work directly here. Never use the original `~/fyagent` checkout or an auto-created branch with older files.
- Native predecessors stopped after shared quota exhaustion. You own ONLY the paths listed below until you explicitly report writer stopped. You are not alone: preserve every existing edit, and do not reset, stash, clean, switch branches, commit, push, or create/close PRs/Issues. Root coordinates integration; Antigravity has a separate CLI worktree.

## Required results

1. P1, Issue #35: `services/provider/usage.rs::test_usage_script` currently resolves the stored SecretRef, then combines its key with caller-controlled script and base URL. A blank/masked key must not authorize sending saved credentials to a changed URL/script/template/user/target. Reuse the existing credential material/UsageTarget owner for admission; bind the effective execution parameters before resolving/using saved secrets. Keep same-target tests working and explicitly fresh credentials usable for changed targets, without silently reusing other saved secrets. Account for common-config route changes and the inference-key fallback when there is no saved usage key. Do not send real credentials or perform real user network tests.
2. P2, Issue #47: `commands/provider/live_summary.rs::codex_connection` only reads the selected model provider's base_url/wire_api. Formal config also permits top-level routes (and the selected explicit profile). E.g. `model='m'`, `base_url='https://gateway.example/v1'`, `wire_api='responses'` currently loses the actual route; a base_url-only file can become not_configured. Correct precedence using the formal readers, preserve strict value/URL validation and secret filtering, ignore inactive profiles, and test actual temporary config files.
3. Update `provider-credentials.md` to describe the already implemented versioned bundle (inference + usage secrets), masks/retention, common-target binding, native opaque drafts, and the fixed test-script admission contract. Preserve evidence boundaries.

## Ownership

- `src-tauri/src/services/provider/usage.rs` and new narrowly scoped usage test module if useful.
- `src-tauri/src/services/provider/credentials.rs`, `credentials/material.rs`, `credentials/tests.rs`.
- `src-tauri/src/commands/provider/live_summary.rs`, `live_summary/tests.rs`.
- `.trellis/spec/backend/provider-credentials.md`.
- This task's `research/cursor-backend-result.md` and focused test logs.

Other code read-only. If a necessary fix crosses the boundary, record the exact proposed path and return it to root rather than broadening silently. No renderer, installer, proxy, manifest, Cargo dependency, or global config edits.

## Workflow and acceptance

Read the applicable backend specs and the existing task artifacts; task execution, fixes, and focused tests are already authorized. Use Debug mode to form/reproduce concrete hypotheses, then implement, not just report a plan. Existing fixtures are sufficient; do not ask the human to send a real secret. All terminal commands start with `rtk`; canonical checks use `mise run` (read available task arguments). Run meaningful focused Rust tests for target changes, script changes, masks, unchanged target, fresh input, no execution on rejection, and Codex top-level/selected provider/profile precedence. Use the existing safe temporary home/memory backend infrastructure. No weakening TLS or auth checks. No global build/bundle, avoid unnecessary full-suite repetition. Root will run final integration checks.

Record exact changed files, tests/exit codes, remaining findings, actual product/model displayed, and writer-stopped status in `research/cursor-backend-result.md`. Do not claim fixture checks prove native OS keychain/HIL, release, or Windows acceptance. On a real blocker preserve artifacts and report precisely.
