# Prompt / Memory 产品设计静态评审

- 评审阶段：A（现状复核与产品设计评审）
- 评审日期：2026-08-12
- 关闭复审：2026-08-12；复审对象为新增技术设计、详细设计、执行计划及更新后的 PRD/入口文档
- 证据等级：`code_audit`（仅文档与代码静态审阅）
- 评审范围：`prd.md`、`design.md`、`implement.md`、指定 research 文档、Prompt / Memory / `agentTargets` 实现及聚焦测试
- 执行边界：本评审未运行 lint、typecheck、unit/integration/browser test、Playwright、build、dev server、截图或 pixel diff
- 改动边界：评审建议只落在 Prompt、Memory、共享目标合同及其测试；不建议修改 Agent 目录、模型、Skills、MCP、`navigation`、PrimaryNav 或 V2 Shell

## 1. 结论

`DESIGN_REVIEW=PASS`

初审记录的 1 个 P0、5 个 P1 和 3 个 P2 均已被 `technical-design-overview.md`、`detailed-design-overview.md` 与 `execution-plan.md` 明确承接。原 P0/P1 现在都有数据模型、状态转换、文件 owner 和聚焦测试设计；三个 P2 也已有实现落点与测试设计。产品设计层不再存在冻结阻断。

本段只描述 **2026-08-12 设计关闭时点**：当时源码仍保留初审发现的行为，尚未按冻结设计实施，也未在该轮执行任何测试或运行验收；下表的“设计已关闭、实现待验证”是历史阶段状态。实施后的新鲜证据与最终结论见第 6 节。

### 1.1 P0/P1 关闭复审

| 初审项 | 设计关闭证据 | Owner 与验证设计 | 复审状态 |
| --- | --- | --- | --- |
| P0-01 同步预览误标“已同步” | `technical-design-overview.md` 6.3、7.2、8 将 preview task 与 durable result 分离；`detailed-design-overview.md` 6.1、6.4 明确 `previewState=pending`、`durableState=not-run`，并删除 prototype durable“已同步” | Memory owner；详细设计 10.2 第 3 项要求断言逐目标 pending、durable 未执行且不存在“已同步” | **设计已关闭、实现待验证** |
| P1-01 缺少逐目标状态与 partial 表达 | `technical-design-overview.md` 6.3、9、12 定义逐目标 preview task、独立 preview/sync port 和未来 durable/partial 状态；`detailed-design-overview.md` 6.1、6.4、6.5、6.7 定义每目标 task、revision 与显示责任 | Memory owner；详细设计 10.2 第 3、4 项覆盖每目标任务和 revision 失效 | **设计已关闭、实现待验证** |
| P1-02 缺少路由级 dirty guard | `technical-design-overview.md` 3、7.1、7.2 将 blocker 限定在各页面；`detailed-design-overview.md` 7 给出 `useBlocker` 判定、reset/proceed 和 data-router harness 合同 | Prompt/Memory 各自 Page owner；详细设计 10.1 第 7 项、10.2 第 10 项覆盖取消、确认及保存后离开 | **设计已关闭、实现待验证** |
| P1-03 新建/提炼及目标草稿无 saved baseline | `technical-design-overview.md` 6.2、6.3、7 定义 baseline、transient 与保存门禁；`detailed-design-overview.md` 5.1-5.3、6.2-6.5 给出首次保存、放弃、目标 dirty、revision/task 失效转换 | Prompt/Memory 各自 owner；详细设计 10.1 第 5、6 项及 10.2 第 2、4、7、9 项 | **设计已关闭、实现待验证** |
| P1-04 提炼 provenance 丢失 | `technical-design-overview.md` 6.3 将草稿资源与来源链拆开；`detailed-design-overview.md` 6.1、6.6 固定 source item/target/tool/path/time/summary，并要求右栏显示“提炼自” | Memory owner；详细设计 10.2 第 6 项验证完整来源链与原来源不变 | **设计已关闭、实现待验证** |
| P1-05 Daily 可直接编辑 | `technical-design-overview.md` 6.3、11 锁定 Daily/Session 只读；`detailed-design-overview.md` 6.3、6.4 以 `editableInPrototype=false` 与 category 判定隔离来源能力和本轮操作 | Memory owner；详细设计 10.2 第 5 项验证 Daily/Session textarea 只读且只能提炼 | **设计已关闭、实现待验证** |

