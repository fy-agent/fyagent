# Renderer Agent Directory and Lifecycle UI Contract

## 1. Scope / Trigger

Read this contract before changing the Renderer Agents route, static Agent catalog
projection, local runtime scan, install readiness/inventory cards, target
selection, install/update/launch interaction, job polling, or feature
navigation from an Agent detail page.

Primary owners:

- `src/pages/agents/**`
- `src/shared/features/agents.ts`
- `src/shared/features/agent-install-readiness.ts`
- `src/shared/features/agent-lifecycle-capabilities.ts`
- `src/shared/features/ports.ts`

Native authority is split between
[Agent Catalog and Runtime](../backend/external-agent-catalog-runtime.md) and
[Agent Lifecycle](../backend/external-agent-lifecycle.md). Auth UI has its own
owner: [Renderer Agent Auth](./agent-auth.md).

## 2. Signatures

Routes:

```text
/agents
/agents?target=<AgentCatalogId>&section=<AgentSection>
```

The page accepts only the seven closed catalog IDs:

```text
qoderwork | trae-work | workbuddy | grokbuild |
codex | claude-code | opencode
```

The platform adapter returns and strictly parses:

```text
Agent catalog          contractVersion 5
Install readiness      contractVersion 4
Installation inventory contractVersion 1
Action/job snapshots   contractVersion 4
Runtime status         tri-state detected/running plus sanitized metadata
```

All native access goes through the typed ports assembled in
`src/shared/platform/tauri/features.ts`, including the focused adapters in
`src/shared/platform/tauri/feature-ports/agents.ts` and
`agentInstallReadiness.ts`. Components do not call `invoke()` or mock native
behavior directly.

The renderer sends only closed values:

```text
agentId
destination = home | skills | hooks | models | mcp
action      = install | update | launch
surface?    = cli | desktop
opaque release/target capability fields returned by native inventory
```

It never sends a URL, path, executable, command, package, hash, token, signer
or bypass flag.

## 3. Contracts

### Catalog and route authority

- Render products, names, links, capability IDs and order from the parsed
  native catalog. Do not merge local storage, a hard-coded second list or a
  legacy fallback catalog.
- An unknown `target` is cleared with replace navigation and shows the normal
  directory. A known target without a valid section normalizes to `models`.
  An unknown pathname falls through the router's `/agents` redirect; there is
  no `/agents/:agentId` route. Directory catalog/runtime reads can still run,
  but an invalid target never supplies native action authority.
- Catalog parse is all-or-nothing. Wrong version/order, duplicate IDs,
  unknown/excess keys, invalid capability mode/reason/evidence, or official
  link ID/order drift against the native v5 table does not degrade to
  partially trusted cards.
- Official link IDs for the current catalog are: QoderWork/TRAE Work/WorkBuddy/
  Grok Build `product`; Codex empty; Claude Code `product` (CLI setup, not
  Desktop); OpenCode `product` then `desktop`. The platform parser owns this
  allowlist; page copy must not invent a second link table.
- Pi is not an Agent product. It must not appear in types, filters, fixtures,
  navigation or empty states.
- Official links are rendered only from the validated catalog and open through
  the reviewed external-link adapter. Renderer text or query parameters never
  become launch authority.

### Runtime and readiness projection

- `useAgentDirectoryScan` distinguishes hidden from unmounted owners. Hidden
  mounted pages buffer settled rows and reconcile on return; unmounted owners
  ignore late success, rejection and aggregate completion without recording a
  completion timestamp or dispatching UI state. Retained start/readback callbacks
  do not restart or update a disposed view. This does not cancel native jobs.
- The synchronous scan admission ref keeps its newer `requestId` when StrictMode
  replays an effect from an older render. One pending scan must not issue a second
  set of readiness requests during effect replay. The existing scan hook tests
  cover delayed success/rejection after unmount, retained callbacks, StrictMode
  and hidden-result reconciliation.
- Runtime `detected`/`running` preserve `true | false | null`. Unknown is
  rendered as unknown/unverified, not “not installed.”
- Readiness and inventory are separate queries keyed by canonical Agent ID and
  optional legal surface. Runtime scan must not overwrite catalog capability
  review or synthesize installation state from configuration directories.
- Renderer `surfacesForAgent` / readiness `sourceKind` must match
  `lifecycle_policy.rs`. Compact CLI products currently are:

