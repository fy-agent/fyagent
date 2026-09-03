# 技术设计：统一 Agent 官方登录与账号管理

## 1. Decision summary

本任务采用以下总体方案：

1. **前端先行。** 先冻结 V2 “账号与认证”信息架构、严格 wire DTO、交互状态机和浏览器测试，再接生产 backend。不得先把现有 manager 暴露给页面后补 UX。
2. **一个账号控制面。** 新增 V2 一级路由 `/auth`；Agent 页面只保留摘要与深链。旧 Settings Auth Center 迁为兼容入口，禁止继续发展第二套状态。
3. **一个 backend owner。** 新建 `services::managed_auth` facade，迁入/包裹现有 Codex、xAI、Copilot manager；Proxy、Agent Auth、Provider Form、V2 页面均通过它访问，不再直接持有具体 manager。
4. **身份与凭据 session 分离。** `ManagedIdentity` 表示人/账号；`CredentialSession` 表示一次 OAuth grant 和一个旋转 refresh-token lineage。UI 可把多个 session 聚合成一个账号，backend 不合并其 refresh token。
5. **唯一 refresh owner。** 每个 Credential Session 只有一个 `refresh_owner`。默认按 consumer 建独立 session，禁止把同一 refresh token 同时交给 FyAgent、Codex、OpenCode 或 Grok 各自刷新。
6. **SecretRef 成为生产凭据 owner。** token bundle 存 OS Keychain/Credential Manager；SQLite 只存 opaque ref、version、身份、状态、绑定和 generation；renderer 永远拿不到 secret ref 或 token。
7. **第一方协议优先。** OpenAI browser loopback PKCE/Device Code、xAI OIDC Device Code、Grok/OpenCode native store 以对应第一方 Apache/MIT 实现为权威。`cockpit-tools` 只作为产品与故障处理参考，不直接复制代码。
8. **官方登录与第三方 Provider 正交。** Codex 的第三方 Provider 切换永远不修改官方 Credential Session；“保留官方登录”从用户选项升级为系统不变量。
9. **成功必须 readback。** token exchange、文件写入、应用重启或 callback 到达都不是最终成功；只有 metadata、SecretRef、consumer projection 与 fresh native observation 一致后才发布成功。
10. **能力按平台证据开放。** `file/keyring/auto/ephemeral`、OpenCode reload、Grok helper 等行为未完成真实 HIL 时保持 closed unsupported 状态，不做猜测 fallback。

## 2. Product information architecture

### 2.1 Primary route

在 V2 导航的“AI 软件配置”组新增：

```text
AI 软件配置
├─ AI 软件配置      /agents
└─ 账号与认证        /auth
```

`/auth` 是管理账号、登录 session、软件连接和重新授权的唯一主要入口。它加入现有 primary-page lazy loader、prefetch 与 persistent surface 体系；页面隐藏时普通查询暂停，backend-owned 登录/切换 session 继续运行。

### 2.2 Page hierarchy

```text
AuthPage
├─ Page header
│  ├─ title + aggregate health
│  └─ 添加账号
├─ FeatureTabs
│  ├─ 账号
│  │  └─ AccountMasterDetail
│  │     ├─ Provider filter / search
│  │     ├─ Account cards
│  │     └─ Account detail + connected consumers
│  └─ 软件连接
│     └─ Consumer cards
│        ├─ Codex
│        ├─ Grok Build
│        ├─ OpenCode
│        └─ FyAgent Local Proxy
└─ Backend session surfaces
   ├─ ManagedAuthLoginDialog
   ├─ ConnectionSwitchDialog
   ├─ RemoveAccountDialog
   └─ RestartRequiredDialog
```

首屏以用户任务为中心，不展示 token store、OAuth 参数、credential ID 或 workspace routing ID。

### 2.3 Agent integration

`AgentAuthStatusPanel` 从“完整执行面”收敛成摘要：

```text
Codex
账号：person@example.com
当前：DeepSeek API
OpenAI 登录：已保留
[管理账号]
```