### 1.2 P2 关闭复审

| 初审项 | 设计承接 | 复审状态 |
| --- | --- | --- |
| P2-01 Memory 未展示 exists/path status | `detailed-design-overview.md` 6.1、6.7 定义 `exists / missing / frontend-draft` 和对应可见文案；10.2 第 8 项覆盖 | **设计已关闭、实现待验证** |
| P2-02 可见文案暗示真实读写 | `technical-design-overview.md` 8 与 `detailed-design-overview.md` 5.5、8 要求持续可见“前端原型 · 未读取或写入本机文件”，并把托管区块改为未来条件句 | **设计已关闭、实现待验证** |
| P2-03 canonical path 仅靠人工合并 | `technical-design-overview.md` 6.1、10 与 `detailed-design-overview.md` 4.1、4.2 定义 Prompt 专用 canonical resource key、分组算法和后端 realpath 边界；共享 owner 新增独立合同测试 | **设计已关闭、实现待验证** |

复审同时确认：文件 owner 保持互斥；路由保护由 Prompt/Memory 页面内部承担；设计明确不修改 Agent 目录、模型、Skills、MCP、`navigation`、router、AppShell 或 V2 Shell。

## 2. 初审需求—实现—缺口矩阵（历史代码基线）

以下矩阵保留 2026-08-12 初审时的源码事实，用于实施后的回归对照；其中“不通过/部分通过”不代表新设计仍有缺口，而代表当时代码尚待按设计实现和验证。

