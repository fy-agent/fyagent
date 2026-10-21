import {
  OMP_SUGGESTED_PROVIDER_PREFIX,
  OMP_PROVIDER_APIS,
  hasControlCharacters,
  isValidOmpEnvName,
  isValidOmpModelId,
  isValidOmpProviderId,
  ompModelSelector,
  type OmpConfigRead,
  type OmpModelInput,
  type OmpProviderApi,
  type OmpProviderSummary,
  type OmpSaveRequest,
} from "../../shared/features/omp";
import { isHttpUrl } from "./quickSetup";

export const OMP_NEW_TARGET = "new";
export const OMP_SUGGESTED_NEW_PROVIDER_ID = `${OMP_SUGGESTED_PROVIDER_PREFIX}custom`;
export type OmpTargetSelection = typeof OMP_NEW_TARGET | `existing:${string}`;

export type OmpDraftAuth = "apiKey" | "none" | "oauth";
export type OmpKeySource = "keep" | "env" | "command";

export interface OmpDraft {
  providerId: string;
  baseUrl: string;
  api: OmpProviderApi;
  auth: OmpDraftAuth;
  keySource: OmpKeySource;
  envName: string;
  commandRef: string;
  models: OmpModelInput[];
  setDefault: boolean;
  defaultModelId: string;
}

export type OmpDraftField =
  | "providerId"
  | "baseUrl"
  | "keyRef"
  | "models"
  | "defaultModel";
export type OmpDraftErrors = Partial<Record<OmpDraftField, string>>;

export const OMP_API_LABELS: Readonly<Record<OmpProviderApi, string>> = {
  "openai-completions": "OpenAI Chat Completions",
  "openai-responses": "OpenAI Responses",
};

export function isOmpProviderApi(value: unknown): value is OmpProviderApi {
  return OMP_PROVIDER_APIS.some((api) => api === value);
}

/**
 * omp also accepts protocols FyAgent cannot write (e.g. Codex, Anthropic or
 * Google transports). Those providers stay visible but are edited in omp itself.
 */
export function isEditableOmpProvider(provider: OmpProviderSummary): boolean {
  return provider.api === null || isOmpProviderApi(provider.api);
}

export function providerById(
  read: OmpConfigRead | undefined,
  selection: OmpTargetSelection,
): OmpProviderSummary | null {
  if (!read || selection === OMP_NEW_TARGET) return null;
  const id = selection.slice("existing:".length);
  return read.providers.find((provider) => provider.id === id) ?? null;
}

export function defaultOmpSelection(
  read: OmpConfigRead | undefined,
): OmpTargetSelection {
  const editable = read?.providers.find(isEditableOmpProvider);
  return editable ? `existing:${editable.id}` : OMP_NEW_TARGET;
}

function normalizedAuth(provider: OmpProviderSummary | null): OmpDraftAuth {
  if (provider?.auth === "none" || provider?.auth === "oauth")
    return provider.auth;
  return "apiKey";
}

export function draftFromProvider(
  provider: OmpProviderSummary | null,
  defaultModel: string | null,
): OmpDraft {
  const prefix = provider ? `${provider.id}/` : null;
  const defaultModelId =
    prefix && defaultModel?.startsWith(prefix)
      ? defaultModel.slice(prefix.length)
      : "";
  return {
    providerId: provider?.id ?? OMP_SUGGESTED_NEW_PROVIDER_ID,
    baseUrl: provider?.baseUrl ?? "",
    api:
      provider?.api && isOmpProviderApi(provider.api)
        ? provider.api
        : "openai-completions",
    auth: normalizedAuth(provider),
    keySource: provider?.hasApiKey ? "keep" : "env",
    envName: "",
    commandRef: "",
    models: [],
    setDefault: false,
    defaultModelId,
  };
}

/** Draft entries first, then file models with their current names unchanged. */
export function planModels(
  draft: OmpDraft,
  provider: OmpProviderSummary | null,
): OmpModelInput[] {
  const seen = new Set<string>();
  const output: OmpModelInput[] = [];
  for (const model of [
    ...draft.models,
    ...(provider?.models.map((item) => ({ id: item.id, name: item.name })) ??
      []),
  ]) {
    if (seen.has(model.id)) continue;
    seen.add(model.id);
    output.push(model);
  }
  return output;
}

export function validateOmpDraft(
  draft: OmpDraft,
  provider: OmpProviderSummary | null,
  read: OmpConfigRead,
): OmpDraftErrors {
  const errors: OmpDraftErrors = {};
  const id = draft.providerId.trim();
  if (!provider) {
    if (!isValidOmpProviderId(id)) {
      errors.providerId =
        "供应商 ID 只能包含字母、数字、点、下划线和连字符，最多 80 个字符。";
    } else if (read.providers.some((item) => item.id === id)) {
      errors.providerId = "该供应商已存在，请在上方选择它进行修改。";
    }
  }
  if (!isHttpUrl(draft.baseUrl.trim())) {
    errors.baseUrl = "请输入不含账号、查询参数或片段的 HTTP(S) 地址。";
  }
  if (draft.auth === "apiKey") {
    if (draft.keySource === "keep" && !provider?.hasApiKey) {
      errors.keyRef = "当前没有可保留的密钥引用，请填写环境变量或命令引用。";
    } else if (
      draft.keySource === "env" &&
      !isValidOmpEnvName(draft.envName.trim())
    ) {
      errors.keyRef = "请输入环境变量名，例如 MY_GATEWAY_API_KEY。";
    } else if (draft.keySource === "command") {
      const ref = draft.commandRef.trim();
      if (
        !ref.startsWith("!") ||
        ref.length < 2 ||
        ref.length > 512 ||
        hasControlCharacters(ref)
      ) {
        errors.keyRef = "命令引用需以 ! 开头，例如 !op read op://vault/key。";
      }
    }
  }
  const models = planModels(draft, provider);
  if (models.length === 0) {
    errors.models = "请至少添加一个模型 ID。";
  } else if (!models.every((model) => isValidOmpModelId(model.id))) {
    errors.models = "模型 ID 不能为空，也不能以 / 开头或结尾。";
  }
  if (
    draft.setDefault &&
    !models.some((model) => model.id === draft.defaultModelId)
  ) {
    errors.defaultModel = "请选择要设为默认的模型。";
  }
  return errors;
}

export function buildOmpSaveRequest(
  draft: OmpDraft,
  provider: OmpProviderSummary | null,
  read: OmpConfigRead,
): OmpSaveRequest {
  const id = provider?.id ?? draft.providerId.trim();
  const apiKey =
    draft.auth !== "apiKey" || draft.keySource === "keep"
      ? null
      : draft.keySource === "env"
        ? draft.envName.trim()
        : draft.commandRef.trim();
  const auth =
    draft.auth === "oauth" ||
    (provider && draft.auth === normalizedAuth(provider))
      ? null
      : draft.auth;
  const request: OmpSaveRequest = {
    provider: {
      id,
      baseUrl: draft.baseUrl.trim(),
      api: draft.api,
      apiKey,
      auth,
      models: planModels(draft, provider).map((model) => ({ ...model })),
    },
    defaultModel: draft.setDefault
      ? ompModelSelector(id, draft.defaultModelId)
      : null,
    expectedModelsRevision: read.modelsRevision,
    expectedConfigRevision: read.configRevision,
  };
  return request;
}
