import type {
  AgentTargetId,
  AgentToolId,
} from "@/v2/shared/config/agentTargets";

export type MemoryCategory = "longTerm" | "daily" | "sessions";
export type MemoryStorageKind =
  | "Markdown"
  | "JSON"
  | "JSONL"
  | "SQLite"
  | "SQLite + Markdown"
  | "SQLite + JSON";

export type MemoryResourceState = "exists" | "missing" | "frontend-draft";

export type MemoryLocalState =
  | "source"
  | "saved-preview"
  | "changes-pending"
  | "managed-by-prompts";

export interface MemoryProvenance {
  sourceItemId: string;
  sourceTargetId: AgentTargetId;
  sourceToolId: AgentToolId;
  sourcePath: string;
  sourceUpdatedAt: string;
  capturedAt: string;
  sourceSummary: string;
}

export interface MemoryPreviewTargetTask {
  targetId: AgentTargetId;
  sourceRevision: number;
  previewState: "pending";
  durableState: "not-run";
  createdAt: string;
  error: null;
}

export interface MemoryPrototypeItem {
  id: string;
  category: MemoryCategory;
  toolId: AgentToolId;
  sourceTargetId: AgentTargetId;
  title: string;
  sourceLabel: string;
  purpose: string;
  path: string;
  storageKind: MemoryStorageKind;
  content: string;
  writable: boolean;
  editableInPrototype: boolean;
  searchable: boolean;
  itemCount: number | null;
  updatedAt: string;
  resourceState: MemoryResourceState;
  localState: MemoryLocalState;
  revision: number;
  provenance: MemoryProvenance | null;
  syncTargetIds: AgentTargetId[];
  previewTasks: MemoryPreviewTargetTask[];
  owner: "memory" | "prompts";
}

type MemoryPrototypeSeed = Omit<
  MemoryPrototypeItem,
  | "editableInPrototype"
  | "resourceState"
  | "localState"
  | "revision"
  | "provenance"
  | "syncTargetIds"
  | "previewTasks"
> & {
  syncTargetIds: readonly AgentTargetId[];
} & Partial<
    Pick<
      MemoryPrototypeItem,
      | "editableInPrototype"
      | "resourceState"
      | "localState"
      | "revision"
      | "provenance"
      | "previewTasks"
    >
  >;

function createMemoryPrototypeItem(
  seed: MemoryPrototypeSeed,
): MemoryPrototypeItem {
  return {
    ...seed,
    editableInPrototype:
      seed.editableInPrototype ??
      (seed.category === "longTerm" &&
        seed.owner === "memory" &&
        seed.writable),
    resourceState: seed.resourceState ?? "exists",
    localState:
      seed.localState ??
      (seed.owner === "prompts"
        ? "managed-by-prompts"
        : seed.category === "longTerm" && seed.syncTargetIds.length > 0
          ? "saved-preview"
          : "source"),
    revision: seed.revision ?? 1,
    provenance: seed.provenance ?? null,
    syncTargetIds: [...seed.syncTargetIds],
    previewTasks: seed.previewTasks
      ? seed.previewTasks.map((task) => ({ ...task }))
      : [],
  };
}

