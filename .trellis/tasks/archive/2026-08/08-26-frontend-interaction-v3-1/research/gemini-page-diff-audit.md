# Gemini 3.7 逐页差异审计

> 当前状态：本文件的 11 页视觉事实、真实组件 owner 与验收重点继续有效。用户最新消息已明确前端实施与所有 UI 返工归 Antigravity / Gemini 3.7；当前路由见 `executor-authority-latest-gemini-primary.md`。

## 执行记录

- 执行器：Antigravity CLI
- 模型：`gemini-3.7-flash-high`
- 模式：`plan` / `high`
- 范围：11 张高保真原型、对应 `src/v2/pages/**` owner 与现有测试
- 文件操作：只读；本轮未修改产品代码
- 路径约束：只接受仓库中可定位的真实路径

| 批次 | Conversation ID | 结果 |
| --- | --- | --- |
| 01–03 | `1f4e7bc6-a4c2-475b-aab5-ac2913619153` | 有效 |
| 04–06 | `0e5cc6ed-3381-4402-a0e1-7df8e088352a` | 有效；两个预设 owner 路径缺失，已追踪到现有共享 owner |
| 07–09 | `a4581e1f-6b26-4067-b824-ac585ed7e817` | 有效 |
| 10–11 | `00721550-f43d-4545-a870-d82d875ebe7f` | 有效 |

## 冻结差异表

