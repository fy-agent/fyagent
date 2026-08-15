# Issue #35 凭据引用与本机/硬件可插拔后端 PRD

## 1. 产品结论

FyAgent 的 Codex Provider 不再把主凭据当作 Provider 配置数据复制。FyAgent-owned authority/storage只把真实材料放入当前设备的OS安全存储；唯一持久化例外是用户批准的immutable apply plan点名、且OS-keyring record capability明确允许的exact外部live sink。FyAgent以不可推导的 `secretRef`、本机 owner binding 和无值状态解释“谁在用、现在能否使用、下一步做什么”。未来硬件后端仍保留物理确认、设备绑定、不可落盘和不可静默回退；`persistentTargetProjection=false`时没有例外。

## 2. 用户问题与决策影响

当前 Codex API key 可能沿 `settingsConfig`、live config backfill、proxy、usage/model fetch、SQLite、备份、同步、诊断或 renderer DTO 形成多份副本。用户无法知道：

- 引用是否真的对应可用凭据；
- 缺失、逻辑锁定、系统锁定、权限拒绝、撤销分别该怎么处理；
- 轮换或删除会影响哪些 Provider；
- apply、proxy 或 usage 是否在错误时偷偷使用旧字段、另一 backend 或 failover Provider。

本任务把这些行为改成显式合同：

1. Provider SQLite row 只保留无值配置；`secretRef`、binding、candidate、journal 与 audit 是当前设备的 local authority，不参与 WebDAV/S3/SQL 备份同步。
2. renderer、公开 IPC、plan/job/event/log/diagnostic/fixture/screenshot 不接触 secret value 或 value-derived digest。
3. native consumer 只有在一次性、无 material 的 capability 被重新验证后，才在最终使用边界短时取得材料。
4. 缺失、锁定、拒绝、撤销、硬件确认和设备不匹配均 fail closed；不读 legacy fallback，不换 backend，不因 secret failure 推进 proxy failover。

## 3. 本轮边界与可陈述证据

### 3.1 MVP

- `secret-contract/v1`：严格标识、ref/owner aggregate、presence、stable availability、lifecycle、binding-set CAS、candidate、migration、audit、错误和 action。
- device-local store：`app_local_data_dir/device-local/secrets/v1` 下的 record/binding/candidate/journal/audit compiled truth；不新增 SQLite schema，不占 v17。
- macOS：直接调用 SecurityFramework Keychain；Windows：直接调用 Credential Manager；两端均用 OS 原生 secure capture。
- Codex Provider 第一切片：create/edit/replace、legacy reconcile、轮换、逻辑锁定、删除/撤销、Change Plan apply、proxy、fixed usage/balance/model-fetch/coding-plan primary-key adapter，以及关联的 public projection/import/backup/sync/diagnostic 闭环。
- V2 Models/Credentials 高保真 UI；browser fixture 仅含无值状态，native adapter 仅传合同 DTO。
- 四级 secret scanner 与 macOS/Windows native/failure evidence。

### 3.2 Owner 边界

- v1 runtime 只接受 `provider/codex + codexApiKey + primaryApiKey`。
- `owner.kind=agent` 仅为 wire-reserved discriminant；所有具体 Agent 请求稳定返回 `SECRET_OWNER_KIND_UNSUPPORTED, effect=none`。
- 不承诺本轮 Agent create/query/rotate，也不假造 Agent material consumer。

### 3.3 可声明范围

最终只允许声明：

- `contract_schema`: #35/#55/#41/Codex public contract 没有 forbidden material fields；
- `codex_feature_runtime`: 已枚举的 Codex 调用图与 artifact set 通过 canary 运行扫描。

