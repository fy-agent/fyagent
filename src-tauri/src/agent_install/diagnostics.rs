//! Package-preparation diagnostics for desktop agent installs.
//!
//! Lines go to the process log (`fyagent.log`) before job temp cleanup.
//! They never include raw user names, tokens, or query strings.

use std::path::Path;

use super::types::{AgentActionJobStage, AgentReasonCode};
use crate::codex_desktop::error::{redact_diagnostic_text, InstallerError, InstallerErrorCode};
use crate::services::external_agents::AgentCatalogId;

pub(crate) struct InstallDiagnostic {
    pub agent: AgentCatalogId,
    pub job_stage: AgentActionJobStage,
    pub code: String,
    pub message_key: String,
    pub target: String,
    pub os_error: String,
    pub detail: String,
}

impl InstallDiagnostic {
    pub(crate) fn format_line(&self) -> String {
        format!(
            "agent_install_diagnostic agent={} code={} message_key={} job_stage={} target={} os_error={} detail={}",
            wire_token(serde_token(self.agent), "unknown"),
            wire_token(&self.code, "Unknown"),
            wire_token(&self.message_key, "agentInstall.error.unknown"),
            wire_token(serde_token(self.job_stage), "failed"),
            field(&self.target),
            field(&self.os_error),
            field(&self.detail),
        )
    }
}

pub(crate) fn log_install_diagnostic(record: &InstallDiagnostic) {
    log::warn!("{}", record.format_line());
}

pub(crate) fn diagnostic_from_installer(
    agent: AgentCatalogId,
    job_stage: AgentActionJobStage,
    error: &InstallerError,
    target: Option<&Path>,
) -> InstallDiagnostic {
    let dto = error.to_dto();
    InstallDiagnostic {
        agent,
        job_stage,
        code: format!("{:?}", dto.code),
        message_key: dto.message_key,
        target: redact_path(target),
        os_error: dto
            .details
            .platform_error_code
            .unwrap_or_else(|| "-".to_string()),
        detail: dto
            .details
            .redacted_message
            .unwrap_or_else(|| "-".to_string()),
    }
}

pub(crate) fn diagnostic_from_reason(
    agent: AgentCatalogId,
    job_stage: AgentActionJobStage,
    reason: AgentReasonCode,
    target: Option<&Path>,
    os_error: Option<&str>,
    detail: Option<&str>,
) -> InstallDiagnostic {
    InstallDiagnostic {
        agent,
        job_stage,
        code: format!("{reason:?}"),
        message_key: agent_message_key(reason),
        target: redact_path(target),
        os_error: os_error
            .map(redact_diagnostic_text)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "-".to_string()),
        detail: detail
            .map(redact_diagnostic_text)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "-".to_string()),
    }
}

pub(crate) fn agent_reason_for_package_preparation(error: &InstallerError) -> AgentReasonCode {
    let platform = error.to_dto().details.platform_error_code;
    match package_preparation_reason(error.code(), platform.as_deref()) {
        Some(reason) => reason,
        None => AgentReasonCode::InstallerArtifactUnavailable,
    }
}

// Windows installer mapping is the production caller. macOS lib builds do not compile it.
#[allow(dead_code)]
pub(crate) fn agent_reason_from_installer_parts(
    code: InstallerErrorCode,
    platform_error_code: Option<&str>,
) -> AgentReasonCode {
    if let Some(reason) = platform_reason(platform_error_code) {
        return reason;
    }
    if let Some(reason) = package_preparation_reason(code, platform_error_code) {
        return reason;
    }
    match code {
        InstallerErrorCode::DownloadCancelled => AgentReasonCode::Cancelled,
        InstallerErrorCode::PackageArchitectureMismatch
        | InstallerErrorCode::ArchitectureUnsupported => AgentReasonCode::PlatformUnsupported,
        InstallerErrorCode::ReleaseMetadataInvalid | InstallerErrorCode::ReleaseNotAvailable => {
            AgentReasonCode::SourceNotVerified
        }
        InstallerErrorCode::MetadataChanged => AgentReasonCode::RefreshRequired,
        InstallerErrorCode::WindowsPackageInUse => AgentReasonCode::ApplicationRunning,
        InstallerErrorCode::WindowsDeploymentBlocked
        | InstallerErrorCode::WindowsDependencyMissing => AgentReasonCode::AuthorizationRequired,
        InstallerErrorCode::WindowsDeploymentFailed
        | InstallerErrorCode::LaunchFailed
        | InstallerErrorCode::InternalError => AgentReasonCode::InteractiveUserUnavailable,
        InstallerErrorCode::MultipleInstallations
        | InstallerErrorCode::InstallationVerifyFailed => {
            AgentReasonCode::InstallationVerificationFailed
        }
        InstallerErrorCode::JobAlreadyRunning | InstallerErrorCode::JobNotFound => {
            AgentReasonCode::OperationConflict
        }
        _ => AgentReasonCode::ExecutorNotImplemented,
    }
}

