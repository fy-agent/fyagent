# Renderer Modular Boundaries

## 1. Scope / Trigger

Read this before moving code between `src/v2/**`, leftover `src/**`, or
renderer-neutral `src/shared/**`, before adding a Tauri call, or when splitting
a large leftover renderer module.

FyAgent intentionally has three ownership zones during migration:

```text
src/v2/**       production V2 renderer
src/**          leftover renderer / compatibility code outside V2
src/shared/**   renderer-generation-neutral domain/serialization logic
```

Do not erase those boundaries with cosmetic directory moves.

## 2. Signatures

`src/shared/**` has no React or Tauri runtime dependency. Leftover components
call platform facades such as:

```ts
systemApi.getMigrationResult(): Promise<boolean>;
systemApi.setWindowTheme(theme: "light" | "dark" | "system"): Promise<void>;
systemApi.exit(): Promise<void>;

deeplinkApi.onImport(handler): Promise<() => void>;
deeplinkApi.onError(handler): Promise<() => void>;
deeplinkApi.notifyFrontendReady(): Promise<void>;
```

V2 keeps its stricter FeaturePorts / `shared/platform/tauri` signatures from
the V2 Shell Contract. `src/shared/codex-desktop/**` remains its deliberate
renderer-neutral tree-external exception.

The V2 Tauri facade stays stable:

```ts
createTauriFeaturePorts(): FeaturePorts;
```

`src/v2/shared/platform/tauri/features.ts` is the composition point; capability
validation/parsing/invoke implementations live under its `feature-ports/**`
owner tree.

V2 feature contracts are product-domain owned under
`src/v2/shared/features/**`. The compatibility path remains available:

```ts
import type { InstalledSkill, ProviderSummary } from "./types";
```

but `types.ts` is only an explicit named-export facade. New contract logic
belongs in the owning files such as `assignments.ts`, `skills.ts`, `mcp.ts`,
`agents.ts`, `models.ts`, `prompts.ts`, `memory.ts`, and `settings.ts`.

Leftover provider-config imports also keep one compatibility path:

```ts
import {
  updateCommonConfigSnippet,
  extractCodexBaseUrl,
  setCodexModelName,
} from "@/utils/providerConfigUtils";
```

That file is a re-export facade, not the implementation owner.

## 3. Contracts

- V2 must not import leftover `components`, `hooks`, `lib`, or `i18n` code.
- Shared UI must not import feature/platform runtime. Controls that consume
  FeatureProvider, domain target catalogues or external-open coordination live
  in `shared/features/controls` (`CopyablePath`, `ExternalLinkButton`,
  `InstallTargetDialog`); pages reuse those owners. Visual primitives and
  catalog geometry remain under `shared/ui` without a reverse re-export.
- `src/shared/**` must not import V2, leftover renderer modules, React, or
  `@tauri-apps/*`.
- Leftover `src/components/**` must not import `@tauri-apps/*`; use
  `src/lib/api/**` or an owning hook.
- `src/v2/shared/platform/tauri/features.ts` composes `FeaturePorts` only.
  Command literals, request validation and native-response parsing live in
  capability-owned `feature-ports/**` modules. ACL/adapter tests must scan the
  whole adapter tree rather than assuming every command string lives in the
  root facade.
- `src/v2/shared/features/types.ts` is a compatibility facade only. It may use
  explicit named re-exports and compatibility aliases, but it must not own new
  feature DTOs/constants/functions and must not use `export *`. Domain contract
  files must not import back through `types.ts`, which would invert ownership
  and risk cycles. Existing consumers may keep the facade path during gradual
  migration; new code should prefer the narrow owning domain contract when it
  does not create unnecessary churn.
- `src/main.tsx` is the reviewed bootstrap exception for process lifecycle.
- `App.tsx` is a composition root. Cross-feature startup/event/cache
  coordination belongs in an owning hook such as `useAppRuntimeEffects`.
- Specialized provider forms depend on `ProviderForm.types.ts` and pure model
  helpers, not on the `ProviderForm.tsx` composition root.
- Leftover provider configuration keeps `src/utils/providerConfigUtils.ts` as
  the stable compatibility facade. Implementation ownership is split by
  configuration language:
  - `providerConfigJsonUtils.ts`: JSON common-config merge/remove, API-key
    fields and template substitution;
  - `codexConfigUtils.ts`: Codex TOML inspection/editing and wire/model/base URL
    helpers;
  - `providerConfigStructural.ts`: prototype-pollution-safe structural
    sanitize/merge/remove/subset primitives shared by JSON and TOML readers.
    Do not migrate dozens of consumers merely to change import paths; change the
    facade only when its external API intentionally changes.
