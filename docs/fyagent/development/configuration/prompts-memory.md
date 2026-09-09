# Prompt and daily memory file boundaries

Prompt library persistence is owned by
`src-tauri/src/services/prompt.rs`; daily OpenClaw memory operations are owned
by `src-tauri/src/commands/workspace.rs`. The existing typed renderer ports
remain the UI boundary. Neither flow needs an additional command or database
schema to distinguish a library edit from a native file operation.

## Library saves and live Prompt changes

A disabled Prompt is a saved library entry. Creating it, editing it, or
importing the current file into it must preserve the live file, including its
modification time and existing backup/undo artifacts. If no live file exists,
a disabled library save does not create one.

Every explicit import receives a new opaque ID. Using only a second-resolution
timestamp can collide with an earlier imported and enabled row; a fresh UUID
keeps the import a new library entry. Existing IDs are preserved.

The service reads the prior persisted state for `prompt.id` before saving.
An enabled entry still writes its content. A real enabled-to-disabled
transition retains the existing behavior: clear the live file only when no
other entry is enabled. Saving a new disabled entry while the enabled count
is zero is not that transition. Deleting an enabled entry remains prohibited.

Actual live writes continue through the existing atomic writer and recovery
mechanism. This fix does not restore previously lost user content or make
database and filesystem writes one transaction.

## Daily memory directory admission

The native list, search, read, write, and delete operations use the same
filename validator. It requires ASCII `YYYY-MM-DD.md`, a valid calendar date,
and a year from 0001 through 9999, matching the renderer's date domain.

Directory enumeration skips README files, impossible dates, extra suffixes,
and directories. Search follows the same rule, so a query cannot reintroduce
an entry that would make the renderer reject the response. Valid files retain
descending filename order and the existing Unicode preview/snippet behavior.
Explicit read/write/delete requests for invalid names fail before file I/O;
ignored files are not renamed or removed.

## Verification and recovery

`mise run rust:test issue_141_` exercises the regression cases against
temporary files. Existing Prompt/Memory renderer tests cover ports, pages,
query invalidation, and error presentation. Native acceptance uses a build of
the changed branch with an isolated `FYAGENT_TEST_HOME`, then reads back the
files and Prompt rows; a passing renderer mock alone is insufficient.

Keep the normal installed application and real user files outside that test
profile. Capture fixture preimages before writes, stop the test process when
done, and reopen the normal application. Source rollback uses the scoped fix
commit; file recovery must verify ownership of the current state before
restoring a backup, rather than overwriting newer user changes.