| 页面 | 原型可见事实 | 当前偏差 | 实施 owner | 验收重点 |
| --- | --- | --- | --- | --- |
| 01 扫描完成 | “我的 AI 软件”与“开始扫描”紧凑同行居左；仅展示已安装软件；卡片包含图标、名称、简介、配置入口 | 存在额外眉题、副文案、状态标签、时间、展开介绍和结果提示；按钮文案与原型不一致；结果未按已安装过滤排序 | `src/v2/pages/agents/AgentDirectory.tsx`；`src/v2/pages/agents/Page.css`；agents 单测与 E2E | 只渲染已安装结果；排序稳定；冗余 DOM 节点为零；标题、入口、卡片几何贴合原型 |
| 02 扫描中 | 标题旁为扫描状态胶囊；进度区为单行状态、发现计数和细进度条；已发现项即时出现；其余位置为骨架行 | 当前把扫描状态放在禁用按钮；进度区说明过多；待检测项使用 Spinner 卡片；缺少渐进填充 | `AgentDirectory.tsx`；`Page.css`；`useAgentDirectoryScan.ts`；agents 单测与 E2E | 静止、扫描中、完成、错误四态冻结；扫描中结构与原型一致；发现结果即时上浮；错误退出正常结果列表 |
| 03 Agent 模型选择 | 顶栏为 Agent 图标、名称与“返回”；四个分段横向铺满；“当前模型”结构紧凑；搜索与单列模型卡片按原型排列 | 存在额外眉题、扩写返回文案、安装折叠区、能力说明和提示区；当前为双栏详情结构；原型 Switch 无法在现有跨 Agent 写入合同下形成统一闭环 | `AgentConfiguration.tsx`；`AgentModelsSection.tsx`；`Page.css`；agents 单测与 E2E | 横向分段与右侧内容边界准确；删除原型外内容；按 capability 只读投影并提供管理路由；零 Switch、零模型 mutation |
| 04 Agent Skills 选择 | 单列横向通栏；每项含图标、名称、说明、来源标签与右侧开关；标题区简洁 | 当前为左右双栏；图标、来源标签与卡片级开关层级偏离；额外说明较多 | `AgentAssignmentSections.tsx`；`AgentConfiguration.tsx`；`AgentSectionHeader.tsx`；`Page.css`；agents 单测与 E2E | 通栏列表无横向溢出；开关直连 `ports.skills.toggleApp` 并权威回读；清理额外文案 |
| 05 Agent MCP 选择 | 单列横向通栏；每项含图标、说明、协议、分配状态与右侧开关 | 当前为左右双栏；服务图标、协议与分配信息层级偏离；开关藏在详情区；额外说明较多 | `AgentAssignmentSections.tsx`；`AgentConfiguration.tsx`；`AgentSectionHeader.tsx`；`Page.css`；agents 单测与 E2E | 通栏列表、协议信息和开关结构贴合原型；保留 `ports.mcp.toggleApp`、WorkBuddy 信任流程与回读 |
| 06 Agent 提示词选择 | 原型为左右分栏；顶部含导入、新建、进入管理；左侧提示词库；右侧为编辑、启用、保存、删除和内容区 | 当前 Agent owner 只覆盖 promptAppId 查询与启用；完整 CRUD、live file、dirty guard 和 write lock 归 Page 10 | `AgentPromptsSection.tsx`；`AgentConfiguration.tsx`；`AgentSectionHeader.tsx`；`Page.css`；agents 单测与 E2E | 有 appId 时提供列表、搜索、只读正文、enable/refetch 与管理路由；null appId 零 query；完整 CRUD 仅在 Page 10 |
| 07 模型管理 | 顶栏、应用导轨、已有模型摘要、连接设置与模型 ID 管理区层级清楚、密度轻 | 保存动作散落在详情头；备份披露区占用空间；表单与操作区间距偏大 | `src/v2/pages/models/Page.tsx`；`Page.css`；`modelsShared.tsx`；models 单测与 E2E | 操作层级和页面密度贴合原型；真实配置、拉取、连通测试、保存链路保持完整 |
| 08 Skills 管理 | 顶栏、搜索与三栏工作区清晰；详情与应用分配紧凑 | 批量分配区展开过多；详情和分配面板留白偏大；列表状态层级偏重 | `src/v2/pages/skills/Page.tsx`；`page.css`；skills 样式测试与 features E2E | 三栏比例和密度按原型；来源、路径、卸载、单项与批量分配能力保持完整 |
| 09 MCP 管理 | 顶栏、搜索与三栏工作区；详情、安装来源、传输信息和应用分配紧凑 | 右侧分配区密度偏低；中栏卡片间距偏宽；列表图标与副信息细节有偏差 | `src/v2/pages/mcp/Page.tsx`；`page.css`；`Discovery.tsx`；MCP 样式测试与 features E2E | 三栏比例和卡片密度按原型；编辑、删除、导入、信任确认和分配回读保持完整 |
| 10 提示词管理 | 顶栏动作、三栏几何、通栏搜索、提示词库和右侧编辑区按原型排列 | 搜索位置、列表项层级、编辑头部、元信息和表单顺序存在偏差；部分视觉层级偏重 | `src/v2/pages/prompts/Page.tsx`；`page.css`；prompts 单测、共享样式测试与 E2E | 三栏 pane 宽度、分隔线、搜索与编辑区几何做结构断言；导入、新建、编辑、删除、启停保持真实连接 |
| 11 记忆模块 | 顶栏、双 Tab、两栏工作区、分组资源列表和编辑器按原型排列；编辑器含行号槽视觉 | 左侧缺文档图标；按钮文案偏长；编辑器为普通 textarea；路径与字符数对齐偏差 | `src/v2/pages/memory/Page.tsx`；`page.css`；memory 单测、共享样式测试与 E2E | 两栏 pane 宽度、分隔线、Tab 和编辑器几何做结构断言；读取、复制、保存、工作区入口和脏状态拦截保持完整 |

## 共享边界

1. `AgentSkillsSection.tsx` 与 `AgentMcpSection.tsx` 在仓库中不存在；04、05 的现有 owner 为 `AgentAssignmentSections.tsx`。
2. 03 与 06 的真实能力合同已由 Grok 4.6 冻结，见 `grok-real-capability-contract.md`；Gemini Wave A 必须按合同实施。
3. 页面结构改动优先增加局部 variant；禁止用全局 CSS 连锁覆盖其他页面。
4. 10、11 必须新增 pane 几何与 DOM-negative 断言，避免只验证颜色、字号和边距。
5. 所有 UI 返工由 Gemini 3.7 完成；Codex 只做调度、验证、桌面操作、截图、透明资产、复现包、最终交接和授权后的对外动作，不直接编写页面 JSX/CSS。
