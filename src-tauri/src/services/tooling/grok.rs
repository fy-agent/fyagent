//! macOS Grok Build distribution-owner lifecycle.
//!
//! Native/internal and official npm stay two official owners. macOS never
//! composes `grok update || installer || npm` as one job. Windows command
//! composition is owned by `lifecycle.rs` and is not changed here.

#[cfg(all(target_os = "windows", not(test)))]
use super::ToolLifecycleAction;
#[cfg(any(target_os = "macos", test))]
use super::*;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(any(target_os = "macos", test))]
pub(super) const GROK_DISTRIBUTION_NATIVE: &str = "native_internal";
#[cfg(any(target_os = "macos", test))]
pub(super) const GROK_DISTRIBUTION_NPM: &str = "official_npm";

#[cfg(target_os = "macos")]
const GROK_INSTALL_SCRIPT_URL: &str = "https://x.ai/cli/install.sh";
#[cfg(target_os = "macos")]
const GROK_INSTALL_SCRIPT_MAX_BYTES: usize = 1024 * 1024;
#[cfg(target_os = "macos")]
const GROK_INSTALL_SCRIPT_TIMEOUT: Duration = Duration::from_secs(60);
#[cfg(target_os = "macos")]
const GROK_CHECK_TIMEOUT: Duration = Duration::from_secs(20);
#[cfg(target_os = "macos")]
const GROK_UPDATE_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(target_os = "macos")]
const GROK_INSTALLER_TIMEOUT: Duration = Duration::from_secs(300);
#[cfg(target_os = "macos")]
const GROK_OUTPUT_LIMIT: usize = 32 * 1024;
#[cfg(any(target_os = "macos", test))]
const GROK_LOG_LINES: usize = 12;

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum GrokDistributionOwner {
    NativeInternal,
    OfficialNpm,
}

#[cfg(any(target_os = "macos", test))]
impl GrokDistributionOwner {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::NativeInternal => GROK_DISTRIBUTION_NATIVE,
            Self::OfficialNpm => GROK_DISTRIBUTION_NPM,
        }
    }
}

#[cfg(any(target_os = "macos", test))]
impl From<GrokDistributionOwner> for fyagent_user_helper::GrokOwner {
    fn from(owner: GrokDistributionOwner) -> Self {
        match owner {
            GrokDistributionOwner::NativeInternal => Self::Native,
            GrokDistributionOwner::OfficialNpm => Self::Npm,
        }
    }
}

#[cfg(any(target_os = "macos", test))]
impl From<fyagent_user_helper::GrokOwner> for GrokDistributionOwner {
    fn from(owner: fyagent_user_helper::GrokOwner) -> Self {
        match owner {
            fyagent_user_helper::GrokOwner::Native => Self::NativeInternal,
            fyagent_user_helper::GrokOwner::Npm => Self::OfficialNpm,
        }
    }
}

