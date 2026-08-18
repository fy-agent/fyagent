import { describe, expect, it } from "vitest";

import {
  agentTargetById,
  agentTargets,
  allAgentTargetIds,
  countCoveredAgentInstances,
  groupPromptTargetsByCanonicalResource,
  memoryWritableTargetIds,
  type AgentTargetDefinition,
  type AgentTargetId,
} from "../../../../src/v2/shared/config/agentTargets";

const expectedTargets: readonly AgentTargetDefinition[] = [
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
    promptCanonicalResourceKey:
      "~/.openclaw/workspace-group_liaison/AGENTS.md",
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
];

describe("V2 shared Agent target contract", () => {
  it.each(expectedTargets)(
    "exposes the frozen fields for $id",
    (expectedTarget) => {
      expect(agentTargetById(expectedTarget.id)).toEqual(expectedTarget);
    },
  );

  it("models seven canonical Prompt resources covering eight instances", () => {
    const groups = groupPromptTargetsByCanonicalResource(allAgentTargetIds);
    const uniqueInstances = new Set(
      groups.flatMap((group) => group.instanceNames),
    );

    expect(agentTargets).toHaveLength(7);
    expect(groups).toHaveLength(7);
    expect(uniqueInstances.size).toBe(8);
    expect(countCoveredAgentInstances(allAgentTargetIds)).toBe(8);
  });

  it("deduplicates repeated selections while retaining shared OpenClaw instances", () => {
    const groups = groupPromptTargetsByCanonicalResource([
      "openclaw-default",
      "openclaw-default",
      "openclaw-group",
    ]);

    expect(groups).toEqual([
      {
        key: "~/.openclaw/workspace/AGENTS.md",
        primaryTargetId: "openclaw-default",
        targetIds: ["openclaw-default"],
        instanceNames: ["main", "utility"],
      },
      {
        key: "~/.openclaw/workspace-group_liaison/AGENTS.md",
        primaryTargetId: "openclaw-group",
        targetIds: ["openclaw-group"],
        instanceNames: ["group_liaison"],
      },
    ]);
    expect(countCoveredAgentInstances(["openclaw-default"])).toBe(2);
  });

  it("derives exactly four Memory destinations from verified eligibility", () => {
    expect(memoryWritableTargetIds).toEqual([
      "claude-global",
      "openclaw-default",
      "openclaw-group",
      "hermes-global",
    ]);

    expect(
      agentTargets
        .filter((target) => target.memorySyncEligibility === "source-only")
        .map((target) => target.id),
    ).toEqual(["codex-global", "gemini-global", "opencode-global"]);
    expect(agentTargetById("claude-global")?.memorySyncEligibility).toBe(
      "verified-rule-bridge",
    );
  });

  it("keeps Prompt resource identity separate from Memory destination labels", () => {
    const openClawTargets = agentTargets.filter(
      (target) => target.toolId === "openclaw",
    );

    expect(openClawTargets[0].memoryDestination).toBe(
      openClawTargets[1].memoryDestination,
    );
    expect(
      groupPromptTargetsByCanonicalResource([
        "openclaw-default",
        "openclaw-group",
      ]),
    ).toHaveLength(2);
  });

  it("marks only Gemini and OpenCode Prompt files for creation on enable", () => {
    expect(
      agentTargets
        .filter((target) => target.promptPathState === "create-on-enable")
        .map((target) => target.id),
    ).toEqual(["gemini-global", "opencode-global"]);
  });

  it("returns undefined for an invalid lookup instead of falling back to Codex", () => {
    expect(agentTargetById("missing-target" as AgentTargetId)).toBeUndefined();
  });
});
