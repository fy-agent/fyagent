# Component Guidelines

## Ownership and composition

The only production component tree lives under `app`, `pages`, `widgets` and
`shared`. Route components compose typed feature state and shared controls.
`shared/ui` owns pure visual/interaction primitives; controls that consume
FeaturePorts belong to `shared/features/controls`, never a reverse UI barrel.
Portable serialization belongs to `domain`, not component helpers.

Every `FeatureTabPanel` declares a layout role: `workspace` for bounded page
content and `flow` for ordinary forms. Shared height/scroll ownership lives in
[Surfaces](./surfaces-responsive.md); do not drop a plain block into that chain.

Use the existing `Button`, `Dialog`, `FeatureTabs`, `FeatureList`, `FeatureSearch`,
`FeaturePagination`, `AssignmentPanel`, `BulkAssignmentPanel`, `SecretInput`, `CatalogMasterDetail` and
`SplitPanes` owners. Reuse Phosphor icons, adopted Radix primitives, semantic
tokens and `classNames` from `shared/design-system/classNames.ts`. Do not restore
retired Tailwind/Lucide wrappers or a second UI framework.

## Props and interaction contracts

Props carry typed data/callbacks, not hidden query/native authority. A known
second consumer should use one shared owner immediately; a route-specific
detail stays local unless extraction creates a meaningful reusable contract.
Avoid flag-heavy universal components and arbitrary JSX splits made only to
reduce line counts.

Preserve native button/link/input semantics, accessible names, labels, focus
order and disabled behavior. Decorative icons/materials are hidden from assistive
technology and pointer hit testing. Text must be actually painted above the
selection glass; a declared CSS contrast value is not proof of readability.

The shared Dialog owns portals, modal focus, source origin and exit lifetime.
Its origin prop is explicit at every call. Transient menu items use the shared
capture plus an owned persistent return anchor; switches use the existing
asChild capture. Never duplicate this lifecycle in feature pages. Async content
arrival must not cancel source entry; see [Motion](./motion-system.md).
Do not wrap it in a second presence engine or copy input DOM into an animation
layer. Closing or hiding immediately revokes business actions and clears secrets;
visual exit may finish afterwards. Feature tabs reuse Radix keyboard semantics.

Split panes delegate pointer, keyboard and constraint mechanics to
`react-resizable-panels` through `shared/ui/split/vendor.ts`. Route props supply
minimum/maximum dimensions; direction changes retain editor nodes/drafts.
Skills/MCP reuse the `split/sizing.ts` middle-flexible profile, not page-local
width variables. Bulk actions use one presentation owner with explicit name
and action slots. Intrinsic card/metadata roles belong in shared tokens/styles.

## Copy, CSS and verification

Current product UI is Simplified Chinese. Keep business reasons in closed
presentation mappings and follow [User-Facing Copy](./user-facing-copy.md) and
[Localization](./localization.md). Do not restore unused locale loaders as part
of a source move. Use shared semantic CSS variables and role-owned styles;
page-local colors cannot become a separate theme.

Run `mise run typecheck`, `mise run lint`, `mise run test:unit`, plus browser
regressions for interaction/layout. Assert accessible behavior, actual glyph
painting, container bounds, focus and cleanup. `toBeVisible` alone does not prove
an element is usable, legible or unoccluded. Read [Reuse](./reuse.md),
[Surfaces](./surfaces-responsive.md) and [Motion](./motion-system.md).
