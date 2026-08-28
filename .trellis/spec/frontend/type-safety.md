# Type Safety

Leftover renderer types live in `src/types.ts` and `src/lib/`. V2 feature
wire types live in `src/v2/shared/features/types.ts` and must be parsed at
the platform adapter before React Query sees them. Do not treat leftover
facades as the V2 port contract. Closed V2 catalog, assignment, model, and
prompt ID unions are sourced from `src/v2/shared/features/directory.ts` and
re-exported by `types.ts`; do not widen those unions in a page-local type.

## Compiler Contract

The renderer compiles with TypeScript strict mode. `tsconfig.json` also enables
`noUnusedLocals`, `noUnusedParameters`, and `noFallthroughCasesInSwitch`, and
maps `@/*` to `src/*`. Type-check renderer and test code with
`mise run typecheck`.

## Type Ownership

- Shared frontend domain interfaces and unions live in `src/types.ts` and the
  smaller `src/types/` modules. Components import those with `import type` when
  they only need a type.
- Most feature-level API facades in `src/lib/api/` expose explicit parameter
  and `Promise` return types around Tauri `invoke` calls. Prefer the relevant
  facade when extending a facade-driven feature, but follow the nearest
  established boundary: `src/main.tsx`, `src/components/theme-provider.tsx`,
  and `src/components/DatabaseUpgrade.tsx` are narrow direct-`invoke` paths,
  not a blanket pattern for unrelated features.
- Forms use Zod schemas in `src/lib/schemas/` and infer their form data from
  the schema rather than maintaining a parallel form-only interface.

```tsx
// src/lib/schemas/provider.ts
export const providerSchema = z.object({
  name: z.string(),
  websiteUrl: z.string().url().optional().or(z.literal("")),
  notes: z.string().optional(),
  settingsConfig: z.string().min(1), // also JSON.parse-checked
  icon: z.string().optional(),
  iconColor: z.string().optional(),
});

export type ProviderFormData = z.infer<typeof providerSchema>;
```

## Dynamic and Cross-Layer Data

The existing `Provider.settingsConfig` is deliberately dynamic
(`Record<string, any>`), and several UI paths parse editable JSON before
applying a narrow assertion. This means the repository does not currently
enforce a blanket ban on `any` or type assertions. For a stable new shape,
extend the existing domain type or a Zod schema; retain a narrow boundary for
provider-specific JSON instead of claiming it is universally validated.

The Tauri wire boundary uses camelCase names on the TypeScript side and
`serde(rename = "...")` on Rust fields where needed. A command payload change
must be checked against both sides of that boundary.

V2 Agent install/action wires are owned by
`src/v2/shared/features/agent-install-readiness.ts` and parsed in
`src/v2/shared/platform/tauri/feature-ports/agentInstallReadiness.ts`
before React Query. Pages must not locally cast `invoke` results or add
URL/path/command fields. Exact keys, opaque `v1:` release IDs, and the
forbidden-wire scan are part of that owner. See
[External Agent P0 Safety](../backend/external-agent-p0.md) and
[V2 Agent and Models](./v2-agent-models.md).

Companion /shurufa wire is parsed once in
`src/v2/shared/platform/tauri/feature-ports/shurufa.ts`. Pages poll
`getCompanionSnapshot` only; they must not `invoke()` or own serial
read. See [Shurufa Companion](../backend/shurufa-companion.md).

```rust
// src-tauri/src/provider.rs
#[serde(rename = "settingsConfig")]
pub settings_config: Value,
```

## Evidence

- [tsconfig.json](../../../tsconfig.json) defines strict renderer/test compiler
  options and the `@/*` alias.
- [src/lib/schemas/provider.ts](../../../src/lib/schemas/provider.ts) derives
  `ProviderFormData` from the runtime Zod schema.
- [src-tauri/src/provider.rs](../../../src-tauri/src/provider.rs) shows the
  serialized Rust companion for frontend provider data.
- [src/v2/shared/features/agent-install-readiness.ts](../../../src/v2/shared/features/agent-install-readiness.ts)
  owns the Agent install/action wire parser.
- [src/components/theme-provider.tsx](../../../src/components/theme-provider.tsx)
  shows a narrow direct-`invoke` boundary alongside the facade-based paths.
