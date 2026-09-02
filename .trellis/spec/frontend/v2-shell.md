# V2 Shell Compatibility Router

This stable path preserves archived references to the former broad V2 shell
document. It must remain a short reading map. New work cites the focused owner
instead of restoring route, layout, motion, feature or platform detail here.

## Read by concern

| Concern | Authoritative contract |
| --- | --- |
| Hash routes, navigation registry, literal lazy loaders, persistent pages, hidden query isolation, blockers and Agent return state | [V2 Navigation and Persistent Route](./v2-navigation.md) |
| AppShell/TopBar, native-overlay boundary, selection lens, shared motion/collapse, external opening and architecture imports | [V2 Window Shell and Interaction](./v2-window-shell.md) |
| Frontend directory and layer ownership | [Frontend Directory Structure](./directory-structure.md) and [Frontend Modular Boundaries](./modular-boundaries.md) |
| Feature types and Renderer/native trust boundary | [Frontend Type Safety](./type-safety.md) |
| Server state, draft state, URL state and persistence | [Frontend State Management](./state-management.md) |
| Shared controls, loading/empty/error states and accessibility composition | [Component Guidelines](./component-guidelines.md) |
| Reuse/adoption rules | [Frontend Reuse](./reuse.md) |
| Language detection and locale parity | [Frontend Localization](./localization.md) |
| Required tests, static architecture checks and native evidence limits | [Frontend Quality Guidelines](./quality-guidelines.md) |

## Shared invariants

- V2 production code remains under `src/v2` except for the exact neutral Codex
  Desktop core admitted by architecture tests. Direct Tauri imports stay under
  `src/v2/shared/platform/tauri/`.
- The Host owns filesystem/process/window/native effects. Renderer code uses
  typed Ports, closed IDs, bounded data and authoritative readback.
- Semantic selected/current/focus state belongs to controls and route/data
  owners; glass, lens and motion are presentation and remain optional.
- Browser preview and mocked Ports prove portable UI behavior only. Native
  drag regions, window geometry, external opening and host side effects require
  the corresponding native evidence.
- Keep this router short and index-like. Detailed contracts, error matrices and
  test assertions belong only in the linked focused documents.
