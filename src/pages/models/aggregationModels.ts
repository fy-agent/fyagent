// Adapted from CC Switch v4.0.4 ClaudeStackModelsField and modelMetadataFill.
import type {
  ClaudeStackModel,
  CodexCatalogModel,
  Provider,
} from "../../domain/configuration/types";
import type { KnownModelMetadata } from "../../domain/configuration/modelMetadata";
import { isPlainObject } from "../../domain/configuration/serialization/providerConfigStructural";
import {
  extractCodexModelName,
  setCodexModelName,
} from "../../domain/configuration/serialization/providerConfigUtils";

export type PublishedModel = CodexCatalogModel & ClaudeStackModel;
export const REASONING_LEVELS = [
  "none",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
];

export function normalizeClaudeStackModels(
  rows: ClaudeStackModel[],
): ClaudeStackModel[] {
  const result: ClaudeStackModel[] = [];
  for (const row of rows) {
    const raw = row.model.trim();
    const model = raw.replace(/\[1m\]/gi, "").trim();
    if (!model) continue;
    const oneM = row.oneM === true || /\[1m\]/i.test(raw);
    const displayName = row.displayName?.trim() || undefined;
    const existing = result.find((entry) => entry.model === model);
    if (existing) {
      if (oneM) existing.oneM = true;
      if (!existing.displayName && displayName)
        existing.displayName = displayName;
      continue;
    }
    result.push({
      model,
      ...(displayName ? { displayName } : {}),
      ...(oneM ? { oneM: true } : {}),
    });
  }
  return result;
}

export function publishedModels(
  provider: Provider,
  app: "claude" | "codex",
): PublishedModel[] {
  if (app === "claude") {
    if (provider.meta?.stackModels) return provider.meta.stackModels;
    const env = isPlainObject(provider.settingsConfig.env)
      ? provider.settingsConfig.env
      : {};
    return normalizeClaudeStackModels(
      [
        ["ANTHROPIC_MODEL", null],
        ["ANTHROPIC_DEFAULT_OPUS_MODEL", "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME"],
        [
          "ANTHROPIC_DEFAULT_SONNET_MODEL",
          "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
        ],
        ["ANTHROPIC_DEFAULT_HAIKU_MODEL", "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME"],
        ["ANTHROPIC_DEFAULT_FABLE_MODEL", "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME"],
      ].flatMap(([key, nameKey]) => {
        const value = key ? env[key] : undefined;
        const name = nameKey ? env[nameKey] : undefined;
        return typeof value === "string"
          ? [
              {
                model: value,
                displayName: typeof name === "string" ? name : undefined,
              },
            ]
          : [];
      }),
    );
  }
  const catalog = provider.settingsConfig.modelCatalog;
  const models =
    isPlainObject(catalog) && Array.isArray(catalog.models)
      ? catalog.models.filter(
          (row): row is PublishedModel =>
            isPlainObject(row) &&
            typeof row.model === "string" &&
            row.model.trim() !== "",
        )
      : [];
  if (models.length > 0) return models;
  const model = extractCodexModelName(
    typeof provider.settingsConfig.config === "string"
      ? provider.settingsConfig.config
      : "",
  );
  return model?.trim() ? [{ model: model.trim() }] : [];
}

export function providerDefaultModel(
  provider: Provider,
  app: "claude" | "codex",
): string {
  return app === "claude"
    ? (publishedModels(provider, app)[0]?.model ?? "")
    : (extractCodexModelName(
        typeof provider.settingsConfig.config === "string"
          ? provider.settingsConfig.config
          : "",
      ) ?? "");
}

export function withPublishedModels(
  provider: Provider,
  app: "claude" | "codex",
  rows: PublishedModel[],
  defaultModel: string,
): Provider {
  if (app === "claude")
    return {
      ...provider,
      meta: { ...provider.meta, stackModels: normalizeClaudeStackModels(rows) },
    };
  const catalog = isPlainObject(provider.settingsConfig.modelCatalog)
    ? provider.settingsConfig.modelCatalog
    : {};
  return {
    ...provider,
    settingsConfig: {
      ...provider.settingsConfig,
      modelCatalog: {
        ...catalog,
        models: rows
          .filter((row) => row.model.trim())
          .map((row) => ({ ...row, model: row.model.trim() })),
      },
      config: setCodexModelName(
        typeof provider.settingsConfig.config === "string"
          ? provider.settingsConfig.config
          : "",
        defaultModel,
      ),
    },
  };
}

export function fillCodexCatalogModel<T extends CodexCatalogModel>(
  row: T,
  metadata: KnownModelMetadata,
): T {
  const next = { ...row };
  if (
    (row.contextWindow === undefined ||
      String(row.contextWindow).trim() === "") &&
    metadata.contextWindow
  )
    next.contextWindow = String(metadata.contextWindow);
  if (!row.reasoningLevels?.length && metadata.reasoningEfforts) {
    const levels = REASONING_LEVELS.filter((level) =>
      metadata.reasoningEfforts?.includes(level),
    );
    if (levels.length > 0) next.reasoningLevels = levels;
  }
  if (!row.inputModalities && metadata.inputModalities)
    next.inputModalities = metadata.inputModalities.includes("image")
      ? ["text", "image"]
      : ["text"];
  return next;
}
