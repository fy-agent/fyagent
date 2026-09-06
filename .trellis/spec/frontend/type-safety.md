# Type Safety

## 1. Scope / Trigger

Read before changing a feature DTO, native port, dynamic configuration parser,
identifier union or compiler boundary. The product has one strict TypeScript
entry; independent configuration/domain code is not an alternative renderer.

## 2. Signatures and owners

`tsconfig.json` covers `src/**/*`, `tests/**/*` and `config/**/*.ts`, with
`strict`, unused-symbol checks, switch fallthrough checks and `@/* -> src/*`.
`shared/features/ports.ts` composes typed feature ports; capability modules own
DTOs and parsers. `directory.ts` owns the closed catalogue/assignment/model/
prompt identifiers, re-exported by the named `types.ts` facade.

Portable configuration types and serialization live in `domain/configuration`.
`Provider.settingsConfig` and extension fields use `Record<string, unknown>`;
dynamic input is not permission to replace validation with `any`.

```ts
import { isPlainObject } from "@/domain/configuration/serialization/providerConfigStructural";
// Narrow an unknown child before reading a provider-specific field.
const env = isPlainObject(config.env) ? config.env : undefined;
const baseUrl =
  typeof env?.ANTHROPIC_BASE_URL === "string"
    ? env.ANTHROPIC_BASE_URL
    : undefined;
```

## 3. Contracts

New or changed untrusted native response boundaries start as `unknown` at
`shared/platform/tauri/feature-ports`. Parse before exposing them to components;
reuse the owner's schema, including excess-field rejection where required.
This is not a claim that every existing Port already parses at runtime:
Skills/MCP `simple.ts` and the WorkBuddy/direct-provider methods enumerated in
[Models](./models.md#runtime-parsing-boundary) retain typed-only boundaries.
Their focused specs name the gap; do not manufacture validation evidence from
TypeScript annotations or broaden those exceptions to new inputs.
Closed discriminants, reason codes, revision ordering and opaque IDs retain
their native wire meaning during source moves. `v1:` IDs and versioned DTO
schemas are protocols, not renderer-generation names to replace.

Keep camelCase command arguments synchronized with the Rust serialization.
Do not add filesystem paths, command argv, executable URLs or broad locator
fields to an existing opaque request. `zod/mini` is the adopted strict-schema
subpath for file recovery and managed write contracts; preserve bundle budgets.

Use `import type` for types, meaningful discriminated unions for variants and
runtime narrowing for dynamic objects. A type assertion is not validation.
No broad `any`, `ts-ignore` or weakened compiler settings to make migration pass.

## 4. Validation & Error Matrix

| Condition                                          | Required result                                                                 |
| -------------------------------------------------- | ------------------------------------------------------------------------------- |
| Unknown child is null, array or primitive          | Reject or use the explicitly documented fallback; do not access guessed fields. |
| Native enum/contract version is unrecognized       | Owning parser rejects it; no optimistic capability/success state.               |
| Request contains forbidden locator/extra fields    | Exact native/renderer contract tests reject it.                                 |
| DTO field changes on one side only                 | Fix both owners and shared fixture; do not cast through unknown.                |
| UI needs an identifier outside the closed registry | Review the registry/native contract together, not a page-local widening.        |

## 5. Good / Base / Bad Cases

Good: decode `parseJobSnapshot` in the native adapter and pass its typed result
to the installer UI. Base: editable malformed TOML retains the existing explicit
line-scanning fallback. Bad: `invoke(...) as Result` in a component or converting
an unknown state into logged-in/installed.

## 6. Tests Required

Run `mise run typecheck`, `mise run lint` and `mise run test:unit`.
Port/DTO tests must cover invalid discriminants, null/missing/excess fields,
malformed opaque IDs and exact command payloads. Domain serialization tests
retain prototype safety, literal token replacement and TOML escaping cases.
The architecture suite must scan actual production files, not an empty old tree.

## 7. Wrong vs Correct

Wrong: widen a response to `Record<string, any>` and read nested fields.
Correct: retain unknown input, reuse its owning parser/guard, then expose a
closed typed value. See [Modular Boundaries](./modular-boundaries.md).
