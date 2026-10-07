use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use crate::app_config::AppType;
use crate::error::AppError;
use crate::services::skill::{SkillStorageLocation, SyncMethod};

mod first_use_guide;
pub use first_use_guide::FirstUseGuideState;
pub(crate) use first_use_guide::{
    dismiss_first_use_guide, get_first_use_guide_state, initialize_first_use_guide,
};

/// 自定义端点配置（历史兼容，实际存储在 provider.meta.custom_endpoints）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEndpoint {
    pub url: String,
    pub added_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used: Option<i64>,
}

fn default_true() -> bool {
    true
}

/// 主页面显示的应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleApps {
    #[serde(default = "default_true")]
    pub claude: bool,
    #[serde(
        rename = "claude-desktop",
        alias = "claudeDesktop",
        alias = "claude_desktop",
        default = "default_true"
    )]
    pub claude_desktop: bool,
    #[serde(default = "default_true")]
    pub codex: bool,
    /// WorkBuddy is a top-level application surface, not an AppType/provider domain.
    #[serde(default = "default_true")]
    pub workbuddy: bool,
    #[serde(default = "default_true")]
    pub gemini: bool,
    #[serde(default = "default_true")]
    pub grokbuild: bool,
    #[serde(default = "default_true")]
    pub opencode: bool,
    #[serde(default = "default_true")]
    pub openclaw: bool,
    #[serde(default)]
    pub hermes: bool,
}

impl Default for VisibleApps {
    fn default() -> Self {
        Self {
            claude: true,
            claude_desktop: true,
            codex: true,
            workbuddy: true,
            gemini: true,
            grokbuild: true,
            opencode: true,
            openclaw: true,
            hermes: false, // 默认不显示，需用户手动启用
        }
    }
}

impl VisibleApps {
    /// Check if the specified app is visible
    pub fn is_visible(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::ClaudeDesktop => self.claude_desktop,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::GrokBuild => self.grokbuild,
            AppType::OpenCode => self.opencode,
            AppType::OpenClaw => self.openclaw,
            AppType::Hermes => self.hermes,
        }
    }
}

// Preserve any historical JSON shape without decoding a cloud configuration.
fn deserialize_legacy_cloud_value<'de, D>(
    deserializer: D,
) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(Some)
}

