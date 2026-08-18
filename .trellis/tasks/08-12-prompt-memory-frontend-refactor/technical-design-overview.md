# Prompt / Memory 前端技术设计概要

## 1. 文档状态

- 任务：`prompt-memory-frontend-refactor`
- 设计阶段：冻结完成；实现与运行验收已通过
- 证据等级：`code_audit`
- 基线：`origin/dev/laiyongjie` @ `e33d37dd6f9d58c11207f843b5c33750a79dbb4a`
- 实施分支：`codex/prompt-memory-frontend-refactor`
- 产品事实源：`prd.md`
- 静态现状审查：`reviews/product-design-review.md`
- 本文状态：`DESIGN_FREEZE=2026-08-12`；`ARCHITECTURE_REVIEW=PASS`

本文补齐现有 `design.md` 的技术架构层。`design.md` 保留为权威入口和短摘要，不再单独维护另一套实现结论。

## 2. 目标与非目标

### 2.1 目标

1. 在既有 V2 深蓝 Developer Tool 三栏页面内，补齐 Prompt 与 Memory 的前端产品闭环。
2. Prompt 保持全局规则、多规则同时启用、每条规则多目标、缺失文件状态和未保存修改保护。
3. Memory 保持长期记忆、每日记录、会话记录三类，并严格区分来源、前端草稿、同步预览任务和真实持久化结果。
4. 用一个共享目标资源合同表达 7 个资源覆盖 8 个 Agent 实例、4 个已验证 Memory 目标和 canonical resource 去重。
5. 让 standalone HTML 可靠解析并内联当前 production `dist/index.html` 直接引用的 entry scripts/stylesheets；未来新增 lazy graph 时另行扩展，不在本轮承诺内。
6. 将未来后端接入边界写成 typed port，但本轮不实现 port、不调用 native API。
7. 保持改动可拆分、可独立测试、可按小提交回滚，并降低与并行分支的合并冲突。

### 2.2 非目标

- 不修改 `src-tauri/**`、数据库、Rust command、Tauri payload 或任何真实 Agent 文件。
- 不实现本机扫描、文件写入、SQLite/JSONL adapter、Prompt compose 写入或 Memory durable sync。
- 不修改 `src/v2/pages/agents/**`、`models/**`、`skills/**`、`mcp/**`。
- 不修改 `src/v2/shared/config/navigation.ts`、`src/v2/app/router.tsx`、`src/v2/widgets/app-shell/**` 或现有 V2 Shell 结构。
- 不建立全局 store、service/container 层、通用表单框架或空的 domain hierarchy。
- 不恢复历史的 Codex 单应用 Prompt 模型或“共享/原生/痕迹”Memory 分类。
- 不将匿名化扫描快照描述成当前机器实时状态。

## 3. 现有框架与必须保留的边界

依赖方向保持不变：

```text
app -> pages, widgets, shared, dev (DEV-only)
pages -> shared
widgets -> shared
shared -> third-party packages / shared
```

- `createHashRouter`、六个一级导航顺序、默认 `#/models`、V2 Shell、窗口端口和 Tauri 隔离边界保持不变。
- Prompt 与 Memory 页面继续位于各自 `pages/<route>/`；prototype 数据继续只来自各自 `prototype.ts`。
- 两页共用 `shared/config/agentTargets.ts`；各页面在自身 `Page.tsx` 内使用 React Router `useBlocker` 保护 dirty draft，不得通过 Shell 或导航组件持有页面业务状态。
- 视觉继续采用当前已建立的深蓝 Developer Tool 原型、现有 `--fy-*` token 与三栏工作区。本轮只补必要状态和可达性样式，不重做 V2 全局 token 或 Shell。
- standalone 仍由 `pnpm build:renderer` 生成；生成物不手工编辑。

### 3.1 并行分支冲突控制

仓库另有人员在独立分支实现 Agent 目录、模型、Skills 和 MCP。为使后续合并尽量是加法：

1. 本轮新增测试优先放入 Prompt、Memory 或共享合同专属文件，不继续把业务断言堆入通用 `router-shell.test.tsx` / `shell.spec.ts`。
2. 不改其他四个页面、导航清单、路由表、AppShell、PrimaryNav 和全局 Shell。
3. standalone 构建逻辑从 `dist/index.html` 解析真实入口，不以“最大 JS/CSS 文件”猜入口，避免其他页面加入较大 chunk 后失效。
4. 每个实施线路只修改自己的独占文件；共享合同先锁签名，页面只依赖签名。
5. 设计、三个模块、集成修复分成小提交并只推送当前分支，便于其他分支按需合并或回滚。

