# External Agent Lifecycle, Inventory, and Job Contract

## 1. Scope / Trigger

Read this contract before changing Agent install readiness, installation
inventory, target selection, install/update/launch admission, vendor source
resolution, transfer/job state, desktop identity, or platform deployment.

Primary owners:

- `src-tauri/src/agent_install/lifecycle_policy.rs` — legal product/surface/
  action matrix and default surface;
- `src-tauri/src/agent_install/inventory.rs` — normalized candidates,
  destinations, opaque capabilities, freshness and revalidation;
- `src-tauri/src/agent_install/sources/**` and `fetch.rs` — reviewed source
  metadata, redirects and artifact transport;
- `src-tauri/src/agent_install/desktop.rs`, `windows.rs`, `macos.rs`, `cli.rs`
  — product/platform evidence and execution adapters;
- `src-tauri/src/agent_install/jobs.rs` and `types.rs` — job slot, snapshots,
  transfer state and closed wire types;
- `src-tauri/src/commands/agent_install_readiness.rs` — Tauri transport.

This contract does not own the static catalog/runtime surface
([Catalog and Runtime](./external-agent-catalog-runtime.md)), Auth sessions
([External Agent Auth](./external-agent-auth.md)), or the reusable Codex
installer/native helper primitives
([Codex Desktop Installer](./codex-desktop-installer.md)).

## 2. Signatures

```text
get_agent_install_readiness({ agentId })
  -> AgentInstallReadinessDto

get_agent_installation_inventory({ agentId, surface? })
  -> AgentInstallationInventoryDto

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
AgentInstallReadinessDto.contractVersion = 4
AgentInstallationInventoryDto.contractVersion = 1
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
  - Grok Build: CLI;
  - QoderWork, TRAE Work, WorkBuddy, Codex, Claude and OpenCode: Desktop.
  OpenCode and Claude reject the Agent `cli` surface.
- `inventory.rs` is the only owner of candidate normalization, provenance
  union, deduplication, stable identity, opaque snapshot/target/revision IDs,
  expiry, stale revalidation and implicit-selection policy.
- Platform adapters emit evidence; they do not select a winner, mint renderer
  IDs or implement competing dedup/revision algorithms.
- The readiness card is a projection of the same normalized inventory. A
  known-path shortcut must not report `installed` while inventory is multiple,
  incomplete or unknown.

### Readiness and target capabilities

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

### Product and source policy

| Product | Owner and current lifecycle policy |
| --- | --- |
| Grok Build | CLI Tooling owner; default fresh install uses the official `@xai-official/grok` npm package, a bundled exact-version manifest, and a mainland-first registry chain. Native `x.ai` install is an explicit secondary action. Updates preserve the observed `native_internal` or `official_npm` owner. |
| Codex | Dedicated Codex Desktop installer; Agent action returns `managed_by_codex_desktop` and does not occupy the Agent job slot. |
| QoderWork CN | Managed desktop; install/launch admitted, FyAgent update disabled. Source is the reviewed first-party `/qoder-work-cn/releases/latest/` aliases and same-host Electron-builder version feed. |
| TRAE Work CN | Managed desktop; install/launch admitted, FyAgent update disabled. Resolve `data.solo` with `region=cn`; never TRAE Code/`data.manifest`. |
| WorkBuddy | Managed desktop; install/launch admitted, FyAgent update disabled. Resolve the closed `/v2/update` platform IDs and reviewed macOS suffix rewrite. |
| Claude Desktop | Desktop only; use the reviewed source and closed bundle identity on supported hosts. No public Claude CLI installer. |
| OpenCode Desktop | Desktop only; use reviewed stable desktop artifacts and closed bundle identity on supported hosts. No public OpenCode CLI installer. |

- Qoder display version comes only from an unindented top-level `version:` in
  bounded same-host `latest.yml`/`latest-mac.yml`. The feed ZIP and `sha512`
  are metadata, not admitted artifacts. Windows ARM64 remains unsupported
  until a separately reviewed first-party artifact exists.
- TRAE source selection uses the Work/Solo CN object and closed host/path/
  filename rules. Local comparable version is `tronBuildVersion` from bounded
  `product.json`, not the Electron marketing `appVersion`.
- WorkBuddy uses closed platform IDs and the official download host. On macOS,
  rewrite only the validated terminal `.zip` suffix to `.dmg`. A shorter local
  dotted marketing version may equal a longer remote product-version prefix;
  same-length differing segments remain an update.
- Claude metadata cannot replace the code-owned artifact authority. OpenCode
  uses the reviewed locale-neutral stable Desktop aliases, including
  `windows-x64-nsis` on Windows x64. GitHub latest is display-only and must not
  gate installability. Windows x64 OpenCode install is admitted after the
  reviewed WinVerifyTrust identity contract; ARM64 remains unsupported.
- A missing/drifted source schema, host, redirect or release capability returns
  `source_not_verified`/official-page guidance. Never pin a package URL copied
  from an investigation or infer a version from ETag, Last-Modified or prose.

### Fetch and release capability

- HTTPS only; no userinfo and no explicit non-default port.
- Validate every redirect hop against the product allowlist and bounded hop
  count. Scheme downgrade or unknown host fails closed.
- Metadata is bounded to 1 MiB and artifacts to 2 GiB under the current
  installer transport.
- `expectedReleaseId` binds the canonical, backend-resolved release fields for
  products that require source freshness. A forced refresh must match before
  download. Release IDs never encode or expose a URL.
- Cancellation maps to `cancelled`, not source/schema failure.

### Closed desktop identity

Folder names and vendor config directories are not identity. Current closed
identity examples include:

| Product | macOS bundle ID | Windows closed identity summary |
| --- | --- | --- |
| WorkBuddy | `com.workbuddy.workbuddy` | Closed relative `WorkBuddy.exe`, ProductName and reviewed signer. |
| QoderWork CN | `com.qoder.work.cn` | Closed QoderWork CN relative EXE names, ProductName and signer. |
| TRAE Work CN | `cn.trae.solo.app` | Closed TRAE SOLO/Work CN relative EXE names, ProductName and signer. |
| OpenCode | `ai.opencode.desktop` | Closed relative `@opencode-aidesktop/OpenCode.exe` (and `OpenCode/OpenCode.exe`), ProductName `OpenCode`, reviewed signer `Anomaly Innovations, Inc https://anoma.ly/`, and Uninstall DisplayName `OpenCode` or `OpenCode <bounded-version>`. |
| Claude | `com.anthropic.claudefordesktop` | Windows fails closed until reviewed package/PE identity exists. |

