> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Codex 原生写入集成合同（后端 → 协调方）

- 作者：后端执行者（Cursor claude-opus-5-thinking-high）
- 面向：协调方（独占 `src-tauri/src/session_manager/migrate/native/codex.rs` 与 Codex app-server 隔离研究）
- 状态：接口已在本分支落地并编译通过；`native/codex.rs` 未创建，由协调方提供。
- 依据：`deliverables/technical-design.md` §3/§5.1/§8/§9.1、`evidence/codex-probe/findings.md`、`implementation/wire-contract.md`

## 1. 你需要提供什么

一个文件 `src-tauri/src/session_manager/migrate/native/codex.rs`，导出一个实现
`super::NativeSessionWriter` 的零大小类型（建议命名 `CodexWriter`），外加
`native/mod.rs` 里两行注册（该文件由后端 owner 维护，已经预留注释位）：

```rust
// native/mod.rs 第 22 行附近
pub mod codex;

// native/mod.rs writer_for()
"codex" => Some(&codex::CodexWriter),
```

这两行是**唯一**需要改动后端 owner 文件的地方。请在你的报告里说明改动，
不要顺手改 `native/mod.rs` 的其他内容。

## 2. trait 定义（已落地，不要改签名）

来源：`src-tauri/src/session_manager/migrate/native/mod.rs`

```rust
pub struct NativeRestoreInput {
    pub attempt_id: String,
    pub session: MigratableSession,   // final-only，正文逐字
    pub target_workspace: PathBuf,    // 用户选择、已 canonicalize 的现存目录
    pub target_store_id: String,
    pub target_native_nonce: String,  // 写前预分配
}

pub enum NonceCarrier { NotUsed, SessionTitleSuffix, SessionSlug }

pub enum NativeWriteOutcome {
    Written { target_native_id: Option<String>, nonce_carrier: NonceCarrier },
    ProvenNoSideEffect { error: MigrationError },
    Unresolved { error: MigrationError },
}

pub enum ReadbackVerdict {
    Visible { observed_digest: String },
    Mismatch { observed: String },
    NotVisible,
    Blocked { reason: String },
    Ambiguous { candidates: Vec<String> },
}

pub trait NativeWriteContext {
    fn record_native_id(&self, native_id: &str) -> MigrationResult<()>;
}

pub trait NativeSessionWriter: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn write_strategy(&self) -> &'static str;
    fn verified_write_versions(&self) -> &'static [&'static str]; // 写入门槛，空=不放行
    fn resolve_target_store_id(&self) -> MigrationResult<String>;
    fn restore(&self, input: &NativeRestoreInput, context: &dyn NativeWriteContext)
        -> NativeWriteOutcome;
    fn verify_readback(&self, native_id: &str, expected: &MigratableSession)
        -> MigrationResult<ReadbackVerdict>;
    fn locate_by_nonce(&self, nonce: &str) -> MigrationResult<Vec<String>>;
    fn resume_argument(&self, native_id: &str) -> Option<String>; // 有默认实现
}
```

## 3. Codex 专属的硬约束

### 3.1 写序：ID 必须先落回执，再产生内容副作用

技术方案 §8.2 第 3 步。`restore()` 内必须是：

```
thread/start            -> thread_id
context.record_native_id(&thread_id)?     // 必须在这里，且返回 Err 就中止
thread/inject_items     -> 内容副作用
```

`record_native_id` 失败必须直接返回 `Unresolved`，**不要**继续注入。
`thread/start` 返回前崩溃是 `ambiguous`（编排层按回执里"pending 且无 ID"识别，
你不需要额外处理），**禁止盲重试**。

### 3.2 返回值到阶段的映射（编排层做，你只要如实报）

| 你的返回 | 编排层落库阶段 | 允许重试 |
|---|---|---|
| `Written` | `nativeWritten`，随后自动跑 `verify_readback` | — |
| `ProvenNoSideEffect` | `failed` | 允许幂等重试，复用同一行与 slot |
| `Unresolved` | `needsReconciliation` | **禁止**再次原生写入 |

只有能正面证明无副作用才用 `ProvenNoSideEffect`：可执行文件不存在、进程启动
失败、参数解析阶段即退出、协议在任何写方法发出之前断开。超时、被杀、
stderr 无法解析、退出码未知一律 `Unresolved`。

