# FyAgent Session 跨设备原生恢复研究（Codex / Claude Code / OpenClaw）

日期：2026-09-21。范围：只查官方文档；未读取真实会话、附件或凭据，未运行付费模型。证据分级：**官方**=文档明确写出；**推断**=由官方描述的存储/API约束推导；**未验证**=官方资料没有承诺，需用合成数据实测。

## 结论先行

“Session 替代记忆”可以作为 FyAgent 的一级产品模块，但产品层仅保存“用户输入 + 每轮 AI 最终答复”时，不能把这份投影直接当作 Codex、Claude Code 或 OpenClaw 的原生可恢复会话。三者的原生恢复都依赖运行时自己的会话 ID、完整或结构化的内部 transcript/store，以及环境/账号绑定。可行架构是：

1. FyAgent 保存一份脱敏的显示投影（用户输入、最终答复、时间、来源、原生 session/thread ID、cwd/项目标识）。
2. 原生恢复保留并迁移各产品自己的原始状态，或使用其官方跨设备通道；工具事件可以在 FyAgent 显示层隐藏，但不能在原生 store 中任意删除后再期待 resume。
3. 若只迁移显示投影，产品应提供“以历史摘要/最后答复启动新会话”，标记为新会话，而不是声称原生 resume。

## 对比表

| 产品 | 官方原生恢复入口与最小身份 | 工具事件过滤后的原生 resume | 跨设备/目录/账号依赖 | 官方导入/导出边界 |
|---|---|---|---|---|
| Codex CLI / App Server | CLI `codex resume` 恢复已保存聊天；App Server `thread/resume` 以此前记录的 `thread.id` 恢复。官方还说明线程有 turns/items，item 包括用户消息、agent 消息、命令运行、文件变更、tool call 等。 | **原 thread：不能据官方文档把过滤后的投影回写后原地 resume。** 但 App Server 有官方 `thread/inject_items`：对已加载 thread 追加预构造 Responses API items 并持久化到 rollout；可先 `thread/start` 新建 thread，再注入精简 user/assistant items，随后 `turn/start`，但这是新 thread、新 ID。 | Codex 默认本地 transcript 在 `CODEX_HOME`（如 `~/.codex/history.jsonl`）；项目文档称聊天保留 transcript 和记录的工作目录。App Server `thread/start`/`turn/start` 支持 `cwd`，所以迁移需保存 thread ID、原生状态、目标机器可用的 cwd/工作树。账号/云端权限依当前部署和登录态；官方未承诺仅凭 thread ID 跨账号恢复。 | 官方 App Server 提供 `thread/read`、`thread/list`、`thread/turns/list`；`thread/inject_items` 是明确的官方注入路径，`thread/fork` 是复制存储历史并产生新 ID 的方式。 |
| Claude Code | `/resume` 返回旧对话；本地 transcript 位于 `~/.claude/projects/<project>/<session>.jsonl`。官方描述该文件为“每条消息、工具调用和工具结果”的完整 transcript；`/export` 只导出当前对话为纯文本。 | **推断：不支持。** 任意删掉 tool call/result 后再放回 `.jsonl` 没有官方导入契约，且 `/export` 是输出而非恢复格式；需保留原生 JSONL。`/resume` 是本机 local history；云端用 `--teleport`，不能把精简投影变成同一会话。 | 本地 session 绑定项目路径编码的 `<project>` 和 session 文件；`/cd` 可在会话内移动目录，但跨机器仍需迁移原生文件/配置。云端 `--teleport` 要求同一仓库、干净 git state、分支已推送、同一 claude.ai 账号。Claude 官方说明 `~/.claude` 在 Windows 是 `%USERPROFILE%\.claude`。 | `/export`：纯文本导出；`/import [codex|gemini|cursor]`：导入配置文件、MCP、命令、subagent、skills，不是会话 transcript。官方云端可用 `--teleport`/`--cloud`，但这是 Anthropic 托管的 session handoff，不是任意 transcript 导入。 |
| OpenClaw | 原生状态在每 agent 的 SQLite session store：默认 `~/.openclaw/agents/<agentId>/agent/openclaw-agent.sqlite`；`sessionId` 用于寻址 transcript 行。`sessions_history` 默认排除工具结果；`sessions_send`/`session_status` 使用 session key。 | **官方 + 推断：可过滤读取，不能据此重建。** 官方明确 `sessions_history` 的 `includeTools` 只控制 tool-result messages，且工具 XML/嵌入文本仍可能存在；精确原始 transcript 要读受限 SQLite rows。`sessions` 的 resume 还要求受信任 Gateway admission、原 session store provenance 和 controller ownership。删除工具事件会破坏原生 store 语义，除非 OpenClaw 自己重新生成合法 store（官方未提供此导入流程）。 | 依赖 Gateway、agentId、session key/ID、SQLite store 以及原运行时身份。官方 Session Share 只把选定 native sessions 发布给 paired receiver，明确是只读，不允许 continuation/terminal/worker execution。跨机器需要同一 Gateway/paired host 的官方能力或迁移完整 store；非同一账号/权限不可假设可见。 | 官方提供 `sessions_list`、`sessions_history`、`sessions_search`、`sessions_send` 等运行时工具；没有“导入用户自定义过滤 transcript”的官方接口。`sessions_history` 是受限视图，不是可回写格式。 |

