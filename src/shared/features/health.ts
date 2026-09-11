import type { AgentCatalogId } from "./directory";

export const HEALTH_STALE_AFTER_MS = 5 * 60 * 1000;
export const HEALTH_STATUSES = [
  "ready",
  "needs_attention",
  "blocked",
  "not_configured",
  "stale",
] as const;
export type HealthStatus = (typeof HEALTH_STATUSES)[number];
export const HEALTH_CHECK_IDS = [
  "installation",
  "helper",
  "conflicts",
  "configuration",
  "secret",
  "endpoint",
  "auth",
  "model",
  "drift",
  "proxy",
  "restart",
  "last_request",
] as const;
export type HealthCheckId = (typeof HEALTH_CHECK_IDS)[number];
export const HEALTH_CHECK_STATES = [
  "ok",
  "attention",
  "blocked",
  "not_configured",
  "unknown",
  "not_supported",
] as const;
export type HealthCheckState = (typeof HEALTH_CHECK_STATES)[number];
export const HEALTH_SEVERITIES = ["info", "warning", "error"] as const;
export type HealthSeverity = (typeof HEALTH_SEVERITIES)[number];
export const HEALTH_ACTIONS = [
  "installation",
  "configuration",
  "authentication",
  "model_test",
  "refresh",
] as const;
export type HealthAction = (typeof HEALTH_ACTIONS)[number];
export const HEALTH_REASON_CODES = [
  "installation_found",
  "installation_not_found",
  "installation_unknown",
  "installation_not_runnable",
  "multiple_installations",
  "installation_conflict",
  "single_installation",
  "helper_available",
  "helper_unavailable",
  "helper_not_required",
  "configuration_present",
  "configuration_missing",
  "configuration_unreadable",
  "credential_available",
  "credential_missing",
  "credential_unknown",
  "credential_not_required",
  "endpoint_configured",
  "endpoint_missing",
  "endpoint_invalid",
  "endpoint_not_checked",
  "auth_logged_in",
  "auth_logged_out",
  "auth_unknown",
  "auth_managed",
  "auth_handoff",
  "model_configured",
  "model_missing",
  "model_unknown",
  "configuration_in_sync",
  "configuration_drifted",
  "configuration_drift_unknown",
  "proxy_running",
  "proxy_stopped",
  "proxy_not_used",
  "proxy_unknown",
  "restart_required",
  "restart_not_required",
  "restart_unknown",
  "request_succeeded",
  "request_failed",
  "request_not_recorded",
  "not_supported",
  "read_failed",
  "check_timeout",
] as const;
export type HealthReasonCode = (typeof HEALTH_REASON_CODES)[number];

export interface HealthCheck {
  id: HealthCheckId;
  state: HealthCheckState;
  severity: HealthSeverity;
  reasonCode: HealthReasonCode;
  checkedAt: string;
  evidenceAt: string | null;
  value: string | null;
  action: HealthAction | null;
}
export interface AgentHealthSnapshot {
  contractVersion: 1;
  agentId: AgentCatalogId;
  checkedAt: string;
  checks: HealthCheck[];
}
export interface HealthPort {
  get(agentId: AgentCatalogId): Promise<AgentHealthSnapshot>;
}

export function agentHealthPath(agentId: AgentCatalogId): string {
  return `/health?agent=${agentId}`;
}
