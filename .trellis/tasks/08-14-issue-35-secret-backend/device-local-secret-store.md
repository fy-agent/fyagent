# Issue #35 device-local secret store / native closure

## 0. 文档定位与结论

本附件只闭环 device-local metadata/state/journal、双平台 native store/capture、启动/导入/恢复顺序与 native evidence 路径。它是主线程修订 `technical-design-overview.md`、`detailed-design-overview.md`、`execution-plan.md` 和三份 review disposition 的输入，不单独构成 `DESIGN_FREEZE`，也不是实现或 runtime evidence。

静态核对基线：`afc317a7be3883a4a438d48fad5fb8b6d1f1f5ab`。本附件生成阶段没有运行 dependency resolution、test、build、browser、renderer、server、native runtime 或 screenshot。

本附件记录以下待复核裁定：

1. **#35 撤回 SQLite v17 所有权。** `SCHEMA_VERSION`、`database/schema.rs`、数据库 migration/test 不由 #35 修改；Prompt/Memory native lane 保留既有 v17 adjudication。#35 的 secret record、binding、operation journal、audit 与 per-device capability 全部落在独立的 device-local file store，版本号为 `device-local-secret-store/v1`，与 SQLite `PRAGMA user_version` 无关。
2. **device-local authority 不使用 `get_app_config_dir()`。** 生产根目录固定从 Tauri `app.path().app_local_data_dir()` 解析，避免 `~/.fyagent` override、SQL export、binary backup、WebDAV/S3 snapshot 或用户把配置目录指向同步盘时把 binding 带走。
3. **撤回 keyring store crate 候选。** 不引入 `keyring`、`keyring-core`、`apple-native-keyring-store` 或 `windows-native-keyring-store`。macOS exact direct chain冻结为`security-framework =3.7.0 + security-framework-sys =2.17.0 + core-foundation =0.10.1 + core-foundation-sys =0.8.7`，raw CFDictionary/`SecItemAdd`只负责new-record create；Windows直接使用已有`windows 0.61`的`CredReadW/CredWriteW/CredDeleteW`；`zeroize 1.8.2`作为direct dependency。
4. **MVP 的 `rust-version` 保持 `1.85.0`。** `security-framework 3.7.0` 声明Rust 1.85，`zeroize 1.8.2`声明Rust 1.60，`windows 0.61`已在当前manifest/lock中。Freeze后仍须做exact lock、license、advisory，并在matching native macOS/Windows x64各跑Rust 1.85.0 locked workspace all-targets gate；当前Rust 1.97不能替代。本结论不是dependency-resolution/build evidence。
5. **OS store 与 local state 之间采用 durable write-ahead journal。** 任一 backend mutation 前先持久化 intent；operation kind恰好八类，每类有独立required authority/phase。Crash只映射四臂strict recovery或staged/discard自己的checkpoint，不依赖keyring枚举，也不创建generic recovery operation。
6. **device-local binding 永不进入 Provider row。** Provider SQLite 只保留 scrub 后的非敏感配置；`secretRef`、binding、backend instance/capability 只存在于本机 store。远端 Provider row 导入后按本机 owner key 重新关联，远端不能夺走本机 binding。
7. **local schema/ID/候选语义采用并发收敛的 `secret-contract-v1.md`。** capture/replace/rotate/migrate先产生`verifiedPendingPlan` candidate；它们不直接改binding或scrub Provider。只有#55 admitted immutable Change Plan触发的native `activateCandidate`才能切binding/scrub；discard/expiry不确定时以immutable `pendingTerminalDisposition`保持candidate/journal可达。
8. **Provider delete先做orthogonal binding/legacy discovery。** 任一current legacy source都阻断preview并返回effect-none resolve action；只有no-legacy bound/unbound分支可mint impact。Owner detach不删除backend record。
9. **Staged import只有一条authority顺序。** `temp token/projection -> #55 admission -> main-integration authority-match receipt -> #35 prepare/confirm -> cutover context -> staged source read/validate/compare -> scrub/readback -> cutover`；任何其他排序都不能写journal或进入source/cutover port。唯一public resume request是`stageId + expectedResumeCas{revision,digest}`，独立no-value result三arm每次都返回`currentResumeCas`且不含candidate/owner/ref/summary；durable object、fresh process nonce、owner、admission、record/backend/checkpoint与promoted-owner identity只作为digest preimage。每次fresh nonce/admission都递增revision、重算digest并使旧request stale；cancel/discard只由同一broker持有的一次性authority完成。
10. **Backend authority先于platform API。** Durable authority只使用`DeviceInstanceId=dev_*`；每次open另mint non-Serde/non-Clone的process-local `DeviceSecretStoreInstanceId`，live handle/scope以`Arc<DeviceSecretStoreInstanceId>`同时绑定两种identity、exact registered `Arc`、instance/generation与record handle。Process identity绝不进入state/journal/audit/backend locator。Platform raw result必须回报backend/device generation，registered wrapper在任何material、receipt或hint出界前复核。Central/device revocation只能由显式`Revoke` authorization驱动`observe_revocation_once`并产生full-CAS、non-clone、consuming receipt；普通read/probe最多返回不可持久化hint。
11. **Capture authority来自broker独占的native短期registry。** Backend options query在一个snapshot内读取durable/process store identity、owner/purpose、capture kind、current owner-binding revision与hidden bound expectation；唯一`CodexLegacySourceInventoryBridge`构造`CompleteLegacySourceInventoryAuthority`，由`pub(crate) LegacySourceCoverageReceipt::checked_from_complete_inventory_authority`按value消费并mint原子包含non-value-derived inventory revision、exact 11-domain identity、current-scrubbable exact expectations与adjacent-blocked observations的opaque receipt。Begin只接受intent id和exact backend instance id；operation broker原子claim、bridge fresh revalidate完整receipt并选择自己的private operation context，renderer从不构造binding/legacy authority。New、replace与legacy-conflict reconcile共用这一typed flow。

## 1. Review closure 对照

| Finding | 本附件的 closure input |
| --- | --- |
| AR-001 / DD-008 | §5–§7：durable intent、phase、crash matrix、startup reconciliation |
| AR-002 | §3、§9：独立 local-data root；binding/state/journal/audit 不进入 WebDAV/S3/SQL/backup |
| AR-003 / DD-007 | §8：startup、live import、SQL import、manual restore、sync download 的 staged gate |
| AR-005 | §0、§12：撤回 v17；canonical manifest owner仍只允许 `#35 module | #55 | #41 | main integration`，Prompt/Memory是保留的外部schema lane/dependency而非第五个owner literal |
| AR-006 / PR-003 | §11：capability 绑定 record/binding/backend/capability revisions，resolve 前重检 |
| AR-008 / PR-005 | §7.2：只有成功 read + constant-time equality 才能 scrub existing binding 的 inline value |
| AR-009 / PR-010 / DD-009 | §4、§7：monotonic binding-set revision + exact owner impact CAS；lifecycle 拆列 |
| AR-010 / DD-009 | §4.1：严格 UUIDv4 `SecretRef`；所有 JSON/IPC 反序列化统一验证 |
| AR-011 / DD-004 | §10：exact `CredUIPromptForCredentialsW`、flags、HWND、buffer、zeroize；macOS feature/oneshot |
| AR-012 / PR-012 | §11：stable backend instance、per-secret capability/generation；未注册 hardware 时 UI 不可选 |
| DD-005 | §10.4：direct dependency/MSRV 决策 |
| DD-010 | §9.3、§10.3：AppState 同 DB 注入、spawn_blocking、锁序、真实 keyring test gate |
| DD-013 | §13：Windows x64 exact-SHA delivery、pre-evidence push、real/injected evidence 分类 |
| PR-006 | §8.5：managed history scan/report 与用户确认 cleanup 分离 |
| PR-008 | §4.3、§11：稳定 summary 不含 operation-scoped confirmation；step 只在 prepare response |
| PV7-001 | §8.3：public staged resume只含`stageId + expectedResumeCas`；full identity只进入CAS preimage，fresh nonce/admission使旧request stale |
| PV7-002 | §7.1.1、§11.3：terminal expiry先返回`refreshSummary`，再由fresh owner/card snapshot mint全新capture/rotation authority；不复用旧candidate/operation |
| PV7-003 | §4.5、§7.2：legacy conflict进入同一typed capture-intent registry；renderer只回传intent id与exact backend selection |
| CAV7-001 | §8.3：唯一staged顺序固定为temp token/projection → #55 admission → authority-match receipt → #35 prepare/confirm → cutover context → source read/validate/scrub/readback → cutover |
| CAV7-002–004 | §4.2、§4.4、§6–§7：delete finalization、capture compensation、activation/activation recovery各自拆分delete与fresh missing-readback slot、authorization、receipt和durable checkpoint |
| CAV7-005 | §4.4、§7.5、§11.3：只有显式`Revoke` authorization可调用`observe_revocation_once`并mint持久化receipt；ordinary hint不可持久化 |
| CAV7-006 | §4.4、§10：scope/receipt绑定lifetime store instance与exact registered Arc；wrapper先复核platform返回generation |
| CAV7-007–008 | §4.4、§11.3：operation broker封装private capability id、claim/discard与role extraction；每种backend context私有并消费本种opaque authority |
| CAV7-009 | §8.2、§11.1：`SecretBootstrapToken`为合法可命名的`pub(crate)` sibling token、字段/constructor仍private且只能从opened store借用 |
| CAV7-010 | §11.3：device/native只使用合同的exhaustive private `SecretInternalError` factories与total action destination；无unrouted fallback或unregistered legacy destination |

此表只代表本模块给出的 closure design。主线程仍须把裁定同步到所有权威文件并由三位 reviewer 对同一 exact working tree 重新回读。

## 2. Authority 与 ownership 边界

### 2.1 #35 secret module 独占的新文件

唯一canonical exact path list是`research/codex-secret-call-graph.md` §9.1；本附件不维护第二份文件表。该列表内每个新secret/core/platform/capture/V2/scanner path的manifest owner均为`#35 module`，且它们不拥有Provider lease、Change Plan、SQLite schema、backup cutover或app startup registration。

### 2.2 canonical owner `main integration` 的 shared files

唯一canonical shared-file list是`research/codex-secret-call-graph.md` §9.4；本附件不使用“task docs”等泛称扩张owner。Canonical owner `main integration`（executor `root/MainIntegrationOwner`）串行负责该exact list中的dependency、AppState/bootstrap、startup/register、Provider/DAO、SQL/restore/sync staging、sanitized backup/export、现有call-site迁移及UI composition。

### 2.3 其他 task 的保留所有权

- Prompt/Memory native lane：SQLite v17 与其 `database/{mod,schema,tests}.rs` migration。#35 不占新 schema number。
- #55：Change Plan preview/readiness、plan hash、owner/ref/sink/capability revision、immutable baseline；不得计算 material 或 secret-bearing Provider/live projection 的 digest。
- #41：Provider lease、baseline recheck、sanitized backup、existing writer、readback/rollback 与 `PreparedSecretCapability` 的最终消费顺序。
- main integration：public `Provider` DTO boundary、legacy/live import staging、proxy/model-fetch/usage/deeplink 等共享 call graph。device-local store 只提供 exact service API，不接管这些文件。

## 3. Device-local 文件布局与权限

### 3.1 固定根目录

生产路径：

```text
<app.path().app_local_data_dir()>/device-local/secrets/v1/
```

按 Tauri 当前 path contract，`app_local_data_dir()` 解析为 OS local-data root + `com.fyagent.desktop`。该路径不可被 app config override 改写。测试必须显式注入 `TempDir`，不得读取生产路径。

目录布局：

```text
v1/
├── store.lock
├── state.json
├── .tmp-state-<32-lowercase-uuidv4-hex>.json
├── journal/
│   ├── sop_<32-lowercase-uuidv4-hex>.json
│   ├── .tmp-journal-sop_<32-lowercase-uuidv4-hex>-<32-lowercase-uuidv4-hex>.json
│   └── .retired-sop_<32-lowercase-uuidv4-hex>.json   # Windows only, transient
└── audit/
    └── sae_<32-lowercase-uuidv4-hex>.json
```

- `state.json`：本机 compiled truth；record、candidate、binding、migration gate、backend instance/capability 与 managed artifact scan summary。
- `journal/*.json`：未完成/待回收操作；一 operation 一文件，文件名只由服务端 `OperationId` 生成。
- `audit/*.json`：一 event 一文件，append-only create-new；不保存 raw path、OS error、material、material digest 或 arbitrary JSON。
- `store.lock`：进程 lifetime exclusive lock；内容为空，open handle 保持到 `SecretRuntime` Drop。

`secretRef` 永不参与文件名或路径拼接，只作为 validated JSON field 与 OS-store entry key。

初始化规则：只有 root 是本进程刚创建、除已锁定的 `store.lock` 外为空时，才允许生成第一份 state 与 `DeviceInstanceId`。已存在 root 中 `state.json` 缺失、hash invalid、存在未知文件或仅剩 journal/audit 时，一律 recovery-required；不得生成空 state 覆盖可能仍存在的 OS entries。唯一可自动处置的残留是 §5.2 精确验证通过的 durable-replace temp 与 Windows terminal `.retired-*` tombstone；验证、identity或terminal audit任一不成立仍 fail closed。MVP 不自动清理 audit；retention 需要后续显式策略/用户动作。

### 3.2 Unix/macOS 权限与对象检查

- root、`journal/`、`audit/`：`0700`；创建后 `fchmod`，每次 open 重新验证 `st_uid == geteuid()` 且 `(mode & 0o077) == 0`。
- `store.lock`、state、journal、audit、temp file：`0600`。
- 目录 component 使用 no-follow open；文件使用 `O_NOFOLLOW | O_CLOEXEC`，`fstat` 必须为 regular file、link count 1。遇 symlink、owner drift、world/group bit、non-regular object一律 `SECRET_OPERATION_RECOVERY_REQUIRED`（existing store）或 `SECRET_BACKEND_UNAVAILABLE`（fresh open），不自动修复或回退。
- lifetime lock 使用 `flock(fd, LOCK_EX | LOCK_NB)`；无法取得锁时本实例不开放 secret mutation/resolve。

### 3.3 Windows ACL 与对象检查

- root 及全部子对象使用 protected DACL，只允许 frozen interactive-user SID 与 `LOCAL_SYSTEM` `FILE_ALL_ACCESS`；移除继承。创建后 read back owner/DACL，不匹配即 fail closed。
- `CreateFileW` 使用 `FILE_FLAG_OPEN_REPARSE_POINT`，对每个 component 拒绝 `FILE_ATTRIBUTE_REPARSE_POINT`；state/journal/audit 必须是 non-directory regular file。
- `store.lock` open handle 使用 `LockFileEx(LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)` 覆盖 byte 0..1，handle 保持到 runtime Drop。
- desktop process token SID 必须与启动时冻结的 interactive-user SID 相同；若 elevated/different-user token，OS store 与 local root 都标 `SECRET_BACKEND_UNAVAILABLE`，不得写入管理员 credential set 或调用另一用户的 helper 传送 material。

### 3.4 Bounds

- `state.json` 最大 4 MiB；单 journal 64 KiB；单 audit event 32 KiB；超限 fail closed。
- JSON object 全部 `deny_unknown_fields`；枚举未知值、duplicate identifier、unsorted/duplicate record、非法 timestamp、zero revision 均拒绝。
- timestamps 为 native service 生成的 RFC 3339 UTC；public caller 不提供 timestamp、operation/event/device/backend instance ID。

## 4. Exact local schema 与 strict identifiers

### 4.1 `SecretRef`

唯一合法形式：`sec_` + 32 位 lowercase UUIDv4 simple hex，总长度 36 ASCII bytes。

验证顺序必须一致用于 IPC、journal/state load、backend lookup 与 import：

1. 不 trim；精确长度 36，bytes `0..4 == b"sec_"`。
2. suffix 每个 byte 只能是 `0-9` 或 `a-f`；uppercase、Unicode、separator、NUL 全拒绝。
3. suffix 第 13 个 hex nibble（zero-based index 12）必须为 `4`。
4. suffix 第 17 个 nibble（index 16）必须为 `8|9|a|b`。
5. `uuid::Uuid` parse 后必须为 RFC4122 variant、version 4，且 `uuid.simple().to_string()` 与原 suffix byte-for-byte 相等。

Rust `SecretRef` 只有 `generate()` 与 validating `TryFrom<String/&str>`；Serde 使用 validating `try_from`。`SecretRefDisplay` 是单独 output-only type，绝不被 parser 接受。不存在从 owner/value/backend/hash 构造 ref 的 API。

同类 strict native IDs：

```text
DeviceInstanceId  = dev_ + UUIDv4 simple lowercase hex
SecretCandidateId = scd_ + UUIDv4 simple lowercase hex
SecretBackendInstanceId   = sbi_ + UUIDv4 simple lowercase hex
SecretOperationId         = sop_ + UUIDv4 simple lowercase hex
SecretAuditEventId        = sae_ + UUIDv4 simple lowercase hex
SecretConfirmationStepId  = scs_ + UUIDv4 simple lowercase hex
SecretRecoveryId          = src_ + UUIDv4 simple lowercase hex
SecretCaptureIntentId     = sci_ + UUIDv4 simple lowercase hex
```

`DeviceSecretStoreInstanceId`不是上述durable/public ID。它是`SecretBootstrap::open`每次成功取得lifetime lock后随机mint的process-local `[u8;16]` opaque nonce，type本身无Serde/text/Clone/Debug；opened store创建唯一`Arc<DeviceSecretStoreInstanceId>`，随后只把该Arc的clone放入同一`SecretService`拥有的live handle/scope/pending/receipt，teardown时整体失效。Durable `DeviceInstanceId`仍表示当前设备namespace，二者不可互换。`state.json`、journal、audit、backend instance identity和所有canonical hash/preimage只编码durable `deviceInstanceId=dev_*`，绝不编码process nonce、其地址或派生值。每条loaded record在进入backend broker前都包装为同时持有`DeviceInstanceId + Arc<DeviceSecretStoreInstanceId> + RegisteredBackendHandleBinding`的process-local handle；任一durable或process identity不匹配都在platform call前拒绝，因而复制state不能伪造live handle，旧进程的scope/receipt也不能在新open store中重放。

`BackendDeleteAppliedCas { revision: BackendDeleteAppliedRevision, digest: RecoveryStructureDigest }`是不同于store/recovery CAS的operation-bound structural checkpoint。Broker在hardware prepare前只能持有`BackendDeleteAppliedCasReservation { operationId, expectedRevision }`；delete/already-missing receipt已durable写入exact `backendApplied|OldRecordDeleteApplied`后，authority才从该journal的credential-free preimage mint actual CAS。Missing-readback authorization必须消费与reservation同operation/revision一致的actual CAS，`BackendMissingReadbackReceipt`也绑定它；因此预先physical confirm不能越过durable delete checkpoint。

### 4.2 `state.json`

Envelope：

```json
{
  "schemaVersion": 1,
  "hashAlgorithm": "sha256",
  "payloadSha256": "64-lowercase-hex",
  "payload": {
    "deviceInstanceId": "dev_...",
    "storeRevision": 1,
    "createdAt": "RFC3339 UTC",
    "updatedAt": "RFC3339 UTC",
    "backendInstances": [],
    "secrets": [],
    "candidates": [],
    "recoveries": [],
    "ownerBindings": [],
    "ownerMigrations": [],
    "managedArtifactScan": null
  }
}
```

数组按 stable primary key byte-order 排序；duplicate key 拒绝。Native-only payload exact records：

