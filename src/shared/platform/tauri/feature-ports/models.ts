import {
  apiProtocolsForTarget,
  isApiProtocol,
  type ApiConnection,
} from "../../../../domain/configuration/providerApi";
import { invoke } from "@tauri-apps/api/core";
import { parseFileWriteTarget as parseModelWriteTarget } from "../../../features/file-writes";
import { parseProviderLiveSummary } from "./providerLiveSummary";

import type { FeaturePorts } from "../../../features/ports";
import type {
  FetchedModelList,
  FetchedModelRef,
  ModelProbeRequest,
  ModelProbeResult,
  OpenCodeFetchModelsRequest,
  OpenCodeModelSnapshot,
  OpenCodeSaveModelsRequest,
  ProviderAppId,
  ProviderQuickSetupRequest,
  ProviderSummaryQueryData,
  ReachabilityResult,
  WorkBuddySaveModelsResult,
} from "../../../features/types";
import {
  hasExactKeys,
  hasRequiredAndOptionalKeys,
  isOneOf,
  isRecord,
  isStringArray,
} from "./validation";

function assertOpenCodeFetchRequest(
  request: OpenCodeFetchModelsRequest,
): OpenCodeFetchModelsRequest {
  if (
    !isRecord(request) ||
    !hasExactKeys(request, ["baseUrl", "apiKey", "allowNoApiKey"]) ||
    typeof request.baseUrl !== "string" ||
    typeof request.apiKey !== "string" ||
    typeof request.allowNoApiKey !== "boolean"
  )
    throw new Error("OpenCode model request is invalid");
  return { ...request };
}

function assertOpenCodeSaveRequest(
  request: OpenCodeSaveModelsRequest,
): OpenCodeSaveModelsRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      [
        "providerId",
        "providerName",
        "baseUrl",
        "apiKey",
        "selectedModelIds",
        "expectedRevision",
      ],
      ["removedModelIds", "overwriteToken"],
    ) ||
    (request.providerId !== null &&
      (typeof request.providerId !== "string" ||
        request.providerId.trim().length === 0)) ||
    typeof request.providerName !== "string" ||
    typeof request.baseUrl !== "string" ||
    typeof request.apiKey !== "string" ||
    !isStringArray(request.selectedModelIds) ||
    (request.removedModelIds !== undefined &&
      !isStringArray(request.removedModelIds)) ||
    (request.expectedRevision !== null &&
      typeof request.expectedRevision !== "string") ||
    (request.overwriteToken !== undefined &&
      typeof request.overwriteToken !== "string")
  )
    throw new Error("OpenCode model request is invalid");
  return { ...request };
}

function parseFetchedModelRef(value: unknown): FetchedModelRef {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(value, ["id"], ["ownedBy"]) ||
    typeof value.id !== "string" ||
    value.id.trim().length === 0 ||
    (value.ownedBy !== undefined &&
      value.ownedBy !== null &&
      typeof value.ownedBy !== "string")
  )
    throw new Error("Model list is unavailable");
  return {
    id: value.id,
    ...(value.ownedBy === undefined
      ? {}
      : { ownedBy: value.ownedBy as string | null }),
  };
}

function parseFetchedModelRefs(value: unknown): FetchedModelRef[] {
  if (!Array.isArray(value)) throw new Error("Model list is unavailable");
  return value.map(parseFetchedModelRef);
}

function parseFetchedModelList(value: unknown): FetchedModelList {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["models", "truncated"]) ||
    !Array.isArray(value.models) ||
    typeof value.truncated !== "boolean"
  )
    throw new Error("Model list is unavailable");
  return {
    models: value.models.map(parseFetchedModelRef),
    truncated: value.truncated,
  };
}

function parseReachabilityResult(value: unknown): ReachabilityResult {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(
      value,
      ["status", "success", "message"],
      [
        "responseTimeMs",
        "httpStatus",
        "modelUsed",
        "testedAt",
        "retryCount",
        "errorCategory",
      ],
    ) ||
    !isOneOf(value.status, ["operational", "degraded", "failed"]) ||
    typeof value.success !== "boolean" ||
    typeof value.message !== "string" ||
    (value.responseTimeMs !== undefined &&
      value.responseTimeMs !== null &&
      (typeof value.responseTimeMs !== "number" ||
        !Number.isFinite(value.responseTimeMs))) ||
    (value.httpStatus !== undefined &&
      value.httpStatus !== null &&
      (typeof value.httpStatus !== "number" ||
        !Number.isInteger(value.httpStatus)))
  )
    throw new Error("Reachability result is unavailable");
  return {
    success: value.success,
    status: value.status,
    message: value.message,
    responseTimeMs:
      typeof value.responseTimeMs === "number" ? value.responseTimeMs : null,
    httpStatus: typeof value.httpStatus === "number" ? value.httpStatus : null,
  };
}