```text
Grok Build
xAI 账号：person@example.com
状态：需要重新登录
[管理登录]
```

```text
OpenCode
已连接 2 个 Provider
OpenAI · xAI
[管理连接]
```

- 仍保留 agent-owned Claude observer 和三个国产 Desktop handoff；它们不自动进入 managed-auth。
- 从 Agent 进入 `/auth` 时传递闭集 consumer/filter/return descriptor，不把任意 callback URL、token 或路径放进 search params。
- 返回使用现有 `AgentReturnDescriptor` 模式，扩充时保持严格 parser。

## 3. Frontend experience design

### 3.1 Account list

账号卡片字段：

```text
brand
login / display name
health: ready | needs_reauth | checking | unavailable | migration_blocked
provider: OpenAI | xAI | GitHub
isDefault
connectedConsumerCount
plan/quota summary?        // 独立可空，不决定 health
lastAuthenticatedAt
```

状态优先级：

```text
migration_blocked
> secret_unavailable / needs_reauth
> checking
> ready
> unknown
```

额度失败只影响额度行，不把账号状态降为退出登录。

### 3.2 Account detail

详情按三段显示：

1. **账号状态**：登录名、Provider、默认账号、上次认证、重新登录。
2. **已连接软件**：consumer、目标、credential owner、连接状态、当前请求模式。
3. **管理动作**：设为默认、添加独立连接、移除账号。

用户文案不使用 `refresh owner`，对应翻译：

| 内部状态 | 用户文案 |
| --- | --- |
| `fyagent` | “由 FyAgent 自动续期” |
| `codex_native` | “由 Codex 自动续期” |
| `grok_native` | “由 Grok Build 自动续期” |
| `opencode` | “由 OpenCode 自动续期” |

### 3.3 Connections view

每个 consumer 卡片同时显示三条信息：

```text
账号连接：OpenAI · person@example.com
当前模型来源：DeepSeek API
官方登录：已保留
```

`current_model_source` 与 `auth_connection` 来自不同 backend owner，UI 不从一个字段推导另一个字段。

动作由 backend `allowedActions` 决定：

```text
connect
switch_account
reconnect
refresh_status
disconnect
open_app
restart_app
switch_to_official
```

前端不自行构造 capability matrix。

### 3.4 Login dialog

#### OpenAI browser loopback

```text
select provider/purpose
  -> preparing
  -> opening official browser
  -> awaiting callback
  -> exchanging authorization code
  -> saving account
  -> connecting consumer? (optional)
  -> verifying
  -> completed
```

UI只显示：

- 官方域名 `auth.openai.com` / `chatgpt.com`；
- 当前阶段；
- “重新打开官方登录页”“改用设备码”“取消”；
- fallback/错误说明。

授权 URL 由 backend 验证并直接打开；renderer 不持久化、分析或记录 URL query。

#### Device Code

显示：

- 官方 verification host；
- user code；
- 复制与重新打开；
- backend 计算的剩余有效状态；
- 取消。

前端不自行 setInterval 调 token endpoint。它只轮询/订阅 backend session snapshot。

#### Session lifecycle

- dialog 关闭不等于取消；关闭后顶部显示进行中任务 pill，可重新打开。
- “取消登录”调用 backend cancel；late callback/poll result必须因 session generation 不匹配而丢弃。
- hidden route 不停止 session；`getActiveSession` 恢复状态。
- app restart 后未完成 session 进入 `cancelled_on_restart`，不恢复 verifier/code。

### 3.5 Destructive actions

移除账号前 backend 返回 impact preview：

```text
将断开：Codex、FyAgent Local Proxy
不会改变：当前 DeepSeek API 配置
需要重启：OpenCode
```

确认 payload 只包含 preview ID/revision + account ID，不由前端提交任意 consumer 列表。提交前 backend fresh revalidates preview revision。

### 3.6 Restart experience