WebDAV/S3 密码、OAuth manager、非 Codex Provider、Codex MCP 的独立 `env/http_headers` credential，以及 ZenMux手填key/Volc AK-SK/team-login 等**独立** coding-plan/login credential 是公开的 pre-existing debt；它们进入 repository inventory/no-regression，但阻止 `repository-global secret-free` 声明。Codex MCP debt固定名为`codexMcpEnvOrHeaderCredential`，必须枚举DB/live/export/sync与fixture occurrence，但不计入Provider-primary Level 2 PASS。Codex Provider primary API key 进入 fixed coding-plan adapter时使用独立的`codingPlanUsageProbe` consumer（产品/证据分类仍属于`usageProbe/codex_feature_runtime`），不能借“coding-plan debt”排除，也不能与generic `UsageProbeKind=Usage|Balance`混用。

### 3.4 非目标

- 云托管、跨设备 secret 同步、团队共享、自建加密 vault、重型 KMS。
- 本轮实现硬件厂商 SDK、远程审批或真实 hardware adapter。
- 迁移所有 Provider 或改变官方 Codex OAuth 所有权；OAuth 保留在其既有 authority，不复制进 FyAgent。
- 让 Codex 主凭据进入 user-authored JavaScript usage script、Provider terminal env/temp file 或 deep link。
- 合 main、部署，或把生成图/静态设计当 runtime evidence。

## 4. 核心产品合同

### 4.1 SecretRef 与本机 owner binding

- `SecretRef = sec_ + lowercase UUIDv4 simple hex`，随机生成且不能从 owner、backend、value 或 digest 推导。
- 一个 ref 是一个逻辑 secret；轮换生成新 ref。
- 一个 ref 可绑定多个 owner；同一 owner/slot 只能绑定一个 ref。
- Provider row 不保存 ref。UI 查询时把 Provider id 与 device-local owner projection join。
- full ref 是非敏感 contract identity；普通 UI 只显示 `sec_…` + 后四位，日志/audit 使用 display 或稳定 event id。

### 4.2 Stable summary 与 operation state

稳定 ref availability 只有：

`ready | missing | locked | denied | stale | revoked | unavailable`

其中：

- `presence = present | missing | unknown`；锁定、拒绝、不可用不得假称 missing。
- `locked` 必须带 `lockSource=fyAgentPolicy | backend`，动作分别是 FyAgent unlock 或系统/backend unlock。
- `revoked` 必须带非敏感 source/time/action；用户确认删除、中心/backend 撤销和设备管理撤销均与 accidental missing 区分。
- migration 是 owner-level state，不塞进 ref availability。
- `confirmationRequired` 只属于一次 readiness/prepare operation；稳定 summary、列表缓存和 device-local state 不保存 step 或 capability。
- `SECRET_OPERATION_RECOVERY_REQUIRED` 必须带按kind判别的recovery pointer；v1至少覆盖 `activationCleanup | captureCompensation | deleteFinalization | ownerDetachFinalization`。每个kind都有同一公开impact/retry入口下的可执行分支，startup自动重试不能替代用户可见出口。
- 唯一无pointer例外是仍可通过candidate id到达的discard journal；summary必须同时给出immutable `pendingTerminalDisposition=discarded|expired`，terminal后该字段消失。
- 每个condition/source/recovery kind只映射一个closed action，且action必须指向已注册command、精确的main-integration flow或明确的外部指导；禁止同一错误写“按source任选动作”、使用没有command/route的generic `retry`，或把未注册hardware变成无尽循环。capture/new/replace/legacy conflict统一进入typed capture flow：native先读取当前owner/binding/legacy snapshot并mint单次短期`SecretCaptureIntentId`，renderer选择已注册backend后只回传intent id与backend id。已经终止的一次性readiness不能返回“重复原operationId”：delete重建delete impact，recovery重建recovery impact，apply/activation重开Change Plan，staged import恢复其exact CAS flow。

### 4.3 Capability 与 hardware 差异

capability 按 backend instance 和 record 冻结：instance/generation、device-binding generation、capability revision、per-operation confirmation、allowed consumer/sink、storage residency、persistent projection、central revocation、`silentFallback=false`。

MVP 未注册 hardware adapter 时：

- Add/Replace 不显示 hardware 选项；
- 已有/imported hardware binding 显示 unavailable/device mismatch；
- 永不回退 OS keyring；
- future device display 与 timeout 只出现在 operation-scoped confirmation。

