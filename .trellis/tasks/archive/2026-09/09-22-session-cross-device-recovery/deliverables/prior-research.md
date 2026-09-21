# FyAgent Session 跨设备恢复：既有调研回读

> 此文是本轮开始时的历史索引。后续 Codex 持久化、重启及请求捕获实验见协调交付说明；“未做”仅指历史调研时。

研究日期：2026-09-22  
范围：只读回 fyagent 既有文档、Trellis 任务资料与相关 Codex 历史任务；未改 fyagent 源码、未提交、未发布。

## 一、当前状态

- 当前任务 [09-22-session-cross-device-recovery](/Users/serendipity/fyagent/.trellis/tasks/09-22-session-cross-device-recovery/prd.md) 仍处于 planning，`prd.md` 和 `task.json` 的 Goal/Requirements/Acceptance Criteria 都是 TBD；说明实施尚未正式拆解或开始。
- 既有主研究 [恢复方案.md](/Users/serendipity/fyagent/research/session-recovery-20260921/%E6%81%A2%E5%A4%8D%E6%96%B9%E6%A1%88.md) 标明“调研完成，尚未实施产品改造”，是目前最接近决策基线的文件。
- 代码基础研究 [session-feasibility.md](/Users/serendipity/fyagent/.trellis/tasks/09-21-remove-client-projects/research/session-feasibility.md) 判定：已有七个 provider 的本地读取/预览/部分 resume launcher，但没有跨设备 Session 持久化、统一导出导入协议或可靠 final-only 提取器；现有配置备份/WebDAV 同步不覆盖原生 session 文件与 SQLite。

## 二、已形成的产品决定（可作为架构约束）

1. Session 作为 FyAgent 的一级产品模块，目标是把过去对话带到另一台设备，并在目标 AI 软件中继续。
2. 可迁移内容严格定义为：用户原始输入 + 每轮 AI 最终答复原文，保留角色、顺序、消息标识、来源、原始 session 标识、标题、时间和格式版本。
3. 迁移包不包含工具调用、参数、工具结果、终端输出、思考过程、进度播报、附件、工作目录文件，也不通过隐藏备份夹带完整原生日志。用户主动粘贴文本和最终答复中的代码示例属于正文，保留。
4. 采用“统一精简会话记录 → 各软件的原生会话重建”：源读取与 final-only 判定 → FyAgent 中间 envelope → 目标设备选择软件/工作目录/账号/模型 → provider adapter 重建 → 打开并发送下一条消息验证。
5. 目标设备原生 session ID 可以变化；产品要保留 source-to-target 映射，不能把原 ID 保留作为跨设备成功条件。
6. “导入成功”“目标软件已打开”“发送下一条消息且上下文连续”是三个不同状态；只有最后一项通过才能称为可恢复。可查看、可复制、可粘贴摘要都不能标成 native resume。
7. 旧 MEMORY.md、USER.md 等长期记忆文件不因 Session 入口替换而自动删除，也不把旧记忆转换成 Session；记忆是并行能力。

建议的 FyAgent envelope（研究稿中的 v1 草案）是 `schema`、`provider`、`source_session_id`、可选 `title/cwd/model`、有序 `turns[{user,assistant}]`。cwd 只能作为可选逻辑工作区标识，不应成为 transcript-only resume 的必需凭据；不得写入 token、原始工具输出、完整本地路径或文件快照。

## 三、按 provider 的证据、结论与缺口

