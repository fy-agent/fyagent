# 历史文本（工作站路径已匿名化）：evidence/native-continuation/writer-rework/rust-native-tests.log

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 该日志先前未被 Git 跟踪，原稿不能从来源提交恢复；原始字节单独保留在仓库外的本地 archive-original-logs 备份中，不纳入 PR。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：3564
- 原稿 SHA-256：`39db3804c9abd6e16981ae939692242c733b247e63e145d14582bf7a224970d4`
- 匿名化载荷字节数：3563
- 匿名化载荷 SHA-256：`5aaba0becb803b1dbc53c7bac48a1295a1a486789cdde71cdbf27fb1e6475c54`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
[rust:test] $ node scripts/tasks/rust.mjs test
   Compiling fyagent v0.4.6 (/Users/<username>/Documents/Codex/2026-09-22/new-chat/work/fyagent-session/src-tauri)
warning: function `extract_session` is never used
   --> src/session_manager/migrate/extract/mod.rs:100:8
    |
100 | pub fn extract_session(
    |        ^^^^^^^^^^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `sanitize_stderr_tail` is never used
   --> src/session_manager/migrate/model.rs:642:8
    |
642 | pub fn sanitize_stderr_tail(raw: &str) -> String {
    |        ^^^^^^^^^^^^^^^^^^^^

warning: function `keep_tail` is never used
   --> src/session_manager/migrate/model.rs:671:4
    |
671 | fn keep_tail(raw: &str) -> &str {
    |    ^^^^^^^^^

warning: constant `STDERR_TAIL_BYTES` is never used
  --> src/session_manager/migrate/model.rs:37:15
   |
37 |     pub const STDERR_TAIL_BYTES: usize = 2 * 1024;
   |               ^^^^^^^^^^^^^^^^^

warning: field `target_native_nonce` is never read
  --> src/session_manager/migrate/native/mod.rs:42:9
   |
33 | pub struct NativeRestoreInput {
   |            ------------------ field in this struct
...
42 |     pub target_native_nonce: String,
   |         ^^^^^^^^^^^^^^^^^^^
   |
   = note: `NativeRestoreInput` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: variant `SessionSlug` is never constructed
  --> src/session_manager/migrate/native/mod.rs:54:5
   |
50 | pub enum NonceCarrier {
   |          ------------ variant in this enum
...
54 |     SessionSlug,
   |     ^^^^^^^^^^^
   |
   = note: `NonceCarrier` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: variant `NonceLookup` is never constructed
   --> src/session_manager/migrate/native/mod.rs:113:5
    |
106 | pub enum ReconciliationModel {
    |          ------------------- variant in this enum
...
113 |     NonceLookup,
    |     ^^^^^^^^^^^
    |
    = note: `ReconciliationModel` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

warning: field `reason` is never read
   --> src/session_manager/migrate/native/mod.rs:219:18
    |
219 |     NotStarted { reason: String },
    |     ----------   ^^^^^^
    |     |
    |     field in this variant
    |
    = note: `RunOutcome` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: method `claim_batch` is never used
  --> src/session_manager/migrate/receipt.rs:69:12
   |
32 | impl<'a> ReceiptStore<'a> {
   | ------------------------- method in this implementation
...
69 |     pub fn claim_batch(&self, attempts: Vec<RestoreAttempt>, snapshot_ids: &[String]) -> MigrationResult<Vec<AttemptClaim>> {
   |            ^^^^^^^^^^^

warning: `fyagent` (lib) generated 9 warnings
warning: function `get_app_config_dir` is never used
  --> tests/session_migration_model.rs:53:12
   |
53 |     pub fn get_app_config_dir() -> PathBuf {
   |            ^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `fyagent` (test "session_migration_model") generated 1 warning
error[E0425]: cannot find function `sanitize_stderr_tail` in this scope
   --> src/session_manager/migrate/model.rs:846:25
    |
846 |         let sanitized = sanitize_stderr_tail(&raw);
    |                         ^^^^^^^^^^^^^^^^^^^^ not found in this scope

```
