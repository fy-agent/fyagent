import { invoke } from "@tauri-apps/api/core";

import type { FeaturePorts } from "../../../features/ports";
import {
  AGENT_CAPABILITY_IDS,
  AGENT_CAPABILITY_MODES,
  AGENT_CAPABILITY_REASON_CODES,
  AGENT_CATALOG_CONTRACT_VERSION,
  AGENT_CATALOG_IDS,
  AGENT_EVIDENCE_IDS,
  AGENT_OFFICIAL_LINK_IDS,
  EXTERNAL_AGENT_INSTALL_SOURCES,
  EXTERNAL_AGENT_LAUNCH_DESTINATIONS,
  EXTERNAL_AGENT_RUNTIME_STATES,
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
} from "../../../features/types";
import { hasExactKeys, isOneOf, isRecord } from "./validation";

const EXPECTED_AGENT_LINK_IDS = {
  qoderwork: ["product"],
  "trae-work": ["product"],
  workbuddy: ["product"],
  grokbuild: ["product"],
  codex: [],
  "claude-code": ["desktop"],
  opencode: ["product", "desktop"],
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

export function createAgentFeaturePorts(): Pick<
  FeaturePorts,
  "catalog" | "externalAgents"
> {
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
  };
}
