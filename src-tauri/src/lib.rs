pub mod agent_install;
mod app_config;
mod app_store;
mod auto_launch;
mod claude_desktop_config;
mod claude_mcp;
mod claude_plugin;
mod codex_config;
pub mod codex_desktop;
mod codex_desktop_runtime;
mod codex_history_migration;
mod codex_state_db;
mod commands;
mod config;
mod database;
mod deeplink;
mod error;
mod gemini_config;
mod gemini_mcp;
mod grok_config;
pub mod hermes_config;
mod init_status;
mod lightweight;
#[cfg(target_os = "linux")]
mod linux_fix;
mod mcp;
mod model_capabilities;
mod openclaw_config;
mod opencode_config;
mod panic_hook;
mod platform;
mod prompt;
mod prompt_files;
mod provider;
mod proxy;
mod services;
mod session_manager;
mod settings;
mod store;

mod tray;
mod usage_events;
mod usage_script;
mod window_layout;
mod windows_runtime;
#[cfg(any(target_os = "windows", test))]
mod windows_window_state;

use crate::codex_desktop::{
    jobs::{ProcessLifecycleClaim, ProcessLifecycleCoordinator, ProcessLifecycleTransition},
    types::JobStage,
};
pub use app_config::{AppType, InstalledSkill, McpApps, McpServer, MultiAppConfig, SkillApps};
pub use codex_config::{
    get_codex_auth_path, get_codex_config_path, read_codex_live_settings, write_codex_live_atomic,
};
pub use commands::open_provider_terminal;
pub use commands::*;
pub use config::{get_claude_mcp_path, get_claude_settings_path, read_json_file};
pub use database::{Database, Profile};
pub use deeplink::{import_provider_from_deeplink, parse_deeplink_url, DeepLinkImportRequest};
pub use error::AppError;
pub use grok_config::get_grok_config_path;
pub use mcp::{
    import_from_claude, import_from_codex, import_from_gemini, import_from_grokbuild,
    remove_server_from_claude, remove_server_from_codex, remove_server_from_gemini,
    remove_server_from_grokbuild, sync_enabled_to_claude, sync_enabled_to_codex,
    sync_enabled_to_gemini, sync_single_server_to_claude, sync_single_server_to_codex,
    sync_single_server_to_gemini, sync_single_server_to_grokbuild,
};
pub use prompt::Prompt;
pub use provider::{Provider, ProviderMeta};
pub use services::{
    profile::{ProfilePayload, ProfileScope, ProfileService},
    provider::reapply_current_codex_official_live,
    skill::{migrate_skills_to_ssot, ImportSkillSelection},
    ConfigService, EndpointLatency, McpService, PromptService, ProviderService, ProxyService,
    SkillService, SpeedtestService,
};
pub use settings::{update_settings, AppSettings};
pub use store::AppState;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
pub use windows_runtime::{initialize_windows_user_context, WindowsStartupErrorCode};

use std::{
    collections::VecDeque,
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::RunEvent;
use tauri::{Emitter, Listener, Manager};
#[cfg(not(target_os = "windows"))]
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

#[cfg(target_os = "windows")]
fn set_windows_app_user_model_id(app: &tauri::AppHandle) {
    let app_id = app.config().identifier.clone();
    let wide_app_id: Vec<u16> = app_id.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(wide_app_id.as_ptr())
    };

    if result < 0 {
        log::warn!("设置 Windows AppUserModelID 失败: 0x{result:08X}");
    } else {
        log::debug!("Windows AppUserModelID 已设置为 {app_id}");
    }
}

pub(crate) struct RedactedUrl<'a> {
    url: &'a str,
    known_secrets: &'a [String],
}

impl fmt::Display for RedactedUrl<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&redact_url_for_log_with_secrets(
            self.url,
            self.known_secrets,
        ))
    }
}

/// 为持有确切认证材料的调用方提供优先精确匹配、再启发式兜底的 URL 脱敏。
pub(crate) fn url_for_log_with_secrets<'a>(
    url: &'a str,
    known_secrets: &'a [String],
) -> RedactedUrl<'a> {
    RedactedUrl { url, known_secrets }
}

/// 已知密钥参与子串脱敏的最短长度：过短的值(如 "api")当作子串会误伤无关文本，
/// 所以只对足够长、几乎不可能是普通词的值做替换。
const MIN_KNOWN_SECRET_LEN: usize = 8;

/// 唯一的密钥脱敏原语：把字符串里出现的、我们确切握有的密钥值替换为 [REDACTED]。
/// 不做任何“看起来像密钥”的形状猜测——只隐藏已知值，天然收敛、不误伤正常路径。
fn redact_known_secrets(text: &str, known_secrets: &[String]) -> String {
    let mut output = text.to_string();
    for secret in known_secrets {
        if secret.chars().count() >= MIN_KNOWN_SECRET_LEN {
            output = output.replace(secret.as_str(), "[REDACTED]");
        }
    }
    output
}

/// 无 scheme 的裸 authority 形态(如 `user:pass@host/path`)剥掉 userinfo：
/// 仅当 `@` 出现在第一个 `/` 之前时才视为凭据。
fn strip_bare_userinfo(input: &str) -> &str {
    let authority_end = input.find('/').unwrap_or(input.len());
    match input[..authority_end].rfind('@') {
        Some(at) => &input[at + 1..],
        None => input,
    }
}

pub(crate) fn redact_url_for_log(url_str: &str) -> String {
    redact_url_for_log_with_secrets(url_str, &[])
}

/// 为日志脱敏 URL：剥掉 userinfo(user:pass@) 与整个 query/fragment，保留
/// scheme/host/port/path 供诊断(如 base_url 配错路径导致 404)，最后再抹掉已知密钥值。
pub(crate) fn redact_url_for_log_with_secrets(url_str: &str, known_secrets: &[String]) -> String {
    let scheme_relative = url_str.starts_with("//");
    let parsed = if scheme_relative {
        url::Url::parse(&format!("https:{url_str}"))
    } else {
        url::Url::parse(url_str)
    };

    let sanitized = match parsed {
        Ok(mut url) if url.has_host() => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            let rendered = url.as_str();
            if scheme_relative {
                rendered
                    .strip_prefix("https:")
                    .unwrap_or(rendered)
                    .to_string()
            } else {
                rendered.to_string()
            }
        }
        _ => {
            // 解析失败(相对路径、含裸 userinfo 的非法 URL 等)：丢掉 query/fragment，
            // 尽力剥掉 userinfo，其余原样保留。
            let without_tail = url_str.split(['?', '#']).next().unwrap_or(url_str);
            strip_bare_userinfo(without_tail).to_string()
        }
    };

    redact_known_secrets(&sanitized, known_secrets)
}

/// 只保留 `scheme://host:port`，丢掉 path/query/userinfo。用于我们手里没有任何
/// 已知密钥可脱敏 path 的场景——凭据可能整个内嵌在 base_url 的 path 里，此时
/// 记录 path 无法保证不泄漏，只能退回到 origin。
pub(crate) fn redact_url_origin_for_log(url_str: &str) -> String {
    let scheme_relative = url_str.starts_with("//");
    let parsed = if scheme_relative {
        url::Url::parse(&format!("https:{url_str}"))
    } else {
        url::Url::parse(url_str)
    };

    match parsed {
        Ok(url) if url.has_host() => {
            let authority = &url[url::Position::BeforeHost..url::Position::AfterPort];
            if scheme_relative {
                format!("//{authority}")
            } else {
                format!("{}://{authority}", url.scheme())
            }
        }
        _ => "[invalid target]".to_string(),
    }
}

fn runtime_log_level_allows(level: log::Level, max_level: log::LevelFilter) -> bool {
    max_level.to_level().is_some_and(|maximum| level <= maximum)
}

const WINDOW_LAYOUT_EVENT: &str = "layout-mode-changed";
const WINDOW_LAYOUT_DEBOUNCE: Duration = Duration::from_millis(150);

fn layout_mode_label(mode: window_layout::LayoutMode) -> &'static str {
    match mode {
        window_layout::LayoutMode::Normal => "normal",
        window_layout::LayoutMode::Constrained => "constrained",
    }
}

/// Converts the monitor work area to logical pixels before the layout policy
/// sees it. The policy is deliberately independent of monitor identity so the
/// diagnostic path never needs to retain a display name or serial number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalMonitorWorkArea {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

fn physical_monitor_work_area(monitor: &tauri::Monitor) -> PhysicalMonitorWorkArea {
    let work_area = monitor.work_area();
    PhysicalMonitorWorkArea {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width,
        height: work_area.size.height,
    }
}

fn fallback_monitor_index(
    available: &[PhysicalMonitorWorkArea],
    primary: Option<PhysicalMonitorWorkArea>,
) -> Option<usize> {
    primary
        .and_then(|primary| available.iter().position(|candidate| *candidate == primary))
        .or_else(|| (!available.is_empty()).then_some(0))
}

fn selected_window_monitor(window: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    match window.current_monitor() {
        Ok(Some(monitor)) => return Some(monitor),
        Ok(None) => {
            log::debug!(
                "Main window is not on a connected monitor; selecting a safe restore target"
            );
        }
        Err(error) => {
            log::debug!("Unable to read main-window monitor for layout policy: {error}");
        }
    }

    let primary_monitor = match window.primary_monitor() {
        Ok(primary) => primary,
        Err(error) => {
            log::debug!("Unable to identify the primary monitor for layout policy: {error}");
            None
        }
    };
    let available = match window.available_monitors() {
        Ok(monitors) if !monitors.is_empty() => monitors,
        Ok(_) => {
            log::debug!("No connected monitor is available for main-window layout policy");
            return primary_monitor;
        }
        Err(error) => {
            log::debug!("Unable to enumerate monitors for main-window layout policy: {error}");
            return primary_monitor;
        }
    };
    let primary = primary_monitor.as_ref().map(physical_monitor_work_area);
    let available_work_areas = available
        .iter()
        .map(physical_monitor_work_area)
        .collect::<Vec<_>>();
    let selected = fallback_monitor_index(&available_work_areas, primary)?;
    Some(available[selected].clone())
}

fn current_logical_work_area(
    window: &tauri::WebviewWindow,
) -> Option<(window_layout::LogicalWorkArea, f64)> {
    let monitor = selected_window_monitor(window)?;
    let scale_factor = monitor.scale_factor();
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        log::debug!("Ignoring invalid main-window scale factor for layout policy");
        return None;
    }

    let work_area = monitor.work_area();
    Some((
        window_layout::LogicalWorkArea {
            x: f64::from(work_area.position.x) / scale_factor,
            y: f64::from(work_area.position.y) / scale_factor,
            width: f64::from(work_area.size.width) / scale_factor,
            height: f64::from(work_area.size.height) / scale_factor,
        },
        scale_factor,
    ))
}

fn emit_main_window_layout_mode(
    window: &tauri::WebviewWindow,
    work_area: window_layout::LogicalWorkArea,
    scale_factor: f64,
) -> tauri::Result<()> {
    let mode = window_layout::layout_mode(work_area.width);
    window.emit(WINDOW_LAYOUT_EVENT, layout_mode_label(mode))?;
    log::debug!(
        "Main window layout policy v{} updated: logical_work_area_width={:.0}, scale_factor={scale_factor:.2}, mode={}",
        window_layout::LAYOUT_VERSION,
        work_area.width,
        layout_mode_label(mode),
    );
    Ok(())
}

/// Re-evaluates only the current monitor constraint. It intentionally does not
/// reset size or position: after returning to a large work area a legal user
/// size stays untouched, while the product minimum becomes available again.
fn refresh_main_window_layout(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let Some((work_area, scale_factor)) = current_logical_work_area(window) else {
        return Ok(());
    };
    let minimum = window_layout::effective_minimum_size(work_area);
    window.set_min_size(Some(tauri::LogicalSize::new(minimum.width, minimum.height)))?;
    emit_main_window_layout_mode(window, work_area, scale_factor)
}

/// Restores the main window while it is still hidden, normalizes legacy saved
/// geometry, and only then restores maximization. `window-state` continues to
/// own persistence; this is a controlled migration layer above its raw state.
fn restore_hidden_main_window_layout(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "windows")]
    let _tracking_suspension = crate::windows_window_state::suspend_tracking();

    #[cfg(target_os = "windows")]
    if let Err(error) = crate::windows_window_state::restore(window) {
        log::warn!("Unable to restore Shell-user main-window state: {error}");
    }

    #[cfg(not(target_os = "windows"))]
    if let Err(error) = window.restore_state(window_state_flags()) {
        // A corrupt or unavailable persisted state must not block startup.
        log::warn!("Unable to restore saved main-window state; using current geometry: {error}");
    }

    let Some((work_area, scale_factor)) = current_logical_work_area(window) else {
        return Ok(());
    };

    let was_maximized = window.is_maximized().unwrap_or(false);
    if was_maximized {
        window.unmaximize()?;
    }

    let size = window.inner_size()?;
    let position = window.outer_position()?;
    let geometry = window_layout::clamp_window_geometry(
        window_layout::WindowGeometry {
            x: f64::from(position.x) / scale_factor,
            y: f64::from(position.y) / scale_factor,
            width: f64::from(size.width) / scale_factor,
            height: f64::from(size.height) / scale_factor,
            maximized: was_maximized,
        },
        work_area,
    );
    let minimum = window_layout::effective_minimum_size(work_area);

    window.set_min_size(Some(tauri::LogicalSize::new(minimum.width, minimum.height)))?;
    window.set_size(tauri::LogicalSize::new(geometry.width, geometry.height))?;
    window.set_position(tauri::LogicalPosition::new(geometry.x, geometry.y))?;
    if geometry.maximized {
        window.maximize()?;
    }
    emit_main_window_layout_mode(window, work_area, scale_factor)
}

/// Coalesces monitor/DPI changes so transient move events cannot repeatedly
/// apply constraints or flood the renderer with layout-mode notifications.
fn install_main_window_layout_listener(window: &tauri::WebviewWindow) {
    let generation = Arc::new(AtomicU64::new(0));
    let window_for_events = window.clone();

    window.on_window_event(move |event| {
        if !matches!(
            event,
            tauri::WindowEvent::Moved(_) | tauri::WindowEvent::ScaleFactorChanged { .. }
        ) {
            return;
        }

        let revision = generation.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
        let generation = generation.clone();
        let window = window_for_events.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(WINDOW_LAYOUT_DEBOUNCE).await;
            if generation.load(Ordering::Acquire) != revision {
                return;
            }
            if let Err(error) = refresh_main_window_layout(&window) {
                log::debug!("Unable to refresh main-window layout after display change: {error}");
            }
        });
    });
}

pub(crate) fn prepare_main_webview(window: &tauri::WebviewWindow) {
    if let Err(error) = restore_hidden_main_window_layout(window) {
        log::warn!("Unable to apply main-window layout policy: {error}");
    }
    install_main_window_layout_listener(window);
}

const FRONTEND_DEEPLINK_READY_EVENT: &str = "frontend-deeplink-ready";
const MAX_PENDING_ACTIVATIONS: usize = 16;

#[derive(Debug, Clone)]
enum PendingActivation {
    Focus,
    InvalidDeepLink {
        focus_main_window: bool,
    },
    DeepLink {
        request: Box<crate::deeplink::DeepLinkImportRequest>,
        focus_main_window: bool,
    },
}

impl PendingActivation {
    fn should_wake_main_window(&self) -> bool {
        match self {
            Self::Focus => true,
            Self::InvalidDeepLink { focus_main_window }
            | Self::DeepLink {
                focus_main_window, ..
            } => *focus_main_window,
        }
    }
}

fn should_exit_lightweight_mode(is_lightweight: bool, activation: &PendingActivation) -> bool {
    is_lightweight && activation.should_wake_main_window()
}

