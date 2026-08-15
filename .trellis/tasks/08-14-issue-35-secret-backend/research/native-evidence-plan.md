# Issue #35 native/runtime/failure-path 取证计划

## 0. 文档状态与边界

- `DESIGN_FREEZE=PENDING`；本文文档状态为 `STATIC_EVIDENCE_PLAN`。
- 本文证据级别：`source_report + code_audit + static_evidence_plan`。
- 本轮只静态读取 workflow、task manifest、依赖声明、runtime preflight、PRD/技术/详细/执行设计和初始评审；未运行 dependency resolution、test、build、browser、renderer、server、native runtime、OS prompt 或 screenshot。
- 下文命令和 task 名是 **DESIGN_FREEZE 后、SOURCE_FREEZE 前必须落地的执行合同**，不是本轮已执行结果。
- 本轮新增的macOS accessibility/raw-create assertions、Rust 1.85.0 matching-host gate、15+1 registration、coverage/store-identity与operation-slot/checkpoint/staged-authority assertions同样只是计划；它们没有native result、manifest或pass状态。
- 当前 product/architecture/detailed reviews 仍是 `REQUEST_CHANGES`。本文不能替代三项 reviewer 对修订后 exact tree 的重新评审，也不能单独授权 implementation、push 或外部 host 操作。

## 1. 静态基线结论

### 1.1 现有 CI 能证明什么

`.github/workflows/ci.yml` 当前事实：

1. `workflow_dispatch` 会把 change plan 强制为 full，因而可在 source freeze 后跑完整 jobs。
2. `backend-windows` 使用 `windows-2025` x64，执行 Rust `check`、`clippy`、ordinary `cargo test --features fyagent/test-hooks`。
3. `windows-native-contracts` 有 `windows-2025/X64` 与 `windows-11-arm/ARM64`，但只运行现有 Codex Desktop explicit-SID native smoke；没有 Issue #35 Credential Manager case。
4. `backend-macos` 只执行 Rust `check`、`clippy`、ordinary tests。
5. `desktop-acceptance-contract` 明确是 mock/MSW/visual-policy，不是 desktop native acceptance。
6. issue branch `codex/issue-35-secret-backend` 不在 push trigger 的 `[main, dev/laiyongjie]` 中；pre-evidence push 本身不会自动启动 CI，必须显式 `workflow_dispatch` 或创建 PR。本任务采用前者，PR 留到全部 evidence 之后。
7. 现有 workflow 没有 `FYAGENT_NATIVE_SECRET_TEST`、ignored real-store filter、Credential Manager CRUD、secure-capture UAT、Keychain CRUD 或 secret evidence manifest。

因此现有 CI 输出只能按实际 job 标为 `ci_compile`、`unit_test`、`integration_test` 或既有 `native_contract`。即使 job 名含 `Windows Native Contracts`，也不能标为 Issue #35 的 `native_runtime`；GitHub hosted runner 的 ordinary tests也不能标为 interactive `UAT`。

### 1.2 change classifier 缺口

当前 classifier 会把 `src-tauri/Cargo.toml`/`Cargo.lock` 和既有 Codex Windows paths 路由到 `windowsNative`，但一般 `src-tauri/src/secret/**` 只落入 `backend`。实现阶段必须在 SOURCE_FREEZE 前把以下路径纳入 `windowsNative`，并以 classifier unit contract 固定：

```text
src-tauri/src/secret/platform/windows.rs
src-tauri/src/secret/capture/windows.rs
Issue #35 Windows native/failure harness
Issue #35 Windows native mise/task wrapper
```

否则依赖提交之后的 Windows-only source fix 可能跳过 ARM64/X64 native contract matrix。

### 1.3 mise/package task 现状

- 现有 canonical tasks 包括 `env:check`、`system:check`、`rust:fmt:check`、`rust:check`、`rust:clippy`、`rust:test`、`test:v2`、`test:v2:browser`、`tasks:validate` 等。
- `mise run check` 是 current-host aggregate；它包含 ordinary Rust/frontend tests，但不包含 real keyring 或 interactive capture。
- `rust:test` 通过 `scripts/tasks/rust.mjs` 运行普通 Cargo tests，当前没有清除 native-secret env，也没有 exact ignored native test contract。
- `package.json` 当前没有 Issue #35 secret/native scripts。没有必要仅为别名重复增加 package script；native evidence 应由 canonical mise task 统一 platform/env/SHA/manifest guard。
- 当前没有下文所列 `secret:*` tasks。未注册、未静态验证这些 task 之前，runtime path 仍是 `BLOCKED`。

### 1.4 runtime-preflight 与初始 review 的约束

- `research/runtime-preflight.md` 已明确 Windows matching host 尚未 provision，且 Windows native evidence 不可由 cross-compile、WSL、mock 或复制 artifact 替代。
- DD-010 要求 ordinary tests 只能使用 injected in-memory backend；真实 store 必须 ignored + explicit env gate。
- DD-013 要求冻结 Windows target、pre-evidence push/checkout 顺序、真实 OS 与 injection 分类、cleanup 和 manifest。
- DD-014 要求 source-freeze 后使用 exact task/command，而不是自然语言类别。
- AR-011/DD-004 要求 Windows capture 只选一套 Win32 API/flags/buffer contract，不能混用两套 Credential UI family。

### 1.5 当前 native dependency 事实

当前 `src-tauri/Cargo.toml`：

- Windows 已直接依赖 `windows = 0.61` 与 `windows-sys = 0.61`，但 `windows` features 尚无 `Win32_Security_Credentials` 和 `Win32_Graphics_Gdi`，不存在 Issue #35 的 CredMan/CredUI implementation。
- macOS 已直接依赖 `objc2 = 0.5`、`objc2-app-kit = 0.2`，但 AppKit 只启用 `NSColor`；尚不足以构造 `NSAlert + NSSecureTextField`。
- `security-framework 3.7.0`、`security-framework-sys 2.17.0`、`core-foundation 0.10.1`、`core-foundation-sys 0.8.7`与`zeroize 1.8.2` 虽已作为 transitive lock entries 出现，但不是 FyAgent 当前完整direct dependency shape，不能把lockfile presence当作可用API/实现证据；lock中另有`core-foundation 0.9.4`，实现必须精确选择0.10.1链。
- manifest 的 `rust-version` 是 `1.85.0`。初始 `os-keyring-options.md` 候选 `windows-native-keyring-store 1.1.0` 要求更高 MSRV且默认 persistence 与目标不一致；当前 device-local design 已改为 direct APIs。

实现 authority 应采用 direct dependencies：

