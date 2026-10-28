//! One short-lived confirmation of the existing Codex installer plan.

use super::*;
use crate::codex_desktop::platform::ConfirmedInstallTarget;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::io::{Read, Write};

const INSTALL_CAPABILITY_TTL: Duration = Duration::from_secs(300);
#[cfg(target_os = "windows")]
const STAGING_PROBE_BYTES: &[u8] = b"FyAgent staging probe";

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
    pub staging_label: String,
    pub custom_staging: bool,
    pub updating: bool,
    pub available_bytes: u64,
    pub download_size_hint: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareInstallRequest {
    pub expected_release_id: String,
    pub staging_capability_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStagingSelection {
    pub contract_version: u16,
    pub capability_id: String,
    pub label: String,
    pub available_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmedStartInstallRequest {
    pub expected_release_id: String,
    pub confirmation_id: String,
}

pub(super) struct PendingInstallConfirmation {
    release_id: String,
    target: ConfirmedInstallTarget,
    temp_root: JobTempRoot,
}

struct ExpiringCapability<T> {
    id: String,
    created_at: Instant,
    value: T,
}

pub(super) struct ExpiringCapabilitySlot<T> {
    pending: Arc<Mutex<Option<ExpiringCapability<T>>>>,
}

impl<T> Clone for ExpiringCapabilitySlot<T> {
    fn clone(&self) -> Self {
        Self {
            pending: self.pending.clone(),
        }
    }
}

impl<T> Default for ExpiringCapabilitySlot<T> {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T: Send + 'static> ExpiringCapabilitySlot<T> {
    fn insert(&self, id: String, value: T, runtime: &tokio::runtime::Handle) {
        *recover_lock(&self.pending) = Some(ExpiringCapability {
            id: id.clone(),
            created_at: Instant::now(),
            value,
        });
        let slot = self.clone();
        std::mem::drop(runtime.spawn(async move {
            tokio::time::sleep(INSTALL_CAPABILITY_TTL).await;
            slot.discard(&id);
        }));
    }
    fn evict_expired_locked(pending: &mut Option<ExpiringCapability<T>>) {
        if pending
            .as_ref()
            .is_some_and(|pending| capability_expired(pending.created_at))
        {
            *pending = None;
        }
    }

    fn evict_expired(&self) {
        let mut pending = recover_lock(&self.pending);
        Self::evict_expired_locked(&mut pending);
    }
    fn take_valid(&self, id: &str) -> Option<T> {
        let mut pending = recover_lock(&self.pending);
        Self::evict_expired_locked(&mut pending);
        if pending.as_ref().is_some_and(|pending| pending.id == id) {
            pending.take().map(|pending| pending.value)
        } else {
            None
        }
    }

    fn discard(&self, id: &str) {
        let mut pending = recover_lock(&self.pending);
        Self::evict_expired_locked(&mut pending);
        if pending.as_ref().is_some_and(|pending| pending.id == id) {
            *pending = None;
        }
    }

    #[cfg(test)]
    fn backdate(&self, age: Duration) {
        if let Some(pending) = recover_lock(&self.pending).as_mut() {
            pending.created_at = Instant::now() - age;
        }
    }
}

impl CodexDesktopService {
    #[cfg(target_os = "windows")]
    pub fn select_staging_directory(
        &self,
        path: &Path,
    ) -> Result<CodexStagingSelection, InstallerError> {
        let root = JobTempRoot::from_selected_directory(path)
            .map_err(staging_directory_unavailable_error)?;
        let probe = root
            .create_job(&uuid::Uuid::new_v4().hyphenated().to_string())
            .map_err(staging_permission_error)?;
        let result = (|| {
            probe
                .revalidate()
                .map_err(staging_directory_unavailable_error)?;
            let mut artifact = probe
                .create_part_file(ArtifactKind::Msix)
                .map_err(staging_permission_error)?;
            artifact
                .write_all(STAGING_PROBE_BYTES)
                .map_err(staging_probe_io_error)?;
            artifact.sync_all().map_err(staging_probe_io_error)?;
            probe
                .finalize_part_file(ArtifactKind::Msix, artifact)
                .map_err(staging_permission_error)?;
            let mut reopened = probe
                .open_final_artifact_for_read(ArtifactKind::Msix)
                .map_err(staging_permission_error)?;
            let mut bytes = [0_u8; STAGING_PROBE_BYTES.len()];
            reopened
                .read_exact(&mut bytes)
                .map_err(staging_probe_io_error)?;
            let mut trailing = [0_u8; 1];
            if bytes.as_slice() != STAGING_PROBE_BYTES
                || reopened
                    .read(&mut trailing)
                    .map_err(staging_probe_io_error)?
                    != 0
            {
                return Err(staging_permission_error(InstallerError::new(
                    InstallerErrorCode::InternalError,
                )));
            }
            drop(reopened);
            probe
                .remove_artifact_if_present(&probe.final_path(ArtifactKind::Msix))
                .map_err(staging_permission_error)?;
            let volume = self
                .disk_space_probe
                .volume_key(probe.path())
                .map_err(|_| staging_space_unavailable_error())?;
            self.disk_space_probe
                .available_bytes(&volume)
                .map_err(|_| staging_space_unavailable_error())
        })();
        let cleanup = probe.cleanup().map_err(staging_cleanup_error);
        let available_bytes = complete_staging_probe(result, cleanup)?;
        if available_bytes == 0 {
            return Err(
                InstallerError::new(InstallerErrorCode::InsufficientDiskSpace)
                    .with_platform_error_code("staging_insufficient_space"),
            );
        }
        let capability_id = format!("s1:{}", uuid::Uuid::new_v4().simple());
        let response = CodexStagingSelection {
            contract_version: 1,
            capability_id: capability_id.clone(),
            label: "用户选择的下载临时目录".to_owned(),
            available_bytes,
        };
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
            InstallerError::new(InstallerErrorCode::InternalError)
                .with_diagnostic_message("staging capability expiry scheduler is unavailable")
        })?;
        self.staging_selection.insert(capability_id, root, &runtime);
        Ok(response)
    }

    pub async fn prepare_install(
        &self,
        request: PrepareInstallRequest,
    ) -> Result<CodexInstallPreflight, InstallerError> {
        self.staging_selection.evict_expired();
        StartInstallRequest {
            expected_release_id: request.expected_release_id.clone(),
        }
        .validate()?;
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
        let (temp_root, staging_label, custom_staging) =
            self.consume_staging_selection(request.staging_capability_id.as_deref())?;
        let directory = temp_root
            .create_job(&uuid::Uuid::new_v4().to_string())
            .map_err(|error| map_staging_operation_error(error, custom_staging))?;
        let result = async {
            let plan = self.platform.preflight(&release, directory.path()).await?;
            directory
                .revalidate()
                .map_err(|error| map_staging_directory_error(error, custom_staging))?;
            let additional_paths = plan.additional_disk_paths();
            if let Some(size) = release.download_size_hint {
                ensure_preflight_required_space(
                    self.disk_space_probe.as_ref(),
                    directory.path(),
                    additional_paths,
                    size,
                    custom_staging,
                )?;
            }
            let staging_unavailable = || {
                if custom_staging {
                    staging_space_unavailable_error()
                } else {
                    disk_space_unavailable_error()
                }
            };
            let staging_volume = self
                .disk_space_probe
                .volume_key(directory.path())
                .map_err(|_| staging_unavailable())?;
            let mut available = self
                .disk_space_probe
                .available_bytes(&staging_volume)
                .map_err(|_| staging_unavailable())?;
            if available == 0 {
                return Err(map_staging_capacity_error(
                    InstallerError::new(InstallerErrorCode::InsufficientDiskSpace),
                    custom_staging,
                ));
            }
            for path in additional_paths {
                let volume = self
                    .disk_space_probe
                    .volume_key(path)
                    .map_err(|_| disk_space_unavailable_error())?;
                let target_available = self
                    .disk_space_probe
                    .available_bytes(&volume)
                    .map_err(|_| disk_space_unavailable_error())?;
                if target_available == 0 {
                    return Err(InstallerError::new(
                        InstallerErrorCode::InsufficientDiskSpace,
                    ));
                }
                available = available.min(target_available);
            }
            directory
                .revalidate()
                .map_err(|error| map_staging_directory_error(error, custom_staging))?;
            let target = ConfirmedInstallTarget {
                local,
                target_root: plan.confirmation_target().map(Path::to_path_buf),
            };
            let target_label = target_label(&target, release.platform);
            let confirmation_id = format!("i1:{}", uuid::Uuid::new_v4().simple());
            let pending = PendingInstallConfirmation {
                release_id: request.expected_release_id.clone(),
                target,
                temp_root,
            };
            let snapshot = CodexInstallPreflight {
                contract_version: 1,
                confirmation_id,
                expected_release_id: request.expected_release_id,
                platform: release.platform,
                architecture: release.architecture,
                display_version: release.display_version.clone(),
                download_url: release.download_endpoint.url().to_string(),
                target_label,
                staging_label,
                custom_staging,
                updating: matches!(pending.target.local, LocalInstallStatus::Installed { .. }),
                available_bytes: available,
                download_size_hint: release.download_size_hint,
            };
            Ok((snapshot, pending))
        }
        .await;
        directory
            .cleanup()
            .map_err(|error| map_staging_operation_error(error, custom_staging))?;
        let (snapshot, pending) = result?;
        self.install_confirmation.insert(
            snapshot.confirmation_id.clone(),
            pending,
            &tokio::runtime::Handle::current(),
        );
        Ok(snapshot)
    }

    pub fn start_confirmed_install(
        &self,
        request: ConfirmedStartInstallRequest,
    ) -> Result<JobSnapshot, InstallerError> {
        let selected = self
            .install_confirmation
            .take_valid(&request.confirmation_id)
            .filter(|pending| pending.release_id == request.expected_release_id)
            .ok_or_else(changed)?;
        let snapshot = self.start_install_with_target(
            StartInstallRequest {
                expected_release_id: request.expected_release_id,
            },
            Some(selected.target),
            selected.temp_root,
        )?;
        Ok(snapshot)
    }

    pub fn discard_install_confirmation(&self, confirmation_id: &str) {
        self.install_confirmation.discard(confirmation_id);
    }

    fn consume_staging_selection(
        &self,
        capability_id: Option<&str>,
    ) -> Result<(JobTempRoot, String, bool), InstallerError> {
        self.staging_selection.evict_expired();
        let Some(capability_id) = capability_id else {
            return Ok((
                self.temp_root.clone(),
                "FyAgent 安全下载临时目录".to_owned(),
                false,
            ));
        };
        let root = self
            .staging_selection
            .take_valid(capability_id)
            .ok_or_else(changed)?;
        Ok((root, "用户选择的下载临时目录".to_owned(), true))
    }

    #[cfg(test)]
    pub(super) fn backdate_staging_selection(&self, age: Duration) {
        self.staging_selection.backdate(age);
    }

    #[cfg(test)]
    pub(super) fn backdate_install_confirmation(&self, age: Duration) {
        self.install_confirmation.backdate(age);
    }
}

