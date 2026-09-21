# Codex 原生路径独立复核（Grok 4.7）

范围：`src-tauri/src/session_manager/migrate/native/codex.rs`，以及它用到的 `native/mod.rs`、`model.rs`、`identity.rs`。证据是 `evidence/codex-native/result.json`、`capture-result.json`、`incomplete-result.json`，外加隔离目录里的 rollout 与 `capture-wire.json`。对照只读克隆 `codex-source` 的 `external-agent-migration/src/sessions/export.rs`、`protocol/src/protocol.rs`、`app-server-protocol/src/protocol/thread_history.rs`。未改生产源码，未跑真实模型。`writer_for` 已注册 `codex`。

## P0

没有发现。

## P1

没有发现。

## P2

1. 版本只写在声明里，发布前不核对本机 CLI。`verified_write_versions` 返回 `=0.154.0`（`codex.rs:34`），`session_meta.cli_version` 固定写成该常量（`codex.rs:250`）。`restore` 只检查二进制存在（`codex.rs:49`、`cli_path`）。最小修法：`record_native_id` 之前读本机版本，不一致就 `ProvenNoSideEffect`。其它版本会不会拒读或搞坏索引，没有运行证据，不记为崩溃。
2. 空白 assistant 被拒绝得比官方投影更宽。注释写的是空 assistant 与空白 user（`codex.rs:234-236`），代码对两种 kind 都用 `trim().is_empty()`（`codex.rs:237`）。官方 `handle_agent_message` 只丢 `message.is_empty()`（`thread_history.rs:489-492`）；`build_user_inputs` 才对 user 做 trim（`thread_history.rs:1488-1498`）。只有空格或换行的 final 官方会保留，这里在回执前就失败。最小修法：user 继续 trim；assistant 只拒绝 `is_empty()`。这是多拒绝，不是写坏。
3. 读回没有「目标说没有」这条结果。`verify_readback` 只产生 `Visible`、`Mismatch` 或 `Err`（`codex.rs:87-99`），超时、断开、缺 thread 都是 `Err`。`NotVisible` 没有出口。缺席样本没打过，所以不把它写成会误重试。最小修法：能证明 not-found 时返回 `NotVisible`；超时保持 `Err`，编排不得把它当成 `ProvenNoSideEffect` 再调 `restore`。`restore` 每次都 `Uuid::new_v4`（`codex.rs:54`），第二次调用会再发布一个新文件。

## 没有发现

- 写序和发布：编码失败（含空文本）在 `record_native_id` 之前返回，此时没有回执 ID，也没有 rollout（`codex.rs:57-60`、`237-238`）。回执成功后才建目录。同目录临时文件、`sync_all`、`persist_noclobber`；碰撞和目录 fsync 失败是 `Unresolved`（`codex.rs:77-82`），不覆盖已有 jsonl，也不把这次当成可盲重试的无副作用。persist 之前的创建/写入失败仍是 `ProvenNoSideEffect`，目标文件名上还没有这份 rollout。
- 读回与磁盘分开：只调 `thread/read` + `includeTurns`。`locate_by_nonce` 固定空向量，不按内容或时间扫目录。比较用 `content_digest`，只含 kind 与原文。
- 可见历史与证据一致。`result.json` 的 read、resume、restart_read 都是四条原文，phase 为 `final_answer`。`incomplete-result.json` 里连续 user / 缺 final 的 turn 状态是 `interrupted`，正文仍在。这和 `turn_aborted` / `reason: interrupted` 一致，也和官方 `handle_turn_aborted` 只改状态、不删 user item 一致。
- 下一轮模拟请求：`capture-wire.json` 中迁移正文的角色与文本顺序是 user、assistant、user、assistant，与四条原文一致，assistant 带 `phase: final_answer`。同一次请求里的 developer / environment 是 Codex 在 `turn/start` 时追加的运行时注入，不是编码器写进历史投影的。
- 旧的 `inject_items` → `content_item_kinds: ["unknown"]` 不约束这条路径。resume 之后，原始 response_item 仍是 `user.text` 与 `assistant.final_answer`，assistant 另有 `phase: final_answer`。提取规则对 user 看 kinds、对 assistant 看 phase，可以对上。未知 item（单测里的 `commandExecution`）整段拒绝，不拼接、不降级成正文。
- 未使用 `ThreadResumeParams.history`。恢复参数走现有 UUID 白名单。不写官方导入标记 `<EXTERNAL SESSION IMPORTED>`：那条 marker 的 phase 为空，会被本解码器拒绝并污染 digest。官方 `export.rs` 对缺 final 一律 `task_complete`；这里改成 `turn_aborted` 有 incomplete 证据，不是多余框架。
- 子进程：只读 `app-server --stdio`，20 秒超时，单帧 256MB、累计 512MB，stderr 丢弃。macOS 新进程组后 `SIGKILL`。正式 Windows 构建在写文件前就 `ProvenNoSideEffect`，不启动 CLI。`configure_shell_user_command` 只出现在非该构建的 Windows 分支。客户端没有第二套写入协议。

## 证据限制

- 原生 `thread/read` 没有空串、纯空白、CRLF 或 emoji。`unfinished\r\n…🪁` 和带首尾空白的 final 只在编码器单测里。
- read、resume、重启、模拟下一轮都在 macOS、隔离 `CODEX_HOME`、Codex 0.154.0，无真实推理。没有 Windows 非提权子进程、Linux 进程组、或官方 App 同时占用同一 sessions 目录的运行证据。
- `capture-result.json` 的 `history_role_order_exact` 会滤掉非期望对。逐字结论来自 `capture-wire.json`，不是那个布尔值。
- 没有「回执已写入 ID、文件尚未 persist」时 `thread/read` 的 not-found 样本。

## 请协调者转 Jev

空白 assistant final（`trim` 后为空，但不是 `""`）现在会被拒绝。官方 0.154.0 会把这种非空消息留在可见历史里。

- A. 维持现状：user 与 assistant 都按 trim 拒绝。少恢复一些会话，不会发布读回对不上的文件。
- B. 拆开条件：user 用 `trim().is_empty()`，assistant 只用 `is_empty()`。只有空格或换行的 final 可以发布。

建议 B。空字符串 final 继续在回执前拒绝；不要用占位文本冒充它。包里的 digest 仍把空 final 和缺失 final 分成两个身份。