```text
Windows: existing windows 0.61 + Win32_Security_Credentials + Win32_Graphics_Gdi
macOS:   direct exact security-framework =3.7.0,
         security-framework-sys =2.17.0,
         core-foundation =0.10.1, core-foundation-sys =0.8.7;
         create uses
         SecAccessControl::create_with_protection(
           Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
           AccessControlOptions::empty().bits())
         plus raw six-key non-sync CFDictionary -> create-only SecItemAdd;
         explicit data-only update preserves policy, duplicate never updates
capture: existing objc2-app-kit 0.2.2 + NSAlert/NSApplication/NSButton/
         NSControl/NSResponder/NSSecureTextField/NSTextField/NSView features
memory:  direct zeroize 1.8.2
compare: direct subtle 2.6.1 (content-independent equal-length comparison)
```

只读lock/local-registry事实：四个macOS crates的checksums分别为`b7f4bc775c73d9a02cde8bf7b2ec4c9d12743edf609006c7facc23998404cd1d`、`6ce2691df843ecc5d231c0b14ece2acc3efb62c0a398c7e1d875f3983ce020e3`、`b2a6cd9ae233e7f62ba4e9353e81a88df7fc8a5987b8d445b4d90c879bd156f6`、`773648b94d0e5d620f64f280777445740e61fe701025087ec8b57f45c791888b`；registry license均`MIT OR Apache-2.0`，declared MSRV依次为1.85/1.70/1.65/undeclared。Dependency resolution、license/advisory、exact lock diff与actual MSRV check只能在DESIGN_FREEZE后执行；它们仍是build/static evidence，不是native runtime。

## 2. Acceptance target disposition

| Target | Required evidence | Issue #35 disposition |
| --- | --- | --- |
| Windows x64 (`x86_64-pc-windows-msvc`) | real CredMan CRUD + `LOCAL_MACHINE`; ≥3 distinct failure paths; real interactive capture success/cancel UAT; zero-residue cleanup | **Mandatory / DONE blocker** |
| Windows ARM64 (`aarch64-pc-windows-msvc`) | existing CI native-architecture/compile contract | `compile_only`, `runtime_not_accepted`; cannot substitute for x64 and no ARM64 secret-runtime/UAT claim |
| macOS current native host | real non-sync, `AccessibleWhenUnlockedThisDeviceOnly` Keychain create/update/read/delete/missing; query/update-policy assertions; real `NSAlert/NSSecureTextField` success/cancel UAT | **Mandatory / DONE blocker** |
| Linux | unavailable/no fallback contract only | no native store claim |

ARM64 disposition is deliberately explicit: this MVP may close only with an x64 Windows runtime claim. If the product later claims the secret backend is runtime-supported on Windows ARM64, that claim requires a separate real ARM64 CRUD/capture run or an explicit fail-closed unsupported capability until such evidence exists. ARM64 compile success alone never upgrades `runtime_not_accepted`.

## 3. SOURCE_FREEZE 之前必须落地的 harness/task 合同

所有脚本、ignored tests、fault injectors、manifest validator、CI classifier rules 和 task registrations 都是 evidence-producing source，必须包含在 `FREEZE_SHA` 内；不得 source freeze 后临时手写 harness。

### 3.1 Canonical task names

```text
mise run secret:native:windows:crud
mise run secret:native:windows:failure
mise run secret:native:windows:uat
mise run secret:native:macos:crud
mise run secret:native:macos:uat
mise run secret:scan:codex -- <runtime-artifact-manifest>
mise run secret:artifact:scan
mise run secret:evidence:verify
```

`secret:scan:codex` is the low-level canary scanner. `secret:artifact:scan` first enforces host/source/worktree guards and enumerates the exact artifact/allowed-sink manifest, then invokes `secret:scan:codex`; its output becomes the one `artifact_scan` evidence item. They are distinct registered tasks with a fixed call relationship.

八项task名保持不变；MSRV不是第九task alias，而是每个matching-host source gate必须保存的raw/CI leaf：

```text
macOS native host:
cargo +1.85.0 check --locked --workspace --all-targets \
  --manifest-path src-tauri/Cargo.toml --target <matching-macos-native-triple>

Windows x64 native host:
cargo +1.85.0 check --locked --workspace --all-targets \
  --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
```

Leaf记录`rustc 1.85.0` exact release/host、target、`Cargo.lock` SHA-256、完整command、start/end、exit与raw artifact hash。当前`rust-toolchain.toml`/host的Rust 1.97结果不能替代1.85.0 leaf；cross-host/cross-target结果也不能替代matching native host。

Source-registration gate另行解析Rust union与Tauri invoke list，要求exactly 15 #35 `SecretCommandName`项：`list_secret_summaries|list_secret_backend_options|begin_secret_capture|rotate_secret|list_secret_candidates|discard_secret_candidate|set_secret_locked|get_secret_delete_impact|delete_secret|get_secret_cleanup_impact|retry_secret_cleanup|validate_secret|check_secret_apply_readiness|migrate_legacy_codex_secrets|list_secret_audit`；main integration另有且仅有`resume_staged_import_cutover`。Resume不是第16个#35 command。Static assertion必须证明15项各自reachable、两组无duplicate/missing/extra，staged crash/UAT调用独立resume handler。

每个 task 必须：

1. 验证 exact OS/arch/target triple；错误 host 直接 fail。
2. 验证 `HEAD == FYAGENT_EVIDENCE_SOURCE_SHA`、detached/approved branch source identity、tracked worktree clean。
3. 只写 repository-ignored `artifacts/issue-35-secret-evidence/<source-sha>/<run-id>/`。
4. 使用 exact test/binary selector；不得用宽泛 `--include-ignored`。
5. 在 `finally` 中运行 direct cleanup 与 readback；case 失败不跳过 cleanup。
6. 输出一个 machine-readable manifest；stdout 只含 run id、case id、stable result 和 artifact path，不含 material、full ref、target、raw OS message。

`secret:native:*:uat` 必须启动 `FREEZE_SHA` 构建出的真实 FyAgent development desktop，调用 production `NativeSecretCapture` path，并使用真实 main window/main thread。独立 Win32/Cocoa sample、renderer password input、mock dialog 或 headless test不能关闭 UAT。

### 3.2 Ordinary test 永不触碰真实 keyring

真实 store tests 同时具备以下 gates：

```rust
#[ignore = "real OS keyring; run only through secret:native:* task"]
```

```text
FYAGENT_NATIVE_SECRET_TEST=1
FYAGENT_EVIDENCE_SOURCE_SHA=<40 lowercase hex>
FYAGENT_SECRET_EVIDENCE_DIR=<absolute path under repository /artifacts/>
```

