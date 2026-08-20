# Main Window Layout Contract

## 1. Scope / Trigger

Read this contract before changing `src-tauri/src/window_layout.rs`,
`restore_hidden_main_window_layout`, `refresh_main_window_layout`, the
`Moved` / DPI layout listener in `src-tauri/src/lib.rs`, or Windows
`windows_window_state` restore/save.

The V2 app-shell Overlay chrome is a renderer React widget under
`src/v2/widgets/app-shell/` (window chrome, not a feature route). The
[V2 Shell Contract](../frontend/v2-shell.md) owns that composition,
including the macOS Overlay drag strip. **V2-owned chrome** means that
React widget only. It does not mean V2 owns host geometry, and it does
not place Overlay outside the V2 tree.

This is a host geometry contract. Do not fix native window overflow in
Overlay, React, or CSS. The V2 shell does not own maximize, restore,
min-size, or work-area clamping.

## 2. Signatures

```rust
pub const LAYOUT_VERSION: u32 = 2;
pub const TARGET_MIN_WIDTH: f64 = 1152.0;
pub const TARGET_MIN_HEIGHT: f64 = 640.0;
pub const MAXIMUM_WORK_AREA_SHARE: f64 = 0.9;

pub fn should_apply_runtime_geometry_constraints(
    maximized: bool,
    fullscreen: bool,
) -> bool;

pub fn layout_mode(work_area_width: f64) -> LayoutMode; // Normal | Constrained
pub fn clamp_window_geometry(saved: WindowGeometry, work_area: LogicalWorkArea)
    -> WindowGeometry;
```

Host event (string payload, not a JSON object):

```text
event: layout-mode-changed
payload: "normal" | "constrained"
debounce: 150ms after Moved or ScaleFactorChanged
```

`refresh_main_window_layout` may call `set_min_size` only when
`should_apply_runtime_geometry_constraints` is true. It must still emit
`layout-mode-changed`. It must not call `set_size` or `set_position`.

## 3. Contracts

- Product “fullscreen” on Windows is **system maximize** (work area, taskbar
  stays). Exclusive fullscreen (`setFullscreen` / F11) is out of scope.
- `should_apply_runtime_geometry_constraints` is `false` when the window is
  maximized or exclusive-fullscreen. Runtime `set_min_size`, `set_size`, and
  `set_position` must not run in that state.
- Windows maximize emits `Moved` near `(-8,-8)` / `(-9,-9)` to hide resize
  borders. The outer rect can be a few pixels larger than the work area.
  That is normal; it is not overflow to “fix” from the renderer.
- On Windows, `set_min_size` while maximized unmaximizes the window but
  **keeps the maximized client size**, then jumps to the previous normal
  origin. Visible UI then sits partly off-screen.
- `MAXIMUM_WORK_AREA_SHARE` (90%) clamps **normal** rectangles only. Do not
  rewrite a maximized client area into a 90% pseudo-maximized window.
- macOS keeps the native Tauri title bar (`titleBarStyle: Overlay`). That
  native Overlay style is host chrome. The V2 app-shell Overlay widget may
  add the inert drag strip in the React tree; it is not a feature page and
  it is not outside V2. Windows keeps a Visible system title bar and no
  Overlay drag strip. Do not switch either `titleBarStyle` to compensate
  for drag or maximize bugs, and do not treat the React Overlay chrome as
  a geometry owner.
- Windows persistence stays in Shell-user `windows_window_state`; macOS stays
  on `tauri-plugin-window-state`. Do not change the JSON field set unless
  runtime evidence proves a missing field.

## 4. Validation & Error Matrix

| Condition | Required result |
| --------- | --------------- |
| `maximized == true` or `fullscreen == true` | `should_apply_runtime_geometry_constraints` is false; no `set_min_size` / `set_size` / `set_position` |
| User maximizes on Windows | Window stays maximized; origin stays the system maximize origin (about `-8,-8`); UI remains in the work area |
| User restores from maximize | Previous normal rectangle returns; not the maximized client size at the old origin |
| `Moved` / DPI while maximized | Emit `layout-mode-changed` only; skip geometry mutation |
| Saved geometry off-screen or non-finite | `clamp_window_geometry` keeps `maximized` and fits the **normal** rect into the work area |
| Layout refresh error | Log at debug; do not panic or force-unmaximize |

## 5. Good / Base / Bad Cases

- **Good:** Windows maximize at 125% DPI (`workArea` 2048×1240 logical) stays
  `maximized: true` after the 150ms refresh; `applyGeometry` / min-size is
  skipped.
- **Base:** A normal window on a large work area gets `set_min_size(1152,640)`
  after move/DPI so the product minimum returns.
- **Bad:** `refresh_main_window_layout` always calls `set_min_size`. On
  Windows this unmaximizes, keeps inner `2560×1521`, and teleports to the
  last normal origin (for example `208,182`), overflowing right and bottom.

## 6. Tests Required

```bash
mise run rust:fmt:check
mise run rust:test
```

- `src-tauri/tests/window_layout.rs` must assert
  `should_apply_runtime_geometry_constraints(false, false) == true` and that
  maximized or fullscreen is `false`.
- `mise run rust:test` on macOS does **not** prove Windows maximize. Do not
  report `test:desktop:mock`, Playwright, or a Windows-target compile as
  maximize-overflow evidence.
- A Windows maximize/restore check needs a native Windows run. Residual
  risk remains until that evidence exists for the current change.

## 7. Wrong vs Correct

#### Wrong

```rust
window.set_min_size(Some(LogicalSize::new(minimum.width, minimum.height)))?;
```

Call this from the `Moved` debounce while `is_maximized()` is true.

#### Correct

```rust
if window_layout::should_apply_runtime_geometry_constraints(maximized, fullscreen) {
    window.set_min_size(Some(LogicalSize::new(minimum.width, minimum.height)))?;
}
emit_main_window_layout_mode(window, work_area, scale_factor)
```
