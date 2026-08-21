import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  assertExpectedReleaseId,
  parseJobSnapshot,
  parseLocalInstallStatus,
  parseOptionalJobSnapshot,
  parseRemoteReleaseStatus,
} from "@/shared/codex-desktop";

import type { FeaturePorts } from "../../features/ports";
import {
  AGENT_CAPABILITY_IDS,
  AGENT_CAPABILITY_MODES,
  AGENT_CAPABILITY_REASON_CODES,
  AGENT_CATALOG_CONTRACT_VERSION,
  AGENT_CATALOG_IDS,
  AGENT_EVIDENCE_IDS,
  AGENT_OFFICIAL_LINK_IDS,
  EXTERNAL_MCP_FINDING_REASON_CODES,
  EXTERNAL_MCP_TRANSPORTS,
  EXTERNAL_AGENT_INSTALL_SOURCES,
  EXTERNAL_AGENT_LAUNCH_DESTINATIONS,
  EXTERNAL_AGENT_RUNTIME_STATES,
  QODERWORK_HOOK_EVENTS,
  TRAE_MODEL_API_FORMATS,
  TRAE_MODEL_DURATION_BUCKETS,
  TRAE_MODEL_RESULT_REASON_CODES,
  TRAE_MODEL_RESULT_STATES,
  TRAE_MODEL_STATUS_CLASSES,
  TRAE_MODEL_URL_MODES,
  type AgentCapabilityId,
  type AgentCatalogEntry,
  type AgentCatalogId,
  type AgentCatalogResult,
  type AgentEvidenceId,
  type AgentOfficialLink,
  type AgentOfficialLinkId,
  type DeclaredAgentCapability,
  type ExternalAgentLaunchDestination,
  type ExternalAgentLaunchResult,
  type ExternalAgentRuntimeCapability,
  type ExternalAgentRuntimeStatus,
  type ExternalMcpAgentId,
  type ExternalMcpFinding,
  type ExternalMcpValidationResult,
  type ProviderQuickSetupRequest,
  type ProviderSummaryQueryData,
  type QoderWorkCommandHook,
  type QoderWorkHookGroup,
  type QoderWorkHooksSnapshot,
  type SaveQoderWorkHooksRequest,
  type SaveQoderWorkHooksResult,
  type TraeModelProbeResult,
  type TraeModelValidationResult,
  type TraeWorkModelRequest,
  type CancelTraeModelProbeResult,
  type TraeWorkModelIdsResult,
  type FetchedModelList,
  type FetchedModelRef,
  type OpenCodeFetchModelsRequest,
  type OpenCodeModelSnapshot,
  type OpenCodeSaveModelsRequest,
  type WorkBuddySaveModelsResult,
  type ModelProbeRequest,
  type ModelProbeResult,
  type ReachabilityResult,
  type DailyMemoryFileInfo,
  type DailyMemorySearchResult,
  HERMES_MEMORY_KINDS,
  type HermesMemoryKind,
  type HermesMemoryLimits,
  type ManagedPrompt,
  MEMORY_DOCUMENT_IDS,
  type MemoryDocumentId,
  type OpenClawDirectory,
  PROMPT_APP_IDS,
  type PromptAppId,
} from "../../features/types";

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function hasExactKeys(value: Record<string, unknown>, keys: string[]): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
  );
}

const CODEX_DESKTOP_JOB_UPDATED_EVENT = "codex-desktop-installer://job-updated";

const EXPECTED_AGENT_LINK_IDS = {
  qoderwork: ["product"],
  "trae-work": ["product"],
  workbuddy: ["product"],
  grokbuild: ["product"],
  codex: [],
  "claude-code": ["cli", "desktop"],
  opencode: ["product", "cli"],
} as const satisfies Readonly<
  Record<AgentCatalogId, readonly AgentOfficialLinkId[]>
>;

const EXPECTED_AGENT_VARIANT_IDS = {
  qoderwork: "qoderwork-cn",
  "trae-work": "trae-work-cn",
  workbuddy: "workbuddy",
  grokbuild: "grokbuild",
  codex: "codex",
  "claude-code": "claude-code",
  opencode: "opencode",
} as const;

