# Agent 一键安装与 Codex 多账号认证

## Goal

在不引入第二套 Agent 注册表、第二套安装器或第二套 OAuth 管理器的前提下，完成两条能力：

1. **P0 — Agent 安装 / 更新与原生登录入口**：让 Agent 目录按各产品真实分发与认证模型提供可执行的一键安装/更新、安装状态和登录入口；中国大陆产品存在稳定第一方分发契约时直接使用第一方下载，不再只跳官网。
2. **P1 — Codex 多 ChatGPT 账号**：在 FyAgent 已有 `CodexOAuthManager`、统一 Auth Center 和 `ProviderMeta.authBinding` 上完善真正的多凭据账号管理、Provider 绑定、并发/取消和 Codex 原生凭据投影，避免串号、覆盖和错误账单归属。

本任务以**复用现有 FyAgent 与可审计上游实现**为第一原则。只有现有边界无法表达需求时才增加薄适配层，不创建通用 shell/URL/path 执行能力。

## Scope

### P0 — Agent 安装、更新与 Agent-owned Auth

- Canonical Agent 集合保持现有 7 个：`qoderwork | trae-work | workbuddy | grokbuild | codex | claude-code | opencode`。
- **Pi 明确不在本任务范围内**，不得因 CC Switch v3.20.0 同步而加入 catalog、安装、认证、UI 或测试。
- Agent Catalog 继续是唯一产品/能力 SSOT；现有 `agent_install` readiness contract 演进为受控编排入口，不建立第二个 installer registry。
- CLI 类能力优先复用 `services/tooling.rs` 及 `services/tooling/{discovery,lifecycle,terminal,versions}.rs`：
  - Claude Code：同步当前官方 native installer / `claude update` 事实；原生认证使用官方 `claude auth login/logout/status`，不解析或复制 Claude 凭据。
  - Grok Build：复用官方 installer、自更新与现有 Tooling 探测；登录/登出由 `grok` 官方命令拥有，FyAgent 不读取 `~/.grok/auth.json`。
  - OpenCode：复用现有 install/update 探测；认证按 **Provider connection** 建模，入口启动官方 `/connect`/等价稳定 CLI 流程，不伪造“全局 OpenCode 登录态”，不读取 `auth.json`。
  - Codex CLI 生命周期保持现有独立边界；本任务不得顺手开启当前明确关闭的通用 Codex CLI install/update 路径。Codex 桌面包继续走 managed-package installer。
- 桌面可执行包统一受 `.trellis/spec/backend/codex-desktop-installer.md` 的 **One-click Executable Software Installer Contract** 约束：
  - 复用并逐步泛化现有 Codex Desktop source/download/job/cancel/temp/post-install-verify 编排内核；不得再造第二套下载 Job。
  - 只有包格式相同才复用现有具体部署适配器：macOS DMG 可优先复用受控 mount/copy/replace 事务；Windows MSIX 可复用 PackageBridge/PackageManager。若 QoderWork/TRAE Work/WorkBuddy 官方实际提供 EXE/NSIS/MSI/PKG 等不同格式，只允许在同一个 managed-package core 下增加**闭集 package-format adapter**，不得把 Codex MSIX 机制硬套到不同格式，也不得引入任意 executable/path runner。
  - QoderWork CN：使用官网当前自身发布的固定 `/qoder-work-cn/releases/latest/` User-x64 / macOS ARM64 / macOS x64 第一方别名。该入口可以可靠取得“最新版安装包”，但当前没有同等级的机器可读远端 semver 证据，因此远端版本保持 `unknown`；不使用 Last-Modified/ETag/文档版本猜 semver。
  - TRAE Work CN：使用官方 `api.trae.cn/icube/api/v1/native/version/trae/cn/latest`（同契约下可 bounded fallback 到官网代码当前使用的 `api.trae.ai`），只读取 `data.solo` + `region=cn`，按平台/架构从返回值选择并严格校验 `TraeWork_CN-*` 第一方 URL；不得读取 `data.manifest` 的 TRAE Code 包。
  - WorkBuddy：使用官网自身的 `/v2/update?platform=<closed-id>` latest API；Windows 使用通过 allowlist/grammar 校验的返回 `.exe`，macOS 仅按官网当前相同规则把已验证平台 URL 的精确 `.zip` 后缀改成 `.dmg`。API 返回版本用于 update/display，返回 hash 不作为 FyAgent executable admission gate。
  - 中国大陆优先使用产品自己的第一方 CN 分发；“国内可下载”不是允许第三方镜像、抓取搜索结果或按 IP 猜源。
  - 不硬编码带当前版本号的 CDN URL，不把网页 bundle/DOM 抓取当稳定安装 API。
  - 任一 source resolver 请求失败、schema/host/path/format 漂移或目标包不可达时，readiness 必须明确降级为 source unavailable / official-page fallback；不得把任务调研时看到的具体版本 URL 固化成 stale fallback。