- 默认不自动关闭桌面软件。
- projection 需要重启时，先完成安全准备，再显示确认。
- “稍后重启”将 connection 标记为 `pending_restart`，不能显示已生效。
- 用户确认后由现有 trusted launch/process owner关闭与重启，再 readback。
- 失败时显示“连接已保存但应用尚未加载”，提供重试/打开应用，不把整个账号删除。

### 3.7 Accessibility and responsive behavior

- `FeatureTabs`、`CatalogMasterDetail`、shared Dialog/Button/InlineNotice/Spinner 优先复用。
- desktop 使用 master-detail；窄窗口详情覆盖列表并有明确返回。
- 所有 session stage 用文本 + icon，不只用颜色/动画。
- dialog focus trap、initial focus、Esc/取消语义、关闭后焦点恢复有 browser tests。
- `prefers-reduced-motion` 下 spinner 可保留语义但不依赖位移动画。
- email/Provider 采用视觉截断，accessible name 保留完整安全文本。

## 4. Frontend code ownership

建议仓库形态：

```text
src/v2/
  app/
    primaryPages.tsx                 # add auth route loader
  shared/
    config/navigation.ts             # add closed navigation item
    features/
      managed-auth.ts                # strict wire types/parsers/port
      managed-auth-queries.ts?       # only if queries.ts would become oversized
      agent-auth.ts                  # summary/session evolution
    platform/tauri/feature-ports/
      managedAuth.ts                 # invoke + strict parsing
    ui/
      ManagedAccountPicker.tsx       # only after >=2 real consumers
      ManagedAuthStatus.tsx          # shared presentational primitive if reused
  pages/auth/
    Page.tsx
    AccountsView.tsx
    AccountDetail.tsx
    ConnectionsView.tsx
    ManagedAuthLoginDialog.tsx
    RemoveAccountDialog.tsx
    RestartRequiredDialog.tsx
    Page.css
```

Rules：

- `pages/auth/**` 只做 composition/state selection，不 import Tauri。
- wire parser exact-key、closed enum、ID/date/length validation 与 forbidden-field rejection 放在 `managed-auth.ts`。
- backend/resource state 用 TanStack Query；dialog draft 用 local state；backend session 由专用 hook恢复。
- 不 import V1 `AuthCenterPanel`、`useManagedAuth` 或 `src/lib/api/auth.ts`。
- 只有真实复用出现两次以上才抽 shared component；不先创建通用 OAuth UI framework。

### 4.1 Legacy UI convergence

迁移顺序：

1. V2 Auth Page 达到现有 Codex/xAI/Copilot功能 parity；
2. Settings Auth tab 改为进入 V2 页的兼容入口，或在旧 shell 中调用同一 FeaturePort adapter；
3. Provider forms 改用 shared account picker/port；
4. 删除旧 page-owned polling、旧 DTO 和重复 mutation owner。

在 parity 前不删除旧页面，但旧页面不得新增新的 browser loopback/Grok/OpenCode能力。

## 5. Wire contracts

Exact names可按代码风格调整，但语义和 forbidden fields稳定。

### 5.1 Overview

```text
ManagedAuthOverviewDto {
  contractVersion,
  checkedAt,
  providers: ManagedAuthProviderSummary[],
  accounts: ManagedAuthAccountSummary[],
  connections: ManagedAuthConnectionSummary[],
  activeSessions: ManagedAuthSessionSnapshot[]
}
```

### 5.2 Account

```text
ManagedAuthAccountSummary {
  accountId,                  // opaque public account id
  provider: openai | xai | github_copilot,
  login,
  displayName?,
  avatarUrl?,
  health,
  isDefault,
  lastAuthenticatedAt,
  connectedConsumerCount,
  planSummary?,
  quotaSummary?,
  allowedActions,
  reasonCodes
}
```

禁止：`secretRef`、`credentialId`、`refreshOwner`、token、workspace routing ID、raw claims。

### 5.3 Connection

