# Blue Appearance and Theme Reveal

## 1. Scope / Trigger

Read before changing blue palettes, initial appearance, preference persistence,
native chrome synchronization or the top-bar theme control. Layout, routing,
account authority and secret lifetimes remain unchanged. Material details live
in [Surfaces](./surfaces-responsive.md); general motion in [Motion](./motion-system.md).

## 2. Signatures and owners

```ts
type Theme = "light" | "dark";
type ThemePreference = Theme | "system";
const THEME_STORAGE_KEY = "fyagent-theme";
initializeAppearance(): void; // app/appearance.ts, before initial route loading
useAppearance(): { theme: Theme; toggle(source: HTMLElement, origin?: {x: number; y: number}): void };
synchronizeWindowTheme(theme: ThemePreference): Promise<void>;
```

`shared/design-system/appearance.ts` validates and applies the preference;
`shared/features/useAppearance.ts` is its single mounted shell owner.
`widgets/app-shell/ThemeToggle.tsx` composes Button and existing icons outside
native drag regions. `shared/ui/ThemeReveal.ts` owns only the ephemeral browser
transition. `shared/platform/tauri/theme.ts` invokes the existing native
`set_window_theme` command with `{ theme }`; no new native API/permission is added.

## 3. Contracts

- Existing `light`/`dark` are respected; `system` remains live system-following
  until explicit user selection. Missing/invalid/denied storage uses light.
  Preference failures never block startup or a visible switch. Do not add a
  second settings file or store resource/credential data in appearance state.
- Initial root `data-theme` is applied before route loading, including startup
  errors. Native window reveal still belongs to content readiness, not theme IPC.
- Both palettes are blue: clear light blue and medium-depth mist/steel blue,
  not black. Roles cover text, status, input, hover, focus, selected, material,
  rim and scrim. Primary button ink and filled background are paired.
- Only the shell control rerenders on explicit theme changes; color roles
  update together via root CSS variables. No route reconstruction/query
  invalidation, page-wide React theme context or global color interpolation.
- A user reveal uses one native View Transition and one WAAPI circle on
  `::view-transition-new(root)`. Pointer uses exact viewport coordinates;
  keyboard uses the current trigger center. Radius covers the farthest corner.
  CSS owns `--fy-motion-theme` (560ms); reuse the unit-aware parser and spatial
  curve. Suppress competing CSS transitions throughout capture and reveal,
  not only until the update callback resolves.
- New requests cancel old presentation. Old update/ready/finished callbacks
  cannot commit or clear the new one. Resize, hidden document and live reduced
  motion/forced colors settle the latest intent. External storage supersedes
  a pending capture without persisting that stale local intent.
- Missing View Transition/pseudo animation/media support, reduced motion,
  forced colors or an active modal use direct appearance updates. Browser-owned
  transient views are never read, cloned, exported or kept by application code;
  credentials are not copied into an animation tree.
- Native calls are serialized/coalesced through the platform owner without
  delaying the visible commit. Failures produce a bounded diagnostic, not raw
  payloads. An older native completion cannot be the final queued write.

## 4. Validation / Error Matrix

| Condition                                                 | Required result                                                 |
| --------------------------------------------------------- | --------------------------------------------------------------- |
| Arbitrary stored value or denied store                    | Light fallback; controls stay usable.                           |
| Existing system preference changes                        | Update effective theme, preserve system preference.             |
| Explicit theme selected                                   | Store the closed choice; stop following system changes.         |
| Capture throws/rejects or pseudo animation is unavailable | Commit latest choice; release handles/markers.                  |
| Rapid opposite clicks                                     | Latest intent wins, no queued full-screen transitions.          |
| Resize/background/live reduced motion                     | Settle; no stuck clip or hit-test lock.                         |
| External storage during capture                           | Cancel stale local commit without overwriting external storage. |
| Modal exists                                              | No root reveal prolonging sensitive content presentation.       |
| Native theme command fails                                | Keep CSS/UI usable; no query/startup failure.                   |

## 5. Good / Base / Bad Cases

Good: update semantic tokens and validate real selected/hovered composites in
both engines. Base: a WebView lacks View Transitions; the same button switches
directly. Bad: invert the application, copy account DOM to canvas, import another
theme framework, or wait for IPC inside capture.

## 6. Tests Required

`ThemeReveal.test.ts` covers request identity, failures, timing and origins.
`useAppearance.test.tsx` covers system/storage and subscriptions. Platform tests
retain exact IPC and coalescing; the ACL gate scans every literal native command.

`blue-themes.spec.ts` covers real pointer/keyboard, drafts, persistence,
interruption and dark composite text on all seven pages. Existing material tests
cover light. Outlined controls require contrast on both sides; opaque filled
controls are identified by their silhouette against the outside, not a border
contrasted against identical internal fill. Text samples must intersect overflow
clipping: an ellipsized Range includes unpainted tails. The independent
`contrast-sampling.spec.ts` protects these distinctions without lowering budgets.

Performance tests belong only to `config/playwright.performance.config.ts`, one worker,
production build and no concurrent compile/browser suites. Functional config
excludes `*-performance.spec.ts`. Separate preparation from reveal thirds, one
cold and twenty warm cycles at 1x/4x CPU cost; warm 1x frame p95 stays at 33.4ms.
Report each run, middle-third measurements and native/GPU limitations.

## 7. Wrong vs Correct

Wrong: commit theme context for every page, start competing color transitions
and native queries, then check only the final theme. Correct: apply closed root
tokens, keep the browser views stable, reject superseded callbacks and separately
verify preparation, each reveal third and final cleanup.