- 不改变现有 executable-installer 的信任模型：远端 checksum、size、publisher/Team ID、package version/arch/min-OS、Gatekeeper/notarization 等 publication 字段不得新增为 FyAgent 自己的下载内容 admission gate；固定产品源、bounded metadata、HTTPS/redirect/timeout/cancel/body cap、受保护临时对象、OS 原生安装和 post-install operational verification 继续保留。
- Agent 原生认证必须有明确 ownership：
  - `fyagent_managed`：FyAgent Auth Center 管理的 OAuth（当前 GitHub Copilot / Codex OAuth / xAI OAuth）。
  - `agent_owned`：Claude Code、Grok Build、QoderWork/TRAE Work/WorkBuddy 等由 Agent/应用自己保存凭据；FyAgent 只发起官方登录流程并观察有证据的状态。
  - `provider_owned`：OpenCode 这类按 Provider 连接凭据的产品。
  - `unavailable/unknown`：没有稳定状态接口时必须保持未知，不能通过文件存在、目录存在或文案推断已登录。
- Auth Center 继续复用当前统一命令和 UI；只补本任务需要的 provider-neutral DTO、取消/互斥、状态刷新或 Adapter，不新建第二套 Auth Center。

### P1 — Codex 多 ChatGPT 账号与 Provider 绑定

- 复用现有 `CodexOAuthManager` 的账号集合、refresh lock/cache、refresh-token persistence 和 Auth Center 命令；不得新增平行 token store。
- 修复现有账号主键把 `chatgpt_account_id`（workspace/account routing identity）当 credential identity 的问题：
  - credential identity 与 ChatGPT workspace/account identity 必须是两个字段/概念；
  - canonical credential ID 必须来自经验证的稳定用户凭据标识；若当前 OpenAI token/API 没有可证明稳定的用户 claim，则使用 FyAgent 自己持久化的随机 credential UUID，并把 workspace ID 仅作为上游路由元数据；
  - 不得再次以 workspace/account ID 作为 map key。
- `ProviderMeta.authBinding` 继续作为唯一 Provider→managed-account 绑定；Provider 行只存 credential ID，不复制 access/refresh token。
- “凭据来源”和“请求发往哪个 upstream”必须独立建模。Official/custom endpoint、proxy takeover、managed-account binding 不得再由一个 `is_official` 布尔值同时决定。
- CC Switch v3.20.0 的以下行为可选择性移植，并保留来源/许可证归因：
  - 任意数量 ChatGPT managed accounts；
  - add/default/remove/logout；
  - Provider 绑定指定账号或 unbound/follow-native；
  - login/logout/remove/provider-switch 互斥；
  - 设备登录可取消且取消真正终止后台请求；
  - bounded OAuth network timeout；
  - 绑定账号与实际请求账号不匹配时 fail closed，不静默转到其他账号；
  - 写 live 前回采同一凭据被 Codex 自身轮换后的 refresh token。
- 不原样复制 CC Switch v3.20.0 “直接把完整 token package 写到 `~/.codex/auth.json`”的实现：
  - 先解析 Codex 当前有效 `cli_auth_credentials_store` 与安装版本对应的官方存储语义；
  - `file` 模式才允许在既有 Codex live-write/backup/lock 边界内投影 `auth.json`；
  - 官方公开的 `keyring` / `auto`，以及源码/未来版本可能出现的任何其他非 `file` 模式，都不实现私有 keyring 协议，不复制 OpenAI monorepo 内部 storage 源码形成维护分叉；若没有稳定官方写入 API，则 managed-account 仍可用于 FyAgent proxy/takeover，但“把该账号设为 Codex 原生登录”必须显示为不可用并引导用户用 Codex 自己登录；
  - 不得仅通过 `auth.json` 是否存在来推断 effective store。