```text
BackendInstanceRecord {
  backendInstanceId,
  backendKind: "osKeyring" | "hardware",
  deviceInstanceId,
  generation: u64 >= 1,
  pluginId?: validated ASCII <= 64,
  deviceDisplay?: {
    displayName: service-produced validated SafeDisplayText,
    deviceClass: "osAccount" | "securityKey" | "secureElement" | "unknown",
    transport: "platform" | "usb" | "nfc" | "ble" | "unknown"
  },
  registered: bool,
  createdAt,
  updatedAt
}

SecretRecord {
  secretRef,
  purpose: "codexApiKey",
  backendInstanceId,
  backendLocator?: validated opaque non-secret ASCII <= 128,
  recordRevision: u64 >= 1,
  bindingSetCas: { revision, digest, count },
  backendGeneration: u64 >= 1,
  deviceBindingGeneration: u64 >= 1,
  capabilityRevision: u64 >= 1,
  policyState: "active" | "locked",
  retirementState: "live" | "stale" | "revoked",
  capabilities: SecretRecordCapabilities,
  rotatedFromRef?: SecretRef,
  createdAt,
  updatedAt,
  rotatedAt?,
  lastValidatedAt?,
  revokedAt?,
  revocationSource?: "userDelete" | "centralBackend" |
                     "deviceAdministration" | "supersededByRotation"
}

SecretCandidateRecord {
  candidateId,
  candidateRevision: u64 >= 1,
  kind: "newBinding" | "replaceBinding" | "rotateBindingSet" |
        "legacyReconcile" | "legacyScrubExistingBinding",
  state: "verifiedPendingPlan" | "activated" |
         "discarded" | "cleanupRequired" | "expired",
  secretRef,
  recordRevision,
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  targetOwners: sorted SecretOwner[],
  expectedBindings: sorted OwnerBindingExpectation[],
  legacySourcesToScrub: sorted LegacySourceExpectation[],
  createdAt,
  expiresAt,
  updatedAt,
  pendingTerminalDisposition?: "discarded" | "expired"
}

SecretRecoveryRecord =
  | ActivationCleanupRecoveryRecord
  | CaptureCompensationRecoveryRecord
  | DeleteFinalizationRecoveryRecord
  | OwnerDetachFinalizationRecoveryRecord

RecoveryAffectedOwner {
  owner,
  ownerBindingRevision,
  secretRef,
  bindingRevision
}

ActivationCleanupRecoveryRecord {
  kind: "activationCleanup",
  recoveryId,
  recoveryCas: { revision: SecretRecoveryRevision, digest: 64-lowercase-hex },
  candidateId,
  candidateRevision,
  activeSecretRef,
  activeRecordRevision,
  affectedOwners: sorted non-empty RecoveryAffectedOwner[],
  state: ActivationCleanupRecoveryState,
  createdAt,
  updatedAt
}

ActivationCleanupRecoveryState =
  | {
      phase: "stateFinalized" | "providerFinalized" |
             "oldRecordDeleteIntent",
      remainingSteps: sorted non-empty ActivationCleanupRecoveryStep[]
    }
  | {
      phase: "oldRecordDeleteApplied",
      deleteDisposition: "deleted" | "alreadyMissing",
      backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas,
      remainingSteps: [ActivationVerifyOldRecordMissingStep]
    }
  | {
      phase: "recoveryRequired",
      checkpoint:
        | { state: "none" }
        | {
            state: "oldRecordDeleteApplied",
            deleteDisposition: "deleted" | "alreadyMissing",
            backendCompletedAt,
            deleteAppliedCas: BackendDeleteAppliedCas
          },
      remainingSteps: sorted non-empty ActivationCleanupRecoveryStep[]
    }
  | {
      phase: "terminal",
      oldRecord:
        | { status: "notApplicable" }
        | {
            status: "deleted" | "alreadyMissing",
            supersession: {
              source: "supersededByRotation",
              revokedAt
            }
          },
      remainingSteps: []
    }

ActivationCleanupRecoveryStep =
  | {
      kind: "finalizeLegacyScrub",
      expectedStoreRevision: SecretStoreRevision,
      activeSecretRef,
      activeRecordRevision,
      activeBindingSetCas: { revision, digest, count },
      backendInstanceId,
      backendGeneration,
      deviceBindingGeneration,
      capabilityRevision,
      sourceExpectations: sorted non-empty LegacySourceExpectation[],
      readConfirmation: "never" | "optional" | "required",
      structureDigest: RecoveryStructureDigest
    }
  | {
      kind: "deleteOldRecord",
      expectedStoreRevision: SecretStoreRevision,
      oldSecretRef,
      oldRecordRevision,
      expectedOldBindingSetCas: { revision, digest, count: 0 },
      backendInstanceId,
      backendGeneration,
      deviceBindingGeneration,
      capabilityRevision,
      deleteConfirmation: "never" | "optional" | "required",
      authorizationSlot: "RecoveryConfirmationSlot::OldRecordDelete",
      requiredBindingState: "noBindings"
    }
  | ActivationVerifyOldRecordMissingStep

ActivationVerifyOldRecordMissingStep {
  kind: "verifyOldRecordMissing",
  expectedStoreRevision: SecretStoreRevision,
  oldSecretRef,
  oldRecordRevision,
  expectedOldBindingSetCas: { revision, digest, count: 0 },
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  readConfirmation: "never" | "optional" | "required",
  authorizationSlot: "RecoveryConfirmationSlot::OldRecordMissingReadback",
  requiresDeleteAppliedCas: true
}

这里以及`CaptureVerifyMissingStep`、`DeleteVerifyMissingStep`的`readConfirmation`都不是独立missing policy：checked factory必须从同一record的`SecretBackendOperation::Validate`/`operationConfirmation.validate`复制并在CAS preimage中固定它。三种step仍分别使用自己的`*MissingReadback` slot/scope、actual delete-applied CAS、one-shot authorization、platform call、receipt与checkpoint；相同Validate confirmation不能合并authority。

`verifyOldRecordMissing`是activation cleanup的最后一步，不存在第四个finalize-supersession step。独立missing authorization消费actual `BackendDeleteAppliedCas`；fresh `BackendMissingReadbackReceipt`只在同一device-authority durable transaction中按value消费，该事务持久化`supersededByRotation + revokedAt=BackendDeleteReceipt.completedAt + terminal`，而不编码receipt/`missingCheckedAt`。Crash只能看见事务前`oldRecordDeleteApplied + [verifyOldRecordMissing]`，或事务后`terminal + []`；不持久/公开standalone old-record missing checkpoint或空suffix nonterminal。

CaptureCompensationRecoveryRecord {
  kind: "captureCompensation",
  recoveryId,
  recoveryCas: { revision: SecretRecoveryRevision, digest: 64-lowercase-hex },
  candidateId,
  candidateRevision,
  secretRef,
  recordRevision,
  expectedStoreRevision,
  expectedBindingSetCas: { revision, digest, count: 0 },
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  state: CaptureCompensationRecoveryState,
  createdAt,
  updatedAt
}

CaptureCompensationRecoveryState =
  | {
      phase: "deleteIntent",
      remainingSteps: [
        CaptureDeleteUncommittedRecordStep,
        CaptureVerifyMissingStep,
        CaptureFinalizeCompensationStep
      ]
    }
  | {
      phase: "deleteApplied",
      deleteDisposition: "deleted" | "alreadyMissing",
      backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas,
      remainingSteps: [CaptureVerifyMissingStep, CaptureFinalizeCompensationStep]
    }
  | {
      phase: "missingReadbackVerified",
      deleteDisposition: "deleted" | "alreadyMissing",
      backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas,
      missingCheckedAt,
      remainingSteps: [CaptureFinalizeCompensationStep]
    }
  | {
      phase: "recoveryRequired",
      checkpoint:
        | { state: "none" }
        | {
            state: "deleteApplied",
            deleteDisposition: "deleted" | "alreadyMissing",
            backendCompletedAt,
            deleteAppliedCas: BackendDeleteAppliedCas
          }
        | {
            state: "missingReadbackVerified",
            deleteDisposition: "deleted" | "alreadyMissing",
            backendCompletedAt,
            deleteAppliedCas: BackendDeleteAppliedCas,
            missingCheckedAt
          },
      remainingSteps: sorted non-empty CaptureCompensationRecoveryStep[]
    }
  | {
      phase: "stateFinalized" | "terminal",
      terminalCandidateState: "discarded",
      remainingSteps: []
    }

CaptureCompensationRecoveryStep =
  | CaptureDeleteUncommittedRecordStep
  | CaptureVerifyMissingStep
  | CaptureFinalizeCompensationStep

CaptureDeleteUncommittedRecordStep {
  kind: "deleteUncommittedRecord",
  deleteConfirmation: "never" | "optional" | "required",
  authorizationSlot: "RecoveryConfirmationSlot::UncommittedRecordDelete"
}

CaptureVerifyMissingStep {
  kind: "verifyUncommittedRecordMissing",
  readConfirmation: "never" | "optional" | "required",
  authorizationSlot: "RecoveryConfirmationSlot::UncommittedRecordMissingReadback",
  requiresDeleteAppliedCas: true
}

CaptureFinalizeCompensationStep {
  kind: "finalizeCaptureCompensation",
  requiredBindingState: "noBindings",
  terminalCandidateState: "discarded",
  requiredRecordState: "absent"
}

DeleteFinalizationRecoveryRecord {
  kind: "deleteFinalization",
  recoveryId,
  recoveryCas: { revision: SecretRecoveryRevision, digest: 64-lowercase-hex },
  deleteAdmission: {
    admissionId: 32-lowercase-hex,
    readinessOperationId: SecretOperationId,
    admittedAt
  },
  secretRef,
  recordRevision,
  expectedStoreRevision,
  expectedBindingSetCas: { revision, digest, count },
  affectedOwners: sorted non-empty RecoveryAffectedOwner[],
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  revocationSource: "userDelete",
  state: DeleteFinalizationRecoveryState,
  createdAt,
  updatedAt
}

DeleteFinalizationRecoveryState =
  | {
      phase: "deleteIntent",
      remainingSteps: [
        DeleteAdmittedRecordStep,
        DeleteVerifyMissingStep,
        DeleteFinalizeStateStep
      ]
    }
  | {
      phase: "deleteApplied",
      deleteDisposition: "deleted" | "alreadyMissing",
      backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas,
      remainingSteps: [DeleteVerifyMissingStep, DeleteFinalizeStateStep]
    }
  | {
      phase: "missingReadbackVerified",
      deleteDisposition: "deleted" | "alreadyMissing",
      backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas,
      missingCheckedAt,
      remainingSteps: [DeleteFinalizeStateStep]
    }
  | {
      phase: "recoveryRequired",
      checkpoint:
        | { state: "none" }
        | {
            state: "deleteApplied",
            deleteDisposition: "deleted" | "alreadyMissing",
            backendCompletedAt,
            deleteAppliedCas: BackendDeleteAppliedCas
          }
        | {
            state: "missingReadbackVerified",
            deleteDisposition: "deleted" | "alreadyMissing",
            backendCompletedAt,
            deleteAppliedCas: BackendDeleteAppliedCas,
            missingCheckedAt
          },
      remainingSteps: sorted non-empty DeleteFinalizationRecoveryStep[]
    }
  | {
      phase: "stateFinalized" | "terminal",
      revokedAt,
      revocationSource: "userDelete",
      remainingSteps: []
    }

DeleteFinalizationRecoveryStep =
  | DeleteAdmittedRecordStep
  | DeleteVerifyMissingStep
  | DeleteFinalizeStateStep

DeleteAdmittedRecordStep {
  kind: "deleteAdmittedRecord",
  deleteConfirmation: "never" | "optional" | "required",
  authorizationSlot: "RecoveryConfirmationSlot::AdmittedRecordDelete"
}

DeleteVerifyMissingStep {
  kind: "verifyDeletedRecordMissing",
  readConfirmation: "never" | "optional" | "required",
  authorizationSlot: "RecoveryConfirmationSlot::AdmittedRecordMissingReadback",
  requiresDeleteAppliedCas: true
}

DeleteFinalizeStateStep {
  kind: "finalizeDeletedRecord",
  requiredBindingState: "retainedTombstones",
  revocationSource: "userDelete"
}

OwnerDetachBindingView =
  | {
      state: "bound",
      secretRef,
      bindingRevision,
      bindingSetCas: { revision, digest, count },
      remainingOwners: sorted SecretOwner[]
    }
  | { state: "unbound", remainingOwners: [] }

OwnerDetachFinalizationRecoveryRecord {
  kind: "ownerDetachFinalization",
  recoveryId,
  recoveryCas: { revision: SecretRecoveryRevision, digest: 64-lowercase-hex },
  providerDeleteImpactId,
  providerRowRevision,
  providerDetachTransactionId: 32-lowercase-hex,
  providerDetachCommitId: 32-lowercase-hex,
  detachedOwner,
  expectedOwnerBindingRevision,
  expectedStoreRevision,
  currentLegacyState: "none",
  bindingView: OwnerDetachBindingView,
  state: OwnerDetachFinalizationRecoveryState,
  createdAt,
  updatedAt
}

OwnerDetachFinalizationRecoveryState =
  | {
      phase: "providerDetachCommitted" | "localOwnerCasIntent" |
             "recoveryRequired",
      remainingSteps: [OwnerDetachFinalizeLocalStateStep]
    }
  | {
      phase: "localOwnerCasApplied" | "terminal",
      remainingSteps: []
    }

OwnerDetachFinalizeLocalStateStep {
  kind: "finalizeOwnerDetach",
  confirmation: "never",
  backendMutation: "forbidden"
}

OwnerBindingAuthorityRecord {
  owner: { kind: "provider", namespace: "codex", ownerId, slot: "primaryApiKey" },
  purpose: "codexApiKey",
  ownerBindingRevision: SecretOwnerBindingRevision,
  state: "unbound" | "bound",
  secretRef?: SecretRef,          // required only when bound
  bindingRevision?: SecretBindingRevision, // required only when bound
  createdAt,
  updatedAt
}

OwnerMigrationRecord {
  owner,
  status: "none" | "migrationRequired" | "conflict" | "inProgress" |
          "approvalRequired" | "cleanupRequired" | "complete",
  sources: sorted LegacySourceExpectation[],
  lastErrorCode?: SecretErrorCode,
  updatedAt
}

ManagedArtifactScanSummary {
  scanRevision: u64 >= 1,
  lastScanAt,
  enumeratedCategories: sorted HistoricalArtifactCategory[],
  scannedCount,
  cleanCount,
  valueFoundCount,
  reportOnlyCount,
  failedCount
}
```

`policyState` 与 `retirementState` 分列，禁止 logical unlock 把 `stale/revoked` 复活。合法 transition：

```text
active/live -> locked/live -> active/live
active|locked/live -> active|locked/stale -> active|locked/revoked
active|locked/live -> active|locked/revoked
stale|revoked -> live    forbidden
revoked -> any other     forbidden
```

`retirementState=revoked` iff `revokedAt + revocationSource`同时存在；非revoked时两者必须absent。Rotation old-record cleanup terminal使用truthful `supersededByRotation`，普通用户delete使用`userDelete`，其余两类只在backend/device确实报告时使用。

`presence` 不持久化；每次由 backend probe 生成。`confirmationRequired` 不进入 `state.json` 或稳定 summary，只能存在于 operation-scoped prepare response。

`pendingTerminalDisposition` 是 checked public/durable field，不是 UI 推断：它只允许出现在 `state=verifiedPendingPlan` 且同一candidate存在nonterminal `discardCandidate` journal、`issue={code:SECRET_OPERATION_RECOVERY_REQUIRED,action:discardCandidate}` 时；值由首次intent固定为`discarded|expired`，retry/startup不得改写。`activated|cleanupRequired|discarded|expired`以及没有该issue的pending candidate都必须absent。只有delete/already-missing、fresh missing readback、candidate/record state commit均durable后才能进入terminal candidate state并移除此字段。

`OwnerBindingAuthorityRecord` 对每个已知 provider/codex owner 始终存在，即使当前 unbound；这提供合同中的 `ownerBindingRevision` tombstone，避免 unbound→bound→unbound ABA。bind、unbind、rebind 每次递增 `ownerBindingRevision`。进入 bound 时 `bindingRevision` 取该 owner 的新单调 revision（不得重置为 1）；离开 bound 时 authority row保留，但 bound-only字段必须同时 absent。`OwnerMigrationRecord` 只覆盖该 owner 的 legacy状态，不替代 owner authority row。

`SecretRecoveryRecord` 是完整四臂strict tagged union，不再等同于activation `cleanupRequired`。`activationCleanup`是唯一可把candidate映射为`cleanupRequired`的arm；另外三臂保持各自真实状态。每个arm有独立phase与step algebra；nonterminal必须有该kind的nonempty remaining suffix，terminal必须为空。decoder先按`kind`选arm，再对每个nested object做`deny_unknown_fields`；未知kind、跨kind字段/step/phase、空required set、乱序/重复row或从ref/count推导缺失authority均拒绝。`stagedImport`只有自己的checkpoint/CAS，不是第五个recovery kind。

`recoveryCas.digest` 使用下列四个互斥exact UTF-8 preimage。`\n`是一字节LF，`\0`是一字节NUL；所有scalar禁止NUL/LF，revision/count为无sign/leading-zero的最短base-10 ASCII，timestamp为canonical RFC3339 UTC，digest/opaque receipt为固定lowercase hex。共同首行只有`fyagent.secret.recovery.v1\n`；随后必须且只能选择一个kind block：

`activationCleanup`：

```text
recovery\0activationCleanup\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
candidate\0<candidateId>\0<candidateRevision>\0<activeSecretRef>\0<activeRecordRevision>\n
owner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<ownerBindingRevision>\0<activeSecretRef>\0<bindingRevision>\n
... all affected owner rows in SecretOwner byte order ...
step\0finalizeLegacyScrub\0<activeSecretRef>\0<activeRecordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\0<readConfirmation>\0<structureDigest>\n
source\0<locationId>\0<category>\0<origin>\0<structuralRevision>\n
... all source rows in LegacySourceRef byte order ...
step\0deleteOldRecord\0<oldSecretRef>\0<oldRecordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\0<deleteConfirmation>\0noBindings\n
checkpoint\0<none|deleteApplied|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
step\0verifyOldRecordMissing\0<readConfirmation>\n
oldRecordTerminal\0notApplicable\n
# OR, when old-record deletion was planned and the missing receipt has been consumed:
oldRecordTerminal\0<deleted|alreadyMissing>\n
supersession\0supersededByRotation\0<backendCompletedAt>\n
```

这里只编码仍remaining的step，rank固定`finalizeLegacyScrub < deleteOldRecord < verifyOldRecordMissing`；old-record `bindingSetCount`必须为`0`。Crash-visible checkpoint只能为`none|deleteApplied|stateFinalized`：`deleteApplied`编码delete receipt、backend terminal time、actual `BackendDeleteAppliedCas`与唯一remaining `verifyOldRecordMissing`；missing authorization消费该CAS与fresh missing receipt后，同一device-authority transaction只持久化`supersededByRotation + revokedAt=backendCompletedAt + terminal`，绝不编码standalone missing checkpoint/receipt、`missingCheckedAt`或nonterminal empty suffix。Terminal移除intermediate delete receipt并只编码`oldRecordTerminal\0notApplicable`，或`oldRecordTerminal\0<deleted|alreadyMissing> + supersession\0supersededByRotation\0<backendCompletedAt>`。Delete receipt不能授权missing readback，missing hint不能授权supersession。

`captureCompensation`：

```text
recovery\0captureCompensation\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
candidate\0<candidateId>\0<candidateRevision>\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\n
bindingSet\0<bindingSetRevision>\0<bindingSetDigest>\00\n
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
checkpoint\0<none|deleteApplied|missingReadbackVerified|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
missingReceipt\0<missingCheckedAt>\n
finalized\0discarded\n
step\0deleteUncommittedRecord\0<deleteConfirmation>\n
step\0verifyUncommittedRecordMissing\0<readConfirmation>\n
step\0finalizeCaptureCompensation\0noBindings\0discarded\0absent\n
```

`deleteReceipt`只在`deleteApplied|missingReadbackVerified`或对应`recoveryRequired.checkpoint`时出现，并绑定同一durable checkpoint实际mint的`BackendDeleteAppliedCas`；`missingReceipt`只在`missingReadbackVerified`或对应recovery checkpoint时出现。进入`stateFinalized|terminal`后不再编码两张intermediate receipt，只编码`checkpoint\0stateFinalized + finalized\0discarded`。step只编码该phase仍remaining的exact suffix，rank固定delete→missing-readback→state-finalize。

`deleteFinalization`：

```text
recovery\0deleteFinalization\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
admission\0<admissionIdHex>\0<readinessOperationId>\0<admittedAt>\0userDelete\n
record\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
owner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<ownerBindingRevision>\0<secretRef>\0<bindingRevision>\n
... all affected owner rows in SecretOwner byte order ...
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
checkpoint\0<none|deleteApplied|missingReadbackVerified|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
missingReceipt\0<missingCheckedAt>\n
revocation\0userDelete\0<revokedAt>\n
step\0deleteAdmittedRecord\0<deleteConfirmation>\n
step\0verifyDeletedRecordMissing\0<readConfirmation>\n
step\0finalizeDeletedRecord\0retainedTombstones\0userDelete\n
```

Intermediate delete/missing receipt只在对应progress phase或`recoveryRequired.checkpoint`中编码；delete receipt同时绑定同一durable checkpoint实际mint的`BackendDeleteAppliedCas`。`stateFinalized|terminal`不再编码intermediate receipt，只编码`checkpoint\0stateFinalized + revocation\0userDelete\0<revokedAt>`。`revokedAt`只能来自validated backend delete/already-missing receipt的`completedAt`并由fresh missing-readback checkpoint放行，不能用startup wall clock或missing observation伪造。step rank固定delete→missing-readback→state-finalize。

`ownerDetachFinalization`：

```text
recovery\0ownerDetachFinalization\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
provider\0<providerDeleteImpactId>\0<providerRowRevision>\0<providerDetachTransactionIdHex>\0<providerDetachCommitIdHex>\n
detach\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<expectedOwnerBindingRevision>\0<expectedStoreRevision>\n
legacy\0none\n
```

随后exactly one binding continuation。Bound：

