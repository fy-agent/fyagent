//! Managed-Agent adapter for the shared Codex-tested macOS DMG transaction.
//!
//! This module owns only product policy and translation to the Agent lifecycle
//! contract. Mounting, staging, replacement, rollback and generated-path
//! confinement remain in `codex_desktop::platform::macos::dmg`.

use std::path::PathBuf;

#[cfg(target_os = "macos")]
use std::{fs::File, os::fd::AsRawFd};

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
fn callback_error(reason: AgentReasonCode) -> crate::codex_desktop::error::InstallerError {
    use crate::codex_desktop::error::{InstallerError, InstallerErrorCode};
    let code = if reason == AgentReasonCode::Cancelled {
        InstallerErrorCode::DownloadCancelled
    } else {
        InstallerErrorCode::InternalError
    };
    InstallerError::new(code)
        .with_diagnostic_message("managed Agent commit gate rejected the transaction")
}

#[cfg(target_os = "macos")]
fn verification_error() -> crate::codex_desktop::error::InstallerError {
    use crate::codex_desktop::error::{InstallerError, InstallerErrorCode};
    InstallerError::new(InstallerErrorCode::InstallationVerifyFailed)
        .with_diagnostic_message("managed Agent inventory readback rejected the installation")
}

#[cfg(target_os = "macos")]
fn map_failure(
    kind: crate::codex_desktop::platform::macos::dmg::ManagedDmgFailureKind,
) -> AgentReasonCode {
    use crate::codex_desktop::platform::macos::dmg::ManagedDmgFailureKind;
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
pub(super) fn deploy_macos_dmg<CommitStage, BeforeVerify>(
    product: AgentCatalogId,
    artifact: &DownloadedArtifact,
    target: DesktopDeploymentTarget,
    expected_release_version: Option<String>,
    mut commit_stage: CommitStage,
    mut before_verify: BeforeVerify,
) -> Result<MacosDeploymentResult, AgentReasonCode>
where
    CommitStage: FnMut(bool) -> Result<(), AgentReasonCode>,
    BeforeVerify: FnMut() -> Result<(), AgentReasonCode>,
{
    use crate::codex_desktop::platform::macos::{
        dmg::{install_managed_exact, ManagedDmgInstallIntent, ManagedDmgInstallRequest},
        StdMacosFilesystem, SystemCommandRunner,
    };

    use super::desktop::{
        capture_desktop_installation_baseline, macos_bundle_id_for, user_applications_dir,
        verify_desktop_deployment,
    };

    let bundle_id = macos_bundle_id_for(product).ok_or(AgentReasonCode::SourceNotVerified)?;
    let product_policy = managed_dmg_product_policy(product, bundle_id)?;
    let system_target = matches!(
        target,
        DesktopDeploymentTarget::Existing {
            scope: InstallationScope::AllUsers,
            ..
        } | DesktopDeploymentTarget::Fresh(
            super::inventory::FreshDestinationCapability::MacSystemApplications
        )
    );
    if system_target {
        return deploy_macos_system_dmg(
            product,
            artifact,
            target,
            expected_release_version,
            &product_policy,
            &mut commit_stage,
            &mut before_verify,
        );
    }
    let (intent, expected_scope) = match target {
        DesktopDeploymentTarget::Existing {
            path,
            scope: InstallationScope::CurrentUser,
            package_kind: InstallationPackageKind::AppBundle,
        } => (
            ManagedDmgInstallIntent::Update { target: path },
            InstallationScope::CurrentUser,
        ),
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
            super::inventory::FreshDestinationCapability::MacSystemApplications => unreachable!(),
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
        || commit_stage(false).map_err(callback_error),
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

#[cfg(target_os = "macos")]
fn deploy_macos_system_dmg<CommitStage, BeforeVerify>(
    product: AgentCatalogId,
    artifact: &DownloadedArtifact,
    target: DesktopDeploymentTarget,
    expected_release_version: Option<String>,
    product_policy: &crate::codex_desktop::platform::macos::dmg::ManagedDmgProductPolicy,
    commit_stage: &mut CommitStage,
    before_verify: &mut BeforeVerify,
) -> Result<MacosDeploymentResult, AgentReasonCode>
where
    CommitStage: FnMut(bool) -> Result<(), AgentReasonCode>,
    BeforeVerify: FnMut() -> Result<(), AgentReasonCode>,
{
    use crate::codex_desktop::platform::macos::{
        dmg::{
            install_managed_system_exact, ManagedDmgSystemCommitRequest, ManagedDmgSystemFailure,
            ManagedDmgSystemIntent,
        },
        StdMacosFilesystem, SystemCommandRunner,
    };
    use crate::macos_system_commit::{
        product_for_agent, production_enabled, production_port, resolve_slot,
        AuthorizedSystemCommit, MacSystemCommitPort, SystemCommitAction, SystemCommitOutcome,
        UserIntent,
    };

    use super::desktop::{capture_desktop_installation_baseline, verify_desktop_deployment};

    if !production_enabled() {
        return Err(crate::macos_system_commit::system_scope_rejection());
    }
    let system_product = product_for_agent(product)?;
    let slot = resolve_slot(system_product, 1)?;
    let fixed_target = PathBuf::from("/Applications").join(slot.basename);
    let (intent, action) = match target {
        DesktopDeploymentTarget::Fresh(
            super::inventory::FreshDestinationCapability::MacSystemApplications,
        ) => (
            ManagedDmgSystemIntent::Fresh,
            SystemCommitAction::FreshInstall,
        ),
        DesktopDeploymentTarget::Existing {
            path,
            scope: InstallationScope::AllUsers,
            package_kind: InstallationPackageKind::AppBundle,
        } if path == fixed_target => (
            ManagedDmgSystemIntent::Update,
            SystemCommitAction::UpdateExisting,
        ),
        _ => return Err(AgentReasonCode::TargetChanged),
    };

    let baseline = capture_desktop_installation_baseline(product);
    artifact
        .revalidate()
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    let runner = SystemCommandRunner;
    let filesystem = StdMacosFilesystem;
    let installed = install_managed_system_exact(
        &runner,
        &filesystem,
        ManagedDmgSystemCommitRequest {
            artifact_path: artifact.path(),
            target_path: &fixed_target,
            intent,
            product: product_policy,
            expected_release_version: expected_release_version.as_deref(),
        },
        |source| {
            commit_stage(true)?;
            let source_directory = File::open(&source.bundle_path)
                .map_err(|_| AgentReasonCode::SourceCapabilityInvalid)?;
            if !source_directory
                .metadata()
                .map_err(|_| AgentReasonCode::SourceCapabilityInvalid)?
                .is_dir()
            {
                return Err(AgentReasonCode::SourceCapabilityInvalid);
            }
            let port = production_port();
            port.ensure_helper_ready(UserIntent::attested())?;
            commit_stage(false)?;
            let request = AuthorizedSystemCommit::new(
                system_product,
                1,
                action,
                *uuid::Uuid::new_v4().as_bytes(),
                source.source_revision,
                source.target_revision,
                source_directory.as_raw_fd(),
            )?;
            match port.commit_known_application(request)? {
                SystemCommitOutcome::Committed => Ok(()),
                SystemCommitOutcome::RollbackRestored => Err(AgentReasonCode::RollbackRestored),
                SystemCommitOutcome::RecoveryRequired => Err(AgentReasonCode::RecoveryRequired),
            }
        },
    )
    .map_err(|error| match error {
        ManagedDmgSystemFailure::Package(kind) => map_failure(kind),
        ManagedDmgSystemFailure::Commit(reason) => reason,
    })?;

    before_verify()?;
    verify_desktop_deployment(
        product,
        &baseline,
        &installed.target_path,
        InstallationScope::AllUsers,
        &installed.local_version,
    )?;
    Ok(MacosDeploymentResult {
        target_path: installed.target_path,
        local_version: installed.local_version,
    })
}

#[cfg(not(target_os = "macos"))]
pub(super) fn deploy_macos_dmg<CommitStage, BeforeVerify>(
    _product: AgentCatalogId,
    _artifact: &DownloadedArtifact,
    _target: DesktopDeploymentTarget,
    _expected_release_version: Option<String>,
    _commit_stage: CommitStage,
    _before_verify: BeforeVerify,
) -> Result<MacosDeploymentResult, AgentReasonCode>
where
    CommitStage: FnMut(bool) -> Result<(), AgentReasonCode>,
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
