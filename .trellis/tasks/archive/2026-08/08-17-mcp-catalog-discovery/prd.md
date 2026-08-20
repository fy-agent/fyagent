# MCP 精选发现与安装

## Goal

用户进入 MCP 后，既能继续管理自己已有的 MCP，也能浏览内置精选条目；无需理解 JSON 配置结构，只在必要时填写 API Key、App ID/Secret、目录或工具集，并选择要同步到哪些 Agent，即可完成注册。

用户价值：把「发现并注册常用 MCP」从手动填表变成可浏览、可筛选、凭据字段最小化的安装流程，同时不把 FyAgent 变成 npm/pip 包管理器，也不引入远程市场 API。

## Confirmed facts

- 现有 MCP 页已具备列表、搜索、详情、编辑、删除、导入现有、添加 MCP、六目标分配；安装继续走 `ports.mcp.upsert`。
- 直接 MCP 目标固定为 Claude、Codex、Gemini、Grok Build、OpenCode、Hermes；不扩增 QoderWork / TRAE Work。
- 前端 `McpServer.source` 不能稳定往返到 Rust；Catalog 的厂商、分类、风险等元数据只保留在前端静态数据，不写入后端。
- `buildMcpSearchText` 当前会索引完整 `args` 和 `url`；普通详情也会明文显示这两项。高德把 Key 放在 URL query，飞书把 Secret 放在 `-s` 参数后，因此必须先做脱敏再启用这些精选。
- `mcpPresets` 里 Time 仍使用过时的 npm `@modelcontextprotocol/server-time`；当前参考实现是 `uvx mcp-server-time`。
- `npxCommand()` 只对 Windows / macOS 给出命令，unknown 返回 `null`，导致 npx 模板在未知宿主上消失。FyAgent 原生平台检测没有独立 Linux 枚举；非 Windows 宿主应使用直接 `npx`。
- 新收录远程 MCP 只用 `type: "http"` Streamable HTTP，不为 Catalog 新增 SSE 项。
- 已锁定产品决策：已安装/发现双页签；内置人工精选；安装=注册配置并分配；凭据用轻量弹窗；同时覆盖国内服务与通用开发精选。

## Requirements

### 信息架构

- R1. MCP 页增加永久页签「已安装」和「发现」，视觉复用现有 Skills feature tabs；默认落在「已安装」。
- R2. 「已安装」保留现有导入、添加、搜索、列表/详情、编辑、删除、分配；不为发现页改写这套管理语义。
- R3. 「发现」展示内置精选卡片：搜索、分类筛选、安装状态；不出现远程市场来源切换或在线加载。

### 精选 Catalog

- R4. Catalog 为前端静态数据。每条有 typed builder，把业务字段映射到 `command` / `args` / `env` / `url` / `headers`，禁止大字符串模板替换凭据。
- R5. 首发启用：高德 `amap`、百度 `baidu-map`、飞书 `feishu`、钉钉 `dingtalk`、云效 `yunxiao`、Context7 `context7`、Playwright `playwright`、Filesystem `filesystem`、Time `time`、Memory `memory`、Fetch `fetch`。
- R6. MiniMax、GitHub MCP 保持未启用；Sequential Thinking 保留「添加 MCP」模板，不作为发现页置顶精选。
- R7. 卡片至少显示名称、短描述、来源（官方 / 官方参考实现 / 社区）、分类、运行前置、认证提示；高权限项必须显示权限风险。
- R8. 分类筛选：全部、国内服务、开发工具、办公协作、地图出行、AI 多模态、基础能力。条目可同时属于多个分类。
- R9. Windows npx 使用 `cmd /c npx -y`；非 Windows 使用直接 `npx -y`。Time / Fetch 使用 `uvx`。

### 安装行为

