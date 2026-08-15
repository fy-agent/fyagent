# Issue #35 独立产品设计评审

## 评审结论

- `PRODUCT_REVIEW=REQUEST_CHANGES`
- Open findings: `P0=0`, `P1=7`, `P2=6`, `P3=0`
- 证据级别：`source_report + static_design_review`。本评审未运行 test、build、browser、renderer、server、native runtime、dependency resolution 或 screenshot；不能作为实现或运行验收证据。
- 评审对象：当前工作树中的 `prd.md`、`design.md`、`technical-design-overview.md`、`detailed-design-overview.md`、`execution-plan.md`、`implement.md`、`research/*.md`，以及仓库静态源码事实。Issue #35 正文和评论没有以可复核快照保存在任务目录中，因此对 GitHub authority 的判断仅能依赖 `research/source-audit.md` 的 source-report 声明。
- 通过条件：以下所有 P0/P1/P2 关闭后，由独立产品 reviewer 对最终精确工作树重新完整读取，才可记录 `PRODUCT_REVIEW=PASS`。

## Findings

### PR-001 — P1 — Codex 第一切片与“仓库全局无 secret value”验收互相矛盾

- 状态：OPEN
- 证据：`prd.md` §3.1/§3.2 把实现限定为 Codex Provider 第一切片并明确不迁移全部 Provider；同文件 §4.3、§9“泄漏与运行证据”、§10 却无范围限定地要求 UI/IPC/event/log/diagnostic/screenshot/fixture/SQLite/export/Workspace Pack 全部无值并宣称“所有用户可见与诊断表面”可证明无值。`research/secret-surface-inventory.md` §“Codex first-slice surfaces”、§“Existing adjacent value-bearing surfaces”、§“Required design adjudication” 已证明 WebDAV、S3、coding-plan、usage-script、非 Codex Provider、OAuth、UniversalProvider 等仍有值路径，并明确要求 freeze 前裁决。`detailed-design-overview.md` §9 的 scanner 描述同样未锁定验收域。
- 影响：实现可能只扫描新 DTO 或单个 canary 就错误报告 repository-global pass；UniversalProvider、model fetch、usage/balance 等仍可绕回 Codex 值路径，导致核心“Codex 不再把主凭据作为 FyAgent 数据传递”也未真正闭环。
- 必须闭环：在 freeze 前二选一并写入所有权威文件。推荐保持 MVP，冻结一份精确、可枚举的 `codex_feature_runtime` 调用图和 artifact 清单，至少覆盖 inventory 中每个 Codex first-slice surface；阻断共享 legacy API 对 Codex 的值传递；把邻接路径列为明确 pre-existing debt；所有报告只允许声称 `contract_schema` / `codex_feature_runtime`，明确禁止 repository-global claim。若选择全仓迁移，则必须扩 PRD、owner map、计划和 native/runtime acceptance，不得只扩大 scanner 文案。

### PR-002 — P1 — #55/#41 的 prepare/confirm/resolve 合同尚未收敛为一个可消费序列

- 状态：OPEN
- 证据：`prd.md` §2.2 已改为 `prepare-for-apply` 生成一次性 capability，但 §6.2 仍是 `checkApplyReadiness -> HardwareConfirmStep -> resolve_for_apply(secretRef)`；`technical-design-overview.md` §2 的 exact enum 仍含旧消费者 `contextControlledUse`，§3 使用 `prepare/confirm/PreparedSecretCapability`，§8 capture verification 又引用 backend `resolve`，§11 才描述 #41 Configuration Apply；`detailed-design-overview.md` §6“Live write/apply”仍写 `with_resolved_secret(ref, ...)`；`research/secretRef-contract-handoff.md` §“Consumer call shape”则是 prepare 后再取 Provider lease、baseline recheck、backup、one-shot resolve。
- 影响：#55 与 #41 可各自按不同 API、消费者名和时序冻结实现；硬件确认可能落在 Provider lease 内造成长锁/死锁，也可能被 by-ref resolve 绕过，一次性 capability 失去意义。
- 必须闭环：选择一个 canonical sequence，并逐字同步 PRD flow、exact JSON/Rust contract、详细调用序列、handoff 和 failure matrix：#55 preview/readiness；#41 在 Provider lease 前 prepare 并完成可选确认；取得 lease 后做 baseline recheck/backup；仅在 existing writer closure 内一次性 resolve。明确 public IPC 与 native-only API，移除 `contextControlledUse` 和所有 by-ref writer resolve 的旧称谓，并定义 confirmation cancel/expiry 的一致结果。

### PR-003 — P1 — Prepared capability 允许提前持有 material，且未绑定 secret 生命周期版本