function isOneOf<T extends string>(
  value: unknown,
  candidates: readonly T[],
): value is T {
  return typeof value === "string" && candidates.some((item) => item === value);
}

function isReviewedDate(value: unknown): value is string {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value))
    return false;
  const parsed = new Date(`${value}T00:00:00.000Z`);
  return (
    !Number.isNaN(parsed.getTime()) &&
    parsed.toISOString().slice(0, 10) === value
  );
}

function isOfficialHttpsUrl(value: unknown): value is string {
  if (typeof value !== "string" || value.trim() !== value) return false;
  try {
    const parsed = new URL(value);
    return (
      parsed.protocol === "https:" &&
      parsed.hostname.length > 0 &&
      parsed.username === "" &&
      parsed.password === "" &&
      parsed.search === "" &&
      parsed.hash === ""
    );
  } catch {
    return false;
  }
}

function parseDeclaredAgentCapability(
  value: unknown,
  expectedId: AgentCapabilityId,
): DeclaredAgentCapability {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["id", "mode", "reasonCode", "evidenceIds"]) ||
    value.id !== expectedId ||
    !isOneOf(value.mode, AGENT_CAPABILITY_MODES) ||
    !isOneOf(value.reasonCode, AGENT_CAPABILITY_REASON_CODES) ||
    !Array.isArray(value.evidenceIds) ||
    value.evidenceIds.length === 0
  )
    throw new Error("Agent catalog is unavailable");

  const evidenceIds = value.evidenceIds.map((evidenceId) => {
    if (!isOneOf(evidenceId, AGENT_EVIDENCE_IDS))
      throw new Error("Agent catalog is unavailable");
    return evidenceId;
  });
  if (new Set<AgentEvidenceId>(evidenceIds).size !== evidenceIds.length)
    throw new Error("Agent catalog is unavailable");

  return {
    id: expectedId,
    mode: value.mode,
    reasonCode: value.reasonCode,
    evidenceIds,
  };
}

function parseAgentOfficialLink(value: unknown): AgentOfficialLink {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["id", "label", "url"]) ||
    !isOneOf(value.id, AGENT_OFFICIAL_LINK_IDS) ||
    typeof value.label !== "string" ||
    value.label.trim().length === 0 ||
    value.label.trim() !== value.label ||
    !isOfficialHttpsUrl(value.url)
  )
    throw new Error("Agent catalog is unavailable");
  return { id: value.id, label: value.label, url: value.url };
}

function parseAgentCatalogEntry(
  value: unknown,
  expectedId: AgentCatalogId,
): AgentCatalogEntry {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "id",
      "variantId",
      "displayName",
      "description",
      "officialLinks",
      "capabilities",
    ]) ||
    value.id !== expectedId ||
    value.variantId !== EXPECTED_AGENT_VARIANT_IDS[expectedId] ||
    typeof value.displayName !== "string" ||
    value.displayName.trim().length === 0 ||
    value.displayName.trim() !== value.displayName ||
    typeof value.description !== "string" ||
    value.description.trim().length === 0 ||
    value.description.trim() !== value.description ||
    !Array.isArray(value.officialLinks) ||
    !Array.isArray(value.capabilities) ||
    value.capabilities.length !== AGENT_CAPABILITY_IDS.length
  )
    throw new Error("Agent catalog is unavailable");

  const officialLinks = value.officialLinks.map(parseAgentOfficialLink);
  const linkIds = new Set<AgentOfficialLinkId>();
  const linkLabels = new Set<string>();
  for (const link of officialLinks) {
    if (linkIds.has(link.id) || linkLabels.has(link.label))
      throw new Error("Agent catalog is unavailable");
    linkIds.add(link.id);
    linkLabels.add(link.label);
  }

  const expectedLinkIds = EXPECTED_AGENT_LINK_IDS[expectedId];
  if (
    officialLinks.length !== expectedLinkIds.length ||
    officialLinks.some((link, index) => link.id !== expectedLinkIds[index])
  )
    throw new Error("Agent catalog is unavailable");

  const capabilityValues = value.capabilities as unknown[];
  const capabilities = AGENT_CAPABILITY_IDS.map((capabilityId, index) =>
    parseDeclaredAgentCapability(capabilityValues[index], capabilityId),
  );

  return {
    id: expectedId,
    variantId: EXPECTED_AGENT_VARIANT_IDS[expectedId],
    displayName: value.displayName,
    description: value.description,
    officialLinks,
    capabilities,
  };
}

