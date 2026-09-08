# Issue #141 独立审阅

日期：2026-09-08。工作树：`独立修复工作树`；分支：`codex/issue-141-prompt-memory-closeout`；基线：`2f264d2f89326601a33f610c72a9f0143306d066`。

## Findings (fixed)

- File: `src-tauri/src/services/prompt.rs`
- Issue: 显式导入仍使用秒级 `imported-{timestamp}` 主键。同一时间值下导入、启用、再次导入会覆盖原启用项，随后 `was_enabled` 分支清空 live 文件并轮转恢复副本，未满足“导入只更新库”的要求。固定时间值回归真实失败，文件快照显示 `CLAUDE.md` 变为空字节。
- Fix: 经主控确认，复用已采用的 `uuid::Uuid::new_v4()` 为每次显式导入生成新的 opaque ID，保持 `imported-` 前缀，不迁移旧 ID。公共入口委托同一个私有 `import_from_file_at`，仅注入时间供确定性回归使用；没有第二条写入路径或新依赖。新增回归在固定 timestamp=1000 下验证两次导入身份独立、原条目仍启用、新条目停用、两项内容和时间正确，live 与 backup/undo 的内容、mtime、权限均不变。

## Findings (not fixed)

本次两项 P1 的审阅范围内未保留未修复缺陷。完整仓库门禁与本分支构建的原生 GUI 验收由主控继续执行；本审阅不据单测宣称原生、Windows、主线或正式发布通过。

## 审阅范围与结论

- 完整核对两份 Rust diff 及测试、当前 `.trellis/spec/frontend/prompts-memory.md`、维护文档 `docs/fyagent/development/configuration/prompts-memory.md` 和索引，以及任务 PRD、design、implement、research、初版实现记录。
- Prompt 的旧状态查询以 app 和真实持久化 `prompt.id` 为准，与 DAO 的主键/保存路径一致。disabled 新建、编辑、导入和删除不进入 live writer；真实 enabled→disabled、enabled 内容保存、存在其他 enabled 项时的行为和禁止删除 enabled 项保持。
- Daily 的 ASCII 正则先于字节切片，随后复用 chrono 校验真实公历日期；四位年份和 year>=1 与 renderer 的 0001–9999 日期域一致。list/search 在元数据/内容读取前过滤非法名称，并跳过目录；直接 CRUD 先验名再 I/O。排序、预览、搜索片段与合法闰日路径保持。
- 核对 renderer `assertPromptId` 与仓库 `imported-` 引用：ID 作为 opaque string 传递，没有解析时间后缀的兼容依赖。Cargo 已采用 uuid v4，无 manifest/lock、schema、IPC、ACL 或平台配置变更，不需要模板同步。
- 使用测试 home 的用例均带现有 `serial_test` 锁，临时目录与内存 DB 隔离，guard 退出时恢复原测试 home 和 settings cache；纯日期验证不修改全局状态。未增加生产测试绕过或外部账号操作。
- 核对原 10 例日志：旧实现 6 失败/4 通过，初版修复后 10 通过，修复前后具体断言对应数据清空与目录过滤问题。初版源码摘要在审阅开始时与文件一致；新增碰撞修复单独保留红绿证据，没有重写原日志。

## Verification

- Lint: `mise run lint` 实际退出 0。初版 `mise run rust:clippy` 日志/回执退出 0；新增 UUID 修复后的最终 Rust Clippy 随主控完整门禁执行，不用旧日志代替。
- TypeCheck: `mise run typecheck` 实际退出 0；此后仅改 Rust 和文档，无 TypeScript 修改。
- Tests: 新增碰撞用例以固定时间输入运行 `mise run rust:test issue_141_same_timestamp_import`，旧秒级 ID 实现 0 通过/1 失败、mise 退出 1、Cargo 退出 101。改为 UUID 后，`mise run rust:test issue_141_` 最终 11 通过/0 失败、退出 0。
- Format: reviewer 新用例首次 `mise run rust:fmt:check` 提示两处换行；按 formatter 输出修正后重跑退出 0。`git diff --check` 退出 0。
- 主控原有 renderer/架构基线日志为 55 通过；审阅未重复该套件。最终全门禁、独立分支构建、隔离原生 GUI 与文件/DB 回读仍为主控剩余 gate。

证据目录：`本机外部证据包 fyagent-issue141-fix（2026-09-08）`。新增日志为 `reviewer-import-collision-red.log`、`reviewer-regression-green.log`，回执与最终摘要为 `reviewer-checks.json`。

最终产品/测试源码 SHA-256：

| File                                  | SHA-256                                                            |
| ------------------------------------- | ------------------------------------------------------------------ |
| `src-tauri/src/services/prompt.rs`    | `03c02bd9b9eaa9d8ea14ca85c5d000d8ecc6851f908840d042d41720f4083276` |
| `src-tauri/src/commands/workspace.rs` | `295ff4437897751e9e2f92c0e68a1a6717b395510355eb92c8b0cb865acb3b3e` |

审阅者仅修改 Prompt 碰撞修复与回归、本文和初版记录的历史标识；未提交、推送、发布或操作应用，未触碰共享交接工作树。
