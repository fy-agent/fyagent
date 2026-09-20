# Agent Health Observation

## Scope and owners

Read before changing `get_agent_health` or its twelve local checks.
`commands/health.rs` admits one closed Agent ID and serializes health reads.
`services/health/` composes metadata from installation, configuration,
managed authentication, proxy runtime and historical request owners.
The renderer contract and presentation are owned by
[Frontend Agent Health](../frontend/health.md).

This feature owns observation only. Installation, authentication, model
testing and configuration repairs keep their existing commands and approval
boundaries. No new scheduler, persisted health database or mutation endpoint
is introduced.

## Wire and source boundaries

`get_agent_health({agentId}) -> AgentHealthSnapshot` has version 1, camelCase
fields, snake_case closed enums, UTC RFC3339 timestamps and exactly twelve
unique checks: installation, helper, conflicts, configuration, secret,
endpoint, auth, model, drift, proxy, restart and last_request.

Each check carries state, severity, stable reasonCode, local checkedAt,
nullable original evidenceAt, nullable safe value and nullable closed action.
The collector never returns native errors, paths, URLs, auth documents,
account identity, credential material or SecretRef. Display values are
limited to safe installation/version/source and model metadata; rejecting a
decoration must not turn an unknown fact into a positive result.

Evidence rules:

- Local file discovery proves presence, not successful execution or login.
  CLI health may reuse Tooling search directories (login PATH, process PATH,
  product env) as filesystem roots. It still never executes `--version`,
  never spawns the CLI through a login shell, never fetches remote release
  metadata, and never uses the regular CLI readiness observer.
- Desktop health uses the explicit Desktop-only inventory entry point.
  Existing inventory capabilities remain owned by the installation service;
  the health view cannot manufacture target IDs or invoke a lifecycle action.
- Configuration reads do not use `get_effective_current_provider` or
  `ProviderService::current`: those may clear stale persisted selection.
  Invalid source data fails without forwarding or logging its raw diagnostic.
  Drift compares the selected request source, including managed proxy
  projection, rather than unrelated personal/MCP settings.
- Credential presence and a native connection observation are separate facts.
  Neither proves remote authorization, remaining quota or successful pickup
  by a running application. Pending restart retains its native meaning.
  A disconnected Codex slot with no persisted managed connection may still
  have recognizable native ChatGPT OAuth material on the official file route.
  The Codex consumer preserves that fact as non-serialized native metadata;
  Health keeps auth and credentials unknown instead of claiming logged out or
  missing. Missing native material and persisted managed disconnects retain
  their existing results. This exception never establishes remote login.
- Per-Agent proxy intent uses SELECT-only `health_proxy_enabled`.
  Missing rows remain unknown; the legacy proxy getter can seed rows and is
  prohibited on this path. A running global listener alone cannot establish
  this Agent's route is correct.
- Last request uses `latest_health_request`: exact app_type, optional selected
  provider_id and data_source=proxy. Session imports, other Agents, global
  counters and usage-cost backfill are not request evidence. The original
  request timestamp remains distinct from today's local observation.
- Vendor model metadata stays in the vendor owner. TRAE health opens SQLite
  without creating a missing cache or shared-memory sidecars. An existing
  WAL, shared-memory file or rollback journal makes health observation unknown;
  it must not silently ignore pending journal data. The health-only reader
  limits the model map to 2 MiB, 128 matching keys and 512 bytes per key,
  and checks file stability. Existing model flows retain their prior capacity.
  Missing optional/private observations stay unknown or explicitly unsupported.
- Helper inspection never triggers installation, elevation or registration.
  An unimplemented observer stays unsupported, not helper_available.

## Failure and product semantics

Admission belongs to one detached read coordinator, not the IPC caller's
waiting lifetime. The caller returns `health_timeout` after eight seconds;
the coordinator retains its owned permit and awaits every started local read.
Until it actually completes, retries return `health_busy` without starting
another observation. Inner health readers do not time out and detach their
blocking tasks. A timeout or caller cancellation never manufactures a fresh
snapshot; the renderer keeps its prior result stale.