```text
grokbuild    surface=cli  sourceKind=cli_tooling
claude-code  surface=cli  sourceKind=cli_tooling
```

  Desktop products use `surface=desktop` and `managed_desktop` except Codex
  (`codex_desktop` plus `fyagent_managed`). Compact single-surface payloads
  omit `surfaces`. A legal CLI `not_installed` + `install` payload must parse;
  requiring `managed_desktop` or treating `cli` as illegal for Claude Code
  fails the directory scan as 「读取失败」 instead of showing install.
- Inventory states remain exact: `not_observed`, `single`, `multiple`,
  `unsupported`, `unknown`. Multiple candidates show a selection surface and
  never choose the first item automatically.
- Opaque `releaseId`, `inventoryId`, `targetId` and revision values are treated
  as uninterpreted strings. The UI may retain them only for the current
  interaction/query lifetime; it must not parse paths from them or persist them
  as durable target preferences.
- The page distinguishes installed, update available, latest unknown,
  unsupported, source unverified and inventory unknown. One generic
  green/red badge is insufficient.

### Action admission and job state

- Action buttons derive from native readiness `allowedActions` plus the shared
  lifecycle capability projection; the UI does not maintain a product/action
  matrix.
- Install/update/launch forwards the selected native capabilities exactly. If
  target selection or a fresh release is required and absent, disable/guide the
  action rather than inventing defaults.
- One action mutation is active per current Agent view. Native
  `operation_conflict` remains authoritative if another page/window/job is
  active. After that conflict, keep the last requested action for Retry even
  if a later readiness reread omits it from `allowedActions`. Generic and
  Codex directory slots show Retry before a scanning-only status so a
  recoverable conflict is not hidden behind 「正在扫描」.
- A returned terminal action result is rendered immediately. A background job
  is polled through `get_agent_action_job` until its native terminal stage.
- The renderer may stop polling when the route unmounts, but it must not paint
  an arbitrary poll cap as job failure while the native stage is still active.
  Reopening the same Agent may query the known job/session owner where the
  feature supports recovery.
- Display raw transfer totals only when native provides them. Percent/speed are
  derived renderer values; unknown `totalBytes` does not become 100% or zero.
- Cancellation is offered only while native reports `cancellable=true`.
  `operation_conflict` after a side-effect boundary is not presented as a
  successful cancel.

### Product and feature navigation

The directory owns install/update/launch controls. `AgentConfiguration` does
not mount a second `AgentInstallReadinessSection` or Codex installer; it keeps
the selected Models/Skills/MCP/Prompts section and authentication handoff.
Managed consumers show one compact status plus the central Auth entry, not
stacked duplicate descriptions. Back returns to the existing directory and its
installation controls. Configuration navigation never starts an installation.
When inventory reports more than one eligible install destination, the directory
card opens a shared `Dialog` from the 「选择安装目标」 control (origin animation
returns to that control) and reuses `LifecycleTargetPicker` with opaque
`targetId` values. A `locationLabel` that starts with `/Applications` shows a
small 「推荐」 mark; confirmation still requires an explicit dialog confirm.
Confirming a destination starts the native action and immediately dismisses the
dialog back to the originating control so the card can show transfer progress.
Do not keep the picker open until the job finishes, and do not unmount the
return anchor when the slot switches to busy status.
Do not send the user to the Models section to pick a filesystem destination.

- “Open product” calls the closed native launch destination; it never sends a
  path/URL or shells out from the renderer.
- Skills, Hooks, Models and MCP navigation appears only when the parsed catalog
  capability and current route contract permit it. Do not infer support from a
  local file, installed state or feature page existence.
- Codex install/update continues to route through the dedicated Codex Desktop
  owner when native returns `managed_by_codex_desktop`; the Agent page does not
  duplicate the installer.
- Auth uses the shared Auth panel/hook and remains separate from lifecycle
  actions. Do not put `login` or `logout` into `start_agent_action`.
- Claude Code is CLI-only: its Agent lifecycle actions reuse Tooling and the
  Agent job observer. Backend readiness/inventory determines install/update;
  the page does not offer Claude Desktop or infer CLI presence from an app
  bundle. The install-readiness parser and `surfacesForAgent` must admit
  `cli` / `cli_tooling` like Grok Build. Install chrome must not title the
  component 「Claude Desktop」. Missing Node/npm or an unconfirmed
  installation owner produces the actionable closed reason; npm success
  alone is not installation proof.