Windows scan identity is the installed target, not the downloaded installer
leaf:

- Freeze `windows_relative_exes` from a WinVerifyTrust-Valid installed EXE
  under Alice `LocalAppData\Programs`, plus any reviewed installer-stub
  relative that still occurs on disk. OpenCode's official `windows-x64-nsis`
  stub may be i386 `OpenCode/OpenCode.exe` while the current-user install is
  AMD64 `%LOCALAPPDATA%\Programs\@opencode-aidesktop\OpenCode.exe`. Keep both.
- KnownPath `Missing` is dropped. It is not retained evidence and cannot
  become `not_installed` by itself. Uninstall/App Paths hints remain.
- Uninstall `DisplayName` matches a closed ProductName exactly
  (ASCII-case-insensitive) or `{name} {bounded_version}`. Reject channel
  words (`Dev`, `Beta`) and prerelease suffixes (`1.18.27-beta`). Empty
  `InstallLocation` is allowed; `DisplayIcon` and derived uninstall directories
  are hints, never commands.
- Fresh Windows destination for OpenCode is `WindowsCurrentUser` (same
  family as QoderWork CN). Destination `location_label` uses the catalog
  display name and is not the known-path folder; do not invent scan relatives
  from that label.
- In-app NSIS handoff and a later user-run official installer share this scan.
  Successful `ShellExecute` still does not prove installed.
- macOS scans direct-child `.app` bundles in user/system Applications roots,
  rejects symlinks and verifies plist/bundle identity. Absence on a shipped
  host is `not_installed`; Linux development remains `unknown`.
- Windows combines the frozen Explorer user and machine Uninstall/App Paths in
  explicit registry views with known roots. Registry strings are evidence,
  never commands. Open inventory parents with query+enumerate rights and
  children query-only; rejected shared-view links are absence, while access,
  enumeration, bound or Shell-context failure makes the aggregate unknown.
- A Windows candidate is actionable only after stable no-reparse file
  identity, supported application architecture, closed ProductName,
  `WinVerifyTrust`, exactly one signer and reviewed signer leaf.
- Do not infer installation from `.workbuddy`, `.qoderwork*`, `.trae*` or any
  settings directory.

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

