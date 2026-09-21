# 后端返工轮接口（orchestrator/native 边界）

作者：backend rework（Cursor claude-opus-5-thinking-high）。本文件只描述**我这一轮实际已落盘**的接口，未落盘的一律写明。文件路径均相对 `src-tauri/src/`。

## 1. `session_manager/migrate/native/mod.rs` 新增/变更

### 1.1 `provider_cli_path`（probe 与 writer 的唯一可执行文件解析）

```rust
pub(crate) fn provider_cli_path(provider_id: &str) -> Option<PathBuf>;
pub(crate) fn provider_binary(provider_id: &str) -> Option<&'static str>; // grokbuild -> "grok"
```

- 解析顺序：**先 `PATH`，后安装器位置**（`~/.local/bin`、`~/.npm-global/bin`，以及 codex/opencode/hermes/grokbuild 各自的 `~/.<name>/bin`）。
- 这个顺序是刻意的：用户 `PATH` 上是 Gemini 0.59 时必须报"版本不支持"，不允许偷偷改用另一处 0.46。
- 只接受**绝对路径**候选；相对 `PATH` 条目在 probe（无 cwd）与写入（目标工作区 cwd）下会解析到不同文件。
- Windows 下每个目录按 `name.cmd` → `name.exe` → `name` 顺序取。
- 返回 `None` 只表示"没装"，**不表示 blocked**，blocked 由 1.2 单独回答。

`capability::probe_local_provider` 已改为用它解析，`detect_version` 的入参从 `binary: &str` 改成 `executable: &Path`。**writer 请一律改用 `provider_cli_path`**：`native/opencode.rs`、`hermes.rs`、`gemini.rs` 已在用；`native/codex.rs` 目前仍自建 `extras = [~/.npm-global/bin, ~/.local/bin]` 并用 `resolve_cli`（extras 优先于 PATH），与统一解析顺序不一致，**请 root 改为 `provider_cli_path("codex")`**，否则仍可能 probe 一个安装、写另一个安装。

### 1.2 Windows 正式版执行边界

```rust
pub(crate) fn user_cli_execution_blocked() -> Option<&'static str>; // Some(原因) 表示禁止执行
```

- 语义与 `services::tooling` 既有的 elevated 边界一致：正式 Windows 版禁止以管理员身份拉起用户自有 CLI，**包括 `--version`**。
- 已在 `run_with_timeout` 内部 spawn 之前拦截，返回 `RunOutcome::NotStarted`（什么都没跑，写入方可安全当作无副作用）。
- `capability::probe_local_provider` 在探测前单独判断，返回 `capabilityProbeFailed` 而不是"未安装"——CLI 可能装着，只是这个构建不许跑。
- `restore::open_restored_session` 同样在拉起终端前拒绝。
- **注意签名是 `Option<&'static str>` 不是 `bool`**：`opencode.rs` 用 `.is_some()` 是对的；`codex.rs` 一度按 `bool` 调用导致编译失败（现已由 root 修好）。

### 1.3 `run_with_timeout` 契约（全部 writer 依赖）

- 整个进程树有界：Unix 下子进程单独 `process_group(0)`，超时按进程组 `SIGKILL`（`libc` 在本 crate 只对 macOS 生效，与 `services::tooling::terminate_child_tree` 同构；其他平台只杀直接子进程，所以下面的读取也是有界的）。
- stdout/stderr 各自上限 1 MiB，后台线程读取。
- reader 用 `recv_timeout(2s)`，不做无限 `join`——孙进程仍持有管道时无限 join 会把整个 IPC 卡死。
- **`NotStarted` 只在 spawn 失败或 1.2 阻断时返回**。spawn 之后的任何异常（含 `try_wait` 失败）一律 `TimedOut`，因为此时已经不能排除部分写入。writer 请保持 `NotStarted → ProvenNoSideEffect`、其余 → `Unresolved` 的映射。
- 原因串只进日志（writer 普遍刻意丢弃 CLI 输出以免泄漏凭据）。

### 1.4 注册表

`writer_for` 现包含 `codex | opencode | hermes | gemini`。`native/mod.rs` 的注册表测试同步更新：注册的 writer 必须有非空 `verified_write_versions` 与 `provider_binary`。

### 1.5 nonce 路径现状

四个 writer 现在都预分配原生 ID（`ReconciliationModel::IdBeforeContent`），因此 `NativeRestoreInput::target_native_nonce`、`NonceCarrier::SessionSlug`、`ReconciliationModel::NonceLookup` 目前**无人构造**，已加 `#[allow(dead_code)]` 与理由注释保留（回执表仍存 nonce 列，留给无法预分配 ID 的 writer）。谁要改回 nonce 方案请先说一声。

## 2. `session_manager/migrate/restore.rs`

- `NativeRestoreInput.target_native_id` 填 `attempt.target_native_id.clone()`：**失败重试必须复用回执里已有的 ID**，不要新铸一个（DAO 不允许替换已有 ID）。
- 批量：`restore_package` 先构造全部 proposed attempt，一次 `ReceiptStore::claim_batch(proposed, &selected_snapshot_ids)`，再逐项 `execute_claim`。拆分为 `build_proposed_attempt`（纯函数）+ `execute_claim`。provider 不匹配 / `requestId` 非法 / 目录不存在 / 能力门全部在第一次原生写之前判完。
- `verify_native_readback` / `open_restored_session` / `reconcile_all` 均先 `require_same_store`（重新解析当前 store + 重跑写入版本门），再动任何东西。
- 读回失败（`Err`）保留既有 stage 只记错误；`Blocked` 同样保留；`Visible` 不会把已经更靠前的 stage 降级；`Mismatch`/`NotVisible`/`Ambiguous` 是新的反向证据，会覆盖。
- `open_restored_session` **不再产生 `targetOpened`**，也不再接受未读回的 `nativeWritten`。它只对 `stage.is_restored()` 的行拉起终端并返回 `true`。
- resume 不再是裸命令：用 `provider_cli_path` 的绝对路径 + 对应 store 环境变量（`CODEX_HOME` / `OPENCODE_DB`+`OPENCODE_CONFIG_DIR` / `HERMES_HOME` / `GEMINI_CLI_HOME`=`.gemini` 的父目录），单引号转义。Hermes 没有已验证的 resume 入口，直接显式拒绝而不是猜一个 `--resume`。

## 3. `commands/`

- `preview_session_migration` / `export_session_package` 已加 `State<'_, AppState>` 并把 `&Database` 作为 `export::*` 首参；前端 invoke 参数未变。
- `commands/session_migration_dialogs.rs` 的两个 picker 已注册（`commands/mod.rs` 用 `pub(crate) use`，因为命令是 `pub(crate)`），已进 `lib.rs` 的 `generate_handler!`、`permissions/session-migration.toml`，ACL 测试计数 428 → 430。

## 4. 已知跨包缺口（不在本轮权限内，需对应 owner 处理）

1. `native/codex.rs` 的可执行文件解析未走 `provider_cli_path`（见 1.1）。
2. `extract/rules/opencode.rs:214` 触发 `clippy::collapsible_match`，仓库 clippy 是 `-D warnings`，会挡住整轮门禁。
3. `model.rs` 里未被任何人使用的 `sanitize_stderr_tail` / `STDERR_TAIL_BYTES` 已删除（writer 一律不外泄 stderr）。若将来要填 `MigrationError::NativeImportFailed.stderr_tail`，必须自带脱敏。
4. OpenCode 的 next-turn 证据来自手搭 fixture 而非本 writer 产物，能力矩阵因此不标 `nextTurnRequestVerified`；要标请补一次经由本 writer 的抓包。
