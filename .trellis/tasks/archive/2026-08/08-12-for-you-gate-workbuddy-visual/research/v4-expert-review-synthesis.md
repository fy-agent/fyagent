# v4 三方专家评审汇总

## 评审角色

- 代码能力审计：读取当前 `main` 的组件、类型、API 与 Rust 服务，证据等级 `code_audit`。
- 产品经理评审：核对 R4/CAP-100、v3 信息架构与当前数据契约，区分当前能力和未来愿景。
- 视觉评审：对比用户提供的深色参考图与 v3 六页，输出可读性、材质和跨平台 token。

## 共同结论

v3 的六个一级入口、三栏布局和按钮逻辑成立；问题集中在两处：

1. 视觉过浅：背景、面板、字段和文字的亮度过于接近，白色文字失去层级。
2. 内容越界：把候选 Agent 的未来愿景画成了当前可操作的数据，例如五 Agent 路由/分配/可见范围、Skill 能力标签、MCP 权限与连接健康、提示词共享优先级。

## 必须纠正的能力边界

| v3 表达 | 当前证据 | v4 处理 |
| --- | --- | --- |
| 五 Agent 全局模型路由 | 当前是按 App 的 Provider；普通 App 是当前 Provider，OpenCode/OpenClaw/Hermes 是累加配置 | 模型页改为当前应用的接入源与配置详情；统一路由保留为页签概念，不画成已配置矩阵 |
| Skills 分配给五个候选 Agent | 当前只支持 Claude、Codex、Gemini、Grok Build、OpenCode、Hermes | 右栏标题改为“启用于应用”，只展示真实支持范围 |
| Skill 的读取/编辑/附件能力 | `InstalledSkill` 没有能力枚举字段 | 删除能力胶囊；详情显示描述、目录、仓库、分支、README、更新时间 |
| MCP 权限与健康检查 | 当前只有配置与应用投影，没有工具权限识别或健康状态 | 删除权限和检查连接；详情展示 ID、传输配置、脱敏环境变量/Headers、元数据 |
| 提示词跨 Agent 分配、共享模板、优先级 | 当前按 App CRUD，每个 App 一次只启用一条并写入单一文件 | 右栏显示当前应用、目标文件、启用状态；删除共享和优先级 |
| 记忆跨 Agent 可见范围 | 当前为 OpenClaw workspace/daily Markdown 与 Hermes MEMORY/USER blob | 右栏显示来源归属、文件状态/大小或 Hermes 字符预算，不画 Agent 开关 |

## 视觉决策

- 使用深蓝灰而非纯黑：`#172D43` 基底、`#27465E` 面板、`#213F56` 强面板。
- 主文字 `#F5F8FC`、次文字 `#D1DDE8`、辅助文字 `#A9BDCF`；辅助文字也必须达到正文 AA。
- signal blue 只用于一级选中、当前列表行和每页唯一主动作；cyan 只表达连接/活动。
- 左栏连续行、中栏唯一主焦点、右栏单一检查器；不要每行再包独立发光卡。
- Windows 使用右上最小化/最大化/关闭；macOS 使用原生标题栏，不在共享页面中复制另一平台控件。

## 代码锚点

- Provider：`src/types.ts:11`、`src/components/providers/ProviderCard.tsx:353`、`ProviderActions.tsx:87`
- 统一供应商：`src/types.ts:542`、`src/components/universal/UniversalProviderFormModal.tsx:395`
- Skills：`src/lib/api/skills.ts:27`、`src/components/skills/UnifiedSkillsPanel.tsx:773`
- MCP：`src/types.ts:478`、`src/components/mcp/UnifiedMcpPanel.tsx:389`、`McpWizardModal.tsx:84`
- 提示词：`src/lib/api/prompts.ts:4`、`src/components/prompts/PromptListItem.tsx:27`、`PromptFormPanel.tsx:76`
- 记忆：`src/components/workspace/WorkspaceFilesPanel.tsx:30`、`DailyMemoryPanel.tsx:443`、`src/components/hermes/HermesMemoryPanel.tsx:129`
- 支持应用集合：`src/config/appConfig.tsx:29`
