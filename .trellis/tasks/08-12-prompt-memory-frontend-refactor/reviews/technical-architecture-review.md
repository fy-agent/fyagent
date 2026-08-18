# Prompt / Memory 技术架构静态评审

- 评审阶段：B（技术架构设计与评审）
- 评审日期：2026-08-12
- 证据等级：`code_audit`（仅文档与代码静态审阅）
- 主评审对象：`technical-design-overview.md`
- 对照范围：`prd.md`、现有 `design.md`、`.trellis/spec/frontend/v2-shell.md`、Prompt / Memory / `agentTargets` / router / navigation / AppShell / standalone 相关源码
- 执行边界：本评审未运行 lint、typecheck、unit/integration/browser test、Playwright、build、dev server、截图或 pixel diff
- 写入边界：仅新增本评审文档；未修改任务状态、实现源码、测试、规范或其他评审文档

## 1. 结论

`ARCHITECTURE_REVIEW=PASS`

未发现新的 P0 或 P1 技术架构阻断项。技术概要延续了现有 CC Switch / FyAgent 的 V2 分层、Hash Router、六路导航、AppShell、窗口端口与 Tauri 隔离边界；采用“页面内显式草稿基线 + 窄共享合同”，没有引入全局 store、service/container、空 domain 层或跨页面业务状态，属于能够关闭阶段 A 既有产品缺口的最小增量方案。

本次记录 4 个 P2 和 1 个 P3。P2 均可在详细设计中通过锁定字段语义、状态转换和文件所有权关闭，不要求改动 Agent 目录、模型、Skills、MCP、`src-tauri`、router、navigation 或 AppShell，因此不阻断阶段 B 通过。该 PASS 仅表示技术架构方案可进入详细设计；阶段 A 产品评审中记录的实现缺口仍须按原评审完成实现与静态复审，不能因本结论被视为已经关闭。

## 2. 架构核对矩阵

| 核对项 | 静态证据 | 结论 |
| --- | --- | --- |
| 延续 CC Switch / FyAgent 与 V2 架构 | `technical-design-overview.md:28-53` 保留现有目录、依赖方向、Hash Router、六路导航、V2 Shell、窗口端口和 Tauri 边界；与 `v2-shell.md:16-28,210-227` 一致 | 通过 |
| 避免不必要抽象或重构 | 技术概要比较三种方案并排除全局 store / Shell 级 guard；采用页内状态与窄 shared 合同（`technical-design-overview.md:65-116`） | 通过 |
| Prompt / Memory 状态归属 | 两页分别持有 saved snapshot、draft、baseline；Prompt 与 Memory 不互相 import，Shell 不持有业务状态（`technical-design-overview.md:97-138,169-259`） | 通过，见 P2-02 |
| 共享 target / canonical / Memory 资格 | `agentTargets` 作为唯一 scope catalog，资格从字段派生，canonical 分组为纯前端合同；未来 realpath 归后端扫描层（`technical-design-overview.md:140-167,297-312`） | 通过，见 P2-01 |
| `src-tauri` 与 native 边界不变 | 非目标明确排除 Rust、数据库、command、payload 和真实文件写入；typed port 仅设计、不创建实现（`technical-design-overview.md:28-36,270-295`） | 通过 |
| router / navigation / AppShell 不变 | 非目标明确列出 `navigation.ts`、`router.tsx`、`widgets/app-shell/**`；每页在自身 `Page.tsx` 内只注册一个 `useBlocker`，只拦 pathname 改变，取消 `reset`、确认 `proceed`，不把状态提升到 Shell，也不扩展到刷新/关窗（`technical-design-overview.md:32-34,49-52,134-136,240-241,379-383`） | 通过 |
| Agent / models / skills / MCP 并行分支保护 | 明确不改四个页面，业务断言进入专属测试，模块独占文件，小提交只在当前分支推进（`technical-design-overview.md:55-63`） | 通过，见 P2-04 |
| Prototype 与未来 adapter 边界 | seed 只含匿名化结构；preview 与 durable sync 类型分离；页面未来只消费 typed projection，不解析 raw Tauri / SQLite / JSONL（`technical-design-overview.md:261-295`） | 通过 |
| 只读来源与可写目标 | Daily / Session 只读，Prompt-owned 只读，4 个已验证 Memory 目标只生成 preview task，未验证存储不列为目标（`technical-design-overview.md:217-221,314-325`） | 通过 |
| standalone 边界 | builder 只消费构建输出，不 import 页面业务；从 `dist/index.html` 识别入口，生成物不手改（`technical-design-overview.md:53,61,131-137`） | 通过，见 P2-03 |
| 视觉与响应式边界 | 保留现有深蓝 Developer Tool、`--fy-*` token、三栏页面与 Shell，不做全局视觉重构；符合 `v2-shell.md:195-208` | 通过 |