```text
binding\0bound\0<secretRef>\0<bindingRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
remainingOwner\0<kind>\0<namespace>\0<ownerId>\0<slot>\n
... all remaining owner rows in SecretOwner byte order ...
step\0finalizeOwnerDetach\0never\0backendMutationForbidden\n
```

Unbound：

```text
binding\0unbound\n
step\0finalizeOwnerDetach\0never\0backendMutationForbidden\n
```

两个binding arm互斥且都证明preview/current legacy discovery为`none`：`bound`要求full per-owner binding/binding-set CAS与exact remaining owners；`unbound`禁止secretRef/binding/CAS并要求空remaining owners。任何current legacy source在preview后、journal intent前或Provider transaction fresh-check时出现，都会使Provider impact stale并以`effect=none`终止；因此durable `detachProviderOwner` journal与`ownerDetachFinalization` recovery不存在legacy arm，loader遇到`binding=legacy`或legacy-only字段必须按unknown/cross-kind corruption fail closed。此kind没有backend/record/delete line，任何backend field或delete step同样拒绝。

每个实际preimage只拼接所选arm和该phase允许的receipt/remaining-step rows；不能以空字符串、placeholder或optional bag占位。`RecoveryAffectedOwner`是owner row唯一durable source；`RecoveryStructureDigest`只覆盖activation scrub的同一sorted source rows。`SecretBindingSetCas.digest`/structure digest按64-character lowercase hex进入preimage；material/value及其derived digest、raw path、backend locator均禁止。任一phase、checkpoint、step/field、owner/revision/backend/receipt变化都先递增`recoveryCas.revision`再重算digest。全部完成后先durable terminal、audit/readback，再retire；原八类operation journal在`recoveryRequired` phase携带同一`recoveryId+kind+recoveryCas` pointer，不创建generic recovery operation。

`LegacySourceRef` 与 `LegacySourceExpectation { source, structuralRevision }` 精确镜像 `secret-contract-v1.md`：`locationId` 由 `origin + category + internal structural locator` 生成，绝不含/哈希 source value；它区分 Provider row、live auth/config、SQL import、DB restore、sync download，以及 top/active/inactive/inline TOML位置。Candidate/journal/provider scrub必须携带排序后的逐 occurrence expectations；每个 locator自带 revision，不能降级为单一 aggregate revision或category count。

### 4.2.1 `audit/*.json`

每个文件使用 §5 envelope，payload精确镜像 `secret-contract-v1` 的 material-free `SecretAuditEvent`：`schemaVersion=1, eventId, occurredAt, operationId, scope, outcome, effect, owner?, secretRefDisplay?, backendKind?, backendInstanceId?, errorCode?`。`scope` 使用同一判别联合：`{kind:"general",action:SecretGeneralAuditAction}` 或 `{kind:"apply",action:SecretApplyAuditAction,role:"target"|"rollback"}`；flat `action + role?` 不得持久化。禁止 full ref（除 journal/state native identity）、raw path/message、material/hash。文件名必须与 payload `eventId` 相等；append-only create-new，重复 ID fail closed。

所有 contract-visible revision（record/candidate/binding/binding-set/backend/device-binding/capability）必须在 `1..=9_007_199_254_740_991`，不得用超过 JS-safe integer 的本地 u64 值回传。`storeRevision` 是 native-only，但同样采用该上限，overflow 前 fail closed/需要版本升级。

### 4.3 Binding CAS

每个 ref 有独立 `SecretBindingSetCas { revision, digest, count }`。每次 bind/unbind/任一 binding revision 变化都递增该 ref revision；public activate/rotate/lock/delete impact DTO 返回：

```text
expectedBindingSet: { revision, digest, count }
sortedAffectedOwners[]
sortedBindingRevisions[]
```

`expectedStoreRevision` 是 native-only journal/capability CAS，不进入 renderer wire。Service在public impact转为native intent时原子捕获当前 store revision；capability/activation重检同时验证它。Public authorization依赖 exact record/binding revisions与binding-set digest，不能把native store revision当作用户提供字段。

mutation 必须在同一 state commit 中验证 revision + exact sorted owner set + affected-row count。只比较 dependency count 不合法。`ownerId` 必须通过 `secret-contract-v1.md` 的 canonical grammar `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$`，并且只能由既有 native Provider authority解析为 `provider/codex` owner；renderer-supplied arbitrary string不得直接创建authority row。

digest 精确采用 `secret-contract-v1.md` 的非 material 输入：domain line `fyagent.secret.binding-set.v1\n`、`secretRef`，再按 byte-order 排序的 `kind\0namespace\0ownerId\0slot\0bindingRevision\n`。count 只用于显示；revision + digest + exact rows 才能授权 mutation。

### 4.4 Per-secret capability

capability 是 record snapshot，不是 backend-wide static bool：

```text
SecretRecordCapabilities {
  schemaVersion: 1,
  backendKind,
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  deviceBinding: "hostUser" | "hardwareDevice",
  storageResidency: "osProtectedStore" | "hardwareOnly",
  operationConfirmation: {
    captureVerify, validate, resolveForApply, delete,
    revoke
  },
  allowedConsumers: SecretRuntimeConsumer[],
  allowedSinks: SecretRuntimeSink[],
  persistentTargetProjection: bool,
  centralRevocation: bool,
  silentFallback: false
}
```

上表是storage projection，不是public-field Rust struct。Load/backend registration只能通过合同的private-field validated `SecretRecordCapabilities::try_new`/custom decoder，强制arrays排序去重、record/backend identity一致，并验证`changePlanApply↔externalConfigFile+persistentTargetProjection`与`proxyRequest/usageProbe/modelFetch↔processMemory`矩阵；reserved consumer/sink或任意不一致均拒绝整份state/backend result。

OS backend record由当前 `DeviceInstanceId` 的唯一 `BackendInstanceRecord` 产生。future hardware 每条 secret 保存 exact instance/locator/generation/capability revision；registry 只按 `backendInstanceId` exact lookup，缺失或 generation drift 返回 `SECRET_DEVICE_MISMATCH`/`SECRET_BACKEND_UNAVAILABLE`，永不尝试 OS keyring。

所有platform call（capability/read/write/probe/delete/revoke/prepare/confirm/cancel）都必须先执行`registry.get_exact(backendInstanceId, backendGeneration)`，再从同一registered instance构造不可伪造的`BackendRecordHandle`。该handle绑定`DeviceInstanceId + Arc<DeviceSecretStoreInstanceId> + RegisteredBackendHandleBinding + secretRef + expectedStoreRevision + recordRevision + bindingSetCas + backendInstanceId + backendGeneration + deviceBindingGeneration + capabilityRevision + private locator`。`RegisteredBackendHandleBinding`保留exact `Arc<RegisteredSecretBackend>`；每次调用以`Arc::ptr_eq`和durable/process identity、instance/generation双重核对，platform adapter只能借用private view，不能接受raw ref/locator或自行枚举/fallback。任一store-lifetime/Arc/instance/generation/record/store/binding/device/capability drift在platform call前失败，不能把调用结果再解释给另一个handle。

Platform leaf的raw result必须是带identity的封闭variant：至少同时返回`backendGeneration + deviceBindingGeneration`，prepare/confirm还返回其operation/requirement identity，read/write/delete/probe/revocation还返回本操作的typed payload。Registered wrapper在material、delete receipt、missing-readback receipt、revocation hint/receipt或capability离开wrapper前，重新执行exact Arc、store instance、record handle与returned generations核对；不匹配时payload在wrapper内drop/zeroize并返回`SECRET_DEPENDENCY_CHANGED, effect=none`。Platform result不能由caller补generation，也不能在出界后由service“相信并补验”。

`operationConfirmation`恰好对应`SecretBackendOperation::{CaptureVerify,Validate,ResolveForApply,Delete,Revoke}`五种operation；不存在第六种`MissingReadback` operation或独立`missingReadback` policy字段。每个`*MissingReadback` step/plan row中的`readConfirmation|missingReadbackConfirmation`都只能复制同一record当前`operationConfirmation.validate`，并在prepare与consume时fresh核对；这些字段是Validate policy的CAS snapshot，不是另一份可分叉policy。

`BackendAuthorizationHandle`与`BackendPendingConfirmation`都不可Serialize/Deserialize/Clone/Debug，并拥有完整record handle snapshot、operation id、closed slot、scope、expiry和single-use nonce；pending不是只有`stepId`的自由registry row。`confirm`原子consume pending，重新`get_exact`同一instance/generation/Arc/store instance并核对完整snapshot后才产生authorization；`cancel/expiry/replay`终止该nonce。authorization在read/write/delete/revoke/missing-readback前再次按同一handle消费，不能跨scope、slot、record、backend或recovery kind移植。Delete与其fresh missing readback始终是两个不同slot、两个不同authorization和两次platform call；missing slot执行的operation固定为`Validate`并复用`operationConfirmation.validate`，但slot、scope、authorization、`BackendDeleteAppliedCas`消费、receipt与durable checkpoint仍全部独立。

central/device revocation只能产生下列process-local consuming receipt；它不是可由caller或普通probe构造的observation object：

```text
BackendRevocationObservation { // non-clone, consuming scope receipt
  operationId,
  scope: "General::Revoke",
  deviceSecretStoreInstanceId: Arc<DeviceSecretStoreInstanceId>,
  registeredBackend,           // exact Arc binding, no public representation
  deviceInstanceId,
  secretRef,
  expectedStoreRevision,
  recordRevision,
  bindingSetCas,
  affectedOwners: sorted RecoveryAffectedOwner[],
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  observation: {
    source: "centralBackend" | "deviceAdministration",
    revokedAt
  }
}
```

receipt无Clone/Serde/Debug，字段private，只能由exact registered handle在operation broker消费显式`SecretNonApplyBackendOperation::Revoke` / `General::Revoke` authorization后调用`observe_revocation_once`返回。调用platform前必须验证record capability `centralRevocation=true`且registered backend发布`BackendRevocationObservationCapability::SourceAndTime`；platform返回的source/time在wrapper内仍只是`PlatformBackendRevocationHint`，wrapper复核exact Arc/store instance/returned generations与full CAS后才mint上述receipt。普通authorized read或probe最多返回不可持久化、不可Clone/Serde的`BackendRevocationHint`以阻断当前操作；它不能调用receipt factory。Receipt写state前在同一mutation critical section再次验证完整ref/store/record/binding/backend/device/capability CAS与该capability。`persist_backend_revocation_receipt(receipt)`按value消费一次；失败/取消不留下可重放receipt。OS keyring固定`centralRevocation=false`；missing/locked/denied/unavailable、caller-supplied ref/source/time或普通`BackendProbeResult`均不能mint、clone或移植central revocation authority。

### 4.5 Capture intent registry 与 private operation broker

`SecretCaptureIntentRegistry`是`BackendOperationBroker`独占的短期process-local registry，不进入`state.json`、journal、audit、cache或renderer持久层。`list_secret_backend_options`的request携带`owner + purpose + intent(newBinding|replaceBinding|legacyReconcile)`；native在同一authority snapshot内完成current legacy inventory/coverage与owner-binding join，然后只能经broker调用`SecretCaptureIntentRegistry::mint_from_atomic_snapshot`来mint：

```text
SecretCaptureIntent {                 // fields private; no Clone/Serde/Debug
  captureIntentId,
  deviceInstanceId,                    // durable dev_* namespace snapshot
  deviceSecretStoreInstanceId: Arc<DeviceSecretStoreInstanceId>,
  owner,
  purpose: "codexApiKey",
  intent: "newBinding" | "replaceBinding" | "legacyReconcile",
  currentOwnerBindingRevision,
  hiddenBoundExpectation:
    | { state: "unbound", ownerBindingRevision }
    | { state: "bound", ownerBindingRevision, secretRef,
        bindingRevision, bindingSetCas, recordRevision,
        backendInstanceId, backendGeneration,
        deviceBindingGeneration, capabilityRevision },
  legacySourceCoverageReceipt: LegacySourceCoverageReceipt,
  registeredBackendSetRevision,
  expiresAt,
  state: "available" | "claimed" | "terminal"
}
```

Public options result只返回`captureIntent{captureIntentId,owner,purpose,intent,currentBinding,legacySourceCoverage:LegacySourceCoverageView,expiresAt} + sorted registered backend views/capabilities`；view无值且不返回hidden bound expectation、full ref、`LegacySourceRef`、legacy revision、raw locator/path或store/backend authority。Registry row只绑定一个原子`LegacySourceCoverageReceipt`：其`inventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked`不可拆分、替换或由caller重组；其中current expectations才可授权后续read/compare/scrub，adjacent observations只能阻断并投影无值coverage。`begin_secret_capture` request只有`schemaVersion + captureIntentId + backendInstanceId`。`SecretCaptureIntentRegistry::claim_once`在打开dialog前原子`available -> claimed`，并将返回的backend id解析为exact registered backend instance/Arc；随后把receipt按value交回唯一bridge，fresh重读durable/process store identity、owner/purpose、current owner-binding revision、完整四字段receipt、hidden bound arm与registered backend set。任何proof/data脱绑定、omitted-domain/incomplete/stale/drift/expiry/replay都terminalize intent并返回合同指定fresh action，且backend/dialog/journal call count为0。Renderer不能提交owner/purpose/intent/`OwnerBindingExpectation`来覆盖registry truth。

`newBinding`只接受current unbound且没有要求替换的legacy/binding；`replaceBinding`要求hidden bound arm与replace impact完全匹配；`legacyReconcile`要求current `sourcesConflict|bindingConflict`的完整typed source set，并生成`explicitReplacement` candidate。三者在claim后才进入同一个native secure capture → durable intent → write/read/verify → `verifiedPendingPlan` flow。Legacy UI的`resolveLegacyConflict`/`captureReplacement`因此都先重新调用options query来mint intent，不是prose-only external destination；没有unreachable guidance，也没有renderer拼装binding authority。

唯一`Arc<SecretService>`的private field直接持有唯一`Arc<BackendOperationBroker>`；broker真实独占capture-intent、prepared-capability与pending-confirmation恰好三个registry，其他service/deps/module不得并列持有registry `Arc`或测试替身。Backend authorization由consuming scope/handle ownership承载，不创建第四个authorization registry。Private production assembly可在non-public `SecretServiceDeps`中搬运same broker Arc，但不存在caller/public/test setter、trait injection、registry参数或extractor；`AppStateBuilder`只选择closed fixture mode。Registry entry id/nonce均为private field；caller只持不可Clone/Serde/Debug的role-specific opaque token/bundle。Capability的atomic claim与role extraction只能通过broker-owned bundle完成，terminalize/discard按value消费；caller从不接收/回传private capability id，也不能先extract role再claim或重排claim/discard。所有`Backend*OperationContext`类型、fields和factories都private；唯一组合入口是`BackendOperationBroker::for_apply|for_runtime|for_activation|for_recovery|for_migration|for_staged_import|for_non_apply`，它们按value消费本种opaque admission/readiness/journal/runtime/staged claim并返回`BrokeredBackendOperationContext`。Exact registered wrapper随后只能调用`prepare_brokered_operation(record, brokeredContext)`；context不能互转或由scalar重建。

Device/native code不直接构造public error tuple。`SecretInternalError`的fields、raw constructor与platform detail均private；唯一literal constructor是impl-private `SecretInternalError::checked(code, SecretTerminalOperationContext, SecretErrorSources)`。Source-free分支只能通过closed `SecretSourceFreeErrorCode` + exact terminal context；需要source的四类分别只走typed `locked|revoked|backend_unavailable|operation_recovery_required` factory。一般`SECRET_OPERATION_RECOVERY_REQUIRED`必须携带typed recovery pointer；`candidate_terminal_cleanup_pending()`是唯一无pointer例外。合同的47 codes/24 actions完整match同时决定`retryable + action + effect + condition + discriminator`；未识别的platform/internal失败只能以`SecretSourceFreeErrorCode::Internal + exact terminal context`进入同一total table，不能自行指定无动作结果。Capture四actions都进入fresh `secretCaptureFlow`，四个runtime retry各只进入自己的exact `fixedRuntimeFlow`；无unrouted fallback action或unregistered legacy destination。

## 5. Hash、atomic replace 与 durability

### 5.1 Canonical payload hash

- hash domain：state 为 `b"fyagent-device-local-secret-state/v1\0"`；journal 为 `b"fyagent-secret-operation-journal/v1\0"`；audit 为 `b"fyagent-secret-audit-event/v1\0"`。
- payload 使用 typed Rust struct、fixed field order、sorted arrays/BTreeMap、compact JSON 编码；hash 为 `SHA-256(domain || canonical_payload_bytes)`。
- load 时先做 size/JSON/unknown-field/identifier 验证，再 canonical re-encode 并 constant-time 比较 64 lowercase hex hash。
- hash 只证明 accidental corruption/torn write detection，不宣称抵抗能重写文件与 hash 的本机攻击者；不引入循环依赖的 HMAC key。
- 任一 hash mismatch 均 fail closed；不从 `state.prev` 静默回滚，因为回滚可能复活已 lock/delete/rotate 的 credential。

### 5.2 Durable write primitive

现有 `config::atomic_write` 只有 `write_all + flush + replace`，没有 `sync_all`/parent directory durability，不能用于 secret state/journal。secret module 提供独立 `durable_replace`：

1. 在同目录用 closed temp grammar与随机 UUID、`create_new`/no-follow 创建 `0600` 文件：state 为 `.tmp-state-<uuid>.json`，journal update 为 `.tmp-journal-<OperationId>-<uuid>.json`；其他 temp 名称非法。
2. write all；`flush`；file `sync_all`（Windows 对应 `FlushFileBuffers`）；read back bytes并验证 envelope/hash。
3. Unix：`rename` 到目标，然后 open parent directory 并 `fsync`。
4. Windows：已有目标用 `ReplaceFileW(..., REPLACEFILE_WRITE_THROUGH, ...)`；首次创建用 `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`；再次 open/read back目标并验证 hash。
5. 任一步失败删除 temp（best effort）并保留旧目标。目标 readback不一致返回 `SECRET_OPERATION_RECOVERY_REQUIRED`，不新增 contract外 error code。

首次创建 journal/audit 使用 `create_new + sync_all + parent sync`，不允许覆盖同 ID。phase update 与 state update使用 `durable_replace`。删除 terminal journal 后再次 sync parent directory；Windows 使用 write-through rename 到 `journal/.retired-<OperationId>.json` 后 delete，确保重启不会把半删除误认成 active operation。

Startup在持有lifetime lock后先枚举这些残留。合法 temp必须匹配closed filename grammar、是同目录regular/no-follow/0600文件且envelope/hash有效；若canonical target存在且有效，temp只能删除并同步parent，后续由canonical journal重放；canonical target缺失/invalid或temp identity不匹配时fail closed，不提升temp为authority。Windows retired tombstone只有在filename中的`OperationId`等于其有效journal envelope、phase=`terminal`、同operation terminal audit已存在且hash/readback有效时才幂等delete并同步parent；其他`.retired-*`、缺audit、非terminal或内容不匹配一律`SECRET_OPERATION_RECOVERY_REQUIRED`。Scanner在处置前后均把这些recognised variants纳入device-local artifact enumeration。

### 5.3 Lock/order

local-only lifecycle/candidate mutation 的顺序固定为：

```text
StoreLifetimeFileLock (runtime lifetime)
  -> CaptureCoordinator reservation (不跨 OS dialog 持 Mutex guard)
  -> SecretMutationGate (单一 async semaphore / blocking mutex)
  -> durable journal/state file I/O
  -> exact backend-instance mutex
```

activation/import 是外部 authority-first 的独立路径：

```text
#41 Provider lease 或 import-cutover authority（coordinator已持有）
  -> final admitted baseline receipt
  -> SecretMutationGate
  -> durable journal/state I/O + exact backend-instance mutex
  -> 使用传入的 lease-bound transaction port；gate内禁止获取Provider/Database lock
```

- 不在持有 Database mutex 时调用 OS store、显示 dialog、await 或反向调用 SecretService。
- `SecretMutationGate` 可跨 synchronous backend I/O，但 backend I/O 整体运行在 `spawn_blocking`；不占 Tauri async worker。
- OS dialog 前不持有 mutation/file/backend/DB lock；dialog 返回后按 expected revisions 重新验证。
- backend read/write/delete 与同一 ref 的 journal transition 在一个 blocking operation 中序列化。

