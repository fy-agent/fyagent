import { invoke } from "@tauri-apps/api/core";

import type { FeaturePorts } from "../../../features/ports";
import {
  EXTERNAL_MCP_FINDING_REASON_CODES,
  EXTERNAL_MCP_TRANSPORTS,
  QODERWORK_HOOK_EVENTS,
  TRAE_MODEL_API_FORMATS,
  TRAE_MODEL_DURATION_BUCKETS,
  TRAE_MODEL_RESULT_REASON_CODES,
  TRAE_MODEL_RESULT_STATES,
  TRAE_MODEL_STATUS_CLASSES,
  TRAE_MODEL_URL_MODES,
  type CancelTraeModelProbeResult,
  type ExternalMcpAgentId,
  type ExternalMcpFinding,
  type ExternalMcpValidationResult,
  type QoderWorkCommandHook,
  type QoderWorkHookGroup,
  type QoderWorkHooksSnapshot,
  type SaveQoderWorkHooksRequest,
  type SaveQoderWorkHooksResult,
  type TraeModelProbeResult,
  type TraeModelValidationResult,
  type TraeWorkModelIdsResult,
  type TraeWorkModelRequest,
} from "../../../features/types";
import {
  hasExactKeys,
  hasRequiredAndOptionalKeys,
  isOneOf,
  isRecord,
  isStringArray,
} from "./validation";

function parseQoderWorkCommandHook(value: unknown): QoderWorkCommandHook {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(value, ["type", "command"], ["timeout"]) ||
    value.type !== "command" ||
    typeof value.command !== "string" ||
    value.command.trim().length === 0 ||
    value.command.length > 4_096 ||
    value.command.includes("\0") ||
    (value.timeout !== undefined &&
      (!Number.isInteger(value.timeout) ||
        (value.timeout as number) <= 0 ||
        (value.timeout as number) > 600))
  )
    throw new Error("QoderWork Hooks are unavailable");
  return {
    type: "command",
    command: value.command,
    ...(value.timeout === undefined
      ? {}
      : { timeout: value.timeout as number }),
  };
}

function parseQoderWorkHookGroup(value: unknown): QoderWorkHookGroup {
  if (
    !isRecord(value) ||
    !hasRequiredAndOptionalKeys(value, ["event", "hooks"], ["matcher"]) ||
    !isOneOf(value.event, QODERWORK_HOOK_EVENTS) ||
    (value.matcher !== undefined &&
      (typeof value.matcher !== "string" ||
        value.matcher.length > 4_096 ||
        value.matcher.includes("\0"))) ||
    !Array.isArray(value.hooks) ||
    value.hooks.length > 64
  )
    throw new Error("QoderWork Hooks are unavailable");
  return {
    event: value.event,
    ...(value.matcher === undefined ? {} : { matcher: value.matcher }),
    hooks: value.hooks.map(parseQoderWorkCommandHook),
  };
}

function parseQoderWorkHooksSnapshot(value: unknown): QoderWorkHooksSnapshot {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "revision",
      "exists",
      "groups",
      "restartRequired",
      "supportedStructure",
    ]) ||
    (value.revision !== null &&
      (typeof value.revision !== "string" ||
        value.revision.trim().length === 0 ||
        value.revision.trim() !== value.revision)) ||
    typeof value.exists !== "boolean" ||
    !Array.isArray(value.groups) ||
    value.groups.length > 256 ||
    value.restartRequired !== true ||
    typeof value.supportedStructure !== "boolean"
  )
    throw new Error("QoderWork Hooks are unavailable");
  const groups = value.groups.map(parseQoderWorkHookGroup);
  if (groups.reduce((count, group) => count + group.hooks.length, 0) > 1_024)
    throw new Error("QoderWork Hooks are unavailable");
  return {
    revision: value.revision,
    exists: value.exists,
    groups,
    restartRequired: true,
    supportedStructure: value.supportedStructure,
  };
}

