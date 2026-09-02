# V2 Shared Assignment Contract

## 1. Scope / Trigger

Read this contract before changing the V2 target order, assignment DTO
defaults, shared switch/radio UI, pending-state behavior, serialized mutation,
or authoritative reread used by Agent, Skill, and MCP views.

Primary owners are:

- `src/v2/shared/features/directory.ts` for the closed seven-target IDs/order;
- `src/v2/shared/features/assignments.ts` for Skill/MCP assignment projection;
- `src/v2/shared/features/authoritative-assignment.ts` for serialized mutation
  and readback confirmation;
- `src/v2/shared/ui/AssignmentPanel.tsx` for shared switch/radio rendering;
- `src/v2/shared/features/ports.ts` for `SkillsPort` and `McpPort`;
- `src/v2/pages/agents/AgentAssignmentSections.tsx` for the current
  target-bound Skill/MCP mutation composition.

Feature contracts remain in [V2 Skills](./v2-skills.md) and
[V2 MCP](./v2-mcp.md). Native ordering and cross-resource failure semantics
remain in [Skill Management](../backend/skill-management.md) and
[MCP Management](../backend/mcp-management.md).

## 2. Signatures

The V2 presentation target set and order are closed:

```text
qoderwork | trae-work | workbuddy | grokbuild | codex | claude | opencode
```

`SKILL_TARGET_IDS`, `MCP_TARGET_IDS`, and `SUPPORTED_APP_IDS` share that order.
`createMcpAssignments()` returns exactly those seven booleans.
`createSkillAssignments()` returns those seven plus compatibility-only
`claude-desktop=false` and `openclaw=false`; the extra fields are not V2
rendered targets.

The shared panel has two explicit modes:

```ts
type AssignmentPanelProps =
  | {
      mode: "switch";
      targets: readonly { id: AssignmentTargetId; label: string }[];
      apps: Record<string, boolean>;
      onToggle(id: AssignmentTargetId, enabled: boolean): void;
      disabled?: boolean;
      labelSuffix?: (id: AssignmentTargetId) => string | undefined;
    }
  | {
      mode: "radio";
      targets: readonly { id: AssignmentTargetId; label: string }[];
      value: AssignmentTargetId;
      onChange(id: AssignmentTargetId): void;
      disabled?: boolean;
      ariaLabel: string;
    };
```

The mutation owner is:

```ts
useAuthoritativeAssignmentMutation<TItemId, TSnapshot>({
  mutate: (itemId, enabled) => Promise<boolean | void>,
  reread: () => Promise<{ data: TSnapshot | undefined; error: unknown }>,
  readValue: (snapshot, itemId) => boolean | undefined,
}) -> {
  busy: boolean;
  pendingId: TItemId | null;
  run(itemId, enabled): Promise<
    | { status: "confirmed" }
    | { status: "rejected" }
    | { status: "busy" }
  >;
}
```

Resource mutations remain domain-specific:

```text
SkillsPort.toggleApp(skillId, targetId, enabled) -> boolean
McpPort.toggleApp(serverId, targetId, enabled) -> void
```

## 3. Contracts

### Target projection and rendering

- The target registry in `directory.ts` owns V2 order and labels. Feature pages
  do not maintain independent arrays or reorder targets from observed data.
- The target unions/tuples provide a compile-time closed set for normal V2
  call sites. `createSimpleFeaturePorts` is a thin typed adapter and does not
  runtime-parse a target before IPC; a value forced through the TypeScript
  boundary is forwarded and the native Rust enum remains the runtime rejector.
  If a future URL, JSON, storage, or plugin value can supply a target, parse it
  before invoking the Port rather than assuming the TypeScript annotation is a
  runtime guard.
- Native DTOs can round-trip nine target flags, while V2 deliberately displays
  seven. Gemini/Hermes or compatibility fields must not appear merely because
  they exist in a native row.
- `AssignmentPanel` renders semantic checkboxes for switch mode and one
  radiogroup for radio mode. Labels are visible, inputs remain accessible, and
  the whole panel honors its `disabled` prop.
- The shared component has no native-path, installation, persistence, or
  per-target capability logic. Feature pages decide whether it is available and
  supply any `labelSuffix` evidence.

### Serialized authoritative mutation

- One hook instance permits only one pending item. `pendingRef` rejects a
  concurrent `run` with `{status:"busy"}` before calling the native mutation.
- A mutation is confirmed only when `mutate` does not return `false`, `reread`
  has no error, and `readValue(reread.data, itemId)` exactly equals the requested
  boolean.
- A thrown mutation, explicit `false`, reread error, missing value, or readback
  mismatch returns `{status:"rejected"}`. The helper attempts one additional
  best-effort reread in the failure path so the feature can converge to current
  authority.
- The hook exposes only `pendingId`/`busy`; it does not apply an optimistic
  assignment value or claim rollback. Feature state must be replaced by the
  reread snapshot, not by the click intent.
