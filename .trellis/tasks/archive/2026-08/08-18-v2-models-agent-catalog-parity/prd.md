# V2 模型目录与 Agent 能力对齐

## Goal

让 V2 的 Agent 目录、模型页、Skills、MCP 对同一组应用说同一套话：名称和图标正确，支持的能力可跳转，模型页能真正拉取/添加/删除模型，而不是“测试”或“请去应用里完成”。

用户打开模型页或 Agent 目录时，应能立刻看出每个应用 **支持什么、不支持什么、下一步去哪**，并且对 TRAE Work CN、OpenCode、Claude Code 完成与 WorkBuddy 同级的模型管理。

## Background

当前生产壳是 `src/v2`。Agent 目录与模型页共用 catalog v3 六条目：QoderWork CN、TRAE Work、WorkBuddy、Codex、Claude Code、OpenCode。Skills/MCP 的分配目标仍是 leftover 的 Gemini / Grok Build / Hermes 集合，与目录不一致。

已核实：

- TRAE 官网入口仍是 `https://work.trae.cn/`，侧栏文案是 “TRAE Work” / “测试模型连接”。
- QoderWork CN 目录能力已标模型为 unsupported，但模型页仍引导去完成模型设置。本机 `/Applications/QoderWork CN.app` 图标是绿色卡通脸，仓库里的 `qoderwork.svg` 是错误的立方体。
- WorkBuddy V2 已具备连接、拉取、添加、删除、保存。OpenCode leftover 表单已能写 `opencode.json`，V2 只有引导。Claude leftover 已能 `fetch_models_for_config`，V2 只有单模型快配。
- TRAE SOLO CN 自定义模型实际落在 `User/globalStorage/state.vscdb` 的 `AI.agent.model.model_list_map`，行内含 `ak`/`sk`。官方文档要求添加时做连接检测。本机暂无自定义行可当模板。
- WorkBuddy 本机有 `~/.workbuddy/skills` 与 `~/.workbuddy/.mcp.json`。

## Requirements

### R1 TRAE Work CN 身份

- 所有用户可见名称由 “TRAE Work” 改为 **TRAE Work CN**（含目录、模型侧栏、Skills/MCP 标签、测试夹具文案）。稳定 id 仍为 `trae-work` / `trae-work-cn`。
- 官方 product 链接改为 `https://www.trae.cn/sem-work`。仍由 Rust catalog 持有 URL，渲染层继续走 `ExternalLinkButton`。

### R2 QoderWork CN 模型不支持

- 模型页不得再写“在 QoderWork 中完成模型设置”或把 Hooks/MCP 包装成模型能力。
- 明确说明 **不支持第三方模型配置**。可保留跳到 Agent 目录管理 Hooks，以及打开官网。
- Catalog 中 `models.validate` / `models.write` 保持 `unsupported`。

### R3 模型页：TRAE Work CN、OpenCode、Claude Code 达到 WorkBuddy 完整度

WorkBuddy 已有能力是验收下限：读取已有模型、拉取远程模型、添加、删除、保存。禁止再使用“测试”“请在应用中完成模型设置”作为主状态。

- **TRAE Work CN**：连接设置 + 拉取 + 已有自定义模型列表 + 添加/删除 + 保存到 TRAE SOLO CN 的 `model_list_map`（见 design）。保存前仍做 URL/Key 准入和连接检测。永不把 `ak`/`sk` 送到 V2。
- **OpenCode**：在 V2 内管理 `opencode.json` 的第三方模型（复用 leftover 已有后端能力，不 import leftover React）。
- **Claude Code**：在现有 reserved quick setup 上增加拉取和模型芯片；保存仍走 `applyQuickSetupWithResult`。Codex 保持现有快配，仅补模型图标。
- 每个展示出的模型名称前有本地小图标（见 R6）。

### R4 Agent 目录文案与跳转

