# Issue #35 独立架构评审

## 评审结论

- `ARCHITECTURE_REVIEW=REQUEST_CHANGES`
- Open findings: `P0=0`, `P1=8`, `P2=4`, `P3=0`
- 证据级别：`source_report + code_audit + static_architecture_review`。本评审未运行 test、build、browser、renderer、server、native runtime、dependency resolution 或 screenshot，不能作为实现或运行验收证据。
- 评审对象：当前工作树的 `prd.md`、`design.md`、`technical-design-overview.md`、`detailed-design-overview.md`、`execution-plan.md`、`implement.md`、全部 `research/*.md`，以及 base `afc317a7...` 的相关 Rust/TypeScript/schema/provider 静态源码；兼容性核对还读取了本地可用的 #55 remote-tracking tree `6859e9ce` 与已冻结 Prompt/Memory native design tree `e12f07a2`，未 fetch、未修改这些树。
- 通过条件：以下所有 P0/P1/P2 关闭后，由独立 architecture reviewer 对最终精确工作树重新完整读取，才可记录 `ARCHITECTURE_REVIEW=PASS`。

## Findings

### AR-001 — P1 — Keyring 外部副作用与 SQLite 之间没有 crash-consistent operation journal

- 状态：OPEN
- 证据：`technical-design-overview.md` §6 只有 `secret_records`、`secret_owner_bindings`、`secret_audit_events`；§8 Capture 在 backend write/verify 之后才开启 record/binding transaction，且只在“已观察到 DB failure 且 compensation delete 也失败”后写未定义的 recovery marker；Rotation 同样在新 entry 写入后才落 DB。`detailed-design-overview.md` §4 的 repository API 没有 operation/phase/recovery 记录。进程可在 backend write 成功、DB commit 前退出，此时既不会执行 compensation，也没有持久 ref/phase 可供重启恢复。
- 影响：capture、legacy migration、rotation 都能留下无法枚举/无法归属的 keyring orphan；delete 也可能在 backend 已删、DB 仍 active 时丢失用户意图。SQLite transaction 和事后 compensation 不能覆盖进程终止、断电或 response-loss，这与可审计生命周期、可重试清理和原子轮换声明冲突。
- 必须闭环：在任何 backend mutation 前持久化不含 material 的 operation intent，冻结 exact phase/state machine（至少 intent、backend-applied、DB-finalized、compensating、recovery-required、terminal）、operation/ref/backend/owner/revision 与幂等规则；每个 phase 都要有重启 reconciliation。可用 SQLite operation table 或固定 app-data journal，但必须先 durable 再做外部副作用，并定义 journal/DB 双写顺序、fsync/readback、清理和 unknown outcome。Capture/migrate/rotate/delete 的 crash matrix 与 retry identity 必须进入 schema、DTO、owner map、failure matrix。

### AR-002 — P1 — 设备本地 secret records/bindings 会被现有 WebDAV/S3 数据库同步覆盖

- 状态：OPEN
- 证据：`prd.md` §3.2 明确跨设备同步非目标，`research/os-keyring-options.md` 又选择 non-sync macOS Keychain 和 Windows `LOCAL_MACHINE`；但当前 `src-tauri/src/database/backup.rs:74-90` 的 `SYNC_SKIP_TABLES` / `SYNC_PRESERVE_TABLES` 不含任何 secret table，`export_sql_string_for_sync`/`import_sql_string_for_sync` 在同文件 `108-147` 会同步其 rows。`detailed-design-overview.md` §1 没有把 `database/backup.rs` 或 sync contract 分配给任何 owner。
- 影响：另一设备的随机 ref/binding 会替换本机 binding，本机 OS entry 成为 orphan，远端 ref 在本机变成 missing；audit/recovery journal 也可能跨设备混合。No-fallback 能阻止误用值，却不能阻止本机凭据元数据被静默夺走。
- 必须闭环：明确 secret records、bindings、audit、operation journal 全部是 device-local authority；把它们按 FK 顺序加入 sync skip/preserve 合同并增加双设备 round-trip 设计用例，远端 provider rows 导入后继续绑定本机 owner/ref。若任何 metadata 允许同步，必须另行设计 device identity、per-device binding 和 reconcile UX，不能复用当前单表。同步涉及文件须加入 owner map/conflict budget。

