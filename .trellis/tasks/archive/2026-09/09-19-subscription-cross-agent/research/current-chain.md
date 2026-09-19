# Research: FyAgent 订阅账号跨工具链

- Query: 核对 origin/main=64f4d8f6 上 OpenAI OAuth / xAI OAuth 登录、Managed Auth、代理转换、Codex / Grok Build / Claude Code / OpenCode 入口是否闭环；确认 OpenCode 缺口并给出最小实现范围。
- Scope: mixed
- Date: 2026-09-19

## Findings

### 当前结论

当前代码有两条不同的能力：

1. `bind_managed_proxy_provider` 是统一“订阅账号 → 本地代理 Provider”绑定，只支持 `claude`、`codex`、`grokbuild`。它把 OpenAI 或 xAI 的 `ProxyUpstream` credential 解析成 `codex_oauth` / `xai_oauth` provider，并在 Claude/Grok Build 直接 quick-setup；Codex 保存 provider 后继续走 Change Plan。OpenCode 不在这条代理绑定链上。
2. OpenCode 已有独立的 Managed Auth consumer，但它要求同一个 identity 下存在 `CredentialPurpose::OpencodeProvider` 的独立 credential；它通过 `opencode-data/auth.json` 做 native auth 投影，并以 CAS、备份、读回、pending-restart 状态完成连接事务。已经存在的 `ProxyUpstream`、`CodexNative`、`GrokNative`、`Copilot` lineage 会被明确拒绝，不能直接复用。

因此，OpenAI OAuth / xAI OAuth 登录后，Codex、Claude Code、Grok Build 的“共用代理订阅”路径成立；OpenCode 只能“为 OpenCode 单独登录同一 provider 后再投影”，不能从已经登录的代理订阅账号一键绑定。用户若把“登录一次、四个工具使用”理解为统一账号复用，则 OpenCode 是真实缺口。

### 1. OpenAI OAuth → Managed Auth → proxy → Claude/Codex/Grok

- `src-tauri/src/services/managed_auth/login.rs:735-773` 按 consumer 分配 credential purpose：Codex 使用 `CodexNative`，OpenCode 使用 `OpencodeProvider`，其余默认进入 `ProxyUpstream/FyagentProxy`。这说明登录凭据按消费者隔离，不会自动把 proxy credential 复制给 OpenCode。
- `src-tauri/src/services/provider/managed_proxy.rs:63-85` 的目标解析只接受 `claude`、`codex`、`grokbuild`、`claude-desktop`，未知目标（包括 `opencode`）返回 `InvalidRequest`。
- `src-tauri/src/services/provider/managed_proxy.rs:100-127` 将 OpenAI source 绑定为 `codex_oauth`、ChatGPT Codex base URL；xAI source 绑定为 `xai_oauth`、xAI subscription base URL。Grok Build 的配置固定 `api_key = "PROXY_MANAGED"`、`api_backend = "responses"`，实际 token 不写入工具配置。
- `src-tauri/src/services/provider/managed_proxy.rs:317-335` 只从 `auth.proxy_account()` 读取 `ProxyUpstream` 且限定 OpenAI/xAI；随后构造 provider、做冲突检查。
- `src-tauri/src/services/provider/managed_proxy.rs:349-360` Claude/Grok Build 立即 quick-setup；Codex 只保存 provider，仍需 Change Plan/连接入口确认。
- `src-tauri/src/proxy/providers/managed_responses.rs:1-35` 是代理最终 wire policy：OpenAI OAuth request 强制 `store=false` 并补 encrypted reasoning；xAI/Grok request 清理不兼容 reasoning、hoist instructions、处理 Responses schema。对应单测在同文件 `:118-180` 左右，覆盖 OpenAI 幂等和 Grok 工具/指令变换。
- `src-tauri/src/services/managed_auth/subscription_tests.rs:580-710` 的跨层测试保存并应用 Claude、Codex、Grok Build 三个 provider，验证 Codex live config 指向本地端口、Grok config 指向 `/grokbuild/v1`、原生 auth 文件保持不变，并观测 proxy route。没有 OpenCode 目标测试。
- `tests/browser/xai-subscription.spec.ts:1-120` 覆盖同一 xAI 账号用于 Claude + Codex；`:120-180` 覆盖 ChatGPT 账号绑定 Grok Build。没有 OpenCode subscription section 或 bind 调用。

### 2. OpenCode 当前已有的 native consumer 闭环

