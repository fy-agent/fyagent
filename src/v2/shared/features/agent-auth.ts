import { AGENT_CATALOG_IDS, type AgentCatalogId } from "./directory";

export const AGENT_AUTH_CONTRACT_VERSION = 1 as const;

export const AGENT_AUTH_INTENTS = [
  "login",
  "logout",
  "connect_provider",
] as const;
export type AgentAuthIntent = (typeof AGENT_AUTH_INTENTS)[number];

export const AGENT_AUTH_AUTHORITIES = [
  "verified",
  "unverified",
  "unavailable",
] as const;
export type AgentAuthAuthority = (typeof AGENT_AUTH_AUTHORITIES)[number];

export const AGENT_AUTH_OWNERSHIPS = [
  "fyagent_managed",
  "agent_owned",
  "provider_owned",
  "unavailable",
] as const;
export type AgentAuthOwnership = (typeof AGENT_AUTH_OWNERSHIPS)[number];

export const AGENT_AUTH_ACCOUNT_STATES = [
  "logged_in",
  "logged_out",
  "unknown",
] as const;
export type AgentAuthAccountState = (typeof AGENT_AUTH_ACCOUNT_STATES)[number];

export const AGENT_AUTH_PROVIDER_CONNECTION_STATES = [
  "configured",
  "empty",
  "unknown",
] as const;
export type AgentAuthProviderConnectionState =
  (typeof AGENT_AUTH_PROVIDER_CONNECTION_STATES)[number];

export const AGENT_AUTH_MANAGED_DESTINATIONS = ["auth_center"] as const;
export type AgentAuthManagedDestination =
  (typeof AGENT_AUTH_MANAGED_DESTINATIONS)[number];

export const AGENT_AUTH_REASON_CODES = [
  "auth_state_unknown",
  "auth_observer_unavailable",
  "auth_output_invalid",
  "interactive_user_unavailable",
  "operation_conflict",
  "provider_selection_required",
  "provider_changed",
  "monitoring_stopped",
  "timed_out",
  "handoff_only",
  "managed_by_auth_center",
  "target_selection_required",
  "target_changed",
  "target_not_executable",
  "inventory_expired",
  "command_failed",
  "cancelled",
  "executor_not_implemented",
] as const;
export type AgentAuthReasonCode = (typeof AGENT_AUTH_REASON_CODES)[number];

export const AGENT_AUTH_SESSION_STAGES = [
  "preparing",
  "launching",
  "awaiting_user",
  "verifying",
  "verified",
  "handoff_complete",
  "failed",
  "cancelled",
  "timed_out",
] as const;
export type AgentAuthSessionStage = (typeof AGENT_AUTH_SESSION_STAGES)[number];

export const AGENT_AUTH_SESSION_OUTCOMES = [
  "verified_logged_in",
  "verified_logged_out",
  "verified_provider_change",
  "handoff_only",
  "failed",
  "cancelled",
  "timed_out",
] as const;
export type AgentAuthSessionOutcome =
  (typeof AGENT_AUTH_SESSION_OUTCOMES)[number];

export interface AgentAuthProviderSummary {
  providerId: string;
  label: string;
}

interface AgentAuthObservationBase {
  contractVersion: typeof AGENT_AUTH_CONTRACT_VERSION;
  agentId: AgentCatalogId;
  ownership: AgentAuthOwnership;
  authority: AgentAuthAuthority;
  allowedIntents: AgentAuthIntent[];
  checkedAt: string;
  reasonCodes: AgentAuthReasonCode[];
}

export type AgentAuthObservation =
  | (AgentAuthObservationBase & {
      kind: "account";
      ownership: "agent_owned";
      state: AgentAuthAccountState;
    })
  | (AgentAuthObservationBase & {
      kind: "provider_connections";
      ownership: "provider_owned";
      state: AgentAuthProviderConnectionState;
      providers: AgentAuthProviderSummary[];
    })
  | (AgentAuthObservationBase & {
      kind: "handoff_only";
      ownership: "agent_owned";
      authority: "unverified";
    })
  | (AgentAuthObservationBase & {
      kind: "fyagent_managed";
      ownership: "fyagent_managed";
      authority: "verified";
      destination: AgentAuthManagedDestination;
    })
  | (AgentAuthObservationBase & {
      kind: "unavailable";
      authority: "unavailable";
    });

export interface StartAgentAuthSessionRequest {
  agentId: AgentCatalogId;
  intent: AgentAuthIntent;
  providerId?: string;
  inventoryId?: string;
  targetId?: string;
  expectedTargetRevision?: string;
}