### AR-003 — P1 — 热 import/sync/backup restore 可重新引入 legacy plaintext，却不会经过 startup migration gate

- 状态：OPEN
- 证据：`detailed-design-overview.md` §7 只在启动时、`app.manage` 前运行 legacy migration；当前 `src-tauri/src/database/backup.rs:125-211` 会把 SQL 在 temp DB 迁移 schema 后直接替换 main DB，`src-tauri/src/commands/import_export.rs:40-59` 随即运行 post-import live sync，`63-68` 还能用新建 `AppState` 直接同步 live。二进制 backup restore 同样只跑 schema migration。`technical-design-overview.md` §9 的“import 拒绝 inline”没有区分 Provider import、完整 SQL import、WebDAV/S3 import和 binary restore，也没有定义这些 hot path 如何刷新 migration authority。
- 影响：旧同步快照或 v16 backup 可在 app 已启动后把 Codex 明文重新写回 `providers.settings_config`；startup migration 不会重跑，旧 migration status/SecretService cache 也可能继续被信任。即使新 writer 最终 fail closed，DB/backup 已再次持有 plaintext；若 post-import path 尚有遗漏，还可能立即投影到 live。
- 必须闭环：为完整 SQL import、sync import、binary restore 分别冻结 staged preflight 和 cutover 顺序：识别 legacy inline/remote refs，完成或明确阻断 secret migration/rebind，刷新 SecretService/migration authority，然后才允许 main DB replacement 后的 provider IPC/live sync。Locked/denied/backend unavailable 必须返回 partial/blocked 且 `effect=none`，不能继续 post-sync；恢复旧 backup 后的 ref probe/reconcile 也要定义。把 `database/backup.rs`、sync protocol、restore command 与相关 tests 纳入 owner map。

### AR-004 — P1 — #55 的实际落地树仍对 secret-bearing Provider/live projection 求摘要，handoff 与 source audit 已失真

- 状态：OPEN
- 证据：`research/source-audit.md:9-12` 把 #55 remote 记录为 `4bfee69c...`，而评审时本地 `refs/remotes/origin/codex/unified-change-plan-codex-switch` 已指向 `6859e9ce`。该树 `src-tauri/src/change_plan.rs:275-277` 直接序列化完整 `Provider` 求 definition digest，`301-359` 对 `read_live_settings(Codex)` 与 effective target 求 digest，当前两者都可能含 API key；`454-474` 直接调用现有 `ProviderService::switch`。这与 `research/secretRef-contract-handoff.md:25-31` “plan/receipt/audit 不得含 value-derived digest”及 `technical-design-overview.md` §11 的 ref/capability plan 合同冲突。
- 影响：#35 即使自身不落值，#55 的 baseline/plan/job 仍会持久化 value-derived fingerprint；existing writer 也没有 consume one-shot capability 的入口。基于旧 planning SHA 的 conflict budget 无法证明当前 consumer 可合并。
- 必须闭环：先选择并记录要兼容的 immutable #55 SHA/merge-base，更新 source audit 与 exact name-status/conflict map；冻结 #55 adapter 改造：Provider definition/live projection 先做结构脱敏，plan 只绑定 owner/ref/sink/capability contract revision，不计算 material 或 value-derived digest；目标 live token 的正确性只能在 native resolve/write/readback 闭包中得到 boolean/stable code，不能持久化 fingerprint。对 `6859e9ce` 或后继选定 SHA 做 draft compatibility readback 后才允许 #35 freeze。

### AR-005 — P1 — v17 与 shared writer 文件存在已知双重所有权，当前 conflict budget 不可执行