- `src-tauri/src/services/managed_auth/consumers/opencode.rs:1-5` 明确按 OpenCode 官方 `Global.Path.data/auth.json` schema 实现；`:27-28` 明确外部写入 hot reload 未被证明。
- `src-tauri/src/services/managed_auth/consumers/opencode.rs:161-203` 为 OpenAI、xAI、GitHub Copilot 建立固定 slot，并对 auth.json 做 revision/CAS upsert；`:242-365` 读取 auth.json，给出 connected/disconnected/pending-restart 和 allowed actions。
- `src-tauri/src/services/managed_auth/service.rs:1467-1551` 的 `opencode_connect` 只查找同一账号的 `OpencodeProvider + Ready` credential；没有时若发现 `ProxyUpstream/CodexNative/GrokNative/Copilot` lineage，返回 `ProviderNotSupported`。成功后把 bundle 投影为 OAuth/API entry，并把 refresh owner 转成 `Opencode`。
- `src-tauri/src/services/managed_auth/service.rs:1978-1983` 在 overview 中合并 OpenCode 三个 provider slot；`src-tauri/src/services/managed_auth/service.rs:2087-2105` 将 refresh/access bundle 转为 OpenCode OAuth（expires 从 Managed Auth 秒转换为毫秒）或 API key entry。
- `src-tauri/src/services/managed_auth/login.rs:183-185` 将 OpenCode connection action 路由到 `apply_opencode_connection_action`；`:747-763` 的 login purpose 明确支持 OpenCode 独立认证。
- `src/pages/agents/AgentAuthStatusPanel.tsx:41-47` 将 OpenCode 映射到 managed consumer；`:69-74` 观察 Provider 连接；`:90-92` 的产品文案明确“OpenCode 分别连接各个 Provider，没有统一登录状态”。这与当前隔离 lineage 设计一致。
- 关键回归测试：`src-tauri/src/services/managed_auth/service.rs:2800-2835` 验证 auth.json 观察不泄露 token；`:2837-2964` 验证 proxy lineage 被拒绝、独立 OpenCode credential 可经 preview/apply 投影、CAS 单次使用、备份/pending restart、refresh owner 转移；`src-tauri/src/services/managed_auth/consumers/opencode.rs:720-845` 覆盖默认路径、未知 key 保留、0600 权限、CAS stale 拒绝。

### 3. OpenCode 官方协议核验

