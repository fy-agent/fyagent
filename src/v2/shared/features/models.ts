export type ProviderAppId = "claude" | "codex" | "grokbuild";

export interface ProviderQuickSetupRequest {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
  codexFeatures?: {
    imageExtension?: boolean;
    websockets?: boolean;
  };
}

export interface ProviderSummary {
  id: string;
  name: string;
  modelId?: string;
}

export type ProviderSummaryMap = Record<string, ProviderSummary>;
export interface ProviderSummaryQueryData {
  providers: ProviderSummaryMap;
  currentId: string;
}

export type CodexProviderMutationWarning =
  | "CODEX_WEBSOCKET_NON_GPT_MODEL"
  | "CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED";

export interface ProviderMutationResult<T> {
  value: T;
  liveConfigChanged: boolean;
  app: ProviderAppId;
  warningCodes?: CodexProviderMutationWarning[];
}

export interface ProviderSwitchResult {
  warnings: string[];
}

export type ProviderQuickSetupFailureCode =
  | "APPLY_FAILED_ROLLED_BACK"
  | "ROLLBACK_PARTIAL_STATE_UNKNOWN";

export interface ProviderQuickSetupCommandError {
  code: ProviderQuickSetupFailureCode;
}

export interface WorkBuddyStatus {
  path: string;
  exists: boolean;
  modelCount: number;
  revision: string | null;
  backupExists: boolean;
  format: "legacyArray" | "objectRoot" | "missing";
}

export interface WorkBuddyModelIdsResult {
  ids: string[];
  revision: string | null;
}

export interface WorkBuddyFetchModelsRequest {
  baseUrl: string;
  apiKey: string;
  allowNoApiKey: boolean;
}

export interface WorkBuddyFetchModelsResult {
  models: string[];
  truncated: boolean;
}

export interface WorkBuddySaveModelsRequest
  extends WorkBuddyFetchModelsRequest {
  selectedModelIds: string[];
  manualModelIds: string[];
  removedModelIds?: string[];
  clearExistingApiKeys: boolean;
  expectedRevision: string | null;
  overwriteToken?: string;
}

export type WorkBuddyChangePlanRequest = Omit<
  WorkBuddySaveModelsRequest,
  "overwriteToken"
>;

export interface WorkBuddySaveModelsSavedResult {
  state: "saved";
  revision: string;
  modelCount: number;
  createdEntries: number;
  updatedEntries: number;
}

export interface WorkBuddyOverwriteConfirmationRequiredResult {
  state: "overwrite_confirmation_required";
  token: string;
  existingIds: string[];
}

export interface WorkBuddyConcurrentModificationResult {
  state: "concurrent_modification";
}

export type WorkBuddySaveModelsResult =
  | WorkBuddySaveModelsSavedResult
  | WorkBuddyOverwriteConfirmationRequiredResult
  | WorkBuddyConcurrentModificationResult;

export interface FetchedModelRef {
  id: string;
  ownedBy?: string | null;
}

export interface FetchedModelList {
  models: FetchedModelRef[];
  truncated: boolean;
}

export type ReachabilityStatus = "operational" | "degraded" | "failed";
export interface ReachabilityResult {
  success: boolean;
  status: ReachabilityStatus;
  message: string;
  responseTimeMs: number | null;
  httpStatus: number | null;
}

export type ModelProbeAppId =
  | "claude"
  | "codex"
  | "grokbuild"
  | "workbuddy"
  | "opencode";

export interface ModelProbeRequest {
  app: ModelProbeAppId;
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

export interface ModelProbeResult {
  success: boolean;
  status: ReachabilityStatus;
  message: string;
  responseTimeMs: number | null;
  httpStatus: number | null;
  modelUsed: string;
  errorCategory: string | null;
}

export interface TraeWorkModelIdsResult {
  modelIds: string[];
  revision: string | null;
  truncated: boolean;
}

export interface OpenCodeProviderSnapshot {
  id: string;
  name: string;
  modelIds: string[];
}

export interface OpenCodeModelSnapshot {
  providers: OpenCodeProviderSnapshot[];
  revision: string | null;
}

export interface OpenCodeFetchModelsRequest {
  baseUrl: string;
  apiKey: string;
  allowNoApiKey: boolean;
}

export interface OpenCodeSaveModelsRequest {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  selectedModelIds: string[];
  removedModelIds?: string[];
  expectedRevision: string | null;
  overwriteToken?: string;
}

export type OpenCodeSaveModelsResult =
  | WorkBuddySaveModelsSavedResult
  | WorkBuddyOverwriteConfirmationRequiredResult
  | WorkBuddyConcurrentModificationResult;