只有exact registered backend handle在消费显式`Revoke` operation authorization、验证instance/generation与`centralRevocation=true + SourceAndTime`后，才能执行`observe_revocation_once`并返回不可clone、一次消费且绑定lifetime device-store instance、registered backend object、ref/store/record/binding-set/backend/device/capability CAS的revocation receipt；持久化前还要fresh revalidate同一authority snapshot。普通read/probe最多返回不可持久化hint，不能mint authority receipt或绕过hardware revoke confirmation。所有backend record/scope/receipt都绑定同一store instance与registered object，platform返回的backend/device generation在材料或receipt出界前复核。OS keyring capability固定`centralRevocation=false`。missing/locked/denied/unavailable、caller-supplied ref或仅有source/time的自由对象不能推断或移植revocation。

### 4.4 Destructive CAS

rotate、lock、delete 的 preview 返回完整 affected owners、每条 binding revision 以及 binding-set `{revision,digest,count}`。执行时在同一 mutation critical section 比对 revision + digest + exact sorted rows；只比较 count 不可授权。任何 drift 都是 `SECRET_DEPENDENCY_CHANGED, effect=none`。

## 5. 用户流程

### 5.1 Capture / create / replace

1. renderer先以owner、purpose与intent请求backend options；native在同一快照读取current owner-binding revision、hidden bound expectation与完整legacy source expectations，mint单次短期`SecretCaptureIntentId`。renderer不能自造`OwnerBindingExpectation`。
2. 用户选择已注册backend后，renderer只提交`captureIntentId + backendInstanceId`，不提交value/legacy/binding authority；native原子claim并fresh revalidate该intent后才打开secure dialog。cancel/invalid不产生record、candidate、binding或backend write。
3. 在任何 OS-store mutation 前落 durable material-free intent。
4. native 写入随机新 ref，读回并常量时间验证；材料与读回 buffer 随即 zeroize。
5. 只创建 `verifiedPendingPlan` candidate；此时不切 binding、不改 Provider、不写 live target。
6. #55 preview 展示 candidate activation 的无值影响，并冻结legacy comparison policy：自动迁移/`legacyScrubExistingBinding` 为 `candidateEquality`；用户从source/binding conflict进行的显式replace/reconcile/rotate为 `explicitReplacement`。用户批准 immutable activation plan 后，#41先完成activation-specific prepare/confirmation，再取得activation Provider lease并完成#55 final baseline recheck，才调用native `activateCandidate`。#35在intent/CAS前重读完整source set/revisions/backend：前者要求每个值与candidate常量时间相等；后者只在plan明确展示“替换这些旧来源”且exact set/revisions未漂移时scrub，不要求旧值等于新candidate。成功后按 exact CAS切binding并scrub已批准sources。
7. Candidate activation不写live target；#41释放activation lease。随后重新读取已绑定owner，#55创建独立apply plan，#41再走prepare/confirm/new lease/backup/writer。两段只共享coordinator规则，不共享plan、capability或lease，也不能把activation变成#35或#55的直接Provider writer。

### 5.2 Change Plan / Configuration Apply

以下只描述已绑定owner的独立live-apply operation；unbound candidate必须先完成§5.1 activation并释放其lease。唯一合法顺序：

```text
#55 sanitized preview/readiness
  -> #55 atomic plan admission
  -> #41 prepare target and optional rollback capability before Provider lease
  -> optional hardware confirmation
  -> Provider lease
  -> final baseline/admission recheck
  -> sanitized structural backup
  -> #35 atomically consumes one-shot capability
  -> revalidate owner/ref/record/binding-set/backend/device/capability/sink/expiry
  -> acquire material inside existing native writer/readback closure
  -> return typed non-sensitive result
```