function parseAgentCatalog(value: unknown): AgentCatalogResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["contractVersion", "reviewedAt", "agents"]) ||
    value.contractVersion !== AGENT_CATALOG_CONTRACT_VERSION ||
    !isReviewedDate(value.reviewedAt) ||
    !Array.isArray(value.agents) ||
    value.agents.length !== AGENT_CATALOG_IDS.length
  )
    throw new Error("Agent catalog is unavailable");

  const candidates = value.agents;
  return {
    contractVersion: AGENT_CATALOG_CONTRACT_VERSION,
    reviewedAt: value.reviewedAt,
    agents: AGENT_CATALOG_IDS.map((expectedId, index) =>
      parseAgentCatalogEntry(candidates[index], expectedId),
    ),
  };
}

function parseNullableBoolean(value: unknown): boolean | null {
  if (value === null || typeof value === "boolean") return value;
  throw new Error("External agent status is unavailable");
}

function parseRuntimeCapability(
  value: unknown,
): ExternalAgentRuntimeCapability {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["id", "state", "reasonCode"]) ||
    !isOneOf(value.id, AGENT_CAPABILITY_IDS) ||
    !isOneOf(value.state, EXTERNAL_AGENT_RUNTIME_STATES) ||
    !isOneOf(value.reasonCode, AGENT_CAPABILITY_REASON_CODES)
  )
    throw new Error("External agent status is unavailable");
  return { id: value.id, state: value.state, reasonCode: value.reasonCode };
}

function parseExternalAgentRuntimeStatus(
  value: unknown,
  requestedAgentId: AgentCatalogId,
): ExternalAgentRuntimeStatus {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "agentId",
      "detected",
      "running",
      "version",
      "installSource",
      "capabilities",
    ]) ||
    value.agentId !== requestedAgentId ||
    (value.version !== null &&
      (typeof value.version !== "string" ||
        value.version.trim().length === 0 ||
        value.version.trim() !== value.version)) ||
    (value.installSource !== null &&
      !isOneOf(value.installSource, EXTERNAL_AGENT_INSTALL_SOURCES)) ||
    !Array.isArray(value.capabilities)
  )
    throw new Error("External agent status is unavailable");

  const capabilities = value.capabilities.map(parseRuntimeCapability);
  if (
    capabilities.length !== 2 ||
    capabilities[0]?.id !== "app.detect" ||
    capabilities[1]?.id !== "app.launch" ||
    new Set(capabilities.map((capability) => capability.id)).size !==
      capabilities.length
  )
    throw new Error("External agent status is unavailable");

  return {
    agentId: requestedAgentId,
    detected: parseNullableBoolean(value.detected),
    running: parseNullableBoolean(value.running),
    version: value.version,
    installSource: value.installSource,
    capabilities,
  };
}

function parseExternalAgentLaunchResult(
  value: unknown,
  requestedAgentId: AgentCatalogId,
  requestedDestination: ExternalAgentLaunchDestination,
): ExternalAgentLaunchResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["agentId", "destination", "state", "reasonCode"]) ||
    value.agentId !== requestedAgentId ||
    value.destination !== requestedDestination ||
    !isOneOf(value.state, EXTERNAL_AGENT_RUNTIME_STATES) ||
    !isOneOf(value.reasonCode, AGENT_CAPABILITY_REASON_CODES)
  )
    throw new Error("External agent launch result is unavailable");
  return {
    agentId: requestedAgentId,
    destination: requestedDestination,
    state: value.state,
    reasonCode: value.reasonCode,
  };
}

function hasRequiredAndOptionalKeys(
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[],
): boolean {
  const allowed = new Set([...required, ...optional]);
  return (
    required.every((key) => Object.prototype.hasOwnProperty.call(value, key)) &&
    Object.keys(value).every((key) => allowed.has(key))
  );
}

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

