# SecretRef Native Backend Contract

## 1. Scope / Trigger

Read this contract before changing SecretRef identity, secret-material lifetime,
native credential-store behavior, SecretBackend error projection, or the CI
evidence used to claim macOS/Windows credential-store support.

This owner is intentionally narrow. It does not own Provider persistence,
Change Plan schema/ledger, renderer commands/UI, credential migration, device
binding journals, or hardware-backed secret stores.

## 2. Signatures

The private service source lives under `src-tauri/src/services/secret/`. The
core slice is compiled through its integration contract and matching-host HIL,
but it is intentionally not registered in `services/mod.rs` until the first
production consumer lands. Registering a dormant module would create a large
dead-code surface and weaken the repository's warnings-denied discipline. The
core slice exposes no Tauri command.

Durable identities are opaque random references:

```text
SecretRef     = sec_<lowercase UUIDv4 simple hex>
SecretVersion = sv_<lowercase UUIDv4 simple hex>
```

The private backend boundary supports:

```text
create_new(ref, material)
replace(ref, material)
read(ref)
probe(ref)
delete(ref)
```

Projectable metadata may expose reference/version/purpose/backend/presence/
availability only. Secret material, value-derived digests, native backend
locators, raw OS errors, and credentials never cross that boundary.

## 3. Contracts

### Secret material

- `SecretMaterial` owns a bounded UTF-8 byte buffer under `Zeroizing<Vec<u8>>`.
- Material is not cloneable or serializable and has redacted `Debug` output.
- Observation is restricted to the sealed native callback/service boundary;
  buffers are zeroized when dropped.
- A backend failure never selects a plaintext, environment, Provider-field, or
  second-backend fallback.

### macOS Keychain

FyAgent stores generic-password items with:

```text
service = com.fyagent.secrets.v1
account = full SecretRef
kSecUseDataProtectionKeychain = true
kSecAttrSynchronizable = false
kSecAttrAccessible = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
```

`kSecUseDataProtectionKeychain=true` is included on add/read/probe/update/delete
queries that identify the item. This is required for the macOS
`kSecAttrAccessible` contract while retaining device-local, non-iCloud
behavior. Do not replace it with `kSecAttrSynchronizable=true` merely to make
the accessibility attribute apply.

Create uses `SecItemAdd`, so an existing composite identity returns duplicate
instead of silently replacing it. Create/replace must read back and compare the
material in constant time; failed create verification attempts cleanup.

### Windows Credential Manager

FyAgent stores generic credentials with:

```text
target   = FyAgent/secret/v1/<SecretRef>
type     = CRED_TYPE_GENERIC
persist  = CRED_PERSIST_LOCAL_MACHINE
username = FyAgent
```

Windows `CredWriteW` is an upsert API: it creates a missing credential and
replaces an existing credential with the same target/type. FyAgent must **not**
claim an OS-level atomic create-only primitive. `create_new` uses a
process-local mutex, an explicit pre-write probe, an unpredictable UUIDv4
`SecretRef`, and mandatory readback. This prevents silent replacement by
FyAgent's own concurrent create paths; an external process racing the same
random target is outside the Win32 API guarantee and belongs to later
handle/version/CAS lifecycle defense rather than being modeled as impossible.

Replace requires a pre-existing record and performs mandatory readback.
Credential Manager memory returned by `CredReadW` is always released with
`CredFree`. Raw Win32 codes/messages remain inside the platform leaf.

### Native evidence

- macOS Data Protection Keychain support requires a matching-host CRUD/readback/
  cleanup test executed by an app-like, code-signed host whose access-group
  entitlements are authorized by an embedded provisioning profile. A plain
  `cargo test` executable has no qualifying app identity and a
  `errSecMissingEntitlement` result is expected fail-closed evidence, not a DPK
  product failure.
- Windows support requires the same test to execute on a native Windows hosted
  runner. Compilation or ignored-test discovery is not Credential Manager HIL.
- The Windows backend CI job runs that exact ignored integration test after the
  ordinary workspace Rust tests and includes it in collected required-step
  outcomes.
