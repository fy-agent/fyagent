// Stable compatibility facade for provider configuration utilities.
// Keep existing imports on this path while JSON and Codex TOML concerns live
// in separate implementation modules.

export {
  applyTemplateValues,
  getApiKeyFromConfig,
  hasApiKeyField,
  hasCommonConfigSnippet,
  setApiKeyInConfig,
  updateCommonConfigSnippet,
  validateJsonConfig,
  type UpdateCommonConfigResult,
} from "@/utils/providerConfigJsonUtils";

export {
  codexApiFormatFromWireApi,
  extractCodexBaseUrl,
  extractCodexExperimentalBearerToken,
  extractCodexModelName,
  extractCodexTopLevelInt,
  extractCodexWireApi,
  getCodexBaseUrl,
  hasTomlCommonConfigSnippet,
  isCodexAnthropicWireApi,
  isCodexChatWireApi,
  isCodexGoalModeEnabled,
  isCodexRemoteCompactionEnabled,
  removeCodexTopLevelField,
  setCodexBaseUrl,
  setCodexGoalMode,
  setCodexModelName,
  setCodexRemoteCompaction,
  setCodexTopLevelInt,
  setCodexWireApi,
  updateCodexExperimentalBearerToken,
} from "@/utils/codexConfigUtils";
