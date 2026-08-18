# Prompt 与 Memory 前端现状索引

> 本文是改造前的仓库阅读记录，用于说明旧合同与风险，不代表当前页面设计。现行产品与实现以 `prd.md`、`design.md`、`implement.md` 和 `research/local-agent-inventory.md` 为准。

## 1. 阅读结论

本轮相关前端代码已经读完。后续实现不需要再回头探索旧模块，只需按 design.md 和 implement.md 分批落地，并在出现新验证证据时修正实现。

权威顺序如下：

1. dev/laiyongjie 的 src/v2 与 .trellis/spec/frontend/v2-shell.md：结构和视觉合同。
2. 旧版 Prompt、Workspace、Daily Memory、Hermes Memory：行为合同。
3. codex/前端设计 提交 fd54598f：信息架构和内容密度参考。
4. 旧版通用组件和样式：仅用于理解交互，不可被 V2 导入。

## 2. 分支与环境基线

| 项目 | 当前事实 |
| --- | --- |
| 基线 | origin/dev/laiyongjie |
| 基线提交 | e33d37dd6f9d58c11207f843b5c33750a79dbb4a |
| 工作分支 | codex/prompt-memory-frontend-refactor |
| V2 壳引入提交 | 82ea583a feat(frontend): add v2 visual shell |
| 运行入口 | src/index.html 指向 src/v2/main.tsx |
| 目标窗口 | 默认 1232 × 700，需覆盖 900 × 600 至 1440 × 900 |
| 本机预检 | mise 不在 PATH；node_modules 未安装；系统 Node 24.13.0，而仓库要求 24.19.0 |

环境预检失败发生在依赖加载前，不能被解释为代码失败。实现开始前先恢复仓库规定的 mise、Node 和 pnpm 环境，再取得干净基线。

## 3. V2 框架完整索引

已阅读范围：

- src/v2 下全部生产文件。
- tests/v2 下全部单元、架构和平台测试。
- tests/v2-browser 下全部浏览器几何和交互测试。
- .trellis/spec/frontend/v2-shell.md。
- 已归档任务 .trellis/tasks/archive/2026-08/08-12-frontend-v2-shell 下的 PRD、设计与实施文档。
- package.json、mise.toml、.mise/tasks/frontend.toml、tsconfig.v2.json、eslint.v2.config.mjs、vitest.v2.config.ts、playwright.v2.config.ts、vite.config.ts。

### 3.1 壳层与路由

| 文件 | 作用 | 本任务约束 |
| --- | --- | --- |
| src/v2/main.tsx | V2 启动入口 | 不回接旧版 main 或 App |
| src/v2/app/router.tsx | Hash 路由和六个主页面 | Prompt、Memory 保持既有路由与顺序 |
| src/v2/app/RootError.tsx | 根错误壳 | 页面错误不能破坏顶栏 |
| src/v2/widgets/app-shell/AppShell.tsx | 顶层布局 | 页面只填充 ContentViewport |
| src/v2/widgets/app-shell/TopBar.tsx | 顶栏 | 高度 68 像素 |
| src/v2/widgets/app-shell/PrimaryNav.tsx | 六项主导航 | 路由 location 是唯一选中态 |
| src/v2/widgets/app-shell/ContentViewport.tsx | 页面内容容器 | 页面自己管理内部滚动与最小宽度 |
| src/v2/shared/config/navigation.ts | 六路由清单 | 不增加 Prompt 或 Memory 子路由到主导航 |

### 3.2 设计系统与平台层

| 文件 | 作用 | 结论 |
| --- | --- | --- |
| src/v2/app/styles/tokens.css | 独立的 fy token | 浅色 Liquid Glass 是权威视觉 |
| src/v2/app/styles/globals.css | 基础排版和页面背景 | 不导入旧版 index.css |
| src/v2/app/styles/shell.css | 顶栏、导航、内容几何 | 页面断点必须与壳协同 |
| src/v2/app/styles/motion.css | 动效和 reduced motion | 新交互继续遵守 |
| src/v2/shared/ui/primitives.tsx | GlassButton、IconButton、Tooltip | 可扩展但不做新设计系统 |
| src/v2/shared/ui/vendor.ts | Popover、Tabs 的受控出口 | Memory 来源切换可复用 Tabs |
| src/v2/shared/platform | runtime、window、lifecycle 端口 | 新命令和事件必须沿用相同隔离方式 |
| src/v2/shared/platform/tauri | 原生适配器唯一入口 | 只有这里可直接导入 Tauri |

