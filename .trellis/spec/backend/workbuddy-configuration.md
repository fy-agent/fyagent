# WorkBuddy Configuration Contract

## 1. Scope / Trigger

Read this contract before changing WorkBuddy model discovery, URL admission,
credential handling, `models.json` persistence, overwrite/revision semantics,
or renderer navigation and query isolation. WorkBuddy is a top-level
configuration domain; it is not an `AppType`, Provider, MCP, Skill, Prompt,
Profile, Session, usage, environment, migration, or local-proxy domain.

## 2. Signatures

```text
get_workbuddy_status() -> WorkBuddyStatus
get_workbuddy_model_ids() -> WorkBuddyModelIdsResult

fetch_workbuddy_models({ baseUrl, apiKey, allowNoApiKey })
  -> { models: string[], truncated: boolean }

save_workbuddy_models({
  baseUrl,
  apiKey,
  allowNoApiKey,
  selectedModelIds,
  manualModelIds,
  removedModelIds,
  clearExistingApiKeys,
  expectedRevision,
  overwriteToken?,
})
  -> { state: "saved", revision, modelCount, createdEntries, updatedEntries }
   | { state: "overwrite_confirmation_required", token, existingIds }
   | { state: "concurrent_modification" }
```

The dedicated commands accept no `AppType`, Provider ID, renderer-controlled
filesystem path, arbitrary request URL, or log/debug echo field. An overwrite
token is opaque, short-lived, one-time, and bound to the normalized but
otherwise exact save request plus the expected revision.

## 3. Contracts

### User-owned location and URL normalization

- Read and write only the current user's `~/.workbuddy/models.json`, or the
  established `FYAGENT_TEST_HOME` override in hermetic tests. The only backup is
  same-folder `models.json.backup`. Never probe `.codebuddy`, a project path, or
  the real profile from a test.
- Accept only absolute HTTP(S) base URLs with a host and no user information,
  query, or fragment. Strip only terminal `/models`, `/chat/completions`, or
  `/responses`. Append `/v1` only when no decoded path segment already equals
  `v1`. Request exactly `<normalized-base>/models`.
- The `/v1` segment is a live third-party API protocol contract, not an FyAgent
  application-version label. Do not rewrite or remove it during version or
  documentation migrations.

### Elevated Windows storage identity

- A formal Windows build is elevated, while the WorkBuddy document belongs to
  the frozen interactive Explorer user. Treat every path component below the
  volume root as untrusted input; never convert a validated component back into
  a path-based read, backup, temporary-file, delete, or replace operation.
- Open the volume-to-profile chain component by component with relative
  `NtCreateFile`, `OBJ_DONT_REPARSE`, and `FILE_OPEN_REPARSE_POINT`. Hold every
  ancestor handle for the operation, reject reparse/offline/recall objects, and
  bind namespace checks to volume/file identity. Only the final profile handle
  may request directory-create rights, and only the pinned `.workbuddy` handle
  may request leaf-create/delete-child rights.
- Open `models.json` and `models.json.backup` relative to the pinned
  `.workbuddy` handle. Accepted leaves are regular, single-link files on the
  same volume. A directory junction, file symlink, hard-linked leaf, leaf type
  change, missing/existing race, or namespace identity drift fails closed with
  a generic configuration-storage error.
- A save snapshot opens the primary with `FILE_SHARE_READ |
  FILE_SHARE_DELETE` but never `FILE_SHARE_WRITE`, records its identity and
  exact bounded preimage, and holds that guard through the handle-relative
  replacement. Delete sharing is required for Windows replacement semantics;
  omitting write sharing still prevents a second writer, and an already-open
  write-compatible handle prevents snapshot acquisition. Recheck the held
  identity, namespace binding, bytes, and frozen interactive-user context
  immediately before commit. Never reopen the target through an absolute
  string path.
- Create backup and primary temporary leaves relative to the same pinned
  directory handle, flush them, preserve an existing target DACL, and rename
  the already-open temporary handle with `FileRenameInformationEx`. Commit the
  backup first. Replace an existing target only when its held identity still
  matches; create a missing primary without replace semantics so a raced create
  is rejected. Failed precommit work deletes only the already-open temporary
  handle and leaves redirected targets untouched.
- Revalidate the frozen interactive-user context before side effects and again
  immediately before the handle-relative rename. All checks that can still
  fail run before the rename commit point; do not report an unwritten failure
  after a successful namespace mutation.

### Bounded model discovery

- Use a short-lived restricted client with a 15-second total deadline, manual
  maximum of three redirects, same-origin enforcement, no HTTPS downgrade, and
  a 2 MiB streamed response limit.