/// 本机自动迁移状态。
///
/// 这里记录的是本机启动时执行过的一次性迁移；标记不随数据库同步。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalMigrations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_third_party_history_provider_bucket_v1:
        Option<CodexThirdPartyHistoryProviderBucketMigration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_provider_template_v1: Option<CodexProviderTemplateMigration>,
    /// 统一会话开关的官方历史迁移标记。开关关闭时会被清除，
    /// 这样重新开启能把"关闭期间"落入 openai 桶的官方会话补迁进来。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_official_history_unify_v1: Option<CodexOfficialHistoryUnifyMigration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexThirdPartyHistoryProviderBucketMigration {
    pub completed_at: String,
    pub target_provider_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_provider_ids: Vec<String>,
    #[serde(default)]
    pub migrated_jsonl_files: usize,
    #[serde(default)]
    pub migrated_state_rows: usize,
    #[serde(default)]
    pub scanned_history_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexProviderTemplateMigration {
    pub completed_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrated_provider_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexOfficialHistoryUnifyMigration {
    pub completed_at: String,
    pub target_provider_id: String,
    #[serde(default)]
    pub migrated_jsonl_files: usize,
    #[serde(default)]
    pub migrated_state_rows: usize,
    /// 迁移时的规范化 Codex 目录。标记只对同一目录生效：
    /// 切换 codex_config_dir 后旧标记不会挡住新目录的迁移。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
}

/// 应用设置结构
///
/// 存储设备级别设置，保存在本地 `~/.fyagent/settings.json`，不随数据库同步。
/// 这确保了云同步场景下多设备可以独立运作。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    // ===== 设备级 UI 设置 =====
    #[serde(default = "default_show_in_tray")]
    pub show_in_tray: bool,
    #[serde(default = "default_minimize_to_tray_on_close")]
    pub minimize_to_tray_on_close: bool,
    /// 是否启用 Claude 插件联动
    #[serde(default)]
    pub enable_claude_plugin_integration: bool,
    /// 是否跳过 Claude Code 初次安装确认
    #[serde(default)]
    pub skip_claude_onboarding: bool,
    /// 是否开机自启
    #[serde(default)]
    pub launch_on_startup: bool,
    /// 静默启动（程序启动时不显示主窗口，仅托盘运行）
    #[serde(default)]
    pub silent_startup: bool,
    /// 是否在主页面启用本地代理功能（默认关闭）
    #[serde(default)]
    pub enable_local_proxy: bool,
    /// User has confirmed the local proxy first-run notice
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_confirmed: Option<bool>,
    /// User has confirmed the usage query first-run notice
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_confirmed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_dashboard_refresh_interval_ms: Option<u32>,
    /// Whether to show the failover toggle independently on the main page
    #[serde(default)]
    pub enable_failover_toggle: bool,
    /// Whether to show the project profile switcher on the main page header
    #[serde(default = "default_show_profile_switcher")]
    pub show_profile_switcher: bool,
    /// Legacy compatibility field. Third-party Codex switches always preserve
    /// official ChatGPT login material; this value is ignored.
    #[serde(default = "default_preserve_codex_official_auth_on_switch")]
    pub preserve_codex_official_auth_on_switch: bool,
    /// Run official Codex providers under the shared "custom" model_provider id
    /// so official sessions share one resume-history bucket with third-party
    /// providers. Opt-in: defaults to false.
    #[serde(default)]
    pub unify_codex_session_history: bool,
    /// User opted in (via the enable dialog checkbox) to migrate existing
    /// official sessions ("openai" bucket) into the shared bucket. Persisted so
    /// a failed migration retries at startup; cleared when the toggle turns off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unify_codex_migrate_existing: Option<bool>,
    /// User has confirmed the failover toggle first-run notice
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failover_confirmed: Option<bool>,
    /// User has confirmed the first-run welcome notice
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_run_notice_confirmed: Option<bool>,
    /// Device-local first-use eligibility, maintained only by the native guide owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_use_guide_state: Option<FirstUseGuideState>,
    /// User has confirmed the common config first-run notice
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub common_config_confirmed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Closed shell appearance preference. Renderer localStorage is a cache;
    /// this device file is the restart authority. `save_settings` snapshots
    /// cannot clobber it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance_theme: Option<String>,

    // ===== 主页面显示的应用 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_apps: Option<VisibleApps>,

    // ===== 设备级目录覆盖 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grok_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openclaw_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hermes_config_dir: Option<String>,

    // ===== 当前供应商 ID（设备级）=====
    /// 当前 Claude 供应商 ID（本地存储，优先于数据库 is_current）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_claude: Option<String>,
    /// 当前 Claude Desktop 供应商 ID（本地存储，优先于数据库 is_current）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_claude_desktop: Option<String>,
    /// 当前 Codex 供应商 ID（本地存储，优先于数据库 is_current）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_codex: Option<String>,
    /// 当前 Gemini 供应商 ID（本地存储，优先于数据库 is_current）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_gemini: Option<String>,
    /// 当前 Grok Build 供应商 ID（本地存储，优先于数据库 is_current）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_grokbuild: Option<String>,
    /// 当前 OpenCode 供应商 ID（本地存储，对 OpenCode 可能无意义，但保持结构一致）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_opencode: Option<String>,
    /// 当前 OpenClaw 供应商 ID（本地存储，对 OpenClaw 可能无意义，但保持结构一致）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_openclaw: Option<String>,
    /// 当前 Hermes 供应商 ID（本地存储，保持结构一致）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_hermes: Option<String>,

    // ===== Skill 同步设置 =====
    /// Skill 同步方式：auto（默认，优先 symlink）、symlink、copy
    #[serde(default)]
    pub skill_sync_method: SyncMethod,
    /// Skill 存储位置：fyagent（默认）或 unified（~/.agents/skills/）
    #[serde(default)]
    pub skill_storage_location: SkillStorageLocation,

    // Retired cloud configuration: opaque preservation only; never interpreted.
    #[serde(
        default,
        deserialize_with = "deserialize_legacy_cloud_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub webdav_sync: Option<serde_json::Value>,

    // Retired S3 configuration: opaque preservation only.
    #[serde(
        default,
        deserialize_with = "deserialize_legacy_cloud_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub s3_sync: Option<serde_json::Value>,

    // ===== WebDAV 备份设置（旧版，保留向后兼容）=====
    #[serde(
        default,
        deserialize_with = "deserialize_legacy_cloud_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub webdav_backup: Option<serde_json::Value>,

    // ===== 备份策略设置 =====
    /// Auto-backup interval in hours (default 24, 0 = disabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_interval_hours: Option<u32>,
    /// Maximum number of backup files to retain (default 10)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_retain_count: Option<u32>,

    // ===== 终端设置 =====
    /// 首选终端应用（可选，默认使用系统默认终端）
    /// - macOS: "terminal" | "iterm2" | "warp" | "alacritty" | "kitty" | "ghostty" | "wezterm" | "kaku"
    /// - Windows: "cmd" | "powershell" | "wt" (Windows Terminal)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_terminal: Option<String>,

    // ===== 本机自动迁移状态 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_migrations: Option<LocalMigrations>,
}