### 3.3 强制架构规则

- src/v2 不得导入旧版 App、components、hooks、lib、i18n、index.css。
- pages 只能依赖 shared；widgets 只能依赖 shared；app 可组装 pages、widgets、shared 和 dev。
- 直接 Tauri 导入只允许在 shared/platform/tauri。
- 禁止动态导入、重复 currentView 状态、Lucide 与 glasscn。
- 六个路由顺序固定，默认进入 /models。
- 当前 Prompt 与 Memory 页面都是空占位，说明没有必须兼容的 V2 页面内部结构。

## 4. Prompt 旧版行为合同

### 4.1 文件索引

| 文件 | 已确认职责 |
| --- | --- |
| src/App.tsx | 旧版入口、应用上下文、Prompt 页导航和全局 busy 锁 |
| src/lib/api/prompts.ts | Prompt 类型及七个前端命令包装 |
| src/hooks/usePromptActions.ts | 加载、写入、启用、回滚、请求代次和通知 |
| src/components/prompts/PromptPanel.tsx | 列表、搜索、事件刷新、并发队列、表单和删除确认 |
| src/components/prompts/PromptFormPanel.tsx | 名称、描述、Markdown 正文、目标文件名和保存 |
| src/components/prompts/PromptListItem.tsx | 列表项展示与编辑、删除入口 |
| src/components/prompts/PromptToggle.tsx | 启用开关 |
| src/components/deeplink/PromptConfirmation.tsx | 深链导入确认，不属于本次页面首期入口 |
| tests/hooks/usePromptActions.test.tsx | hook 的加载、写入、切换、回滚和竞态覆盖 |
| tests/components/PromptPanel.test.tsx | 页面状态、外部事件和 busy 锁覆盖 |
| tests/components/PromptFormPanel.test.tsx | 表单校验与目标文件映射覆盖 |

### 4.2 数据与调用合同

Prompt 字段：

- id
- name
- content
- description，可选
- enabled
- createdAt，可选
- updatedAt，可选

现有前端命令：

- get_prompts
- upsert_prompt
- delete_prompt
- enable_prompt
- import_prompt_from_file
- get_current_prompt_file_content

本次只需要页面已有的查询、保存、删除、启用和当前文件读取合同。导入命令继续保留在旧深链流程，不扩成 V2 页面功能。

### 4.3 必须保留的状态语义

- 数据按 appId 隔离。
- Claude Desktop 在旧版共享 Claude 的 Prompt 语义。
- 搜索覆盖名称、描述和正文。
- 启用开关采用乐观更新；失败必须回滚。
- 同一应用最多一个 enabled 项。
- 应用切换后旧请求不得覆盖新应用数据。
- 写入串行化；写入期间到达的外部刷新需要排队，不能覆盖正在编辑或刚保存的状态。
- prompt-imported 与 profile-applied 会触发刷新。
- 旧页面把交互锁与导航锁分开，避免保存和删除中的误操作。
- currentFileContent 已加载但旧页面没有展示，不应在本次无依据地新增对比功能。

### 4.4 旧版入口范围

- Claude、Claude Desktop、Codex、Gemini、Grok Build、OpenCode 的头部会暴露 Prompt 入口。
- OpenClaw 与 Hermes 的旧版头部采用专属菜单，因此没有暴露 Prompt 入口。
- sharedFeatureApp 和底层 Prompt API 可以接受 OpenClaw 与 Hermes。
- WorkBuddy 不在 Prompt 范围。

因此“V2 是否顺便把 OpenClaw 与 Hermes 纳入应用选择器”属于产品范围决定，而不是纯技术决定。

## 5. Memory 旧版行为合同

### 5.1 文件索引