```text
ManagedAuthConnectionSummary {
  connectionId,
  consumer: codex | grokbuild | opencode | fyagent_proxy,
  targetId?,
  targetLabel?,
  accountId?,
  providerConnectionId?,
  authStatus,
  credentialManagementLabel,
  requestMode: official_subscription | third_party_api | provider_connections | none | unknown,
  requestProviderLabel?,
  officialSessionPreserved?,
  pendingRestart,
  allowedActions,
  checkedAt,
  revision,
  reasonCodes
}
```

### 5.4 Login session

```text
ManagedAuthLoginSessionSnapshot {
  contractVersion,
  sessionId,
  provider,
  purpose,
  consumer?,
  method: browser_loopback | device_code,
  stage,
  canCancel,
  canRetry,
  canSwitchToDeviceCode,
  officialHost,
  userCode?,                  // only device flow
  verificationUri?,           // validated, query-free/safe display URL
  expiresAt?,
  accountId?,
  connectionId?,
  reasonCode?,
  terminal
}
```

禁止返回 authorization URL、callback URL、code、state、verifier、device code、token、native path、raw error。

### 5.5 Mutations

所有 mutation request采用：

```text
opaque id + expected revision + closed action + optional closed target capability
```

禁止 URL/path/command/args/environment/free-form Provider credential。

## 6. Backend architecture

### 6.1 Repository shape

```text
src-tauri/src/
  services/managed_auth/
    mod.rs                    # ManagedAuthService facade
    types.rs                  # domain types/closed enums
    repository.rs             # SQLite metadata only
    secret_bundle.rs          # typed SecretRef bundle codec
    login_sessions.rs         # backend-owned sessions
    refresh_coordinator.rs    # per-session lock/generation/lease
    migration.rs              # legacy JSON -> SecretRef
    providers/
      openai.rs               # PKCE + Device Code + refresh
      xai.rs                  # discovery + Device Code + refresh
      github_copilot.rs       # adapter over existing owner
    consumers/
      codex.rs                # native observation/projection/switch
      grok.rs                 # auth command or registry projection
      opencode.rs             # provider auth/store projection
      proxy.rs                # token resolution for forwarder
  commands/managed_auth.rs    # thin closed commands
```

文件名可按最终模块规模调整；职责边界不可合并回 `commands/auth.rs` 或 `proxy/providers/*_oauth_auth.rs` 的大文件。

### 6.2 Facade

`ManagedAuthService` 是唯一运行时 owner，持有：

```text
MetadataRepository
SecretService<NativeSecretBackend>
OpenAiAdapter
XaiAdapter
CopilotAdapter
LoginSessionStore
RefreshCoordinator
ConsumerCoordinator
```

不使用 async trait object。Provider/consumer dispatch使用闭集 enum + concrete module methods；只有存在多个真实实现且可读性更好时引入同步 trait。

现有初始化迁移：

```text
lib.rs
  -> create NativeSecretBackend / SecretService
  -> open DB metadata repository
  -> run/inspect credential migration
  -> construct ManagedAuthService
  -> remap existing Provider authBinding
  -> Proxy/commands receive shared State<ManagedAuthService>
```

删除/收敛：

- `CodexOAuthState` / `XaiOAuthState` 作为独立 public state；
- commands直接读写 manager lock；
- proxy forwarder直接依赖具体 manager类型；
- V1 page-owned Device Code polling。

## 7. Persistence model

### 7.1 SQLite metadata

建议三张表：

```text
managed_auth_identities
  id PK
  provider
  provider_subject
  provider_tenant
  login
  display_name
  avatar_url
  created_at
  updated_at
  UNIQUE(provider, provider_subject, provider_tenant)

managed_auth_credentials
  id PK
  identity_id FK
  purpose
  consumer
  secret_ref
  secret_version
  refresh_owner
  generation
  access_expires_at
  status
  authenticated_at
  refreshed_at
  created_at
  updated_at

managed_auth_connections
  id PK
  consumer
  target_id
  provider_slot
  credential_id FK
  desired_revision
  observed_revision
  status
  pending_restart
  created_at
  updated_at
  UNIQUE(consumer, target_id, provider_slot)
```

