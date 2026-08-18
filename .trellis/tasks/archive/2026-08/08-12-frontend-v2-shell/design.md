# FyAgent frontend V2 shell - Technical Design

## Boundaries

The V2 renderer is an isolated composition root. It may depend on reviewed
third-party packages, but it may not import legacy renderer modules. Rust/Tauri
remains an unchanged host boundary. The only native bridge introduced here is
a narrow TypeScript port implemented below `src/v2/shared/platform/tauri`.

## Runtime structure

```text
src/v2/
|- main.tsx
|- app/                  router, root error, styles
|- pages/                six empty route elements
|- widgets/app-shell/    shell and visible chrome
|- shared/
|  |- config/            navigation contract
|  |- assets/            transparent header mark
|  |- design-system/     V2 helpers and tokens
|  |- platform/          runtime port plus browser/Tauri adapters
|  `- ui/                V2-owned Radix-backed primitives
`- dev/                  development-only UI Lab
```

`createHashRouter` owns navigation state. The root route renders `AppShell` and
an `Outlet`; index and wildcard routes redirect to `/models`. The six page
elements intentionally render no product content. The UI Lab route is included
only when `import.meta.env.DEV` is true.

## Internal contracts

```ts
export type NavigationItem = {
  id: "agents" | "models" | "skills" | "mcp" | "prompts" | "memory";
  path: "/agents" | "/models" | "/skills" | "/mcp" | "/prompts" | "/memory";
  label: string;
};

export interface WindowFramePort {
  isNative: boolean;
  platform: "browser" | "windows" | "macos" | "linux" | "unknown";
  prepareFrame(): Promise<void>;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
}
```

Stable accessible names and test IDs identify the brand, primary navigation,
Search, Settings, Avatar, minimize, maximize/restore, close, and content
viewport. Navigation uses links and `aria-current="page"`; icon-only controls
use native buttons and explicit `aria-label` values.

## Visual system and layout

V2 imports only its own token, global, motion, and namespaced shell styles.
Semantic tokens cover background, stable content surface, glass control
surface, text hierarchy, borders, selected navigation, focus ring, shadows,
radii, geometry, spacing, and timing. Selected text and focus colors are
darkened from the initial visual draft so composed contrast stays accessible.

The top bar uses CSS Grid. Wide layouts balance brand and the combined
tools/window region around the centered navigation; constrained layouts allow
the navigation to shift but never overlap, hide, iconify, or scroll. The
content viewport uses `min-width: 0`, `min-height: 0`, and fills the remaining
shell height. Motion is restricted to functional color, shadow, opacity, and
small transform changes.

## Platform flow

`runtime.ts` identifies browser/native and platform without directly importing
Tauri modules. A factory exposes either a browser no-op port or the Tauri port.
The Tauri implementation obtains the current window, prepares Windows
decorations, and delegates window actions. Browser preview always renders the
Windows control cluster but its port remains inert.

Shell startup calls frame preparation and an idempotent lifecycle function. The
lifecycle module caches its promise so React StrictMode or repeated callers do
not emit `frontend-deeplink-ready` more than once. This event preserves the
minimum host activation handshake but does not claim that legacy consumers have
been migrated.

## Dependency decision

The implementation retains existing Radix packages behind V2-owned wrappers.
`glasscn-ui` is not installed because its broad dependency surface, global CSS
contract, token requirements, and limited glass variants meet the approved
stop-loss. React Router 7 and Phosphor are the only new runtime families;
ESLint/TypeScript tooling and Playwright are development-only.

## Compatibility and rollback

- Legacy source stays present; the only legacy runtime change is the HTML
  module entry selection.
- No persisted data or wire contract changes, so no migration exists.
- The transparent Y is a header-only asset; application icon generation is
  untouched. The user-supplied, approved 128x128 repository copy is identified
  by SHA-256
  `f58e48540e3b13ee6dee5ae26ff9aca4d34a06605a76331bb53e5656bc70327c`;
  no external path or runtime dependency is retained.
- If a dependency or browser integration fails, fall back within V2 to a
  thinner native/Radix primitive rather than upgrading React/Tailwind.
- If native window behavior fails, revise or revert the V2 platform slice; do
  not expand the task into Rust.
- Reverting the HTML entry restores the legacy runtime without deleting V2,
  providing a narrow rollback while diagnosis continues.
