# OS keyring and native capture decision

## Evidence boundary

- Decision date: 2026-08-14.
- Audited implementation base: `afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`.
- Evidence level: `source_report + code_audit + static_design` only.
- No dependency resolution, test, build, browser, renderer, server, native runtime or screenshot was run for this decision.

## Decision

FyAgent owns one `SecretBackend` abstraction but calls the platform APIs directly. Do not add `keyring`, `keyring-core`, `apple-native-keyring-store` or `windows-native-keyring-store`; their facade/default persistence and error surfaces are not the product contract.

Planned manifest delta after `DESIGN_FREEZE`:

```toml
[dependencies]
zeroize = { version = "1.8.2", features = ["derive"] }
subtle = { version = "2.6.1", default-features = false }

[target.'cfg(target_os = "macos")'.dependencies]
security-framework = { version = "=3.7.0", default-features = false }
security-framework-sys = { version = "=2.17.0", default-features = false }
core-foundation = "=0.10.1"
core-foundation-sys = "=0.8.7"

[target.'cfg(target_os = "windows")'.dependencies]
# extend the existing windows = "0.61" feature set
# with Win32_Security_Credentials and the exact supporting handle types.
```

Current `src-tauri/Cargo.lock` read-only evidence freezes the macOS chain to `security-framework 3.7.0` (`b7f4bc775c73d9a02cde8bf7b2ec4c9d12743edf609006c7facc23998404cd1d`), `security-framework-sys 2.17.0` (`6ce2691df843ecc5d231c0b14ece2acc3efb62c0a398c7e1d875f3983ce020e3`), `core-foundation 0.10.1` (`b2a6cd9ae233e7f62ba4e9353e81a88df7fc8a5987b8d445b4d90c879bd156f6`) and `core-foundation-sys 0.8.7` (`773648b94d0e5d620f64f280777445740e61fe701025087ec8b57f45c791888b`). The separate locked `core-foundation 0.9.4` is not this API chain. Local registry manifests declare `MIT OR Apache-2.0`; MSRVs are respectively 1.85, 1.70, 1.65 and undeclared for the sys crate. This is static availability, not dependency resolution or advisory evidence. Repository `rust-version = "1.85.0"` remains unchanged; after freeze exact lock diff, license/advisory and matching-host Rust 1.85.0 all-targets checks are mandatory. Linux is explicitly `SECRET_BACKEND_UNAVAILABLE`; there is no file/env/plaintext fallback.

Material equality uses `subtle::ConstantTimeEq` rather than a handwritten loop. Its slice implementation intentionally short-circuits only on unequal length and is content-independent for equal-length inputs; this contract does not claim to hide API-key length. Both operands are already inside zeroizing native buffers and no comparison result exposes bytes.

所有direct API都位于registered wrapper后：durable state/journal/backend identity只使用`DeviceInstanceId=dev_*`；本次opened store另有type本身non-Clone/non-Serde的process-local identity，scope/handle/pending/receipt以`Arc<DeviceSecretStoreInstanceId>`同时绑定两种id与exact registered `Arc`。Wrapper以`Arc::ptr_eq`、durable/process identity、instance/generation和full record CAS校验。Platform raw results必须回报backend/device-binding generation；material、hint或receipt在wrapper出界前再核对，drift时在内层zeroize/drop。唯一`Arc<SecretService>` field直接持有唯一`Arc<BackendOperationBroker>`；private assembly/deps只搬运same Arc，caller/test不可注入或提取。Broker独占capture-intent/capability/pending恰好三个registries与private id；authorization由consuming scope/handle ownership承载，不另设registry。Owner模块只持role-specific opaque bundle，不能重排claim或把一个operation context用于另一个slot。

