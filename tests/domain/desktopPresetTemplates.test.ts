import { describe, expect, it } from "vitest";

import { providerPresets } from "@/domain/configuration/presets/claudeProviderPresets";
import {
  DESKTOP_PRESET_TEMPLATES,
  buildBatchTargetSpecs,
} from "@/domain/configuration/presets/desktopPresetTemplates";

describe("Desktop preset templates", () => {
  it("aligns Claude Anthropic endpoints with authoritative claudeProviderPresets", () => {
    const claudeNameMap: Record<string, string> = {
      deepseek: "DeepSeek",
      kimi: "Kimi",
      zhipu: "Zhipu GLM",
      dashscope: "千问AI平台",
      volcengine: "Volcengine Doubao",
      minimax: "MiniMax",
      siliconflow: "SiliconFlow",
      openrouter: "OpenRouter",
    };

    for (const template of DESKTOP_PRESET_TEMPLATES) {
      const claudePreset = providerPresets.find(
        (p) => p.name === claudeNameMap[template.id],
      );
      expect(claudePreset, template.id).toBeDefined();
      const env = (
        claudePreset!.settingsConfig as { env: Record<string, string> }
      ).env;
      expect(template.targets.claude?.baseUrl, template.id).toBe(
        env.ANTHROPIC_BASE_URL,
      );
      expect(template.targets.claude?.modelId, template.id).toBe(
        env.ANTHROPIC_MODEL,
      );
      expect(template.apiKeyUrl).toMatch(/^https:\/\//u);
    }
  });

  it("builds compliant per-target specs only for supported targets", () => {
    const dashscope = DESKTOP_PRESET_TEMPLATES.find(
      (p) => p.id === "dashscope",
    )!;
    const specs = buildBatchTargetSpecs(dashscope, [
      "workbuddy",
      "codex",
      "claude",
      "grokbuild",
    ]);
    expect(specs.map((s) => s.target)).toEqual([
      "workbuddy",
      "codex",
      "claude",
    ]);
  });
});
