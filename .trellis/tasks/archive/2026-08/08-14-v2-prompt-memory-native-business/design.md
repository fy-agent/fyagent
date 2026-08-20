# V2 Prompt and Memory native business integration — Design

## Boundaries

The feature remains inside the existing V2 renderer architecture. Pages consume
new `FeaturePorts.prompts` and `FeaturePorts.memory`; only the Tauri feature
adapter imports `@tauri-apps/api/core`. The adapter invokes existing commands,
so Rust commands, database tables, file formats, ACLs, and permissions remain
unchanged. The browser adapter rejects every Prompt/Memory operation as native
only and never creates sample records.

The old Prompt/Memory prototype models, per-page dark theme, fake agent targets,
cross-tool memory discovery, sessions, revisions, and sync task simulations are
removed. The six-route shell and the other four business pages remain unchanged.

## V2 Contracts

### Prompts

`PromptAppId` is the closed union `claude | codex | gemini | grokbuild |
opencode | openclaw | hermes`. `ManagedPrompt` mirrors the existing Rust DTO.

`PromptsPort` exposes list, current-live-file read, upsert, delete, enable, and
import operations. The Tauri adapter validates input IDs and parses every list
record from `unknown`. Upsert derives the redundant native `id` argument from
`prompt.id`. Disabling calls upsert with `enabled: false`; enabling calls the
dedicated command so the backend preserves live-file content and its one-enabled
invariant.

Queries are keyed by application and resource (`prompts(app)`,
`promptLiveFile(app)`). Mutations use a component write lock, invalidate only
the active application, and await an authoritative refetch. A successful write
followed by a failed reread is a warning state, not a false synchronized state.

The page defaults to Claude and uses a shared V2 header/toolbar plus master
detail layout. Application selection and search are local state. New/edit uses
the shared Dialog, deletion and dirty-discard use shared ConfirmDialog, and
enable uses the shared Switch. No multi-app assignment UI remains.

### Memory

`MemoryDocumentId` is closed to `openclaw-memory`, `openclaw-user`,
`hermes-memory`, and `hermes-user`. The Tauri adapter maps those IDs to the
existing whitelisted workspace filenames or Hermes kinds. The page never sends
an arbitrary long-term filename.

`MemoryPort` exposes long-term document read/write, Hermes limits/toggles,
OpenClaw daily list/read/write/delete/search, and the existing workspace/memory
directory open action. Daily filenames are validated as `YYYY-MM-DD.md` before
invoke even though Rust validates them again.

Long-term queries are keyed by document ID; Hermes limits have a separate key.
Daily list, search query, and file content use independent keys. Search is
debounced by 300ms. Writes invalidate and refetch the exact document or daily
resource. Missing OpenClaw documents return `null`, render an empty editor with
a create-on-save notice, and are not materialized until save.

The Memory page has Long-term and Daily tabs using shared V2 feature styles.
Long-term renders the four fixed real resources and a content editor. Only
Hermes resources expose enable state and character budgets. Daily renders the
authoritative file/search list, create-today action, editor, delete confirmation,
and open-directory action. Dirty guards cover every resource/tab/route change.

## Validation and Failure Behavior

- Page loading never writes or imports data.
- Browser mode renders a truthful native-only state.
- Invalid adapter input or malformed IPC output fails closed before UI state is
  treated as authoritative.
- Duplicate write actions are ignored while the write lock is held.
- Cached data remains visible when a post-write refresh fails, accompanied by a
  warning that the write may have completed.
- Native smoke is read-only on the real profile. Write behavior is proven by
  existing isolated Rust tests, exact invoke adapter tests, and stateful injected
  page ports so acceptance never overwrites private Prompt/Memory files.
- No user content is embedded in generated preview output or diagnostic logs.

## Compatibility and Rollback

Existing Prompt database rows and live files are read in place. Existing
OpenClaw and Hermes files are read in place. There is no migration or seed.
Reverting the focused implementation commit restores the old UI and adapters;
it does not delete legitimate data explicitly written by a user through the new
UI.
