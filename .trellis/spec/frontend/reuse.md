# Frontend Reuse Contract

## 1. Scope / Trigger

Read this contract before adding a renderer component, hook, feature helper,
platform adapter, dependency, page-local UI pattern, or repeated state/DTO
logic. Reuse means preserving one semantic owner and adapting at explicit
boundaries; it does not mean bypassing V2/leftover, renderer/native, secret, or
platform separation.

Binding placement and import rules also live in
[Directory Structure](./directory-structure.md) and
[Renderer Modular Boundaries](./modular-boundaries.md). Backend reuse is owned
by [Backend Reuse](../backend/reuse.md). The short preparation checklist is
[Code Reuse Thinking Guide](../guides/code-reuse-thinking-guide.md).

## 2. Owner and placement signatures

Use this order:

```text
existing FyAgent owner
  -> already-adopted framework/dependency primitive
  -> reviewed maintained open-source primitive
  -> one shared FyAgent adapter/composition
  -> justified local implementation
```

V2 placement roles are:

| Location                                                          | Owner                                                                                                                   |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `src/v2/shared/ui/**`                                             | Reusable visual/interaction primitives with no page business authority.                                                 |
| `src/v2/shared/features/**`                                       | Shared feature types, ports, query keys/hooks, projections, and reusable feature workflows.                             |
| `src/v2/shared/platform/**`                                       | Browser/Tauri adapters and `unknown` wire parsing; only approved Tauri adapters import `@tauri-apps/**`.                |
| `src/v2/widgets/**`                                               | App-shell or multi-page composition whose owner is above one route.                                                     |
| `src/v2/pages/<route>/**`                                         | Route composition and genuinely route-specific presentation.                                                            |
| `src/shared/**`                                                   | Explicit renderer-neutral bridge approved by an owning feature/backend contract.                                        |
| Leftover `src/components/common/**`, `src/hooks/**`, `src/lib/**` | Reuse within leftover V1 only, following [Directory Structure](./directory-structure.md); never a direct V2 dependency. |

Leftover work reuses the existing leftover owner rather than creating another
local copy. That tree may provide behavior evidence for V2, but it is not a V2
component library. V2 must not import leftover components, hooks, state, i18n,
or Tauri façades unless an owning contract names a narrow bridge.

Current shared owner families include:

Feature-aware controls (`ExternalLinkButton`, `CopyablePath`,
`InstallTargetDialog`) live in `shared/features/controls`, not `shared/ui`.
They may consume feature context and compose pure visual primitives. Pure
`shared/ui` never imports back through the feature or platform layer; the
dependency-cruiser gate enforces this direction. Do not re-export these controls
through a UI barrel to conceal that dependency.

- feature controls and lists: `FeatureTabs`, `FeatureSearch`, `FeatureList`,
  `FeaturePagination`;
- assignment/install flows: `AssignmentPanel`, `InstallTargetDialog`, shared
  confirmation/dialog primitives;
- download/progress projection: Codex + Agent job transfer share
  `projectTransferPresentation` in `src/shared/codex-desktop/snapshots.ts`
  (Agent adapter: `src/v2/shared/features/transfer-projection.ts`);
- layouts: `SplitPanes`, `CatalogMasterDetail`, feature page/panel chrome;
- external and secret controls: `ExternalLinkButton`, `SecretInput`;
- shell motion/selection primitives owned under `shared/ui`;
- visited-route visibility: `PersistentSurface`, `usePersistentSearchParams`,
  `useStickyVisibleValue`;
- Agent directory lifecycle chrome: `AgentLifecycleActionSlot` plus closed
  `AGENT_DIRECTORY_UPDATE_UI` in `agent-lifecycle-capabilities.ts`;
- primary-route module table: `prefetchPrimaryRoutes` / `primaryPages` in
  `app/primaryPages.tsx`.

Their exact behavior belongs in the feature/shell specs that use them:
[V2 Shell](./v2-shell.md),
[V2 Agent and Models](./v2-agent-models.md),
[V2 Skills and MCP](./v2-skills-mcp.md), and
[V2 Prompts and Memory](./v2-prompts-memory.md). This list is an owner map, not
a duplicate API contract.

## 3. Contracts

### Search and extend before creating

- Search source, tests, types, styles, query keys, ports, and manifests for the
  semantic capability before choosing a new owner.
- Prefer a small compatible extension to the existing owner over a page-local
  near-copy. Keep optional props/variants closed and meaningful; do not turn a
  shared primitive into a page switchboard.
- Reuse an adopted dependency through the existing project boundary. Do not
  import a package directly from every page when one adapter can contain it.

### Review new dependencies

Before adding a package, verify from primary sources that the reviewed version
supports the requirement and has acceptable license, maintenance, provenance,
advisories, browser/Tauri support, bundle/build cost, and transitive footprint.
A component package must fit the existing token, accessibility, state, and
platform architecture rather than introduce a second UI/state framework.

### Promote a concrete shared owner early

- Put a new capability in `shared/**` on the first implementation when an
  existing or concrete near-term second consumer is known.
- Keep a one-off page detail local when sharing would require speculative
  options or leak page business rules.