interactive capture 再额外要求：

```text
FYAGENT_NATIVE_SECRET_CAPTURE_UAT=1
```

执行保护：

- ordinary `mise run rust:test`、`mise run check` 和 standard CI jobs必须显式移除/拒绝 `FYAGENT_NATIVE_SECRET_TEST` 与 `FYAGENT_NATIVE_SECRET_CAPTURE_UAT`，且不传 `--ignored`。
- native task 只运行一个 exact ignored case/filter；case 内再次校验 env、platform、interactive session 和 evidence directory。
- ordinary `AppState` tests只能构造同一 DB 上的 in-memory backend/capture；production constructor 不得在构造时 probe/read/write OS store。
- fault plan 只能由 `cfg(test)`/`test-hooks` 下的 typed injector 构造，production registry和普通 runtime env不能启用 arbitrary fault。
- CI 若未来增加 noninteractive real CRUD job，也必须是显式 opt-in workflow-dispatch path；不得把 real store test塞进 `backend-windows`/`backend-macos` ordinary aggregate。

双 gate 的意义是：误设 env 不会越过 `#[ignore]`，误用 `--ignored` 也不会越过 env/platform/source checks。

### 3.3 V8 authority/phase harness assertions

SOURCE_FREEZE内的portable module/integration harness还必须把本轮static findings变成machine assertions；这些结果按实际标`unit_test|integration_test|native_contract|failure_path`，不能因为涉及backend type就标`native_runtime`：

1. `LegacySourceCoverageReceipt`必须是siblings可命名/移动/消费、data fields/struct literal不可见、无Clone/Serde/Debug的`pub(crate)` opaque authority；唯一checked constructor是`pub(crate) checked_from_complete_inventory_authority`。Factory按value消费只有`CodexLegacySourceInventoryBridge`能构造的`CompleteLegacySourceInventoryAuthority`，并原子mint `LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable exact expectations + adjacentBlocked observations`。Identity恰好11域：`currentProviderLive`加`processEnvironment|windowsRegistryCurrentUser|windowsRegistryLocalMachine|shellStartupFile|commonConfigJson|commonConfigBackup|commonConfigMigrated|commonConfigSqlite|rendererLocalStorage|liveConfigMerge`；每域都有structural revision/presence/count且与两组数据逐域一致。Receipt禁止raw path/raw locator/value/value-derived digest；current exact expectations可保留non-value-derived `LegacySourceLocationId`，adjacent observations不可转换成`LegacySourceRef`。Capture options只能在该原子验证后经broker内`SecretCaptureIntentRegistry::mint_from_atomic_snapshot`生成短期单次`SecretCaptureIntentId`，registry row绑定receipt、durable/process store identity、owner/purpose/`newBinding|replaceBinding|legacyReconcile`、current owner-binding revision与hidden bound arm。Startup `SecretBootstrapCleanReceipt`、summary/readiness、capture options/claim与Provider-delete preview/confirm都必须各自把receipt按value交回bridge fresh revalidate全部四字段；缺失/stale/incomplete/omitted/duplicate/unknown domain、proof/data脱绑定或空集合没有exact 11 absent proofs与两组empty data时在effect前Blocked。Begin wire只有intent id+exact registered backend id；`claim_once`失败时dialog/backend/journal call count为0。Terminal candidate expiry返回`refreshSummary`，刷新后mint新intent，旧candidate/operation重放零写。
2. 唯一`Arc<SecretService>` field直接持有唯一`Arc<BackendOperationBroker>`；private production assembly/deps只可搬运same Arc，caller/public/test没有setter、trait injection或extractor。Broker独占capture-intent/capability/pending恰好三个registries；authorization由brokered consuming scope/handle持有，不创建第四registry。Compile/static gate证明deps无registry字段、builder/owner modules看不到registry/id、所有private `Backend*OperationContext`字段/factory或跨role conversion。只有`BackendOperationBroker::for_apply|for_runtime|for_activation|for_recovery|for_migration|for_staged_import|for_non_apply`可生成`BrokeredBackendOperationContext`，且exact wrapper只调用`prepare_brokered_operation`。Bootstrap的`SecretBootstrapToken`是合法可命名`pub(crate)` opaque sibling type，但fields/constructor仍private且只能从same opened store借用。
3. Durable state/journal/backend identity只编码`DeviceInstanceId=dev_*`；type本身non-Clone/non-Serde的process-local identity只以`Arc<DeviceSecretStoreInstanceId>`进入live record handle/scope/pending/authorization/receipt。每个live对象同时绑定两种id与exact registered Arc；wrong durable device、wrong process store、same scalar/different Arc都拒绝。Platform raw read/write/delete/probe/revocation结果回报backend/device generations；wrapper在material/receipt出界前检测mismatch并使payload zeroize/drop、effect none。Static serde/hash scan证明process identity/address/derived value不进入state/journal/audit/preimage。
4. Activation与activationCleanup都必须观测`old-record delete authorization -> durable three-field checkpoint -> independent old-missing authorization consumes that CAS + fresh receipt -> one durable transaction writes supersededByRotation/revokedAt=backendCompletedAt + Terminal`。Normal runtime使用`ActivationOldRecordDeleteCheckpoint`且process-local `ActivationOldRecordDeleteApplied`另持postcondition；failure journal使用`ActivationOldRecordDurableCheckpoint`；recovery使用`RecoveryOldRecordDeleteCheckpoint`。三者均保留exact `deleteDisposition + backendCompletedAt + deleteAppliedCas`，只能checked reconstruct，不能只存CAS或unchecked互转。Crash-visible只能是事务前三字段checkpoint + `[verifyOldRecordMissing]`或事务后`Terminal + []`，terminal preimage不保留missing receipt/time或intermediate delete receipt。Candidate discard另观测`CandidateDiscardConfirmationSlot::RecordDelete -> durable CandidateDiscardDeleteCheckpoint{three fields} -> RecordMissingReadback(Validate) consumes actual CAS -> MissingReadbackVerified -> Terminal`；没有stateFinalized phase/general recovery。`captureCompensation`/`deleteFinalization`继续分别观测`delete authorization -> deleteApplied{deleteAppliedCas} -> missing authorization consumes CAS -> missingReadbackVerified -> finalize`。Harness在每个边界crash断言fresh missing前没有terminal/supersession，并禁止任何`delete_and_readback_missing`组合API/receipt。
5. Hardware confirmation projection恰好五种operation：`CaptureVerify|Validate|ResolveForApply|Delete|Revoke`，不得出现第六种`MissingReadback` policy。Activation slots为3个，recovery slots为7个，原两组仍共10个，其中delete/missing-specific 8个；candidate discard另有`CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`两个delete/missing slot。因此operation-specific prepared slots总数exactly 12，delete/missing-specific总数exactly 10。每个missing slot执行`Validate`并逐字复制`operationConfirmation.validate`，但相同device/policy不能折叠任何slot/authorization/CAS/receipt/checkpoint；journal仍8、general recovery仍4。
6. Ordinary read/probe的revocation source/time只能形成不可持久化`BackendRevocationHint`。只有exact `General::Revoke` authorization可调用`observe_revocation_once`并mint nonclone `BackendRevocationObservation`；wrong scope/Arc/store/generation/full-CAS receipt与hint persistence全部零写。
7. Staged import trace必须严格为`temp token/projection -> #55 admission -> authority-match receipt -> #35 prepare/confirm -> cutover context -> source read/validate/compare -> scrub/readback -> cutover`。`prepare_staged_import`/`confirm_staged_import`只准备/确认未来candidate-read authorization；Context构造成功前，candidate material read与staged source value read/parse/compare/validate/scrub/readback/cutover call count必须均为0。Resume digest preimage必须编码same journal `operationId`与exact `StagedImportResumePhase::{Intent,SourcesScrubbed,CutoverCommitted,LiveOwnerMinted,LocalBindingFinalized}`；fields逐arm累计为none、after-scrub CAS、+cutover receipt、+promoted owner、local同三字段，未列field forbidden。Canonical fixtures与相邻crash cases恰好为`staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`；每个断言exact bytes/digest、required/forbidden rows、只恢复suffix，并断言phase或fresh nonce/admission变化都递增revision/recompute digest、旧CAS零写。Public request/result仍使用冻结的no-value五字段边界；full identity只进入digest preimage。
8. Error/action parity gate证明唯一literal是private `SecretInternalError::checked(code,SecretTerminalOperationContext,SecretErrorSources)`；source-free只接受closed `SecretSourceFreeErrorCode`，四类source-bearing error只走typed factory，general recovery必须pointer而candidate-terminal-cleanup是唯一例外。47-code/24-action parity、四个capture actions的`secretCaptureFlow`与四个runtime actions的exact `fixedRuntimeFlow`缺一即fail closed；unrouted fallback action、unregistered legacy destination、unknown action或缺destination均禁止。
9. Registration gate对same `FREEZE_SHA`同时解析15-name `SecretCommandName`与Tauri invoke wiring，并单独验证main-integration `resume_staged_import_cutover` handler；15+1不得合成16-name union。Resume的source/UAT trace必须命中该独立handler，不能直接调用service绕过registration receipt。
10. Coverage failure harness固定table-driven cases：missing receipt、stale `LegacySourceInventoryRevision`、11个domain逐一omitted、duplicate/unknown domain、任一domain structural revision drift、presence/count与`currentScrubbable|adjacentBlocked`不匹配、任一current expectation或adjacent observation drift、拆分/重组receipt，以及“所有count=0但仅aggregate empty list”伪证。Compile/static gate另证未经bridge构造的authority不能调用`pub(crate)` factory，receipt没有raw path/raw locator/value/value-derived digest字段而允许current expectation内的non-value-derived `LegacySourceLocationId`。Startup/summary/readiness/capture/options/claim/Provider-delete preview/confirm在每个failure下均断言effect-none，且backup/publish/dialog/backend/journal/impact/transaction call count为0；只有exact 11个absent-domain proofs与两组empty data的原子receipt可通过complete-coverage gate。

