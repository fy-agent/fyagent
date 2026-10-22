import { invoke } from "@tauri-apps/api/core";

import {
  OMP_CHANGED_FILES,
  OMP_FILE_STATUSES,
  OmpPortError,
  classifyOmpNativeError,
  type OmpChangedFile,
  type OmpCliDiscovery,
  type OmpConfigRead,
  type OmpModelSummary,
  type OmpModelsPort,
  type OmpProviderSummary,
  type OmpSavePreview,
  type OmpSaveRequest,
  type OmpSaveResult,
} from "../../../features/omp";
import { hasExactKeys, isOneOf, isRecord } from "./validation";

function fail(): never {
  throw new OmpPortError("unknown");
}

function text(value: unknown): string {
  if (typeof value !== "string" || value.length === 0) fail();
  return value;
}

function nullableText(value: unknown): string | null {
  if (value === null) return null;
  if (typeof value !== "string") fail();
  return value;
}

function bool(value: unknown): boolean {
  if (typeof value !== "boolean") fail();
  return value;
}

function parseModel(value: unknown): OmpModelSummary {
  if (!isRecord(value) || !hasExactKeys(value, ["id", "name"])) fail();
  return { id: text(value.id), name: nullableText(value.name) };
}

export function parseOmpProviderSummary(value: unknown): OmpProviderSummary {
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
    !Array.isArray(value.models)
  )
    fail();
  const hasApiKey = bool(value.hasApiKey);
  const hasCommandKey = bool(value.hasCommandKey);
  if (hasCommandKey && !hasApiKey) fail();
  const models = value.models.map(parseModel);
  if (new Set(models.map((model) => model.id)).size !== models.length) fail();
  return {
    id: text(value.id),
    baseUrl: nullableText(value.baseUrl),
    api: nullableText(value.api),
    auth: nullableText(value.auth),
    hasApiKey,
    hasCommandKey,
    models,
  };
}

function parseUniqueProviders(value: unknown): OmpProviderSummary[] {
  if (!Array.isArray(value)) fail();
  const providers = value.map(parseOmpProviderSummary);
  if (
    new Set(providers.map((provider) => provider.id)).size !== providers.length
  )
    fail();
  return providers;
}

export function parseOmpConfigRead(value: unknown): OmpConfigRead {
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
    !isOneOf(value.modelsFileStatus, OMP_FILE_STATUSES) ||
    !isOneOf(value.configFileStatus, OMP_FILE_STATUSES)
  )
    fail();
  return {
    profile: text(value.profile),
    modelsPath: text(value.modelsPath),
    configPath: text(value.configPath),
    modelsRevision: text(value.modelsRevision),
    configRevision: text(value.configRevision),
    providers: parseUniqueProviders(value.providers),
    defaultModel: nullableText(value.defaultModel),
    modelsFileStatus: value.modelsFileStatus,
    configFileStatus: value.configFileStatus,
  };
}

function parseChangedFiles(value: unknown): OmpChangedFile[] {
  if (!Array.isArray(value)) fail();
  const files = value.map((item) => {
    if (!isOneOf(item, OMP_CHANGED_FILES)) fail();
    return item;
  });
  if (new Set(files).size !== files.length) fail();
  return files;
}

function parseWarnings(value: unknown): string[] {
  if (!Array.isArray(value)) fail();
  return value.map(text);
}

/** Binds the preview to the submitted request so a mismatched plan is never confirmed. */
export function parseOmpSavePreview(
  value: unknown,
  request: OmpSaveRequest,
): OmpSavePreview {
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
    ])
  )
    fail();
  const preview: OmpSavePreview = {
    planId: text(value.planId),
    planDigest: text(value.planDigest),
    profile: text(value.profile),
    modelsPath: text(value.modelsPath),
    configPath: text(value.configPath),
    provider: parseOmpProviderSummary(value.provider),
    defaultModel: nullableText(value.defaultModel),
    changedFiles: parseChangedFiles(value.changedFiles),
    warnings: parseWarnings(value.warnings),
  };
  const requested = request.provider;
  const previewModelIds = new Set(preview.provider.models.map(({ id }) => id));
  const requestedDefault = request.defaultModel ?? null;
  if (
    preview.provider.id !== requested.id ||
    preview.provider.baseUrl !== requested.baseUrl ||
    preview.provider.api !== requested.api ||
    !requested.models.every(({ id }) => previewModelIds.has(id)) ||
    (requestedDefault !== null && preview.defaultModel !== requestedDefault)
  )
    fail();
  return preview;
}

export function parseOmpSaveResult(value: unknown): OmpSaveResult {
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
    ])
  )
    fail();
  return {
    modelsPath: text(value.modelsPath),
    configPath: text(value.configPath),
    modelsRevision: text(value.modelsRevision),
    configRevision: text(value.configRevision),
    provider: parseOmpProviderSummary(value.provider),
    defaultModel: nullableText(value.defaultModel),
    modelsWritten: bool(value.modelsWritten),
    configWritten: bool(value.configWritten),
  };
}

export function parseOmpCliDiscovery(value: unknown): OmpCliDiscovery {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["detected", "version", "executablePath"])
  )
    fail();
  const detected = bool(value.detected);
  const version = nullableText(value.version);
  const executablePath = nullableText(value.executablePath);
  if (!detected && version !== null) fail();
  return { detected, version, executablePath };
}

async function call<T>(run: () => Promise<T>): Promise<T> {
  try {
    return await run();
  } catch (cause) {
    if (cause instanceof OmpPortError) throw cause;
    throw new OmpPortError(classifyOmpNativeError(cause));
  }
}

export function createOmpModelsPort(): OmpModelsPort {
  return {
    readConfig: () =>
      call(async () =>
        parseOmpConfigRead(await invoke<unknown>("read_omp_config")),
      ),
    discoverCli: () =>
      call(async () =>
        parseOmpCliDiscovery(await invoke<unknown>("discover_omp_cli")),
      ),
    previewSave: (request) =>
      call(async () =>
        parseOmpSavePreview(
          await invoke<unknown>("preview_omp_save", { plan: request }),
          request,
        ),
      ),
    applySave: (confirmation) =>
      call(async () =>
        parseOmpSaveResult(
          await invoke<unknown>("apply_omp_save", {
            plan: {
              planId: confirmation.planId,
              planDigest: confirmation.planDigest,
            },
          }),
        ),
      ),
  };
}