## 4. 方案比较

### 4.1 方案 A：维持当前散落 `useState`，只修文案和测试

做法：保留现有数据结构，只把“已同步”换成“待执行”，追加少量条件判断。

优点：代码 diff 最小，短期实现快。

问题：

- 新建 Prompt、提炼 Memory 没有 saved baseline，dirty 判定继续失真。
- Memory 的内容草稿、目标草稿和同步结果仍混在同一个 item 状态里。
- 现有 `confirmDiscard` 只能保护页面内部动作，仍无法保护路由离开。
- canonical path 去重仍是页面文案而非可执行合同。

结论：不采用。它只能隐藏现有错误，不能关闭阶段 A 的 P0/P1。

### 4.2 方案 B：建立全局 store、统一 domain service 和 Shell 级 guard

做法：把 Prompt/Memory/目标/导航阻塞统一放到全局 store 或 Context，由 AppShell 处理离开确认。

优点：状态集中，未来接后端时有统一入口。

问题：

- 会修改 AppShell、导航或 router，与并行分支的冲突面最大。
- Prompt 与 Memory 的状态语义不同，过早统一会产生抽象泄漏。
- V2 目前明确不创建没有真实数据边界的 store/service 层。
- 本轮只是 prototype，无法验证全局持久化抽象是否正确。

结论：不采用。收益尚未成立，且违反最小增量和并行协作边界。

### 4.3 方案 C：页面内显式草稿基线 + 窄共享合同（采用）

做法：

- Prompt 和 Memory 各自在页面模块内维护已保存快照、当前草稿和明确的 `hasSavedBaseline / isNew` 状态。
- 用纯比较/转换函数集中各自的 dirty、保存、放弃、新建/提炼转换，不引入全局 store。
- 仅把两页确实共享的数据合同放入 shared：目标资源/canonical 分组与 Memory 同步资格。路由 guard 留在各自页面，避免并行实现依赖和无必要的导航抽象。
- Memory 将本地条目状态、来源 provenance、预览任务和未来 durable 状态拆开。
- standalone 只增强入口资源解析，不改变 V2 启动框架。

优点：

- 直接关闭当前 P0/P1，且行为可由模块单测独立证明。
- 页面状态不泄漏到 Shell；与其他四个页面并行开发的冲突最小。
- 未来 data source port 可替换 prototype，不要求重写展示组件。
- 共享层只有已经出现两次或跨页面必须一致的合同。

代价：两页仍各自拥有领域状态，少量表单逻辑不会被强行统一。

结论：采用。它是当前约束下最小且完整的方案。

## 5. 模块关系

```text
src/v2/pages/prompts/prototype.ts
        | PromptPrototypeItem seeds
        v
src/v2/pages/prompts/Page.tsx ----\
                                    > shared/config/agentTargets.ts
src/v2/pages/memory/Page.tsx -----/             |
        ^                                      | canonical grouping
        | MemoryPrototypeItem seeds             | verified sync eligibility
src/v2/pages/memory/prototype.ts

scripts/build-v2-preview.mjs <- dist/index.html + dist/assets/*
```

- Prompt 与 Memory 不互相 import；语义归属通过 `owner` 和共享目标合同表达。
- 每页只注册一个 `useBlocker`；Prompt 与 Memory 是互斥一级路由，不会同时产生多个 blocker。
- 目标合同不执行文件系统操作；canonical key 是扫描快照给出的资源身份，不由浏览器解析符号链接。
- standalone builder 只消费 build 输出，不 import 页面业务模块。

## 6. 前端数据模型

### 6.1 Agent 目标资源

```ts
type PromptPathState = "exists" | "create-on-enable";
type MemorySyncEligibility =
  | "source-only"
  | "verified-rule-bridge"
  | "verified-native";

interface AgentTargetDefinition {
  id: AgentTargetId;
  toolId: AgentToolId;
  name: string;
  scopeLabel: string;
  instanceNames: readonly string[];
  promptFile: string;
  promptPath: string;          // 仅展示
  promptCanonicalResourceKey: string; // 仅用于 Prompt instruction resource
  promptPathState: PromptPathState;
  memoryDestination: string;
  memorySyncEligibility: MemorySyncEligibility;
  detected: boolean;
}
```

