# Design：Codex `auth.json` 最小交换与 Provider 协作

## 1. 设计摘要

本设计不再把“官方账号”和“请求 Provider”合并为一套大状态机，也不新增
Change Plan operation。后端先观察两个正交维度，再计算最小 delta：

```text
Managed Auth action
  -> observe live auth + effective store + provider route
  -> materialize selected official auth only when account differs
  -> atomically swap auth.json only when needed
  -> call existing ProviderService only when route differs
  -> read back live authorities
  -> update connection / refresh owner
  -> return authoritative overview
```

纯官方账号 A→B 只写 `auth.json`。第三方→当前已保存官方账号只切 Provider。
两者都变化时，在同一 Codex mutation guard 下先让目标 auth 就绪，再复用现有
ProviderService 切 official。

## 2. Owner 边界

| 能力 | 唯一 owner | 本任务改动 |
| --- | --- | --- |
| OAuth/refresh/identity | `managed_auth::providers::openai` | 复用；只补窄 materializer helper |
| SecretRef/generation/refresh owner | `ManagedAuthService` + repository | 复用 CAS 与 credential lock |
| auth/config 路径与文件原语 | `codex_config` + `config` | 增加强类型 auth observation/swap/readback |
| Provider route/backfill/policy/current/MCP | `ProviderService` | 暴露 crate-private lock-held official seam；不复制 |
| action concurrency/revision | Managed Auth connection action + Codex Provider mutation guard | 复用；不新增 durable job |
| restart | Codex Desktop service | 复用 pending/restart/forced confirmation |
| V2 状态 | Managed Auth FeaturePort | 只消费闭集状态和 authoritative overview |

不新增：

- Change Plan operation/resource/contract；
- Provider CRUD 或第二 writer；
- OAuth client；
- secret backend/vault；
- process manager；
- Rust/npm dependency。

## 3. Live 观察模型

由 Codex consumer/codex_config 暴露 crate-private 单次观察结果：

```rust
struct CodexManagedAuthObservation {
    config_revision: String,
    auth_revision: Option<String>,
    effective_store: CodexEffectiveCredentialStore,
    provider_route: CodexProviderRoute,
    auth_state: CodexNativeAuthState,
    codex_running: bool,
}
```

所有 capability、delta、expected revision、readback 和 overview 使用同一观察结构，
避免各层分别读取文件后得出冲突结论。

### 3.1 Effective credential store

```text
unset       -> FileByDefault
file        -> FileExplicit
auto        -> UnsupportedAuto
keyring     -> UnsupportedKeyring
ephemeral   -> UnsupportedEphemeral
unknown     -> UnsupportedUnknown
invalid TOML-> Invalid
```

只有 `FileByDefault | FileExplicit` 可执行本任务投影。unset 不补写
`cli_auth_credentials_store`；显式非 file 不被改写。

### 3.2 Provider route

```text
model_provider missing  -> Official("openai")
model_provider="openai" -> Official("openai")
known custom id          -> ThirdParty(id)
invalid/unowned value    -> Unknown
invalid TOML             -> Invalid
```

### 3.3 Native auth state

```text
Missing
ChatGptKnown(account_id, revision)
ChatGptUnmanaged(identity_fingerprint, revision)
ThirdPartyApiKeyOnly(revision)
PersonalAccessToken(revision)
AgentIdentityOnly(revision)
Bedrock(revision)
Unsupported(revision)
Invalid(revision)
Unreadable
Oversized
```

fingerprint/revision 只用于比较，不把 token、完整路径或原文暴露给 renderer。

## 4. Delta planner

定义一个纯函数：

```rust
fn plan_codex_managed_auth_delta(
    live: &CodexManagedAuthObservation,
    target_account_id: &str,
) -> Result<CodexManagedAuthDelta, ManagedAuthCoreError>
```

输出闭集：

```rust
enum CodexManagedAuthDelta {
    Noop,
    AuthOnly,
    ProviderOnly,
    AuthThenProvider,
}
```

规则：

| live account | route | target account | delta |
| --- | --- | --- | --- |
| target | official | target | `Noop` |
| other/missing | official | target | `AuthOnly` |
| target | third-party | target | `ProviderOnly` |
| other/missing | third-party | target | `AuthThenProvider` |
| unmanaged/unsupported | 任意 | target | error / safe recovery only |

`Noop` 不刷新 token、不重写文件、不更新 current Provider、不触发 restart。

## 5. 第一方 auth 文档

以 research 固定的 OpenAI Codex `AuthDotJson` / `TokenData` 为依据，FyAgent 定义
最小 crate-private serde adapter：

```rust
struct CodexChatGptAuthDocument {
    auth_mode: ChatGpt,
    openai_api_key: None,
    tokens: CodexTokenData,
    last_refresh: DateTime<Utc>,
}
```

`CodexTokenData` 必须包含：

- raw ID token；
- access token；
- refresh token；
- 可选、经 claim 验证的 account ID。

约束：