function assertStringArray(value: unknown): value is string[] {
  return (
    Array.isArray(value) && value.every((item) => typeof item === "string")
  );
}

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
        "providerName",
        "baseUrl",
        "apiKey",
        "selectedModelIds",
        "expectedRevision",
      ],
      ["removedModelIds", "overwriteToken"],
    ) ||
    typeof request.providerName !== "string" ||
    typeof request.baseUrl !== "string" ||
    typeof request.apiKey !== "string" ||
    !assertStringArray(request.selectedModelIds) ||
    (request.removedModelIds !== undefined &&
      !assertStringArray(request.removedModelIds)) ||
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

function assertModelProbeRequest(request: ModelProbeRequest): ModelProbeRequest {
  if (
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
    request.modelId.trim().length === 0
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
    }),
  );
}

function parseTraeWorkModelIdsResult(value: unknown): TraeWorkModelIdsResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["modelIds", "revision", "truncated"]) ||
    !assertStringArray(value.modelIds) ||
    (value.revision !== null && typeof value.revision !== "string") ||
    typeof value.truncated !== "boolean"
  ) {
    throw new Error("TRAE model list is unavailable");
  }
  return {
    modelIds: value.modelIds,
    revision: value.revision,
    truncated: value.truncated,
  };
}

function parseOpenCodeModelSnapshot(value: unknown): OpenCodeModelSnapshot {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["providers", "revision"]) ||
    !Array.isArray(value.providers) ||
    (value.revision !== null && typeof value.revision !== "string")
  )
    throw new Error("OpenCode model snapshot is unavailable");
  const providers = value.providers.map((provider) => {
    if (
      !isRecord(provider) ||
      !hasExactKeys(provider, ["id", "name", "modelIds"]) ||
      typeof provider.id !== "string" ||
      typeof provider.name !== "string" ||
      !assertStringArray(provider.modelIds)
    )
      throw new Error("OpenCode model snapshot is unavailable");
    return {
      id: provider.id,
      name: provider.name,
      modelIds: provider.modelIds,
    };
  });
  return { providers, revision: value.revision };
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
      !assertStringArray(value.existingIds)
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

function parseProviderSummary(value: unknown): ProviderSummaryQueryData {
  if (!isRecord(value) || !hasExactKeys(value, ["providers", "currentId"]))
    throw new Error("Provider public summary is unavailable");
  if (!isRecord(value.providers) || typeof value.currentId !== "string")
    throw new Error("Provider public summary is unavailable");

  const providers: ProviderSummaryQueryData["providers"] = {};
  for (const [key, candidate] of Object.entries(value.providers)) {
    if (
      !isRecord(candidate) ||
      !hasRequiredAndOptionalKeys(candidate, ["id", "name"], ["modelId"]) ||
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
      ...(typeof candidate.modelId === "string" && candidate.modelId
        ? { modelId: candidate.modelId }
        : {}),
    };
  }
  if (value.currentId !== "" && !(value.currentId in providers))
    throw new Error("Provider public summary is unavailable");
  return { providers, currentId: value.currentId };
}

function assertQuickSetupRequest(
  request: ProviderQuickSetupRequest,
): ProviderQuickSetupRequest {
  if (
    !isRecord(request) ||
    !hasRequiredAndOptionalKeys(
      request,
      ["name", "baseUrl", "apiKey", "modelId"],
      ["codexFeatures"],
    ) ||
    !["name", "baseUrl", "apiKey", "modelId"].every(
      (key) => typeof request[key] === "string",
    ) ||
    (request.codexFeatures !== undefined &&
      !isValidCodexFeatures(request.codexFeatures))
  )
    throw new Error("Provider quick setup request is invalid");
  return request;
}

function isValidCodexFeatures(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return (
    Object.keys(value).every((key) =>
      ["imageExtension", "websockets"].includes(key),
    ) && Object.values(value).every((item) => typeof item === "boolean")
  );
}

function validateExternalUrl(url: string): void {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error("外部链接无效");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("只允许打开 HTTP(S) 链接");
  }
}

function assertJobId(jobId: string): string {
  if (jobId.trim().length === 0 || jobId.trim() !== jobId)
    throw new Error("Codex desktop installer request is invalid");
  return jobId;
}

function assertAgentId(agentId: AgentCatalogId): AgentCatalogId {
  if (!isOneOf(agentId, AGENT_CATALOG_IDS))
    throw new Error("External agent request is invalid");
  return agentId;
}