不再由独立数组手工维护 Memory 可写名单；`memoryWritableTargetIds` 可以从 `memorySyncEligibility !== "source-only"` 派生，避免展示与资格漂移。

### 6.2 Prompt 页面状态

```ts
interface PromptDraftState {
  value: PromptPrototypeItem;
  baseline: PromptPrototypeItem | null;
  hasSavedBaseline: boolean;
}
```

- `items` 只代表已保存到当前前端预览的快照。
- 新建规则可以在列表显示为 transient row，但 `baseline=null`，首次保存前始终 dirty。
- 选中只决定编辑对象；开关仍是每条规则独立动作，不做互斥。
- 目标选择属于规则草稿，保存后进入 `items`。
- 放弃一个从未保存的新建草稿时，transient row 一并移除；不能留下 committed 空项。

### 6.3 Memory 来源、草稿与 provenance

```ts
interface MemoryProvenance {
  sourceItemId: string;
  sourceTargetId: AgentTargetId;
  sourceToolId: AgentToolId;
  sourcePath: string;
  sourceUpdatedAt: string;
  capturedAt: string;
  sourceSummary: string;
}

interface MemoryPrototypeItem {
  // 现有来源字段
  provenance: MemoryProvenance | null;
  resourceState: "exists" | "missing" | "frontend-draft";
  editableInPrototype: boolean;
  syncTargetIds: AgentTargetId[]; // 最后一次保存的目标草稿
  localState: "source" | "saved-preview" | "changes-pending";
  previewTasks: MemoryPreviewTargetTask[];
}

interface MemoryPreviewTargetTask {
  targetId: AgentTargetId;
  previewState: "pending";
  durableState: "not-run";
  createdAt: string;
  error: null;
}
```

- Daily 和 Session 本轮统一是只读来源；即使将来 adapter 报告原文件可写，也需要单独产品入口才能直接编辑。
- 只有可编辑长期记忆和提炼出的长期草稿允许修改。
- 提炼草稿保留完整 `provenance`；草稿自己的 path/state 与来源 path/time 分开。
- prototype 点击同步只生成 `previewTasks`，不得产生 `synced` durable 状态。
- 未来 durable 状态预留 `pending / syncing / synced / conflict / failed / unsupported`，但本轮所有 task 的 durable state 都是 `not-run`。

## 7. 页面状态与数据流

### 7.1 Prompt

```text
prototype seeds -> saved items -> select -> draft + baseline
                                   | edit/target
                                   v
                                dirty draft
                      save ------> saved items
                      discard ---> baseline / remove new draft
```

启用转换：

- saved item 有至少一个 saved target：可以独立切换 enabled。
- 无目标：拒绝启用，选中该条并提示先选择/保存目标。
- 已启用规则的 draft 不能移除最后一个目标。
- 开关是对 saved rule 的即时前端提交：原子更新 saved item 的 `enabled`。若切换的是当前项，只更新 `draft.value.enabled` 与 `baseline.enabled`，不得用整个 item 覆盖 draft 或 baseline 的其他字段。
- clean 当前项切换后保持 clean；已有名称/正文/目标草稿时，切换后仍只由这些未保存字段保持 dirty，`DISCARD` 恢复它们但保留已经提交的 enabled 状态。
- dirty 时条目、新建和路由离开共用同一确认语义。
- `useBlocker` 只处理 SPA 路由离开；本轮不扩展到刷新、关窗或跨域导航。

### 7.2 Memory

```text
prototype source -> selected source -> draft + saved target baseline
                         | long-term editable only
                         v
                      dirty draft --save--> saved-preview revision

daily/session source --promote--> unsaved long-term draft + provenance
saved-preview revision + >=1 saved targets --preview sync-->
                         per-target pending tasks (durable not-run)
```

- 目标选择计入 dirty；未保存目标不能用于生成任务。
- 新提炼草稿首次保存前不能生成任务。
- 切分类、切来源、扫描、再次提炼或离开路由均检查 dirty。
- 生成任务不修改来源内容、不修改 durable 状态、不声称文件写入。

## 8. Prototype 数据源边界