- 状态：OPEN
- 证据：`research/source-audit.md:31-35` 说 #55 拥有 provider commands/service、#41 拥有 Provider lease/backup/readback/recovery；`detailed-design-overview.md:40-60` 又把 `commands/mod.rs`、`store.rs`、`lib.rs`、`provider.rs`、`codex_config.rs`、`services/provider/{mod,live,usage}.rs`、`commands/provider.rs` 全部分给 #35 writer，且 §6 声称改变 existing writer 顺序。另一方面，已冻结 tree `e12f07a2` 的 `.trellis/tasks/08-13-prompt-memory-native-integration/design.md` §12.1 明确占用 SQLite v17，`detailed-design.md` §2.1 又把 `Cargo*`、`lib.rs`、`database/{mod,schema}.rs` 锁给 MainIntegrationOwner；#35 `technical-design-overview.md:239-256` 同样单方面占用 v17。`research/source-audit.md` 只列 Prompt/Memory V2 shell，漏掉 native/v17 lane。
- 影响：并行 worker 无法遵守“一文件一 owner”；两个已经冻结的 v16→v17 migration 不能同时成立，任何一方先落地都会让另一方 migration number、rollback gate、schema tests 和 source SHA 失效。#41/#55 与 #35 也会在 writer/command/schema 入口发生实质覆盖。
- 必须闭环：freeze 前完成跨任务 adjudication，而不是把问题推迟到 source-freeze：选择唯一 integration base、migration numbering/组合方式、落地顺序和 dependency SHA；更新 #35、Prompt/Memory、#55、#41 的 task artifact/owner map/rollback plan。#35 应只拥有 secret module 与明确 seam；Provider lease/writer ordering、Change Plan ledger、shared registration 各保留单一 owner，并用 exact trait/function handoff 接入。未获得各 lane readback 前 v17 reservation 与 DESIGN_FREEZE 必须保持 blocked。

### AR-006 — P1 — PreparedSecretCapability 没有绑定 lifecycle/binding revision，无法保证撤销与并发 fail-closed

- 状态：OPEN
- 证据：`technical-design-overview.md:104-120` 的 exact public contract 仍使用过时 consumer `contextControlledUse`；`145-179` 引用未定义的 `OperationId`、`SecretConsumer`、`HardwareConfirmationReceipt`，`PreparedSecretCapability` 只是注释并允许持有 “material or backend lease”。它只声明 operation/ref/sink/expiry/consume-once，未绑定 record `updated_at`、owner binding revision、backend generation 或 lifecycle。`research/secretRef-contract-handoff.md:40-43` 让 capability 跨越 Provider lease、baseline recheck 和 backup；这期间 lock/rotate/delete 可并发发生。
- 影响：prepare 后被逻辑锁定、轮换、删除/撤销的旧 ref 仍可能被 capability 写入 target；若 capability 提前持有 material，真实值还会在 writer closure 外等待 lease/backup，违反 PRD 的最短驻留边界。过时 consumer 和未定义 receipt 也让 #41 无法实现唯一时序。
- 必须闭环：给出可编译的 exact SecretService/backend API 与 lock order。Capability 推荐只持 opaque backend authorization/nonce而不持 material；无论实现为何，都必须由 native 生成 operation identity，绑定 owner/ref/sink/consumer、record revision、binding-set revision、backend instance/generation、expiry，并以原子 single-consume CAS 管理。#41 取得 Provider lease 后、首次 target mutation 前，SecretService 必须重新验证 binding 仍指向该 ref、record active/unlocked、capability revision 未变；rotate/delete/lock 自动使旧 capability 稳定失败并 `effect=none`。同步删除 `contextControlledUse` 和 stale confirmation receipt 形态。

### AR-007 — P1 — Provider/public IPC trust boundary 仍靠调用约定，且 Codex first-slice 调用图没有完整 owner

- 状态：OPEN
- 证据：`detailed-design-overview.md:195-206` 保留可 `Serialize` 的内部 `Provider`，只要求各 command 手动调用 `provider_public_projection`，没有冻结独立 `PublicProvider` DTO。当前 `src-tauri/src/provider.rs:9-15` 的 `Provider.settings_config: Value` 可直接序列化，`src-tauri/src/commands/provider.rs:40-48` 原样返回它，`923-958` 还接受 `apiKey/accessToken` 并返回 raw live settings；`src-tauri/src/commands/model_fetch.rs:94-112` 接收 `api_key: String`。`research/secret-surface-inventory.md:7-17` 已列出 model fetch、usage、UniversalProvider、export/backup/deeplink 等必闭环入口，但 `detailed-design-overview.md` §1 owner map没有 `commands/model_fetch.rs`、`database/backup.rs`、sync/restore、deeplink 入口及相应 frontend/API files。Exact IPC 列表还引用未定义的 `SecretDeleteImpact`、`SecretMigrationReport`、`SecretAuditPage`。
- 影响：任一漏调 projection 的 command、新增返回点或 shared legacy API 都能把值重新送进 renderer；scanner 只能发现部分名称，不能替代编译期边界。实现 worker 也没有权限修改 inventory 要求关闭的所有路径，验收域无法闭合。
- 必须闭环：冻结独立、不可承载已知 secret fields 的 `PublicProvider`/Codex projection DTO，并让所有 renderer-facing Codex command 在类型签名上只能返回该 DTO；raw live read 对 Codex 必须禁用或返回结构脱敏 projection。逐项把 inventory 变成 call-graph→file→owner→test matrix，包含 model fetch、usage/balance/test script、UniversalProvider Codex conversion、deep-link/import、backup/sync/diagnostic；共享非 Codex API 可保留，但 Codex branch 必须 owner/ref controlled-resolve 或明确拒绝。补齐所有 request/result/filter/page/audit/migration DTO 的 Rust+TS exact schema。