function assertLaunchDestination(
  destination: ExternalAgentLaunchDestination,
): ExternalAgentLaunchDestination {
  if (!isOneOf(destination, EXTERNAL_AGENT_LAUNCH_DESTINATIONS))
    throw new Error("External agent request is invalid");
  return destination;
}

const MEMORY_DOCUMENT_TARGETS = {
  "openclaw-memory": { source: "openclaw", filename: "MEMORY.md" },
  "openclaw-user": { source: "openclaw", filename: "USER.md" },
  "hermes-memory": { source: "hermes", kind: "memory" },
  "hermes-user": { source: "hermes", kind: "user" },
} as const satisfies Record<
  MemoryDocumentId,
  | { source: "openclaw"; filename: "MEMORY.md" | "USER.md" }
  | { source: "hermes"; kind: HermesMemoryKind }
>;

function hasOnlyKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
): boolean {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key));
}

function assertPromptAppId(app: PromptAppId): PromptAppId {
  if (!isOneOf(app, PROMPT_APP_IDS))
    throw new Error("Prompt application is invalid");
  return app;
}

function assertPromptId(value: unknown): string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.trim() !== value ||
    [...value].some((character) => {
      const code = character.charCodeAt(0);
      return code <= 31 || code === 127;
    })
  )
    throw new Error("Prompt identifier is invalid");
  return value;
}

function isOptionalTimestamp(value: unknown): value is number | undefined {
  return (
    value === undefined ||
    (typeof value === "number" && Number.isSafeInteger(value) && value >= 0)
  );
}

function parseManagedPrompt(
  value: unknown,
  expectedId?: string,
): ManagedPrompt {
  if (
    !isRecord(value) ||
    !hasOnlyKeys(value, [
      "id",
      "name",
      "content",
      "description",
      "enabled",
      "createdAt",
      "updatedAt",
    ]) ||
    typeof value.name !== "string" ||
    typeof value.content !== "string" ||
    (value.description !== undefined &&
      typeof value.description !== "string") ||
    typeof value.enabled !== "boolean" ||
    !isOptionalTimestamp(value.createdAt) ||
    !isOptionalTimestamp(value.updatedAt)
  )
    throw new Error("Prompt data is unavailable");

  const id = assertPromptId(value.id);
  if (expectedId !== undefined && id !== expectedId)
    throw new Error("Prompt data is unavailable");

  return {
    id,
    name: value.name,
    content: value.content,
    ...(value.description === undefined
      ? {}
      : { description: value.description }),
    enabled: value.enabled,
    ...(value.createdAt === undefined ? {} : { createdAt: value.createdAt }),
    ...(value.updatedAt === undefined ? {} : { updatedAt: value.updatedAt }),
  };
}

function assertManagedPrompt(prompt: ManagedPrompt): ManagedPrompt {
  const parsed = parseManagedPrompt(prompt);
  if (parsed.name.trim().length === 0)
    throw new Error("Prompt name is required");
  return parsed;
}

function parsePromptCollection(value: unknown): ManagedPrompt[] {
  if (!isRecord(value)) throw new Error("Prompt data is unavailable");
  return Object.entries(value).map(([id, prompt]) =>
    parseManagedPrompt(prompt, assertPromptId(id)),
  );
}

function parseNullableContent(value: unknown, message: string): string | null {
  if (value !== null && typeof value !== "string") throw new Error(message);
  return value;
}

function parseImportedPromptId(value: unknown): string {
  try {
    return assertPromptId(value);
  } catch {
    throw new Error("Imported Prompt data is unavailable");
  }
}

function assertMemoryDocumentId(id: MemoryDocumentId): MemoryDocumentId {
  if (!isOneOf(id, MEMORY_DOCUMENT_IDS))
    throw new Error("Memory document is invalid");
  return id;
}

