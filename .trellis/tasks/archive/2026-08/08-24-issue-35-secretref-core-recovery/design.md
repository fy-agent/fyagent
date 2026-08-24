# Issue #35 SecretRef core recovery design

## Boundary

The recovery is a private modular-monolith service under `src-tauri/src/services/secret/`. It owns reference/value safety, a sealed backend port, native OS leaves, source-free metadata, and focused tests. It owns no database state, Provider mutation, UCP ledger, renderer surface, network call, or device journal.

The canonical Change Plan work has released `services/mod.rs`, but this slice
still has no production SecretRef consumer. Trial registration exposed a broad
dormant dead-code surface under warnings-denied Rust checks. The core therefore
remains compiled by the integration contract/native HIL and is registered only
with the first real Provider/Change Plan consumer. No public Tauri command or
`lib.rs`/AppState surface is added here.

## Core types

- `SecretRef`: private inner string, generated from UUIDv4, strict parser for durable/read requests, redacted `Debug`, no display implementation.
- `SecretMaterial`: owned byte buffer, non-empty and bounded, zeroized on drop; the only observation API consumes the value into a sealed callback.
- `SecretBackendKind`: `osKeyring` only for production v1; no hardware registration.
- `SecretPresence`: `present | missing | unknown`.
- `SecretAvailability`: `ready | missing | locked | denied | stale | revoked | unavailable`.
- `SecretSummaryDto`: no-value camelCase output with schema version, ref/display, purpose, backend, presence, availability, and an opaque non-secret handle generation token. The token is not an OS-store revision and core does not claim CAS without a later authoritative binding owner.
- `SecretServiceError`: closed source-free code/category/action tuple. Raw platform status and messages never cross the backend leaf.

## Backend port

`SecretBackend` is crate-private and sealed by module visibility. It supports:

1. `create_new(ref, material)` — create-only, duplicate is an explicit conflict.
2. `replace(ref, material)` — existing-record-only update followed by readback equality.
3. `with_material(ref, callback)` — reads into native memory, invokes a non-clone/non-serializable callback, then zeroizes.
4. `probe(ref)` — source-free presence/availability only.
5. `delete(ref)` — delete existing record; missing is distinct and never treated as success unless the caller explicitly performs a later missing readback.

`SecretService` owns exactly one backend instance. Errors never cause fallback or selection of a second backend.

## Platform leaves

### macOS

Use Security.framework generic-password items:

- service: `com.fyagent.secrets.v1`
- account: full random `SecretRef`
- `kSecUseDataProtectionKeychain=true` on create/read/probe/update/delete queries
- `kSecAttrSynchronizable=false`
- `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
- create via `SecItemAdd`, replace via `SecItemUpdate`, read via `SecItemCopyMatching`, delete via `SecItemDelete`

Each create/replace performs a readback and constant-time comparison. The Data
Protection Keychain selector is mandatory because macOS only supports
`kSecAttrAccessible` with the Data Protection Keychain or synchronizable items;
this contract chooses device-local Data Protection Keychain plus explicit
non-sync. Raw `OSStatus` values are mapped inside the leaf and not stored or
returned. A failed create readback does not authorize a compensating delete:
another entitled process could have updated the item after `SecItemAdd`.
Because that fail-closed rule can leave an unverified native item, the first
production consumer must retain durable create-admission/recovery authority
until verification settles.
All macOS native-store operations share one process-global mutex rather than an
instance-local lock, so constructing another backend instance cannot bypass
FyAgent's own serialization boundary.

### Windows

Use Credential Manager generic credentials:

- target: `FyAgent/secret/v1/<SecretRef>`
- type: `CRED_TYPE_GENERIC`
- persistence: `CRED_PERSIST_LOCAL_MACHINE`
- username: `FyAgent`

Create probes first and rejects a record already visible to this FyAgent
process; all native Credential Manager operations are serialized by one
process-global backend mutex even if multiple backend instances are constructed.
Replace requires an existing record. `CredWriteW`, `CredReadW`,
`CredDeleteW`, and `CredFree` stay inside the leaf. Microsoft defines
`CredWriteW` as create-or-replace, so the OS does not provide an atomic
create-only primitive here. The generated ref is random and remains private to
the caller path, which makes accidental external collision negligible, but the
backend does not claim protection from a malicious external writer that races
the exact target name. Post-write verification failure returns fail-closed and
does not blindly delete a value that may have been replaced externally. Raw
Win32 error codes/messages never cross the leaf.

## Security invariants

- No secret-bearing SHA/HMAC is persisted or serialized.
- No plaintext fallback, environment fallback, Provider-field fallback, or backend fallback.
- Backend locators are private implementation details.
- All value comparisons are constant-time.
- DTO and error tests use runtime-generated canaries and assert absence, rather than relying on `[REDACTED]` placeholders.

## Deferred work

Binding/device store, candidates, rotation, lock/delete impact CAS,
audit/journals, legacy migration, `services/mod.rs` production registration,
commands, AppState wiring, Provider create/edit, V2 Credentials UI, and
hardware adapters remain follow-up slices. The PR must enumerate these and keep
#35 open.

## Native evidence

- macOS matching-host test runs the ignored CRUD contract locally with
  `FYAGENT_NATIVE_SECRET_TEST=1` only when hosted by an app-like binary whose
  access-group entitlement is authorized by a provisioning profile. Plain Cargo
  test binaries are not valid DPK HIL and are expected to fail missing
  entitlement rather than fall back to the legacy file-based keychain.
- Windows `Backend Checks (Windows)` owns an explicit native Credential Manager
  CRUD step. The step captures Cargo output and requires exactly one passed
  ignored integration test so a zero-test filter cannot be misreported as HIL.
- Cross-compilation is compile evidence only and never substitutes for either
  matching-host credential-store run.
- The current core remains unregistered, so signed-app macOS DPK HIL is a hard
  gate on the first production consumer rather than on this dormant core slice.
