# Implementation log

## Baseline

- Branch base: `a44ed49c` (`codex/ucp-integration-35-41`).
- Product dependencies: #132 SecretRef core, #130 plan ledger, #134 executor,
  #136 V2 native Change Plan UI.
- Merge/release authority is explicitly out of scope; the output is a stacked
  Draft PR with evidence.

## Implemented slice

- Added `codex_provider_upsert_and_switch` as a closed UCP operation with the
  ordered business steps `save_provider` and `set_current_provider`.
- Kept submitted API-key material and the full desired live Provider only in
  the process-private proof map. SQLite/IPC/events retain a shortened
  credential display plus safe SecretRef backend metadata only.
- Reused the existing Provider mutation lock, snapshot, atomic writer,
  compensation and readback. One admitted apply calls that writer exactly once;
  duplicate apply returns the existing job.
- Added create-only OS-keyring admission, failure cleanup, edit rotation and
  post-success old-reference cleanup. The existing switch adapter now resolves
  SecretRef-backed Providers without changing legacy Provider compatibility.
- Routed the V2 Codex quick-setup form through the shared preview, one-confirm,
  five-phase snapshot/event/polling and recovery surface. The direct mutation
  command is no longer used for this path.
- Native UAT exposed a pre-existing empty-key validation collision. Reserved
  Provider IDs are now rejected by exact equality, so an empty key retains the
  correct required-field error; a focused regression test locks that behavior.

## Native UAT

- Ran the debug macOS bundle against the isolated home
  `/tmp/fyagent-ucp63-uat.AicS6j`; production FyAgent was relaunched afterward.
- Before confirmation, independent readback found one ready plan, zero jobs,
  only the seeded official Codex Provider, no matching Keychain item, and no
  canary in FyAgent-owned files.
- The native preview showed two business steps, shortened SecretRef display,
  the external plaintext boundary, drift admission and no-replay recovery. It
  exposed no submitted key, digest or absolute path.
- One confirmation produced plan `b69ed3b6-b9b8-47f5-b53a-e0958536e127`
  and job `5139a2cb-b43e-4915-98f5-726fa3bbaceb`; the terminal snapshot was
  `succeeded/applied_restart_recommended`, revision/event sequence `7/7`, with
  all four resources matched and usage evidence `not_observed`.
- Independent readback found the SecretRef-backed Provider current, the exact
  Keychain account present, and the test canary only in Codex's external
  `auth.json`, never the FyAgent database/log or Codex `config.toml`.
- Completion screenshot SHA-256:
  `2eaa881a1859f0a8ea7b392ef54f9295f92184f8578f645dc629e04b01cfbda8`
  (121479 bytes).
- The exact test Keychain item was deleted and the isolated test home was moved
  to Trash after readback; the installed `/Applications/FyAgent.app` process
  was verified running again.
