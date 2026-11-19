import { describe, expect, it } from "vitest";

import {
  AGENT_CATALOG_IDS,
  AGENT_DISPLAY_ORDER,
  isOfficialOnlyAgent,
  isOfficialOnlyModelTarget,
  MCP_TARGET_IDS,
  MODEL_DIRECTORY_IDS,
  PRODUCT_DIRECTORY,
  PROMPT_APP_IDS,
  PROMPT_ONLY_DIRECTORY,
} from "@/shared/features/directory";

describe("Shared product directory", () => {
  it("keeps Agent, Skills/MCP, Models, and Prompt IDs aligned to one ordered catalog", () => {
    expect(PRODUCT_DIRECTORY.map((entry) => entry.agentId)).toEqual([
      ...AGENT_CATALOG_IDS,
    ]);
    expect(
      PRODUCT_DIRECTORY.flatMap((entry) =>
        entry.assignmentId ? [entry.assignmentId] : [],
      ),
    ).toEqual([...MCP_TARGET_IDS]);
    expect(new Set(PRODUCT_DIRECTORY.map((entry) => entry.modelTarget))).toEqual(
      new Set(MODEL_DIRECTORY_IDS),
    );
    expect([
      ...PRODUCT_DIRECTORY.flatMap((entry) =>
        entry.promptAppId ? [entry.promptAppId] : [],
      ),
      ...PROMPT_ONLY_DIRECTORY.map((entry) => entry.promptAppId),
    ]).toEqual([...PROMPT_APP_IDS]);
    expect(AGENT_CATALOG_IDS).toEqual([
      "qoderwork",
      "trae-work",
      "workbuddy",
      "grokbuild",
      "codex",
      "claude-code",
      "opencode",
      "omp",
    ]);
    expect(AGENT_DISPLAY_ORDER).toEqual([
      "workbuddy",
      "codex",
      "claude-code",
      "opencode",
      "omp",
      "grokbuild",
      "qoderwork",
      "trae-work",
    ]);
    expect(MODEL_DIRECTORY_IDS).toEqual([
      "workbuddy",
      "codex",
      "claude",
      "opencode",
      "omp",
      "grokbuild",
      "qoderwork",
      "trae",
    ]);
    expect(AGENT_CATALOG_IDS.filter(isOfficialOnlyAgent)).toEqual([
      "qoderwork",
      "trae-work",
    ]);
    expect(MODEL_DIRECTORY_IDS.filter(isOfficialOnlyModelTarget)).toEqual([
      "qoderwork",
      "trae",
    ]);
  });
});