fn package_preparation_reason(
    code: InstallerErrorCode,
    platform_error_code: Option<&str>,
) -> Option<AgentReasonCode> {
    if let Some(reason) = platform_reason(platform_error_code) {
        return Some(reason);
    }
    Some(match code {
        InstallerErrorCode::DownloadFailed => AgentReasonCode::DownloadFailed,
        InstallerErrorCode::DownloadTimeout => AgentReasonCode::DownloadTimeout,
        InstallerErrorCode::InsufficientDiskSpace => AgentReasonCode::InsufficientDiskSpace,
        InstallerErrorCode::ChecksumMismatch | InstallerErrorCode::PackageSignatureInvalid => {
            AgentReasonCode::ChecksumMismatch
        }
        InstallerErrorCode::PackageParseFailed => AgentReasonCode::PackageParseFailed,
        InstallerErrorCode::PackageIdentityMismatch => AgentReasonCode::PackageIdentityMismatch,
        InstallerErrorCode::RedirectRejected | InstallerErrorCode::SourceUnavailable => {
            AgentReasonCode::SourceNotVerified
        }
        _ => return None,
    })
}

fn platform_reason(platform_error_code: Option<&str>) -> Option<AgentReasonCode> {
    match platform_error_code {
        Some("agent_installer_user_cancelled") => Some(AgentReasonCode::InstallerUserCancelled),
        Some("agent_installer_process_unobservable") => {
            Some(AgentReasonCode::InstallerProcessUnobservable)
        }
        Some("agent_installer_timed_out") => Some(AgentReasonCode::InstallerTimedOut),
        Some("agent_installer_exited_nonzero") => Some(AgentReasonCode::InstallerExitedNonzero),
        Some("insufficient_disk_space") => Some(AgentReasonCode::InsufficientDiskSpace),
        Some("target_permission_denied") | Some("tool_permission_denied") => {
            Some(AgentReasonCode::PermissionDenied)
        }
        Some("target_not_writable") => Some(AgentReasonCode::PathNotWritable),
        Some("tool_candidate_conflict") => Some(AgentReasonCode::CandidateConflict),
        Some("tool_target_changed") => Some(AgentReasonCode::TargetChanged),
        _ => None,
    }
}

fn agent_message_key(reason: AgentReasonCode) -> String {
    let wire = serde_token(reason);
    let mut camel = String::new();
    let mut upper = false;
    for ch in wire.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            camel.extend(ch.to_uppercase());
            upper = false;
        } else {
            camel.push(ch);
        }
    }
    format!("agentInstall.error.{camel}")
}

fn serde_token<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn wire_token(value: impl AsRef<str>, fallback: &str) -> String {
    let token: String = value
        .as_ref()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-') {
                ch
            } else {
                '_'
            }
        })
        .take(120)
        .collect();
    if token.is_empty() {
        fallback.to_string()
    } else {
        token
    }
}

fn field(value: &str) -> String {
    let redacted = redact_diagnostic_text(value);
    let flattened: String = redacted
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .take(300)
        .collect();
    let trimmed = flattened.trim();
    if trimmed.is_empty() {
        "-".to_string()
    } else {
        trimmed.to_string()
    }
}

