//! One short-lived confirmation of the existing Codex installer plan.

use super::*;
use crate::codex_desktop::platform::ConfirmedInstallTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexInstallPreflight {
    pub contract_version: u16,
    pub confirmation_id: String,
    pub expected_release_id: String,
    pub platform: DesktopPlatform,
    pub architecture: CpuArchitecture,
    pub display_version: String,
    pub download_url: String,
    pub target_label: String,
    pub updating: bool,
    pub available_bytes: u64,
    pub download_size_hint: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmedStartInstallRequest {
    pub expected_release_id: String,
    pub confirmation_id: String,
}

#[derive(Clone)]
pub(super) struct PendingInstallConfirmation {
    id: String,
    release_id: String,
    created_at: Instant,
    target: ConfirmedInstallTarget,
}

impl CodexDesktopService {
    pub async fn prepare_install(
        &self,
        request: StartInstallRequest,
    ) -> Result<CodexInstallPreflight, InstallerError> {
        request.validate()?;
        let release = self
            .resolve_latest(CacheMode::ForceRefresh, &NeverCancelled)
            .await?;
        if request.expected_release_id != release.release_id() {
            return Err(changed());
        }
        let local = self.platform.inspect_local().await?;
        match &local {
            LocalInstallStatus::Unsupported { reason } => {
                return Err(unsupported_status_error(reason.clone()))
            }
            LocalInstallStatus::Ambiguous { error, .. } => {
                return Err(ambiguous_local_status_error(error.code))
            }
            _ => {}
        }
        let directory = self
            .temp_root
            .create_job(&uuid::Uuid::new_v4().to_string())?;
        let result = async {
            let plan = self.platform.preflight(&release, directory.path()).await?;
            directory.revalidate()?;
            let paths = || {
                std::iter::once(directory.path())
                    .chain(plan.additional_disk_paths().iter().map(PathBuf::as_path))
            };
            if let Some(size) = release.download_size_hint {
                ensure_required_disk_space(self.disk_space_probe.as_ref(), paths(), size)?;
            }
            let mut available = u64::MAX;
            for path in paths() {
                let volume = self.disk_space_probe.volume_key(path).map_err(|_| {
                    InstallerError::new(InstallerErrorCode::InternalError)
                        .with_platform_error_code("disk_space_unavailable")
                })?;
                available = available.min(self.disk_space_probe.available_bytes(&volume).map_err(
                    |_| {
                        InstallerError::new(InstallerErrorCode::InternalError)
                            .with_platform_error_code("disk_space_unavailable")
                    },
                )?);
            }
            if available == 0 {
                return Err(InstallerError::new(
                    InstallerErrorCode::InsufficientDiskSpace,
                ));
            }
            directory.revalidate()?;
            let target = ConfirmedInstallTarget {
                local,
                target_root: plan.confirmation_target().map(Path::to_path_buf),
            };
            let target_label = target_label(&target, release.platform);
            let pending = PendingInstallConfirmation {
                id: format!("i1:{}", uuid::Uuid::new_v4().simple()),
                release_id: request.expected_release_id.clone(),
                created_at: Instant::now(),
                target,
            };
            let snapshot = CodexInstallPreflight {
                contract_version: 1,
                confirmation_id: pending.id.clone(),
                expected_release_id: request.expected_release_id,
                platform: release.platform,
                architecture: release.architecture,
                display_version: release.display_version.clone(),
                download_url: release.download_endpoint.url().to_string(),
                target_label,
                updating: matches!(pending.target.local, LocalInstallStatus::Installed { .. }),
                available_bytes: available,
                download_size_hint: release.download_size_hint,
            };
            Ok((snapshot, pending))
        }
        .await;
        directory.cleanup()?;
        let (snapshot, pending) = result?;
        *recover_lock(&self.install_confirmation) = Some(pending);
        Ok(snapshot)
    }

    pub fn start_confirmed_install(
        &self,
        request: ConfirmedStartInstallRequest,
    ) -> Result<JobSnapshot, InstallerError> {
        let mut pending = recover_lock(&self.install_confirmation);
        let selected = pending
            .as_ref()
            .filter(|pending| {
                pending.id == request.confirmation_id
                    && pending.release_id == request.expected_release_id
                    && pending.created_at.elapsed() < Duration::from_secs(300)
            })
            .ok_or_else(changed)?;
        let target = selected.target.clone();
        let snapshot = self.start_install_with_target(
            StartInstallRequest {
                expected_release_id: request.expected_release_id,
            },
            Some(target),
        )?;
        *pending = None;
        Ok(snapshot)
    }
}

fn target_label(target: &ConfirmedInstallTarget, platform: DesktopPlatform) -> String {
    if platform == DesktopPlatform::Windows {
        return "当前桌面用户的 Windows 应用目录（由系统管理）".to_string();
    }
    if let LocalInstallStatus::Installed { application } = &target.local {
        if let crate::codex_desktop::types::LaunchTarget::MacBundlePath(path) =
            &application.launch_target
        {
            return crate::agent_install::display_user_path(path);
        }
    }
    target
        .target_root
        .as_deref()
        .map(crate::agent_install::display_user_path)
        .unwrap_or_else(|| "~/Applications".to_string())
}

fn changed() -> InstallerError {
    InstallerError::new(InstallerErrorCode::MetadataChanged).with_diagnostic_message(
        "the confirmed installation target is unavailable or changed; prepare again",
    )
}