- 迁移旧 managed account store 时必须备份、可重复执行、不会把同 workspace 的不同用户再次合并；历史上已经被覆盖而丢失的凭据不得声称可恢复。
- 保留现有 stale third-party Codex auth 清理、official-auth preservation、Quick Setup targeted write 和 proxy takeover 语义；只在本任务明确所有权内修改。

## Non-goals

- 不同步 Pi。
- 不做 CC Switch v3.20.0 全量 merge；本任务只做来源清晰的选择性 backport/重写适配。
- 不引入第三方软件下载镜像、通用镜像站或 FyAgent 自托管 Agent 安装包。
- 不向 renderer/helper 暴露任意 URL、path、shell command、executable、hash、publisher、scope 或 bypass 参数。
- 不接管 Claude/Grok/OpenCode/Qoder/TRAE/WorkBuddy 的原生 token 文件或系统 Keychain。
- 不将 Qoder CN CLI 当作 QoderWork CN 桌面应用安装包。
- 不把 TRAE IDE/Code 当作 TRAE Work CN。
- 不把 OpenCode 的 Provider `/connect` 伪装成单一账号登录。
- 不以静态测试结果宣称 Windows/macOS native HIL 已通过。
- 不移除、改名或降级现有 Settings/Tooling 中 Gemini CLI、OpenClaw、Hermes 等非 Agent-Catalog 工具的安装/更新能力；本任务新增的 Catalog façade 是增量消费者，不取代 Tooling 自己的既有产品入口。

## Acceptance Criteria

### P0

- [ ] Agent Catalog 仍只有既有 canonical 7，Pi 在 catalog/runtime/install/auth/UI/测试中均未新增。
- [ ] `agent_install` 从只读 readiness 演进为闭集 action contract；renderer 只能提交 canonical `agentId + action`，不能提交 URL/path/command。
- [ ] Managed-package readiness 只向 renderer 暴露 backend-generated release/source state，renderer 不能自行构造下载 locator。TRAE/WorkBuddy 这类有机器版本元数据的 resolver 使用 opaque `releaseId`/revision，并在 install/update 前强制刷新、拒绝版本漂移。QoderWork 的固定 `/latest/` alias 没有可靠远端 semver/source revision 时不得伪造 exact-version coherence：动作语义明确为“安装/更新当前最新版”，开始前重新验证固定 alias，UI 不宣称某个未证实的 remote version。
- [ ] Claude/Grok/OpenCode 安装更新复用现有 Tooling discovery/lifecycle；没有第二套 CLI detector/version resolver/update anchor。
- [ ] 现有 Gemini CLI / OpenClaw / Hermes 等 Tooling install/update 行为和测试保持兼容；Catalog 适配不会缩窄或重写它们的生命周期策略。
- [ ] Claude Code Windows 事实更新到官方 native installer/WinGet 现状，但正式 elevated Windows release 仍遵守 Shell-user runtime contract；若普通用户 helper 未达到认证/closed-command 安全条件，则 Windows CLI automation fail closed。
- [ ] Codex Desktop 现有 one-click installer 继续工作；新的桌面 Agent package flow 复用同一个 managed-package job/platform core。
- [ ] 每个桌面 source adapter 明确 `platform + architecture + packageFormat`，没有正向证据时不猜测 ARM64/x64 兼容性；不同 package format 只通过同一 managed-package core 下的 closed adapter 扩展。
- [ ] QoderWork 使用官网源码当前直接引用且实际可达的三条 versionless `/releases/latest/` User/macOS 地址；不从文档、ETag、Last-Modified 或第三方索引猜远端 semver，installed 状态只展示本机权威版本并提供“更新到最新版”。
- [ ] TRAE Work resolver 从官方 latest API 的 `data.solo`/CN 行动态得到当前包；当前 fixture 能解析出 `2.3.76922` 与三条已验证用户提供 URL，且 `data.manifest`/TRAE Code 永远不能成为 Work 包。
- [ ] WorkBuddy resolver 使用官网 `/v2/update` 的三个 closed platform ID；当前 fixture 能解析 `5.3.14.36279234`，Windows URL与 macOS `.zip -> .dmg` 官方转换均与已验证用户提供 URL一致。
- [ ] 三个桌面 source 的网络/schema/allowlist/target probe 失败都回退官方产品下载页，不硬编码本次调研的 `2.3.76922` / `5.3.14.36279234` 作为离线或错误恢复版本。
- [ ] 每个 enabled source adapter 的初始主机和允许 redirect 主机都是 Rust 侧产品常量/descriptor；跨出 allowlist、scheme 降级、redirect 超限或 metadata/body 超限均 fail closed。
- [ ] 安装任务支持 single-flight、进度、取消边界、超时/错误清理与 post-install authoritative reread；不会由 download success 直接宣称 installed。
- [ ] Claude `auth status` 使用官方稳定 JSON/exit-code；Claude/Grok/OpenCode/Qoder/TRAE/WorkBuddy 的凭据不被 FyAgent 读取、复制或写入。
- [ ] OpenCode 暴露 Provider connection 入口而不是全局 logged-in 状态；没有稳定结构化状态证据时状态为 unknown。
- [ ] 现有 Auth Center 继续作为 managed OAuth 唯一 UI/backend；新增动作复用统一 commands/DTO，不新建平行中心。
- [ ] `external-agent-p0.md`、`codex-desktop-installer.md`、`windows-runtime-security.md` 中被本任务改变的契约同步更新。

