import { MODEL_DIRECTORY_IDS } from "../../shared/features/directory";

export const MODEL_TARGETS = MODEL_DIRECTORY_IDS;

export type ModelTarget = (typeof MODEL_TARGETS)[number];
export type ProviderQuickSetupTarget = Extract<
  ModelTarget,
  "claude" | "codex" | "grokbuild"
>;

export const QUICK_SETUP_PROVIDER_IDS: Record<
  ProviderQuickSetupTarget,
  string
> = {
  claude: "fyagent-v2-quick-setup-claude",
  codex: "fyagent-v2-quick-setup-codex",
  grokbuild: "fyagent-v2-quick-setup-grokbuild",
};

export interface QuickSetupFormInput {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

export type QuickSetupField = keyof QuickSetupFormInput;
export type QuickSetupErrors = Partial<Record<QuickSetupField, string>>;

export interface NormalizedQuickSetupInput {
  name: string;
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

export type QuickSetupValidation =
  | { ok: true; value: NormalizedQuickSetupInput }
  | { ok: false; errors: QuickSetupErrors };

export const isHttpUrl = (value: string): boolean => {
  try {
    const parsed = new URL(value);
    return (
      (parsed.protocol === "http:" || parsed.protocol === "https:") &&
      Boolean(parsed.hostname) &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.search === "" &&
      parsed.hash === ""
    );
  } catch {
    return false;
  }
};

export function parseModelTarget(value: string | null): ModelTarget {
  return MODEL_TARGETS.includes(value as ModelTarget)
    ? (value as ModelTarget)
    : "qoderwork";
}

export function validateQuickSetup(
  input: QuickSetupFormInput,
  target?: ProviderQuickSetupTarget,
): QuickSetupValidation {
  const value: NormalizedQuickSetupInput = {
    name: input.name.trim(),
    baseUrl: input.baseUrl.trim(),
    apiKey: input.apiKey.trim(),
    modelId: input.modelId.trim(),
  };
  const errors: QuickSetupErrors = {};

  if (!value.name) errors.name = "请输入配置名称";
  if (!isHttpUrl(value.baseUrl))
    errors.baseUrl = "请输入不含账号信息的 HTTP(S) 地址";
  if (!value.apiKey) errors.apiKey = "请输入 API Key";
  if (!value.modelId) errors.modelId = "请输入模型 ID";
  if (value.apiKey && value.name.includes(value.apiKey))
    errors.name = "配置名称不能包含 API Key";
  if (value.apiKey && value.modelId.includes(value.apiKey))
    errors.modelId = "模型 ID 不能包含 API Key";
  if (target && QUICK_SETUP_PROVIDER_IDS[target].includes(value.apiKey))
    errors.apiKey = "API Key 不能使用该值";
  if (value.apiKey && isHttpUrl(value.baseUrl)) {
    const parsed = new URL(value.baseUrl);
    const pathCollision = parsed.pathname.split("/").some((segment) => {
      if (segment.includes(value.apiKey)) return true;
      try {
        return decodeURIComponent(segment).includes(value.apiKey);
      } catch {
        return false;
      }
    });
    if (
      parsed.hostname.includes(value.apiKey.toLocaleLowerCase("en-US")) ||
      pathCollision
    )
      errors.baseUrl = "服务地址不能包含 API Key";
  }

  return Object.keys(errors).length
    ? { ok: false, errors }
    : { ok: true, value };
}

export function buildQuickSetupRequest(
  target: ProviderQuickSetupTarget,
  input: NormalizedQuickSetupInput,
  codexFeatures?: import("../../shared/features/types").ProviderQuickSetupRequest["codexFeatures"],
): import("../../shared/features/types").ProviderQuickSetupRequest {
  return {
    name: input.name,
    baseUrl: input.baseUrl,
    apiKey: input.apiKey,
    modelId: input.modelId,
    ...(target === "codex" && codexFeatures ? { codexFeatures } : {}),
  };
}

export function parseManualModelIds(value: string): string[] {
  const seen = new Set<string>();
  const result: string[] = [];

  for (const candidate of value.split(/[\n,]/u)) {
    const id = candidate.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    result.push(id);
  }

  return result;
}
