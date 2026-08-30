# Stage 4 — Agent Auth 验证状态机

## Goal

把当前 Auth action 的“启动终端/打开应用后立即 `Succeeded`”改为一个独立、可观察、可停止等待、可权威回读的认证会话系统，并保持不同 Agent 的认证所有权真实可见：

- Claude Code：Agent account，可通过官方 `auth status` 验证；
- OpenCode：Provider-owned connections，不是全局登录布尔值；
- Grok Build：Agent login/logout，但当前官方参考未提供等价结构化 status；
- QoderWork/TRAE Work/WorkBuddy：桌面应用登录入口，当前无 reviewed external status API；
- Codex：继续委托 FyAgent Auth Center，不在 Agent lifecycle 中复制 OAuth。

## Requirements

### 1. Separate Auth from install jobs

- 新增独立 `AuthSession` domain、stage enum、session store 和 command façade。
- 不把 `awaiting_user`、Provider connection 或 handoff-only 语义硬塞进安装专用 `AgentActionJobStage`。
- Auth session 至少表达：
  - `preparing`；
  - `launching`；
  - `awaiting_user`；
  - `verifying`；
  - terminal `verified`；
  - terminal `handoff_complete`；
  - `failed | cancelled | timed_out`。
- `verified` 只用于 backend observer 已证明目标状态；`handoff_complete` 只表示官方入口/命令已成功交给用户，不表示已登录。
- Session snapshot 包含 closed intent、stage、cancellable/stop-waiting、verification authority、bounded reason 和最新 auth observation；不包含 token、URL、device code、raw stdout/stderr 或 vendor file path。

### 2. Auth observation discriminated union

Replace the overloaded global bool with a strict union:

- `account`: `logged_in | logged_out | unknown`；
- `provider_connections`: bounded provider summaries + `configured | empty | unknown`；
- `handoff_only`: no authoritative observer；
- `fyagent_managed`: link/delegate to Auth Center；
- `unavailable`。

Observation must separately report:

- ownership (`fyagent_managed | agent_owned | provider_owned | unavailable`)；
- authority (`verified | unverified | unavailable`)；
- checked time and closed reason codes。

File existence, profile directory, config residue or token file presence cannot become `logged_in`.

### 3. Closed auth adapters

建立一个 crate-scoped `AgentAuthAdapter`/dispatcher。Adapter capabilities are explicit:

```text
observe
start_login
start_logout
start_connect_provider
verify_after_handoff
```

Unsupported operation is absent from `allowedActions`; it is not attempted and then mapped to a generic failure.

#### Claude Code

- Login: official `claude auth login` through the existing terminal/interactive-user owner.
- Logout: official `claude auth logout` through closed command execution.
- Observation: official `claude auth status` JSON + exit status, bounded timeout.
- Parse only allowlisted status fields; reject secret-bearing/unexpected output and return unknown.
- A login session may poll status until verified, user stops waiting or deadline expires.

#### OpenCode

- Ownership remains `provider_owned`.
- Connect: official `opencode auth login` or reviewed TUI `/connect` entry; prefer the CLI auth surface because it is explicit and provider-aware.
- Observation: official `opencode auth list`, parsed into bounded provider identifiers/names without reading `auth.json`.
- Logout is provider-scoped/interactive according to the installed CLI's documented capabilities; do not expose global logout if the CLI requires provider selection.
- Verification compares the bounded provider set before/after; presence of one provider cannot imply all providers are authenticated.

#### Grok Build

- Login/logout use official `grok login` / `grok logout`.
- Since the reviewed official CLI reference does not expose a structured auth-status command, login completion is `handoff_complete`, not verified success.
- Do not read Grok's cached credential file. A future official status surface requires a separate adapter capability review.

#### Desktop Agents

- QoderWork/TRAE Work/WorkBuddy Auth action launches the exact Stage 1 selected trusted application or product login entry when officially supported.
- Without a reviewed auth-status API, terminal state is `handoff_complete` and observation remains `handoff_only/unknown`.
- Button copy must be “打开登录入口” or “打开应用完成登录”，not “登录成功”.

#### Codex

- Agent UI delegates to existing Auth Center/Codex OAuth owner.
- Do not create a second Codex login observer/session, token store or OAuth callback.

### 4. Session concurrency and lifecycle