#41 coordinator 是唯一外层 lease authority。Candidate activation 与 live apply 是两个 plan/lease：`activation admission → candidate-read + old-delete + old-missing-readback各自prepare/confirm → activation lease/final baseline → SecretMutationGate → pre-mutation source/backend compare → binding CAS/exact scrub → delete receipt durable checkpoint → consume independent missing-readback slot → atomic supersession/terminal → release`；绑定成立后才是 `apply readiness/new plan → apply prepare/confirm → new lease/final baseline/backup → SecretMutationGate → exact backend read → owner-private consuming writer/readback → release`。Recovery按kind分流：`activationCleanup`才走active-read/old-delete/old-missing-readback三个独立slot→#41 cleanup lease→CAS→scrub/delete/checkpoint/readback/atomic terminal；`captureCompensation|deleteFinalization`分别由local broker预备delete slot与reservation-bound missing slot，先consume delete并durable写receipt/actual CAS，之后才能consume missing authorization，不能组合调用；`ownerDetachFinalization`只消费main-integration already-held detach context→local CAS且无backend call。所有prepare/confirm都不持secret mutation/DB/Provider lock；#35代码绝不反向申请Provider lease或在lease内弹窗。

## 6. Operation journal exact schema

每个journal仍使用§5 hash envelope，但共享envelope只能含非authority字段：

```text
OperationJournalEnvelope {
  schemaVersion: 1,
  operationId,
  deviceInstanceId,
  createdAt,
  updatedAt,
  payload: OperationJournalPayload
}

OperationJournalPayload =
  | CaptureCandidateJournal
  | MigrateLegacyJournal
  | RotateCandidateJournal
  | ActivateCandidateJournal
  | DiscardCandidateJournal
  | DeleteSecretJournal
  | DetachProviderOwnerJournal
  | StagedImportJournal
```

operation kind**恰好八类**：`captureCandidate | migrateLegacy | rotateCandidate | activateCandidate | discardCandidate | deleteSecret | detachProviderOwner | stagedImport`。不存在`recovery`、`cleanup`或其他第九generic operation；四类recovery row由原operation的typed `recoveryRequired` phase指向，retry更新原journal和state recovery row。

下列checked helper是封闭required record，不是optional property bag：

```text
JournalBackendIdentity {
  deviceInstanceId,
  secretRef,
  recordRevision,
  bindingSetCas,
  backendInstanceId,
  backendGeneration,
  deviceBindingGeneration,
  capabilityRevision,
  confirmation: "never" | "optional" | "required"
}

JournalCandidateIdentity {
  candidateId,
  candidateRevision,
  candidateKind,
  comparisonPolicy,
  comparisonImpact
}

JournalPlanAdmissionIdentity<Operation> {
  operation: Operation,
  admissionId: 32-lowercase-hex,
  planId,
  planDigest,
  projectionDigest
}
```

八个variant逐一声明完整authority与独立phase algebra：

```text
CaptureCandidateJournal {
  operationKind: "captureCandidate",
  attempt: u32 >= 1,
  expectedStoreRevision,
  ownerExpectation,
  targetOwners: sorted non-empty SecretOwner[],
  expectedBindings: sorted non-empty OwnerBindingExpectation[],
  candidate: JournalCandidateIdentity,
  sourceAuthority:
    | { kind: "none" }
    | { kind: "currentExplicitReplacement",
        sourceExpectations: sorted non-empty LegacySourceExpectation[] },
  backend: JournalBackendIdentity,
  phase: CaptureCandidateJournalPhase
}

CaptureCandidateJournalPhase =
  | { state: "intent" }
  | { state: "backendApplied", verifyReceiptId: 32-lowercase-hex }
  | { state: "stateFinalized" }
  | { state: "compensationIntent" }
  | { state: "recoveryRequired", lastErrorCode,
      recovery: { kind: "captureCompensation", recoveryId, recoveryCas } }
  | { state: "terminal", outcome: "candidateStaged" | "compensated" }

MigrateLegacyJournal {
  operationKind: "migrateLegacy",
  attempt: u32 >= 1,
  expectedStoreRevision,
  migrationReportId,
  ownerExpectation,
  targetOwners: sorted non-empty SecretOwner[],
  expectedBindings: sorted non-empty OwnerBindingExpectation[],
  candidate: JournalCandidateIdentity,
  comparisonPolicy: "candidateEquality",
  sourceExpectations: sorted non-empty LegacySourceExpectation[],
  backend: JournalBackendIdentity,
  phase: MigrateLegacyJournalPhase
}

MigrateLegacyJournalPhase =
  | { state: "intent" }
  | { state: "backendApplied", verifyReceiptId: 32-lowercase-hex }
  | { state: "stateFinalized" }
  | { state: "compensationIntent" }
  | { state: "recoveryRequired", lastErrorCode,
      recovery: { kind: "captureCompensation", recoveryId, recoveryCas } }
  | { state: "terminal", outcome: "candidateStaged" | "compensated" }

RotateCandidateJournal {
  operationKind: "rotateCandidate",
  attempt: u32 >= 1,
  expectedStoreRevision,
  oldRecord: JournalBackendIdentity,
  expectedOldBindingSet,
  affectedOwners: sorted non-empty RecoveryAffectedOwner[],
  candidate: JournalCandidateIdentity,
  comparisonPolicy: "explicitReplacement",
  newRecord: JournalBackendIdentity,
  phase: RotateCandidateJournalPhase
}

RotateCandidateJournalPhase =
  | { state: "intent" }
  | { state: "backendApplied", verifyReceiptId: 32-lowercase-hex }
  | { state: "stateFinalized" }
  | { state: "compensationIntent" }
  | { state: "recoveryRequired", lastErrorCode,
      recovery: { kind: "captureCompensation", recoveryId, recoveryCas } }
  | { state: "terminal", outcome: "candidateStaged" | "compensated" }

ActivateCandidateJournal {
  operationKind: "activateCandidate",
  attempt: u32 >= 1,
  expectedStoreRevision,
  admission: JournalPlanAdmissionIdentity<"secretCandidateActivation">,
  candidate: JournalCandidateIdentity,
  activeRecord: JournalBackendIdentity,
  affectedOwners: sorted non-empty RecoveryAffectedOwner[],
  targetOwners: sorted non-empty SecretOwner[],
  expectedBindings: sorted non-empty OwnerBindingExpectation[],
  sourceExpectations: sorted LegacySourceExpectation[],
  oldRecordDelete:
    | { kind: "notApplicable" }
    | { kind: "deleteAfterActivation", oldRecord: JournalBackendIdentity,
        deleteSlot: "ActivationConfirmationSlot::OldRecordDelete",
        missingReadbackSlot: "ActivationConfirmationSlot::OldRecordMissingReadback",
        deleteConfirmation: "never" | "optional" | "required",
        missingReadbackConfirmation: "never" | "optional" | "required",
        requiredBindingState: "noBindings" },
  phase: ActivateCandidateJournalPhase
}

ActivateCandidateJournalPhase =
  | { state: "intent" }
  | { state: "stateFinalized" }
  | { state: "providerFinalized" }
  | { state: "oldRecordDeleteIntent" }
  | { state: "oldRecordDeleteApplied",
      deleteDisposition: "deleted" | "alreadyMissing", backendCompletedAt,
      deleteAppliedCas: BackendDeleteAppliedCas }
  | { state: "recoveryRequired", lastErrorCode,
      recovery: { kind: "activationCleanup", recoveryId, recoveryCas } }
  | { state: "terminal", outcome: "activated" }

Normal activation也不存在standalone old-record missing checkpoint。`ActivationConfirmationSlot::OldRecordMissingReadback`消费`oldRecordDeleteApplied.deleteAppliedCas`与fresh missing receipt后，同一个device-authority durable transaction只持久化`supersededByRotation`、`revokedAt=BackendDeleteReceipt.completedAt`与`terminal`；不编码receipt/`missingCheckedAt`，后者也不是revokedAt来源。

DiscardCandidateJournal {
  operationKind: "discardCandidate",
  attempt: u32 >= 1,
  expectedStoreRevision,
  terminalDisposition: "discarded" | "expired",
  candidate: JournalCandidateIdentity,
  targetOwners: sorted non-empty SecretOwner[],
  expectedBindings: sorted non-empty OwnerBindingExpectation[],
  record: JournalBackendIdentity,
  phase: DiscardCandidateJournalPhase
}

DiscardCandidateJournalPhase =
  | { state: "intent" }
  | { state: "backendApplied",
      deleteDisposition: "deleted" | "alreadyMissing", backendCompletedAt }
  | { state: "missingReadbackVerified", missingCheckedAt }
  | { state: "stateFinalized", terminalDisposition: "discarded" | "expired" }
  | { state: "recoveryRequired", lastErrorCode,
      checkpoint: "intent" | "backendApplied" | "missingReadbackVerified" }
  | { state: "terminal", terminalDisposition: "discarded" | "expired" }

DeleteSecretJournal {
  operationKind: "deleteSecret",
  attempt: u32 >= 1,
  expectedStoreRevision,
  deleteAdmission: {
    admissionId: 32-lowercase-hex,
    readinessOperationId,
    admittedAt
  },
  record: JournalBackendIdentity,
  affectedOwners: sorted non-empty RecoveryAffectedOwner[],
  expectedOwnerBindingRevisions: sorted non-empty SecretOwnerBindingRevision[],
  revocationSource: "userDelete",
  phase: DeleteSecretJournalPhase
}

DeleteSecretJournalPhase =
  | { state: "intent" }
  | { state: "backendApplied",
      deleteDisposition: "deleted" | "alreadyMissing", backendCompletedAt }
  | { state: "missingReadbackVerified", missingCheckedAt }
  | { state: "stateFinalized", revokedAt, revocationSource: "userDelete" }
  | { state: "recoveryRequired", lastErrorCode,
      recovery: { kind: "deleteFinalization", recoveryId, recoveryCas } }
  | { state: "terminal", revokedAt, revocationSource: "userDelete" }

DetachProviderOwnerJournal {
  operationKind: "detachProviderOwner",
  attempt: u32 >= 1,
  expectedStoreRevision,
  providerDeleteImpactId,
  providerRowRevision,
  providerDetachTransactionId: 32-lowercase-hex,
  detachedOwner,
  expectedOwnerBindingRevision,
  currentLegacyState: "none",
  bindingView: OwnerDetachBindingView,
  phase: DetachProviderOwnerJournalPhase
}

DetachProviderOwnerJournalPhase =
  | { state: "intent" }
  | { state: "providerDetachCommitted", providerDetachCommitId: 32-lowercase-hex }
  | { state: "localOwnerCasApplied", providerDetachCommitId: 32-lowercase-hex }
  | { state: "recoveryRequired", lastErrorCode,
      providerDetachCommitId: 32-lowercase-hex,
      recovery: { kind: "ownerDetachFinalization", recoveryId, recoveryCas } }
  | { state: "terminal", providerDetachCommitId: 32-lowercase-hex }

StagedImportResumeCas {
  revision: StagedImportResumeRevision,
  digest: StagedImportResumeDigest
}

StagedImportJournal {
  operationKind: "stagedImport",
  attempt: u32 >= 1,
  expectedStoreRevision,
  stageAuthority: {
    stageId,
    stageKind: "sqlImport" | "binaryRestore" | "syncDownload",
    tempDatabaseDurableObjectId: 32-lowercase-hex,
    processNonce: 32-lowercase-hex,
    owner,
    stagedRowRevision,
    stagedSourceSetCas
  },
  admission: JournalPlanAdmissionIdentity<"stagedSecretImportActivation">,
  candidate: JournalCandidateIdentity,
  sourceExpectations: sorted non-empty LegacySourceExpectation[],
  record: JournalBackendIdentity,
  expectedLiveBinding: OwnerBindingExpectation,
  resumeCas: StagedImportResumeCas,
  phase: StagedImportJournalPhase
}

StagedImportJournalPhase =
  | { state: "intent" }
  | { state: "sourcesScrubbed", stagedSourceSetCasAfterScrub }
  | { state: "cutoverCommitted", cutoverReceiptId }
  | { state: "liveOwnerMinted", cutoverReceiptId,
      promotedLiveOwner: { owner, ownerBindingRevision, providerRowRevision } }
  | { state: "localBindingFinalized", cutoverReceiptId,
      promotedLiveOwner: { owner, ownerBindingRevision, providerRowRevision } }
  | { state: "recoveryRequired", lastErrorCode,
      checkpoint:
        | { state: "sourcesScrubbed", stagedSourceSetCasAfterScrub }
        | { state: "cutoverCommitted", cutoverReceiptId }
        | { state: "liveOwnerMinted", cutoverReceiptId, promotedLiveOwner } }
  | { state: "terminal", cutoverReceiptId, promotedLiveOwner }
```

每个variant与每个phase object都`deny_unknown_fields`并通过private checked factory；上表未列字段必须absent，不能用`Option`/flatten/default把phase receipt、plan、backend、owner或recovery authority藏成generic bag。共同名字不代表可跨variant复用：例如只有activation/staged import有plan admission，只有delete有user-delete admission，只有detach有Provider impact/transaction，缺失的authority不能用null/空串占位。`OperationJournalEnvelope.deviceInstanceId`与每个`JournalBackendIdentity.deviceInstanceId`必须等于opened store加载的durable `DeviceInstanceId`；journal/backend identity绝不编码`DeviceSecretStoreInstanceId`，process binding只在live `BackendRecordHandle/BackendAuthorizationScope`中以Arc保留。Activation old-delete、user-delete和其各自fresh missing readback的slot literal必须与prepared bundle、journal phase、recovery step/CAS逐字一致；任何组合`deleteAndReadback` phase/API均非法。禁止material、secret/value/token/password、raw error、source path、backend locator、secret-bearing Provider JSON/TOML、material/value-derived hash及DB/file content hash。

Crash-to-recovery映射是封闭表：

| Journal kind | 可创建/引用的nonterminal recovery |
| --- | --- |
| `captureCandidate` | `captureCompensation` |
| `migrateLegacy` | `captureCompensation` |
| `rotateCandidate` | `captureCompensation` |
| `activateCandidate` | `activationCleanup` |
| `deleteSecret` | `deleteFinalization` |
| `detachProviderOwner` | `ownerDetachFinalization` |
| `discardCandidate` | 不创建recovery row；留在同一journal并公开`pendingTerminalDisposition` |
| `stagedImport` | 不创建recovery row；使用typed staged checkpoint + `resumeCas` |

进入`recoveryRequired`时，原journal的typed pointer与`state.json`四臂row必须逐字段匹配`recoveryId+kind+recoveryCas`；其他phase不存在pointer。未知kind/CAS drift为`SECRET_RECOVERY_CHANGED/effect=none`，不得从generic ref/count重建。`activateCandidate`没有#55 single-consume admission不得写intent；`stagedImport`只能消费独立staged admission与ImportCutover authority，不能把`StagedSecretOwnerToken`用于ordinary readiness/runtime。

### 6.1 General operation recovery

现有`get_secret_cleanup_impact/retry_secret_cleanup`是唯一公开recovery入口，command count不增加；其DTO按`recovery.kind`判别：

- `activationCleanup`：只解码`finalizeLegacyScrub|deleteOldRecord|verifyOldRecordMissing`；携带exact candidate/active ref/affected owners与逐step完整backend identity。`RecoveryConfirmationSlot::{ActiveRecordRead,OldRecordDelete,OldRecordMissingReadback}`是三个独立slot；全部hardware confirmation在#41 Provider lease前完成。Old delete写durable receipt + `BackendDeleteAppliedCas`；missing authorization消费与该actual CAS匹配的reservation及fresh missing receipt，同一durable transaction只持久化`supersededByRotation`、`revokedAt=BackendDeleteReceipt.completedAt`与terminal，无第四step/空suffix phase或terminal receipt/time字段。
- `captureCompensation`：只解码`deleteUncommittedRecord → verifyUncommittedRecordMissing → finalizeCaptureCompensation`；exact uncommitted candidate/record/backend identity。Broker预备`RecoveryConfirmationSlot::UncommittedRecordDelete`与reservation-bound `UncommittedRecordMissingReadback`；先consume delete，durable `deleteApplied{deleteAppliedCas}`后actual CAS才能满足reservation并consume missing authorization。两者不能共享authorization/context；fresh missing另写`missingReadbackVerified`，随后才retire未提交record/candidate intent。
- `deleteFinalization`：只解码`deleteAdmittedRecord → verifyDeletedRecordMissing → finalizeDeletedRecord`；exact durable user-delete admission、record/owners/backend与`userDelete` provenance。`RecoveryConfirmationSlot::AdmittedRecordDelete`与reservation-bound `AdmittedRecordMissingReadback`是两个独立prepared slot；delete receipt + actual CAS checkpoint先durable，missing authorization再consume该CAS。Confirmed missing后写truthful revoked tombstones；没有该admission的missing仍是accidental missing。
- `ownerDetachFinalization`：只解码`finalizeOwnerDetach`；Provider detach impact/transaction/commit及`currentLegacyState=none`已durable，只接受main-integration mint的already-held detach context。Bound arm完成owner tombstone与ref binding-set CAS，unbound arm只完成owner tombstone；不存在legacy arm，也没有backend handle/step，backend secret始终保留。

每个kind的impact/result都是独立outer-tagged arm，只返回该arm的material-free identity、remaining steps、CAS和唯一action；retry先single-use claim exact`kind+recoveryId+recoveryCas`。Startup与显式retry复用同一strict decoder/step executor：可幂等自动推进`confirmation=never`的local step，但UI入口始终存在；需要hardware确认、#41 Provider lease或main-integration detach context时startup只保留row，绝不后台弹窗/伪造authority。Staged import只走`resume_staged_import_cutover({ stageId, expectedResumeCas })`，不得进入这两个cleanup command。

四kind的module/integration/UAT manifest必须分别记录`kind + startingCas + completed/remaining steps + terminal/readback result + evidence_origin`；一条activation happy path或“recovery count=4”不能替代capture delete/readback、user-delete provenance、owner-detach no-backend-mutation等独立case。需要hardware的case按real/injected来源标注，静态设计不把它们写成已通过。

`terminal` 不是直接删除：先 durable terminal + create-new audit event；两者 readback 通过后才 retire journal。若 audit write失败，保留 terminal journal，startup 重试 audit，不重复 backend/state mutation。

## 7. Write-ahead sequences 与 crash reconciliation

### 7.1 Capture/create candidate

正常序列：

1. `list_secret_backend_options(owner,purpose,intent)`先完成同一snapshot的owner-binding/current-legacy inventory并mint短期单次`SecretCaptureIntentId`。`begin_secret_capture(captureIntentId,backendInstanceId)`由broker原子claim；fresh revalidate hidden bound/source expectations和exact registered Arc后才预留唯一dialog slot。若 record-specific `captureVerify` 需要 hardware confirmation，先完成operation-scoped confirm，再显示 native secure input。cancel/empty/invalid terminalize该intent，不创建backend-mutation journal。
2. dialog 返回 `SecretMaterial` 后生成 candidate id/ref，再次验证intent绑定的owner/purpose/kind、owner-binding revision、完整legacy occurrence set、record/binding-set/backend/device/capability revisions；renderer没有可重写这些expectation的字段。
3. **durable `intent`**（必须早于 backend write）。
4. exact backend write；read back；constant-time equality；read buffer zeroize。
5. durable `backendApplied`。
6. state commit：insert unbound record + `verifiedPendingPlan` candidate，binding/Provider/live target 均不变；递增 store/candidate revisions。
7. durable `stateFinalized`；append audit；durable `terminal`；retire journal。

重启：

| Last durable phase | Reconcile |
| --- | --- |
| `intent` | probe exact new ref；missing→retire failed/cancelled；present→视为“写入结果未知且未持久化验证”，durable `compensationIntent` 后按delete→missing-readback→state-finalize推进；unknown→建立含exact candidate/record/backend identity的`captureCompensation` recovery row并阻断该 owner |
| `backendApplied` | 若 `OwnerBindingExpectation` 仍满足，则完成 candidate state commit；若 expectation 已冲突，则删除新 entry；delete unknown→同一`captureCompensation` recovery |
| `stateFinalized` | 不删除 verified candidate entry；补 audit/terminal，等待 #55 plan；绝不自动 bind |
| `compensationIntent` | 按四臂row的typed steps幂等重试；只有delete/already-missing + fresh missing readback + state finalize后才 terminal |

`captureCompensation`的三步不是一个backend helper：broker从current recovery CAS预备`RecoveryConfirmationSlot::UncommittedRecordDelete`与持有`BackendDeleteAppliedCasReservation`的`UncommittedRecordMissingReadback`，并可在任何mutation前完成各自hardware confirmation。Wrapper consume delete并复核returned generations后，durable写`deleteApplied` receipt并mint actual `BackendDeleteAppliedCas`；missing authorization只有消费与该actual CAS同operation/revision匹配的reservation后才能执行fresh authorized `Validate` missing readback，再独立durable `missingReadbackVerified` receipt。Crash在两者之间只继续第二slot，绝不重放“delete+readback”组合或从delete disposition推断missing。

#### 7.1.1 Candidate expiry/discard cleanup