| Condition | Required result |
| --- | --- |
| Unknown Agent/action/surface or excess request field | Reject; no job. |
| Product/surface is illegal | `surface_not_supported`; no source or side effect. |
| Product action is disabled | `action_not_supported` before target/network/write. |
| Renderer supplies URL/path/command/token/hash/package/signer/bypass | Reject; no job. |
| Target triplet is partial or malformed | `refresh_required`; no side effect. |
| Inventory expired or selected identity/revision/scope changed | Closed inventory/target error; no launch/write. |
| Multiple candidates and no selected target | `target_selection_required`; never choose first. |
| Managed desktop update is disabled/up-to-date/non-single/ineligible | Omit `update`; refuse the job. |
| Codex install/update uses Agent action | `managed_by_codex_desktop`; Agent slot stays free. |
| Source host/schema/redirect/port/body grammar fails | `source_not_verified`/official-page fallback; no stale pin. |
| Fetch is cancelled | `cancelled`; do not remap to source failure. |
| Selected macOS system target while helper gate is closed | `authorization_required`; zero write/fallback. |
| App is running or staging permission denied | Closed running/permission error; preserve original target. |
| macOS post-install identity/path/scope/version check fails | Restore/reverify; `rollback_restored` or `recovery_required`. |
| Windows inventory view is incomplete | `unknown` + native projection reason; no fresh destination. |
| Windows EXE product/signer/trust/arch/helper/pipe binding fails | Fail before installer launch. |
| User cancels Windows UAC/vendor launch | Cancelled/installer-user-cancelled result. |
| Windows official EXE ShellExecute succeeds | Job succeeded as handoff; do not claim installed proof. |
| OpenCode Windows x64 identity is complete | Admit current-user NSIS handoff; GitHub latest must not gate the stable source. |
| OpenCode Windows ProductName/relative EXE/signer is empty | Reject EXE download/install; do not claim supported. |
| OpenCode known-path `@opencode-aidesktop\OpenCode.exe` exists with closed ProductName and reviewed signer | Inventory `installed`; `launch` is legal. Manual NSIS uses the same scan as in-app handoff. |
| OpenCode Uninstall DisplayName is `OpenCode <bounded-version>` | Keep the ARP hint; do not require exact `OpenCode`. |
| OpenCode Uninstall DisplayName is `OpenCode Dev`, `OpenCodeAI`, or a prerelease version | Skip that ARP entry. |
| OpenCode KnownPath relative is missing | Drop the observation; do not retain KnownPath Missing. |
| Grok default install has no native expected owner | Plan official npm from the bundled exact-version manifest; never `@latest`. |
| Cancel after `launching_installer`/`installing` | `operation_conflict`; do not kill external/commit operation. |
| Secret/path/raw native identity reaches DTO/log/DOM | Security regression. |

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
- **Good:** after a user-run official OpenCode NSIS, a complete scan finds
  `@opencode-aidesktop\OpenCode.exe` (or an Uninstall DisplayName
  `OpenCode 1.18.27` plus DisplayIcon) and readiness exposes `launch`.
- **Base:** complete Windows discovery finds no candidate and exposes an
  eligible reviewed destination; an inaccessible view instead remains
  unknown.
- **Bad:** use a researched CDN URL, infer install from a config directory,
  update Qoder/TRAE/WorkBuddy, choose the first candidate, fake percent without
  total bytes, or label Windows wizard handoff as installed evidence.
- **Bad:** install Grok with `@latest`, change the user's global npmrc, or
  claim mainland sign-in/inference because the CLI installed.
- **Bad:** treat GitHub latest failure as OpenCode uninstallable, freeze only
  the NSIS stub path `OpenCode/OpenCode.exe`, require exact Uninstall
  DisplayName equality, or describe Windows OpenCode as supported while
  ProductName/relative EXE/signer stay empty.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:clippy
mise run rust:test
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Assertion points:

- exact wire versions/enums/keys, `deny_unknown_fields`, forbidden-field scan,
  opaque ID grammars and seven catalog IDs;
- lifecycle policy is the sole action/surface owner; disabled actions perform
  no source lookup or side effect;
- inventory merges duplicate provenance but preserves multiple/conflicting/
  incomplete evidence, expires capabilities and rejects drift;
