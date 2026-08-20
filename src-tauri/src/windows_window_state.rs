//! Windows window-state persistence bound directly to the frozen Shell user.
//!
//! `tauri-plugin-window-state` always resolves and creates the process user's
//! app-config directory before joining its filename. That is incorrect for an
//! elevated Bob-process/Alice-Shell launch, so Windows owns this narrow state
//! file while macOS retains the plugin. The JSON shape intentionally
//! remains compatible with the plugin's existing state file.

#![cfg_attr(target_os = "macos", allow(dead_code))]

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard, OnceLock},
};
use tauri::Manager;

const MAIN_WINDOW_LABEL: &str = "main";
const WINDOW_STATE_MAX_BYTES: usize = 256 * 1024;

static RESTORING_WINDOW_STATE: Mutex<()> = Mutex::new(());

/// Holds the same restoration exclusion across the app-owned restore and the
/// monitor/DPI clamp layered above it. Native move/resize events caused by
/// those programmatic operations must not become the next persisted geometry.
pub(crate) fn suspend_tracking() -> MutexGuard<'static, ()> {
    RESTORING_WINDOW_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn tracking_is_enabled() -> bool {
    RESTORING_WINDOW_STATE.try_lock().is_ok()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct PersistedWindowState {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    // The plugin records the position before the most recent maximize move and
    // uses it when recreating a maximized window. Preserve that behavior and
    // its on-disk fields so upgrades do not discard existing geometry.
    prev_x: i32,
    prev_y: i32,
    // A maximized window can move between monitors without ever exposing a
    // new normal rectangle. Keep that monitor target separately so the
    // plugin-compatible normal and pre-maximize coordinates remain intact.
    // Optional fields keep legacy JSON readable and avoid rewriting its shape
    // until a maximized position has actually been observed.
    #[serde(skip_serializing_if = "Option::is_none")]
    maximized_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximized_y: Option<i32>,
    maximized: bool,
    visible: bool,
    decorated: bool,
    fullscreen: bool,
}

impl Default for PersistedWindowState {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            x: 0,
            y: 0,
            prev_x: 0,
            prev_y: 0,
            maximized_x: None,
            maximized_y: None,
            maximized: false,
            visible: true,
            decorated: true,
            fullscreen: false,
        }
    }
}

type WindowStateMap = HashMap<String, PersistedWindowState>;

fn state_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    crate::windows_runtime::tauri_window_state_path(&app.config().identifier)
}

fn load_state_map(app: &tauri::AppHandle) -> WindowStateMap {
    let path = state_path(app);
    match crate::config::read_bounded_file(&path, WINDOW_STATE_MAX_BYTES) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(states) => states,
            Err(error) => {
                log::warn!("Unable to parse Shell-user window state: {error}");
                WindowStateMap::new()
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => WindowStateMap::new(),
        Err(error) => {
            log::warn!("Unable to read Shell-user window state: {error}");
            WindowStateMap::new()
        }
    }
}

fn state_cache(app: &tauri::AppHandle) -> &'static Mutex<WindowStateMap> {
    static CACHE: OnceLock<Mutex<WindowStateMap>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(load_state_map(app)))
}

fn current_window_state(window: &tauri::WebviewWindow) -> Result<PersistedWindowState, String> {
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let maximized = window.is_maximized().unwrap_or(false);
    Ok(PersistedWindowState {
        width: size.width,
        height: size.height,
        x: position.x,
        y: position.y,
        prev_x: position.x,
        prev_y: position.y,
        maximized_x: maximized.then_some(position.x),
        maximized_y: maximized.then_some(position.y),
        maximized,
        visible: true,
        decorated: true,
        fullscreen: false,
    })
}

fn seed_and_get_state(window: &tauri::WebviewWindow) -> Result<PersistedWindowState, String> {
    let initial = current_window_state(window)?;
    let mut states = state_cache(window.app_handle())
        .lock()
        .map_err(|_| "window-state cache is poisoned".to_owned())?;
    Ok(*states
        .entry(MAIN_WINDOW_LABEL.to_owned())
        .or_insert(initial))
}