#[cfg(any(target_os = "macos", test))]
impl From<GrokOwnerObservation> for fyagent_user_helper::GrokOwnerObservation {
    fn from(observation: GrokOwnerObservation) -> Self {
        match observation {
            GrokOwnerObservation::NativeInternal => Self::Native,
            GrokOwnerObservation::OfficialNpm => Self::Npm,
            GrokOwnerObservation::Ambiguous => Self::Ambiguous,
            GrokOwnerObservation::Absent => Self::Absent,
        }
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrokOwnerObservation {
    NativeInternal,
    OfficialNpm,
    Ambiguous,
    Absent,
}

#[cfg(any(target_os = "macos", test))]
impl GrokOwnerObservation {
    fn owner(self) -> Option<GrokDistributionOwner> {
        match self {
            Self::NativeInternal => Some(GrokDistributionOwner::NativeInternal),
            Self::OfficialNpm => Some(GrokDistributionOwner::OfficialNpm),
            Self::Ambiguous | Self::Absent => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn wire(self) -> Option<&'static str> {
        self.owner().map(GrokDistributionOwner::as_str)
    }
}

#[cfg(target_os = "macos")]
pub(super) fn owner_observation_wire(observation: GrokOwnerObservation) -> Option<&'static str> {
    observation.wire()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrokLifecycleStage {
    #[cfg(target_os = "macos")]
    Checking,
    #[cfg(target_os = "macos")]
    Preflight,
    #[cfg(target_os = "macos")]
    Executing,
    #[cfg(target_os = "macos")]
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
}

impl GrokLifecycleStage {
    #[cfg(any(target_os = "macos", test))]
    fn as_str(self) -> &'static str {
        match self {
            #[cfg(target_os = "macos")]
            Self::Checking => "checking",
            #[cfg(target_os = "macos")]
            Self::Preflight => "preflight",
            #[cfg(target_os = "macos")]
            Self::Executing => "executing",
            #[cfg(target_os = "macos")]
            Self::Verifying => "verifying",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct GrokLifecycleSnapshot {
    stage: String,
    action: String,
    owner: Option<String>,
    reason: Option<String>,
    redacted_log: String,
    exit_code: Option<i32>,
    timed_out: bool,
    source_category: Option<String>,
}

impl GrokLifecycleSnapshot {
    pub fn stage(&self) -> &str {
        &self.stage
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn redacted_log(&self) -> &str {
        &self.redacted_log
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }
}

fn grok_terminal_stage(stage: &str) -> Option<GrokLifecycleStage> {
    match stage {
        "failed" => Some(GrokLifecycleStage::Failed),
        "cancelled" => Some(GrokLifecycleStage::Cancelled),
        "succeeded" => Some(GrokLifecycleStage::Succeeded),
        _ => None,
    }
}

pub(super) fn last_grok_lifecycle_error() -> Option<String> {
    let snapshot = last_grok_lifecycle_snapshot()?;
    let stage = grok_terminal_stage(snapshot.stage())?;
    if !stage.is_terminal() || matches!(stage, GrokLifecycleStage::Succeeded) {
        return None;
    }
    let copy = grok_fail_copy(snapshot.reason().unwrap_or("cancelled"));
    let log = snapshot.redacted_log();
    Some(if log.is_empty() {
        format!("Grok Build 未完成。{copy}")
    } else {
        format!("Grok Build 未完成。{copy}\n{log}")
    })
}

static LAST_GROK_JOB: Mutex<Option<GrokLifecycleSnapshot>> = Mutex::new(None);

pub(crate) fn last_grok_lifecycle_snapshot() -> Option<GrokLifecycleSnapshot> {
    LAST_GROK_JOB.lock().ok().and_then(|guard| guard.clone())
}

#[cfg(any(target_os = "macos", test))]
fn store_grok_snapshot(snapshot: GrokLifecycleSnapshot) {
    if let Ok(mut guard) = LAST_GROK_JOB.lock() {
        *guard = Some(snapshot);
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GrokPlan {
    NativeFresh,
    NativeUpdate { bin_path: String },
    OfficialNpm { bin_path: Option<String> },
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrokPlanError {
    reason: &'static str,
    owner: Option<GrokDistributionOwner>,
    detail: String,
}

#[cfg(any(target_os = "macos", test))]
impl GrokPlanError {
    fn new(
        reason: &'static str,
        owner: Option<GrokDistributionOwner>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            reason,
            owner,
            detail: detail.into(),
        }
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GrokCheckResult {
    AlreadyCurrent { version: String },
    UpdateAvailable { version: String },
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn parse_grok_cli_installer(config_toml: &str) -> Option<GrokDistributionOwner> {
    let document: toml::Value = config_toml.parse().ok()?;
    let installer = document.get("cli")?.get("installer")?.as_str()?;
    fyagent_user_helper::grok::owner_from_installer_value(installer).map(Into::into)
}

#[cfg(any(target_os = "macos", test))]
fn owner_from_install(
    bin_path: &str,
    real_target: &str,
    install_source: &str,
    config_owner: Option<GrokDistributionOwner>,
) -> GrokDistributionOwner {
    fyagent_user_helper::grok::owner_from_install_paths(
        bin_path,
        real_target,
        install_source,
        config_owner.map(Into::into),
    )
    .into()
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn observe_grok_owner(
    installs: &[ToolInstallation],
    config_toml: Option<&str>,
) -> GrokOwnerObservation {
    if installs.is_empty() {
        return GrokOwnerObservation::Absent;
    }
    let config_owner = config_toml.and_then(parse_grok_cli_installer);
    let mut owners = std::collections::BTreeSet::new();
    for install in installs {
        owners.insert(owner_from_install(
            &install.path,
            &install.real.to_string_lossy(),
            &install.source,
            config_owner,
        ));
    }
    if owners.len() > 1 {
        return GrokOwnerObservation::Ambiguous;
    }
    match owners.iter().next() {
        Some(GrokDistributionOwner::NativeInternal) => GrokOwnerObservation::NativeInternal,
        Some(GrokDistributionOwner::OfficialNpm) => GrokOwnerObservation::OfficialNpm,
        None => GrokOwnerObservation::Absent,
    }
}

#[cfg(target_os = "macos")]
pub(super) fn grok_owner_wire_label(
    installs: &[ToolInstallation],
    config_toml: Option<&str>,
) -> Option<&'static str> {
    observe_grok_owner(installs, config_toml).wire()
}

#[cfg(target_os = "macos")]
pub(super) fn grok_owner_wire_from_disk(installs: &[ToolInstallation]) -> Option<&'static str> {
    grok_owner_wire_label(installs, read_grok_config_toml().as_deref())
}

#[cfg(target_os = "macos")]
fn read_grok_config_toml() -> Option<String> {
    let path = crate::grok_config::get_grok_config_path();
    let bytes = std::fs::read(&path).ok()?;
    if bytes.len() > 256 * 1024 {
        return None;
    }
    String::from_utf8(bytes).ok()
}

#[cfg(target_os = "macos")]
pub(super) fn observe_installed_grok_owner() -> GrokOwnerObservation {
    let installs = enumerate_tool_installations("grok");
    observe_grok_owner(&installs, read_grok_config_toml().as_deref())
}

#[cfg(target_os = "macos")]
pub(super) fn native_latest_from_update_check(local: Option<&str>) -> Option<String> {
    let installs = enumerate_tool_installations("grok");
    let bin_path = default_install(&installs)?.path.clone();
    let output = run_anchored_grok(&bin_path, &["update", "--check"], GROK_CHECK_TIMEOUT).ok()?;
    if !output.status.success() {
        return None;
    }
    let combined = format!(
        "{}\n{}",
        decode_command_output(&output.stdout),
        decode_command_output(&output.stderr)
    );
    match parse_grok_update_check(&combined, local) {
        Ok(GrokCheckResult::AlreadyCurrent { version })
        | Ok(GrokCheckResult::UpdateAvailable { version }) => Some(version),
        Err(_) => None,
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn grok_plan_from_installs(
    action: ToolLifecycleAction,
    installs: &[ToolInstallation],
    config_toml: Option<&str>,
) -> Result<GrokPlan, GrokPlanError> {
    let observation = observe_grok_owner(installs, config_toml);
    let (tool_action, expected_owner) = match action {
        ToolLifecycleAction::Install => (fyagent_user_helper::GrokToolAction::Install, None),
        ToolLifecycleAction::InstallOfficialNpm => (
            fyagent_user_helper::GrokToolAction::Install,
            Some(fyagent_user_helper::GrokOwner::Npm),
        ),
        ToolLifecycleAction::Update => (fyagent_user_helper::GrokToolAction::Update, None),
    };
    match fyagent_user_helper::grok::plan_grok_operation(
        tool_action,
        observation.into(),
        expected_owner,
    ) {
        Ok(fyagent_user_helper::GrokPlanKind::NativeFresh) => Ok(GrokPlan::NativeFresh),
        Ok(fyagent_user_helper::GrokPlanKind::NativeUpdate) => {
            let install = default_install(installs).ok_or_else(|| {
                GrokPlanError::new(
                    "distribution_owner_mismatch",
                    Some(GrokDistributionOwner::NativeInternal),
                    "未找到可锚定的 Grok Build 安装",
                )
            })?;
            Ok(GrokPlan::NativeUpdate {
                bin_path: install.path.clone(),
            })
        }
        Ok(fyagent_user_helper::GrokPlanKind::OfficialNpm) => Ok(GrokPlan::OfficialNpm {
            bin_path: default_install(installs).map(|install| install.path.clone()),
        }),
        Ok(fyagent_user_helper::GrokPlanKind::Observe) => Err(GrokPlanError::new(
            "distribution_owner_mismatch",
            None,
            "Grok Build 安装来源不一致，需要先选择目标",
        )),
        Err(fyagent_user_helper::GrokPlanFailure::OwnerMismatch) => Err(GrokPlanError::new(
            "distribution_owner_mismatch",
            observation.owner(),
            match observation {
                GrokOwnerObservation::OfficialNpm => "当前是官方 npm 安装，不会改用官方命令行安装",
                GrokOwnerObservation::Ambiguous => "Grok Build 安装来源不一致，需要先选择目标",
                _ => "Grok Build 安装来源不一致，不能自动选择更新方式",
            },
        )),
        Err(fyagent_user_helper::GrokPlanFailure::NotDetected) => Err(GrokPlanError::new(
            "post_install_not_observed",
            None,
            "未发现可更新的 Grok Build 安装",
        )),
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn parse_grok_update_check(
    output: &str,
    local_version: Option<&str>,
) -> Result<GrokCheckResult, &'static str> {
    let text = output.trim();
    if text.is_empty() {
        return Err("official_source_unreachable");
    }
    let lower = text.to_ascii_lowercase();
    let extracted = extract_version(text);
    let parsed = super::versions::compare_semver(&extracted, &extracted)
        .is_some()
        .then_some(extracted.as_str());
    let current_markers = [
        "up to date",
        "already current",
        "already up-to-date",
        "no update",
        "is current",
    ];
    let looks_current = current_markers.iter().any(|marker| lower.contains(marker));

    if let Some(version) = parsed {
        if looks_current || local_version.is_some_and(|local| local == version) {
            return Ok(GrokCheckResult::AlreadyCurrent {
                version: version.to_string(),
            });
        }
        return Ok(GrokCheckResult::UpdateAvailable {
            version: version.to_string(),
        });
    }

    if looks_current {
        if let Some(local) = local_version {
            return Ok(GrokCheckResult::AlreadyCurrent {
                version: local.to_string(),
            });
        }
    }

    Err("official_source_unreachable")
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn grok_installer_url_is_allowed(url: &url::Url) -> bool {
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return false;
    }
    match url.host_str() {
        Some("x.ai") => true,
        Some(host) if host.ends_with(".x.ai") && host.len() > ".x.ai".len() => true,
        _ => false,
    }
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn redact_grok_lifecycle_text(raw: &str) -> String {
    let home = crate::config::get_home_dir();
    let temp = crate::config::get_user_temp_dir();
    let mut text = raw.replace('\0', "");
    let home_display = home.to_string_lossy();
    if !home_display.is_empty() {
        text = text.replace(home_display.as_ref(), "~");
    }
    let temp_display = temp.to_string_lossy();
    if !temp_display.is_empty() {
        text = text.replace(temp_display.as_ref(), "<temp>");
    }
    text = redact_url_queries(&text);
    text = redact_secret_tokens(&text);
    last_lines(text.trim(), GROK_LOG_LINES)
}

#[cfg(any(target_os = "macos", test))]
fn redact_url_queries(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, part) in text.split("https://").enumerate() {
        if index > 0 {
            out.push_str("https://");
        }
        if let Some((head, rest)) = part.split_once('?') {
            out.push_str(head);
            out.push_str("?<redacted>");
            if let Some(tail) = rest.find(|c: char| c.is_whitespace()) {
                out.push_str(&rest[tail..]);
            }
        } else {
            out.push_str(part);
        }
    }
    out
}

#[cfg(any(target_os = "macos", test))]
fn redact_secret_tokens(text: &str) -> String {
    let mut out = text.to_string();
    for prefix in ["npm_", "ghp_", "gho_", "github_pat_"] {
        while let Some(start) = out.find(prefix) {
            let rest = &out[start + prefix.len()..];
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(rest.len());
            let replace_to = start + prefix.len() + end;
            out.replace_range(start..replace_to, "<redacted>");
        }
    }
    out
}

#[cfg(any(target_os = "macos", test))]
fn action_wire(action: ToolLifecycleAction) -> String {
    match action {
        ToolLifecycleAction::Install => "install".to_string(),
        ToolLifecycleAction::Update => "update".to_string(),
        ToolLifecycleAction::InstallOfficialNpm => "install_official_npm".to_string(),
    }
}

fn grok_fail_copy(reason: &str) -> &'static str {
    match reason {
        "source_exhausted" => "官方来源都不可用。",
        "official_source_unreachable" => "暂时无法访问官方来源。",
        "distribution_owner_mismatch" => "当前安装方式与所选操作不一致。",
        "post_install_not_observed" => "安装后未发现可用的 Grok Build。",
        "external_installer_failed" => "官方安装未完成。",
        "external_installer_timed_out" => "官方安装超时。",
        "cancelled" => "操作已取消。",
        _ => "操作未完成。",
    }
}

#[cfg(target_os = "macos")]
fn store_stage(
    stage: GrokLifecycleStage,
    action: ToolLifecycleAction,
    owner: Option<GrokDistributionOwner>,
    source_category: Option<&str>,
) {
    store_grok_snapshot(GrokLifecycleSnapshot {
        stage: stage.as_str().to_string(),
        action: action_wire(action),
        owner: owner.map(GrokDistributionOwner::as_str).map(str::to_string),
        reason: None,
        redacted_log: String::new(),
        exit_code: None,
        timed_out: false,
        source_category: source_category.map(str::to_string),
    });
}

#[cfg(any(target_os = "macos", test))]
fn fail_job(
    action: ToolLifecycleAction,
    owner: Option<GrokDistributionOwner>,
    reason: &str,
    detail: &str,
    exit_code: Option<i32>,
    timed_out: bool,
    source_category: Option<&str>,
) -> String {
    let redacted = redact_grok_lifecycle_text(detail);
    store_grok_snapshot(GrokLifecycleSnapshot {
        stage: GrokLifecycleStage::Failed.as_str().to_string(),
        action: action_wire(action),
        owner: owner.map(GrokDistributionOwner::as_str).map(str::to_string),
        reason: Some(reason.to_string()),
        redacted_log: redacted.clone(),
        exit_code,
        timed_out,
        source_category: source_category.map(str::to_string),
    });
    let copy = grok_fail_copy(reason);
    if redacted.is_empty() {
        format!("Grok Build 未完成。{copy}")
    } else {
        format!("Grok Build 未完成。{copy}\n{redacted}")
    }
}

#[cfg(any(target_os = "macos", test))]
fn succeed_job(
    action: ToolLifecycleAction,
    owner: GrokDistributionOwner,
    detail: &str,
    source_category: Option<&str>,
) {
    store_grok_snapshot(GrokLifecycleSnapshot {
        stage: GrokLifecycleStage::Succeeded.as_str().to_string(),
        action: action_wire(action),
        owner: Some(owner.as_str().to_string()),
        reason: None,
        redacted_log: redact_grok_lifecycle_text(detail),
        exit_code: Some(0),
        timed_out: false,
        source_category: source_category.map(str::to_string),
    });
}

#[cfg(target_os = "macos")]
pub(super) async fn run_macos_grok_lifecycle(action: ToolLifecycleAction) -> Result<(), String> {
    store_stage(GrokLifecycleStage::Checking, action, None, None);

    let prepared = tokio::task::spawn_blocking(move || {
        let installs = enumerate_tool_installations("grok");
        let config = read_grok_config_toml();
        grok_plan_from_installs(action, &installs, config.as_deref())
            .map(|plan| (plan, observe_grok_owner(&installs, config.as_deref())))
    })
    .await
    .map_err(|error| format!("tool lifecycle task join error: {error}"))?;

    let (plan, observation) = match prepared {
        Ok(value) => value,
        Err(error) => {
            return Err(fail_job(
                action,
                error.owner,
                error.reason,
                &error.detail,
                None,
                false,
                None,
            ));
        }
    };

    store_stage(
        GrokLifecycleStage::Preflight,
        action,
        observation.owner(),
        None,
    );

    match plan {
        GrokPlan::NativeFresh => run_native_fresh_install(action).await,
        GrokPlan::NativeUpdate { bin_path } => run_native_update(action, bin_path).await,
        GrokPlan::OfficialNpm { bin_path } => run_official_npm(action, bin_path).await,
    }
}

#[cfg(target_os = "macos")]
async fn run_native_fresh_install(action: ToolLifecycleAction) -> Result<(), String> {
    let script = match fetch_official_installer_script().await {
        Ok(bytes) => bytes,
        Err(reason) => {
            return Err(fail_job(
                action,
                Some(GrokDistributionOwner::NativeInternal),
                reason,
                "官方安装脚本不可用。可以使用独立的官方 npm 安装动作，而不是自动切换。",
                None,
                false,
                Some("official_primary"),
            ));
        }
    };

    tokio::task::spawn_blocking(move || execute_native_fresh(action, script))
        .await
        .map_err(|error| format!("tool lifecycle task join error: {error}"))?
}

#[cfg(target_os = "macos")]
async fn fetch_official_installer_script() -> Result<Vec<u8>, &'static str> {
    let client = crate::proxy::http_client::get();
    let request = client
        .get(GROK_INSTALL_SCRIPT_URL)
        .timeout(GROK_INSTALL_SCRIPT_TIMEOUT);
    let response = request
        .send()
        .await
        .map_err(|_| "official_source_unreachable")?;
    if !response.status().is_success() {
        return Err("official_source_unreachable");
    }
    if !grok_installer_url_is_allowed(response.url()) {
        return Err("official_source_unreachable");
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|_| "official_source_unreachable")?;
    if bytes.is_empty() || bytes.len() > GROK_INSTALL_SCRIPT_MAX_BYTES {
        return Err("official_source_unreachable");
    }
    if !bytes.starts_with(b"#!") {
        return Err("official_source_unreachable");
    }
    Ok(bytes.to_vec())
}

#[cfg(target_os = "macos")]
fn execute_native_fresh(action: ToolLifecycleAction, script: Vec<u8>) -> Result<(), String> {
    store_stage(
        GrokLifecycleStage::Executing,
        action,
        Some(GrokDistributionOwner::NativeInternal),
        Some("official_primary"),
    );

    let path = match write_persisted_temp_file("fyagent_grok_install_", ".sh", &script) {
        Ok(path) => path,
        Err(error) => {
            return Err(fail_job(
                action,
                Some(GrokDistributionOwner::NativeInternal),
                "external_installer_failed",
                &error,
                None,
                false,
                Some("official_primary"),
            ));
        }
    };

    let output = run_bash_script(&path, GROK_INSTALLER_TIMEOUT);
    let _ = std::fs::remove_file(&path);
    finish_native_command(
        action,
        GrokDistributionOwner::NativeInternal,
        output,
        Some("official_primary"),
    )
}

#[cfg(target_os = "macos")]
async fn run_native_update(action: ToolLifecycleAction, bin_path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || execute_native_update(action, bin_path))
        .await
        .map_err(|error| format!("tool lifecycle task join error: {error}"))?
}

#[cfg(target_os = "macos")]
fn execute_native_update(action: ToolLifecycleAction, bin_path: String) -> Result<(), String> {
    let before = grok_post_observe();
    let local = before
        .as_ref()
        .ok()
        .and_then(|observed| observed.version.clone());

    store_stage(
        GrokLifecycleStage::Executing,
        action,
        Some(GrokDistributionOwner::NativeInternal),
        Some("native_updater"),
    );

    let check = run_anchored_grok(&bin_path, &["update", "--check"], GROK_CHECK_TIMEOUT);
    let check_output = match check {
        Ok(output) => output,
        Err(error) => {
            let timed_out = error.contains("timed out");
            return Err(fail_job(
                action,
                Some(GrokDistributionOwner::NativeInternal),
                if timed_out {
                    "external_installer_timed_out"
                } else {
                    "official_source_unreachable"
                },
                &error,
                None,
                timed_out,
                Some("native_updater"),
            ));
        }
    };

    if !check_output.status.success() {
        return finish_native_command(
            action,
            GrokDistributionOwner::NativeInternal,
            Ok(check_output),
            Some("native_updater"),
        );
    }

    let combined = format!(
        "{}\n{}",
        decode_command_output(&check_output.stdout),
        decode_command_output(&check_output.stderr)
    );
    let parsed = parse_grok_update_check(&combined, local.as_deref());
    match parsed {
        Ok(GrokCheckResult::AlreadyCurrent { version }) => verify_after_action(
            action,
            GrokDistributionOwner::NativeInternal,
            Some(&format!("already current {version}")),
            Some("native_updater"),
            before.ok(),
        ),
        Ok(GrokCheckResult::UpdateAvailable { version }) => {
            if !version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            {
                return Err(fail_job(
                    action,
                    Some(GrokDistributionOwner::NativeInternal),
                    "official_source_unreachable",
                    "官方检查返回了无法使用的版本号",
                    None,
                    false,
                    Some("native_updater"),
                ));
            }
            let update = run_anchored_grok(
                &bin_path,
                &["update", "--version", &version],
                GROK_UPDATE_TIMEOUT,
            );
            finish_native_command(
                action,
                GrokDistributionOwner::NativeInternal,
                update,
                Some("native_updater"),
            )
        }
        Err(reason) => Err(fail_job(
            action,
            Some(GrokDistributionOwner::NativeInternal),
            reason,
            &redact_grok_lifecycle_text(&combined),
            check_output.status.code(),
            false,
            Some("native_updater"),
        )),
    }
}

#[cfg(target_os = "macos")]
async fn run_official_npm(
    action: ToolLifecycleAction,
    bin_path: Option<String>,
) -> Result<(), String> {
    let client = crate::proxy::http_client::get();
    let frozen = super::versions::fetch_npm_latest_for_package(&client, "@xai-official/grok").await;
    let Some(frozen) = frozen else {
        return Err(fail_job(
            action,
            Some(GrokDistributionOwner::OfficialNpm),
            "official_source_unreachable",
            "官方 npm 包版本不可用",
            None,
            false,
            Some("official_npm"),
        ));
    };
    if !frozen
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err(fail_job(
            action,
            Some(GrokDistributionOwner::OfficialNpm),
            "official_source_unreachable",
            "官方 npm 包版本无法使用",
            None,
            false,
            Some("official_npm"),
        ));
    }

    tokio::task::spawn_blocking(move || execute_official_npm(action, bin_path, frozen))
        .await
        .map_err(|error| format!("tool lifecycle task join error: {error}"))?
}

#[cfg(target_os = "macos")]
fn execute_official_npm(
    action: ToolLifecycleAction,
    bin_path: Option<String>,
    frozen: String,
) -> Result<(), String> {
    let before = grok_post_observe().ok();
    store_stage(
        GrokLifecycleStage::Executing,
        action,
        Some(GrokDistributionOwner::OfficialNpm),
        Some("official_npm"),
    );

    let package = format!("@xai-official/grok@{frozen}");
    let command = match bin_path
        .as_deref()
        .and_then(|path| anchored_npm_command(path, &format!("i -g {package}")))
    {
        Some(anchored) => anchored,
        None => format!("npm i -g {package}"),
    };

    let output = run_login_bash(&command, GROK_UPDATE_TIMEOUT);
    let result = finish_native_command(
        action,
        GrokDistributionOwner::OfficialNpm,
        output,
        Some("official_npm"),
    );
    if result.is_err() {
        let _ = before;
    }
    result
}

#[cfg(target_os = "macos")]
struct GrokObservation {
    version: Option<String>,
    owner: GrokOwnerObservation,
    runnable: bool,
}

#[cfg(target_os = "macos")]
fn grok_post_observe() -> Result<GrokObservation, String> {
    let installs = enumerate_tool_installations("grok");
    let owner = observe_grok_owner(&installs, read_grok_config_toml().as_deref());
    let default = default_install(&installs);
    Ok(GrokObservation {
        version: default.and_then(|install| install.version.clone()),
        owner,
        runnable: default.is_some_and(|install| install.runnable),
    })
}

#[cfg(target_os = "macos")]
fn finish_native_command(
    action: ToolLifecycleAction,
    expected_owner: GrokDistributionOwner,
    output: Result<std::process::Output, String>,
    source_category: Option<&str>,
) -> Result<(), String> {
    match output {
        Err(error) => {
            let timed_out = error.contains("timed out");
            Err(fail_job(
                action,
                Some(expected_owner),
                if timed_out {
                    "external_installer_timed_out"
                } else {
                    "external_installer_failed"
                },
                &error,
                None,
                timed_out,
                source_category,
            ))
        }
        Ok(output) => {
            let combined = format!(
                "{}\n{}",
                decode_command_output(&output.stdout),
                decode_command_output(&output.stderr)
            );
            if !output.status.success() {
                return Err(fail_job(
                    action,
                    Some(expected_owner),
                    "external_installer_failed",
                    &combined,
                    output.status.code(),
                    false,
                    source_category,
                ));
            }
            verify_after_action(
                action,
                expected_owner,
                Some(&combined),
                source_category,
                None,
            )
        }
    }
}

#[cfg(target_os = "macos")]
fn verify_after_action(
    action: ToolLifecycleAction,
    expected_owner: GrokDistributionOwner,
    log: Option<&str>,
    source_category: Option<&str>,
    previous: Option<GrokObservation>,
) -> Result<(), String> {
    store_stage(
        GrokLifecycleStage::Verifying,
        action,
        Some(expected_owner),
        source_category,
    );

    let observed = match grok_post_observe() {
        Ok(observed) if observed.runnable && observed.version.is_some() => observed,
        Ok(_) | Err(_) => {
            let preserved = previous
                .as_ref()
                .and_then(|before| before.version.as_deref())
                .unwrap_or("previous");
            return Err(fail_job(
                action,
                Some(expected_owner),
                "post_install_not_observed",
                &format!(
                    "安装后未能回读 grok --version；原版本应保持可用（{preserved}）\n{}",
                    log.unwrap_or("")
                ),
                None,
                false,
                source_category,
            ));
        }
    };

    if observed.owner.owner() != Some(expected_owner)
        && observed.owner != GrokOwnerObservation::Absent
    {
        return Err(fail_job(
            action,
            Some(expected_owner),
            "distribution_owner_mismatch",
            "安装后的分发方式与本次选择不一致",
            None,
            false,
            source_category,
        ));
    }

    succeed_job(
        action,
        expected_owner,
        &format!("version {}", observed.version.unwrap_or_default()),
        source_category,
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn grok_args_are_closed(args: &[&str]) -> bool {
    args.iter().all(|arg| {
        !arg.is_empty()
            && arg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    })
}

#[cfg(target_os = "macos")]
fn validate_anchored_grok_bin(path: &str) -> Result<&Path, String> {
    let path = Path::new(path);
    if !path.is_absolute() || path.as_os_str().is_empty() {
        return Err("未找到可锚定的 Grok Build 安装".to_string());
    }
    if path.file_name().and_then(|name| name.to_str()) != Some("grok") {
        return Err("未找到可锚定的 Grok Build 安装".to_string());
    }
    let meta = std::fs::symlink_metadata(path)
        .map_err(|_| "未找到可锚定的 Grok Build 安装".to_string())?;
    if meta.is_dir() {
        return Err("未找到可锚定的 Grok Build 安装".to_string());
    }
    Ok(path)
}

#[cfg(target_os = "macos")]
fn run_anchored_grok(
    bin_path: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::process::{Command, Stdio};

    if !grok_args_are_closed(args) {
        return Err("Invalid tool command arguments".to_string());
    }
    let path = validate_anchored_grok_bin(bin_path)?;
    let mut cmd = Command::new(path);
    cmd.args(args)
        .current_dir(crate::config::get_home_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path_value) = grok_execution_path() {
        cmd.env("PATH", path_value);
    }
    isolate_child_process_group(&mut cmd);
    let child = cmd
        .spawn()
        .map_err(|error| format!("启动 Grok Build 失败: {error}"))?;
    wait_child_output_with_limit(
        child,
        CommandDeadline::from_timeout(Some(timeout)),
        Some(GROK_OUTPUT_LIMIT),
    )
}

#[cfg(target_os = "macos")]
fn grok_execution_path() -> Option<String> {
    let inherited = std::env::var("PATH").unwrap_or_default();
    login_shell_path().map(|login| merge_path_segments(&login, &inherited))
}

#[cfg(target_os = "macos")]
fn run_bash_script(path: &Path, timeout: Duration) -> Result<std::process::Output, String> {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("设置安装脚本权限失败: {error}"))?;

    let mut cmd = Command::new("/bin/bash");
    cmd.arg(path)
        .current_dir(crate::config::get_home_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path_value) = grok_execution_path() {
        cmd.env("PATH", path_value);
    }
    isolate_child_process_group(&mut cmd);
    let child = cmd
        .spawn()
        .map_err(|error| format!("启动官方安装脚本失败: {error}"))?;
    wait_child_output_with_limit(
        child,
        CommandDeadline::from_timeout(Some(timeout)),
        Some(GROK_OUTPUT_LIMIT),
    )
}

#[cfg(target_os = "macos")]
fn run_login_bash(command: &str, timeout: Duration) -> Result<std::process::Output, String> {
    use std::process::{Command, Stdio};

    let mut cmd = Command::new("/bin/bash");
    cmd.arg("-c")
        .arg(command)
        .current_dir(crate::config::get_home_dir())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path_value) = grok_execution_path() {
        cmd.env("PATH", path_value);
    }
    isolate_child_process_group(&mut cmd);
    let child = cmd
        .spawn()
        .map_err(|error| format!("启动 npm 安装失败: {error}"))?;
    wait_child_output_with_limit(
        child,
        CommandDeadline::from_timeout(Some(timeout)),
        Some(GROK_OUTPUT_LIMIT),
    )
}

#[cfg(target_os = "windows")]
pub(super) async fn run_windows_grok_helper_lifecycle(
    action: ToolLifecycleAction,
) -> Result<(), String> {
    let (tool_action, expected_owner) = grok_helper_request(action);
    tokio::task::spawn_blocking(move || {
        let context = crate::windows_runtime::require_interactive_user_context();
        crate::codex_desktop::platform::windows::run_grok_tool_operation(
            context,
            tool_action,
            expected_owner,
        )
        .map_err(map_grok_helper_error)?;
        Ok(())
    })
    .await
    .map_err(|error| format!("tool lifecycle task join error: {error}"))?
}

#[cfg(target_os = "windows")]
pub(super) async fn observe_windows_grok_via_helper(
) -> Result<fyagent_user_helper::ToolOperationResult, String> {
    tokio::task::spawn_blocking(|| {
        let context = crate::windows_runtime::require_interactive_user_context();
        crate::codex_desktop::platform::windows::run_grok_tool_operation(
            context,
            fyagent_user_helper::GrokToolAction::Observe,
            None,
        )
        .map_err(map_grok_helper_error)
    })
    .await
    .map_err(|error| format!("tool observe task join error: {error}"))?
}

#[cfg(target_os = "windows")]
fn grok_helper_request(
    action: ToolLifecycleAction,
) -> (
    fyagent_user_helper::GrokToolAction,
    Option<fyagent_user_helper::GrokOwner>,
) {
    match action {
        ToolLifecycleAction::Install => (fyagent_user_helper::GrokToolAction::Install, None),
        ToolLifecycleAction::InstallOfficialNpm => (
            fyagent_user_helper::GrokToolAction::Install,
            Some(fyagent_user_helper::GrokOwner::Npm),
        ),
        ToolLifecycleAction::Update => (fyagent_user_helper::GrokToolAction::Update, None),
    }
}

#[cfg(target_os = "windows")]
fn map_grok_helper_error(error: crate::codex_desktop::error::InstallerError) -> String {
    use fyagent_user_helper::HelperErrorCode;
    match error.to_dto().details.platform_error_code.as_deref() {
        Some("grok_tool_host_missing") => HelperErrorCode::ToolHostMissing.redacted_message(),
        Some("grok_tool_timed_out") => HelperErrorCode::ToolTimedOut.redacted_message(),
        Some("grok_tool_output_limit") => HelperErrorCode::ToolOutputLimit.redacted_message(),
        Some("grok_tool_owner_mismatch") => HelperErrorCode::ToolOwnerMismatch.redacted_message(),
        Some("grok_tool_not_detected") => HelperErrorCode::ToolNotDetected.redacted_message(),
        Some("grok_tool_execution_failed") => {
            HelperErrorCode::ToolExecutionFailed.redacted_message()
        }
        _ => "Grok Build is unavailable for the current Windows user.",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(path: &str, real: &str, source: &str, is_default: bool) -> ToolInstallation {
        ToolInstallation {
            path: path.to_string(),
            version: Some("1.0.5".to_string()),
            runnable: true,
            error: None,
            source: source.to_string(),
            is_path_default: is_default,
            real: PathBuf::from(real),
        }
    }

    #[test]
    fn characterization_native_layout_is_internal_owner() {
        let installs = [install(
            "/Users/me/.grok/bin/grok",
            "/Users/me/.grok/downloads/grok-macos-aarch64",
            "system",
            true,
        )];
        assert_eq!(
            observe_grok_owner(&installs, Some("[cli]\ninstaller = \"internal\"\n")),
            GrokOwnerObservation::NativeInternal
        );
    }

    #[test]
    fn npm_layout_is_official_npm_owner() {
        let installs = [install(
            "/Users/me/.nvm/versions/node/v22.14.0/bin/grok",
            "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@xai-official/grok/bin/grok",
            "nvm",
            true,
        )];
        assert_eq!(
            observe_grok_owner(&installs, None),
            GrokOwnerObservation::OfficialNpm
        );
    }

    #[test]
    fn native_layout_with_npm_installer_config_stays_npm_owner() {
        let installs = [install(
            "/Users/me/.grok/bin/grok",
            "/Users/me/.grok/bin/grok",
            "system",
            true,
        )];
        assert_eq!(
            observe_grok_owner(&installs, Some("[cli]\ninstaller = \"npm\"\n")),
            GrokOwnerObservation::OfficialNpm
        );
    }

    #[test]
    fn mixed_native_and_npm_installs_are_ambiguous() {
        let installs = [
            install(
                "/Users/me/.grok/bin/grok",
                "/Users/me/.grok/downloads/grok-macos-aarch64",
                "system",
                true,
            ),
            install(
                "/Users/me/.nvm/versions/node/v22.14.0/bin/grok",
                "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@xai-official/grok/bin/grok",
                "nvm",
                false,
            ),
        ];
        assert_eq!(
            observe_grok_owner(&installs, Some("[cli]\ninstaller = \"internal\"\n")),
            GrokOwnerObservation::Ambiguous
        );
    }

    #[test]
    fn native_failure_plan_does_not_switch_to_npm() {
        let installs = [install(
            "/Users/me/.grok/bin/grok",
            "/Users/me/.grok/downloads/grok-macos-aarch64",
            "system",
            true,
        )];
        let plan = grok_plan_from_installs(
            ToolLifecycleAction::Update,
            &installs,
            Some("[cli]\ninstaller = \"internal\"\n"),
        )
        .expect("native update plan");
        assert_eq!(
            plan,
            GrokPlan::NativeUpdate {
                bin_path: "/Users/me/.grok/bin/grok".to_string()
            }
        );
        assert!(
            grok_plan_from_installs(
                ToolLifecycleAction::InstallOfficialNpm,
                &installs,
                Some("[cli]\ninstaller = \"internal\"\n"),
            )
            .is_ok(),
            "explicit npm remains a separate action"
        );
    }

    #[test]
    fn default_install_does_not_convert_existing_npm() {
        let installs = [install(
            "/Users/me/.nvm/versions/node/v22.14.0/bin/grok",
            "/Users/me/.nvm/versions/node/v22.14.0/lib/node_modules/@xai-official/grok/bin/grok",
            "nvm",
            true,
        )];
        let error = grok_plan_from_installs(ToolLifecycleAction::Install, &installs, None)
            .expect_err("must not convert npm to native");
        assert_eq!(error.reason, "distribution_owner_mismatch");
    }

    #[test]
    fn update_without_install_does_not_silently_install_npm() {
        let error = grok_plan_from_installs(ToolLifecycleAction::Update, &[], None)
            .expect_err("update must not invent an npm install");
        assert_eq!(error.reason, "post_install_not_observed");
    }

    #[test]
    fn parse_update_check_freezes_newer_version() {
        let parsed =
            parse_grok_update_check("Update available: 1.0.6 (current 1.0.5)\n", Some("1.0.5"))
                .expect("parse");
        assert_eq!(
            parsed,
            GrokCheckResult::UpdateAvailable {
                version: "1.0.6".to_string()
            }
        );
    }

    #[test]
    fn parse_update_check_already_current() {
        let parsed =
            parse_grok_update_check("grok 1.0.5 is up to date\n", Some("1.0.5")).expect("parse");
        assert_eq!(
            parsed,
            GrokCheckResult::AlreadyCurrent {
                version: "1.0.5".to_string()
            }
        );
    }

    #[test]
    fn parse_update_check_rejects_empty() {
        assert_eq!(
            parse_grok_update_check("   ", None),
            Err("official_source_unreachable")
        );
    }

    #[test]
    fn installer_url_allows_only_https_xai() {
        assert!(grok_installer_url_is_allowed(
            &url::Url::parse("https://x.ai/cli/install.sh").expect("url")
        ));
        assert!(!grok_installer_url_is_allowed(
            &url::Url::parse("http://x.ai/cli/install.sh").expect("url")
        ));
        assert!(!grok_installer_url_is_allowed(
            &url::Url::parse(
                "https://storage.googleapis.com/grok-build-public-artifacts/install.sh"
            )
            .expect("url")
        ));
        assert!(!grok_installer_url_is_allowed(
            &url::Url::parse("https://example.com/install.sh").expect("url")
        ));
    }

    #[test]
    fn redaction_hides_home_query_and_tokens() {
        let home = crate::config::get_home_dir();
        let raw = format!(
            "downloading https://x.ai/cli/install.sh?token=secret npm_abc123DEF from {}/.grok/bin",
            home.display()
        );
        let redacted = redact_grok_lifecycle_text(&raw);
        assert!(!redacted.contains("secret"), "{redacted}");
        assert!(!redacted.contains("npm_abc123DEF"), "{redacted}");
        assert!(
            !redacted.contains(&home.to_string_lossy().into_owned()),
            "{redacted}"
        );
        assert!(
            redacted.contains('~') || redacted.contains("<redacted>"),
            "{redacted}"
        );
    }

    #[test]
    fn failed_snapshot_is_terminal_and_persists() {
        let message = fail_job(
            ToolLifecycleAction::Update,
            Some(GrokDistributionOwner::NativeInternal),
            "source_exhausted",
            "exit 1 from official updater",
            Some(1),
            false,
            Some("native_updater"),
        );
        assert!(message.contains("官方来源都不可用"));
        assert!(!message.contains("source_exhausted"));
        let stored = last_grok_lifecycle_snapshot().expect("snapshot");
        assert_eq!(stored.stage(), "failed");
        assert_eq!(stored.reason(), Some("source_exhausted"));
        assert_eq!(stored.owner(), Some(GROK_DISTRIBUTION_NATIVE));
        assert!(!stored.redacted_log().is_empty());
        assert!(!stored.timed_out());
        assert!(GrokLifecycleStage::Failed.is_terminal());
        assert!(GrokLifecycleStage::Cancelled.is_terminal());
        assert_eq!(GrokLifecycleStage::Cancelled.as_str(), "cancelled");
        assert_eq!(
            last_grok_lifecycle_error().as_deref(),
            Some(message.as_str())
        );
        succeed_job(
            ToolLifecycleAction::Update,
            GrokDistributionOwner::NativeInternal,
            "updated",
            Some("native_updater"),
        );
        assert_eq!(last_grok_lifecycle_error(), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn anchored_grok_rejects_relative_and_wrong_names() {
        assert!(validate_anchored_grok_bin("grok").is_err());
        assert!(validate_anchored_grok_bin("/tmp/not-grok").is_err());
        assert!(validate_anchored_grok_bin("").is_err());
    }
}
