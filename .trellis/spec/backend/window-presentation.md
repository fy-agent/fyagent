# Main Window Presentation

## 1. Scope / Trigger

Read before changing initial show/focus, renderer readiness, main-WebView
recreation, tray/Dock activation or startup error recovery. `lib.rs` owns the
existing ActivationInbox; `lightweight.rs` and `tray.rs` request presentation
through it. Geometry and persistence remain in [Main Window Layout](./main-window-layout.md).
This is not a new window manager or a second deep-link queue.

## 2. Signatures

```text
renderer event: frontend-deeplink-ready (no payload, unchanged)
useFrontendReady(ready?: boolean): void
signalFrontendReady(): Promise<void>
request_main_window_focus(app: &tauri::AppHandle)
prepare_main_webview(window: &tauri::WebviewWindow)
ActivationInbox::{mark_ready, mark_window_prepared, mark_unready}
ActivationInbox::{arm_recovery, can_recover, finish_recovery}
native FeatureProvider QueryClient: focusManager.setFocused(true)
```

`ActivationInbox` owns renderer-ready, window-prepared, drain state, existing
bounded pending semantics, load generation and one recovery-armed bit. Readiness
never includes a URL, secret, target command or caller-selected window name.

## 3. Contracts

The configured main WebView starts hidden. Native layout restore/clamp/listener
installation finishes before `mark_window_prepared`. Ordinary Focus and parsed
deep-link effects drain only after both preparation and renderer readiness, in
either arrival order. Existing capacity, focus coalescing and non-waking
rejection semantics remain unchanged. Shell appearance restore is not a reveal
gate: `prepare_main_webview` applies the persisted window theme and the main
page-load start seeds the renderer cache. See
[Blue Appearance](../frontend/appearance.md).

Normal startup, tray open, Dock reopen and lightweight-mode exit use the same
`request_main_window_focus` boundary. Only the effect owner shows/unminimizes/
focuses and restores taskbar/Dock visibility. Lightweight exit clears its mode
flag before submitting Focus, avoiding recursive recreation. New WebView
creation clears window-prepared; ordinary page reload clears renderer-ready
and advances the generation without discarding parsed activations.

Silent startup adds no Focus request and keeps the existing hide/taskbar/Dock
policy. A later explicit activation may show it after readiness. The existing
database-version recovery branch still forces its recovery UI visible and
retains close-to-exit behavior; it is not blocked by this normal-startup policy.

Renderer readiness is a content-commit acknowledgement, not a claim about compositor
paint or native HIL. Agents waits for the local catalog snapshot and Auth for
its local overview to settle; errors are valid presentable outcomes. Do not
wait for all installed-software scans, remote models, accounts' network work,
animation frames or document visibility. The first route module loads before
other optional modules are prefetched. No unconditional success delay.

Those gating queries run while the configured WebView is still `document.hidden`.
TanStack Query's default retry path pauses when `focusManager.isFocused()` is
false (`document.visibilityState === "hidden"`), leaving `isPending` true and
blocking `useFrontendReady(!query.isPending)`. Native `FeatureProvider` therefore
calls `focusManager.setFocused(true)` when creating the QueryClient. Route
isolation still uses `PersistentSurface` / `usePersistentVisibility()` to disable
hidden-page queries; do not treat `document.hidden` as a Query pause signal
before `frontend-deeplink-ready`. Browser previews keep the default focus
manager.

Shared local brand images carry `data-fy-startup-image`. Before the first
acknowledgement, use the browser's `decode()` for already-mounted, non-hidden,
non-lazy same-origin/data artwork. Remote/unmarked images do not gate startup.
Decode rejection is not fatal to usable content; stale completion after hidden
or unmount is ignored. Feature-detect this optional API rather than raising
the native WebView requirement. This is not a new general image loader.