- API key、PAT、Bedrock 等互斥 material 不写入；
- 未知字段读取可容忍；
- 跨账号不复制未知 secret subtree 或 agent identity；
- 只有能用 account/chatgpt-user claim 证明属于同一目标账号的 agent identity 才可
  保留，否则让 Codex 后续自行 bootstrap；
- `Debug`/error/log 全部脱敏；
- 上游 fixture 变化触发 planning review。

## 6. Credential materialization

### 6.1 新登录

1. 完成现有 OAuth 和 identity 验证；
2. 先成功 admission 到 SecretRef；
3. 本次 grant 的 ID/access/refresh 完整时直接构造 auth；
4. 不为“统一流程”再次 refresh；
5. grant 不完整时保存账号，但 Codex projection 返回 `requires_reauth/partial`，
   不写残缺 auth。

### 6.2 历史账号

在目标 credential lock 下：

1. 重读 generation/status/owner；
2. 解码现有 bundle；
3. token 完整、身份匹配且可用时直接构造；
4. 只有缺必要字段或 access 不可用时，才复用现有 OpenAI refresh；
5. rotation 先通过 SecretRef generation CAS 持久化；
6. refresh 响应缺字段时，只能复用仍有效且身份匹配的旧字段；
7. 最终不完整则 `requires_reauth`；
8. generation 冲突最多有界重读一次，不无界重试。

### 6.3 Secret 容量停止条件

当前 Managed OAuth bundle 可能因 Windows 2560-byte 限制省略 token。实现测试必须
证明“登录多个账号后可以切走再切回”。如果 refresh 不能可靠重建完整目标文档，
不得用明文文件或提高全局上限绕过；回到 planning 评审基于现有 SecretService 的
component-secret manifest。该评审属于存储能力，不应污染 auth 交换状态机。

## 7. 当前活动账号对账

Codex 会刷新并原地回写活动 `auth.json`。覆盖前：

1. bounded-read live auth；
2. 验证 `auth_mode=chatgpt` 与完整 token；
3. 从 ID/access claim 得到稳定 identity；
4. 只在 identity 唯一匹配某个 `CodexNative` credential 时，对账最新 token；
5. CAS 条件包含 expected owner=`CodexNative` 与 generation；
6. 对账成功后旧账号 owner 转回 `Fyagent`；
7. 未知账号、外部换号、损坏或删除不覆盖 SecretRef，也不覆盖 live 文件。

正常运行期间 Codex 是活动账号的 refresh owner；FyAgent 不与 Codex 并发主动刷新。

## 8. Auth-only 交换器

建议位于 `services/managed_auth/consumers/codex/`，文件写入继续由 `codex_config`
owner 提供：

```rust
fn swap_codex_chatgpt_auth(
    expected_auth_revision: Option<&str>,
    target: &CodexChatGptAuthDocument,
) -> Result<CodexAuthSwapReceipt, AppError>
```

执行：

1. 获取 Codex auth writer lock；
2. 重新 bounded-read 并比较 expected revision；
3. 捕获 exact preimage；
4. deterministic serialize；
5. 复用 atomic write；
6. Unix 新文件/替换后保证 owner-only `0600`；
7. 立即 bounded readback；
8. 验证 auth mode、目标 identity、token presence 和新 revision；
9. 返回只含 revision/identity code/change bool 的 receipt。

失败：

- 写前 stale → 零写入；
- 写入/readback失败 → 仅当 live revision仍为本次值时恢复 exact preimage；
- 外部 revision改变 → 停止覆盖，返回 external change；
- rollback失败 → recovery required；
- 任何路径均不返回 raw auth/token/path。

`AuthOnly` 路径不得读写 `config.toml`、Provider DB/device current 或 MCP。

## 9. Provider 协作

### 9.1 统一锁

为避免 auth swap 与 Provider switch 竞态，所有 Codex Managed Auth mutation 先获取
现有 per-app Provider mutation guard。纯 `AuthOnly` 也获取同一 guard，但不调用
Provider writer。这样无需新增另一把跨系统总锁。

### 9.2 ProviderOnly

live auth 已是目标账号时：

1. 不物化、不写 auth；
2. 复用 `ProviderService` lock-held official switch seam；
3. 读回 `model_provider`、DB/device current 和 live identity；
4. 提交 connection state。

### 9.3 AuthThenProvider

1. 获取 Provider mutation guard；
2. 重读 live observation；
3. 若 current auth 是 legacy API-key-only，调用现有 Provider current-backfill seam；
4. 未证明 key 可恢复则停止；
5. 对账 current official account（如有）；
6. 物化 target；
7. auth-only swap + identity readback；
8. 调用现有 ProviderService 切固定 `codex-official`；
9. 读回 route/current/live identity；
10. promote target owner=`CodexNative` 并更新 connection。

Provider switch 失败时：

- 当前 route 仍第三方且 auth revision未被外部改写：恢复 auth preimage；
- route 已到 official 且 auth是目标：按成功目标读回收敛，不反向切换；
- DB/device/live mixed 或外部 revision变化：返回 recovery/external change，不猜测；
- overview 可从真实 authority 重建，不依赖持久步骤 ledger。

### 9.4 不复制 ProviderService

