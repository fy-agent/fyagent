# CLI Session 跨设备恢复研究（2026-09-21）

范围：只研究官方文档/官方源码公开信息；目标是 FyAgent 保存“用户提示词 + 每轮 AI 最终答复”，过滤具体文件/工具调用输出后，迁移到另一台 Mac/Windows 继续原 AI 对话。文中“支持”是官方明确说明；“推断”是由官方源码/格式推导；没有证据的部分标为未知。

## 结论表

| 产品 | 官方恢复/导入能力 | 官方持久化形态与关键依赖 | 过滤工具事件后的可恢复性 | Mac↔Windows 判断 |
|---|---|---|---|---|
| OpenCode | 官方 v2 页面示例为 `opencode session export/import`；本机 1.18.30 help 核验的是顶层 `opencode export <id>` / `opencode import <file>`，可用 `--sanitize`，版本命令不可混用 | 官方源码的 Message 包含 `role`、`parts`、`metadata.sessionID`；assistant metadata 有 model/provider/path(cwd,root)；parts 可能是 text、reasoning、tool-invocation、file、source-url、step-start | `--sanitize` 文案是“redact sensitive transcript and file data”，没有官方证据表明会删除 tool-invocation；不能当作 final-only 导出。若自行只保留 user/assistant text，官方 import 是否接受该简化 JSON 未知 | 导出 JSON 可搬运的官方证据；导入时可指定 directory（v2 页面能力）。模型/提供商认证、版本兼容未在页面保证；cwd/path 是会话元数据，跨 OS 路径应重写/避免依赖 |
| Gemini CLI | 自动保存；`gemini --resume [latest/index/UUID]`；`/chat share file.md|file.json` 导出当前对话；`--session-file` 为本机 help 证据（由主 Agent 核验） | 官方文档：`~/.gemini/tmp/<project_hash>/chats/`，项目 hash/目录绑定；保存历史含 prompts、responses、tool executions inputs/outputs、token stats、thought summaries（若有）；手动 checkpoint 同一 project 才可 resume | 官方 share 导出的 JSON/Markdown 是否可作为 `--session-file` 的输入、以及如何安全删除 tool events，文档未承诺。官方明确自动 session 包含工具输入/输出，因此 final-only 需要 FyAgent 自己转换并再合成验证 | Windows 与 macOS 路径位置均列出；但 project_hash 与项目根路径绑定，跨设备必须在目标机对应目录/重新映射；账号/版本/模型兼容未知 |
| Grok Build CLI | xAI 网页 reference 列出 `grok --resume <id/title>`、`-r/-c`、`--fork-session`、`grok export` 和 `grok import`；本机 1.0.34 help 未列 `import`，因此必须先做 capability 检测再选择路径 | xAI 官方源码/用户指南：`~/.grok/sessions/<encoded-cwd>/<session-id>/`；`updates.jsonl` 是恢复权威日志，另有 `summary.json`、`chat_history.jsonl`、plan/rewind 等。官方 docs 明确全历史含 tool calls/results、TODO、snapshots 等 | 官方没有 final-only export/import；Markdown export 是 transcript，不保证可被 resume。过滤 `updates.jsonl` 会破坏 ACP 事件序列，不能假设可加载。`--resume` 不带 `--restore-code` 时只恢复对话；remote/worktree 文件恢复需另行条件（本机 help提示） | 官方支持自定义 `GROK_HOME`，但会话目录仍按 encoded cwd 分组；跨 OS 复制需保留/重建目录映射。当前本机版本是否支持网页所列 import 未确认，不能直接承诺 |
| Hermes Agent | `hermes --resume ID/title/latest`、`--continue`；`hermes sessions import` 可导入 Claude/Codex 日志并生成新 Hermes session；本机 help 另有顶层 `hermes backup`、`hermes import`，而 `sessions` 下的导入/导出命令需按版本核验 | 官方文档：canonical `~/.hermes/state.db` SQLite（WAL），sessions 表元数据，messages 表完整历史；字段含 id/source/user_id/model/title/timestamps，messages 含 role/content/tool_calls/tool_name/token_count；`sessions.json` 只是 gateway routing 镜像，不是 session 列表 | 官方跨 CLI importer 明确把 ordered user/assistant conversation 带入，并把工具活动压成 assistant 内 `[ran tool: …]`；system prompt、注入上下文、reasoning、raw tool output 丢弃。这与 final-only 目标最接近，但官方只承诺 Claude/Codex importer，未承诺 FyAgent 自定义 JSON/SQLite 可导入 | SQLite 文件可理论复制，但官方未承诺 Mac↔Windows 直接复制的锁/WAL/版本安全；更稳妥是官方 importer/导出备份后在目标机导入。Hermes “记忆”与 session 分开：文档说明 reset 前保存 memories/skills，session 本身仍是 state.db 历史 |

