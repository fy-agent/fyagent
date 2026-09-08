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

```rust
// settings.json field appearanceTheme: "light" | "dark" | "system"
parse_appearance_theme(value: &str) -> Option<&'static str>
persist_appearance_theme(theme: &str) -> ()
appearance_theme_preference() -> Option<String>
appearance_bootstrap_script() -> Option<String>
async set_window_theme(window: tauri::Window, theme: String) -> Result<(), String>
```

`shared/design-system/appearance.ts` validates and applies the preference;
`shared/features/useAppearance.ts` is its single mounted shell owner.
`widgets/app-shell/ThemeToggle.tsx` composes Button and existing icons outside
native drag regions. `shared/ui/ThemeReveal.ts` owns only the ephemeral browser
transition. `shared/platform/tauri/theme.ts` invokes the existing native
`set_window_theme` command with `{ theme }`; no new native API/permission is added.

Device restart authority is `~/.fyagent/settings.json` `appearanceTheme`, written
only by `persist_appearance_theme`. Renderer `localStorage["fyagent-theme"]` is a
synchronous cache. `save_settings` snapshots must keep the existing appearance
field; they are not an appearance writer.
The native owners are `src-tauri/src/settings.rs`,
`src-tauri/src/commands/{settings,system}.rs` and the startup wiring in
`src-tauri/src/lib.rs`.

## 3. Contracts

- Existing `light`/`dark` are respected; `system` remains live system-following
  until explicit user selection. Missing/invalid/denied storage uses light.
  Preference failures never block startup or a visible switch. Do not add a
  second settings file or store resource/credential data in appearance state.
- On toggle the visible commit writes the renderer cache; native persist and
  chrome IPC stay outside the View Transition snapshot, through the existing
  coalesced `set_window_theme` path. Invalid values are not persisted.
- Native normalization trims whitespace but accepts only lowercase
  `light`/`dark`/`system`. Persistence is best-effort: the helper logs a write
  failure and returns `()`, then the command still attempts window chrome.
  Command success therefore proves the chrome call, not durable persistence;
  a failed disk write does not guarantee the new preference survives relaunch.
  Invalid command input is not saved, but currently selects native system
  chrome (`None`) rather than returning a validation error. Renderer callers
  must continue to send only the closed preference set.
- Process restart restores native window chrome in `prepare_main_webview` from
  the device field, then seeds the renderer cache/`data-theme` on main
  `PageLoadEvent::Started` from the live field. `initializeAppearance` and the
  shell owner re-read the cache; they do not wait on theme IPC. Native window
  reveal still belongs to content readiness.
- Initial root `data-theme` is applied before route loading, including startup
  errors.
- Both palettes are blue: clear light blue and medium-depth mist/steel blue,
  not black. Roles cover text, status, input, hover, focus, selected, material,
  rim and scrim. Primary button ink and filled background are paired.
- Only the shell control rerenders on explicit theme changes; color roles
  update together via root CSS variables. No route reconstruction/query
  invalidation, page-wide React theme context or global color interpolation.
- A user reveal uses one native View Transition and one WAAPI circle on
  `::view-transition-new(root)`. `shared/ui/ThemeReveal.ts` is the only owner;
  Mac and Windows share that path. Pointer uses exact viewport coordinates;
  keyboard uses the current trigger center. Radius covers the farthest corner
  of the reference box, with padding for subpixel snapshot height. Clip
  geometry is a percentage `circle()` of that box. CSS owns `--fy-motion-theme`
  (560ms) and `--fy-motion-theme-ease`; JS reads the duration through the
  unit-aware parser and uses the matching `fyThemeRevealEasing` curve. Do not
  reuse `fySpatialEasing` here: that front-loaded curve spends most of the
  duration on the last slice of radius and reads as a stuck far corner.
  Active reveal layers keep `transform: none` and `transform-origin: 0 0` so a
  centered view-transition origin cannot shift the circle off the trigger. A
  finished reveal skips the view transition before cancelling its clip
  animation so the next toggle cannot inherit a full-circle mask. Suppress
  competing CSS transitions throughout capture and reveal, not only until the
  update callback resolves.
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
| Software relaunch after a successfully persisted choice   | Restore that closed preference; do not replace it with the default. |
| `save_settings` payload includes a different theme        | Keep the device appearance field; do not clobber it.            |
| Native input is padded lowercase / unknown or uppercase  | Normalize the valid choice; invalid values leave persistence unchanged and use system chrome. |
| Disk persistence fails but native chrome succeeds        | UI stays usable; IPC may resolve successfully; durable restart preference is not proven. |
| Capture throws/rejects or pseudo animation is unavailable | Commit latest choice; release handles/markers.                  |
| Rapid opposite clicks                                     | Latest intent wins, no queued full-screen transitions.          |
| Resize/background/live reduced motion                     | Settle; no stuck clip or hit-test lock.                         |
| External storage during capture                           | Cancel stale local commit without overwriting external storage. |
| Modal exists                                              | No root reveal prolonging sensitive content presentation.       |
| Native theme command or persist fails                     | Keep CSS/UI usable; no query/startup failure.                   |

## 5. Good / Base / Bad Cases

Good: update semantic tokens and validate real selected/hovered composites in
both engines; relaunch restores the last closed preference from the device
field. Base: a WebView lacks View Transitions; the same button switches
directly; browser fixtures keep using the renderer cache. Bad: invert the
application, copy account DOM to canvas, import another theme framework, wait
for IPC inside capture, or treat WebView `localStorage` as the only restart
authority.

## 6. Tests Required

`ThemeReveal.test.ts` covers request identity, failures, timing, percentage
clip origins and the shared reveal easing. `motionDuration.test.ts` keeps
`--fy-motion-theme-ease` aligned with `fyThemeRevealEasing`.
`tests/renderer/app/appearance.test.ts` covers startup restore from the renderer
cache. `useAppearance.test.tsx` covers system/storage, mount restore and
subscriptions. Platform tests retain exact IPC and coalescing; the ACL gate
scans every literal native command. Native tests cover the closed preference
parser, bootstrap script and `save_settings` merge preservation. These tests
do not perform a packaged-app relaunch or inject a settings-file write failure;
do not cite them as evidence of durable persistence under failed I/O.

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

Wrong: keep the preference only in WebView `localStorage`, then treat a missing
cache on relaunch as a new light default. Correct: persist the closed choice
through the existing `set_window_theme` owner into `appearanceTheme`, restore
chrome before reveal, seed the renderer cache from the live field, and apply
closed root tokens without competing color transitions or theme IPC inside
capture.

Wrong: scale clip coordinates by `devicePixelRatio`, leave a `fill: both`
circle after the transition, or branch Mac/Windows playback. Correct: keep
one `ThemeReveal` path, percentage clip on the CSS-pixel reference box, skip
then cancel leftover `::view-transition-new(root)` animations, and use
`fyThemeRevealEasing` on every platform.