- R10. 「安装」只 upsert 配置并分配 Agent，不在 FyAgent 进程里执行 npm/pip/uvx。
- R11. 无必填业务字段的条目可一键安装，默认 Agent 与现有 `DEFAULT_NEW_APPS` 一致（Claude、Codex、Gemini、Grok Build）。
- R12. 有业务字段的条目打开轻量弹窗：只暴露 Catalog schema 字段和六个 Agent 多选，不暴露 command/args/env/header 实现细节。password 控件遮罩；关闭弹窗后清除输入状态。
- R13. Filesystem 至少配置一个允许目录才能安装；禁止空目录表示全盘。
- R14. 钉钉 profiles 不默认 `ALL`；用户按需选择。
- R15. 同 ID 已存在时不得静默覆盖。配置签名一致显示「已安装」；签名不同显示「已存在」，仅在用户确认「重新配置」后才 upsert。
- R16. 安装成功后刷新现有 MCP query；可选切到已安装并选中该 ID。

### 安全

- R17. 普通详情、搜索、Toast、日志不得出现 Token / Secret / AK / Key 明文。
- R18. URL query 对 `key` / `ak` / `token` / `secret` / `password` / `authorization` 等通用敏感键脱敏；搜索文本只索引 URL 的 origin 与 pathname。
- R19. 敏感 args 标志（至少 `-s`、`--secret`、`--token`、`--api-key`、`--password`）后的值在普通详情遮罩，且不得进入搜索。
- R20. `env` 与 `headers` 继续只显示项数；值仅出现在显式编辑窗或安装弹窗。

### 预设兼容

- R21. 「添加 MCP」模板入口保留。Time 改为 `uvx mcp-server-time`；Memory / Fetch 与 Catalog 命令保持一致。Context7 精选走远程 HTTP；手动模板可继续提供 npx 作为高级回退，不得在发现卡片上同时暴露两套配置。

## Acceptance criteria

- [ ] AC1. MCP 页有「已安装 / 发现」页签；默认已安装；导入现有、添加 MCP、编辑、删除、六目标分配行为无回退。
- [ ] AC2. 发现页能按名称/描述/标签/厂商/分类搜索，且搜不到用户凭据。
- [ ] AC3. 11 条首发精选均可按 schema 安装；无字段条目一键 upsert；有字段条目只填业务参数。
- [ ] AC4. 每个 Catalog builder 有单测：缺必填不可 build；Windows/macOS npx 命令正确；高德 URL 与飞书 `-s` 脱敏。
- [ ] AC5. 同 ID 不静默覆盖；重新配置需确认。
- [ ] AC6. 安装目标只有六个 MCP Target；默认值与手动添加一致。
- [ ] AC7. MiniMax / GitHub 不以可用状态出现在发现页。
- [ ] AC8. 普通详情对高德 query key 与飞书 Secret 显示遮罩；搜索 `buildMcpSearchText` 不含这些明文。
- [ ] AC9. 不改 Skills / Providers / Prompts / Memory / Settings 的用户可见行为；不新增后端市场 API 或 MCP schema 迁移。
- [ ] AC10. `mise run lint:v2`、`mise run typecheck:v2`、`mise run test:v2`、`mise run test:v2:browser`、`mise run build:renderer`、`mise run format:check`、`git diff --check` 通过。

## Out of scope

- 接入 ModelScope、腾讯云、Smithery 或其他远程 MCP 市场 API。
- 在 FyAgent 内执行全局包安装、运行时探测/自动安装 Node/uv/Docker。
- 通用 OAuth 浏览器授权、Keychain 加密承诺。
- 为 Catalog 元数据扩展 Rust `McpServer`。
- 把 MCP 目标扩到 QoderWork / TRAE Work。
- 启用 MiniMax、GitHub MCP，或把 Sequential Thinking 做成发现页置顶精选。
- 改动 Skills、Providers、Prompts、Memory、Settings 页面。

## Technical notes

- Catalog 安装成功后的持久化实体只包含现有统一 MCP 字段。
- 未知宿主平台按非 Windows 处理 npx，不新增 Linux 产品承诺。
- 远程精选只用 HTTP，不新增 SSE catalog 项。