fn redact_path(path: Option<&Path>) -> String {
    path.map(|path| redact_diagnostic_text(&path.to_string_lossy()))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_desktop::error::{artifact_io_error, classify_artifact_io};
    use std::io::ErrorKind;

    #[test]
    fn diagnostic_line_keeps_stage_codes_and_redacts_user_material() {
        let error = InstallerError::new(InstallerErrorCode::DownloadFailed)
            .with_platform_error_code("5")
            .with_diagnostic_message(
                "write C:\\Users\\alice\\AppData\\Local\\Temp token=secret failed",
            );
        let line = diagnostic_from_installer(
            AgentCatalogId::WorkBuddy,
            AgentActionJobStage::Downloading,
            &error,
            Some(Path::new("C:\\Users\\alice\\AppData\\Local\\Temp\\job")),
        )
        .format_line();

        assert!(line.contains("agent=workbuddy"));
        assert!(line.contains("code=DownloadFailed"));
        assert!(line.contains("message_key=codexDesktop.error.downloadFailed"));
        assert!(line.contains("job_stage=downloading"));
        assert!(line.contains("os_error=5"));
        assert!(line.contains("%USERPROFILE%"));
        assert!(!line.to_ascii_lowercase().contains("alice"));
        assert!(!line.contains("secret"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn diagnostic_line_survives_without_a_job_directory() {
        let line = diagnostic_from_reason(
            AgentCatalogId::WorkBuddy,
            AgentActionJobStage::Staging,
            AgentReasonCode::ChecksumMismatch,
            None,
            None,
            Some("checksum did not match"),
        )
        .format_line();
        assert!(line.contains("code=ChecksumMismatch"));
        assert!(line.contains("message_key=agentInstall.error.checksumMismatch"));
        assert!(line.contains("job_stage=staging"));
        assert!(line.contains("target=-"));
        assert!(line.contains("detail=checksum did not match"));
    }

    #[test]
    fn package_preparation_distinguishes_download_disk_permission_and_checksum() {
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::DownloadFailed, None),
            AgentReasonCode::DownloadFailed
        );
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::DownloadTimeout, None),
            AgentReasonCode::DownloadTimeout
        );
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::InsufficientDiskSpace, None),
            AgentReasonCode::InsufficientDiskSpace
        );
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::ChecksumMismatch, None),
            AgentReasonCode::ChecksumMismatch
        );
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::PackageParseFailed, None),
            AgentReasonCode::PackageParseFailed
        );
        assert_eq!(
            agent_reason_from_installer_parts(InstallerErrorCode::PackageIdentityMismatch, None),
            AgentReasonCode::PackageIdentityMismatch
        );
        assert_eq!(
            agent_reason_from_installer_parts(
                InstallerErrorCode::DownloadFailed,
                Some("target_permission_denied"),
            ),
            AgentReasonCode::PermissionDenied
        );
        assert_eq!(
            agent_reason_from_installer_parts(
                InstallerErrorCode::DownloadFailed,
                Some("target_not_writable"),
            ),
            AgentReasonCode::PathNotWritable
        );
        assert_eq!(
            agent_reason_for_package_preparation(
                &InstallerError::new(InstallerErrorCode::InternalError)
                    .with_diagnostic_message("staging directory unavailable"),
            ),
            AgentReasonCode::InstallerArtifactUnavailable
        );
    }

    #[test]
    fn io_classification_separates_disk_permission_and_path() {
        assert_eq!(
            classify_artifact_io(ErrorKind::PermissionDenied, None).1,
            Some("target_permission_denied")
        );
        assert_eq!(
            classify_artifact_io(ErrorKind::Other, Some(112)).0,
            InstallerErrorCode::InsufficientDiskSpace
        );
        assert_eq!(
            classify_artifact_io(ErrorKind::Other, Some(30)).1,
            Some("target_not_writable")
        );
        let error = artifact_io_error(
            &std::io::Error::new(ErrorKind::PermissionDenied, "denied"),
            "installer partial file could not be written",
        );
        assert_eq!(
            agent_reason_for_package_preparation(&error),
            AgentReasonCode::PermissionDenied
        );
    }
}