Capture同样不接受renderer构造的binding authority。`LegacySourceCoverageReceipt`是可由siblings命名/移动/消费但不可Clone/Serde/Debug的`pub(crate)` opaque authority；data fields与struct literal均private，唯一checked constructor是`pub(crate) checked_from_complete_inventory_authority`。Factory按value消费只有main-integration `CodexLegacySourceInventoryBridge`才能构造的`CompleteLegacySourceInventoryAuthority`，mint原子包含non-value-derived `LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable exact expectations + adjacentBlocked observations`的receipt。Identity恰好11个固定domain proof：`currentProviderLive`加`processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile|commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge`，每域都有structural revision、`absent|present`与匹配count，并与两组数据逐域一致。Receipt禁止raw path/raw locator/value/value-derived digest；只允许current exact expectations内部使用non-value-derived `LegacySourceLocationId`，adjacent observation不能变成`LegacySourceRef`。Backend-options row绑定durable/process store identity、owner/purpose/`newBinding|replaceBinding|legacyReconcile`、current owner-binding revision、该原子receipt与hidden bound expectation，并只能由broker内的`SecretCaptureIntentRegistry::mint_from_atomic_snapshot`生成短期单次`SecretCaptureIntentId`。Begin只回传intent id与exact registered backend instance；`claim_once`把receipt按value交回同一bridge，四字段与owner/backend全部fresh后才打开secure control。Startup Clean、owner summary/readiness与Provider-delete preview/confirm也必须各自通过该bridge fresh revalidate；缺失/stale/incomplete/omitted-domain、proof/data脱绑定或空集合没有11个absent proofs与两组empty data时一律effect-none Blocked。

## macOS Keychain store

High-level `security-framework 3.7.0` supplies `security_framework::access_control::{SecAccessControl,ProtectionMode}` and `security_framework::passwords::AccessControlOptions`; direct `security-framework-sys 2.17.0` plus `core-foundation 0.10.1`/`core-foundation-sys 0.8.7` supplies the create-only raw dictionary call. Locked 3.7.0 takes raw `CFOptionFlags`, so the policy shape is:

```rust
let access_control = SecAccessControl::create_with_protection(
    Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
    AccessControlOptions::empty().bits(),
)?;

```

The new-record arm uses lock-local `core_foundation::dictionary::CFDictionary<CFType,CFType>::from_CFType_pairs` to build a heterogeneous dictionary with exactly six entries and calls raw `security_framework_sys::keychain_item::SecItemAdd(dictionary.as_concrete_TypeRef(), null_mut())` once:

```text
kSecClass              = kSecClassGenericPassword
kSecAttrService        = CFString("com.fyagent.secrets.v1")
kSecAttrAccount        = CFString(validated SecretRef)
kSecAttrSynchronizable = CFBoolean::false_value()
kSecAttrAccessControl  = access_control
kSecValueData          = CFData(material)
```

No return-data/ref/attributes, label, authentication context, separate accessibility selector or sync-any/default selector is allowed. `PasswordOptions::set_generic_password_options` is not a create implementation. The raw wrapper is callable only from new-record create and never implements upsert.

`errSecDuplicateItem` never calls update. If the wrapper proves a fresh record/store/backend identity collision or drift it returns `SECRET_BACKEND_CHANGED/effect=none`; otherwise the operation maps to `SECRET_WRITE_FAILED`. The checked terminal context derives action/retry/effect; either branch leaves state/journal/binding unchanged and has `SecItemUpdate` call count zero.

`AccessibleWhenUnlockedThisDeviceOnly` is mandatory: the item is available only while the device is unlocked and does not migrate to another device. Empty access-control flags deliberately add no per-use biometric/passcode confirmation; this remains `hostUser + osProtectedStore`, not a hardware/Secure-Enclave claim. `kSecAttrSynchronizable=CFBoolean::false_value()` is independently mandatory. Disabling a Cargo feature is not proof of non-synchronizing behavior.

Find/read and delete use fresh query-only options containing exactly generic-password class, fixed service, validated account and `synchronizable=false`; read additionally asks for data. They never carry `SecAccessControl`, authentication context, label or caller policy. The access-control object is stored create policy, not a lookup selector, caller authority or delete capability.

Replace uses an explicit update arm rather than `set_generic_password_options` with create options. Under the exact backend-instance mutex it first query-reads data/attributes with only service/account/non-sync and verifies the existing item has `AccessibleWhenUnlockedThisDeviceOnly`. Then lock-local `security_framework::item::update_item`/`SecItemUpdate` equivalent uses that same query and a data-only update dictionary, preserving access control. Query/update not-found is stale/dependency/backend-changed with `effect=none` and never creates; duplicate create never updates. No branch may create an item with default accessibility, and no update/search dictionary may contain the access-control object.

Create and update both finish with query-only material readback/constant-time equality plus attribute readback asserting `kSecAttrSynchronizable=false` and `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`. Any mismatch fails closed and enters the typed compensation/recovery path.

