# 当前 FyAgent 集成接缝

> 用途：让实施者先找到现有 owner，再决定迁移位置。本文只描述当前基线与目标接缝，不替代源码；修改前仍需读取列出的文件及相邻测试。

## 1. 前端 owner

### 1.1 V2 Agent Auth

| Owner | 当前职责 | 本任务接缝 |
| --- | --- | --- |
| `src/v2/shared/features/agent-auth.ts` | 严格 observation/session DTO、closed enums、FeaturePort | 演进为中央账号页与 Agent 摘要共用的 managed-auth wire；不要另建宽松 DTO |
| `src/v2/shared/platform/tauri/feature-ports/agentAuth.ts` | 唯一 V2 Tauri adapter | 新 `ManagedAuthPort` 沿用相同 strict adapter 模式；页面不直接 invoke |
| `src/v2/shared/features/queries.ts` | Query key 与 observation owner | 账号/connection/session query key 在此或聚焦 sibling owner，避免页面私有 cache |
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | Agent 详情的完整 Auth 动作面 | 收敛成安全摘要、状态与 `/auth` deep link；Claude/国产 handoff 保持 |
| `src/v2/pages/agents/useAgentAuthSession.ts` | backend-owned Auth session 恢复/轮询 | 复用其 lifecycle 原则；若抽共用 hook，必须先有中央账号页第二个真实 consumer |
| `src/v2/shared/config/navigation.ts`、`src/v2/app/primaryPages.tsx` | 关闭的 route/nav 映射、lazy/prefetch | 新增 `/auth`，保持 closed page type 与 chunk/prefetch 测试 |
| `src/v2/shared/ui/**` | Tabs、master-detail、dialog、button、notice、spinner、persistent surface | 优先组合，禁止把 V1 shadcn 组件带入 V2 |

### 1.2 旧账号中心

| Owner | 当前能力 | 迁移决策 |
| --- | --- | --- |
| `src/components/settings/AuthCenterPanel.tsx` | Copilot/Codex/xAI tabs | 作为 parity 清单和兼容入口，不继续成为产品 authority |
| `src/components/providers/forms/CodexOAuthSection.tsx`、`XaiOAuthSection.tsx`、`CopilotAuthSection.tsx` | 多账号列表、默认账号、Device Code、额度 | 复用交互经验，不直接 import 到 V2 |
| `src/components/providers/forms/hooks/useManagedAuth.ts` | renderer 轮询、mutation、错误字符串解析 | 轮询/超时/取消迁移到 backend session；旧 hook 最终删除或变 compatibility adapter |
| `src/lib/api/auth.ts` | 宽松旧 DTO 与 Tauri commands | 保留过渡 ABI，V2 不能依赖；最终由严格 versioned contract 替代 |

必须避免的错误迁移：让 `/auth` 包装旧 `AuthCenterPanel`。这会立刻违反 V2 import boundary，并保留 page-owned polling/raw error。

## 2. Backend Auth owner

### 2.1 通用命令面

`src-tauri/src/commands/auth.rs` 已经将 Copilot、Codex OAuth、xAI OAuth 暴露为统一命令，但它仍：

- 直接持有三个具体 manager state；
- 复用 GitHub 数据结构承载 OpenAI/xAI 账号；
- 主要以 Device Code DTO 为中心；
- renderer 可见状态缺少 identity/session/consumer 层次；
- raw `String` error 仍可能越过边界。

目标不是再加一组 `managed_auth_*` 命令后永久保留旧组，而是：

1. 建立 `ManagedAuthService`；
2. 新严格命令只依赖该 service；
3. 旧命令在过渡期调用该 service compatibility methods；
4. 所有 consumer 迁完后删除具体 manager 注入。

### 2.2 OpenAI/Codex manager

`src-tauri/src/proxy/providers/codex_oauth_auth.rs` 已拥有：

- OpenAI 特殊 Device Code；
- authorization code + verifier 换 token；
- 多账号与 opaque credential ID；
- access-token 内存 cache；
- 每账号 refresh lock；
- refresh-token rotation；
- Provider `authBinding` 迁移。

需要迁移而非复制：

- 协议核心移动到 `services::managed_auth::providers::openai`；
- 账号/默认/refresh metadata 迁到 SQLite；
- token bundle 迁到 SecretRef；
- browser loopback PKCE 与 Device Code 共享同一 account/session persistence；
- Proxy 通过 resolver 访问，不再 import concrete manager。

不得继续使用：明文 `codex_oauth_auth.json` 作为新写入 owner、把 OpenAI 账号塞进 `GitHubAccount`、由不同功能各自启动 refresh loop。

### 2.3 xAI manager

`src-tauri/src/proxy/providers/xai_oauth_auth.rs` 已拥有：

- OIDC discovery；
- HTTPS/x.ai endpoint 校验；
- Device Code、`slow_down`、取消/过期分类；
- access cache、refresh、reauth 状态和 mutation lock。

迁移目标与 OpenAI 相同：协议复用、SecretRef、SQLite、backend session、统一 resolver。Grok consumer 不能再复制一份 xAI Device Code/refresh 实现。

### 2.4 Agent Auth observation/session

| Owner | 当前事实 | 目标 |
| --- | --- | --- |
| `src-tauri/src/agent_install/auth_actions.rs` | Claude CLI observer；OpenCode CLI provider observer；Grok handoff；Codex FyAgent-managed占位 | managed consumer observation从 `ManagedAuthService`读取；Claude/国产 handoff保持各自 owner |
| `src-tauri/src/agent_install/auth_sessions.rs` | backend session、terminal snapshot、poll/stop semantics | 复用状态机原则；中央登录 session可共享底层 primitive，但不能把不同协议硬塞进一个函数 |
| `src-tauri/src/commands/agent_auth.rs` | V2 transport | Agent 摘要继续使用；账号管理 mutation走managed-auth命令，职责不混合 |

