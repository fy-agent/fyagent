# AI 软件目录渐进扫描、安装更新与侧栏动效

## Goal

让 `/agents` 的「我的 AI 软件」成为一个始终有内容、状态逐步变得可信的目录：目录加载后立即显示全部受支持软件，首次进入自动在后台读取每个软件的安装/更新状态，按真实扫描结果开放配置或安装/更新动作，并让用户看到动作进度。同时让左侧「配置管理」展开/收起拥有与现有 SelectionLens 一致的顺滑弹簧手感。

本任务的主要价值是把已经存在的 catalog、readiness、installer/action、Motion、Radix 和 TanStack Query 能力正确组合起来，而不是新增第二套扫描器、安装器、软件注册表或动画体系。

规划采用「稳定边界 + 推荐实现 + 可替换细节」：用户可见行为、真实性、安全边界、复用原则和串行阶段目标属于稳定要求；内部 hook 名称、组件颗粒度、状态字段命名、少量布局细节可在实现时按最新代码调整。

## Background / Confirmed Facts

- 当前生产 V2 Agent 目录来自唯一 `AgentCatalog`，固定支持 7 个 catalog ID；不应增加第二份前端软件表。
- 当前 `AgentDirectory` 在未扫描时为空，并且扫描中/完成后只投影已安装项，这正是本次要改变的用户体验。
- 当前 `useAgentDirectoryScan` 已通过 TanStack Query 的 disabled queries + `refetch()` 读取 7 个 `AgentInstallReadiness`，有 request id、partial settle、失败保留旧成功结果等基础能力。
- `AgentInstallReadiness` 已提供 `installState`、`updateState`、`releaseId`、`allowedActions` 和 reason codes；renderer 不需要也不允许猜测 URL、命令、安装路径或 package format。
- 通用 Agent action façade 已提供 `start_agent_action` / `get_agent_action_job` / `cancel_agent_action`。Job snapshot 当前提供阶段而非下载百分比。
- Codex 安装/更新仍由独立 `codexDesktop` port 和 `useCodexDesktopInstaller` 管理；该 view model 已有真实下载百分比、字节数/速度、事件订阅和终态刷新能力。通用 Agent façade 对 Codex install/update 明确返回 `managed_by_codex_desktop`。
- 当前 `AgentInstallReadinessSection` 已实现通用 Agent action 的 start → poll job → terminal → readiness reread 流程，但没有接入目录；应优先抽取/复用这段能力，而不是再写一份轮询器。
- 当前 SelectionLens 使用 `framer-motion` physics spring：`stiffness: 520`、`damping: 42`、`mass: 0.62`。侧栏内容本身仍由 `hidden` 立即切换，没有展开/收起动画。
- 项目已经依赖 `@radix-ui/react-collapsible`、`framer-motion` 和 `@tanstack/react-query`。V2 不能运行时导入 legacy `src/components/ui/collapsible.tsx`，但可以在 V2 shared owner 中复用已采用的 package 能力。
- 当前工作树存在与 V3 Agent/Shell 相关的未提交变更。实施前先识别并保护这些现有改动；本任务后续只使用一个开发分支，不创建并行 Worktree，也不覆盖用户未授权的变更。

## Requirements

### R1 — 目录先显示，状态后填充

- Agent catalog 成功加载后，目录按现有 catalog / `PRODUCT_DIRECTORY` 的权威顺序显示全部受支持软件，不因尚未扫描、未安装、unknown 或单项读取失败而把软件从目录中移除。
- 首次进入目录视图自动开始一次后台扫描。用户不需要先点击「开始扫描」才能看到软件或触发第一次状态读取。
- 扫描应是渐进式体验：单项完成即可更新该行，不需要等待全部 7 项结束后统一揭示结果。
- 「lazy / 渐进」的核心要求是 display-first + background readiness，而不是强制一种调度算法。实现可以保持并发，也可以在有证据时做有限并发；不得仅为了“看起来 lazy”自造复杂任务调度器。
- 页面存活期间返回目录时保留已有结果；首次自动扫描只触发一次，仍保留用户可操作的「重新扫描」。

