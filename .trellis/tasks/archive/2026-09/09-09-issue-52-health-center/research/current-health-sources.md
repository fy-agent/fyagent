# #52 Health Center：current-main 事实来源

基线：worktree `codex/issue-52-health-center`，`11c13339`（与 `81e06aae` 文件树一致；只读源码映射；未运行真实凭据、CLI 或网络探测）。原则：Health Center 默认只组合本机只读观察；不要把“刷新/测试/登录/取模型”当无副作用检查。

## 可直接复用的来源

| #52 事实 | 当前 source / 精确入口 | 性质、时间与边界 |
|---|---|---|
| 版本/来源 | `src-tauri/src/commands/agent_install_readiness.rs:17-30`; `src-tauri/src/agent_install/mod.rs:81-130`; `src-tauri/src/agent_install/cli.rs:22-75`；`get_agent_install_readiness(agentId)`、`get_agent_installation_inventory(agentId,surface)` | CLI 观察返回 `local_version/latest_version/detected/runnable/update_supported`；desktop inventory 返回候选、scope/owner/package/evidence、local version。当前 inventory 的 `probe_cli` 仍经 `observe_cli -> tooling::get_tool_versions`，所以可能读远端 metadata；不能把现有 readiness/inventory facade 当默认无副作用刷新。结果没有统一 `checkedAt`，需 Health Center 自己在读取完成时标记采样时间。 |
| Helper / 安装冲突 | `src-tauri/src/agent_install/inventory.rs:111-163,165-180`; `src-tauri/src/agent_install/mod.rs:95-129,393-405`; `src-tauri/src/agent_install/desktop.rs:122-139` | `InstallationInventoryState::{NotObserved,Single,Multiple,Unknown,Unsupported}`；`Multiple` 自动要求 target selection，`Unknown` 不应显示已安装。候选有 `stable_key/revision/launch_eligible/update_eligible/reason_codes/evidence_codes/location_label`；不要显示路径原文，使用既有 location label/脱敏来源。 |
| 配置 | `src-tauri/src/commands/provider.rs:362-370,378-417`; `src/shared/features/models.ts:16-30`; `src/shared/platform/tauri/feature-ports/models.ts:336-365,483-495`；`get_provider_summary(app)` | 不能视为纯本机只读：调用 `ProviderService::current -> get_effective_current_provider`，stale selection 时会 `set_current_provider` 修复配置。返回 provider `id/name/modelId`、currentId、writeTargets；Public summary 不返回 base URL、headers、API key，不能凭它宣称 endpoint 已知。Health Center 应改用仅读取 metadata/current selector、不修复的独立 facade。 |
| secret 状态 | `src/shared/features/managed-auth.ts:157-190,213-221`; `src/shared/platform/tauri/feature-ports/managedAuth.ts:18-23`; `src-tauri/src/commands/managed_auth.rs:17-20`; `src-tauri/src/services/managed_auth/service.rs:988-1037` | `managed_auth_get_overview` 是 DB/repository + OS secret **元数据** overview；返回 account `health`, `lastAuthenticatedAt`, `reasonCodes`，connection `authStatus`, `credentialManager`, `officialSessionPreserved`, `pendingRestart`, `checkedAt`。源码 overview 不读 secret material、不 refresh token；绝不输出 account/token/secret 值。 |
| auth | `src/shared/platform/tauri/feature-ports/agentAuth.ts:11-21`; `src/shared/features/queries.ts:134-143`; `src-tauri/src/agent_install/auth_actions.rs:59-103,185-235,343-430`; `src-tauri/src/commands/agent_auth.rs:16-20`；`get_agent_auth_observation(agentId)` | 观察是 agent-specific：Codex `FyagentManaged`；Qoder/Trae/WorkBuddy `HandoffOnly`；Grok 检查工具可用性后 handoff；Claude 执行 bounded `claude auth status`；OpenCode 读本地 auth JSON；均返回 `authority/state/reasonCodes/checkedAt`。Claude/Grok 的 CLI 观察可能启动外部进程；Handoff 不等于已登录。 |
| model | 仅配置摘要：`ProviderSummary.modelId`（上表）。`src-tauri/src/commands/model_fetch.rs:90-114`; `src/shared/platform/tauri/feature-ports/models.ts:496-500` 是 `fetch_models_for_config(baseUrl,apiKey,...)` | `fetch_models_for_config` 发 GET `/v1/models`，携带 secret，可能联网，不能 Health Center 默认调用。OpenCode `get_opencode_models` (`commands/model_fetch.rs:18-60`) 执行 `opencode models`，可能刷新/读取 CLI 状态，也不能当纯本机无副作用检查。无配置/未支持时显示 unknown/not_supported。 |
| proxy | `src-tauri/src/commands/proxy.rs:44-75`; `src-tauri/src/proxy/types.rs:58-100`; `src-tauri/src/commands/global_proxy.rs:11-24,160-180`；`get_proxy_takeover_status`、`get_proxy_status`、`get_upstream_proxy_status` | `get_proxy_status` 为运行态内存只读，含 running、active/total/success/fail、current provider、`last_request_at`、targets；适合 proxy/recent request。`get_global_proxy_url` 是 DB 只读，但 URL 可能带凭据，必须 mask/只显示 enabled。`get_upstream_proxy_status` 返回当前是否启用及 URL，亦须脱敏。 |
| restart | `src/shared/features/change-plans.ts:60-82,114-166`; ChangeJobSnapshot 的 `restartRequirement`, `resultCode`, `updatedAt`; `src-tauri/src/commands/settings.rs:199-217` `restart_app` | 已有 apply job 是最可靠来源：`not_required/recommended/unknown`、pending/recovery/result 与时间字段。`restart_app` 是写/进程生命周期动作，Health Center 只能导航或显示 pending，不自动调用。Managed Auth connection 也有 `pendingRestart`。 |
| 最后真实请求 | `src-tauri/src/proxy/types.rs:83-86` 的 `ProxyStatus.last_request_at`；`src-tauri/src/commands/usage.rs:9-43` 的 DB 汇总入口；`src-tauri/src/services/usage_stats.rs:600,760` | Proxy `last_request_at` 是当前本地代理的最近请求时间；但 `usage_stats::get_request_logs` 路径可能调用 `maybe_backfill_log_costs` 写库，不能作为 Health 默认读取。可显示“有本地代理请求记录/无记录”，但不能推断 provider 认证成功或外部 Agent 真实使用；应接独立只读 latest-request DAO。 |

