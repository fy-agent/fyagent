# 旧数据库迁移兼容修复

结论：两个统一后端检查暴露的旧 schema 失败已修复；原测试保留并补强，没有放宽异常处理。此切片未提交。

## 根因与修改

启动和 SQL 导入均先执行 `create_tables_on_conn`，再执行有序 schema 迁移。新增资源代际初始化在第一步就对旧 `skills(key, installed, installed_at)` 表执行 `SELECT id`；旧表尚未经过 v1→v2→v3 的身份重建，因此报 `projects_storage_unavailable`。修复前单独运行原迁移测试，复现了同一 `create tables` 错误。

- `src-tauri/src/database/dao/projects.rs`：只对 schema 小于 3 且尚无 `skills.id` 的旧表，延后 Skill 代际 seed/trigger。已有 21→22 迁移在 Skills 重建之后正常补齐；新数据库、其他资源和当前 schema 不走此例外。
- `src-tauri/src/database/tests.rs`：原旧版本迁移测试保留，补查旧身份阶段不装错误 trigger、迁移后四个 Skill trigger、插入/更新推进代际、重开不重置代际。新增当前 schema 伪装旧 Skills 表仍失败的负例。
- `src-tauri/src/database/backup.rs`：仅强化原旧单行 SQL 导入测试，确认导入后 Provider 更新继续推进代际、Skill trigger 已补齐。

SQL 导入授权、临时库验证、事务、二进制 restore、sync 本地快照恢复、统一 advance、trigger 白名单均未改动；所有真实 SQLite 错误仍沿原路径返回。没有改动 UI、Provider 业务代码或 commands。

## 定向验证

执行目录：`~/.codex/worktrees/fyagent-fde-control/fyagent`。

| 检查命令 | 结果 |
| --- | --- |
| `rtk proxy mise run rust:test -- migration_from_v3_8_schema_v1_to_current_schema_v3` | 1/1 通过；修复前同一测试失败 |
| `rtk proxy mise run rust:test -- import_still_accepts_legacy_single_row_insert_exports` | 1/1 通过 |
| `rtk proxy mise run rust:test -- project_generations_reject_current_schema_with_legacy_skill_identity` | 1/1 通过 |
| `rtk proxy mise run rust:test -- projects_resource_generations` | 5/5 通过，覆盖首次 seed、事务回滚、ABA、删除重建、跨应用隔离、自定义端点、身份变更、代际耗尽及只读快照 |
| 三个改动 Rust 文件的 `rustfmt --check --edition 2021` | 通过 |
| 三个改动 Rust 文件的 `git diff --check` | 通过 |

日志位于 `~/fyagent/tmp/fde-workstreams-20260919/control-evidence/`：

- `legacy-migration-fixed.log`
- `legacy-import-fixed.log`
- `legacy-generation-fail-closed.log`
- `legacy-resource-generations-fixed.log`

首次尝试将两个过滤器一起传入任务入口，被入口的单过滤器限制拒绝，未运行测试；之后全部按独立过滤器执行。以上属于真实 native 定向测试，不等同于完整后端重验、桌面实操或发布验收；全体组合检查由 root 统一完成。