### R2 — 每行状态与配置门禁

- 第一次状态尚未返回的行必须明确显示「正在扫描」或等价状态；「进行配置」不可点击。
- 扫描确认 `installed` 或既有兼容语义 `installed_not_runnable` 后，可按既有配置能力开放「进行配置」。
- `not_installed`、`unknown`、`unavailable`、首次读取失败或没有可信 readiness 的行不得开放「进行配置」。
- 重新扫描期间如果存在上一次成功的安装结果，可以保留已知可配置状态，同时显式展示正在刷新；不要求为重新扫描制造整页闪烁或把已有成功结果清空。
- 单项技术失败必须与「未安装」区分；失败不能被渲染成绿色/已安装，也不能偷偷移除该软件。

### R3 — 一键安装 / 一键更新

- 每行操作区以 backend readiness 的 `allowedActions` 和专用 Codex owner 为唯一权限来源。
- 扫描确认未安装且后端允许 `install` 时，在「进行配置」左侧显示「一键安装」；扫描确认已安装且 `updateState === update_available` 且后端允许 `update` 时显示「一键更新」。
- 已安装且无需更新时不显示多余的一键安装按钮。
- backend 明确不允许当前平台/来源的一键安装时，前端不得因为产品文案而伪造一个可点击按钮。可以显示受控的不可用原因或只保留状态；具体文案由集成任务结合现有 reason copy 收敛。
- Codex install/update 必须继续复用现有 `codexDesktop` installer owner；不得把 Codex 塞进通用 Agent job，也不得复制 Codex 下载/校验逻辑。
- 其他 Agent 必须复用现有 `agentInstallReadiness.startAction/getActionJob/cancelAction`；不得增加第二套 invoke、下载器或安装命令表。

### R4 — 真实进度和终态回读

- 点击安装或更新后，用户必须在对应行感知当前动作正在进行；动作区至少展示 spinner + 当前阶段文案。
- 通用 Agent job 只提供 stage 时，只展示真实 stage（checking / downloading / installing / verifying 等），不得伪造百分比。
- Codex 已有真实 determinate progress 时继续展示已有百分比/下载信息，允许目录卡片用更紧凑的投影，但不得重新计算一套不一致的进度。
- 动作成功不能直接把 UI 乐观改成「已安装」。终态后必须重新读取权威 readiness / Codex local status；只有回读确认后才开放配置或移除安装/更新按钮。
- 失败、取消、`operation_conflict`、`refresh_required` 等保持可理解且可恢复；具体重试按钮形式可由实现结合现有 Button/notice 组件决定。

### R5 — 配置管理展开/收起动效

- 左侧「配置管理」展开/收起不再使用瞬时 `hidden` 切换作为唯一视觉反馈。
- 运动手感复用现有 SelectionLens physics spring，而不是新增独立 cubic-bezier 或另一套运动常量。
- 优先使用项目已安装的 Radix Collapsible 语义能力和现有 Motion 能力；不得引入新的动画/Disclosure 依赖。
- 保留当前 Router 选中、`aria-expanded`、ArrowRight / ArrowLeft / Escape / Home / End 等键盘行为。
- `prefers-reduced-motion` 下必须即时或近即时切换，不能强迫用户观看 layout animation。
- 允许实现阶段把 motion token 抽到更合理的 shared owner、增加一个薄 V2 shared collapsible adapter，或采用等价的 shared 组合；不要把具体文件名固化为产品契约。

### R6 — 复用与快速迭代约束

- 搜索/复用优先级：现有 shared owner → 已采用依赖 → 维护良好的开源组件 → 才考虑自研。
- 本轮调研已证明现有依赖足够，因此默认不新增 npm package / Rust crate。
- 执行代理不需要重新做一轮宽泛技术选型；以本任务 research/design 为默认方案。只有代码已变化、现有 owner 不能满足、或出现新的兼容/安全问题时，再做针对性补充调研并记录证据。
- 规划不锁死内部命名或微观实现。如果执行阶段发现更小、更复用、且不改变本 PRD 可观察行为的实现，可自行调整并在 review 中说明。