## 最小会话结构（用于 FyAgent 自有中间格式）

建议保存与产品无关的 envelope，再按 provider 生成产品原生格式；不要把某个 CLI 的内部 ID 当作跨产品 ID：

```json
{
  "schema": "fyagent.session.v1",
  "provider": "opencode|gemini|grok|hermes",
  "source_session_id": "opaque-id",
  "title": "optional",
  "cwd": "optional logical workspace, never required for transcript-only resume",
  "model": "optional provider model id",
  "turns": [
    {"user": "原始用户提示词", "assistant": "该轮 AI 最终答复"}
  ]
}
```

这是“FyAgent 兼容的恢复材料”，不是任何产品已经承诺接受的原生导入文件。续聊时可把 turns 重放为新会话的 user/assistant context，或通过 provider 的官方 import（若有）导入后继续；过滤掉工具调用后，文件修改状态、工具返回值、隐含系统提示词和 reasoning 都不可恢复，产品可能无法准确理解旧代码状态。

最低必需字段：有序 user/assistant 文本、provider、导入目标的 model（可在目标机重新选择）、一个本地 source/record ID。建议另存 `created_at/updated_at`、原始 session ID、cwd 的逻辑标识和 exporter/CLI 版本，便于审计与失败回退。不要把认证 token、原始工具输出、完整路径、文件快照放进跨设备 transcript。

## 各产品证据与边界

### OpenCode