- Every native HIL creates a random reference, creates/reads/replaces/reads/
  deletes it, then verifies the reference is missing. No persistent test secret
  may remain after a successful run.
- The first production consumer must register `services::secret` in the same
  reviewed integration change; do not pre-register it with broad dead-code or
  unused-import lint allowances.
- The first macOS production consumer must also prove the signed FyAgent host
  carries an authorized data-protection-keychain access group. The current
  Developer ID/notarization chain must not be assumed to provide that identity
  merely because the app is signed.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| malformed/non-v4/uppercase SecretRef | reject before native access |
| empty/NUL/non-UTF8/oversized material | reject before native write |
| macOS query uses `kSecAttrAccessible` without Data Protection Keychain | reject implementation; accessibility contract is invalid on macOS |
| macOS duplicate create | return stable already-exists error; no overwrite |
| plain cargo test returns `errSecMissingEntitlement` with DPK enabled | classify the harness as unauthorized; do not fall back to file-based keychain and do not count it as native DPK acceptance |
| first production consumer lacks signed-app provisioning/access-group evidence | block macOS SecretRef activation |
| Windows create sees an existing target | return stable already-exists error before `CredWriteW` |
| Windows documentation/code claims `CredWriteW` is atomic create-only | reject; Win32 specifies create-or-replace semantics |
| native store locked/denied/unavailable | source-free stable error/probe; no fallback |
| create/replace readback differs | fail verification; create attempts cleanup |
| serialized DTO/error/debug contains secret canary | test failure / NO-GO |
| Windows matching-host HIL did not execute | Windows SecretRef merge gate remains incomplete |
| dormant SecretRef module requires `allow(dead_code)` to stay registered | remove premature registration; register with the first real consumer instead |

## 5. Good / Base / Bad Cases

- **Good:** random SecretRef, one native backend, bounded zeroizing material,
  constant-time readback, source-free errors, Data Protection Keychain on
  macOS, and explicit Windows Credential Manager CRUD HIL.
- **Base:** Windows cannot make `CredWriteW` atomically create-only. State the
  limitation accurately and use random identities + process serialization +
  later lifecycle CAS rather than inventing an OS guarantee.
- **Bad:** persist plaintext in SQLite, logs, DTOs, errors, Provider fields, or
  environment fallback because the OS credential store is unavailable.
- **Bad:** set Keychain synchronizable merely to activate `kSecAttrAccessible`;
  that would change the device-local requirement into iCloud synchronization.

## 6. Tests Required

```bash
mise run rust:fmt:check
cargo test --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract
# This command is useful only inside an authorized app-like macOS host; a plain
# cargo test binary is expected to fail DPK access with missing entitlement.
FYAGENT_NATIVE_SECRET_TEST=1 cargo test --locked --manifest-path src-tauri/Cargo.toml --test secret_service_contract native_os_backend_crud_readback -- --ignored --exact --nocapture --test-threads=1
mise run test:unit -- tests/ciWorkflow.test.ts
mise run rust:check
mise run rust:clippy
mise run supported-platform:check
mise run check:contracts
```

The local native-HIL command proves only the current host and, on macOS, only
when the host process has the required authorized access-group identity.
Windows acceptance must be bound to the hosted Windows job on the exact
PR/merge-group SHA.

## 7. Wrong vs Correct

Wrong:

```text
CredReadW says missing -> CredWriteW therefore atomically creates
```

Correct:

```text
CredReadW says missing
  -> process-local create serialization
  -> random unguessable target
  -> CredWriteW (documented create-or-replace API)
  -> mandatory readback
  -> later lifecycle/version CAS owns cross-process stale-handle defense
```

Wrong on macOS:

```text
kSecAttrAccessibleWhenUnlockedThisDeviceOnly
kSecAttrSynchronizable = false
# no kSecUseDataProtectionKeychain
```

Correct:

```text
kSecUseDataProtectionKeychain = true
kSecAttrSynchronizable = false
kSecAttrAccessibleWhenUnlockedThisDeviceOnly
```