The returned `Vec<u8>` is immediately moved into `Zeroizing<Vec<u8>>`. A write is accepted only after a fresh read and constant-time equality check; delete is accepted only after a fresh read returns item-not-found. The backend is `hostUser + osProtectedStore`; it is not described as Secure Enclave or hardware-backed.

| OSStatus | Stable result |
| --- | --- |
| `errSecDuplicateItem (-25299)` during new-record create | proven fresh identity collision/drift → `SECRET_BACKEND_CHANGED`, otherwise operation-specific `SECRET_WRITE_FAILED`; effect none and never update |
| `errSecItemNotFound (-25300)` | `SECRET_MISSING`, presence `missing` |
| `errSecInteractionNotAllowed (-25308)` / `errSecInteractionRequired (-25315)` | `SECRET_LOCKED`, `lockSource=backend`, presence `unknown`; expected locked mapping for the chosen accessibility |
| `errSecAuthFailed (-25293)` / `errSecMissingEntitlement (-34018)` | `SECRET_PERMISSION_DENIED`, presence `unknown` |
| `errSecNotAvailable (-25291)` / `errSecNoDefaultKeychain (-25307)` / `errSecNoStorageModule (-25312)` | `SECRET_BACKEND_UNAVAILABLE`, presence `unknown` |
| `errSecUserCanceled (-128)` | operation-scoped cancel; stable state unchanged |
| `errSecDataTooLarge (-25302)` or validated material call returns `errSecParam (-50)` | `SECRET_INPUT_INVALID` for capture/write |
| fixed access-control/query/update construction returns `errSecParam (-50)` | `SecretSourceFreeErrorCode::Internal + exact SecretTerminalOperationContext`; derive action/retryable/effect only from the exhaustive 47-code/24-action table and never blame caller input |
| other numeric status | operation-specific read/write/delete failure; presence `unknown` |

Only numeric status and stable mapping may cross the adapter. `Display`, `Debug`, arbitrary `source()` or OS text never reaches logs, audit, diagnostics, IPC or renderer.

