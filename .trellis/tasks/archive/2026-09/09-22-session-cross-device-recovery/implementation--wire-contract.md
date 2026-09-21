> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# 实施公共合同

协调方锁定以下公开边界，执行者可补充内部实现；变化先在报告说明，跨包同步后调整。

- Rust `src-tauri/src/session_manager/migrate` 为功能域；现有扫描/展示不改语义。model.rs 按技术方案 §3 的最终修订定义：snapshotId、origin.originId、requestId 必备，默认 slot 不能用包原始hash。
- Frontend 域和严格解析归 `src/shared/features/session-migration.ts`，原生调用只在 `src/shared/platform/tauri/feature-ports/sessionMigration.ts`；页面通过 FeaturePorts.sessions 使用。UI Agent拥有以上及FeaturePorts接线。
- 现有 get_sessions/get_session_messages 可用于浏览，导出只走 strict preview。
- `preview_session_migration(providerId, sourcePath)` -> MigratableSession；`export_session_package(items: [{providerId,sourcePath}], targetPath)` -> {path, sessionCount, byteLen, packageFileDigest}。
- `read_session_package(path)` -> {package: SessionPackage, attempts: RestoreAttempt[]}。
- `probe_local_provider(providerId)` -> LocalProviderProbe（技术方案§3）；`get_release_capability_matrix()` -> ReleaseCapability[]。
- `restore_session_package(request: RestoreRequest)` -> RestoreAttempt[]；request字段 packagePath, requestId, snapshotIds[], targetProviderId, targetWorkspace, requestKind(defaultImport|saveAsNewCopy)。目标只能与来源provider对应，不把跨模型转换当恢复。
- `verify_native_readback(attemptId)` -> RestoreAttempt；`list_restore_attempts()` -> RestoreAttempt[]；`reconcile_restore_attempts()` -> RestoreAttempt[]。
- `record_user_attestation(attemptId, claimedStage, note?)` -> RestoreAttempt，仅用户自报栏。`open_restored_session(attemptId)` -> boolean：后端验证存储映射，生成原生目标ID命令，禁止包携带可执行字符串。打开只标targetOpened，不伪造continuation。
- 错误使用 {code, detail?}，code camelCase；readonly stage 按技术方案。Rust DTO不要让 renderer 写入验证结论。
- 包、来源、快照等最小DTO严格拒绝未知字段。最终答复确定性来源规则，用户正文原样，排除runtime注入。真正无答复可保留，任意歧义阻断；正文内路径代码不删改。
- 后端公开实现入口是 migrate 模块。各 provider writer 放 `migrate/native/<provider>.rs`。协调方独占 native/codex.rs 及相关Codex隔离研究，其余后端由Opus owner。后端先为Codex调用定义 NativeRestoreInput/NativeRestoreOutput 和 trait或简单函数接口，写 implementation/codex-integration-contract.md 交协调方；不要自行伪造Codex成功或永久禁用。
- 生产仅版本实证能力放行；七家是产品范围，未验证需具体原因与后续路径，不能用“返回不支持”代替应做实现。开发中不启动真实付费推理。