fn default_show_in_tray() -> bool {
    true
}

fn default_minimize_to_tray_on_close() -> bool {
    true
}

fn default_show_profile_switcher() -> bool {
    true
}

fn default_preserve_codex_official_auth_on_switch() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            show_in_tray: true,
            minimize_to_tray_on_close: true,
            enable_claude_plugin_integration: false,
            skip_claude_onboarding: false,
            launch_on_startup: false,
            silent_startup: false,
            enable_local_proxy: false,
            proxy_confirmed: None,
            usage_confirmed: None,
            usage_dashboard_refresh_interval_ms: None,
            enable_failover_toggle: false,
            show_profile_switcher: true,
            preserve_codex_official_auth_on_switch: true,
            unify_codex_session_history: false,
            unify_codex_migrate_existing: None,
            failover_confirmed: None,
            first_run_notice_confirmed: None,
            first_use_guide_state: None,
            common_config_confirmed: None,
            language: None,
            appearance_theme: None,
            visible_apps: None,
            claude_config_dir: None,
            codex_config_dir: None,
            gemini_config_dir: None,
            grok_config_dir: None,
            opencode_config_dir: None,
            openclaw_config_dir: None,
            hermes_config_dir: None,
            current_provider_claude: None,
            current_provider_claude_desktop: None,
            current_provider_codex: None,
            current_provider_gemini: None,
            current_provider_grokbuild: None,
            current_provider_opencode: None,
            current_provider_openclaw: None,
            current_provider_hermes: None,
            skill_sync_method: SyncMethod::default(),
            skill_storage_location: SkillStorageLocation::default(),
            webdav_sync: None,
            s3_sync: None,
            webdav_backup: None,
            backup_interval_hours: None,
            backup_retain_count: None,
            preferred_terminal: None,
            local_migrations: None,
        }
    }
}

const MACOS_PREFERRED_TERMINALS: &[&str] = &[
    "terminal",
    "iterm2",
    "alacritty",
    "kitty",
    "ghostty",
    "wezterm",
    "kaku",
    "warp",
];
const WINDOWS_PREFERRED_TERMINALS: &[&str] = &["cmd", "powershell", "wt"];

#[derive(Clone, Copy)]
struct PreferredTerminalConfiguration {
    supported: &'static [&'static str],
    default: &'static str,
}

const MACOS_PREFERRED_TERMINAL_CONFIGURATION: PreferredTerminalConfiguration =
    PreferredTerminalConfiguration {
        supported: MACOS_PREFERRED_TERMINALS,
        default: "terminal",
    };
const WINDOWS_PREFERRED_TERMINAL_CONFIGURATION: PreferredTerminalConfiguration =
    PreferredTerminalConfiguration {
        supported: WINDOWS_PREFERRED_TERMINALS,
        default: "cmd",
    };

fn preferred_terminal_configuration() -> Option<PreferredTerminalConfiguration> {
    if cfg!(target_os = "macos") {
        Some(MACOS_PREFERRED_TERMINAL_CONFIGURATION)
    } else if cfg!(target_os = "windows") {
        Some(WINDOWS_PREFERRED_TERMINAL_CONFIGURATION)
    } else {
        None
    }
}

fn normalize_preferred_terminal_for_configuration(
    value: Option<&str>,
    configuration: PreferredTerminalConfiguration,
) -> Option<String> {
    // Absence remains absence in persisted settings. Consumers that need an
    // executable choice obtain the platform default through the effective getter.
    value.map(|value| {
        let value = value.trim();
        if configuration.supported.contains(&value) {
            value.to_string()
        } else {
            configuration.default.to_string()
        }
    })
}

fn normalize_preferred_terminal(value: Option<&str>) -> Option<String> {
    normalize_preferred_terminal_for_configuration(value, preferred_terminal_configuration()?)
}

fn effective_preferred_terminal(value: Option<&str>) -> Option<String> {
    normalize_preferred_terminal(value).or_else(|| {
        preferred_terminal_configuration().map(|configuration| configuration.default.to_string())
    })
}

impl AppSettings {
    fn settings_path() -> Option<PathBuf> {
        // settings.json 保留用于旧版本迁移和无数据库场景
        Some(
            crate::config::get_home_dir()
                .join(".fyagent")
                .join("settings.json"),
        )
    }

    fn normalize_paths(&mut self) {
        self.claude_config_dir = self
            .claude_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.codex_config_dir = self
            .codex_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.gemini_config_dir = self
            .gemini_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.grok_config_dir = self
            .grok_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.opencode_config_dir = self
            .opencode_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.openclaw_config_dir = self
            .openclaw_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.hermes_config_dir = self
            .hermes_config_dir
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        self.language = self
            .language
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| matches!(*s, "en" | "zh" | "zh-TW" | "ja"))
            .map(|s| s.to_string());