- Per Agent/intent single-flight; duplicate start returns current session or a closed conflict reason.
- CLI observation and launch bind to the interactive shell user on Windows and the current application user on macOS.
- Polling/deadlines are bounded and cancellable at the session-monitor level.
- If external terminal/browser/app cannot be safely terminated, UI action is “停止等待”，not “取消登录”；the handoff may continue outside FyAgent.
- Renderer reload can recover a still-active bounded session snapshot where practical; no secret or raw command is persisted.
- Terminal snapshots are immutable after terminal state.

### 5. Frontend shared architecture

- `shared/features` owns Auth DTOs/parsers/FeaturePort/query keys and capability projection.
- Add a shared `AuthStatusPanel` or extend the authoritative action status surface so Agent directory and Agent detail can share:
  - current observation；
  - ownership explanation；
  - login/logout/connect actions；
  - awaiting-user/verify/stop-waiting/retry；
  - handoff-only copy。
- State machine remains Auth-domain-owned; do not create one generic hook that conflates install, Auth and arbitrary async mutations solely because they show a spinner.
- Any provider list is bounded, keyboard accessible and never rendered with credential values.

### 6. Same-domain defect policy

During testing, fix same-domain defects in command selection, terminal launch, observation parsing, polling, session recovery, ownership copy, Provider scoping and secret redaction. Model configuration, installation and unrelated Auth Center behavior stay outside unless a regression is directly caused by this integration.

## Non-goals

- 不读取或迁移任何 vendor token/credential file。
- 不替代 Auth Center，不统一不同服务的 OAuth tokens。
- 不通过真实模型请求、额度请求或计费 API 来“测试登录”。
- 不向 renderer 返回账号邮箱、组织 ID、token expiry 或 Provider secret。
- 不承诺 Grok/桌面 Agent 的 verified status，除非新增官方稳定 observer。
- 不自动填写网页登录、扫码或设备码。

## Acceptance Criteria

- [x] Auth 有独立 contract version、stage/outcome/observation union 和 strict Rust/TS parser。
- [x] 启动 terminal/browser/app 不再返回 `verified/succeeded` 登录状态。
- [x] Claude `auth status` exit 0/1 + JSON 分别映射 verified logged-in/logged-out；malformed/secret-bearing/timeout 输出为 unknown。
- [x] Claude login 只有在后续 status 改为 logged-in 时进入 verified；未变化会 awaiting-user/timeout，不伪造成功。
- [x] Claude logout 后必须 status 回读 logged-out 才 verified。
- [x] OpenCode UI 显示 Provider connections；一个 Provider 的存在不生成 global logged-in。
- [x] OpenCode connect/logout 通过 `auth list` 的 bounded before/after set 验证；不读取 `auth.json`。
- [x] Grok login/logout 仅报告 official command handoff/command completion；没有 status observer 时 auth observation 保持 handoff-only/unverified。
- [x] Desktop Agents 只显示打开登录入口/重新检查，不显示登录成功。
- [x] Codex 只委托 Auth Center，无第二套 OAuth command 或 storage。
- [x] Per-agent single-flight、deadline、stop-waiting、按 session ID 恢复查询和 terminal immutability 有测试。
- [x] Windows elevated host 的 Tooling/Auth 路径在 frozen Explorer user 不可用时 fail closed，且不复用 installer helper 执行 CLI。
- [x] 所有 DTO、errors、logs、DOM snapshots 和 tests 均不包含 token、Authorization、device code、raw auth file path 或完整 raw output。
- [x] Agent directory/detail 使用同一个 shared Auth status surface，无复制状态机。
- [x] Component/Playwright tests 覆盖 awaiting-user、verified、handoff-only、Provider scoped logout、Auth Center delegation 和 unsupported action absence。
- [ ] 完整 renderer reload 自动恢复：当前后端会话可按已返回的 session ID 查询，但 renderer 丢失该 ID 后不会凭空猜测或持久化路径/命令。
- [ ] Native macOS/Windows HIL 分别验证 Claude、OpenCode、Grok 和至少一个桌面 handoff；当前只证明 portable/contract/browser 行为，未支持 observer 的产品明确为 handoff-only。

## Dependency

Stage 1 is required for desktop-Agent launch target authority. CLI Auth work may be developed independently but must integrate with the final Stage 1 directory model.