#[derive(Debug, Default)]
struct ActivationInbox {
    renderer_ready: bool,
    draining: bool,
    pending: VecDeque<PendingActivation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivationEnqueueResult {
    Queued,
    StartDrain,
    Coalesced,
    RejectedAtCapacity,
}

impl ActivationInbox {
    fn enqueue(&mut self, activation: PendingActivation) -> ActivationEnqueueResult {
        if matches!(activation, PendingActivation::Focus)
            && self
                .pending
                .iter()
                .any(|queued| matches!(queued, PendingActivation::Focus))
        {
            return ActivationEnqueueResult::Coalesced;
        }
        if self.pending.len() >= MAX_PENDING_ACTIVATIONS {
            let evictable = if activation.should_wake_main_window() {
                self.pending
                    .iter()
                    .position(|queued| !queued.should_wake_main_window())
            } else {
                None
            };
            if let Some(index) = evictable {
                let _ = self.pending.remove(index);
            } else {
                return ActivationEnqueueResult::RejectedAtCapacity;
            }
        }

        self.pending.push_back(activation);
        if self.renderer_ready && !self.draining {
            self.draining = true;
            ActivationEnqueueResult::StartDrain
        } else {
            ActivationEnqueueResult::Queued
        }
    }

    fn mark_ready(&mut self) -> bool {
        self.renderer_ready = true;
        if self.pending.is_empty() || self.draining {
            false
        } else {
            self.draining = true;
            true
        }
    }

    fn mark_unready(&mut self) {
        self.renderer_ready = false;
    }

    fn take_next(&mut self) -> Option<PendingActivation> {
        if !self.renderer_ready {
            self.draining = false;
            return None;
        }
        let next = self.pending.pop_front();
        if next.is_none() {
            self.draining = false;
        }
        next
    }
}

fn activation_inbox() -> &'static Mutex<ActivationInbox> {
    static INBOX: OnceLock<Mutex<ActivationInbox>> = OnceLock::new();
    INBOX.get_or_init(|| Mutex::new(ActivationInbox::default()))
}

fn with_activation_inbox<T>(f: impl FnOnce(&mut ActivationInbox) -> T) -> T {
    let mut inbox = activation_inbox()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut inbox)
}

pub(crate) fn mark_activation_renderer_unready() {
    with_activation_inbox(ActivationInbox::mark_unready);
}

fn show_and_focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(target_os = "linux")]
        {
            linux_fix::nudge_main_window(window.clone());
        }
    }
}

fn dispatch_activation(app: &tauri::AppHandle, activation: PendingActivation) {
    match activation {
        PendingActivation::Focus => show_and_focus_main_window(app),
        PendingActivation::InvalidDeepLink {
            focus_main_window: should_focus,
        } => {
            emit_safe_deeplink_error(app);
            if should_focus {
                show_and_focus_main_window(app);
            }
        }
        PendingActivation::DeepLink {
            request,
            focus_main_window: should_focus,
        } => {
            emit_deeplink_request(app, &request, false);
            if should_focus {
                show_and_focus_main_window(app);
            }
        }
    }
}

fn drain_pending_activations(app: &tauri::AppHandle) {
    loop {
        let next = with_activation_inbox(ActivationInbox::take_next);
        let Some(activation) = next else {
            return;
        };
        dispatch_activation(app, activation);
    }
}

fn mark_activation_renderer_ready(app: &tauri::AppHandle) {
    if with_activation_inbox(ActivationInbox::mark_ready) {
        drain_pending_activations(app);
    }
}

fn activation_ready_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("activation-ready")
        .setup(|app, _api| {
            let activation_app = app.clone();
            app.listen(FRONTEND_DEEPLINK_READY_EVENT, move |_| {
                mark_activation_renderer_ready(&activation_app);
            });
            Ok(())
        })
        .build()
}

fn submit_activation(app: &tauri::AppHandle, activation: PendingActivation) {
    let was_lightweight = crate::lightweight::is_lightweight_mode();
    let should_exit_lightweight = should_exit_lightweight_mode(was_lightweight, &activation);
    if was_lightweight {
        // A destroyed WebView cannot still own a live listener even if an old
        // ready event raced with the native single-instance callback. A
        // non-focusing rejection remains queued without rebuilding the UI.
        mark_activation_renderer_unready();
    }

    match with_activation_inbox(|inbox| inbox.enqueue(activation)) {
        ActivationEnqueueResult::StartDrain => drain_pending_activations(app),
        ActivationEnqueueResult::Queued | ActivationEnqueueResult::Coalesced => {}
        ActivationEnqueueResult::RejectedAtCapacity => {
            // A local sender can deliberately fill this queue while the
            // renderer is unavailable. Rejecting excess semantics after a
            // waking activation has tried to displace one non-waking item is
            // the bounded DoS policy. Capacity never suppresses the separate
            // decision to wake an existing or rebuilt main window.
            log::warn!(
                "Rejected a semantic activation because the bounded renderer-ready queue is full"
            );
        }
    }

    if should_exit_lightweight {
        if let Err(error) = crate::lightweight::exit_lightweight_mode(app) {
            log::error!("退出轻量模式重建窗口失败: {error}");
        }
    }
}

fn emit_safe_deeplink_error(app: &tauri::AppHandle) {
    // Do not expose the rejected URL or parser diagnostic. Deep links may
    // legitimately carry credentials, and the renderer only needs a safe,
    // localized failure category.
    if let Err(error) = app.emit(
        "deeplink-error",
        serde_json::json!({ "code": "invalid_deeplink" }),
    ) {
        log::error!("Failed to emit safe deep-link error event: {error}");
    }
}

fn emit_deeplink_request(
    app: &tauri::AppHandle,
    request: &crate::deeplink::DeepLinkImportRequest,
    focus_main_window: bool,
) {
    log::info!("Successfully parsed deep link request");

    if let Err(e) = app.emit("deeplink-import", request) {
        log::error!("✗ Failed to emit deeplink-import event: {e}");
    } else {
        log::info!("✓ Emitted deeplink-import event to frontend");
    }

    if focus_main_window {
        show_and_focus_main_window(app);
        log::info!("✓ Window shown and focused");
    }
}

/// 统一处理 fyagent:// 深链接 URL
///
/// - 解析 URL
/// - 向前端发射 `deeplink-import` / `deeplink-error` 事件
/// - 可选：在成功时聚焦主窗口
fn handle_deeplink_url(
    app: &tauri::AppHandle,
    url_str: &str,
    focus_main_window: bool,
    source: &str,
) -> bool {
    if !url_str.starts_with("fyagent://") {
        return false;
    }

    log::info!("Deep link URL detected from {source}");

    match crate::deeplink::parse_deeplink_url(url_str) {
        Ok(request) => submit_activation(
            app,
            PendingActivation::DeepLink {
                request: Box::new(request),
                focus_main_window,
            },
        ),
        Err(_) => {
            log::warn!("Rejected invalid deep link from {source}");
            submit_activation(
                app,
                PendingActivation::InvalidDeepLink { focus_main_window },
            );
        }
    }

    true
}

/// Builds the configured main WebView. Windows supplies the frozen Shell
/// user's absolute LocalAppData directory after disabling Tauri's automatic
/// config-window creation; this bypasses the elevated process path resolver.
pub(crate) fn create_main_webview(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .ok_or_else(|| tauri::Error::Io(std::io::Error::other("main window config missing")))?;

    let builder = tauri::WebviewWindowBuilder::from_config(app, &window_config)?;
    #[cfg(target_os = "windows")]
    let builder = builder.data_directory(crate::windows_runtime::webview_user_data_dir(
        &app.config().identifier,
    ));
    builder.build()
}