- prepared capability 不含 material，不能 Serialize/Clone/Debug，也不能进入 job/event/backup。
- target 与 rollback 若都可能用到，分别 prepare；backup 不保存 exact secret-bearing bytes。
- #55 只能 hash typed sanitized structural projection；value、secret-bearing Provider/live bytes及其 digest 都禁止。
- confirmation cancel/expiry/replay、revision drift 或 secret failure 都在首次 target mutation 前失败。
- OS keyring 只有 capability 明确允许且 plan 点名 exact external sink 时才可投影；hardware `persistentTargetProjection=false` 在 preview 与 writer 内各拒绝一次。
- Codex v1 的 closed live-sink ID 只有 `codexAuthJsonOpenAiApiKey` 与 `codexConfigTomlExperimentalBearerToken`。每个 target/rollback credential projection 必须携带且只携带一个 `liveSinkId`；它进入 #55 plan digest、#41 writer construction/readback 与 final baseline。ID 只表达字段槽位，不包含相对/绝对路径。
- `codexAuthJsonOpenAiApiKey` 只能投影 `auth.json` 的 API-key slot，`codexConfigTomlExperimentalBearerToken` 只能投影 `config.toml` 的 bearer-token slot；OAuth fields、其他 TOML token 或 unknown sink 一律拒绝。

### 5.3 Proxy、usage、balance、model fetch、coding plan

- proxy 每次 Provider attempt 在最终 header/send 边界 controlled resolve；secret failure 不联网、circuit-neutral、不得尝试下一 Provider。
- fixed native usage/balance/model-fetch与Provider-primary coding-plan adapter可以 controlled resolve；后者固定`consumer=codingPlanUsageProbe`和closed `CodingPlanPrimaryAdapter`。raw upstream body/error 不进入 public result。ZenMux手填key、Volc AK/SK与team/login不被混入primary-key resolver。
- proxy、usage/balance、model-fetch与primary coding-plan的 credential-bearing request 使用 dedicated native client，固定 `redirect::Policy::none()`；3xx 不跟随、不转发 Authorization，也不触发第二次 network request。
- Codex primary secret 不提供给 generic QuickJS usage script、public `get_balance(apiKey)`、`fetch_models(apiKey)`、terminal env 或 child-process temp file。
- Codex mutation拒绝arbitrary header/body request override；已有Codex行命中时fail closed并进入无值处置，不能把主凭据或Authorization复制进shared `HeaderMap`/raw HTTP `Vec<u8>`。非Codex override如保留，只能作为Level 3相邻域。
- stream check/proxy health不以secret-bearing Provider主动联网探测；Codex diagnostic在写DB或进入query/UI前映射为closed status/category/latency，禁止raw URL、network/OS message或upstream body。上游反射Authorization也只能变成稳定错误。

### 5.4 Rotate

1. 先取得 exact impact/CAS；capture 新材料只生成 candidate，新旧 binding 不变。
2. 用户批准 activation plan 后一次切换完整 owner set，旧 record 进入 stale cleanup。
3. 切换前失败补偿新 entry，旧 binding 不变。
4. 切换后旧 entry 删除失败时不回滚；完整 owner set仍绑定新 ref，但active new ref统一为`stale + SECRET_OPERATION_RECOVERY_REQUIRED`、candidate=`cleanupRequired`且所有consumer fail closed。旧record保留pending cleanup；old delete与fresh missing readback是两个独立prepared/confirmed authorization，中间先durable记录delete receipt。只有missing receipt持久化后cleanup才terminal、新ref恢复ready、旧record写入`supersededByRotation` tombstone。
5. hardware candidate read/old-delete/old-missing-readback与后续cleanup read/delete/readback的任何物理确认都必须在对应Provider lease前完成；cancel/expiry/replay不改变binding或recovery row。

### 5.5 Lock、delete 与 revoke