### AR-008 — P1 — Legacy “existing binding + inline unknown” 会在未证明相等时删除凭据

- 状态：OPEN
- 证据：`detailed-design-overview.md:228-247` 把 `existing binding + inline same/unknown` 合并为“binding probe 后 scrub-only”。Probe 只能证明 entry presence，无法证明 keyring material 与 inline value 相同；locked/denied 时尤其无法比较。`technical-design-overview.md` §9 只定义两个 inline 位置彼此不同的 conflict，没有定义 binding=A、inline=B 或无法 resolve 的状态。
- 影响：存在绑定 A 而 inline 遗留/新写 B 时，scrub-only 会不可逆删除 B，或让用户在不知情下继续使用 A；这违反“失败保留原数据”“不猜优先级”和 migration idempotency。
- 必须闭环：只有成功 resolve existing binding 并 constant-time 证明与每个 inline source 相等时才允许 scrub-only。不同值返回显式 conflict；locked/denied/unavailable 返回 pending/unknown，内部 plaintext 保留、public projection 继续脱敏。冻结无值 reconcile/replace 流程、重试 identity、最终 scrub 条件以及 hot import/restore 下的相同规则。

### AR-009 — P2 — dependency count 不是 binding-set CAS，不能保护 rotate/delete/lock 的预览

- 状态：OPEN
- 证据：`technical-design-overview.md:310-320` 和 `detailed-design-overview.md:156-174` 只用 dependency count/`expected_updated_at`，并声称可防止 changed binding；public rotate/lock 命令甚至没有 expected impact 参数。相同 count 下 owner 集合或某个 owner→ref 关系仍可改变，新增与移除也可能在两步之间抵消。
- 影响：用户确认的是一组依赖，执行时却可影响另一组 owner；shared ref 的 rotate/delete/lock 结果不可归因，且 #55 plan baseline 无法精确判断 dependency drift。
- 必须闭环：为 ref 维护单调 binding-set revision，或对排序后的 `(owner kind, namespace, ownerId, slot, bindingRevision)` 生成非 material-derived digest；impact DTO 返回 revision/digest，mutation transaction 以 exact expected revision/digest + affected-row count 做 CAS。Rotate、delete、lock 都要使用同一机制并明确 shared-ref 用户确认。

### AR-010 — P2 — SQLite SecretRef CHECK 不实现“32 位 lowercase UUIDv4 hex”合同

- 状态：OPEN
- 证据：`technical-design-overview.md:243-256` 使用 `secret_ref GLOB 'sec_[0-9a-f]*' AND length=36`。SQLite GLOB 的 `*` 是任意字符串，不是重复前一个字符类；该表达式只约束 `sec_` 后第一个字符属于 `[0-9a-f]`，余下字符可为任意值，也未约束 UUID version nibble=`4` 和 variant=`8|9|a|b`。完整 SQL import 会绕过 Rust constructor，因此不能只依赖 `SecretRef::parse`。
- 影响：导入、migration bug 或直接 DB 写可形成无法被 service 解析、却被 FK/binding 接受的记录，破坏 list/migrate/rotate/recovery 的 totality。
- 必须闭环：冻结与 Rust parser 等价的 DDL invariant（prefix、length、每一位 lowercase hex、v4/variant），或使用受约束 binary UUID representation；为 malformed SQL import、uppercase、非 v4、非法尾字符增加 migration/restore fail-closed 设计用例。Owner namespace/ID 的长度和合法字符也应在同一 schema contract 中明确，而非只写“service validates”。

