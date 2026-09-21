# Session Migration Contract

## 1. Scope / Trigger

Read before changing Session export/import, final-answer extraction, native
history adapters, restore receipts, or the `/sessions` renderer route. The
semantic owner is `session_manager::migrate`; Tauri commands remain thin.
Existing session browsing and Memory document editing keep their own owners.

## 2. Signatures

```text
preview_session_migration(providerId, sourcePath) -> MigratableSession
export_session_package(items[{providerId, sourcePath}], targetPath) -> ExportOutcome
read_session_package(path) -> { package, attempts }
probe_local_provider(providerId) -> LocalProviderProbe
get_release_capability_matrix() -> ReleaseCapability[]
restore_session_package(request) -> RestoreAttempt[]
verify_native_readback(attemptId) -> RestoreAttempt
list_restore_attempts() -> RestoreAttempt[]
reconcile_restore_attempts() -> RestoreAttempt[]
record_user_attestation(attemptId, claimedStage, note?) -> RestoreAttempt
open_restored_session(attemptId) -> boolean
pick_session_package_file() -> path | null
pick_session_package_export_path(defaultName) -> path | null
```

`RestoreRequest` has `packagePath`, `requestId`, `snapshotIds`,
`targetProviderId`, `targetWorkspace` and `requestKind` (`defaultImport` or
`saveAsNewCopy`). There is no overwrite operation or renderer-supplied command.
The existing FyAgent database owns one `session_restore_attempts` table.
Receipts contain identity, target mapping, stage and errors; no conversation
bodies or authentication material. Native history stays in the provider store.

## 3. Contracts

### Text and package boundary

- `fyagent.session.v1` is the closed envelope. Reject unknown versions,
  duplicate JSON keys, trailing documents, unknown fields and invalid shapes.
  Apply bounded file, nesting, session, message and identity limits before
  mutation. Backend validation remains authoritative.
- Each message has dense `seq`, `kind` (`userText` or `assistantFinal`), exact
  `text` and optional timestamp. Preserve Unicode, CRLF, whitespace, embedded
  code and paths. Do not insert separators or fabricate missing answers.
- A genuine unanswered user message is preserved at its original position.
  An assistant message whose final status cannot be determined blocks the
  whole export, including ambiguity in the middle of a conversation.
- Tool calls/results, progress, reasoning, files, attachments and runtime
  injections do not enter the transcript. Classify by versioned structured
  provenance, never by a guess that the last assistant must be final.
- Workspace metadata is a basename/path-family hint. It cannot select a
  target directory or rewrite paths inside message text. The target directory,
  account and model belong to the target installation.

### Identity and interrupted writes

- `originId` identifies a source, `contentDigest` hashes only ordered
  `(kind,text)` values, and `snapshotId` hashes origin plus content digest.
  Package byte hashes, export time, title and source paths are not dedup keys.
- The default slot includes snapshot, target provider/store and current-device
  binding. Equal bodies from different sources never collapse.
- One explicit import action gets one `requestId`. Transport retries reuse it;
  another explicit copy action gets a new ID and retains every prior mapping.
  `installation_id` binds the action across target stores; `request_fingerprint`
  binds the complete sorted snapshot selection, provider, store, canonical
  workspace and request kind. Reject an empty selection; restoring all sessions requires their explicit IDs.
  Claim every selected receipt in one SQLite
  transaction before any native write. A changed selection or target cannot
  reuse the same action ID; malformed batch sets fail without inserting rows.
- Installation identity binds a random seed to the machine/user and the actual
  state directory instance. Renaming that directory preserves identity;
  copying it does not. Locked identity IO stays anchored to the opened directory,
  and a replaced path cannot receive writes protected by an old directory lock.
  A preallocated native ID must be stored successfully or match the existing
  mapping before publication. A missing row or conflicting ID blocks writing.
- Record the intent before the native write and a preallocated native ID
  before publication whenever supported. SQLite cannot atomically commit a
  provider subprocess or external native file.
