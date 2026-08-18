export type AgentToolId =
  | "codex"
  | "claude"
  | "gemini"
  | "opencode"
  | "openclaw"
  | "hermes";

export type AgentTargetId =
  | "codex-global"
  | "claude-global"
  | "gemini-global"
  | "opencode-global"
  | "openclaw-default"
  | "openclaw-group"
  | "hermes-global";

export type PromptPathState = "exists" | "create-on-enable";

export type MemorySyncEligibility =
  | "source-only"
  | "verified-rule-bridge"
  | "verified-native";

export interface AgentTargetDefinition {
  id: AgentTargetId;
  toolId: AgentToolId;
  name: string;
  scopeLabel: string;
  instanceNames: readonly string[];
  promptFile: string;
  promptPath: string;
  promptCanonicalResourceKey: string;
  promptPathState: PromptPathState;
  memoryDestination: string;
  memorySyncEligibility: MemorySyncEligibility;
  detected: boolean;
}

export interface CanonicalPromptTargetGroup {
  key: string;
  primaryTargetId: AgentTargetId;
  targetIds: AgentTargetId[];
  instanceNames: string[];
}

export const agentTargets: readonly AgentTargetDefinition[] = [
  {
    id: "codex-global",
    toolId: "codex",
    name: "Codex",
    scopeLabel: "全局",
    instanceNames: ["Codex"],
    promptFile: "AGENTS.md",
    promptPath: "~/.codex/AGENTS.md",
    promptCanonicalResourceKey: "~/.codex/AGENTS.md",
    promptPathState: "exists",
    memoryDestination: "派生记忆只读 · 不作为写入目标",
    memorySyncEligibility: "source-only",
    detected: true,
  },
  {
    id: "claude-global",
    toolId: "claude",
    name: "Claude Code",
    scopeLabel: "全局",
    instanceNames: ["Claude Code"],
    promptFile: "CLAUDE.md",
    promptPath: "~/.claude/CLAUDE.md",
    promptCanonicalResourceKey: "~/.claude/CLAUDE.md",
    promptPathState: "exists",
    memoryDestination: "~/.claude/memory/ · 本机 CLAUDE.md 引用",
    memorySyncEligibility: "verified-rule-bridge",
    detected: true,
  },
  {
    id: "gemini-global",
    toolId: "gemini",
    name: "Gemini CLI",
    scopeLabel: "全局",
    instanceNames: ["Gemini CLI"],
    promptFile: "GEMINI.md",
    promptPath: "~/.gemini/GEMINI.md",
    promptCanonicalResourceKey: "~/.gemini/GEMINI.md",
    promptPathState: "create-on-enable",
    memoryDestination: "仅发现会话 · 暂不可同步",
    memorySyncEligibility: "source-only",
    detected: true,
  },
  {
    id: "opencode-global",
    toolId: "opencode",
    name: "OpenCode",
    scopeLabel: "全局",
    instanceNames: ["OpenCode"],
    promptFile: "AGENTS.md",
    promptPath: "~/.config/opencode/AGENTS.md",
    promptCanonicalResourceKey: "~/.config/opencode/AGENTS.md",
    promptPathState: "create-on-enable",
    memoryDestination: "本机维护文件 · 未被 Agent 指导文件引用",
    memorySyncEligibility: "source-only",
    detected: true,
  },
  {
    id: "openclaw-default",
    toolId: "openclaw",
    name: "OpenClaw",
    scopeLabel: "默认工作区 · main + utility",
    instanceNames: ["main", "utility"],
    promptFile: "AGENTS.md",
    promptPath: "~/.openclaw/workspace/AGENTS.md",
    promptCanonicalResourceKey: "~/.openclaw/workspace/AGENTS.md",
    promptPathState: "exists",
    memoryDestination: "原生 MEMORY.md · USER.md",
    memorySyncEligibility: "verified-native",
    detected: true,
  },
  {
    id: "openclaw-group",
    toolId: "openclaw",
    name: "OpenClaw",
    scopeLabel: "群聊工作区 · group_liaison",
    instanceNames: ["group_liaison"],
    promptFile: "AGENTS.md",
    promptPath: "~/.openclaw/workspace-group_liaison/AGENTS.md",
    promptCanonicalResourceKey: "~/.openclaw/workspace-group_liaison/AGENTS.md",
    promptPathState: "exists",
    memoryDestination: "原生 MEMORY.md · USER.md",
    memorySyncEligibility: "verified-native",
    detected: true,
  },
  {
    id: "hermes-global",
    toolId: "hermes",
    name: "Hermes",
    scopeLabel: "全局角色",
    instanceNames: ["Hermes"],
    promptFile: "SOUL.md",
    promptPath: "~/.hermes/SOUL.md",
    promptCanonicalResourceKey: "~/.hermes/SOUL.md",
    promptPathState: "exists",
    memoryDestination: "原生 memories/MEMORY.md · USER.md",
    memorySyncEligibility: "verified-native",
    detected: true,
  },
] as const;

export const allAgentTargetIds: readonly AgentTargetId[] = agentTargets.map(
  (target) => target.id,
);

export const codingAgentTargetIds: readonly AgentTargetId[] = [
  "claude-global",
  "codex-global",
  "gemini-global",
  "opencode-global",
] as const;

export const memoryAwareTargetIds: readonly AgentTargetId[] = [
  "codex-global",
  "claude-global",
  "opencode-global",
  "openclaw-default",
  "openclaw-group",
  "hermes-global",
] as const;

export const memoryWritableTargetIds: readonly AgentTargetId[] = agentTargets
  .filter((target) => target.memorySyncEligibility !== "source-only")
  .map((target) => target.id);

export function agentTargetById(
  id: AgentTargetId,
): AgentTargetDefinition | undefined {
  return agentTargets.find((target) => target.id === id);
}

export function groupPromptTargetsByCanonicalResource(
  targetIds: readonly AgentTargetId[],
): CanonicalPromptTargetGroup[] {
  const groupsByKey = new Map<string, CanonicalPromptTargetGroup>();
  const seenTargetIds = new Set<AgentTargetId>();

  for (const targetId of targetIds) {
    if (seenTargetIds.has(targetId)) {
      continue;
    }
    seenTargetIds.add(targetId);

    const target = agentTargetById(targetId);
    if (!target) {
      continue;
    }

    const existingGroup = groupsByKey.get(target.promptCanonicalResourceKey);
    if (existingGroup) {
      existingGroup.targetIds.push(target.id);
      for (const instanceName of target.instanceNames) {
        if (!existingGroup.instanceNames.includes(instanceName)) {
          existingGroup.instanceNames.push(instanceName);
        }
      }
      continue;
    }

    groupsByKey.set(target.promptCanonicalResourceKey, {
      key: target.promptCanonicalResourceKey,
      primaryTargetId: target.id,
      targetIds: [target.id],
      instanceNames: [...target.instanceNames],
    });
  }

  return [...groupsByKey.values()];
}

export function countCoveredAgentInstances(
  targetIds: readonly AgentTargetId[],
): number {
  const instanceNames = new Set(
    groupPromptTargetsByCanonicalResource(targetIds).flatMap(
      (group) => group.instanceNames,
    ),
  );
  return instanceNames.size;
}
