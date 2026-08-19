# 统一配置变更实施计划

## 总体原则

每阶段独立提交、可回退、只扩大一层能力。先完成 Codex Provider 纵切，再接 WorkBuddy。下列命令是实施阶段的验证计划，本设计子任务按要求不运行测试。

## 阶段 0：合同冻结

- 定义共享 DTO、状态机、error codes、evidence 语义和 adapter trait。
- 定义 SecretBackend 消费边界，不实现云端存储。
- 为三个 Codex operation 写 fixture 和序列化合同测试。
- 禁止任意资源 DSL、跨 adapter plan 和网络 probe。

验证命令：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml change_plan_contract
rtk npm run type-check
```

提交边界：只包含 DTO、trait、序列化测试与前端类型，不接入现有 mutation。

## 阶段 1：PlanStore、JobStore 与 IPC 骨架

- 新增 SQLite migration：plans、jobs、events。
- 实现 plan digest、TTL、一次 approval、eventSeq 与完整 snapshot 回读。
- IPC event 只做失效通知，状态以 `get_change_job` 为准。
- 增加启动扫描非终态 job 的入口，暂不执行领域恢复。

验证命令：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml change_store
rtk cargo test --manifest-path src-tauri/Cargo.toml change_orchestrator
rtk npm run type-check
```

提交边界：持久化与 IPC 可独立存在，不改变 Provider/WorkBuddy 行为。

## 阶段 2：Codex Provider plan

- 实现 `CodexProviderAdapter.inspect/plan/precheck`。
- create/update/switch 明确 operation；create 的“保存”和“设为当前”不可由默认参数混合。
- 计划展示脱敏 diff、受影响资源、restart requirement 与恢复策略。
- baseline 覆盖 Provider DB/current route/live config 的领域摘要。
- 不联网、不触发 mutation。

验证命令：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml codex_change_plan
rtk cargo test --manifest-path src-tauri/Cargo.toml plan_has_no_side_effects
rtk npm run type-check
```

提交边界：可生成和展示 plan，但旧 mutation 仍是唯一执行入口。

## 阶段 3：Codex Provider apply/readback/partial

- adapter 复用现有 ProviderService writer。
- apply 前重检 baseline，stale 时零写入。
- 逐资源记录 DB/current route/live config 结果。
- readback predicate 决定成功；`liveConfigChanged` 只决定重启提示。
- 固化 create/update/switch 的补偿矩阵与 partial 表达。
- 启动恢复对 Codex 非终态 job 执行 inspect-first 收敛。

验证命令：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml codex_change_apply
rtk cargo test --manifest-path src-tauri/Cargo.toml codex_change_partial
rtk cargo test --manifest-path src-tauri/Cargo.toml change_job_recovery
```

提交边界：后端纵切完成，尚不切换用户入口。

## 阶段 4：Codex 统一前端体验

- 新增共享预览页、一次确认、执行进度、readback 结果和恢复状态组件。
- Codex 新建、编辑、切换接入新 IPC；一次 action 只走一条 execution path。
- UI 不根据 mutation promise 或 toast 推断成功。
- 所有完成页显示配置证据与 `usageEvidence=not_observed`。
- 通过 feature flag 保留可回退入口，不允许双写。

验证命令：

```bash
rtk npm run type-check
rtk npm test -- change-plan
rtk npm test -- provider
```

提交边界：只切 Codex 三种 operation；不接 additive 或其他 AppType。

## 阶段 5：WorkBuddy adapter 与真实回读恢复

- adapter 复用现有 revision、schema、backup 与原子写函数。
- plan 中展示覆盖已有 model ID 的语义；统一 approval 后不再弹第二次确认。
- apply 内部处理旧 overwrite capability 兼容，不让 token 穿过共享 UI。
- 写 primary 后重新读取并核对；不一致则恢复确认前 bytes，再回读恢复结果。
- revision drift 返回 stale 并重新预览，不支持强制覆盖。
- `/models` 获取保持独立用户动作，不进入 apply。

验证命令：

```bash
rtk cargo test --manifest-path src-tauri/Cargo.toml workbuddy_change_plan
rtk cargo test --manifest-path src-tauri/Cargo.toml workbuddy_readback
rtk cargo test --manifest-path src-tauri/Cargo.toml workbuddy_restore
rtk cargo test --manifest-path src-tauri/Cargo.toml workbuddy_revision_drift
```

提交边界：后端能力完成，旧 WorkBuddy 页面尚未切换。

## 阶段 6：WorkBuddy 前端迁移

- WorkBuddy 使用共享预览、一次确认、job 与结果组件。
- 移除页面层 `pendingOverwriteSave` 和专属 overwrite dialog 调用链。
- 临时 API key 先换取 secretRef；query cache 不存 secret value。
- stale、restored、restore_failed 使用稳定错误码和共享页面表达。

验证命令：

```bash
rtk npm run type-check
rtk npm test -- workbuddy
rtk npm test -- change-plan
```

提交边界：只迁移 WorkBuddy UI，不下线旧 IPC。

## 阶段 7：兼容收口与迁移清理

- 搜索并确认新 UI 不再调用旧直接 mutation。
- 评估旧 IPC 的版本兼容窗口，另提交删除或 deprecated 标记。
- 固化 job 保留期、清理策略和活动记录入口。
- 更新对应 backend/frontend spec 与用户文档。

验证命令：

```bash
rtk rg -n "add_provider_with_result|update_provider_with_result|switch_provider_with_result|save_workbuddy_models" src src-tauri
rtk cargo test --manifest-path src-tauri/Cargo.toml
rtk npm run type-check
rtk npm test
```

提交边界：兼容清理单独提交，可在不回滚已完成纵切的情况下延期。

## 跨阶段评审门

- 每阶段提交前检查 git diff 只包含该阶段范围。
- 后端成功必须有真实 readback 测试，不能只断言 writer 返回值。
- drift、partial、restore_failed、crash recovery 必须各有失败路径测试。
- IPC 序列化测试必须证明 secret value、绝对路径、原始配置不会出现在 DTO/event。
- UI 验收区分 code audit、runtime screenshot 与真实机器证据；前两者不等于生产验收。
- 任何阶段若需要动态事务 DSL、跨 adapter 原子性或主动联网，应停止并拆为新的产品决策，不在本任务扩 scope。

## 推荐提交序列

1. `feat(change-plan): define shared contracts`
2. `feat(change-plan): persist plans jobs and events`
3. `feat(provider): plan codex provider changes`
4. `feat(provider): apply and read back codex changes`
5. `feat(ui): route codex changes through change plans`
6. `feat(workbuddy): add readback and restore adapter`
7. `feat(ui): migrate workbuddy to shared change flow`
8. `refactor(change-plan): retire legacy mutation paths`

