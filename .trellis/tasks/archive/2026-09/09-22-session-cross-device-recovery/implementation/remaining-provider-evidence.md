# Remaining provider evidence: Claude / Gemini / Grok / OpenClaw

日期：2026-09-22  
范围：只读取已有 Session 研究、当前仓库 provider adapter、四个本机 CLI 的版本/帮助，以及官方文档/源码；未读取真实会话正文、未读取凭据、未调用真实模型、未写入任何 provider store。

## 先给实施结论

四家不能归为同一种“暂不支持”：

| Provider | 当前最确定的实现路线 | 当前阻塞 |
| --- | --- | --- |
| Claude Code | 读取完整本地 JSONL，按结构提取 user 与 terminal assistant 文本；原生恢复只能保留完整 transcript 后 `--resume`，或使用官方 cloud `--teleport`。 | 官方没有 final-only 精简 JSONL 导入契约；`/export` 是纯文本，`/import` 是配置导入。最终答复不能靠“最后一个 assistant”猜定。 |
| Gemini CLI | `--session-file` 是明确的原生构造入口；官方源码给出 `ConversationRecord`，导入时只保留 user/gemini 消息并生成新的 session ID。适合先做合成 fixture + 版本门槛。 | 当前本机是 `0.46.0`，旧报告为 `0.59.0`；需把官方当前 schema 与本机 exact version 的合成导入/重启/下一轮读回补成证据。`gemini` 消息仍可携带 toolCalls/thoughts，final-only 判定需要状态机。 |
| Grok Build | 原生 `--resume`/`--continue` 读取自身 session 目录；上游源码存在未公开 `_x.ai/session/state` / `_x.ai/session/import`，可作为 capability-detected 的 Grok→Grok 跨主机候选。 | 本机 `1.0.40` CLI help 没有 import；公开 help 的 export 只出 Markdown。ACP 扩展未证明在本机版本和目标路径可调用，需隔离 probe，不能把私有源码存在等同于产品可用。 |
| OpenClaw | 用 authenticated Gateway 的 `sessions_history` 做 final-only 显示投影，精确读取时走 scoped SQLite rows；保留原 Gateway/session store 才能继续。 | 官方没有自定义精简 transcript import。Session Share 是只读；直接改 SQLite 或将 `includeTools=false` 的 projection 回灌都没有官方恢复契约。 |

因此 backend 可以先实现：版本/能力矩阵、四家只读扫描、final-only 的“可判定/不可判定”状态、Gemini `--session-file` 的隔离 writer，以及 Claude/Grok/OpenClaw 的“原生恢复待验证/必须保留完整原生状态”分支。不要把后三家的缺口写成永久不支持；产品状态应是 `unverified` 或 `nativeStoreRequired`，直到 exact-version probe 有新证据。

## 当前本机版本与帮助

本次只读回读：

```text
Claude Code 2.1.220
Gemini CLI 0.46.0
grok 1.0.40 (eb1a2256660d)
OpenClaw 2026.7.1-2 (0790d9f)
```

关键 CLI surface：

- Claude：`--resume [value]`、`--continue`、`--fork-session`；当前帮助还没有 transcript file import 参数。交互 `/export` 为 plain text，`/import [codex|gemini|cursor]` 是配置导入（instruction/MCP/commands/subagents/skills），不是会话。
- Gemini：`--resume`、`--list-sessions`、`--delete-session`、`--session-file <JSON>`、`--session-id`。`--resume`、`--session-id`、`--session-file` 互斥。
- Grok：`--resume`、`--continue`、`--fork-session`、`--session-id`；`grok sessions` 只有 list/search/delete，`grok export` 只导出 Markdown，没有公开 import 子命令。
- OpenClaw：`sessions list`、`sessions export-trajectory`、`sessions_history` 相关 Gateway 工具；CLI 没有自定义 transcript import。

## 1. Claude Code 2.1.220

### 本地格式与官方入口

本地 Claude 项目会话位于 `~/.claude/projects/<project>/<session>.jsonl`。官方 `.claude` 文档把它定义为完整 conversation transcript，包含 every message、tool call、tool result；文件是 plaintext，默认清理周期为 30 天。当前仓库 adapter 对每行读取 `message.role`、`message.content`、`timestamp`，并把“整个 content 都是 `tool_result` 的 user 行”重分类为 `tool`：`src-tauri/src/session_manager/providers/claude.rs:33-86`。

官方 commands 文档：

- `/resume` 返回本机历史会话；`claude --resume <id>` 是同一能力。
- `/export` 只导出当前对话为 plain text。
- `/import [codex|gemini|cursor]` 导入配置，不是 Claude transcript。

官方 cloud `--teleport <session-id>` 是另一条完整原生路径：要求同一仓库、干净 Git 状态、云分支已推送、同一 claude.ai 账号；它加载完整 cloud conversation，并在终端建立本地副本。它不是 FyAgent envelope 的任意导入。

