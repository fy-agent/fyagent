# External Agent Lifecycle, Inventory, and Job Contract

## 1. Scope / Trigger

Read this contract before changing Agent install readiness, installation
inventory, target selection, install/update/launch admission, vendor source
capability consumption, transfer/job state, deployment orchestration, rollback,
or recovery.

Primary owners:

- `src-tauri/src/agent_install/lifecycle_policy.rs` — legal product/surface/
  action matrix and default surface;
- `src-tauri/src/agent_install/inventory.rs` — normalized candidates,
  destinations, opaque capabilities, freshness and revalidation;
- `src-tauri/src/agent_install/desktop.rs`, `windows.rs`, `macos.rs`, `cli.rs`
  — execution adapters consuming already admitted source/identity evidence;
- `src-tauri/src/agent_install/jobs.rs` and `types.rs` — job slot, snapshots,
  transfer state and closed wire types;
- `src-tauri/src/commands/agent_install_readiness.rs` — Tauri transport.

This contract does not own the static catalog/runtime surface
([Catalog and Runtime](./external-agent-catalog-runtime.md)), Auth sessions
([External Agent Auth](./external-agent-auth.md)), or the reusable Codex
installer/native helper primitives
([Codex Desktop Installer](./codex-desktop-installer.md)). Product source,
artifact and closed desktop identity rules are owned by
[External Agent Product Sources and Desktop Identity](./external-agent-sources.md).

## 2. Signatures

```text
get_agent_install_readiness({ agentId })
  -> AgentInstallReadinessDto

get_agent_installation_inventory({ agentId, surface? })
  -> AgentInstallationInventoryDto

get_agent_install_preflight({ request: StartAgentActionRequest })
  -> AgentInstallPreflightDto | AgentActionErrorDto

start_agent_action({
  agentId,
  action,
  surface?,
  inventoryId?,
  targetId?,
  expectedTargetRevision?,
  expectedReleaseId?
}) -> AgentActionResult | AgentActionErrorDto

cancel_agent_action({ jobId })
  -> AgentActionJobSnapshot | AgentActionErrorDto

get_agent_action_job({ jobId })
  -> AgentActionJobSnapshot | AgentActionErrorDto
```

Current wire versions:

```text
AgentInstallReadinessDto.contractVersion = 5
AgentInstallationInventoryDto.contractVersion = 1
AgentInstallPreflightDto.contractVersion = 1
AgentActionResult.contractVersion = 4
AgentActionJobSnapshot.contractVersion = 4
```

Closed values:

```text
action  = install | update | launch
surface = cli | desktop

installState   = not_installed | installed | installed_not_runnable |
                 unknown | unavailable
inventoryState = not_observed | single | multiple | unsupported | unknown
updateState    = unavailable | unknown | up_to_date |
                 update_available | latest_unknown
sourceKind     = cli_tooling | managed_desktop | codex_desktop

stage = checking | downloading | staging | launching_installer |
        awaiting_user | installing | verifying_installation |
        succeeded | failed | cancelled | incomplete
```

Opaque capability grammar:

```text
releaseId              = "v1:" + 64 lowercase hex
inventoryId            = "i1:" + 32 lowercase hex
candidate targetId     = "c1:" + 32 lowercase hex
fresh destinationId    = "d1:" + 32 lowercase hex
expectedTargetRevision = "r1:" + 64 lowercase hex
```

The complete target binding is all of
`inventoryId + targetId + expectedTargetRevision` or none of them. Requests
use `deny_unknown_fields` and never accept URL, raw path, registry identity,
command, argument vector, token, hash, package format, signer or bypass flags.

## 3. Contracts

### One policy and one inventory authority

- `lifecycle_policy.rs` is the only legal surface/action matrix. Readiness,
  source resolution and `start_agent_action` consult it before network,
  filesystem, helper or process side effects. The renderer and catalog copy do
  not maintain a second allowlist.
