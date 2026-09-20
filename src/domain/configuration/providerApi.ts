export const API_PROTOCOLS = ["anthropic", "responses", "chat"] as const;
export type ApiProtocol = (typeof API_PROTOCOLS)[number];
export type ApiConfigTarget = "claude" | "codex" | "grokbuild";
export interface ApiConnection {
  baseUrl: string;
  modelId: string;
  protocol: ApiProtocol;
}
export const API_PROTOCOL_LABELS: Record<ApiProtocol, string> = {
  anthropic: "Anthropic Messages",
  responses: "OpenAI Responses",
  chat: "OpenAI Chat Completions",
};
export const isApiProtocol = (value: unknown): value is ApiProtocol =>
  API_PROTOCOLS.some((protocol) => protocol === value);
export function apiProtocolsForTarget(
  target: ApiConfigTarget,
): readonly ApiProtocol[] {
  return target === "claude"
    ? ["anthropic"]
    : target === "codex"
      ? ["responses", "chat"]
      : ["responses"];
}
export function defaultApiProtocol(target: ApiConfigTarget): ApiProtocol {
  return target === "claude" ? "anthropic" : "responses";
}

export interface ProviderApiPreset {
  id: string;
  name: string;
  region: string;
  keyScope: string;
  usage: string;
  modelId: string;
  endpoints: Partial<Record<ApiProtocol, string>>;
  docsUrl: string;
  toolOnly: boolean;
}
// Reviewed 2026-09-21 against the linked official product docs. These are the
// production form presets; no legacy Provider snapshot or credential is copied.
export const PROVIDER_API_PRESETS: readonly ProviderApiPreset[] = [
  {
    id: "aliyun-payg",
    name: "阿里百炼 · 按量 API",
    region: "中国内地（北京）",
    keyScope:
      "百炼按量 API Key；地域和工作空间需匹配，不能使用 Coding Plan 的 sk-sp Key。",
    usage:
      "按量计费。模型须在当前账号下可用，并支持所选协议；可改用官方工作空间专属地址。",
    modelId: "",
    toolOnly: false,
    endpoints: {
      responses: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      chat: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      anthropic: "https://dashscope.aliyuncs.com/apps/anthropic",
    },
    docsUrl: "https://help.aliyun.com/en/model-studio/base-url",
  },
  {
    id: "aliyun-coding",
    name: "阿里百炼 · Coding Plan",
    region: "中国内地（北京）",
    keyScope:
      "Coding Plan 专属 sk-sp- API Key，与百炼按量 Key 和 Token Plan Key 不互通。",
    usage:
      "仅用于套餐允许的交互式编程工具。本页不发送模型列表或测试请求，请在目标工具中验证。",
    modelId: "qwen3.7-plus",
    toolOnly: true,
    endpoints: {
      chat: "https://coding.dashscope.aliyuncs.com/v1",
      anthropic: "https://coding.dashscope.aliyuncs.com/apps/anthropic",
    },
    docsUrl: "https://help.aliyun.com/zh/model-studio/coding-plan",
  },
  {
    id: "tencent-tokenhub",
    name: "腾讯 TokenHub · Hy3",
    region: "中国内地",
    keyScope:
      "TokenHub API Key，创建时需授予 Hy3 范围；不是 SecretId/SecretKey，也不是 Coding Plan 或 Token Plan Key。",
    usage:
      "使用 TokenHub /v1 服务。其他腾讯接入产品的地址、Key 和额度不能混用。",
    modelId: "hy3",
    toolOnly: false,
    endpoints: { responses: "https://tokenhub.tencentmaas.com/v1" },
    docsUrl: "https://cloud.tencent.com/document/product/1823/133532",
  },
  {
    id: "ark-payg",
    name: "火山方舟 Ark · 普通 API",
    region: "中国内地（北京）",
    keyScope:
      "方舟普通 API Key，具备所选模型或推理接入点权限；不使用 Coding Plan 额度。",
    usage:
      "按量 API。填写账号内可用的模型 ID 或接入点 ID，并确认模型支持所选协议。",
    modelId: "",
    toolOnly: false,
    endpoints: { responses: "https://ark.cn-beijing.volces.com/api/v3" },
    docsUrl: "https://www.volcengine.com/docs/82379/1541595",
  },
  {
    id: "ark-coding",
    name: "火山方舟 Ark · Coding Plan",
    region: "中国内地（北京）",
    keyScope:
      "Coding Plan 专属 API Key，需已开通相应套餐；不要配到普通 /api/v3 地址。",
    usage:
      "仅使用套餐允许的编程工具和模型。本页不发送模型列表或测试请求，请在目标工具中验证。",
    modelId: "ark-code-latest",
    toolOnly: true,
    endpoints: {
      responses: "https://ark.cn-beijing.volces.com/api/coding/v3",
      anthropic: "https://ark.cn-beijing.volces.com/api/coding",
    },
    docsUrl: "https://www.volcengine.com/docs/82379/2556056",
  },
];
export function presetConnection(
  preset: ProviderApiPreset,
  target: ApiConfigTarget,
): ApiConnection | null {
  const protocol = apiProtocolsForTarget(target).find(
    (candidate) => preset.endpoints[candidate],
  );
  if (!protocol) return null;
  const baseUrl = preset.endpoints[protocol];
  return baseUrl ? { baseUrl, modelId: preset.modelId, protocol } : null;
}
export function presetForEndpoint(
  baseUrl: string,
): ProviderApiPreset | undefined {
  const normalized = baseUrl.trim().replace(/\/+$/u, "");
  return PROVIDER_API_PRESETS.find((preset) =>
    Object.values(preset.endpoints).includes(normalized),
  );
}
function parsedEndpoint(baseUrl: string): URL | null {
  try {
    return new URL(baseUrl.trim());
  } catch {
    return null;
  }
}
export function isToolOnlyApi(baseUrl: string, apiKey = ""): boolean {
  const url = parsedEndpoint(baseUrl);
  if (!url) return false;
  return (
    apiKey.trim().startsWith("sk-sp-") ||
    [
      "coding.dashscope.aliyuncs.com",
      "coding-intl.dashscope.aliyuncs.com",
      "token-plan.cn-beijing.maas.aliyuncs.com",
    ].includes(url.hostname) ||
    (url.hostname === "ark.cn-beijing.volces.com" &&
      /^\/api\/coding(?:\/|$)/u.test(url.pathname))
  );
}
export function providerApiCredentialError(
  baseUrl: string,
  apiKey: string,
  protocol?: ApiProtocol,
): string | null {
  const url = parsedEndpoint(baseUrl);
  if (!url) return null;
  if (
    [
      "coding.dashscope.aliyuncs.com",
      "coding-intl.dashscope.aliyuncs.com",
    ].includes(url.hostname)
  ) {
    if (!apiKey.trim().startsWith("sk-sp-"))
      return "请使用 Coding Plan 专属 sk-sp- API Key。";
    if (protocol === "responses")
      return "阿里 Coding Plan 此地址仅支持 Chat Completions 或 Anthropic Messages。";
  } else if (
    [
      "dashscope.aliyuncs.com",
      "dashscope-intl.aliyuncs.com",
      "dashscope-us.aliyuncs.com",
      "cn-hongkong.dashscope.aliyuncs.com",
    ].includes(url.hostname) &&
    apiKey.trim().startsWith("sk-sp-")
  )
    return "Coding Plan API Key 不能用于百炼按量地址，请检查接入产品。";
  return null;
}