function parseModelProbeResult(value: unknown): ModelProbeResult {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(
      value,
      ["status", "success", "message", "modelUsed"],
      [
        "responseTimeMs",
        "httpStatus",
        "testedAt",
        "retryCount",
        "errorCategory",
      ],
    ) ||
    !isOneOf(value.status, ["operational", "degraded", "failed"]) ||
    typeof value.success !== "boolean" ||
    typeof value.message !== "string" ||
    typeof value.modelUsed !== "string" ||
    (value.responseTimeMs !== undefined &&
      value.responseTimeMs !== null &&
      (typeof value.responseTimeMs !== "number" ||
        !Number.isFinite(value.responseTimeMs))) ||
    (value.httpStatus !== undefined &&
      value.httpStatus !== null &&
      (typeof value.httpStatus !== "number" ||
        !Number.isInteger(value.httpStatus))) ||
    (value.errorCategory !== undefined &&
      value.errorCategory !== null &&
      typeof value.errorCategory !== "string")
  )
    throw new Error("Model probe result is unavailable");
  return {
    success: value.success,
    status: value.status,
    message: value.message,
    responseTimeMs:
      typeof value.responseTimeMs === "number" ? value.responseTimeMs : null,
    httpStatus: typeof value.httpStatus === "number" ? value.httpStatus : null,
    modelUsed: value.modelUsed,
    errorCategory:
      typeof value.errorCategory === "string" ? value.errorCategory : null,
  };
}

function assertModelProbeRequest(
  request: ModelProbeRequest,
): ModelProbeRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      ["app", "baseUrl", "apiKey", "modelId"],
      ["codexImageExtension", "protocol"],
    ) ||
    !isOneOf(request.app, [
      "claude",
      "codex",
      "grokbuild",
      "workbuddy",
      "opencode",
    ]) ||
    typeof request.baseUrl !== "string" ||
    typeof request.apiKey !== "string" ||
    typeof request.modelId !== "string" ||
    request.modelId.trim().length === 0 ||
    (request.protocol !== undefined &&
      (!isApiProtocol(request.protocol) ||
        (request.app === "workbuddy" || request.app === "opencode"
          ? request.protocol !== "chat"
          : !apiProtocolsForTarget(request.app).includes(request.protocol)))) ||
    (request.codexImageExtension !== undefined &&
      typeof request.codexImageExtension !== "boolean")
  )
    throw new Error("Model probe request is invalid");
  return request;
}

async function invokeReachability(
  baseUrl: string,
): Promise<ReachabilityResult> {
  return parseReachabilityResult(
    await invoke<unknown>("stream_check_url", { baseUrl }),
  );
}

async function invokeModelProbe(
  request: ModelProbeRequest,
): Promise<ModelProbeResult> {
  const payload = assertModelProbeRequest(request);
  return parseModelProbeResult(
    await invoke<unknown>("stream_check_model", {
      app: payload.app,
      baseUrl: payload.baseUrl,
      apiKey: payload.apiKey,
      modelId: payload.modelId,
      codexImageExtension: payload.codexImageExtension,
      ...(payload.protocol !== undefined ? { protocol: payload.protocol } : {}),
    }),
  );
}

function parseOpenCodeModelSnapshot(value: unknown): OpenCodeModelSnapshot {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(
      value,
      ["providers", "revision", "path", "backupPath", "exists"],
      ["selectedModel"],
    ) ||
    !Array.isArray(value.providers) ||
    (value.revision !== null && typeof value.revision !== "string") ||
    typeof value.path !== "string" ||
    typeof value.backupPath !== "string" ||
    typeof value.exists !== "boolean"
  )
    throw new Error("OpenCode model snapshot is unavailable");
  const providers = value.providers.map((provider) => {
    if (
      !isRecord(provider) ||
      !hasExactKeys(provider, ["id", "name", "modelIds", "editable"]) ||
      typeof provider.id !== "string" ||
      typeof provider.name !== "string" ||
      typeof provider.editable !== "boolean" ||
      !isStringArray(provider.modelIds)
    )
      throw new Error("OpenCode model snapshot is unavailable");
    return {
      id: provider.id,
      name: provider.name,
      modelIds: provider.modelIds,
      editable: provider.editable,
    };
  });
  if (
    value.selectedModel !== undefined &&
    value.selectedModel !== null &&
    !providers.some((provider) =>
      provider.modelIds.some(
        (model) => value.selectedModel === `${provider.id}/${model}`,
      ),
    )
  )
    throw new Error("OpenCode selected model is unavailable");
  return {
    providers,
    ...(value.selectedModel === undefined
      ? {}
      : { selectedModel: value.selectedModel as string | null }),
    revision: value.revision,
    path: value.path,
    backupPath: value.backupPath,
    exists: value.exists,
  };
}

