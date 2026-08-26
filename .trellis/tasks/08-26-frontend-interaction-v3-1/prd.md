# Frontend Interaction V3.1 High-Fidelity Refactor

## Goal

在分支 `codex/frontend-interaction-v3-1-20260826` 上，按 11 张已批准高保真原型和用户逐字提供的飞书原文完成 V3.1 前端重构。Antigravity / Gemini 3.7 是前端实现 owner，Cursor / Grok 4.6 负责后端、共享组件边界、调研、复杂度挑战与强制评审，Codex / `gpt-5.6-sol` / `max` 负责总调度、验证、桌面操作、截图、透明资产与经单独授权的对外动作；真实业务能力、数据连接和工程质量必须保持可用。

## Authority

冲突按以下顺序裁决：

1. 用户 2026-08-26 最新消息中明确给出的 Gemini 前端实施分工；
2. [飞书原文逐字留档](./source-feishu-verbatim.md)及[当前执行主体权威](./research/executor-authority-latest-gemini-primary.md)；
3. `docs/fyagent/design/frontend-interaction-v3/01-*.png` 至 `11-*.png`；
4. 当前真实业务端口、路由和数据契约；
5. 当前 V3 代码与历史材料。

原文文件逐字保留。最新消息把上一版“Codex 主实施前端、Gemini 仅协作”的执行结论标记为 `SUPERSEDED_DO_NOT_EXECUTE`；产品目标、界面要求、交互规则、高保真要求、Grok 门槛和禁止事项全部保持有效。

## Confirmed Facts

- 旧任务 `.trellis/tasks/08-26-frontend-interaction-v3-e2e` 已标记 `SUPERSEDED_DO_NOT_EXECUTE`；旧 Windows 等待、证据收口、对外图文与 `0ad9a7e1` 候选口径已经停止。
- 新分支从干净的旧 V3 HEAD 创建，`main` 未被切换或修改。
- 仓库内 11 张 PNG 原型均可访问；核心目标视口为原型图的桌面横向布局。
- `mise run env:check` fresh pass；原生 `mise run dev` 在清除注入的 `NODE_PATH` 后成功编译并启动 FyAgent v0.4.2。
- 当前代码确实包含原型外的页面眉题、说明、状态提示、完整介绍入口、能力说明和额外披露区；扫描完成态会显示全部目录项，并显示 `unknown` / `not_installed` 文案。
- 当前扫描状态源位于 `useAgentDirectoryScan.ts`，页面投影位于 `AgentDirectory.tsx`；现有底层端口无需新增 API 即可完成结果过滤和错误分流。
- Antigravity 当前可用模型包含 `gemini-3.7-flash-high`。
- Cursor Agent CLI 已安装，当前命令行身份未认证；本机 Grok Build CLI 已确认 `grok-4.6` 可用。用户已授权 Grok Build 作为候补评审通道。
- 用户最新消息明确前端实现必须交给 Antigravity / Gemini 3.7，Cursor / Grok 4.6 负责后端、组件、调研和复杂度门槛，Codex 不接管页面实现。

## Requirements

### R1｜分支与状态边界

- 所有实现只发生在 `codex/frontend-interaction-v3-1-20260826`。
- 本轮不切换、提交、合并或发布 `main`。
- 本轮不继续旧 V3 的 Windows 等待、旧证据收口或对外图文。
- `0ad9a7e1` 仅保留为历史代码基线，不进入 V3.1 完成或候选判定。
- push、PR、merge、tag、Release、production 和对外发布均需另行授权。

### R2｜执行分工

- Antigravity / Gemini 3.7 是前端页面、布局、样式、文案、状态反馈、前端状态逻辑、局部 UI variant 与相关前端测试的唯一实施 owner；两个波次及所有 UI 返工均回到该路由。
- Cursor / Grok 4.6 负责后端与共享组件边界、扫描数据语义、技术调研、复杂度挑战、原型一致性和回归风险；当前没有计划中的后端扩展，确需后端或共享组件改动时先给出最短合同和明确 owner。
- Codex / `gpt-5.6-sol` / `max` 负责总调度、任务与分支治理、监工催办、运行验证、桌面操作、截图、透明资产、复现包、最终交接和经独立授权的对外发布；Codex 不直接修改页面 JSX/CSS，也不接管 Gemini 的 UI 返工。
- Cursor/Grok 4.6 为首选评审通道；Cursor CLI 未认证时使用已授权的 Grok Build / `grok-4.6` 候补通道，并记录实际路由。
- Grok Bot App 作为桌面观察员和监工；透明底图图标、桌面操作、截图和经独立授权的对外发布由 Codex 负责。
- Grok 结论为强制门槛。UI、布局、样式、文案、前端状态和 UI 测试问题退回 Gemini；后端、共享组件边界、数据语义或复杂度问题由 Grok 给出精确合同并按明确 owner 处理。Codex 生成复现包、调度返工并重新验证，但不接管页面代码。所有修订重新评审，直至 `PASS`。

