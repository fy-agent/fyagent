import { describe, expect, it } from "vitest";

import {
  AGENT_CATALOG_IDS,
  MCP_TARGET_IDS,
  MODEL_DIRECTORY_IDS,
  PRODUCT_DIRECTORY,
  PROMPT_APP_IDS,
  PROMPT_ONLY_DIRECTORY,
} from "@/v2/shared/features/directory";

describe("V2 shared product directory", () => {
  it("keeps Agent, Skills/MCP, Models, and Prompt IDs aligned to one ordered catalog", () => {
    expect(PRODUCT_DIRECTORY.map((entry) => entry.agentId)).toEqual([
      ...AGENT_CATALOG_IDS,
    ]);
    expect(PRODUCT_DIRECTORY.map((entry) => entry.assignmentId)).toEqual([
      ...MCP_TARGET_IDS,
    ]);
    expect(PRODUCT_DIRECTORY.map((entry) => entry.modelTarget)).toEqual([
      ...MODEL_DIRECTORY_IDS,
    ]);
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
    ]);
    expect(
      PRODUCT_DIRECTORY.map((entry) => [
        entry.agentId,
        entry.directoryPriority,
      ]),
    ).toEqual([
      ["qoderwork", "domestic"],
      ["trae-work", "domestic"],
      ["workbuddy", "domestic"],
      ["grokbuild", "standard"],
      ["codex", "standard"],
      ["claude-code", "standard"],
      ["opencode", "standard"],
    ]);
  });
});