### AR-011 — P2 — Windows native capture 设计混用了两套 CredUI API/flag/buffer 合同

- 状态：OPEN
- 证据：`detailed-design-overview.md:143-148` 同时指定 `CREDUI_FLAGS_GENERIC_CREDENTIALS`/`ALWAYS_SHOW_UI`（`CredUIPromptForCredentialsW` 族）和“unpacked buffers”（通常对应 `CredUIPromptForWindowsCredentialsW` + `CredUnPackAuthenticationBufferW`/`CREDUIWIN_*` 族），但未命名 exact API、owner HWND、线程/返回通道、buffer allocator/free/zero 顺序。`research/os-keyring-options.md` 也只写“Credential UI with no-persist flags”。
- 影响：实现者可选出不匹配的 flags/返回类型，造成保存 checkbox、错误 buffer 释放、dialog 无父窗口/藏在后台，或无法证明 native capture 的 zeroize/UAT 边界。
- 必须闭环：在 freeze 前选择一套 exact Win32 API，列出 flags、父 HWND、username/password 输出形态、最大长度、allocator ownership、`SecureZeroMemory`/free 顺序、cancel/error mapping、Tauri main/UI thread 与 async response channel，以及 Cargo `windows` features。macOS 同样应给出 main-thread dispatch/return channel，但不需要本阶段运行验证。

### AR-012 — P2 — `hardware` singleton/static capability 不能表达真正的 per-device pluggable backend

- 状态：OPEN
- 证据：`technical-design-overview.md` §2 只有 `SecretBackendId = osKeyring | hardware`，§3 的 `capabilities(&self)` 是 backend-wide static 值；`detailed-design-overview.md` §2 要求 registry 只注册一个 `hardware` adapter。Schema 只保存 backend enum，没有 backend instance、opaque non-sensitive locator、device binding generation；`HardwareConfirmStep` 也没有非敏感 device identity。与此同时 PRD 把 device binding、不同 physical-confirmation、central revocation 与 no-disk 作为未来硬件合同的核心。
- 影响：两个硬件设备/插件、同 backend 内不同 credential capability、设备更换或 backend generation 变化都无法被稳定寻址和纳入 capability/replay CAS；“device mismatch”只能靠未定义的 singleton 内部状态猜测。静态 bool 也无法证明某条 secret 的实际 projection/revocation 语义。
- 必须闭环：二选一并明确写入成功边界：若 v1 只支持进程内唯一配置的 hardware adapter，声明 singleton 限制并禁止把它称为通用可插拔合同；若要冻结未来可插拔合同，则增加 stable backend instance/opaque locator、per-secret capabilities/device generation、非敏感 device display，以及 registry lookup/rotation/rebind 规则。所有变化仍须 exact backend lookup，绝不引入 fallback。

## 覆盖结论

- Trust/data boundary：renderer 无值、native-only resolve、exact backend lookup 方向成立；AR-002、AR-003、AR-007 说明 sync/import/public projection 仍有跨边界缺口。
- SQLite v17/repository：metadata-only schema 方向成立；AR-001、AR-005、AR-009、AR-010 阻止 migration/transaction/concurrency freeze。
- Backend/native capture：platform store 分层和 no-fallback 方向成立；AR-006、AR-011、AR-012 说明 capability replay、Win32 feasibility、hardware instance 仍未形成 exact port。
- Legacy/provider integration：先写入验证、再 scrub 的主方向成立；AR-003、AR-004、AR-007、AR-008 说明 hot restore、#55 digest、公共 DTO 与 unknown comparison 尚未闭环。
- #55/#41 compatibility：draft handoff 标记正确，但 AR-004、AR-005、AR-006 必须在 immutable handoff 前关闭；目前不能把“未来会 read back”当作兼容证据。
- Rollback/conflict budget：v16 binary too-new gate 可阻止静默 downgrade，但 AR-001/AR-005 表明 operation recovery 与 migration/file ownership 仍不足；现阶段不得写 DESIGN_FREEZE receipt。

## Gate

`ARCHITECTURE_REVIEW=REQUEST_CHANGES`

只有 `P0=0, P1=0, P2=0` 且 reviewer 对修订后的最终精确工作树完成重新读取，才允许改为 `ARCHITECTURE_REVIEW=PASS`。
