import { invoke } from "@tauri-apps/api/core";
import type { FeaturePorts } from "../../../features/ports";
import type {
  BindXaiManagedRequest,
  BindXaiManagedResult,
  BindManagedProxyRequest,
  BindOpenCodeManagedRequest,
} from "../../../features/types";
import {
  isXaiSubscriptionModelId,
  xaiBindErrorCode,
} from "../../../features/xai-subscription";
import { hasExactKeys, isOneOf, isRecord } from "./validation";

function isManagedAccountId(value: unknown): value is string {
  return typeof value === "string" && /^ma1:[0-9a-f]{32}$/u.test(value);
}

function assertSubscriptionBindRequest<
  T extends BindXaiManagedRequest | BindManagedProxyRequest,
>(
  request: T,
  apps: readonly (
    | BindXaiManagedRequest["app"]
    | BindManagedProxyRequest["app"]
  )[],
): T {
  if (
    !isRecord(request) ||
    !hasExactKeys(request, ["app", "accountId", "modelId"]) ||
    !isOneOf(request.app, apps) ||
    !isManagedAccountId(request.accountId) ||
    !isXaiSubscriptionModelId(request.modelId)
  )
    throw new Error("Subscription bind request is invalid");
  return request;
}

function parseSubscriptionBindResult<
  T extends {
    app:
      | BindXaiManagedRequest["app"]
      | BindManagedProxyRequest["app"]
      | "opencode";
  },
>(
  value: unknown,
  request: T,
): Omit<BindXaiManagedResult, "app"> & { app: T["app"] } {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "providerId",
      "providerName",
      "app",
      "alreadyBound",
      "activated",
    ]) ||
    typeof value.providerId !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9:._-]{0,159}$/u.test(value.providerId) ||
    typeof value.providerName !== "string" ||
    !value.providerName.trim() ||
    value.providerName.length > 200 ||
    /[\r\n\0]/u.test(value.providerName) ||
    value.app !== request.app ||
    typeof value.alreadyBound !== "boolean" ||
    typeof value.activated !== "boolean" ||
    value.activated !==
      (request.app !== "codex" && request.app !== "claude-desktop")
  )
    throw new Error("Subscription bind result is unavailable");
  return {
    providerId: value.providerId,
    providerName: value.providerName,
    app: request.app,
    alreadyBound: value.alreadyBound,
    activated: value.activated,
  };
}

function parseXaiManagedModels(value: unknown): {
  models: string[];
  truncated: boolean;
} {
  if (!Array.isArray(value) || value.length > 2_000)
    throw new Error("xAI models are unavailable");
  const models: string[] = [];
  for (const entry of value) {
    const id =
      typeof entry === "string" ? entry : isRecord(entry) ? entry.id : null;
    if (!isXaiSubscriptionModelId(id))
      throw new Error("xAI models are unavailable");
    if (!models.includes(id)) models.push(id);
  }
  return { models, truncated: false };
}

async function invokeSubscriptionBind<
  T extends
    | BindXaiManagedRequest["app"]
    | BindManagedProxyRequest["app"]
    | "opencode",
>(response: Promise<unknown>, app: T) {
  try {
    return parseSubscriptionBindResult(await response, { app });
  } catch (error) {
    throw { code: xaiBindErrorCode(error) ?? "rollback_partial_state_unknown" };
  }
}

export const managedProviderPorts: Pick<
  FeaturePorts["providers"],
  "bindXaiManaged" | "bindManagedProxy" | "fetchXaiManagedModels"
> = {
  bindXaiManaged: async (request) => {
    const validated = assertSubscriptionBindRequest(request, [
      "claude",
      "claude-desktop",
      "codex",
    ]);
    return invokeSubscriptionBind(
      invoke<unknown>("bind_xai_managed_provider", { request: validated }),
      validated.app,
    );
  },
  bindManagedProxy: async (request) => {
    const validated = assertSubscriptionBindRequest(request, [
      "claude",
      "codex",
      "grokbuild",
    ]);
    return invokeSubscriptionBind(
      invoke<unknown>("bind_managed_proxy_provider", { request: validated }),
      validated.app,
    );
  },
  fetchXaiManagedModels: async (accountId) => {
    if (!isManagedAccountId(accountId))
      throw new Error("xAI account selection is invalid");
    return parseXaiManagedModels(
      await invoke<unknown>("get_xai_oauth_models", { accountId }),
    );
  },
};

export const managedOpenCodePorts: Pick<
  FeaturePorts["opencodeModels"],
  "restoreManagedProxy" | "bindManagedProxy"
> = {
  restoreManagedProxy: async () => {
    if (
      (await invoke<unknown>("set_proxy_takeover_for_app", {
        appType: "opencode",
        enabled: false,
      })) !== null
    )
      throw new Error("OpenCode restore is unconfirmed");
  },
  bindManagedProxy: async (request: BindOpenCodeManagedRequest) => {
    if (
      !isRecord(request) ||
      !hasExactKeys(request, ["accountId", "modelId", "expectedRevision"]) ||
      !isManagedAccountId(request.accountId) ||
      !isXaiSubscriptionModelId(request.modelId) ||
      (request.expectedRevision !== null &&
        (typeof request.expectedRevision !== "string" ||
          !request.expectedRevision ||
          request.expectedRevision.length > 160))
    )
      throw new Error("Subscription bind request is invalid");
    return invokeSubscriptionBind(
      invoke<unknown>("bind_opencode_managed_proxy", { request }),
      "opencode",
    );
  },
};