Primary sources:

- [Claude Code commands](https://code.claude.com/docs/en/commands)（`/resume`、`/export`、`/import`）
- [Claude `.claude` directory](https://code.claude.com/docs/en/claude-directory)（完整 JSONL、plaintext、retention）
- [Claude Code cloud / teleport](https://code.claude.com/docs/en/claude-code-on-the-web)（同仓库/账号/分支要求）

### 用户原文与最终答复判定

- 用户原文：结构上取 `type=user` 或 `message.role=user` 的真实用户行；排除 `isMeta=true`、`<local-command-caveat>`、`<command-name>` 等本地命令/系统注入。当前 adapter 已有“首个真实 user”标题过滤，但不是 final-only 判定。
- 工具结果：Claude 会把 `tool_result` 包在 `user` message 中；不能按顶层 role 过滤。content 全为 tool_result 时可以排除，但混合 content 仍需逐 block 解析。
- 最终答复：Claude transcript 没有一个可依赖的“这是最终答复”的稳定公共字段。单独看到 `assistant` text 不足以证明该轮结束；后续可能还有 `tool_use`、工具结果和新的 assistant。实现需按消息顺序把 assistant content block 与 tool-use/tool-result 关联，只有到该轮终结且没有待处理 tool-use 时才候选 final；无法闭合就 `finalAnswerIndeterminate`，阻止导出。

“取最后一条 assistant”在以下反例会错：assistant 先解释再调用工具；assistant 同一消息含 text + tool_use；工具结果以 user role 回来；中断后没有最终 assistant。该规则必须进 fixture，而不是靠自然语言判断。

### 最小原生文件与阻塞

官方支持的原生 resume 最小单位是 Claude 自己生成的完整 session JSONL（连同其事件/元数据语义），不是一个公开的 final-only JSONL。可以在隔离 `CLAUDE_CONFIG_DIR` 中生成合成完整 transcript 做 parser/readback fixture，但不应把猜出的删减 JSONL 当生产 writer。

实现建议：

1. 保留完整原生 transcript 的只读 provenance（不进入 FyAgent final-only 迁移包），或调用同账号、同仓库的 `--teleport`。
2. FyAgent envelope 只存确定的 user/final 文本；如果目标只允许 final-only，显示为“迁移后的新会话/待人工继续”，不能标 native resume。
3. 先实现 exact-version reader 与状态机，待隔离 fixture 证明完整 JSONL resume；不要删除 tool events 后尝试 `/resume`。

阻塞是“没有公开精简导入契约”，不是 Claude 永久不可恢复。

## 2. Gemini CLI 0.46.0

### 本地格式与官方构造入口

当前本机实际版本是 `0.46.0`，不是既有报告中的 `0.59.0`。本机 help 明确提供 `--session-file <JSON>`。

Google 官方源码 `packages/core/src/services/chatRecordingTypes.ts` 定义：

```ts
interface ConversationRecord {
  sessionId: string;
  projectHash: string;
  startTime: string;
  lastUpdated: string;
  messages: MessageRecord[];
  summary?: string;
  directories?: string[];
  kind?: 'main' | 'subagent';
}

type MessageRecord = BaseMessageRecord &
  ({ type: 'user' | 'info' | 'error' | 'warning' } |
   { type: 'gemini'; toolCalls?: ToolCallRecord[]; thoughts?: ...; tokens?: ...; model?: string });
```

`BaseMessageRecord` 需要 `id`、`timestamp`、`content`；`content` 是 Gemini `PartListUnion`，文本可用 `{text: "..."}`。官方 CLI source `packages/cli/src/gemini.tsx` 的 `--session-file` 路径会：加载 JSON、仅保留 `type=user` 或 `type=gemini` 且有 `content` 的消息；前置一条 info；重新生成 `sessionId`、`projectHash`、时间字段；写入当前项目 chats 目录。这是确定存在的“构造最小原生文件 → 新 session ID → 原生 `--resume`”路径。

Primary sources:

- [Gemini session management](https://geminicli.com/docs/cli/session-management/)（存储、resume、项目作用域）
- [Gemini `chatRecordingTypes.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/services/chatRecordingTypes.ts)（官方 schema/type）
- [Gemini `gemini.tsx`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/gemini.tsx)（`--session-file` import/rewrite 行为）

### 用户原文与最终答复判定

- 用户原文：`type="user"` 的 `content`，优先提取 text parts；不要把 `functionResponse`/内部 hook continuation 当用户正文。Google 源码自身记录路径也特别区分 function response 与真正 user content。
- 候选答复：`type="gemini"` 的 `content` text parts。这个记录可附带 `toolCalls`、`thoughts`、`tokens`、`model`；不能因为类型是 `gemini` 就直接视为 final-only。
- 最终答复：必须结合记录顺序和 `toolCalls`/工具结果生命周期。若同一 `gemini` 记录有未完成 tool call、或后续还有同一轮的工具/continuation 记录，不能定为 final。没有可靠终结信号时标 `finalAnswerIndeterminate`，不要只取最后一个 `gemini`。

### 最小文件与当前验证缺口

可实现的合成最小文件形状（示意；只用临时目录和合成文本）：

```json
{
  "sessionId": "synthetic-source-id",
  "projectHash": "synthetic-project",
  "startTime": "2026-09-22T00:00:00.000Z",
  "lastUpdated": "2026-09-22T00:00:02.000Z",
  "messages": [
    {"id":"u1","timestamp":"2026-09-22T00:00:00.000Z","type":"user","content":[{"text":"U1"}]},
    {"id":"a1","timestamp":"2026-09-22T00:00:01.000Z","type":"gemini","content":[{"text":"A1"}],"model":"synthetic"}
  ]
}
```

官方 import 会生成新的 session ID，因此目标映射必须记录 source→target，不能要求保留 source ID。需要在本机 `0.46.0` 隔离 `GEMINI_CLI_HOME`/临时 project 上补四步证据：`--session-file` 接受 fixture、`--list-sessions`/`--resume` 读回、重启后读回、下一轮请求能看到 U1/A1。当前已有官方 source 证据，但没有 exact `0.46.0` 的这组运行证据；所以能力应标 `candidateNativeRestore`，不是已完成跨设备支持。

## 3. Grok Build 1.0.40（模型 Grok 4.7 另行标注）

### 本地格式与官方入口

本机版本：`grok 1.0.40 (eb1a2256660d)`；模型 `grok-4.7` 是运行选择，不是 CLI 版本。当前 help：

- session 目录：`~/.grok/sessions/<url-encoded-cwd>/<session-id>/`。
- 官方 `summary.json`：ID、cwd、标题、timestamps、model、message counts 等元数据。
- `updates.jsonl`：官方文档称为驱动 `/resume` 与 session restore 的 authoritative conversation log；包含 ACP session updates、对话和 tool calls。
- `chat_history.jsonl`：模型发送的 raw chat messages；适合抽取 user/assistant 候选，但不是单独的 resume authority。
- `grok --resume <id>` / `--continue` 是公开原生恢复；`grok export` 只生成 Markdown。

Primary source:

- [xai-org/grok-build session guide](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/17-sessions.md)（目录、`updates.jsonl` authority、resume）
- [xai-org/grok-build source tree](https://github.com/xai-org/grok-build)（版本变化需 pinned commit/CLI probe）

上游源码/技术审计还显示未公开 ACP 扩展 `_x.ai/session/state` 与 `_x.ai/session/import`：state 读取 selected metadata，import 可在另一主机重建 metadata + `updates.jsonl`。这给 Grok→Grok 跨主机提供了一个有希望的官方实现方向，但扩展未出现在本机 CLI help，且不是当前公开稳定契约；必须对 `1.0.40` 做 isolated ACP capability probe，成功后才能启用。

### 用户原文与最终答复判定

- 用户原文：`chat_history.jsonl` 中 `type="user"`，但开头可能有系统 preamble/synthetic user。只采集带真实 user-turn 结构的记录，剥离 `<system-reminder>`、环境注入和 `prompt_context` 引用。
- assistant 候选：`type="assistant"` 的文本；`tool`、reasoning/internal records 不能进入 final-only。
- 最终答复：不能用最后一条 assistant。Grok 双日志分工意味着 UI replay (`updates.jsonl`) 与 model-facing projection (`chat_history.jsonl`) 可能在 rewind/compaction 后不同；需要按 ACP `session/update`/turn completion、tool call/update 配对和 rewind branch 过滤确定某轮 closed final。没有 closed-turn 证据就 `finalAnswerIndeterminate`。

### 最小原生文件与阻塞

公开稳定 writer 的最小格式是未知；不能猜写 `chat_history.jsonl` 或 `updates.jsonl`。优先顺序：

1. 在隔离 Grok home 创建合成 session，记录 `summary.json`、`chat_history.jsonl`、`updates.jsonl` 的 exact shape。
2. capability probe `_x.ai/session/state` / `_x.ai/session/import`；若 `1.0.40` 返回 method-not-found 或 schema error，保留真实回执，不宣称支持。
3. 若扩展可用，使用官方返回/接收的完整 updates stream 重建新 ID，再用 `grok --resume`、重启和下一轮 fixture 验证。
4. 若扩展不可用，只提供 final-only display projection 或“新会话”引导，不能标 native restore。

## 4. OpenClaw 2026.7.1-2

### 本地格式与官方入口

本机版本：`2026.7.1-2 (0790d9f)`。公开 CLI 提供：

- `openclaw sessions list`；
- `openclaw sessions export-trajectory`（redacted trajectory bundle）；
- Gateway session tools `sessions_list`、`sessions_history`、`sessions_search`、`sessions_send`、`session_status`。

官方 OpenClaw 文档明确：`sessions_history` 默认排除 tool results，`includeTools=true` 才加入；但 reasoning tags、tool-call XML、control tokens 等仍可能残留在消息文本，且有截断/大小上限。若要 exact raw transcript，官方要求读取 scoped SQLite transcript rows，而不是把 `sessions_history` 当无过滤 dump。源 runtime 的 SQLite/session key/sessionId/provenance/controller ownership 是 native continuation 的边界。

Session Share plugin 只把 source Gateway 的选定 session 发布给 paired receiver；官方明确 receiver 是 read-only，不允许 continuation、terminal access 或 worker execution。

Primary sources:

- [OpenClaw session tools](https://docs.openclaw.ai/concepts/session-tool)（history projection、includeTools、redaction、raw SQLite boundary）
- [OpenClaw session keys/IDs/schema](https://docs.openclaw.ai/reference/session-management-compaction/schema)（session identity/event semantics）
- [OpenClaw node session catalogs](https://docs.openclaw.ai/nodes/session-catalogs)（Session Share read-only boundary）

本仓库 reader 当前直接读 `~/.openclaw/agents/<agent>/sessions/*.jsonl`，筛 `type="message"`，把 `toolResult` 映射为 tool，并剥离 `[message_id: ...]` 尾标：`src-tauri/src/session_manager/providers/openclaw.rs:21-123`。这是现有读取适配器，不能当作 OpenClaw 官方导入格式。

### 用户原文与最终答复判定

- 用户原文：从 scoped raw rows 或 authenticated `sessions_history` 中读取 role=user 的正文；需要排除 injected prompt、pendingInputs、tool payload 和 redacted/truncated rows。
- assistant 候选：role=assistant 的 text；`includeTools=false` 只隐藏 tool-result rows，不能保证 assistant text 内没有工具 XML、reasoning scaffold 或控制标记。
- 最终答复：需要 runtime 的 turn/agent lifecycle、工具调用配对和完成状态；不能取最后 assistant，也不能把 `sessions_history` 的 latest row 当 final。若 bounded history 标记 `truncated`、`contentRedacted`、`droppedMessages` 或出现无法闭合的 tool sequence，必须阻止 final-only 导出。

### 最小原生文件与阻塞

没有公开的“导入自定义精简 transcript”入口。`sessions_history` 是读取投影，不是写入格式；Session Share 是查看入口，不是恢复入口；直接编辑 SQLite 会破坏 schema/provenance/ownership 假设。`sessions_send` 只能对已被 Gateway 认可的现有 session 发送输入，不能在另一 Gateway 伪造 native identity。

可实现路径：

1. 只读阶段用 authenticated Gateway history + scoped SQLite rows 生成 final-only projection。
2. 原生恢复阶段保留/迁移 OpenClaw 自己的完整 store，或等待目标版本公开/核验 writer；保留 agentId、session key、sessionId、store fingerprint 和 owner/provenance 映射。
3. 在 writer 未核验前，能力显示为 `nativeStoreRequired` / `unverified`，不能把 session share 或 trajectory export 标为恢复。

## 统一验收与下一步

四家都需要把“确定最终答复”作为结构化状态，而不是简单 role filter：

```text
candidate user/final text
→ tool-call / tool-result / continuation correlation
→ turn closed proof
→ exact text + provenance
→ export allowed
```

无闭合证明：`finalAnswerIndeterminate`，阻止导出。无官方 writer：`nativeStoreRequired`，允许只读/显示投影，但不标 native resume。

最小下一步按风险排序：

1. Gemini 0.46.0：隔离 `--session-file` synthetic fixture，验证导入→resume→重启→下一轮；这是四家中最确定的自定义精简构造入口。
2. Claude 2.1.220：隔离完整 JSONL `/resume` fixture，确认保留完整 transcript 时原生恢复；final-only 仍走 display projection/新会话。
3. Grok 1.0.40：隔离 ACP capability probe，确认 `_x.ai/session/state/import` 是否存在及参数；无证据则保留只读/新会话路线。
4. OpenClaw 2026.7.1-2：只在认证 Gateway 与临时 agent store 上验证历史投影、owner/provenance 和 native continuation；不要写 SQLite，不要把 Session Share 当恢复。

## 证据限制

官方文档/源码版本可能先于或晚于本机 CLI；报告将 Gemini 官方 `main` schema 和 Grok 上游未公开 ACP 扩展标为“方向/待 exact-version probe”，没有把它们升级为当前产品支持。所有真实会话、凭据、真实模型调用和跨设备运行仍未读取/执行。
