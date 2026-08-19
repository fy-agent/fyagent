# 统一配置变更概要与详细设计

## 1. 设计结论

采用“薄共享编排 + 独立领域 adapter”。共享层只拥有 plan、approval、job snapshot、事件持久化和恢复调度；Provider 与 WorkBuddy adapter 拥有各自资源的 inspect、plan projection、precheck、apply、readback 和 compensate。

这不是通用事务引擎。首版只注册 `CodexProviderAdapter`，第二阶段注册 `WorkBuddyAdapter`；不提供动态资源 DSL、跨 adapter 原子提交或插件市场。

## 2. 现状证据

- Provider add/update/switch 仍由 IPC 直接执行 mutation：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:57](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:57)、[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:104](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:104)、[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:258](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/provider.rs:258)。
- `ProviderMutationResult` 只有业务值、bytes 变化、app 与 warning codes，没有 plan/job/readback：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/provider.rs:46](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/provider.rs:46)。
- Codex 当前只在 mutation 前后读取 live config bytes；后读失败时 mutation 可能已成功：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/provider/mod.rs:2894](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/provider/mod.rs:2894)。
- WorkBuddy IPC 已与 Provider/AppType 隔离：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/workbuddy.rs:39](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/commands/workbuddy.rs:39)。
- WorkBuddy status 已暴露非敏感 revision 与 backup 状态：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/types.rs:20](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/types.rs:20)。
- WorkBuddy 保存请求当前携带明文 API key、revision 与 overwrite token：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/types.rs:60](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/types.rs:60)。
- WorkBuddy 已有 revision drift 拒绝、覆盖预检、备份和原子替换：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:172](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:172)、[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:229](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:229)、[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:486](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:486)。
- 当前保存后直接用待写 bytes 计算 revision 并返回，未重新读取磁盘：[/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:236](/Users/serendipity/.codex/worktrees/c282/fyagent/src-tauri/src/services/workbuddy/config.rs:236)。
- WorkBuddy 前端目前会处理领域专属 overwrite 二次确认：[/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:299](/Users/serendipity/.codex/worktrees/c282/fyagent/src/components/workbuddy/WorkBuddyPage.tsx:299)。
- Provider 前端切换仍直接 mutation 并展示“切换成功”：[/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/query/mutations.ts:390](/Users/serendipity/.codex/worktrees/c282/fyagent/src/lib/query/mutations.ts:390)、[/Users/serendipity/.codex/worktrees/c282/fyagent/src/hooks/useProviderActions.ts:344](/Users/serendipity/.codex/worktrees/c282/fyagent/src/hooks/useProviderActions.ts:344)。

## 3. 组件边界

```text
Renderer
  ChangePlanPage / ChangeJobPage
          | typed IPC
ChangePlanCommand
  ChangeOrchestrator
    PlanStore + JobStore + EventJournal
    AdapterRegistry (closed enum, not dynamic plugins)
      CodexProviderAdapter -> existing ProviderService/writers
      WorkBuddyAdapter     -> existing workbuddy config service
    SecretResolver -> local SecretBackend
```

### 共享层拥有

- plan 身份、digest、TTL、baseline 绑定和一次 approval。
- job 状态机、step 顺序、取消边界、事件序号、脱敏 snapshot。
- adapter 调度、崩溃恢复、活动记录和前端订阅。
- 公共 evidence 状态：`applied`、`warning`、`failed`、`not_observed`。

### adapter 拥有

- 资源定位、baseline 计算、语义 diff、风险与恢复说明。
- 领域校验、调用既有 writer、真实 readback、逐资源结果与 compensate。
- Provider 的 DB/live config/current route/restart 语义。
- WorkBuddy 的 JSON schema/revision/unknown-field preservation/backup/restore。

### Renderer 拥有

- 未提交表单草稿和临时 secret 输入。
- plan 展示、一次确认动作、job snapshot 渲染。
- 不根据 toast、mutation 返回或本地缓存推导成功。

## 4. DTO 与状态机

DTO 使用 serde `camelCase`，所有枚举为可穷举 discriminated union。示意字段如下：

