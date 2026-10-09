import { describe, expect, it } from "vitest";
import type { Provider } from "@/domain/configuration/types";
import {
  fillCodexCatalogModel,
  normalizeClaudeStackModels,
  publishedModels,
  withPublishedModels,
} from "@/pages/models/aggregationModels";

describe("upstream aggregation model persistence", () => {
  it("publishes a managed Codex binding's explicit model and preserves its auth binding on edit", () => {
    const provider: Provider = {
      id: "managed-codex",
      name: "ChatGPT",
      meta: {
        providerType: "codex_oauth",
        authBinding: {
          source: "managed_account",
          authProvider: "codex_oauth",
          accountId: "ma1:00000000000000000000000000000001",
        },
      },
      settingsConfig: {
        auth: {},
        config:
          'model_provider = "fyagent_chatgpt"\nmodel = "gpt-6-sol"\n[model_providers.fyagent_chatgpt]\nwire_api = "responses"\n',
      },
    };
    const rows = publishedModels(provider, "codex");
    expect(rows).toEqual([{ model: "gpt-6-sol" }]);
    const saved = withPublishedModels(provider, "codex", rows, "gpt-6-sol");
    expect(saved.meta).toEqual(provider.meta);
    expect(saved.settingsConfig.auth).toEqual({});
    expect(saved.settingsConfig.modelCatalog).toEqual({ models: rows });
  });
  it.each([
    undefined,
    {},
    { models: [] },
    { models: [null, { model: " " }, { model: 1 }] },
  ])(
    "falls back to the explicit Codex model when the catalog has no models (%#)",
    (modelCatalog) => {
      expect(
        publishedModels(
          {
            id: "codex",
            name: "Codex",
            settingsConfig: {
              config: 'model = "explicit-model"\n',
              ...(modelCatalog ? { modelCatalog } : {}),
            },
          },
          "codex",
        ),
      ).toEqual([{ model: "explicit-model" }]);
    },
  );
  it("keeps a nonempty Codex catalog authoritative over the configured model", () => {
    const models = [{ model: "catalog-model", displayName: "Saved model" }];
    expect(
      publishedModels(
        {
          id: "codex",
          name: "Codex",
          settingsConfig: {
            config: 'model = "fallback-model"\n',
            modelCatalog: { models },
          },
        },
        "codex",
      ),
    ).toEqual(models);
  });
  it("does not invent a Codex model from an empty or section-only config", () => {
    for (const config of [
      "",
      'model = " "\n',
      '[profiles.work]\nmodel = "nested"\n',
    ]) {
      expect(
        publishedModels(
          { id: "codex", name: "Codex", settingsConfig: { config } },
          "codex",
        ),
      ).toEqual([]);
    }
  });
  it("normalizes Claude 1M IDs and merges duplicate model rows", () => {
    expect(
      normalizeClaudeStackModels([
        { model: " model-a[1M] " },
        { model: "model-a", displayName: "My model" },
        { model: " " },
      ]),
    ).toEqual([{ model: "model-a", oneM: true, displayName: "My model" }]);
  });
  it("fills only empty Codex parameters with canonical reasoning levels", () => {
    expect(
      fillCodexCatalogModel(
        { model: "model-a", contextWindow: 1000, inputModalities: ["text"] },
        {
          contextWindow: 2000,
          reasoningEfforts: ["high", "invalid", "none"],
          inputModalities: ["text", "image"],
        },
      ),
    ).toEqual({
      model: "model-a",
      contextWindow: 1000,
      inputModalities: ["text"],
      reasoningLevels: ["none", "high"],
    });
  });
  it("preserves Codex settings and catalog fields while changing models/default", () => {
    const result = withPublishedModels(
      {
        id: "a",
        name: "A",
        settingsConfig: {
          auth: { OPENAI_API_KEY: "keep" },
          config: 'model = "old"\nuser_setting = true\n',
          modelCatalog: { custom: true, models: [] },
        },
      },
      "codex",
      [{ model: "new" }],
      "new",
    );
    expect(result.settingsConfig).toMatchObject({
      auth: { OPENAI_API_KEY: "keep" },
      modelCatalog: { custom: true, models: [{ model: "new" }] },
    });
    expect(result.settingsConfig.config).toContain('model = "new"');
    expect(result.settingsConfig.config).toContain("user_setting = true");
  });
});