        self.appearance_theme = self
            .appearance_theme
            .as_deref()
            .and_then(parse_appearance_theme)
            .map(str::to_string);

        self.preferred_terminal = normalize_preferred_terminal(self.preferred_terminal.as_deref());
    }

    fn from_json(content: &str) -> Result<Self, serde_json::Error> {
        let mut settings = serde_json::from_str::<Self>(content)?;
        settings.normalize_paths();
        Ok(settings)
    }

    fn load_from_file() -> Self {
        let Some(path) = Self::settings_path() else {
            return Self::default();
        };
        if let Ok(content) = fs::read_to_string(&path) {
            match Self::from_json(&content) {
                Ok(settings) => settings,
                Err(err) => {
                    log::warn!(
                        "解析设置文件失败，将使用默认设置。路径: {}, 错误: {}",
                        path.display(),
                        err
                    );
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }
}

fn save_settings_file(settings: &AppSettings) -> Result<(), AppError> {
    let mut normalized = settings.clone();
    normalized.normalize_paths();
    let Some(path) = AppSettings::settings_path() else {
        return Err(AppError::Config("无法获取用户主目录".to_string()));
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let json = serde_json::to_string_pretty(&normalized)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    #[cfg(target_os = "macos")]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| AppError::io(&path, e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| AppError::io(&path, e))?;
    }

    #[cfg(target_os = "windows")]
    {
        fs::write(&path, json).map_err(|e| AppError::io(&path, e))?;
    }

    Ok(())
}

static SETTINGS_STORE: OnceLock<RwLock<AppSettings>> = OnceLock::new();

fn settings_store() -> &'static RwLock<AppSettings> {
    SETTINGS_STORE.get_or_init(|| RwLock::new(AppSettings::load_from_file()))
}

fn resolve_override_path(raw: &str) -> PathBuf {
    if raw == "~" {
        return crate::config::get_home_dir();
    } else if let Some(stripped) = raw.strip_prefix("~/") {
        return crate::config::get_home_dir().join(stripped);
    } else if let Some(stripped) = raw.strip_prefix("~\\") {
        return crate::config::get_home_dir().join(stripped);
    }

    PathBuf::from(raw)
}

pub fn get_settings() -> AppSettings {
    let mut settings = settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .clone();
    settings.normalize_paths();
    settings
}

pub fn get_settings_for_frontend() -> AppSettings {
    let mut settings = get_settings();
    // Retired configuration never crosses the renderer boundary.
    settings.webdav_sync = None;
    settings.s3_sync = None;
    settings.webdav_backup = None;
    settings
}

pub(crate) fn parse_appearance_theme(value: &str) -> Option<&'static str> {
    match value.trim() {
        "light" => Some("light"),
        "dark" => Some("dark"),
        "system" => Some("system"),
        _ => None,
    }
}

pub(crate) fn appearance_theme_preference() -> Option<String> {
    get_settings()
        .appearance_theme
        .as_deref()
        .and_then(parse_appearance_theme)
        .map(str::to_string)
}

pub(crate) fn persist_appearance_theme(theme: &str) {
    let Some(theme) = parse_appearance_theme(theme) else {
        return;
    };
    if appearance_theme_preference().as_deref() == Some(theme) {
        return;
    }
    if let Err(error) = mutate_settings(|settings| {
        settings.appearance_theme = Some(theme.to_string());
    }) {
        log::warn!("Unable to persist appearance preference: {error}");
    }
}

pub(crate) fn appearance_bootstrap_script() -> Option<String> {
    appearance_bootstrap_script_for(appearance_theme_preference().as_deref())
}

pub(crate) fn appearance_bootstrap_script_for(preference: Option<&str>) -> Option<String> {
    let preference = parse_appearance_theme(preference?)?;
    let encoded = serde_json::to_string(preference).ok()?;
    Some(format!(
        r#"try{{var key="fyagent-theme";var preference={encoded};try{{localStorage.setItem(key,preference);}}catch(e){{}}var theme=preference==="system"?((window.matchMedia&&window.matchMedia("(prefers-color-scheme: dark)").matches)?"dark":"light"):preference;document.documentElement.setAttribute("data-theme",theme);}}catch(e){{}}"#,
    ))
}

pub fn update_settings(mut new_settings: AppSettings) -> Result<(), AppError> {
    new_settings.normalize_paths();
    save_settings_file(&new_settings)?;

    let mut guard = settings_store().write().unwrap_or_else(|e| {
        log::warn!("设置锁已毒化，使用恢复值: {e}");
        e.into_inner()
    });
    *guard = new_settings;
    Ok(())
}

