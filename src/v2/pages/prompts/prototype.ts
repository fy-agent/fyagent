import {
  allAgentTargetIds,
  codingAgentTargetIds,
  memoryAwareTargetIds,
  type AgentTargetId,
} from "@/v2/shared/config/agentTargets";

export type PromptCategory =
  | "通用规则"
  | "角色与表达"
  | "执行流程"
  | "验收规则"
  | "记忆规则"
  | "自动运行"
  | "自定义";

export interface PromptPrototypeItem {
  id: string;
  name: string;
  description: string;
  content: string;
  enabled: boolean;
  kind: "builtin" | "custom";
  category: PromptCategory;
  origin: "本机规则提炼" | "官方基础模板" | "用户创建";
  targetIds: AgentTargetId[];
  updatedAt: string;
}

const targets = (ids: readonly AgentTargetId[]): AgentTargetId[] => [...ids];

export const promptPrototypeItems: readonly PromptPrototypeItem[] = [
  {
    id: "chinese-communication",
    name: "中文与回复风格",
    description: "中文优先、先说结论、减少空话",
    content:
      "# 沟通与回复\n\n- 默认使用简体中文；技术名词、命令和代码标识在更准确时保留英文。\n- 先给结论和下一步，再补充必要依据。\n- 需要提问时一次问清会改变方案的关键问题，避免多轮追问。",
    enabled: true,
    kind: "builtin",
    category: "角色与表达",
    origin: "本机规则提炼",
    targetIds: targets(allAgentTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "goal-boundary-evidence",
    name: "目标、边界与完成证据",
    description: "先确认要做什么，再用真实结果收口",
    content:
      "# 任务闭环\n\n- 开始前确认目标、范围、依赖和验收条件。\n- 优先读取项目已有规则与真实代码，不凭空补全关键事实。\n- 声称完成前运行与风险相称的验证，并说明本轮真实结果。",
    enabled: true,
    kind: "builtin",
    category: "验收规则",
    origin: "本机规则提炼",
    targetIds: targets(allAgentTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "context-first",
    name: "先读项目上下文",
    description: "按全局与项目层级加载规则",
    content:
      "# 上下文加载\n\n- 会话开始时先读取当前目录与上级目录生效的指导文件。\n- 项目规则优先于通用习惯；发现冲突时说明采用了哪一层。\n- 只加载当前任务需要的规则、记忆和工具说明。",
    enabled: false,
    kind: "builtin",
    category: "通用规则",
    origin: "本机规则提炼",
    targetIds: targets(allAgentTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "planning-design",
    name: "规划与方案对比",
    description: "先搭框架，比较方案后再执行",
    content:
      "# 规划与设计\n\n- 先明确用户问题、使用场景和成功标准。\n- 对非显然选择至少比较可行方案、代价与风险。\n- 计划必须写出外部依赖、关键分岔和可验证的下一步。",
    enabled: false,
    kind: "builtin",
    category: "执行流程",
    origin: "本机规则提炼",
    targetIds: ["claude-global", "codex-global"],
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "implementation-discipline",
    name: "最小改动与真实验证",
    description: "遵循现有架构，只改完成任务所需内容",
    content:
      "# 代码实现\n\n- 修改前搜索已有实现、类型、测试和约定，优先复用。\n- 做最小且完整的改动，保护与任务无关的现有内容。\n- 先运行最接近改动的检查，再覆盖受影响范围。",
    enabled: false,
    kind: "builtin",
    category: "执行流程",
    origin: "本机规则提炼",
    targetIds: targets(codingAgentTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "code-review",
    name: "代码审查",
    description: "优先发现正确性、回归与数据风险",
    content:
      "# 代码审查\n\n- 优先检查正确性、数据丢失、竞态、兼容性和回归风险。\n- 只报告能够从代码或运行结果证明的问题。\n- 每个问题说明影响、触发条件、准确位置和修复方向。",
    enabled: false,
    kind: "builtin",
    category: "验收规则",
    origin: "官方基础模板",
    targetIds: targets(codingAgentTargetIds),
    updatedAt: "内置 v1",
  },
  {
    id: "memory-continuity",
    name: "记忆读取与写回",
    description: "先读稳定记忆，重要变化及时记录",
    content:
      "# 记忆连续性\n\n- 行动前读取与当前任务相关的用户偏好、决策和历史教训。\n- 重要决定、重复失败模式或用户明确要求记住的内容，写入对应的长期记忆。\n- 每日记录、会话原文和长期结论分开保存，不用原始聊天替代结论。",
    enabled: false,
    kind: "builtin",
    category: "记忆规则",
    origin: "本机规则提炼",
    targetIds: targets(memoryAwareTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "tool-troubleshooting",
    name: "工具失败先定位根因",
    description: "保留错误证据，逐层确认真实阻塞",
    content:
      "# 工具与故障\n\n- 先保留准确错误、输入与环境，再判断是配置、权限、网络还是运行时问题。\n- 不用重复重试代替诊断；每次尝试都应验证一个明确假设。\n- 修复后重跑原失败路径，并记录可复用的预防方法。",
    enabled: false,
    kind: "builtin",
    category: "执行流程",
    origin: "本机规则提炼",
    targetIds: targets(allAgentTargetIds),
    updatedAt: "本机扫描 · 08-12",
  },
  {
    id: "heartbeat-boundary",
    name: "定时任务与心跳边界",
    description: "只在有变化或到期时主动运行",
    content:
      "# 定时与心跳\n\n- 每次心跳先读取上次状态，只处理新增、到期或失败的事项。\n- 没有实质变化时保持简短，不制造重复提醒。\n- 自动任务的写入、外部发送和高风险动作仍遵守明确授权边界。",
    enabled: false,
    kind: "builtin",
    category: "自动运行",
    origin: "本机规则提炼",
    targetIds: ["openclaw-default", "openclaw-group"],
    updatedAt: "本机扫描 · 08-12",
  },
] as const;
