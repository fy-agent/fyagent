import {
  MCP_TARGET_IDS,
  SKILL_TARGET_IDS,
  type McpTargetId,
  type SkillTargetId,
} from "./directory";

export type McpAssignments = Record<McpTargetId, boolean> &
  Record<string, boolean | undefined>;

export type SkillAssignments = Record<SkillTargetId, boolean> &
  Record<string, boolean | undefined>;

/** @deprecated Use SkillAssignments or McpAssignments. */
export type AppAssignments = SkillAssignments;

export function createSkillAssignments(
  enabled: readonly SkillTargetId[] = [],
): SkillAssignments {
  const enabledSet = new Set(enabled);
  return {
    ...(Object.fromEntries(
      SKILL_TARGET_IDS.map((id) => [id, enabledSet.has(id)]),
    ) as SkillAssignments),
    "claude-desktop": false,
    openclaw: false,
  };
}

export function createMcpAssignments(
  enabled: readonly McpTargetId[] = [],
): McpAssignments {
  const enabledSet = new Set(enabled);
  return Object.fromEntries(
    MCP_TARGET_IDS.map((id) => [id, enabledSet.has(id)]),
  ) as McpAssignments;
}

/** @deprecated Use createSkillAssignments or createMcpAssignments. */
export const createAssignments = createSkillAssignments;