fn ensure_preflight_required_space(
    probe: &dyn DiskSpaceProbe,
    staging_path: &Path,
    additional_paths: &[PathBuf],
    expected_size: u64,
    custom_staging: bool,
) -> Result<(), InstallerError> {
    ensure_required_disk_space(probe, std::iter::once(staging_path), expected_size)
        .map_err(|error| map_staging_required_space_error(error, custom_staging))?;
    if !additional_paths.is_empty() {
        ensure_required_disk_space(
            probe,
            additional_paths.iter().map(PathBuf::as_path),
            expected_size,
        )?;
    }
    Ok(())
}

fn complete_staging_probe(
    result: Result<u64, InstallerError>,
    cleanup: Result<(), InstallerError>,
) -> Result<u64, InstallerError> {
    match (result, cleanup) {
        (Ok(available), Ok(())) => Ok(available),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => {
            log::warn!(
                "selected staging probe and cleanup both failed; primary_code={:?}",
                primary.code()
            );
            Err(cleanup)
        }
    }
}

fn map_staging_required_space_error(error: InstallerError, custom_staging: bool) -> InstallerError {
    if !custom_staging {
        return error;
    }
    match error.code() {
        InstallerErrorCode::InsufficientDiskSpace => {
            error.with_platform_error_code("staging_insufficient_space")
        }
        InstallerErrorCode::InternalError => {
            error.with_platform_error_code("staging_space_unavailable")
        }
        _ => error,
    }
}