/// 更新托盘菜单的Tauri命令
#[tauri::command]
async fn update_tray_menu(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    match tray::create_tray_menu(&app, state.inner()) {
        Ok(new_menu) => {
            if let Some(tray) = app.tray_by_id(tray::TRAY_ID) {
                tray.set_menu(Some(new_menu))
                    .map_err(|e| format!("更新托盘菜单失败: {e}"))?;
                return Ok(true);
            }
            Ok(false)
        }
        Err(err) => {
            log::error!("创建托盘菜单失败: {err}");
            Ok(false)
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_tray_icon() -> Option<Image<'static>> {
    const ICON_BYTES: &[u8] = include_bytes!("../icons/tray/macos/statusbar_template_3x.png");

    match Image::from_bytes(ICON_BYTES) {
        Ok(icon) => Some(icon),
        Err(err) => {
            log::warn!("Failed to load macOS tray icon: {err}");
            None
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 设置 panic hook，在应用崩溃时记录日志到 <app_config_dir>/crash.log（默认 ~/.fyagent/crash.log）
    panic_hook::setup_panic_hook();

    let builder = tauri::Builder::default().plugin(activation_ready_plugin());

    // The plugin transport is only instance coordination, not authentication.
    // Validate its complete argv envelope before the existing lightweight,
    // deep-link, and focus-only behavior sees any local WM_COPYDATA content.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            log::info!("=== Single Instance Callback Triggered ===");
            log::debug!("Args count: {}", args.len());
            #[cfg(target_os = "windows")]
            let args = match crate::windows_runtime::normalize_single_instance_args(args) {
                Some(args) => args,
                None => {
                    log::warn!("Rejected an invalid single-instance argument envelope");
                    return;
                }
            };
            // A protocol-looking argument must pass scheme/version/action and
            // DTO validation before even lightweight/focus behavior runs. The
            // renderer receives only the parsed confirmation request, never
            // the raw local transport payload.
            let deeplink_request = match args.iter().find(|arg| arg.starts_with("fyagent://")) {
                Some(candidate) => match crate::deeplink::parse_deeplink_url(candidate) {
                    Ok(request) => Some(request),
                    Err(_) => {
                        log::warn!("Rejected invalid deep link from single_instance args");
                        submit_activation(
                            app,
                            PendingActivation::InvalidDeepLink {
                                // Preserve the historical macOS/Linux focus
                                // behavior. Windows rejects protocol-looking
                                // local input without giving it focus effects.
                                focus_main_window: !cfg!(target_os = "windows"),
                            },
                        );
                        return;
                    }
                },
                None => None,
            };

            if let Some(request) = deeplink_request {
                submit_activation(
                    app,
                    PendingActivation::DeepLink {
                        request: Box::new(request),
                        focus_main_window: true,
                    },
                );
            } else {
                log::info!("ℹ No deep link URL found in args (this is expected on macOS when launched via system)");
                submit_activation(app, PendingActivation::Focus);
            }
        }));

    let builder = builder
        .on_page_load(|webview, payload| {
            if webview.label() == "main"
                && matches!(payload.event(), tauri::webview::PageLoadEvent::Started)
            {
                mark_activation_renderer_unready();
            }
        })
        // 注册 deep-link 插件（处理 macOS AppleEvent 和其他平台的深链接）
        .plugin(tauri_plugin_deep_link::init())
        // 拦截窗口关闭：根据设置决定是否最小化到托盘
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 数据库版本过新的恢复模式下没有托盘可唤回，关闭即退出，避免应用隐身后台
                let in_db_recovery = crate::init_status::get_init_error()
                    .map(|p| p.kind.as_deref() == Some("db_version_too_new"))
                    .unwrap_or(false);
                if in_db_recovery {
                    api.prevent_close();
                    window.app_handle().exit(0);
                    return;
                }

                let settings = crate::settings::get_settings();

                if settings.minimize_to_tray_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "windows")]
                    {
                        let _ = window.set_skip_taskbar(true);
                    }
                    #[cfg(target_os = "macos")]
                    {
                        tray::apply_tray_policy(window.app_handle(), false);
                    }
                } else {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    #[cfg(not(target_os = "windows"))]
    let builder = builder
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(window_state_flags())
                .skip_initial_state("main")
                .build(),
        );

    let builder = builder
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                create_main_webview(app.handle())?;
                // Plugin setup has already admitted this process as the primary
                // instance. Cleanup owns one known legacy value only and must
                // never block startup if Alice's hive is temporarily unavailable.
                if let Err(error) = auto_launch::enforce_platform_auto_launch_policy() {
                    log::warn!("Unable to clean the legacy Shell-user auto-launch value: {error}");
                }
            }

            let _ = rustls::crypto::ring::default_provider().install_default();

            // 预先刷新 Store 覆盖配置，确保后续路径读取正确（日志/数据库等）
            app_store::refresh_app_config_dir_override(app.handle());
            panic_hook::init_app_config_dir(crate::config::get_app_config_dir());

            // 初始化日志（输出到 <app_config_dir>/logs/fyagent.log）
            {
                use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

                let log_dir = panic_hook::get_log_dir();

                // 确保日志目录存在
                if let Err(e) = std::fs::create_dir_all(&log_dir) {
                    eprintln!("创建日志目录失败: {e}");
                }

                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        // 底层保留 Trace 能力，便于加载用户配置后动态调高级别。
                        // 插件注册后会立即把全局级别收紧到 Info，避免启动阶段全量 Trace。
                        .level(log::LevelFilter::Trace)
                        // plugin-log 的前端 command 会直达 logger，绕过 log 宏的全局
                        // max_level；在分发层补一次过滤，确保动态总开关同样约束前端日志。
                        .filter(|metadata| {
                            runtime_log_level_allows(metadata.level(), log::max_level())
                        })
                        .targets([
                            Target::new(TargetKind::Stdout),
                            Target::new(TargetKind::Folder {
                                path: log_dir,
                                file_name: Some("fyagent".into()),
                            }),
                        ])
                        // KeepSome(4) 保留 4 个轮转归档，加上当前文件最多约 100 MiB。
                        // 轮转仅按大小触发；跨重启继续追加，不再丢失上一次运行的日志。
                        .rotation_strategy(RotationStrategy::KeepSome(4))
                        .max_file_size(20 * 1024 * 1024)
                        .timezone_strategy(TimezoneStrategy::UseLocal)
                        .build(),
                )?;

                // 用户配置存在数据库中，数据库尚未打开时使用保守的 Info 级别。
                log::set_max_level(log::LevelFilter::Info);
                log::info!("=== FyAgent v{} started ===", env!("CARGO_PKG_VERSION"));
            }

            // 首次读取覆盖路径时 logger 尚未可用；此处重放一次，
            // 让 Store 损坏或路径无效等启动警告能够真正落盘。
            let _ = app_store::refresh_app_config_dir_override(app.handle());

            match crate::codex_desktop::temp::JobTempDir::cleanup_stale_system_root() {
                Ok(removed) if removed > 0 => {
                    log::info!(
                        "Codex desktop installer removed {removed} stale temporary job directories"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!(
                        "Codex desktop installer stale temporary cleanup failed with {:?}",
                        error.code()
                    );
                }
            }

            #[cfg(target_os = "windows")]
            set_windows_app_user_model_id(app.handle());

            // 注入 AppHandle 给 usage_events，让无 AppHandle 持有的写日志路径
            // 也能向前端推送 `usage-log-recorded`。
            // 放在日志系统初始化之后，确保 init 的日志能正常输出。
            usage_events::init(app.handle().clone());

            // 初始化数据库
            let app_config_dir = crate::config::get_app_config_dir();
            let db_path = app_config_dir.join("fyagent.db");
            let json_path = app_config_dir.join("config.json");

            // 检查是否需要从 config.json 迁移到 SQLite
            let has_json = json_path.exists();
            let has_db = db_path.exists();

            // 如果需要迁移，先验证 config.json 是否可以加载（在创建数据库之前）
            // 这样如果加载失败用户选择退出，数据库文件还没被创建，下次可以正常重试
            let migration_config = if !has_db && has_json {
                log::info!("检测到旧版配置文件，验证配置文件...");

                // 循环：支持用户重试加载配置文件
                loop {
                    match crate::app_config::MultiAppConfig::load() {
                        Ok(config) => {
                            log::info!("✓ 配置文件加载成功");
                            break Some(config);
                        }
                        Err(e) => {
                            log::error!("加载旧配置文件失败: {e}");
                            // 弹出系统对话框让用户选择
                            if !show_migration_error_dialog(app.handle(), &e.to_string()) {
                                // 用户选择退出（此时数据库还没创建，下次启动可以重试）
                                log::info!("用户选择退出程序");
                                std::process::exit(1);
                            }
                            // 用户选择重试，继续循环
                            log::info!("用户选择重试加载配置文件");
                        }
                    }
                }
            } else {
                None
            };

            // 现在创建数据库（包含 Schema 迁移）
            //
            // 说明：从 v3.8.* 升级的用户通常会走到这里的 SQLite schema 迁移，
            // 若迁移失败（数据库损坏/权限不足/user_version 过新等），需要给用户明确提示，
            // 否则表现可能只是“应用打不开/闪退”。
            //
            // 预检：数据库版本过新时，必须先于任何 schema 写操作（create_tables 内含
            // DROP/ALTER 等 DDL）进入恢复界面，避免旧应用对读不懂的更新版 DB 落写。
            match crate::database::Database::stored_user_version_exceeds_supported(&db_path) {
                Ok(Some(version)) => {
                    log::warn!("数据库版本过新（v{version}），引导用户在应用内升级应用");
                    crate::init_status::set_init_error(crate::init_status::InitErrorPayload {
                        path: db_path.display().to_string(),
                        error: format!(
                            "数据库版本过新（{version}），当前应用仅支持 {}，请升级应用后再尝试。",
                            crate::database::SCHEMA_VERSION
                        ),
                        kind: Some("db_version_too_new".to_string()),
                        db_version: Some(version),
                        supported_version: Some(crate::database::SCHEMA_VERSION),
                    });
                    // 主窗口默认 visible:false，恢复界面必须强制显示
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    return Ok(());
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("预检数据库版本失败，继续正常初始化流程: {e}");
                }
            }

            let db = loop {
                match crate::database::Database::init() {
                    Ok(db) => break Arc::new(db),
                    Err(e) => {
                        log::error!("Failed to init database: {e}");

                        if !show_database_init_error_dialog(app.handle(), &db_path, &e.to_string())
                        {
                            log::info!("用户选择退出程序");
                            std::process::exit(1);
                        }

                        log::info!("用户选择重试初始化数据库");
                    }
                }
            };

            // 数据库可用后立即应用持久化日志级别，避免后续服务初始化
            // 继续使用启动阶段的 Info 回退。损坏配置显式 fail-closed 到 Info。
            match db.get_log_config() {
                Ok(log_config) => {
                    log::set_max_level(log_config.to_level_filter());
                    log::info!(
                        "已加载日志配置: enabled={}, level={}",
                        log_config.enabled,
                        log_config.level
                    );
                }
                Err(e) => {
                    log::set_max_level(log::LevelFilter::Info);
                    log::warn!("读取日志配置失败，已回退到 info: {e}");
                }
            }

            // 如果有预加载的配置，执行迁移
            if let Some(config) = migration_config {
                log::info!("开始执行数据迁移...");

                match db.migrate_from_json(&config) {
                    Ok(_) => {
                        log::info!("✓ 配置迁移成功");
                        // 标记迁移成功，供前端显示 Toast
                        crate::init_status::set_migration_success();
                        // 归档旧配置文件（重命名而非删除，便于用户恢复）
                        let archive_path = json_path.with_extension("json.migrated");
                        if let Err(e) = std::fs::rename(&json_path, &archive_path) {
                            log::warn!("归档旧配置文件失败: {e}");
                        } else {
                            log::info!("✓ 旧配置已归档为 config.json.migrated");
                        }
                    }
                    Err(e) => {
                        // 配置加载成功但迁移失败的情况极少（磁盘满等），仅记录日志
                        log::error!("配置迁移失败: {e}，将从现有配置导入");
                    }
                }
            }

            let app_state = AppState::new(db);

            // 设置 AppHandle 用于代理故障转移时的 UI 更新
            app_state.proxy_service.set_app_handle(app.handle().clone());

            let job_event_handle = app.handle().clone();
            app_state
                .codex_desktop_service
                .attach_job_event_sink(Arc::new(move |snapshot| {
                    if let Err(error) = job_event_handle.emit(
                        crate::services::codex_desktop::JOB_UPDATED_EVENT,
                        snapshot,
                    ) {
                        // Event delivery is observational only. The service remains the source
                        // of truth and a later query can recover the complete snapshot.
                        log::warn!(
                            "Codex desktop installer could not emit a job snapshot: {error}"
                        );
                    }
                }));

            let log_opener_handle = app.handle().clone();
            app_state
                .codex_desktop_service
                .attach_log_directory_opener(Arc::new(move |directory: &std::path::Path| {
                    tauri::async_runtime::block_on(
                        crate::platform::process_launch::open_directory_as_user(
                            log_opener_handle.clone(),
                            directory.to_path_buf(),
                        ),
                    )
                    .map_err(|_| {
                            crate::codex_desktop::error::InstallerError::new(
                                crate::codex_desktop::error::InstallerErrorCode::InternalError,
                            )
                            .with_diagnostic_message(
                                "the application log directory could not be opened",
                            )
                    })
                }));

            // ============================================================
            // 按表独立判断的导入逻辑（各类数据独立检查，互不影响）
            // ============================================================

            // 1. 初始化默认 Skills 仓库（已有内置检查：表非空则跳过）
            match app_state.db.init_default_skill_repos() {
                Ok(count) if count > 0 => {
                    log::info!("✓ Initialized {count} default skill repositories");
                }
                Ok(_) => {} // 表非空，静默跳过
                Err(e) => log::warn!("✗ Failed to initialize default skill repos: {e}"),
            }

            // 1.1. Skills 统一管理迁移：当数据库迁移到 v3 结构后，自动从各应用目录导入到 SSOT
            // 触发条件由 schema 迁移设置 settings.skills_ssot_migration_pending = true 控制。
            match app_state.db.get_setting("skills_ssot_migration_pending") {
                Ok(Some(flag)) if flag == "true" || flag == "1" => {
                    // 安全保护：如果用户已经有 v3 结构的 Skills 数据，就不要自动清空重建。
                    let has_existing = app_state
                        .db
                        .get_all_installed_skills()
                        .map(|skills| !skills.is_empty())
                        .unwrap_or(false);

                    if has_existing {
                        log::info!(
                            "Detected skills_ssot_migration_pending but skills table not empty; skipping auto import."
                        );
                        let _ = app_state
                            .db
                            .set_setting("skills_ssot_migration_pending", "false");
                    } else {
                        match crate::services::skill::migrate_skills_to_ssot(&app_state.db) {
                            Ok(count) => {
                                log::info!("✓ Auto imported {count} skill(s) into SSOT");
                                if count > 0 {
                                    crate::init_status::set_skills_migration_result(count);
                                }
                                let _ = app_state
                                    .db
                                    .set_setting("skills_ssot_migration_pending", "false");
                            }
                            Err(e) => {
                                log::warn!("✗ Failed to auto import legacy skills to SSOT: {e}");
                                crate::init_status::set_skills_migration_error(e.to_string());
                                // 保留 pending 标志，方便下次启动重试
                            }
                        }
                    }
                }
                Ok(_) => {} // 未开启迁移标志，静默跳过
                Err(e) => log::warn!("✗ Failed to read skills migration flag: {e}"),
            }

            // 1.5. 自动导入 live 配置 + seed 官方预设供应商（Claude / Codex / Gemini）
            //
            // 先 import 后 seed 是有意为之：先把用户手动配置的 settings.json / auth.json / .env
            // 落成 "default" provider 设为 current，再追加官方预设（is_current=false）。
            // 这样用户切到官方预设时，回填机制会保护原 live 配置不丢失。
            //
            // 捕获首次运行快照：所有全新装用户都会看到欢迎弹窗介绍 FyAgent 的工作方式。
            // 读失败时默认不弹，宁可漏弹也不要因为故障打扰用户。
            let first_run_already_confirmed = crate::settings::get_settings()
                .first_run_notice_confirmed
                .unwrap_or(false);
            let fresh_install_at_startup =
                app_state.db.is_providers_empty().unwrap_or(false);

            for app_type in
                crate::app_config::AppType::all().filter(|t| !t.is_additive_mode())
            {
                if !crate::services::provider::should_import_default_config_on_startup(
                    &app_state,
                    &app_type,
                )
                .unwrap_or(false)
                {
                    log::debug!(
                        "○ {} already has providers; live import skipped",
                        app_type.as_str()
                    );
                    continue;
                }

                match crate::services::provider::import_default_config(
                    &app_state,
                    app_type.clone(),
                ) {
                    Ok(true) => log::info!(
                        "✓ Imported live config for {} as default provider",
                        app_type.as_str()
                    ),
                    Ok(false) => log::debug!(
                        "○ {} already has providers; live import skipped",
                        app_type.as_str()
                    ),
                    Err(e) => log::debug!(
                        "○ No live config to import for {}: {e}",
                        app_type.as_str()
                    ),
                }
            }

            match app_state.db.init_default_official_providers() {
                Ok(count) if count > 0 => {
                    log::info!("✓ Seeded {count} official provider(s)");
                }
                Ok(_) => {}
                Err(e) => log::warn!("✗ Failed to seed official providers: {e}"),
            }

            {
                let db_for_codex_history_migration = app_state.db.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    match crate::codex_history_migration::maybe_migrate_codex_third_party_history_provider_bucket(
                        &db_for_codex_history_migration,
                    ) {
                        Ok(outcome) => {
                            if let Some(reason) = outcome.skipped_reason {
                                log::debug!("○ Codex history provider bucket migration skipped: {reason}");
                            } else {
                                log::info!(
                                    "✓ Codex history provider bucket migration completed: sources={}, jsonl_files={}, state_rows={}",
                                    outcome.source_provider_ids.len(),
                                    outcome.migrated_jsonl_files,
                                    outcome.migrated_state_rows
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("✗ Codex history provider bucket migration failed: {e}");
                        }
                    }

                    match crate::codex_history_migration::maybe_migrate_codex_provider_template_bucket(
                        &db_for_codex_history_migration,
                    ) {
                        Ok(outcome) => {
                            if let Some(reason) = outcome.skipped_reason {
                                log::debug!("○ Codex provider template bucket migration skipped: {reason}");
                            } else if !outcome.migrated_provider_ids.is_empty() {
                                log::info!(
                                    "✓ Codex provider template bucket migration completed: providers={}",
                                    outcome.migrated_provider_ids.len()
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("✗ Codex provider template bucket migration failed: {e}");
                        }
                    }

                    // 统一会话开关的官方历史迁移：开关开启但上次未完成（如文件被占用
                    // 中途失败）时在启动期重试；函数内部自门控，开关关闭时直接跳过。
                    match crate::codex_history_migration::maybe_migrate_codex_official_history_to_unified_bucket() {
                        Ok(outcome) => {
                            if let Some(reason) = outcome.skipped_reason {
                                log::debug!("○ Codex official history unify migration skipped: {reason}");
                            } else {
                                log::info!(
                                    "✓ Codex official history unify migration completed: jsonl_files={}, state_rows={}",
                                    outcome.migrated_jsonl_files,
                                    outcome.migrated_state_rows
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("✗ Codex official history unify migration failed: {e}");
                        }
                    }
                });
            }

            // 老用户 / 已确认的路径由 `fresh_install_at_startup` 自行拦截，这里不做写入。
            // 字段只由前端在用户点击"我知道了"时 save_settings 回写，语义是"用户显式确认过"。
            if !first_run_already_confirmed && fresh_install_at_startup {
                log::info!("✓ First-run welcome notice pending");
            }

            // 1.6. 自动同步 OpenCode / OpenClaw 的 live providers 到数据库
            //
            // additive 模式（OpenCode / OpenClaw）的 import 函数按 id 幂等——
            // 新 id 执行导入，已有 id 则更新 settings 和 display name，所以每次
            // 启动都跑是安全的：既保证新装用户开箱可见 live 中的供应商，也让外部
            // 修改的 live 文件能在重启后同步到数据库（与之前依赖前端"导入当前配置"
            // 按钮手动触发不同）。
            //
            // 底层 read_*_config 在文件不存在时返回默认空配置，因此新装且无
            // live 文件的用户走 Ok(0) 路径，不会产生错误日志噪音。
            match crate::services::provider::import_opencode_providers_from_live(&app_state) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Synced {count} OpenCode provider(s) from live config");
                }
                Ok(_) => log::debug!("○ No OpenCode provider changes from live config"),
                Err(e) => log::warn!("✗ Failed to import OpenCode providers: {e}"),
            }
            match crate::services::provider::import_openclaw_providers_from_live(&app_state) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Synced {count} OpenClaw provider(s) from live config");
                }
                Ok(_) => log::debug!("○ No OpenClaw provider changes from live config"),
                Err(e) => log::warn!("✗ Failed to import OpenClaw providers: {e}"),
            }
            match crate::services::provider::import_hermes_providers_from_live(&app_state) {
                Ok(count) if count > 0 => {
                    log::info!("✓ Synced {count} Hermes provider(s) from live config");
                }
                Ok(_) => log::debug!("○ No Hermes provider changes from live config"),
                Err(e) => log::warn!("✗ Failed to import Hermes providers: {e}"),
            }

            // 2. OMO 配置导入（当数据库中无 OMO provider 时，从本地文件导入）
            {
                let has_omo = app_state
                    .db
                    .get_all_providers("opencode")
                    .map(|providers| providers.values().any(|p| p.category.as_deref() == Some("omo")))
                    .unwrap_or(false);
                if !has_omo {
                    match crate::services::OmoService::import_from_local(&app_state, &crate::services::omo::STANDARD) {
                        Ok(provider) => {
                            log::info!("✓ Imported OMO config from local as provider '{}'", provider.name);
                        }
                        Err(AppError::OmoConfigNotFound) => {
                            log::debug!("○ No OMO config to import");
                        }
                        Err(e) => {
                            log::warn!("✗ Failed to import OMO config from local: {e}");
                        }
                    }
                }
            }

            // 2.3 OMO Slim config import (when no omo-slim provider in DB, import from local)
            {
                let has_omo_slim = app_state
                    .db
                    .get_all_providers("opencode")
                    .map(|providers| {
                        providers
                            .values()
                            .any(|p| p.category.as_deref() == Some("omo-slim"))
                    })
                    .unwrap_or(false);
                if !has_omo_slim {
                    match crate::services::OmoService::import_from_local(&app_state, &crate::services::omo::SLIM) {
                        Ok(provider) => {
                            log::info!(
                                "✓ Imported OMO Slim config from local as provider '{}'",
                                provider.name
                            );
                        }
                        Err(AppError::OmoConfigNotFound) => {
                            log::debug!("○ No OMO Slim config to import");
                        }
                        Err(e) => {
                            log::warn!("✗ Failed to import OMO Slim config from local: {e}");
                        }
                    }
                }
            }

            // 3. 导入 MCP 服务器配置（表空时触发）
            if app_state.db.is_mcp_table_empty().unwrap_or(false) {
                log::info!("MCP table empty, importing from live configurations...");

                match crate::services::mcp::McpService::import_from_claude(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from Claude");
                    }
                    Ok(_) => log::debug!("○ No Claude MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import Claude MCP: {e}"),
                }

                match crate::services::mcp::McpService::import_from_codex(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from Codex");
                    }
                    Ok(_) => log::debug!("○ No Codex MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import Codex MCP: {e}"),
                }

                match crate::services::mcp::McpService::import_from_gemini(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from Gemini");
                    }
                    Ok(_) => log::debug!("○ No Gemini MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import Gemini MCP: {e}"),
                }

                match crate::services::mcp::McpService::import_from_grokbuild(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from Grok Build");
                    }
                    Ok(_) => log::debug!("○ No Grok Build MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import Grok Build MCP: {e}"),
                }

                match crate::services::mcp::McpService::import_from_opencode(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from OpenCode");
                    }
                    Ok(_) => log::debug!("○ No OpenCode MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import OpenCode MCP: {e}"),
                }

                match crate::services::mcp::McpService::import_from_hermes(&app_state) {
                    Ok(count) if count > 0 => {
                        log::info!("✓ Imported {count} MCP server(s) from Hermes");
                    }
                    Ok(_) => log::debug!("○ No Hermes MCP servers found to import"),
                    Err(e) => log::warn!("✗ Failed to import Hermes MCP: {e}"),
                }
            }

            // 4. 导入提示词文件（表空时触发）
            if app_state.db.is_prompts_table_empty().unwrap_or(false) {
                log::info!("Prompts table empty, importing from live configurations...");

                for app in [
                    crate::app_config::AppType::Claude,
                    crate::app_config::AppType::Codex,
                    crate::app_config::AppType::Gemini,
                    crate::app_config::AppType::GrokBuild,
                    crate::app_config::AppType::OpenCode,
                    crate::app_config::AppType::OpenClaw,
                    crate::app_config::AppType::Hermes,
                ] {
                    match crate::services::prompt::PromptService::import_from_file_on_first_launch(
                        &app_state,
                        app.clone(),
                    ) {
                        Ok(count) if count > 0 => {
                            log::info!("✓ Imported {count} prompt(s) for {}", app.as_str());
                        }
                        Ok(_) => log::debug!("○ No prompt file found for {}", app.as_str()),
                        Err(e) => log::warn!("✗ Failed to import prompt for {}: {e}", app.as_str()),
                    }
                }
            }

            // 迁移旧的 app_config_dir 配置到 Store
            if let Err(e) = app_store::migrate_app_config_dir_from_settings(app.handle()) {
                log::warn!("迁移 app_config_dir 失败: {e}");
            }

            // 启动阶段不再无条件保存,避免意外覆盖用户配置。

            // 注册 deep-link URL 处理器（使用正确的 DeepLinkExt API）
            log::info!("=== Registering deep-link URL handler ===");

            // Linux 和 Windows 调试模式需要显式注册
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                #[cfg(target_os = "linux")]
                {
                    // Use Tauri's path API to get correct path (includes app identifier)
                    // tauri-plugin-deep-link writes to: ~/.local/share/com.fyagent.desktop/applications/fyagent-handler.desktop
                    // Only register if .desktop file doesn't exist to avoid overwriting user customizations
                    let should_register = app
                        .path()
                        .data_dir()
                        .map(|d| !d.join("applications/fyagent-handler.desktop").exists())
                        .unwrap_or(true);

                    if should_register {
                        if let Err(e) = app.deep_link().register_all() {
                            log::error!("✗ Failed to register deep link schemes: {}", e);
                        } else {
                            log::info!("✓ Deep link schemes registered (Linux)");
                        }
                    } else {
                        log::info!("⊘ Deep link handler already exists, skipping registration");
                    }
                }

                #[cfg(all(debug_assertions, windows))]
                {
                    if let Err(e) = app.deep_link().register_all() {
                        log::error!("✗ Failed to register deep link schemes: {}", e);
                    } else {
                        log::info!("✓ Deep link schemes registered (Windows debug)");
                    }
                }
            }

            // 注册 URL 处理回调（所有平台通用）
            app.deep_link().on_open_url({
                let app_handle = app.handle().clone();
                move |event| {
                    log::info!("=== Deep Link Event Received (on_open_url) ===");
                    let urls = event.urls();
                    log::info!("Received {} URL(s)", urls.len());

                    if crate::lightweight::is_lightweight_mode() {
                        if let Err(e) = crate::lightweight::exit_lightweight_mode(&app_handle) {
                            log::error!("退出轻量模式重建窗口失败: {e}");
                        }
                    }

                    for url in urls {
                        let url_str = url.as_str();

                        if handle_deeplink_url(&app_handle, url_str, true, "on_open_url") {
                            break; // Process only first fyagent:// URL
                        }
                    }
                }
            });
            log::info!("✓ Deep-link URL handler registered");

            // 创建动态托盘菜单
            let menu = tray::create_tray_menu(app.handle(), &app_state)?;

            // 构建托盘
            let mut tray_builder = TrayIconBuilder::with_id(tray::TRAY_ID)
                .tooltip("FyAgent") // 鼠标悬停提示
                .on_tray_icon_event(|tray, event| match event {
                    // 鼠标悬停/点击到托盘图标时，后台异步刷新用量缓存，
                    // 让用户下一次（或快速打开菜单的那一刻）看到较新的数字。
                    // refresh_all_usage_in_tray 内部有 10 秒防抖。
                    TrayIconEvent::Enter { .. } | TrayIconEvent::Click { .. } => {
                        let app = tray.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            crate::tray::refresh_all_usage_in_tray(&app).await;
                        });
                    }
                    _ => log::debug!("unhandled event {event:?}"),
                })
                .menu(&menu)
                .on_menu_event(|app, event| {
                    tray::handle_tray_menu_event(app, &event.id.0);
                })
                .show_menu_on_left_click(true);

            // 使用平台对应的托盘图标（macOS 使用模板图标适配深浅色）
            #[cfg(target_os = "macos")]
            {
                if let Some(icon) = macos_tray_icon() {
                    tray_builder = tray_builder.icon(icon).icon_as_template(true);
                } else if let Some(icon) = app.default_window_icon() {
                    log::warn!("Falling back to default window icon for tray");
                    tray_builder = tray_builder.icon(icon.clone());
                } else {
                    log::warn!("Failed to load macOS tray icon for tray");
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                if let Some(icon) = app.default_window_icon() {
                    tray_builder = tray_builder.icon(icon.clone());
                } else {
                    log::warn!("Failed to get default window icon for tray");
                }
            }

            let _tray = tray_builder.build(app)?;
            crate::services::webdav_auto_sync::start_worker(
                app_state.db.clone(),
                app.handle().clone(),
            );
            crate::services::s3_auto_sync::start_worker(
                app_state.db.clone(),
                app.handle().clone(),
            );
            // 将同一个实例注入到全局状态，避免重复创建导致的不一致
            app.manage(app_state);

            // 初始化 SkillService
            let skill_service = SkillService::new();
            app.manage(commands::skill::SkillServiceState(Arc::new(skill_service)));

            // 初始化 CopilotAuthManager
            {
                use crate::proxy::providers::copilot_auth::CopilotAuthManager;
                use commands::CopilotAuthState;
                use tokio::sync::RwLock;

                let app_config_dir = crate::config::get_app_config_dir();
                let copilot_auth_manager = CopilotAuthManager::new(app_config_dir);
                app.manage(CopilotAuthState(Arc::new(RwLock::new(copilot_auth_manager))));
                log::info!("✓ CopilotAuthManager initialized");
            }

            // 初始化 CodexOAuthManager (ChatGPT Plus/Pro 反代)
            {
                use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
                use commands::CodexOAuthState;
                use tokio::sync::RwLock;

                let app_config_dir = crate::config::get_app_config_dir();
                let codex_oauth_manager = CodexOAuthManager::new(app_config_dir);
                app.manage(CodexOAuthState(Arc::new(RwLock::new(codex_oauth_manager))));
                log::info!("✓ CodexOAuthManager initialized");
            }

            // 初始化 xAI OAuthManager (Grok API 反代)
            {
                use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
                use commands::XaiOAuthState;
                use tokio::sync::RwLock;

                let app_config_dir = crate::config::get_app_config_dir();
                let xai_oauth_manager = XaiOAuthManager::new(app_config_dir);
                app.manage(XaiOAuthState(Arc::new(RwLock::new(xai_oauth_manager))));
                log::info!("✓ XaiOAuthManager initialized");
            }

            // 初始化全局出站代理 HTTP 客户端
            {
                let db = &app.state::<AppState>().db;
                let proxy_url = db.get_global_proxy_url().ok().flatten();

                if let Err(e) = crate::proxy::http_client::init(proxy_url.as_deref()) {
                    log::error!(
                        "[GlobalProxy] [GP-005] Failed to initialize with saved config: {e}"
                    );

                    // 清除无效的代理配置
                    if proxy_url.is_some() {
                        log::warn!(
                            "[GlobalProxy] [GP-006] Clearing invalid proxy config from database"
                        );
                        if let Err(clear_err) = db.set_global_proxy_url(None) {
                            log::error!(
                                "[GlobalProxy] [GP-007] Failed to clear invalid config: {clear_err}"
                            );
                        }
                    }

                    // 使用直连模式重新初始化
                    if let Err(fallback_err) = crate::proxy::http_client::init(None) {
                        log::error!(
                            "[GlobalProxy] [GP-008] Failed to initialize direct connection: {fallback_err}"
                        );
                    }
                }
            }

            // 异常退出恢复 + 代理状态自动恢复
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle.state::<AppState>();

                // 检查是否有 Live 备份（表示上次异常退出时可能处于接管状态）
                let has_backups = match state.db.has_any_live_backup().await {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("检查 Live 备份失败: {e}");
                        false
                    }
                };
                // 检查 Live 配置是否仍处于被接管状态（包含占位符）
                let live_taken_over = state.proxy_service.detect_takeover_in_live_configs();

                if has_backups || live_taken_over {
                    log::warn!("检测到上次异常退出（存在接管残留），正在恢复 Live 配置...");
                    if let Err(e) = state.proxy_service.recover_from_crash().await {
                        log::error!("恢复 Live 配置失败: {e}");
                    } else {
                        log::info!("Live 配置已恢复");
                    }
                }

                // 必须排在 auto-extract 之前：先把历史泄漏进 Gemini 共享片段的凭据
                // 清干净，否则紧接着的提取会基于被污染的 live 再写一遍。
                if let Err(e) =
                    crate::services::provider::ProviderService::scrub_leaked_gemini_common_config(
                        &state,
                    )
                    .await
                {
                    log::warn!("清理 Gemini 通用配置泄漏凭据失败: {e}");
                }

                initialize_common_config_snippets(&state);

                // 检查 settings 表中的代理状态，自动恢复代理服务
                restore_proxy_state_on_startup(&state).await;

                // Periodic backup check (on startup)
                if let Err(e) = state.db.periodic_backup_if_needed() {
                    log::warn!("Periodic backup failed on startup: {e}");
                }

                // Periodic maintenance timer: run once per day while the app is running
                let db_for_timer = state.db.clone();
                tauri::async_runtime::spawn(async move {
                    const PERIODIC_MAINTENANCE_INTERVAL_SECS: u64 = 24 * 60 * 60;
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                        PERIODIC_MAINTENANCE_INTERVAL_SECS,
                    ));
                    interval.tick().await; // skip immediate first tick (already checked above)
                    loop {
                        interval.tick().await;
                        if let Err(e) = db_for_timer.periodic_backup_if_needed() {
                            log::warn!("Periodic maintenance timer failed: {e}");
                        }
                    }
                });

                // Session log usage sync: 启动时同步一次，之后每 60 秒检查
                let db_for_session_sync = state.db.clone();
                tauri::async_runtime::spawn(async move {
                    const SESSION_SYNC_INTERVAL_SECS: u64 = 60;

                    async fn run_session_sync(db: std::sync::Arc<crate::database::Database>, backfill: bool) {
                        let _guard = crate::services::session_usage::session_sync_mutex()
                            .lock()
                            .await;
                        let task = tauri::async_runtime::spawn_blocking(move || {
                            if backfill {
                                if let Err(error) = db.backfill_missing_usage_costs() {
                                    log::warn!("Usage cost startup backfill failed: {error}");
                                }
                            }
                            crate::services::session_usage::sync_all_unlocked(&db)
                        });
                        match task.await {
                            Ok(result) if !result.errors.is_empty() => {
                                log::warn!(
                                    "Session usage sync completed with {} error(s)",
                                    result.errors.len()
                                );
                            }
                            Ok(_) => {}
                            Err(error) => log::warn!("Session usage blocking task failed: {error}"),
                        }
                    }

                    // 首次同步（含费用回填）
                    run_session_sync(db_for_session_sync.clone(), true).await;

                    // 定期同步
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                        SESSION_SYNC_INTERVAL_SECS,
                    ));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    interval.tick().await; // skip immediate first tick
                    loop {
                        interval.tick().await;
                        run_session_sync(db_for_session_sync.clone(), false).await;
                    }
                });
            });

            // Linux: 禁用 WebKitGTK 硬件加速，防止 EGL 初始化失败导致白屏
            #[cfg(target_os = "linux")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|webview| {
                        use webkit2gtk::{WebViewExt, SettingsExt, HardwareAccelerationPolicy};
                        let wk_webview = webview.inner();
                        if let Some(settings) = WebViewExt::settings(&wk_webview) {
                            SettingsExt::set_hardware_acceleration_policy(&settings, HardwareAccelerationPolicy::Never);
                            log::info!("已禁用 WebKitGTK 硬件加速");
                        }
                    });
                }
            }

            // 静默启动：根据设置决定是否显示主窗口
            let settings = crate::settings::get_settings();
            if let Some(window) = app.get_webview_window("main") {
                // The configured window begins hidden. Restore, clamp and
                // reapply maximization before either normal or silent startup
                // decides its visibility, avoiding off-screen/legacy flashes.
                prepare_main_webview(&window);

                // 在窗口首次显示前同步装饰状态，避免前端加载后再切换导致标题栏闪烁
                // 仅 Linux 生效：解决 Wayland 下系统窗口按钮不可用的问题
                #[cfg(target_os = "linux")]
                let _ = window.set_decorations(!settings.use_app_window_controls);
                if settings.silent_startup {
                    // 静默启动模式：保持窗口隐藏
                    let _ = window.hide();
                    #[cfg(target_os = "windows")]
                    let _ = window.set_skip_taskbar(true);
                    #[cfg(target_os = "macos")]
                    tray::apply_tray_policy(app.handle(), false);
                    log::info!("静默启动模式：主窗口已隐藏");
                } else {
                    // 正常启动模式：显示窗口
                    let _ = window.show();
                    log::info!("正常启动模式：主窗口已显示");

                    // Linux: 解决首次启动 UI 无响应问题（Tauri #10746 + wry #637）。
                    // 启动时 webview 未获取焦点 + surface 尺寸协商失败，导致点击无效。
                    // 这里做 set_focus + 伪 resize，等价于无视觉版本的"最大化-还原"。
                    #[cfg(target_os = "linux")]
                    {
                        linux_fix::nudge_main_window(window.clone());
                    }
                }
            }


            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_providers,
            commands::get_current_provider,
            commands::add_provider,
            commands::add_provider_with_result,
            commands::update_provider,
            commands::update_provider_with_result,
            commands::delete_provider,
            commands::delete_provider_with_result,
            commands::remove_provider_from_live_config,
            commands::switch_provider,
            commands::switch_provider_with_result,
            commands::import_default_config,
            commands::import_default_config_with_result,
            commands::analyze_codex_provider_features,
            commands::patch_codex_provider_features,
            commands::get_claude_desktop_status,
            commands::get_claude_desktop_default_routes,
            commands::import_claude_desktop_providers_from_claude,
            commands::ensure_claude_desktop_official_provider,
            commands::ensure_codex_official_provider,
            commands::ensure_grokbuild_official_provider,
            commands::get_claude_config_status,
            commands::get_config_status,
            commands::get_claude_code_config_path,
            commands::get_config_dir,
            commands::open_config_folder,
            commands::pick_directory,
            commands::open_external,
            commands::get_init_error,
            commands::get_runtime_privilege_status,
            commands::get_migration_result,
            commands::get_skills_migration_result,
            commands::get_app_config_path,
            commands::open_app_config_folder,
            commands::get_claude_common_config_snippet,
            commands::set_claude_common_config_snippet,
            commands::get_common_config_snippet,
            commands::set_common_config_snippet,
            commands::update_toml_common_config_snippet,
            commands::extract_common_config_snippet,
            commands::read_live_provider_settings,
            commands::get_settings,
            commands::get_user_home_dir,
            commands::save_settings,
            commands::has_codex_unify_history_backup,
            commands::restore_codex_unified_history,
            commands::get_rectifier_config,
            commands::set_rectifier_config,
            commands::get_optimizer_config,
            commands::set_optimizer_config,
            commands::get_copilot_optimizer_config,
            commands::set_copilot_optimizer_config,
            commands::get_log_config,
            commands::set_log_config,
            commands::restart_app,
            commands::exit_app,
            commands::is_portable_mode,
            commands::copy_text_to_clipboard,
            commands::get_claude_plugin_status,
            commands::read_claude_plugin_config,
            commands::apply_claude_plugin_config,
            commands::is_claude_plugin_applied,
            commands::apply_claude_onboarding_skip,
            commands::clear_claude_onboarding_skip,
            // Claude MCP management
            commands::get_claude_mcp_status,
            commands::read_claude_mcp_config,
            commands::upsert_claude_mcp_server,
            commands::delete_claude_mcp_server,
            commands::validate_mcp_command,
            // usage query
            commands::queryProviderUsage,
            commands::testUsageScript,
            // subscription quota
            commands::get_subscription_quota,
            commands::get_codex_oauth_quota,
            commands::get_codex_oauth_models,
            commands::get_xai_oauth_models,
            commands::get_xai_oauth_quota,
            commands::get_coding_plan_quota,
            commands::get_balance,
            // New MCP via config.json (SSOT)
            commands::get_mcp_config,
            commands::upsert_mcp_server_in_config,
            commands::delete_mcp_server_in_config,
            commands::set_mcp_enabled,
            // Unified MCP management
            commands::get_mcp_servers,
            commands::upsert_mcp_server,
            commands::delete_mcp_server,
            commands::toggle_mcp_app,
            commands::import_mcp_from_apps,
            // Prompt management
            commands::get_prompts,
            commands::upsert_prompt,
            commands::delete_prompt,
            commands::enable_prompt,
            commands::import_prompt_from_file,
            commands::get_current_prompt_file_content,
            // Profile management (项目配置方案)
            commands::list_profiles,
            commands::create_profile,
            commands::update_profile,
            commands::delete_profile,
            commands::clear_current_profile,
            commands::apply_profile,
            // model list fetch (OpenAI-compatible /v1/models)
            commands::fetch_models_for_config,
            commands::get_opencode_models,
            // ours: endpoint speed test + custom endpoint management
            commands::test_api_endpoints,
            commands::get_custom_endpoints,
            commands::add_custom_endpoint,
            commands::remove_custom_endpoint,
            commands::update_endpoint_last_used,
            // app_config_dir override via Store
            commands::get_app_config_dir_override,
            commands::set_app_config_dir_override,
            // provider sort order management
            commands::update_providers_sort_order,
            // theirs: config import/export and dialogs
            commands::export_config_to_file,
            commands::import_config_from_file,
            commands::webdav_test_connection,
            commands::webdav_sync_upload,
            commands::webdav_sync_download,
            commands::webdav_sync_save_settings,
            commands::webdav_sync_fetch_remote_info,
            commands::s3_test_connection,
            commands::s3_sync_upload,
            commands::s3_sync_download,
            commands::s3_sync_save_settings,
            commands::s3_sync_fetch_remote_info,
            commands::save_file_dialog,
            commands::open_file_dialog,
            commands::open_zip_file_dialog,
            commands::create_db_backup,
            commands::list_db_backups,
            commands::restore_db_backup,
            commands::rename_db_backup,
            commands::delete_db_backup,
            commands::sync_current_providers_live,
            // Deep link import
            commands::parse_deeplink,
            commands::merge_deeplink_config,
            commands::import_from_deeplink,
            commands::import_from_deeplink_unified,
            update_tray_menu,
            // Environment variable management
            commands::check_env_conflicts,
            commands::delete_env_vars,
            commands::restore_env_backup,
            // Skill management (v3.10.0+ unified)
            commands::get_installed_skills,
            commands::get_skill_backups,
            commands::delete_skill_backup,
            commands::install_skill_unified,
            commands::uninstall_skill_unified,
            commands::restore_skill_backup,
            commands::toggle_skill_app,
            commands::scan_unmanaged_skills,
            commands::import_skills_from_apps,
            commands::discover_available_skills,
            commands::check_skill_updates,
            commands::update_skill,
            commands::migrate_skill_storage,
            commands::search_skills_sh,
            // Skill management (legacy API compatibility)
            commands::get_skills,
            commands::get_skills_for_app,
            commands::install_skill,
            commands::install_skill_for_app,
            commands::uninstall_skill,
            commands::uninstall_skill_for_app,
            commands::get_skill_repos,
            commands::add_skill_repo,
            commands::remove_skill_repo,
            commands::install_skills_from_zip,
            // Auto launch
            commands::set_auto_launch,
            commands::get_auto_launch_status,
            // Proxy server management
            commands::start_proxy_server,
            commands::stop_proxy_server,
            commands::stop_proxy_with_restore,
            commands::get_proxy_takeover_status,
            commands::set_proxy_takeover_for_app,
            commands::get_proxy_status,
            commands::get_proxy_config,
            commands::update_proxy_config,
            // Global & Per-App Config
            commands::get_global_proxy_config,
            commands::update_global_proxy_config,
            commands::get_proxy_config_for_app,
            commands::update_proxy_config_for_app,
            commands::get_default_cost_multiplier,
            commands::set_default_cost_multiplier,
            commands::get_pricing_model_source,
            commands::set_pricing_model_source,
            commands::is_proxy_running,
            commands::is_live_takeover_active,
            commands::switch_proxy_provider,
            // Proxy failover commands
            commands::get_provider_health,
            commands::reset_circuit_breaker,
            commands::get_circuit_breaker_config,
            commands::update_circuit_breaker_config,
            commands::get_circuit_breaker_stats,
            // Failover queue management
            commands::get_failover_queue,
            commands::get_available_providers_for_failover,
            commands::add_to_failover_queue,
            commands::remove_from_failover_queue,
            commands::get_auto_failover_enabled,
            commands::set_auto_failover_enabled,
            // Usage statistics
            commands::get_usage_summary,
            commands::get_usage_summary_by_app,
            commands::get_usage_trends,
            commands::get_provider_stats,
            commands::get_model_stats,
            commands::get_request_logs,
            commands::get_request_detail,
            commands::get_model_pricing,
            commands::update_model_pricing,
            commands::update_model_pricing_batch,
            commands::delete_model_pricing,
            commands::get_models_dev_sync_config,
            commands::save_models_dev_sync_config,
            commands::record_models_dev_sync_result,
            commands::check_provider_limits,
            // Session usage sync
            commands::sync_session_usage,
            commands::rebuild_codex_usage,
            commands::get_usage_data_sources,
            // Stream health check
            commands::stream_check_provider,
            commands::stream_check_all_providers,
            commands::get_stream_check_config,
            commands::save_stream_check_config,
            // Session manager
            commands::list_sessions,
            commands::get_session_messages,
            commands::delete_session,
            commands::delete_sessions,
            commands::launch_session_terminal,
            commands::get_tool_versions,
            commands::run_tool_lifecycle_action,
            commands::probe_tool_installations,
            // Provider terminal
            commands::open_provider_terminal,
            // Universal Provider management
            commands::get_universal_providers,
            commands::get_universal_provider,
            commands::upsert_universal_provider,
            commands::delete_universal_provider,
            commands::sync_universal_provider,
            // OpenCode specific
            commands::import_opencode_providers_from_live,
            commands::get_opencode_live_provider_ids,
            // OpenClaw specific
            commands::import_openclaw_providers_from_live,
            commands::get_openclaw_live_provider_ids,
            commands::get_openclaw_live_provider,
            commands::scan_openclaw_config_health,
            commands::get_openclaw_default_model,
            commands::set_openclaw_default_model,
            commands::get_openclaw_model_catalog,
            commands::set_openclaw_model_catalog,
            commands::get_openclaw_agents_defaults,
            commands::set_openclaw_agents_defaults,
            commands::get_openclaw_env,
            commands::set_openclaw_env,
            commands::get_openclaw_tools,
            commands::set_openclaw_tools,
            // Hermes specific
            commands::import_hermes_providers_from_live,
            commands::get_hermes_live_provider_ids,
            commands::get_hermes_live_provider,
            commands::get_hermes_model_config,
            commands::open_hermes_web_ui,
            commands::launch_hermes_dashboard,
            commands::get_hermes_memory,
            commands::set_hermes_memory,
            commands::get_hermes_memory_limits,
            commands::set_hermes_memory_enabled,
            // Global upstream proxy
            commands::get_global_proxy_url,
            commands::set_global_proxy_url,
            commands::test_proxy_url,
            commands::get_upstream_proxy_status,
            commands::scan_local_proxies,
            // Window theme control
            commands::set_window_theme,
            // Generic managed auth commands
            commands::auth_start_login,
            commands::auth_poll_for_account,
            commands::auth_list_accounts,
            commands::auth_get_status,
            commands::auth_remove_account,
            commands::auth_set_default_account,
            commands::auth_logout,
            // Copilot OAuth commands (multi-account support)
            commands::copilot_start_device_flow,
            commands::copilot_poll_for_auth,
            commands::copilot_poll_for_account,
            commands::copilot_list_accounts,
            commands::copilot_remove_account,
            commands::copilot_set_default_account,
            commands::copilot_get_auth_status,
            commands::copilot_logout,
            commands::copilot_is_authenticated,
            commands::copilot_get_token,
            commands::copilot_get_token_for_account,
            commands::copilot_get_models,
            commands::copilot_get_models_for_account,
            commands::copilot_get_usage,
            commands::copilot_get_usage_for_account,
            // OMO commands
            commands::read_omo_local_file,
            commands::get_current_omo_provider_id,
            commands::disable_current_omo,
            commands::read_omo_slim_local_file,
            commands::get_current_omo_slim_provider_id,
            commands::disable_current_omo_slim,
            // Workspace files (OpenClaw)
            commands::read_workspace_file,
            commands::write_workspace_file,
            // Daily memory files (OpenClaw workspace)
            commands::list_daily_memory_files,
            commands::read_daily_memory_file,
            commands::write_daily_memory_file,
            commands::delete_daily_memory_file,
            commands::search_daily_memory_files,
            commands::open_workspace_directory,
            // lightweight mode (for testing or low-resource environments)
            commands::enter_lightweight_mode,
            commands::exit_lightweight_mode,
            commands::is_lightweight_mode,
            // WorkBuddy is an isolated top-level configuration domain.
            commands::get_workbuddy_status,
            commands::get_workbuddy_model_ids,
            commands::fetch_workbuddy_models,
            commands::save_workbuddy_models,
            commands::codex_desktop_get_local_status,
            commands::get_codex_desktop_runtime_status,
            commands::request_codex_desktop_restart,
            commands::continue_codex_desktop_restart_with_force,
            commands::cancel_codex_desktop_restart_with_force,
            commands::codex_desktop_check_latest,
            commands::codex_desktop_get_job,
            commands::codex_desktop_start_install,
            commands::codex_desktop_cancel_install,
            commands::codex_desktop_launch,
            commands::codex_desktop_open_log_directory,
            commands::agent_install_list_catalog,
            commands::agent_install_get_contract,
            commands::agent_install_refresh_preflight,
            commands::agent_install_create_plan,
            commands::agent_install_reconfirm_plan,
            commands::agent_install_start_install,
            commands::agent_install_get_job,
            commands::agent_install_cancel_install,
            commands::agent_install_probe_health,
            commands::agent_install_open_official_guide,
        ]);

    let context = tauri::generate_context!();
    #[cfg(target_os = "windows")]
    let context = {
        let mut context = context;
        if let Some(window) = context
            .config_mut()
            .app
            .windows
            .iter_mut()
            .find(|window| window.label == "main")
        {
            window.create = false;
            window.data_directory = None;
        }
        context
    };

    let app = builder
        .build(context)
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        // 处理退出请求（所有平台）
        if let RunEvent::ExitRequested { api, code, .. } = &event {
            match classify_exit_request(*code) {
                // code 为 None 表示运行时自动触发（如隐藏窗口的 WebView 被回收导致无存活窗口），
                // 此时应仅阻止退出、保持托盘后台运行。
                ExitRequestAction::StayInTray => {
                    log::info!("运行时触发退出请求（无存活窗口），阻止退出以保持托盘后台运行");
                    api.prevent_exit();
                    return;
                }
                // 重启不拦截：Tauri 的默认 re-exec 路径会在主线程保存窗口状态，避免
                // 自定义异步清理和 window-state 插件的退出钩子争用同一把锁。
                ExitRequestAction::DeferToTauriRestart => {
                    log::info!("收到重启请求 (code={code:?})，交由 Tauri 默认重启流程处理");
                    return;
                }
                // 其它 Some(_)：用户主动调用 app.exit() 退出（如托盘菜单"退出"）。
                ExitRequestAction::CleanupAndExit => {}
            }

            let installer_job = match app_handle.try_state::<store::AppState>() {
                Some(state) => match state.codex_desktop_service.get_job() {
                    Ok(job) => job,
                    Err(error) => {
                        log::warn!(
                            "Codex desktop installer state could not be inspected during exit: {:?}",
                            error.code()
                        );
                        api.prevent_exit();
                        show_codex_desktop_exit_status_unavailable_dialog(app_handle);
                        return;
                    }
                },
                None => None,
            };

            match classify_codex_desktop_exit_protection(
                installer_job
                    .as_ref()
                    .map(|job| (job.stage, job.cancellable)),
            ) {
                CodexDesktopExitProtection::AllowExit => {}
                CodexDesktopExitProtection::ConfirmCancellation => {
                    api.prevent_exit();
                    let Some(job) = installer_job else {
                        return;
                    };
                    if !confirm_codex_desktop_installation_cancellation(app_handle) {
                        return;
                    }

                    let Some(state) = app_handle.try_state::<store::AppState>() else {
                        show_codex_desktop_exit_status_unavailable_dialog(app_handle);
                        return;
                    };
                    match state.codex_desktop_service.cancel_install(&job.job_id) {
                        Ok(snapshot) if snapshot.stage == JobStage::Cancelled => {
                            match claim_process_lifecycle_transition_for_exit(app_handle) {
                                Ok(claim) => {
                                    start_claimed_exit_cleanup(app_handle.clone(), claim);
                                }
                                Err(_) => {
                                    show_codex_desktop_installation_wait_dialog(app_handle);
                                }
                            }
                        }
                        Ok(snapshot)
                            if snapshot.stage.is_cancellable() && !snapshot.cancellable =>
                        {
                            exit_after_installer_cancellation(app_handle.clone(), snapshot.job_id);
                        }
                        Ok(snapshot)
                            if matches!(
                                snapshot.stage,
                                JobStage::Installing | JobStage::VerifyingInstallation
                            ) =>
                        {
                            show_codex_desktop_installation_wait_dialog(app_handle);
                        }
                        Ok(_) | Err(_) => {
                            show_codex_desktop_exit_status_unavailable_dialog(app_handle);
                        }
                    }
                    return;
                }
                CodexDesktopExitProtection::WaitForCancellation => {
                    api.prevent_exit();
                    let Some(job) = installer_job else {
                        return;
                    };
                    show_codex_desktop_cancellation_wait_dialog(app_handle);
                    exit_after_installer_cancellation(app_handle.clone(), job.job_id);
                    return;
                }
                CodexDesktopExitProtection::WaitForInstallation => {
                    api.prevent_exit();
                    show_codex_desktop_installation_wait_dialog(app_handle);
                    return;
                }
            }

            let claim = match claim_process_lifecycle_transition_for_exit(app_handle) {
                Ok(claim) => claim,
                Err(error) => {
                    log::warn!(
                        "Process exit could not claim the installer lifecycle slot: {:?}",
                        error.code()
                    );
                    api.prevent_exit();
                    show_codex_desktop_installation_wait_dialog(app_handle);
                    return;
                }
            };
            api.prevent_exit();
            start_claimed_exit_cleanup(app_handle.clone(), claim);
            return;
        }

        #[cfg(target_os = "macos")]
        {
            match event {
                // macOS 在 Dock 图标被点击并重新激活应用时会触发 Reopen 事件，这里手动恢复主窗口
                RunEvent::Reopen { .. } => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = window.set_skip_taskbar(false);
                        }
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                        tray::apply_tray_policy(app_handle, true);
                    } else if crate::lightweight::is_lightweight_mode() {
                        if let Err(e) = crate::lightweight::exit_lightweight_mode(app_handle) {
                            log::error!("退出轻量模式重建窗口失败: {e}");
                        }
                    }
                }
                // 处理通过自定义 URL 协议触发的打开事件（例如 fyagent://...）
                RunEvent::Opened { urls } => {
                    if let Some(url) = urls.first() {
                        let url_str = url.to_string();

                        if url_str.starts_with("fyagent://") {
                            if crate::lightweight::is_lightweight_mode() {
                                if let Err(e) = crate::lightweight::exit_lightweight_mode(app_handle)
                                {
                                    log::error!("退出轻量模式重建窗口失败: {e}");
                                }
                            }

                            let _ = handle_deeplink_url(
                                app_handle,
                                &url_str,
                                true,
                                "run_event_opened",
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app_handle, event);
        }
    });
}