需要新增的 seam 只能是 crate-private 的 prepare/backfill/switch-with-lock-held 组合；
必须复用：

- proxy takeover 和 official 安全 policy；
- effective current Provider；
- common config sync；
- current live backfill；
- DB/device current；
- target live config writer；
- stale third-party auth cleanup；
- MCP re-projection。

## 10. 官方切第三方

不进入本任务的新 coordinator：

- Provider 页面继续使用现有 Change Plan；
- 第三方配置写入仍为 config-only；
- 加回归断言：`auth.json` 写前/写后字节相等；
- Managed Auth overview 可报告官方 session preserved；
- 之后 `switch_to_official` 根据 delta 执行 ProviderOnly 或 AuthThenProvider。

## 11. Refresh owner 与 connection 提交

顺序：

1. current source token 对账成功；
2. target auth readback成功；
3. route readback（如需要）成功；
4. target owner CAS 为 `CodexNative`；
5. connection desired/observed revision、request mode、pending restart 更新；
6. 返回 overview。

如果 owner/metadata 在 live 已到目标后提交失败，不盲目回滚一个可能已经被 Codex
读取/刷新的会话。返回 partial/recovery，overview 以 live identity 为准并允许
有界 reconcile。

## 12. 状态与 actions

| 事实 | 状态/reason | actions |
| --- | --- | --- |
| ready credential，live 未使用 | disconnected / saved-not-projected | connect/switch |
| live target + official | connected | switch account / disconnect / open |
| live target + third-party | connected + third-party + session preserved | switch to official / switch account |
| 写入后运行进程可能缓存旧值 | pending restart | restart / refresh / open |
| explicit non-file store | unavailable + store unsupported | refresh / settings guidance |
| token无法组成完整文档 | requires reauth | reauthenticate |
| live unknown account | unmanaged/external change | refresh / safe recovery |
| rollback/owner/metadata不确定 | partial/recovery required | re-read / review |

`switch_to_official` 没有明确 connection credential 时不广告；不能隐式选第一个或
默认账号。

## 13. 重启

- Codex 未运行：成功读回后无需 pending restart；
- Codex 运行中：未证明热加载时返回 pending restart；
- 重启复用 `services/codex_desktop`；
- 不按进程名 kill、不猜安装路径；
- 重启后重新观察 identity/provider，只有一致才清 pending。

## 14. 锁顺序

```text
Codex Provider mutation guard
  -> credential locks（按 credential_id 排序）
  -> auth writer lock
  -> filesystem
  -> repository generation/owner/connection CAS
```

网络 refresh 应尽量在 Provider guard 外完成候选 materialization，再在 guard 内重验
credential generation；若必须在锁内，使用现有 bounded timeout。不得持有
credential lock后反向调用会重新获取 Provider guard 的 public API。

## 15. 模块改动边界

### Backend

- `services/managed_auth/consumers/codex*`：observation、delta、auth adapter、swap；
- `services/managed_auth/login.rs` / `service.rs`：materialization、owner、connection；
- `services/managed_auth/providers/openai.rs`：复用现有 refresh 的窄 helper；
- `commands/managed_auth.rs`：组合取得 ManagedAuthState/AppState；
- `services/provider/mod.rs`：只提炼 lock-held backfill/official seam；
- `codex_config*` / `config.rs`：effective defaults、atomic auth readback/permissions；
- DAO：优先零 schema；仅复用现有 generation/owner/connection CAS。

明确不改 Change Plan public registry/domain/contract，除非实施发现停止条件并回到
planning。

### Frontend

- Managed Auth FeaturePort/presentation/ConnectionsView/MutationDialogs；
- 复用 Provider 页面 deep-link；
- 不增加 Change Plan job UI 或第三方 Provider 表单。

## 16. 测试设计

### Pure/fixture

- store effective defaults；
- missing provider→openai；
- delta 四分支；
- AuthDotJson 完整/残缺/互斥模式/未知字段；
- no-op 不产生 materialization/write/restart。

### Secret/owner

- fresh grant direct；
- complete bundle direct；
- incomplete bundle bounded refresh；
- refresh字段缺失与 requires reauth；
- native token回收；
- owner/generation冲突；
- 多账号切走再切回。

### File/provider

- A→B 只改 auth；
- third-party+A→official+A auth byte-equal；
- third-party+A→official+B 最小两类写入；
- official→third-party auth byte-equal；
- legacy API key backfill失败零覆盖；
- stale/external revision、permission、readback、rollback failure；
- concurrent account/provider mutation串行；
- proxy/forced policy在目标提交前失败。

### V2/security

- saved-not-projected；
- switch_to_official action矩阵；
- pending/reauth/store unsupported/external/recovery；
- mutation后 authoritative overview；
- token/path/SecretRef/raw auth不进 DTO/log/DOM。

## 17. HIL 定位

可选 smoke 覆盖 macOS/Windows、CLI/Desktop、A→B、官方→第三方→官方、运行中
restart、Codex 自动 refresh 和外部改写。HIL 失败形成兼容性 issue；不恢复 blanket
runtime gate，也不替代自动化 readback/fault injection。