## 4. Exact source-freeze delivery 与执行顺序

### 4.1 Freeze candidate

1. 三项 design review 对同一 exact tree 达到 `P0=P1=P2=0`，并写入 DESIGN_FREEZE receipt。
2. 完成 implementation、focused tests、integration/failure policy tests和 task registration。
3. 生成 source-freeze commit `F`；定义 `FREEZE_SHA=git rev-parse F`。
4. 在 `F` clean tree 上 fresh-run non-native canonical gates。任何修复生成新 commit `F2`；`F` 的结果不可转移给 `F2`。
5. 验证 task docs/classifier/evidence schema 已包含在 `FREEZE_SHA`；此后禁止修改 source/harness/fixtures。

### 4.2 Pre-evidence push

Windows host无法取得未 push commit，因此正式 native evidence 前允许一次受控 branch push；它不是 main merge、deployment 或 final PR：

```text
local FREEZE_SHA
  -> push exact SHA to refs/heads/codex/issue-35-secret-backend
  -> read back remote branch SHA
  -> require remote SHA == FREEZE_SHA
```

执行时使用 explicit refspec，不使用 `git add -A`、force push 或 main ref。remote readback、时间和 operator记录入 manifest source envelope。若 remote ref 不等于 `FREEZE_SHA`，不 dispatch CI、不让 manual host执行。

### 4.3 CI 在 manual native host 之前

1. remote exact-SHA readback成功后，记录 dispatch start timestamp。
2. 对 `ci.yml` 使用 `workflow_dispatch --ref codex/issue-35-secret-backend`。在 CI 完成前禁止移动该 remote branch。
3. 从 dispatch timestamp 后的 runs 中选择 exactly one new workflow-dispatch run；要求 run `headSha == FREEZE_SHA`。不允许“最近一次绿色 run”猜测。
4. 等待 `CI / Required` terminal success，并记录 run id/attempt/job conclusions。
5. CI failure先修 source，产生/推送新 freeze SHA并整轮重启；不要继续 costly/manual UAT。

当前 workflow 的 dispatch force-full 会覆盖 Windows x64 backend、X64/ARM64 native-contract compile path和 macOS backend。其 evidence class仍分别是 compile/unit/contract；不能因 host 是 Windows/macOS 就改称 native keyring evidence。

### 4.4 Manual host detached checkout

CI exact-SHA pass 后，每台 native host执行：

1. fetch remote issue branch，不 fetch/merge其他工作；确认 fetched tip等于 `FREEZE_SHA`。
2. `checkout --detach FREEZE_SHA`，不在 moving branch 上取证。
3. 记录 `git rev-parse HEAD`、target triple、OS build、arch、interactive-session status；要求 tracked/untracked worktree clean（ignored evidence dir除外）。
4. 运行 `env:check --json` 与 `system:check --json` 作为 host preflight；它们只证明环境 readiness。
5. 在matching native host先执行§3.1 exact Rust 1.85.0 locked workspace all-targets command，记录toolchain/target/Cargo.lock hash/raw artifact；若实际rustc不是1.85.0或exit非0，停止native lane。Rust 1.97 pass不可继续充当MSRV receipt。
6. 顺序执行 noninteractive CRUD → failure cases → interactive UAT → final artifact scan/evidence verify。
7. 每个 task 后再次验证 `HEAD == FREEZE_SHA`、tracked tree clean。