- secret ref/version只在 backend DB，不进入 renderer。
- `provider_subject`/tenant来自经一致性检查的稳定 claim，不用 email 做唯一键。
- `target_id` 是现有 inventory 产生的 opaque capability/稳定目标 ID，不是绝对路径。
- 删除 identity前必须处理所有 credential和connection，外键启用。

### 7.2 Secret bundle

一个 Credential Session 对应一个 secret bundle：

```text
ManagedOAuthSecretBundleV1 {
  schemaVersion,
  credentialId,
  provider,
  generation,
  accessToken?,
  refreshToken?,
  idToken?,
  tokenType?,
  grantedScopes,
  issuedAt?,
  expiresAt?
}
```

要求：

- 只存在 OS-native secret backend；
- 序列化/反序列化 buffer 使用 `Zeroizing`，离开 callback 即清零；
- `Debug` 永不打印字段；
- `services::secret` 增加窄的 typed decoder/encoder callback，不暴露通用原始 bytes API；
- replace后返回新的 SecretVersion；metadata generation与bundle generation必须一致。

### 7.3 Secret write transaction

Create：

```text
validate identity/session intent
begin DB transaction -> insert provisioning metadata
create SecretRef
probe/readback via typed decoder
commit active metadata with SecretRef/version
commit DB
```

由于 OS keyring与SQLite不能形成一个物理事务，使用 recovery journal/state：

- DB `provisioning` + secret存在：启动时验证并完成；
- DB row存在 + secret缺失：标记 `secret_missing`，不自动重新登录/伪造；
- secret创建成功但DB commit失败：记录 in-process receipt并best-effort删除；下一次启动按 deterministic recovery key清理；
- 永不以普通文件作为 fallback。

Refresh：

```text
acquire process credential lock
acquire cross-operation generation lease
read metadata + secret version/generation
network refresh
re-check generation
replace secret bundle
update metadata/version/generation in DB
publish account/connection invalidation
```

晚到结果、旧 refresh owner或generation变化时丢弃，不覆盖新 token。

## 8. OAuth provider adapters

### 8.1 OpenAI browser loopback

默认流程对齐 OpenAI第一方实现：

- public client identity、authorize/token endpoint、scope与hosted wrapper从 reviewed first-party source冻结；
- 32-byte cryptographic state；
- PKCE S256；
- 只绑定 `127.0.0.1`；
- 首选 registered port `1455`，再尝试 first-party fallback `1457`；
- path `/auth/callback`；
- GET only、bounded request、one-shot、deadline；
- exact session/state/login generation校验；
- callback handler只写入 process-private one-shot channel；
- token exchange由provider adapter完成；
- browser通过backend opener打开，授权URL不进入普通日志/持久化。

端口被未知进程占用时不发送通用cancel/kill；尝试下一个注册端口，再转 Device Code。只有本进程已知 session可以复用/取消。

### 8.2 OpenAI Device Code

复用当前 `CodexOAuthManager` 已实现的特殊流程：

```text
/api/accounts/deviceauth/usercode
/api/accounts/deviceauth/token
/oauth/token
```

但移入 OpenAI adapter，由 backend session按server interval轮询；renderer不持有 `device_auth_id`。

### 8.3 xAI

复用当前 `XaiOAuthManager`：

- OIDC discovery；
- host/scheme validation；
- Device Code；
- bounded JSON；
- `slow_down`；
- reauth状态；
- refresh rotation。

补齐 session cancel、backend polling和SecretRef persistence。endpoint metadata可缓存，但每次加载都验证host；不持久化未验证endpoint。

### 8.4 Identity validation

- token来自HTTPS token endpoint与对应PKCE/device session。
- decode JWT只用于稳定identity/display metadata，不作为本地权限授权。
- OpenAI至少交叉检查可获得的 account/workspace ID；xAI要求稳定 `sub`，tenant/team metadata可选。
- claim冲突、缺失稳定subject、provider不匹配均不保存。
- raw claim不进入UI/日志；只存allowlisted metadata。