fn update_cached_state(window: &tauri::WebviewWindow) -> Result<(), String> {
    let maximized = window.is_maximized().map_err(|error| error.to_string())?;
    let minimized = window.is_minimized().map_err(|error| error.to_string())?;
    let size = (!maximized && !minimized)
        .then(|| window.inner_size().map_err(|error| error.to_string()))
        .transpose()?;
    let position = (!minimized)
        .then(|| window.outer_position().map_err(|error| error.to_string()))
        .transpose()?;

    let mut states = state_cache(window.app_handle())
        .lock()
        .map_err(|_| "window-state cache is poisoned".to_owned())?;
    let state = states.entry(MAIN_WINDOW_LABEL.to_owned()).or_default();
    record_maximized_state(state, maximized);
    if let Some(size) = size.filter(|size| size.width > 0 && size.height > 0) {
        state.width = size.width;
        state.height = size.height;
    }
    if let Some(position) = position {
        record_window_position(state, position.x, position.y, false, maximized, true);
    }
    Ok(())
}

fn record_maximized_state(state: &mut PersistedWindowState, maximized: bool) {
    if maximized && !state.maximized {
        state.prev_x = state.x;
        state.prev_y = state.y;
    }
    state.maximized = maximized;
}

fn record_normal_position(
    state: &mut PersistedWindowState,
    x: i32,
    y: i32,
    minimized: bool,
    maximized: bool,
    tracking_enabled: bool,
) {
    if minimized || maximized || !tracking_enabled {
        return;
    }
    state.x = x;
    state.y = y;
    state.prev_x = x;
    state.prev_y = y;
}

fn record_maximized_position(
    state: &mut PersistedWindowState,
    x: i32,
    y: i32,
    minimized: bool,
    maximized: bool,
    tracking_enabled: bool,
) {
    if minimized || !maximized || !tracking_enabled {
        return;
    }
    state.maximized_x = Some(x);
    state.maximized_y = Some(y);
}

fn record_window_position(
    state: &mut PersistedWindowState,
    x: i32,
    y: i32,
    minimized: bool,
    maximized: bool,
    tracking_enabled: bool,
) {
    record_normal_position(state, x, y, minimized, maximized, tracking_enabled);
    record_maximized_position(state, x, y, minimized, maximized, tracking_enabled);
}

fn restore_position(state: &PersistedWindowState) -> (i32, i32) {
    if state.maximized {
        match (state.maximized_x, state.maximized_y) {
            (Some(x), Some(y)) => (x, y),
            _ => (state.prev_x, state.prev_y),
        }
    } else {
        (state.x, state.y)
    }
}

fn install_tracking(window: &tauri::WebviewWindow) {
    let tracked_window = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { .. } => {
            let _ = update_cached_state(&tracked_window);
        }
        tauri::WindowEvent::Moved(position) => {
            let minimized = tracked_window.is_minimized().unwrap_or_default();
            let maximized = tracked_window.is_maximized().unwrap_or_default();
            let tracking_enabled = tracking_is_enabled();
            if let Ok(mut states) = state_cache(tracked_window.app_handle()).lock() {
                let state = states.entry(MAIN_WINDOW_LABEL.to_owned()).or_default();
                record_window_position(
                    state,
                    position.x,
                    position.y,
                    minimized,
                    maximized,
                    tracking_enabled,
                );
            }
        }
        tauri::WindowEvent::Resized(size)
            if size.width > 0
                && size.height > 0
                && tracking_is_enabled()
                && !tracked_window.is_minimized().unwrap_or_default()
                && !tracked_window.is_maximized().unwrap_or_default() =>
        {
            if let Ok(mut states) = state_cache(tracked_window.app_handle()).lock() {
                let state = states.entry(MAIN_WINDOW_LABEL.to_owned()).or_default();
                state.width = size.width;
                state.height = size.height;
            }
        }
        _ => {}
    });
}