/// Merge and persist a full settings payload against the latest in-memory
/// settings while holding the settings write lock.
///
/// Renderer settings payloads are snapshots. Backend-owned fields can change
/// after a renderer read (for example, while switching the current Provider),
/// so callers that merge protected fields must do the merge and persistence
/// under the same lock instead of composing `get_settings` + `update_settings`.
pub(crate) fn update_settings_with_latest<F>(
    incoming: AppSettings,
    merge: F,
) -> Result<(AppSettings, AppSettings), AppError>
where
    F: FnOnce(AppSettings, &AppSettings) -> AppSettings,
{
    let mut guard = settings_store().write().unwrap_or_else(|e| {
        log::warn!("设置锁已毒化，使用恢复值: {e}");
        e.into_inner()
    });
    let mut existing = guard.clone();
    existing.normalize_paths();
    let mut merged = merge(incoming, &existing);
    merged.normalize_paths();
    save_settings_file(&merged)?;
    *guard = merged.clone();
    Ok((existing, merged))
}

fn mutate_settings<F>(mutator: F) -> Result<(), AppError>
where
    F: FnOnce(&mut AppSettings),
{
    let mut guard = settings_store().write().unwrap_or_else(|e| {
        log::warn!("设置锁已毒化，使用恢复值: {e}");
        e.into_inner()
    });
    let mut next = guard.clone();
    mutator(&mut next);
    next.normalize_paths();
    save_settings_file(&next)?;
    *guard = next;
    Ok(())
}

pub fn is_codex_third_party_history_provider_bucket_migrated() -> bool {
    get_settings()
        .local_migrations
        .as_ref()
        .and_then(|migrations| {
            migrations
                .codex_third_party_history_provider_bucket_v1
                .as_ref()
        })
        .is_some_and(|m| m.scanned_history_files)
}

pub fn mark_codex_third_party_history_provider_bucket_migrated(
    migration: CodexThirdPartyHistoryProviderBucketMigration,
) -> Result<(), AppError> {
    mutate_settings(|settings| {
        let migrations = settings
            .local_migrations
            .get_or_insert_with(Default::default);
        migrations.codex_third_party_history_provider_bucket_v1 = Some(migration);
    })
}

pub fn is_codex_provider_template_migrated() -> bool {
    get_settings()
        .local_migrations
        .as_ref()
        .and_then(|migrations| migrations.codex_provider_template_v1.as_ref())
        .is_some()
}

pub fn mark_codex_provider_template_migrated(
    migration: CodexProviderTemplateMigration,
) -> Result<(), AppError> {
    mutate_settings(|settings| {
        let migrations = settings
            .local_migrations
            .get_or_insert_with(Default::default);
        migrations.codex_provider_template_v1 = Some(migration);
    })
}

/// 统一会话迁移标记是否覆盖指定目录。标记里没记目录（不应出现的旧格式）
/// 视为不匹配——重跑迁移是幂等的，宁可重迁也不漏迁。
pub fn is_codex_official_history_unify_migrated_for_dir(codex_dir: &str) -> bool {
    get_settings()
        .local_migrations
        .as_ref()
        .and_then(|migrations| migrations.codex_official_history_unify_v1.as_ref())
        .is_some_and(|migration| migration.codex_config_dir.as_deref() == Some(codex_dir))
}

/// 条件写入迁移完成标记：仅当此刻开关仍开启且迁移意愿仍在时才写。
/// 检查与写入在 settings 写锁内原子完成，与关闭开关路径
/// （`update_settings` / 清标记）串行，消除"迁移线程复查开关后、写标记前
/// 用户恰好关闭开关"的竞态窗口。返回是否实际写入。
pub fn mark_codex_official_history_unify_migrated_if_enabled(
    migration: CodexOfficialHistoryUnifyMigration,
) -> Result<bool, AppError> {
    let mut written = false;
    mutate_settings(|settings| {
        if settings.unify_codex_session_history
            && settings.unify_codex_migrate_existing.unwrap_or(false)
        {
            settings
                .local_migrations
                .get_or_insert_with(Default::default)
                .codex_official_history_unify_v1 = Some(migration);
            written = true;
        }
    })?;
    Ok(written)
}

pub fn clear_codex_official_history_unify_migration() -> Result<(), AppError> {
    mutate_settings(|settings| {
        if let Some(migrations) = settings.local_migrations.as_mut() {
            migrations.codex_official_history_unify_v1 = None;
        }
    })
}