推荐实际 host 顺序：macOS CRUD/UAT，Windows x64 CRUD/failure/UAT，最后可选 ARM64 compile readback。Windows x64仍是 terminal blocker；任一平台 source fix使此前所有 native/failure/UAT manifests invalid。

### 4.5 Evidence-only follow-up commit

原始 logs留在 ignored `/artifacts/`。所有 case通过、cleanup readback为 missing、scanner为零命中后，才创建 evidence-only commit `E`。其精确path allowlist为 `.trellis/tasks/08-14-issue-35-secret-backend/evidence/**`与`research/evidence-index.json`；文件必须sanitized且不得定义/改写command、contract、authority、harness或fixture。`git diff --name-only FREEZE_SHA..E`必须是该集合子集。独立final reviewer读取`F+E`后，review-only `V`只新增`reviews/final-review.md`。后续governance-only `G`只可修改`task.json`的status/evidence-pointer/PR fields、向`implement.md|implement.jsonl`追加status row、更新`reviews/index.md` pointer并新增`research/github-readback.md`，且通过narrow-field/append-only verifier。若改动 source、harness、fixture、authority或task-contract，则必须生成新`FREEZE_SHA`并重跑；纯证据/评审/治理追加只重跑evidence verifier/readback as applicable。

## 5. Windows x64 real native runtime

### 5.1 Host preflight

- native Windows x64、真实 unlocked interactive user session；不是 WSL、cross-compile、Windows service session或远程复制 artifact。
- 推荐使用隔离的标准本地 test user；CredMan CRUD与 UAT必须在同一 user/session中完成。
- 预先检查固定 capture target `FyAgent/secret-capture/v1` 不存在。若已存在，停止并人工裁决；不得静默覆盖/删除未知 entry。
- 每次 run生成新的 valid random `SecretRef` 和 runtime canary；canary只存在 native process memory/real store，不进 argv、env、stdout、manifest或普通 file。
- case串行运行；不同 run/ref不复用。

### 5.2 Credential Manager CRUD + persistence

`secret:native:windows:crud` 必须使用 production direct backend调用真实：

```text
CredWriteW -> CredReadW -> CredWriteW(replace) -> CredReadW -> CredDeleteW
            -> CredReadW(ERROR_NOT_FOUND)
```

动态断言：

1. `Type == CRED_TYPE_GENERIC`。
2. `TargetName == FyAgent/secret/v1/<generated SecretRef>`。
3. `Persist == CRED_PERSIST_LOCAL_MACHINE`；任何 `SESSION`/`ENTERPRISE` 都 fail。
4. `Flags == 0`，username为固定 non-sensitive label，blob size `1..=2560`。
5. first read与 first canary constant-time equal。
6. replace 后 read只与 second canary相等，仍为 `LOCAL_MACHINE`。
7. delete 后 direct read返回 `ERROR_NOT_FOUND`。
8. 返回 block先清零 blob再 `CredFree`；manifest只记录 boolean assertion results。

这个 case是 `native_runtime`。它证明 real CredMan CRUD与 readback persistence attribute；不证明 interactive capture。

## 6. Windows failure-path matrix

### 6.1 分类规则

- `real_os`：真实 Win32 call返回的结果，未由 wrapper改写。
- `fault_injection`：typed test injector稳定在指定 call/phase触发；即使其他步骤使用真实 CredMan，也不能把注入的 failure标为 real OS。
- fault plan按 `runId + operationId + SecretRef + exact call ordinal`匹配，一次性触发，并断言实际 call count；错过、重复或打到另一个 ref均 fail。
- 不通过停用 Credential Manager service、破坏正常用户 policy、锁工作站后盲跑或修改真实 credential ACL来制造“真实拒绝”。只有可重置的隔离 VM/policy fixture才可追加 `real_os` denial evidence。

### 6.2 Formal cases

| Scenario | Mandatory mechanism | Stable induction | Required invariant | Cleanup/readback | Evidence |
| --- | --- | --- | --- | --- | --- |
| `missing` | **real OS** | 对 never-written fresh ref执行 `CredReadW`；另在 CRUD delete 后再 read | 两次均为 `ERROR_NOT_FOUND -> SECRET_MISSING`；binding/owner不被静默删除；无 fallback | idempotent direct delete；read仍 missing | `failure_path`, origin=`real_os` |
| `locked` | deterministic injection；real OS optional | wrapper在 first probe/read前返回 typed locked，OS call count必须为0 | presence unknown；stable locked action；target/live/state effect=none；无 fallback | direct read fresh ref missing；无 journal residue | `failure_path`, origin=`fault_injection`; optional isolated fixture另发 `real_os` case |
| `denied` | deterministic injection；real OS optional | wrapper在 first probe/read前返回 typed denied，OS call count必须为0 | presence unknown；stable denied action；target/live/state effect=none；无 fallback | direct read fresh ref missing；无 journal residue | `failure_path`, origin=`fault_injection`; optional isolated fixture另发 `real_os` case |
| `backend_unavailable` | deterministic injection | backend constructor/probe wrapper返回 unavailable，OS call count=0 | stable unavailable；no fallback；no record/binding/target mutation | fresh ref direct read missing；operation terminal/clean | `failure_path`, origin=`fault_injection` |
| `verification_fail` | real write + deterministic verify injection | real `CredWriteW`成功；one-shot verify read返回不同 in-memory material或 comparator forced mismatch | `SECRET_VERIFY_FAILED`；owner/old binding不切换；compensation预备独立delete与reservation-bound missing slots，先consume delete并durable写`deleteApplied{deleteAppliedCas}`，随后missing authorization才可consume该CAS | injector disabled后 direct read new ref必须 missing；两个slot/receipt/checkpoint均terminal，cleanup ledger removed | `failure_path`, origin=`fault_injection` |
| `rotation_old_delete_fail` | real old/new CRUD + deterministic delete injection | old entry与binding ready；new entry real write/read verify；DB/local-state switch后，wrapper在 old `CredDeleteW` **之前** one-shot fail | 完整owner set仍绑定new ref，但active new ref=`stale + recovery-required`、candidate=`cleanupRequired`且consumer fail closed；old record pending cleanup；old-delete/old-missing-readback是独立slot，绝不回滚/no fallback | 先验证active ref stale与consumer blocked，再禁用injector：delete receipt durable后独立fresh missing readback，missing前不得supersede；terminal后new ref恢复ready；最后删new，两ref direct read均missing | `failure_path`, origin=`fault_injection` |
| `capture_cancel` | **real interactive OS UI** | operator在 real `CredUIPromptForCredentialsW`点击 Cancel | `SECRET_INPUT_CANCELLED`；store write count=0；无 owner/binding/success audit；fixed capture target absent | read fixed target和generated store target均 missing | 独立 `uat` case；另发 `failure_path`, origin=`real_os` case |