- Legal defaults are currently:
  - Grok Build and Claude Code: CLI;
  - QoderWork, TRAE Work, WorkBuddy, Codex and OpenCode: Desktop.
    OpenCode rejects CLI; Claude rejects Desktop and generic launch.
- `inventory.rs` is the only owner of candidate normalization, provenance
  union, deduplication, stable identity, opaque snapshot/target/revision IDs,
  expiry, stale revalidation and implicit-selection policy.
- Platform adapters emit evidence; they do not select a winner, mint renderer
  IDs or implement competing dedup/revision algorithms.
- The readiness card is a projection of the same normalized inventory. A
  known-path shortcut must not report `installed` while inventory is multiple,
  incomplete or unknown.

### Readiness and target capabilities

- Readiness v5 adds required `configurationEligibility { state, evidence }`.
  `state` is `eligible | not_detected | unknown | unavailable`; positive
  evidence is `cli_runnable | cli_detected | installation_detected`, otherwise
  `none`. Compute this from native observation before the inventory overlay.
  CLI evidence comes from Tooling, never Health color or a config directory.
  Multiple/unknown installation inventory must not erase positive CLI evidence.
  Installation state/version/target rules still reflect inventory uncertainty.
- Configuration eligibility authorizes navigation only. It never supplies
  lifecycle actions, target capabilities, login, or vendor write permission.
  Failed/missing CLI probes do not become eligible even if inventory is single.
  Renderer and host v5 ship together; older readiness payloads fail closed.

- `not_observed` means a complete supported scan found no trusted candidate;
  it may expose a reviewed fresh destination. `unknown` means the scan was
  incomplete or authority was unavailable and must not become
  `not_installed`.
- Multiple trusted candidates remain `multiple`, clear the single local
  version, require target selection and expose no implicit first/nearest
  choice.
- Candidate/destination IDs are short-lived in-process capabilities, not paths
  or durable preferences. Do not store them in application settings.
- Immediately before launch/write, re-enumerate and compare the selected
  capability. Expired inventory, missing candidate, changed scope/owner/file
  identity/revision or newly ambiguous evidence authorizes zero side effects.
- `install` binds one fresh destination. `update` binds one existing eligible
  candidate. Legacy launch may omit a target only when exactly one trusted
  launchable candidate exists.
- Compact single-surface products omit the `surfaces` readiness array and the
  inventory `surface` field. A multi-surface product must make each surface
  explicit instead of collapsing status.
- Compact CLI readiness uses `sourceKind=cli_tooling`. The renderer
  `parseAgentInstallReadiness` / `surfacesForAgent` table must stay aligned
  with `lifecycle_policy.rs`. Treating Claude Code as `managed_desktop` is
  not a product-absent signal; it is a contract parse failure.

### Source and desktop-identity routing

[External Agent Product Sources and Desktop Identity](./external-agent-sources.md)
owns product release discovery, exact npm source admission, redirect/artifact
bounds, closed Desktop identity, and platform scan evidence. This lifecycle
contract consumes only the resulting release capability or normalized platform
observation; it does not duplicate product URLs, registry/hash rules, bundle
IDs, EXE relatives, signer leaves or Uninstall matching.

Source and identity failure must remain evidence-strength preserving:

- missing/drifted source data yields `source_not_verified`, never a stale URL;
- incomplete native identity yields `unknown`/unsupported, never installed;
- vendor-wizard handoff is not installed evidence;
- CLI package resolution supplies an exact plan, while lifecycle owns only the
  action/job transition around that plan.

### Download preflight and confirmation

- The legacy `run_tool_lifecycle_action` IPC keeps its argument shape but
  rejects all install/update actions with a direction to the software detail
  preflight. It cannot bypass confirmation through a native install action;
  only `start_agent_action` forwards the native-owned confirmed target.
- Install/update first calls `get_agent_install_preflight` with the same closed
  request used for execution. It creates no action job and performs no Agent
  mutation: validate the live inventory revision, release/platform/architecture,
  required native or CLI runtime, ordinary-user target permissions, and available
  space on the temporary and target volumes. Metadata resolution is permitted;
  package download, npm install and helper mutation are not.