fn disk_space_unavailable_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError)
        .with_platform_error_code("disk_space_unavailable")
}

fn capability_expired(created_at: Instant) -> bool {
    created_at.elapsed() >= INSTALL_CAPABILITY_TTL
}

#[cfg(target_os = "windows")]
fn staging_probe_io_error(error: std::io::Error) -> InstallerError {
    if matches!(error.raw_os_error(), Some(39 | 112 | 1816)) {
        InstallerError::new(InstallerErrorCode::InsufficientDiskSpace)
            .with_platform_error_code("staging_insufficient_space")
    } else {
        staging_permission_error(InstallerError::new(InstallerErrorCode::InternalError))
    }
}

fn map_staging_capacity_error(error: InstallerError, custom_staging: bool) -> InstallerError {
    if custom_staging && error.code() == InstallerErrorCode::InsufficientDiskSpace {
        error.with_platform_error_code("staging_insufficient_space")
    } else {
        error
    }
}

fn map_staging_operation_error(error: InstallerError, custom_staging: bool) -> InstallerError {
    if custom_staging {
        staging_permission_error(error)
    } else {
        error
    }
}

fn map_staging_directory_error(error: InstallerError, custom_staging: bool) -> InstallerError {
    if custom_staging {
        staging_directory_unavailable_error(error)
    } else {
        error
    }
}

