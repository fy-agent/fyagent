# Frozen health IPC (2026-09-09)

Main and frontend implementers share this exact contract. Public Rust enums serialize snake_case, DTO fields camelCase. No additional dependency. `get_agent_health` argument `{ agentId }`, result starts as unknown in native TS adapter. Caller supplies no other locator or operation.

```ts
type HealthStatus = 'ready' | 'needs_attention' | 'blocked' | 'not_configured' | 'stale';
type HealthCheckId = 'installation' | 'helper' | 'conflicts' | 'configuration' | 'secret' | 'endpoint' | 'auth' | 'model' | 'drift' | 'proxy' | 'restart' | 'last_request';
type HealthCheckState = 'ok' | 'attention' | 'blocked' | 'not_configured' | 'unknown' | 'not_supported';
type HealthSeverity = 'info' | 'warning' | 'error';
type HealthAction = 'installation' | 'configuration' | 'authentication' | 'model_test' | 'refresh';
type HealthCheck = {
  id: HealthCheckId;
  state: HealthCheckState;
  severity: HealthSeverity;
  reasonCode: HealthReasonCode;
  checkedAt: string; // UTC RFC3339: last local read attempt, not remote test time
  evidenceAt: string | null; // original request/observation event time when known
  value: string | null; // bounded safe version/source/model label only, never path/key/URL/raw error
  action: HealthAction | null;
};
type AgentHealthSnapshot = {
  contractVersion: 1;
  agentId: AgentCatalogId;
  checkedAt: string;
  checks: HealthCheck[]; // exactly all 12 unique ids, no omission/duplicate
};
interface HealthPort { get(agentId: AgentCatalogId): Promise<AgentHealthSnapshot>; }
```

`FeaturePorts.health` owns the port; shared feature module `src/shared/features/health.ts` owns portable types, status/copy/navigation projections. zod/mini strict parser lives in `src/shared/platform/tauri/feature-ports/health.ts`, composes into Tauri ports; browser fixtures provide explicitly fake snapshots. No native imports outside adapter.

Allowed HealthReasonCode values (closed union):

```
installation_found installation_not_found installation_unknown installation_not_runnable
multiple_installations installation_conflict single_installation
helper_available helper_unavailable helper_not_required
configuration_present configuration_missing configuration_unreadable
credential_available credential_missing credential_unknown credential_not_required
endpoint_configured endpoint_missing endpoint_invalid endpoint_not_checked
auth_logged_in auth_logged_out auth_unknown auth_managed auth_handoff
model_configured model_missing model_unknown
configuration_in_sync configuration_drifted configuration_drift_unknown
proxy_running proxy_stopped proxy_not_used proxy_unknown
restart_required restart_not_required restart_unknown
request_succeeded request_failed request_not_recorded
not_supported read_failed check_timeout
```

Status order: a failed reread of an existing snapshot or age >= 5 minutes -> stale (retain all previous facts); otherwise blocked -> not_configured -> attention or unknown critical check -> needs_attention -> ready. `last_request` unknown/no-record is informational and does not by itself invalidate local readiness. `not_supported` is always informational and never fabricated as supported. Unknown core checks (installation/configuration/secret/auth/model/drift/proxy) prevent ready. Ready label is “本机检查正常”; each actual request result remains separately scoped to existing local request records.

Frontend maps reasons to concise Chinese text and the five closed action tokens to existing routes. It must not render arbitrary backend error strings. It may display bounded `value` only after adapter validation and React escaping. For unknown fields, show explicit uncertainty with source limits. There is no hidden retry that spawns shell/network/writes. Explicit model_test navigates to the existing form and its original confirmation; it does not invoke a model from Health Center.

The local refresh path never calls existing `readiness_for` (remote metadata), `auth_observation_for` (external CLI), OAuth refresh, model fetch/probe, configuration write, session usage importer or installer. Reuse local inventory only after proving its selected path avoids those calls; otherwise extend its owner with a minimal local-only read.
