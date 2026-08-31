//! Managed-Agent adapter for the shared Codex-tested macOS DMG transaction.
//!
//! This module owns only product policy and translation to the Agent lifecycle
//! contract. Mounting, staging, replacement, rollback and generated-path
//! confinement remain in `codex_desktop::platform::macos::dmg`.

use std::path::PathBuf;

#[cfg(target_os = "macos")]
use super::types::{InstallationPackageKind, InstallationScope};
use super::{inventory::DesktopDeploymentTarget, types::AgentReasonCode};
use crate::codex_desktop::download::DownloadedArtifact;
use crate::services::external_agents::AgentCatalogId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MacosDeploymentResult {
    pub target_path: PathBuf,
    pub local_version: String,
}

#[cfg(target_os = "macos")]
pub(super) fn deploy_macos_dmg<BeforeCommit, BeforeVerify>(
    product: AgentCatalogId,
    artifact: &DownloadedArtifact,
    target: DesktopDeploymentTarget,
    expected_release_version: Option<String>,
    mut before_commit: BeforeCommit,
    mut before_verify: BeforeVerify,
) -> Result<MacosDeploymentResult, AgentReasonCode>
where
    BeforeCommit: FnMut() -> Result<(), AgentReasonCode>,
    BeforeVerify: FnMut() -> Result<(), AgentReasonCode>,
{
    use crate::codex_desktop::{
        error::{InstallerError, InstallerErrorCode},
        platform::macos::{
            dmg::{
                install_managed_exact, ManagedBundleVersionSource, ManagedDmgFailureKind,
                ManagedDmgInstallIntent, ManagedDmgInstallRequest, ManagedDmgProductPolicy,
                ManagedVersionEquivalence,
            },
            StdMacosFilesystem, SystemCommandRunner,
        },
    };

    use super::desktop::{
        capture_desktop_installation_baseline, macos_bundle_id_for, user_applications_dir,
        verify_desktop_deployment,
    };

    fn callback_error(reason: AgentReasonCode) -> InstallerError {
        let code = if reason == AgentReasonCode::Cancelled {
            InstallerErrorCode::DownloadCancelled
        } else {
            InstallerErrorCode::InternalError
        };
        InstallerError::new(code)
            .with_diagnostic_message("managed Agent commit gate rejected the transaction")
    }

    fn verification_error() -> InstallerError {
        InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
            .with_diagnostic_message("managed Agent inventory readback rejected the installation")
    }

    fn map_failure(kind: ManagedDmgFailureKind) -> AgentReasonCode {
        match kind {
            ManagedDmgFailureKind::Cancelled => AgentReasonCode::Cancelled,
            ManagedDmgFailureKind::ApplicationRunning => AgentReasonCode::ApplicationRunning,
            ManagedDmgFailureKind::PermissionDenied => AgentReasonCode::PermissionDenied,
            ManagedDmgFailureKind::SourceInvalid => AgentReasonCode::SourceNotVerified,
            ManagedDmgFailureKind::TargetChanged => AgentReasonCode::TargetChanged,
            ManagedDmgFailureKind::VerificationFailedRestored => AgentReasonCode::RollbackRestored,
            ManagedDmgFailureKind::RecoveryRequired => AgentReasonCode::RecoveryRequired,
            ManagedDmgFailureKind::MountFailed
            | ManagedDmgFailureKind::DetachFailed
            | ManagedDmgFailureKind::Failed => AgentReasonCode::ExecutorNotImplemented,
        }
    }

    let bundle_id = macos_bundle_id_for(product).ok_or(AgentReasonCode::SourceNotVerified)?;
    let product_policy = match product {
        AgentCatalogId::QoderWork => ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::Exact,
        },
        AgentCatalogId::TraeWork => ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::TraeProductJson,
            version_equivalence: ManagedVersionEquivalence::Exact,
        },
        AgentCatalogId::WorkBuddy => ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::DottedPrefix,
        },
        AgentCatalogId::OpenCode => ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::Exact,
        },
        _ => return Err(AgentReasonCode::ExecutorNotImplemented),
    };
    let (intent, expected_scope) = match target {
        DesktopDeploymentTarget::Existing {
            path,
            scope: InstallationScope::CurrentUser,
            package_kind: InstallationPackageKind::AppBundle,
        } => (
            ManagedDmgInstallIntent::Update { target: path },
            InstallationScope::CurrentUser,
        ),
        DesktopDeploymentTarget::Existing {
            scope: InstallationScope::AllUsers,
            ..
        } => return Err(AgentReasonCode::AuthorizationRequired),
        DesktopDeploymentTarget::Existing { .. } => {
            return Err(AgentReasonCode::TargetScopeUnsupported)
        }
        DesktopDeploymentTarget::Fresh(destination) => match destination {
            super::inventory::FreshDestinationCapability::MacUserApplications => (
                ManagedDmgInstallIntent::Fresh {
                    parent: user_applications_dir()?,
                },
                InstallationScope::CurrentUser,
            ),
            super::inventory::FreshDestinationCapability::MacSystemApplications => {
                return Err(AgentReasonCode::AuthorizationRequired)
            }
            _ => return Err(AgentReasonCode::TargetScopeUnsupported),
        },
    };
    let baseline = capture_desktop_installation_baseline(product);
    artifact
        .revalidate()
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    let runner = SystemCommandRunner;
    let filesystem = StdMacosFilesystem;
    let installed = install_managed_exact(
        &runner,
        &filesystem,
        ManagedDmgInstallRequest {
            artifact_path: artifact.path(),
            intent,
            product: &product_policy,
            expected_release_version: expected_release_version.as_deref(),
        },
        || before_commit().map_err(callback_error),
        |result| {
            before_verify().map_err(callback_error)?;
            verify_desktop_deployment(
                product,
                &baseline,
                &result.target_path,
                expected_scope,
                &result.local_version,
            )
            .map_err(|_| verification_error())
        },
    )
    .map_err(|error| map_failure(error.kind()))?;
    Ok(MacosDeploymentResult {
        target_path: installed.target_path,
        local_version: installed.local_version,
    })
}

#[cfg(not(target_os = "macos"))]
pub(super) fn deploy_macos_dmg<BeforeCommit, BeforeVerify>(
    _product: AgentCatalogId,
    _artifact: &DownloadedArtifact,
    _target: DesktopDeploymentTarget,
    _expected_release_version: Option<String>,
    _before_commit: BeforeCommit,
    _before_verify: BeforeVerify,
) -> Result<MacosDeploymentResult, AgentReasonCode>
where
    BeforeCommit: FnMut() -> Result<(), AgentReasonCode>,
    BeforeVerify: FnMut() -> Result<(), AgentReasonCode>,
{
    Err(AgentReasonCode::PlatformUnsupported)
}