- 状态：OPEN
- 证据：`prd.md` §2.2 要求真实材料只在受控写入闭包内短时存在；`technical-design-overview.md` §3 却把 `PreparedSecretCapability` 定义为可含 “material or backend lease”，它在 #41 Provider lease、baseline recheck 和 backup 前产生，只声明绑定 operation/ref/sink/短 expiry；同文件 §8/§11 允许期间发生 lock、rotate、delete/revoke，但未要求 resolve 时校验 record revision、当前 binding 或 lifecycle。`research/secretRef-contract-handoff.md` 的序列会让 capability 跨越 lease/recheck/backup。
- 影响：OS/hardware material 可能在 writer 外驻留更久；用户在 prepare 后执行锁定、轮换或删除，旧 capability 仍可能把过期凭据写入目标，破坏 fail-closed、撤销和 destructive action 的用户预期。
- 必须闭环：capability 应是无 material 的 opaque one-shot authorization/backend lease；若确实必须承载 material，需修改产品原则并给出可验收的最短生命周期、zeroize 和取消保证。无论哪种实现，都必须绑定 secret record revision/backend generation，并在 writer 内、目标首次 mutation 前重新验证未锁定、未撤销、binding 仍指向该 ref、sink/capability 未变；rotate/delete/lock 后旧 capability 必须稳定失败且 `effect=none`。

### PR-004 — P1 — Provider/Agent MVP 范围无法从现有合同得出唯一实现

- 状态：OPEN
- 证据：`prd.md` §3.1 声称 Agent owner 可创建、查询和轮换，但没有 Agent-specific material consumer；§5.2 只有 `codexApiKey` purpose，§6 与 §8 的用户流程只描述 Codex Provider。`technical-design-overview.md` §2 同时公开 `owner.kind="agent"`，但又写 MVP 只接受 `provider/codex`，purpose/slot 也只有 Codex 形态；`detailed-design-overview.md` §8 只有 Provider credential list。
- 影响：实现者无法判断 Agent binding 是必须可用的当前功能、仅 wire reservation，还是应被拒绝；ownerId/namespace/purpose、UI 入口、删除影响和验收都无法一致，未来 Agent consumer 还可能被一个伪通用、实际 Codex-specific 的 v1 合同锁死。
- 必须闭环：推荐 MVP 只接受并验收 `provider/codex + codexApiKey`，保留 `agent` 作为未来 wire-reserved enum 但所有具体 Agent binding 请求稳定拒绝，删去“Agent 可创建/查询/轮换”的本轮承诺。若坚持本轮支持 Agent，则必须列出允许的 Agent namespace、稳定 ownerId、purpose/slot、用户入口、生命周期/删除/轮换语义和 native acceptance；不能只靠通用 DTO 宣称支持。

### PR-005 — P1 — Legacy “existing binding + inline unknown” 路径可能静默删除不同的有效凭据

- 状态：OPEN
- 证据：`detailed-design-overview.md` §7 把 `existing binding + inline same/unknown` 归为“binding probe 后 scrub-only”；probe 只证明 entry presence，不能证明 keyring material 与 inline value 相同。`prd.md` §6.5 只定义两个 inline 字段彼此不同时的 conflict，没有定义已有 binding 与 inline 值不同、locked/denied 时无法比对的状态和恢复流程。
- 影响：binding 指向 A、inline 留有 B 时，存在性 probe 后清除 B 会造成不可逆凭据丢失或静默切换身份；locked/denied 下的 “unknown” 尤其不能被当成 same。
- 必须闭环：只有成功 resolve 并常量时间证明相等时才允许 scrub-only。不同或无法证明相等时必须保留内部 plaintext、所有 public projection 继续脱敏，并进入明确的 migration conflict/pending 状态；提供无值的 native replace/reconcile 流程，定义 retry/idempotency/补偿以及最终清除条件。

### PR-006 — P1 — 自动改写历史 export/backup 的所有权、授权和部分失败语义缺失

- 状态：OPEN
- 证据：`prd.md` §6.5.4 要迁移时清理 FyAgent 管理的 JSON export、diagnostic、backup；`detailed-design-overview.md` §7 要把既有 exports/backups 写到 temp 后原子替换，corrupt artifact 留存并报告；但没有列出受管目录、文件所有权、用户导出与应用缓存的边界、确认步骤、restore 兼容性、部分清理状态或 retry contract。`research/secret-surface-inventory.md` 也只列类别，没有给出可穷举路径。
- 影响：静默重写用户持有的备份/导出可能破坏恢复依据或审计历史；DB scrub 成功而历史文件清理失败时，产品仍残留明文，却没有准确状态和下一步，scanner 也无法对应用不可枚举的外部路径作零命中承诺。
- 必须闭环：列出唯一受管路径和 artifact 类型；应用私有 cache 可自动处理，但用户导出/备份默认只 scan/report，任何重写都需要明确 preview、影响说明和确认。定义逐 artifact outcome、partial/incomplete 状态、重试和 restore 兼容规则；runtime evidence 只能覆盖报告中实际枚举并有权限读取的路径。