Formal run执行上表全部 cases。固定failure-path case IDs为`missing|locked|denied|backend_unavailable|verification_fail|rotation_old_delete_fail|capture_cancel`，每项都必须有独立item且`result=pass`；其中`capture_cancel`还必须由同一次真实交互raw artifact另发独立`uat` item。Windows failure count gate至少要求三个distinct scenario，但该计数绝不允许跳过任何固定case。

### 6.3 Cleanup failure 的处理

1. harness开始时在 ignored、user-only permissions 的 recovery ledger写入 full generated refs；ledger不上传、不提交。
2. 每个 case使用 `try/finally`；fault injector只覆盖 product operation，不覆盖 final direct cleanup。
3. final cleanup直接调用 production platform backend的 delete，并用 direct read确认 missing。
4. cleanup success 后删除 local recovery ledger，再 finalize manifest。
5. cleanup或readback失败：manifest result=`failed`、`cleanup.status=failed`，保留 local recovery ledger供 same-host人工/repair task使用；禁止上传绿色 artifact、禁止计入 failure count、禁止 DONE。
6. 修复 residue 后整个 case从新的 random ref重跑；不能只编辑 manifest把 cleanup改成 pass。

## 7. Windows `CredUIPromptForCredentialsW` secure-capture UAT

`secret:native:windows:uat` 只在 Windows x64 user-visible session、双 env gate下运行。必须走真实 FyAgent main window HWND与 production capture path，并证明：

1. exact API 是 `CredUIPromptForCredentialsW`，不调用 `CredUIPromptForWindowsCredentialsW`/`CredUnPackAuthenticationBufferW`。
2. parent HWND非 null且属于 FyAgent main window；取不到 HWND 时 fail closed，不弹 desktop-level dialog。
3. flags包含 `GENERIC_CREDENTIALS | ALWAYS_SHOW_UI | DO_NOT_PERSIST | EXCLUDE_CERTIFICATES | KEEP_USERNAME`；不含 `SHOW_SAVE_CHECK_BOX`、`PERSIST`、`EXPECT_CONFIRMATION`。
4. username固定且非敏感；password control masked；UI无 save checkbox。
5. success pass：operator输入 disposable value，native捕获后由相同 production service写入 random CredMan target并 read-verify；renderer/IPC只收到 non-sensitive summary。
6. cancel pass：再次打开并点击 Cancel；无 store/state成功副作用。
7. success/cancel 后查询固定 capture target，均必须 `ERROR_NOT_FOUND`，证明 Credential UI 没有自行持久化；真正存储只发生在随机 `FyAgent/secret/v1/<ref>` target。
8. username/password buffers在每条 return path zeroize；值不写 stdout、test report、screenshot、manifest。
9. dialog关闭后运行 artifact canary scan，再删除真实 entry并 readback missing。

UAT manifest含 human checklist/attestation，不要求拍摄含输入控件的 screenshot。若额外截图，它只能是单独 `runtime_screenshot` evidence，且必须在字段为空/对话框关闭后扫描；截图不能替代 UAT。

## 8. macOS native runtime 与 capture UAT

### 8.1 Real Keychain CRUD

`secret:native:macos:crud` 对当前 user default Keychain使用 production direct backend：

```text
SecAccessControl::create_with_protection(
  Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
  AccessControlOptions::empty().bits())
-> raw CFDictionary with exactly:
   class=genericPassword, service, account, synchronizable=false,
   access-control, value-data
-> one create-only security_framework_sys::keychain_item::SecItemAdd
-> query-only read + attribute readback
-> explicit SecItemUpdate-equivalent
   (search class/service/account/synchronizable=false; update data only)
-> query-only read + attribute readback
-> query-only delete
-> query-only read -> errSecItemNotFound
```

Fixed identity：

```text
service = com.fyagent.secrets.v1
account = generated SecretRef
```

Formal assertions：

1. create dictionary恰好六键：`kSecClass=kSecClassGenericPassword`、exact service/account、`kSecAttrSynchronizable=false`、由`AccessibleWhenUnlockedThisDeviceOnly + empty flags`生成的`kSecAttrAccessControl`、`kSecValueData`；没有return/label/auth-context/separate-accessibility/sync-any/default selector。Create只执行一次raw `SecItemAdd`且result pointer为null，不调用`set_generic_password_options`。Duplicate必须fail closed：fresh identity collision/drift才为`SECRET_BACKEND_CHANGED`，否则`SECRET_WRITE_FAILED`；两者`SecItemUpdate` call count都为0。
2. create后attribute readback为`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`且`kSecAttrSynchronizable=false`；first material read constant-time equal。
3. replace的search dictionary只含class/service/account/non-sync，update dictionary只含new data；两者都不含access-control/auth-context。Replace query/update not-found fail closed且`SecItemAdd` call count为0；replace material read constant-time equal且attributes仍逐字相同。
4. find/delete query只含class/service/account/non-sync（read可另加return-data）；access-control object从不作为caller authority、lookup或delete selector。
5. delete后`errSecItemNotFound`；all platform results携带backend/device generations，wrapper在material/receipt出界前完成store-instance/exact-Arc/generation recheck。
6. `errSecInteractionNotAllowed|errSecInteractionRequired`固定映射`SECRET_LOCKED + lockSource=backend + presence=unknown`。可重复真实locked fixture才另发`failure_path, origin=real_os`；否则只记录injected mapping，不冒充native locked run。

Random ref、runtime canary、finally cleanup、artifact scan与Windows规则相同。CRUD item标`native_runtime`；missing另发`failure_path` origin=`real_os`。Dictionary/accessibility assertions可由同一raw native trace支撑，但仍只是该exact source SHA的runtime evidence；本文没有执行它们。

### 8.2 `NSAlert + NSSecureTextField` UAT

`secret:native:macos:uat` 必须从真实 FyAgent desktop的 Tauri main thread运行：

