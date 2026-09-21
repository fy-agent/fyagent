# Guard proxy exit against external configuration edits

## Goal

Implement issue #64 and the proxy recovery portion of #61: prove ownership before restoring legacy API-key takeover files, preserve conflicts and backups, make repeated and partial recovery safe, and verify recovery guidance with isolated fixtures.

## Requirements

- Preserve live files and authoritative backups when an external edit invalidates proxy ownership.
- Accept compatible legacy API-key backups with real writer evidence; never use a proxy placeholder alone as overwrite authority.
- Treat each already-restored file as complete on retry, and report incomplete restoration truthfully.
- Keep native login files outside proxy ownership and show conflict/reopen guidance on actual renderer recovery flows.
- Modify only the assigned checkout. Do not operate on real user configuration or run Rust/full browser/full frontend checks during the release quiet window.

## Acceptance Criteria

- [ ] Isolated filesystem fixtures cover ordinary restore, external edits, corrupt receipts, already-restored files, and interrupted multi-file recovery.
- [ ] Legacy backup branches no longer perform unchecked writes.
- [ ] UI guidance and actual state readback are verified with focused tests.
- [ ] Report exact executed and pending checks, Windows runtime limits, and the local delivery commit.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