- A nonempty API key is sent only to the original or validated same-origin URL.
  When the user explicitly allows an empty key, omit Authorization entirely.
  Never copy credentials to a redirect outside the admitted origin.
- Before any request or persistence preflight, reject a Base URL whose raw or
  decoded hostname/path contains the complete nonempty trimmed API key. This
  fail-closed comparison prevents the separately entered credential from being
  persisted in a URL or exposed through DNS, proxy, or access-log metadata.
- Redirect targets also reject userinfo, query, fragment, or complete
  credential containment in raw/decoded host/path before issuing the next hop.
  An upstream already knowing the bearer key is not allowed to copy it into a
  client-visible URL.
- A valid response is an object containing `data: []`; every element has a
  nonempty string `id`. Preserve upstream order, case, and first occurrence.
- Treat the submitted trimmed API key as a credential sentinel. If any model ID
  in a successful response contains it in full, reject the complete response with
  a generic error before constructing the DTO; never render or cache the
  matched value.
- Return at most 1,000 unique IDs. Set `truncated: true` when a valid 1,001st
  unique ID exists, but continue validating the rest of the bounded response so
  truncation cannot conceal a malformed element.

### Revision, overwrite capability, and persistence

- A save takes the in-process write lock, rereads current bytes, checks the
  opaque expected revision, validates the complete existing array and every
  entry ID, detects duplicate target IDs, and only then considers a write.
- Existing target IDs without a valid matching confirmation capability return
  `overwrite_confirmation_required` with one opaque token and unique
  `existingIds`. This preflight creates neither backup nor primary write. The UI
  freezes the exact request and retries only that request with the token. V2
  existing-model delete confirms once in the UI, then may auto-replay that
  token so the user is not asked a second time.
- The backend consumes the token before rereading, validates request and
  revision binding, rereads under the lock, and checks the revision again.
  Malformed, mismatched, expired, or reused tokens never authorize a write.
- The public revision is a process-local-key HMAC of the complete file bytes,
  not a bare digest. It detects an external API-key-only change without giving
  the renderer a public credential-guess oracle. The key is never persisted or
  serialized; after host restart, old revisions and tokens fail safely and the
  renderer refreshes status.
- Preserve non-target entries, array order, target positions, unknown fields,
  existing `onlyReasoning`, and unknown `reasoning` members. Update only the
  documented connection fields (`url` and policy-controlled `apiKey`); do not
  rebuild or normalize existing entries. `removedModelIds` delete matching
  entries and prune them from a populated `availableModels` list. A
  removal-only save does not require a Base URL or API key. An ID present in
  both the target set and `removedModelIds` fails closed.
- Commit backup then primary using flush/sync and same-directory atomic
  replacement. Windows uses replacement semantics with no delete-before-rename
  gap. Unix primary and backup credential files remain mode `0600`.

### Credential and renderer isolation

- API keys exist only in component memory, the Tauri request, and the protected
  credential files. They never enter localStorage, sessionStorage, query cache,
  logs, telemetry, URLs, revisions, overwrite tokens, or error DTOs.
- Apply the same fail-closed invariant to the user-owned document: if any
  trimmed model ID contains any nonempty trimmed `apiKey` in that document,
  status/model-ID reads and saves return a generic configuration error before a
  non-secret DTO is constructed. This protects data written by older versions
  or another process; opening the page must not place such a value in Query
  cache or the DOM.
- `TopLevelAppId = AppId | "workbuddy"`; `AppId` remains the Provider-domain
  type. WorkBuddy follows Codex and precedes Gemini in the app switcher.
- Missing legacy `visibleApps.workbuddy` resolves to `true`. Entering WorkBuddy
  mounts only its status/configuration surface and performs no Provider,
  current-provider, MCP, Skills, profile, usage, environment/migration, or proxy
  query. The API key is never refilled from disk. V2 discovery fetch keeps the
  in-memory key; save terminal outcomes still clear it. The V2 Models page keeps
  that key while it stays mounted across sidebar navigation and target switches.
  Actual unmount of the persistent Models page still clears it.
- A truncated-fetch warning remains visible until a later successful,
  non-truncated fetch replaces it. Failed or stale requests do not silently
  convert the warning into a complete result.

## 4. Validation & Error Matrix

