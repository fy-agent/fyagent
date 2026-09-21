# Grok 4.7 最终架构评审

- 任务：`.trellis/tasks/09-22-session-cross-device-recovery`
- 只读评审。未改作者文件，未跑真实会话，未调用 Jev。有限选择已由本轮证据裁定。
- 基线：`design.md`、`prd.md`、`deliverables/technical-design.md`、`test-plan.md`、`test-cases.json`、`ux-spec.md`、`session-prototype.html`、`evidence/codex-probe/findings.md`、`decisions/restore-gate-response.json`。`redundancy-review.md` 只作 4.6 历史意见。
- 裁定：设计方向可继续，当前稿不能冻结。下面严重项未改前，不接受“六家 positional”“Codex 已可恢复”或“35 条已通过”。

## 严重问题

1. **Codex 最终答复不是 positional。** `technical-design.md` §6.5 写“磁盘 rollout 里无我们读到的完成字段”，§6.5 结论把除 OpenCode 外六家都标成 Positional，§6.3 用“下一条 user 前最后一条 assistant”。这三处与证据冲突。
   - Schema：`evidence/codex-schema/v2/ThreadItemsListResponse.json` 的 `MessagePhase` 只有 `commentary`（中途播报，后面还可能有工具或输出）和 `final_answer`（本轮终端正文）。`phase` 可空；空值定义就是 phase unknown，不是“取最后一条”。
   - 持久化：隔离 rollout `.../rollout-2026-09-22T01-16-15-01a0c4f7-e678-7063-8763-390ed30a27e1.jsonl` 的 assistant 行带 `"phase":"final_answer"`，并带 `internal_chat_message_metadata_passthrough.content_item_kinds`。注入的四条原文被运行时标成 `["unknown"]`；随后真实下一问是 `["user.text"]`。
   - 下轮请求：`request-capture/captured-request.json` 在开发者规则与环境上下文之后，按 user/assistant/user/assistant 保留四条原文，两条 assistant 均有 `phase=final_answer`。端点是本机模拟，`model_called=false`。
   - 现有 reader `src-tauri/src/session_manager/providers/codex.rs:220-266` 只留 `role/content/ts`，丢掉 `phase` 和 `content_item_kinds`。reader 丢字段不能证明原始记录没有这些字段。
   - `content_item_kinds=unknown` 不能当干净正文，也不能携带工具或进度。没有 `phase=final_answer` 时不得改走 positional。

2. **技术方案漏掉 Codex 展示缺口，并把已有探针写成“从未验证”。** §9.2 写构造从未验证、①–⑥全未验证。`findings.md` 与 `request-capture/result.json` 已经分开：
   - 有证据：`thread/start` 得到 `thread_id=01a0c4f7-e678-7063-8763-390ed30a27e1`；`inject` 返回 `{}`；四条原文落盘；重启后 `thread/resume` 成功；模拟端点请求保留角色、顺序和 `final_answer`。
   - 无证据：`thread/read` 的 `turns=[]` 且 `historyMode=paginated`；重启后 `items/list` 与 `turns/list` 的 `data=[]`；`turn/start.itemsView=notLoaded`。没有桌面 UI、真实模型回复、Mac/Windows。
   - `scan_sessions` 读 JSONL 或 inject 成功，都不能升成“原生历史可见”。`Imported=原生存储确认可读回` 会把磁盘扫描误当成展示通道。

3. **原生成功、receipt 失败时，§8.3 仍会盲目另建一份。** §8.1 正确：原生写入不在 FyAgent SQLite 事务里。但 §8.2 未命中就 `unknown` 交用户，§8.3 又写“重试=新建另一份”。`migration_key` 单列主键也无法同时保住两次 `SaveAsNew` 的两个 native id。目标若只回 `{}`、列表又为空，内容指纹对账会因运行时注入（skills、AGENTS.md、`environment_context`，且 role 可以是 user）对不上包内 key。此时不能承诺 exactly-once。

4. **QA 与技术合同不是改名能对齐的。** `test-plan.md` §11 要求行为 oracle 不变、只做机械改名。下列是行为冲突，不是拼写：

| 主题 | 技术设计 | 测试合同 |
|---|---|---|
| 两条 assistant、无完成信号 | 取最后一条，Positional，可导出 | `F-INDETERMINATE` → `FINAL_ANSWER_INDETERMINATE`，整包失败 |
| 末轮只有 user | `incompleteTailTurn`，允许导出提问 | `FINAL_ANSWER_MISSING`，整包失败 |
| 前缀变长 | key 变化，默认另存，不合并 | `QA-IMP-003` 要求原地只追加 |
| 读回为空 | 无此错误；有 ID 即 `imported` | `NATIVE_READBACK_MISMATCH`，停在 `native_written` |
| receipt 失败 | SQL `UPDATE`；未命中可再写 | `atomic rename` + `RECONCILIATION_REQUIRED`；0 命中可重试，多命中 `AMBIGUOUS_NATIVE_MATCH` |
| 未知字段 | `deny_unknown_fields`，无独立码 | `PACKAGE_UNKNOWN_FIELD` |
| 深度 | 无 JSON 深度上限 | `JSON_TOO_DEEP` |
| 大小 | 256MiB 源 / 4MiB 消息 / 8MiB 轮 / 2000 轮或 32MiB / 200 会话或 128MiB 包；码为 camelCase | `MESSAGE_TOO_LARGE`、`TOO_MANY_TURNS`、`PACKAGE_TOO_LARGE` 等 SCREAMING_SNAKE，且把轮数与深度拆开 |
| 目录 / 版本 | `TargetDirectoryInvalid`、`UnsupportedVersion` | `TARGET_DIRECTORY_NOT_FOUND`、`PROVIDER_VERSION_UNSUPPORTED` |

   `test-cases.json` 35 条全部 `status=not_run`。`test-plan.md` §11 写明本轮未跑生产测试。不得说执行通过。§8 已经拒绝七家×全 OS×全异常笛卡尔积，这份矩阵作为门槛清单可以留，不能当成必跑全表。