function assertQoderWorkHooksRequest(
  request: SaveQoderWorkHooksRequest,
): SaveQoderWorkHooksRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      ["groups"],
      ["expectedRevision", "overwriteToken"],
    ) ||
    (request.expectedRevision !== undefined &&
      request.expectedRevision !== null &&
      (typeof request.expectedRevision !== "string" ||
        request.expectedRevision.trim().length === 0 ||
        request.expectedRevision.trim() !== request.expectedRevision)) ||
    (request.overwriteToken !== undefined &&
      (typeof request.overwriteToken !== "string" ||
        request.overwriteToken.trim().length === 0 ||
        request.overwriteToken.trim() !== request.overwriteToken)) ||
    !Array.isArray(request.groups)
  )
    throw new Error("QoderWork Hooks request is invalid");
  return {
    ...(request.expectedRevision === undefined
      ? {}
      : { expectedRevision: request.expectedRevision }),
    groups: request.groups.map(parseQoderWorkHookGroup),
    ...(request.overwriteToken === undefined
      ? {}
      : { overwriteToken: request.overwriteToken }),
  };
}

function parseSaveQoderWorkHooksResult(
  value: unknown,
): SaveQoderWorkHooksResult {
  if (!isRecord(value) || typeof value.state !== "string")
    throw new Error("QoderWork Hooks save result is unavailable");
  switch (value.state) {
    case "saved":
      if (!hasExactKeys(value, ["state", "snapshot"]))
        throw new Error("QoderWork Hooks save result is unavailable");
      return {
        state: "saved",
        snapshot: parseQoderWorkHooksSnapshot(value.snapshot),
      };
    case "overwrite_confirmation_required":
      if (
        !hasExactKeys(value, ["state", "token"]) ||
        typeof value.token !== "string" ||
        value.token.trim().length === 0 ||
        value.token.trim() !== value.token
      )
        throw new Error("QoderWork Hooks save result is unavailable");
      return { state: "overwrite_confirmation_required", token: value.token };
    case "concurrent_modification":
      if (!hasExactKeys(value, ["state"]))
        throw new Error("QoderWork Hooks save result is unavailable");
      return { state: "concurrent_modification" };
    default:
      throw new Error("QoderWork Hooks save result is unavailable");
  }
}

function assertExternalMcpAgentId(
  agentId: ExternalMcpAgentId,
): ExternalMcpAgentId {
  if (agentId !== "qoderwork" && agentId !== "trae-work")
    throw new Error("External MCP validation request is invalid");
  return agentId;
}

function assertExternalMcpConfig(
  config: Record<string, unknown>,
): Record<string, unknown> {
  if (
    !isRecord(config) ||
    !hasExactKeys(config, ["mcpServers"]) ||
    !isRecord(config.mcpServers)
  )
    throw new Error("External MCP validation request is invalid");
  return config;
}

function externalMcpSecretValues(config: Record<string, unknown>): string[] {
  if (!isRecord(config.mcpServers)) return [];
  const values: string[] = [];
  for (const server of Object.values(config.mcpServers)) {
    if (!isRecord(server)) continue;
    for (const field of ["env", "headers"] as const) {
      const secretMap = server[field];
      if (!isRecord(secretMap)) continue;
      for (const candidate of Object.values(secretMap)) {
        if (typeof candidate === "string" && candidate.length > 0)
          values.push(candidate);
      }
    }
  }
  return values;
}

function parseExternalMcpFinding(value: unknown): ExternalMcpFinding {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "serverId",
      "transport",
      "reasonCodes",
      "executableAvailable",
      "hasSecrets",
    ]) ||
    typeof value.serverId !== "string" ||
    value.serverId.trim().length === 0 ||
    value.serverId.trim() !== value.serverId ||
    !isOneOf(value.transport, EXTERNAL_MCP_TRANSPORTS) ||
    !Array.isArray(value.reasonCodes) ||
    value.reasonCodes.length === 0 ||
    (value.executableAvailable !== null &&
      typeof value.executableAvailable !== "boolean") ||
    typeof value.hasSecrets !== "boolean"
  )
    throw new Error("External MCP validation result is unavailable");
  const reasonCodes = value.reasonCodes.map((reasonCode) => {
    if (!isOneOf(reasonCode, EXTERNAL_MCP_FINDING_REASON_CODES))
      throw new Error("External MCP validation result is unavailable");
    return reasonCode;
  });
  if (new Set(reasonCodes).size !== reasonCodes.length)
    throw new Error("External MCP validation result is unavailable");
  return {
    serverId: value.serverId,
    transport: value.transport,
    reasonCodes,
    executableAvailable: value.executableAvailable,
    hasSecrets: value.hasSecrets,
  };
}

