# Execution and validation

- [x] Read implementation-level shared/backend/frontend contracts and complete file-writer call-site inventory.
- [x] Implement shared backup/atomic/recovery safety and focused failure-injection tests.
- [x] Separate Codex auth projection from Provider switching; enforce supplied revisions and remove implicit login projection.
- [x] Add native impact/recovery transport, strict parsers and shared confirmation/recovery UI.
- [x] Test large commented Codex configs, absent provider credentials, external edits, cancel, backup failure and first creation.
- [x] Run focused Rust/V2 checks and full `mise run check`; record [verification](../09-06-claude-cli-safe-auth/verification.md).
- [x] Update owning specs before archival.
- [x] Pass `mise run check:prearchive --exclude-active-task .trellis/tasks/09-06-reversible-config-auth` with the matching session task active.
- [x] Commit work in `6d8ffc9c` and archive this child after SPEC and prearchive validation.

Review checkpoints: no token/raw output in DTOs; no renderer paths in writes; no silent keyring conversion; no backup omission on direct auth writers; no rollback that overwrites a later external edit.