## 9. Credential session and refresh ownership

### 9.1 Model

```text
RefreshOwner = fyagent | codex_native | grok_native | opencode | unavailable
CredentialPurpose = proxy_upstream | codex_native | grok_native | opencode_provider | copilot
```

不提供“shared” owner。

### 9.2 Default connection policy

- 用户已有同一身份但没有目标consumer session时，UI显示“连接此软件需要一次独立官方授权，以避免多个程序争用登录凭据”。
- 完成新授权后按 stable identity合并到同一账号卡片，但新增 Credential Session。
- 只有实施阶段证明 provider支持安全 token broker/owner不转移时，才可复用现有 FyAgent session。
- 不提供高级“强制共享 refresh token”开关。

### 9.3 Native owner reconciliation

当 refresh owner转为 native consumer：

1. FyAgent停止该session的主动refresh；
2. consumer启动/使用前投影当前generation；
3. 切换、退出或状态刷新前读取权威native store/API；
4. 只有 identity与projection marker匹配时，将更新后的token chain写回SecretRef并提高generation；
5. 若native store被其他账号替换，不覆盖，连接变为`external_change_detected`；
6. 同步失败不使用旧refresh token竞争刷新。

## 10. Consumer adapters

### 10.1 Codex

职责：

- 通过 official app-server `account/read` 或支持的native store观察账号状态；
- 将专用 Codex Credential Session投影到支持的store；
- 切号前同步当前native generation；
- 停止/重启由现有 Codex Desktop process/instance owner执行；
- config provider与auth credential分开提交；
- 完成后重新观察账号和active model provider。

Credential-store matrix：

| Mode | 计划 |
| --- | --- |
| `file` | merge + atomic write `auth.json`，权限/readback/marker齐全后启用 |
| `keyring` | 只在对应平台存在受审查native adapter或通过official app-server安全写入时启用 |
| `auto` | 解析实际authority并按其路径处理；无法证明时unsupported，不回退file |
| `ephemeral` | 不支持持久多账号投影；引导在Codex中登录或改用受支持模式，不自动修改设置 |

`preserveCodexOfficialAuthOnSwitch` migration：

1. 所有第三方 Provider writer先改为config-only，不触碰官方store；
2. UI删除该危险开关；
3. 兼容读取旧值但行为始终preserve；
4. 一次版本迁移后删除字段和分支；
5. negative tests覆盖每条writer/proxy/rollback路径。

### 10.2 Grok Build

优先方案：

```text
Grok auth_provider_command
  -> shipped narrow FyAgent credential helper
  -> request valid access token for one opaque connection
  -> FyAgent remains refresh owner
```

Gate：

- 当前正式Grok版本、配置schema、stdout合同、刷新时传参、签名/路径和hot reload在macOS/Windows HIL通过；
- helper只能接受opaque connection ID，不能接受URL/command/provider/token；
- stdout只写token，stderr限长脱敏；
- consumer校验helper identity/位置。

若该Gate不满足，fallback不是旧`grok login` handoff，而是：

- 为Grok建立独立native session；
- merge官方registry entry；
- Grok成为refresh owner；
- 使用官方 `auth.json.lock` 兼容策略与generation reconcile；
- 未证明平台保持disabled。

### 10.3 OpenCode Desktop

两条显式路径：

1. **由 OpenCode 管理**：打开官方Desktop Provider Connect；FyAgent只观察provider list/metadata。
2. **由 FyAgent发起并连接**：为OpenCode创建独立Credential Session，写入官方 `auth.json` entry，然后将refresh owner转为OpenCode。

约束：

- 不扫描Desktop内部随机sidecar端口/密码；
- 路径从OpenCode官方 `Global.Path.data`规则和目标用户上下文解析，不使用CLI PATH；
- read-modify-write保留未知provider和合法字段；
- schema validation + `0600` + atomic replace；
- 外部文件变化通过hash/revision检测；
- 不把Codex/FyAgent Proxy的同一refresh token直接复制过来；
- restart由真实HIL决定，不凭源码假定hot reload。

