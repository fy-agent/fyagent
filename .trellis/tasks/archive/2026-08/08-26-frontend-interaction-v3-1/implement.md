# Frontend Interaction V3.1 Implementation Plan

## Closure Checklist

- [ ] 11 页差异清单完成并映射到文件与验收检查。
- [ ] Antigravity / Gemini 3.7 完成全部前端实施与 UI 返工，路由与产出可追溯；Codex 未直接编写页面 JSX/CSS。
- [ ] 扫描结果数据投影符合 PRD R5。
- [ ] 页面 03 的横向 tabs、返回入口和当前模型结构通过运行核验。
- [ ] 页面 10、11 完成结构级重构并保留真实端口。
- [ ] Grok 4.6 最终 verdict 为 `PASS`。
- [ ] Planning Gate、Gate A 与 Final Gate 均独立 `PASS`；没有条件通过、未复审返工或跨门槛抢跑。
- [ ] 全部必需工程命令 fresh pass。
- [ ] 原生应用启动并完成内部页面核验。
- [ ] `main`、旧 Windows 等待、对外发送和发布动作均未发生。

## Phase 0｜Planning Gate

1. 保存两段飞书原文并标记旧任务 superseded。
2. 确认新分支、原型文件、环境、运行入口与外部模型通道。
3. 复用 Antigravity / Gemini 3.7 已完成的逐页差异清单，并将全部前端实施与 UI 返工 owner 固定到同一 Gemini 路由。
4. 由 Grok 4.6 只读审查 PRD、设计、实施计划和门槛完整性。
5. 将 01-11 每页的原型要求、当前偏差、file:line、owner、最小修改和验收检查固化到 `research/gemini-page-diff-audit.md`；缺页即阻塞。
6. 固化 `prd.md`、`design.md`、`implement.md`、`implement.jsonl`、`check.jsonl`。
7. 只有 Grok Planning Gate 明确 `PASS` 后才向用户提交最终规划摘要；`CHANGES_REQUIRED` 时禁止请求 `task.py start` 或 Wave A。用户在看到该最新摘要后的下一条明确批准，才允许运行 `task.py start`。

## Phase 1｜Gemini Implementation Wave A: Pages 01-06

前端实现 owner：Antigravity / Gemini 3.7。Gemini 负责 01-06 的页面、局部 UI variant、状态投影与相关测试；Codex 只调度、记录路由、运行验证和生成复现包。

1. 启动项目，复核 01、02、03 的真实运行态与原型。
2. 建立扫描结果 view model，完成安装集合过滤、稳定排序和错误分流。
3. 按 design 的 frozen UI states 重构 `AgentDirectory` idle、scanning、complete 与 error。
4. 重构 `AgentConfiguration` identity、返回入口与 full-width tabs。
5. 模型段只读投影既有 capability query 并提供管理路由；禁止模型 Switch 与模型 mutation。WorkBuddy、OpenCode、TRAE Work、Qoderwork 和 Provider 按 `grok-real-capability-contract.md` 分别处理。
6. Skills 与 MCP 改为原型通栏列表，保留现有 toggle owner 与 authoritative readback。
7. 提示词段只在真实 `promptAppId` 上提供列表、搜索、只读正文、enable/refetch 与管理路由；CRUD、停用、live file 和脏稿流程只留 Page 10。
8. 更新相关 Vitest 与 browser tests：删除旧自造状态的正向断言，增加未安装、未知、读取失败、环境不可用、检测能力缺失、失败项和原型外文案的缺席断言。
9. 增加能力合同测试：Page 03 零 Switch/零模型 mutation；Page 06 仅真实 appId 查询与 enable 回读，null appId 零 query。
10. 生成完整 Gate A review packet 后停止；packet 必须包含 diff 文件清单、原型外文案/状态负面扫描、目标视口审查截图或几何、测试结果、未决项和实际模型路由。缺包不得开启 Gate A；没有 Grok Gate A `PASS` 不得进入 Phase 3。packet 中的截图/几何只用于过程审查，不是完成截图或完成口径。

波次验证：

