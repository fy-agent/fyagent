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
first production consumer is [Managed Auth Core](./managed-auth.md), which
registers `services::secret` and constructs `SecretService<NativeSecretBackend>`
in the application composition root. The leaf still exposes no Tauri command;
renderer traffic stays on Managed Auth DTOs that never include SecretRef or
secret material.

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
  app-owned second backend. Choosing between macOS Security.framework's Data
  Protection Keychain and file-based login keychain remains one native
  `OsKeyring` backend, not a FyAgent file-store fallback.

### macOS Keychain

FyAgent stores generic-password items with:

```text
service = com.fyagent.secrets.v1
account = full SecretRef
kSecAttrSynchronizable = false
```

Prefer Data Protection Keychain when the process can use it:

```text
kSecUseDataProtectionKeychain = true
kSecAttrAccessible = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
```

`kSecUseDataProtectionKeychain=true` is included on add/read/probe/update/delete
queries that identify a DPK item. This is required for the macOS
`kSecAttrAccessible` contract while retaining device-local, non-iCloud
behavior. Do not replace it with `kSecAttrSynchronizable=true` merely to make
the accessibility attribute apply. Do not set `kSecAttrAccessible` without DPK.

`MacOsSecretBackend` starts with an unknown process-wide DPK mode and caches the
result of a bounded capability probe. An unpackaged test binary stays fail
closed when DPK returns `errSecMissingEntitlement` (-34018). For an executable
inside `.app/Contents/MacOS/`:

- a capability probe or create that returns -34018 disables DPK for the
  process; create retries once with the same service/account/non-sync identity
  and omits both DPK and `kSecAttrAccessible`;
- a DPK read, probe, or replace that reports item-not-found retries the same
  identity in the file-based login keychain, so an item created earlier in the
  signed-app mode remains discoverable; a successful retry latches that mode;
- delete may retry the file-based identity after item-not-found, auth-failed,
  or missing-entitlement and remains idempotent when neither flavor has an
  item.

Once DPK is disabled, subsequent create/read/probe/replace/delete operations in
that process use the file-based login-keychain flavor. This is still the OS
Keychain, never a FyAgent-owned plaintext file or environment store.

The repository's current signed-app entitlement file intentionally omits
`keychain-access-groups`. Do not add that restricted entitlement unless the
packaging path also embeds a provisioning profile that authorizes the group;
an entitlement-plist edit alone is not DPK activation and may prevent launch.

Create uses `SecItemAdd`, so an existing composite identity returns duplicate
instead of silently replacing it. Create/replace must read back and compare the
material in constant time. A failed create verification must not blindly delete
the item: after the add returns, a same-access-group process could have raced an
update, and FyAgent may delete only a value whose ownership is still proven.
If `SecItemAdd` succeeds but authoritative readback does not settle, do not
blindly delete the item. Managed Auth retains the generated `SecretRef` in a
SQLite `provisioning` / `secret_missing` row until verification settles, then
recovers or marks the session unusable. Do not drop a native-created handle
from memory without a durable admission record.

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
process-global backend mutex, an explicit pre-write probe, an unpredictable UUIDv4
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
- `services::secret` is registered and constructed only because Managed Auth
  consumes it. Do not keep the module with `allow(dead_code)` if that consumer
  is removed.
- Managed Auth owns create admission/recovery so a native create that succeeded
  but could not be authoritatively read back remains a durable
  `provisioning`/`secret_missing` row instead of an unreachable native item.
- A signed-app CRUD run that selects the file-based login keychain proves only
  that residual mode. DPK production acceptance still requires an app-like host
  whose access-group entitlement is authorized by an embedded provisioning
  profile. Plain `cargo test` -34018 remains fail-closed DPK evidence and must
  not opt into the signed-app residual path.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| malformed/non-v4/uppercase SecretRef | reject before native access |
| empty/NUL/non-UTF8/oversized material | reject before native write |
| macOS query uses `kSecAttrAccessible` without Data Protection Keychain | reject implementation; accessibility contract is invalid on macOS |
| macOS duplicate create | return stable already-exists error; no overwrite |
| plain cargo test returns `errSecMissingEntitlement` with DPK enabled | classify the harness as unauthorized; do not fall back to file-based keychain and do not count it as native DPK acceptance |
| bundled app capability probe/create returns `errSecMissingEntitlement` on DPK | latch the file-based login-keychain mode; create retries once with the same identity and without DPK/Accessible |
| bundled app DPK read/probe/replace misses an existing file-based item | retry the same identity without DPK; latch the mode only after successful discovery/update |
| bundled app DPK delete returns missing/auth-failed/missing-entitlement | retry non-DPK delete; missing after both attempts remains idempotent success |
| macOS SecretRef claimed supported without signed-app HIL | block the DPK capability claim; entitlement plist alone is not DPK evidence; file-based login keychain is a residual host path, not DPK HIL |
| Windows create sees an existing target | return stable already-exists error before `CredWriteW` |
| Windows documentation/code claims `CredWriteW` is atomic create-only | reject; Win32 specifies create-or-replace semantics |
| native store locked/denied/unavailable | source-free stable error/probe; no fallback |
| create/replace readback differs | fail verification; never delete an unproven current value; production activation additionally requires recoverable create ownership |
| serialized DTO/error/debug contains secret canary | test failure / NO-GO |
| Windows matching-host HIL did not execute | Windows SecretRef merge gate remains incomplete |
| SecretRef module requires `allow(dead_code)` to stay registered | remove dead registration or restore a real consumer |

## 5. Good / Base / Bad Cases

- **Good:** random SecretRef, one native backend, bounded zeroizing material,
  constant-time readback, source-free errors, process-latched macOS Keychain
  mode, and explicit matching-host CRUD HIL for the mode being claimed.
- **Base:** Windows cannot make `CredWriteW` atomically create-only. State the
  limitation accurately and use random identities + process-global serialization +
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

The focused contract suite also source-checks the target-gated native leaves so
DPK/non-sync query selection, app-bundle-only file-keychain residual, and the
no-blind-compensation rule remain executable on hosts that cannot access the
opposite platform credential store.

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

Correct DPK path (authorized access-group host):

```text
kSecUseDataProtectionKeychain = true
kSecAttrSynchronizable = false
kSecAttrAccessibleWhenUnlockedThisDeviceOnly
```

Correct residual path (signed `.app` whose DPK `SecItem*` returns -34018):

```text
kSecAttrSynchronizable = false
# omit kSecUseDataProtectionKeychain
# omit kSecAttrAccessible
```
