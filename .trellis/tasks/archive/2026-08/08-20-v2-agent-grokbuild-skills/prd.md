# V2 Agent 目录、Grok Build 与 Skills 本机识别

## Goal

把 V2 Agent 目录收成「只展示支持的功能」的导航面，抽出共享产品目录给提示词页对齐；把 Grok Build 接入 Agent / Skills / MCP / Models / Prompts（记忆页不动）；修复本机已存在于 Codex 等应用目录中的 Skills 在「已安装」中不可见的问题；让 Skills 发现页按整个功能页滚动。

## Confirmed facts

- 本机 `~/.codex/skills` 有大量带 `SKILL.md` 的用户 Skills（如 `lark-*`、`grill-me`）；`~/.fyagent/skills` 为空。`get_installed_skills` 只读 SQLite，磁盘 Skills 被藏在「导入本地 Skill」后面。这不是 Mac 路径分隔符问题，Windows 侧往往是经 FyAgent 安装所以库里有记录。
- Agent 详情目前还渲染应用状态、配置概览、不适用功能、支持计数、使用说明、Qoder Hooks 编辑器和 TRAE/Qoder MCP 校验面板。
- Qoder/TRAE 右上角官方按钮已是主按钮；其余 Agent 的打开按钮样式不统一。
- Skills/MCP 后端已有 `grokbuild` 目标；V2 前端六目标列表把它藏掉了。Prompts 已有 Grok Build。Provider quick setup 目前只允许 claude/codex。
- Skills 发现页在 `.fy-feature-page:has(.fy-feature-workspace)` 的 overflow hidden 里再套一层 `.fy-feature-discovery-scroll`，可视区域被压成窄条。

## Requirements

### Agent 目录

- R1. 除 QoderWork / TRAE Work 的官方入口文案保持目录原文外，其余有官方链接的 Agent 右上角打开按钮统一为主按钮样式；抽到共享组件。Codex 仍无官方链接、仍挂桌面安装器。
- R2. Agent 详情只展示 `mode === "direct"` 的功能，以及对应跳转（模型 / Skills / MCP）。移除：应用状态、配置概览、不适用的功能、支持计数、「暂无法确认」、使用说明、Hooks 编辑、MCP 校验面板。
- R3. Qoder 模型页「管理 Hooks 和 MCP」改为进入 MCP 页，不再把用户带到已删除的 Agent 内嵌面板。

### 共享目录与 Grok Build

- R4. 抽出一份产品目录，作为 Agent / Skills / MCP / Models 的顺序与显示名单一来源。提示词左栏按该目录对齐（有提示词后端的条目），提示词独有应用（Gemini / OpenClaw / Hermes）跟在后面。记忆页不改。
- R5. Agent 目录加入 Grok Build（官方页 `https://x.ai/grok`）。Skills、MCP、Models 增加对应 Grok Build 支持；Models 走现有 Provider 快速配置边界并扩展到 `grokbuild`。

### Skills 已安装识别

- R6. `get_all_installed` 必须把各目标应用 skills 目录里已存在、且带 `SKILL.md` 的条目并入已安装列表。GET 不写库、不复制到 SSOT。点开头目录（如 Codex `.system`）仍跳过。
- R7. 对尚未入库的已观察 Skills，首次 toggle / uninstall 再按现有 `import_from_apps` 收养到 SSOT。不要在列表读取时静默接管并删掉用户原目录。

### Skills 发现滚动

- R8. 发现页由 Skills 功能页（含「从仓库或 skills.sh 浏览可安装的 Skills。」标题所在父容器）整体滚动，而不是只滚动标题下方那一小条卡片区。

### MCP / 模型审查

- R9. 保持：WorkBuddy 模型可写；Qoder 不支持第三方模型；TRAE 不通过改本地 sqlite 让 Work CN 识别自定义模型。审查其余 Agent 的 Skills/MCP/模型契约，发现破坏性偏差才改。

## Acceptance criteria

- [ ] AC1. 各目标应用 skills 目录里带 `SKILL.md` 的用户 Skills（Codex、Claude、WorkBuddy、Grok Build、Qoder、TRAE、OpenCode 等）都出现在已安装列表，并标记发现来源；点开头目录（如 Codex `.system`）不下发。GET 不写库。
- [ ] AC2. Agent 详情不再出现应用状态 / 配置概览 / 不适用功能 / 支持计数 / 使用说明 / Hooks / MCP 校验。
- [ ] AC3. WorkBuddy / Claude / OpenCode 官方按钮与 Qoder/TRAE 同为主按钮样式；Qoder/TRAE 文案不变。
- [ ] AC4. Agent / Skills / MCP / Models 左栏含 Grok Build；提示词左栏与 Agent 目录对齐且仍能选 Gemini / OpenClaw / Hermes；记忆页不变。
- [ ] AC5. Skills 发现页从标题所在功能页开始滚动，不再只露出窄条卡片。
- [ ] AC6. 相关 V2 / Rust 测试更新并通过。

## Out of scope

- 记忆页、Claude Desktop 提示词、TRAE sqlite 写入、Qoder 第三方模型编辑器。
- leftover V1 Skills 页改版。
