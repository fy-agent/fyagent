# Execution

- [x] Reuse the isolated worktree and confirm a clean baseline (`590e845e`).
- [x] Read Trellis workflow/spec indexes and initialize through `mise run bootstrap`.
- [x] Complete and archive `09-06-reversible-config-auth` with focused evidence and spec updates.
- [x] Complete and archive `09-06-claude-code-cli` with focused evidence and spec updates.
- [x] Review cross-child user-visible copy, target/path disclosure, native permissions and no-main-checkout writes.
- [x] Record [verification and remaining platform/runtime limits](./verification.md).
- [x] Run `mise run check:prearchive --exclude-active-task .trellis/tasks/09-06-claude-cli-safe-auth` in this task's session context.
- [x] Commit implementation/SPEC in `6d8ffc9c`, then archive the parent and both children.

Final worktree cleanliness is checked after the separate archive and journal bookkeeping commits. No push or main-checkout integration is included.