fn staging_permission_error(cause: InstallerError) -> InstallerError {
    cause
        .with_platform_error_code("staging_permission_denied")
        .with_diagnostic_message("the selected staging directory rejected artifact operations")
}

fn staging_cleanup_error(cause: InstallerError) -> InstallerError {
    cause
        .with_platform_error_code("staging_cleanup_failed")
        .with_diagnostic_message("the selected staging probe could not be removed safely")
}

fn staging_directory_unavailable_error(cause: InstallerError) -> InstallerError {
    cause
        .with_platform_error_code("staging_directory_unavailable")
        .with_diagnostic_message("the selected staging directory failed safe identity admission")
}

fn staging_space_unavailable_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError)
        .with_platform_error_code("staging_space_unavailable")
        .with_diagnostic_message("the selected staging volume capacity is unavailable")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_desktop::verify::{DiskSpaceProbeError, VolumeKey};

    struct RoleDiskProbe {
        staging: VolumeKey,
        target: VolumeKey,
    }

    impl RoleDiskProbe {
        fn new() -> Self {
            Self {
                staging: VolumeKey::new("staging").unwrap(),
                target: VolumeKey::new("target").unwrap(),
            }
        }
    }

    impl DiskSpaceProbe for RoleDiskProbe {
        fn volume_key(&self, path: &Path) -> Result<VolumeKey, DiskSpaceProbeError> {
            if path == Path::new("staging") {
                Ok(self.staging.clone())
            } else {
                Ok(self.target.clone())
            }
        }

        fn available_bytes(&self, volume: &VolumeKey) -> Result<u64, DiskSpaceProbeError> {
            Ok(if volume == &self.staging { 1_000 } else { 1 })
        }
    }

    #[test]
    fn ancestor_and_child_admission_failures_keep_distinct_staging_codes() {
        let ancestor = staging_directory_unavailable_error(InstallerError::new(
            InstallerErrorCode::InternalError,
        ));
        let child =
            staging_permission_error(InstallerError::new(InstallerErrorCode::InternalError));

        assert_eq!(
            ancestor.to_dto().details.platform_error_code.as_deref(),
            Some("staging_directory_unavailable")
        );
        assert_eq!(
            child.to_dto().details.platform_error_code.as_deref(),
            Some("staging_permission_denied")
        );
    }

    #[test]
    fn target_shortage_is_not_reported_as_custom_staging_shortage() {
        let error = ensure_preflight_required_space(
            &RoleDiskProbe::new(),
            Path::new("staging"),
            &[PathBuf::from("programdata")],
            100,
            true,
        )
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InsufficientDiskSpace);
        assert_eq!(error.to_dto().details.platform_error_code, None);
    }

    #[test]
    fn cleanup_failure_wins_when_probe_and_cleanup_both_fail() {
        let error = complete_staging_probe(
            Err(InstallerError::new(InstallerErrorCode::DownloadFailed)),
            Err(staging_cleanup_error(InstallerError::new(
                InstallerErrorCode::InternalError,
            ))),
        )
        .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert_eq!(
            error.to_dto().details.platform_error_code.as_deref(),
            Some("staging_cleanup_failed")
        );
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