- Grok CLI install/update stays on the Tooling owner. The Agent Grok panel
  must not send a registry, version, hash, or npm command. Default one-click
  install is official npm; official CLI is an explicit secondary control.
  Native-owned installs may offer “改用官方 npm 方式”; that must not auto-run.
  CLI latest/update availability comes from native `latest_version` /
  `allowedActions`. The renderer never embeds a reviewed npm version.
- Windows vendor-wizard success uses
  `官方安装窗口已打开。完成安装后请刷新安装状态。` It must not say the product
  is installed. OpenCode Windows ARM64 remains unavailable.
- OpenCode catalog description must not say 「本机识别和启动暂无法确认」
  once Windows identity is admitted. After a complete native scan with a
  trusted candidate, including a user-run official NSIS, show Launch from
  `allowedActions`. Do not keep a previous `not_installed` card because the
  user did not install through FyAgent.

### State, errors and accessibility

- Query cache owns server/native observations; component state owns only the
  current selection, confirmation and transient presentation.
- Mutation success invalidates/rereads readiness, inventory, runtime and job
  keys needed for the visible Agent. Never keep an optimistic “installed”
  state after a vendor handoff.
- Closed reason codes map to localized, evidence-strength-correct messages.
  Raw backend errors, paths, registry details, signer data or command lines do
  not render.
- Product cards, target rows, action controls, progress and error alerts remain
  keyboard reachable with visible focus, semantic labels and non-color status.

## 4. Validation & Error Matrix

| Condition                                                           | Required UI result                                                                                     |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Catalog/version/order/parser failure                                | Fail the catalog boundary visibly; do not render a partial/legacy catalog.                             |
| Unknown Agent target                                                | Replace invalid search with the directory; never issue an action for the unknown ID.                   |
| Runtime value is `null`                                             | Render unknown/unverified, not absent/stopped.                                                         |
| Inventory is `multiple`                                             | Require explicit target selection; no implicit first candidate.                                        |
| Inventory is `unknown`/expired or target drifts                     | Refresh guidance; no action retry with stale capability.                                               |
| Action is absent from `allowedActions`                              | Hide/disable with closed reason; do not call native except Retry of the last `operation_conflict` action. |
| Native returns `operation_conflict`                                 | Preserve native job/other-operation state; keep last action for Retry; do not create a local parallel action. |
| Background job remains active after UI poll budget                  | Stop/slow UI polling as designed, but do not mark failed.                                              |
| Cancel is no longer permitted                                       | Disable cancel and preserve active/terminal state.                                                     |
| Windows vendor wizard handoff succeeds but inventory remains absent | Explain handoff/completion scope; do not paint installed.                                              |
| Native DTO contains unknown/excess/forbidden field                  | Strict parser failure; never spread raw object into UI.                                                |
| Inventory is `multiple` and the user confirms a destination         | Start the native action and immediately dismiss the picker back to the originating control; the card shows job progress. Do not keep 「安装中…」 on the dialog until the job finishes. |
| Claude/Grok compact CLI readiness uses `cli_tooling`                | Parse and project install/update; do not fail the directory scan.                                      |
| Renderer embeds a reviewed Claude/Grok npm version                 | Contract regression; show native `latest_version` only.                                                 |
| Claude/Grok readiness uses `managed_desktop` or `desktop` surface   | Fail closed at the parser; do not render a Desktop install card.                                       |
| Route changes/unmounts                                              | Clear transient selection/confirmation; do not cancel native work unless user explicitly requested it. |

## 5. Good / Base / Bad Cases

- **Good:** catalog v5 drives the seven cards; inventory reports two apps; the
  user confirms a destination in the directory picker; the dialog returns to
  the originating control and the card shows transfer progress while the job
  runs.
- **Good:** a job has unknown total bytes; the UI shows stage and completed
  bytes without fabricated percentage.
- **Base:** a Windows vendor installer was opened. The UI reports vendor handoff
  and offers a later rescan rather than claiming installed state.
- **Base:** native observation is unavailable on the current host; the card
  remains useful with official guidance and unverified labels.