### PR-007 — P1 — Revocation 被折叠成 missing，无法满足“可解释撤销”和硬件差异合同

- 状态：OPEN
- 证据：`prd.md` §2 把撤销时的可解释失败列为用户问题，§3.1/§6.4 又承诺删除/撤销生命周期；但 §5.3 的 availability 没有 `revoked`。`technical-design-overview.md` §4 暴露 `centralRevocation`，§5 没有 revoked error，§6 有 lifecycle `revoked`；`detailed-design-overview.md` §5 明确把 lifecycle revoked 归一化为 `missing`。
- 影响：UI 和 #55/#41 无法区分用户主动删除、硬件/中心撤销与意外 keyring 缺失，因而无法给出正确原因、审计和恢复动作；`centralRevocation` 只是不可观察的布尔宣传。
- 必须闭环：增加可观察的 revoked availability/error（含非敏感 source/timestamp/action）并定义 OS delete、central revoke、backend missing 的不同映射与 UI/#55/#41 行为；或从 v1 capability 与成功声明中移除 central revocation/撤销支持，保留为未来未实现项。不能同时承诺“可解释撤销”又输出 missing。

### PR-008 — P2 — 稳定 SecretSummary 混入 operation-scoped hardware confirmation

- 状态：OPEN
- 证据：`technical-design-overview.md` §2 的 `SecretSummary` 同时含 `availability="confirmationRequired"` 和 `hardwareConfirmStep`，`list_secret_summaries` 会返回它；同节 `ApplyReadiness` 又重复这些字段。§3 规定确认 step 必须绑定 operationId + secretRef + targetSink 并过期；`detailed-design-overview.md` §5 还把 `operationStage` 混入一般 availability 派生。
- 影响：没有具体操作和 sink 的列表页无法产生合法确认步骤，容易显示过期状态或把一次操作的 step 复用于另一操作；用户会在未发起 apply 时看到无意义的“需确认”。
- 必须闭环：让 `SecretSummary` 只表达稳定 record/binding/presence 状态；confirmationRequired/step 只存在于本次 readiness/prepare operation response。定义 cancel/expiry 后 UI invalidation 和重新发起规则，禁止 summary list 缓存或重放 step。

### PR-009 — P2 — 逻辑锁与 OS/backend 锁共用一个状态，UI 无法给出正确下一步

- 状态：OPEN
- 证据：`prd.md` §6.4 明确区分 FyAgent logical lock 与 OS keyring lock/access denial；`technical-design-overview.md` §5 只有一个 `SECRET_LOCKED`/`availability=locked`/`action=unlock`，§8 又说二者分别归一化，但 public DTO 没有 lock origin；`detailed-design-overview.md` §5 把两者折叠为同一优先级结果。
- 影响：用户不知道应点击 FyAgent“解锁”、触发 OS prompt，还是去系统设置；点击逻辑 unlock 可能成功返回但 backend 仍锁定，违背“现在能否使用、下一步是什么”的 UI 目标。
- 必须闭环：公开非敏感 `lockSource`，或拆分稳定 error/action（policy locked 与 backend locked）；分别定义 UI 动作、retry 条件、审计和 #55/#41 映射，并验证 logical unlock 不会伪装 OS unlock 成功。

### PR-010 — P2 — shared ref 的 rotate/lock 没有依赖影响预览与并发前置条件

- 状态：OPEN
- 证据：`prd.md` §5.2/§6.3 允许一个 ref 被多个 binding 依赖并在轮换时切换全部依赖，§8 要求 destructive action 展示依赖；`technical-design-overview.md` §2 的 `rotate_secret`/`set_secret_locked` 只接收 secretRef，只有 delete 有 `get impact + expectedDependencyCount`；§8 的 rotation 只在 service 内读取 count。`detailed-design-overview.md` §8 也只为 delete 定义依赖确认。
- 影响：用户从一个 Provider 触发轮换/锁定时可能同时影响其他 Provider/Agent，且 UI preview 后新增依赖不会让操作失败；这与可解释 destructive semantics 和 no-silent-fallback 不一致。
- 必须闭环：二选一：MVP 强制一 ref 只绑定一个 owner；或为 rotate/lock 增加 exact impact DTO、owner summary、expected count/revision 和确认步骤，依赖变化时稳定失败。UI 必须明确所有受影响 owner 及不会自动回退的结果。

### PR-011 — P2 — Legacy conflict/partial migration 没有可执行恢复流程，相关 public result DTO 也未冻结