- `prototype.ts` 只保存匿名化结构、固定示例正文、来源能力与演示计数。
- 页面根节点继续标记 `data-data-source="prototype"`，并新增用户可见的“前端原型 · 未读取或写入本机文件”。
- Prompt 标题说明改为“在前端预览中组合长期规则”；托管区块改为未来条件句。
- Memory 标题说明改为“用匿名化结构预览来源”；“重新扫描本机”反馈必须是模拟结果。
- prototype seed 不使用未经外部回读证明的“已同步”。
- 不在仓库写入 `/Users/<name>` 绝对路径、私人 Prompt/Memory 正文、凭据或完整会话。

## 9. 未来后端适配接口（仅设计）

```ts
interface PromptDataSource {
  listRules(): Promise<PromptRuleProjection[]>;
  saveRule(input: SavePromptRuleInput): Promise<PromptRuleProjection>;
  previewComposition(resourceIds: string[]): Promise<PromptPreviewResult[]>;
  syncResources(resourceIds: string[]): Promise<PromptTargetResult[]>;
}

interface MemoryDataSource {
  listResources(input: MemoryResourceQuery): Promise<MemoryResourcePage>;
  readResource(resourceId: string): Promise<MemoryResourceContent>;
  saveLongTermDraft(input: SaveMemoryDraftInput): Promise<MemoryDraftProjection>;
  promote(input: PromoteMemoryInput): Promise<MemoryDraftProjection>;
  previewSync(input: PreviewMemorySyncInput): Promise<MemoryTargetPreview[]>;
  syncTargets(input: SyncMemoryTargetsInput): Promise<MemoryTargetResult[]>;
}
```

适配规则：

- 页面只消费 typed projection，不解析 raw Tauri/SQLite/JSONL payload。
- 扫描返回 resource/capability，正文按 resource ID 延迟读取。
- `preview` 与 `sync` 是不同方法和不同结果类型；调用 preview 永远不能更新 durable sync result。
- 真正 port 将来放入 V2 shared domain/platform 边界；本轮不创建空实现或 fake native adapter。

## 10. Canonical resource 去重

### 10.1 Prototype 规则

1. 每个目标带 `promptCanonicalResourceKey`，格式是用户目录占位符后的规范化、大小写策略明确的 Prompt instruction resource 键。
2. Prompt 目标选择先按 ID 取资源，再按该 key 分组；Memory 的 4 个 adapter/scope 目标组不使用 Prompt key 推断 `MEMORY.md` / `USER.md` 文件身份。
3. 每组保留第一个稳定 resource ID，并对 `instanceNames` 去重合并。
4. 资源任务数量取分组数；覆盖实例数量取所有分组实例并集。
5. 当前快照必须得到 7 个唯一 Prompt 资源、8 个实例；OpenClaw 默认 workspace 只出现一次并覆盖 `main + utility`。

### 10.2 未来扫描规则

- `~` 展开、绝对化、`.` / `..` 消解、separator 统一、符号链接解析和文件系统大小写由后端扫描层负责。
- 前端不得尝试通过字符串替换模拟 `realpath`。
- 相同 canonical path 的扫描项合并 resource，但保留所有 instance IDs 和原始展示路径。
- 权限不足或 realpath 失败时返回稳定 resource ID 和 `canonicalizationStatus`，不得默默当成两个可写目标。

## 11. 只读来源、可写目标与所有权

| 对象 | 页面行为 | 本轮写入 |
| --- | --- | --- |
| Prompt instruction/identity | Prompt 可编辑规则草稿与目标 | 仅保存到 React 状态 |
| Prompt 缺失目标 | 显示“启用时创建” | 不创建文件 |
| Memory long-term Markdown projection | `editableInPrototype=true` 时可编辑 | 仅保存到 React 状态 |
| Memory daily/session | 只读、可搜索/提炼 | 不改原来源 |
| 派生 SQLite/JSONL | 只读来源 | 不写 |
| Prompt-owned context | Memory 只读并引导 Prompt | 不写 |
| 4 个已验证 Memory 目标 | 可选择并生成 preview tasks | durable `not-run` |
| Codex/Gemini/OpenCode 未验证存储 | 来源或不可用 | 不显示为同步目标 |

## 12. 错误、冲突与缺失状态

本轮可真实触发的状态：

- 本地校验：名称为空、启用无目标、已启用移除最后目标、同步无目标、同步前草稿未保存。
- 草稿冲突：用户尝试切条目/分类/路由、扫描或提炼时存在 dirty。
- 资源状态：`exists`、`create-on-enable`、`frontend-draft`、只读、Prompt-owned。
- prototype 反馈：保存到前端预览、模拟扫描、生成待执行任务。