function parseRevisionedSaveResult(value: unknown): WorkBuddySaveModelsResult {
  if (!isRecord(value) || typeof value.state !== "string")
    throw new Error("Model save result is unavailable");
  if (value.state === "saved") {
    if (
      !hasExactKeys(value, [
        "state",
        "revision",
        "modelCount",
        "createdEntries",
        "updatedEntries",
      ]) ||
      typeof value.revision !== "string" ||
      typeof value.modelCount !== "number" ||
      typeof value.createdEntries !== "number" ||
      typeof value.updatedEntries !== "number"
    )
      throw new Error("Model save result is unavailable");
    return {
      state: "saved",
      revision: value.revision,
      modelCount: value.modelCount,
      createdEntries: value.createdEntries,
      updatedEntries: value.updatedEntries,
    };
  }
  if (value.state === "overwrite_confirmation_required") {
    if (
      !hasExactKeys(value, ["state", "token", "existingIds"]) ||
      typeof value.token !== "string" ||
      !isStringArray(value.existingIds)
    )
      throw new Error("Model save result is unavailable");
    return {
      state: "overwrite_confirmation_required",
      token: value.token,
      existingIds: value.existingIds,
    };
  }
  if (value.state === "concurrent_modification") {
    if (!hasExactKeys(value, ["state"]))
      throw new Error("Model save result is unavailable");
    return { state: "concurrent_modification" };
  }
  throw new Error("Model save result is unavailable");
}

function parsePublicConnection(value: unknown): ApiConnection {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["baseUrl", "modelId", "protocol"]) ||
    typeof value.baseUrl !== "string" ||
    typeof value.modelId !== "string" ||
    !value.modelId.trim() ||
    value.modelId.length > 256 ||
    !isApiProtocol(value.protocol)
  ) {
    throw new Error("Provider public summary is unavailable");
  }
  try {
    const url = new URL(value.baseUrl);
    if (
      !["http:", "https:"].includes(url.protocol) ||
      !url.hostname ||
      url.username ||
      url.password ||
      url.search ||
      url.hash ||
      value.baseUrl.length > 2048
    )
      throw new Error();
  } catch {
    throw new Error("Provider public summary is unavailable");
  }
  return {
    baseUrl: value.baseUrl,
    modelId: value.modelId,
    protocol: value.protocol,
  };
}

function parseProviderSummary(
  value: unknown,
  app: ProviderAppId,
): ProviderSummaryQueryData {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(
      value,
      ["providers", "currentId", "writeTargets"],
      ["live"],
    )
  )
    throw new Error("Provider public summary is unavailable");
  if (
    !isRecord(value.providers) ||
    typeof value.currentId !== "string" ||
    !Array.isArray(value.writeTargets)
  )
    throw new Error("Provider public summary is unavailable");

  const providers: ProviderSummaryQueryData["providers"] = {};
  for (const [key, candidate] of Object.entries(value.providers)) {
    if (
      !isRecord(candidate) ||
      !hasRequiredAndOptionalKeys(
        candidate,
        ["id", "name", "writeTargets"],
        ["modelId", "connection"],
      ) ||
      !Array.isArray(candidate.writeTargets) ||
      typeof candidate.id !== "string" ||
      typeof candidate.name !== "string" ||
      (candidate.modelId !== undefined &&
        typeof candidate.modelId !== "string") ||
      candidate.id !== key
    )
      throw new Error("Provider public summary is unavailable");
    providers[key] = {
      id: candidate.id,
      name: candidate.name,
      writeTargets: candidate.writeTargets.map(parseModelWriteTarget),
      ...(candidate.connection !== undefined
        ? { connection: parsePublicConnection(candidate.connection) }
        : {}),
      ...(typeof candidate.modelId === "string" && candidate.modelId
        ? { modelId: candidate.modelId }
        : {}),
    };
  }
  if (value.currentId !== "" && !(value.currentId in providers))
    throw new Error("Provider public summary is unavailable");
  return {
    providers,
    currentId: value.currentId,
    writeTargets: value.writeTargets.map(parseModelWriteTarget),
    live: parseProviderLiveSummary(value.live, app),
  };
}