static PRE_APP_PROCESS_LIFECYCLE: Mutex<ProcessLifecycleCoordinator> =
    Mutex::new(ProcessLifecycleCoordinator::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessLifecycleCoordinatorOrigin {
    Service,
    PreApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessLifecycleClaimReceipt {
    pub(crate) claim: ProcessLifecycleClaim,
    origin: ProcessLifecycleCoordinatorOrigin,
}

pub(crate) fn claim_process_lifecycle_transition(
    app_handle: &tauri::AppHandle,
    requested: ProcessLifecycleTransition,
) -> Result<ProcessLifecycleClaimReceipt, crate::codex_desktop::error::InstallerError> {
    let Some(state) = app_handle.try_state::<store::AppState>() else {
        // Recovery UI can be shown before AppState is managed. Without that
        // state no installer worker can exist, but multiple recovery exit
        // channels can still race. Keep them on the same typed single-flight
        // coordinator instead of granting each one cleanup ownership.
        return PRE_APP_PROCESS_LIFECYCLE
            .lock()
            .map(|mut coordinator| ProcessLifecycleClaimReceipt {
                claim: coordinator.claim(requested),
                origin: ProcessLifecycleCoordinatorOrigin::PreApp,
            })
            .map_err(|_| {
                crate::codex_desktop::error::InstallerError::new(
                    crate::codex_desktop::error::InstallerErrorCode::InternalError,
                )
                .with_diagnostic_message(
                    "pre-application process lifecycle synchronization is unavailable",
                )
            });
    };
    state
        .codex_desktop_service
        .claim_process_lifecycle_transition(requested)
        .map(|claim| ProcessLifecycleClaimReceipt {
            claim,
            origin: ProcessLifecycleCoordinatorOrigin::Service,
        })
}

fn claim_process_lifecycle_transition_for_exit(
    app_handle: &tauri::AppHandle,
) -> Result<ProcessLifecycleClaimReceipt, crate::codex_desktop::error::InstallerError> {
    claim_process_lifecycle_transition(app_handle, ProcessLifecycleTransition::Exit)
}

fn start_claimed_exit_cleanup(app_handle: tauri::AppHandle, receipt: ProcessLifecycleClaimReceipt) {
    match receipt.claim {
        ProcessLifecycleClaim::StartCleanup(_) => {
            start_process_lifecycle_cleanup(app_handle, receipt, std::time::Duration::ZERO);
        }
        ProcessLifecycleClaim::CleanupInProgress(selected) => {
            log::info!(
                "Process lifecycle cleanup is already in progress; merged exit into {selected:?}"
            );
        }
    }
}

/// Starts the one process-lifecycle cleanup worker authorized by
/// `ProcessLifecycleClaim::StartCleanup`.
///
/// The first accepted action remains frozen through cleanup. Conflicting later
/// requests join this worker without racing it with another cleanup/terminal
/// task or reversing that first intent.
pub(crate) fn start_process_lifecycle_cleanup(
    app_handle: tauri::AppHandle,
    receipt: ProcessLifecycleClaimReceipt,
    response_delay: std::time::Duration,
) {
    if !matches!(receipt.claim, ProcessLifecycleClaim::StartCleanup(_)) {
        log::warn!("Ignored process lifecycle cleanup start without ownership");
        return;
    }
    tauri::async_runtime::spawn(async move {
        if !response_delay.is_zero() {
            tokio::time::sleep(response_delay).await;
        }
        save_window_state_before_exit(&app_handle);
        cleanup_before_exit(&app_handle).await;

        let selected_transition = match receipt.origin {
            ProcessLifecycleCoordinatorOrigin::Service => {
                let Some(state) = app_handle.try_state::<store::AppState>() else {
                    log::error!(
                        "Installer lifecycle service disappeared before cleanup completion"
                    );
                    return;
                };
                match state
                    .codex_desktop_service
                    .finalize_process_lifecycle_transition()
                {
                    Ok(Some(selected)) => selected,
                    Ok(None) => {
                        log::warn!(
                            "Ignored duplicate process lifecycle completion after shared cleanup"
                        );
                        return;
                    }
                    Err(error) => {
                        log::error!(
                            "Process lifecycle state could not be finalized after cleanup: {:?}",
                            error.code()
                        );
                        return;
                    }
                }
            }
            ProcessLifecycleCoordinatorOrigin::PreApp => match PRE_APP_PROCESS_LIFECYCLE.lock() {
                Ok(mut coordinator) => match coordinator.finalize() {
                    Some(selected) => selected,
                    None => {
                        log::warn!(
                            "Ignored duplicate pre-application process lifecycle completion"
                        );
                        return;
                    }
                },
                Err(_) => {
                    log::error!(
                        "Pre-application process lifecycle state could not be finalized after cleanup"
                    );
                    return;
                }
            },
        };

        if selected_transition == ProcessLifecycleTransition::Restart {
            log::info!("清理完成，重启应用");
            app_handle.restart();
        }

        // 先于 std::process::exit 显式移除托盘图标。
        // 进程直接退出时 Tauri 运行时不走正常 Drop 流程，
        // 不会向 Windows Shell 发送 NIM_DELETE，导致已退出的进程
        // 注册的图标仍残留在系统托盘（鼠标悬停 Shell 才会重绘发现进程已死）。
        remove_tray_icon_before_exit(&app_handle);
        log::info!("清理完成，退出应用");

        // 短暂等待确保所有 I/O 操作（如数据库写入）刷新到磁盘。
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // 使用 std::process::exit 避免再次触发 ExitRequested。
        std::process::exit(0);
    });
}

/// Waits for a previously accepted installer cancellation to become terminal
/// before the normal exit cleanup begins. The job remains non-terminal until
/// its worker has stopped cancellable I/O and removed its temporary data.
fn exit_after_installer_cancellation(app_handle: tauri::AppHandle, job_id: String) {
    tauri::async_runtime::spawn(async move {
        loop {
            let job = match app_handle.try_state::<store::AppState>() {
                Some(state) => match state.codex_desktop_service.get_job() {
                    Ok(job) => job,
                    Err(error) => {
                        log::warn!(
                            "Codex desktop installer state could not be read while waiting to exit: {:?}",
                            error.code()
                        );
                        show_codex_desktop_exit_status_unavailable_dialog(&app_handle);
                        return;
                    }
                },
                None => {
                    show_codex_desktop_exit_status_unavailable_dialog(&app_handle);
                    return;
                }
            };

            match job {
                Some(snapshot)
                    if snapshot.job_id == job_id && snapshot.stage == JobStage::Cancelled =>
                {
                    match claim_process_lifecycle_transition_for_exit(&app_handle) {
                        Ok(claim) => {
                            start_claimed_exit_cleanup(app_handle.clone(), claim);
                        }
                        Err(error) => {
                            log::warn!(
                                "Process exit lost the installer lifecycle claim race: {:?}",
                                error.code()
                            );
                            show_codex_desktop_installation_wait_dialog(&app_handle);
                        }
                    }
                    return;
                }
                Some(snapshot)
                    if snapshot.job_id == job_id
                        && snapshot.stage.is_cancellable()
                        && !snapshot.cancellable => {}
                Some(snapshot)
                    if snapshot.job_id == job_id
                        && matches!(
                            snapshot.stage,
                            JobStage::Installing | JobStage::VerifyingInstallation
                        ) =>
                {
                    show_codex_desktop_installation_wait_dialog(&app_handle);
                    return;
                }
                _ => {
                    show_codex_desktop_exit_status_unavailable_dialog(&app_handle);
                    return;
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
}

// ============================================================
// 应用退出清理
// ============================================================

/// 应用退出前的清理工作
///
/// 在应用退出前检查代理服务器状态，如果正在运行则停止代理并恢复 Live 配置。
/// 确保 Claude Code/Codex/Gemini 的配置不会处于损坏状态。
/// 使用 stop_with_restore_keep_state 保留 settings 表中的代理状态，下次启动时自动恢复。
pub async fn cleanup_before_exit(app_handle: &tauri::AppHandle) {
    if let Some(state) = app_handle.try_state::<store::AppState>() {
        let proxy_service = &state.proxy_service;

        // 退出时也需要兜底：代理可能已崩溃/未运行，但 Live 接管残留仍在（占位符/备份）。
        let has_backups = match state.db.has_any_live_backup().await {
            Ok(v) => v,
            Err(e) => {
                log::error!("退出时检查 Live 备份失败: {e}");
                false
            }
        };
        let live_taken_over = proxy_service.detect_takeover_in_live_configs();
        let needs_restore = has_backups || live_taken_over;

        if needs_restore {
            log::info!("检测到接管残留，开始恢复 Live 配置（保留代理状态）...");
            // 使用 keep_state 版本，保留 settings 表中的代理状态
            if let Err(e) = proxy_service.stop_with_restore_keep_state().await {
                log::error!("退出时恢复 Live 配置失败: {e}");
            } else {
                log::info!("已恢复 Live 配置（代理状态已保留，下次启动将自动恢复）");
            }
            return;
        }

        // 非接管模式：代理在运行则仅停止代理
        if proxy_service.is_running().await {
            log::info!("检测到代理服务器正在运行，开始停止...");
            if let Err(e) = proxy_service.stop().await {
                log::error!("退出时停止代理失败: {e}");
            }
            log::info!("代理服务器清理完成");
        }
    }
}

/// 主动从系统托盘移除托盘图标。
///
/// `std::process::exit` 会绕过 Tauri 运行时，触发不了 `TrayIcon::drop()`，
/// 也就不会向 Windows Shell 发 `NIM_DELETE`。结果是进程退出后托盘里
/// 仍保留一个死图标的缓存占位（Shell 不会主动重绘，需要鼠标悬停才刷新）。
///
/// 通过 `set_visible(false)` 走 `WM_USER_HIDE_TRAYICON` 消息路径，
/// 触发 tray-icon 内部的 `remove_tray_icon` → `Shell_NotifyIconW(NIM_DELETE)`，
/// 在进程结束前干净地把图标摘掉。其它平台 `set_visible(false)` 也是
/// 正常的隐藏/移除语义，作为跨平台兜底也安全。
pub(crate) fn remove_tray_icon_before_exit(app_handle: &tauri::AppHandle) {
    if let Some(tray) = app_handle.tray_by_id(tray::TRAY_ID) {
        if let Err(e) = tray.set_visible(false) {
            log::warn!("退出时移除托盘图标失败: {e}");
        } else {
            log::info!("已显式从系统托盘移除图标");
        }
    }
}

// ============================================================
// 启动时恢复代理状态
// ============================================================

/// 启动时根据 proxy_config 表中的代理状态自动恢复代理服务
///
/// 检查 `proxy_config.enabled` 字段，如果有任一应用的状态为 `true`，
/// 则自动启动代理服务并接管对应应用的 Live 配置。
const PROXY_STARTUP_APP_TYPES: [&str; 4] = ["claude", "codex", "gemini", "grokbuild"];

async fn enabled_proxy_apps_on_startup(db: &database::Database) -> Vec<&'static str> {
    let mut apps = Vec::new();
    for app_type in PROXY_STARTUP_APP_TYPES {
        if db
            .get_proxy_config_for_app(app_type)
            .await
            .is_ok_and(|config| config.enabled)
        {
            apps.push(app_type);
        }
    }
    apps
}

async fn restore_proxy_state_on_startup(state: &store::AppState) {
    // 收集需要恢复接管的应用列表（从 proxy_config.enabled 读取）
    let apps_to_restore = enabled_proxy_apps_on_startup(&state.db).await;

    if apps_to_restore.is_empty() {
        log::debug!("启动时无需恢复代理状态");
        return;
    }

    log::info!("检测到上次代理状态需要恢复，应用列表: {apps_to_restore:?}");

    // 逐个恢复接管状态
    for app_type in apps_to_restore {
        match state
            .proxy_service
            .set_takeover_for_app(app_type, true)
            .await
        {
            Ok(()) => {
                log::info!("✓ 已恢复 {app_type} 的代理接管状态");
            }
            Err(e) => {
                log::error!("✗ 恢复 {app_type} 的代理接管状态失败: {e}");
                // 失败时清除该应用的状态，避免下次启动再次尝试
                if let Err(clear_err) = state
                    .proxy_service
                    .set_takeover_for_app(app_type, false)
                    .await
                {
                    log::error!("清除 {app_type} 代理状态失败: {clear_err}");
                }
            }
        }
    }
}

fn initialize_common_config_snippets(state: &store::AppState) {
    // Auto-extract common config snippets from clean live files when snippet is missing.
    // This must run before proxy takeover is restored on startup, otherwise we'd read
    // proxy-placeholder configs instead of the user's actual live settings.
    for app_type in crate::app_config::AppType::all() {
        if !state
            .db
            .should_auto_extract_config_snippet(app_type.as_str())
            .unwrap_or(false)
        {
            continue;
        }

        let settings = match crate::services::provider::ProviderService::read_live_settings(
            app_type.clone(),
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        match crate::services::provider::ProviderService::extract_common_config_snippet_from_settings(
            app_type.clone(),
            &settings,
        ) {
            Ok(snippet) if !snippet.is_empty() && snippet != "{}" => {
                match state.db.set_config_snippet(app_type.as_str(), Some(snippet)) {
                    Ok(()) => {
                        let _ = state.db.set_config_snippet_cleared(app_type.as_str(), false);
                        log::info!(
                            "✓ Auto-extracted common config snippet for {}",
                            app_type.as_str()
                        );
                    }
                    Err(e) => log::warn!(
                        "✗ Failed to save config snippet for {}: {e}",
                        app_type.as_str()
                    ),
                }
            }
            Ok(_) => log::debug!(
                "○ Live config for {} has no extractable common fields",
                app_type.as_str()
            ),
            Err(e) => log::warn!(
                "✗ Failed to extract config snippet for {}: {e}",
                app_type.as_str()
            ),
        }
    }

    let should_run_legacy_migration = state
        .db
        .is_legacy_common_config_migrated()
        .map(|done| !done)
        .unwrap_or(true);

    if should_run_legacy_migration {
        for app_type in [
            crate::app_config::AppType::Claude,
            crate::app_config::AppType::Codex,
            crate::app_config::AppType::Gemini,
        ] {
            if let Err(e) = crate::services::provider::ProviderService::migrate_legacy_common_config_usage_if_needed(
                state,
                app_type.clone(),
            ) {
                log::warn!(
                    "✗ Failed to migrate legacy common-config usage for {}: {e}",
                    app_type.as_str()
                );
            }
        }

        if let Err(e) = state.db.set_legacy_common_config_migrated(true) {
            log::warn!("✗ Failed to persist legacy common-config migration flag: {e}");
        }
    }
}

// ============================================================
// 迁移错误对话框辅助函数
// ============================================================

/// 检测是否为中文环境
fn is_chinese_locale() -> bool {
    std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .map(|lang| lang.starts_with("zh"))
        .unwrap_or(false)
}

/// 显示迁移错误对话框
/// 返回 true 表示用户选择重试，false 表示用户选择退出
fn show_migration_error_dialog(app: &tauri::AppHandle, error: &str) -> bool {
    let title = if is_chinese_locale() {
        "配置迁移失败"
    } else {
        "Migration Failed"
    };

    let message = if is_chinese_locale() {
        format!(
            "从旧版本迁移配置时发生错误：\n\n{error}\n\n\
            您的数据尚未丢失，旧配置文件仍然保留。\n\
            建议回退到旧版本 FyAgent 以保护数据。\n\n\
            点击「重试」重新尝试迁移\n\
            点击「退出」关闭程序（可回退版本后重新打开）"
        )
    } else {
        format!(
            "An error occurred while migrating configuration:\n\n{error}\n\n\
            Your data is NOT lost - the old config file is still preserved.\n\
            Consider rolling back to an older FyAgent version.\n\n\
            Click 'Retry' to attempt migration again\n\
            Click 'Exit' to close the program"
        )
    };

    let retry_text = if is_chinese_locale() {
        "重试"
    } else {
        "Retry"
    };
    let exit_text = if is_chinese_locale() {
        "退出"
    } else {
        "Exit"
    };

    // 使用 blocking_show 同步等待用户响应
    // OkCancelCustom: 第一个按钮（重试）返回 true，第二个按钮（退出）返回 false
    app.dialog()
        .message(&message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCancelCustom(
            retry_text.to_string(),
            exit_text.to_string(),
        ))
        .blocking_show()
}

/// 显示数据库初始化/Schema 迁移失败对话框
/// 返回 true 表示用户选择重试，false 表示用户选择退出
fn show_database_init_error_dialog(
    app: &tauri::AppHandle,
    db_path: &std::path::Path,
    error: &str,
) -> bool {
    let title = if is_chinese_locale() {
        "数据库初始化失败"
    } else {
        "Database Initialization Failed"
    };

    let message = if is_chinese_locale() {
        format!(
            "初始化数据库或迁移数据库结构时发生错误：\n\n{error}\n\n\
            数据库文件路径：\n{db}\n\n\
            您的数据尚未丢失，应用不会自动删除数据库文件。\n\
            常见原因包括：数据库版本过新、文件损坏、权限不足、磁盘空间不足等。\n\n\
            建议：\n\
            1) 先备份整个配置目录（包含 fyagent.db）\n\
            2) 如果提示“数据库版本过新”，请升级到更新版本\n\
            3) 如果刚升级出现异常，可回退旧版本导出/备份后再升级\n\n\
            点击「重试」重新尝试初始化\n\
            点击「退出」关闭程序",
            db = db_path.display()
        )
    } else {
        format!(
            "An error occurred while initializing or migrating the database:\n\n{error}\n\n\
            Database file path:\n{db}\n\n\
            Your data is NOT lost - the app will not delete the database automatically.\n\
            Common causes include: newer database version, corrupted file, permission issues, or low disk space.\n\n\
            Suggestions:\n\
            1) Back up the entire config directory (including fyagent.db)\n\
            2) If you see “database version is newer”, please upgrade FyAgent\n\
            3) If this happened right after upgrading, consider rolling back to export/backup then upgrade again\n\n\
            Click 'Retry' to attempt initialization again\n\
            Click 'Exit' to close the program",
            db = db_path.display()
        )
    };

    let retry_text = if is_chinese_locale() {
        "重试"
    } else {
        "Retry"
    };
    let exit_text = if is_chinese_locale() {
        "退出"
    } else {
        "Exit"
    };

    app.dialog()
        .message(&message)
        .title(title)
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCancelCustom(
            retry_text.to_string(),
            exit_text.to_string(),
        ))
        .blocking_show()
}

fn confirm_codex_desktop_installation_cancellation(app: &tauri::AppHandle) -> bool {
    let (title, message, confirm, keep_running) = if is_chinese_locale() {
        (
            "FyAgent",
            "Codex 桌面应用仍在下载或校验。取消后，FyAgent 会等待清理完成再退出。",
            "取消并退出",
            "继续安装",
        )
    } else {
        (
            "FyAgent",
            "Codex desktop is still downloading or verifying. FyAgent will wait for cleanup before it exits.",
            "Cancel and exit",
            "Keep installing",
        )
    };

    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            confirm.to_string(),
            keep_running.to_string(),
        ))
        .blocking_show()
}

fn show_codex_desktop_installation_wait_dialog(app: &tauri::AppHandle) {
    let (title, message, acknowledge) = if is_chinese_locale() {
        (
            "FyAgent",
            "Codex 桌面应用正在安装或进行安装后校验。为避免中断系统部署，请等待完成后再退出。",
            "我知道了",
        )
    } else {
        (
            "FyAgent",
            "Codex desktop is installing or being verified after installation. Wait for it to finish before exiting so the system deployment is not interrupted.",
            "OK",
        )
    };

    let _ = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCustom(acknowledge.to_string()))
        .blocking_show();
}

fn show_codex_desktop_cancellation_wait_dialog(app: &tauri::AppHandle) {
    let (title, message, acknowledge) = if is_chinese_locale() {
        (
            "FyAgent",
            "Codex 桌面应用的取消请求正在完成清理。FyAgent 会在清理完成后退出。",
            "继续等待",
        )
    } else {
        (
            "FyAgent",
            "Codex desktop is finishing cancellation cleanup. FyAgent will exit when cleanup completes.",
            "Keep waiting",
        )
    };

    let _ = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCustom(acknowledge.to_string()))
        .blocking_show();
}

fn show_codex_desktop_exit_status_unavailable_dialog(app: &tauri::AppHandle) {
    let (title, message, acknowledge) = if is_chinese_locale() {
        (
            "FyAgent",
            "无法确认 Codex 桌面应用安装任务的安全状态。请稍候后重试退出。",
            "我知道了",
        )
    } else {
        (
            "FyAgent",
            "FyAgent could not confirm that the Codex desktop installation task is safe to exit. Please wait briefly and try again.",
            "OK",
        )
    };

    let _ = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCustom(acknowledge.to_string()))
        .blocking_show();
}

// ============================================================
// 退出请求分类
// ============================================================

/// `RunEvent::ExitRequested` 的三类来源，处理方式必须区分。
///
/// 关键约束：重启请求（`code == RESTART_EXIT_CODE`）上 `prevent_exit()` 会被
/// Tauri 静默忽略（见 `ExitRequestApi::prevent_exit` 文档），事件循环必定继续
/// 退出并触发各插件的 `RunEvent::Exit` 钩子；任何与之并发的自定义清理任务都
/// 可能与插件退出钩子争用同一状态而死锁。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitRequestAction {
    /// `code` 为 `None`：运行时自动触发（如隐藏窗口的 WebView 被回收导致无存活
    /// 窗口），阻止退出、保持托盘后台运行。
    StayInTray,
    /// `code` 为 `RESTART_EXIT_CODE`：`app.restart()` / 自更新 relaunch 发起的
    /// 重启，不拦截、不做自定义清理，交还 Tauri 默认 re-exec 流程。
    DeferToTauriRestart,
    /// 其它 `Some(_)`：用户主动退出（托盘「退出」等），执行完整异步清理后结束进程。
    CleanupAndExit,
}