- A second consumer should import the same owner; copying and “cleaning up
  later” is not an accepted staging strategy.
- Shared does not mean globally public. Export the minimum surface and keep
  feature/platform internals private to their owner.

### Keep business and wire authority out of visual primitives

- Shared UI receives typed values and callbacks. It does not invoke Tauri,
  construct native paths/URLs/commands, own query caches, or infer capability.
- Decode Tauri/events/config once at the owning platform/feature boundary.
  Pages must not repeat `as` casts or private wire parsers.
- Server state remains authoritative. Shared interaction components may expose
  pending/selection state but must not manufacture a successful write.
- Secrets remain in the narrow component/mutation lifetime defined by the
  feature contract and never become a convenience shared store.

### Settings CLI lifecycle owner

- Leftover Settings (`AboutSection` / `ToolInstallRow`) must not keep a
  page-local npm/Shell/PowerShell command table. Writable lifecycle buttons
  exist only for Grok Build and call the existing Tooling action port.
- Do not duplicate Grok install/update in a second Agent CLI card. Desktop
  products stay on the Agent directory owner.

### Preserve dependency direction

```text
pages/widgets -> shared/features + shared/ui
shared/features -> shared/platform + shared/ui
shared/platform -> native/browser APIs
shared/ui -> tokens and UI-only helpers
```

Lower layers must not import pages/widgets. A reuse attempt that creates a
cycle or imports business authority into `shared/ui` is a boundary violation,
not successful deduplication.

### Avoid fake abstraction

Do not extract a wrapper that only renames one line, combine unrelated feature
rules because their markup looks similar, or add a universal component with
boolean flags for every current page. Prefer shared chassis plus feature-owned
content/ports.

## 4. Validation & Error Matrix

| Condition                                                                        | Required result                                                       |
| -------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| A shared owner already provides the capability                                   | Extend/reuse it or document why its contract is unsuitable.           |
| Two pages create near-identical controls or reducers                             | Promote one owner before merging the second copy.                     |
| A page directly imports Tauri or parses a shared raw payload                     | Move the boundary to the approved platform/feature owner.             |
| V2 imports leftover UI/hook/state without an explicit bridge                     | Architecture test fails; use V2 owners/ports.                         |
| A new package duplicates an adopted primitive                                    | Reject unless the task records a concrete capability gap and review.  |
| A dependency fails license/security/platform/footprint review                    | Do not adopt it.                                                      |
| Sharing needs speculative flags for one consumer                                 | Keep the implementation local and revisit with a concrete second use. |
| Shared UI begins owning filesystem, network, query, secret, or capability policy | Split authority back into the feature/platform owner.                 |
| A reusable component changes accessible name/order/keyboard behavior per page    | Define one semantic API and test every supported variant.             |

## 5. Good / Base / Bad Cases

- **Good:** Skills and MCP use one assignment/picker owner while their native
  ports keep distinct business mutations.
- **Good:** Agents and Models reuse catalog/split-pane geometry but retain
  separate lifecycle and model workflows.
- **Good:** Prompts and Memory share search/list/tabs primitives without
  merging their native data models.
- **Base:** one route-specific form stays under its page because no second
  consumer exists and extracting it would add speculative parameters.
- **Bad:** copy a component, CSS block, target-order table, payload decoder, or
  query reducer into another page because changing the shared owner seems
  slower.
- **Bad:** move native capability logic into a generic visual component so two
  pages can call it.

## 6. Tests Required

- Import-boundary tests keep V2, leftover, pages/widgets, shared features/UI,
  and platform adapters in the approved direction.
- Shared owner tests cover semantic variants, keyboard/focus/accessibility,
  pending/disabled behavior, and maintained responsive viewports where
  applicable.
- Feature tests prove every consumer invokes the same shared component/helper
  while the owning feature port still controls the mutation/readback.
- Type tests keep ID/order maps exhaustive and prevent page-local fallback
  tables.
- Dependency changes update lockfiles and pass repository dependency, license,
  provenance, type, lint, unit, browser, and build gates appropriate to the
  changed surface.
- Negative scans reject page-local clones of named shared owners, direct Tauri
  imports outside adapters, and repeated unsafe raw payload casts.

## 7. Wrong vs Correct

Wrong: duplicate a shared interaction and native boundary inside a page.

```tsx
function PageInstallTargetPicker() {
  return targets.map((target) => (
    <button onClick={() => invoke("install", { path: target.path })}>
      {target.label}
    </button>
  ));
}
```

Correct: reuse the shared interaction; the feature port accepts only its
reviewed typed request and owns native authority.

```tsx
<InstallTargetDialog
  defaultTarget={selectedTarget}
  pathForTarget={projectDisplayDestination}
  onConfirm={(target) => featurePort.install(target)}
/>
```

Wrong: extract unrelated page behavior into one flag-driven component.

```tsx
<UniversalPanel isAgent isModel isMcp useLegacyFallback />
```

Correct: share the chassis and keep feature content/ports at their owner.

```tsx
<SplitPanes>
  <FeatureOwnedList />
  <FeatureOwnedDetail />
</SplitPanes>
```
