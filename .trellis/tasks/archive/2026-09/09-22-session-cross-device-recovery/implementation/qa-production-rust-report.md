# QA production Rust report

## 结果

执行模型：Cursor GPT-5.6 Sol High。

现有集成 harness 已删除全部 `#[path]` source include、本地 `session_manager` module tree、`codex_config` stub 与 `config` stub，改为：

`use fyagent_lib::migration_test_hooks::{extract, identity, model, package};`

实际 Cargo package 名为 `fyagent`，integration test 可导入的 `[lib] name` 为 `fyagent_lib`。因此 harness 现在只可能编译并调用 Cargo 构建出的生产 crate，不再测试源文件副本。

## 测试范围

- `src-tauri/tests/session_migration_model.rs` 保留 11 个 `session_migration_*` 测试。
- 覆盖闭集 schema、未知消息种类、正文 CRLF/Unicode/路径/代码保真、无 overwrite 请求、未知副作用禁止重放、重复 key/尾随文档、digest 篡改、Codex final-only 提取、未知 final/version fail-closed、连续/open user 顺序、同正文不同 origin 的 snapshot 隔离。
- fixture 均为 `tests/session-migration/fixtures` 下合成数据。
- 未新增 `session_migration_orchestration.rs`：生产 `restore.rs` 已有 counted fake writer 测试，覆盖同 request 单写、未决结果禁止重写、已证明无副作用后的同 ID 重试、并发单写、batch 预检、provider mismatch 与 readback 状态；外部重复测试不会增加证据。

## 实际命令与结果

命令：

`rtk mise run rust:test session_migration_`

结果：编译失败，退出码 1；0 个测试实际执行。

唯一编译错误：

```text
error[E0425]: cannot find function `with_local_state_dir` in module `identity`
  --> tests/session_migration_model.rs:21:15
note: found an item that was configured out
  --> src/session_manager/migrate/identity.rs:263:15
262 | #[cfg(test)]
263 | pub(crate) fn with_local_state_dir<...>
```

这次编译已经成功解析 `fyagent_lib::migration_test_hooks` 以及生产 `extract`、`identity`、`model`、`package` 模块；失败点仅为隔离状态目录 hook 不在 integration-test feature build 中。

## 精确接口阻塞

当前 `mise run rust:test` 使用：

`--features fyagent/test-hooks`

但 `identity::with_local_state_dir` 仍只有 `#[cfg(test)]` 且为 `pub(crate)`。integration test 把 library 当作依赖编译时不会设置 library 的 `cfg(test)`，所以该函数被裁掉。

Opus/backend 需要保持此 override 仅在测试面可用，例如：

1. identity 内部实现受 `#[cfg(any(test, feature = "test-hooks"))]` 控制；
2. 通过 `migration_test_hooks` 提供 public wrapper 或 feature-gated public re-export；
3. 默认/release feature 不包含 `test-hooks`，不扩大生产接口。

不能在 harness 中改用真实 app config 目录：提取器会创建/读取 installation identity，可能触碰用户状态。QA 因此没有绕过隔离、没有恢复 stub，也没有重复运行失败命令。

## 安全与验收边界

- 未读取用户会话、凭据或真实 provider store。
- 未启动 CLI、模型或付费推理。
- 未修改 `lib.rs`、module declarations、restore 实现或其他生产 Rust。
- 未运行 whole Rust suite；Windows UAT 按授权 waived。
- 11 个 integration tests 目前是“已迁移到真实 production hooks、编译被单一 hook 可见性阻塞”，不是通过。

## Release ownership

- Opus/backend：完成 `with_local_state_dir` 的 `test-hooks` 可见性后通知 root。
- Root：接口落地后仅需重跑 `rtk mise run rust:test session_migration_`；成功结果应实际显示 11 个该 harness 测试执行，再决定完整 Rust gate。
- QA：本报告与 harness 修改已完成，释放 `src-tauri/tests/session_migration_model.rs` 和 `qa-production-rust-*` 所有权，不提交 commit。
