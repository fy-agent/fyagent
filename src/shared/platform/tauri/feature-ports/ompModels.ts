import { invoke } from "@tauri-apps/api/core";

import {
  OMP_CHANGED_FILES,
  OMP_FILE_STATUSES,
  OmpPortError,
  classifyOmpNativeError,
  type OmpApplyConfirmation,
  type OmpCliDiscovery,
  type OmpConfigRead,
  type OmpModelSummary,
  type OmpModelsPort,
  type OmpProviderSummary,
  type OmpSavePreview,
  type OmpSaveRequest,
  type OmpSaveResult,
} from "../../../features/omp";
import { hasExactKeys, isOneOf, isRecord, isStringArray } from "./validation";

function parseModelSummary(value: unknown): OmpModelSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["id", "name"]) ||
    typeof value.id !== "string" ||
    value.id.length === 0 ||
    (value.name !== null && typeof value.name !== "string")
  ) {
    throw new OmpPortError("unknown");
  }
  return { id: value.id, name: value.name };
}

function parseProviderSummary(value: unknown): OmpProviderSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "id",
      "baseUrl",
      "api",
      "auth",
      "hasApiKey",
      "hasCommandKey",
      "models",
    ]) ||
    typeof value.id !== "string" ||
    (value.baseUrl !== null && typeof value.baseUrl !== "string") ||
    (value.api !== null && typeof value.api !== "string") ||
    (value.auth !== null && typeof value.auth !== "string") ||
    typeof value.hasApiKey !== "boolean" ||
    typeof value.hasCommandKey !== "boolean" ||
    !Array.isArray(value.models)
  ) {
    throw new OmpPortError("unknown");
  }
  return {
    id: value.id,
    baseUrl: value.baseUrl,
    api: value.api,
    auth: value.auth,
    hasApiKey: value.hasApiKey,
    hasCommandKey: value.hasCommandKey,
    models: value.models.map(parseModelSummary),
  };
}

function parseOmpConfigRead(value: unknown): OmpConfigRead {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "profile",
      "modelsPath",
      "configPath",
      "modelsRevision",
      "configRevision",
      "providers",
      "defaultModel",
      "modelsFileStatus",
      "configFileStatus",
    ]) ||
    typeof value.profile !== "string" ||
    typeof value.modelsPath !== "string" ||
    typeof value.configPath !== "string" ||
    typeof value.modelsRevision !== "string" ||
    typeof value.configRevision !== "string" ||
    !Array.isArray(value.providers) ||
    (value.defaultModel !== null && typeof value.defaultModel !== "string") ||
    !isOneOf(value.modelsFileStatus, OMP_FILE_STATUSES) ||
    !isOneOf(value.configFileStatus, OMP_FILE_STATUSES)
  ) {
    throw new OmpPortError("unknown");
  }
  return {
    profile: value.profile,
    modelsPath: value.modelsPath,
    configPath: value.configPath,
    modelsRevision: value.modelsRevision,
    configRevision: value.configRevision,
    providers: value.providers.map(parseProviderSummary),
    defaultModel: value.defaultModel,
    modelsFileStatus: value.modelsFileStatus,
    configFileStatus: value.configFileStatus,
  };
}

function parseOmpCliDiscovery(value: unknown): OmpCliDiscovery {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["detected", "version", "executablePath"]) ||
    typeof value.detected !== "boolean" ||
    (value.version !== null && typeof value.version !== "string") ||
    (value.executablePath !== null && typeof value.executablePath !== "string")
  ) {
    throw new OmpPortError("unknown");
  }
  return {
    detected: value.detected,
    version: value.version,
    executablePath: value.executablePath,
  };
}

function parseOmpSavePreview(value: unknown): OmpSavePreview {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "planId",
      "planDigest",
      "profile",
      "modelsPath",
      "configPath",
      "provider",
      "defaultModel",
      "changedFiles",
      "warnings",
    ]) ||
    typeof value.planId !== "string" ||
    typeof value.planDigest !== "string" ||
    typeof value.profile !== "string" ||
    typeof value.modelsPath !== "string" ||
    typeof value.configPath !== "string" ||
    (value.defaultModel !== null && typeof value.defaultModel !== "string") ||
    !Array.isArray(value.changedFiles) ||
    !value.changedFiles.every((item) => isOneOf(item, OMP_CHANGED_FILES)) ||
    !isStringArray(value.warnings)
  ) {
    throw new OmpPortError("unknown");
  }
  return {
    planId: value.planId,
    planDigest: value.planDigest,
    profile: value.profile,
    modelsPath: value.modelsPath,
    configPath: value.configPath,
    provider: parseProviderSummary(value.provider),
    defaultModel: value.defaultModel,
    changedFiles: value.changedFiles,
    warnings: value.warnings,
  };
}

function parseOmpSaveResult(value: unknown): OmpSaveResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "modelsPath",
      "configPath",
      "modelsRevision",
      "configRevision",
      "provider",
      "defaultModel",
      "modelsWritten",
      "configWritten",
    ]) ||
    typeof value.modelsPath !== "string" ||
    typeof value.configPath !== "string" ||
    typeof value.modelsRevision !== "string" ||
    typeof value.configRevision !== "string" ||
    (value.defaultModel !== null && typeof value.defaultModel !== "string") ||
    typeof value.modelsWritten !== "boolean" ||
    typeof value.configWritten !== "boolean"
  ) {
    throw new OmpPortError("unknown");
  }
  return {
    modelsPath: value.modelsPath,
    configPath: value.configPath,
    modelsRevision: value.modelsRevision,
    configRevision: value.configRevision,
    provider: parseProviderSummary(value.provider),
    defaultModel: value.defaultModel,
    modelsWritten: value.modelsWritten,
    configWritten: value.configWritten,
  };
}

async function runOmpInvoke<T>(
  call: () => Promise<unknown>,
  parse: (raw: unknown) => T,
): Promise<T> {
  try {
    const raw = await call();
    return parse(raw);
  } catch (error) {
    if (error instanceof OmpPortError) throw error;
    throw new OmpPortError(classifyOmpNativeError(error));
  }
}

export function createOmpModelsPort(): OmpModelsPort {
  return {
    readConfig: () =>
      runOmpInvoke(
        () => invoke<unknown>("read_omp_config"),
        parseOmpConfigRead,
      ),
    discoverCli: () =>
      runOmpInvoke(
        () => invoke<unknown>("discover_omp_cli"),
        parseOmpCliDiscovery,
      ),
    previewSave: (request: OmpSaveRequest) =>
      runOmpInvoke(
        () => invoke<unknown>("preview_omp_save", { plan: request }),
        parseOmpSavePreview,
      ),
    applySave: (confirmation: OmpApplyConfirmation) =>
      runOmpInvoke(
        () => invoke<unknown>("apply_omp_save", { plan: confirmation }),
        parseOmpSaveResult,
      ),
  };
}