1. `run_on_main_thread` 构造 `NSAlert`，唯一输入为 `NSSecureTextField`；无 renderer password input。
2. operator验证 masked secure field、localized Continue/Cancel、focus/keyboard、window-close cancel。
3. success只通过 native oneshot返回 `SecretMaterial`；main-thread closure不做 Keychain/DB/file I/O或 await。
4. cancel/window-close产生 stable cancelled且无 store/state成功副作用。
5. success 后真实 Keychain write/read verify，随后 artifact scan、delete、missing readback。
6. manifest只声称 app-controlled Rust buffer zeroized；不得声称可证明 Cocoa framework内部所有 temporary copies为零。

Keychain locked/interaction-not-allowed只有在可重复、可恢复的 dedicated fixture下才标 `evidence_class=failure_path, evidence_origin=real_os`；否则只保留 injected mapping，不冒充 native OS lock。

## 9. Evidence manifest schema

### 9.1 One class per evidence item

一个 run可以产生多个 evidence items，但每个 item只有一个 `evidence_class`：

```text
source_report
code_audit
ci_compile
unit_test
integration_test
native_contract
native_runtime
failure_path
uat
runtime_screenshot
artifact_scan
```

禁止使用 `native_runtime+failure_path`、`native/UAT` 等复合字符串。同一次 real missing call可产生两个引用同一 raw artifact的 items：一个 CRUD run的 `native_runtime` item和一个 `failure_path` item；interactive capture同理单独产生 `uat`。`failure_path` 必须再有 `evidence_origin=real_os|fault_injection`，不能从 job/host name推断。

### 9.2 Required JSON shape

```json
{
  "schema": "fyagent-secret-evidence/v1",
  "run_id": "uuid",
  "source": {
    "freeze_sha": "40-lowercase-hex",
    "remote_ref": "refs/heads/codex/issue-35-secret-backend",
    "remote_sha": "same-as-freeze-sha",
    "head_sha": "same-as-freeze-sha",
    "checkout_mode": "detached",
    "tracked_tree_clean_before": true,
    "tracked_tree_clean_after": true
  },
  "host": {
    "os": "windows|macos",
    "os_version": "non-sensitive-version",
    "arch": "x86_64|aarch64",
    "target_triple": "exact-triple",
    "runner_kind": "manual_user_visible|github_hosted",
    "interactive_session": true
  },
  "toolchain_gate": {
    "rustc_release": "1.85.0",
    "rustc_host": "matching-host-triple",
    "target": "same-matching-native-target",
    "cargo_lock_sha256": "64-lowercase-hex",
    "command": "cargo +1.85.0 check --locked --workspace --all-targets --manifest-path src-tauri/Cargo.toml --target <matching-native-triple>",
    "exit_code": 0,
    "raw_artifact_sha256": "64-lowercase-hex"
  },
  "task": {
    "name": "canonical-mise-task",
    "started_at": "RFC3339 UTC",
    "finished_at": "RFC3339 UTC",
    "exit_code": 0,
    "native_gate": true,
    "capture_uat_gate": false
  },
  "evidence": [
    {
      "case_id": "stable-case-id",
      "scenario": "stable-scenario-enum",
      "evidence_class": "native_runtime",
      "evidence_origin": "not_applicable",
      "platform_api": ["stable API names only"],
      "result": "pass|fail|blocked",
      "stable_error_code": null,
      "assertions": [{"name": "stable-assertion", "passed": true}],
      "fault": null,
      "ref_display_suffix": "last-4-only",
      "cleanup": {
        "attempted": true,
        "delete_result": "success",
        "readback": "missing",
        "residual_entries": 0,
        "recovery_ledger_removed": true
      }
    }
  ],
  "artifact_scan": {
    "scope_id": "codex_feature_runtime/v1",
    "enumerated_artifact_count": 1,
    "canary_match_count": 0,
    "unreadable_artifact_count": 0,
    "result": "pass"
  },
  "artifacts": [
    {
      "path": "relative-sanitized-path",
      "sha256": "artifact-sha256",
      "media_type": "application/json",
      "secret_scan_passed": true
    }
  ],
  "uat_attestation": null,
  "result": "pass|fail|blocked"
}
```

`toolchain_gate`是matching-host manifest的required object：`rustc_release`不是`1.85.0`、host/target不matching、Cargo.lock hash不等于same freeze checkout、command缺`--locked|--workspace|--all-targets`或exit非0都使整份manifest blocked；不得把1.97结果改写成1.85 receipt。Failure injector item的 `fault` 额外记录 stable `fault_id`、matched call ordinal、expected/actual call count；不记录伪造的 raw OS error。UAT item的 `uat_attestation` 记录 non-sensitive attestor id、observed checklist、accepted boolean和 timestamp。

manifest禁止包含：canary/material、credential blob、full target、full generated ref、argv/env secret、raw OS error text、Provider config、value-derived digest或 screenshot中的输入内容。`freeze_sha`和 sanitized artifact sha不是 secret digest，可以记录。

## 10. Final evidence index 与 DONE gate

`secret:evidence:verify` 对 sanitized manifests建立一个 source-SHA单一 index，并 fail closed检查：