- logical lock 只改变 FyAgent policy；logical unlock 后仍 fresh-probe backend，不能假装系统已解锁。
- delete 前必须 exact impact + confirmation；durable intent 早于 backend delete。
- 用户确认 delete 后保留 binding/tombstone以解释影响，availability=`revoked`, source=`userDelete`。
- 无 admitted delete/revoke intent而 backend entry消失时为 `missing`。
- central/device revocation只有显式`observe_revocation_once`消费Revoke authorization并持久化full-scope receipt后才为`revoked`且带稳定source/time/action；普通read/probe hint只阻断当前操作，不改变compiled truth。
- rotation cleanup terminal后的旧record为`revoked, source=supersededByRotation`；不能伪装成userDelete/central/device revocation。
- Provider 删除只 detach 该 owner；共享 secret 不隐式级联删除。
- Provider delete preview把device-local binding与current legacy source正交读取。若没有legacy，bound/unbound分支可mint一次性impact；bound分支展示remaining owners、detach后是否orphan、`secretRetained=true`和单独secret-delete入口。若存在任何current legacy source（无论owner同时是否bound），preview必须为`blockedLegacyResolutionRequired + deleteAllowed=false + effect=none`，不mint impact id，只展示无值source count/categories、现有binding view与`resolveLegacyConflict`；先迁移/替换/清理这些current sources后才能重新preview。不得删除Provider-row中唯一明文或误称其已保留。
- Provider确认文案不能继续只说“删除且不可撤销”而让用户误以为凭据也被删除；Provider impact stale固定走Provider-owned `refreshProviderDeleteImpact`，不能误跳到secret-delete的`refreshDeleteImpact`。

### 5.6 Operation recovery 与 candidate expiry

- 通用cleanup impact/result按kind判别：activation cleanup使用#41-held Provider lease；capture compensation与delete finalization是local-only且如需hardware确认必须在mutation前显式prepare/confirm；owner detach finalization只接受不可伪造的already-held Provider detach context。`deleteFinalization`从intent恢复时有独立admitted-delete与missing-readback slot；`captureCompensation`也必须在delete与readback之间durable落checkpoint，不能用一次组合调用绕过第二个authorization。
- `expired` 不是时钟驱动的metadata flip。到期candidate先持久化`discardCandidate + terminalState=expired` intent；只有backend delete/already-missing、fresh missing readback和state commit完成后才公开terminal `expired + action=refreshSummary`。刷新后的owner/candidate card按current truth mint全新的capture intent或rotation authority；不能直接复用旧candidate/operation或把所有kind都写成`retryCapture`。
- explicit discard与expiry delete/confirmation/readback失败时candidate都保持`verifiedPendingPlan`，issue=`SECRET_OPERATION_RECOVERY_REQUIRED`，唯一action=`discardCandidate`，并公开immutable `pendingTerminalDisposition=discarded|expired`；retry不能改写最初disposition，UI也不能隐藏仍可达backend entry。

## 6. Legacy reconcile 与导入/恢复

### 6.1 必须枚举的 Codex sources

- Provider/live `auth.OPENAI_API_KEY`；
- config TOML top-level、active table、每个 inactive table、每个 inline table 的 `experimental_bearer_token`；
- process environment、Windows HKCU/HKLM与shell startup files中的Codex `OPENAI_*`；检测只返回name/presence/stable source category，Codex删除/恢复不建立明文env backup。
- legacy `config.json/.bak/.migrated`、SQLite `common_config_codex`、renderer localStorage与live merge中的Codex common-config TOML；新secret-bearing snippet直接reject，既有命中进入blocked no-value reconcile且不再raw IPC/editor回灌。
- legacy proxy aliases、UsageScript primary override、UniversalProvider Codex conversion、deep link、import/restore/sync payload，以及arbitrary request header/body override与stream/proxy raw diagnostic反射面。
- Add/Edit dialog、Provider list/card、shared types/schema/query/sort、Codex feature hook/form/editor/section/templates、usage/model-fetch请求、MSW/update/list/sort fixtures与Codex deep-link preview helper；这些renderer表面只能接无值draft/public/mutation DTO或在native ingress前reject，Codex不得再接shared API-key input。
- startup Codex template/history migration与它生成的Provider settings backup；新backup只能在clean gate后写结构化placeholder/non-secret config，既有raw generations只scan/report。

非 canonical alias 不享受 fallback。malformed/duplicate/non-string source fail closed。

### 6.2 决策规则