- 官方 OpenCode 源码 `packages/opencode/src/auth/index.ts`（当前 `dev`，2026-09-19 抓取）定义 `Global.Path.data/auth.json`，OAuth entry 必须是 `{type:"oauth", refresh, access, expires, accountId?}`，API entry 是 `{type:"api", key, metadata?}`，并用 `Auth.all()` 读取后按 provider id 索引；写入权限为 `0600`。来源：[OpenCode auth/index.ts](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/auth/index.ts)。本仓库的 `consumers/opencode.rs` 与该 schema 对齐。
- 官方文档 Providers 说明 `/connect` 的 API key 保存在 `~/.local/share/opencode/auth.json`，provider/model 仍需在 `opencode.json` 中配置；来源：[OpenCode Providers](https://opencode.ai/docs/providers)。因此 auth.json 投影本身只提供 credential，不等于 FyAgent 已配置 OpenCode provider/model。
- 官方源码还允许通过 `OPENCODE_AUTH_CONTENT` 覆盖 auth 内容（源码 `:475-489`），但当前 FyAgent 未使用该路径；这不是缺口修复的必要条件。
- 官方 issue 记录 OpenAI ChatGPT OAuth 入口曾在 OpenCode 版本间回归（例如 #28636），说明直接把 OpenAI OAuth token 投影到 OpenCode 依赖版本/官方 OAuth 菜单行为；FyAgent 当前把 OpenCode OAuth 作为独立 provider credential 并保留 pending-restart，不能宣称已完成真实 OpenCode CLI/Desktop E2E。

### 4a. 官方 OpenCode provider/source 对 loopback 设计的依据

- 官方 `packages/llm/src/providers/openai.ts` 当前 `dev` 源码将 OpenAI facade 的 routes 声明为 Responses、Responses WebSocket 和 Chat；`configure()` 通过 `configuredRoute()` 把 `baseURL`/query 参数写入 endpoint，并返回 `responses(id)`、`chat(id)` 等按 model id 构造的模型。其 `responses(id)` 使用 `withOpenAIOptions(...).model({id})`，因此专属 loopback provider 应优先声明 Responses 协议、显式 baseURL 和用户选定 model id，而不是依赖 OpenCode auth.json 的 OAuth provider 选择。
- 官方 `packages/opencode/src/provider/provider.ts` 对 `openai` 和 `xai` provider 的 `getModel()` 均调用 `sdk.responses(modelID)`；这确认 OpenCode 的默认模型路线是 Responses。OpenCode provider 配置也允许 provider options 的 `baseURL`，但 model 目录/模型 id 仍需在 `opencode.json` 的 provider/model 结构中解析。
- 官方 providers 文档明确：若端点提供 `/v1/responses`，应使用 `@ai-sdk/openai`；`@ai-sdk/openai-compatible` 用于 `/v1/chat/completions`。因此 FyAgent 的 loopback provider 应使用 OpenAI Responses-compatible 入口，必要时在 OpenCode 配置中显式 `npm: "@ai-sdk/openai"`、`options.baseURL` 和 provider 下的 model id；不能把 Chat-only adapter 当作 Responses route。
- 以上官方 source 只证明 OpenCode 的配置/请求形状，不证明 FyAgent loopback route 已实现，也不证明 OpenAI/xAI 订阅 entitlement 可由 OpenCode 官方直接消费。

### 4. xAI/Grok 路径判断

- xAI OAuth provider 在 `src-tauri/src/services/managed_auth/providers/xai.rs`，登录 purpose 对 Grok Build 使用 `GrokNative`，对 OpenCode 使用 `OpencodeProvider`，其余使用 `ProxyUpstream`。统一 proxy bind 则从 `ProxyUpstream` 解析 xAI identity。
- Grok Build 的 proxy target 已进入 backend parser（`managed_proxy.rs:63-68`）和 frontend port allowlist（`src/shared/platform/tauri/feature-ports/models.ts:530-535`）；Grok provider config 强制 Responses backend，变换逻辑和 fixture tests 已存在。
- 这条路径证明“OpenAI ChatGPT 订阅能被 Grok Build 使用”是设计支持的代理路由；它不证明 Grok 原生 auth.json 投影，因为 Grok native projection 仍由其独立 consumer/evidence gate 管理。

### 最小实现建议（已按主控设计更新）

此前“允许 `ProxyUpstream` 派生/转投 `OpencodeProvider`、再写入 OpenCode `auth.json`”的建议已 **superseded**。原因是它会把 Proxy owner 的 refresh lineage 交给 OpenCode native consumer，破坏当前按消费者隔离和单一 refresh owner 的安全边界；现有 `opencode_connect` 对该 lineage 的 fail-closed 拒绝应保留。

当前选定的最小范围是：新增专属 OpenCode bind request（带 `expectedRevision`），复用现有订阅账号选择、Proxy 解析和事务/回读框架，但写入由 OpenCode owner 管理的 loopback provider 配置；OpenCode `auth.json` 保持不变。新增独立 OpenCode route，由 FyAgent Local Proxy 作为唯一 token owner 解出 OpenAI/xAI credential，再把 OpenCode 的 Responses 请求转发至既有 `codex_oauth`/`xai_oauth` upstream。不要把 OpenCode 加入通用 quick-setup，也不要开放 auth.json token 投影。

最小实现边界：

1. 新增 OpenCode-specific binding DTO/command/renderer port，校验 `accountId`、`modelId`、`expectedRevision`，只允许 OpenAI/xAI 的 ready `ProxyUpstream` lineage；写入 OpenCode `opencode.json` 的专属 loopback provider/model 结构，并保存备份、CAS、readback 与恢复信息。
2. 新增 OpenCode loopback route/route observation，使用官方 OpenCode 的 Responses 形状（`@ai-sdk/openai` / `sdk.responses(modelID)`）作为入口；Proxy 解 token、刷新和上游请求，OpenCode 端只看到 loopback baseURL 与 model id，不接触 OAuth/refresh token。
3. 保留现有 `consumers/opencode.rs` native auth consumer 和 `auth.json` 路径，继续支持用户单独登录 OpenCode provider；新 loopback binding 不得转移 `RefreshOwner`，也不应改变 native connection 状态。
4. 测试覆盖：OpenCode bind 写入配置的 CAS/backup/readback/rollback；同一账号的 OpenAI 与 xAI route 选择；token 不出现在 OpenCode 文件；Responses body/header/stream 转换、401 refresh 与 route observation；stale revision、非 ready、错误 provider、重复绑定和并发 mutation 失败。真实 CLI smoke 另列为后续 E2E，不由 fixture 测试替代。

若产品只要求“OpenCode 能单独登录并使用 provider”，当前实现已足够：在 Auth Center 选择 OpenCode connection，单独完成 OpenAI/xAI OAuth，再由现有 `opencode_connect` 投影 auth.json；这不等价于统一 subscription bind。

## Related specs

- `.trellis/spec/backend/managed-auth.md`
- `.trellis/spec/backend/managed-auth-consumers.md`
- `.trellis/spec/backend/managed-account-proxy.md`
- `.trellis/spec/backend/local-proxy-pipeline.md`
- `.trellis/spec/backend/external-agent-auth.md`
- `.trellis/spec/backend/external-agent-configuration.md`
- `.trellis/spec/frontend/agent-models.md`

## Caveats / Not Found

- 只研究 origin/main=64f4d8f6 的隔离工作树；未改代码、未做 git 操作、未读取或输出真实凭据。
- `rtk cargo test --manifest-path src-tauri/Cargo.toml managed_auth::service::tests::opencode -- --nocapture` 曾因构建目录被其他并行进程持有锁而等待；已发送 Ctrl-C 停止我自己启动的会话（exit 130），未触碰主控测试进程，未取得最终 pass/fail；静态测试证据与测试名称已列于上文。
- 当前测试大量使用 memory backend、fixture auth.json 和 synthetic upstream；它们证明代码事务/转换行为，不证明真实 OpenAI/xAI entitlement、已安装 OpenCode CLI/Desktop 的真实调用、热加载或生产服务接受。
- OpenCode 官方文档页面抓取超时，使用官方 GitHub source 直接核对 auth schema，另保留官方 docs 链接作为配置入口参考。
- 未执行第三方真实账号登录；不作“订阅额度实际可用”或“OpenCode 版本兼容所有 OAuth token”结论。