function assertHermesMemoryKind(kind: HermesMemoryKind): HermesMemoryKind {
  if (!isOneOf(kind, HERMES_MEMORY_KINDS))
    throw new Error("Hermes memory kind is invalid");
  return kind;
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function parseHermesMemoryLimits(value: unknown): HermesMemoryLimits {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["memory", "user", "memoryEnabled", "userEnabled"]) ||
    !isNonNegativeInteger(value.memory) ||
    !isNonNegativeInteger(value.user) ||
    typeof value.memoryEnabled !== "boolean" ||
    typeof value.userEnabled !== "boolean"
  )
    throw new Error("Hermes memory limits are unavailable");

  return {
    memory: value.memory,
    user: value.user,
    memoryEnabled: value.memoryEnabled,
    userEnabled: value.userEnabled,
  };
}

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

function assertDailyMemoryFilename(value: unknown): string {
  if (typeof value !== "string")
    throw new Error("Daily memory filename is invalid");
  const match = /^(\d{4})-(\d{2})-(\d{2})\.md$/.exec(value);
  if (!match) throw new Error("Daily memory filename is invalid");

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const daysInMonth = [
    31,
    isLeapYear(year) ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ];
  if (
    year < 1 ||
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > daysInMonth[month - 1]
  )
    throw new Error("Daily memory filename is invalid");
  return value;
}

function parseDailyMemoryFileInfo(value: unknown): DailyMemoryFileInfo {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "filename",
      "date",
      "sizeBytes",
      "modifiedAt",
      "preview",
    ]) ||
    typeof value.date !== "string" ||
    !isNonNegativeInteger(value.sizeBytes) ||
    !isNonNegativeInteger(value.modifiedAt) ||
    typeof value.preview !== "string"
  )
    throw new Error("Daily memory list is unavailable");

  const filename = assertDailyMemoryFilename(value.filename);
  if (value.date !== filename.slice(0, -3))
    throw new Error("Daily memory list is unavailable");
  return {
    filename,
    date: value.date,
    sizeBytes: value.sizeBytes,
    modifiedAt: value.modifiedAt,
    preview: value.preview,
  };
}

function parseDailyMemoryFiles(value: unknown): DailyMemoryFileInfo[] {
  if (!Array.isArray(value))
    throw new Error("Daily memory list is unavailable");
  return value.map(parseDailyMemoryFileInfo);
}

function parseDailyMemorySearchResult(value: unknown): DailyMemorySearchResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "filename",
      "date",
      "sizeBytes",
      "modifiedAt",
      "snippet",
      "matchCount",
    ]) ||
    typeof value.date !== "string" ||
    !isNonNegativeInteger(value.sizeBytes) ||
    !isNonNegativeInteger(value.modifiedAt) ||
    typeof value.snippet !== "string" ||
    !isNonNegativeInteger(value.matchCount)
  )
    throw new Error("Daily memory search is unavailable");

  const filename = assertDailyMemoryFilename(value.filename);
  if (value.date !== filename.slice(0, -3))
    throw new Error("Daily memory search is unavailable");
  return {
    filename,
    date: value.date,
    sizeBytes: value.sizeBytes,
    modifiedAt: value.modifiedAt,
    snippet: value.snippet,
    matchCount: value.matchCount,
  };
}

function parseDailyMemorySearchResults(
  value: unknown,
): DailyMemorySearchResult[] {
  if (!Array.isArray(value))
    throw new Error("Daily memory search is unavailable");
  return value.map(parseDailyMemorySearchResult);
}

function assertSearchQuery(query: string): string {
  if (typeof query !== "string")
    throw new Error("Daily memory search query is invalid");
  return query;
}

function assertOpenClawDirectory(subdir: OpenClawDirectory): OpenClawDirectory {
  if (subdir !== "workspace" && subdir !== "memory")
    throw new Error("OpenClaw directory is invalid");
  return subdir;
}