### 10.4 Proxy

`proxy/forwarder.rs` 不再依赖 `CodexOAuthManager`/`XaiOAuthManager` concrete type。它调用：

```text
ManagedAuthTokenResolver.resolve(connection/credential id, request context)
```

Resolver只允许 `refresh_owner=fyagent` 的session主动refresh。native-owned session不得被Proxy使用；用户需建立proxy-purpose session。

## 11. Connection transaction

统一consumer mutation：

```text
fresh account/session/target observation
build impact preview + revision
user confirms closed preview
acquire consumer + credential locks
revalidate preview/target/generation
sync old native owner if needed
prepare target projection in memory/staging
stop/restart prompt boundary if required
commit secret/metadata/native projection in ordered transaction
readback native state + active provider
update connection observed revision
publish success
```

Rollback：

- native write前失败：无用户可见变更；
- native write后readback失败：恢复exact preimage（hash/revision仍匹配时）；
- preimage已被外部修改：不覆盖，标记`recovery_required`；
- metadata commit失败：从native marker/readback恢复或标记repair，不重复写旧token；
- restart失败：保留安全projection，connection为`pending_restart`，不回滚账号secret。

## 12. Legacy migration

### 12.1 Sources

- `codex_oauth_auth.json`
- `xai_oauth_auth.json`
- 现有 manager default account metadata
- Provider `meta.authBinding`

### 12.2 Algorithm

1. 启动前计算source hash，读取并严格解析；不记录内容。
2. 在SQLite创建versioned migration journal。
3. 逐账号创建SecretRef bundle（现有JSON通常只有refresh token，access/id可空）。
4. probe + typed readback；写identity/credential metadata。
5. 所有账号成功后在一个DB事务提交mapping/default/bindings/migration complete。
6. 将旧文件原子rename为bounded backup；不立即删除。
7. 下一版本/明确cleanup动作后删除backup。

失败：

- 已创建SecretRef记录在journal，retry复用或清理，不重复账号；
- 旧文件保持原样；
- managed-auth进入`migration_blocked`，禁止新登录/refresh/projection；
- 用户解锁/修复OS vault后重试；
- 不回退继续向旧明文文件写token。

### 12.3 Existing native logins

- 默认只观察，不静默导入。
- “导入当前登录”是显式动作；先显示来源软件和影响。
- keyring/opaque native store不能安全读取时，不声称可导入；允许用户重新走官方登录创建managed session。

## 13. Error model

Backend返回闭集 reason code，例如：

```text
browser_open_failed
loopback_port_unavailable
authorization_cancelled
authorization_timed_out
state_mismatch
callback_invalid
token_exchange_failed
device_code_expired
refresh_rejected
refresh_owner_conflict
credential_generation_changed
secret_store_locked
secret_store_denied
secret_store_unavailable
credential_missing
migration_blocked
identity_mismatch
native_store_unsupported
native_store_changed
consumer_running
restart_required
restart_failed
projection_failed
projection_readback_failed
recovery_required
target_selection_required
target_changed
network_unavailable
```

Raw error只进入限长、脱敏的backend debug context；renderer收到reason + safe detail fields。V2拥有唯一reason-to-copy map，V1 compatibility adapter复用同一safe message。

## 14. Security and privacy

- callback server只绑定loopback，bounded request/timeout/one-shot。
- Authorization URL、callback query、state、verifier、device code、token不写日志；user code可在UI显示但不持久化。
- Secret bundle只在zeroizing内存中解码；禁止Clone/Debug/Serialize到DTO。
- SQLite backup/export默认排除OS secret material，只包含opaque refs；导入到另一设备不能恢复账号，UI明确提示。
- screenshot/error telemetry不包含email时应优先使用匿名account ID；用户可见页才显示login。
- account delete顺序：先解除/验证连接，再删除SecretRef，最后删除metadata；失败可重试且不悬空。
- external callback URL手工粘贴只作为最后恢复能力；parser只接受当前session exact loopback origin/path/state，永不发送给网络或日志。
- OAuth host allowlist、redirect URI、client ID、scope来自代码闭集，不能由remote config任意替换。