Primary references: [Apple Keychain Services](https://developer.apple.com/documentation/security/keychain-services), and lock-local `security-framework-3.7.0/{access_control.rs,passwords_options.rs,item.rs}`, `security-framework-sys-2.17.0/{item.rs,keychain_item.rs}` plus `core-foundation-0.10.1/{base.rs,dictionary.rs}`. Native evidence must cite the exact lock hash, not a floating docs page.

## macOS native capture

The native dialog is separate from Keychain I/O. Extend the existing `objc2-app-kit 0.2.2` features with `NSAlert`, `NSApplication`, `NSButton`, `NSControl`, `NSResponder`, `NSSecureTextField`, `NSTextField` and `NSView`.

The async command reserves a single capture slot, schedules `NSAlert + NSSecureTextField` through `AppHandle::run_on_main_thread`, and returns through a `tokio::sync::oneshot`. The main-thread closure does no Keychain/file/DB I/O and does not await. On accept, the field is copied once into `Zeroizing<Vec<u8>>`, the field is cleared, and only native code receives `SecretMaterial`; cancel/window close produces `SECRET_INPUT_CANCELLED` with no record, candidate, binding or backend write.

The acceptance claim is limited to secure native control, no renderer/IPC material and zeroization of application-controlled Rust buffers. It does not claim the framework has no internal temporary copies.

## Windows Credential Manager store

Use existing `windows 0.61` bindings for `CredWriteW`, `CredReadW`, `CredDeleteW` and `CredFree`, adding `Win32_Security_Credentials` and any exact handle type feature required by the generated bindings.

```text
Type       = CRED_TYPE_GENERIC
TargetName = "FyAgent/secret/v1/" + validated SecretRef
Persist    = CRED_PERSIST_LOCAL_MACHINE
Flags      = 0
UserName   = "FyAgent"
Blob       = raw UTF-8 material, length 1..=2560 bytes
```

No default/Enterprise persistence, search/enumeration or fallback is allowed. The exact backend instance serializes store calls. After `CredReadW`, validate type/target/persistence/blob length, copy into `Zeroizing<Vec<u8>>`, zero the returned blob with a non-optimizable volatile clear plus compiler fence, then call `CredFree`. Write requires readback equality; delete requires confirmed missing. `ERROR_NOT_FOUND` is missing for probe and idempotent success while reconciling an admitted delete.

Every leaf result carries returned backend/device-binding generations. The registered wrapper verifies those values plus lifetime store instance and exact Arc before any blob/material/delete/missing receipt exits; the service cannot fill generations in after the call. Delete and fresh missing readback remain two operation-broker slots and two platform calls with a durable receipt checkpoint between them.

| Windows condition | Stable result |
| --- | --- |
| `ERROR_NOT_FOUND` | `SECRET_MISSING`, presence `missing` |
| `ERROR_NO_SUCH_LOGON_SESSION` | `SECRET_BACKEND_UNAVAILABLE`, presence `unknown` |
| access denied / policy denial | `SECRET_PERMISSION_DENIED`, presence `unknown` |
| invalid parameter / oversize | `SECRET_INPUT_INVALID` |
| any ambiguous or unclassified store result | operation-specific read/write/delete failure, presence `unknown` |

Raw Win32 error text/code is adapter-private and never serialized or logged.

Primary references: [CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw), [CredWriteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew), [CredReadW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw), and [CredDeleteW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creddeletew).

## Windows exact native capture

Use only `CredUIPromptForCredentialsW`; do not mix it with `CredUIPromptForWindowsCredentialsW`, `CredUnPackAuthenticationBufferW` or `CREDUIWIN_*` flags.

- `CREDUI_INFOW.hwndParent` is the non-null HWND of FyAgent's `main` Tauri window. Failure to get that HWND is `SECRET_BACKEND_UNAVAILABLE`; do not fall back to a desktop-parent dialog.
- Allocate username `CREDUI_MAX_USERNAME_LENGTH + 1` and password `CREDUI_MAX_PASSWORD_LENGTH + 1` UTF-16 code units in zeroizing buffers.
- Use `CREDUI_FLAGS_GENERIC_CREDENTIALS | CREDUI_FLAGS_ALWAYS_SHOW_UI | CREDUI_FLAGS_DO_NOT_PERSIST | CREDUI_FLAGS_EXCLUDE_CERTIFICATES | CREDUI_FLAGS_KEEP_USERNAME`.
- Do not set `SHOW_SAVE_CHECK_BOX`, `PERSIST` or `EXPECT_CONFIRMATION`; `save` remains false. Capture target `FyAgent/secret-capture/v1` differs from the explicit store target.
- `NO_ERROR` parses only the password prefix before NUL, validates UTF-16/UTF-8/length/non-empty, copies once into `SecretMaterial`, then zeroizes both full buffers. `ERROR_CANCELLED` is `SECRET_INPUT_CANCELLED`; no side effect occurs.
- UI runs via main-thread scheduling + oneshot; it performs no Credential Manager/file/DB I/O.

Primary references: [CredUIPromptForCredentialsW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-creduipromptforcredentialsw) and [CREDUI_INFOW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credui_infow).

## Error and material boundary

Because production does not use `keyring-core`, its byte-carrying `BadEncoding` / `BadDataFormat` variants and ambiguous store variants are outside the production graph. A dependency/source gate rejects the four store crates named above.

Any future plugin adapter with a byte-carrying error must destructure it by value, zeroize the bytes, and return only a stable code. It may never format, chain, log or serialize the original error. `SecretMaterial` and process-local prepared capabilities have no `Serialize`, `Deserialize`, `Clone` or ordinary `Debug`; material is acquired only after one-shot capability revalidation and cannot leave the sealed native consumer result type.

Every `Backend*OperationContext` type, field and factory is private. Only `BackendOperationBroker::for_apply|for_runtime|for_activation|for_recovery|for_migration|for_staged_import|for_non_apply` may consume the matching opaque admission/readiness/journal/runtime/staged claims and return `BrokeredBackendOperationContext`; the exact registered wrapper then calls only `prepare_brokered_operation`. The broker exclusively owns capture-intent/capability/pending registry state and private ids, atomically combines capability claim with role extraction and consumes discard/terminalization by value; authorization remains owned by the resulting consuming scope/handle, not a fourth registry. Non-public production `SecretServiceDeps` may carry the same broker Arc into `SecretService`, but it has no registry fields and no caller/public/test setter, trait injection or extractor; the one `Arc<SecretService>` cannot expose either broker or registries.

Device/native errors enter the contract only through closed `SecretSourceFreeErrorCode + SecretTerminalOperationContext` or the typed `locked|revoked|backend_unavailable|operation_recovery_required` factories behind the sole private `SecretInternalError::checked` literal. General recovery requires a pointer; candidate-terminal cleanup is the only pointer-free exception. The exhaustive 47-code/24-action table routes every capture action to `secretCaptureFlow` and each of four runtime retries to its exact `fixedRuntimeFlow`; there is no raw-error constructor, unrouted fallback action or unregistered legacy placeholder.

Persistent central/device revocation is a separate explicit operation. Ordinary read/probe may return only a non-persistable `BackendRevocationHint`. Only an exact registered handle that consumes `SecretNonApplyBackendOperation::Revoke` / exact `General::Revoke` authorization may call `observe_revocation_once`; after the wrapper validates `centralRevocation=true + SourceAndTime`, lifetime store instance, exact Arc, full CAS and returned generations, it may mint the non-clone consuming `BackendRevocationObservation`. OS keyring always advertises `centralRevocation=false`.

## Hardware preservation

The OS backends do not model hardware as a Boolean on a singleton. Future hardware registration is per instance and per record, with exact instance/generation, device-binding generation, capability revision, device display metadata, per-operation physical confirmation, allowed consumers/sinks, storage residency, persistent projection and revocation behavior. Confirmation projection has exactly five operations: `CaptureVerify|Validate|ResolveForApply|Delete|Revoke`; there is no `MissingReadback` operation/policy. Activation uses `ActivationConfirmationSlot::{CandidateRead,OldRecordDelete,OldRecordMissingReadback}`; recovery uses `RecoveryConfirmationSlot::{ActiveRecordRead,OldRecordDelete,OldRecordMissingReadback,UncommittedRecordDelete,UncommittedRecordMissingReadback,AdmittedRecordDelete,AdmittedRecordMissingReadback}`. Thus delete/missing-specific slots are exactly eight and the two enums contain ten activation/recovery slots total. Every missing slot executes `Validate` and copies the record's `operationConfirmation.validate`, while retaining its own scope/authorization/receipt/checkpoint and consuming the actual `BackendDeleteAppliedCas` minted only after its matching delete receipt is durable; one hardware prompt/authorization never silently covers both slots.

No hardware adapter is implemented in MVP. When none is registered, Add/Replace UI hides hardware; an existing hardware binding displays unavailable/device mismatch and never falls back to OS keyring. `persistentTargetProjection=false` is checked at #55 preview and again inside #35 immediately before resolve.

## Native evidence boundary

Mocks/fault injectors prove only module/failure behavior. Acceptance separately requires:

- macOS real Keychain create/read/replace/delete/missing plus user-visible `NSSecureTextField` accept/cancel UAT; native assertions must prove raw six-key create calls `SecItemAdd` once, duplicate never updates, replace-not-found never creates, create attributes are `AccessibleWhenUnlockedThisDeviceOnly + synchronizable=false`, replace preserves them, update/find/delete search excludes the access-control object, and locked statuses map to `SECRET_LOCKED/backend/unknown`;
- Windows x64 real Credential Manager CRUD/replace/delete/missing with `LOCAL_MACHINE`, plus user-visible CredUI accept/cancel UAT;
- every run uses a fresh random valid ref and confirms cleanup;
- injected unavailable/delete failures use `evidence_class=failure_path, evidence_origin=fault_injection`, never native OS denial;
- matching native macOS and Windows x64 each record an exact Rust 1.85.0 `cargo check --locked --workspace --all-targets` raw/CI leaf plus Cargo.lock hash; Rust 1.97 cannot substitute;
- source registration proves exactly 15 #35 `SecretCommandName` handlers plus the separately registered `resume_staged_import_cutover` main-integration handler, never a 16th #35 command;
- portable source/integration evidence proves the unique `CodexLegacySourceInventoryBridge`/unforgeable authority/`pub(crate)` factory path, the atomic `LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked` receipt, all 11 domain proofs/data bindings, and effect-none failure for missing/stale/incomplete/omitted/duplicate/unknown/drifted or split coverage—including aggregate-empty without 11 absent proofs and two empty data sets—across startup, summary/readiness, capture options/claim and Provider-delete preview/confirm;
- exact source SHA, OS/arch, command, timestamps, exit/counts and artifact scan appear in the evidence manifest.

These are future evidence requirements only; adding accessibility/query assertions here does not claim they ran. Without Windows x64 `native_runtime`, all fixed Windows failure-path items and Windows capture UAT, Issue #35 remains non-DONE.
