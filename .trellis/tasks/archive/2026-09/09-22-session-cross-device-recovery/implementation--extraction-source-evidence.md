> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Extraction source evidence: OpenCode 1.18.30 and Hermes v0.20.5

日期：2026-09-22。范围只覆盖已安装/已定位的 provider 源码、现有合成 fixture 和固定版本 OpenCode 官方源码；不读取真实会话、数据库内容或凭据，不运行真实模型。

## 结论

两家都已有可实现的 final-only 规则，但不能写成 `最后一条 assistant`：

* OpenCode 的 `time.completed` 只表示当前 assistant 记录被收尾写回，清理/中断也会设置它；真正的正常终结来自 `info.finish` 在 `step-finish` 事件处写入，并且必须结合 tool part 状态、error 和文本 part 判断。
* Hermes 的持久化行有 `role`、`tool_calls`、`tool_name`、`finish_reason`、`api_content`；当前官方源码已提供 O(1) lifecycle 分类。`api_content` 是发给 API 的用户消息副本，不能当用户原文；展示/筛选应优先 `content`，并用 `display_kind`/合成标记排除运行时注入。

## OpenCode 1.18.30

### 版本和源码定位

本机固定版本来自 `research/session-recovery-20260921/local-cli-evidence.json`：`1.18.30`。同版本官方源码 tag 为 [`v1.18.30`](https://github.com/anomalyco/opencode/tree/v1.18.30)。本机已安装二进制是 `/Users/<username>/.opencode/bin/opencode`；本报告不把当前 `dev` 源码混入版本结论。

关键源码：

* [`packages/opencode/src/session/processor.ts`](https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/session/processor.ts)：`handleEvent` 在 `step-finish` 写 `ctx.assistantMessage.finish = value.reason`，并写入 `step-finish` part；`text-start/delta/end` 单独维护文本 part。`finish` stream event 本身不写消息完成字段。
* 同文件 `cleanup`：无论正常结束、abort 或异常收尾，都会把仍在生成的文本/reasoning/tool parts 结束或标为 error，并设置 `ctx.assistantMessage.time.completed = Date.now()`。因此 `time.completed` 单独不能证明正常最终答复。
* [`packages/opencode/src/session/message-v2.ts`](https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/session/message-v2.ts)：`latest()` 只把有 `info.finish` 的 assistant 放入 `finished`；`filterCompacted()` 只把 `summary && finish && !error` 的 summary assistant 当作完成的 compaction。该函数是可直接复用的来源语义。
* [`packages/opencode/src/session/message.ts`](https://github.com/anomalyco/opencode/blob/v1.18.30/packages/opencode/src/session/message.ts)：旧 message shape 的 parts 区分 `text`、`reasoning`、`tool-invocation`、`step-start`；tool invocation 有 `partial-call`、`call`、`result` 三态。

### 可实现的 final-only 规则

对每个 assistant message，以 message ID/created time 排序并沿 parent 关系分组：

1. 必须有 `info.role === "assistant"`、`info.finish`，且没有 `info.error`。`time.completed` 只能作为持久化完整性字段，不能替代 `finish`。
2. `finish` 为 `tool-calls` 表示该 assistant step 结束在工具调用；它不是最终自然语言答复。要求每个 tool part 具有 completed/result 或明确 error 终结；pending/running，或 error 且 `metadata.interrupted === true`，都使这一轮 `finalAnswerIndeterminate`。
3. 有 `finish` 且含正常 `step-finish`、无未闭合工具、含非空 `text` part 的 assistant，才是可导出的最终答复候选。reasoning、step-start、tool-invocation/commentary 只作为结构证据，不输出为 final text。
4. OpenCode 没有独立“commentary”role；模型流中的文本都落在 assistant 的 text part。若一个 assistant 先输出文本又继续 tool step，必须按 `finish` 和 tool 状态判断，不能选该消息的最后一段文本。
5. 用户原文只取 `role=user` 的非合成、非 ignored text parts。OpenCode 自己会制造 synthetic user parts（例如 tool 后的 “The following tool was executed by the user”/继续提示）；合成 fixture 必须覆盖 `synthetic: true` 与普通用户文本混合形状。

Fail-closed 形状：缺 `finish`、有 `error`、只有 `time.completed`、任意 tool pending/running/interrupted、只有 reasoning/tool parts、或 compaction/summary 没有 `summary && finish && !error` 时，都不得声称存在最终答复。

### 合成回归 fixture

使用现有 `research/session-recovery-20260921/check_opencode_import.py` 的隔离 XDG store 和 `opencode-synthetic-result.json`，增加四组纯合成消息：

* `user text → assistant text + step-finish(reason=stop)`：应导出一条 final。
* `user → assistant tool call(finish=tool-calls) → tool result → assistant text + stop`：只导出第二个 assistant 的 text。
* `assistant text + time.completed` 但无 `finish`，以及 `finish=tool-calls` 仍有 pending tool：均为 indeterminate。
* synthetic user continuation/tool-summary 与真实 user text 相邻：只保留真实 user 原文。

已有隔离 import/export 已证明文本 role、顺序和内容可往返；尚未证明下一轮模型响应，因此 native continuation 与 final-only extraction 仍应分别验收。

## Hermes v0.20.5

### 版本和写入点

版本证据：`research/session-recovery-20260921/local-cli-evidence.json` 记录 `v0.20.5`、upstream `f293e720`、本机携带补丁 `987064ca (+1 carried commit)`；源码树是 `/Users/<username>/.hermes/hermes-agent`。

核心写入点：

* `hermes_state.py:9805-9920` 的 `SessionDB.append_message()` 把 `role`, `content`, `tool_name`, `tool_calls`, `finish_reason`, `reasoning*`, `api_content`, `display_kind`, `display_metadata` 原样分列写入；`tool_calls` 列表会 JSON 序列化。
* `hermes_state.py:10272-10353` 的 `_insert_message_rows()` 是 replace/import/compact 的批量写入路径，保留相同字段；因此合成 writer 必须至少明确 role/content/timestamp，若有工具或终结状态必须保留 tool_calls/tool_name/finish_reason。
* `agent/chat_completion_helpers.py` 的 assistant 构造/流结束路径把 provider finish reason 规范化为 `stop`、`length`、`tool_calls` 等；`gateway/session.py:3815-3838` 将 transcript message 送进 `append_message`，并从消息提取 `api_content` sidecar。
* `agent/turn_context.py:640-710` 先把干净用户原文作为 `content` 持久化，再把 plugin/gateway/memory 上下文拼入 API 副本；`api_content` 是“实际发给 API 的 bytes”而非用户原文。`display_kind` 会给自注入/合成 user 行打标签。

### 可实现的 final-only 规则

当前源码直接提供 `hermes_state.py:3550-3580` 的 `classify_session_status()`：

* 最后一行 `role=assistant` 且没有 `tool_calls` → `complete`；
* 最后一行 assistant 带 `tool_calls` → `interrupted`（工具结果尚未使该轮闭合）；
* 最后一行 `role=user` 或 `role=tool` → `interrupted`；
* `finish_reason` 为 `error`、`agent_error`、`content_filter` → `error`；
* 未知形状当前函数保守返回 `complete`，但 FyAgent 的 final-only 导出应比 UI session picker 更严格：未知 role/finish、缺 finish_reason 且存在工具链、或 `length` 截断应标 `finalAnswerIndeterminate`，不能沿用这个 benign default。

因此可导出的最终 assistant 候选须满足：`role=assistant`、`tool_calls` 为空、无错误 finish、存在文本 content，且它是最后一个可见且闭合的 assistant；前一 assistant 的 tool_calls 必须能与后续 `role=tool` 的 `tool_call_id`/`tool_name` 配对。`finish_reason=stop` 是最强正常终结证据；`length` 虽被源码 status 视为结束，仍应按产品策略标为截断/不确定，除非明确允许不完整答复。

`tool_name`/`tool_calls` 是工具活动字段，不能拼进最终文本。`reasoning`, `reasoning_content`, `reasoning_details` 也不能当最终答复。`api_content` 只用于恢复模型上下文的精确重放，不用于展示用户原文。

### 运行时注入的结构识别

`turn_context.py` 的规则是：保存的 user `content` 保持干净，plugin/gateway/memory 注入写入 API 副本；注入若需要持久化其展示语义，会写 `display_kind`/`display_metadata`。已有源码的 `ContextCompressor.is_user_originated_turn()`（约 `context_compressor.py:8029-8049`）明确排除任何 `display_kind`、压缩 summary 和 synthetic compression user。

因此提取用户原文时：取 `role=user` 的 `content`，排除 `display_kind`、压缩/summary/synthetic 形状；若只有 `api_content` 没有 clean `content`，或一个 user 行同时缺 provenance 与合成标识，必须 fail-closed，不能猜测注入边界。读取 `api_content` 可验证模型重放，但不能把其中的 memory/plugin/gateway 文本回填为用户原话。

### 合成回归 fixture

沿用 `research/session-recovery-20260921/check_hermes_import.py` 的隔离 `HERMES_HOME` 与 synthetic Codex transcript，不接触真实 `state.db`：

* `user(content=U1, api_content=U1+synthetic-context) → assistant(content=A1, finish_reason=stop)`：用户输出 U1，final 输出 A1，API replay 可单独核对 sidecar。
* `assistant(tool_calls=[c1], finish_reason=tool_calls) → tool(tool_name=t, tool_call_id=c1) → assistant(A2, stop)`：final 只能是 A2。
* `assistant(tool_calls=[c1], finish_reason=tool_calls)` 无对应 tool，或最后一行 user/tool：`interrupted`。
* `assistant(A, finish_reason=None)`、`finish_reason=length`、error/content_filter、只有 reasoning 或空 content：全部按 extraction policy 验证为 indeterminate/error。
* `role=user, display_kind=internal_notification`、compression summary、synthetic continuation 与真实 U1 相邻：只保留 U1；`api_content` 注入内容不能作为 U1 的替代。

已有 Hermes synthetic result 已证明 Codex-format final-only 问答可由安装版 importer 写入隔离 native store、按续聊结构读回并追加离线用户消息；尚未证明真实下一轮模型响应，不能把 import/readback 证据升级为在线执行保证。

## 给 backend 的落地边界

1. OpenCode extractor 使用 `finish` + tool-part closure + error/text checks；`time.completed` 只做辅助诊断。
2. Hermes extractor 可复用 `classify_session_status` 的角色/工具/错误判定，但对 final-only 采用更严格 unknown/length fail-closed；`api_content` 与 clean `content` 分开暴露。
3. 两家都记录 `sourceVersion`, `sourcePath/commit`, `finalEvidence`（finish reason、tool closure、injection markers）；缺关键证据返回 `finalAnswerIndeterminate`，不静默选择最后 assistant。

## 证据限制

OpenCode 的行号/语义来自官方 `v1.18.30` tag；当前仓库只读了本机 adapter、已有 synthetic 结果和源码。Hermes 的源码是本机固定 upstream/带补丁树，当前工作包没有把补丁与 upstream 差异逐行展开；因此 Hermes 的字段存在性可信，若要声明跨版本稳定仍应 pin 版本并跑上述纯合成 fixture。无真实会话、凭据、生产数据库或真实模型调用。