fn classify_exit_request(code: Option<i32>) -> ExitRequestAction {
    match code {
        None => ExitRequestAction::StayInTray,
        Some(tauri::RESTART_EXIT_CODE) => ExitRequestAction::DeferToTauriRestart,
        Some(_) => ExitRequestAction::CleanupAndExit,
    }
}

/// Installer-aware exit outcome. Cancellation remains distinct from terminal
/// `Cancelled`: the latter is published only after the background worker has
/// acknowledged its temporary-directory cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexDesktopExitProtection {
    AllowExit,
    ConfirmCancellation,
    WaitForCancellation,
    WaitForInstallation,
}

fn classify_codex_desktop_exit_protection(
    job: Option<(JobStage, bool)>,
) -> CodexDesktopExitProtection {
    match job {
        None => CodexDesktopExitProtection::AllowExit,
        Some((stage, _)) if stage.is_terminal() => CodexDesktopExitProtection::AllowExit,
        Some((stage, true)) if stage.is_cancellable() => {
            CodexDesktopExitProtection::ConfirmCancellation
        }
        Some((stage, false)) if stage.is_cancellable() => {
            CodexDesktopExitProtection::WaitForCancellation
        }
        Some((JobStage::Installing | JobStage::VerifyingInstallation, _)) => {
            CodexDesktopExitProtection::WaitForInstallation
        }
        // New non-terminal stages must fail closed until their exit semantics
        // are deliberately classified with the state-machine owner.
        Some(_) => CodexDesktopExitProtection::WaitForInstallation,
    }
}