### R7 — 单任务串行开发形态

- 本需求只保留当前这一个 Trellis 任务，后续在同一个开发分支中完成，不创建子任务或并行 Worktree。
- 推荐按以下顺序推进：侧栏动效 → 渐进扫描状态 → 生命周期动作/进度复用 → Agent Directory 卡片接线 → 集成测试与 SPEC 收口。
- 每个阶段完成后先运行 focused validation，再进入下一阶段。阶段边界用于降低回归和方便定位问题，不要求人为制造独立公共 API 或提交。
- 如果最新代码结构表明相邻两个阶段合并实现更简单，可以做最小调整；不要为了遵守阶段形式重复包装、制造 adapter 或抽象。
- 整个任务由同一执行方持续维护上下文，最终统一完成页面/浏览器验收和 spec 更新。

## Out of Scope

- 新增支持软件、改变 7 个 Agent catalog 的产品范围或顺序。
- 重写后端 Agent installer/source policy、放宽 Windows 安全边界、给 renderer 暴露 URL/path/hash/命令。
- 为通用 Agent job 新增虚假百分比或仅为本 UI 扩大 backend DTO；若实现中证明真实数值进度是必要的新产品需求，应单独评审。
- 重构整个 Agent configuration 四分区、Models/Skills/MCP/Prompts 的业务写入路径。
- 新增第二个设计系统、动画库、query 库或安装器框架。

## Acceptance Criteria

- [ ] Catalog 成功后，无论是否扫描，7 个支持软件均按权威顺序显示；初始列表不再为空。
- [ ] 首次进入目录自动扫描；每行有独立的 pending/scanning → settled 反馈，全部完成前已完成行可以先更新。
- [ ] 首次未确认/未安装/失败的软件「进行配置」不可用；已确认安装的软件按既有语义可进入配置。
- [ ] 未安装 + backend 允许 install 的行显示「一键安装」；可更新 + backend 允许 update 的行显示「一键更新」；不允许的动作不被前端伪造。
- [ ] 一键安装/更新点击后，对应行显示真实阶段；Codex 复用现有真实 determinate progress，通用 Agent 不伪造百分比。
- [ ] 安装/更新终态后重新读取权威状态；没有成功回读时不提前开放配置或宣称成功。
- [ ] Codex 继续走 `codexDesktop` owner；其他 Agent 继续走 `agentInstallReadiness` action façade；无第二套下载器、invoke wrapper 或软件注册表。
- [ ] 「配置管理」展开/收起使用与 SelectionLens 同源的 spring 手感，并保留键盘、ARIA 与 reduced-motion 行为。
- [ ] 未新增 animation/disclosure/query 依赖；新增 shared 能力（如有）有明确复用理由且没有复制现有组件。
- [ ] 全部实现只在一个开发分支中串行完成，不创建并行 Worktree/子任务；现有未提交产品改动得到识别和保护，没有被覆盖。
- [ ] `mise run typecheck:v2`、相关 Vitest、`mise run test:v2:browser`（受影响 Agent/Shell 场景）和 `mise run build:renderer` 通过；最终整体验收按当时项目 gate 执行。

## Risks / Deferred Items

- 通用 Agent job 当前只有阶段，没有数值型进度；本轮明确接受 stage progress，不把“感知到进度”错误解释成伪百分比。
- 某些平台/Agent 的 backend 本身会拒绝一键安装（例如受安全约束的安装格式）。UI 必须忠实反映 `allowedActions`；如果产品之后要求所有平台都可一键安装，这是单独的 backend 能力项目。
- 当前相关 V3 改动尚未全部提交。开始产品代码实现前必须先检查这些改动与本任务的关系，并在同一开发分支内兼容处理；不要通过额外 Worktree 规避冲突。

## Planning Evidence

- 见 `research/planning-evidence.md`。该文件记录本地代码证据、现有复用点以及 Radix / Motion / TanStack Query 官方资料。