| 文件 | 已确认职责 |
| --- | --- |
| src/App.tsx | OpenClaw Workspace 与 Hermes Memory 的旧版入口 |
| src/components/workspace/WorkspaceFilesPanel.tsx | 九个工作区文件、存在状态、每日记录入口 |
| src/components/workspace/WorkspaceFileEditor.tsx | 工作区文件读取、编辑和保存 |
| src/components/workspace/DailyMemoryPanel.tsx | 每日文件列表、搜索、创建、编辑、删除、打开目录 |
| src/lib/api/workspace.ts | Workspace 与 Daily Memory 类型及命令包装 |
| src/components/hermes/HermesMemoryPanel.tsx | 两类 Hermes 记忆、启用、限制、编辑、保存和 WebUI |
| src/hooks/useHermes.ts | Hermes 查询、保存、启用、限制和 WebUI mutation |
| src/lib/api/hermes.ts | Hermes Memory 前端命令包装 |
| src/types/hermes.ts 与 src/types/index.ts | Hermes Memory 类型出口 |
| src/components/MarkdownEditor.tsx | 旧版 CodeMirror 编辑器，只能参考，不能导入 V2 |

### 5.2 OpenClaw Workspace

固定文件共九个：

1. AGENTS.md
2. SOUL.md
3. USER.md
4. IDENTITY.md
5. TOOLS.md
6. MEMORY.md
7. HEARTBEAT.md
8. BOOTSTRAP.md
9. BOOT.md

现有前端通过读取判断文件是否存在；选中后加载 Markdown，手动保存。V2 应把“固定文件清单”和“真实读取状态”分开，不把未读取当成不存在。

### 5.3 每日记录

现有能力：

- 列出文件，包含 filename、date、sizeBytes、modifiedAt、preview。
- 读取、写入、删除指定日期。
- 创建今日记录时先建立前端草稿，保存后才持久化。
- 搜索采用 300 毫秒防抖，支持 Cmd/Ctrl + F 和 Escape。
- 搜索结果包含 snippet 与 matchCount。
- 可打开每日记录所在目录。
- 编辑和列表使用旧版全屏切换。

V2 改为同页三段式，但需保持“今日草稿未保存不产生文件”的语义。

### 5.4 Hermes Memory

现有两类内容：

- Agent memory，对应 MEMORY.md。
- User profile，对应 USER.md。

每类都包含启用开关、字符数、限制、Markdown 编辑和手动保存。首次查询只在本地未编辑时注入内容，避免后台刷新覆盖脏草稿。WebUI 配置入口保留。

### 5.5 测试缺口

未发现以下专属组件或 hook 的自动化测试：

- WorkspaceFilesPanel
- WorkspaceFileEditor
- DailyMemoryPanel
- HermesMemoryPanel
- useHermes 的 Memory 编辑语义

这意味着 Memory 重构不能只做视觉测试，必须先补领域状态与命令参数测试，尤其覆盖：

- 脏草稿不被刷新覆盖。
- 来源和条目快速切换时旧请求失效。
- 创建今日记录的延迟持久化。
- 删除失败后的列表与选中态恢复。
- Hermes 启用和保存失败。

## 6. 可复用与不可复用

### 可直接复用

- V2 AppShell、TopBar、PrimaryNav、ContentViewport。
- V2 token、全局排版、动效、Tooltip、Tabs、GlassButton、IconButton。
- 既有六路由、Hash 路由状态和窗口平台端口模式。
- 旧版 API 包装中体现的命令名称、参数和返回类型，作为前端合同参考。
- 旧 Prompt 测试中体现的并发、回滚和事件语义。

### 只能参考，不能导入

- 旧版 PromptPanel、PromptFormPanel、Prompt hook。
- 旧版 Workspace、Daily Memory、Hermes Memory 组件。
- 旧版 MarkdownEditor。
- 旧版 Tailwind 类、Radix 组件出口、Toast、i18n、QueryClient。
- codex/前端设计 的暗色视觉 token 和截图样式。

## 7. 原型参考的取舍

控制台 V4 原型给出的有效信息架构：

- Prompt：左侧应用和列表，中间内联编辑，右侧目标文件与状态。
- Memory：OpenClaw、每日记录、Hermes 三来源，列表、编辑、检查器三段式。

不采纳的部分：

- 暗色深蓝灰主题。
- 独立于 dev 壳的顶部栏、间距、字体和圆角。
- 原型中未被现有能力支持的跨 Agent、向量、自动提取或同步概念。

## 8. 阅读边界与完成声明

本轮完整阅读的是前端生产文件、前端测试、V2 规范、V2 构建配置和原型文档。按用户要求，没有进入 src-tauri，也没有用后端实现反推或扩展产品范围。后续若命令返回与前端类型不一致，应作为实现期的前端合同异常单独处理，不能顺手改后端。