- `pendingId` is cleared in `finally` for every confirmed, rejected, or thrown
  path. A rejected result may coexist with a changed durable/native value when
  the backend operation crossed a non-atomic boundary; the feature must show
  the reread state plus an error/retry affordance.

### Current feature bindings

- `AgentSkillsSection` binds `entry.assignmentId` as the target, uses the Skill
  ID as the pending item, calls `ports.skills.toggleApp`, refetches installed
  Skills, and reads that Skill's `apps[target]` flag.
- `AgentMcpSection` binds the same closed Agent assignment target, uses the MCP
  server ID as the pending item, calls `ports.mcp.toggleApp`, refetches the MCP
  map, and reads `server.apps[target]`.
- Management pages may use `AssignmentPanel` for a resource-wide target matrix,
  but the panel remains presentation-only. Real mutations still require the
  domain Port and authoritative reread behavior documented here or in the
  owning feature contract.
- Success/error copy distinguishes `busy`, rejected mutation/readback, and
  confirmed persistence. A checked switch alone is not evidence that a vendor
  process loaded the Skill/MCP server.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Typed V2 code needs a target outside the seven-ID set | Reject the design; update the shared registry/contract deliberately rather than adding an ad hoc row. |
| An invalid target is forced into the thin simple Port at runtime | The adapter currently forwards it; native enum deserialization must reject it before mutation. Add a renderer parser first if an untrusted runtime source is introduced. |
| A second `run` starts while one item is pending | Return `busy`; do not call `mutate` or `reread`. |
| `mutate` returns `false` | Treat as rejected, perform the failure-path reread, and clear pending state. |
| `mutate` throws | Treat as rejected, attempt failure-path reread, and clear pending state. |
| First reread returns an error | Reject and attempt one best-effort reread. |
| First reread lacks the item/target or value differs | Reject and attempt one best-effort reread; do not force the requested value. |
| Mutation/readback matches | Return confirmed and render the reread snapshot. |
| Failure reread also throws | Keep the original rejected outcome and clear pending state. |
| Native operation errors after a partial side effect | Render current reread authority plus error/retry; never claim atomic rollback. |
| Panel is disabled | All switch/radio inputs are disabled and no feature mutation starts. |

## 5. Good / Base / Bad Cases

- **Good:** toggle one Skill for the current Agent target, wait for the native
  boolean result, reread installed Skills, confirm only when the target flag
  matches, then clear the pending row.
- **Good:** an MCP enable returns an error after its durable flag changed; the
  failure reread shows current durable state while the page still reports the
  projection failure and offers repair/retry.
- **Base:** a radio panel changes local install-target selection only; it uses
  the shared semantic radio UI but does not invoke the authoritative mutation
  helper until a real native write occurs.
- **Base:** a compatibility Skill snapshot contains `openclaw`; V2 keeps it out
  of the seven displayed targets.
- **Bad:** keep separate target arrays in Skills and MCP, optimistically write
  `apps[target]=enabled`, allow two overlapping writes, or treat a success toast
  as proof that the vendor loaded the assignment.

## 6. Tests Required

Required assertion owners include:

- `tests/v2/features/authoritativeAssignment.test.tsx`: one pending mutation,
  concurrent `busy`, confirmation only after exact readback, two rereads on a
  mismatch, rejected outcome, and pending cleanup;
- shared assignment/component tests: exact seven-target order, compatibility
  fields excluded from rendering, checkbox/radiogroup semantics, disabled
  behavior, and accessible labels;
- Agent Skill/MCP section tests: domain Port wiring, fixed assignment target,
  stable resource IDs, feature-specific reread/readValue, warning copy, and no
  direct native invocation;
- V2 platform tests: exact valid target-ID transport; native command tests:
  rejection of unknown IDs before mutation. Add renderer-parser rejection tests
  only when such a runtime parser actually becomes an owner;
- backend tests named by the linked Skill/MCP contracts: true write ordering,
  failure/partial-state behavior, and all nine native flags round-trip.

## 7. Wrong vs Correct

Wrong:

```ts
setAssignments((current) => ({ ...current, [targetId]: enabled }));
await ports.skills.toggleApp(skillId, targetId, enabled);
showSuccess();
```

Correct:

```ts
const assignment = useAuthoritativeAssignmentMutation({
  mutate: (skillId, enabled) =>
    ports.skills.toggleApp(skillId, entry.assignmentId, enabled),
  reread: async () => {
    const readback = await installedSkillsQuery.refetch();
    return { data: readback.data, error: readback.error };
  },
  readValue: (skills, skillId) =>
    Boolean(
      skills?.find((skill) => skill.id === skillId)?.apps[
        entry.assignmentId
      ],
    ),
});

const outcome = await assignment.run(skillId, enabled);
// Render query data. `confirmed` requires matching reread; rejected never
// manufactures rollback or success.
```