Recovery uses existing Tokio and the native nonblocking dialog API. A 15-second
watchdog is armed only with an unresolved waking activation, once per load.
Expiry never marks ready or shows a loading shell. It offers reload or later;
the callback rechecks generation and unresolved readiness before reloading or
recreating a lightweight WebView. Superseded callbacks cannot reset a newer
watchdog or reload a now-ready page. A silent instance with no wake request
does not display a recovery dialog. Later explicit wake may rearm after a
user declines. No credential, configuration or background service is reset.

## 4. Validation & Error Matrix

| Condition                                            | Result                                                                                                                                            |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Renderer acknowledges before native layout finishes  | Preserve queue; do not show until prepared.                                                                                                       |
| Layout finishes before chunk/local snapshot          | Preserve hidden window; wait for committed surface.                                                                                               |
| Duplicate readiness/Focus                            | No duplicate drain; reuse existing coalescing.                                                                                                    |
| Silent startup without explicit wake                 | No automatic show or watchdog dialog.                                                                                                             |
| Module/local snapshot fails                          | Show recoverable error content when committed. Query must settle to error while the WebView is still hidden; do not remain `fetchStatus: paused`. |
| Catalog/overview fetch fails before reveal           | `useFrontendReady(!isPending)` still acknowledges the error surface; the 15s watchdog is not the success path.                                    |
| Renderer never acknowledges                          | Native recovery dialog for pending wake, not a success reveal.                                                                                    |
| User reloads/destroys WebView while watchdog waits   | Old generation cannot act on replacement.                                                                                                         |
| Renderer becomes ready while recovery dialog is open | Late retry does not reload the ready page.                                                                                                        |
| Post-write/native job continues in background        | Do not stop it for a frontend reload.                                                                                                             |

## 5. Good / Base / Bad Cases

Good: restore native geometry, enqueue Focus, load initial module/local catalog,
commit the directory, then acknowledge; scan cards update afterward.

Base: a failed chunk renders a reloadable error surface and acknowledges that
surface. Browser tests prove ordering, not native compositor timing.

Bad: reveal after a fixed sleep, signal from shell mount, wait for hidden
animation frames, wait for `visibilitychange` to unpause a gating Query retry,
bypass silent mode, or discard queued deep-link semantics.

## 6. Tests Required

Run `mise run typecheck`, `mise run lint`, `mise run test:unit`,
`mise run test:browser`, `mise run build:renderer`, `mise run check:backend`
and the full active-task prearchive gate.

- `lib.rs` tests assert both arrival orders, reload/FIFO, silent/non-waking
  recovery admission, generation supersession, repeated readiness and no
  success reveal from timeout.
- `tests/frontendStartupContract.test.ts` guards entry ordering, no shell
  readiness, centralized native show, failure-only recovery, and native
  `focusManager.setFocused(true)` on the FeatureProvider QueryClient.
- `tests/renderer/platform/featurePorts.test.ts` freezes catalog official-link IDs
  against the native v5 table, including Claude `product` (not Desktop).
- `tests/renderer/platform/frontendReady.test.tsx` guards suspended/hidden content,
  local pending, error fallback and closed hash selection.
- `tests/renderer/pages/agents/Page.test.tsx` delays the local catalog snapshot.
- `tests/browser/startup.spec.ts` delays/aborts the real route import and
  asserts no premature event or loading shell, one readiness signal and
  recovery action across four sizes.
- Native Windows/macOS first-paint, tray/Dock and lightweight interaction must
  be reported separately; portable tests/builds are not real host UX evidence.

## 7. Wrong vs Correct

Wrong: `setup -> window.show()`, `AppShell.useEffect -> ready`, or letting a
gating catalog/overview Query stay `paused` because the still-hidden WebView
reports `document.hidden`.

Correct: `prepare geometry + committed usable/error surface -> activation
queue drain -> show/focus`; native QueryClient pins focus so a failed local
snapshot can settle; failed startup offers explicit bounded recovery.