1. 所有 required manifests的 `freeze_sha/head_sha/remote_sha` 等于同一 `FREEZE_SHA`。
2. CI run `headSha == FREEZE_SHA` 且 required gate success；CI classes不被重新标为 native/UAT。
3. Windows x64有 `native_runtime` real CredMan CRUD pass，且动态断言 `CRED_PERSIST_LOCAL_MACHINE`。
4. Windows x64有固定`failure_path` items `missing|locked|denied|backend_unavailable|verification_fail|rotation_old_delete_fail|capture_cancel`；每一项均`result=pass`，origin与上表一致。至少三个distinct scenario是附加计数，不是替代条件。
5. Windows x64有real interactive `CredUIPromptForCredentialsW` success与cancel各自`uat` item且`result=pass`；cancel raw artifact同时支持上一步独立的real-OS failure item。
6. macOS有real Keychain `native_runtime`，其assertions逐项通过：create/replace attributes=`AccessibleWhenUnlockedThisDeviceOnly + non-sync`、explicit data-only update保留policy、update/find/delete query没有access-control object、locked mapping精确；并有real `NSAlert/NSSecureTextField` success + cancel `uat`。
7. 每个创建过 real entry的 case均 `cleanup.readback=missing`、`residual_entries=0`、recovery ledger removed。
8. artifact scanner `canary_match_count=0` 且没有 unreadable required artifact。
9. ARM64只显示 `compile_only/runtime_not_accepted`，没有被计入 Windows runtime/failure/UAT gate。
10. 任何 `blocked`、mixed SHA、manual manifest edit、cleanup failure、missing raw artifact hash或 evidence-only commit越界都使 final verdict fail。
11. Matching native macOS与Windows x64各有same-SHA Rust 1.85.0 locked workspace all-targets leaf，toolchain release/host、target、Cargo.lock hash、command/exit/raw artifact hash完整；1.97、cross-host或普通backend check均不能替代。
12. Source registration evidence断言15个#35 `SecretCommandName`逐项registered/reachable，独立`resume_staged_import_cutover` main-integration handler另行registered/reachable且staged trace命中；任何15+1合并、missing/duplicate/extra均fail。
13. SOURCE_FREEZE portable source/integration evidence断言`LegacySourceCoverageReceipt`只有`CodexLegacySourceInventoryBridge`可构造authority并由`pub(crate) checked_from_complete_inventory_authority`消费的mint path，且`LegacySourceInventoryRevision + CompleteLegacySourceCoverageIdentity + currentScrubbable + adjacentBlocked`不可拆分。Missing receipt、stale revision、11域逐一omitted、duplicate/unknown、proof/data drift、presence/count mismatch、split/reconstructed receipt及aggregate-empty伪证必须在startup、summary/readiness、capture options/claim、Provider-delete preview/confirm全部effect-none；任一case、consumer或zero-call assertion缺失均fail。
14. Candidate discard/expiry source evidence断言`CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`分别mint/consume；missing固定`Validate`且只能消费durable `CandidateDiscardDeleteCheckpoint{deleteDisposition,backendCompletedAt,deleteAppliedCas}`。BackendApplied与MissingReadbackVerified crash fixture分别只恢复suffix，phase严格到Terminal、无StateFinalized/general recovery；任一slot/CAS/checkpoint/call-count assertion缺失均fail。
15. Activation source evidence断言`ActivationOldRecordDeleteCheckpoint|ActivationOldRecordDurableCheckpoint|RecoveryOldRecordDeleteCheckpoint`都保留exact disposition/time/CAS且只能checked reconstruct；process-local `ActivationOldRecordDeleteApplied`另持postcondition。Old-missing前不supersede，terminal `revokedAt`必须等于checkpoint backend completion time；仅CAS checkpoint或missing time均fail。
16. Staged resume evidence必须存在五个且仅五个canonical digest fixtures/crash cases：`staged_resume_intent_v1|staged_resume_sources_scrubbed_v1|staged_resume_cutover_committed_v1|staged_resume_live_owner_minted_v1|staged_resume_local_binding_finalized_v1`。每个编码same journal operationId、exact five-arm phase、累计required/forbidden rows与expected digest，并证明phase/fresh nonce/admission改变revision/digest、旧CAS零写；missing/duplicate fixture或generic checkpoint arm均fail。

状态规则是机械的：

```text
missing Windows x64 native_runtime
OR any fixed Windows failure_path item missing/not-pass
OR missing Windows interactive UAT
OR cleanup/artifact scan incomplete
OR complete eleven-domain coverage source/integration evidence missing/not-pass
OR ARR candidate/activation/staged phase source evidence missing/not-pass
=> ISSUE_35_STATUS != DONE
```

允许的状态只能是 `review`、`evidence_blocked` 或项目当前等价非终态。不得以 Windows CI compile/unit、ARM64 contract、macOS runtime、mock、screenshot、设计文档或人工口头说明替代缺失的 Windows x64 native/failure/UAT evidence。

## 11. Runtime readiness checklist（执行时）

- [ ] Design reviews exact-tree `P0=P1=P2=0`，DESIGN_FREEZE receipt存在。
- [ ] macOS四个exact direct pins的resolved lock rows/checksums与本文一致，license/advisory gate通过；任何lock drift已重新审查raw API shape。
- [ ] 所有 `secret:*` tasks、ignored gates、fault plan、manifest validator已在 `FREEZE_SHA`。
- [ ] 15个#35 commands与独立resume handler的same-SHA registration/reachability assertion通过。
- [ ] `LegacySourceCoverageReceipt`唯一bridge/unforgeable authority/`pub(crate)` factory、exact 11-domain identity与原子`currentScrubbable + adjacentBlocked` assertion通过；missing/stale/incomplete/逐域omitted/duplicate/unknown/proof-data drift/presence-count/split-reconstruction/aggregate-empty失败矩阵覆盖startup、summary/readiness、capture options/claim与Provider-delete preview/confirm，且effect/call-count为零。
- [ ] Candidate discard两slot/Validate/三字段checkpoint/BackendApplied→MissingReadbackVerified→Terminal crash matrix通过；operation-specific slots=12、delete/missing-specific=10、hardware operations=5、journals=8、recoveries=4。
- [ ] Activation normal/durable-failure/recovery三种role checkpoint逐字段保真、checked reconstruction及backendCompletedAt terminal timestamp matrix通过。
- [ ] 五个`staged_resume_*_v1` phase fixture/crash case齐全，operationId、累计required/forbidden rows、digest与fresh identity/phase revision变化全部通过。
- [ ] Windows classifier覆盖 secret platform/capture/harness paths。
- [ ] Windows x64 user-visible host已 provision；Credential Manager preflight可用。
- [ ] macOS user-visible host/default Keychain preflight可用。
- [ ] source-freeze commit clean，pre-evidence push获执行授权，remote readback exact。
- [ ] workflow-dispatch run唯一且 `headSha == FREEZE_SHA`；`CI / Required` success。
- [ ] manual hosts detached checkout exact SHA，worktree clean。
- [ ] macOS与Windows x64 matching native host各有Rust 1.85.0 `--locked --workspace --all-targets` pass leaf与exact Cargo.lock hash；1.97未被当作替代。
- [ ] Windows CRUD/LOCAL_MACHINE pass且cleanup missing。
- [ ] Windows formal failure matrix全部运行、origin正确、至少三类 pass。
- [ ] Windows CredUI success/cancel UAT pass。
- [ ] macOS Keychain create/update/read/delete/missing pass；raw six-key create/one `SecItemAdd`、duplicate-no-update、replace-not-found-no-create、accessibility/non-sync、data-only update、query-no-access-control与locked mapping assertions齐全；NSAlert success/cancel UAT pass。
- [ ] final scanner zero match/unreadable=0；所有 recovery ledger removed。
- [ ] sanitized evidence index verified；independent final reviewer read back exact manifests。

在以上 checklist 全部关闭前，`runtime-preflight` 应继续显示 Windows/manual evidence lane未闭环，任务不得进入 DONE。