## 15. Reuse and dependency plan

### 15.1 Reuse map

| Need | Owner to reuse |
| --- | --- |
| OpenAI Device Code/token refresh/account binding | current `CodexOAuthManager` logic |
| xAI discovery/device/refresh | current `XaiOAuthManager` logic |
| browser PKCE security behavior | OpenAI first-party login server + current adopted crypto/http crates |
| OS secret storage | `services::secret` |
| backend auth session lifecycle | existing `AgentAuthSessionStore` pattern; extract common pure mechanics only if two owners truly match |
| V2 strict DTO/port/query | current V2 feature architecture |
| master-detail/tabs/dialog/buttons/notices | current V2 shared UI |
| target selection/process restart | existing Agent inventory/Codex/OpenCode process owners |
| Provider account binding | current `AuthBinding`, migrated to connection projection |
| native file atomic writes | existing reviewed atomic/config writers where semantics match; otherwise one auth-specific adapter |

### 15.2 Dependency gate

在实施Phase 0生成`research/dependency-and-license-decision.md`：

- 先证明现有依赖足够；
- 对候选OAuth/OIDC/keyring crate评估维护、Rust版本、license、advisory、WASM/desktop footprint、PKCE/Device Code/provider特殊流程；
- 只有减少实质安全敏感代码时加入；
- exact lock + license/NOTICE；
- 禁止引入整套代理/账号管理项目作为二进制或源码vendor来绕过设计。

## 16. Tests and evidence

测试按四层建立，完整矩阵由 `implement.md` 持有：

1. **Frontend contract/browser**：严格 parser、账号/连接/请求来源三层展示、登录向导、focus/keyboard/narrow viewport/hidden route，以及 DOM/console/wire 无敏感字段。
2. **Backend protocol/domain**：PKCE/Device Code、identity 校验、SecretRef recovery、refresh owner/generation CAS、迁移幂等与 consumer transaction rollback。
3. **Native store adapters**：Codex/Grok/OpenCode 的 merge、未知字段保留、权限、锁、external write、readback 和 pending restart。
4. **macOS/Windows HIL**：正式签名/安装构建验证真实登录、刷新、官方/第三方往返、外部应用竞争和崩溃恢复。

任何 production capability 必须同时通过适用的自动化合同和正式平台 HIL；mock、源码阅读或交叉编译不能替代 HIL。

## 17. Rollout

```text
contract_and_ux_mock
  -> secretref_migration_ready
  -> openai_managed_login_ready
  -> codex_connection_hil_ready
  -> xai_grok_hil_ready
  -> opencode_hil_ready
  -> legacy_center_retired
```

每个consumer有独立backend capability。未通过阶段不显示可执行按钮；可以显示“当前版本暂不支持此连接方式”和官方应用手动登录入口。

Feature flag只用于阶段发布，不能绕过secret store或readback。回滚关闭consumer capability并保留SecretRef/metadata，不自动删除用户账号或原生登录。

## 18. Rejected alternatives

以下方案不采用：保留三套 Auth owner；只做 `app-server` handoff 而不满足统一账号需求；复制受限许可项目；跨 consumer 共享同一 refresh-token lineage；继续使用明文 JSON；把 token 放进 callback URL；扫描 OpenCode 私有 sidecar；页面自行轮询 token endpoint；不支持 store 时静默降级到 file；继续让用户决定是否保护 Codex 官方登录。它们分别违反统一体验、许可证、安全所有权、协议或 fail-closed 原则。

## 19. Spec convergence

完成前必须更新 frontend/backend owning specs，使其只指向中央账号页、`ManagedAuthService`、SecretRef、consumer adapter 与单 refresh-owner 合同；旧 handoff、旧 Auth Center 和可选保留官方登录描述必须删除或标为历史兼容。具体文件清单由 `implement.md` Phase 11 持有。
