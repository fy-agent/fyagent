# 原生写入与严格提取独立复核（Grok 4.7）

模型：本轮就是 Grok 4.7，没有换路由。范围：`migrate/native/{codex,opencode,hermes,gemini}.rs` 与 `migrate/extract/**`。对照 `writer-rework-report.md`、`strict-extraction-report.md`，以及 `evidence/native-continuation/writer-rework/`、`evidence/codex-native/`。未改生产代码，未跑会读用户历史或真实推理的测试，也未跑整包 cargo。`native/mod.rs`、restore、reconcile、model 仍在改，只标依赖。身份包的结论不重复。

## P0

没有发现。

## P1

同一个 Codex 会话 id 可以再发布一个 rollout。`restore` 复用调用方给的 `target_native_id`，文件名却带当前时间（`codex.rs:83-116`）。`persist_noclobber` 只避免覆盖这一条路径（`129-132`）。目录里已经有 `rollout-*-{id}.jsonl` 时，函数不查找。触发：第一次 `persist` 已经成功，之后再次进入 `restore` 且仍带这个 id。macOS 上目录 `sync_all` 失败时，文件已经在盘上，返回值却是 `Unresolved`（`134-139`），这次重入是可达的。影响：同一 `session_id` 有两份历史；`thread/read` 按 id 读，不能证明读到的是哪一份。不会覆盖旧文件的字节。最小修法：在 `record_native_id` 之后、创建文件之前，若该 id 的 rollout 已存在，返回 `Unresolved` 且不再写。是否允许重入由编排决定，本轮不审 reconcile 的现状。

## P2

1. Codex 在回执已经记下 id 之后，建目录和暂存失败仍返回 `ProvenNoSideEffect`（`codex.rs:103-127`）。`persist` 失败才是 `Unresolved`。触发：`record_native_id` 成功，随后 `create_dir_all` 或暂存写入失败。影响：若编排把 `ProvenNoSideEffect` 当成可以新开一次写入，会和上面的第二份 rollout 叠在一起。最小修法：`record_native_id` 成功之后的失败一律 `Unresolved`。编排如何消费这个枚举，依赖仍在改的 restore。
2. OpenCode 的存在检查和会 upsert 的 import 不是一件事。导入前用只读 SQL 看 `session.id`（`opencode.rs:75-81`、`267-286`），随后才 `import --pure`（`109-113`）。注释写明官方导入会 upsert。触发：检查通过之后、导入开始之前，同一 id 被写成会话。影响：第二次导入可以改写已有行，而不是停在「已存在」。单次调用在检查到已存在时会在记回执前失败，那条路径不写。最小修法：导入必须是创建失败即停止；不能靠事先的 `SELECT EXISTS`。
3. OpenCode 助手部分的 `ignored: true` 会进终稿。用户部分把 `synthetic` 和 `ignored` 都排除（`extract/rules/opencode.rs:263-266`）。助手部分只跳过 `synthetic`（`335-337`），`ignored` 只检查是不是布尔（`329-333`）。触发：一条助手消息只有一个 `ignored: true` 的 text part，且 `finish=stop`、`time.completed` 已有。影响：被原生标成忽略的文本成为 `assistantFinal`。最小修法：助手的 `ignored: true` 与 `synthetic` 同样只计数、不进入正文。
4. OpenCode、Hermes、Gemini 在原生调用还没开始时，回执写入失败返回 `Unresolved`（`opencode.rs:106-108`，`hermes.rs:136-138`，`gemini.rs:184-186`）。此时 importer、SDK `create_session`、`wx` 都没跑。触发：`record_native_id` 返回错误。影响：没有原生副作用，重试却被未知结果挡住。Codex 在同一点返回 `ProvenNoSideEffect`（`codex.rs:103-104`）。最小修法：三处与 Codex 对齐，仅在原生调用发出之后才用 `Unresolved`。会不会因此永久停住，取决于编排，不在本轮终审。

## 依赖，不是这四个写入器的终审