- 能力状态用短词：**支持** / **需在应用中完成** / **不支持** / **暂无法确认**。禁止 “可在 FyAgent 中完成” 这类主语含混的句子。
- 详情默认展示 **支持的功能** 和对应跳转（模型 / Skills / MCP / Hooks / 官网）。不适用项不占满一屏。
- 跳转只用已有路由 query（`/models?target=`、`/skills`、`/mcp`）和 catalog HTTPS 链接。不传可执行路径，不把 `null` 运行时显示成“未安装”。

### R5 Skills / MCP 与目录对齐

- V2 Skills 分配目标：`claude`、`codex`、`opencode`、`qoderwork`、`trae-work`、`workbuddy`。
- V2 MCP 分配目标：`claude`、`codex`、`opencode`、`workbuddy`。
- leftover 的 Gemini / Grok Build / Hermes 标志保留在后端，V2 页面不展示。
- 补齐 WorkBuddy Skills 复制到 `~/.workbuddy/skills`，MCP 写入 `~/.workbuddy/.mcp.json`。不把 WorkBuddy 做成 `AppType`。
- QoderWork / TRAE Work 不进入 MCP 直接分配。

### R6 模型图标

- 凡模型页展示模型 id/名称（已有列表、草稿、拉取结果、Claude/Codex 当前模型），左侧有约 14px 的本地 vendor 图标。
- 图标来源优先 leftover `src/icons/extracted/`，拷贝进 `src/v2/shared/assets/models/`。缺失的常见厂商再从公开商标 SVG 补进 V2 资源。禁止远程 URL。

### R7 QoderWork CN 图标

- 用本机 QoderWork CN.app 的 `icon.icns` 生成的 PNG 替换错误立方体 SVG。Agent 目录、模型侧栏、Skills 目标共用该资源。

## Out of Scope

- 不把 Gemini / Grok Build / Hermes / OpenClaw 加进 Agent 目录。
- 不把 WorkBuddy 升级为通用 `AppType`（Provider/Session/Usage）。
- 不写 QoderWork / TRAE 的 MCP 厂商私有 connectors 文件。
- 不重做 Codex Desktop 安装器，不为 Codex 做第二套 WorkBuddy 式多模型表。
- 不修改 leftover V1 路由或四语言 i18n（除非测试夹具必须跟着 catalog 文案改）。
- 不做真实 TRAE/OpenCode/Qoder Windows HIL。

## Acceptance Criteria

- [ ] AC1 模型侧栏与 Agent 目录显示 “TRAE Work CN”；打开官方入口的 IPC URL 为 `https://www.trae.cn/sem-work`。
- [ ] AC2 QoderWork CN 模型页主文案声明不支持第三方模型配置；不再出现“在 QoderWork 中完成模型设置”。
- [ ] AC3 TRAE Work CN 模型页可拉取模型、添加/删除自定义模型并保存；成功文案不再写“请回 TRAE 保存”；密钥不出现在 DOM/URL/storage/query。
- [ ] AC4 OpenCode 模型页可读取已有模型、拉取、添加、删除并写入 OpenCode 配置；不再是纯引导页。
- [ ] AC5 Claude Code 模型页可拉取模型并以芯片添加/删除后走 quick setup 保存。
- [ ] AC6 Agent 目录能力徽章为“支持/需在应用中完成/不支持/暂无法确认”；支持项可跳到对应功能页或官网。
- [ ] AC7 Skills 页分配开关为 R5 的六个目标，含 WorkBuddy；MCP 页为四个目标，含 WorkBuddy、不含 Qoder/TRAE 直接分配。
- [ ] AC8 模型芯片/名称左侧有本地小图标；未知厂商有中性占位，无远程图。
- [ ] AC9 QoderWork CN 列表/详情图标来自本机应用提取的绿色卡通脸资源，不再使用立方体 SVG。
- [ ] AC10 `mise run lint:v2 typecheck:v2 test:v2` 以及受影响的 `rust:test` catalog/skill/mcp 测试通过。

## Technical Notes

- Catalog 形状保持 v3；改的是 displayName、URL、capability mode/reason、前端文案。
- V2 不得 import `src/components`、`src/hooks`、`src/lib`、`src/i18n`。
- 密钥与 MCP env/headers 规则不放宽。
