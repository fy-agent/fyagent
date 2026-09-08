# Renderer Modular Boundaries

## 1. Scope / Trigger

Read before moving source roles, adding native calls or splitting a shared
contract. The renderer is single-generation; role names describe ownership,
not old/new implementations. See [Directory Structure](./directory-structure.md).

## 2. Signatures and owners

```ts
createTauriFeaturePorts(): FeaturePorts;
// shared/platform/tauri/features.ts composes capability-owned feature-ports.

// Portable parsing/serialization, no React or native runtime:
import { parseJobSnapshot } from "@/domain/codex-desktop";
import { updateCommonConfigSnippet } from
  "@/domain/configuration/serialization/providerConfigUtils";
```

`shared/features/types.ts` remains an explicit named-export facade; product
DTOs/constants belong to `agents`, `models`, `skills`, `mcp`, `prompts`, `memory`,
`settings` and their focused contract modules. It has no wildcard export or
new implementation. Domain files do not import this facade back from UI.

## 3. Dependency and state contracts

- `app` composes pages/widgets/shared/dev; `main.tsx` composes application startup.
- `pages` imports the same route, shared and domain, not widgets/app/dev.
- `widgets` imports shared and sibling widgets, not pages/app/dev.
- `shared` imports shared/domain, not pages/widgets/app/dev.
- `domain` imports only domain or runtime-neutral dependencies. It has no React,
  Tauri, notification, renderer-cache or UI-lifetime ownership.
- `shared/ui` does not import feature/platform runtime. Feature-aware controls
  such as CopyablePath, ExternalLinkButton and FileRecoveryButton belong to
  `shared/features/controls`.
- Tauri package imports, native command literals and boundary decoding live in
  `shared/platform/tauri/**`. The root facade composes ports; capability-owned
  modules validate payloads and parse native responses. ACL tests scan the
  entire adapter tree, not just the facade.
- Motion and glass dependencies have reviewed shared owners; the resize library
  has its own `shared/ui/split/vendor.ts` owner so chrome does not eagerly load
  pane implementation through a ubiquitous vendor barrel.
- Preserve native command names, DTO/schema versions, persisted provider IDs,
  one-shot startup readiness, cancellation and authoritative rereads during
  moves. An in-memory query namespace may change as one transaction; use the
  shared key factory, including prefix invalidations.

The provider-config facade re-exports JSON and Codex TOML owners. Structural
operations use `Record<string, unknown>` and guards; own-property writes must
not recurse into prototypes or trigger inherited setters. The JSON encoder
supplies compatible basic-string escapes for TOML values. Replacement callbacks
preserve literal `$&`/`$1` tokens instead of interpreting replacement syntax.

A large route root can remain large when it is the single owner of selection,
draft and dirty-blocker state. Extract a cohesive module with an explicit props
boundary, not arbitrary JSX slices made only to satisfy a line limit.

## 4. Failure matrix

| Condition                                               | Required result                                                                   |
| ------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Import points to a retired root or unknown source role  | Type/build and architecture checks fail.                                          |
| Domain imports React/Tauri/shared runtime               | Ownership gate fails, move runtime back to its adapter.                           |
| Shared UI owns a query or native workflow               | Move feature-aware behavior to its feature owner.                                 |
| Platform facade grows payload parsers                   | Keep parsing in its capability-owned module.                                      |
| Query prefix invalidation names a previous namespace    | Fix the shared key factory consumer and rerun behavior tests.                     |
| Migration scanner reads zero files from an old path     | Positive entry/coverage assertions fail; never report this as clean architecture. |
| A native write or error changes during a directory move | Stop and treat it as a behavior change, not cosmetic cleanup.                     |

## 5. Good / Bad cases

Good: `shared/codex-desktop` reuses the independent `domain/codex-desktop`
parsers and the existing native port. Bad: move hooks into `domain` just to
make an import pass, or keep a hidden second application under another name.

## 6. Tests required

`tests/renderer/app/architecture.test.ts` parses actual TS import syntax and
requires known production/domain roots plus positive coverage. It checks layer
direction, native access, package ownership and feature facades. The independent
dependency-cruiser test requires TypeScript support and actual current entry
coverage, rejects runtime cycles/unresolved imports and retains negative fixtures.
`tests/architecture/frontendModuleBoundaries.test.ts` checks the pure facade;
domain configuration tests retain escaping, sanitation and prototype defenses.

Run the unified type/lint/unit gate, affected feature tests and production boot.
Import graphs are not proof of business correctness or native runtime behavior.