### R3｜原型一致性

- 11 张高保真原型控制元素存在性、位置、顺序、宽高比例、字体层级、图标、形态、留白、密度、文案和过程状态。
- 删除原型外的大标题、小标题、副标题、解释、标签、卡片、分组和操作入口。
- 保持原型中的左右关系与横向伸展关系。
- 页面视觉保持轻快、清楚、克制；颜色、边框、阴影和圆角取值以原型与已有 token 为准。
- 没有原型依据的内容保持缺省，不补写说明。

### R4｜第一页与第二页

- `AI 配置` 等原型指定位置补齐准确的小图标；只为原型明确标注的标题或配置项添加。
- `开始扫描` 位于左侧并贴近 `我的 AI 软件`，字号、字重和间距与原型一致。
- 移除页面中的额外 `AI 软件配置` 眉题和扫描说明。
- 清除前两页其余原型外标题、说明、标签、提示框和辅助入口。
- 扫描前、扫描中、扫描完成与技术错误四种可见状态齐全；错误状态使用最小反馈并保留重试入口。
- 扫描中界面使用原型给出的反馈，不新增百分比、步骤数、预计时间、日志或复杂动画。

### R5｜扫描数据语义

- 扫描完成列表只投影本机已安装工具；`installed` 与 `installed_not_runnable` 视为已安装集合。
- 已安装结果按原型优先顺序与稳定目录顺序展示。
- `not_installed` 不进入完成列表。
- `unknown`、读取失败、环境不可用和检测能力缺失不进入正常结果列表，也不获得面向普通用户的正常状态标签。
- 真正技术错误进入独立错误反馈路径；失败项不混入正常软件卡片。
- 过滤、排序和状态映射在 TypeScript 数据逻辑层完成，禁止仅用 CSS 隐藏。
- 重新扫描保留既有真实配置和最近一次可用结果的安全语义。

### R6｜第三页与 Agent 四段页面

- 返回入口使用原型准确文案、图标、位置和点击范围。
- `模型 / Skills / MCP / 提示词` 为铺满右侧内容区的横向长条。
- 当前模型区域按原型重排信息顺序、容器、模型层级、状态和操作。
- Agent 四段页删除原型外能力说明、解释卡、内部模式标签和披露区。
- Skills 与 MCP 沿用原型的搜索、通栏列表和开关结构，继续使用现有 toggle port 与 authoritative readback。
- Agent 模型段只读投影现有 capability query，并提供 `进入模型管理`；Agent 页不渲染模型 Switch、不调用模型 mutation。
- Agent 提示词段只对有 `promptAppId` 的 Agent 提供列表、搜索、只读正文、`enable + refetch` 与 `进入提示词管理`；导入、新建、编辑、保存、删除、停用和 live file 继续由页面 10 持有。
- `promptAppId=null` 的 Agent 不调用提示词 query，也不生成本地库。
- 保留既有真实能力端口与 authoritative readback；禁止新增模型协议、提示词 API 或第二套 draft/writeLock。

### R7｜模型、Skills、MCP 全局管理

- 页面 07、08、09 逐区域复核，并修正原型外内容、错位、密度与结构差异。
- 保留现有连接、安装、发现、导入、分配、保存、失败和确认能力。
- 页面级修改优先复用共享组件；共享组件无法表达原型时新增窄 variant。

### R8｜提示词模块

- 页面 10 按原型重构应用栏、库、编辑区、操作区、搜索和空状态的结构与宽度。
- 删除原型外说明、标签、卡片和操作。
- 保留七个 `PromptAppId`、原生端口、写锁、dirty guard、authoritative reread 和错误处理。
- 禁止静态假数据、第二套提示词状态或数据连接回退。

### R9｜记忆模块

- 页面 11 按原型重构页签、资源列表、编辑头、编辑区和操作区。
- 复制动作复制当前记忆内容。
- 保留四个长期资源、每日记忆、目录操作、写锁、dirty guard、authoritative reread 和原生端口。
- 禁止新增记忆分类、统计、引导、同步任务或静态假数据。

### R10｜工程约束

- 保留现有技术栈、六路由和核心业务端口。
- 不新增 UI 框架或第三方依赖。
- 不改变 API 契约；扫描页面用现有 readiness 查询完成投影。
- 避免全局 CSS 大改、固定截图拼接和大面积绝对定位。
- 改动范围限于 V3.1 页面、相关共享 variant 和对应测试。

### R11｜Grok 强制门槛

Grok 是规划、Wave A 与最终全量 diff 的强制过程门，不是最终收口时的抽查。阶段顺序固定为：Planning Gate `PASS` → 用户批准 → Gemini Wave A → Grok Gate A `PASS` → Gemini Wave B → Grok Final Gate `PASS` → Codex 最终验证。任何门槛未通过，后续阶段立即锁定。

Grok 每个实施门槛至少检查：