pub fn unify_codex_migrate_existing_requested() -> bool {
    get_settings().unify_codex_migrate_existing.unwrap_or(false)
}

pub fn clear_codex_unify_migrate_existing() -> Result<(), AppError> {
    mutate_settings(|settings| {
        settings.unify_codex_migrate_existing = None;
    })
}

/// Test guards restore their cached snapshot without resolving a real user
/// profile or writing that snapshot (which may contain secrets) to disk.
#[cfg(test)]
pub(crate) fn replace_settings_in_memory_for_test(settings: AppSettings) {
    let mut guard = settings_store()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = settings;
}

/// 从文件重新加载设置到内存缓存
/// 用于导入配置等场景，确保内存缓存与文件同步
pub fn reload_settings() -> Result<(), AppError> {
    let fresh_settings = AppSettings::load_from_file();
    let mut guard = settings_store().write().unwrap_or_else(|e| {
        log::warn!("设置锁已毒化，使用恢复值: {e}");
        e.into_inner()
    });
    *guard = fresh_settings;
    Ok(())
}

pub fn get_claude_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .claude_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_codex_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .codex_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_gemini_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .gemini_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_grok_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .grok_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_opencode_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .opencode_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_openclaw_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .openclaw_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

pub fn get_hermes_override_dir() -> Option<PathBuf> {
    let settings = settings_store().read().ok()?;
    settings
        .hermes_config_dir
        .as_ref()
        .map(|p| resolve_override_path(p))
}

#[allow(dead_code)]
pub fn preserve_codex_official_auth_on_switch() -> bool {
    let _ = settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .preserve_codex_official_auth_on_switch;
    true
}

pub fn unify_codex_session_history() -> bool {
    settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .unify_codex_session_history
}

// ===== 当前供应商管理函数 =====

/// 获取指定应用类型的当前供应商 ID（从本地 settings 读取）
///
/// 这是设备级别的设置，不随数据库同步。
/// 如果本地没有设置，调用者应该 fallback 到数据库的 `is_current` 字段。
pub fn get_current_provider(app_type: &AppType) -> Option<String> {
    let settings = settings_store().read().ok()?;
    match app_type {
        AppType::Claude => settings.current_provider_claude.clone(),
        AppType::ClaudeDesktop => settings.current_provider_claude_desktop.clone(),
        AppType::Codex => settings.current_provider_codex.clone(),
        AppType::Gemini => settings.current_provider_gemini.clone(),
        AppType::GrokBuild => settings.current_provider_grokbuild.clone(),
        AppType::OpenCode => settings.current_provider_opencode.clone(),
        AppType::OpenClaw => settings.current_provider_openclaw.clone(),
        AppType::Hermes => settings.current_provider_hermes.clone(),
    }
}

/// 设置指定应用类型的当前供应商 ID（保存到本地 settings）
///
/// 这是设备级别的设置，不随数据库同步。
/// 传入 `None` 会清除当前供应商设置。
pub fn set_current_provider(app_type: &AppType, id: Option<&str>) -> Result<(), AppError> {
    let id_owned = id.map(|s| s.to_string());
    mutate_settings(|settings| match app_type {
        AppType::Claude => settings.current_provider_claude = id_owned.clone(),
        AppType::ClaudeDesktop => settings.current_provider_claude_desktop = id_owned.clone(),
        AppType::Codex => settings.current_provider_codex = id_owned.clone(),
        AppType::Gemini => settings.current_provider_gemini = id_owned.clone(),
        AppType::GrokBuild => settings.current_provider_grokbuild = id_owned.clone(),
        AppType::OpenCode => settings.current_provider_opencode = id_owned.clone(),
        AppType::OpenClaw => settings.current_provider_openclaw = id_owned.clone(),
        AppType::Hermes => settings.current_provider_hermes = id_owned.clone(),
    })
}

/// 获取有效的当前供应商 ID（验证存在性）
///
/// 逻辑：
/// 1. 从本地 settings 读取当前供应商 ID
/// 2. 验证该 ID 在数据库中存在
/// 3. 如果不存在则清理本地 settings，fallback 到数据库的 is_current
///
/// 这确保了返回的 ID 一定是有效的（在数据库中存在）。
/// 多设备云同步场景下，配置导入后本地 ID 可能失效，此函数会自动修复。
pub fn get_effective_current_provider(
    db: &crate::database::Database,
    app_type: &AppType,
) -> Result<Option<String>, AppError> {
    // 1. 从本地 settings 读取
    if let Some(local_id) = get_current_provider(app_type) {
        // 2. 验证该 ID 在数据库中存在
        if db
            .get_provider_by_id(&local_id, app_type.as_str())?
            .is_some()
        {
            // 存在，直接返回
            return Ok(Some(local_id));
        }

        // 3. 不存在，清理本地 settings
        log::warn!(
            "本地 settings 中的供应商 {} ({}) 在数据库中不存在，将清理并 fallback 到数据库",
            local_id,
            app_type.as_str()
        );
        let _ = set_current_provider(app_type, None);
    }

    // Fallback 到数据库的 is_current
    db.get_current_provider(app_type.as_str())
}

