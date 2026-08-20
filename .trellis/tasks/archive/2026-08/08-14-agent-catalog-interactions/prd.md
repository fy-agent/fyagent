# 完善 Agent 目录交互与布局

## Goal

让 Agent 目录成为可正常使用的受控入口：图标不再带人为白底，左右栏保持各自内容高度，QoderWork、TRAE Work、WorkBuddy 与 Claude 能通过系统浏览器打开准确的官方页面，Codex 在详情中使用 FyAgent 现有的一键安装能力；同时为 Skills/MCP 的六应用分配控件补充可识别图标，并保持 Agent/模型目录与 Skills/MCP 分配合同的真实边界。

## Background and Confirmed Facts

- Agent 列表与详情图标共用 `src/v2/shared/assets/agents`；当前浅白底由 `src/v2/pages/agents/Page.css:68-80,118-126` 显式添加。
- Agent 双栏由 `src/v2/pages/agents/Page.css:1-6` 的 CSS Grid 实现；默认交叉轴拉伸使左侧目录面板跟随右侧详情等高。
- Agent 目录与模型目录共享同一份五候选原生合同：QoderWork、TRAE Work、WorkBuddy、Codex、Claude Code。模型页为相同候选提供受控配置或官方指引。
- Skills/MCP 使用另一份六应用分配合同：Claude、Codex、Gemini、Grok Build、OpenCode、Hermes。`src/v2/shared/ui/AssignmentPanel.tsx` 当前只显示文字与开关。
- 原生 Agent 目录合同 v1 每项只有一个 `officialUrl`，无法表达 Claude CLI 与 Desktop 两个独立目标，也无法诚实表达 Codex “无外链、内置安装”的行为。
- 外链经 V2 Settings port 调用已注册的 `open_external` 原生命令；现有测试只证明 IPC payload，尚未证明本机系统浏览器实际启动。
- Codex 一键安装器已有固定 Rust 命令、受校验 DTO、完整 JobSnapshot、事件顺序、下载/安装状态、取消、启动、日志和脱敏错误合同。V2 不得直接导入旧版组件、Hook 或 Tauri API，必须在 V2 平台边界内复用同一原生协议和共享纯状态规则。

## Product Decisions

- Agent 目录与模型目录继续对应同一组五候选及既有顺序，不新增或移除候选。
- Skills/MCP 继续对应现有六个真实可分配应用；本任务只补充图标和必要的可访问呈现，不改变分配 ID、持久化结构或后端命令。
- 原生目录合同升级为 v2，用结构化官方链接表达零个、一个或多个外部目标：QoderWork、TRAE Work、WorkBuddy 各一个，Claude Code CLI 与 Claude Desktop 各一个，Codex 为零个。
- Codex 目录状态与动作应诚实表达 FyAgent 内置安装可用，不继续显示“手动安装”或外部官网按钮。

## Requirements

### R1 — Agent 图标呈现

- Agent 列表和右侧详情不得为品牌图标绘制白色或近白色底板。
- 保留素材自身背景、透明度、原始比例和 TRAE 48px 原生尺寸例外；不得为消除白底而重绘或错误裁切第三方标识。

### R2 — 版本化官方链接与系统浏览器

- Agent 目录合同 v2 必须为每个外部动作提供稳定 ID、按钮标签和绝对 HTTPS URL；Rust、TypeScript 运行时解析、测试 fixture 与 UI 必须使用同一结构。
- QoderWork 打开经核验的 Qoder CN 官方产品/下载页面；TRAE Work 打开 `https://work.trae.cn/`；WorkBuddy 打开 `https://www.workbuddy.cn/`。
- Claude 详情必须同时提供“Claude Code CLI”和“Claude Desktop”两个含义明确的按钮，分别打开 Anthropic 官方 CLI 设置文档与 Desktop 下载页。
- 所有外链继续通过 V2 Settings port 和原生 `open_external` 打开系统默认浏览器；不得使用 `window.open`、内嵌 WebView 或未校验 fallback 冒充原生成功。
- 打开失败必须显示受控错误。实施后必须用真实 Tauri 应用验证至少一个官方链接确实交给系统浏览器；若现有原生命令失败，应在不放宽交互用户/URL 安全边界的前提下修复根因。

### R3 — Codex 内置一键安装