| 需求 | 当前实现证据 | 结论 | 缺口 / 后续动作 |
| --- | --- | --- | --- |
| 不残留 Codex 单应用限制 | `src/v2/shared/config/agentTargets.ts:29-116` 定义 6 个工具、7 个目标；`src/v2/pages/prompts/Page.tsx:354-385` 渲染全部目标；聚焦测试断言不存在旧“同一应用仅启用一条”文案 | 通过 | 无 Codex 胶囊、应用切换器或单应用互斥逻辑残留 |
| Prompt 多条规则可同时启用 | `PromptsPage.togglePrompt` 只更新目标规则；prototype 中两条规则默认启用；`tests/v2/pages/prompts/Page.test.tsx` 验证第三条启用后前两条保持启用 | 通过 | 保留独立 `enabled` 状态，不恢复旧 `enable_prompt` 互斥语义 |
| Prompt 每条规则支持多个目标 | `PromptPrototypeItem.targetIds[]`、`toggleDraftTarget`、`savePrompt`；7 个 checkbox；启用前至少一个目标与已启用规则最后目标保护已实现 | 通过 | 当前交互确为多选，不是应用单选伪装 |
| 7 个目标资源覆盖 8 个实例 | `openclaw-default.instanceNames = ["main", "utility"]`，其余目标各覆盖一个实例；`countCoveredAgentInstances` 用于页面摘要 | 通过（当前快照） | 规范化路径去重仍只靠手工种子，见 P2-03 |
| Prompt 显示文件存在/缺失状态及来源、类型、用途 | `promptFileExists` 渲染“已存在 / 启用时创建”；列表、详情展示描述、category、origin、updatedAt | 通过 | 缺失文件状态符合 PRD；真实性文案需补上下文，见 P2-02 |
| Prompt 新建、保存和 dirty guard | 条目切换、新建前调用 `confirmDiscard`；保存反馈为“已保存到前端预览” | 部分通过 | 路由切换未拦截；新建项在保存前已进入 committed 列表，见 P1-02、P1-03 |
| Memory 分类与来源展示 | `longTerm / daily / sessions` 三类独立保存 query 和 selectedId；来源显示工具、实例标签、格式、位置、能力、数量和更新时间 | 通过 | `exists` 路径状态未渲染，见 P2-01 |
| Memory 严格区分来源和同步目标 | prototype 覆盖 6 工具来源；`memoryWritableTargetIds` 仅含 Claude、两个 OpenClaw workspace、Hermes；聚焦测试明确排除 Codex 和 OpenCode 目标 | 通过（目标集合） | 目标状态仍是单一总状态，无法表达逐目标结果，见 P1-01 |
| 可编辑项、只读项和 Prompt 归属分离 | `editorReadOnly = !writable || owner === "prompts"`；Prompt-owned 项只读并指向提示词页 | 通过 | 不存在 Memory 与 Prompt 同时编辑同一规则文件的 UI |
| 原始每日/会话记录保持不变 | session seed 为只读；但 daily seed 均为 `writable: true`，通用保存处理器也会更新 daily 内容 | 不通过 | 每日来源可被直接编辑保存，见 P1-05 |
| 每日/会话提炼为长期草稿且保留来源 | `promoteToLongTerm` 创建新对象，不改原数组项 | 部分通过 | 源路径、源更新时间、源条目和摘要未进入 provenance，见 P1-04 |
| 同步前保存并至少选择一个目标 | `syncLongTermMemory` 检查 `draftTargetIds.length` 和正文 `isDirty` | 部分通过 | 新提炼草稿可不经显式保存直接同步；目标选择不计 dirty，见 P1-03 |
| 原型扫描/同步不得伪装真实文件读写 | 扫描、保存和同步反馈多数带“预览”；根节点有 `data-data-source="prototype"` | 不通过 | 同步动作和 seed 状态显示“已同步”，且部分页面主文案使用已落地口吻，见 P0-01、P2-02 |
| 逐目标状态与部分成功 | PRD 要求待处理、同步中、已同步、冲突、失败、不支持和 partial | 不通过 | 当前只有 item 级 `syncState`，没有 target result，见 P1-01 |

## 3. 发现