### P1

- [ ] 两个属于同一 ChatGPT Team/Business workspace 的不同用户可同时保留，不因共享 workspace/account ID 覆盖。
- [ ] Credential ID 与 ChatGPT workspace/account routing ID 在存储、DTO、绑定和测试中分离；workspace ID 不能作为 credential map key。
- [ ] Provider 继续通过 `ProviderMeta.authBinding` 绑定 managed credential；Provider DB 不保存 token package。
- [ ] 默认账号、显式绑定账号和 unbound/follow-native 三种语义都有确定测试；绑定账号失效时不 fallback 到其他账号。
- [ ] login/logout/remove/switch 的竞态被串行化；取消登录真正终止对应设备流；OAuth 请求有 bounded timeout。
- [ ] refresh token 的唯一持久真相源仍是 managed auth store；回采 Codex 自身轮换 token 时必须证明属于同一 credential 后再更新。
- [ ] Codex effective auth store 被显式解析；官方公开 `file/keyring/auto` 有明确语义，源码/未来版本出现的其他非 `file` 或未知模式一律按非文件权威 fail closed，除非另有稳定官方写入契约。
- [ ] 非 `file` store 不通过 FyAgent 私自写 `auth.json` 假装切换 Codex 原生账号；UI 能明确区分“FyAgent managed account 可用于 routing”和“Codex native projection 不可用”。
- [ ] v1→v2 managed account store 迁移有备份、幂等、损坏/冲突 fixture，并且不声称恢复已被旧 bug 覆盖的数据。
- [ ] 现有 Codex stale-auth cleanup、official-auth preservation、Quick Setup targeted writes、proxy takeover 与 Provider Change Plan 回归测试全部保持通过。
- [ ] OAuth token/refresh token/API key 不进入 DTO、日志、错误、Trellis ledger、DOM、URL 或普通剪贴板。

### Delivery / Evidence

- [ ] Rust/TS unit + contract tests覆盖 closed enums、source resolver、job state、auth ownership、multi-account migration/binding/concurrency/store modes。
- [ ] macOS 当前宿主只对实际执行过的安装/登录路径给出 native 结论；Windows x64/ARM64、UAC Bob/Alice、PackageManager/普通用户 helper 等没有 HIL 的部分明确列为 residual risk。
- [ ] 被选择性移植的 CC Switch/OpenAI/其他上游代码记录原始 URL/issue/commit 与许可证，不把第三方代码改写为 FyAgent 自有来源。

## Notes

- 本任务是复杂跨层任务，实施前以 `design.md`、`implement.md`、`research/` 为准；当前状态保持 `planning`，等待批准后再 `task.py start`。
- 研究与评审记录见 `research/`。至少三轮正式评审完成后才交付本规划。
