# 资源代际阶段独立审查

审查对象：固定 `19220715`，相对 `e0175050`。只读该提交的源码和已有测试；另按 root 要求仅核对 `0fec6c75` 的 Skill guard。没有读取漂移工作区作为完成证据，没有修改产品或运行全库。

结论：代际设计可继续集成，以下两项需闭合；尚不能把备份/恢复阶段标为已通过。Skill 判定缺口已在后续固定提交修复。

## 必须修复

### P2：二进制恢复的新失败步骤发生在主库替换之后

- 位置：`src-tauri/src/database/backup.rs:778-792`，尤其新调用第 792 行；代际范围和增长在 `src-tauri/src/database/dao/projects.rs` 的 `create_project_resource_generations_on_conn`、`advance_project_resource_generations_on_conn`。
- 具体失败：候选备份中的 generation 为 schema 合法最大值 `9007199254740990` 时，trigger 审查可通过，Backup 已把候选复制到主库；随后 `generation+1` 违反 CHECK，方法返回错误但先前主库已被替换。固定提交自己的 exhaustion 用例也明确该增长必须失败。这违反 persistence contract 中恢复失败保持/恢复原主库的要求。
- 最小补救：在候选的临时副本上完成迁移、代际增长和验证，再由 Backup 替换主库；不能通过饱和计数、忽略错误或删除 tombstone 使旧证据重新 current。增加合法最大代际候选恢复失败、原主库 sentinel 保留的用例。
- 证据：直接从固定提交提取 generation DDL 与增长 SQL，用 Python SQLite 的两个内存连接和 backup API 重现相同次序。输出：`post-copy advance: CHECK constraint failure`，`live value after failed restore stage: backup`，`prior live value retained: False`。这是隔离 SQLite 次序复现，不冒称已执行 Rust `restore_from_backup`。

### P2：新增同步表使已有测试必然失败，且尚未证明 preserve→advance 的运行次序

- 位置：`src-tauri/src/database/backup.rs:1919-1942`。
- 具体失败：第 1924 行只为原四个项目表各写入一行；第 1930 行新把 `fde_resource_generations` 加入统一 tables；第 1937-1942 行却对每张表断言一行。该 fixture 没有 providers/prompts/MCP/skills 源表，也没有代际插入，故新表在 source/target 都是零行。
- 最小补救：插入一个真实资源代际/tombstone，验证完整同步路径确实先恢复本机代际，再增长，且远端不能覆盖本机代际；验证被保留项目的旧 pinned generation 不再匹配。保留零文件访问的现有约束。不要删掉新表或放宽成只查表存在。

## 后续已修复

固定 `19220715` 的 `resources.rs:72-84` 为 Skill 提供数据库版本，但当时 `projects/mod.rs:350` 将相等版本统一标 Matched，无法代表磁盘 Skill 内容。实现 handoff 已识别此边界。已只读核对后续 `0fec6c75`：相等版本且 `ResourceKind::Skill` 返回 Unverifiable，其余相等资源返回 Matched，版本变化仍 Drifted。此项闭合，不要求重新实现文件哈希框架。

## 无新增阻断的核对项

- 计数表仅保存 kind/app/resource identity 和整数 generation，无配置内容、秘密或秘密裸哈希。源表写入与触发器增长在同一 SQLite 操作中；DELETE 保留 tombstone，rekey 增长新旧身份，A→B→A 不能恢复旧代际；自定义 endpoint 的增删改会增长所属 provider。
- `project_resource_version` 是 SELECT，resource options/snapshot 不负责补写版本。五个新增定向测试涵盖 owner 修改、ABA、删除重插、app scope、endpoint、schema21 seed、savepoint rollback、只读 total_changes 与 exhaustion，尚待 root 真正执行。
- fresh 与 21→22 都到达同一 project schema owner；seed 使用 INSERT OR IGNORE，重复创建不覆盖旧计数。没有额外版本推进分叉。
- 外部 SQL authorizer 仍拒绝所有 CREATE TRIGGER/CREATE TEMP TRIGGER。导出仅跳过规范 owned trigger，应用在外部 SQL 执行结束后重建；未知 trigger 仍在导出中保留并被导入拒绝，旧恶意凭据复制 trigger 测试的拒绝路径没有被绕开。
- 二进制 trigger 守卫比较完整规范 SQL，只归一化外围空白/末尾分号及 `CREATE TRIGGER IF NOT EXISTS`。没有按名称放行任意 body；需由 root 的后续备份定向测试补齐真实 owned-trigger 往返成功与同名变体拒绝证据。
- sync skip/preserve 均加入资源代际；源码次序为恢复本机 local snapshot 后 advance，再复制到主库。整个失败过程仍留在临时库，这部分未发现先污染 live 的新路径。上述同步测试修复后应验证实际读回，不能只用数组包含断言代替。

## 验证边界

本轮证据为固定源码审查、测试代码审查和一个隔离 SQLite 失败次序复现。没有运行 native 全库、真实用户数据库恢复、跨设备同步或 Windows；不把待冻结的 native 测试记为通过。报告仅拥有本文件，产品修复与最终测试由 root 完成。

## 审查后的定向回归交付

root 接受上述两项，另授权本 Agent 只新增 `src-tauri/src/database/fde_restore_tests.rs`，未修改 backup.rs。新测试 `binary_restore_generation_exhaustion_preserves_live_database` 创建正式 schema/owned triggers 的合法最大代际备份，调用真正的 `restore_from_backup`；要求 CHECK 失败、主库 sentinel 和代际保持原样、候选备份字节不变。已 rustfmt，按 root 要求未编译/执行。root 需在 backup.rs 的现有 `mod tests` 内增加 `mod fde_restore_tests { include!("fde_restore_tests.rs"); }`，复用该模块私有 TestHomeGuard。此回归文件是审查完成后的单独授权，不代表产品修复已经验证。