## 3. SecretRef owner

`src-tauri/src/services/secret/` 已包含：

- `SecretRef` / `SecretVersion` opaque handle；
- create、replace、read-with-callback、probe、delete；
- `SecretMaterial` zeroization；
- macOS Keychain adapter；
- Windows Credential Manager adapter；
- memory test backend。

本任务扩展点：

- 增加 OAuth token bundle purpose；
- typed bundle codec与sealed callback；
- production construction/state；
- signed macOS entitlement与Windows安装用户上下文证据；
- OS vault + SQLite 跨存储恢复 journal；
- version/generation 一致性。

不要建立另一个 keyring abstraction，也不要为 Linux 新增 fallback；FyAgent 当前产品平台是 macOS/Windows。

## 4. Persistence owner

- `src-tauri/src/database/mod.rs` 的 `SCHEMA_VERSION` 当前基线为 20。
- `src-tauri/src/database/schema.rs` 拥有表创建、逐版本 migration、future-version fail-closed 与测试 seam。
- `src-tauri/src/database/backup.rs` 拥有 export/import/backup 安全边界。

新 managed-auth metadata 必须进入同一 DB owner和版本迁移，不创建旁路 SQLite/JSON index。普通数据库导出不能包含 secret material；跨设备导入只能恢复 metadata，账号应显示“需要重新登录”。

## 5. Provider/Proxy owner

### 5.1 Provider binding

`src-tauri/src/provider.rs` 的 `meta.authBinding` 是现有 Provider 到受管账号的兼容指针。迁移时：

- 先生成 old ID → new Credential Session ID mapping；
- 在一个 DB migration/repair 流程中 remap；
- dangling binding 必须显式 reason，不能回落默认账号；
- 长期 connection metadata 为 authority，Provider binding 是配置投影/兼容字段。

### 5.2 Proxy

`src-tauri/src/proxy/forwarder.rs` 当前直接识别 `codex_oauth` / `xai_oauth` 并访问具体 manager。目标：

```text
forwarder -> ManagedAuthTokenResolver -> purpose=proxy_upstream, refresh_owner=fyagent
```

resolver 返回短生命周期 access material给请求构建边界；不得返回 refresh token、SecretRef 或账号全集。native-owned session若被误绑定，返回 closed owner-conflict reason。

## 6. Codex Provider owner

关键文件：

- `src-tauri/src/codex_config.rs`
- `src-tauri/src/codex_config/**`
- `src-tauri/src/services/provider/live.rs`
- `src-tauri/src/services/provider/mod.rs`
- `src-tauri/src/services/proxy.rs`
- `src/config/codexProviderPresets.ts`
- `.trellis/spec/backend/codex-provider-configuration.md`

当前代码已经有 official provider、第三方 Provider、`experimental_bearer_token`、credential-store 判定和 `preserveCodexOfficialAuthOnSwitch`。实施必须先建立“第三方 writer config-only”的全路径测试，再删除/废弃开关；不能仅修改一个 quick-setup writer。

## 7. Consumer-specific seams

### 7.1 Codex

- inventory/process/launch owner继续复用现有 Codex Desktop modules；managed-auth不创建第二套安装/进程扫描。
- native Auth observation优先使用官方 app-server或受支持store；projection必须经过现有路径和credential-store解析。
- 切号事务要在关闭进程前完成目标凭据准备，在写后readback，失败按exact preimage/revision回滚。

### 7.2 Grok Build

- 安装/版本 owner仍是现有 tooling/Agent lifecycle；Auth不能夹带安装动作。
- 当前 `auth_actions.rs` handoff需要被managed observation替换。
- 第一方 `auth_provider_command` helper若通过HIL，应复用现有受信 helper/打包机制，不创建脚本路径和任意命令配置。
- native registry fallback必须复用/对齐Grok官方lock、scope key和merge语义。

### 7.3 OpenCode Desktop

- installation inventory/launch target继续归 Agent lifecycle owner。
- Desktop Auth不依赖 `opencode` PATH CLI；CLI只是另一个可选surface。
- credential store必须按OpenCode官方data path规则在目标用户上下文解析。
- 不连接Desktop私有随机sidecar；公开Provider Auth无法从外部驱动时，launch handoff与直接store projection是两条不同能力。

## 8. Required architecture tests

实施应扩展/新增：

- `tests/v2/app/architecture.test.ts`：V2不得导入V1、Tauri只在adapter、layer方向。
- `tests/architecture/rustModuleBoundaries.test.ts`：commands/proxy不得依赖具体OAuth manager；managed-auth模块可见性。
- 新 wire contract tests：exact keys/closed enums/forbidden fields。
- 新 repository scan：renderer DTO、logs、DB/export和task fixtures无token-like fields。
- 现有 Provider/Proxy/Codex regression tests：第三方切换不触碰官方auth。

## 9. Stop conditions

遇到以下情况停止启用相应capability，而不是增加猜测性fallback：

- 第一方登录参数或redirect allowlist无法确认；
- OS vault entitlement/user-context未在正式构建验证；
- native store版本或authority不明；
- OpenCode/Grok运行中刷新owner无法证明；
- external writer导致generation无法安全reconcile；
- HIL与源码推断不一致。
