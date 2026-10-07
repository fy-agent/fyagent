import { describe, expect, it } from "vitest";

import {
  codexPresetModelSources,
  hermesPresetModelSources,
  openclawPresetModelSources,
  opencodePresetModelSources,
} from "@/domain/configuration/presets/presetModelMetadata";
import { resolveModelMetadata } from "@/domain/configuration/modelMetadata";
import type { ModelsDevResponse } from "@/domain/configuration/modelsDev";

describe("preset model metadata sources", () => {

  it.each([
    ["codex", codexPresetModelSources],
    ["openclaw", openclawPresetModelSources],
    ["hermes", hermesPresetModelSources],
    ["opencode", opencodePresetModelSources],
  ])("collects reviewed model values from %s presets", (_app, sources) => {
    const list = sources();
    expect(list.length).toBeGreaterThan(0);
    expect(sources()).toBe(list);
    for (const source of list) {
      expect(source.baseUrl).toBeTruthy();
      expect(source.models.size).toBeGreaterThan(0);
    }
  });

  it("keeps a partner's own window over the vendor default", () => {
    // FluxA 转售的百度国际 team 部署：glm-5.2 是 500K，不是原厂的 1M。
    const modelsDev: ModelsDevResponse = {
      zhipuai: {
        api: "https://open.bigmodel.cn/api/paas/v4",
        models: { "glm-5.2": { limit: { context: 1000000 } } },
      },
      openrouter: {
        models: {
          "z-ai/glm-5.2": { canonical_model_id: "zhipuai/glm-5.2" },
        },
      },
    };
    expect(
      resolveModelMetadata("glm-5.2", {
        baseUrl: "https://api.baiduqianfan.ai/v2/tokenplan/team",
        presets: codexPresetModelSources(),
        modelsDev,
      })?.contextWindow,
    ).toBe(500000);
  });

  it("reads OpenCode limits from the preset for the same address", () => {
    expect(
      resolveModelMetadata("kimi-k2.6", {
        baseUrl: "https://api.moonshot.cn/v1",
        presets: opencodePresetModelSources(),
      }),
    ).toMatchObject({
      contextWindow: 262144,
      maxOutputTokens: 262144,
      inputModalities: ["text", "image"],
    });
  });
});
