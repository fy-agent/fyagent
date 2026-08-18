# 本机 Agent 数据盘点（只读）

盘点时间：2026-08-12

用途：用本机真实安装、文件和会话存储校准 Prompt / Memory 产品模型。本文只记录结构、数量和匿名化用途，不收录凭据或私人正文。

## 1. 实际安装

| 工具 | 本机版本 | 长期指导入口 | 记忆与历史入口 |
| --- | --- | --- | --- |
| Codex | `0.140.0` | `~/.codex/AGENTS.md`，项目级 `AGENTS.md` | `session_index.jsonl`、当前/归档 rollout、`memories_1.sqlite`、人工记忆目录 |
| Claude Code | `2.1.220` | `~/.claude/CLAUDE.md`，项目级 `CLAUDE.md` | `~/.claude/memory/*.md`、项目 JSONL、transcripts、history |
| Gemini CLI | `0.46.0` | 本机未发现 `~/.gemini/GEMINI.md` | 8 个项目会话 JSONL、project root 映射 |
| OpenCode | `1.18.5` | 本机未发现全局 `AGENTS.md`；项目可使用 `AGENTS.md` | `MEMORY.md`、3 个日期文件、SQLite session/message/part |
| OpenClaw | `2026.7.1-2` | 每个 workspace 的 `AGENTS.md` / `SOUL.md` 等 | `MEMORY.md`、`memory/*.md`、每个 Agent 的 session/trajectory |
| Hermes | `0.19.0` | `~/.hermes/SOUL.md` | `memories/MEMORY.md`、`memories/USER.md`、JSON session、SQLite/FTS |

## 2. 真实规模与差异

### Codex

- 全局 `AGENTS.md` 已存在，内容不是一条提示词，而是执行规则、Skill 加载、外部工具、记忆、用户偏好、知识路由和周期维护等多个规则域。
- 当前 rollout 约 2736 个，另有 72 个归档；`session_index.jsonl` 的列表字段为 `id / thread_name / updated_at`。
- 机器记忆不是一个 Markdown 文件：`memories_1.sqlite` 包含按 thread 生成的 `raw_memory`、`rollout_summary`、时间和选取状态。
- 结论：Prompt 需要“规则组合”，Memory 需要同时理解索引、会话和派生摘要，不能只映射一个 `MEMORY.md`。

### Claude Code

- 全局 `CLAUDE.md` 已存在，真实规则域包括角色定位、沟通、信息获取、任务执行、分层验收、记忆管理和任务队列。
- `~/.claude/memory/` 目前有 5 个 Markdown：索引、长期记忆、用户画像、执行偏好和跨工具经验。
- 本机还有项目 JSONL、history 和大量 transcript 文件；history 索引含 `display / project / sessionId / timestamp`。
- 结论：用户画像、执行偏好、经验和会话历史是不同对象；前端不应把它们压成一个文本框。

### Gemini CLI

- CLI 已安装并有项目/会话记录，但本机没有 `GEMINI.md`。
- 会话 JSONL 顶层包含 `sessionId / projectHash / startTime / lastUpdated`，消息在增量记录中维护。
- 结论：目标文件需要明确显示“工具已检测、文件未创建”，而不是伪造为已配置。

### OpenCode

- 全局配置目录目前有 `MEMORY.md` 和 3 个日期记忆文件，但没有全局 `AGENTS.md`。
- SQLite 中有 1517 个 session；session 已具备 project、directory、title、agent、model、时间、token、cost 和归档等字段，消息单独存储。
- 本机没有全局 `AGENTS.md` 引用这组 memory 文件。结论：会话浏览应优先读结构化索引；`MEMORY.md` 目前只作为维护来源，不能在没有 adapter/指导引用证据时宣称 OpenCode 会自动读取或把它列为同步目标。

### OpenClaw

