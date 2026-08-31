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
fn managed_dmg_product_policy(
    product: AgentCatalogId,
    bundle_id: &'static str,
) -> Result<crate::codex_desktop::platform::macos::dmg::ManagedDmgProductPolicy, AgentReasonCode> {
    use crate::codex_desktop::platform::macos::dmg::{
        ManagedBundleVersionSource, ManagedDmgProductPolicy, ManagedVersionEquivalence,
    };
    match product {
        AgentCatalogId::QoderWork => Ok(ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::Exact,
        }),
        AgentCatalogId::TraeWork => Ok(ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::TraeProductJson,
            version_equivalence: ManagedVersionEquivalence::Exact,
        }),
        AgentCatalogId::WorkBuddy => Ok(ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::DottedPrefix,
        }),
        AgentCatalogId::OpenCode | AgentCatalogId::ClaudeCode => Ok(ManagedDmgProductPolicy {
            expected_bundle_id: bundle_id,
            version_source: ManagedBundleVersionSource::InfoPlist,
            version_equivalence: ManagedVersionEquivalence::Exact,
        }),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
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
                install_managed_exact, ManagedDmgFailureKind, ManagedDmgInstallIntent,
                ManagedDmgInstallRequest,
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
    let product_policy = managed_dmg_product_policy(product, bundle_id)?;
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
        } => {
            // MacSystemCommitPort owns privileged /Applications commit.
            // production_enabled() is false until signed/notarized HIL.
            return Err(crate::macos_system_commit::system_scope_rejection());
        }
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
                return Err(crate::macos_system_commit::system_scope_rejection());
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

#[cfg(test)]
mod tests {
    use super::super::types::AgentReasonCode;

    #[test]
    fn production_system_scope_stays_authorization_required() {
        assert!(!crate::macos_system_commit::production_enabled());
        assert_eq!(
            crate::macos_system_commit::system_scope_rejection(),
            AgentReasonCode::AuthorizationRequired
        );
        assert!(crate::macos_system_commit::resolve_slot(
            crate::macos_system_commit::KnownSystemProduct::QoderWork,
            1,
        )
        .is_ok());
    }
}

#[cfg(all(test, target_os = "macos"))]
mod policy_drift_tests {
    use super::super::desktop::macos_bundle_id_for;
    use crate::macos_system_commit::{resolve_slot, KnownSystemProduct};
    use crate::services::external_agents::AgentCatalogId;

    #[test]
    fn helper_policy_bundle_ids_match_macos_bundle_id_for() {
        let pairs = [
            (
                KnownSystemProduct::OpenCodeDesktop,
                AgentCatalogId::OpenCode,
            ),
            (KnownSystemProduct::QoderWork, AgentCatalogId::QoderWork),
            (KnownSystemProduct::TraeWork, AgentCatalogId::TraeWork),
            (KnownSystemProduct::WorkBuddy, AgentCatalogId::WorkBuddy),
        ];
        for (product, agent) in pairs {
            assert_eq!(
                resolve_slot(product, 1).expect("slot").bundle_id,
                macos_bundle_id_for(agent).expect("bundle id")
            );
        }
    }

    #[test]
    fn claude_desktop_uses_info_plist_exact_like_opencode() {
        use crate::codex_desktop::platform::macos::dmg::{
            ManagedBundleVersionSource, ManagedVersionEquivalence,
        };

        let claude = super::managed_dmg_product_policy(
            AgentCatalogId::ClaudeCode,
            "com.anthropic.claudefordesktop",
        )
        .expect("claude desktop policy");
        let opencode =
            super::managed_dmg_product_policy(AgentCatalogId::OpenCode, "ai.opencode.desktop")
                .expect("opencode desktop policy");
        assert!(matches!(
            claude.version_source,
            ManagedBundleVersionSource::InfoPlist
        ));
        assert!(matches!(
            claude.version_equivalence,
            ManagedVersionEquivalence::Exact
        ));
        assert!(matches!(
            opencode.version_source,
            ManagedBundleVersionSource::InfoPlist
        ));
        assert!(matches!(
            opencode.version_equivalence,
            ManagedVersionEquivalence::Exact
        ));
        assert_eq!(
            super::managed_dmg_product_policy(AgentCatalogId::GrokBuild, "unused")
                .expect_err("unknown products stay unimplemented"),
            crate::agent_install::AgentReasonCode::ExecutorNotImplemented
        );
    }
}