// ============================================================
// 在应用主动退出前显式持久化窗口状态
// ============================================================

#[cfg(not(target_os = "windows"))]
fn window_state_flags() -> StateFlags {
    StateFlags::POSITION | StateFlags::SIZE | StateFlags::MAXIMIZED
}

/// 当前应用的退出路径会拦截 `ExitRequested` 并最终直接 `std::process::exit(0)`，
/// 这里需要在真正结束进程前手动落盘，避免 window-state 插件的默认退出钩子被绕过。
pub fn save_window_state_before_exit(app_handle: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    let result = crate::windows_window_state::save(app_handle);
    #[cfg(not(target_os = "windows"))]
    let result = app_handle
        .save_window_state(window_state_flags())
        .map_err(|error| error.to_string());

    if let Err(err) = result {
        log::error!("退出前保存窗口状态失败: {err}");
    } else {
        log::info!("已在退出前保存窗口状态");
    }
}

/// 主动释放 single-instance 锁。
///
/// 所有桌面平台使用 single-instance 插件。我们有若干路径会直接
/// `std::process::exit(0)`，不会触发插件挂在 `RunEvent::Exit` 上的清理钩子。
/// 重启前主动释放可以避免新进程误连旧 listener 后自行退出。
pub fn destroy_single_instance_lock(app_handle: &tauri::AppHandle) {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    tauri_plugin_single_instance::destroy(app_handle);

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let _ = app_handle;
}

