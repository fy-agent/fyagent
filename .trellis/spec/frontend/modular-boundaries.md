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

## 3. Contracts

- V2 must not import leftover `components`, `hooks`, `lib`, or `i18n` code.
- `src/shared/**` must not import V2, leftover renderer modules, React, or
  `@tauri-apps/*`.
- Leftover `src/components/**` must not import `@tauri-apps/*`; use
  `src/lib/api/**` or an owning hook.
- `src/main.tsx` is the reviewed bootstrap exception for process lifecycle.
- `App.tsx` is a composition root. Cross-feature startup/event/cache
  coordination belongs in an owning hook such as `useAppRuntimeEffects`.
- Specialized provider forms depend on `ProviderForm.types.ts` and pure model
  helpers, not on the `ProviderForm.tsx` composition root.
- Refactors preserve command names, payloads, event cleanup, cache
  invalidation, one-shot migration semantics and user-visible errors.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| V2 imports leftover implementation | Architecture test fails; port/extract through an approved boundary |
| `src/shared/**` imports React/Tauri/V2/leftover code | Architecture test fails; move runtime code back to its generation |
| Leftover component imports Tauri | Architecture test fails; use an API/hook facade |
| Specialized provider form imports `./ProviderForm` | Architecture test fails; depend on types/model modules |
| Event facade does not return unlisten | Reject; cleanup semantics must remain intact |

## 5. Good / Base / Bad Cases

- **Good:** `DeepLinkImportDialog` subscribes through `deeplinkApi` while the
  facade owns Tauri events and returns unlisten.
- **Good:** `src/shared/codex-desktop/**` contains neutral DTO/state parsing;
  V2 UI remains inside V2.
- **Base:** leftover `src/lib/api/**` may import Tauri because it is the
  leftover platform facade.
- **Bad:** a V2 page imports `@/hooks/useSettings`.
- **Bad:** React hooks are moved into `src/shared/**` only so both generations
  can import them.

## 6. Tests Required

```bash
mise run typecheck
mise run test:unit -- tests/architecture/rendererBoundaries.test.ts
mise run lint:v2
mise run test:v2
```

The architecture test asserts neutral shared imports, leftover Tauri access,
and provider-form dependency direction. Run nearest feature/integration tests
too; dependency tests do not prove behavior.

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