const longTermMemorySeeds = [
  {
    id: "codex-derived-memory",
    category: "longTerm",
    toolId: "codex",
    sourceTargetId: "codex-global",
    title: "Codex · 派生记忆",
    sourceLabel: "Codex 全局",
    purpose: "从会话自动提炼的摘要与长期记忆",
    path: "~/.codex/memories/ + memories_1.sqlite",
    storageKind: "SQLite + Markdown",
    content:
      "# Codex 派生记忆\n\n本机已发现会话级摘要、原始记忆、更新时间与二阶段选取状态。\n\n此处先展示存储结构；真实内容需要通过 Codex 记忆索引读取。",
    writable: false,
    searchable: true,
    itemCount: 502,
    updatedAt: "今天 20:43",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "gemini-long-term-missing",
    category: "longTerm",
    toolId: "gemini",
    sourceTargetId: "gemini-global",
    title: "Gemini CLI · 长期记忆入口",
    sourceLabel: "Gemini CLI 全局",
    purpose: "当前扫描快照未发现可验证的原生长期记忆文件",
    path: "~/.gemini/（未发现长期记忆文件）",
    storageKind: "Markdown",
    content:
      "# 未发现长期记忆入口\n\n当前只发现项目会话记录；此项用于明确缺失状态，不代表已创建文件或可同步目标。",
    writable: false,
    searchable: false,
    itemCount: 0,
    updatedAt: "扫描快照",
    resourceState: "missing",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "claude-long-term",
    category: "longTerm",
    toolId: "claude",
    sourceTargetId: "claude-global",
    title: "Claude Code · 长期记忆",
    sourceLabel: "Claude Code 全局",
    purpose: "稳定偏好、经验教训与重要决定",
    path: "~/.claude/memory/MEMORY.md",
    storageKind: "Markdown",
    content:
      "# 长期记忆\n\n- 稳定偏好与工具定位\n- 重要经验与反复出现的问题\n- 已确认的关键决定\n\n本原型只保留结构，不复制本机私人正文。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "03-30 01:23",
    syncTargetIds: ["claude-global"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "claude-user-profile",
    category: "longTerm",
    toolId: "claude",
    sourceTargetId: "claude-global",
    title: "Claude Code · 用户画像",
    sourceLabel: "Claude Code 全局",
    purpose: "用户角色、工作偏好与沟通方式",
    path: "~/.claude/memory/user_profile.md",
    storageKind: "Markdown",
    content:
      "# 用户画像\n\n- 角色与工作方式\n- 工具分工\n- 沟通和决策偏好\n\n同步时应映射到目标 Agent 的用户资料区域，而不是写入行为规则。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "03-30 01:23",
    syncTargetIds: ["claude-global", "openclaw-default", "hermes-global"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "opencode-memory",
    category: "longTerm",
    toolId: "opencode",
    sourceTargetId: "opencode-global",
    title: "OpenCode · 维护记忆",
    sourceLabel: "OpenCode 全局",
    purpose: "版本、配置、故障与运行验收经验",
    path: "~/.config/opencode/MEMORY.md",
    storageKind: "Markdown",
    content:
      "# OpenCode Memory\n\n本机文件按日期记录工具升级、配置变化、故障根因与验证结果。\n\n这类记忆适合保留在工具范围，不应默认同步为所有 Agent 的用户偏好。",
    writable: true,
    editableInPrototype: false,
    searchable: true,
    itemCount: 3,
    updatedAt: "07-26 03:07",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "openclaw-context-index",
    category: "longTerm",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · 上下文档案",
    sourceLabel: "默认工作区 · main + utility",
    purpose: "7 个核心上下文文件及各自职责",
    path: "~/.openclaw/workspace/",
    storageKind: "Markdown",
    content:
      "# OpenClaw 上下文档案\n\n- AGENTS.md：工作规则\n- SOUL.md / IDENTITY.md：身份与表达\n- USER.md：用户资料\n- MEMORY.md：长期记忆\n- TOOLS.md / HEARTBEAT.md：工具与定时运行\n\n规则类文件由提示词页面负责，避免两处同时修改。",
    writable: false,
    searchable: true,
    itemCount: 7,
    updatedAt: "今天 03:01",
    syncTargetIds: [],
    localState: "managed-by-prompts",
    owner: "prompts",
  },
  {
    id: "openclaw-memory",
    category: "longTerm",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · 长期记忆",
    sourceLabel: "默认工作区 · main + utility",
    purpose: "身份、决定、教训、项目状态与偏好索引",
    path: "~/.openclaw/workspace/MEMORY.md",
    storageKind: "Markdown",
    content:
      "# 长期记忆骨架\n\n- 核心身份\n- 关键决定\n- 重要教训\n- 当前项目状态\n- 偏好摘要\n- 记忆文件索引",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "08-06 16:50",
    syncTargetIds: ["openclaw-default"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "openclaw-user",
    category: "longTerm",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · 用户资料",
    sourceLabel: "默认工作区 · main + utility",
    purpose: "关于用户的稳定背景与沟通偏好",
    path: "~/.openclaw/workspace/USER.md",
    storageKind: "Markdown",
    content:
      "# USER.md\n\n此文件保存稳定的用户背景和协作偏好。同步时只进入其他 Agent 的用户资料区域。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "07-27 03:26",
    syncTargetIds: ["openclaw-default", "claude-global", "hermes-global"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "openclaw-group-memory",
    category: "longTerm",
    toolId: "openclaw",
    sourceTargetId: "openclaw-group",
    title: "OpenClaw · 群聊长期记忆",
    sourceLabel: "群聊工作区 · group_liaison",
    purpose: "群聊前台独立的长期上下文",
    path: "~/.openclaw/workspace-group_liaison/MEMORY.md",
    storageKind: "Markdown",
    content:
      "# 群聊长期记忆\n\n此工作区与 main / utility 分离，应独立读取和同步，不能由默认工作区覆盖。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "06-17 23:36",
    syncTargetIds: ["openclaw-group"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "hermes-memory",
    category: "longTerm",
    toolId: "hermes",
    sourceTargetId: "hermes-global",
    title: "Hermes · 长期记忆",
    sourceLabel: "Hermes 全局",
    purpose: "长期环境、决定与执行经验",
    path: "~/.hermes/memories/MEMORY.md",
    storageKind: "Markdown",
    content:
      "# Hermes Memory\n\n长期环境、关键决定和可复用经验保存在此。旁边存在 lock 文件，写入前必须检查占用。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "07-20 23:33",
    syncTargetIds: ["hermes-global"],
    localState: "saved-preview",
    owner: "memory",
  },
  {
    id: "hermes-user",
    category: "longTerm",
    toolId: "hermes",
    sourceTargetId: "hermes-global",
    title: "Hermes · 用户资料",
    sourceLabel: "Hermes 全局",
    purpose: "用户偏好、工作模式和优先级",
    path: "~/.hermes/memories/USER.md",
    storageKind: "Markdown",
    content:
      "# Hermes User\n\n用户偏好、工作模式和优先级单独保存，不与 Agent 身份或会话原文混写。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "04-18 10:27",
    syncTargetIds: ["hermes-global", "claude-global", "openclaw-default"],
    localState: "saved-preview",
    owner: "memory",
  },
] as const satisfies readonly MemoryPrototypeSeed[];

export const longTermMemoryItems: readonly MemoryPrototypeItem[] =
  longTermMemorySeeds.map(createMemoryPrototypeItem);

const dailyMemorySeeds = [
  {
    id: "openclaw-daily-latest",
    category: "daily",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · 今日记录",
    sourceLabel: "默认工作区 · 87 个日期/专题文件",
    purpose: "按日期保存的工作痕迹与短期上下文",
    path: "~/.openclaw/workspace/memory/2026-08-12.md",
    storageKind: "Markdown",
    content:
      "# 2026-08-12\n\n本机已发现今日记录。当前原型只展示文件结构；联调后从实际文件读取正文。",
    writable: true,
    searchable: true,
    itemCount: 87,
    updatedAt: "今天 20:47",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "opencode-daily-latest",
    category: "daily",
    toolId: "opencode",
    sourceTargetId: "opencode-global",
    title: "OpenCode · 最近维护记录",
    sourceLabel: "OpenCode · 3 个日期文件",
    purpose: "单次维护的结果、证据与遗留事项",
    path: "~/.config/opencode/memory/2026-07-26.md",
    storageKind: "Markdown",
    content:
      "# OpenCode maintenance\n\n记录一次工具维护的结果、问题根因、验证证据和遗留事项。",
    writable: true,
    searchable: true,
    itemCount: 3,
    updatedAt: "07-26 03:07",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "openclaw-group-daily",
    category: "daily",
    toolId: "openclaw",
    sourceTargetId: "openclaw-group",
    title: "OpenClaw · 群聊记录",
    sourceLabel: "群聊工作区 · 1 个日期文件",
    purpose: "群聊 Agent 独立产生的每日痕迹",
    path: "~/.openclaw/workspace-group_liaison/memory/2026-06-17.md",
    storageKind: "Markdown",
    content: "# 群聊记录\n\n群聊前台的每日记录与默认工作区分开保存。",
    writable: true,
    searchable: true,
    itemCount: 1,
    updatedAt: "06-17 23:36",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
] as const satisfies readonly MemoryPrototypeSeed[];

export const dailyMemoryItems: readonly MemoryPrototypeItem[] =
  dailyMemorySeeds.map(createMemoryPrototypeItem);

const sessionSourceSeeds = [
  {
    id: "codex-sessions",
    category: "sessions",
    toolId: "codex",
    sourceTargetId: "codex-global",
    title: "Codex · 任务与会话",
    sourceLabel: "2736 当前 · 72 归档",
    purpose: "任务索引、rollout 与派生摘要",
    path: "~/.codex/session_index.jsonl + sessions/",
    storageKind: "JSONL",
    content:
      "# Codex 会话来源\n\n索引字段：id、thread_name、updated_at。\n原始 rollout 与派生记忆分开存储，可先按索引筛选再读取正文。",
    writable: false,
    searchable: true,
    itemCount: 2808,
    updatedAt: "今天 20:48",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "claude-sessions",
    category: "sessions",
    toolId: "claude",
    sourceTargetId: "claude-global",
    title: "Claude Code · 项目会话",
    sourceLabel: "1347 transcripts · 3 项目 JSONL",
    purpose: "项目会话、历史输入与转录",
    path: "~/.claude/projects/ + transcripts/",
    storageKind: "JSONL",
    content:
      "# Claude 会话来源\n\nhistory 索引提供 display、project、sessionId、timestamp；项目记录和 transcripts 需要分别接入。",
    writable: false,
    searchable: true,
    itemCount: 1350,
    updatedAt: "07-27 00:46",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "gemini-sessions",
    category: "sessions",
    toolId: "gemini",
    sourceTargetId: "gemini-global",
    title: "Gemini CLI · 项目会话",
    sourceLabel: "8 个 JSONL 会话",
    purpose: "按 project root 分组的 CLI 会话",
    path: "~/.gemini/tmp/*/chats/",
    storageKind: "JSONL",
    content:
      "# Gemini CLI 会话来源\n\n会话元数据包括 sessionId、projectHash、startTime、lastUpdated；消息以增量记录保存。",
    writable: false,
    searchable: false,
    itemCount: 8,
    updatedAt: "07-12 04:39",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "opencode-sessions",
    category: "sessions",
    toolId: "opencode",
    sourceTargetId: "opencode-global",
    title: "OpenCode · 会话数据库",
    sourceLabel: "1517 个 SQLite session",
    purpose: "项目、会话、消息、模型与成本索引",
    path: "~/.local/share/opencode/opencode.db",
    storageKind: "SQLite",
    content:
      "# OpenCode 会话来源\n\n可用字段：project、directory、title、agent、model、创建/更新时间、token、cost 与归档状态。\n消息和内容块使用独立表。",
    writable: false,
    searchable: true,
    itemCount: 1517,
    updatedAt: "今天",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "openclaw-main-sessions",
    category: "sessions",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · main 会话",
    sourceLabel: "54 个主会话 + trajectory",
    purpose: "主 Agent 的消息、模型变化与执行轨迹",
    path: "~/.openclaw/agents/main/sessions/",
    storageKind: "JSONL",
    content:
      "# OpenClaw main 会话\n\n事件类型包括 session、model_change、thinking_level_change 与 message；trajectory 和 checkpoint 需要独立标识。",
    writable: false,
    searchable: true,
    itemCount: 54,
    updatedAt: "今天",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "openclaw-utility-sessions",
    category: "sessions",
    toolId: "openclaw",
    sourceTargetId: "openclaw-default",
    title: "OpenClaw · utility 会话",
    sourceLabel: "168 个主会话 + trajectory",
    purpose: "信息采集 Agent 的独立会话与轨迹",
    path: "~/.openclaw/agents/utility/sessions/",
    storageKind: "JSONL",
    content:
      "# OpenClaw utility 会话\n\n会话独立保存，但 Prompt 与长期记忆目标和 main 共用默认 workspace。",
    writable: false,
    searchable: true,
    itemCount: 168,
    updatedAt: "今天",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "openclaw-group-sessions",
    category: "sessions",
    toolId: "openclaw",
    sourceTargetId: "openclaw-group",
    title: "OpenClaw · group_liaison 会话",
    sourceLabel: "5 个主会话 + trajectory",
    purpose: "群聊前台的独立会话与轨迹",
    path: "~/.openclaw/agents/group_liaison/sessions/",
    storageKind: "JSONL",
    content:
      "# OpenClaw group_liaison 会话\n\n此实例使用独立 workspace，不能与 main / utility 合并来源。",
    writable: false,
    searchable: true,
    itemCount: 5,
    updatedAt: "08-11",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
  {
    id: "hermes-sessions",
    category: "sessions",
    toolId: "hermes",
    sourceTargetId: "hermes-global",
    title: "Hermes · 会话与全文索引",
    sourceLabel: "78 个 SQLite session · 84 个 JSON session",
    purpose: "会话、消息、工具、token、成本与全文搜索",
    path: "~/.hermes/state.db + sessions/",
    storageKind: "SQLite + JSON",
    content:
      "# Hermes 会话来源\n\nSQLite 已包含 sessions、messages、FTS 与 trigram 索引；会话还保留 JSON 文件。\n读取时应以索引为入口，并标明数据库与文件数量差异。",
    writable: false,
    searchable: true,
    itemCount: 78,
    updatedAt: "08-11 18:59",
    syncTargetIds: [],
    localState: "source",
    owner: "memory",
  },
] as const satisfies readonly MemoryPrototypeSeed[];

export const sessionSourceItems: readonly MemoryPrototypeItem[] =
  sessionSourceSeeds.map(createMemoryPrototypeItem);
