# Issue #141 最小实现研究

日期：2026-09-08  
基线：独立分支 `codex/issue-141-prompt-memory-closeout`，`origin/main` = `2f264d2f89326601a33f610c72a9f0143306d066`。本文件只做实现前研究，未修改产品代码、测试或 GitHub。

## 结论

两个 P1 都可以做成小范围修复：Prompt 修复应只改变“禁用态 upsert 是否触碰 live 文件”的判定；Daily Memory 修复应把同一个严格日期解析 helper 用到 list/search/read/write/delete。现有数据库、文件格式、IPC 命令和前端 port 不需要扩展。

## Prompt：问题、最小设计和边界

当前 [services/prompt.rs:28-58](../../../../src-tauri/src/services/prompt.rs) 先保存 DB，再按传入 `prompt.enabled` 分支。禁用分支在保存后重新查表；若查不到任何 enabled prompt，就清空 live 文件。因此新建 disabled prompt，或从 live 导入一个 disabled prompt，在当前没有 enabled prompt 时也会清空原 live 内容。`import_from_file` [146-172](../../../../src-tauri/src/services/prompt.rs) 固定构造 `enabled: false`，所以会触发同一问题。

建议在 `upsert_prompt` 保存前读取同 app、同 id 的旧记录，形成 `was_enabled`；保存后禁用分支只有在 `was_enabled == true` 且保存后没有其他 enabled prompt 时才清空 live。新建 disabled、disabled 编辑、disabled 导入均只写 DB，不碰 live。启用态仍写入目标 live 文件，保持当前同步语义。

关键边界：

- enabled → disabled：若没有其他 enabled 项，清空 live；若仍有其他 enabled 项，保留当前 live 语义，不把 disabled 内容写入 live。
- disabled 新建/编辑/导入：live 文件内容与 SHA/mtime 保持不变。
- enabled 内容更新：仍写入 live；不能因为“已有记录”而跳过同步。
- delete：现有 enabled 删除拒绝逻辑保持不变；disabled 删除只删 DB，不动 live。
- `enable_prompt` 的 live 回填、单 enabled 互斥、原子写入保持不变。
- `_id` 当前未使用；实现可用它查旧记录，但应以 `prompt.id` 与数据库实际 key 一致为前提，避免引入第二身份规则。

历史可复用：`adb868d0` 曾加入“全部禁用时清空 live”的逻辑，说明清空行为本身是已有产品语义；本次应收窄触发条件，不能回滚该语义。`d32ceb9b` 的 prompt live 回填优先级修复可作为 `enable_prompt` 不被误改的边界参考。

## Daily Memory：问题、最小设计和边界

当前 [commands/workspace.rs:33-43](../../../../src-tauri/src/commands/workspace.rs) 的 `validate_daily_memory_filename` 只用正则检查 `YYYY-MM-DD.md` 外形，不验证真实日期（如 `2026-02-31.md` 会通过）。更关键的是 `list_daily_memory_files` [59-114](../../../../src-tauri/src/commands/workspace.rs) 和 `search_daily_memory_files` [193-284](../../../../src-tauri/src/commands/workspace.rs) 只检查 `.md` 后缀，没有调用 helper，因此会把 `README.md` 等非日期文件混入列表/搜索。

建议把 helper 改为两步：先保留固定 ASCII 形状，再用 `chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")` 验证日历日期；返回统一的 invalid filename 错误。list/search 在读取 metadata 前调用该 helper，遇到非日期或非法日期文件直接跳过；read/write/delete 继续调用同一 helper。这样 list/read/write/search/delete 共享严格日期准入，且不会把 README 送入日期文件读取路径。

关键边界：

- 接受 `YYYY-MM-DD.md` 且日期真实存在，包括闰年 2 月 29 日。
- 拒绝 `README.md`、`2026-2-01.md`、`2026-02-31.md`、路径穿越、附加后缀和控制字符。
- list/search 对目录中的非法文件跳过，不因一个 README 使整页失败。
- read/write/delete 对非法 filename 仍返回错误；不能静默改写任意 Markdown。
- 排序继续按文件名日期倒序；不要恢复历史按 mtime 排序行为。
- `floor_char_boundary`/`ceil_char_boundary` 仅负责搜索片段 UTF-8 边界，不是日期校验替代品。

历史可复用：`d1bb4480` 引入日记文件管理和正则 helper；`75323085` 固定按日期文件名排序；`a8dbea13` 引入搜索。三者提供现有接口/排序/搜索结构，但都没有完整日历校验，因此应在原 helper 上收紧，不新增命令或文件格式。

## 现有测试与建议红→绿用例

已有证据：

- [src-tauri/tests/profile_roundtrip.rs:209,547,592](../../../../src-tauri/tests/profile_roundtrip.rs) 覆盖 `PromptService::enable_prompt` 在 profile 切换中的互斥状态，但没有覆盖 disabled 新建/导入保留 live。
- [src-tauri/src/deeplink/tests.rs:982](../../../../src-tauri/src/deeplink/tests.rs) 覆盖 Prompt 导入能到达 `PromptService`，不验证 live 文件保持。
- [tests/renderer/platform/featurePorts.test.ts:1400-1600](../../../../tests/renderer/platform/featurePorts.test.ts) 覆盖 prompt/memory 命令映射、合法日期 payload 和 renderer 侧非法资源拒绝；不覆盖 Rust 目录中 README/非法日期过滤。
- 本轮未发现针对 `PromptService::upsert_prompt` 或 `validate_daily_memory_filename` 的专门 Rust 单测；不应把“没有 test 名称”写成“功能不存在”。

最小回归矩阵：

1. Prompt：隔离 `AppState` + 临时 FYAGENT_TEST_HOME/live 文件，预置 live sentinel；调用 disabled 新建/`import_from_file`，断言 DB 有记录且 live 内容、SHA、mtime 不变。
2. Prompt：预置 enabled A 和 live；将 A 更新为 disabled，断言 live 清空；再预置 enabled B，禁用 A 时断言 live 按既有语义不被误清空。
3. Prompt：enabled A 内容更新，断言 live 更新；disabled 编辑/删除不改变 live；enabled 删除仍报错。
4. Daily：临时 memory 目录放入合法日期、闰日、README、`2026-02-31.md`；list/search 只返回合法真实日期文件，且按日期文件名倒序。
5. Daily：read/write/delete 对 README、非法日期、路径穿越均返回错误；合法日期继续成功。
6. Daily：保留 UTF-8 search snippet 用例，证明日期 helper 收紧不破坏现有字符边界逻辑。

## 重复工作与执行边界

本轮没有发现当前 open PR 可直接复用这两个修复；历史提交只提供局部语义参考。实现应限制在 `services/prompt.rs`、`commands/workspace.rs` 与对应最小测试文件，不改数据库 schema、IPC payload、renderer port、安装链或其他 Agent 的交接内容。主控后续负责本分支新构建应用的隔离 GUI/native 复验；Rust/renderer 测试通过不能替代该复验。

建议完成条件：红→绿回归矩阵通过；`git diff --check`；主控在隔离的本分支构建应用中确认 disabled 新建/导入不改 live、enabled/排序/删除语义未回归，并确认 Daily 页面面对 README 与非法日期文件不再失败。