### 3.3 `verify_readback` 当前预期是 `Blocked`

`evidence/codex-probe/findings.md` 记录：`thread/read` 的 `turns` 为空、
`thread/items/list` 与 `thread/turns/list` 的 `data` 为空、legacy 模式
`items/list` 报 JSON-RPC `-32601`。因此在你找到可用展示通道之前，
`verify_readback` 应返回

```rust
ReadbackVerdict::Blocked { reason: "thread/items/list and thread/turns/list return empty; no verified visibility channel".into() }
```

而**不是** `NotVisible`。两者语义不同：`NotVisible` 是"目标权威地说没有"，
`Blocked` 是"这个版本没有可信读取通道"。编排层对 `Blocked` 保持
`nativeWritten`，不会晋升到 `nativeReadbackVerified`，也不会误报失败。

不要用 `scan_sessions()` 扫 rollout JSONL 来顶替 ③——那是我们自己读磁盘文件，
不是目标的展示通道。

### 3.4 `resolve_target_store_id`

`CODEX_HOME/installation_id`（该文件在隔离实测里存在）。读不到就返回
`MigrationError::TargetStoreUnidentified { provider_id: "codex" }`，
编排层会因此禁止一切原生写入，不会退化成"用路径当身份"。

### 3.5 注入内容

`input.session.messages` 已经是 final-only 的有序序列，`kind` 只有
`userText` / `assistantFinal`，正文逐字。不要再过滤、不要补齐成一问一答、
不要合并连续同角色消息。`open_user_messages` 在
`input.session.extraction.open_user_messages`，是派生事实，不要当错误。

### 3.6 禁止项

- 不用 `ThreadResumeParams.history`（schema 明确 `[UNSTABLE] FOR CODEX CLOUD - DO NOT USE`）。
- 不接受包里的任何命令字符串；拉起目标只能由 `resume_argument()` 从
  **目标返回的** ID 生成，走 `terminal::session_resume_argument` 白名单。
- 开发期不发起真实模型推理。

## 4. 已知往返退化（需要你给结论）

实测显示我们自己 `inject_items` 注入的消息，`content_item_kinds` 是
`["unknown"]`。而提取规则 `codex.rollout.phase-v1` 把 `["unknown"]` 判为
`Indeterminate`。后果：**由本功能恢复出来的 Codex thread 不能用本规则再次导出**。

需要你在 Codex 侧确认二选一并回报：

- A：`inject_items` 可以携带合法 kinds（如 `user.text` / final_answer），
  往返退化消除；
- B：不能，需要一条专门的 "FyAgent 迁移消息" 识别规则。

提取侧目前按 A 不成立处理（阻断）。若结论是 B，提取规则需要新增一个
版本号（`codex.rollout.phase-v2`）并补 fixture，由后端 owner 落地。

## 5. 后端已经就绪的部分

- 回执表 `session_restore_attempts`（schema v25）、slot 唯一约束、
  `device_binding` 行级失效、`attempt_count`。
- 编排 `migrate::restore`：写前 INSERT、pending 状态转移、崩溃点分类、
  `ProvenNoSideEffect` → `failed` 的幂等重试。
- `reconcile_restore_attempts`：有 ID 走 ID，无 ID 有 nonce 走
  `locate_by_nonce`，多候选 → `ambiguous`，0 候选 → 保持
  `needsReconciliation`（不是 `failed`）。
- 能力矩阵 `capability.rs`：Codex 的 `extraction_rule` 已实证
  （`=0.154.0`），`write_strategy` 为 `None`。你注册 writer 之后请把
  `write_strategy` 改成你验证出的标识，并把 `verified_stages` 按级填真值；
  在 ③ 由 blocked 变可解之前不要填 `nativeReadbackVerified`。

## 6. 我没有做、也不会替你做的

- 没有实现 stdio JSON-RPC 客户端（全仓 `app.?server|jsonrpc|json_rpc` 在
  `src-tauri/src` 零匹配，这一块仍是从零）。
- 没有伪造 Codex 成功，也没有把 Codex 永久标记为不支持。
- 没有运行任何真实 Codex 会话、没有读用户 `CODEX_HOME`。