#[cfg(test)]
mod tests {
    use super::{
        classify_codex_desktop_exit_protection, classify_exit_request,
        enabled_proxy_apps_on_startup, fallback_monitor_index, redact_url_for_log,
        redact_url_for_log_with_secrets, redact_url_origin_for_log, runtime_log_level_allows,
        should_exit_lightweight_mode, ActivationEnqueueResult, ActivationInbox,
        CodexDesktopExitProtection, ExitRequestAction, PendingActivation, PhysicalMonitorWorkArea,
        MAX_PENDING_ACTIVATIONS,
    };
    use crate::codex_desktop::types::JobStage;
    use crate::database::Database;

    fn monitor_area(x: i32, width: u32) -> PhysicalMonitorWorkArea {
        PhysicalMonitorWorkArea {
            x,
            y: 0,
            width,
            height: 1080,
        }
    }

    #[test]
    fn disconnected_monitor_fallback_prefers_primary_then_first_available() {
        let available = [monitor_area(-1920, 1920), monitor_area(0, 2560)];

        assert_eq!(
            fallback_monitor_index(&available, Some(monitor_area(0, 2560))),
            Some(1)
        );
        assert_eq!(fallback_monitor_index(&available, None), Some(0));
        assert_eq!(
            fallback_monitor_index(&available, Some(monitor_area(5000, 1280))),
            Some(0)
        );
        assert_eq!(fallback_monitor_index(&[], None), None);
    }

    #[test]
    fn semantic_activation_queue_waits_for_ready_and_drains_fifo() {
        let mut inbox = ActivationInbox::default();
        let request = crate::deeplink::DeepLinkImportRequest {
            version: "v1".to_owned(),
            resource: "provider".to_owned(),
            name: Some("queued".to_owned()),
            ..Default::default()
        };

        assert_eq!(
            inbox.enqueue(PendingActivation::DeepLink {
                request: Box::new(request),
                focus_main_window: true,
            }),
            ActivationEnqueueResult::Queued
        );
        assert_eq!(
            inbox.enqueue(PendingActivation::InvalidDeepLink {
                focus_main_window: false,
            }),
            ActivationEnqueueResult::Queued
        );
        assert_eq!(
            inbox.enqueue(PendingActivation::Focus),
            ActivationEnqueueResult::Queued
        );
        assert!(inbox.mark_ready());
        assert!(matches!(
            inbox.take_next(),
            Some(PendingActivation::DeepLink { .. })
        ));
        assert!(matches!(
            inbox.take_next(),
            Some(PendingActivation::InvalidDeepLink { .. })
        ));
        assert!(matches!(inbox.take_next(), Some(PendingActivation::Focus)));
        assert!(inbox.take_next().is_none());
        assert!(!inbox.draining);
    }

    #[test]
    fn semantic_activation_wake_policy_matches_its_focus_effect() {
        let request = || {
            Box::new(crate::deeplink::DeepLinkImportRequest {
                version: "v1".to_owned(),
                resource: "provider".to_owned(),
                ..Default::default()
            })
        };

        assert!(PendingActivation::Focus.should_wake_main_window());
        assert!(PendingActivation::InvalidDeepLink {
            focus_main_window: true,
        }
        .should_wake_main_window());
        assert!(!PendingActivation::InvalidDeepLink {
            focus_main_window: false,
        }
        .should_wake_main_window());
        assert!(PendingActivation::DeepLink {
            request: request(),
            focus_main_window: true,
        }
        .should_wake_main_window());
        assert!(!PendingActivation::DeepLink {
            request: request(),
            focus_main_window: false,
        }
        .should_wake_main_window());
    }

    #[test]
    fn semantic_activation_queue_is_bounded_and_coalesces_focus() {
        let mut inbox = ActivationInbox::default();
        assert_eq!(
            inbox.enqueue(PendingActivation::Focus),
            ActivationEnqueueResult::Queued
        );
        assert_eq!(
            inbox.enqueue(PendingActivation::Focus),
            ActivationEnqueueResult::Coalesced
        );

        for index in 1..MAX_PENDING_ACTIVATIONS {
            let request = crate::deeplink::DeepLinkImportRequest {
                version: "v1".to_owned(),
                resource: "provider".to_owned(),
                name: Some(format!("queued-{index}")),
                ..Default::default()
            };
            assert_eq!(
                inbox.enqueue(PendingActivation::DeepLink {
                    request: Box::new(request),
                    focus_main_window: true,
                }),
                ActivationEnqueueResult::Queued
            );
        }

        assert_eq!(inbox.pending.len(), MAX_PENDING_ACTIVATIONS);
        assert_eq!(
            inbox.enqueue(PendingActivation::DeepLink {
                request: Box::new(crate::deeplink::DeepLinkImportRequest::default()),
                focus_main_window: true,
            }),
            ActivationEnqueueResult::RejectedAtCapacity
        );
    }

