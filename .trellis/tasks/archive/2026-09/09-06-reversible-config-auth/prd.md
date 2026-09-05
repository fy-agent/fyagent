# 认证与配置修改安全闭环

## Goal

统一用户配置备份和原子写入，Codex 认证不触碰模型配置，修改前披露并提供恢复入口

## Background

`consumers/codex/project.rs` invokes a complete Provider switch from credential projection. `login.rs:273` supplies a freshly reread auth revision instead of enforcing the displayed request revision. `config.rs` already has an atomic writer and rolling backup helper, but callers opt into backup individually. `MutationDialogs.tsx` confirms actions without native file/backup disclosure.

## Requirements

- Auth-only actions never mutate Codex model configuration; keep source selection an explicit, separately disclosed operation.
- Login grants are saved without silently changing consumer files. Connecting the saved account requires a file-impact confirmation.
- All shared JSON/text user-configuration writes and direct auth projections must pass backup/atomic recovery protection. Existing domain transaction owners remain authoritative; do not duplicate them.
- A single retained exact preimage enables undo; first creation is undoable as deletion. Backup/permission/readback failure must not be reported as success. No-op writes must not rotate recovery history.
- Native-resolved path metadata is display-only. Confirm/restore requests use closed targets or opaque capabilities, never renderer paths or credential material. External edits and stale revisions fail closed.
- Preserve unrelated Codex comments/keys/providers on explicit source switching; missing third-party credentials produce no writes.

## Acceptance Criteria

- [x] Credential switch/login tests prove byte-identical untouched `config.toml` and rejection of a stale displayed revision.
- [x] Backup failure and primary replacement failure preserve primary bytes and previous useful recovery data.
- [x] Exact-preimage undo, first-creation undo, no-op retention, external drift, corrupt recovery data and restrictive permissions are covered.
- [x] Auth confirmation displays actual target/backup paths and reversible semantics; cancel performs no mutation and confirmation cannot precede successful preview.
- [x] Consumer login success does not automatically overwrite auth/config files.
- [x] Frontend strict parsers, command registration/permissions, architecture tests and owning specs match the new contract.

## Out of Scope

No OAuth token copying between consumers, keyring-store conversion, automatic user configuration repair, arbitrary filesystem API or speculative replacement of the existing Change Plan framework.