```text
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

## Phase 2｜Grok Gate A

执行者：Cursor / Grok 4.6；CLI 身份不可用时切换 Grok Build / `grok-4.6`。

1. 对照 01-06 原型检查 diff 与运行结果。
2. 逐页检查原型外大标题、小标题、副标题、解释文字、填充卡片、分组、说明、状态标签和操作入口。
3. 检查“未确认”及同类自造状态与解释，检查 scan view model 的过滤、排序、错误和 stale-response 语义。
4. 检查左右位置、tabs/长条宽度、页面密度、组件复用和复杂度。
5. 检查 Pages 03-06 没有原型外披露或能力说明。
6. 检查本波次中的 Page 06 及任何被触碰的 Prompts/Memory 表面是否完成结构级改造，禁止只换色或微调间距；尚未实施的 Pages 10-11 必须记录为 Final Gate 未决范围，不得被 Gate A 冒充为已通过，也不得因尚未开始而误判 Gate A 失败。
7. 任一适用必查项存在即 `CHANGES_REQUIRED`。UI 问题退回 Gemini；Codex 自有的桌面、截图、透明资产、证据、交接和消息问题退回 Codex；后端、共享组件边界、数据语义和复杂度问题由 Grok 给出精确合同和 owner。Codex 不写页面 JSX/CSS。
8. 任何新 diff 都使旧 Gate A verdict 失效；只有基于最新 diff 的明确 `PASS` 才解锁 Phase 3，禁止条件通过。

## Phase 3｜Gemini Implementation Wave B: Pages 07-11

前端实现 owner：Antigravity / Gemini 3.7。Gemini 负责 07-11 的页面、局部 UI variant 与相关测试；Codex 只调度与验证，真实数据合同由现有 port 和 Grok 门槛共同约束。

1. 对照 07-09 修正全局管理页确认的结构与视觉偏差。
2. 按页面 10 重构 Prompts 的应用栏、库、编辑区和操作区。
3. 按页面 11 重构 Memory 的页签、资源区、编辑头和编辑区。
4. 保持现有 ports、query keys、write locks、dirty guards 和 reread。
5. 更新单元、交互、响应式与负面文案测试，并为页面 10、11 增加 pane 顺序、宽度占比、主动作位置和 overflow 几何断言。
6. 生成完整 Final Gate review packet 后停止；packet 必须包含全量 diff、五类负面扫描、目标视口审查截图或几何、测试结果、未决项和实际模型路由。缺包不得开启 Final Gate；没有 Grok Final Gate `PASS` 不得进入 Phase 5。packet 中的截图/几何只用于过程审查，不是完成截图或完成口径。

波次验证沿用 Phase 1，并补充 Prompts/Memory focused tests。

## Phase 4｜Grok Final Gate

1. 对照 07-11 原型检查结构、宽度、密度、左右关系和操作位置。
2. 检查全量 diff 中所有原型外标题、解释、填充卡片/分组/标签/入口和自造状态均为零。
3. 重点确认 Prompts/Memory 已完成 pane、列表、编辑区与操作区的结构级改造，不接受只换色或微调间距。
4. 检查 shared variant 与局部 CSS，阻止全局副作用和过度抽象。
5. 先确认 review packet 完整，再对完整 diff、五类负面扫描、目标视口审查截图/几何证据、测试结果、未决项和实际模型路由执行最终门槛审查；缺任一项直接 `CHANGES_REQUIRED`。
6. 任何 UI/前端 Blocking 或 Major 项退回 Gemini；Codex 自有问题退回 Codex；后端、共享组件、数据语义和复杂度项按 Grok 指定合同与 owner 处理。
7. 每次返工后重新执行 Final Gate；只有明确 `PASS` 才解锁 Phase 5，禁止条件通过、候选冻结、完成截图和对外完成口径。

## Phase 5｜Codex Verification

Codex 在 Grok `PASS` 后执行：

```text
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run typecheck
mise run format:check
mise run test:unit
mise run check
```

若 diff 触碰 Rust/backend，增加：

```text
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
```

随后从新分支启动原生应用，内部核验 11 页、浏览器控制台与失败路径。截图只用于内部验收，不对外发送。

必须生成 01-11 逐页验收表，每页独立记录：

1. 原型差异关闭结果；
2. 真实 port 与 authoritative readback 结果；
3. 内部运行截图路径；
4. 仍受工程边界限制的原型差异；
5. 对应测试与命令 exit code。

任何 UI 或前端行为失败均由 Codex 生成复现包并退回 Gemini；Codex 不接管页面 JSX/CSS。后端、共享组件、数据语义或复杂度问题按 Grok 指定 owner 处理。所有改动重新进入 Grok 门槛。

## Phase 6｜Branch Handoff

1. 记录主要文件、命令、exit code、Grok verdict、01-11 逐页验收表和未决原型缺口。
2. 运行 stale-reference 与禁用文案扫描。
3. 保持 `main` 未修改，不 push、不建 PR、不发布。
4. 等待用户对后续提交、push、PR、截图或对外消息的独立授权。

## Rollback Points

- Wave A 仅包含 Pages 01-06 与相关 tests。
- Wave B 仅包含 Pages 07-11 与相关 tests。
- Shared variant 单独提交，便于在跨页面回归时独立回退。
- 任务卡与原文保留在分支历史，任何回退都不得恢复旧候选口径。
