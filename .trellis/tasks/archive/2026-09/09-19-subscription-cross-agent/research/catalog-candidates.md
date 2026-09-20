# Research: Agent 目录扩展候选

- Query: 用户已澄清要找新的 AI 编程／办公工具；在当前 7 项 Agent 目录之外，核对 Gemini CLI 是否停维护、Google Antigravity 的当前定位，并筛选适合 FyAgent 扩展的独立 Agent。
- Scope: mixed
- Date: 2026-09-19

## Findings

### 当前基线与判断标准

当前目录由 `src-tauri/src/commands/agent_catalog.rs:642-699` 固定为 7 项：QoderWork CN、TRAE Work CN、WorkBuddy、Grok Build、Codex、Claude Code、OpenCode。`src/shared/features/directory.ts:62-66` 将 Gemini、OpenClaw、Hermes 放在 `PROMPT_ONLY_DIRECTORY`，所以它们已有提示词入口，但不是 Agent Catalog 产品卡。

目录卡要求的不只是名称：后端 catalog contract 暴露 product link、安装/探测/启动、Skills、Hooks、模型和 MCP capability（`src/shared/features/agents.ts:12-24,85-99`）；运行时当前仍对外部 Agent 统一保持证据边界，`src-tauri/src/services/external_agents/mod.rs:231-245` 将已有非 Codex 产品标为 `Unverified/TrustedRuntimeIdentityUnavailable`。因此候选排序优先考虑：已有 AppType/配置读写/MCP/Skills 接口能否复用、官方是否提供稳定 CLI/配置协议、是否有独立 Agent 身份，而不是只看热度。

### 排序建议

| 排名 | 候选                                  | 类型                                             | 价值                                                                                                                              | 当前接入成熟度                                                                                                                                             | 最小接入                                                                                                                                                     | 主要风险                                                                                                        |
| ---- | ------------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| 1    | Google Antigravity（2.0 / CLI / IDE） | 独立桌面 Agent、终端 CLI、Agentic IDE            | Google 当前主推的 agent-first 平台，覆盖并行 Agent、浏览器、Artifacts、Skills/MCP 与长任务；比单一 CLI 更贴近“编程／办公工具”目录 | 低：当前仓库没有 Antigravity AppType、catalog ID 或配置 adapter；官方已有独立安装、CLI 和统一认证                                                          | 先做独立产品卡，覆盖安装/启动/登录/版本与桌面/CLI 入口；后续再评估 agent status、Skills/MCP 和任务回读，不复用 Gemini CLI auth 假设                          | 账户额度和权限由统一 Google 认证控制；桌面、CLI、IDE/扩展是不同表面，远程/浏览器 Agent 的状态不能由本机进程推断 |
| 2    | Gemini CLI                            | 独立终端 Agent（企业/API 条件成立）              | CLI 仍有官方 stable/preview/nightly 发布，企业 Gemini Code Assist 与 API key 路径仍受支持；可补齐现有 Gemini 配置基础             | 高：AppType、配置、模型、代理、MCP 代码已在仓库；但个人 Pro/Ultra/免费层自 2026-06-18 起不再由 Gemini CLI 提供请求                                         | 仅面向企业/API 用户增加 catalog ID/variant、安装与 runtime evidence；不要把个人 Google 订阅登录写成可用闭环                                                  | 个人订阅迁移到 Antigravity；企业、API key、Cloud Project/配额分支不同，真实 entitlement 不能由文件存在推断      |
| 3    | Hermes Agent                          | 独立 CLI + Desktop/Web Dashboard + Gateway Agent | 一次 Nous Portal OAuth 可覆盖模型与工具网关；本地模型、Telegram/Discord 等远程入口与 Skill Hub 适合工作室自动化                   | 高：`AppType::Hermes`、YAML config writer、provider/model、MCP/Skills 目标、工具生命周期已有；当前只在 prompt-only 语义中                                  | 增加目录卡、安装/启动/认证官方链接和 runtime evidence；复用 `hermes_config.rs` 与现有 MCP/Skill adapter                                                      | 配置/凭据可能属于远程 Gateway；模型、工具和频道是多层状态，不能把 Dashboard 可见当作本机 Agent runtime 已验收   |
| 4    | OpenClaw                              | 独立 Gateway Agent / 多渠道自动化平台            | 适合把模型、频道、Gateway、Skills 和远程节点统一编排；当前已有 OpenClaw 配置/MCP/Skills/提示词基础                                | 中高：`AppType::OpenClaw`、additive config、MCP/skill/prompt 代码已有，但代理明确关闭（`src-tauri/src/services/proxy.rs:1090-1092`），并非普通本地编码 CLI | 先作为“Gateway Agent”目录卡，加入安装、Gateway 状态、模型/auth status、Skills/MCP capability；不宣称 local proxy 或本机模型路由                              | Gateway 可能在另一台机器；OAuth/profile 属于 agent store；频道与 Agent 本体分开，权限和远程写入边界复杂         |
| 5    | Cursor Agent CLI / Cursor Desktop     | 独立商业编码 Agent（CLI + IDE）                  | 用户覆盖面大，官方已把 CLI、Rules、Skills、MCP、Plugins 聚合；可成为开发者目录入口                                                | 低：当前代码没有 Cursor `AgentCatalogId`、配置读写或安装/认证 adapter；需新 adapter                                                                        | 先做 CLI-only catalog entry：官方安装/登录/版本/启动、`.cursor/mcp.json` 与 `.cursor/rules` 只读/受控写；模型通过 CLI `--model`/Cursor 账户，不自建 provider | 账户/云端策略、私有 auth store、IDE 与 CLI 行为差异；“可管理 MCP/Rules”不等于可管理 Cursor 订阅或模型额度       |