function parseExternalMcpValidationResult(
  value: unknown,
  requestedAgentId: ExternalMcpAgentId,
  config: Record<string, unknown>,
): ExternalMcpValidationResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "agentId",
      "valid",
      "findings",
      "redactedTemplate",
    ]) ||
    value.agentId !== requestedAgentId ||
    typeof value.valid !== "boolean" ||
    !Array.isArray(value.findings) ||
    !isRecord(value.redactedTemplate)
  )
    throw new Error("External MCP validation result is unavailable");
  const findings = value.findings.map(parseExternalMcpFinding);
  if (
    new Set(findings.map((finding) => finding.serverId)).size !==
    findings.length
  )
    throw new Error("External MCP validation result is unavailable");

  const redactedTemplate = value.redactedTemplate;
  const redactedSerialized = JSON.stringify(redactedTemplate);
  if (
    !hasExactKeys(redactedTemplate, ["mcpServers"]) ||
    !isRecord(redactedTemplate.mcpServers) ||
    externalMcpSecretValues(config).some((secret) =>
      redactedSerialized.includes(secret),
    )
  )
    throw new Error("External MCP validation result is unavailable");

  return {
    agentId: requestedAgentId,
    valid: value.valid,
    findings,
    redactedTemplate,
  };
}

function assertTraeModelRequest(
  request: TraeWorkModelRequest,
): TraeWorkModelRequest {
  if (
    !isRecord(request) ||
    !hasExactKeys(request, [
      "apiFormat",
      "urlMode",
      "url",
      "modelId",
      "apiKey",
      "allowNoApiKey",
      "allowLoopback",
      "allowPrivateNetwork",
    ]) ||
    !isOneOf(request.apiFormat, TRAE_MODEL_API_FORMATS) ||
    !isOneOf(request.urlMode, TRAE_MODEL_URL_MODES) ||
    typeof request.url !== "string" ||
    typeof request.modelId !== "string" ||
    typeof request.apiKey !== "string" ||
    typeof request.allowNoApiKey !== "boolean" ||
    typeof request.allowLoopback !== "boolean" ||
    typeof request.allowPrivateNetwork !== "boolean"
  )
    throw new Error("TRAE model request is invalid");
  return { ...request };
}

const UUID_V4_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function assertCanonicalRequestId(requestId: string): string {
  if (!UUID_V4_PATTERN.test(requestId))
    throw new Error("TRAE model request is invalid");
  return requestId;
}

function parseTraeModelValidationResult(
  value: unknown,
): TraeModelValidationResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "requestId",
      "state",
      "reasonCode",
      "durationBucket",
      "statusClass",
    ]) ||
    typeof value.requestId !== "string" ||
    !UUID_V4_PATTERN.test(value.requestId) ||
    value.state !== "valid" ||
    value.reasonCode !== "TRAE_MODEL_CONFIG_VALID" ||
    value.durationBucket !== "lt_1s" ||
    value.statusClass !== null
  )
    throw new Error("TRAE model validation result is unavailable");
  return {
    requestId: value.requestId,
    state: "valid",
    reasonCode: "TRAE_MODEL_CONFIG_VALID",
    durationBucket: "lt_1s",
    statusClass: null,
  };
}

const PROBE_REASON_BY_STATE = {
  reachable: ["TRAE_ENDPOINT_REACHABLE"],
  auth_rejected: ["TRAE_ENDPOINT_AUTH_REJECTED"],
  model_rejected: ["TRAE_ENDPOINT_MODEL_REJECTED"],
  network_rejected: [
    "TRAE_ENDPOINT_HTTP_REJECTED",
    "TRAE_ENDPOINT_NETWORK_REJECTED",
    "TRAE_DNS_RESOLUTION_FAILED",
    "TRAE_DNS_ADDRESS_REJECTED",
    "TRAE_DNS_ADDRESS_CLASS_MIXED",
    "TRAE_ENDPOINT_RESPONSE_TOO_LARGE",
    "PROXY_DNS_PIN_UNSUPPORTED",
  ],
  timeout: ["TRAE_ENDPOINT_TIMEOUT"],
  cancelled: ["TRAE_ENDPOINT_CANCELLED"],
} as const;