- The v1 summary contains the exact request, platform, architecture,
  `versionOrChannel`, native-resolved `downloadUrl` (null for CLI operations),
  redacted `targetLabel`, `availableBytes`, nullable `requiredBytes` and
  `artifactSizeBytes`, closed `spaceBudgetBasis`, and closed `runtime`
  (`native_installer|node_npm|existing_cli`) and `execution`
  (`current_user|system_authorization|vendor_wizard`). Desktop capacity is checked
  on every distinct temporary/target volume against the shared checked 3× budget:
  `source_size` uses metadata for the exact downloaded artifact; `download_limit`
  uses the existing 2 GiB downloader cap (6 GiB reserve). A ZIP/other architecture
  size never describes a DMG. The size hint, source URL, and budget are bound to
  the release/prepared target and rechecked before consuming confirmation.
  Unavailable space, arithmetic overflow, insufficient space, and source/budget
  drift fail closed. The preview calls these conservative budgets, not vendor
  exact requirements. CLI package/dependency footprint is not currently bounded;
  `cli_unknown` returns null required/size values and explicitly does not claim
  measured free capacity is sufficient. A nonzero CLI readout is not a completed
  installation-capacity check. No extra GET/HEAD was introduced.
  The URL is display metadata from the existing platform/architecture resolver;
  execution never accepts it back, and its presence does not prove the downloaded
  artifact has already passed validation.
- Inventory owns one prepared native target per existing short-lived snapshot.
  Execution repeats the necessary checks and consumes the exact request/path/
  runtime/identity-scope binding once. A changed target requires a new preflight;
  no confirmation-time target substitution or fallback is allowed.
- Windows CLI permission/runtime checks run inside the authenticated ordinary-user
  helper. CLI preview binds the helper-resolved npm prefix/cache/temp and npm
  identity rather than guessed Local/Roaming AppData volumes, transfers the native
  reserve budget, stores that exact dest in prepared authority, and maps helper
  space/conflict/dest-drift failures to the existing actionable reasons.
  Official EXE wizards choose their own final destination: the summary
  must describe this handoff, not claim the elevated host checked Alice's access
  to a destination that the wizard has not chosen.
- `insufficient_disk_space`, `disk_space_unavailable`, runtime, identity and
  permission failures have specific renderer recovery copy. An uncertain
  `recovery_required` terminal result is retained; a same-Agent retry cannot
  replace that known unresolved result in the job slot.

### Jobs and platform side effects

- One non-terminal Agent job may exist. A second start returns
  `operation_conflict`; terminal jobs release the slot.
- Transfer snapshots report raw monotonic `completedBytes`, optional
  `totalBytes`, attempt/maxAttempts, sequence and RFC3339 observation time.
  Unknown total remains unknown; the renderer derives speed/percent.
- On macOS, download and staging remain cancellable. The atomic commit boundary
  is `installing`; after it, `cancellable=false`. Success requires fresh
  re-enumeration of the exact selected canonical path, scope, closed identity
  and comparable version. Verification failure restores and re-verifies the
  prior bundle when possible, returning `rollback_restored` or
  `recovery_required`, never green.
- System `/Applications` writes remain rejected with
  `authorization_required` while
  [macOS Privileged System-Commit Helper](./macos-system-commit.md)
  keeps its production gate closed. Never silently fall back to
  `~/Applications` and call it a system install.
- On Windows, official vendor EXE launch uses the protected retained artifact,
  closed product/action helper protocol, Alice-owned authenticated pipe and
  reviewed signer/product admission. `launching_installer` is the
  non-cancellable side-effect boundary; `awaiting_user` means vendor UI/UAC
  owns interaction.
