# Project resource generations — implementation handoff

Status: implementation frozen for root integration; targeted runtime check pending root's shared-worktree freeze. No commit was created.

## Changes

- `src-tauri/src/database/dao/projects.rs`: adds local-only `fde_resource_generations(kind, app_type, resource_id, generation)`, retained tombstones and canonical SQLite triggers. Provider/prompt identities include app type; MCP/skill identities are global. Insert/update/delete, identity moves and provider custom endpoint mutations increment the owning generation atomically. Integer bounds fail the source mutation closed. Existing rows seed once; repeated schema creation preserves generations and source records.
- `src-tauri/src/services/projects/resources.rs`: supplies `db:<generation>` native versions for all four resource kinds through SELECT-only DAO calls. No raw content, credentials or content hashes enter the generation table or version string.
- `src-tauri/src/services/projects/resource_versions_tests.rs`: five focused tests for owner A→B→A, delete/reinsert, scope separation, custom endpoints, predecessor migration/seed/reopen, source-write rollback, read-only total_changes, secret-neutral snapshots, identity move and exhaustion rollback.
- `src-tauri/src/services/projects/tests.rs`: imports the new test module and updates the existing prompt snapshot expectation from unverifiable to matched.

## Root integration requirements

1. Add `fde_resource_generations` to both sync export skip and import preserve sets in `database/backup.rs`.
2. Sync replacement must restore local-only generations including tombstones, then call `Database::advance_project_resource_generations_on_conn(conn)` inside its import transaction. This invalidates old local evidence even when remote content comes back to an earlier value; it also seeds new identities and restores missing canonical triggers. Do not keep remote generation counters instead of the local snapshot.
3. `Database::project_resource_trigger_sql()` is the sole canonical trigger-body source. Backup import/restore currently reject all persistent triggers (`backup.rs` authorizer, `reject_persistent_triggers`, SQL import and binary restore). Allow only exact canonical schema bodies, never a matching trigger name with different SQL. SQLite strips `IF NOT EXISTS` in sqlite_schema; obtain canonical stored form from a trusted empty database or normalize only that fixed prefix. Retain rejection of arbitrary credential-copying triggers.
4. In `projects/mod.rs`, a Skill whose DB version is unchanged must remain `Unverifiable`; changed DB versions can be `Drifted`. This generation proves metadata observations only, not current filesystem contents. Root owns this snapshot integration edit.
5. Memory remains `None` for this DAO; its context generation is a separate owner.

## Verification

- rustfmt passed for changed Rust implementation and the new tests.
- scoped `git diff --check` passed.
- First `mise run rust:test projects_resource_generations` did not reach execution: during concurrent root work the new `project_recovery_tests` module file was not yet present; it also exposed two errors in this change (Memory match arm and unavailable rusqlite total_changes method). Both owned compile errors were fixed: Memory returns None, and read-only test uses SQLite `SELECT total_changes()`.
- Per root instruction, no second Rust build was started while concurrent writers continued. Root must run the five `projects_resource_generations` cases plus `projects_resource_missing_and_owned_content_versions`, and its import/sync tests, after integration freezes.

No real user data, application installation, frontend or unrelated files were modified.