- 配置中有 3 个 Agent 实例：`main`、`utility`、`group_liaison`。
- `main` 与 `utility` 共用默认 workspace；`group_liaison` 使用另一套 workspace。因此 3 个实例只对应 2 个实际写入目标，写入时必须按路径去重。
- 默认 workspace 有 7 个核心上下文文件：`AGENTS.md`、`SOUL.md`、`USER.md`、`MEMORY.md`、`IDENTITY.md`、`TOOLS.md`、`HEARTBEAT.md`，另有 87 个每日/专题记忆文件。
- 3 个实例分别约有 54、168、5 个主 session，并有配套 trajectory 与 checkpoint。
- session JSONL 可区分 `session / model_change / thinking_level_change / message` 等事件。
- 结论：产品必须区分“工具”“Agent 实例”“workspace”“目标文件”；OpenClaw 不能只显示成一张应用卡。

### Hermes

- `SOUL.md`、`memories/MEMORY.md`、`memories/USER.md` 均已存在；记忆文件旁有 lock 文件，写入需要处理并发占用。
- 本机有 78 个 SQLite session、84 个 JSON session；session 含来源、模型、标题、时间、工作目录、分支、消息/工具/token/cost 等字段。
- messages 已有 FTS 与 trigram 索引，适合做搜索而不是逐文件扫描全文。
- 结论：Memory 需要搜索、来源和只读/可写能力；后端适配器不能只做 Markdown 文件读写。

## 3. 文件语义边界

本机文件证明“上下文”至少有五种语义，前端必须显示用途，不能只按扩展名分类：

| 语义 | 真实例子 | 页面归属 |
| --- | --- | --- |
| 行事规则与工作流 | `AGENTS.md`、`CLAUDE.md`、`GEMINI.md` | Prompt 主责 |
| 身份与表达方式 | `SOUL.md`、`IDENTITY.md` | Prompt 主责；Memory 只显示来源关系 |
| 用户资料与稳定偏好 | `USER.md`、Claude `user_profile.md` | Memory |
| 长期事实、决策、经验 | `MEMORY.md`、分类记忆 Markdown、机器摘要 | Memory |
| 每日/会话痕迹 | `memory/YYYY-MM-DD.md`、JSON/JSONL、SQLite session/message | Memory |
| 工具与定时运行上下文 | `TOOLS.md`、`HEARTBEAT.md` | Prompt 的高级规则；不默认参与跨 Agent 同步 |

## 4. 对前端模型的直接修订

1. Prompt 的右栏不再叫“应用范围”，改为“注入目标”。每项展示 Agent/实例、目标文件、是否存在、共享 workspace 情况。
2. Prompt 库增加 `规则类型` 与 `来源`，优先使用本机已有规则域提炼的匿名化模板；不把私人原文复制进产品种子。
3. 多条 Prompt 可同时启用；同一目标文件按稳定顺序组合。多个实例共用同一路径时只写一次。
4. Memory 的顶层使用用户能直接理解的 `长期记忆 / 每日记录 / 会话记录`，不再强行使用抽象的“共享/原生/痕迹”分类。
5. Memory 左栏展示真实来源；中栏预览或编辑实际内容；右栏显示来源、存储形式、读写能力和同步目标。
6. `SOUL/AGENTS/TOOLS/HEARTBEAT` 即使在 Memory 来源扫描中被发现，也标明“由 Prompt 管理”，避免两个页面同时改同一文件。
7. “同步”必须有来源引用、目标引用、内容摘要、最后修改时间、冲突状态和逐目标结果；本地保存不能冒充同步成功。

## 5. 后端适配器最小字段

```ts
type LocalAgentSource = {
  toolId: string;
  instanceId: string;
  instanceName: string;
  workspaceId: string | null;
  workspacePath: string | null;
  version: string | null;
  detected: boolean;
};

type LocalContextResource = {
  id: string;
  toolId: string;
  instanceIds: string[];
  semanticType:
    | "instruction"
    | "identity"
    | "user_profile"
    | "long_term_memory"
    | "daily_memory"
    | "session_store"
    | "operational_context";
  storageKind: "markdown" | "json" | "jsonl" | "sqlite";
  path: string;
  exists: boolean;
  readable: boolean;
  writable: boolean;
  searchable: boolean;
  itemCount: number | null;
  updatedAt: string | null;
};
```

这些字段应由扫描/适配层返回。前端不应硬编码本机绝对路径、会话数量或“已同步”状态。