- Successful Windows `ShellExecute` is vendor-wizard handoff and settles the
  job as succeeded under the current contract. It is not proof that the wizard
  installed anything; inventory can remain `not_installed` until a later scan.
  FyAgent does not wait for/kill the wizard or delete the retained bridge EXE
  leaf during successful settlement.
- Grok Build on formal elevated Windows uses the closed ordinary-user
  `grok-tool` helper and never falls back to running the user CLI elevated.
  Default install executes a host-supplied exact-version npm plan; the helper
  does not resolve `@latest`. Native install is `install_native` only.
  Installed updates preserve the observed `native_internal` or
  `official_npm` owner; switching owner is a separate explicit action.
  Installing the CLI does not claim that Grok sign-in or inference works on
  the user's network.

## 4. Validation & Error Matrix

| Condition                                                                              | Required result                                                                                 |
| -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Unknown Agent/action/surface or excess request field                                   | Reject; no job.                                                                                 |
| Product/surface is illegal                                                             | `surface_not_supported`; no source or side effect.                                              |
| Product action is disabled                                                             | `action_not_supported` before target/network/write.                                             |
| Renderer supplies URL/path/command/token/hash/package/signer/bypass                    | Reject; no job.                                                                                 |
| Target triplet is partial or malformed                                                 | `refresh_required`; no side effect.                                                             |
| Inventory expired or selected identity/revision/scope changed                          | Closed inventory/target error; no launch/write.                                                 |
| Multiple candidates and no selected target                                             | `target_selection_required`; never choose first.                                                |
| Managed desktop update is disabled/up-to-date/non-single/ineligible                    | Omit `update`; refuse the job.                                                                  |
| Codex install/update uses Agent action                                                 | `managed_by_codex_desktop`; Agent slot stays free.                                              |
| Source host/schema/redirect/port/body grammar fails                                    | `source_not_verified`/official-page fallback; no stale pin.                                     |
| Fetch is cancelled                                                                     | `cancelled`; do not remap to source failure.                                                    |
| Selected macOS system target while helper gate is closed                               | `authorization_required`; zero write/fallback.                                                  |
| App is running or staging permission denied                                            | Closed running/permission error; preserve original target.                                      |
| macOS post-install identity/path/scope/version check fails                             | Restore/reverify; `rollback_restored` or `recovery_required`.                                   |
| Windows inventory view is incomplete                                                   | `unknown` + native projection reason; no fresh destination.                                     |
| Windows EXE product/signer/trust/arch/helper/pipe binding fails                        | Fail before installer launch.                                                                   |
| User cancels Windows UAC/vendor launch                                                 | Cancelled/installer-user-cancelled result.                                                      |
| Windows official EXE ShellExecute succeeds                                             | Job succeeded as handoff; do not claim installed proof.                                         |
| Focused source/identity owner emits one complete trusted candidate                     | Lifecycle may normalize it and expose only policy-legal actions.                                |
| Source/identity evidence is absent, conflicting, stale or incomplete                   | Preserve unknown/not-installed distinction; do not synthesize a candidate or action.            |
| Source/desktop identity owner cannot produce an admitted release or installed identity | Preserve its fail-closed reason; do not create or advance a lifecycle job.                      |
| Claude/Grok source owner cannot produce an exact current-platform npm plan             | No package action; never substitute a tag, fixture, foreign platform package or stale manifest. |
| CLI execution exits but owner/version verification is not satisfied                    | Terminal verification failure; do not report Agent `succeeded`.                                 |
| Cancel after `launching_installer`/`installing`                                        | `operation_conflict`; do not kill external/commit operation.                                    |
| Secret/path/raw native identity reaches DTO/log/DOM                                    | Security regression.                                                                            |

## 5. Good / Base / Bad Cases

- **Good:** inventory returns two trusted candidates, readiness becomes
  `multiple`, the user selects one opaque target, and start revalidates it
  before launch.
- **Good:** a Qoder source refresh validates the `/latest/` alias and same-host
  version feed, then starts with the matching opaque release capability.
- **Good:** macOS updates the exact selected app path and either verifies that
  target or restores the prior bundle.