```rust
struct CreateChangePlanRequest {
    operation: ChangeOperation,
    input: ChangeInput, // CodexProviderInput | WorkBuddyInput
}

struct ChangePlan {
    plan_id: String,
    plan_digest: String,
    baseline_digest: String,
    expires_at: String,
    operation: ChangeOperation,
    summary: Vec<PlanChange>,
    risks: Vec<RiskCode>,
    restart_requirement: RestartRequirement,
    recovery: RecoverySummary,
}

struct ApplyChangePlanRequest {
    plan_id: String,
    plan_digest: String,
    approval: ApprovalToken,
}

struct ChangeJobSnapshot {
    job_id: String,
    plan_id: String,
    status: JobStatus,
    current_step: Option<JobStep>,
    resources: Vec<ResourceResult>,
    evidence: UsageEvidence,
    last_event_seq: u64,
}
```

`ChangeOperation` 首版为 `CodexProviderCreate | CodexProviderUpdate | CodexProviderSwitch`；WorkBuddy 接入时增加 `WorkBuddyModelsUpdate`。不使用任意字符串资源类型。

`JobStatus`：`planned -> queued -> running -> succeeded | warning | failed | cancelled`。崩溃恢复期间使用 `recovering`。`stale` 是 apply 前终态，不创建写入 job。

`JobStep`：`precheck | apply | readback | compensate`。每一步写入开始/完成事件，snapshot 是事件折叠后的最新视图。

`ResourceResult` 包含 `resourceKey`、`status`、`changed`、`errorCode`、`recoveryStatus`、`restartRequired`；不含绝对路径、原始配置和 digest 原值。

## 5. Plan、一次确认与 drift

1. `create_change_plan` 调用 adapter.inspect，计算领域 baseline 和脱敏语义 diff，全程无写入、无网络。
2. 后端保存不可变 plan payload；`planDigest` 覆盖 operation、脱敏输入、secretRef、baseline、风险和恢复策略。
3. 用户在共享预览页确认一次。approval 绑定 `planId + planDigest + baselineDigest`，短时有效且仅消费一次。
4. `apply_change_plan` 先重新 inspect；baseline 不同则返回 `stale`，不创建写入事件，前端要求重新预览。
5. WorkBuddy 不再向用户暴露 overwrite token。若覆盖已有 model ID 已在 plan 中展示，统一 approval 即为授权；adapter 内部可生成并立即消费兼容 capability，或在迁移完成后改为只接受已验证 approval context。

## 6. secretRef

- 表单中的 secret value 只提交给 SecretBackend 写入命令，返回不透明 `secretRef`。
- ChangeInput、plan、job 与事件只保存 `secretRef`；计划预览仅显示“已提供/沿用/清除”等投影。
- apply 时 adapter 通过 `SecretResolver` 临时解析，调用 writer 后立即释放，不进入 `Debug`、error、event 或 query cache。
- WorkBuddy 第三方文件确需明文 API key 时，由 adapter 在执行期写入；这不改变 FyAgent 自身只持有 secretRef 的边界。
- SecretBackend 不可用或 ref 失效在 precheck 阶段失败，不发生业务写入。

## 7. IPC 与前后端合同

新增命令：

- `create_change_plan(request) -> ChangePlan`
- `approve_change_plan(planId, planDigest) -> ApprovalToken`
- `apply_change_plan(request) -> ChangeJobSnapshot`
- `get_change_job(jobId) -> ChangeJobSnapshot`
- `list_recoverable_change_jobs() -> Vec<ChangeJobSummary>`
- Tauri event `change-job://updated` 只携带 `jobId + eventSeq`；前端收到后重新获取完整 snapshot，避免漏事件导致错误状态。

旧 Provider/WorkBuddy mutation 在迁移窗口保留给旧调用方，但新 UI 不直接调用。最终移除由单独兼容性任务决定。

错误使用稳定 code：`plan_expired`、`plan_stale`、`approval_invalid`、`secret_unavailable`、`precheck_failed`、`apply_failed`、`readback_mismatch`、`restore_failed`、`job_interrupted`。用户文案由前端本地化。

## 8. Provider partial 与回读

Codex Provider operation 可能影响 Provider DB、current route、`~/.codex/config.toml` 和 restart requirement。adapter 必须把它们列为固定资源序列，并记录每项是否尝试、是否改变、回读结果和恢复结果。