- **Bad:** hard-code product order/actions, infer installation from settings,
  store a target ID in local storage, send a URL/path, select the first target,
  or treat a browser/app/installer handoff as verified completion.
- **Bad:** treat Claude Code as `managed_desktop` / `desktop` in the
  renderer parser, or label its install chrome 「Claude Desktop」. The native
  compact payload is `sourceKind=cli_tooling` with no `surfaces` array; a
  kind mismatch throws and the directory shows 「读取失败」 while the host
  still reports `not_installed` plus `install`.

## 6. Tests Required

```bash
mise run typecheck
mise run lint
mise run test:unit
mise run test:browser
```

Required assertions:

- exact seven-product catalog/version/order/capability parsing and no Pi;
- official-link ID allowlist matches native v5 (Claude `product`, not Desktop);
- Claude Code and Grok Build share CLI `surfacesForAgent` and `cli_tooling`
  sourceKind; compact Claude `not_installed` + `install` parses; Claude
  `managed_desktop` / `desktop` and CLI `launch` fail closed;
- unknown/excess/duplicate/future/legacy catalog values fail closed;
- runtime tri-state and every readiness/inventory/action/job enum render
  evidence-correct states;
- multiple target selection, opaque capability forwarding, expiry/drift refresh
  and no path/URL/command fields;
- directory 「选择安装目标」 opens a Dialog picker instead of navigating to
  configuration; `/Applications` labels render 「推荐」; confirming a
  destination dismisses that dialog immediately and the card shows
  「正在检查来源」 (or later transfer copy) without waiting for job terminal;
- allowed-actions projection, Codex owner routing, Auth/lifecycle separation and
  feature-navigation capability checks;
- Retry remains after `operation_conflict` even when reread omits the action,
  and directory slots show Retry before scanning-only status;
- polling survives active native jobs without synthetic failure, transfer
  totals remain raw, and cancel respects `cancellable`;
- success/error invalidation/reread clears stale installed/update state;
- Grok default install stays on official npm; native/owner-switch controls
  are explicit and do not auto-run; renderer never sends registry/version/hash;
- Windows vendor-wizard success copy reports an opened installer, not an
  installed product; OpenCode Windows ARM64 remains unavailable;
- OpenCode catalog copy omits 「本机识别和启动暂无法确认」; a later inventory
  that reports `installed` must render Launch after a manual official install;
- keyboard/focus/label/status semantics and localized closed-reason copy;
- browser suites use fixtures only for layout/interaction and keep native HIL
  claims separate.

## 7. Wrong vs Correct

Wrong:

```tsx
const installed = exists(`~/.${agent.id}`);
const target = inventory.candidates[0];
await invoke("start_agent_action", {
  agentId: agent.id,
  action: "launch",
  path: target.path,
});
```

Correct:

```tsx
const catalog = useAgentCatalog();
const readiness = useAgentReadiness(agentId, surface);
const inventory = useAgentInventory(agentId, surface);

await ports.agentInstallReadiness.startAction({
  agentId,
  action: "launch",
  inventoryId: inventory.data?.inventoryId,
  targetId: selectedTarget?.targetId,
  expectedTargetRevision: selectedTarget?.expectedTargetRevision,
});
```

Wrong:

```ts
surfacesForAgent("claude-code") === ["desktop"]
parseAgentInstallReadiness requires sourceKind === "managed_desktop"
```

Correct:

```ts
surfacesForAgent("grokbuild") === ["cli"]
surfacesForAgent("claude-code") === ["cli"]
parseAgentInstallReadiness admits sourceKind === "cli_tooling"
```

Wrong:

```ts
const CLAUDE_REVIEWED_VERSION = "2.1.261";
```

Correct:

```ts
readiness.localVersion; // native latest_version / allowedActions decide update
```

Native owns identity, legality and side effects; the page owns strict
projection, explicit user selection and evidence-correct wording.

Wrong:

```tsx
void lifecycle.run(action, selectedTarget).then(() => setPickingTarget(null));
if (lifecycle.busy) return { kind: "status", label };
```

Correct:

```tsx
setPickingTarget(null);
void lifecycle.run(action, selectedTarget);
// AgentLifecycleActionSlot keeps a connected host for dialogReturnRef
// while the inner control swaps from 「选择安装目标」 to busy status.
```