## “配置/endpoint/drift”应如何表达

- 配置状态优先用 `get_provider_summary` 的 currentId/provider modelId/writeTargets（`commands/provider.rs:378-417`）；endpoint 仅在既有安全 public DTO 有来源时显示“已配置/未暴露”，不要从 Provider 私有结构把 URL 或 key 透传到 Health Center。
- drift 可复用 Change Plan 的 `planDigest/baselineDigest` 与 job `resources/status/resultCode`（`shared/features/change-plans.ts:60-166`），以及 WorkBuddy `get_workbuddy_status` 的 `revision/backupExists/format`（`commands/workbuddy.rs:12-25`; `services/workbuddy/config.rs:82-94`; `shared/features/models.ts:55-68`）。没有既有 baseline/observed pair 时返回 `unknown`，不要自行写 DB 指纹。
- WorkBuddy status 是本机文件读取，但没有统一 `checkedAt`；调用完成时由 Health Center 加采样时间。其 `path/backupPath` 属敏感本机元数据，前端只显示 presence/format/revision 摘要。

## 默认刷新安全分级

**可批量纯本机只读（建议默认）**：agent catalog；managed-auth overview（DB + secret presence/status，不取 material）；`get_proxy_status` / `get_proxy_takeover_status` / `get_upstream_proxy_status`（内存/DB，URL 脱敏）；已存在 ChangeJobSnapshot/WorkBuddy status 的读取；`agent_auth` 仅在 Claude/OpenCode 路径明确标为 bounded local observation。安装、当前 provider、latest request 均应使用新的独立只读 facade：desktop 专用 inventory、tooling 本机文件存在扫描（不执行 `--version`）、仅 metadata current-selector 读取、latest-request DAO。

**不要默认批量调用**：`get_agent_install_readiness` 与当前 `get_agent_installation_inventory`（inventory 的 `probe_cli` 仍走 `observe_cli -> tooling::get_tool_versions`，可能调用远端 metadata）；`get_provider_summary`（stale selection 可能写 current provider）；usage summary/request-log 读取（`maybe_backfill_log_costs` 可能写库）；Claude `auth status`/Grok availability 会启动外部 CLI；`get_agent_auth_observation` 对非 handoff Agent 不是单纯文件读取。

**明确联网、可能刷新/收费或带秘密**：`fetch_models_for_config`、`fetch_workbuddy_models`、任何 balance/endpoint test、managed-auth login/refresh/connect/apply、启动或切换 login session。`managed_auth_get_overview` 本身不 refresh，但 UI 不应把 account `health=Ready` 解释成额度/真实请求成功。

**明确写入/改变状态**：`managed_auth_set_default_account`、remove/apply connection action；Provider mutations/apply Change Plan；`set_global_proxy_url`（DB 写后应用）；`restart_app`；session usage 的增量沉降会写 `proxy_request_logs`。这些只能作为用户触发的修复/动作，不属于健康总览刷新。

## 未覆盖/未知边界

当前没有一个统一 `HealthCenterSnapshot` 或统一 `checkedAt`；各来源时间字段不一致（managed auth/agent auth 有 checkedAt，proxy 有 lastRequestAt，job 有 created/updatedAt，inventory/Provider/WorkBuddy 没有）。Health Center 应保存本次聚合采样时间，并对每个字段保留 `unknown/not_supported/observer_unavailable`，不把缺失升级为 `Blocked`。`last real request` 只能来自 proxy/status 或独立只读 latest-request DAO；不能主动发请求证明，也不能使用会触发 cost backfill 的通用 usage 查询作为默认健康读取。
