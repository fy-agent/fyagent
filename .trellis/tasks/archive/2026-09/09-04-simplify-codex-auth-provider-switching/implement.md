# Implementation plan

> 状态：调研与规划完成，等待批准。当前不修改产品代码；收到明确实施指令后才执行
> `python ./.trellis/scripts/task.py start 09-04-simplify-codex-auth-provider-switching`。

## Phase 0：基线与失败测试

- [ ] 运行 Trellis context，确认 current task、branch、worktree 和并行任务无冲突。
- [ ] 对比实施时 OpenAI Codex HEAD 与任务固定 commit；schema/default/store/refresh
  语义变化先回到 planning。
- [ ] 建立失败测试：unset store、missing model_provider、credential-only伪 connected、
  gate 后无 writer、official→third-party auth byte-equality。
- [ ] 建立锁图和故障注入矩阵。
- [ ] 保留旧 production gate，直到安全 writer、readback、rollback 和状态测试完成；
  不先把常量改成 `true`。

## Phase 1：观察与最小 delta

- [ ] 实现单次 `CodexManagedAuthObservation`：config/auth revision、effective store、
  provider route、native auth identity、runtime state。
- [ ] 修正 unset `cli_auth_credentials_store` → file default。
- [ ] 修正 missing `model_provider` → openai default。
- [ ] 解析完整 ChatGPT、未知账号、legacy API-key-only、PAT、Bedrock、agent identity、
  invalid/oversized/unreadable。
- [ ] 实现 `Noop | AuthOnly | ProviderOnly | AuthThenProvider` 纯 delta planner。
- [ ] 修正 overview：credential presence 不等于 connected。
- [ ] 测试 no-op 零 refresh、零文件写、零 Provider 写、零 restart。

## Phase 2：第一方 auth adapter 与凭据物化

- [ ] 根据固定上游 fixture 实现 crate-private ChatGPT AuthDotJson adapter。
- [ ] 新 login grant 完整时直接物化，不重复 refresh。
- [ ] 历史 bundle 完整且可用时直接物化。
- [ ] 只有缺字段/不可用时复用现有 OpenAI refresh；rotation先 SecretRef CAS。
- [ ] refresh缺字段时只保留仍有效且身份匹配的旧值；仍不完整则 requires reauth。
- [ ] 实现当前 live native token回收与 refresh-owner handoff。
- [ ] 测试多账号切走再切回；若现有 SecretRef容量策略无法满足，停止并回到
  planning评审 existing SecretService component-secret manifest，禁止明文快照或
  提高全局上限。
- [ ] 测试 identity/workspace/forced-login、generation/owner冲突和 secret redaction。

## Phase 3：窄 `auth.json` 交换器

- [ ] 在 Codex consumer/codex_config owner 内实现 bounded observation、expected
  revision、exact preimage、atomic write、Unix `0600`、identity readback。
- [ ] 复用现有 `config::atomic_write`/Codex storage；不新建 tempfile/rename体系。
- [ ] 参考/窄化复用 OpenCode consumer 的 writer lock、revision、readback、rollback
  模式；只有双方真正共用才抽公共 helper。
- [ ] `AuthOnly` 不读写 config、Provider DB/device current、MCP。
- [ ] 回滚仅在 live revision仍属于本次 writer时执行；外部变化停止覆盖。
- [ ] 故障注入：write、permission、readback、rollback、external revision、并发 swap。

## Phase 4：复用 ProviderService 的最小组合

- [ ] Managed Auth command 同时取得 ManagedAuthState 与 AppState。
- [ ] 所有 Codex managed-auth mutation 获取现有 per-app Provider mutation guard。
- [ ] 在 ProviderService提炼 crate-private lock-held current-backfill/official-switch seam；
  普通 Provider switch继续复用同一核心步骤。
- [ ] `ProviderOnly`：auth byte-equality，只有 route/current/MCP 的既有变化。
- [ ] `AuthThenProvider`：legacy key预检/回填 → auth swap/readback → official switch/
  readback → owner/connection提交。
- [ ] Provider失败时按 live route/auth revision分类 target、baseline、safe-retry 或
  recovery；不新增 Change Plan operation/ledger。
- [ ] 保持 proxy takeover、official安全限制、forced login/workspace策略。
- [ ] official→third-party继续现有 Provider Change Plan，并补 auth byte-equality测试。

## Phase 5：动作、状态与 V2

- [ ] 接通 connect_account、switch_account、login connect_consumer 和
  switch_to_official，共用同一 delta/coordinator。
- [ ] 删除 blanket gate及只服务于“永远 unavailable”的分支/测试。
- [ ] allowed actions按 live capability生成；无明确connection账号不广告
  switch_to_official。
- [ ] 增加/复用 saved-not-projected、store unsupported、requires reauth、unmanaged/
  external、pending restart、recovery reason。
- [ ] 前端只增加准确状态、确认和 Provider页 deep-link；不增加第三方配置表单或
  Change Plan job UI。
- [ ] mutation 后刷新 authoritative overview，不 optimistic success。
- [ ] 复用 Codex Desktop restart coordinator；不新增进程控制。

## Phase 6：验证、Spec 与归档

- [ ] 更新 `.trellis/spec/backend/managed-auth-consumers.md`。
- [ ] 更新 `.trellis/spec/backend/managed-auth.md`、`managed-auth-login.md`。
- [ ] 更新 `.trellis/spec/backend/codex-provider-configuration.md`。
- [ ] 如 Provider internal seam事实变化，更新 `.trellis/spec/backend/modular-boundaries.md`
  与 `reuse.md`；Change Plan公共spec原则上不变。
- [ ] 更新 `.trellis/spec/frontend/v2-managed-auth.md` 与相关测试规范。
- [ ] 删除 blanket HIL gate描述，把 HIL标为非阻断版本/平台 smoke evidence。
- [ ] 运行 focused tests，再按实际 `mise tasks` 核实并运行完整门禁，至少覆盖：

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run test:unit -- tests/architecture/rustModuleBoundaries.test.ts
mise run check:contracts
```

- [ ] 搜索 token/code/verifier/SecretRef/path/raw auth/TOML 是否进入 DTO、log、event、
  snapshot或DOM。
- [ ] 两轮独立 review：①最小写入/Provider复用/rollback；②identity/refresh owner/
  SecretRef容量/V2状态。
- [ ] 归档前更新 Spec、记录自动化证据、已知限制和可选 HIL smoke。

## Stop conditions

出现以下任一情况立即回到 planning：

- 上游 AuthDotJson/default/store/refresh契约发生实质变化；
- 多账号切回无法由现有 SecretRef安全保留/重建完整材料；
- ProviderService无法提供不复制逻辑的 lock-held seam；
- legacy第三方 API key无法在 auth覆盖前证明可恢复；
- 需要新 Change Plan operation、第二套 Provider writer/secret store/事务框架；
- 需要新增依赖但未完成 license、维护、安全、平台、MSRV/体积和重复栈评审；
- live authority无法把中间态分类为目标、基线或可安全重试状态；
- 目标平台无法满足安全原子替换和 owner-only权限。