Independent sources may fail without hiding the other checks. Fixed errors
and bounded local read attempts replace native diagnostic strings.
Unsupported checks are informational with no automatic action; critical
unknown observations prevent a local-ready result. The UI expires results
after five minutes and preserves original facts after a failed reread.

Health calls Managed Auth's internal `observe_overview` facade. Proxy slots
are projected in memory from SELECT-only rows, including missing/default and
historical slots; only the existing management overview reconciles them to DB.
The shared route observer reads saved provider selection without the repairing
`get_effective_current_provider` selector. A stale selected ID stays unchanged;
the account's route stays unknown unless another target proves a matching
active route for that same account.
Tests compare complete fixture database dumps, timestamps, native auth bytes
and vault operation counts across repeated observations.

Drift uses only current request-route facts. Closed source/endpoint/model/
credential-state reason codes explain one changed category; multiple categories
retain the generic drift reason. No raw comparison values cross IPC, and no
repair action is run. Syntactically valid Grok configuration whose selected
profile/schema is not recognized is `configuration_profile_unknown` (unknown),
not file corruption or a successful builtin/native login. Write/proxy validation
remains strict; unknown vendor formats are never rewritten automatically.

Default refresh never performs target CLI execution, network requests,
OAuth/token refresh, model discovery/probes, session import, file repair or
business database writes. A user-requested model test navigates to the
existing form and its explicit execution boundary.

## Tests required

Use `mise run rust:test -- health` for observation, DTO, per-Agent record,
route/credential distinction and negative write/secret tests. Before
completion run the standard prearchive gate and production renderer/browser
checks from the task record. Runtime smoke in an isolated macOS home proves
only that tested local flow; Windows, signing, releases and production
require their own evidence.

## Validation and errors

| Input or source                                                          | Result                                                                 |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| Agent outside the seven `AgentCatalogId` values                          | Reject before collection                                               |
| Another admitted read is still running                                   | Fixed `health_busy`; no second read starts                             |
| Missing, malformed or unreadable local evidence                          | Relevant check remains unknown or not configured; no raw error escapes |
| Unsupported private vendor fact                                          | `not_supported`, informational severity, no repair action              |
| Invalid display metadata                                                 | Omit `value`; never expose a path, URL or credential                   |
| Renderer sees duplicate/missing check, unknown enum or invalid timestamp | Reject the snapshot and retain previous valid facts                    |

The caller receives fixed `health_timeout` after eight seconds or
`health_read_failed` if the admitted coordinator fails. A timed-out reader
retains capacity until actual completion, so retry remains `health_busy`.

No new environment variable is required by the command. Native smoke tests may
use the existing `FYAGENT_TEST_HOME` isolation mechanism; it is not a wire input.

## Good, base and bad cases

- Good: a selected third-party request source matches its local configuration;
  configuration is in sync while remote request availability remains untested.
- Base: no locally recorded proxy request exists; show no record, without
  fabricating success or treating another Agent's traffic as evidence.
- Bad: a vendor SQLite WAL exists; return unknown rather than ignoring its
  pending data or opening it in a way that creates a shared-memory sidecar.

## Wrong versus correct

Wrong: reuse `ProviderService::current` or `get_proxy_config_for_app` because
their names sound like reads. They can repair selection or seed persisted rows.

Correct: compose the pure selected-source projection with SELECT-only
`health_proxy_enabled` and `latest_health_request`; assert the database and
vendor configuration remain unchanged after observation.

### Observer path isolation

`tooling::observe_local_tool_health` must use the filesystem path collector with
no login-shell PATH input. It must not call the interactive readiness collector:
resolving a login environment launches user shell startup files and can mutate
user state even when the eventual CLI binary is never executed. Health considers
only existing process PATH and known local directories; a shell-only installation
may remain unobserved. Installer/readiness keeps its existing executable probes.
A public-observer fixture uses a fake shell and CLI with an execution marker;
neither may run. The seeded admission fixture also compares all persisted rows
(including timestamps), native bytes, settings bytes and vault operation count.
Only the SQL export header's generation-time comment is excluded from comparison.