| Provider / 调研版本 | 已有路线与证据 | 当前可宣称程度 | 必须补的验证 |
|---|---|---|---|
| Codex 0.154.0 | App Server 有 `thread/start`、`thread/inject_items`、`turn/start`；官方说明注入 items 会进入后续 model-visible history 并持久化。`thread/resume` 适合原生已有 thread。schema 中 `resume.history` 标记 `[UNSTABLE]`、`DO NOT USE`。 | “新 thread 注入精简 user/assistant 后继续”是最明确的公开重建路线；不是原 thread 原地恢复。 | 运行合成 fixture：注入、读回、重启、下一轮引用历史；跨 Mac/Windows 的 cwd 映射和账号边界。 |
| OpenCode 1.18.30 | 隔离合成测试已用本机原生 JSON `import/export` 导入四条 user/final assistant 消息，角色/顺序保留。 | 局部导入/读回可行；尚未证明下一轮模型回复与跨 OS。 | 下一轮真实续聊、重启、重复导入/ID 冲突、旧路径元数据处理。不要把新版 v2 子命令当作 1.18.30 能力。 |
| Hermes v0.20.5（本地含补丁） | 合成 Codex 格式问答经 `sessions import` 写入 Hermes SessionDB，读回并离线追加用户消息成功；官方 importer 会将工具活动压成 assistant 内短注记。 | 最接近 final-only 目标的官方先例，但只明确支持 Claude/Codex importer，非 FyAgent 自定义 JSON 契约。 | 真实下一轮模型回复、过滤 progress/tool 注记、目标版本兼容、跨 OS 导入而非直接复制 WAL/SQLite。 |
| Gemini CLI 0.59.0 | 本机 help 有 `--resume`、`--session-file`；官方自动 session 绑定 project hash，内容含 prompts/responses/tool executions/token stats/thought summaries。 | `--session-file` 的精简 JSON 契约未证明，不能承诺。 | 目标版本 JSON schema、跨项目/跨目录、错误 cwd/model、重启与下一轮续聊。 |
| Claude Code 2.1.220 | `/resume` 使用完整本地 JSONL transcript；官方 `/export` 是纯文本；`/import` 是配置（MCP、commands、skills 等），不是 transcript。云端 `--teleport` 要求同账号、同仓库、干净 git 状态。 | 原生完整 transcript 恢复有证据；final-only 精简 JSONL 没有公开导入契约，仍待验证。 | 合成完整 JSONL 与过滤副本的 `/resume` 对比；云端 teleport 不能当作 FyAgent envelope 导入；注意 plaintext transcript 隐私与默认保留期。 |
| Grok Build 1.0.34 | 本机支持 `--resume`、`--fork-session`、`--restore-code`；官方用户指南将 `updates.jsonl` 作为恢复权威日志，含 tool calls/results、TODO、snapshots 等。网页 reference 列 `import`，但本机 help 未列。 | 必须先 capability detect；不能承诺网页 `import` 对本机可用。Markdown export 不是可回灌格式；过滤事件会破坏 ACP 序列。 | 本机版本的 import 能力、官方支持的重建入口、只恢复对话 vs restore-code、跨 OS encoded cwd。 |
| OpenClaw 2026.7.1-2 | 原生 SQLite/Gateway/session key 有所有权与 provenance 约束；`sessions_history(includeTools=false)` 是展示投影；Session Share 明确只读，不能 continuation/terminal/worker execution。 | 可过滤读取，不等于可重建；无公开 FyAgent 自定义 transcript 导入。 | 按目标 Gateway/安装版核验受支持写入接口；不能直接改 SQLite 或把只读 Session Share 算恢复。 |

来源依据集中在 [cli-formats.md](/Users/serendipity/fyagent/research/session-recovery-20260921/cli-formats.md)、[codex-claude.md](/Users/serendipity/fyagent/research/session-recovery-20260921/codex-claude.md)、[local-cli-evidence.json](/Users/serendipity/fyagent/research/session-recovery-20260921/local-cli-evidence.json)、[codex-protocol-excerpt.json](/Users/serendipity/fyagent/research/session-recovery-20260921/codex-protocol-excerpt.json)、[opencode-synthetic-result.json](/Users/serendipity/fyagent/research/session-recovery-20260921/opencode-synthetic-result.json) 和 [hermes-synthetic-result.json](/Users/serendipity/fyagent/research/session-recovery-20260921/hermes-synthetic-result.json)。

## 四、已做实测与未做实测

已做：OpenCode 隔离导入/导出/读回；Hermes 合成 Codex 格式导入、SessionDB 读回、离线追加用户消息；本机各 CLI help/version 与 Codex 生成 schema 核对。两项实测都只用临时合成数据，没有读取真实会话或调用模型，测试存储已清理。

