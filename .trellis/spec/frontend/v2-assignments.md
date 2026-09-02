# V2 Shared Assignment Contract

## 1. Scope / Trigger

Read this contract before changing the shared assignment target model,
`AssignmentPanel`, Skill/MCP target toggles, optimistic/rollback behavior,
target order, or authoritative assignment reread.

Primary owners:

- `src/v2/shared/features/assignments.ts`
- `src/v2/shared/features/authoritative-assignment.ts`
- `src/v2/shared/components/AssignmentPanel.tsx`
- target-specific ports in `features/skills.ts` and `features/mcp.ts`

Resource semantics remain in [V2 Skills](./v2-skills.md) and
[V2 MCP](./v2-mcp.md). Native target writes are governed by
[Skill Management](../backend/skill-management.md) and
[MCP Management](../backend/mcp-management.md).

## 2. Signatures

The presentation target set is closed and ordered:

```text
qoderwork | trae-work | workbuddy | grokbuild |
codex | claude | opencode
```

An assignment state contains a canonical resource ID, resource kind
(`skill | mcp`), the closed target IDs, enabled/available state and any closed
native reason required to explain an unavailable target.

`AssignmentPanel` receives data and callbacks. It never imports desktop ports,
Tauri `invoke`, database code, vendor paths or resource-specific native DTOs.

Resource adapters expose one target-scoped write that returns authoritative
resource/assignment state. The shared helper coordinates:

```text
previous authoritative state
  -> optional local pending projection
  -> native target mutation
  -> authoritative result/reread
  -> commit or restore previous state
```

## 3. Contracts

### One target model

- Skills and MCP reuse the same target IDs, order, labels, icon identity and
  availability semantics. Do not maintain separate target arrays in pages.
- QoderWork, TRAE Work and WorkBuddy are direct target IDs, not renderer
  conversions to leftover `AppType` values.
- Grok Build uses the catalog identity `grokbuild`; Claude presentation uses
  `claude`. Mapping to native enums occurs in the typed feature/desktop adapter,
  never in `AssignmentPanel`.
- Pi is not an assignment target.

### Authoritative write and rollback

- The database/native live-file result is authoritative. A checked switch is
  not evidence that a vendor document was written or reread.
- At most the selected resource/target mutation is pending. Other controls may
  remain usable only when their operations cannot race the same authoritative
  resource; otherwise disable them explicitly.
- An optimistic visual state is permitted only when the previous complete
  state is captured and every failure/cancel/stale-result path restores it.
- Prefer rendering the native returned resource. If the backend contract
  requires a follow-up query, invalidate/reread before calling the state
  committed.
- Use a monotonic mutation/request identity so a late result for a prior
  toggle cannot overwrite a newer selection.
- Do not retry non-idempotent target writes automatically. A network/query
  library retry policy must not duplicate filesystem/database mutation.

### Availability and evidence

- A target may be visible but unavailable with a closed reason (unsupported,
  app root absent, capability unverified, native write blocked). Disabled state
  must remain distinguishable from unchecked.
- Catalog capability controls feature/navigation eligibility; native target
  adapter/readback controls assignment availability. Do not infer either from
  a local directory in the renderer.
- Successful FyAgent write/readback is described as assigned/configured by
  FyAgent. It is not proof that the vendor app reloaded or accepted the change.

### Component and accessibility boundary

- `AssignmentPanel` owns layout, labels, pending indicators, keyboard behavior
  and non-color status. It does not own resource fetching, notifications,
  confirmation policy or server errors.
- Every switch has an accessible name containing the resource/target context;
  pending and failure states are announced without moving focus unexpectedly.
- Error copy is localized and mapped from closed feature errors. Raw paths,
  config bodies, commands, secrets and backend stack text never render.

## 4. Validation & Error Matrix

| Condition | Required UI result |
| --- | --- |
| Unknown target/resource kind | Strict parser/type failure; no toggle. |
| Target is unavailable | Visible disabled state + closed reason; no mutation. |
| Mutation is already pending for same resource/target | Suppress duplicate request. |
| Native write fails | Restore previous authoritative state and announce error. |
| Native result differs from optimistic projection | Render native result; never keep optimistic value. |
| Follow-up reread fails after write | Show uncertain/stale state and require refresh; do not claim committed. |
| Late response belongs to superseded request | Ignore for current view/cache. |
| Page/resource changes during mutation | Keep cache update scoped by resource ID; clear local pending presentation safely. |
| Target config is written but vendor reload is unverified | Describe FyAgent write only. |
| Secret/path appears in assignment error | Redaction/security regression. |

## 5. Good / Base / Bad Cases

- **Good:** capture Skill state, toggle WorkBuddy through `SkillPorts`, replace
  cache with the returned authoritative Skill, then clear pending state.
- **Good:** MCP write succeeds but follow-up read is unavailable; show an
  uncertain/refresh state rather than leaving an optimistic green switch.
- **Base:** a target is visible but its vendor root is absent; the switch is
  disabled with a reason and the rest of the resource remains usable.
- **Bad:** keep one target list per page, call `invoke` from the panel, retry a
  failed filesystem write automatically, or leave a toggle enabled after
  native rollback.

## 6. Tests Required

```bash
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
```

Required assertions:

- exact seven-target order/mapping, no Pi and no second page-local target list;
- direct QoderWork/TRAE Work/WorkBuddy IDs are not routed through `AppType`;
- success uses returned/reread authoritative state; every error/cancel/stale
  result restores or invalidates correctly;
- duplicate/late mutations cannot overwrite current resource state;
- disabled, unchecked, checked and pending states are distinguishable;
- Skill and MCP both render through the shared panel/helper while keeping their
  own ports and error mappings;
- keyboard, accessible names, focus persistence and status announcements;
- raw secrets/paths/config values never enter rendered errors or snapshots.

## 7. Wrong vs Correct

Wrong:

```tsx
setEnabled(targetId, next);
await invoke("toggle_skill_app", { id, targetId, enabled: next });
// Failure leaves optimistic state behind.
```

Correct:

```tsx
await applyAuthoritativeAssignment({
  previous: skill,
  resourceId: skill.id,
  targetId,
  nextEnabled,
  mutate: () => ports.skills.setAssignment(skill.id, targetId, nextEnabled),
  commit: (authoritative) => setSkill(authoritative),
  restore: (previous) => setSkill(previous),
});
```

Use the actual helper/port names exported by the source; the invariant is that
the native returned/readback value, not the switch click, is authority.