- A long V2 route root may remain long when it is the single owner of route
  selection/query/dirty-blocker state and its internal panels are already
  cohesive. Extract a route-local module only when an explicit props boundary
  and independent test seam improve change locality; do not split JSX by line
  count alone.
- Refactors preserve command names, payloads, event cleanup, cache
  invalidation, one-shot migration semantics and user-visible errors.

## 4. Validation & Error Matrix

| Condition                                                             | Required result                                                                                                              |
| --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| V2 imports leftover implementation                                    | Architecture test fails; port/extract through an approved boundary                                                           |
| `src/shared/**` imports React/Tauri/V2/leftover code                  | Architecture test fails; move runtime code back to its generation                                                            |
| Leftover component imports Tauri                                      | Architecture test fails; use an API/hook facade                                                                              |
| Specialized provider form imports `./ProviderForm`                    | Architecture test fails; depend on types/model modules                                                                       |
| `providerConfigUtils.ts` regrows JSON/TOML implementation logic       | `frontendModuleBoundaries` fails; keep it a compatibility re-export facade                                                   |
| V2 Tauri root facade regrows capability parsing/validation            | Reject; move logic to the owning `feature-ports/**` module and keep root composition-only                                    |
| V2 feature `types.ts` adds a DTO/constant/function or wildcard export | V2 architecture test fails; put the contract in its product-domain owner and explicitly re-export only compatibility surface |
| A product-domain feature contract imports `./types`                   | Reject; domain owners may depend on neutral directory/assignment primitives, never on their compatibility facade             |
| Event facade does not return unlisten                                 | Reject; cleanup semantics must remain intact                                                                                 |

## 5. Good / Base / Bad Cases

- **Good:** `DeepLinkImportDialog` subscribes through `deeplinkApi` while the
  facade owns Tauri events and returns unlisten.
- **Good:** `src/shared/codex-desktop/**` contains neutral DTO/state parsing;
  V2 UI remains inside V2.
- **Good:** existing callers keep importing `@/utils/providerConfigUtils`, while
  JSON, Codex TOML and structural-safety implementations have separate owners.
- **Good:** one V2 Tauri composition facade returns `FeaturePorts` while each
  capability adapter owns its wire validation/parser.
- **Good:** Skills, Agents, Models, MCP, Prompts, Memory and Settings contracts
  live in their own files while `types.ts` remains a small explicit
  compatibility facade.
- **Base:** leftover `src/lib/api/**` may import Tauri because it is the
  leftover platform facade.
- **Base:** a large route page may remain one file when moving its panels would
  only relocate JSX while making route-state ownership less obvious.
- **Bad:** a V2 page imports `@/hooks/useSettings`.
- **Bad:** React hooks are moved into `src/shared/**` only so both generations
  can import them.
- **Bad:** split a file solely to satisfy a line-count target, then add a barrel
  or cross-module state plumbing that increases dependency surface.

## 6. Tests Required

```bash
mise run typecheck
mise run test:unit -- tests/architecture/rendererBoundaries.test.ts tests/architecture/frontendModuleBoundaries.test.ts
mise run test:unit -- tests/architecture/dependencyGraph.test.ts
mise run lint:v2
mise run typecheck:v2
mise run test:v2
```

The architecture test asserts neutral shared imports, leftover Tauri access,
provider-form dependency direction, the provider-config compatibility facade,
and V2 feature-contract facade ownership. Run nearest feature/integration tests
too; dependency tests do not prove behavior. The dependency-cruiser gate checks
real TypeScript coverage, runtime cycles/unresolved edges and layer direction;
it also has negative fixtures so missing parser support cannot silently pass.
See [Renderer and Build Input Security](./security-boundaries.md).

## 7. Wrong vs Correct

Wrong:

```tsx
import { invoke } from "@tauri-apps/api/core";
await invoke("get_migration_result");
```

Correct:

```tsx
import { systemApi } from "@/lib/api";
await systemApi.getMigrationResult();
```

Wrong: put React/Tauri adapters into `src/shared/**`.

Correct: keep shared code runtime-neutral and put adapters in the owning
leftover or V2 runtime layer.
