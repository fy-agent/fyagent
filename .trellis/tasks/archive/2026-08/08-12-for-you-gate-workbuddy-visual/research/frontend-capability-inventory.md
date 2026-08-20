# FyAgent 现有前端能力盘点与新一级信息架构

## 结论

现有前端已经覆盖模型接入、Skills、MCP、提示词和多种记忆文件，但导航以“当前 Agent + 历史页面”为中心，导致同一任务被拆散。新原型改为六个稳定一级入口：`Agent 目录 / 模型 / Skills / MCP / 提示词 / 记忆`。设置、会话、环境变量、工具和 Agent 默认值继续作为上下文操作，不占一级导航。

## 现有能力 → 新页面

| 一级页面 | 当前代码能力 | 新页面归并 | 本轮愿景补位 |
| --- | --- | --- | --- |
| Agent 目录 | `AppSwitcher` 支持 Claude、Claude Desktop、Codex、WorkBuddy、Gemini、GrokBuild、OpenCode、OpenClaw、Hermes；`AgentsPanel` 仍是 Coming Soon | 只展示评审通过的 Codex、Claude Code、Qwen Code、WorkBuddy、QClaw；列表选择 + 右侧详情 | 候选状态、支持能力、接入动作统一展示；不伪造安装/验证状态 |
| 模型 | Provider 列表/排序/搜索、官方与第三方接入、OAuth/API Key、统一供应商、Profile、代理接管、故障切换、模型拉取、路由映射、请求日志、供应商/模型统计 | `接入源 / 路由 / 用量` 三个工作区放在同一页 | 以 Agent 为列统一查看默认模型与备用模型；跨 Agent 复用接入源 |
| Skills | 已安装列表、按 App 启停、批量启停、更新/全部更新、卸载、导入未托管 Skill、ZIP 安装、备份恢复、仓库管理、skills.sh 搜索 | `已安装 / 发现 / 来源 / 备份` 四个页内视图 | 将 App 开关提升为 Agent 分配矩阵；清楚区分本地、仓库和公共目录来源 |
| MCP | 服务列表、按 App 启停、批量启停、导入现有配置、预设、自定义 JSON/TOML、stdio/HTTP/SSE 向导、env/headers、文档链接 | `服务 / 分配 / 新建` 同页完成 | 增加健康检查与权限摘要的展示位；不在原型里宣称检查已通过 |
| 提示词 | 按当前 App 管理提示词、搜索、新建/编辑/删除、启用一个提示词、名称/说明/内容编辑 | `提示词库 / Agent 分配 / 编辑器` 三栏 | 支持共享模板和 Agent 覆盖层的概念；版本历史作为后续能力标注 |
| 记忆 | OpenClaw 工作区文件（AGENTS/SOUL/USER/IDENTITY/TOOLS/MEMORY/HEARTBEAT/BOOTSTRAP/BOOT）、每日记忆的创建/搜索/编辑/删除、Hermes 的 Agent/User 记忆开关与字数限制 | `长期记忆 / 每日记录 / 身份与偏好` 三类来源 | 增加 Agent 可见范围和本地边界；不暗示云同步或跨设备已实现 |

## 不进入一级导航的现有能力

- 全局设置、语言、主题、导入导出：右上角设置。
- 会话管理：Agent 详情中的“会话”。
- OpenClaw 环境变量、工具、Agent 默认值：Agent 详情中的“高级设置”。
- Hermes Web UI：Agent 详情中的外部控制台。
- 请求日志与价格表：模型页的“用量”视图。

## 六页统一交互模型

1. 顶部胶囊导航只负责切换六个一级对象，不混入二级操作。
2. 页面标题只写对象名；标题右侧最多一个主动作和一个溢出菜单。
3. 左侧是对象列表或页内视图，中间是主要工作区，右侧只放会改变决策的状态/分配。
4. 行内信息只保留名称、关键状态和一个主操作；来源、长说明、文档链接进入详情。
5. 所有 Agent 分配都复用同一种五列矩阵，避免 Skills、MCP、提示词各自发明一套开关。

## 代码证据

- 导航和历史视图：`src/App.tsx`
- 模型与接入：`src/components/providers/`、`src/components/universal/`、`src/components/proxy/`、`src/components/usage/`
- Skills：`src/components/skills/SkillsPage.tsx`、`UnifiedSkillsPanel.tsx`、`RepoManagerPanel.tsx`
- MCP：`src/components/mcp/UnifiedMcpPanel.tsx`、`McpFormModal.tsx`、`McpWizardModal.tsx`
- 提示词：`src/components/prompts/PromptPanel.tsx`、`PromptFormPanel.tsx`
- 记忆：`src/components/workspace/WorkspaceFilesPanel.tsx`、`DailyMemoryPanel.tsx`、`src/components/hermes/HermesMemoryPanel.tsx`
