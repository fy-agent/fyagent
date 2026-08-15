# Issue #35 设计入口

## 权威顺序

1. `prd.md` — 产品边界、用户流程、状态机与验收。
2. `research/source-audit.md` — exact base、writer/冲突预算与当前明文路径。
3. `secret-contract-v1.md` — exact TS/Rust wire、native-only API、错误/状态/审计矩阵。
4. `device-local-secret-store.md` — no-v17 state/journal、crash reconcile、native store/capture与导入顺序。
5. `research/codex-secret-call-graph.md` — Codex closed graph、共享文件 owner、#55/#41兼容要求与scanner范围。
6. `research/os-keyring-options.md` — direct OS API与MSRV裁决。
7. `technical-design-overview.md` / `detailed-design-overview.md` — 架构索引、调用序列、文件和验证矩阵。
8. `execution-plan.md` — 阶段门禁、提交、source freeze 与 native closure。
9. `reviews/*.md` — 产品、架构、详细设计的独立静态评审。
10. `research/secretRef-contract-handoff.md` — 给 #55/#41 的合同草案；只有 freeze receipt 指定的 immutable SHA 可消费。

## 当前状态

- `DESIGN_FREEZE=PENDING`
- `P0/P1/P2 closure=PENDING`
- 当前阶段只允许静态阅读与设计文件写入。
- 在 freeze receipt 生成前禁止代码实现、dependency resolution、test、build、browser、renderer、server、native runtime 或截图。

## 采用方案摘要

采用“device-local secret authority + staged candidate + 显式 platform backend + native secure capture”：

- #35 不新增 SQLite schema、不占 v17；record/ref/binding/candidate/journal/audit只存在于 `app_local_data_dir/device-local/secrets/v1`，Provider row仅保存无值配置。
- durable `DeviceInstanceId=dev_*` 与每次打开 store 重新生成的 process-local `DeviceSecretStoreInstanceId` 是两种身份：前者进入 state/journal/backend record，后者禁止持久化/Serde/Clone并只密封当前进程的 opened-store scope；live handle同时校验二者与exact registered backend object。
- OS keyring backend 为 macOS Keychain / Windows Credential Manager；Linux fail closed 为 unavailable。
- renderer 不提供 API-key 文本框。backend-option command在native读取current owner/binding/legacy snapshot并mint单次`SecretCaptureIntentId`；renderer选择backend后只提交intent id与backend id，实际值由OS原生secure control捕获，不能自造binding authority。
- `BackendOperationBroker` 是有状态的唯一 registry owner，私有持有capture-intent/capability/pending-confirmation registries并由`SecretService`持有同一`Arc`；list→mint、begin→claim+fresh revalidate和cancel/error→terminalize都不能退化为全局变量或renderer重算。
- capture/migrate/rotate先生成 verified candidate；只有#55 admitted immutable plan可切 binding/scrub。
- activation projection冻结`candidateEquality|explicitReplacement`；自动同值迁移必须compare，用户确认的conflict replace/reconcile/rotate按exact source set/revision替换，不要求旧值等于新candidate。
- material 只在 material-free one-shot capability完成writer内重检后由 native `SecretService::resolve_for_apply` 交给sealed闭包，类型不可 Serialize/Clone/Debug，Drop 时 zeroize。
- hardware backend 在同一 port 上保留 confirmation/device/disk/revocation 差异，不允许 software fallback；capability使用五种operation policy，fresh missing-readback复用`validate` confirmation policy但仍拥有独立authorization与durable checkpoint。所有record/pending/authorization/receipt绑定durable device、lifetime device-store instance与exact registered object。普通read/probe的revocation hint不可持久化，只有显式Revoke authorization驱动的`observe_revocation_once`可mint完整CAS-bound consuming receipt。
- #55 消费 apply readiness/plan projection；#41 Configuration Apply 在 Provider lease 前消费 `prepare-for-apply`，并在既有 writer 内消费一次性 capability。二者都不自建 secret storage。
- startup唯一顺序为opened store → no-backup DB preflight → same AppState/SecretService reconcile（含current-scrubbable refs与adjacent-blocked env/common-config observations的无值coverage receipt）→ app.manage/static registration（15个#35 commands + 1个main-integration staged-resume handler）→ Clean sanitized backup → publish gate → workers。Coverage receipt是`pub(crate)`可命名但不可伪造的opaque authority，只有named main-integration inventory bridge可mint；它绑定非值派生inventory revision与完整11域structural revision/presence/count proof，空集合缺任一域仍Blocked。Blocked保留同一managed state但不启动backup/worker/consumer。
- staged import唯一顺序为temp authority/token+projection → #55 admission → authority-match receipt → #35 prepare/confirm → cutover context；prepared failure显式discard并终结admission。public resume只接受`stageId + expectedResumeCas{revision,digest}`，并通过独立closed result在每个arm只返回`stageId + currentResumeCas + status/action/issue`，绝不返回candidate/owner/ref/summary。CAS preimage绑定operationId与exact五阶段`intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized`，每个phase/nonce/admission变化都递增revision并重算digest；fresh process/admission生成新CAS，cutover后才转换为live owner。
- public recovery是按kind判别的同一impact/retry合同，覆盖activation、capture compensation、delete finalization与owner detach；device-local state/CAS逐kind镜像。每个delete与fresh missing readback使用独立authorization并在中间durable checkpoint；checkpoint原子保留`deleteDisposition + backendCompletedAt + deleteAppliedCas`，rotation supersession只能在missing receipt后terminal且其revokedAt取backendCompletedAt。candidate explicit discard/expiry虽不新增recovery kind，也必须准备独立delete/missing slots，以同一三字段checkpoint解锁Validate missing-readback；保留immutable pending disposition直到confirmed missing后terminal。terminal expiry先refresh current summary再mint全新capture/rotation flow。
- Provider delete把binding与legacy source正交读取：legacy存在时no-impact-id阻断并要求先处理；只有no-legacy preview才能确认detach，backend secret删除永远是另一操作。
- 一次性readiness终止后只能进入fresh impact/plan/staged/capture-intent flow，不能把已消费operationId包装成无command的generic retry；renderer可见action都有closed destination。Codex环境/common-config/public Provider/request override/stream diagnostic必须进入第一切片闭环；Codex MCP env/header作为命名Level-3 debt，不计主凭据PASS。

被排除方案：

1. SQLite secret identity/binding表或 encrypted SQLite vault：会让本机 authority进入数据库备份/同步，并把 schema冲突、master-key/backup/rotation问题带回应用层。
2. `keyring` all-in-one facade 直接散落在 provider code：无法冻结硬件差异、错误语义、审计和 no-fallback。
3. renderer password input + IPC write command：违反本任务明确的 UI/IPC 无值边界。
4. 从环境变量或 live config 静默回退：无法证明 owner 和 material 是同一凭据，且绕过生命周期状态。