## 逐产品证据

### Codex

- [Codex App Server（官方）](https://developers.openai.com/docs/app-server)：线程是对话，turn 包含 items；items 明确覆盖 user message、agent message、command runs、file changes、tool calls 等。`thread/resume` 以 `threadId` 恢复；`thread/read` 只读；`thread/fork` 由服务端复制存储历史并产生新 ID。文档还指出 `thread.sessionId` 应从返回值读取，不应由 thread ID 自行推导。另有 `thread/inject_items`，可将预构造 Responses API items 追加到已加载 thread 的 model-visible history，并持久化到 rollout。
- [Projects and chats（官方 ChatGPT Learn）](https://learn.chatgpt.com/docs/projects)：从目录启动 Codex 后，聊天保留 transcript 和记录的 working directory；CLI 用 `/resume` 或 `codex resume` 继续保存的聊天。
- [Advanced config / history persistence（官方 OpenAI 文档）](https://developers.openai.com/docs/config-file/config-advanced)：本地 session transcript 默认保存到 `CODEX_HOME`，可关闭或限制大小。该页支持“原生状态在本地配置目录”这一结论，但没有定义可移植导入格式。

补充边界（**源码/生成 schema 检查，非官方稳定契约**）：本机 Codex 0.154.0 的 `ThreadResumeParams.json` 虽出现 `history`、`path` 字段，但字段说明将其标为 `[UNSTABLE]`，并注明 `history` 面向 Codex Cloud、`DO NOT USE`；稳定要求仍是 `threadId`。因此不能把该 schema 的 history/path 当作 FyAgent 的公共导入 API，也不能据此承诺跨版本或跨设备恢复。

因此，FyAgent 若只保存最终答复，无法保留 Codex 的工具/文件/命令上下文；在 Codex App Server 中可以把精简 user/assistant items 注入一个新 thread，随后继续生成，但必须标记为“迁移新会话”。仅隐藏 UI 事件、不删除 Codex 原生 store，才可能在原会话继续。

### Claude Code

- [Commands（官方）](https://code.claude.com/docs/en/commands)：`/resume` 返回早先 conversation；`/export` 将当前 conversation 导出为 plain text；`/import [codex|gemini|cursor]` 导入的是配置（instruction files、MCP servers、commands、subagents、skills），不是会话。`/clear` 会新建空上下文。
- [.claude directory（官方）](https://code.claude.com/docs/en/claude-directory)：`projects/<project>/<session>.jsonl` 是完整 transcript（每条消息、工具调用、工具结果），并且工具传递的文件内容/命令输出会落盘；这些是 plaintext。默认 retention sweep 为 30 天（可配置），所以长期恢复不能只依赖默认保留期。
- [Claude Code in the cloud（官方）](https://code.claude.com/docs/en/claude-code-on-the-web)：本地 `--resume` 不列云端 session；云端迁回终端用 `--teleport`，要求干净 git 状态、同一仓库、分支可从 remote 获取、同一 claude.ai 账号。官方同时说明 teleport 会加载完整 conversation history，但终端得到的是该 session 的本地副本。
- [Project memory（官方）](https://code.claude.com/docs/en/memory)：每个 session 初始是 fresh context；CLAUDE.md 与 auto memory 才是跨 session 的持久知识。auto memory 在 `~/.claude/projects/<project>/memory/`，machine-local，不跨机器/cloud 自动共享。

记忆文件不是原生 `/resume` 的最低必要条件：恢复同一 session 依赖 transcript/session store；但如果目标是“新会话仍具备项目习惯/决策”，CLAUDE.md/auto memory 是额外的持久上下文，必须单独迁移并标明它不是会话本体。

### OpenClaw

- [Session management（官方）](https://docs.openclaw.ai/session)：Gateway session rows/transcripts 默认在 `~/.openclaw/agents/<agentId>/agent/openclaw-agent.sqlite`，并说明 Gateway 重启会尽力在同一 session 继续；失败后 transcript 仍可用，但可能需 Resume in new session。
- [Session tools（官方）](https://docs.openclaw.ai/concepts/session-tool)：`sessions_history` 默认不含 tool results；`includeTools` 只影响 tool-result messages，嵌入的 tool-call XML/reasoning 等仍可能存在。返回的是结构化受限历史，不是原始转储；精确原始 transcript 要读 scoped SQLite rows。`sessions` resume 受 Gateway admission、session store provenance 和 controller ownership 约束。
- [Session keys, IDs and transcript events（官方）](https://docs.openclaw.ai/reference/session-management-compaction/schema)：SQLite `SessionEntry` 的关键字段包括 `sessionId`（寻址 transcript rows）、时间戳、归档状态；`sessionId` 会因 daily/idle reset 变化。说明 session key、sessionId 与 transcript 是运行时状态，不是单纯消息数组。
- [Node session catalogs / Session Share（官方）](https://docs.openclaw.ai/nodes/session-catalogs)：Session Share receiver 只读，不能 continuation、terminal access 或 worker execution；外部 runtime transcript 仍留在原 runtime。该页也说明 Codex/Claude 等“恢复”由拥有该 session 的 runtime 执行。

OpenClaw 的“隐藏工具结果后给产品展示”已有官方的 `includeTools=false` 方向，但这不等于删除底层事件；产品层应直接使用受限 projection，而不是重写 SQLite transcript。

## 最小可验证实验（仅合成数据）

不触碰真实用户会话或凭据，创建一个临时 repo/目录和两轮固定文本：用户 `U1: 计算 2+2`，AI `A1: 4`；第二轮用户 `U2: 继续并说明上一轮结果`。分别记录原生 ID/cwd。

1. **Codex**：用本地 App Server `thread/start` 创建线程，第一轮只使用静态文本（不启用 MCP/命令工具），保存返回的 `thread.id`、`sessionId` 与 thread store；重启连接后 `thread/resume`，发送 U2，核对答复能引用 A1。另起一个 `thread/start`，调用官方 `thread/inject_items` 注入合成的 U1/A1 `message` items，再 `turn/start` 发送 U2，记录新 thread 是否能引用 A1；分别标为“旧 thread 原生 resume”和“新 thread 官方注入迁移”。
2. **Claude Code**：合成 session 后复制 `.jsonl` 到临时 `CLAUDE_CONFIG_DIR` 对应项目目录，先用原文件 `/resume`，再用只保留用户/最终答复的过滤副本。验证原文件可恢复；过滤副本若不被 picker 接受或无法继续，记录为不支持。用 `/export` 验证它是纯文本导出而非恢复格式。
3. **OpenClaw**：在临时 Gateway/agent store 创建合成 session；用 `sessions_history(includeTools=false)` 验证展示 projection，再用正常 `sessions_send`/resume 验证底层 session 继续。不要直接编辑 SQLite；若将过滤 JSON 当 transcript 输入无官方路径，记录为不支持。
4. **跨设备矩阵**：只用同一测试账号和可丢弃 repo，Mac→Windows 与 Windows→Mac 各做一次：Codex 携带原生 store/cwd 映射；Claude 用 `--teleport` 的官方路径（需同一账号/仓库）；OpenClaw 只测 paired Gateway 的只读 Session Share，不能把“可查看”算作可继续。

实验通过标准：原生入口接受并产生同一 session/thread identity 的下一轮；若只能新建会话并粘贴摘要，产品状态应是“迁移后的新会话”，不标为 native resume。

## 主要限制与产品建议

- 官方文档没有给 Codex 或 Claude Code 定义“只含 user/final assistant 的可导入 session 格式”；过滤事件后原生恢复只能标**不支持/未验证**，不能靠猜测 JSONL schema 实现。
- 保存完整原生 transcript 会包含文件内容、命令输出、工具调用甚至可能的凭据；Claude 官方明确 transcript 是 plaintext。FyAgent 的显示 projection 与原生恢复备份应分开、脱敏并按不同权限/保留期管理。
- cwd/path 是上下文的一部分：Mac 与 Windows 路径不同，原生恢复要有路径映射或在目标机器重新选择工作树；不存在的 repo、分支或 worktree 不能靠对话文本修复。
- 账号与运行时所有权是硬边界：Claude teleport 明确要求同一 claude.ai account；OpenClaw resume 绑定 Gateway/store provenance；Codex 文档只保证已保存 thread 的运行时恢复，未承诺跨账号导入。
- “记忆”应作为可选并行模块：Claude 的 CLAUDE.md/auto memory 和 Codex/项目中的 AGENTS.md 可补充跨 session 知识，但不能替代原生 session transcript，也不能使过滤投影变成同一 session。
