# Issue #141 P1 实现结果

本文件保留实施初版及其 10 例红绿证据；独立审阅新增的同秒导入碰撞修复、最终 11 例回归和源码摘要见 [review.md](review.md)。下述“无生产 helper”与测试数量描述仅对应初版。

日期：2026-09-08。工作树：`独立修复工作树`；基线：`2f264d2f89326601a33f610c72a9f0143306d066`。

## 变更与复用

- `src-tauri/src/services/prompt.rs`：disabled upsert 在保存前按 app 和实际持久化主键 `prompt.id` 检查旧启用状态；仅真正 enabled→disabled 才进入原有清空分支。新建、编辑和导入 disabled 条目不触碰 live 文件与恢复历史。启用写入、显式停用、仍有其他 enabled 项时的保留行为和禁止删除 enabled 项的规则不变。
- `src-tauri/src/commands/workspace.rs`：原日期准入 helper 使用严格 ASCII 外形与已有 `chrono::NaiveDate` 验证真实公历日期，范围与 renderer 的 0001–9999 年一致；列表和搜索复用它。直接读写删仍先验证，排序和 UTF-8 片段逻辑不变。
- 测试就近加入这两个文件的 `#[cfg(test)]` 模块，共 10 项。使用 `FYAGENT_TEST_HOME`、临时目录、内存 DB 和现有 `serial_test`；测试 guard 在退出或断言失败时恢复原测试 home。不改 `HOME`。
- 非测试逻辑约 18 行新增、6 行替换，无新依赖、schema、IPC、生产 helper 或平行日期解析器。未改共享交接树，未进行 commit/push/PR/native app 操作。

## 红到绿证据

完整日志保留在外部证据目录 `本机外部证据包 fyagent-issue141-fix（2026-09-08）`，避免编译日志进入仓库。

| 检查                                   | 结果                                        | 外部证据                                                         |
| -------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------- |
| 修复前 `mise run rust:test issue_141_` | 4 通过、6 失败；mise 退出 1，Cargo 退出 101 | `rust-regression-red.log`、`rust-regression-red.result.json`     |
| 修复后相同命令                         | 10 通过、0 失败；退出 0                     | `rust-regression-green.log`、`rust-regression-green.result.json` |
| `mise run rust:fmt:check`              | 退出 0                                      | `implementation-checks.json` 与本轮工具回执                      |
| `git diff --check`                     | 退出 0                                      | `implementation-checks.json` 与本轮工具回执                      |
| `mise run rust:clippy`                 | 退出 0，所有 target kind 且拒绝 warnings    | `rust-clippy.log`、`rust-clippy.result.json`                     |

修复前失败的六项：disabled create、edit、import 的 live/恢复历史保留；Daily filename 的真实日期准入；Daily 混合目录过滤；非法 Daily CRUD 拒绝。Prompt 失败回读明确出现 live 空字节、mtime 变化、backup/undo 轮转；Daily 失败明确返回 README、Unicode 数字、非法日期及 0000 年文件。四项兼容用例在修复前已通过，修复后仍通过。

绿色用例还验证缺失 live 文件保持缺失、已有恢复副本和权限/mtime 不变、按真实主键停用、已启用条目删除拒绝、保留其他应用记录、合法闰日/世纪年边界、日记按日期倒序、大小与预览、大小写不敏感搜索和 UTF-8 片段、合法日记创建/更新/备份/幂等删除。源码 SHA-256 记录于 `implementation-checks.json`。

## 后续验证边界

- 本实现者的 Clippy 已通过；其他全量检查不重复运行，由主控的 `check:prearchive` 统一承担。
- 主控负责独立审阅、全门禁、本分支新构建的原生 GUI 隔离复验、维护文档及最终提交。
- 当前证据是本机源码回归，不等于原生 GUI、Windows、主线合入或正式发布。只修本次两个 P1，不关闭 #141 汇总票，也不自动恢复历史已丢失内容。