- 无 binding + 一个唯一值（多处相等可视为一个）：verified candidate，等待 plan；不自动 bind/scrub。
- 多个不同值：`sourcesConflict`；不选优先级、不清除。
- existing binding + inline：必须成功读 backend并与每个 occurrence 常量时间相等才可生成 scrub-only candidate。
- locked/denied/unavailable 无法比对：`bindingComparisonPending`，内部 legacy plaintext 暂存，所有 public projection仍脱敏，consumer fail closed。
- 值不同：`bindingConflict`；`resolveLegacyConflict`不是dead guidance，而是以current owner/owner-binding/完整source expectation mint typed capture intent，用户选backend后native reconcile capture新candidate；plan明确采用`explicitReplacement`并展示被替换的exact sources。任何普通retry都不选值，也不要求旧值等于新candidate。
- binding 已切但 Provider scrub 未 durable 完成：owner保持bound；ref=`stale + SECRET_OPERATION_RECOVERY_REQUIRED`；candidate=`cleanupRequired`，Codex consumer继续 fail closed直到完成 scrub。

### 6.3 Startup/import/restore/sync

- startup唯一顺序：`SecretBootstrap::open`取得lifetime lock → DB `open_preflight_without_backup` → one AppState/SecretService消费同一opened handle（不reopen/不接PathBuf）→ same-service journal/legacy reconcile → `app.manage`与static command registration receipt → 仅Clean分支生成sanitized backup → publish consumer gate → 最后启动sync workers/Codex consumers。Blocked分支仍保留managed AppState和修复入口，但无backup/worker/consumer；修复后用同一service/handle resume。旧automatic raw DB backup在gate前禁止。
- manual SQL import、binary restore、WebDAV/S3 download 均先在 temp DB preflight。ImportCoordinator为新owner mint绑定`stageId + tempDatabaseDurableObjectId + fresh process nonce + owner + staged row revision`的`StagedSecretOwnerToken`并形成projection；#55先以该authority/projection mint专用admission，main integration再产生不可伪造的authority-match receipt，之后#35才能prepare/confirm，最后才构造cutover context并exact scrub/readback/cutover。staged token不能用于live readiness/runtime。prepared cancel/discard必须同时terminalize backend state与该admission。post-cutover resume的唯一public request是`stageId + expectedResumeCas{revision,digest}`；full object/process/admission/record/backend/checkpoint/live-owner identity只进入内部digest preimage。crash先终止/核对旧admission，再open同一staged object、mint fresh process identity/recovery admission与新CAS；旧CAS/replay零写。需要用户选择或plan时main DB/live/local binding保持`effect=none`。
- remote snapshot不能携带 device-local refs/bindings/journal/audit，也不能覆盖本机 state；同 owner保留本机 binding，新增 owner走 reconcile。
- runtime path 不得新建第二个 `AppState::new(db)` 或把 post-import secret sync 当 best-effort warning。

## 7. Backup、export 与历史 artifact

- future managed backup/export/sync snapshot 必须从结构化 sanitized projection生成，发布前 readback + canary scan；recovery/migration pending 时 fail closed。
- app-private temp 可自动删除；不把 secure overwrite宣传成 SSD 保证。
- 已存在的 FyAgent-owned backup/cache/diagnostic 在v1永久只做scan/report；不提供rewrite/delete命令，也不把历史文件纳入activation projection。
- user-selected export、被用户移动的 backup和任意外部路径不可穷举；只有用户重新选择时 scan/report。
- corrupt/unsupported artifact保持原样并报告unsupported/read-failed count；不得为了“零命中”静默破坏恢复依据。只有当前Provider/live `LegacySourceExpectation` 可由approved activation exact scrub。

## 8. UI 与可用性

