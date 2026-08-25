import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export const AGENT_INSTALL_READINESS_CONTRACT_VERSION = 2 as const;
export const AGENT_ACTION_CONTRACT_VERSION = 1 as const;

export const AGENT_INSTALL_STATES = [
  "not_installed",
  "installed",
  "installed_not_runnable",
  "unknown",
  "unavailable",
] as const;
export type AgentInstallState = (typeof AGENT_INSTALL_STATES)[number];

export const AGENT_UPDATE_STATES = [
  "unavailable",
  "unknown",
  "up_to_date",
  "update_available",
  "latest_unknown",
] as const;
export type AgentUpdateState = (typeof AGENT_UPDATE_STATES)[number];

export const AGENT_AUTH_OWNERSHIPS = [
  "fyagent_managed",
  "agent_owned",
  "provider_owned",
  "unavailable",
] as const;
export type AgentAuthOwnership = (typeof AGENT_AUTH_OWNERSHIPS)[number];

export const AGENT_AUTH_STATES = [
  "unknown",
  "logged_in",
  "logged_out",
  "provider_connection_required",
  "unavailable",
] as const;
export type AgentAuthState = (typeof AGENT_AUTH_STATES)[number];

export const AGENT_SOURCE_KINDS = [
  "cli_tooling",
  "managed_desktop",
  "codex_desktop",
] as const;
export type AgentSourceKind = (typeof AGENT_SOURCE_KINDS)[number];

export const AGENT_ACTION_IDS = [
  "install",
  "update",
  "launch",
  "auth_login",
  "auth_logout",
  "auth_connect_provider",
] as const;
export type AgentActionId = (typeof AGENT_ACTION_IDS)[number];

export const AGENT_REASON_CODES = [
  "official_page_only",
  "source_not_verified",
  "platform_unsupported",
  "interactive_user_unavailable",
  "installed_not_runnable",
  "auth_state_unknown",
  "provider_connection_required",
  "credential_store_unsupported",
  "binding_account_missing",
  "binding_identity_mismatch",
  "operation_conflict",
  "cancelled",
  "managed_by_codex_desktop",
  "native_projection_unavailable",
  "refresh_required",
  "executor_not_implemented",
] as const;
export type AgentReasonCode = (typeof AGENT_REASON_CODES)[number];

export const AGENT_ACTION_JOB_STAGES = [
  "checking",
  "downloading",
  "installing",
  "verifying_installation",
  "succeeded",
  "failed",
  "cancelled",
] as const;
export type AgentActionJobStage = (typeof AGENT_ACTION_JOB_STAGES)[number];

export interface AgentInstallReadiness {
  contractVersion: typeof AGENT_INSTALL_READINESS_CONTRACT_VERSION;
  agentId: AgentCatalogId;
  reviewedAt: string;
  installState: AgentInstallState;
  updateState: AgentUpdateState;
  releaseId: string | null;
  localVersion: string | null;
  remoteVersion: string | null;
  authOwnership: AgentAuthOwnership;
  authState: AgentAuthState;
  sourceKind: AgentSourceKind;
  allowedActions: AgentActionId[];
  reasonCodes: AgentReasonCode[];
}

export interface StartAgentActionRequest {
  agentId: AgentCatalogId;
  action: AgentActionId;
  expectedReleaseId?: string;
}

export interface AgentActionResult {
  contractVersion: typeof AGENT_ACTION_CONTRACT_VERSION;
  agentId: AgentCatalogId;
  action: AgentActionId;
  jobId: string | null;
  stage: AgentActionJobStage;
  reasonCode: AgentReasonCode | null;
}

export interface AgentActionJobSnapshot {
  contractVersion: typeof AGENT_ACTION_CONTRACT_VERSION;
  jobId: string;
  agentId: AgentCatalogId;
  action: AgentActionId;
  stage: AgentActionJobStage;
  cancellable: boolean;
  reasonCode: AgentReasonCode | null;
}

export interface AgentInstallReadinessPort {
  get(agentId: AgentCatalogId): Promise<AgentInstallReadiness>;
  startAction(request: StartAgentActionRequest): Promise<AgentActionResult>;
  cancelAction(jobId: string): Promise<AgentActionJobSnapshot>;
  getActionJob(jobId: string): Promise<AgentActionJobSnapshot>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  keys: readonly string[],
): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return (
    actual.length === expected.length &&
    actual.every((key, index) => key === expected[index])
  );
}

function isOneOf<T extends string>(
  value: unknown,
  candidates: readonly T[],
): value is T {
  return typeof value === "string" && candidates.includes(value as T);
}

function parseStringList<T extends string>(
  value: unknown,
  candidates: readonly T[],
): T[] {
  if (!Array.isArray(value)) {
    throw new Error("Agent install readiness is unavailable");
  }
  return value.map((item) => {
    if (!isOneOf(item, candidates)) {
      throw new Error("Agent install readiness is unavailable");
    }
    return item;
  });
}

const READINESS_KEYS = [
  "contractVersion",
  "agentId",
  "reviewedAt",
  "installState",
  "updateState",
  "releaseId",
  "localVersion",
  "remoteVersion",
  "authOwnership",
  "authState",
  "sourceKind",
  "allowedActions",
  "reasonCodes",
] as const;