- Codex 详情不得显示官方页面按钮；目录合同的外部链接列表必须为空，browse 能力应明确不可用，install 能力应明确由 FyAgent 支持。
- Codex 详情必须在 V2 边界内接入现有安装器的本地状态、远端版本、JobSnapshot 事件和操作，覆盖安装、更新、启动、刷新/重试、取消与打开日志等原流程已有能力。
- V2 必须复用既有 DTO、版本比较、JobSnapshot 接纳顺序、下载速度/进度语义、错误脱敏和后端固定命令；不得复制后端安装逻辑、接受 renderer 提供的 URL/路径或降低安装器安全合同。
- Browser preview 不得模拟真实安装成功；权威读写在非 Tauri 环境中保持明确不可用。

### R4 — 双栏高度解耦

- 桌面双栏布局中，左侧 Agent 目录面板保持自身内容高度，不再被右侧详情高度拉伸。
- 右侧长内容仍应通过既有页面/ContentViewport 滚动访问；本任务不新增相互竞争的嵌套滚动区。
- 单栏响应式布局继续按文档流排列，不能因桌面端修复造成溢出或不可访问内容。

### R5 — Skills/MCP 六应用分配图标

- Skills 与 MCP 共用的 Agent 分配面板必须为 Claude、Codex、Gemini、Grok Build、OpenCode、Hermes 展示对应本地图标。
- 六应用 ID 与图标必须由一个 typed V2 映射维护；可复用现有已审查资产字节，但不得直接导入旧版组件或远程 URL。
- 图标在已有文字标签旁作为装饰呈现，不能制造重复可访问名称；缺少任何已支持应用的本地图标必须由类型/资产测试失败，而不是显示破图。

## Constraints

- 不改变 Agent/模型五候选顺序、Skills/MCP 六应用分配 ID、持久化数据结构或已有用户配置。
- 不新增第三方依赖，不重写 Codex 安装器后端，不把自动安装扩展到其他 Agent。
- 不把目录候选、浏览器打开、Provider 观察或 mock fixture 描述为已安装、已登录或运行成功。
- V2 继续遵守 `src/v2` 隔离、平台端口、响应式、可访问性与 browser-preview 非权威边界。
- 保留其他活动任务和用户未授权改动；只修改本任务验收所需文件。

## Acceptance Criteria

- [x] AC1：Agent 列表与详情中的所有五个品牌图标不再出现 CSS 人为白底，比例与透明度正确。
- [x] AC2：QoderWork、TRAE Work、WorkBuddy 各显示一个准确的官方按钮；Claude 同时显示不同目标的 CLI 与 Desktop 按钮；Codex 不显示任何官方页按钮。
- [x] AC3：目录合同升级为 v2，结构化链接的 ID、标签、HTTPS URL、条目顺序和 Codex 零链接行为在 Rust与 V2 合同测试中被冻结。
- [x] AC4：真实 Tauri 应用中的官方按钮能将经校验 URL 交给系统默认浏览器；失败时显示受控错误且不执行安装或配置。
- [x] AC5：Codex 详情使用现有安装器合同呈现完整适用状态与操作；fake-Tauri 测试证明 exact IPC、事件更新、错误与操作锁，正常浏览器预览不伪造权威成功。
- [x] AC6：右侧详情增长时左侧面板保持自身高度；900x600、1152x640、1232x700、1440x900 均无新增文档溢出、遮挡或不可访问内容。
- [x] AC7：Skills 与 MCP 的分配面板都为六个受支持应用显示正确本地图标，并保持一个面板、六个唯一开关和既有可访问名称。
- [x] AC8：Agent/Models 仍使用相同五候选与顺序，Skills/MCP 仍使用现有六应用分配合同；现有配置与其他 V2 页面无行为回归。
- [x] AC9：适用的 V2 lint、type-check、unit/component、browser、renderer build、Rust format/check/clippy/test、任务合同与 diff 检查通过；未执行的真实安装或跨平台 HIL 被明确记录为剩余风险。

## Out of Scope

- 新增 Agent、模型、Skill、MCP 或新的目录数据来源。
- 把 Skills/MCP 的六应用分配集合改成 Agent/模型目录的五候选集合。
- 为 QoderWork、TRAE Work、WorkBuddy 或 Claude 新增自动安装、登录探测或私有配置写入。
- 重构无关 V2 路由、视觉系统、Provider 模型配置、安装器后端、打包或发布流程。
- 将 mock/browser 静态验证提升为真实 Codex 安装、签名、发布或跨平台兼容结论。