- `create`：保存 Provider 与“设为当前”是不同语义；首版 request 必须明确选择，不能由 add 默认值暗中决定。
- `update`：回读 DB 中目标 Provider；若当前 Provider 的 live projection 受影响，再回读 live config。
- `switch`：回读 current route、目标 Provider 和 live config，三者不一致即 `warning/failed`，不得显示整体成功。
- `liveConfigChanged` 仅作为 restart 提示输入，不作为 apply 成功证据。
- 已写 DB 但 live config 失败时返回 partial，并按 adapter 能力补偿；补偿失败必须保留逐资源事实。

## 9. WorkBuddy readback 与 restore

WorkBuddy apply 固定顺序：

1. 锁内重新读取并比较 revision。
2. 将确认前原始 bytes 原子写入 backup；缺失文件记录 `previous=missing`。
3. 使用现有原子替换写 primary。
4. 从 primary 重新读取 bytes，重新解析 schema，并核对目标模型、规范 URL、secret 写入策略、unknown fields 与预期 revision。
5. 核对成功才返回 applied。
6. 核对失败时，用确认前 bytes 原子恢复；原来缺失则安全删除本次创建文件。
7. 再次回读恢复后的真实状态。恢复成功返回 `failed + restored`；恢复失败返回 `failed + restore_failed`，保留 backup，不声称回滚完成。

远程 `/models` 获取仍是用户主动的独立辅助动作，不进入 plan/apply。

## 10. 事件、持久化与崩溃恢复

- 使用本机 SQLite 表保存 `change_plans`、`change_jobs`、`change_job_events`；敏感输入不入库。
- 每个 step 在副作用前写 `started`，副作用后写 `completed`，同事务更新 snapshot 与递增 eventSeq。
- job 对同一领域资源使用稳定 resource lock，避免 FyAgent 内部并发；外部并发由 baseline/revision 检测。
- 启动恢复只扫描非终态 job，不自动重放未知状态的写操作。
- 若 `apply started` 无完成事件，adapter 先 inspect/readback：已达到目标则继续完成；仍是 baseline 则标记 interrupted/failed；处于第三态则尝试领域 compensate 或标记 partial。
- 恢复结果必须可回读，不能仅依赖实时 Tauri event。
- 保留期和清理策略由后续实现任务确定；首版建议终态 job 保留 30 天，仅保存脱敏摘要。

## 11. 迁移策略

1. 先引入 DTO、store、orchestrator 与 Codex adapter，不改变旧页面。
2. 将 Codex 新建、编辑、切换 UI 切到新 IPC；保留 feature flag 便于回退到旧 UI，但不能让同一动作双写。
3. 稳定后接入 WorkBuddy adapter；统一预览取代专属 overwrite dialog。
4. 观察兼容窗口后，再单独决定旧 `*_with_result` 与 `save_workbuddy_models` IPC 是否下线。

回滚只切换调用入口，不删除 journal 或降级真实状态。已经启动的 job 由新 orchestrator 收敛后才能关闭 feature flag。

## 12. 风险与控制

- 范围膨胀：registry 使用封闭 enum，首版拒绝跨 adapter plan。
- secret 泄漏：DTO 类型不包含 value，日志默认 redacted，测试检查序列化。
- partial 被误报成功：成功必须满足 adapter readback predicate；UI 只渲染 snapshot。
- crash 后重复写：恢复先 inspect，不盲目 replay。
- WorkBuddy 外部编辑：revision drift 直接 stale，重新预览，不提供强制覆盖。
- 旧 API 与新 API 双写：迁移期每个 UI action 只能绑定一个 execution path。
- Provider writer 复杂：adapter 复用现有 ProviderService，不复制 mutation 逻辑。
- 过度承诺真实使用：统一固定 `usageEvidence=not_observed`，直到未来存在稳定机器观测合同。

## 13. 未决点

- plan、job 的终态保留期及用户清理入口。
- Provider partial 的最小补偿矩阵需结合现有 DB/live writer 再逐 operation 固化。
- WorkBuddy 首次创建后 readback 失败时，“删除新文件”的平台级安全实现需详细设计。
- SecretBackend 的具体 trait 与本机 keyring 实现由 `#35` 定义，本设计只消费其稳定合同。