### P0-01 预览同步被呈现为真实“已同步”

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:187` 要求原型同步只生成本地任务反馈；`prd.md:205-206` 要求逐目标状态和部分成功真实表达；`research/local-agent-inventory.md:83,121` 明确本地保存不能冒充同步成功、前端不应硬编码“已同步”。
- 实现证据：`src/v2/pages/memory/Page.tsx:216-243` 的 `syncLongTermMemory` 没有任何持久化调用，却将条目 `syncState` 设为“已同步”；`src/v2/pages/memory/prototype.ts:77,161,203,224` 还预置了多个无回读证据的“已同步”状态。
- 测试证据：`tests/v2/pages/memory/Page.test.tsx:62-66` 在“前端预览：已生成 2 个同步任务”之后明确断言“已同步”，把错误语义固化成了预期。
- 影响：用户会把任务生成误认为 Claude/OpenClaw/Hermes 文件已经落盘，与本轮“只改前端、无真实文件写入”的边界正面冲突。
- 建议：prototype 使用独立的预览态，例如“任务待执行 / 仅前端预览”，seed 不展示未经扫描回读证明的“已同步”；同步点击后生成逐目标 preview task，不更新 durable sync state。同步修改对应聚焦测试。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P1-01 Memory 状态模型无法表达逐目标结果和 partial

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:166,203-206` 要求逐目标任务、完整来源链、六类目标状态和部分成功；`research/local-agent-inventory.md:83` 要求目标引用、冲突状态和逐目标结果。
- 设计/实现证据：`design.md:144` 与 `src/v2/pages/memory/prototype.ts:14-33` 都把同步状态压成 item 级 `MemorySyncState`，`syncTargetIds[]` 只有 ID，没有每目标状态、错误或时间；`src/v2/pages/memory/Page.tsx:485-499` 只渲染一个总状态。
- 影响：即使只做前端预览，也无法表达“4 个任务中的 3 个待执行、1 个不支持”，更无法为后续真实 adapter 保留 partial/conflict/failed 的产品位置。
- 建议：在冻结设计中拆成“条目本地状态”和 `targetTasks[]`，后者至少包含 `targetId / previewState / durableState / error / updatedAt`；原型只产生 preview/pending，不伪造 durable result。右栏按目标渲染任务结果。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P1-02 dirty guard 未覆盖路由切换

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:117` 明确要求切换条目、新建或切换路由前确认放弃；Memory 的 `prd.md:184` 也要求未保存修改不能直接切换分类、来源、扫描或提炼。
- 实现证据：Prompt 的 `confirmDiscard` 只由 `selectPrompt`、跨条目 `togglePrompt` 和 `createPrompt` 调用；Memory 的 `confirmDiscard` 只由分类、来源、扫描和提炼处理器调用。`src/v2/widgets/app-shell/PrimaryNav.tsx:14-26` 是直接 `NavLink`，`src/v2/app/router.tsx:38-49` 没有 route blocker；相关文件也没有 `beforeunload`/`useBlocker`。
- 测试证据：`tests/v2/app/router-shell.test.tsx` 只验证页面可达和导航选中态，没有编辑后离开页面的拦截场景。
- 影响：编辑 Prompt 或 Memory 后点击主导航会直接卸载页面并丢失草稿。
- 建议：在 Prompt / Memory 页面内部锁定最小的 page-level route blocker 合同，同时覆盖 Hash Router 导航；保持 `navigation`、PrimaryNav、router 配置和 V2 Shell 不变。聚焦测试应验证取消时留在原路由、确认时离开。是否处理窗口关闭可作为独立明确决策，不能与路由 guard 混写。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P1-03 新建/提炼草稿与目标草稿没有可靠的保存基线

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:117,184-186` 要求未保存拦截、同步前必须保存、提炼产物是长期记忆草稿。
- Prompt 证据：`src/v2/pages/prompts/Page.tsx:117-142` 的 `createPrompt` 立即把新项放进 `items`，同时把相同对象设为 `draft`；因此 `isDirty` 为 false。用户不填写、不保存就切换条目时不会确认，空白项仍留在库中。
- Memory 证据：`src/v2/pages/memory/Page.tsx:246-279` 同样立即把提炼项放进 `itemsByCategory.longTerm` 并加载为相同 draft；选择目标后 `isDirty` 仍为 false，因为 `src/v2/pages/memory/Page.tsx:104-107` 只比较标题和正文，不比较 `draftTargetIds`。随后可在未显式保存提炼草稿的情况下生成同步任务。
- 影响：产品无法稳定区分“刚创建的草稿”“保存到前端预览的条目”和“准备同步的已确认版本”；目标选择也可能在切换来源时无提示丢失。
- 建议：增加明确的 `isNew/hasSavedBaseline` 或 revision 基线；新建/提炼在首次保存前必须视为 dirty；Memory 明确目标选择是同步参数还是可保存草稿——若是草稿，纳入 dirty 比较；同步必须引用最后一次已保存 revision。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P1-04 提炼产物未保留 PRD 要求的完整来源链

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:160,186,203` 要求只复制草稿和来源引用，并保留源工具、实例、workspace、资源路径、时间和摘要；`research/product-model-backend-gap.md:170,173,209` 指出缺少 provenance 会导致无法证明来源、去重或回到原记录。
- 实现证据：`MemoryPrototypeItem` 只有 `toolId / sourceTargetId / sourceLabel / path / updatedAt`，没有 `sourceResourceId / sourceEntryId / sourcePath / sourceUpdatedAt / capturedAt / sourceSummary`。`promoteToLongTerm` 虽保留 tool/target/label，却把 `path` 覆盖为“FyAgent 草稿 · 同步时按目标适配”、把 `updatedAt` 覆盖为“刚刚”，且没有另存原路径和原时间。
- 影响：提炼后无法从长期草稿准确回到原始 daily/session 条目，也无法解释“这条结论来自哪个资源的哪个版本”。
- 建议：冻结一个最小 provenance 对象，并在右栏显示“提炼自”链路；前端 prototype 也应保留原 resource/path/time，而不是等后端阶段再补。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P1-05 Daily 来源被实现为可编辑，违背“原记录不变”

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.1；以下保留初审源码证据。

- 需求证据：`prd.md:160,186,208` 要求每日/会话用于检索与提炼，提炼只复制草稿且不修改原始记录；跨工具整库复制也不是本轮能力。
- 实现证据：`src/v2/pages/memory/prototype.ts:239-309` 的三类 daily seed 均为 `writable: true`；`src/v2/pages/memory/Page.tsx:102-106` 因而把 Daily 编辑器判定为可写；`saveMemory` 在 `Page.tsx:179-204` 对当前 category 通用更新，用户可直接保存 Daily 标题和正文。
- 测试证据：`tests/v2/pages/memory/Page.test.tsx` 只验证 Daily 可提炼，没有断言 Daily textarea 只读或保存不可用。
- 影响：页面允许用户改动本应作为提炼来源保留的原始每日记录，破坏“原来源保持不变”的产品承诺，也混淆“编辑长期草稿”和“编辑采集来源”两种动作。
- 建议：本轮把 Daily 与 Session 一样作为只读来源，只开放“提炼为长期记忆”；若未来确需编辑某工具原生 Daily，应另行定义 adapter capability、保存风险和明确的直接编辑入口，不能由 prototype 的通用 `writable` 开关顺带开放。
- 是否阻断设计冻结：**否（设计层已关闭；当前代码仍待实现与验证）**。

### P2-01 Memory 有 `exists` 字段但没有展示路径状态

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.2；以下保留初审源码证据。

- 需求证据：`prd.md:52` 要求未创建或只读来源明确展示；`prd.md:165` 要求位置、能力、数量、状态和更新时间完整呈现。
- 实现证据：`src/v2/pages/memory/prototype.ts:27` 定义 `exists`，提炼草稿在 `src/v2/pages/memory/Page.tsx:262` 设为 false；但 `src/v2/pages/memory/Page.tsx:460-499` 的检查器没有读取 `selectedItem.exists`。
- 影响：用户看不到资源是“已存在 / 缺失 / 前端草稿”，提炼项与真实本机文件在状态呈现上混在一起。
- 建议：在来源详情加入路径状态；prototype 草稿使用“前端草稿 · 未创建文件”，真实扫描后再映射 exists/permission/error。
- 是否阻断设计冻结：否；已进入详细设计和 Memory 验收项，当前代码待实现与验证。

### P2-02 可见文案仍有真实读写已落地的暗示

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.2；以下保留初审源码证据。

- 需求证据：`prd.md:118,187` 和 `design.md:341` 要求原型反馈不使用真实写入语义。
- 实现证据：`src/v2/pages/prompts/Page.tsx:207` 写“把长期规则组合注入本机 Agent 的真实指导文件”，`Page.tsx:405` 写“现有文件内容由托管区块保护”；`src/v2/pages/memory/Page.tsx:299` 写“读取本机 6 个 Agent 工具的记忆文件与会话来源”。这些能力本轮并未发生。`data-data-source="prototype"` 是机器属性，不是用户可见说明。
- 影响：即使保存 feedback 正确，用户仍可能把页面理解成已经扫描真实文件、已经启用受管区块保护。
- 建议：增加持续可见的“前端预览 / 未读取或写入本机文件”状态；将能力描述改成未来条件句，例如“接入真实同步后，同一路径只执行一次并保护区块外内容”。PRD 要求的“启用时创建”可保留，但应处于这个可见预览上下文中。
- 是否阻断设计冻结：否；冻结文案与测试要求已明确，当前代码待实现与验证。

### P2-03 路径去重目前是人工合并，不是规范化路径合同

复审状态：**设计已关闭、实现待验证**。关闭依据见 1.2；以下保留初审源码证据。

- 需求证据：`prd.md:108,256` 与 `design.md:129` 要求共用路径合并、只生成一次任务；本轮目标特别要求按规范化路径去重。
- 实现证据：`src/v2/shared/config/agentTargets.ts:81-90` 将 main + utility 手工写成一个目标，当前 7/8 快照正确；但 `AgentTargetDefinition` 只有展示用 `promptPath`，没有 canonical key/normalized path，`allAgentTargetIds` 和 `countCoveredAgentInstances` 也不执行去重。
- 影响：当前 seed 不重复，但一旦扫描层返回等价路径、符号链接或重复 workspace，前端合同无法证明仍只产生一个资源任务。
- 建议：详细设计锁定 `canonicalResourceId` 或规范化路径 key，以及“分组后保留 instanceIds”的规则；共享合同聚焦测试静态验证 7 个唯一资源、8 个实例和 canonical key 唯一。
- 是否阻断设计冻结：否；技术设计已写清前端分组算法与后端 canonicalization 边界，当前代码待实现与验证。

## 4. 聚焦测试的静态缺口

本节仅审阅测试源码，没有执行测试。

需要在实现阶段补充或修正的聚焦用例：

1. 修正 Memory 同步预览测试：生成任务后断言“未写入 / 待执行”，禁止断言“已同步”。
2. Prompt 与 Memory 分别覆盖编辑后点击主导航的取消/确认分支。
3. Prompt 覆盖新建后未保存直接切换，不能静默留下 committed 空项。
4. Memory 覆盖提炼草稿未保存不能同步，以及目标选择后的 dirty 行为。
5. Memory 覆盖 Daily 与 Session 均只读，只能提炼，不能直接保存原始来源。
6. Memory 覆盖提炼产物保留 source resource/path/time/entry，并验证原始项未变。
7. Memory 覆盖 `exists=true/false` 的可见路径状态。
8. Memory 覆盖逐目标 preview task 与 partial 状态，而不是单一 item 状态。
9. 共享目标配置增加独立合同测试，验证 7 个唯一资源覆盖 8 个实例、4 个 Memory 目标以及 OpenClaw 默认路径只出现一次。

## 5. 初审提出的设计冻结关闭条件（历史）

1. 去掉 prototype 的 durable“已同步”语义，改为逐目标 preview/pending 任务，并修正聚焦测试预期。
2. 补齐路由级 dirty guard；新建 Prompt、提炼 Memory 和 Memory 目标选择都要有一致的保存基线。
3. 为提炼条目补齐最小 provenance，能回到原来源并保留原路径/时间。
4. 将 Daily 恢复为只读来源，只通过提炼产生可编辑长期草稿。
5. 将 Memory 状态模型从 item 级总状态拆到逐目标状态，能够表达 partial/conflict/failed/unsupported。
6. 在详细设计中明确路径状态、可见 prototype 文案和 canonical path 去重合同。
7. 把 `prd.md` 已勾选但与实现不一致的脏草稿、提炼来源、同步真实性条目恢复为待关闭，待实现与静态复审后再勾选。

关闭复审结果：上述条件均已由新技术设计、详细设计和执行计划承接，故本评审已改为 `DESIGN_REVIEW=PASS`。这些条件只有在对应源码修改和聚焦测试取得新鲜结果后，才能进一步标记为“实现已验证”。

## 6. 实施后证据回读（2026-08-13）

- 原 P0/P1 已映射到 saved baseline、逐目标 `pending / not-run` task、Daily/Session 只读、完整 provenance 和 page-level dirty guard。
- exact Node 24.19.0 下 12 files / 82 unit tests、48 browser tests 与 standalone `file://` 验收通过。
- 产品评审结论维持 `DESIGN_REVIEW=PASS`；实现证据见 `research/verification.md`，不把 prototype 反馈冒充真实 Agent 同步。