- A timeout, partial write or missing response is unresolved. A second native
  write is forbidden until authoritative evidence resolves it. Zero search
  matches do not prove that nothing was written. Only proven absence of a
  side effect permits retry as `failed`.
- A restored native session inherits its source identity only through an
  exact current-device/store/provider/native-ID receipt mapping, never through
  content similarity. Continuing it changes the snapshot, not its origin.
- Foreign-device receipts and replaced target stores cannot occupy a local
  slot or authorize opening a native session.

### Capability and evidence

- Keep all seven provider rows visible. Extraction support and write support
  are separate gates, each pinned to actual verified versions and the local
  executable/store. A missing implementation is a concrete capability gap.
- `nativeWritten` means publication succeeded. `nativeReadbackVerified`
  requires the provider's own read interface and an exact transcript digest;
  reading the file we wrote is insufficient. Opening a client does not prove
  a next request or model reply.
- Release evidence and per-attempt state are independent. A local HTTP mock
  can prove request history, never a real model response. A user's manual
  confirmation is `userAttestation`, not a system-stage update.
- Keep formal Windows ordinary-user execution boundaries. An unavailable
  authenticated helper must fail before executing a user CLI elevated.
- Source inspection, portable tests, native API readback, UI inspection,
  real model continuation and Windows/macOS execution are distinct evidence.

### Renderer boundary

Use `FeaturePorts.sessions` and the deferred Tauri adapter. Browser production
is native-only; fixtures belong to tests. Select sessions by provider, source
and native ID. Match receipts by exact origin/snapshot or exact target identity.
Preview every selected export item and freeze the set used for the write.
Multi-provider packages restore only snapshots belonging to the selected
corresponding provider. Probe failure disables the mutation with a useful reason.
Show unresolved and failed results without a success banner. Modal request IDs
and pending work follow the shared Dialog lifecycle contract.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Unknown package version/field, duplicate key, digest mismatch or resource limit | Reject before receipt/native mutation. |
| Genuine missing final answer | Preserve user text; mark incomplete. |
| Unknown assistant phase or mixed/unclassified provenance | Block export; never silently discard real input. |
| Re-export with changed timestamps but same source/content | Same semantic snapshot/default slot. |
| Same request delivered twice | Reuse its receipt; at most one native write. |
| Same origin with changed content | Explicit snapshot conflict/copy decision; no auto-append. |
| Native output unknown or readback differs | Keep unresolved, preserve mappings, prohibit blind retry. |
| Native format cannot represent some message exactly | Fail before writing that shape; no placeholder or trimming. |
| Target model/account unavailable | Actionable local setup error; no source credentials or invented model. |
| User attests successful continuation | Update only the separate attestation. |

## 5. Good / Base / Bad Cases

- Good: an exact native readback verifies a newly allocated session while the
  old session and all Memory documents remain untouched.
- Base: a version lacks a proven native writer; browsing remains available and
  import explains that version's missing capability.
- Bad: import exit zero, a disk scan, or manual confirmation turns every stage
  green; an uncertain timeout triggers another native write.

## 6. Tests Required

Exercise real production parsers/projections with synthetic Unicode/CRLF/code
fixtures, consecutive users/finals and mid-conversation ambiguity. Cover all
envelope levels, malformed input and identity collisions. Use an isolated
receipt database and counted/fault-injected writer for request replay, default
slots, source conflicts and crash windows. Confirm exact request binding and
disabled capabilities in component/browser tests, plus the reachable legacy
Memory route. Native probes use isolated provider stores and synthetic content;
record their version, platform, commands and actual achieved evidence stage.

Run the canonical relevant Rust, frontend, architecture, production build and
browser gates after the final related changes. Current-host tests do not prove
another operating system's runtime. Document waived/unavailable real-machine
acceptance separately from CI and implementation status.

## 7. Wrong vs Correct

Wrong: deduplicate by package bytes or transcript alone; mint a new `requestId`
on every retry; map a subprocess timeout to a write-free failure.

Correct: validate a semantic snapshot, claim its current-device target slot,
persist the request/native mapping, write once and advance only from observed
native evidence. Keep uncertain outcomes visible for reconciliation.