- Qoder/Trae/WorkBuddy/Claude/OpenCode source parsers enforce exact host,
  platform, schema, redirect and version rules without stale URL fallback;
- macOS exact-path deployment, cancellation boundary, running-app protection,
  rollback/recovery and disabled `/Applications` gate;
- Windows registry access masks/views/link handling, trusted PE identity,
  signer leaf, retained artifact, helper protocol/pipe binding, UAC cancel and
  vendor-wizard handoff with no wait/kill/post-install claim;
- OpenCode Windows identity keeps both `@opencode-aidesktop/OpenCode.exe` and
  `OpenCode/OpenCode.exe`, signer `Anomaly Innovations, Inc https://anoma.ly/`,
  and Uninstall DisplayName `{name}` or `{name} {bounded_version}`; OpenCode
  catalog copy must not say 「本机识别和启动暂无法确认」;
- job single-flight, terminal slot release, transfer monotonicity, unknown
  total, cancel refusal after side-effect boundary and unknown job ID;
- Grok owner-preserving lifecycle and ordinary-user helper with no elevated
  fallback;
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

Wrong:

```rust
windows_relative_exes: &["OpenCode/OpenCode.exe"];
if display_name != "OpenCode" { continue; }
```

Correct:

```rust
windows_relative_exes: &[
    "@opencode-aidesktop/OpenCode.exe",
    "OpenCode/OpenCode.exe",
];
uninstall_display_name_matches(display_name, &["OpenCode"])
// exact name, or `OpenCode` + space + bounded_version
```

## Scenario: OpenCode Windows scan after vendor or manual NSIS

### 1. Scope / Trigger

- Trigger: OpenCode Windows x64 is installable through the Agent façade, and
  a later inventory scan must find both in-app handoff and a user-run
  official NSIS. This is a cross-layer readiness/inventory contract: empty
  identity hides Install; a stub-only known-path reports `not_installed`
  after a real current-user install.

### 2. Signatures

```text
windows_exe_install_admitted(opencode)
  -> ProductName/relative EXE nonempty

get_agent_install_readiness({ agentId: opencode })
  -> installState / allowedActions from the same inventory scan

uninstall_display_name_matches(displayName, ["OpenCode"])
  -> exact ASCII-case-insensitive name
     OR name + " " + bounded_version
```

No new wire version. Renderer still sends only `agentId` + action + opaque
capabilities.

### 3. Contracts

- Request: renderer never sends a path, Uninstall key, or signer.
- Response: `installState=installed` and `launch` only after a trusted PE at a
  closed relative or a matching Uninstall/App Paths hint that inspects to the
  same identity.
- Helper product `opencode` admits download/handoff. It does not prove the
  scan relatives. Identity lives in `desktop.rs` / `windows.rs`.
- Environment: Alice `LocalAppData\Programs` plus frozen Uninstall/App Paths.
  ARM64 stays `platform_unsupported`.

### 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Identity fields empty | No Install; catalog may not claim local recognition |
| Installed `@opencode-aidesktop\OpenCode.exe` Valid | `installed` + Launch |
| DisplayName `OpenCode 1.18.27`, InstallLocation empty, DisplayIcon points at that EXE | Keep Uninstall hint; inspect the icon/derived path |
| DisplayName `OpenCode Dev` / `OpenCodeAI` | Skip |
| KnownPath `OpenCode\OpenCode.exe` missing | Drop; do not fail the aggregate |

### 5. Good / Base / Bad Cases

- Good: user uninstalls, runs official NSIS, reopens Agents; card shows
  Launch.
- Base: wizard handoff succeeds; status stays unchanged until the next scan.
- Bad: exact DisplayName equality, or treating the destination label
  `Programs\OpenCode` as the known-path.

### 6. Tests Required

- `opencode_windows_identity_is_frozen_from_winverifytrust_hil`
- `uninstall_display_name_matches_closed_name_or_bounded_version_suffix`
- OpenCode catalog description omits 「本机识别和启动暂无法确认」
- complete empty Windows discovery still exposes OpenCode
  `WindowsCurrentUser`

### 7. Wrong vs Correct

#### Wrong

```text
installer stub path only -> scan miss after electron-builder current-user install
DisplayName == "OpenCode" -> drop `OpenCode 1.18.27`
```

#### Correct

```text
installed-target relative + stub relative
DisplayName exact or `{name} {bounded_version}`
WinVerifyTrust + ProductName + reviewed signer remain admission
```
