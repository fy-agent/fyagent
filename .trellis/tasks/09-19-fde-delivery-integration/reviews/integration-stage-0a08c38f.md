# FDE 最终组合阶段审查

对象：固定 `0a08c38f`；备份问题同时对照 `19220715` 至当前提交的修复。只读固定源码和定向测试，不复审整套既有领域，不运行全库或真实模型请求。路径均相对仓库。

结论：项目→包检查→长期证据→交接的组合已接通；有两项具体问题须在最终交付前修复。原备份阶段的两项问题在源码层已闭合，最终 native 重测由 root 统一提供。

## 必须修复

### P1：Codex 保存对象的模型检查可能将凭据发到非当前服务商

- 新调用位置：`src-tauri/src/services/fde_workspace.rs:171-178`。
- 被复用实现：`src-tauri/src/services/provider/mod.rs:6858-6875`。Codex 分支用正则取整个 TOML 第一处 `base_url`，不识别 `model_provider`，也不排除注释。
- 具体触发：保存配置声明 `model_provider = "active"`，但前面有 `[model_providers.old]` 的 URL，或 `# base_url = "https://old.example"` 注释；当前分支会用 old URL 搭配取出的 API key。`saved_model_probe` 随后向该 URL 发请求。即使 DB 代际稳定、返回模型名匹配，检查的也不是保存对象的实际当前路由，同时存在向错误服务商发送凭据的风险。
- 最小修复：Codex URL 复用现有 `codex_config::extract_codex_base_url`（`src-tauri/src/codex_config.rs:147-163`），其规则是活动 provider 节优先、仅允许顶层 fallback、忽略非活动节。缺失有效路由按现有 unavailable 关闭。增加 inactive 节排前、注释 URL 排前、active 路由缺失的定向回归；不需要新的解析器。

### P2：同一项目修订后的迟到检查响应可以覆盖新状态

- 组合位置：`src/app/ProjectsWorkspace.tsx:79-84` 按 projectId/revision 重建 EvidencePanel，但缓存仍仅以 projectId 为键。
- 写回位置：`src/shared/features/verification/VerificationPanel.tsx:83-94` 在 await 后没有实例存活/会话检查，旧实例仍可 `setQueryData`；组合中另一路 `src/app/ProjectsWorkspace.tsx:32-61` 的 recordLocalFixture 也会在 await 后直接写同一缓存。
- 具体触发：A rev1 检查响应在原生侧已生成而 IPC 迟到；用户在项目面板将 A 更新到 rev2，新验证实例已刷新显示旧记录 stale；随后旧 rev1 操作返回，旧实例重新把 rev1 snapshot 写入 A 的缓存，旧结果暂时显示 current，直到下一次刷新。只按 React key 重建不能阻止旧异步任务写共享 cache。项目 A/B 不同 key 的隔离本身没有这个问题。
- 最小修复：复用包面板已有的实例存活/会话隔离方式，在成功和失败的异步写回前丢弃已卸载/旧修订响应；组合 adapter 同样需要 projectId/revision 会话门槛。保留原生已落盘的事实，只拒绝过时 UI 写回。增加“rev2 新快照先到，rev1 旧操作后到”的定向用例，断言不会把旧记录重新显示为当前有效。

## 原备份问题闭合

- `backup.rs:784-799` 先校验规范触发器，再复制到内存候选，对候选完成 schema/migration/代际增长；第 810 行才复制候选到主库。最大代际失败不再触碰 live，选中的备份也不被更改。`fde_restore_tests.rs` 直接调用正式 restore 路径，检查主库 sentinel、代际及备份字节；第 46 行已改为断言封闭的 `projects_storage_unavailable`，符合 project DAO 的错误转换，不要求泄漏原始 SQL 错误。
- `backup.rs:1975` 已加入 generation=4 的 deleted-provider tombstone，修复新增表却断言一行的空 fixture。源码保留同步 local snapshot restore→advance→Backup 顺序（233-240）。实际完整同步/恢复重测仍由 root 负责。

## 已核对无新增阻断

- `store.rs:25-35` 只 compose 一组共享 Arc；`lib.rs` 将同一 ProjectsService 和 KitLibrary 注册到 Tauri，verification 从同一 AppState 读取。没有残留 production UnavailableKitReader 替代真实 owner。
- KitReader 调用 native `confirm_identity`；validate_kit 从真实项目绑定取 exact kit identity，调用 KitLibrary 的内置 validator，按闭合 fixture 枚举选择结果。请求不接收 renderer outcome、metrics、路径或脚本。
- VerificationService 检查请求 revision；receipt 的 kit/fixture/样本结构及 passed 语义一致才落盘，执行后重新读取依赖并标记运行中失效。负例 missing-field 保持 Failed，即便符合负例预期也不变成 Passed。
- resource_revision 只散列 native 身份、代际和状态。模型绑定要求一个 Matched provider、项目中保存的 model ID；provider 按 raw_id+agent_id 回读，无代际则不发送检查。代际比较能捕获 DB 修改和 A→B→A；上面的 Codex URL 选择错误是目前剩余的保存对象语义缺口。
- 两个 composition 原生用例的源码覆盖 A/B 项目隔离、baseline 20%/90%、负例、幂等、重新组合后读回、脱敏交接和改 revision 后 stale；未绑定包不能生成通过。此次审查没有重复运行这些测试，也不把它们扩称真实服务或桌面 picker 验收。
- schema 仍由单一 21→22 分支调用 project 与 verification 两个领域迁移后推进一次版本；fresh 也创建两域表。verification 三表与资源代际均在 sync skip/preserve 中；revocations/ledger 通过一项定向事务读回测试覆盖。没有新增第二个 schema version authority。
- ProjectsWorkspace 保存本机检查只发项目/修订/闭合 checker/fixture/runId；不把前端显示的计算结果提交为证据。服务端回读和导出仍保留 fixture 与客户验收的区别。

## 验证边界

本报告是固定提交源码与测试代码审查。没有修改产品、运行全库、调用真实模型、恢复用户数据库或执行跨设备同步。两项必修已即时通知 root；其后续代码和重测不能反向当成本固定提交已通过。