- 状态：OPEN
- 证据：`prd.md` §6.5 能产生 `SECRET_LEGACY_CONFLICT`，但未说明用户如何在不 reveal value 的情况下选定恢复动作；`technical-design-overview.md` §2 自称 exact public contract，却引用未定义的 `SecretDeleteImpact`、`SecretMigrationReport`、`SecretAuditPage`，§5 也未给 legacy conflict/migration failed 定义 action；`detailed-design-overview.md` §7 只说用户可 retry，而 retry 不能消除两个不同值的 conflict，§8 只有通用 failure card。
- 影响：用户可能长期停在 migrationRequired，旧明文继续内部存留，却没有安全的“捕获新值并替换/保留哪一来源/何时 scrub”路径；前后端还会各自发明结果形状，增加泄漏和错误成功状态风险。
- 必须闭环：冻结上述 exact non-sensitive DTO，至少包含 per-owner outcome、已知来源类别、artifact cleanup counts/status、retryability/action、partial/incomplete 和 dependency owner summaries。为 conflict 定义 native capture 新凭据或显式 reconciliation 的恢复流程，确认后才 scrub 两个旧位置；普通 retry、cancel 与最终成功必须有不同可验收状态。

### PR-012 — P2 — hardware 是合同占位还是用户可选后端尚未决定，确认 UI 又缺少设备信息

- 状态：OPEN
- 证据：`prd.md` §3.2 明确本轮不实现真实硬件协议，但 §6.1 让用户选择 backend；`technical-design-overview.md` §2 把 `hardware` 放入 public backend enum，`detailed-design-overview.md` §2 明确 production 只注册 unavailable hardware adapter，§8 仍设计 hardware step panel。与此同时，`prd.md` §8 要在确认步骤清楚说明设备和超时，但 `HardwareConfirmStep` 只有 ids、promptKey、expiresAt，没有可安全显示的设备标签/绑定信息。
- 影响：MVP UI 可能展示一个必然失败的“硬件”选项，制造已支持的错觉；未来有多设备时用户也无法确认正在触摸哪台设备。
- 必须闭环：明确 hardware v1 是 contract-only，未注册可用 adapter 时不得作为 Add/Replace 的可选项；仅对已有 hardware binding 显示不可用状态，或完全隐藏。为真实 adapter 预留非敏感、可本地化的 device display/selection contract 与 timeout/cancel 文案，并把“能力存在”与“当前后端可用”分开。

### PR-013 — P2 — GitHub authority 没有本地可复核证据，source gate 目前只是自我声明

- 状态：OPEN
- 证据：`research/source-audit.md` §“Audit envelope”只有 Issue #35 链接和“current body plus both comments read”的声明，没有 body/comment ID、更新时间、source digest 或 requirement-to-PRD mapping；任务目录也没有 Issue 正文/评论快照。`execution-plan.md` closure checklist 已把“latest Issue #35 body/comments”勾为完成。
- 影响：独立 reviewer 无法判断当前 PRD 是否覆盖 Issue 原始需求或是否在评论后漂移；后续 design-freeze SHA 只能证明设计文件不变，不能证明它对应哪一版 GitHub authority。
- 必须闭环：在 source report 中记录 body `updatedAt`、两条 comment ID/时间、读取时间和 requirement mapping，最好保存静态快照或内容 digest；对任何未覆盖/冲突要求给出显式 disposition。完成后重新核对 PRD，再保留 closure checklist 的完成状态。

## 覆盖结论

- 用户问题/非目标：方向清楚，但 PR-001、PR-004 使成功范围尚不真实可验收。
- Native no-value capture：renderer/IPC 无值、OS 原生 secure control、cancel/no-fallback 方向成立；PR-002、PR-003 仍会在 apply 路径破坏“一次性且仅 writer 内 material”的合同。
- State/lifecycle：presence 与 availability 分离是正确方向；PR-007、PR-008、PR-009 尚未关闭。
- Legacy migration/destructive semantics：先写入验证、再切 binding、失败补偿的主方向成立；PR-005、PR-006、PR-010、PR-011 阻止安全 freeze。
- Hardware differences：no fallback、device binding、physical confirmation、no persistent projection 已被识别；PR-003、PR-007、PR-008、PR-012 表明差异还没有形成可消费、可解释的产品合同。
- #55/#41 impact：handoff 已标为 draft 且 authority SHA pending 是正确的；PR-002、PR-003 必须在发送 immutable handoff 前关闭。
- Usability/acceptance truth：图标+文本+颜色、dependency count、evidence-class 分级方向正确；PR-001、PR-006、PR-009、PR-011、PR-012、PR-013 仍会造成错误成功或无恢复出口。

## Gate

`PRODUCT_REVIEW=REQUEST_CHANGES`

只有 `P0=0, P1=0, P2=0` 且 reviewer 对修订后的最终精确工作树完成重新读取，才允许改为 `PRODUCT_REVIEW=PASS`。
