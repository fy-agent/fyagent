# Agent Health

## Scope and owners

Read before changing the `/health` running-status page, its navigation, local
checks or native DTO. `shared/features/health.ts` owns the portable closed
wire contract and the shared health-route builder. `health-presentation.ts`
owns status/reason/action copy and repair destinations; only the lazy health
page imports it at runtime. Keep this presentation module out of the startup
adapter and query dependency graph. `ports.ts`
composes `HealthPort`; `queries.ts` owns the query key/options and cache
subscriptions. The only native boundary is
`shared/platform/tauri/feature-ports/health.ts`. The Tauri composition exposes
a thin async proxy that imports this adapter only on the first `health.get`;
creating ports or prefetching the page must not load its validation code.
The production graph gate keeps this deferred port outside the initial static
closure and applies the existing route chunk size budget.

`pages/health/Page.tsx` composes existing catalog chrome and pure controls.
`useHealthChecks.ts` owns route-local serial dispatch and cancellation.
The directory's “查看状态” button only navigates; it adds no per-card reads.

## Contract

`FeaturePorts.health.get(agentId)` invokes `get_agent_health({ agentId })`.
The Agent ID comes from the seven-ID catalog. Result starts as `unknown` and
is validated with `zod/mini`: exact camelCase fields, version 1, closed enums,
all 12 unique check IDs, UTC RFC3339 times, correct Agent identity, observation
time no later than its check, and checks no later than snapshot completion.
Future snapshots beyond one minute are rejected. Display values are nullable,
at most 80 characters and allowed only for installation/model labels. The
adapter rejects credential-like strings, URLs, paths, control characters and
unknown fields; errors use fixed copy. No raw native error is rendered.

The 12 checks cover installation version/source, helper, conflicts,
configuration, credentials, endpoint, auth, model, drift, proxy, restart and
last request. Unsupported checks stay informational with no repair action.
Unknown critical installation/configuration/secret/auth/model/drift/proxy
facts prevent ready. Unknown last request alone does not prevent local ready.
Ready is “本机检查正常”; it never establishes remote service, quota or a live
session. The last request belongs to that Agent and retains its event time.

Precedence is failed reread/age >= 5 minutes (`stale`), blocked,
not-configured, attention/unknown critical fact, then ready. Stale preserves
the previous facts and their original times; it does not fabricate new data.

## Lifecycle and navigation

Query observers are disabled for automatic fetching. On visible entry or
selected-Agent change, the controller reads only that Agent. “检查全部软件”
reads all seven serially through the same owner. Duplicate actions are gated;
one failure preserves other results and the previous result of the failed
Agent. Stop revokes later dispatch while the already-running read may settle.
Hiding/unmounting revokes dispatch and expiry timers. Reentry waits for an
outstanding read, then rereads the selected Agent. No background polling,
auth observation CLI, OAuth refresh, model request, usage import or repair is
triggered by the health read flow. The local expiry timer changes presentation
only and performs no IPC.

The URL accepts one closed `agent` value. Invalid/duplicate values cannot
reach the port. The visited page retains the last selected Agent when entered
without a query. Name/status filtering changes the rail, leaving the selected
detail available. Existing Agent return context maps to that Agent's health
route through `agentHealthPath`.

The health rail passes its filtered Agent-ID order as `CatalogList.layoutKey`.
Status updates can move the same selected Agent without changing any row size;
the existing selection lens must remeasure that move. Keep Agent keys and
button/overlay nodes stable, retain focus, and let the rail clip the lens while
scrolling. Do not remount the list to refresh its decorative selection.

Actions are closed `installation`, `configuration`, `authentication`,
`model_test`, `refresh`. Installation navigates to the directory; configuration
and model-test navigate to the existing target-specific Models page. Codex/Grok
authentication enters the existing account connection page; other external
login owners remain in Agent detail. Model testing retains the existing form
and confirmation; navigation never sends a request or applies a repair.

## Verification

- `features/health.test.ts` covers precedence, unknown/unsupported and closed
  destinations.
- `platform/healthPort.test.ts` covers exact IPC, safe parsing and browser
  native-only behavior. ACL tests register and authorize the new literal.
- `pages/health/useHealthChecks.test.tsx` covers StrictMode, serialized batch,
  partial failures, stop, hide/reentry, selection change and expiry without IPC.
- `pages/health/Page.test.tsx` covers 12 facts/times, filtering, failures and
  navigation without mutation.
- `tests/browser/health.spec.ts` covers browser IPC fixtures, actual
  navigation/reread, stopping/hidden work, filters, narrow keyboard actions
  and both themes. Status reorder/Models-return checks compare the lens with
  the selected Agent's real bounds, clipping, focus and node identity.
  Existing material/theme/nav performance suites include the
  eighth route without weakening budgets.

These browser fixtures prove renderer behavior, not native file read-only
behavior, real account availability, Windows UAT or release acceptance. Native
collector tests and runtime evidence belong to the backend owner.