export interface AgentAuthSessionSnapshot {
  contractVersion: typeof AGENT_AUTH_CONTRACT_VERSION;
  sessionId: string;
  agentId: AgentCatalogId;
  intent: AgentAuthIntent;
  stage: AgentAuthSessionStage;
  canStopWaiting: boolean;
  outcome: AgentAuthSessionOutcome | null;
  observation: AgentAuthObservation;
  reasonCode: AgentAuthReasonCode | null;
}

export interface AgentAuthPort {
  getObservation(agentId: AgentCatalogId): Promise<AgentAuthObservation>;
  startSession(
    request: StartAgentAuthSessionRequest,
  ): Promise<AgentAuthSessionSnapshot>;
  getSession(sessionId: string): Promise<AgentAuthSessionSnapshot>;
  stopWaiting(sessionId: string): Promise<AgentAuthSessionSnapshot>;
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
  values: readonly T[],
): value is T {
  return typeof value === "string" && values.includes(value as T);
}

function parseStringList<T extends string>(
  value: unknown,
  values: readonly T[],
): T[] {
  if (!Array.isArray(value)) throw new Error("Agent auth is unavailable");
  const parsed = value.map((item) => {
    if (!isOneOf(item, values)) throw new Error("Agent auth is unavailable");
    return item;
  });
  if (new Set(parsed).size !== parsed.length) {
    throw new Error("Agent auth is unavailable");
  }
  return parsed;
}

function isCheckedAt(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length <= 40 &&
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/u.test(value)
  );
}

function parseCommon(
  value: Record<string, unknown>,
  expectedAgentId: AgentCatalogId,
) {
  if (
    value.contractVersion !== AGENT_AUTH_CONTRACT_VERSION ||
    value.agentId !== expectedAgentId ||
    !isOneOf(value.ownership, AGENT_AUTH_OWNERSHIPS) ||
    !isOneOf(value.authority, AGENT_AUTH_AUTHORITIES) ||
    !isCheckedAt(value.checkedAt)
  ) {
    throw new Error("Agent auth is unavailable");
  }
  return {
    contractVersion: AGENT_AUTH_CONTRACT_VERSION,
    agentId: expectedAgentId,
    ownership: value.ownership,
    authority: value.authority,
    allowedIntents: parseStringList(value.allowedIntents, AGENT_AUTH_INTENTS),
    checkedAt: value.checkedAt,
    reasonCodes: parseStringList(value.reasonCodes, AGENT_AUTH_REASON_CODES),
  };
}

function parseProvider(value: unknown): AgentAuthProviderSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["providerId", "label"]) ||
    typeof value.providerId !== "string" ||
    !/^p1:[0-9a-f]{32}$/u.test(value.providerId) ||
    typeof value.label !== "string" ||
    value.label.trim().length === 0 ||
    value.label.length > 80 ||
    /[\r\n\0/\\]/u.test(value.label)
  ) {
    throw new Error("Agent auth is unavailable");
  }
  return { providerId: value.providerId, label: value.label };
}

export function parseAgentAuthObservation(
  value: unknown,
  expectedAgentId: AgentCatalogId,
): AgentAuthObservation {
  if (!isRecord(value) || typeof value.kind !== "string") {
    throw new Error("Agent auth is unavailable");
  }
  const common = parseCommon(value, expectedAgentId);
  switch (value.kind) {
    case "account": {
      if (
        !hasExactKeys(value, [
          "kind",
          "contractVersion",
          "agentId",
          "ownership",
          "authority",
          "state",
          "allowedIntents",
          "checkedAt",
          "reasonCodes",
        ]) ||
        common.ownership !== "agent_owned" ||
        !isOneOf(value.state, AGENT_AUTH_ACCOUNT_STATES)
      ) {
        throw new Error("Agent auth is unavailable");
      }
      return {
        ...common,
        kind: "account",
        ownership: "agent_owned",
        state: value.state,
      };
    }
    case "provider_connections": {
      if (
        !hasExactKeys(value, [
          "kind",
          "contractVersion",
          "agentId",
          "ownership",
          "authority",
          "state",
          "providers",
          "allowedIntents",
          "checkedAt",
          "reasonCodes",
        ]) ||
        common.ownership !== "provider_owned" ||
        !isOneOf(value.state, AGENT_AUTH_PROVIDER_CONNECTION_STATES) ||
        !Array.isArray(value.providers) ||
        value.providers.length > 64
      ) {
        throw new Error("Agent auth is unavailable");
      }
      const providers = value.providers.map(parseProvider);
      if (
        new Set(providers.map((provider) => provider.providerId)).size !==
          providers.length ||
        (value.state === "empty" && providers.length !== 0) ||
        (value.state === "configured" && providers.length === 0)
      ) {
        throw new Error("Agent auth is unavailable");
      }
      return {
        ...common,
        kind: "provider_connections",
        ownership: "provider_owned",
        state: value.state,
        providers,
      };
    }
    case "handoff_only":
      if (
        !hasExactKeys(value, [
          "kind",
          "contractVersion",
          "agentId",
          "ownership",
          "authority",
          "allowedIntents",
          "checkedAt",
          "reasonCodes",
        ]) ||
        common.ownership !== "agent_owned" ||
        common.authority !== "unverified"
      ) {
        throw new Error("Agent auth is unavailable");
      }
      return {
        ...common,
        kind: "handoff_only",
        ownership: "agent_owned",
        authority: "unverified",
      };
    case "fyagent_managed":
      if (
        !hasExactKeys(value, [
          "kind",
          "contractVersion",
          "agentId",
          "ownership",
          "authority",
          "destination",
          "allowedIntents",
          "checkedAt",
          "reasonCodes",
        ]) ||
        expectedAgentId !== "codex" ||
        common.ownership !== "fyagent_managed" ||
        common.authority !== "verified" ||
        !isOneOf(value.destination, AGENT_AUTH_MANAGED_DESTINATIONS)
      ) {
        throw new Error("Agent auth is unavailable");
      }
      return {
        ...common,
        kind: "fyagent_managed",
        ownership: "fyagent_managed",
        authority: "verified",
        destination: value.destination,
      };
    case "unavailable":
      if (
        !hasExactKeys(value, [
          "kind",
          "contractVersion",
          "agentId",
          "ownership",
          "authority",
          "allowedIntents",
          "checkedAt",
          "reasonCodes",
        ]) ||
        common.authority !== "unavailable"
      ) {
        throw new Error("Agent auth is unavailable");
      }
      return { ...common, kind: "unavailable", authority: "unavailable" };
    default:
      throw new Error("Agent auth is unavailable");
  }
}