`expiresAt`只禁止继续activation；它本身不允许把durable state直接翻成`expired`，也不允许遗留不可达backend entry。Explicit discard与expiry sweep都创建/恢复同一个`operationKind=discardCandidate`，并在首次intent把immutable `terminalDisposition`分别固定为`discarded`或`expired`。完整顺序是：fresh-validate candidate/record/backend/device/capability → durable `intent` → broker consume exact candidate-delete authorization/confirmation → delete/already-missing → durable `backendApplied`（含disposition/time）→ 从该checkpoint mint/consume独立candidate-missing-readback authorization → fresh authorized `Validate` confirms missing → durable `missingReadbackVerified` → atomically remove unbound record并写exact terminal candidate state → `stateFinalized`/audit/terminal。retry/startup只能装载原target，不能把expired改写为discarded或反向改写；cancel/discard authority由broker按value消费且只有这一条terminalization path。

任一confirmation cancel/expiry、locked/denied/unavailable、delete outcome不确定、missing readback失败、crash或state commit失败，都保持candidate=`verifiedPendingPlan`，并公开且仅公开：`pendingTerminalDisposition=<journal target>`、`issue={code:SECRET_OPERATION_RECOVERY_REQUIRED,retryable:true,action:discardCandidate}`。该形态禁止general recovery pointer，但仍由candidate id/revision到达同一journal；它既不是`cleanupRequired`，也不创建第五recovery kind。只有journal terminal后才公开`discarded|expired`并同时移除`pendingTerminalDisposition`与issue。Terminal `expired`的唯一action是`refreshSummary`：先重读current owner/card truth，随后由新的options query mint全新`SecretCaptureIntentId`或由rotation impact mint全新rotation authority；它绝不返回`retryCapture`，也不复用旧candidate、operation、backend authorization或admission。`SECRET_CANDIDATE_EXPIRED`可以解释activation为何被拒绝，但不能代替上述pending cleanup issue或证明backend已删除。

OS-keyring的`delete`与`validate` confirmation均为`never`时startup/list sweep可自动推进；future hardware任一对应policy需要物理确认时，不得后台弹窗或静默回退，只能由显式`discard_secret_candidate` prepare/confirm继续原journal。因此每个terminal candidate都对应exact backend delete/already-missing、fresh authorized Validate missing readback和durable state finalization，任一未清理entry始终可由candidate/journal到达。

### 7.2 Legacy migrate

Discovery 在任何 public projection 前完成。规则：

Discovery输入必须来自`research/codex-secret-call-graph.md`/surface inventory的完整mechanically registered source set，而不是本文件自建较窄allowlist。唯一mint/revalidation入口冻结为main-integration-owned `CodexLegacySourceInventoryBridge`。`LegacySourceCoverageReceipt`及唯一checked factory `LegacySourceCoverageReceipt::checked_from_complete_inventory_authority`均为`pub(crate)`；factory只接受该bridge才能构造的`CompleteLegacySourceInventoryAuthority`并按value消费。其他sibling可命名、移动并按value消费receipt，却看不到data fields/struct literal、不能构造authority，因此不能自行mint、clone、project或从count重建：

```text
pub(crate) struct LegacySourceCoverageReceipt { // no Clone/Serde/Debug
  inventoryRevision: LegacySourceInventoryRevision,
  coverageIdentity: CompleteLegacySourceCoverageIdentity,
  currentScrubbable: CurrentLegacySourceExpectations,
  adjacentBlocked: Vec<AdjacentBlockedLegacySourceObservation>,
  _seal // all data fields and the struct literal are private
}

impl LegacySourceCoverageReceipt {
  pub(crate) fn checked_from_complete_inventory_authority(
    authority: CompleteLegacySourceInventoryAuthority
  ) -> Result<LegacySourceCoverageReceipt, SecretInternalError>;
}

CompleteLegacySourceDomainProof {       // bridge-private
  domain,
  structuralRevision: non-value-derived positive revision,
  presence: "absent" | "present",
  count: 0 iff absent; positive iff present
}
```

`CompleteLegacySourceCoverageIdentity`必须恰好按固定顺序包含11个domain proof、无missing/duplicate/unknown：`currentProviderLive`加`processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile|commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge`。每一域即使`absent/count=0`也必须有自己的structural revision proof；`currentProviderLive` proof的presence/count必须与`currentScrubbable` exact expectations一致，十个supplemental proof也必须逐域与`adjacentBlocked` observations一致。`LegacySourceInventoryRevision`在任一域revision/presence/count、current expectation、adjacent observation或mechanical registration revision变化时递增且不得从value派生。Receipt允许`currentScrubbable`内部使用non-value-derived `LegacySourceLocationId`来保持exact occurrence identity，但绝不含raw path、raw locator、value、value-derived digest、env value或DB/localStorage value；`adjacentBlocked`只有domain/category/state observation，不可转换成`LegacySourceRef`或scrub authority。

`CodexLegacySourceInventoryBridge::fresh_revalidate_*`按consumer区分startup、owner-summary/readiness、capture-options/claim与Provider-delete preview/confirm；每次按value消费receipt，重新取得complete authority并逐字段比较`inventoryRevision + 11 domain proofs + currentScrubbable + adjacentBlocked`，再返回同一原子shape的fresh receipt。只有exact match后，consumer才能从该receipt消费current Provider/live expectations或投影无值`LegacySourceCoverageView`；任何代码都不得把identity proof与两组数据拆成独立authority，supplemental observations也绝不转换成`LegacySourceRef`或scrub authority。Public Provider feature/API/fixture chain、request override/raw transport、stream/proxy diagnostics与Codex MCP env/header仍必须进入完整surface inventory/coverage判断，但shared surface/main-integration owner负责其注册与Level-3 debt；本文件不声称已实现或覆盖。

缺receipt、stale revision、domain omission/duplicate/unknown、任一domain proof drift、proof与current/adjacent数据不匹配或mechanical coverage gap都在effect前`Blocked/effect=none`。即使所有source count都为0，若没有`inventoryRevision + exact 11 absent-domain proofs + empty currentScrubbable + empty adjacentBlocked`这一原子receipt仍必须Blocked；空数组、aggregate count、public view或先前receipt不能证明complete inventory。

- 两个/多个 inline source 的非空值不同：`conflict`，不选择、不清除。
- 已有 binding + inline：只有 backend read 成功并与每个 inline value constant-time equal 才可 scrub-only。
- existing binding read 为 locked/denied/unavailable/unknown：保持内部 plaintext，public projection继续scrub；public owner为 `credentialState=legacy, legacyState=bindingComparisonPending`，并带相应 stable issue；不得把 probe-present当作equality。
- existing binding value 不同：`conflict`，用户必须走无值typed replace/reconcile：fresh options query以`intent=legacyReconcile`绑定current owner-binding revision、hidden bound arm和完整source expectations，renderer只回传intent id与exact backend selection；普通fallback action或prose-only external destination不会选值/构造authority。
- 自动从一个唯一legacy值stage的candidate与`legacyScrubExistingBinding`固定`comparisonPolicy=candidateEquality`。从source/binding conflict显式capture的replacement与rotate固定`explicitReplacement`；plan展示exact source set/revisions并授权替换，不要求旧value等于新candidate。

当前 DB migration 的 staging 序列：

1. discovery/material 只在 native memory；durable `intent`。
2. write/read/verify new entry；durable `backendApplied`。
3. state commit unbound record + `legacyReconcile/verifiedPendingPlan` candidate；owner migration=`approvalRequired`；durable `stateFinalized`。
4. audit/terminal；Provider DB内部 plaintext暂时保留，public projection继续 scrub，等待 #55 对 candidate activation projection 的用户批准。
5. admitted activation 按 §7.3 原子切 binding并只 scrub plan点名的 exact `LegacySourceRef`；checkpoint/VACUUM 由 shared DB owner执行。

crash 在 `backendApplied` 后按 capture-candidate 规则完成 state。candidate staged 后 owner仍处于 legacy/approvalRequired，resolve/apply fail closed；未获 admitted plan不得 scrub或 bind。activation crash 按 §7.3 reconcile；Provider scrub 已完成但 journal未更新时，reconcile结构化读取 exact refs确认值不存在，再标 `providerFinalized`；绝不恢复 plaintext。

staged SQL/restore/sync migration 的 DB cutover 在 §8 定义：需要新 binding/scrub时，candidate必须先由独立secure-capture flow产生并进入immutable Change Plan；context前不从staged source读值/自动迁移。没有 admitted plan不得 cutover。journal 保证 cutover/activation任一步崩溃可完成或阻断，不靠自动 bind。

### 7.3 Bind / activate candidate from admitted Change Plan

`activateCandidate` 是唯一 ordinary current-owner bind入口。#41先分别prepare `ActivationConfirmationSlot::{CandidateRead,OldRecordDelete,OldRecordMissingReadback}`，并在无lease时完成全部hardware confirmation；missing slot只预留`BackendDeleteAppliedCasReservation`，不因预确认而获得probe authority。随后才取得独立activation Provider lease并完成#55 final baseline。#35方法本身不申请lease。它消费native #55 admission/prepared bundle并 re-read candidate/record/backend/capability/`OwnerBindingExpectation`。在lease保护下，对每个 `LegacySourceExpectation` 重新解析当前位置并验证exact set/revision：`candidateEquality`还把每个current value与prepared candidate backend read做常量时间比较；`explicitReplacement`只接受用户已批准的replace impact并scrub exact sources，不做不可能的old==new比较。location/revision缺失或额外source均在durable intent/binding切换前返回 stale/dependency changed、`effect=none`；equality policy的value drift同样失败。成功序列：durable `intent` → exact binding CAS/state commit（candidate=`activated`）→ durable `stateFinalized` →同一lease-bound Provider transaction只scrub projection列明的exact refs并做structural readback→ durable `providerFinalized`。无old record时可audit/terminal；rotation时必须继续：durable `oldRecordDeleteIntent` → consume prepared `OldRecordDelete` → backend delete/already-missing → durable `oldRecordDeleteApplied{deleteAppliedCas}` → `OldRecordMissingReadback`消费与actual CAS匹配的reservation及fresh missing receipt → 同一durable transaction只持久化state old=`revoked/supersededByRotation`、`revokedAt=BackendDeleteReceipt.completedAt`与terminal。不存在standalone old-missing/supersession phase或terminal receipt/time字段；任何实现为一个`delete_and_readback_missing`调用都违反合同。Candidate activation不写live target；#41释放activation lease后，绑定成立的owner重新进入readiness并由#55创建独立live-apply plan，再走prepare/confirm/new lease/backup/resolve。

若 binding 已切换但 `providerFinalized` 尚未 durable，exact public映射为：owner `credentialState=bound`；对应 ref aggregate `availability=stale` 且 issue=`SECRET_OPERATION_RECOVERY_REQUIRED`；candidate state=`cleanupRequired`，action=`completeRecovery`。所有 Codex resolve/apply/proxy/usage/balance/model-fetch/`codingPlanUsageProbe` consumer均 fail closed，直到 startup或显式cleanup command完成结构化 scrub；不能以自由文案或“public projection已脱敏”替代内部副本清理。

重启在 intent 时重新验证 plan admission + expectation；revision/digest/owner-set drift返回 `SECRET_DEPENDENCY_CHANGED`/plan stale，`effect=none`。state已切但 Provider scrub未完成时新 binding保持、owner显示 cleanup pending，startup只补 scrub；不回滚到旧 identity。没有 legacy sources时 `providerFinalized`是可验证的 no-op phase。

### 7.4 Rotate candidate + activation cleanup

1. impact 返回 exact affected owners/binding revisions/CAS；capture new material并生成 candidate/new ref。
2. durable rotate-candidate `intent(old,new,impact)`；write/read/verify new；durable `backendApplied`。
3. state commit new unbound record + `rotateBindingSet/verifiedPendingPlan` candidate；old binding/record不变；durable `stateFinalized`/audit/terminal。
4. #55把完整 activation projection纳入 immutable plan；用户批准后走 §7.3 activation。
5. activation state commit一次性切 plan点名的完整 binding set、candidate=`activated`、old retirement=`stale`；durable `stateFinalized`。
6. durable `oldRecordDeleteIntent`；consume独立`ActivationConfirmationSlot::OldRecordDelete`，delete/already-missing后durable `oldRecordDeleteApplied{deleteDisposition,backendCompletedAt,deleteAppliedCas}`。此时old record仍是pending cleanup，绝不写supersession。
7. `ActivationConfirmationSlot::OldRecordMissingReadback`消费与actual CAS匹配的reservation并执行fresh authorized `Validate` missing readback。Confirmed missing receipt在同一device-authority durable transaction内按value消费，该事务只持久化`revoked, revocationSource=supersededByRotation, revokedAt=backendCompletedAt`与terminal/audit；无standalone old-record-missing/supersession-finalized phase或terminal receipt/time字段，`missingCheckedAt`不能替代backend completion time。

crash matrix：

- intent 且 new present：未持久化 verification，删除 new；old binding 不变。
- backendApplied/candidate state未落：expectation仍匹配则落 verified candidate；不匹配则补偿删除 new。
- candidate stateFinalized、activation前：old binding保持；new entry只可 discard或重新计划，绝不自行切换。
- activation stateFinalized 之后：**永不回滚 binding 到 old**；startup按exact checkpoint恢复。delete unknown/failure时完整owner set仍绑定new ref，但active new ref=`stale + SECRET_OPERATION_RECOVERY_REQUIRED`、candidate=`cleanupRequired`、所有consumer blocked；old record保持pending cleanup，journal/recovery row提供typed recovery。Delete成功但receipt未durable时只可重做幂等delete；receipt durable后绝不再次把delete结果当missing证明。
- `oldRecordDeleteApplied`：只能用durable receipt的actual `BackendDeleteAppliedCas`满足已预备missing-readback reservation；present/unknown/locked/denied保留recovery与`remaining=[verifyOldRecordMissing]`，不能supersede。
- missing success：不形成空suffix intermediate phase；同一事务consume missing receipt并只持久化supersession/backend-completion timestamp与terminal，因而crash只见事务前`oldRecordDeleteApplied`或事务后`terminal`。

### 7.5 Delete/revoke

1. impact返回exact record revision、affected owner rows与full `SecretBindingSetCas`；用户确认后process-local readiness被原子claim，并mint durable `deleteAdmission.admissionId`，绑定readiness operation、owners、record/store/binding/backend/device/capability identity与expiry。没有该admission不得写`userDelete` intent。
2. Broker以admission预备`RecoveryConfirmationSlot::AdmittedRecordDelete`与持有`BackendDeleteAppliedCasReservation`的`AdmittedRecordMissingReadback`，`registry.get_exact`核对store instance/exact Arc/record/full CAS；在任何`CredDelete`/Keychain/hardware call前完成各自physical confirmation并再次核对全部identity。预确认missing slot不等于可执行probe。
3. durable `deleteSecret.intent`早于backend delete；consume delete authorization，platform返回generation-bound delete/already-missing receipt；wrapper复核后写durable `backendApplied`并mint actual `BackendDeleteAppliedCas`。该receipt/CAS只证明delete call的durable checkpoint，不授权或证明fresh missing。
4. `AdmittedRecordMissingReadback`只有消费与actual CAS同operation/revision匹配的reservation后才能执行fresh authorized `Validate` missing readback。Wrapper复核exact Arc/store instance/returned generations后才写`missingReadbackVerified`；unknown/present/`BackendRevocationHint`不能冒充missing。任何combined delete+readback backend API均禁止。
5. state retirement=`revoked`、source=`userDelete`、`revokedAt=validated backend completion time`，保留owner binding tombstones解释影响；stable availability/error为`revoked/SECRET_REVOKED`。`stateFinalized` → audit/readback → terminal。

任一crash/不确定性创建或恢复同一`deleteFinalization` arm：它保留delete admission、record/owners、backend identity、`userDelete` provenance与exact未完成delete/readback/state-finalize suffix。Startup/explicit retry只能从durable checkpoint继续；impact或CAS drift保留recovery并返回`effect=none`，不从row缺失猜测完成。没有admitted user-delete intent时观察到OS entry missing仍映射`missing`，与user/central/device revoke严格区分。

central/device revocation不能用自由source/time对象写state：broker必须先从explicit revoke impact/admission构造`SecretNonApplyBackendOperation::Revoke`并mint exact `General::Revoke` authorization，exact handle消费它后才可调用`observe_revocation_once`。Platform source/time先是不可持久化`BackendRevocationHint`；wrapper复核lifetime store instance、exact registered Arc、returned generations与完整ref/store/record/binding/backend/device/capability CAS后才mint§4.4的non-clone `BackendRevocationObservation`；state mutation前fresh revalidate并按value consume一次。普通read/probe即使看见同样source/time也只能阻断当前操作，不能进入receipt factory。OS keyring固定`centralRevocation=false`；普通not-found、locked、denied、unavailable、caller-supplied ref/source/time或跨record receipt绝不推断/移植revoked authority。

### 7.6 Provider owner detach

Provider deletion spans SQLite与device-local authority，但legacy discovery与binding必须正交读取，不能把“有plaintext”折叠成`unbound`，也不能把“有binding”当作没有legacy。Fresh preview在同一Provider row revision下生成两条checked axis：

Preview在生成两条axis前必须把fresh `LegacySourceCoverageReceipt`按value交给`CodexLegacySourceInventoryBridge`；只有bridge fresh重验同一`inventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked`原子shape后，receipt内的current Provider/live refs才能进入下面的current discovery，任一adjacent-blocked observation则直接返回no-impact/effect-none resolution-required arm。Confirm在claim impact、mint transaction id和写journal前再通过同一bridge消费新receipt并要求四字段仍complete/equal；public只投影`LegacySourceCoverageView` count/category，不能返回raw path/raw locator/value/value-derived digest或内部`LegacySourceLocationId`。旧receipt、summary、aggregate `sourceCount=0`或没有11域absent proofs及两组empty data的空集合都不能授权detach。

```text
ProviderDeleteBindingView =
  | { state: "bound", ownerBindingRevision, secretRefDisplay,
      bindingRevision, bindingSetCas, remainingOwners, becomesOrphan }
  | { state: "unbound", ownerBindingRevision }

ProviderDeleteLegacyDiscovery =
  | { state: "none", sourceCount: 0, categories: [] }
  | { state: "present", sourceCount: u32 >= 1,
      categories: sorted non-empty LegacySourceCategory[] }
```

任何current `providerRow|liveAuth|liveConfig` legacy source存在时，无论binding view是bound还是unbound，返回唯一blocked arm：

```text
{
  status: "blockedLegacyResolutionRequired",
  deleteAllowed: false,
  effect: "none",
  owner,
  providerRowRevision,
  bindingView,
  legacySources: { sourceCount, categories },
  action: "resolveLegacyConflict"
}
```

该arm没有`providerDeleteImpactId`、preview expiry、confirm request、`secretRetained`或secret-delete action；只显示无值count/categories与现有binding view。Provider唯一plaintext因此不会被一次Provider-row delete悄然销毁，也不会被虚报为“secret retained”。用户必须先通过current-source migration/replace/reconcile/scrub消除全部legacy occurrence，再fresh preview。

只有`legacySources.state=none`才mint opaque single-use impact：bound arm绑定Provider row/owner revisions、per-owner binding revision、complete binding-set CAS、sorted remaining owners、orphan result，并truthfully显示`secretRetained=true`与独立`get_secret_delete_impact`入口；unbound arm绑定Provider row/owner tombstone revision并显示`secretRetention=notApplicable`，不得伪造ref或retained secret。Confirmation只回传impact id；registry claim后、mint transaction id或写`detachProviderOwner.intent`之前，必须fresh-check两条axis均与preview完全一致。若此时发现任何new/current legacy source，立即terminally invalidate impact并返回Provider stale/effect-none；不得开始Provider transaction，也不得创建detach journal/recovery row。

任一missing/expired/replayed claim或Provider/binding/legacy drift都在两边mutation前返回Provider-owned：

```text
{ code: "PROVIDER_DELETE_IMPACT_STALE",
  action: "refreshProviderDeleteImpact", effect: "none" }
```

这里不能返回secret-delete的`refreshDeleteImpact`，也不能静默mint新impact。

Exact admitted sequence：

```text
claim no-legacy ProviderDeleteImpact + explicit Provider-delete confirmation
  -> begin exact Provider detach transaction; mint transaction id
  -> durable detachProviderOwner.intent
       (impact id + transaction id + Provider/owner/binding authority)
  -> Provider transaction deletes/archives exact metadata row
  -> receive durable Provider detach commit id
  -> durable providerDetachCommitted
  -> device-local owner tombstone/binding-set CAS only
  -> durable localOwnerCasApplied
  -> audit/readback + terminal
```

此flow不取得backend handle、不调用backend delete/revoke，也不改变其他owner binding/record。Nonterminal期间该owner的Codex consumers均blocked。Crash before Provider commit保留两边unchanged并终止/重开preview；crash after commit但before local CAS创建或恢复`ownerDetachFinalization`，其row必须带exact impact/transaction/commit identity、`currentLegacyState=none`、bound|unbound view、remaining owners与local store/owner/binding CAS。Startup不自行推进该arm，只strict-load并公开recovery；main integration从durable transaction/commit receipt重开Provider-owned recovery flow并mint already-held detach context后，显式retry才可做local CAS。任何drift保留callable recovery，不能从Provider row absence猜测成功。Backend secret deletion永远是之后独立impact/confirmation。