未来 adapter 状态（只在类型/文档预留，不在本轮伪造）：

- 扫描：loading、empty、partial、permission denied、invalid format、unavailable。
- Prompt/Memory target：pending、syncing、synced、conflict、failed、unsupported。
- canonicalization：resolved、unresolved、ambiguous。
- partial：逐目标结果汇总得出，不用单个 success boolean。

## 13. 响应式与可访问性原则

- 保持 900×600、1152×640、1232×700、1440×900 四档支持；页面级不产生横向溢出。
- 三栏在宽屏保持三列；窄屏使用页面已有两列/单列与面板内部滚动，不改变 Shell。
- 主操作、列表项、switch、checkbox、tab、表单和确认流程保持键盘可达。
- page-level blocker 由路由状态驱动，取消后焦点保留在当前页面；确认后才继续导航。
- `aria-live` 的反馈必须区分“保存到前端预览”“待执行任务”和真实 durable 状态。
- 路径状态不能只靠颜色；使用明确文本“已存在 / 启用时创建 / 前端草稿”。
- 保留 `prefers-reduced-motion`，不增加 `transition: all` 或布局动画。

## 14. 兼容性、迁移与回滚

### 14.1 兼容性

- Prompt/Memory 路由、组件导出名和现有 V2 import 方向不变。
- prototype 数据字段是 V2 内部合同，没有旧后端兼容承诺；测试与两页同步迁移。
- 共享 target ID 保持不变，避免现有种子和测试大范围重写。
- standalone 文件名、`file://` 默认 `#/prompts` 和 normal Vite/Tauri 启动方式不变。

### 14.2 迁移

1. 先扩展共享目标合同并保留兼容派生导出。
2. Prompt 迁移到显式 saved baseline。
3. Memory 迁移到 provenance、resource/local state 和 preview tasks。
4. 页面/测试不再读取 durable“已同步” prototype seed。
5. standalone builder 改为解析真实 entry assets。

### 14.3 回滚

- 设计、共享合同、Prompt、Memory、集成修复使用独立小提交；需要回退时按提交逆序回滚。
- prototype 没有数据迁移或真实文件副作用，回滚不会丢本机 Agent 数据。
- standalone 是生成物；回滚源代码后重新生成，不手工修补产物。
- 不依赖 `src-tauri`，所以回滚不涉及数据库 schema 或 Rust command。

## 15. 风险与取舍

| 风险 | 控制 | 取舍 |
| --- | --- | --- |
| 页面状态继续复杂化 | 显式 baseline/provenance/preview task；转换集中在页面模块 | 不引入全局 store |
| route blocker 影响所有导航 | 每页仅在 `isDirty=true` 且 pathname 将改变时阻塞；取消调用 `reset`，确认调用 `proceed` | 不修改 PrimaryNav/Shell，也不拦同页状态变化 |
| canonical key 被误当 realpath | 文档明确 key 来自扫描层；prototype 只验证分组合同 | 本轮不做文件系统解析 |
| 可见路径暴露本机信息 | prototype 只用 `~` 匿名路径；真实接入由 projection 决定展示 | 保留用户理解目标所需的路径语义 |
| 其他页面新增较大 chunk 破坏 standalone | 从 `dist/index.html` 解析 entry，不猜最大文件 | 脚本逻辑略复杂但可验证 |
| 新模型与旧“已同步”截图不同 | 真实性优先，旧 screenshot 只作历史快照 | 重新生成本轮 runtime evidence |
| 并行分支修改通用测试 | 新增专属测试；不继续改其他页面与 Shell | 已有未提交通用测试改动保留，不再扩张 |

## 16. 架构验收条件

- [x] 技术架构评审确认没有修改 CC Switch/FyAgent 框架、V2 Shell、导航或 Tauri 边界。
- [x] 目标资源、Memory 资格和 canonical 分组只有一个共享事实源。
- [x] Prompt 与 Memory 的 saved baseline、dirty 和路由离开语义明确。
- [x] Memory preview task 与 durable result 是不同类型和不同状态。
- [x] Daily/Session 只读，提炼保留 provenance。
- [x] 未来 backend port 仅停留在设计，没有创建假实现或 native 调用。
- [x] 并行分支冲突控制进入详细文件所有权和测试布局。
- [x] P0/P1 产品评审意见均能映射到本文或后续详细设计的明确关闭项。