## 3. 分级发现

### P0

无。

### P1

无。

### P2-01 `canonicalResourceKey` 必须明确是 Prompt 资源身份，不能拿来给 Memory 目的地去重

- 证据：拟议的 `AgentTargetDefinition` 只有一个泛称的 `canonicalResourceKey`，同时携带 `promptPath` 和 `memoryDestination`（`technical-design-overview.md:151-164`）；去重算法最终又明确验收“7 个唯一 Prompt 资源”（`technical-design-overview.md:301-305`）。现有 catalog 中 Claude 的 Memory 目的地是目录，而 OpenClaw / Hermes 的一个目标组包含 `MEMORY.md` 与 `USER.md` 两种语义资源（`src/v2/shared/config/agentTargets.ts:45-54,81-114`），它们不是 Prompt 文件 canonical path。
- 风险：若实现把泛称 key 同时用于两页，Memory 可能按 `AGENTS.md / SOUL.md` 路径去重，或者把 `USER.md` 与 `MEMORY.md` 错当成同一个文件资源；这会把正确的“4 个已验证目标组”误解成 4 个具体文件。
- 建议：详细设计将字段命名为 `promptCanonicalResourceKey`，或用等价注释锁死其仅服务 Prompt instruction resource；Memory 本轮继续按 4 个已验证 adapter/scope 目标组展示，未来真实写入再使用扫描返回的 Memory `resourceId + canonicalPath + semanticType` 去重。不要为此新建第二套 Agent catalog。
- 是否阻断：否。技术概要的去重验收已经明确限定为 Prompt，补足字段语义即可。

### P2-02 Preview task 必须绑定已保存 revision，不能与后续草稿产生陈旧耦合

- 证据：Memory 模型把 `syncTargetIds` 定义为最后一次保存的目标草稿，并把 `previewTasks` 存在 item 上（`technical-design-overview.md:198-214`）；数据流规定只有 saved-preview revision 与 saved targets 可以生成任务（`technical-design-overview.md:243-259`）。但尚未定义任务生成后再次编辑、保存、放弃或改目标时，旧任务如何处理。
- 风险：旧 `previewTasks` 可能继续显示为当前内容的待执行任务，实际却基于旧正文或旧目标；这会重新混淆 draft、saved preview 和 sync preview 三层状态。
- 建议：详细设计选择一个最小规则并写入状态表：任务保存 `sourceRevision`，或在正文/标题/目标形成新的 saved revision 时清空旧 preview tasks。无需引入全局 revision service；页面内递增 revision 或保存时失效即可。放弃草稿不得改变最后一次已保存 revision 及其任务。
- 是否阻断：否。状态所有权已正确留在 Memory 页面，补状态转换即可。

### P2-03 standalone 的承诺应限定到可实现的 entry graph

- 证据：当前脚本按体积选择最大 JS/CSS（`scripts/build-v2-preview.mjs:12-42`），技术概要正确提出改为读取 `dist/index.html` 的真实入口（`technical-design-overview.md:24,61,132-138`）。但“构建产物拆 chunk 后仍可靠”的措辞尚未说明是否包含动态 import 产生的非 HTML 直链 chunk。
- 风险：只解析一个 `<script>` 和一个 stylesheet 可以修复“最大文件猜测”，但不能天然保证任意 lazy chunk 都被内联到单 HTML；详细实现若不区分两者会留下超出证据的完成声明。
- 建议：详细设计至少要求解析 `dist/index.html` 中全部 entry script / stylesheet 直链并内联其静态资源；若生产路由出现动态 chunk，则明确递归重写/内联 import graph，或把本轮兼容承诺限定为当前生产 entry graph。不得修改 V2 router、其他四页或手工补生成 HTML 来规避。
- 是否阻断：否。当前 Prompt / Memory 入口是静态依赖，所选方向能消除现存按体积猜测问题。

### P2-04 冻结前要消除权威文档中的旧 executable shape，并锁定独占文件

