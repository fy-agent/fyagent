# FyAgent 当前实现盘点与缺口

> 采集日期：2026-09-04
> 仓库基线：`5c85e8701b95660c54d0d7b39b93e1edfb56e8ae`
> 分支：`dev/laiyongjie`

## 1. 已有 owner，必须复用

### Managed Auth

- `services/managed_auth/login.rs`
  - 已有 OpenAI browser-loopback PKCE与Device Code；
  - login grant包含 access/refresh/可选 ID token；
  - `CodexNative`登录先保存为 `RefreshOwner::Fyagent`；
  - `connect_consumer=codex` 当前只写元数据并因 gate返回 partial。
- `services/managed_auth/secret_bundle.rs`
  - versioned SecretRef bundle、zeroize、无 secret Debug/Clone；
  - Windows 2560-byte backend容量策略可能先省略 ID token，再省略 access token，优先保留 refresh token。
- `services/managed_auth/service.rs` / repository
  - per-credential refresh lock；
  - generation CAS、refresh-owner CAS、SecretRef readback；
  - native-purpose lineage不允许走普通 proxy resolver，避免双 refresh owner；
  - 已有 OpenCode projection/owner transfer可参考事务语义。

### Codex consumer/config

- `consumers/codex.rs`
  - bounded config观察、credential-store分类、request mode和summary；
  - 但 `CODEX_FILE_PROJECTION_PRODUCTION_ENABLED=false`，没有生产 auth writer/readback；
  - allowed actions不含可执行的 `switch_to_official`。
- `codex_config/credential_store.rs`
  - 当前本地策略把 unset保持 fail-closed；
  - OpenAI当前第一方合约明确 unset默认 file，因此本任务需用固定源码测试修正 effective observation，而不是写入配置键。
- `codex_config.rs` / `auth.rs` / `storage.rs`
  - 已有 `toml_edit`、官方/第三方 live配置、config-only第三方路径、auth/config原子写与局部回滚；
  - request mode可从 `model_provider`观察；
  - 还缺 selected account identity、expected revisions、owner handoff和最终 connection一致性。
- `config.rs`
  - 已有 deterministic JSON、atomic write、rolling backup和Windows ReplaceFile；
  - 应扩展现有 owner，不新建第二套文件框架。

### Provider

- `services/provider/mod.rs`
  - 每应用 mutation guard；
  - proxy takeover/official安全限制；
  - current Provider backfill、device/DB current、live write、MCP同步；
  - 内建 `codex-official` 已存在。

**关键顺序事实：** `switch_normal` 会在写目标前读取 live并回填当前 Provider。第三方 API key可能位于 `auth.json`。若 Managed Auth先覆盖 auth再调用 ProviderService，回填会读不到旧 key，造成不可恢复丢失。因此 credential-aware writer必须在 Provider owner内复用/提炼回填步骤，不能简单“先写 auth、再 switch”。

### Change Plan

- `services/change_plan/*` 与 `.trellis/spec/backend/change-plan-executor.md`
  - 已有 Codex Provider switch typed adapter；
  - zero-write plan、digest、single guard、五阶段执行、idempotency、cancel、durable event、partial和crash recovery；
  - public ledger禁止 secret/path/arbitrary config；
  - 当前 credential-neutral adapter明确拒绝 managed-account auth并返回 `SecretDependencyUnavailable`；
  - `apply_change_plan` 已在 `spawn_blocking` 中调用同步 writer；OpenAI refresh 是 async；
  - `ManagedAuthState` 与 `AppState` 当前由Tauri分别管理。

结论：本任务应增加 credential-aware closed operation/resource set，由command层同时取得两个state；复用blocking apply线程桥接bounded async refresh，不应在 Managed Auth另造 coordinator状态机，也不应在async worker嵌套运行时。

### V2

- `pages/auth/ConnectionsView.tsx` 已区分账号连接、请求来源、官方会话保留和credential manager；
- wire/presentation已有 `switch_to_official`表达；
- 仍需让后端真实广告/执行，并把 saved credential与native connected彻底分开。

## 2. 已归档任务关系

`.trellis/tasks/archive/2026-09/09-03-openai-codex-managed-auth/` 已完成 OAuth、用途隔离、第三方保留等基础；未完成项是 matching-host HIL，且当时决策要求 file/keyring projection保持关闭。

本任务是后续产品决策修订：用固定第一方合约、自动化读回、权限、revision、回填、单一owner和Change Plan recovery替代 blanket HIL发布门槛。

## 3. 关键工程结论

1. 不能只翻转常量：后面没有完整 projection transaction。
2. 第三方 Provider已有唯一 owner：Managed Auth不复制 TOML/API key管理。
3. unset在第一方当前合约中默认 file：允许有效 file能力，但不改用户配置。
4. 显式 auto/keyring/ephemeral不能猜测或静默改为 file。
5. Secret bundle可能只有 refresh：refresh也不保证一定返回 ID token，无法形成完整文档时必须 reauth。
6. 切换前必须回收 live当前账号的最新 refresh token，并完成 old/new owner handoff。
7. 当前第三方 API key必须先通过 Provider backfill证明可恢复，才能覆盖 auth。
8. connected必须来自 live identity；metadata/credential presence不是真值。
9. Change Plan已经拥有 durable transaction，不应再造轮子。
10. Grok/OpenCode有不同 store/lock/hot-reload合约，不能机械同时移除门控。

## 4. 主要实施文件

- `src-tauri/src/services/managed_auth/consumers/codex*`
- `src-tauri/src/services/managed_auth/login.rs`
- `src-tauri/src/services/managed_auth/service.rs`
- `src-tauri/src/services/managed_auth/repository.rs`
- `src-tauri/src/services/managed_auth/secret_bundle.rs`
- `src-tauri/src/services/managed_auth/providers/openai.rs`
- `src-tauri/src/services/change_plan/{domain,adapter,service}.rs`
- `src-tauri/src/commands/{change_plan,managed_auth}.rs`
- `src-tauri/src/codex_config.rs`
- `src-tauri/src/codex_config/{credential_store,auth,storage}.rs`
- `src-tauri/src/services/provider/mod.rs`
- `src-tauri/src/config.rs`
- `src/v2/shared/features/{managed-auth,change-plans}.ts`
- `src/v2/shared/platform/tauri/feature-ports/*`
- `src/v2/pages/auth/**`
