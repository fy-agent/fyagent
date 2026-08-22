import type { CodexCatalogModel, CodexChatReasoning } from "@/types";

export const normalizeCodexCatalogModelsForSave = (
  models: CodexCatalogModel[],
): CodexCatalogModel[] => {
  const seen = new Set<string>();
  const normalized: CodexCatalogModel[] = [];

  for (const item of models) {
    const model = item.model.trim();
    if (!model || seen.has(model)) continue;
    seen.add(model);

    const displayName = item.displayName?.trim();
    const rawContextWindow = String(item.contextWindow ?? "").replace(
      /[^\d]/g,
      "",
    );
    const contextWindow = rawContextWindow
      ? Number.parseInt(rawContextWindow, 10)
      : undefined;
    const inputModalities = item.inputModalities?.filter(
      (modality) => typeof modality === "string" && modality.trim(),
    );
    const baseInstructions = item.baseInstructions?.trim();

    normalized.push({
      model,
      ...(displayName ? { displayName } : {}),
      ...(contextWindow && contextWindow > 0 ? { contextWindow } : {}),
      ...(typeof item.supportsParallelToolCalls === "boolean"
        ? { supportsParallelToolCalls: item.supportsParallelToolCalls }
        : {}),
      ...(inputModalities && inputModalities.length > 0
        ? { inputModalities }
        : {}),
      ...(baseInstructions ? { baseInstructions } : {}),
    });
  }

  return normalized;
};

export const normalizeCodexChatReasoningForSave = (
  value?: CodexChatReasoning,
): CodexChatReasoning | undefined => {
  const supportsEffort = value?.supportsEffort === true;
  const supportsThinking = value?.supportsThinking === true || supportsEffort;
  const hasExplicitConfig = value && Object.keys(value).length > 0;

  if (!supportsThinking && !supportsEffort) {
    return hasExplicitConfig
      ? {
          supportsThinking: false,
          supportsEffort: false,
          thinkingParam: "none",
          effortParam: "none",
          outputFormat: value?.outputFormat ?? "auto",
        }
      : undefined;
  }

  return {
    supportsThinking,
    supportsEffort,
    thinkingParam: supportsThinking
      ? (value?.thinkingParam ?? "thinking")
      : "none",
    effortParam: supportsEffort
      ? (value?.effortParam ?? "reasoning_effort")
      : "none",
    effortValueMode: supportsEffort
      ? (value?.effortValueMode ?? "passthrough")
      : undefined,
    outputFormat: value?.outputFormat ?? "auto",
  };
};

export const parseCodexFeatureAuth = (
  value: string,
): Record<string, unknown> => {
  try {
    const parsed: unknown = JSON.parse(value);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
};
