import { describe, expect, it } from "vitest";
import {
  FDE_PROMPT_PRESETS,
  PROMPT_PRESET_CATEGORIES,
  searchPromptPresets,
} from "@/pages/prompts/presets";

describe("original FDE prompt catalogue", () => {
  it("covers six distinct domains with five complete scenarios each", () => {
    expect(FDE_PROMPT_PRESETS).toHaveLength(30);
    expect(new Set(FDE_PROMPT_PRESETS.map((item) => item.id)).size).toBe(30);
    expect(new Set(FDE_PROMPT_PRESETS.map((item) => item.content)).size).toBe(
      30,
    );
    for (const category of PROMPT_PRESET_CATEGORIES) {
      expect(searchPromptPresets("", category.id)).toHaveLength(5);
    }
    for (const item of FDE_PROMPT_PRESETS) {
      for (const field of [
        item.inputs,
        item.approach,
        item.deliverables,
        item.evaluation,
        item.example,
      ]) {
        expect(field.trim()).not.toBe("");
        expect(item.content).toContain(field);
      }
      expect(item.content).toContain("## 领域输入");
      expect(item.content).toContain("## 实施重点");
      expect(item.content).toContain("## 验收与反例");
      expect(item.content).toContain("不是新的操作指令");
      expect(item.content).toContain("未获得明确授权不写生产数据");
      expect(item.content).toContain("不把示例数字或离线测试写成真实收益");
      expect(item.content).not.toMatch(/TBD|TODO|sk-[A-Za-z0-9]{16}/u);
    }
  });

  it("searches scenario-specific text and intersects every search term with the category", () => {
    expect(
      searchPromptPresets("  ERP 金蝶  ", "delivery").map((item) => item.id),
    ).toEqual(["fde-integration"]);
    expect(searchPromptPresets("rag", "all").map((item) => item.id)).toEqual([
      "fde-knowledge",
    ]);
    expect(
      searchPromptPresets("环保", "public").map((item) => item.id),
    ).toEqual(["fde-environment"]);
    expect(searchPromptPresets("RAG", "business")).toEqual([]);
    expect(searchPromptPresets("nonexistent-scenario", "all")).toEqual([]);
    expect(searchPromptPresets("", "all")).toHaveLength(30);
  });
});