## 8. Startup、live import、SQL/restore/sync staged ordering

### 8.1 现有静态事实

当前tree会在managed AppState建立前启动部分import/worker，并允许运行期SQL import、backup restore、WebDAV/S3 download先替换main DB再把post-import sync降级为warning；`sync_current_providers_live`/`run_post_import_sync`还会临时新建`AppState::new(db)`。这些都是待替换的静态缺口，不构成另一套允许顺序。

### 8.2 新 startup exact order

全文唯一normative startup顺序是：

```text
open device-local store/lifetime lock
  -> no-backup DB preflight
  -> construct one AppState + one SecretService from the same opened handle/DB
  -> same-service journal/recovery/legacy/live reconciliation
  -> app.manage(the same AppState) + static command-registration receipt
  -> Clean only: create/readback sanitized backup
  -> publish Clean consumer gate
  -> start workers and Codex consumers
```

跨`crate::secret::device_store`与`crate::database`的opaque bootstrap type必须在Rust可见性上合法，而不是“private type出现在pub(crate) signature”或raw bool：

```text
pub(crate) struct SecretBootstrapToken { // no Clone/Serde/Debug
  deviceSecretStoreInstanceId: Arc<DeviceSecretStoreInstanceId>,
  _seal
}

impl OpenedDeviceLocalSecretStore {
  pub(crate) fn database_preflight_token(&self)
    -> &SecretBootstrapToken;             // sole borrow-only source
}

Database::open_preflight_without_backup(
  authority: DatabaseAuthority,
  token: &SecretBootstrapToken
) -> Result<Arc<Database>>;
```

Struct name可由sibling module引用，fields、constructor与store-instance getter仍private；token只能在持有same opened handle/lifetime lock时借用，不能copy、reconstruct、persist或从DB/path反向mint。`new_production`随后按value消费原`OpenedDeviceLocalSecretStore`，从而证明preflight没有替代或释放该handle。

逐步authority如下：

1. `SecretBootstrap::open(app_handle)`只解析一次immutable `app_local_data_dir`，open/validate root并取得lifetime lock；返回不可clone、不可从path构造的`OpenedDeviceLocalSecretStore`，Drop前不得释放/重开。
2. `Database::open_preflight_without_backup(opened_store.database_preflight_token())`打开/验证同一SQLite authority；#35不增加`user_version`。该入口及其migration路径禁止自动backup/raw copy。
3. `AppState::new_production(db, app_handle, opened_store)`消费上述same handle与same `Arc<Database>`，立即构造唯一managed-candidate `AppState/SecretService`。Constructor不接受PathBuf/root override、第二store、第二DB或临时service，也不调用keyring/worker。
4. 只有该service调用`reconcile_startup(existing_db_context)`：strict decode八类journal与四类recovery，处理confirmation-free crash step，并把fresh `LegacySourceCoverageReceipt`按value交给`CodexLegacySourceInventoryBridge`的startup revalidation。Bridge重新取得complete authority，原子核对`LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked`后，才允许receipt内current Provider/live expectations进入discovery/comparison并以adjacent observations执行supplemental gate。缺receipt、stale/incomplete receipt、omitted/duplicate/unknown domain、proof/data脱绑定、unregistered/moved occurrence或coverage gap一律`Blocked`；即使source总数为0，没有11个absent-domain proof与两组empty data也不能Clean。Empty-DB live import只写从未含value的Provider structural row后由DAO mint existing-owner token并stage unbound candidate。它不自动bind/scrub、不开hardware prompt、不取得Provider lease。结果只能是`Clean(cleanReceipt)`或可修复`Blocked(blockedState)`；`SecretBootstrapCleanReceipt`必须私有持有该bridge fresh revalidation成功后返回的exact `LegacySourceCoverageReceipt`，不能仅保存view/count/bool，也不能Serialize或暴露raw path/raw locator/value/value-derived digest或内部`LegacySourceLocationId`。
5. crate root消费`PreparedProductionAppState`，先且只调用一次`app.manage(state)`，安装静态handler list并在紧邻位置mint不可伪造的`SecretCommandRegistrationReceipt`。Receipt必须逐项证明exactly 15 #35 `SecretCommandName`：`list_secret_summaries|list_secret_backend_options|begin_secret_capture|rotate_secret|list_secret_candidates|discard_secret_candidate|set_secret_locked|get_secret_delete_impact|delete_secret|get_secret_cleanup_impact|retry_secret_cleanup|validate_secret|check_secret_apply_readiness|migrate_legacy_codex_secrets|list_secret_audit`；另一个独立main-integration set必须且只能包含`resume_staged_import_cutover`。后者不是第16个#35 command。`lib.rs`/registration静态断言同时检查两个exact set、无重复/额外/missing handler。之后从`app.state::<AppState>()`取回同一instance并用receipt arm managed runtime；在此之前不得backup/publish gate/start worker。
6. `Clean` arm先由shared DB owner凭持有complete coverage authority的clean receipt构建structured sanitized backup，执行structural scrub、canary与readback；失败保持gate未发布且workers off。成功后`publish_startup_clean(cleanReceipt)`按value消费该receipt，最后才启动WebDAV/S3/sync workers与proxy/model-fetch/usage/apply等consumers。
7. `Blocked` arm保留已managed的同一AppState/SecretService/lifetime lock与scrubbed repair commands，发布Blocked summary，但**不创建backup、不publish consumer-ready、不启动workers/consumers**。Locked/denied/unavailable、legacy conflict或任一nonterminal recovery均属于该arm，不得使用旧inline value。
8. 修复完成后只允许managed state中的same service/handle fresh reconcile并调用`resume_managed_production_secret_startup`：验证初始registration receipt已arm后，重新走`Clean backup → publish gate → workers`。禁止reopen store、重新构造AppState/SecretService、mint第二receipt或用临时authority bypass gate。

### 8.3 Runtime SQL import

`import_config_from_file`不得直接`db.import_sql` + warning。Staged authority是下列五元组的不可伪造组合，不是path、DB bytes/hash、stage id单值或可重放token：

```text
StagedSecretOwnerAuthority {
  tempDatabaseDurableObjectId: 32-lowercase-hex,
  processNonce: fresh random 32-lowercase-hex for this open process,
  stageId,
  owner,
  stagedRowRevision
}
```

其中durable object id由ImportCoordinator写入temp DB自己的stage registry row；每次进程打开成功并从该row证明exact id后才mint新process nonce。`StagedSecretOwnerToken`不可Clone/Serde/Debug，必须同时绑定五元组与`stagedSourceSetCas`，不能满足ordinary readiness/runtime/current-owner activation。

Fresh run exact sequence：

1. Single `ImportCoordinator` suppress auto-sync，创建/打开受限temp DB，校验header/authorizer/schema；在同一durable stage row写/readback`durableObjectId + stageId + owner + stagedRowRevision + stagedSourceSetCas`，随后mint本进程fresh nonce/token。
2. 只枚举same open temp object中无值的staging origin/location/revision/source-set CAS，建立唯一material-free `StagedSecretOwnerToken + StagedSecretImportActivationProjection`。此时禁止读取、解析或比较任何staged source value。Projection只能引用已由独立typed native secure-capture flow写/read/verify为`verifiedPendingPlan`的candidate；若尚无candidate，先返回fresh capture/reconcile action，不从temp source提前migrate。`candidateEquality`仅表示context后必须用staged value验证同值，`explicitReplacement`表示context后验证已批准replace impact；main DB/live/local binding仍unchanged。
3. #55只据该完整token-bound projection生成独立staged plan admission。Admission除`admissionId + planId/digest + projectionDigest`外，私有地保留同一durable object id、fresh process-live identity、stage/owner/row revision；projection digest本身不能替代temp-object authority。此步之前#35不得prepare，之后也不能直接构造cutover context。
4. Main integration的sealed equality port对same open temp object执行process identity/`Arc::ptr_eq`及durable id/stage/owner/row equality，成功后mint不可Clone/Serde/Debug、一次消费的`StagedImportAuthorityMatchReceipt`。任何scalar、path、content hash或caller equality不能替代receipt；receipt factory只有这一处。
5. #35 `prepare_staged_import`按value消费`StagedSecretOwnerToken + #55 admission + authority-match receipt + projection`，operation broker建立private staged backend context；`prepare_staged_import`/`confirm_staged_import`只准备并确认未来的candidate-read authorization与所需hardware slot，不读取candidate material或任何staged value。此时仍不持cutover context/Provider lease，candidate/staged material read call count均为0。任何cancel/expiry/replay/pre-cutover error都由broker唯一的`StagedImportCancelDiscardAuthority`按value消费：terminalize/核对该admission、cancel pending、discard整份prepared bundle；其他模块不能各自cancel/discard同一authority。
6. 只有fully prepared bundle才能由main integration从same open temp DB、same admitted plan和同一consumed match chain构造opaque `ImportCutoverCoordinatorContext`。创建`stagedImport.intent`后fresh-check五元组、material-free stage/source-set CAS、plan admission、candidate/record/backend/device/capability与expected live binding。以上2→3→4→5→6是唯一顺序；context构造成功之前的staged source value read/parse/compare/validate/scrub/readback/cutover call count必须为0。
7. Context按value持有且仍有效后，才首次re-resolve/read exact temp sources；equality mode constant-time compare，replacement mode验证exact approved impact但不要求old==new。Typed receipt后才scrub/readback temp DB并durable `sourcesScrubbed`。
8. Shared DB owner从当前main DB生成sanitized safety backup/readback，并完整validate staged DB。任一failure仍在cutover前：main DB/live/local binding `effect=none`；temp stage/candidate按其durable状态保留或通过同一broker discard authority显式终止。
9. ImportCoordinator执行main DB cutover并返回绑定durable object/stage/owner的`cutoverReceiptId`；durable `cutoverCommitted`。此点之后不能回滚或重开普通import，必须按checkpoint恢复。
10. DAO从cutover后的exact Provider row mint `ExistingSecretOwnerToken`并readback row/revision，durable `liveOwnerMinted`；#35核对receipt/live token/original projection并完成local binding CAS，durable `localBindingFinalized`，再consume admission。Staged token失效且永不进入runtime。
11. Terminal/readback后refresh同一managed AppState/SecretService cache，随后才允许独立#41 live apply；不得新建AppState或reopen device store。

`stagedImport` phase graph仍严格为`intent → sourcesScrubbed → cutoverCommitted → liveOwnerMinted → localBindingFinalized → terminal`；`recoveryRequired`只能携带exact last-completed checkpoint。`intent`的private factory只接受已经构造成功的`ImportCutoverCoordinatorContext`，不能直接接受staged token、projection、admission或scalar identity，因此journal存在本身证明五段authority顺序已完成。它不是四类general recovery之一。Pre-cutover failure（包括已scrub temp但未cutover）对main DB/live/local binding必须`effect=none`；post-cutover failure保留journal并关闭consumer gate，不能从“owner row存在”猜测成功。

Crash reopen/resume固定为同一authority顺序：

1. Startup strict-load journal与temp stage registry，先把旧process nonce对应的pending confirmation/prepared bundle视为不可用；向#55 reconciliation port证明旧`admissionId`后将其`consumed|terminated|supersededForRecovery`之一durable terminal。旧admission未能核对/terminal时停止，不能mint并行admission。
2. ImportCoordinator按journal的`tempDatabaseDurableObjectId`重新打开候选temp object，并从其内部stage row readback exact`durableObjectId + stageId + owner + stagedRowRevision`；path、文件名、snapshot hash或caller text都不能证明identity。
3. 成功后mint**fresh process nonce**与新`StagedSecretOwnerToken`/recovery projection，fresh-check stage/source set CAS、checkpoint、cutover receipt/live owner checkpoint；立刻递增`resumeCas.revision`并以fresh nonce为preimage重算digest，使所有旧request stale。然后#55 mint仅用于该checkpoint的`AdmittedStagedImportRecovery`，绑定fresh五元组、原plan/projection identity、new admissionId与current `resumeCas`。
4. Main integration再次通过sealed equality port对fresh token/projection与fresh #55 admission mint新`StagedImportAuthorityMatchReceipt`；旧match receipt、nonce或admission都不能复用。
5. #35按value消费fresh match，重新prepare/confirm所需backend slot；prepared failure同样由唯一broker discard authority终止fresh admission/bundle。只有完整bundle与同一open object才能构造recovery cutover context并从checkpoint继续。

唯一main-integration resume handler/request是：

```text
resume_staged_import_cutover(
  State<managed AppState>,
  ResumeStagedImportCutoverRequest {
    stageId,
    expectedResumeCas: StagedImportResumeCas
  }
) -> ResumeStagedImportCutoverResultDto

ResumeStagedImportCutoverResultDto =
  | { stageId, status: "activated",
      currentResumeCas: StagedImportResumeCas,
      action: "none", issue: null }
  | { stageId, status: "alreadyActivated",
      currentResumeCas: StagedImportResumeCas,
      action: "none", issue: null }
  | { stageId, status: "cutoverRecoveryRequired",
      currentResumeCas: StagedImportResumeCas,
      action: "resumeStagedImportCutover",
      issue: SecretIssueView }
```

这是与initial activation DTO完全独立的no-value result。Request body恰好只有`stageId + expectedResumeCas`；通用command envelope可另有版本，但`schemaVersion`不进入该body。三个result arm都必须逐字段返回且只返回`stageId + status + currentResumeCas + action + issue`；terminal两arm固定`action="none", issue=null`，recovery arm固定typed issue，不得省略CAS或复用initial payload。Request与public result都没有path、durable object id、process nonce、candidate、owner/ref/backend/receipt、checkpoint detail、summary、audit event或caller-provided admission。Handler由stageId定位durable journal并在同一mutation critical section比较CAS；stale、success、already-terminal与new-recovery响应都返回本次fresh `currentResumeCas`。Full durable/process/admission/record/backend/source/checkpoint/live-owner identity只进入下面的`StagedImportResumeDigest` preimage，不形成第二个public structured identity。`StagedImportResumeDigest` exact UTF-8 preimage为：

```text
fyagent.secret.staged-import-resume.v1\n
operation\0<operationId>\0<phase>\n
stage\0<tempDatabaseDurableObjectIdHex>\0<processNonceHex>\0<stageId>\0<ownerKind>\0<ownerNamespace>\0<ownerId>\0<ownerSlot>\0<stagedRowRevision>\n
sourceSet\0<stagedRowRevision>\0<structureDigest>\0<sourceCount>\n
source\0<locationId>\0<category>\0<origin>\0<structuralRevision>\n
... all staged source rows in LegacySourceRef byte order ...
plan\0<admissionIdHex>\0<planId>\0<planDigest>\0<projectionDigest>\n
candidate\0<candidateId>\0<candidateRevision>\0<comparisonPolicy>\n
comparison\0<candidateEquality|explicitReplacement>\0<verifySameValueMigration|replaceExistingCredential>\0<affectedSourceCount-or-none>\0<replacesBoundBinding-or-none>\n
record\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
expectedLiveBinding\0<unbound|bound>\0<ownerBindingRevision>\0<secretRef-or-none>\0<bindingRevision-or-none>\0<sourceBindingSetRevision-or-none>\0<sourceBindingSetDigest-or-none>\0<sourceBindingSetCount-or-none>\n
checkpoint\0<intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized>\n
sourcesScrubbed\0<stagedRowRevisionAfterScrub>\0<structureDigest>\00\n
cutover\0<cutoverReceiptIdHex>\n
promotedOwner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<providerRowRevision>\0<ownerBindingRevision>\n
```

只有checkpoint已达到时才编码对应`sourcesScrubbed/cutover/promotedOwner`行；`comparison`的policy/userMeaning必须是`candidateEquality/verifySameValueMigration/none/none`或`explicitReplacement/replaceExistingCredential/<count>/<true|false>`完整配对。所有`none`都是literal而非空字段。每次fresh nonce/admission、phase/checkpoint/source/CAS/receipt/live-owner变化都先递增revision再重算digest；mint fresh identity/admission本身就是CAS变化，不能复用旧revision。Stale request、old nonce/admission、receipt drift或temp object proof失败均`effect=none`并保留callable resume state；不降级为general recovery或ordinary import。

### 8.4 Manual binary restore

`restore_db_backup`使用同一typed staged pipeline与`StagedSecretOwnerToken`/`ImportCutoverCoordinatorContext`：先把backup copy到temp DB并完成schema migrate，pre-context只枚举无值structural secret locators/CAS。需要replacement candidate时先走独立secure-capture flow并等待restore Change Plan；只有admission、authority-match、#35 prepare/confirm与exact context均成功后，才能首次读取staged source value并按comparison policy validate/compare/scrub/readback/cutover/finalize。不得把用户选中的backup直接`Backup::new(source, main)`后才补migration。旧backup中的remote/local refs一律忽略，因为refs不应存在于DB；本机state不被restore覆盖。

restore 后 owner reconciliation：

- owner key仍存在：保留本机 binding并 probe；
- owner消失：binding变为 detached impact，但 secret不隐式删除；
- 新 owner无 inline/no binding：`noCredential`；
- 新 owner有单一 legacy structural occurrence：只公开fresh capture/reconcile action；candidate由独立secure-capture flow产生并等待 #55 restore plan，不在context前读值/自动stage/bind；
- 与本机 binding冲突：block，等待用户 reconcile。

### 8.5 WebDAV/S3 sync download

当前 sync manifest 只有 DB SQL、skills ZIP、manifest。保持此 artifact set；device-local root 永不加入 snapshot。

download：verify remote archive hashes → register/open temp DB + schema → preserve local-only DB tables → material-free staged token/projection → #55 admission → authority-match receipt → #35 prepare/confirm → construct exact `ImportCutoverCoordinatorContext` → first staged source read/validate/compare + exact scrub/readback → main cutover receipt → live-owner token/local binding finalize → same-AppState post sync。远端snapshot不能提供state/binding/backend instance/journal/audit。

auto-sync 没有 #55 user-approved plan/交互能力：remote legacy value若需要 candidate activation、遇 conflict、existing-binding unknown、keyring unavailable或需要确认的 hardware step，整个 DB cutover blocked，main DB/live/state `effect=none`。不得自动 stage-and-bind，也不得以 warning继续成功。

### 8.6 Backup/export 与历史 scan/report

边界：

- active DB/WAL/SHM：public projection立即 scrub；内部 legacy source只有 consumed #55 cleanup/activation plan才能 scrub，随后由 shared DB owner执行 checkpoint/VACUUM。崩溃恢复只完成已 durable-admitted intent，不创造新批准。
- **future managed DB backup / SQL export / sync snapshot**：从 snapshot 构建、结构 scrub、canary scan、readback后再原子发布；输出没有 device-local refs/bindings，也没有 Codex value。migration/recovery gate 未收敛时 fail closed，不能复制 raw DB。
- app-private import/download temp：成功/失败后自动删除，best-effort secure overwrite不作为 SSD 保证；不得进入 backup/sync。
- historical managed artifacts：`<get_app_config_dir()>/backups/*.db`、`config.json.bak`、`config.json.migrated` 以及 scanner实际枚举的 FyAgent-owned diagnostic/export cache。只 scan/report，不自动 rewrite/delete。
- user-selected SQL export、用户移动/复制的 backup、任意外部路径：应用无法穷举；只有用户重新选择文件时 scan/report。

v1没有historical artifact cleanup projection、preview、confirm、rewrite或delete command。报告只返回category/count/stable read status，不返回relative/absolute path、secret、snippet或secret-bearing file hash；corrupt/unsupported保持原样并计数。只有当前Provider/live `LegacySourceExpectation` 可进入approved activation并exact scrub。未来若要修改历史文件，必须另起versioned product/contract review，不能从aggregate scan counts推导授权。

## 9. Device-local state 不参与同步的机器可检验 invariants

实现/集成必须验证：

1. `build_local_snapshot` artifact names 精确不含 `device-local/**`，SQL 中不含 state/binding/journal/audit schema或 rows。
2. WebDAV/S3 upload 前后 `state.json` payload hash/store revision 不变。
3. remote SQL import/download 不替换本机 state；相同 owner ID 保留本机 ref，不同 owner走 reconcile。
4. full SQL export/manual DB backup不含 `secretRef` binding；Provider row不含 inline Codex value。
5. restore 一个另一设备生成的 DB不能改变 `DeviceInstanceId`/`BackendInstanceId`。
6. local state file被复制到另一机器但 OS entry缺失时，probe 为 missing/device mismatch并 fail closed；不从环境变量、live config或另一个 backend回退。

## 10. Direct native platform backend 与 capture

本节所有OS API call都只是registered adapter的leaf implementation，不能从service/call-site直接以ref/target/locator调用。进入任一leaf前必须完成§4.4的`get_exact(backendInstanceId,backendGeneration)`、完整record handle与scope-bound authorization/pending消费；leaf返回的generation/device observation还要与handle重检后才能生成receipt。Direct API不等于绕过instance/generation/CAS authority。