- 证据：技术概要声明 `design.md` 只保留权威入口和短摘要（`technical-design-overview.md:14`），但现有 `design.md:133-175` 仍把 Memory 定义为 item 级 `MemorySyncState` 并包含“已同步”；`v2-shell.md:55-93` 也仍将旧 shape 标为 executable signature。它们与新设计的 `localState + previewTasks + durable not-run` 不一致。
- 风险：并行执行 Agent 读取不同文档时会实现两种状态模型；同时 standalone/global 文件若没有独占归属，可能与其他页面分支产生不必要冲突。
- 建议：详细设计和 execution plan 中把 `agentTargets`、Prompt 页内 guard、Memory 页内 guard、standalone builder、相关专属测试分别列入各自单一 owner；实现前将 `design.md` 改成新概要的入口/摘要。`v2-shell.md` 的 Prompt / Memory executable shape 在最终规范回写时同步到冻结模型，只改对应段落，不触碰导航、窗口端口、Agent / models / skills / MCP 合同。
- 是否阻断：否。属于设计冻结与并行派发前的文档/所有权收口条件，不要求改变架构方案。

### P3-01 共享 lookup 不应长期静默回退到首个 Agent target

- 证据：当前 `agentTargetById` 找不到 ID 时回退 `agentTargets[0]`（`src/v2/shared/config/agentTargets.ts:145-147`）。静态 union 和当前 seed 下通常不会触发，但未来 adapter projection 或数据迁移会扩大输入边界。
- 风险：无效 target 可能被静默显示成 Codex，掩盖来源/目标合同错误。
- 建议：共享合同实施时优先返回 `undefined` 并由调用方显式处理，或在仅接受内部闭集的 helper 中抛出清晰 invariant error；增加共享合同聚焦用例即可。不要为此引入 registry/service 抽象。
- 是否阻断：否。

## 4. 对阶段 A 阻断项的架构承接

技术概要已为阶段 A 的严重问题提供正确落点：

1. 预览同步冒充真实同步：拆为 `previewState=pending` 与 `durableState=not-run`。
2. item 级总状态无法表达逐目标结果：改为 `previewTasks[]`，未来 durable result 独立。
3. 路由离开丢草稿：Prompt 与 Memory 各自在自身 `Page.tsx` 使用一个 `useBlocker`，只拦 SPA pathname 变化，不改 Shell、router 或 PrimaryNav。
4. 新建/提炼没有 saved baseline：显式 `baseline=null / hasSavedBaseline=false`。
5. 提炼丢 provenance：保存独立 provenance，草稿 path/time 不覆盖来源 path/time。
6. Daily 可编辑：Daily / Session 统一只读，只通过提炼生成可编辑长期草稿。

这些是对既有缺口的最小修复，不是新架构层，也不需要回接 V1 Prompt API 或 legacy 组件。

## 5. 详细设计与实施必须锁定的边界

- 只允许 Prompt、Memory、共享 target/guard、standalone 及其专属测试产生实现改动。
- `src-tauri/**`、数据库、Rust command、真实文件写入保持零改动。
- `src/v2/pages/agents/**`、`models/**`、`skills/**`、`mcp/**` 保持零改动。
- `src/v2/shared/config/navigation.ts`、`src/v2/app/router.tsx`、`src/v2/widgets/app-shell/**` 保持零改动。
- 不建立全局 store、Context service、空 adapter 实现、domain hierarchy 或通用表单框架。
- Prompt / Memory 页面只依赖冻结后的 shared 签名，不互相 import，不让 standalone builder import 页面业务。
- canonical 去重只由纯合同/纯函数处理；浏览器不做 `realpath`，未来真实 canonicalization 归扫描 adapter。
- 每个模块使用独占文件并形成可独立回滚的小提交；集成修复由主 Agent 统一处理，不让并行执行者越权修改公共壳。

满足以上边界并在详细设计中关闭 P2 后，可以冻结技术设计并进入按模块并行实施。

## 6. 实施后证据回读（2026-08-13）

- shared target、Prompt、Memory 与 standalone 均按冻结依赖方向落地；architecture tests、lint、typecheck 全部通过。
- baseline-to-HEAD 与 worktree 双层审计确认 `src-tauri`、其他四页、navigation、router、AppShell 零改动。
- 架构评审结论维持 `ARCHITECTURE_REVIEW=PASS`；最终保护和冻结 hash 证据见 `research/pre-implementation-protected-hashes.md`。
