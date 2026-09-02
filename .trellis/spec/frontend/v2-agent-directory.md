# V2 Agent Directory and Lifecycle UI Contract

## 1. Scope / Trigger

Read this contract before changing the V2 Agents route, static Agent catalog
projection, local runtime scan, install readiness/inventory cards, target
selection, install/update/launch interaction, job polling, or feature
navigation from an Agent detail page.

Primary owners:

- `src/v2/pages/agents/**`
- `src/v2/shared/features/agents.ts`
- `src/v2/shared/features/agent-install-readiness.ts`
- `src/v2/shared/features/agent-lifecycle-capabilities.ts`
- `src/v2/shared/features/ports.ts`

Native authority is split between
[Agent Catalog and Runtime](../backend/external-agent-catalog-runtime.md) and
[Agent Lifecycle](../backend/external-agent-lifecycle.md). Auth UI has its own
owner: [V2 Agent Auth](./v2-agent-auth.md).

## 2. Signatures

Routes:

```text
/agents
/agents/:agentId
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
`src/v2/shared/platform/tauri/features.ts`, including the focused adapters in
`src/v2/shared/platform/tauri/feature-ports/agents.ts` and
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
- An unknown route ID renders the existing unavailable/not-found state and
  starts no native scan or action.
- Catalog parse is all-or-nothing. Wrong version/order, duplicate IDs,
  unknown/excess keys or invalid capability mode/reason/evidence does not
  degrade to partially trusted cards.
- Pi is not an Agent product. It must not appear in types, filters, fixtures,
  navigation or empty states.
- Official links are rendered only from the validated catalog and open through
  the reviewed external-link adapter. Renderer text or query parameters never
  become launch authority.

### Runtime and readiness projection

- Runtime `detected`/`running` preserve `true | false | null`. Unknown is
  rendered as unknown/unverified, not “not installed.”
- Readiness and inventory are separate queries keyed by canonical Agent ID and
  optional legal surface. Runtime scan must not overwrite catalog capability
  review or synthesize installation state from configuration directories.
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
  active.
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

| Condition | Required UI result |
| --- | --- |
| Catalog/version/order/parser failure | Fail the catalog boundary visibly; do not render a partial/legacy catalog. |
| Unknown Agent route | Render unavailable/not-found; issue no native action. |
| Runtime value is `null` | Render unknown/unverified, not absent/stopped. |
| Inventory is `multiple` | Require explicit target selection; no implicit first candidate. |
| Inventory is `unknown`/expired or target drifts | Refresh guidance; no action retry with stale capability. |
| Action is absent from `allowedActions` | Hide/disable with closed reason; do not call native. |
| Native returns `operation_conflict` | Preserve native job/other-operation state; do not create a local parallel action. |
| Background job remains active after UI poll budget | Stop/slow UI polling as designed, but do not mark failed. |
| Cancel is no longer permitted | Disable cancel and preserve active/terminal state. |
| Windows vendor wizard handoff succeeds but inventory remains absent | Explain handoff/completion scope; do not paint installed. |
| Native DTO contains unknown/excess/forbidden field | Strict parser failure; never spread raw object into UI. |
| Route changes/unmounts | Clear transient selection/confirmation; do not cancel native work unless user explicitly requested it. |

## 5. Good / Base / Bad Cases

- **Good:** catalog v5 drives the seven cards; inventory reports two apps; the
  user selects one opaque target; native revalidates and launches it.
- **Good:** a job has unknown total bytes; the UI shows stage and completed
  bytes without fabricated percentage.
- **Base:** a Windows vendor installer was opened. The UI reports vendor handoff
  and offers a later rescan rather than claiming installed state.
- **Base:** native observation is unavailable on the current host; the card
  remains useful with official guidance and unverified labels.
- **Bad:** hard-code product order/actions, infer installation from settings,
  store a target ID in local storage, send a URL/path, select the first target,
  or treat a browser/app/installer handoff as verified completion.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run test:unit
```

Required assertions:

- exact seven-product catalog/version/order/capability parsing and no Pi;
- unknown/excess/duplicate/future/legacy catalog values fail closed;
- runtime tri-state and every readiness/inventory/action/job enum render
  evidence-correct states;
- multiple target selection, opaque capability forwarding, expiry/drift refresh
  and no path/URL/command fields;
- allowed-actions projection, Codex owner routing, Auth/lifecycle separation and
  feature-navigation capability checks;
- polling survives active native jobs without synthetic failure, transfer
  totals remain raw, and cancel respects `cancellable`;
- success/error invalidation/reread clears stale installed/update state;
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

Native owns identity, legality and side effects; the page owns strict
projection, explicit user selection and evidence-correct wording.