- `run_with_timeout` 把每个流截到 1MB，截断后仍可能以退出码 0 返回（`native/mod.rs:244`、`301-353`）。OpenCode/Hermes/Gemini 的读回要整段 stdout 能解析成一份 JSON，截断通常变成 `Blocked` 或错误，不会变成 `Visible`。上限和政策属于正在改的 runner。写入器没有单独证明「输出完整」。
- 正式 Windows 拒绝在四个 `restore`/`discover` 里都先调用 `user_cli_execution_blocked`。函数本体在 `mod.rs`。产品不做 Windows 真机验收；这条守卫还在。`mod.rs:281` 的进程组仍是 `cfg(unix)`，四个写入器自己的编排测试已是 `cfg(target_os = "macos")`。
- 共享注册表是否把七行能力都标成可写，不在这四个文件里。提取侧仍把 Gemini 原始历史、Claude、OpenClaw、Grok 列为缺口（`extract/rules/mod.rs:59-64`）。有四行写入器不等于七个都能写。

## 没有发现

- 版本和将要使用的可执行文件：Codex 在记 id 前对 `provider_cli_path` 的结果跑 `--version`，必须等于 `0.154.0`（`codex.rs:181-212`）。OpenCode 要求输出 trim 后就是 `1.18.30`（`opencode.rs:202-210`）。Hermes/Gemini 桥在写之前核对安装包版本，不对就 `versionMismatch`，并且不创建会话（`hermes.rs:31-33`、`gemini.rs:37-38`）。
- 正文、顺序、重复和终稿。Codex 读回只收单个 `userMessage` 文本块，以及 `phase=final_answer` 的 `agentMessage`；其它 item、空历史直接失败（`codex.rs:498-539`），不会把进度当成可见终稿。OpenCode 读回要求 `finish=stop`、`time.completed`、没有 error、恰好一个非 synthetic 文本 part，否则 `Blocked`（`opencode.rs:334-357`）。导出失败也是 `Blocked`，不是 `NotVisible`。Hermes 读回要求助手 `finish_reason=stop`，且没有 tool、`api_content`、显示覆盖（`hermes.rs:370-388`）。证据里的官方导出是四条原文加 stop，下一请求 `/v1/responses` 是这四条再加新用户句。空白、合并、末尾未答用户在写库前是 `bodyNotPreserved`。
- Gemini 不把原始历史推断成终稿。写前用安装包里的 `convertSessionToClientHistory` 对每个角色和文本做整表比较，变了就 `bodyNotPreserved` 且 `wx` 之前退出（`gemini.rs:51-102`）。`?literal question` 的证据是这个比较失败，不是另一套前缀表。读回找不到文件才是 `notVisible`；多文件是 `ambiguous`；投影失败进 `Blocked`。下一请求里的 `<session_context>` 是运行时另加的一条，五条原文仍单独保留。重复 id 是 `alreadyExists`，不追加。
- 预分配 id：OpenCode `ses_`、Hermes `fyagent_`、Gemini 带连字符的 UUID 都先校验；已存在则不写内容。请求 JSON 在临时文件里，桥是源码里的固定字面量。
- 提取不把超限文件当成完整会话。`read_json` 多读 1 字节再 `check_budget`（`extract/storage.rs:43-47`）。Codex 有 `final_answer` 事件却没有对应 response item 时整段拒绝，相同文本按次数计数（`extract/rules/codex.rs:198-245`）。Hermes 没有 CLI 版本字段，未闭合工具链和没有来源的运行时标记会挡住，不把 `api_content` 当成用户正文。

## 证据限制

原生证据是 macOS 上的合成 HOME 和本地 HTTP 400，不是模型回答，也不是 Windows。`writer-rework-report.md` 写明最后一次整模块 Rust 测试被无关的 `model.rs` 编译错误挡住，不能当成这四个写入器的最终通过记录。OpenCode 探针证明的是与 Rust 模式一致的导入，不是 Rust 编排进程直接调用了真 CLI。