### 10.1 macOS store

Direct dependency：

```toml
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = { version = "=3.7.0", default-features = false }
security-framework-sys = { version = "=2.17.0", default-features = false }
core-foundation = "=0.10.1"
core-foundation-sys = "=0.8.7"
```

这些版本来自当前`Cargo.lock`与本机registry的只读核对，不是推测或dependency resolution：`security-framework 3.7.0`只配`core-foundation 0.10.1`，其sys链为`security-framework-sys 2.17.0 + core-foundation-sys 0.8.7`；lock中另一个`core-foundation 0.9.4`不得被误选为本接口authority。High-level crate只负责`SecAccessControl`与query/update helper，raw sys+CoreFoundation只负责create-only dictionary/`SecItemAdd`。

Create每次fresh构造exact policy；lock-local imports为`security_framework::access_control::{SecAccessControl,ProtectionMode}`与`security_framework::passwords::AccessControlOptions`。Locked crate 3.7.0的第二参数是`CFOptionFlags`，所以使用`.bits()`：

```rust
let access_control = SecAccessControl::create_with_protection(
    Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
    AccessControlOptions::empty().bits(),
)?;

```

```text
service = "com.fyagent.secrets.v1"
account = validated SecretRef
```

New-record create不用`PasswordOptions`或`set_generic_password_options`。它以lock-local `core_foundation::dictionary::CFDictionary<CFType,CFType>::from_CFType_pairs`构造heterogeneous exact六键create-only dictionary：

```text
kSecClass               = kSecClassGenericPassword
kSecAttrService         = CFString("com.fyagent.secrets.v1")
kSecAttrAccount         = CFString(validated SecretRef)
kSecAttrSynchronizable  = CFBoolean::false_value()
kSecAttrAccessControl   = SecAccessControl created above
kSecValueData           = CFData(SecretMaterial bytes)
```

随后只调用一次`security_framework_sys::keychain_item::SecItemAdd(dictionary.as_concrete_TypeRef(), null_mut())`；result pointer固定null，不请求persistent ref/attributes/data。Dictionary不得多出`kSecAttrAccessible`、label、authentication context、return-data/ref或sync-any/default selector。Raw create wrapper只能从new-record branch调用，不能成为generic upsert。

`errSecDuplicateItem`绝不进入update：若wrapper能以已绑定fresh record/store/backend identity证明collision/drift，返回`SECRET_BACKEND_CHANGED/effect=none`；否则按本capture/write operation的`SECRET_WRITE_FAILED`进入`SecretInternalError::checked` total table。两种结果都不调用`SecItemUpdate`，不重用unknown item，也不修改state/journal/binding。

`AccessibleWhenUnlockedThisDeviceOnly + empty flags`是create contract：entry只在设备解锁时可用、不可迁移到另一设备，并且不额外要求biometric/passcode-per-use confirmation。它仍只是`hostUser/osProtectedStore`，不是Secure Enclave/hardware-backed claim。`kSecAttrSynchronizable=CFBoolean::false_value()`也是强制合同：便捷函数的默认 options没有synchronizable selector，可能读取或更新同步store；不启用`sync-keychain` feature不能替代selector。

Find/read与delete每次fresh构造**query-only** options：仅`class=genericPassword + service + account + synchronizable=false`（read另加return-data）；绝不附加`SecAccessControl`、authentication context、label或caller-provided policy。Access-control object是create-time stored policy，不是caller authority、lookup selector或delete capability。`generic_password(query)`返回的`Vec<u8>`立即移入`Zeroizing<Vec<u8>>`；delete后用同一query-only selector read必须为item-not-found。

Replace/update不能直接复用`set_generic_password_options`的“add后duplicate再update”便利路径，因为把create-time access-control object放进options会同时进入它的update search query。Exact update path与create branch完全分离：

1. 在exact backend-instance mutex内，用query-only selector读取attributes/data并断言现有entry为non-sync且accessibility正是`AccessibleWhenUnlockedThisDeviceOnly`；locked/denied按下表返回，不把policy object当authorization。
2. Existing arm调用lock-local `security_framework::item::update_item`/`SecItemUpdate`等价封装：search dictionary只含class/service/account/synchronizable=false；update dictionary只含新的`kSecValueData`，因而保留既有access control。
3. Replace query或update返回not-found就是stale/dependency/backend changed的effect-none失败；不得转入create。New-record create返回duplicate也同样不得转入update。因而create、replace各自只有一条可审计mutation path，没有upsert/reconciliation loop。
4. Create/update后用query-only fresh read做constant-time material equality，并用attribute readback再次断言`kSecAttrSynchronizable=false`与`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`；任一不符都fail closed并进入typed compensation/recovery。

SOURCE_FREEZE static/native contract必须证明上述raw create/search/update dictionaries逐key一致，并证明只有create leaf引用`SecItemAdd`；不得用默认证书/ACL、同步selector缺省、access-control-as-query或duplicate-to-update近似实现。Freeze gate还必须记录四个direct crate的exact resolved lock rows/checksums、MIT OR Apache-2.0 license、advisory结果与Rust 1.85.0 matching-host compile；任何lock drift重新审查API shape。

OSStatus 只按 numeric code归一化，不格式化 `security_framework::Error` 到日志/IPC：

| OSStatus | Stable result |
| --- | --- |
| `errSecDuplicateItem (-25299)` during new-record create | proven fresh identity collision/drift → `SECRET_BACKEND_CHANGED/effect=none`; otherwise operation-specific `SECRET_WRITE_FAILED`; never update |
| `errSecItemNotFound (-25300)` | `SECRET_MISSING` |
| `errSecInteractionNotAllowed (-25308)` / `errSecInteractionRequired (-25315)` | `SECRET_LOCKED`, `lockSource=backend`, presence `unknown`; expected locked-state mapping for `AccessibleWhenUnlockedThisDeviceOnly` |
| `errSecAuthFailed (-25293)` / `errSecMissingEntitlement (-34018)` | `SECRET_PERMISSION_DENIED` |
| `errSecNotAvailable (-25291)` / `errSecNoDefaultKeychain (-25307)` / `errSecNoStorageModule (-25312)` | `SECRET_BACKEND_UNAVAILABLE` |
| `errSecUserCanceled (-128)` | operation-scoped cancel；stable state unchanged |
| `errSecDataTooLarge (-25302)` or validated material call returns `errSecParam (-50)` | write/capture `SECRET_INPUT_INVALID` |
| fixed access-control/query/update construction returns `errSecParam (-50)` | `SecretSourceFreeErrorCode::Internal + exact SecretTerminalOperationContext`；`action/retryable/effect`只由47-code/24-action total table导出，caller input不得背锅 |
| 其他 | operation-specific READ/WRITE/DELETE failure；presence `unknown` |

Platform leaf的create/update/read/delete result还必须携带registered wrapper要求的`backendGeneration + deviceBindingGeneration`。Wrapper在`Vec<u8>`、verify receipt、delete receipt或missing receipt出界前复核lifetime store instance、exact Arc与returned generations；不匹配时material在leaf/wrapper内zeroize/drop，receipt不生成。

### 10.2 macOS native capture

当前 direct `objc2-app-kit 0.2.2` 至少增加这些 features：

```toml
features = [
  "NSAlert", "NSApplication", "NSButton", "NSControl",
  "NSResponder", "NSSecureTextField", "NSTextField", "NSView"
]
```

exact sequence：

1. async command创建 `tokio::sync::oneshot`；CaptureCoordinator原子预留唯一 dialog slot。
2. `AppHandle::run_on_main_thread(FnOnce() -> ())`；closure取得 `MainThreadMarker`，构造 `NSAlert` + 单个 `NSSecureTextField` accessory view，localized Continue/Cancel，`runModal`。
3. first-button：把 `stringValue` UTF-8 bytes立即复制进 `Zeroizing<Vec<u8>>`，随后 `setStringValue("")`，释放 Cocoa对象/autorelease scope；second/window-close为空 cancel。
4. main-thread closure只通过 oneshot发送 `Result<SecretMaterial, SecretError>`；不做 keychain I/O、DB/file I/O或 await。
5. receiver dropped/app shutdown：buffer zeroize，CaptureCoordinator释放；shutdown flag使新 capture立即 cancel。active modal由 main-thread shutdown hook结束；不把 Objective-C string交给其他 thread。

无法保证清除 Cocoa framework 自身所有内部临时 copy，因此证据只能声称应用控制的 Rust buffer zeroized、secure field/no renderer transport；不得声称整个 OS UI 内存可证明为零。

### 10.3 Windows store

已有 `windows = 0.61` 增加 features：

```text
Win32_Security_Credentials
Win32_Graphics_Gdi      # CREDUI_INFOW / CredUIPromptForCredentialsW binding gate
```

entry mapping：

```text
Type       = CRED_TYPE_GENERIC
TargetName = "FyAgent/secret/v1/" + validated SecretRef
Persist    = CRED_PERSIST_LOCAL_MACHINE
Flags      = 0
UserName   = "FyAgent" (non-sensitive fixed label)
Blob       = raw UTF-8 API-key bytes, 1..=2560
```

- `CredWriteW(&CREDENTIALW, 0)`；不使用 `CRED_PRESERVE_CREDENTIAL_BLOB`，replace 明确覆盖同 target。
- `CredReadW(target, CRED_TYPE_GENERIC, 0, &mut ptr)`；验证 Type/Target/Persist/blob size，复制 blob 到 `Zeroizing<Vec<u8>>`。`windows 0.61`不暴露 header-inline `SecureZeroMemory`，因此本模块的 `secure_zero_raw(ptr,len)` 必须逐 byte `write_volatile(0)` 并在末尾 `compiler_fence(SeqCst)`（等价采用 WinBase `RtlSecureZeroMemory` 的不可优化语义）；先清零返回 block中的 `CredentialBlob`，再 `CredFree` 整个 allocated block。
- `CredDeleteW(target, CRED_TYPE_GENERIC, 0)`；`ERROR_NOT_FOUND` 在 delete reconciliation 中为幂等“已删除”，普通 probe仍为 missing。
- 同一 backend instance所有 read/write/delete由 mutex串行化；没有 Enterprise/default persistence、search/enumerate或fallback。
- 每个leaf result都携带调用时registered wrapper注入并由adapter回报的`backendGeneration + deviceBindingGeneration`；wrapper在blob/material或receipt出界前用lifetime store instance + `Arc::ptr_eq` + returned generations重检。Generation不由service事后补填。

### 10.4 Windows exact native capture

只使用 `CredUIPromptForCredentialsW`，不使用 `CredUIPromptForWindowsCredentialsW`、`CredUnPackAuthenticationBufferW` 或其 `CREDUIWIN_*` flag family。

`CREDUI_INFOW`：

```text
cbSize         = size_of::<CREDUI_INFOW>()
hwndParent     = app.get_webview_window("main")?.hwnd()?   # non-null exact parent
pszMessageText = localized static UTF-16 <= CREDUI_MAX_MESSAGE_LENGTH
pszCaptionText = localized static UTF-16 <= CREDUI_MAX_CAPTION_LENGTH
hbmBanner      = null
```

若 `main` window/HWND获取失败，返回 `SECRET_BACKEND_UNAVAILABLE`；不得以 null desktop parent继续弹窗。

buffers/flags：

```text
target     = L"FyAgent/secret-capture/v1\0"
username   = Zeroizing<Vec<u16>>(CREDUI_MAX_USERNAME_LENGTH + 1 = 514 code units)
             prefilled L"FyAgent\0"
password   = Zeroizing<Vec<u16>>(CREDUI_MAX_PASSWORD_LENGTH + 1 = 257 code units)
save       = BOOL(FALSE)
authError  = 0
context    = NULL
flags      = CREDUI_FLAGS_GENERIC_CREDENTIALS
           | CREDUI_FLAGS_ALWAYS_SHOW_UI
           | CREDUI_FLAGS_DO_NOT_PERSIST
           | CREDUI_FLAGS_EXCLUDE_CERTIFICATES
           | CREDUI_FLAGS_KEEP_USERNAME
```

不设置 `SHOW_SAVE_CHECK_BOX`、`PERSIST` 或 `EXPECT_CONFIRMATION`。`DO_NOT_PERSIST` 保证 UI 不显示/save Credential Manager entry；capture 成功后由 FyAgent 单独 `CredWriteW` 到另一个 target。

capture 通过 Tauri main-thread `run_on_main_thread` + oneshot返回；UI closure内不做 store/DB I/O。`NO_ERROR` 时只读取 password首个 NUL前 code units，严格 UTF-16→UTF-8、empty/NUL/size验证；`ERROR_CANCELLED` 映射 `SECRET_INPUT_CANCELLED`。任何 return path都先对 username/password全 buffer调用 `zeroize::Zeroize`（该 crate使用不可被优化掉的 volatile clear），`save=FALSE`，再发送 stable result；原 buffer不交给 store layer。`ERROR_NO_SUCH_LOGON_SESSION`→unavailable；invalid flags/parameter→input/internal configuration failure；其他→input failure且不记录 raw text。

### 10.5 Direct dependency / byte-carrying error closure

```toml
[dependencies]
zeroize = { version = "1.8.2", features = ["derive"] }
subtle = { version = "2.6.1", default-features = false }
```

不引入 keyring store crates，因此 `keyring_core::Error::BadEncoding(Vec<u8>)`、`BadDataFormat(Vec<u8>, ..)`、`Ambiguous`、`NoDefaultStore`、`NotSupportedByStore` 不存在于 production error graph。dependency/source gate必须拒绝这些 crates。

若 future plugin adapter内部使用类似 byte-carrying error，adapter boundary 必须按 value match：先把 carrying `Vec<u8>` 移入 `Zeroizing`/显式 `zeroize()`，只返回 stable code；不得 `Debug`、`Display`、`source()`、log或把原 error装入 public error。`Ambiguous/NoDefault/NotSupported` 保守映射 unavailable，绝不映射 missing。

### 10.6 MSRV 决策

- `src-tauri/Cargo.toml rust-version = "1.85.0"` 保持。
- macOS direct pins来自当前lock/local registry：`security-framework =3.7.0`（lock checksum `b7f4bc775c73d9a02cde8bf7b2ec4c9d12743edf609006c7facc23998404cd1d`, declared Rust 1.85）、`security-framework-sys =2.17.0`（`6ce2691df843ecc5d231c0b14ece2acc3efb62c0a398c7e1d875f3983ce020e3`, Rust 1.70）、`core-foundation =0.10.1`（`b2a6cd9ae233e7f62ba4e9353e81a88df7fc8a5987b8d445b4d90c879bd156f6`, Rust 1.65）、`core-foundation-sys =0.8.7`（`773648b94d0e5d620f64f280777445740e61fe701025087ec8b57f45c791888b`, no crate rust-version declaration）。四者registry manifest均为`MIT OR Apache-2.0`；这只是read-only static fact，freeze仍须license/advisory/lock diff gate。
- `zeroize 1.8.2`满足 1.60；lock中已有`subtle 2.6.1`（freeze后仍验证MSRV）；`windows 0.61`/`objc2-app-kit 0.2.2`已存在。
- 删除原 research 中 `windows-native-keyring-store 1.1.0 (MSRV 1.88)` 与两类 native-keyring-store候选。
- freeze 后 dependency resolution必须记录 exact lock diff、license/advisory，并在matching native macOS与Windows x64各自执行并保存raw/CI leaf：`cargo +1.85.0 check --locked --workspace --all-targets --manifest-path src-tauri/Cargo.toml --target <matching-native-triple>`。Manifest必须记录`rustc 1.85.0` release/host/target、exact `Cargo.lock` hash、command/exit；当前toolchain或任何Rust 1.97 pass不能替代这个MSRV gate。

## 11. AppState injection、threads 与 one-shot capability

### 11.1 Constructors

`AppState` exact shape由 canonical owner `main integration`（executor `root/MainIntegrationOwner`）落地：

```text
AppState {
  db: Arc<Database>,
  secret_service: Arc<SecretService>,
  ...existing fields
}
```

constructors：

```text
SecretBootstrap::open(app_handle: &AppHandle)
  -> Result<OpenedDeviceLocalSecretStore>
Database::open_preflight_without_backup(
  authority: DatabaseAuthority,
  token: &SecretBootstrapToken
)
  -> Result<Arc<Database>>
new_production(db: Arc<Database>, app_handle: AppHandle,
               opened_store: OpenedDeviceLocalSecretStore)
  -> Result<AppState>
test_support::AppStateBuilder::new()
  -> AppStateBuilder       #[cfg(any(test, feature="test-hooks"))]
AppStateBuilder::with_database(db: Arc<Database>)
  -> AppStateBuilder
```

`SecretBootstrap::open`是唯一从`app_handle.path().app_local_data_dir()`构造private validated `DeviceLocalSecretRoot`的production入口；它在SQLite open/backup前取得lifetime lock、mint process-local `Arc<DeviceSecretStoreInstanceId>`并返回不可clone的opened handle。该handle的`pub(crate) database_preflight_token() -> &SecretBootstrapToken`只是可由database sibling命名的opaque borrow；type fields/constructor仍private，不能泄漏store identity或替代handle。`new_production`只能消费该handle，不能注入PathBuf/override或重新open；handle移入`SecretService.store_lifetime=Production(...)`并由该Arc持有到AppState teardown。Production assembly在消费opened store/validated backend registry后私有创建唯一`Arc<BackendOperationBroker>`；non-public `SecretServiceDeps { authority, backends, broker, readiness, mutationGate, capture, clock, ids }`只搬运same Arc，最终由`SecretService.broker` field持有。Broker独占capture-intent/capability/pending三个registries；authorization由brokered scope/handle按value持有，deps没有registry字段，caller/tests没有broker setter/trait injection/extractor。既有AppState fields/visibility不变，新增`secret_service`与construction seal/token为store-private，足以禁止外部struct literal。Integration crates只通过feature-gated public opaque `fyagent_lib::test_support::AppStateBuilder::new()`选择closed `inMemory|lockedRead|deniedRead|backendUnavailable|verifyMismatchOnce|oldDeleteFailOnce` fixture；builder可选择closed backend behavior，但不能注入/提取broker、registry或private id。需要保留既有non-secret DB identity时只可调用`with_database(Arc<Database>)`。raw deps、traits、material、root/path与service factory仍private。`SecretService`不持有或获取Provider DB/lease。`AppState`同时保留现有`Arc<Database>`与持有同一opened store的service；#41/main-integration coordinator通过既有DB field与crate-private`secret_service()`取得两者并创建不可伪造的already-held Provider/import context。bootstrap/production constructor只打开/验证local files、构造backend objects，不调用真实keyring；DB preflight在sanitized gate前不会自动backup。

现有 production `AppState::new(db)` call（startup、`sync_support`、`sync_current_providers_live`）必须消失；runtime import/sync使用 managed state中的同一 service。ordinary tests统一迁到 injected in-memory backend/capture/clock/id source。

### 11.2 Thread/await rules

- native dialog：main thread + oneshot；不在 dialog内持 service/DB lock。
- state/journal/hash、SecurityFramework、CredMan I/O：`tauri::async_runtime::spawn_blocking`。
- async command只 await blocking result；`SecretMaterial` 不跨 await。
- `SecretMaterial` 无 Serialize/Deserialize/Clone/ordinary Debug；Drop zeroize。
- direct store read返回 material后，只能进入 exact owner module内私有 adapter；不存在 generic result seam 或 crate-wide raw-byte closure constructor。
- runtime只使用合同逐字冻结的 `execute_proxy_request -> ProxyRequestExecutionReceipt`、`execute_usage_probe -> UsageProbeExecutionReceipt`、`execute_model_fetch -> ModelFetchExecutionReceipt`。各 owner-private adapter在 blocking边界内构造完整 closed `Prepared*Request`，该对象只能 consuming `send_once(self)` 并恰好执行一次 network await；不得把借用 bytes、header或prepared request重新存进 Provider/job/cache/event。

### 11.3 Prepared capability

`PreparedSecretCapability` 不持 material。它是不可 Serialize/Clone/Debug 的 process-local one-shot token；private registry entry id只存在于`BackendOperationBroker`内部，调用者不可读、回传或据此claim/discard。Entry绑定：

```text
operationId + owner + secretRef + targetSink + consumer
admissionId + admittedPlanId + planDigest + projectionDigest
Arc<DeviceSecretStoreInstanceId> + exact registered Arc binding
storeRevision + recordRevision + bindingRevision + complete bindingSetCas
deviceInstanceId + backendInstanceId + backendGeneration
deviceBindingGeneration + capabilityRevision
expiresAt + consumed=false
```