- 主列表首先回答 Provider、状态、下一步；不提供 renderer 密钥输入框、复制或 reveal。
- missing/locked/denied/stale/revoked/unavailable 用图标、文本和颜色三重表达。
- logical lock 与 backend lock 提供不同动作；revoked 与 missing提供不同恢复文案。
- staged candidate 显示“等待变更计划”，不误报为已绑定/已应用。
- rotate/lock/delete 展示全部 affected owners 和 no-fallback 结果；preview 变化后执行必须失败并刷新。
- Provider删除确认单独说明owner detach与secret retention；无legacy的bound preview展示remaining owners/orphan结果和后续“单独删除凭据”入口；任一legacy source存在时只显示阻断卡与迁移/替换动作，不显示确认按钮或impact id。
- legacy阻断卡的“迁移/替换”直接进入typed capture flow并展示当前owner、来源类别与所选backend；不要求renderer拼装hidden binding。terminal expired先刷新current owner/card，再开启全新flow。
- recovery card按kind展示唯一下一步；pending expiry不得显示成terminal expired。所有action decoder/UI必须exhaustive，未注册hardware提供reconnect/open-backend-settings等明确指导而不是循环retry。
- hardware 未注册时不展示可选项；operation confirmation 显示设备、操作、超时和 cancel，不复用旧 step。
- 沿用 FyAgent V2 Prompt/Memory tokens；生成图只标 `visual_reference`。

## 9. 验收门槛

### 9.1 设计与 handoff

- [ ] GitHub #35 authority digest/timestamps/comment IDs 与 requirement mapping 可复核。
- [ ] PRD、technical、detailed、exact contract、device-store、call-graph、native-evidence 口径一致。
- [ ] 独立 product/architecture/detailed reviewers 对同一 immutable design commit 得出 P0=0/P1=0/P2=0。
- [ ] 后置 freeze receipt 指向 design authority SHA；#55/#41 收到并回读 exact SHA、types、capability/error matrix、forbidden fields和 open compatibility items。

### 9.2 实现与模块

- [ ] Provider DB/live-backup/public DTO 不保存/返回 Codex value或 device-local binding；local store只有八类operation-specific journal，具备required-field durable intent、四类tagged recovery/CAS、atomic replace、crash reconcile和严格权限。
- [ ] macOS Keychain与Windows Credential Manager真实 CRUD/replace/delete/missing；native capture不把 value送入 renderer/Tauri IPC。
- [ ] candidate/activation、typed capture intent/legacy conflict/equality/explicit replacement、rotate/lock/delete/revoke/provider detach（含legacy-blocked preview）、proxy/usage/model-fetch/primary-coding-plan/apply、Codex env/common-config/public Provider/request-override/diagnostic闭环、import/restore/sync/staged crash resume和failure matrix均有focused module/integration覆盖；Codex MCP env/header debt有Level 3 no-regression覆盖。
- [ ] ordinary test只用 injected backend；真实 keyring test需显式 ignored/env gate。
- [ ] V2 high-fidelity UI通过模块、browser、四档 viewport和 usability review。

### 9.3 Source freeze 与证据

- [ ] source-freeze后 fresh运行 exact lint/typecheck/unit/integration/browser/renderer/native/Trellis/diff/ownership gates；任一源码修复使下游证据失效。
- [ ] scanner分别报告 `contract_schema`, `codex_feature_runtime`, `repository_static_inventory`, `repository_runtime_global=NOT_CLAIMED`。
- [ ] macOS `native_runtime` + capture `UAT`；Windows x64 `native_runtime` + capture `UAT`。
- [ ] Windows fixed set逐项`result=pass`：real missing、injected locked、injected denied、injected unavailable、injected verify failure、injected post-switch old-delete failure，以及real capture cancel的独立failure/UAT items；“至少三类”只是附加计数，不能放宽任一fixed case。真实 OS与 injected清楚标注；CI compile/unit不冒充 native runtime或UAT。
- [ ] evidence manifest记录 exact SHA、host/OS/arch、command、start/end、exit/count、cleanup和 evidence class。
- [ ] 没有Windows native/keyring与failure-path证据时，任务保持非 DONE。

## 10. 完成定义

完成只代表 Codex Provider 第一切片的主凭据已从 FyAgent 数据流中移出，并由双平台本机 store与上游硬件差异合同闭环；不代表全仓所有凭据、云托管、hardware vendor或跨设备共享已经完成。任务可 push专用分支并创建PR，但不合 main、不部署。