## 简化项

- 不要 sidecar receipt 再加一张表。二者是同一回执的两个候选，叠用才是冗余。`redundancy-review.md` 的 sidecar 不作为最终存储。
- 不要第二套会话库、作业引擎、跨设备身份服务、压缩包、云同步、插件框架、`Overwrite`。候选 B 的否决成立。
- 不要把 `ThreadResumeParams.history`、Hermes 顶层全量 `import`、模型补写、用户确认，当作恢复或验收。
- Codex 以外五家在读到各自完成字段之前，保持“未验证”，不要预先钉死 Positional，也不要升成 Explicit。Claude 的 `stop_reason`、Hermes 未 SELECT 的列，同样是 reader 未读，不是 schema 证明。

## 应保留的必要复杂度

回执放进**现有** FyAgent SQLite，一张 `session_recovery_receipts`，`SCHEMA_VERSION` 20→21（`database/mod.rs:56`）。理由是证据，不是“SQLite 更高级”：

- 原生写入本来就不能跟本地回执成一个事务。SQLite 买到的是本地行的原子更新、`migration_key` 冲突闸和并发命令下的单行更新，不是跨进程 exactly-once。
- 回执是设备本地事实。现成排除表是 `database/backup.rs:76-97` 的 `SYNC_SKIP_TABLES` 与 `SYNC_PRESERVE_TABLES`。sidecar 若落在会同步的应用数据里，必须另做一套排除，崩溃窗口并不更小。
- 代价：`schema.rs` 与 `backup.rs` 要排队，且不能再占版本号。这比新库或任务引擎小。
- 独立 strict 抽取器、包版本门、`deny_unknown_fields`、六级分证、按 provider 的能力表，都要留。展示 DTO 继续可以含 `[Tool: ...]`，迁移路径不能复用它。

## 具体改法

1. **Final。** Codex 只接受 `phase=final_answer` 且正文块为 text 的 assistant 为 Explicit。`commentary` 计入 progress，不得当最终答复。`phase` 缺失、多条 `final_answer`、或只剩 `content_item_kinds` 含 `unknown`/工具/进度时，该轮 `Unknown`，非末轮整会话导出失败。Unknown 只计数，不搬运工具、进度、思考、附件。运行时 role=user 的开发者/环境注入与真实用户消息分开，不进包。
2. **阶段。** 拆开 `native_written` 与 `native_readback_verified`。Codex 的 G3 必须看到非空的原生列表/历史，或明确记 `turns/items` 为空为 blocked。JSONL 与模拟请求各算一条证据，不能合并成“可恢复”。
3. **回执主键改为 import instance**（`migration_key` + `attempt`，或单独 instance id）。`thread/start` 或调用方预分配的 native id 在副作用前写入同一行，stage 仍为 `importing`。读回成功才升到已读回。id 从未返回且列表不能唯一命中时，保持待对账，禁止再调用原生写入，文案不得写 exactly-once。`SaveAsNew` 新开一行，不覆盖旧 native id。
4. **对账键。** 有已存 native id 就按 id 查。没有 id 时，只用包内 final-only 摘要探测，排除 developer 与运行时注入。0 或多项都不自动重写。Hermes `LIMIT 500` 与 OpenClaw id 重置保持为已知漏检，漏检结果是 unknown，不是新副本。
5. **合同。** 以 `design.md` 第 3 条为准：不自动合并两端新历史；末轮无答复可导出提问；中间轮不可判定或无答复则整段失败。测试改掉 `QA-IMP-003` 的原地追加，以及把末轮未完成当成 `FINAL_ANSWER_MISSING` 的预期。保留 `F-INDETERMINATE` 的失败关闭。错误码与上限只留一套，建议机器码用技术设计的 camelCase，并补上测试里有行为、设计里缺失的 `nativeReadbackMismatch`、`reconciliationRequired`、`ambiguousNativeMatch`、`packageUnknownField`。深度上限若要保留，写进技术设计的数字；否则从用例删除。
6. **§9.2 重写证据等级**，与 `findings.md` 对齐。OpenCode/Hermes 仍只到隔离导入与读回，不到续聊和双向。

## 接受与未通过

接受：JSON 包加现有库一张回执；不覆盖；能力按版本门控；Jev 不进产品运行时（`restore-gate-response.json` 只支持逐版本门槛，不是验收证明）；35 条作为未执行计划；UX 本读版本的三处旧缺陷已不在。

未通过：六家 positional 与最后一条 assistant 规则；§8.3 盲重试和单键主键；`imported` 吞掉空列表；QA/技术字段、上限、错误码、增量语义；任何 provider 的“支持恢复”或跨设备完成声明。

## UX 读到的版本

终稿前重读了当前工作区的 `deliverables/ux-spec.md` 与 `deliverables/session-prototype.html`，未改它们。相对 `briefs/ux-final-fixes.md` 的旧问题，这一版：

- `markUserConfirmed` 只设 `userConfirmed`，不改 `capability`，也不把 `status` 设为 normal。
- `applyRemap` 只改 `workspace`。
- 没有 `copyContinuationCommand`，没有下载 `fyagent.session.v1`。导出是预览文本；无 final 则省略。
- `handleResumeInTarget` 仍会在模拟写入后把 `status` 设为 `pending_verify`（约第 1693 行）。这是演示写入，不是用户确认升能力。未验证 capability 与任一 `isIndeterminate` 会被拦住。

此段只描述这次读到的字节。Antigravity 若再改，需重读后再引用。