#41 在 Provider lease 前完成 optional hardware confirm/prepare；prepare先`get_exact(instance,generation)`并把完整`BackendRecordHandle + RegisteredBackendHandleBinding + BackendAuthorizationScope`移入token，不能只保存scalar lookup fields。若需要物理确认，`BackendPendingConfirmation`拥有同一handle/scope/expiry；confirm原子consume pending并fresh验证store instance/Arc/instance/generation/record/store/binding/device/capability后才产生authorization。取得lease、baseline recheck与sanitized backup后，在writer首次mutation前把role-specific token按value交回broker。Broker在同一个private method内完成entry atomic claim、role extraction与`consumed=false→true`，随后再次验证全部identity、binding仍指向ref、policy active、retirement live、sink允许；调用者不能先claim后选择role，也没有public/private ID参数。任一lock/rotate/delete/rebind/backend/device/capability change都使旧token`effect=none`失败。显式discard同样按value交给broker，终止entry与authorization而不把ID暴露给caller。

Backend operation context不是共享`pub(crate)`字段bag。Apply、runtime、activation、recovery、migration、staged import、capture/discard/delete与revoke context的fields/constructors均private to broker；每个constructor只消费对应owner模块提供的opaque admission/readiness/journal/runtime/staged authority，并返回不可互转的role-specific bundle。Sibling owner只能调用其固定entrypoint，不能用scalar operation/ref/revision/slot拼出context，也不能把apply authorization当delete、recovery或revoke authority。

Activation-specific bundle同时准备candidate read/compare、old-record delete与old-record missing-readback三个authorization；后两者是不同slot，hardware confirmation都在Provider lease前完成，执行仍由durable delete checkpoint分隔。Cleanup bundle只按recovery kind准备允许的slot：activation active-read/old-delete/old-missing-readback；capture compensation预备uncommitted-delete与reservation-bound uncommitted-missing；delete finalization预备admitted-delete与reservation-bound admitted-missing；owner detach none。每个missing slot都以`SecretBackendOperation::Validate`复用该record的`operationConfirmation.validate`，可在mutation前完成自己独立的physical confirmation，但只有durable delete receipt mint actual `BackendDeleteAppliedCas`后才可消费执行authority。`retry_secret_cleanup`只能按value交给broker消费已确认、未过期且与exact kind/recovery CAS匹配的bundle；cancel/expiry/replay不执行scrub/delete/readback，也不改变recovery row。任何`AuthorizedBackendDeleteAndMissingReadback`、组合receipt或一次authorization覆盖两call的API都不得存在。

operation-scoped `HardwareConfirmStep`只从prepare response返回，不进入`SecretRefAggregate`、`SecretOwnerCredentialSummary`、list cache或state file。Step id只是public continuation key，不是authority；server registry中的pending row必须持有完整handle/scope/slot且不可clone。Activation old delete与old missing-readback、capture-compensation delete与missing-readback、delete-finalization delete与missing-readback各有不同slot/step，即使同一hardware policy也是两个authority。Cancel/expiry清除并terminally consume pending；重新apply/recovery必须创建new operation/slot，不能复用旧step。

Revoke不复用apply/read/probe capability。只有broker从explicit revoke impact/admission构造private revoke context，exact registered handle验证`centralRevocation=true + SourceAndTime`并消费`SecretNonApplyBackendOperation::Revoke` / exact `General::Revoke` authorization后，才可调用platform `observe_revocation_once`。Platform source/time只是wrapper-private `PlatformBackendRevocationHint`；wrapper复核store instance/exact Arc/returned backend+device generation与full CAS后才mint§4.4的non-clone `BackendRevocationObservation` receipt。Receipt由state commit按value消费；任何字段或capability drift均在写state前拒绝。普通probe/read最多形成non-persistable `BackendRevocationHint`；OS-keyring record或caller对象不能构造该receipt。

本层错误出口只接受`SecretInternalError::terminal_operation_failure(SecretSourceFreeErrorCode, SecretTerminalOperationContext)`或四个source-bearing typed factories；没有`SecretInternalError { ... }` literal、raw-message constructor或“所有retryable行共用一个action”的helper。`candidate_terminal_cleanup_pending()`的pointer-free recovery issue必须严格匹配checked pending disposition/journal；其他general recovery均由`operation_recovery_required(pointer)`构造。Action必须命中24-entry total destination table；device/native未识别分支只能以`SecretSourceFreeErrorCode::Internal + exact context`进入该表，不能添加unrouted fallback或unregistered legacy placeholder。

### 11.4 Real keyring test gates

- ordinary unit/integration constructor只能使用 in-memory backend；禁止读取 env后静默切 real store。
- real store tests一律 `#[ignore]` 且双 gate：compile target匹配 + `FYAGENT_NATIVE_SECRET_TEST=1`；每次 random valid ref、finally delete、delete readback missing。
- interactive capture另需 `FYAGENT_NATIVE_SECRET_CAPTURE_UAT=1`，必须 user-visible matching host；CI/noninteractive CRUD不冒充 capture UAT。
- failure injector只能显式包裹 backend并记录 `evidence_class=failure_path, evidence_origin=fault_injection`，不得把 injected denial写成 native OS denial。

## 12. Hardware instance / per-device contract

MVP 没有 hardware implementation。production registry未注册任何 `hardware` instance时，Add/Replace UI不得显示 hardware选项；已有 imported/local hardware record只能显示 unavailable/device mismatch，不能回退 OS keyring。

future plugin registration要求：

1. 服务端生成 `SecretBackendInstanceId`；plugin提供 validated `pluginId`、opaque non-secret locator、backend generation与非敏感 device display。
2. capabilities按 secret/locator返回并写入 record snapshot，不用 singleton static bool。
3. device replacement/rebind生成新 backend instance/generation和新 secret ref；不就地改旧 locator。
4. confirmation step的server-side pending row绑定完整record handle、operation scope、closed slot、lifetime store instance/exact registered Arc、backend instance/generation、store/record/binding/device/capability revisions与expiry；UI只显示step id、device label与timeout，不能拿public step重建authority。Activation exact slots为`ActivationConfirmationSlot::{CandidateRead,OldRecordDelete,OldRecordMissingReadback}`；recovery exact slots为`RecoveryConfirmationSlot::{ActiveRecordRead,OldRecordDelete,OldRecordMissingReadback,UncommittedRecordDelete,UncommittedRecordMissingReadback,AdmittedRecordDelete,AdmittedRecordMissingReadback}`。其中delete/missing-specific slots恰好8个，连同两个read slots共10个；每个missing slot的operation固定为`Validate`并复制`operationConfirmation.validate`，不能用一次physical confirmation/authorization合并任意两slot，future adapter也必须逐slot分别mint并由用户分别确认。
5. `persistentTargetProjection=false` 在 #55 preview和#41 resolve前两次 fail closed。
6. `DeviceInstanceId` 是随机 local namespace，不是硬件指纹/attestation；复制 state到另一机器只能依赖 exact backend probe发现 missing/mismatch，不得宣称硬件身份认证。
7. `centralRevocation=true`还要求adapter发布closed source/time capability；revoke前后fresh验证exact handle，结果只能通过non-clone consuming `BackendRevocationObservation` receipt写state。Device mismatch、missing或自由source/time不能替代receipt。

## 13. Native/failure evidence 与 pre-evidence push

### 13.1 Source freeze delivery order

2026-08-15已裁决并取代早期“final review后才push”的草案：Windows host必须先取得已发布且回读一致的source-freeze SHA。Canonical order为：

1. source-freeze commit；本机静态/module/integration gate通过。
2. **pre-evidence push** 仅推 `codex/issue-35-secret-backend` exact freeze SHA 到 origin；不创建/合并 main，不部署。
3. read back remote ref必须等于 freeze SHA；evidence manifest记录 remote ref与SHA。
4. Windows x64 host/CI以 SHA detached checkout；`git rev-parse HEAD`精确相等、worktree clean后运行。
5. 任一 source fix产生新 freeze commit、重新 push，旧 native/screenshot/failure evidence全部 invalid。
6. final review/PR在新一轮全部 evidence后进行。

MVP mandatory Windows target：`x86_64-pc-windows-msvc`。ARM64不是本轮 acceptance substitute，也不声称已验收。

### 13.2 Canonical tasks to register after freeze

shared manifest owner注册以下 exact task names；本附件阶段不执行：

```text
mise run secret:native:macos:crud
mise run secret:native:macos:uat
mise run secret:native:windows:crud
mise run secret:native:windows:failure
mise run secret:native:windows:uat
mise run secret:scan:codex -- <runtime-artifact-manifest>
mise run secret:artifact:scan
mise run secret:evidence:verify
```

`secret:scan:codex` 是要求显式 artifact/allowed-sink manifest 的低层扫描器；`secret:artifact:scan` 是 evidence-host 枚举/guard 并调用前者；`secret:evidence:verify` 验证完整 manifest/schema/readback。这里与 `research/native-evidence-plan.md`、`execution-plan.md` 共用同一八项 canonical 列表，不另设 subset 或 alias。

八项task以外，source/static registration evidence必须分别断言：#35 `SecretCommandName` exact set为§8.2列出的15个且invoke handler逐一reachable；main integration另有且仅有`resume_staged_import_cutover`。该handler不计入15、不命名为第16 command，staged crash/UAT必须调用这一独立handler。两组任一missing/duplicate/extra registration都使source freeze gate失败。

matching native macOS与Windows x64在任何CRUD/UAT前都必须保存`cargo +1.85.0 check --locked --workspace --all-targets --manifest-path src-tauri/Cargo.toml --target <matching-native-triple>`的raw/CI leaf，记录rustc release/host、exact Cargo.lock hash、command与exit。当前Rust 1.97或其他toolchain的check只能作为额外信息，不能满足该gate。

每个 native task硬检查 platform、env gate、clean exact SHA，输出 machine-readable manifest：source SHA、OS/version/arch、target triple、start/end、exit code、random ref display suffix、cleanup result、evidence class、artifact scan result。manifest无 material/ref full value/raw error。

### 13.3 Windows acceptance classes

| Scenario | Mechanism | Evidence label |
| --- | --- | --- |
| write/read/replace/delete | real CredMan, LOCAL_MACHINE, random ref | `native_runtime` |
| missing read after delete / never-created ref | real `CredReadW -> ERROR_NOT_FOUND` | two items may reference one raw artifact: `native_runtime`; `failure_path` with origin `real_os` |
| capture OK/cancel | user-visible `CredUIPromptForCredentialsW` on Windows x64 | `uat`; cancel may also emit `failure_path` with origin `real_os` |
| backend unavailable | injected direct backend constructor/probe failure | `failure_path`, origin `fault_injection` |
| verification fails after real write | real CredMan write + deterministic one-shot verify mismatch; compensate delete/readback missing | `failure_path`, origin `fault_injection` |
| old delete fails after rotate | fault injector wraps real write/read, blocks old delete before OS call | `failure_path`, origin `fault_injection`; separate real CRUD remains `native_runtime` |
| locked/denied mapping | deterministic injection before OS call, separately for locked and denied; real OS fixture optional only | `failure_path`, origin `fault_injection`; optional reproducible fixture may add `real_os` |

Formal Windows x64 run必须全部通过：real missing、injected backend unavailable、injected verification failure、injected old-delete-after-switch；另须通过locked与denied两种typed mapping以及real interactive capture cancel。三类只是最低计数字段，不能用来跳过上述fixed set；每项按真实origin标注，ARM64/CI/mocks不能替代。

### 13.4 macOS acceptance

- real default Keychain create/read/replace/delete/missing，随机 ref，最终 cleanup；`native_runtime`。Create必须走raw six-key CFDictionary + one `SecItemAdd`，replace只走service/account/non-sync search + data-only update；duplicate create与replace-not-found都fail closed且mutation call count证明绝不跨分支upsert。Create/replace后分别read back attributes，必须为`AccessibleWhenUnlockedThisDeviceOnly + synchronizable=false`。Native contract同时断言find/delete query没有access-control object/auth context，access-control仅出现在create dictionary。
- `NSSecureTextField` user-visible OK/cancel；`UAT`。
- locked/interaction-not-allowed映射固定为`SECRET_LOCKED + lockSource=backend + presence=unknown`；可用deterministic Keychain fixture时记录`evidence_class=failure_path, evidence_origin=real_os`，否则只保留injected module coverage，不冒充。Accessibility/assertion只是新增计划项，本设计阶段没有执行。

### 13.5 Recovery / staged UAT matrix

下列是freeze后必须分别取证的case定义，不是本轮静态文档已通过项：

| Case | Required closure evidence |
| --- | --- |
| `activationCleanup` | binding CAS已切new ref后分别中断legacy scrub、old delete与delete-receipt durable；`RecoveryConfirmationSlot::OldRecordDelete`/`OldRecordMissingReadback`是独立authorization并在Provider lease前完成所需hardware confirm，后者消费durable `BackendDeleteAppliedCas`与fresh missing receipt；同一事务只持久化supersession/revokedAt + terminal，崩溃只可见pre-transaction `OldRecordDeleteApplied + [verifyOldRecordMissing]`或post-transaction `Terminal + []` |
| `captureCompensation` | backend write后、candidate state前注入crash/unknown；exact candidate/record/backend row可达；`RecoveryConfirmationSlot::UncommittedRecordDelete`与`UncommittedRecordMissingReadback`独立claim/authorization，中间`deleteApplied{deleteAppliedCas}` durable checkpoint；fresh missing后state finalize，无orphan entry |
| `deleteFinalization` | durable user-delete admission后在delete、delete-receipt durable、独立missing-readback与state-finalize处中断；`RecoveryConfirmationSlot::AdmittedRecordDelete`/`AdmittedRecordMissingReadback`各自authorization并由`BackendDeleteAppliedCas`分隔，retry保留owners与`userDelete` provenance，未admitted missing仍显示missing而非revoked |
| `ownerDetachFinalization` | Provider commit后、local CAS前中断；impact/transaction/commit/no-legacy literal/bound-or-unbound view/remaining owners exact；retry的backend call count固定为0。另测preview后注入legacy source：impact stale/effect-none发生在transaction/journal前，durable detach/recovery row count仍为0 |
| candidate discard/expiry | 两种flow共用`discardCandidate`且immutable `pendingTerminalDisposition`分别为discarded/expired；任一不确定保持pending可达，无recovery pointer/第五kind；terminal expiry只给`refreshSummary`，refresh后mint全新intent/authority，旧candidate/operation重放零写 |
| staged import resume | 逐次证明token/projection→#55 admission→authority-match receipt→#35 prepare/confirm→cutover context；pre-cutover failure对main/live/local binding effect-none；旧nonce/admission先terminal，fresh nonce/admission递增CAS并使旧request stale；唯一public request body仅`stageId + expectedResumeCas`，独立`ResumeStagedImportCutoverResultDto`三armdata每次exactly回`stageId/currentResumeCas/status/action/issue`，terminal issue=null、recovery typed，schema/audit/candidate/owner/ref/summary均absent；stale request零写 |

用户可见confirmation/recovery card在matching host记录`UAT`；deterministic crash/CAS/backend-call assertions记录`failure_path`或module/integration来源，不能用静态review、总case count或单一activation截图替代。

## 14. Main-document integration invariants

2026-08-15 working-tree design已采用以下裁定；这不是self-approval，fresh static audit与same-SHA reviewers仍必须逐项验证没有回归：

1. 删除所有“#35 reserves SQLite v17”、`secret_records/secret_owner_bindings/secret_audit_events` DDL、v16→v17 test/rollback与相关 file ownership。
2. 把 repository 术语改为 native `DeviceLocalSecretStore`；handoff声明 DB/provider plan不持 `secretRef` binding table。
3. 删除 keyring store crate候选与 `keyring_core::Error` production mapping，改成 §10 direct dependencies/errors/MSRV。
4. Startup全文只保留`open store → no-backup DB preflight → same AppState/SecretService → reconcile → app.manage/static registration receipt → Clean sanitized backup → publish gate → workers`；`SecretBootstrapCleanReceipt`必须持有经唯一bridge fresh验证的`LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked`原子authority。零source但缺任一domain proof或两组empty data仍Blocked；Blocked保留same managed state但无backup/consumer/worker，repair resume不得reopen。
5. Journal operation恰好八类并逐variant定义phase/required authority；四类strict recovery各有独立state/step/CAS preimage。Activation/recovery共有10个exact slots，其中delete/missing-specific恰好8个；每对delete/missing authorization由durable receipt + actual `BackendDeleteAppliedCas`分隔。Activation missing success与supersession/terminal同事务，无第四step或standalone missing phase；capture/delete保留各自`missingReadbackVerified`后再finalize。绝无第九generic recovery、staged第五kind或组合delete+readback API。
6. Candidate explicit discard/expiry共用immutable `discardCandidate` target；不确定时只公开checked `pendingTerminalDisposition + discardCandidate` pending形态。Terminal expiry只给`refreshSummary`，新capture/rotation必须mint全新authority，旧candidate/operation不复用。
7. Provider delete把binding与current legacy discovery正交读取；任何legacy source阻断preview且不mint impact，preview后legacy drift也在transaction/journal前stale/effect-none。Durable detach/recovery只允许no-legacy bound/unbound，stale action固定Provider-owned `refreshProviderDeleteImpact`，owner detach永不backend delete。
8. Staged import唯一顺序为temp token/projection→#55 admission→sealed authority-match receipt→#35 prepare/confirm→cutover context→source read/validate/compare→scrub/readback→cutover；context前source-value/cutover call count为0，cancel/discard authority唯一。Crash先terminal old admission，再mint fresh nonce/admission并递增/重算CAS；resume handler只接受stage id + exact CAS，独立no-value result三arm每次返回current CAS，full identity只进入preimage。
9. Durable `DeviceInstanceId=dev_*`与process-local non-Clone/non-Serde `DeviceSecretStoreInstanceId`严格分离；state/journal/backend identity只持durable id，live handle/scope/pending/receipt以Arc同时持两者与exact registered Arc。Platform returned generations在material/receipt出界前复核。只有显式Revoke authorization可调用`observe_revocation_once`并mint full-CAS consuming receipt；ordinary hint不可持久化。
10. 同步strict SecretRef、binding-set revision、separated policy/retirement state、hardware UI hidden boundary与no-fallback。
11. Manifest owner只使用`#35 module | #55 | #41 | main integration`；Prompt/Memory v17作为保留外部schema lane/dependency记录，不成为owner literal；#35 module worker只占新secret files。
12. 把 pre-evidence push放到 Windows取证之前，并按 evidence class区分 real OS/injected/UAT。
13. `Arc<SecretService>` field直接持有唯一`Arc<BackendOperationBroker>`，private assembly只可搬运same Arc，caller/test不可注入/提取；broker独占capture-intent/capability/pending三个registries，authorization只存在于brokered consuming scope/handle。Capture intent绑定durable/process store identity、owner/purpose/kind/current owner-binding、原子包含11-domain identity/current exact expectations/adjacent observations的opaque coverage receipt与hidden bound arm；begin只有intent id+exact backend，renderer不构造authority。Legacy conflict走同一typed flow。
14. macOS new-record create使用exact direct pins、`AccessibleWhenUnlockedThisDeviceOnly + empty access-control flags`与raw six-key non-sync CFDictionary调用一次`SecItemAdd`；duplicate绝不update。Replace只走query-only/data-only update，not-found不create；find/delete也只按class/service/account/non-sync，access-control object不作caller authority。Native evidence必须断言create/update attributes、branch call count与locked mapping，本静态阶段不冒充已执行。
15. Backend confirmation projection恰好五种operation；所有missing slot固定执行`Validate`并复制`operationConfirmation.validate`，同时保留独立slot/authorization/CAS/receipt/checkpoint。Source gate逐组证明15个#35 command与独立resume handler；matching macOS/Windows必须另有Rust 1.85.0 locked all-targets leaf，1.97不替代。

只有这些invariant在所有权威文件一致、三位 reviewer exact-tree re-read 后，相关 AR/DD/PR finding才可记为 closed。

## 15. Static sources read for this closure

Repository/API facts were statically read from：

- `src-tauri/src/database/{mod.rs,backup.rs,schema.rs}`
- `src-tauri/src/commands/{import_export.rs,sync_support.rs,webdav_sync.rs,s3_sync.rs}`
- `src-tauri/src/services/sync_protocol.rs`
- `src-tauri/src/{lib.rs,store.rs,config.rs}`
- `src-tauri/{Cargo.toml,Cargo.lock,tauri.conf.json,Info.plist}` and `rust-toolchain.toml`
- task-local `secret-contract-v1.md` and `research/issue-35-authority.md` as the current proposed public-contract/authority closure inputs
- lock-local sources for Tauri 2.10.3, `windows 0.61.3`, `security-framework 3.7.0` (`access_control.rs`, `passwords_options.rs`, `passwords.rs`, `item.rs`), `security-framework-sys 2.17.0`, `objc2-app-kit 0.2.2`, `zeroize 1.8.2`
- Microsoft Learn: `CredUIPromptForCredentialsW`, `CREDUI_INFOW`, `CREDENTIALW`, `CredReadW`, `CredWriteW`, `CredDeleteW`, `CredFree`

Evidence level remains `source_report + code_audit + static_design_closure`.