function isValidCodexFeatures(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return (
    Object.keys(value).every((key) =>
      ["imageExtension", "websockets"].includes(key),
    ) && Object.values(value).every((item) => typeof item === "boolean")
  );
}

function assertQuickSetupRequest(
  request: ProviderQuickSetupRequest,
  app: "claude" | "codex" | "grokbuild",
): ProviderQuickSetupRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      ["name", "baseUrl", "apiKey", "modelId"],
      ["codexFeatures", "protocol"],
    ) ||
    !["name", "baseUrl", "apiKey", "modelId"].every(
      (key) => typeof request[key] === "string",
    ) ||
    (request.protocol !== undefined &&
      (!isApiProtocol(request.protocol) ||
        !apiProtocolsForTarget(app).includes(request.protocol))) ||
    (request.codexFeatures !== undefined &&
      !isValidCodexFeatures(request.codexFeatures))
  )
    throw new Error("Provider quick setup request is invalid");
  return request;
}

export function createModelFeaturePorts(): Pick<
  FeaturePorts,
  "providers" | "workbuddy" | "opencodeModels"
> {
  return {
    providers: {
      getSummary: async (app) =>
        parseProviderSummary(
          await invoke("get_provider_summary", { app }),
          app,
        ),
      getProxyRestorePreview: async (app) =>
        (
          await import("./managedSubscriptions")
        ).managedProviderPorts.getProxyRestorePreview(app),
      restoreManagedProxy: async (app) =>
        (
          await import("./managedSubscriptions")
        ).managedProviderPorts.restoreManagedProxy(app),
      applyQuickSetupWithResult: (request, app) =>
        invoke("apply_provider_quick_setup_with_result", {
          request: assertQuickSetupRequest(request, app),
          app,
        }),
      fetchModels: async (baseUrl, apiKey) =>
        parseFetchedModelRefs(
          await invoke<unknown>("fetch_models_for_config", {
            baseUrl,
            apiKey,
          }),
        ),
      checkReachability: invokeReachability,
      checkModel: invokeModelProbe,
      bindXaiManaged: async (request) =>
        (
          await import("./managedSubscriptions")
        ).managedProviderPorts.bindXaiManaged(request),
      bindManagedProxy: async (request) =>
        (
          await import("./managedSubscriptions")
        ).managedProviderPorts.bindManagedProxy(request),
      fetchXaiManagedModels: async (accountId) =>
        (
          await import("./managedSubscriptions")
        ).managedProviderPorts.fetchXaiManagedModels(accountId),
    },
    workbuddy: {
      getStatus: () => invoke("get_workbuddy_status"),
      getModelIds: () => invoke("get_workbuddy_model_ids"),
      fetchModels: (request) => invoke("fetch_workbuddy_models", { request }),
      saveModels: (request) => invoke("save_workbuddy_models", { request }),
      checkReachability: invokeReachability,
      checkModel: invokeModelProbe,
    },
    opencodeModels: {
      restoreManagedProxy: async () =>
        (
          await import("./managedSubscriptions")
        ).managedOpenCodePorts.restoreManagedProxy(),
      bindManagedProxy: async (request) =>
        (
          await import("./managedSubscriptions")
        ).managedOpenCodePorts.bindManagedProxy(request),
      getSnapshot: async () =>
        parseOpenCodeModelSnapshot(
          await invoke<unknown>("get_opencode_model_snapshot"),
        ),
      fetchProviderModels: async (request) =>
        parseFetchedModelList(
          await invoke<unknown>("fetch_opencode_provider_models", {
            request: assertOpenCodeFetchRequest(request),
          }),
        ),
      saveModels: async (request) =>
        parseRevisionedSaveResult(
          await invoke<unknown>("save_opencode_models", {
            request: assertOpenCodeSaveRequest(request),
          }),
        ),
      checkReachability: invokeReachability,
      checkModel: invokeModelProbe,
    },
  };
}
