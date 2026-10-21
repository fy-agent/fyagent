import { describe, expect, it } from "vitest";

import {
  AGENT_CATALOG_IDS,
  isAssignableProductEntry,
  MCP_TARGET_IDS,
  MCP_TARGETS,
  MODEL_DIRECTORY_IDS,
  PRODUCT_DIRECTORY,
  PROMPT_APP_IDS,
  PROMPT_ONLY_DIRECTORY,
  SKILL_TARGET_IDS,
  SKILL_TARGETS,
  SUPPORTED_APP_IDS,
  SUPPORTED_APPS,
} from "@/shared/features/directory";

describe("Shared product directory", () => {
  it("keeps Agent, Skills/MCP, Models, and Prompt IDs aligned to one ordered catalog", () => {
    expect(PRODUCT_DIRECTORY.map((entry) => entry.agentId)).toEqual([
      ...AGENT_CATALOG_IDS,
    ]);
    expect(
      PRODUCT_DIRECTORY.filter(isAssignableProductEntry).map(
        (entry) => entry.assignmentId,
      ),
    ).toEqual([...MCP_TARGET_IDS]);
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
      "omp",
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
      ["omp", "standard"],
    ]);
  });

  it("lists Oh My Pi as an Agent with models only, never for MCP, Skills, Prompts or Supported Apps", () => {
    expect(AGENT_CATALOG_IDS).toContain("omp");
    expect(MODEL_DIRECTORY_IDS).toContain("omp");
    for (const ids of [
      MCP_TARGET_IDS,
      SKILL_TARGET_IDS,
      SUPPORTED_APP_IDS,
      PROMPT_APP_IDS,
    ]) {
      expect(ids as readonly string[]).not.toContain("omp");
    }
    for (const targets of [MCP_TARGETS, SKILL_TARGETS, SUPPORTED_APPS]) {
      expect(targets.map((target) => target.id as string)).not.toContain("omp");
    }
  });
});