结论：若按 Google 当前产品方向和个人用户可用性排序，第一候选改为 Antigravity；Gemini CLI 仅作为企业/API 条件候选保留。若目录要体现工作室 Agent，则 Antigravity + Hermes 是第一批组合；OpenClaw 适合第二批 Gateway 卡，Cursor 仍是高成本预研项。官方资料同时证明“Gemini CLI 个人订阅路径被迁移”与“Gemini CLI 项目停止维护”不是同一命题，不能把前者夸大成后者。

### 0. Google Antigravity：当前主推方向，但接入成本最高

官方资料（截至 2026-09-19）：

- Google Developers Blog 在 2026-05-19 宣布将终端体验从 Gemini CLI 统一到 Antigravity，称 Antigravity 是 agent-first development platform，并推出 Antigravity CLI；公告说明个人 Google AI Pro/Ultra 与免费层自 2026-06-18 起停止由 Gemini CLI 提供请求，企业 Gemini Code Assist、Enterprise Agent Platform API key 路径继续支持 Gemini CLI。来源：[Transitioning Gemini CLI to Antigravity CLI](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)。
- Antigravity 2.0 官方概览定位为独立桌面应用和 Agent command center，可启动、监控、编排多个 Agent，并使用 Skills、MCP、浏览器和 artifacts；官方产品页同时列出 Antigravity CLI、SDK、IDE。来源：[Antigravity 2.0 overview](https://www.antigravity.google/docs/overview)、[Antigravity product](https://www.antigravity.google/product)。
- 官方 CLI 安装与认证文档提供 `agy` 安装、系统 keyring/浏览器登录、SSH OAuth 和 Gemini API key 模式；官方 IDE 扩展文档称 IDE 扩展、CLI、Antigravity 2.0 使用统一认证系统。来源：[CLI installation & auth](https://antigravity.google/docs/cli-install?hl=en)、[IDE extensions](https://antigravity.google/docs/ide/extensions/)。

仓库证据：当前 `AppType`、目录 catalog、配置和 MCP/Skills adapter 没有 Antigravity 专属枚举或配置路径；因此它的产品战略契合度高，但代码接入成熟度低，不能仅把现有 Gemini CLI 条目改名复用。最小可靠范围是新增独立 catalog entry，先管理官方安装/启动/登录入口与版本/runtime evidence，再设计桌面、CLI、IDE/扩展之间的状态边界。

### 1. Gemini CLI：企业／付费 API 条件候选

官方资料（截至 2026-09-19）：

- 安装：官方支持 `npm install -g @google/gemini-cli`、Homebrew 等，并以 `gemini` 启动；要求 Node.js 20+。来源：[Gemini CLI installation](https://geminicli.com/docs/get-started/installation/)（页面最后更新 2026-05-14）。
- 认证：个人 Google AI Pro/Ultra 与免费层的 Gemini CLI 请求路径已被 Google 2026-06-18 的迁移公告明确取代，不能再作为 FyAgent 的个人订阅可用依据；当前应只把企业 Gemini Code Assist、Enterprise Agent Platform API key、`GEMINI_API_KEY`、Vertex AI ADC/service account/API key 视为条件路径。原认证文档仍列个人浏览器登录，属于迁移前/未同步的通用说明；以生效日期明确的[官方迁移公告](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)为准，并参照[Gemini CLI authentication](https://geminicli.com/docs/get-started/authentication/)。
- 模型：`/model` 与 `--model` 支持 Auto、Pro、Flash、Flash-Lite/具体模型；优先级是 CLI flag → `GEMINI_MODEL` → `settings.json` → 默认 Auto。来源：[Gemini model selection](https://geminicli.com/docs/cli/model/)、[model routing](https://geminicli.com/docs/cli/model-routing/)。
- Skills：`.gemini/skills/<name>/SKILL.md`，另有 `~/.gemini/skills/` 与 `.agents/skills/` alias；提供 `gemini skills list/install/uninstall` 和 `/skills` 管理。来源：[Gemini Agent Skills](https://geminicli.com/docs/cli/skills/)、[creating skills](https://geminicli.com/docs/cli/creating-skills/)。
- MCP：官方教程使用配置中的 MCP server，支持重启后 `/mcp list` 验证连接；来源：[Gemini MCP setup](https://geminicli.com/docs/cli/tutorials/mcp-setup/)（最后更新 2026-04-10）。

仓库证据：`AppType::Gemini` 已存在于 `src-tauri/src/app_config.rs:634-680`；Gemini `.env`/`settings.json` 路径及 OAuth/API key selected type 写入在 `src-tauri/src/gemini_config.rs:8-20,340-430`；MCP sync/import 已覆盖 Gemini，`src-tauri/src/services/mcp.rs:158-160,422-430`；proxy takeover 已有 Gemini 状态和 live config 路径（`src-tauri/src/services/proxy.rs:1078-1099`）。缺口主要是把 prompt-only `gemini` 提升为 catalog ID，并补 runtime identity/evidence；`MCP_TARGET_IDS` 与目录常量当前没有 Gemini，需按 capability contract 谨慎扩展。

最小接入不应把 Google OAuth token 导入 FyAgent vault：先管理安装/版本、企业或付费 API 登录入口、配置选择和可验证的模型/Skills/MCP 文件；真实账号登录、Cloud Project entitlement 和 quota 属于 Gemini 官方回读/运行时证据。个人订阅实践优先转向 Antigravity。

### 2. Hermes Agent：最适合工作室型 Agent

官方资料（截至 2026-09-19）：

- 安装：macOS/Windows 可用 Hermes Desktop；CLI 安装脚本为 `curl -fsSL https://hermes-agent.nousresearch.com/install.sh | bash`（Windows 有 PowerShell 安装器）。来源：[Hermes Agent docs](https://hermes-agent.nousresearch.com/docs/)。
- 认证/订阅：`hermes setup --portal` 通过 Nous Portal OAuth，写入 `~/.hermes/auth.json`，设置 `~/.hermes/config.yaml`，并启用 Tool Gateway；一套订阅覆盖 Nous 模型及工具网关。来源：[Nous Portal](https://hermes-agent.nousresearch.com/docs/integrations/nous-portal)、[free tier and signing in](https://hermes-agent.nousresearch.com/docs/user-guide/free-tier)。
- 模型：`config.yaml` 的 `model` 为 provider/default/base_url/api_mode 结构；Dashboard、`hermes model`、`/model` 可切换主模型与辅助模型。来源：[Configuring models](https://hermes-agent.nousresearch.com/docs/user-guide/configuring-models)。
- Skills/MCP：内置和 Skills Hub 技能位于 `~/.hermes/skills`，`hermes skills browse/search/install` 管理；MCP 写入 `~/.hermes/config.yaml` 的 `mcp_servers`。来源：[Hermes quickstart](https://hermes-agent.nousresearch.com/docs/getting-started/quickstart)。
- Dashboard：本机 `hermes dashboard` 默认 localhost:9119，可管理 API keys、模型和 sessions；公开绑定会启用密码/OAuth/OIDC gate。来源：[Hermes Web Dashboard](https://hermes-agent.nousresearch.com/docs/user-guide/features/web-dashboard)。

仓库证据：`AppType::Hermes` 已存在并采用 additive mode，`src-tauri/src/app_config.rs:634-680`；MCP target 与 sync/import 已支持 Hermes（`src-tauri/src/services/mcp.rs:175-176,449-456`）；`hermes_config.rs` 已有 YAML 读写、model/provider 归一化和写入保护；生命周期显示名已存在 `src-tauri/src/services/tooling/lifecycle.rs:90-99`。缺口是 catalog entry、可信 runtime identity 和对多层 Gateway 状态的清晰 UI。

风险边界：Hermes 的 Portal OAuth、Gateway 工具权限和远端频道不能被当作普通本机 provider。目录第一版应显示“官方登录/本机配置/Gateway 状态”三层，而不是只显示一个已登录徽章。

### 3. OpenClaw：适合作为 Gateway/多渠道 Agent 卡

官方资料（截至 2026-09-19）：

- 安装：官方安装页支持 macOS menu bar app、Windows Hub、CLI installer；桌面应用可在首次运行时创建本地 Gateway 或连接远程 Gateway。来源：[OpenClaw install](https://docs.openclaw.ai/install)。
- 认证：支持 API key、OAuth、SecretRef；OpenAI ChatGPT/Codex OAuth 使用 `openclaw onboard --auth-choice openai` 或 `openclaw models auth login --provider openai`，profile 归属 agent store。来源：[OpenClaw authentication](https://docs.openclaw.ai/gateway/authentication)、[official provider plugins](https://docs.openclaw.ai/concepts/model-providers/official-provider-plugins)。
- 模型：模型引用为 `provider/model`，`openclaw models list/status/set/auth` 管理；OpenAI OAuth route 可进入原生 Codex app-server，但必须以官方 route/runtime policy 读取状态，不能从文件名推断。来源：[OpenClaw models](https://docs.openclaw.ai/cli/models)。
- Skills：workspace `skills/`、global `~/.openclaw/skills`、ClawHub 安装/更新/verify；官方还提供 Skill Workshop。来源：[OpenClaw skills](https://docs.openclaw.ai/skills)、[Skills CLI](https://docs.openclaw.ai/cli/skills)。
- MCP/频道：OpenClaw 是 Gateway/多渠道 Agent；官方 provider 文档明确模型 provider 与 Telegram/WhatsApp/Slack 等 chat channels 是不同层。当前 FyAgent 有 OpenClaw app config/MCP/skills/prompt 支持，但 `src-tauri/src/services/proxy.rs:1090-1092` 明确关闭 OpenClaw proxy takeover。

最小接入是目录卡 + Gateway-aware runtime + model/auth status + Skills/MCP links；不要把“支持微信/Telegram/飞书频道”误写成 OpenClaw 本地安装成功，也不要第一批承诺代理接管。

### 4. Cursor：高价值、但不适合作为第一批低成本扩展

官方资料（截至 2026-09-19）：

- 安装/启动：桌面安装器来自 cursor.com；CLI 通过 `curl https://cursor.com/install -fsS | bash` 安装，`cursor-agent` 启动。来源：[Cursor installation](https://docs.cursor.com/get-started/installation)、[Cursor CLI installation](https://docs.cursor.com/en/cli/installation)。
- 认证：Cursor CLI 支持浏览器登录（`cursor-agent login/status/logout`）和 `CURSOR_API_KEY`；认证保存于本地，但官方没有为 FyAgent 暴露可安全迁移的 auth store。来源：[Cursor CLI authentication](https://docs.cursor.com/en/cli/reference/authentication)。
- 模型：CLI 支持 `--model`；模型/订阅主要由 Cursor 账户与云端产品控制，不能等同于 FyAgent Provider model file。来源：[Cursor CLI parameters](https://docs.cursor.com/en/cli/reference/parameters)。
- Skills/Rules/Plugins：官方 Customize 页面管理 Plugins、Skills、Rules、Subagents、Commands、Hooks；项目规则在 `.cursor/rules`，旧 `.cursorrules` 已 deprecated。来源：[Customize Cursor](https://prod.cursor.com/docs/customize-cursor)、[Cursor Rules](https://docs.cursor.com/en/context/rules)。
- MCP：项目 `.cursor/mcp.json` 与全局 `~/.cursor/mcp.json`；Cursor CLI 自动读取同一 `mcp.json`，支持 OAuth/stdio/SSE/Streamable HTTP。来源：[Cursor MCP](https://docs.cursor.com/context/model-context-protocol)、[Cursor CLI using](https://docs.cursor.com/en/cli/using)。

Cursor 适合作为未来独立 `cursor` catalog entry，但最小接入仍需新的 trusted identity、安装/启动 adapter、`.cursor` 文件读写 contract、CLI/IDE 分层和账户状态边界；不应只把 Cursor MCP 文档链接塞进现有目录。

### 独立 Agent 与微信/飞书连接器的边界

独立 Agent 有自己的可启动产品/CLI、模型选择、认证状态、工作区配置和技能/MCP生命周期，应进入 `PRODUCT_DIRECTORY`/后端 Agent Catalog。Antigravity、Gemini CLI、Hermes、OpenClaw、Cursor 属于这类候选。本轮用户已澄清目标是新的 AI 编程／办公工具，因此不把微信/飞书渠道作为本轮候选。

微信、飞书、Telegram、Slack 等是连接器/频道/插件：它们提供消息、文档、表格、审批或远程触发入口，依附于 Agent/Gateway/MCP server，本身通常没有独立的本机 Agent runtime、模型 picker 或 Skills store。它们更适合进入 MCP/Connector/Channel 管理面，不应伪装成 Agent 目录卡。OpenClaw 官方也把 model providers 与 chat channels 分开描述（[Model providers](https://docs.openclaw.ai/concepts/model-providers)），这是该边界的直接依据。

## Related specs

- `.trellis/spec/backend/external-agent-catalog-runtime.md`
- `.trellis/spec/backend/external-agent-configuration.md`
- `.trellis/spec/backend/external-agent-lifecycle.md`
- `.trellis/spec/backend/external-agent-models.md`
- `.trellis/spec/backend/external-agent-auth.md`
- `.trellis/spec/backend/mcp-management.md`
- `.trellis/spec/backend/skill-management.md`
- `.trellis/spec/backend/proxy-runtime.md`

## Caveats / Not Found

- 研究只读当前隔离工作树；工作树存在主控并行修改的 dirty 文件，本报告未触碰或覆盖这些修改。
- 未启动测试、未启动 Agent、未访问本机账号或凭据；所有安装/认证/订阅结论来自官方公开资料与静态代码。
- 官方文档页面的版本、模型名、认证菜单和安装命令会变化；本报告记录抓取日期 2026-09-19，正式实现前需再次核对版本与官方协议。
- 用户已澄清“AI 微信工具”实际指新的 AI 编程／办公工具；本报告已按独立 Agent 产品排序。微信/飞书连接器不属于本轮范围。
