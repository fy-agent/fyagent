# Selected configuration pack #73

## Goal
Users can export selected saved Claude Code and Codex connection settings and import a small versioned file or text through an explicit preview.

## Requirements
- Only supported connection fields are portable; omit secrets, credential references, account bindings, local paths and activation.
- Bound input size and count; reject unknown fields, unsupported versions, executable content and unsafe locations.
- Show actual fields, pending credentials and add/skip/rename/overwrite decisions before saving.
- Confirmation saves precisely the native preview. Changed local state invalidates it.
- Save all selected drafts atomically, recover on failure and return actual database readback. Never switch or enable a connection.
- Use existing Models entry and feature ports. No link service in this request.

## Acceptance
Focused parser, native database, file, port and UI tests cover secret/path rejection, selection, conflicts, changed preview/local state, partial failure and readback.

## Boundaries
No original checkout writes, schema change, live Agent files, credentials implementation, account export, Prompt/Memory export, executable MCP/Skill content or release claim. Parent: 09-20-next-iteration-engineering in root's integration worktree. The user has authorized implementation and local commit.
