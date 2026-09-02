# QoderWork Hooks Configuration Contract

## 1. Scope / Trigger

Read this contract before changing QoderWork Hooks read/write commands,
supported hook events/groups, revision conflicts, overwrite authorization,
backup/atomic persistence, restart projection, or Windows file-safety checks.

Primary owners:

- `src-tauri/src/services/qoderwork.rs`
- `src-tauri/src/commands/qoderwork.rs`
- the QoderWork command DTOs and their Rust tests

This is not the QoderWork CN Skills/MCP path. Skills and MCP use
`.qoderworkcn`; Hooks intentionally use the reviewed
`{trusted-home}/.qoderwork/settings.json` contract.

## 2. Signatures

```text
get_qoderwork_hooks()
  -> secret-free snapshot {
       exists,
       supported hook projection,
       revision,
       restart requirement / unsupported projection state
     }

save_qoderwork_hooks(request)
  -> saved | conflict(overwrite capability) | controlled failure
```

The exact request/response field names and closed hook event/group enums are
owned by `commands/qoderwork.rs`. The mutation includes the expected revision
and may include only an opaque overwrite token previously issued for the exact
conflicting request. It never accepts a file path, arbitrary JSON document,
backup path, command to execute, or validation bypass.

Revisions are native HMAC capabilities over the authoritative preimage. An
overwrite token is opaque, request-digest-bound, current-conflict-bound,
short-lived, and single-use.

## 3. Contracts

### Trusted path and bounded read

Native code derives `{trusted-home}/.qoderwork/settings.json`; the renderer does
not send or reconstruct it. The document is bounded to 2 MiB, must be a regular
reviewed file, and is parsed before projection. Unsupported or unknown hook
content is preserved as document data but is never silently converted into a
supported editor state.

The read DTO exposes only the supported Hooks projection, revision, existence,
and bounded status needed by the UI. It does not return unrelated private
settings, raw bytes, secret values, or executable content.

### Revision and overwrite authorization

Normal save requires the expected revision to match the current file. A
mismatch returns a conflict with one capability tied to:

- the current authoritative preimage/revision;
- the normalized intended Hooks mutation;
- the current process/session secret;
- one bounded expiry and one consumption.

Changing the request, rereading a new revision, replaying the token, or using
it after expiry fails. The UI may ask the user once, then resubmit the exact
frozen request plus that token. It must not request or synthesize an unconditional
force flag.

### Preserve, back up, replace, reread

Save replaces only the supported Hooks field(s). Unknown top-level settings and
unsupported extension fields survive. Before replacing an existing file, the
writer creates/replaces the adjacent rolling backup with the exact preimage.
Backup failure aborts before the primary write.

The primary write uses a same-directory temporary file, permission-preserving
flush/sync, atomic replacement, and an authoritative reread. A successful
return requires the reread to match the normalized requested projection.
Restart-required copy is derived from the committed result, not from the click.

On Windows, opened handles, canonical identity, reparse points, hard links,
replacement races, and file-type checks must remain inside the reviewed
file-safety owner. A safe-looking lexical path is not sufficient proof.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Settings file exceeds 2 MiB, is malformed, or is not a reviewed regular file | Controlled unavailable/error; no partial projection or write |
| Expected revision matches | Apply the bounded Hooks mutation through backup/atomic write |
| Expected revision differs | Return conflict plus exact overwrite capability; do not write |
| Overwrite token is expired, reused, for another request, or for another preimage | Reject before backup/write |
| Backup creation/replacement fails | Abort; primary bytes remain unchanged |
| Unknown top-level fields exist | Preserve them byte/semantic-equivalently outside the managed Hooks field |
| Atomic replacement or reread fails | Non-success; report recovery state without claiming save |
| Reread projection differs from request | Fail closed; do not show committed success |
| Windows path resolves through unsafe reparse/hardlink identity | Reject before write |
| Renderer sends a path, whole settings document, or force boolean | API review/parser fails; such fields are not admitted |

## 5. Good / Base / Bad Cases

- **Good:** Read returns revision R1. The user edits one supported event and
  saves with R1. Native code backs up, atomically replaces only Hooks, rereads,
  and returns the committed projection.
- **Base:** Another process changed the file after R1. Save returns conflict;
  after explicit confirmation, the exact frozen request may consume its one
  overwrite token once.
- **Bad:** `force: true`, rewrite the complete settings object from React, use
  `.qoderworkcn/settings.json`, or report success before reread.

## 6. Tests Required

- Rust tests in `services/qoderwork.rs` cover missing file, bounded valid read,
  malformed/oversized/non-regular inputs, supported projection, and preservation
  of unknown fields.
- Save tests cover revision match, conflict issuance, token expiry, request and
  preimage binding, single use, replay rejection, backup failure, same-directory
  atomic replacement, permission preservation, and authoritative reread.
- Windows tests cover reparse/hardlink/file-identity races and prove lexical
  containment alone cannot authorize a write.
- Command/adapter tests assert exact camel-case payloads, closed event/group
  values, no raw settings/path fields, and secret-safe errors.
- UI tests keep the conflict dialog tied to one frozen request and do not retain
  a force capability after the terminal outcome.

## 7. Wrong vs Correct

Wrong: let the renderer replace arbitrary settings with a force flag.

```ts
await invoke("save_qoderwork_hooks", {
  path: userPath,
  settings: JSON.parse(editorText),
  force: true,
});
```

Correct: submit the supported projection with the observed revision; replay
only the exact conflict capability after explicit confirmation.

```ts
const request = freezeHookRequest(draft, snapshot.revision);
const result = await ports.qoderWork.saveHooks(request);
if (result.kind === "conflict" && confirmed) {
  await ports.qoderWork.saveHooks({ ...request, overwriteToken: result.token });
}
```