export function createTauriFeaturePorts(): FeaturePorts {
  return {
    catalog: {
      get: async () =>
        parseAgentCatalog(await invoke<unknown>("get_agent_catalog")),
    },
    externalAgents: {
      getStatus: async (agentId) => {
        const safeAgentId = assertAgentId(agentId);
        return parseExternalAgentRuntimeStatus(
          await invoke<unknown>("get_external_agent_status", {
            agentId: safeAgentId,
          }),
          safeAgentId,
        );
      },
      launch: async (agentId, destination) => {
        const safeAgentId = assertAgentId(agentId);
        const safeDestination = assertLaunchDestination(destination);
        return parseExternalAgentLaunchResult(
          await invoke<unknown>("launch_external_agent", {
            agentId: safeAgentId,
            destination: safeDestination,
          }),
          safeAgentId,
          safeDestination,
        );
      },
    },
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
    codexDesktop: {
      getLocalStatus: async () =>
        parseLocalInstallStatus(
          await invoke<unknown>("codex_desktop_get_local_status"),
        ),
      checkLatest: async (force) => {
        if (typeof force !== "boolean")
          throw new Error("Codex desktop installer request is invalid");
        return parseRemoteReleaseStatus(
          await invoke<unknown>("codex_desktop_check_latest", { force }),
        );
      },
      getJob: async () =>
        parseOptionalJobSnapshot(
          await invoke<unknown>("codex_desktop_get_job"),
        ),
      startInstall: async (expectedReleaseId) =>
        parseJobSnapshot(
          await invoke<unknown>("codex_desktop_start_install", {
            request: {
              expectedReleaseId: assertExpectedReleaseId(expectedReleaseId),
            },
          }),
        ),
      cancelInstall: async (jobId) =>
        parseJobSnapshot(
          await invoke<unknown>("codex_desktop_cancel_install", {
            jobId: assertJobId(jobId),
          }),
        ),
      launch: async () => {
        await invoke("codex_desktop_launch");
      },
      openLogDirectory: async () => {
        await invoke("codex_desktop_open_log_directory");
      },
      subscribeJobUpdates: async (onSnapshot) =>
        listen<unknown>(CODEX_DESKTOP_JOB_UPDATED_EVENT, (event) => {
          onSnapshot(parseJobSnapshot(event.payload));
        }),
    },
    providers: {
      getSummary: async (app) =>
        parseProviderSummary(await invoke("get_provider_summary", { app })),
      applyQuickSetupWithResult: (request, app) =>
        invoke("apply_provider_quick_setup_with_result", {
          request: assertQuickSetupRequest(request),
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
    skills: {
      getInstalled: () => invoke("get_installed_skills"),
      getBackups: () => invoke("get_skill_backups"),
      deleteBackup: (backupId) => invoke("delete_skill_backup", { backupId }),
      install: (skill, currentApp) =>
        invoke("install_skill_unified", { skill, currentApp }),
      uninstall: (id) => invoke("uninstall_skill_unified", { id }),
      restoreBackup: (backupId, currentApp) =>
        invoke("restore_skill_backup", { backupId, currentApp }),
      toggleApp: (id, app, enabled) =>
        invoke("toggle_skill_app", { id, app, enabled }),
      scanUnmanaged: () => invoke("scan_unmanaged_skills"),
      importFromApps: (imports) =>
        invoke("import_skills_from_apps", { imports }),
      discoverPage: (request) =>
        invoke("discover_available_skills_page", {
          query: request.query,
          repo: request.repo ?? null,
          status: request.status,
          limit: request.limit,
          offset: request.offset,
        }),
      checkUpdates: () => invoke("check_skill_updates"),
      update: (id) => invoke("update_skill", { id }),
      migrateStorage: (target) => invoke("migrate_skill_storage", { target }),
      searchSkillHub: (query, limit, offset, category = "") =>
        invoke("search_skillhub", { query, limit, offset, category }),
      installSkillHub: (slug, currentApp) =>
        invoke("install_skillhub", { slug, currentApp }),
      getRepos: () => invoke("get_skill_repos"),
      addRepo: (repo) => invoke("add_skill_repo", { repo }),
      removeRepo: (owner, name) => invoke("remove_skill_repo", { owner, name }),
      pickZip: () => invoke("open_zip_file_dialog"),
      installFromZip: (filePath, currentApp) =>
        invoke("install_skills_from_zip", { filePath, currentApp }),
    },
    mcp: {
      getAll: () => invoke("get_mcp_servers"),
      upsert: (server) => invoke("upsert_mcp_server", { server }),
      delete: (id) => invoke("delete_mcp_server", { id }),
      toggleApp: (serverId, app, enabled) =>
        invoke("toggle_mcp_app", { serverId, app, enabled }),
      importFromApps: () => invoke("import_mcp_from_apps"),
    },
    prompts: {
      getAll: async (app) => {
        const safeApp = assertPromptAppId(app);
        return parsePromptCollection(
          await invoke<unknown>("get_prompts", { app: safeApp }),
        );
      },
      getCurrentFileContent: async (app) => {
        const safeApp = assertPromptAppId(app);
        return parseNullableContent(
          await invoke<unknown>("get_current_prompt_file_content", {
            app: safeApp,
          }),
          "Prompt live file is unavailable",
        );
      },
      upsert: async (app, prompt) => {
        const safeApp = assertPromptAppId(app);
        const safePrompt = assertManagedPrompt(prompt);
        await invoke("upsert_prompt", {
          app: safeApp,
          id: safePrompt.id,
          prompt: safePrompt,
        });
      },
      delete: async (app, id) => {
        await invoke("delete_prompt", {
          app: assertPromptAppId(app),
          id: assertPromptId(id),
        });
      },
      enable: async (app, id) => {
        await invoke("enable_prompt", {
          app: assertPromptAppId(app),
          id: assertPromptId(id),
        });
      },
      importFromFile: async (app) =>
        parseImportedPromptId(
          await invoke<unknown>("import_prompt_from_file", {
            app: assertPromptAppId(app),
          }),
        ),
    },
    memory: {
      readDocument: async (id) => {
        const target = MEMORY_DOCUMENT_TARGETS[assertMemoryDocumentId(id)];
        if (target.source === "openclaw") {
          return parseNullableContent(
            await invoke<unknown>("read_workspace_file", {
              filename: target.filename,
            }),
            "OpenClaw memory document is unavailable",
          );
        }
        const content = await invoke<unknown>("get_hermes_memory", {
          kind: target.kind,
        });
        if (typeof content !== "string")
          throw new Error("Hermes memory document is unavailable");
        return content;
      },
      writeDocument: async (id, content) => {
        if (typeof content !== "string")
          throw new Error("Memory document content is invalid");
        const target = MEMORY_DOCUMENT_TARGETS[assertMemoryDocumentId(id)];
        if (target.source === "openclaw") {
          await invoke("write_workspace_file", {
            filename: target.filename,
            content,
          });
          return;
        }
        await invoke("set_hermes_memory", { kind: target.kind, content });
      },
      getHermesLimits: async () =>
        parseHermesMemoryLimits(
          await invoke<unknown>("get_hermes_memory_limits"),
        ),
      setHermesEnabled: async (kind, enabled) => {
        if (typeof enabled !== "boolean")
          throw new Error("Hermes memory enabled state is invalid");
        await invoke("set_hermes_memory_enabled", {
          kind: assertHermesMemoryKind(kind),
          enabled,
        });
      },
      listDailyFiles: async () =>
        parseDailyMemoryFiles(await invoke<unknown>("list_daily_memory_files")),
      readDailyFile: async (filename) =>
        parseNullableContent(
          await invoke<unknown>("read_daily_memory_file", {
            filename: assertDailyMemoryFilename(filename),
          }),
          "Daily memory file is unavailable",
        ),
      writeDailyFile: async (filename, content) => {
        if (typeof content !== "string")
          throw new Error("Daily memory content is invalid");
        await invoke("write_daily_memory_file", {
          filename: assertDailyMemoryFilename(filename),
          content,
        });
      },
      deleteDailyFile: async (filename) => {
        await invoke("delete_daily_memory_file", {
          filename: assertDailyMemoryFilename(filename),
        });
      },
      searchDailyFiles: async (query) =>
        parseDailyMemorySearchResults(
          await invoke<unknown>("search_daily_memory_files", {
            query: assertSearchQuery(query),
          }),
        ),
      openOpenClawDirectory: async (subdir) => {
        const opened = await invoke<unknown>("open_workspace_directory", {
          subdir: assertOpenClawDirectory(subdir),
        });
        if (opened !== true)
          throw new Error("OpenClaw directory could not be opened");
      },
    },
    settings: {
      get: () => invoke("get_settings"),
      save: (settings) => invoke("save_settings", { settings }),
      openExternal: async (url) => {
        validateExternalUrl(url);
        await invoke("open_external", { url });
      },
    },
  };
}