function isSessionId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/u.test(
      value,
    )
  );
}

export function parseAgentAuthSessionSnapshot(
  value: unknown,
): AgentAuthSessionSnapshot {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "contractVersion",
      "sessionId",
      "agentId",
      "intent",
      "stage",
      "canStopWaiting",
      "outcome",
      "observation",
      "reasonCode",
    ]) ||
    value.contractVersion !== AGENT_AUTH_CONTRACT_VERSION ||
    !isSessionId(value.sessionId) ||
    !isOneOf(value.agentId, AGENT_CATALOG_IDS) ||
    !isOneOf(value.intent, AGENT_AUTH_INTENTS) ||
    !isOneOf(value.stage, AGENT_AUTH_SESSION_STAGES) ||
    typeof value.canStopWaiting !== "boolean" ||
    (value.outcome !== null &&
      !isOneOf(value.outcome, AGENT_AUTH_SESSION_OUTCOMES)) ||
    (value.reasonCode !== null &&
      !isOneOf(value.reasonCode, AGENT_AUTH_REASON_CODES))
  ) {
    throw new Error("Agent auth session is unavailable");
  }
  const observation = parseAgentAuthObservation(
    value.observation,
    value.agentId,
  );
  const terminal = [
    "verified",
    "handoff_complete",
    "failed",
    "cancelled",
    "timed_out",
  ].includes(value.stage);
  const canStop =
    value.stage === "awaiting_user" || value.stage === "verifying";
  const outcomeMatchesStage = (() => {
    switch (value.stage) {
      case "verified":
        return (
          value.outcome === "verified_logged_in" ||
          value.outcome === "verified_logged_out" ||
          value.outcome === "verified_provider_change"
        );
      case "handoff_complete":
        return value.outcome === "handoff_only";
      case "failed":
        return value.outcome === "failed";
      case "cancelled":
        return value.outcome === "cancelled";
      case "timed_out":
        return value.outcome === "timed_out";
      case "preparing":
      case "launching":
      case "awaiting_user":
      case "verifying":
        return value.outcome === null;
    }
  })();
  if (
    value.canStopWaiting !== canStop ||
    (terminal && value.outcome === null) ||
    (!terminal && value.outcome !== null) ||
    !outcomeMatchesStage
  ) {
    throw new Error("Agent auth session is unavailable");
  }
  return {
    contractVersion: AGENT_AUTH_CONTRACT_VERSION,
    sessionId: value.sessionId,
    agentId: value.agentId,
    intent: value.intent,
    stage: value.stage,
    canStopWaiting: value.canStopWaiting,
    outcome: value.outcome,
    observation,
    reasonCode: value.reasonCode,
  };
}

export function assertAgentAuthId(agentId: AgentCatalogId): AgentCatalogId {
  if (!isOneOf(agentId, AGENT_CATALOG_IDS)) {
    throw new Error("Agent auth request is invalid");
  }
  return agentId;
}