- 原型中不存在的大标题、小标题、副标题和解释文字；
- 为填满页面增加的卡片、分组、说明、状态标签和操作入口；
- “未确认”及同类自造状态和解释；
- 左右位置被颠倒、横向长条被压成短条或短卡片；
- 扫描过滤、排序、错误分流的数据语义；
- 记忆与提示词是否完成结构级改造，而不是只换色或微调间距；
- 新抽象、全局样式、依赖和测试是否超出必要范围。

任一必查项存在即为 `CHANGES_REQUIRED`，不接受带 Blocking/Major 的条件通过。UI 问题退回 Gemini；Codex 自有的调度、桌面、资产、证据、交接与消息问题退回 Codex；后端、共享组件边界、数据语义或复杂度问题由 Grok 给出精确合同和明确 owner。每次返工后必须重新评审，不得沿用旧 verdict。

Gate A `PASS` 前禁止开始 Wave B；Final Gate `PASS` 前禁止进入 Codex 最终验证、冻结候选、制作完成截图、对外发送或宣称高保真完成。

### R12｜对外内容规则

- 未来对外图片、原型、汇报关系、消息和报告禁止使用用户列出的切割性、免责性小标题与同类句式。
- 发送前运行精确文本扫描；命中即撤回草稿并重做。
- 本轮不发送任何对外消息或图片。
- `source-feishu-verbatim.md` 是用户要求逐字保存的内部权威输入，不作为对外材料。

## Acceptance Criteria

- [ ] 11 页均建立逐区域差异清单并完成对应实现。
- [ ] 第一、第二页的小图标、扫描入口、字体与左侧空间关系符合原型。
- [ ] 前两页的额外眉题、说明、状态解释和多余视觉元素已删除。
- [ ] 扫描前、扫描中、扫描完成和技术错误状态可达，过程反馈符合原型与冻结合同。
- [ ] 完成列表只包含已安装工具；过滤、排序和错误分流有单元与浏览器测试。
- [ ] 旧测试中展示自造状态与解释的正向断言已改为缺席断言；未安装、未知、读取失败、环境不可用、检测能力缺失和失败项均有 DOM 级负面断言。
- [ ] 第三页返回入口、四段横条和当前模型区域完成结构级重构。
- [ ] Agent 模型段按 capability 做只读投影，所有 Agent 均无模型 Switch 和模型 mutation；管理入口可达。
- [ ] Agent 提示词段仅在真实 `promptAppId` 上执行 enable 与 authoritative refetch；完整 CRUD 只在页面 10。
- [ ] 页面 04-09 逐页清理原型外内容并保持真实功能。
- [ ] 提示词模块完成结构级重构，真实端口与写回契约通过回归测试。
- [ ] 记忆模块完成结构级重构，真实端口与写回契约通过回归测试。
- [ ] Prompts 与 Memory 有目标视口下的 pane 数量、顺序、边界宽度、主动作位置和 overflow 几何断言。
- [ ] Grok 4.6 对最终 diff 给出明确 `PASS`，且无未关闭的门槛项。
- [ ] Grok Planning Gate、Gate A 与 Final Gate 均有独立、可追溯的 `PASS`；Gate A 前未开始 Wave B，Final Gate 前未进入最终验证或候选冻结。
- [ ] 每次 Grok 门槛均检查五类用户指定问题，任何原型外标题/解释、填充内容、自造状态、左右/长条错位或 Prompts/Memory 表面微调均为零。
- [ ] 所有 `CHANGES_REQUIRED` 均按 owner 退回并在修复后重新评审，没有豁免、条件通过或沿用旧 verdict。
- [ ] 两个前端实施波次和所有 UI 返工均记录 Antigravity / Gemini 3.7 路由；Grok 4.6 的 Gate A / Final Gate 证据可追溯。
- [ ] Codex 任务均记录为 `gpt-5.6-sol` / `max`，且产品 diff 中不存在由 Codex 直接编写的页面 JSX/CSS。
- [ ] `mise run lint:v2`、`typecheck:v2`、`test:v2`、`test:v2:browser`、`build:renderer` 和 `mise run check` fresh pass，或精确记录与本次无关的既有失败。
- [ ] 原生应用启动，目标页面完成内部运行态核验，控制台无本轮新增错误。
- [ ] 01-11 每页均记录偏差关闭结果、真实端口/回读结论和内部截图路径。
- [ ] `main`、push、PR、merge、Release、production、Windows 等待和对外发布均未发生。

## Out of Scope

- Windows 原生等待或验证。
- 旧 V3 证据包、对外图文与候选收口。
- 新业务功能、新扫描协议、新数据库、新 API 或新依赖。
- 全局品牌重做、无关代码清理和无关架构重构。
- push、PR、merge、tag、Release、production 或任何对外发送。

## Open Questions

无阻塞产品决策。实现阶段仍需按 Trellis 阶段门取得用户对本规划摘要的下一条明确批准。
