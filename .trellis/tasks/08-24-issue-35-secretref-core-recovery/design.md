# Issue #35 SecretRef core recovery design

## Boundary

The recovery is a private modular-monolith service under `src-tauri/src/services/secret/`. It owns reference/value safety, a sealed backend port, native OS leaves, source-free metadata, and focused tests. It owns no database state, Provider mutation, UCP ledger, renderer surface, network call, or device journal.

Shared module/command registration is intentionally a serial integration step after the UCP owner releases `services/mod.rs`, `commands/mod.rs`, and `lib.rs`. Until then, an integration-style Rust test imports the module by path so every new source file is compiled and exercised without violating file ownership.

## Core types

- `SecretRef`: private inner string, generated from UUIDv4, strict parser for durable/read requests, redacted `Debug`, no display implementation.
- `SecretMaterial`: owned byte buffer, non-empty and bounded, zeroized on drop; the only observation API consumes the value into a sealed callback.
- `SecretBackendKind`: `osKeyring` only for production v1; no hardware registration.
- `SecretPresence`: `present | missing | unknown`.
- `SecretAvailability`: `ready | missing | locked | denied | stale | revoked | unavailable`.
- `SecretSummaryDto`: no-value camelCase output with schema version, ref/display, purpose, backend, presence, availability, and monotonically changing opaque non-secret revision.
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
- `kSecAttrSynchronizable=false`
- `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
- create via `SecItemAdd`, replace via `SecItemUpdate`, read via `SecItemCopyMatching`, delete via `SecItemDelete`

Each create/replace performs a readback and constant-time comparison. Raw `OSStatus` values are mapped inside the leaf and not stored or returned.

### Windows

Use Credential Manager generic credentials:

- target: `FyAgent/secret/v1/<SecretRef>`
- type: `CRED_TYPE_GENERIC`
- persistence: `CRED_PERSIST_LOCAL_MACHINE`
- username: `FyAgent`

Create probes first and rejects duplicates; replace requires an existing record. `CredWriteW`, `CredReadW`, `CredDeleteW`, and `CredFree` stay inside the leaf. Raw Win32 error codes/messages never cross it.

## Security invariants

- No secret-bearing SHA/HMAC is persisted or serialized.
- No plaintext fallback, environment fallback, Provider-field fallback, or backend fallback.
- Backend locators are private implementation details.
- All value comparisons are constant-time.
- DTO and error tests use runtime-generated canaries and assert absence, rather than relying on `[REDACTED]` placeholders.

## Deferred work

Binding/device store, candidates, rotation, lock/delete impact CAS, audit/journals, legacy migration, commands, AppState wiring, Provider create/edit, V2 Credentials UI, and hardware adapters remain follow-up slices. The PR must enumerate these and keep #35 open.
