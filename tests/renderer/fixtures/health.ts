import type { AgentCatalogId } from "@/shared/features/directory";
import {
  HEALTH_CHECK_IDS,
  type AgentHealthSnapshot,
  type HealthCheck,
  type HealthCheckId,
} from "@/shared/features/health";

/** Explicitly fake local observations for renderer/browser behavior tests. */
export function healthSnapshotFixture(
  agentId: AgentCatalogId = "codex",
  overrides: Partial<Record<HealthCheckId, Partial<HealthCheck>>> = {},
  checkedAt = new Date().toISOString(),
): AgentHealthSnapshot {
  const defaults: Record<
    HealthCheckId,
    Pick<HealthCheck, "state" | "reasonCode">
  > = {
    installation: { state: "ok", reasonCode: "installation_found" },
    helper: { state: "ok", reasonCode: "helper_not_required" },
    conflicts: { state: "ok", reasonCode: "single_installation" },
    configuration: { state: "ok", reasonCode: "configuration_present" },
    secret: { state: "ok", reasonCode: "credential_available" },
    endpoint: { state: "ok", reasonCode: "endpoint_configured" },
    auth: { state: "ok", reasonCode: "auth_managed" },
    model: { state: "ok", reasonCode: "model_configured" },
    drift: { state: "ok", reasonCode: "configuration_in_sync" },
    proxy: { state: "ok", reasonCode: "proxy_not_used" },
    restart: { state: "ok", reasonCode: "restart_not_required" },
    last_request: { state: "unknown", reasonCode: "request_not_recorded" },
  };
  return {
    contractVersion: 1,
    agentId,
    checkedAt,
    checks: HEALTH_CHECK_IDS.map((id) => ({
      id,
      ...defaults[id],
      severity: "info",
      checkedAt,
      evidenceAt: null,
      value:
        id === "installation"
          ? "1.2.3 / 桌面应用"
          : id === "model"
            ? "fixture-model"
            : null,
      action: id === "last_request" ? "model_test" : null,
      ...overrides[id],
    })),
  };
}