const FORBIDDEN_WIRE = [
  "http://",
  "https://",
  "token",
  "secret",
  "apiKey",
  "api_key",
  "sha256",
  "script",
  "packageFormat",
  "managed_package",
] as const;

export function parseAgentInstallReadiness(
  value: unknown,
  expectedAgentId: AgentCatalogId,
): AgentInstallReadiness {
  const encoded = JSON.stringify(value).toLowerCase();
  for (const needle of FORBIDDEN_WIRE) {
    if (encoded.includes(needle.toLowerCase())) {
      throw new Error("Agent install readiness is unavailable");
    }
  }
  if (
    !isRecord(value) ||
    !hasExactKeys(value, READINESS_KEYS) ||
    value.contractVersion !== AGENT_INSTALL_READINESS_CONTRACT_VERSION ||
    value.agentId !== expectedAgentId ||
    typeof value.reviewedAt !== "string" ||
    !/^\d{4}-\d{2}-\d{2}$/u.test(value.reviewedAt) ||
    !isOneOf(value.installState, AGENT_INSTALL_STATES) ||
    !isOneOf(value.updateState, AGENT_UPDATE_STATES) ||
    (value.releaseId !== null && typeof value.releaseId !== "string") ||
    (value.localVersion !== null && typeof value.localVersion !== "string") ||
    (value.remoteVersion !== null && typeof value.remoteVersion !== "string") ||
    !isOneOf(value.authOwnership, AGENT_AUTH_OWNERSHIPS) ||
    !isOneOf(value.authState, AGENT_AUTH_STATES) ||
    !isOneOf(value.sourceKind, AGENT_SOURCE_KINDS)
  ) {
    throw new Error("Agent install readiness is unavailable");
  }
  if (
    typeof value.releaseId === "string" &&
    !/^v1:[0-9a-f]{64}$/u.test(value.releaseId)
  ) {
    throw new Error("Agent install readiness is unavailable");
  }
  const matchesKind =
    expectedAgentId === "codex"
      ? value.sourceKind === "codex_desktop" &&
        value.authOwnership === "fyagent_managed"
      : expectedAgentId === "opencode" ||
          expectedAgentId === "claude-code" ||
          expectedAgentId === "grokbuild"
        ? value.sourceKind === "cli_tooling"
        : value.sourceKind === "managed_desktop";
  if (!matchesKind) {
    throw new Error("Agent install readiness is unavailable");
  }
  return {
    contractVersion: AGENT_INSTALL_READINESS_CONTRACT_VERSION,
    agentId: expectedAgentId,
    reviewedAt: value.reviewedAt,
    installState: value.installState,
    updateState: value.updateState,
    releaseId: value.releaseId,
    localVersion: value.localVersion,
    remoteVersion: value.remoteVersion,
    authOwnership: value.authOwnership,
    authState: value.authState,
    sourceKind: value.sourceKind,
    allowedActions: parseStringList(value.allowedActions, AGENT_ACTION_IDS),
    reasonCodes: parseStringList(value.reasonCodes, AGENT_REASON_CODES),
  };
}

export function parseAgentActionResult(
  value: unknown,
  expectedAgentId: AgentCatalogId,
  expectedAction: AgentActionId,
): AgentActionResult {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "agentId",
      "action",
      "jobId",
      "stage",
      "reasonCode",
    ]) ||
    value.contractVersion !== AGENT_ACTION_CONTRACT_VERSION ||
    value.agentId !== expectedAgentId ||
    value.action !== expectedAction ||
    (value.jobId !== null && typeof value.jobId !== "string") ||
    !isOneOf(value.stage, AGENT_ACTION_JOB_STAGES) ||
    (value.reasonCode !== null &&
      !isOneOf(value.reasonCode, AGENT_REASON_CODES))
  ) {
    throw new Error("Agent action is unavailable");
  }
  return {
    contractVersion: AGENT_ACTION_CONTRACT_VERSION,
    agentId: expectedAgentId,
    action: expectedAction,
    jobId: value.jobId,
    stage: value.stage,
    reasonCode: value.reasonCode,
  };
}

export function parseAgentActionJobSnapshot(
  value: unknown,
): AgentActionJobSnapshot {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "jobId",
      "agentId",
      "action",
      "stage",
      "cancellable",
      "reasonCode",
    ]) ||
    value.contractVersion !== AGENT_ACTION_CONTRACT_VERSION ||
    typeof value.jobId !== "string" ||
    !isOneOf(value.agentId, AGENT_CATALOG_IDS) ||
    !isOneOf(value.action, AGENT_ACTION_IDS) ||
    !isOneOf(value.stage, AGENT_ACTION_JOB_STAGES) ||
    typeof value.cancellable !== "boolean" ||
    (value.reasonCode !== null &&
      !isOneOf(value.reasonCode, AGENT_REASON_CODES))
  ) {
    throw new Error("Agent action job is unavailable");
  }
  return {
    contractVersion: AGENT_ACTION_CONTRACT_VERSION,
    jobId: value.jobId,
    agentId: value.agentId,
    action: value.action,
    stage: value.stage,
    cancellable: value.cancellable,
    reasonCode: value.reasonCode,
  };
}

export function assertAgentInstallReadinessId(
  agentId: AgentCatalogId,
): AgentCatalogId {
  if (!isOneOf(agentId, AGENT_CATALOG_IDS)) {
    throw new Error("Agent install readiness request is invalid");
  }
  return agentId;
}