| Condition                                                                     | Required result                                                                                   |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| URL is non-HTTP(S), lacks a host, or contains credentials, query, or fragment | Return `WORKBUDDY_INVALID_URL`; send no request.                                                  |
| Redirect exceeds three hops, changes origin, downgrades HTTPS, has query/fragment/userinfo, or contains the key | Return `WORKBUDDY_FETCH_REDIRECT_REJECTED`; issue no next request. |
| Fetch exceeds 15 seconds or 2 MiB, or `data[]` is malformed                   | Return the bounded fetch error and retain no model IDs from that response.                        |
| Remote or local model ID contains a complete request/document API key         | Fail closed with a generic error; return, cache, render, and write none of the colliding values.   |
| Empty API key is explicitly allowed                                           | Omit Authorization; do not synthesize an empty bearer value.                                      |
| Base URL hostname/path contains the complete submitted API key                 | Return a generic invalid-request error before network, token, backup, or primary-file activity.    |
| Existing JSON is invalid, not an array, or contains an invalid entry          | Return a safe configuration error, with only an index when useful; do not repair or overwrite it. |
| Revision changes before save or confirmed overwrite                           | Return `concurrent_modification`; write neither backup nor primary.                               |
| Windows profile, `.workbuddy`, primary, or backup resolves through a reparse point or changes identity | Fail closed before the target, backup, or any temporary leaf is mutated. |
| A Windows writer already owns, or tries to acquire, a write-compatible primary handle | Reject the snapshot/save; create neither backup nor temporary leaf. |
| Target IDs already exist without a matching overwrite token                   | Return one confirmation requirement; write neither backup nor primary.                            |
| `removedModelIds` match existing entries without a matching overwrite token   | Return one confirmation requirement listing those IDs; write neither backup nor primary.          |
| A removal-only save commits with a valid token                                | Delete matching entries and prune populated `availableModels`; URL/key are not required.          |
| Token is malformed, expired, mismatched, or reused                            | Consume/reject it, expose no credential or target contents, and write nothing.                    |
| A save updates an existing target                                             | Preserve entry position, unknown fields, and unrelated entries; update only documented fields.    |
| WorkBuddy view unmounts                                                       | Clear the in-memory API key and cancel/isolate its queries from other app domains. V2 Models keep-alive hide is not an unmount. |

## 5. Good / Base / Bad Cases

- Good: `https://gateway.example/api/v1` becomes
  `https://gateway.example/api/v1/models`; the key is sent only to that origin.
- Base: the user explicitly permits an empty key. Discovery sends no
  Authorization header and applies all other network bounds unchanged.
- Good: an external edit changes only an existing API key. The HMAC revision
  changes, the stale save returns `concurrent_modification`, and no public hash
  can be used to test key guesses.
- Bad: append a second `/v1`, follow a credential-bearing redirect, rebuild the
  complete JSON entry, delete the primary before rename on Windows, or store an
  API key in query state.

## 6. Tests Required

- URL fixtures cover terminal endpoint stripping, decoded `/v1` segments,
  spaces/Unicode, invalid schemes, user information, query/fragment rejection,
  same-origin redirects, origin drift, downgrade, hop count, deadline, body
  limit, and direct/prefix/suffix/percent-encoded credential containment in
  host/path without issuing a request or write.
- Response fixtures cover missing/non-array `data`, invalid elements, duplicate
  case-sensitive IDs, stable first occurrence, exactly 1,000/1,001 IDs,
  truncation plus a later malformed element, empty-key header omission, and a
  malicious successful response that echoes the submitted credential as an ID.
- Persistence tests cover empty/new files, invalid root/entries, target
  duplicates, request-bound one-time overwrite tokens, revision drift before
  both initial and confirmed saves, API-key-only external drift, process restart,
  stable ordering/unknown fields, backup ordering, atomic replacement, and Unix
  permissions.
- Windows-native persistence tests cover normal first create and repeated
  backup/primary replacement, a parent `.workbuddy` junction, primary and
  backup leaf reparse points, a profile-directory rename followed by junction
  substitution, an already-open same-size writer, a writer denied after the
  snapshot is pinned, and temporary-leaf cleanup.
  Every rejected case asserts that the redirected second tree and the accepted
  primary/backup preimage remain byte-for-byte unchanged. Add focused coverage
  before extending the implementation to a missing-primary create race or
  additional metadata/DACL preservation behavior; do not describe those cases
  as executed evidence until their native tests exist and pass.
- Security/static tests prove credentials cannot reach logs, URLs, caches,
  errors, revisions, or tokens. A malicious on-disk fixture with an ID equal to
  one of its API keys must fail before status/model-ID DTO construction.
  Renderer tests prove top-level navigation,
  default visibility, domain-query isolation, truncation state, frozen retry
  payload, and key clearing on unmount.

## 7. Wrong vs Correct

Wrong:

```text
normalized = trimTrailingSlash(baseUrl) + "/v1/models"
revision = sha256(modelsJson)
overwrite confirmed = boolean from renderer
```

Correct:

```text
parsed and admitted base + protocol-aware terminal normalization -> /models
revision = HMAC(process-local key, complete current bytes)
overwrite confirmed = one-time request-and-revision-bound backend capability
```