未做：Codex App Server 注入全链路；所有 provider 的真实下一轮模型回复；Mac↔Windows 双向矩阵；路径映射；重启持久性；重复导入；不支持版本错误态；凭据/账号/模型缺失；完整 UI 读回；OpenClaw 受支持重建；Claude/Grok/Gemini 的精简原生输入契约。

## 五、既有代码基线与实施缺口

- `src-tauri/src/session_manager/` 已有 Codex、Claude、OpenCode、OpenClaw、Gemini、Hermes、Grok Build 的读取 adapter；可复用扫描、标题/时间归一化、基础角色文本解析和部分 resume command。
- 公共 `SessionMeta`/`SessionMessage` 只有 provider/session id、标题、目录、时间、source path、resume command 与 role/content/ts；没有 message/turn ID、final 标记、完成状态、可信来源元数据，因此不能直接支撑 final-only 迁移。
- 现有解析会暴露 tool/function call、tool output 或递归嵌入文本；“只筛 assistant”也不能证明是每轮最终答复。
- 终端 launcher 当前明确只支持 macOS；`source_path` 是本机文件/SQLite 引用，不能作为跨设备稳定引用。
- Tauri 有 list/get/delete/launch 命令，但旧研究对前端的判断已过时：当前 v1 存在 src/components/sessions/SessionManagerPage.tsx；v2 仍缺少本轮要求的跨设备导出/导入与恢复流程。统一迁移协议仍未实现。

因此建议拆为：数据模型与 final-only 判定、provider adapter/能力矩阵、导出导入与冲突幂等、目录映射与恢复状态、跨平台 launcher、Session 列表/详情/迁移 UI、测试矩阵与隐私审计。先做 Codex/OpenCode 架构闭环，再做 Hermes/Gemini，最后处理 Claude/Grok/OpenClaw 的版本限定路线。

## 六、历史 Codex 任务核对

- Codex 任务 `01a0c39c-e605-7051-905f-9cf38936e5d2` 的回读明确写出：“Session 部分已完成恢复方案调研，尚未开始实现”；该任务同时记录了客户项目任务的状态，不能把客户项目进度误作 Session 实施进度。
- Codex 任务 `01a086ce-0347-7ce3-827a-018f8952006a` 主要是 Issue #52 Agent Health Center，和 Session 方案无直接实施关系；其中出现的多 Agent/Trellis 经验不能替代本次 Session 验收证据。
- 当前 Codex 任务的用户目标是进行 Session 方案实施准备、A2A 分工和 Grok 冗余评审；历史任务支持“研究完成、实施未开始”的判断，但没有可直接合并的 Session 代码交付。

## 七、主线程应保留的关键决策与风险

已定：精简可迁移内容边界、统一 envelope 思路、provider 原生重建、原生 ID 可变化、导入/打开/续聊分级、记忆与 Session 分离、不得隐藏携带完整原生日志。

未定：首个可交付 provider 范围；final answer 判定规则与无法判定时的用户决策；envelope 是否加 schema hash/checksum/encryption；导入幂等键和冲突 UI；目录映射交互；各 provider 的 capability matrix 与失败状态；是否先做“导出/导入材料”再做“原生续聊”；Windows launcher；保留期与敏感数据提示。

过时或需谨慎使用：早期“只做查看/复制”“隐藏完整原生状态备份”“保留独立记忆一级模块”的建议已被 09-21 方案明确裁定不采用；但这不等于允许删除原生运行时状态，产品只是不把它放入精简迁移包。Codex `history.jsonl` 也不能替代完整 rollout。

### 证据边界

以上 provider 能力判断以调研时版本与官方公开资料为准；网页写出的新版本能力不能外推到本机安装版。没有公开导入契约只表示“未验证/不可承诺”，不应写成技术上不可能。所有跨设备成功结论必须以合成数据的下一轮回复、重启和双向 OS 验证为证据。
