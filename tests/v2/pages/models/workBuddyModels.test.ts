import { describe, expect, it } from "vitest";

import {
  addUniqueModelIds,
  classifyModelType,
  filterModelIds,
  groupModelIds,
  splitWorkBuddyDraft,
} from "@/v2/pages/models/workBuddyModels";

describe("workBuddyModels", () => {
  it("classifies common third-party families and provider-prefixed IDs", () => {
    expect(classifyModelType("gpt-4o")).toBe("gpt");
    expect(classifyModelType("openai/gpt-4.1")).toBe("gpt");
    expect(classifyModelType("o3-mini")).toBe("gpt");
    expect(classifyModelType("gemini-2.5-pro")).toBe("gemini");
    expect(classifyModelType("google/gemini-2.0-flash")).toBe("gemini");
    expect(classifyModelType("claude-sonnet-4")).toBe("claude");
    expect(classifyModelType("grok-4.6")).toBe("grok");
    expect(classifyModelType("custom-router")).toBe("custom");
    expect(classifyModelType("")).toBe("other");
  });

  it("groups IDs by family and keeps first-seen order inside each group", () => {
    expect(
      groupModelIds([
        "gemini-2.5-pro",
        "gpt-4o",
        "gemini-2.0-flash",
        "gpt-4o",
        "grok-4.5",
      ]),
    ).toEqual([
      { type: "gpt", ids: ["gpt-4o"] },
      { type: "gemini", ids: ["gemini-2.5-pro", "gemini-2.0-flash"] },
      { type: "grok", ids: ["grok-4.5"] },
    ]);
  });

  it("splits a unified draft into fetched versus manual IDs", () => {
    expect(
      splitWorkBuddyDraft(
        ["gpt-4o", "custom-router", "gpt-4o", "gemini-2.5-pro"],
        new Set(["gpt-4o", "gemini-2.5-pro", "unused-remote"]),
      ),
    ).toEqual({
      selectedModelIds: ["gpt-4o", "gemini-2.5-pro"],
      manualModelIds: ["custom-router"],
    });
  });

  it("merges incoming IDs without changing already-kept order", () => {
    expect(addUniqueModelIds(["gpt-4o"], ["gemini-2.5-pro", "gpt-4o"])).toEqual(
      ["gpt-4o", "gemini-2.5-pro"],
    );
  });

  it("filters model IDs by a case-insensitive substring", () => {
    expect(
      filterModelIds(["gpt-4o", "gemini-2.5-pro", "grok-4.6"], "GEM"),
    ).toEqual(["gemini-2.5-pro"]);
    expect(filterModelIds(["gpt-4o", "gemini-2.5-pro"], "  ")).toEqual([
      "gpt-4o",
      "gemini-2.5-pro",
    ]);
  });
});