// ===== Skill 同步方式管理函数 =====

/// 获取 Skill 同步方式配置
pub fn get_skill_sync_method() -> SyncMethod {
    settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .skill_sync_method
}

// ===== Skill 存储位置管理函数 =====

/// 获取 Skill 存储位置配置
pub fn get_skill_storage_location() -> SkillStorageLocation {
    settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .skill_storage_location
}

/// 设置 Skill 存储位置
pub fn set_skill_storage_location(location: SkillStorageLocation) -> Result<(), AppError> {
    mutate_settings(|s| {
        s.skill_storage_location = location;
    })
}

// ===== 备份策略管理函数 =====

/// Get the effective auto-backup interval in hours (default 24)
pub fn effective_backup_interval_hours() -> u32 {
    settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .backup_interval_hours
        .unwrap_or(24)
}

/// Get the effective backup retain count (default 10, minimum 1)
pub fn effective_backup_retain_count() -> usize {
    settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .backup_retain_count
        .map(|n| (n as usize).max(1))
        .unwrap_or(10)
}

// ===== 终端设置管理函数 =====

/// 获取当前主机可用的首选终端；未显式设置时返回平台默认值。
pub fn get_preferred_terminal() -> Option<String> {
    let preferred_terminal = settings_store()
        .read()
        .unwrap_or_else(|e| {
            log::warn!("设置锁已毒化，使用恢复值: {e}");
            e.into_inner()
        })
        .preferred_terminal
        .clone();
    effective_preferred_terminal(preferred_terminal.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_config::AppType;

    #[test]
    fn retired_cloud_configuration_roundtrips_without_interpretation() {
        let legacy = serde_json::json!({
            "webdavSync": {"enabled": true, "autoSync": true, "baseUrl": "  untouched  ", "password": "synthetic", "unknown": [1, false]},
            "s3Sync": {"enabled": true, "autoSync": true, "bucket": "  untouched  ", "secretAccessKey": "synthetic", "unknown": {"nested": 1}},
            "webdavBackup": {"unknown": "unchanged"}
        });
        let parsed = AppSettings::from_json(&legacy.to_string()).expect("legacy settings");
        let serialized = serde_json::to_value(parsed).expect("serialize settings");
        for key in ["webdavSync", "s3Sync", "webdavBackup"] {
            assert_eq!(serialized[key], legacy[key], "preserve opaque {key}");
        }
    }

    #[test]
    fn retired_cloud_null_and_non_object_values_are_preserved() {
        for legacy in [
            serde_json::Value::Null,
            serde_json::json!([1, "old"]),
            serde_json::json!(false),
        ] {
            let value =
                serde_json::json!({"webdavSync": legacy, "s3Sync": legacy, "webdavBackup": legacy});
            let parsed = AppSettings::from_json(&value.to_string()).expect("opaque settings");
            let serialized = serde_json::to_value(parsed).expect("serialize settings");
            for key in ["webdavSync", "s3Sync", "webdavBackup"] {
                assert!(serialized.get(key).is_some());
                assert_eq!(serialized[key], value[key]);
            }
        }
    }

    #[test]
    fn retired_window_control_setting_is_ignored_and_not_serialized() {
        let retired_key = ["useApp", "WindowControls"].concat();
        let mut value = serde_json::to_value(AppSettings::default()).expect("default settings");
        value
            .as_object_mut()
            .expect("settings object")
            .insert(retired_key.clone(), serde_json::Value::Bool(true));

        let settings: AppSettings = serde_json::from_value(value).expect("legacy settings");
        let serialized = serde_json::to_value(settings).expect("serialized settings");

        assert!(!serialized
            .as_object()
            .expect("settings object")
            .contains_key(&retired_key));
    }

    #[test]
    fn preferred_terminal_allowlist_keeps_only_current_host_values() {
        let Some(configuration) = preferred_terminal_configuration() else {
            assert_eq!(normalize_preferred_terminal(Some("terminal")), None);
            assert_eq!(effective_preferred_terminal(None), None);
            return;
        };

        for supported in configuration.supported {
            let padded = format!("  {supported}  ");
            assert_eq!(
                normalize_preferred_terminal(Some(&padded)).as_deref(),
                Some(*supported)
            );
        }

        assert_eq!(normalize_preferred_terminal(None), None);
        assert_eq!(
            effective_preferred_terminal(None).as_deref(),
            Some(configuration.default)
        );
    }

    #[test]
    fn persisted_retired_terminal_normalizes_to_supported_host_default() {
        let Some(configuration) = preferred_terminal_configuration() else {
            return;
        };
        let retired_terminal = ["retired", "-terminal"].concat();
        let mut value = serde_json::to_value(AppSettings::default()).expect("default settings");
        value.as_object_mut().expect("settings object").insert(
            "preferredTerminal".to_string(),
            serde_json::Value::String(retired_terminal.clone()),
        );

        let settings = AppSettings::from_json(&value.to_string()).expect("legacy settings");
        let effective = effective_preferred_terminal(settings.preferred_terminal.as_deref())
            .expect("supported host terminal");
        let serialized = serde_json::to_value(&settings).expect("normalized settings");

        assert_eq!(
            settings.preferred_terminal.as_deref(),
            Some(configuration.default)
        );
        assert_eq!(effective, configuration.default);
        assert_eq!(
            serialized
                .get("preferredTerminal")
                .and_then(serde_json::Value::as_str),
            Some(configuration.default)
        );
        assert_ne!(effective, retired_terminal);

        if cfg!(target_os = "macos") {
            assert!(crate::session_manager::terminal::is_supported_terminal_target(&effective));
        }
    }

    #[test]
    fn persisted_retired_macos_terminal_becomes_a_supported_resume_target() {
        let retired_terminal = ["retired", "-terminal"].concat();
        let mut value = serde_json::to_value(AppSettings::default()).expect("default settings");
        value.as_object_mut().expect("settings object").insert(
            "preferredTerminal".to_string(),
            serde_json::Value::String(retired_terminal),
        );
        let settings: AppSettings = serde_json::from_value(value).expect("legacy settings");

        let effective = normalize_preferred_terminal_for_configuration(
            settings.preferred_terminal.as_deref(),
            MACOS_PREFERRED_TERMINAL_CONFIGURATION,
        )
        .expect("macOS terminal target");

        assert_eq!(effective, "terminal");
        assert!(crate::session_manager::terminal::is_supported_terminal_target(&effective));
    }

    #[test]
    fn visible_apps_old_settings_default_claude_desktop_visible() {
        let visible: VisibleApps = serde_json::from_value(serde_json::json!({
            "claude": true,
            "codex": true,
            "gemini": true,
            "opencode": true,
            "openclaw": true,
            "hermes": true
        }))
        .expect("visible apps");

        assert!(visible.is_visible(&AppType::ClaudeDesktop));
        assert!(visible.workbuddy);
    }

    #[test]
    fn visible_apps_accepts_claude_desktop_aliases() {
        let visible: VisibleApps = serde_json::from_value(serde_json::json!({
            "claude": true,
            "claudeDesktop": false,
            "codex": true,
            "gemini": true,
            "opencode": true,
            "openclaw": true,
            "hermes": true
        }))
        .expect("visible apps");

        assert!(!visible.is_visible(&AppType::ClaudeDesktop));
    }

    #[test]
    fn appearance_theme_accepts_only_the_closed_preference_set() {
        assert_eq!(parse_appearance_theme(" dark "), Some("dark"));
        assert_eq!(parse_appearance_theme("light"), Some("light"));
        assert_eq!(parse_appearance_theme("system"), Some("system"));
        assert_eq!(parse_appearance_theme("not-a-theme"), None);
        assert_eq!(parse_appearance_theme(""), None);

        let mut settings = AppSettings {
            appearance_theme: Some("  DARK  ".to_string()),
            ..AppSettings::default()
        };
        settings.normalize_paths();
        assert_eq!(settings.appearance_theme.as_deref(), None);

        settings.appearance_theme = Some("dark".to_string());
        settings.normalize_paths();
        assert_eq!(settings.appearance_theme.as_deref(), Some("dark"));

        let serialized = serde_json::to_value(AppSettings::default()).expect("default settings");
        assert!(!serialized
            .as_object()
            .expect("settings object")
            .contains_key("appearanceTheme"));
    }

    #[test]
    fn appearance_bootstrap_script_seeds_only_a_closed_preference() {
        assert!(appearance_bootstrap_script_for(None).is_none());
        assert!(appearance_bootstrap_script_for(Some("nope")).is_none());

        let script = appearance_bootstrap_script_for(Some("dark")).expect("dark bootstrap");
        assert!(script.contains("fyagent-theme"));
        assert!(script.contains("\"dark\""));
        assert!(script.contains("data-theme"));
        assert!(!script.contains("password"));
        assert!(!script.contains("secret"));
    }
}