官方 v2 CLI 文档页面列出 `session export/import/sanitize/directory`：[OpenCode CLI commands](https://opencode.ai/v2/docs/cli/commands/)。本机 OpenCode 1.18.30 help 核验的是顶层 `opencode export` / `opencode import`；这份报告不把 v2 子命令当作 1.18.30 的可用命令。官方源码 `message.ts` 显示可持久化的 Message 是 `role + parts + metadata`，parts 包含 tool-invocation/file/reasoning 等：[OpenCode message schema](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/message.ts)。

因此 `--sanitize` 只能按字面理解为敏感 transcript/file data 脱敏；页面未说“删除工具事件”，也未证明 sanitize JSON 是 FyAgent 所需的 final-only 结构。官方 import 接受其自身 JSON 文件/URL，未给出可省略字段清单；自行合成最小 JSON 必须做真实合成验证。

### Gemini CLI

官方 session management 文档说明自动保存、`--resume` 按 latest/index/UUID 恢复、session 位于 project hash 目录，并明确项目绑定：[Gemini session management](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/session-management.md)。官方 commands 文档说明 `/chat share` 可导出 Markdown/JSON，且 checkpoint 只能在当前项目恢复：[Gemini CLI commands](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/commands.md)。

官方文档还明确自动历史含工具执行输入/输出、token usage 和（若有）thought summaries；所以不能把原生 session 复制后声称满足 final-only。`--session-file` 仅有本机 help 证据，需主 Agent 用目标版本再核验 JSON schema 与跨目录行为；官方网页没有足够证据证明它是通用导入器。

### Grok Build CLI

xAI 官方 CLI reference 支持 `sessions list/search/delete`、Markdown `export`、`import`（网页文档）、以及 `--resume`、`--continue`、`--fork-session`：[xAI CLI reference](https://docs.x.ai/build/cli/reference)。但本机 Grok Build 1.0.34 的完整 `--help` 未列 `import`；FyAgent 应先运行 capability 检测，不能把网页文档能力直接承诺给已安装版本。官方仓库用户指南给出磁盘布局、`updates.jsonl` 为恢复权威日志、按 encoded cwd 分组，以及 resume/headless/ACP load 语义：[xAI grok-build sessions guide](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/17-sessions.md)。

官方指南明确 session 包含 prompts/responses、tool calls/results、TODO、rewind snapshots、token/turn counters；因此只保留 final answers 不能直接删行后再 resume。Grok 的 `/export` 是 transcript 输出，不是 round-trip import。`grok import` 文档范围是 Claude Code，不能外推到 FyAgent envelope。

### Hermes Agent

NousResearch 官方 sessions 文档明确 SQLite canonical store、表级最小字段和 resume/continue；也明确 Claude/Codex importer 的过滤规则：[Hermes sessions](https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/sessions.md)。官方 developer storage 文档补充 `state.db` 的 sessions/messages/FTS5 架构：[Hermes session storage](https://github.com/NousResearch/hermes-agent/blob/main/website/docs/developer-guide/session-storage.md)。

关键边界：Hermes importer 已证明“有序 user/assistant + 工具活动短注记”的 clean transcript 可进入新 Hermes session；这是最接近 FyAgent 目标的官方先例。但没有证据表明它接受 FyAgent 自定义 JSON，或可以把任意 SQLite 行直接写入后保证升级兼容。memory 是另一层能力：文档描述 reset/expiry 会保存 memories/skills，不能把 session transcript 当作跨会话 memory 文件。

## 跨设备恢复建议与合成验证计划

1. FyAgent 每轮写入上述 envelope，保留原始提示词和最终答复；同时保留 provider/session ID、CLI 版本、逻辑 workspace ID。工具事件只做本地诊断，不进入可迁移 transcript。
2. 目标设备先建立同名/等价工作区、安装匹配 CLI 主版本、重新登录 provider，再导入/恢复。路径、cwd、project hash、encoded cwd 全部视为设备本地映射，不作为唯一恢复凭据。
3. 验证 fixture：3 轮对话（纯文本、包含工具调用的一轮、最后一轮带代码状态说明），生成 native export 与 FyAgent envelope 两份。对每产品分别测试：原生 resume；过滤后 envelope 重建；Mac→Windows 路径映射；错误 model/不存在 cwd；重复导入；缺失工具结果。
4. 通过标准：目标 CLI 启动后能列出预期 session；下一条“请继续上次任务”能看到全部有序 user/final assistant 文本；工具输出和文件快照明确缺失但不会把缺失静默当作已执行；失败时产生可诊断错误而非新建空会话。当前官方资料不足以保证 OpenCode/Gemini/Grok 的“过滤后原生 resume”，应把它们当作“FyAgent transcript 重建新 session”；Hermes 仅对其 Claude/Codex importer 有官方过滤证据。

## 未验证/不要据此承诺

- OpenCode `--sanitize` 是否删除工具事件、是否可直接作为 final-only 输入：未知。
- Gemini `--session-file` 的完整 JSON schema、是否接受 `/chat share *.json`、是否允许跨 project_hash：未知（本机 help 只证明 flag 存在）。
- Grok 任意原生 session 的官方跨设备 import；网页 reference 的 `import` 是否存在于本机 1.0.34；`export` Markdown 是否可再导入：未知/无证据。
- Hermes `state.db` 直接跨 Mac/Windows 复制的 WAL、SQLite 版本和 schema migration 安全性：未知。
- 所有产品的旧版本 session 向新版本迁移、provider model 变更、账号/权限变更后的继续对话保证：未知。

## 本机只读 help 线索（由主 Agent 可复核）

- OpenCode v2 网页文档：`opencode session export/import`；本机 1.18.30：顶层 `opencode export [sessionID] --sanitize`、`opencode import <file>`（JSON file or URL）。两者版本命令不可混用。
- Gemini：`--session-file` “Load a session from a JSON file”。
- Grok：`--resume`、`--fork-session`、`--restore-code`；不带 restore-code 只恢复对话，remote session 的代码恢复需要 worktree 条件。
- Grok 网页 reference 列有 `grok import`，但本机 1.0.34 help 未列出；恢复流程必须先做版本 capability 检测。
- Hermes：`--resume SESSION`、`--no-restore-cwd`、`--in DIR`；本机顶层有 `hermes backup`、`hermes import`，`sessions` 下有 import/export 等但没有 `sessions backup`；裸 `hermes resume` 是暂停恢复语义，不等于会话恢复。