    #[test]
    fn waking_activation_displaces_a_non_waking_item_at_capacity() {
        let mut inbox = ActivationInbox::default();
        for _ in 0..MAX_PENDING_ACTIVATIONS {
            assert_eq!(
                inbox.enqueue(PendingActivation::InvalidDeepLink {
                    focus_main_window: false,
                }),
                ActivationEnqueueResult::Queued
            );
        }

        let activation = PendingActivation::Focus;
        assert!(should_exit_lightweight_mode(true, &activation));
        assert_eq!(inbox.enqueue(activation), ActivationEnqueueResult::Queued);
        assert_eq!(inbox.pending.len(), MAX_PENDING_ACTIVATIONS);
        assert_eq!(
            inbox
                .pending
                .iter()
                .filter(|queued| !queued.should_wake_main_window())
                .count(),
            MAX_PENDING_ACTIVATIONS - 1
        );
        assert!(matches!(
            inbox.pending.back(),
            Some(PendingActivation::Focus)
        ));
    }

    #[test]
    fn capacity_rejection_does_not_change_a_waking_activation_exit_policy() {
        let mut inbox = ActivationInbox::default();
        for index in 0..MAX_PENDING_ACTIVATIONS {
            assert_eq!(
                inbox.enqueue(PendingActivation::DeepLink {
                    request: Box::new(crate::deeplink::DeepLinkImportRequest {
                        version: "v1".to_owned(),
                        resource: "provider".to_owned(),
                        name: Some(format!("waking-{index}")),
                        ..Default::default()
                    }),
                    focus_main_window: true,
                }),
                ActivationEnqueueResult::Queued
            );
        }

        let activation = PendingActivation::Focus;
        let should_exit = should_exit_lightweight_mode(true, &activation);
        assert_eq!(
            inbox.enqueue(activation),
            ActivationEnqueueResult::RejectedAtCapacity
        );
        assert!(should_exit);
        assert_eq!(inbox.pending.len(), MAX_PENDING_ACTIVATIONS);
    }

    #[test]
    fn renderer_reload_pauses_drain_without_losing_the_next_semantic() {
        let mut inbox = ActivationInbox {
            renderer_ready: true,
            ..ActivationInbox::default()
        };
        assert_eq!(
            inbox.enqueue(PendingActivation::Focus),
            ActivationEnqueueResult::StartDrain
        );
        assert!(matches!(inbox.take_next(), Some(PendingActivation::Focus)));
        assert_eq!(
            inbox.enqueue(PendingActivation::InvalidDeepLink {
                focus_main_window: false,
            }),
            ActivationEnqueueResult::Queued
        );

        inbox.mark_unready();
        assert!(inbox.take_next().is_none());
        assert_eq!(inbox.pending.len(), 1);
        assert!(inbox.mark_ready());
        assert!(matches!(
            inbox.take_next(),
            Some(PendingActivation::InvalidDeepLink { .. })
        ));
        assert!(inbox.take_next().is_none());
    }

    #[test]
    fn log_url_redaction_strips_credentials_and_query_keeps_path() {
        // userinfo 与整个 query 剥离，path 保留用于诊断 base_url 配错。
        assert_eq!(
            redact_url_for_log(
                "https://user:secret@example.com:8443/v1/models?key=top-secret&alt=sse"
            ),
            "https://example.com:8443/v1/models"
        );
        // scheme-relative 保持形态，userinfo 去掉。
        assert_eq!(
            redact_url_for_log("//user:sk-secret@gw.example.com/v1"),
            "//gw.example.com/v1"
        );
        // 无 scheme 的裸 userinfo。
        assert_eq!(
            redact_url_for_log("user:sk-secret@gw.example.com/v1"),
            "gw.example.com/v1"
        );
        // 无法解析为绝对 URL 时：丢 query，其余原样保留。
        assert_eq!(redact_url_for_log("not-a-url?token=secret"), "not-a-url");
        // 不再对 path 段做“看起来像密钥”的形状猜测，正常路径完整保留。
        assert_eq!(
            redact_url_for_log("https://host.example/v1/models/gemini-2.5-pro"),
            "https://host.example/v1/models/gemini-2.5-pro"
        );
    }

    #[test]
    fn log_url_redaction_replaces_known_secret_values() {
        // 精确匹配已知密钥值：无论它出现在 path 还是别处都被抹掉。
        let secrets = vec!["k-9f3a7c2b1e".to_string()];
        assert_eq!(
            redact_url_for_log_with_secrets("https://gw.example.com/k-9f3a7c2b1e/v1", &secrets),
            "https://gw.example.com/[REDACTED]/v1"
        );
        // 过短(<8)的已知值不参与子串脱敏，避免误伤 /v1/ 之类的正常路径。
        let short_secrets = vec!["api".to_string()];
        assert_eq!(
            redact_url_for_log_with_secrets("https://api.example.com/v1", &short_secrets),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn log_url_origin_drops_path_for_credential_in_path() {
        // 没有已知密钥可脱敏时，凭据可能整个内嵌在 path，只记 origin。
        assert_eq!(
            redact_url_origin_for_log("https://gw.example.com/k-9f3a7c2b1e/v1"),
            "https://gw.example.com"
        );
        assert_eq!(
            redact_url_origin_for_log("https://user:pass@gw.example.com:8443/secret/v1"),
            "https://gw.example.com:8443"
        );
        assert_eq!(
            redact_url_origin_for_log("//gw.example.com/secret/v1"),
            "//gw.example.com"
        );
    }

    #[test]
    fn runtime_log_filter_honors_dynamic_max_level() {
        assert!(!runtime_log_level_allows(
            log::Level::Error,
            log::LevelFilter::Off
        ));
        assert!(runtime_log_level_allows(
            log::Level::Error,
            log::LevelFilter::Info
        ));
        assert!(runtime_log_level_allows(
            log::Level::Info,
            log::LevelFilter::Info
        ));
        assert!(!runtime_log_level_allows(
            log::Level::Debug,
            log::LevelFilter::Info
        ));
    }

    #[test]
    fn no_code_keeps_app_alive_in_tray() {
        assert_eq!(classify_exit_request(None), ExitRequestAction::StayInTray);
    }

    #[test]
    fn restart_exit_code_defers_to_tauri_default_restart() {
        assert_eq!(
            classify_exit_request(Some(tauri::RESTART_EXIT_CODE)),
            ExitRequestAction::DeferToTauriRestart
        );
    }

    #[test]
    fn user_exit_codes_run_cleanup_then_exit() {
        assert_eq!(
            classify_exit_request(Some(0)),
            ExitRequestAction::CleanupAndExit
        );
        assert_eq!(
            classify_exit_request(Some(1)),
            ExitRequestAction::CleanupAndExit
        );
    }

    #[test]
    fn installer_exit_protection_waits_for_worker_cleanup_before_exit() {
        assert_eq!(
            classify_codex_desktop_exit_protection(None),
            CodexDesktopExitProtection::AllowExit
        );
        assert_eq!(
            classify_codex_desktop_exit_protection(Some((JobStage::Downloading, true))),
            CodexDesktopExitProtection::ConfirmCancellation
        );
        assert_eq!(
            classify_codex_desktop_exit_protection(Some((JobStage::VerifyingDownload, false,))),
            CodexDesktopExitProtection::WaitForCancellation
        );
        assert_eq!(
            classify_codex_desktop_exit_protection(Some((JobStage::Installing, false))),
            CodexDesktopExitProtection::WaitForInstallation
        );
        assert_eq!(
            classify_codex_desktop_exit_protection(Some((JobStage::Succeeded, false))),
            CodexDesktopExitProtection::AllowExit
        );
    }

    #[test]
    fn every_normal_exit_cleanup_path_uses_the_single_lifecycle_cleanup_owner() {
        let source = include_str!("lib.rs").replace("\r\n", "\n");
        let handler_start = source
            .find("if let RunEvent::ExitRequested")
            .expect("the central exit handler remains present");
        let handler_end = source[handler_start..]
            .find("\n        #[cfg(target_os = \"macos\")]")
            .map(|offset| handler_start + offset)
            .expect("the central exit handler remains bounded");
        let handler = &source[handler_start..handler_end];
        let final_claim = handler
            .rfind("claim_process_lifecycle_transition_for_exit(app_handle)")
            .expect("the normal exit path claims the installer lifecycle slot");
        let final_cleanup = handler
            .rfind("start_claimed_exit_cleanup(app_handle.clone(), claim)")
            .expect("the normal exit path delegates to the typed cleanup owner");
        assert!(final_claim < final_cleanup);

        let cancellation_start = source
            .find("fn exit_after_installer_cancellation")
            .expect("the cancellation waiter remains present");
        let cancellation_end = source[cancellation_start..]
            .find("\n// ============================================================\n// 应用退出清理")
            .map(|offset| cancellation_start + offset)
            .expect("the cancellation waiter remains bounded");
        let cancellation = &source[cancellation_start..cancellation_end];
        let cancellation_claim = cancellation
            .find("claim_process_lifecycle_transition_for_exit(&app_handle)")
            .expect("terminal cancellation claims before process cleanup");
        let cancellation_cleanup = cancellation
            .find("start_claimed_exit_cleanup(app_handle.clone(), claim)")
            .expect("terminal cancellation delegates to the typed cleanup owner");
        assert!(cancellation_claim < cancellation_cleanup);

        let helper_start = source
            .find("fn start_claimed_exit_cleanup")
            .expect("the central typed cleanup owner helper remains present");
        let helper_end = source[helper_start..]
            .find("\n/// Starts the one process-lifecycle cleanup worker")
            .map(|offset| helper_start + offset)
            .expect("the cleanup owner helper remains bounded");
        let helper = &source[helper_start..helper_end];
        assert!(helper.contains("ProcessLifecycleClaim::StartCleanup(_)"));
        assert!(helper.contains("ProcessLifecycleClaim::CleanupInProgress(selected)"));
        assert_eq!(
            helper.matches("start_process_lifecycle_cleanup").count(),
            1,
            "an in-flight restart or exit must not spawn another cleanup worker"
        );

        assert!(!source.contains(concat!("pub fn restart_", "process")));
        assert!(!source.contains(concat!("tauri::process::", "restart")));
    }

    #[test]
    fn codex_desktop_ipc_keeps_seven_ordinary_commands_and_four_trusted_restart_commands() {
        const ORDINARY_COMMANDS: [&str; 7] = [
            "codex_desktop_get_local_status",
            "codex_desktop_check_latest",
            "codex_desktop_get_job",
            "codex_desktop_start_install",
            "codex_desktop_cancel_install",
            "codex_desktop_launch",
            "codex_desktop_open_log_directory",
        ];
        const TRUSTED_RESTART_COMMANDS: [&str; 4] = [
            "get_codex_desktop_runtime_status",
            "request_codex_desktop_restart",
            "continue_codex_desktop_restart_with_force",
            "cancel_codex_desktop_restart_with_force",
        ];

        let command_source = include_str!("commands/codex_desktop.rs").replace("\r\n", "\n");
        let library_source = include_str!("lib.rs").replace("\r\n", "\n");
        let type_source = include_str!("codex_desktop/types.rs").replace("\r\n", "\n");

        assert_eq!(
            command_source.matches("#[tauri::command]").count(),
            ORDINARY_COMMANDS.len() + TRUSTED_RESTART_COMMANDS.len()
        );
        for command in ORDINARY_COMMANDS {
            assert_eq!(
                command_source
                    .matches(&format!("pub async fn {command}("))
                    .count(),
                1,
                "ordinary command {command} must be declared exactly once"
            );
        }
        for command in TRUSTED_RESTART_COMMANDS {
            assert_eq!(
                command_source
                    .matches(&format!("pub async fn {command}("))
                    .count(),
                1,
                "trusted restart command {command} must be declared exactly once"
            );
        }
        let cancellation_signature_start = command_source
            .find("pub async fn cancel_codex_desktop_restart_with_force(")
            .expect("the force-confirmation cancellation command remains present");
        let cancellation_signature_end = command_source[cancellation_signature_start..]
            .find(") -> Result")
            .map(|offset| cancellation_signature_start + offset)
            .expect("the force-confirmation cancellation signature remains bounded");
        let cancellation_signature =
            &command_source[cancellation_signature_start..cancellation_signature_end];
        assert!(cancellation_signature.contains("token: String"));
        let cancellation_signature_lowercase = cancellation_signature.to_ascii_lowercase();
        for prohibited in ["pid", "process", "path", "launch", "command", "name"] {
            assert!(
                !cancellation_signature_lowercase.contains(prohibited),
                "force-confirmation cancellation must not accept {prohibited} input"
            );
        }

        let handler_start = library_source
            .find("tauri::generate_handler![")
            .expect("the Tauri invoke handler remains present");
        let handler_end = library_source[handler_start..]
            .find("\n        ]);")
            .map(|offset| handler_start + offset)
            .expect("the Tauri invoke handler remains bounded");
        let handler = &library_source[handler_start..handler_end];

        assert_eq!(
            handler.matches("commands::codex_desktop_").count(),
            ORDINARY_COMMANDS.len()
        );
        assert!(
            handler.contains("commands::agent_install_start_install"),
            "agent install commands must register beside, not inside, Codex Desktop"
        );
        assert_eq!(
            handler.matches("commands::agent_install_").count(),
            10,
            "agent_install IPC is a sibling surface and must not absorb Codex Desktop commands"
        );
        for command in ORDINARY_COMMANDS {
            assert!(
                handler.contains(&format!("commands::{command}")),
                "ordinary command {command} must remain registered"
            );
        }
        for command in TRUSTED_RESTART_COMMANDS {
            assert!(
                handler.contains(&format!("commands::{command}")),
                "trusted restart command {command} must remain registered"
            );
        }
        let handler_lowercase = handler.to_ascii_lowercase();
        assert!(!handler_lowercase.contains("all_users"));
        assert!(!handler_lowercase.contains("all-users"));

        let request_start = type_source
            .find("pub struct StartInstallRequest {")
            .expect("the ordinary start request remains present");
        let request_attribute_start = type_source[..request_start]
            .rfind("#[serde(")
            .expect("the ordinary start request remains serde constrained");
        let request_end = type_source[request_start..]
            .find("\n}\n\nimpl StartInstallRequest")
            .map(|offset| request_start + offset + 2)
            .expect("the ordinary start request remains bounded");
        let request_definition = &type_source[request_attribute_start..request_end];

        assert!(request_definition.contains("deny_unknown_fields"));
        let request_fields = request_definition
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("pub ") && !line.starts_with("pub struct"))
            .collect::<Vec<_>>();
        assert_eq!(request_fields, vec!["pub expected_release_id: String,"]);
        let request_lowercase = request_definition.to_ascii_lowercase();
        for prohibited in [
            "all_users",
            "all-users",
            "scope",
            "url",
            "path",
            "sha",
            "hash",
            "identity",
            "bypass",
        ] {
            assert!(
                !request_lowercase.contains(prohibited),
                "ordinary start request must not accept {prohibited}"
            );
        }
    }

    #[tokio::test]
    async fn startup_restore_includes_enabled_grokbuild_route() {
        let db = Database::memory().expect("initialize database");
        let mut config = db
            .get_proxy_config_for_app("grokbuild")
            .await
            .expect("read Grok Build proxy config");
        config.enabled = true;
        db.update_proxy_config_for_app(config)
            .await
            .expect("enable Grok Build proxy config");

        let apps = enabled_proxy_apps_on_startup(&db).await;

        assert_eq!(apps, vec!["grokbuild"]);
    }
}