- **Base:** Windows vendor wizard opens successfully; the job is a successful
  handoff while installation status stays unchanged until a fresh inventory.
- **Good:** after a manual vendor install, the focused identity owner emits one
  complete trusted candidate and a fresh lifecycle scan exposes only the
  policy-legal action for that normalized identity.
- **Base:** complete Windows discovery finds no candidate and exposes an
  eligible reviewed destination; an inaccessible view instead remains
  unknown.
- **Bad:** use a researched CDN URL, infer install from a config directory,
  update Qoder/TRAE/WorkBuddy, choose the first candidate, fake percent without
  total bytes, or label Windows wizard handoff as installed evidence.
- **Bad:** bypass the source/identity owner with a tag, fixture, researched URL,
  unverified path or stale capability, or treat process exit/vendor handoff as
  installed evidence without the required post-action inventory/owner check.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck
mise run test:unit
mise run test:browser
```

Assertion points:

- exact wire versions/enums/keys, `deny_unknown_fields`, forbidden-field scan,
  opaque ID grammars and seven catalog IDs;
- lifecycle policy is the sole action/surface owner; disabled actions perform
  no source lookup or side effect;
- inventory merges duplicate provenance but preserves multiple/conflicting/
  incomplete evidence, expires capabilities and rejects drift;
- source/identity owner tests independently enforce exact host/platform/schema,
  redirect, current-platform npm admission and closed desktop identity; this
  lifecycle suite proves those failures prevent job creation or advancement;
- Claude/Grok lifecycle tests consume only admitted exact plans, preserve the
  observed owner, require post-action owner/version verification and reject the
  retired/unsupported surface; source mechanics stay in their focused owner;
- renderer `surfacesForAgent` / readiness `sourceKind` stay aligned with
  lifecycle policy: Grok and Claude are compact CLI/`cli_tooling`;
- macOS exact-path deployment, cancellation boundary, running-app protection,
  rollback/recovery and disabled `/Applications` gate;
- platform-specific source, desktop identity, registry/signer and helper
  protocol assertions run in their focused owners; lifecycle integration proves
  only normalized evidence/capabilities can authorize actions;
- job single-flight, terminal slot release, transfer monotonicity, unknown
  total, cancel refusal after side-effect boundary and unknown job ID;
- Grok/Claude owner-preserving lifecycle has no elevated fallback and reaches
  `succeeded` only after the focused execution owner reports verified outcome;
- renderer polls until a terminal native stage and does not paint a poll cap
  as failure while a job remains active. Browser fixtures do not prove native
  inventory, installer or signing behavior.

## 7. Wrong vs Correct

Wrong:

```ts
await invoke("start_agent_action", {
  agentId: "trae-work",
  action: "install",
  url: cachedCdnUrl,
  targetPath: selectedPath,
});
```

Correct:

```ts
const readiness = await ports.agentInstallReadiness.get("trae-work");
const inventory = await ports.agentInstallReadiness.getInventory("trae-work");
const destination = inventory.freshDestinations.find((item) => item.eligible);
if (!destination) throw new Error("No verified destination");

await ports.agentInstallReadiness.startAction({
  agentId: "trae-work",
  action: "install",
  expectedReleaseId: readiness.releaseId ?? undefined,
  inventoryId: inventory.inventoryId,
  targetId: destination.destinationId,
  expectedTargetRevision: destination.destinationRevision,
});
```

Wrong:

```ts
surfacesForAgent("claude-code") === ["desktop"];
sourceKind === "managed_desktop";
```

Correct:

```ts
surfacesForAgent("claude-code") === ["cli"];
sourceKind === "cli_tooling";
```

Wrong:

```rust
let candidate = inventory.candidates.first().unwrap();
launch(candidate.path)?;
```

Correct:

```rust
let validated = validate_action_target(&request, state).await?;
// The validated capability is produced only after fresh re-enumeration.
dispatch_closed_action(validated, state).await
```