pub(crate) fn restore(window: &tauri::WebviewWindow) -> Result<(), String> {
    let state = seed_and_get_state(window)?;
    install_tracking(window);
    if state.width > 0 && state.height > 0 {
        let (x, y) = restore_position(&state);
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())?;
        window
            .set_size(tauri::PhysicalSize::new(state.width, state.height))
            .map_err(|error| error.to_string())?;
        if state.maximized {
            window.maximize().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn save(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        update_cached_state(&window)?;
    }

    let encoded = {
        let states = state_cache(app)
            .lock()
            .map_err(|_| "window-state cache is poisoned".to_owned())?;
        serde_json::to_vec_pretty(&*states).map_err(|error| error.to_string())?
    };
    let path = state_path(app);
    crate::config::atomic_write(&path, &encoded).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        record_maximized_state, record_normal_position, record_window_position, restore_position,
        PersistedWindowState, WindowStateMap,
    };

    #[test]
    fn existing_plugin_state_shape_round_trips() {
        let states: WindowStateMap = serde_json::from_str(
            r#"{"main":{"width":1232,"height":700,"x":10,"y":20,"prev_x":30,"prev_y":40,"maximized":true,"visible":true,"decorated":true,"fullscreen":false}}"#,
        )
        .unwrap();
        assert_eq!(states["main"].prev_x, 30);
        assert!(states["main"].maximized);
        assert_eq!(states["main"].maximized_x, None);
        assert_eq!(restore_position(&states["main"]), (30, 40));

        let encoded = serde_json::to_string(&states).unwrap();
        assert!(!encoded.contains("maximized_x"));
        assert!(!encoded.contains("maximized_y"));
        let decoded: WindowStateMap = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, states);
    }

    #[test]
    fn missing_plugin_fields_use_safe_defaults() {
        let state: PersistedWindowState =
            serde_json::from_str(r#"{"width":100,"height":100}"#).unwrap();
        assert!(state.visible);
        assert!(state.decorated);
        assert!(!state.maximized);
    }

    #[test]
    fn restore_and_maximize_events_do_not_replace_pre_maximize_geometry() {
        let mut state = PersistedWindowState {
            x: -8,
            y: -8,
            prev_x: 500,
            prev_y: 300,
            maximized: true,
            ..PersistedWindowState::default()
        };

        record_normal_position(&mut state, 500, 300, false, false, false);
        record_normal_position(&mut state, -8, -8, false, true, true);

        assert_eq!((state.x, state.y), (-8, -8));
        assert_eq!((state.prev_x, state.prev_y), (500, 300));
    }

    #[test]
    fn maximizing_without_an_intervening_move_uses_the_latest_normal_geometry() {
        let mut state = PersistedWindowState {
            x: 500,
            y: 300,
            prev_x: 100,
            prev_y: 80,
            maximized: false,
            ..PersistedWindowState::default()
        };

        record_maximized_state(&mut state, true);

        assert!(state.maximized);
        assert_eq!((state.prev_x, state.prev_y), (500, 300));
        assert_eq!(restore_position(&state), (500, 300));
    }

    #[test]
    fn maximized_move_from_monitor_a_to_b_preserves_the_normal_rectangle() {
        let mut state = PersistedWindowState {
            width: 1232,
            height: 700,
            x: 320,
            y: 180,
            prev_x: 320,
            prev_y: 180,
            ..PersistedWindowState::default()
        };

        record_maximized_state(&mut state, true);
        record_window_position(&mut state, -8, -8, false, true, true);
        record_window_position(&mut state, 1912, -8, false, true, true);

        assert_eq!((state.x, state.y), (320, 180));
        assert_eq!((state.prev_x, state.prev_y), (320, 180));
        assert_eq!(
            (state.maximized_x, state.maximized_y),
            (Some(1912), Some(-8))
        );
        assert_eq!(restore_position(&state), (1912, -8));
    }

    #[test]
    fn maximized_exit_refreshes_the_target_without_replacing_normal_geometry() {
        let mut state = PersistedWindowState {
            x: 320,
            y: 180,
            prev_x: 320,
            prev_y: 180,
            maximized: true,
            maximized_x: Some(-8),
            maximized_y: Some(-8),
            ..PersistedWindowState::default()
        };

        record_window_position(&mut state, 2552, -8, false, true, true);

        assert_eq!((state.x, state.y), (320, 180));
        assert_eq!((state.prev_x, state.prev_y), (320, 180));
        assert_eq!(restore_position(&state), (2552, -8));
    }

    #[test]
    fn disconnected_maximized_target_is_forwarded_to_existing_monitor_fallback() {
        let state = PersistedWindowState {
            x: 320,
            y: 180,
            prev_x: 320,
            prev_y: 180,
            maximized: true,
            maximized_x: Some(6000),
            maximized_y: Some(100),
            ..PersistedWindowState::default()
        };

        // The layout layer deliberately detects that this coordinate has no
        // current monitor, selects its primary/first-monitor fallback, and
        // clamps before re-maximizing. Keeping the disconnected coordinate
        // here lets that existing policy make the decision.
        assert_eq!(restore_position(&state), (6000, 100));
    }
}