function parseTraeModelProbeResult(
  value: unknown,
  requestedId: string,
): TraeModelProbeResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "requestId",
      "state",
      "reasonCode",
      "durationBucket",
      "statusClass",
    ]) ||
    value.requestId !== requestedId ||
    !isOneOf(value.state, TRAE_MODEL_RESULT_STATES) ||
    value.state === "valid" ||
    !isOneOf(value.reasonCode, TRAE_MODEL_RESULT_REASON_CODES) ||
    value.reasonCode === "TRAE_MODEL_CONFIG_VALID" ||
    !isOneOf(value.durationBucket, TRAE_MODEL_DURATION_BUCKETS) ||
    (value.statusClass !== null &&
      !isOneOf(value.statusClass, TRAE_MODEL_STATUS_CLASSES))
  )
    throw new Error("TRAE endpoint result is unavailable");
  const allowedReasons = PROBE_REASON_BY_STATE[value.state];
  if (!(allowedReasons as readonly string[]).includes(value.reasonCode))
    throw new Error("TRAE endpoint result is unavailable");
  return {
    requestId: requestedId,
    state: value.state,
    reasonCode: value.reasonCode,
    durationBucket: value.durationBucket,
    statusClass: value.statusClass,
  };
}

function parseCancelTraeModelProbeResult(
  value: unknown,
  requestedId: string,
): CancelTraeModelProbeResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["requestId", "cancelled"]) ||
    value.requestId !== requestedId ||
    typeof value.cancelled !== "boolean"
  )
    throw new Error("TRAE endpoint cancellation result is unavailable");
  return { requestId: requestedId, cancelled: value.cancelled };
}

function parseTraeWorkModelIdsResult(value: unknown): TraeWorkModelIdsResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["modelIds", "revision", "truncated"]) ||
    !isStringArray(value.modelIds) ||
    (value.revision !== null && typeof value.revision !== "string") ||
    typeof value.truncated !== "boolean"
  )
    throw new Error("TRAE model list is unavailable");
  return {
    modelIds: value.modelIds,
    revision: value.revision,
    truncated: value.truncated,
  };
}

export function createQoderTraeFeaturePorts(): Pick<
  FeaturePorts,
  "qoderwork" | "externalMcp" | "traeWork"
> {
  return {
    qoderwork: {
      getHooks: async () =>
        parseQoderWorkHooksSnapshot(
          await invoke<unknown>("get_qoderwork_hooks"),
        ),
      saveHooks: async (request) =>
        parseSaveQoderWorkHooksResult(
          await invoke<unknown>("save_qoderwork_hooks", {
            request: assertQoderWorkHooksRequest(request),
          }),
        ),
    },
    externalMcp: {
      validate: async (agentId, config) => {
        const safeAgentId = assertExternalMcpAgentId(agentId);
        const safeConfig = assertExternalMcpConfig(config);
        return parseExternalMcpValidationResult(
          await invoke<unknown>("validate_external_mcp_config", {
            agentId: safeAgentId,
            config: safeConfig,
          }),
          safeAgentId,
          safeConfig,
        );
      },
    },
    traeWork: {
      validateModelConfig: async (request) =>
        parseTraeModelValidationResult(
          await invoke<unknown>("validate_traework_model_config", {
            request: assertTraeModelRequest(request),
          }),
        ),
      testModelEndpoint: async (requestId, request) => {
        const safeRequestId = assertCanonicalRequestId(requestId);
        return parseTraeModelProbeResult(
          await invoke<unknown>("test_traework_model_endpoint", {
            requestId: safeRequestId,
            request: assertTraeModelRequest(request),
          }),
          safeRequestId,
        );
      },
      cancelModelEndpoint: async (requestId) => {
        const safeRequestId = assertCanonicalRequestId(requestId);
        return parseCancelTraeModelProbeResult(
          await invoke<unknown>("cancel_traework_model_endpoint", {
            requestId: safeRequestId,
          }),
          safeRequestId,
        );
      },
      getModelIds: async () =>
        parseTraeWorkModelIdsResult(
          await invoke<unknown>("get_traework_model_ids"),
        ),
    },
  };
}
