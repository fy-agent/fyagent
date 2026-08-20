//! Capability for a single, job-owned installer staging directory.
//!
//! macOS retains the existing system-temporary root. Production
//! Windows builds instead freeze the running FyAgent executable and stage only
//! below its sibling `cache/codex-installer` hierarchy. Callers receive this
//! capability instead of a caller-provided path, so downloads can use only
//! fixed artifact names under one canonical UUID direct child.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[cfg(any(target_os = "macos", test))]
use std::io::ErrorKind;
#[cfg(target_os = "windows")]
use std::path::Component;
#[cfg(target_os = "windows")]
use std::sync::{Arc, OnceLock};

#[cfg(target_os = "windows")]
use fyagent_user_helper::{
    derive_install_layout,
    layout::{CACHE_DIRECTORY, CODEX_INSTALLER_DIRECTORY, INSTALLER_FILE_NAME},
    CanonicalJobId,
};
use uuid::Uuid;

use super::{
    error::{InstallerError, InstallerErrorCode},
    verify::ArtifactKind,
};

#[cfg(target_os = "macos")]
const TEMP_ROOT_DIRECTORY_NAME: &str = "fyagent-codex-installer";
const STALE_JOB_DIRECTORY_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[cfg(target_os = "windows")]
static CURRENT_EXECUTABLE_INSTALL_ROOT: OnceLock<Result<FrozenInstallRoot, InstallerError>> =
    OnceLock::new();

/// Selects the product-owned staging root without making service construction
/// perform filesystem work. Every service clone shares the process-wide
/// frozen Windows executable/install-root identity above.
#[derive(Clone)]
pub(crate) enum JobTempRoot {
    #[cfg(any(target_os = "macos", test))]
    Explicit(PathBuf),
    #[cfg(target_os = "windows")]
    CurrentExecutableInstallRoot,
}

impl fmt::Debug for JobTempRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JobTempRoot(<redacted>)")
    }
}

#[cfg(any(target_os = "macos", test))]
impl From<PathBuf> for JobTempRoot {
    fn from(root: PathBuf) -> Self {
        Self::Explicit(root)
    }
}

impl JobTempRoot {
    /// Production Windows always uses the frozen running executable. Other
    /// hosts preserve the existing system-temporary staging behavior.
    pub(crate) fn for_current_process() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::CurrentExecutableInstallRoot
        }
        #[cfg(target_os = "macos")]
        {
            Self::Explicit(JobTempDir::system_root())
        }
    }

    pub(crate) fn create_job(&self, job_id: &str) -> Result<JobTempDir, InstallerError> {
        match self {
            #[cfg(any(target_os = "macos", test))]
            Self::Explicit(root) => JobTempDir::create(root, job_id),
            #[cfg(target_os = "windows")]
            Self::CurrentExecutableInstallRoot => create_current_executable_job(job_id),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FilesystemIdentity {
    volume_or_device: u64,
    file_index: u64,
}

#[derive(Clone)]
struct TrustedDirectory {
    path: PathBuf,
    identity: FilesystemIdentity,
    #[cfg(target_os = "windows")]
    handle: Arc<fs::File>,
    #[cfg(target_os = "windows")]
    handle_has_delete_access: bool,
}

impl TrustedDirectory {
    fn capture(path: &Path) -> Result<Self, InstallerError> {
        #[cfg(target_os = "windows")]
        {
            let handle = open_windows_directory_path(path)?;
            Self::capture_windows_handle(path, handle, false)
        }
        #[cfg(target_os = "macos")]
        {
            let identity = filesystem_identity(path, FilesystemObjectKind::Directory)?;
            Self::capture_with_identity(path, identity)
        }
    }

    #[cfg(target_os = "macos")]
    fn capture_with_modified(path: &Path) -> Result<(Self, SystemTime), InstallerError> {
        let (identity, modified) = directory_identity_and_modified(path)?;
        let directory = Self::capture_with_identity(path, identity)?;
        Ok((directory, modified))
    }

    #[cfg(target_os = "macos")]
    fn capture_with_identity(
        path: &Path,
        identity: FilesystemIdentity,
    ) -> Result<Self, InstallerError> {
        let canonical_path = fs::canonicalize(path)
            .map_err(|_| temp_error("installer staging directory could not be canonicalized"))?;
        let canonical_identity =
            filesystem_identity(&canonical_path, FilesystemObjectKind::Directory)?;
        if canonical_identity != identity {
            return Err(temp_error(
                "installer staging directory identity changed during inspection",
            ));
        }
        Ok(Self {
            path: canonical_path,
            identity,
        })
    }

    #[cfg(target_os = "windows")]
    fn capture_windows_handle(
        path: &Path,
        handle: fs::File,
        handle_has_delete_access: bool,
    ) -> Result<Self, InstallerError> {
        let identity = windows_filesystem_identity(&handle, FilesystemObjectKind::Directory)?;
        let canonical_path =
            validate_windows_directory_path_identity(path, identity, handle_has_delete_access)?;
        Ok(Self {
            path: canonical_path,
            identity,
            handle: Arc::new(handle),
            handle_has_delete_access,
        })
    }

    #[cfg(target_os = "windows")]
    fn handle(&self) -> &fs::File {
        self.handle.as_ref()
    }

    fn revalidate(&self) -> Result<(), InstallerError> {
        #[cfg(target_os = "windows")]
        {
            if windows_filesystem_identity(self.handle(), FilesystemObjectKind::Directory)?
                != self.identity
            {
                return Err(temp_error(
                    "installer staging directory handle identity no longer matches",
                ));
            }
            let canonical_path = validate_windows_directory_path_identity(
                &self.path,
                self.identity,
                self.handle_has_delete_access,
            )?;
            if canonical_path != self.path {
                return Err(temp_error(
                    "installer staging directory path no longer matches its handle",
                ));
            }
            Ok(())
        }
        #[cfg(target_os = "macos")]
        {
            let current = Self::capture(&self.path)?;
            if current.path != self.path || current.identity != self.identity {
                return Err(temp_error(
                    "installer staging directory identity no longer matches",
                ));
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct TrustedExecutable {
    source_path: PathBuf,
    canonical_path: PathBuf,
    identity: FilesystemIdentity,
}

#[cfg(target_os = "windows")]
impl TrustedExecutable {
    fn capture(source_path: PathBuf) -> Result<Self, InstallerError> {
        validate_executable_path_shape(&source_path)?;
        let identity = filesystem_identity(&source_path, FilesystemObjectKind::RegularFile)?;
        let canonical_path = fs::canonicalize(&source_path)
            .map_err(|_| temp_error("FyAgent executable path could not be canonicalized"))?;
        let canonical_identity =
            filesystem_identity(&canonical_path, FilesystemObjectKind::RegularFile)?;
        if canonical_identity != identity {
            return Err(temp_error(
                "FyAgent executable identity changed during staging-root resolution",
            ));
        }
        Ok(Self {
            source_path,
            canonical_path,
            identity,
        })
    }

    fn revalidate(&self) -> Result<(), InstallerError> {
        let current_path = std::env::current_exe()
            .map_err(|_| temp_error("FyAgent executable path could not be resolved"))?;
        if current_path != self.source_path {
            return Err(temp_error(
                "FyAgent executable path changed after the install root was frozen",
            ));
        }
        let current = Self::capture(current_path)?;
        if current.canonical_path != self.canonical_path || current.identity != self.identity {
            return Err(temp_error(
                "FyAgent executable identity changed after the install root was frozen",
            ));
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
struct FrozenInstallRoot {
    executable: TrustedExecutable,
    install_root: TrustedDirectory,
}

#[cfg(target_os = "windows")]
impl FrozenInstallRoot {
    fn capture_current() -> Result<Self, InstallerError> {
        let executable_path = std::env::current_exe()
            .map_err(|_| temp_error("FyAgent executable path could not be resolved"))?;
        Self::capture(executable_path)
    }

    fn capture(executable_path: PathBuf) -> Result<Self, InstallerError> {
        let executable = TrustedExecutable::capture(executable_path)?;
        let install_root_path = executable
            .source_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| temp_error("FyAgent executable has no installation root"))?;
        let install_root = TrustedDirectory::capture(install_root_path)?;
        if executable.canonical_path.parent() != Some(install_root.path.as_path()) {
            return Err(temp_error(
                "FyAgent executable escaped its frozen installation root",
            ));
        }
        Ok(Self {
            executable,
            install_root,
        })
    }

    fn revalidate(&self) -> Result<(), InstallerError> {
        self.executable.revalidate()?;
        self.install_root.revalidate()?;
        if self.executable.canonical_path.parent() != Some(self.install_root.path.as_path()) {
            return Err(temp_error(
                "FyAgent executable no longer belongs to its frozen installation root",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct JobTempDir {
    /// Direct directory chain ending at the installer staging root. Production
    /// Windows stores install-root/cache/codex-installer; macOS stores
    /// only their explicit staging root.
    ancestors: Vec<TrustedDirectory>,
    path: TrustedDirectory,
    artifact_policy: ArtifactPolicy,
}

#[derive(Clone, Copy)]
enum ArtifactPolicy {
    #[cfg(any(target_os = "macos", test))]
    CrossPlatform,
    #[cfg(any(target_os = "windows", test))]
    WindowsMsixOnly,
}

impl ArtifactPolicy {
    fn permits_file_name(self, file_name: &str) -> bool {
        match self {
            #[cfg(any(target_os = "macos", test))]
            Self::CrossPlatform => matches!(
                file_name,
                "installer.msix" | "installer.msix.part" | "installer.dmg" | "installer.dmg.part"
            ),
            #[cfg(any(target_os = "windows", test))]
            Self::WindowsMsixOnly => matches!(file_name, "installer.msix" | "installer.msix.part"),
        }
    }

    fn cleanup_kinds(self) -> &'static [ArtifactKind] {
        match self {
            #[cfg(any(target_os = "macos", test))]
            Self::CrossPlatform => &[ArtifactKind::Msix, ArtifactKind::Dmg],
            #[cfg(any(target_os = "windows", test))]
            Self::WindowsMsixOnly => &[ArtifactKind::Msix],
        }
    }
}

impl fmt::Debug for JobTempDir {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JobTempDir(<redacted>)")
    }
}

impl JobTempDir {
    #[cfg(target_os = "macos")]
    pub(crate) fn system_root() -> PathBuf {
        std::env::temp_dir().join(TEMP_ROOT_DIRECTORY_NAME)
    }

    /// Removes only stale canonical UUID children from the production root.
    /// Windows never consults the system temporary directory here.
    pub(crate) fn cleanup_stale_system_root() -> Result<usize, InstallerError> {
        #[cfg(target_os = "windows")]
        {
            let Some(ancestors) = open_current_executable_staging_root()? else {
                return Ok(0);
            };
            Self::cleanup_stale_with_ancestors(
                ancestors,
                ArtifactPolicy::WindowsMsixOnly,
                STALE_JOB_DIRECTORY_AGE,
                SystemTime::now(),
            )
        }
        #[cfg(target_os = "macos")]
        {
            Self::cleanup_stale_under(
                &Self::system_root(),
                STALE_JOB_DIRECTORY_AGE,
                SystemTime::now(),
            )
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn cleanup_stale_under(
        root: &Path,
        minimum_age: Duration,
        now: SystemTime,
    ) -> Result<usize, InstallerError> {
        match fs::symlink_metadata(root) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
            Err(_) => {
                return Err(temp_error(
                    "installer staging root could not be inspected for stale cleanup",
                ))
            }
        }
        let root = TrustedDirectory::capture(root)?;
        Self::cleanup_stale_with_ancestors(
            vec![root],
            ArtifactPolicy::CrossPlatform,
            minimum_age,
            now,
        )
    }

    fn cleanup_stale_with_ancestors(
        ancestors: Vec<TrustedDirectory>,
        artifact_policy: ArtifactPolicy,
        minimum_age: Duration,
        now: SystemTime,
    ) -> Result<usize, InstallerError> {
        Self::cleanup_stale_with_ancestors_after_capture(
            ancestors,
            artifact_policy,
            minimum_age,
            now,
            |_| {},
        )
    }

    fn cleanup_stale_with_ancestors_after_capture(
        ancestors: Vec<TrustedDirectory>,
        artifact_policy: ArtifactPolicy,
        minimum_age: Duration,
        now: SystemTime,
        mut after_capture: impl FnMut(&Path),
    ) -> Result<usize, InstallerError> {
        validate_directory_chain(&ancestors)?;
        let root = ancestors
            .last()
            .ok_or_else(|| temp_error("installer staging root chain is empty"))?;
        // This path enumeration supplies candidate names only. Windows must
        // capture each candidate again relative to the held root handle before
        // trusting its time, identity, contents, or delete capability.
        let entries = fs::read_dir(&root.path).map_err(|_| {
            temp_error("installer staging root could not be enumerated for stale cleanup")
        })?;
        let mut removed = 0;

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(job_id) = file_name.to_str() else {
                continue;
            };
            if !is_canonical_job_id(job_id) {
                continue;
            }

            let candidate = entry.path();
            #[cfg(target_os = "windows")]
            let (job_directory, modified) = match windows_capture_relative_directory(
                root,
                job_id,
                WindowsRelativeDisposition::Open,
                true,
            ) {
                Ok(directory) => {
                    let Ok(modified) = directory
                        .handle()
                        .metadata()
                        .and_then(|metadata| metadata.modified())
                    else {
                        continue;
                    };
                    (directory, modified)
                }
                Err(_) => continue,
            };
            #[cfg(target_os = "macos")]
            let (job_directory, modified) =
                match TrustedDirectory::capture_with_modified(&candidate) {
                    Ok((directory, modified))
                        if directory.path.parent() == Some(root.path.as_path()) =>
                    {
                        (directory, modified)
                    }
                    _ => continue,
                };
            after_capture(&candidate);
            if job_directory.revalidate().is_err() {
                continue;
            }
            if !is_stale(modified, now, minimum_age) {
                continue;
            }

            let directory = Self {
                ancestors: ancestors.clone(),
                path: job_directory,
                artifact_policy,
            };
            if directory.revalidate().is_ok() && directory.cleanup().is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Create exactly one canonical UUID direct child. Existing children are
    /// rejected instead of being opened or reused, which prevents a prepared
    /// symlink/reparse point from becoming a download destination.
    #[cfg(any(target_os = "macos", test))]
    pub(crate) fn create(root: &Path, job_id: &str) -> Result<Self, InstallerError> {
        let canonical_job_id = canonical_job_id(job_id)?;
        let root = ensure_root_directory(root)?;
        Self::create_with_ancestors(vec![root], &canonical_job_id, ArtifactPolicy::CrossPlatform)
    }

    fn create_with_ancestors(
        ancestors: Vec<TrustedDirectory>,
        canonical_job_id: &str,
        artifact_policy: ArtifactPolicy,
    ) -> Result<Self, InstallerError> {
        validate_directory_chain(&ancestors)?;
        let root = ancestors
            .last()
            .ok_or_else(|| temp_error("installer staging root chain is empty"))?;
        #[cfg(target_os = "windows")]
        let path = match windows_capture_relative_directory(
            root,
            canonical_job_id,
            WindowsRelativeDisposition::Create,
            true,
        ) {
            Ok(path) => path,
            Err(WindowsRelativeOpenError::AlreadyExists) => {
                return Err(temp_error("installer job staging directory already exists"));
            }
            Err(_) => {
                return Err(temp_error(
                    "installer job staging directory could not be created safely",
                ));
            }
        };
        #[cfg(target_os = "macos")]
        let path = {
            let candidate = root.path.join(canonical_job_id);
            match fs::create_dir(&candidate) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    return Err(temp_error("installer job staging directory already exists"));
                }
                Err(_) => {
                    return Err(temp_error(
                        "installer job staging directory could not be created",
                    ));
                }
            }
            match TrustedDirectory::capture(&candidate) {
                Ok(path) if path.path.parent() == Some(root.path.as_path()) => path,
                _ => {
                    let _ = fs::remove_dir(&candidate);
                    return Err(temp_error(
                        "installer job staging directory escaped its trusted root",
                    ));
                }
            }
        };
        validate_directory_chain(&ancestors)?;

        Ok(Self {
            ancestors,
            path,
            artifact_policy,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path.path
    }

    pub(crate) fn part_path(&self, kind: ArtifactKind) -> PathBuf {
        self.path.path.join(kind.fixed_part_file_name())
    }

    pub(crate) fn final_path(&self, kind: ArtifactKind) -> PathBuf {
        self.path.path.join(kind.fixed_local_file_name())
    }

    pub(crate) fn create_part_file(&self, kind: ArtifactKind) -> Result<fs::File, InstallerError> {
        let path = self.part_path(kind);
        self.validate_artifact_path(&path)?;
        #[cfg(target_os = "windows")]
        {
            windows_open_relative_regular_file(
                &self.path,
                kind.fixed_part_file_name(),
                WindowsRelativeFileAccess::CreateForDownload,
            )
            .map_err(|_| temp_error("installer partial file could not be created safely"))
        }
        #[cfg(target_os = "macos")]
        {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|_| temp_error("installer partial file could not be created"))
        }
    }

    pub(crate) fn ensure_final_artifact_absent(
        &self,
        kind: ArtifactKind,
    ) -> Result<(), InstallerError> {
        let path = self.final_path(kind);
        self.validate_artifact_path(&path)?;
        #[cfg(target_os = "windows")]
        {
            match windows_open_relative_regular_file(
                &self.path,
                kind.fixed_local_file_name(),
                WindowsRelativeFileAccess::OpenForRead,
            ) {
                Err(WindowsRelativeOpenError::NotFound) => Ok(()),
                Ok(_) | Err(_) => Err(temp_error(
                    "installer final file unexpectedly already exists or is unsafe",
                )),
            }
        }
        #[cfg(target_os = "macos")]
        {
            ensure_path_absent_for_temp(&path)
        }
    }

    pub(crate) fn finalize_part_file(
        &self,
        kind: ArtifactKind,
        file: fs::File,
    ) -> Result<(), InstallerError> {
        let part_path = self.part_path(kind);
        let final_path = self.final_path(kind);
        self.validate_artifact_path(&part_path)?;
        self.validate_artifact_path(&final_path)?;
        #[cfg(target_os = "windows")]
        {
            let before = windows_filesystem_identity(&file, FilesystemObjectKind::RegularFile)?;
            windows_rename_relative_file(&self.path, &file, kind.fixed_local_file_name())?;
            drop(file);
            let finalized = windows_open_relative_regular_file(
                &self.path,
                kind.fixed_local_file_name(),
                WindowsRelativeFileAccess::OpenForRead,
            )
            .map_err(|_| temp_error("installer final file could not be reopened safely"))?;
            if windows_filesystem_identity(&finalized, FilesystemObjectKind::RegularFile)? != before
            {
                return Err(temp_error(
                    "installer final file identity did not match its partial handle",
                ));
            }
            self.validate_job_directory()
        }
        #[cfg(target_os = "macos")]
        {
            drop(file);
            ensure_path_absent_for_temp(&final_path)?;
            fs::rename(part_path, final_path)
                .map_err(|_| temp_error("installer partial file could not be finalized"))
        }
    }

    pub(crate) fn open_final_artifact_for_read(
        &self,
        kind: ArtifactKind,
    ) -> Result<fs::File, InstallerError> {
        let path = self.final_path(kind);
        self.validate_artifact_path(&path)?;
        #[cfg(target_os = "windows")]
        {
            windows_open_relative_regular_file(
                &self.path,
                kind.fixed_local_file_name(),
                WindowsRelativeFileAccess::OpenForRead,
            )
            .map_err(|_| temp_error("installer final file could not be opened safely"))
        }
        #[cfg(target_os = "macos")]
        {
            self.validate_existing_artifact(&path)?;
            fs::File::open(path).map_err(|_| temp_error("installer final file could not be opened"))
        }
    }

    /// Re-proves the frozen/direct-child directory identities without
    /// exposing them. The service uses this around path-based disk probing so
    /// a transient staging-root replacement cannot silently authorize the
    /// subsequent download.
    pub(crate) fn revalidate(&self) -> Result<(), InstallerError> {
        self.validate_job_directory()
    }

    /// Check a fixed artifact path before a filesystem operation. This catches
    /// directory replacement and ensures no caller can escape its UUID child.
    pub(crate) fn validate_artifact_path(&self, path: &Path) -> Result<(), InstallerError> {
        let parent = path
            .parent()
            .ok_or_else(|| temp_error("artifact path has no parent"))?;
        if parent != self.path.path {
            return Err(temp_error(
                "artifact path is outside its job staging directory",
            ));
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| temp_error("artifact path has no safe file name"))?;
        if !self.artifact_policy.permits_file_name(file_name) {
            return Err(temp_error(
                "artifact path is not a fixed installer file name",
            ));
        }

        self.validate_job_directory()
    }

    pub(crate) fn validate_existing_artifact(&self, path: &Path) -> Result<(), InstallerError> {
        self.validate_artifact_path(path)?;
        #[cfg(target_os = "windows")]
        {
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| temp_error("installer artifact has no safe file name"))?;
            windows_open_relative_regular_file(
                &self.path,
                file_name,
                WindowsRelativeFileAccess::OpenForRead,
            )
            .map(|_| ())
            .map_err(|_| temp_error("installer artifact could not be opened safely"))
        }
        #[cfg(target_os = "macos")]
        {
            if is_link_or_reparse_point(path)? {
                return Err(temp_error(
                    "installer artifact must not be a link or reparse point",
                ));
            }
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| temp_error("installer artifact could not be inspected"))?;
            if !metadata.is_file() {
                return Err(temp_error("installer artifact must be a regular file"));
            }
            Ok(())
        }
    }

    /// Safely removes only files this capability may have created, then the
    /// empty UUID child. It never recursively owns an unknown cache subtree.
    pub(crate) fn cleanup(&self) -> Result<(), InstallerError> {
        self.validate_job_directory()?;

        for &kind in self.artifact_policy.cleanup_kinds() {
            for path in [self.part_path(kind), self.final_path(kind)] {
                self.remove_artifact_if_present(&path)?;
            }
        }

        self.validate_job_directory()?;
        #[cfg(target_os = "windows")]
        {
            if !self.path.handle_has_delete_access {
                return Err(temp_error(
                    "installer job staging directory has no delete capability",
                ));
            }
            windows_mark_handle_for_deletion(self.path.handle())
        }
        #[cfg(target_os = "macos")]
        {
            fs::remove_dir(&self.path.path)
                .map_err(|_| temp_error("installer job staging directory could not be removed"))
        }
    }

    fn validate_job_directory(&self) -> Result<(), InstallerError> {
        validate_directory_chain(&self.ancestors)?;
        self.path.revalidate()?;
        let root = self
            .ancestors
            .last()
            .ok_or_else(|| temp_error("installer staging root chain is empty"))?;
        if self.path.path.parent() != Some(root.path.as_path()) {
            return Err(temp_error(
                "installer job staging directory is no longer a trusted direct child",
            ));
        }
        Ok(())
    }

    pub(crate) fn remove_artifact_if_present(&self, path: &Path) -> Result<(), InstallerError> {
        self.validate_artifact_path(path)?;

        #[cfg(target_os = "windows")]
        {
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| temp_error("installer artifact has no safe file name"))?;
            let file = match windows_open_relative_regular_file(
                &self.path,
                file_name,
                WindowsRelativeFileAccess::OpenForDelete,
            ) {
                Ok(file) => file,
                Err(WindowsRelativeOpenError::NotFound) => return Ok(()),
                Err(_) => {
                    return Err(temp_error(
                        "installer cleanup refused an unsafe artifact entry",
                    ));
                }
            };
            windows_mark_handle_for_deletion(&file)?;
            drop(file);
            self.validate_job_directory()
        }

        #[cfg(target_os = "macos")]
        {
            let metadata = match fs::symlink_metadata(path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
                Err(_) => {
                    return Err(temp_error(
                        "installer artifact could not be inspected for cleanup",
                    ))
                }
            };

            if is_link_or_reparse_point(path)? || !metadata.is_file() {
                return Err(temp_error(
                    "installer cleanup refused a non-regular artifact entry",
                ));
            }

            fs::remove_file(path)
                .map_err(|_| temp_error("installer artifact could not be removed during cleanup"))
        }
    }
}

#[cfg(target_os = "macos")]
fn ensure_path_absent_for_temp(path: &Path) -> Result<(), InstallerError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(temp_error(
            "installer final file unexpectedly already exists",
        )),
        Err(_) => Err(temp_error("installer final file could not be inspected")),
    }
}

#[cfg(any(target_os = "macos", test))]
fn canonical_job_id(value: &str) -> Result<String, InstallerError> {
    if !is_canonical_job_id(value) {
        return Err(temp_error("installer job ID is not a canonical UUID"));
    }
    Ok(value.to_owned())
}

fn is_canonical_job_id(value: &str) -> bool {
    Uuid::parse_str(value)
        .map(|parsed| parsed.hyphenated().to_string() == value)
        .unwrap_or(false)
}

fn is_stale(modified: SystemTime, now: SystemTime, minimum_age: Duration) -> bool {
    now.duration_since(modified)
        .map(|age| age >= minimum_age)
        .unwrap_or(false)
}

#[cfg(any(target_os = "macos", test))]
fn ensure_root_directory(root: &Path) -> Result<TrustedDirectory, InstallerError> {
    match fs::symlink_metadata(root) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {
            fs::create_dir_all(root)
                .map_err(|_| temp_error("installer staging root could not be created"))?;
        }
        Err(_) => return Err(temp_error("installer staging root could not be inspected")),
    }
    TrustedDirectory::capture(root)
}

fn validate_directory_chain(ancestors: &[TrustedDirectory]) -> Result<(), InstallerError> {
    if ancestors.is_empty() {
        return Err(temp_error("installer staging root chain is empty"));
    }
    for directory in ancestors {
        directory.revalidate()?;
    }
    for pair in ancestors.windows(2) {
        if pair[1].path.parent() != Some(pair[0].path.as_path()) {
            return Err(temp_error(
                "installer staging directory is not a trusted direct child",
            ));
        }
    }
    Ok(())
}

#[cfg(all(target_os = "macos", test))]
fn open_direct_child_directory(
    parent: &TrustedDirectory,
    name: &str,
) -> Result<Option<TrustedDirectory>, InstallerError> {
    parent.revalidate()?;
    let candidate = parent.path.join(name);
    match fs::symlink_metadata(&candidate) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(temp_error("installer staging child could not be inspected")),
    }
    let child = TrustedDirectory::capture(&candidate)?;
    parent.revalidate()?;
    if child.path.parent() != Some(parent.path.as_path()) {
        return Err(temp_error(
            "installer staging child escaped its trusted parent",
        ));
    }
    Ok(Some(child))
}

#[cfg(target_os = "windows")]
fn open_direct_child_directory(
    parent: &TrustedDirectory,
    name: &str,
) -> Result<Option<TrustedDirectory>, InstallerError> {
    match windows_capture_relative_directory(parent, name, WindowsRelativeDisposition::Open, false)
    {
        Ok(directory) => Ok(Some(directory)),
        Err(WindowsRelativeOpenError::NotFound) => Ok(None),
        Err(_) => Err(temp_error(
            "installer staging child could not be opened safely",
        )),
    }
}

#[cfg(target_os = "windows")]
fn ensure_direct_child_directory(
    parent: &TrustedDirectory,
    name: &str,
) -> Result<TrustedDirectory, InstallerError> {
    match windows_capture_relative_directory(
        parent,
        name,
        WindowsRelativeDisposition::OpenIf,
        false,
    ) {
        Ok(directory) => Ok(directory),
        Err(_) => Err(temp_error(
            "installer staging child could not be opened or created safely",
        )),
    }
}

#[cfg(target_os = "windows")]
fn frozen_current_install_root() -> Result<&'static FrozenInstallRoot, InstallerError> {
    match CURRENT_EXECUTABLE_INSTALL_ROOT.get_or_init(FrozenInstallRoot::capture_current) {
        Ok(root) => Ok(root),
        Err(error) => Err(error.clone()),
    }
}

#[cfg(target_os = "windows")]
fn ensure_current_executable_staging_root() -> Result<Vec<TrustedDirectory>, InstallerError> {
    let frozen = frozen_current_install_root()?;
    frozen.revalidate()?;
    let cache = ensure_direct_child_directory(&frozen.install_root, CACHE_DIRECTORY)?;
    let installer = ensure_direct_child_directory(&cache, CODEX_INSTALLER_DIRECTORY)?;
    let ancestors = vec![frozen.install_root.clone(), cache, installer];
    validate_directory_chain(&ancestors)?;
    Ok(ancestors)
}

#[cfg(target_os = "windows")]
fn open_current_executable_staging_root() -> Result<Option<Vec<TrustedDirectory>>, InstallerError> {
    let frozen = frozen_current_install_root()?;
    frozen.revalidate()?;
    let Some(cache) = open_direct_child_directory(&frozen.install_root, CACHE_DIRECTORY)? else {
        return Ok(None);
    };
    let Some(installer) = open_direct_child_directory(&cache, CODEX_INSTALLER_DIRECTORY)? else {
        return Ok(None);
    };
    let ancestors = vec![frozen.install_root.clone(), cache, installer];
    validate_directory_chain(&ancestors)?;
    Ok(Some(ancestors))
}

#[cfg(target_os = "windows")]
fn create_current_executable_job(job_id: &str) -> Result<JobTempDir, InstallerError> {
    let canonical_job_id = CanonicalJobId::parse(job_id)
        .map_err(|_| temp_error("installer job ID is not a canonical UUID"))?;
    let frozen = frozen_current_install_root()?;
    frozen.revalidate()?;

    // Reuse the helper's own executable-relative layout parser so both
    // processes remain locked to the same fixed four-component path.
    let helper_layout = derive_install_layout(&frozen.executable.source_path, &canonical_job_id)
        .map_err(|_| temp_error("FyAgent install-root layout could not be derived"))?;
    if helper_layout.install_root()
        != frozen
            .executable
            .source_path
            .parent()
            .expect("validated executable has an installation root")
    {
        return Err(temp_error(
            "FyAgent helper layout disagrees with the frozen installation root",
        ));
    }

    let directory = JobTempDir::create_with_ancestors(
        ensure_current_executable_staging_root()?,
        canonical_job_id.as_str(),
        ArtifactPolicy::WindowsMsixOnly,
    )?;
    let relative_installer = directory
        .final_path(ArtifactKind::Msix)
        .strip_prefix(&frozen.install_root.path)
        .map(Path::to_path_buf)
        .map_err(|_| temp_error("installer staging path escaped the frozen install root"))?;
    let helper_relative = helper_layout
        .installer_path()
        .strip_prefix(helper_layout.install_root())
        .map(Path::to_path_buf)
        .map_err(|_| temp_error("helper installer path escaped its install root"))?;
    if relative_installer != helper_relative
        || directory
            .final_path(ArtifactKind::Msix)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(INSTALLER_FILE_NAME)
    {
        let _ = directory.cleanup();
        return Err(temp_error(
            "main and helper installer staging layouts do not match",
        ));
    }
    Ok(directory)
}

#[cfg(target_os = "windows")]
fn validate_executable_path_shape(path: &Path) -> Result<(), InstallerError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        || path.file_name().is_none()
        || path
            .parent()
            .is_none_or(|parent| parent.as_os_str().is_empty())
    {
        return Err(temp_error(
            "FyAgent executable path cannot define a fixed installation root",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum FilesystemObjectKind {
    Directory,
    #[cfg(target_os = "windows")]
    RegularFile,
}

#[cfg(target_os = "windows")]
fn open_windows_filesystem_object(
    path: &Path,
    expected_kind: FilesystemObjectKind,
) -> Result<fs::File, InstallerError> {
    use std::os::windows::fs::OpenOptionsExt;

    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
    };

    let flags = FILE_FLAG_OPEN_REPARSE_POINT
        | match expected_kind {
            FilesystemObjectKind::Directory => FILE_FLAG_BACKUP_SEMANTICS,
            FilesystemObjectKind::RegularFile => Default::default(),
        };
    let mut options = fs::OpenOptions::new();
    match expected_kind {
        FilesystemObjectKind::Directory => {
            options
                .access_mode((FILE_READ_ATTRIBUTES | FILE_TRAVERSE).0)
                .share_mode(FILE_SHARE_READ.0);
        }
        FilesystemObjectKind::RegularFile => {
            options
                .access_mode(FILE_READ_ATTRIBUTES.0)
                .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0);
        }
    }
    options.custom_flags(flags.0);
    options
        .open(path)
        .map_err(|_| temp_error("installer staging object could not be opened without follow"))
}

#[cfg(target_os = "windows")]
fn open_windows_directory_path(path: &Path) -> Result<fs::File, InstallerError> {
    open_windows_filesystem_object(path, FilesystemObjectKind::Directory)
}

#[cfg(target_os = "windows")]
fn open_windows_directory_path_for_validation(
    path: &Path,
    share_delete: bool,
) -> Result<fs::File, InstallerError> {
    if !share_delete {
        return open_windows_directory_path(path);
    }

    use std::os::windows::fs::OpenOptionsExt;

    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_TRAVERSE,
    };

    let mut options = fs::OpenOptions::new();
    options
        .access_mode((FILE_READ_ATTRIBUTES | FILE_TRAVERSE).0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0);
    options
        .open(path)
        .map_err(|_| temp_error("installer staging path could not be reopened for validation"))
}

#[cfg(target_os = "windows")]
fn validate_windows_directory_path_identity(
    path: &Path,
    expected_identity: FilesystemIdentity,
    share_delete: bool,
) -> Result<PathBuf, InstallerError> {
    let canonical_path = fs::canonicalize(path)
        .map_err(|_| temp_error("installer staging directory could not be canonicalized"))?;
    let file = open_windows_directory_path_for_validation(&canonical_path, share_delete)?;
    if windows_filesystem_identity(&file, FilesystemObjectKind::Directory)? != expected_identity {
        return Err(temp_error(
            "installer staging directory identity changed during inspection",
        ));
    }
    Ok(canonical_path)
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowsRelativeDisposition {
    Open,
    Create,
    OpenIf,
}

#[cfg(target_os = "windows")]
enum WindowsRelativeOpenError {
    NotFound,
    AlreadyExists,
    Rejected,
}

#[cfg(target_os = "windows")]
fn windows_open_relative(
    root: &fs::File,
    name: &str,
    desired_access: u32,
    share_access: u32,
    disposition: WindowsRelativeDisposition,
    file_attributes: u32,
    create_options: u32,
) -> Result<fs::File, WindowsRelativeOpenError> {
    use std::{
        ffi::OsStr,
        mem::size_of,
        os::windows::{ffi::OsStrExt, io::AsRawHandle, io::FromRawHandle},
    };

    use windows::{
        core::PWSTR,
        Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{
                NtCreateFile, FILE_CREATE, FILE_OPEN, FILE_OPEN_IF,
                NTCREATEFILE_CREATE_DISPOSITION, NTCREATEFILE_CREATE_OPTIONS,
            },
        },
        Win32::{
            Foundation::{
                HANDLE, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, STATUS_NO_SUCH_FILE,
                STATUS_OBJECT_NAME_COLLISION, STATUS_OBJECT_NAME_NOT_FOUND,
                STATUS_OBJECT_PATH_NOT_FOUND, UNICODE_STRING,
            },
            Storage::FileSystem::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE},
            System::IO::IO_STATUS_BLOCK,
        },
    };

    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(component)) if component == OsStr::new(name))
        || components.next().is_some()
    {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    let mut wide = OsStr::new(name).encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    let byte_length = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or(WindowsRelativeOpenError::Rejected)?;
    let object_name = UNICODE_STRING {
        Length: byte_length,
        MaximumLength: byte_length,
        Buffer: PWSTR(wide.as_mut_ptr()),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(root.as_raw_handle()),
        ObjectName: &object_name,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let create_disposition: NTCREATEFILE_CREATE_DISPOSITION = match disposition {
        WindowsRelativeDisposition::Open => FILE_OPEN,
        WindowsRelativeDisposition::Create => FILE_CREATE,
        WindowsRelativeDisposition::OpenIf => FILE_OPEN_IF,
    };
    let mut handle = HANDLE::default();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_ACCESS_RIGHTS(desired_access),
            &object_attributes,
            &mut io_status,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(file_attributes),
            FILE_SHARE_MODE(share_access),
            create_disposition,
            NTCREATEFILE_CREATE_OPTIONS(create_options),
            None,
            0,
        )
    };
    if status.is_err() {
        return if matches!(
            status,
            STATUS_NO_SUCH_FILE | STATUS_OBJECT_NAME_NOT_FOUND | STATUS_OBJECT_PATH_NOT_FOUND
        ) {
            Err(WindowsRelativeOpenError::NotFound)
        } else if status == STATUS_OBJECT_NAME_COLLISION {
            Err(WindowsRelativeOpenError::AlreadyExists)
        } else {
            Err(WindowsRelativeOpenError::Rejected)
        };
    }
    if handle.0.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    Ok(unsafe { fs::File::from_raw_handle(handle.0) })
}

#[cfg(target_os = "windows")]
fn windows_capture_relative_directory(
    parent: &TrustedDirectory,
    name: &str,
    disposition: WindowsRelativeDisposition,
    delete_access: bool,
) -> Result<TrustedDirectory, WindowsRelativeOpenError> {
    use windows::{
        Wdk::Storage::FileSystem::{
            FILE_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
        Win32::Storage::FileSystem::{
            DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_TRAVERSE,
            SYNCHRONIZE,
        },
    };

    parent
        .revalidate()
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    let mut desired_access = (FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE).0;
    if delete_access {
        desired_access |= DELETE.0;
    }
    let handle = windows_open_relative(
        parent.handle(),
        name,
        desired_access,
        FILE_SHARE_READ.0,
        disposition,
        FILE_ATTRIBUTE_DIRECTORY.0,
        (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
    )?;
    let path = parent.path.join(name);
    let directory = TrustedDirectory::capture_windows_handle(&path, handle, delete_access)
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    parent
        .revalidate()
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    if directory.path.parent() != Some(parent.path.as_path()) {
        return Err(WindowsRelativeOpenError::Rejected);
    }
    Ok(directory)
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowsRelativeFileAccess {
    CreateForDownload,
    OpenForRead,
    OpenForDelete,
}

#[cfg(target_os = "windows")]
fn windows_open_relative_regular_file(
    directory: &TrustedDirectory,
    name: &str,
    access: WindowsRelativeFileAccess,
) -> Result<fs::File, WindowsRelativeOpenError> {
    use windows::{
        Wdk::Storage::FileSystem::{
            FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
        Win32::Storage::FileSystem::{
            DELETE, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            FILE_READ_ATTRIBUTES, FILE_SHARE_READ, SYNCHRONIZE,
        },
    };

    directory
        .revalidate()
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    let (desired_access, share_access, disposition) = match access {
        WindowsRelativeFileAccess::CreateForDownload => (
            (FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE).0,
            0,
            WindowsRelativeDisposition::Create,
        ),
        WindowsRelativeFileAccess::OpenForRead => (
            (FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
            FILE_SHARE_READ.0,
            WindowsRelativeDisposition::Open,
        ),
        WindowsRelativeFileAccess::OpenForDelete => (
            (FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE).0,
            FILE_SHARE_READ.0,
            WindowsRelativeDisposition::Open,
        ),
    };
    let file = windows_open_relative(
        directory.handle(),
        name,
        desired_access,
        share_access,
        disposition,
        FILE_ATTRIBUTE_NORMAL.0,
        (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
    )?;
    windows_filesystem_identity(&file, FilesystemObjectKind::RegularFile)
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    directory
        .revalidate()
        .map_err(|_| WindowsRelativeOpenError::Rejected)?;
    Ok(file)
}

#[cfg(target_os = "windows")]
fn windows_rename_relative_file(
    directory: &TrustedDirectory,
    file: &fs::File,
    final_name: &str,
) -> Result<(), InstallerError> {
    use std::{mem::size_of, os::windows::io::AsRawHandle};

    use windows::{
        Wdk::Storage::FileSystem::{
            FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION,
        },
        Win32::{Foundation::HANDLE, System::IO::IO_STATUS_BLOCK},
    };

    directory.revalidate()?;
    let before = windows_filesystem_identity(file, FilesystemObjectKind::RegularFile)?;
    let wide = final_name.encode_utf16().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(temp_error("installer final file name is invalid"));
    }
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| temp_error("installer final file name is too long"))?;
    // Windows requires the complete inline FILE_RENAME_INFORMATION structure plus
    // the variable file-name bytes, not merely the offset of FileName plus
    // those bytes. The latter omits the inline WCHAR and trailing alignment
    // on both supported Windows architectures and is rejected by the native API.
    let buffer_size = windows_rename_information_buffer_size(name_bytes)
        .ok_or_else(|| temp_error("installer final file rename buffer is too large"))?;
    let word_size = size_of::<usize>();
    let mut storage = vec![0_usize; buffer_size.div_ceil(word_size)];
    let information = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        (*information).Anonymous.ReplaceIfExists = false;
        // The native FileRenameInformation contract treats a simple name with
        // a null root as a same-directory rename on this source handle. This
        // avoids both Win32 current-directory resolution and reopening the
        // pinned directory with incompatible sharing requirements.
        (*information).RootDirectory = windows_same_directory_rename_root();
        (*information).FileNameLength = u32::try_from(name_bytes)
            .map_err(|_| temp_error("installer final file name is too long"))?;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            (*information).FileName.as_mut_ptr(),
            wide.len(),
        );
        NtSetInformationFile(
            HANDLE(file.as_raw_handle()),
            &mut io_status,
            information.cast(),
            u32::try_from(buffer_size)
                .map_err(|_| temp_error("installer final file rename buffer is too large"))?,
            FileRenameInformation,
        )
    };
    if status.is_err() {
        return Err(
            temp_error("installer partial file could not be finalized by handle")
                .with_platform_error_code(format!("NTSTATUS 0x{:08X}", status.0 as u32)),
        );
    }
    if windows_filesystem_identity(file, FilesystemObjectKind::RegularFile)? != before {
        return Err(temp_error(
            "installer file identity changed while it was finalized",
        ));
    }
    directory.revalidate()
}

#[cfg(target_os = "windows")]
fn windows_same_directory_rename_root() -> windows::Win32::Foundation::HANDLE {
    windows::Win32::Foundation::HANDLE::default()
}

#[cfg(target_os = "windows")]
fn windows_rename_information_buffer_size(name_bytes: usize) -> Option<usize> {
    use windows::Wdk::Storage::FileSystem::FILE_RENAME_INFORMATION;

    std::mem::size_of::<FILE_RENAME_INFORMATION>().checked_add(name_bytes)
}

#[cfg(target_os = "windows")]
fn windows_mark_handle_for_deletion(file: &fs::File) -> Result<(), InstallerError> {
    use std::{mem::size_of, os::windows::io::AsRawHandle};

    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
        },
    };

    let information = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&raw const information).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(|_| temp_error("installer staging object could not be deleted by handle"))
}

#[cfg(target_os = "windows")]
fn windows_filesystem_identity(
    file: &fs::File,
    expected_kind: FilesystemObjectKind,
) -> Result<FilesystemIdentity, InstallerError> {
    use std::os::windows::io::AsRawHandle;

    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_REPARSE_POINT,
        },
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| temp_error("installer staging object identity could not be queried"))?;

    let is_directory = information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let is_reparse = information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0;
    if is_reparse
        || match expected_kind {
            FilesystemObjectKind::Directory => !is_directory,
            FilesystemObjectKind::RegularFile => is_directory,
        }
    {
        return Err(temp_error(
            "installer staging object has an unexpected type or reparse point",
        ));
    }

    Ok(FilesystemIdentity {
        volume_or_device: u64::from(information.dwVolumeSerialNumber),
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
    })
}

#[cfg(target_os = "windows")]
fn filesystem_identity(
    path: &Path,
    expected_kind: FilesystemObjectKind,
) -> Result<FilesystemIdentity, InstallerError> {
    let file = open_windows_filesystem_object(path, expected_kind)?;
    windows_filesystem_identity(&file, expected_kind)
}

#[cfg(target_os = "macos")]
fn filesystem_identity(
    path: &Path,
    expected_kind: FilesystemObjectKind,
) -> Result<FilesystemIdentity, InstallerError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer staging object could not be inspected"))?;
    if metadata.file_type().is_symlink()
        || match expected_kind {
            FilesystemObjectKind::Directory => !metadata.is_dir(),
            #[cfg(target_os = "windows")]
            FilesystemObjectKind::RegularFile => !metadata.is_file(),
        }
    {
        return Err(temp_error(
            "installer staging object has an unexpected type or link",
        ));
    }
    Ok(FilesystemIdentity {
        volume_or_device: metadata.dev(),
        file_index: metadata.ino(),
    })
}

#[cfg(target_os = "macos")]
fn directory_identity_and_modified(
    path: &Path,
) -> Result<(FilesystemIdentity, SystemTime), InstallerError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer job staging directory could not be inspected"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(temp_error(
            "installer job staging directory has an unexpected type or link",
        ));
    }
    let modified = metadata
        .modified()
        .map_err(|_| temp_error("installer job staging age could not be queried"))?;
    Ok((
        FilesystemIdentity {
            volume_or_device: metadata.dev(),
            file_index: metadata.ino(),
        },
        modified,
    ))
}

#[cfg(target_os = "macos")]
fn is_link_or_reparse_point(path: &Path) -> Result<bool, InstallerError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| temp_error("installer staging path could not be inspected"))?;
    Ok(metadata.file_type().is_symlink())
}

fn temp_error(message: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::InternalError).with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_rename_buffer_includes_the_complete_inline_structure() {
        use std::mem::{offset_of, size_of};

        use windows::Wdk::Storage::FileSystem::FILE_RENAME_INFORMATION;

        let name_bytes = "installer.msix".encode_utf16().count() * size_of::<u16>();
        let buffer_size = windows_rename_information_buffer_size(name_bytes).unwrap();

        assert_eq!(
            buffer_size,
            size_of::<FILE_RENAME_INFORMATION>() + name_bytes
        );
        assert!(buffer_size > offset_of!(FILE_RENAME_INFORMATION, FileName) + name_bytes);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_native_rename_uses_source_handle_same_directory_semantics() {
        assert!(windows_same_directory_rename_root().0.is_null());
    }

    #[test]
    fn creates_only_a_canonical_uuid_direct_child() {
        let root = tempfile::tempdir().unwrap();
        let job_id = Uuid::new_v4().hyphenated().to_string();

        let job_directory = JobTempDir::create(root.path(), &job_id).unwrap();

        assert_eq!(
            job_directory.path().parent(),
            job_directory
                .ancestors
                .last()
                .map(|root| root.path.as_path())
        );
        assert_eq!(
            job_directory.part_path(ArtifactKind::Msix).file_name(),
            Some(std::ffi::OsStr::new("installer.msix.part"))
        );
        assert!(JobTempDir::create(root.path(), &job_id).is_err());
        assert!(JobTempDir::create(root.path(), "not-a-uuid").is_err());
        assert!(JobTempDir::create(root.path(), &job_id.to_ascii_uppercase()).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_a_symlink_root_before_creating_a_job_child() {
        use std::os::unix::fs::symlink;

        let container = tempfile::tempdir().unwrap();
        let real_root = container.path().join("real-root");
        fs::create_dir(&real_root).unwrap();
        let link_root = container.path().join("link-root");
        symlink(&real_root, &link_root).unwrap();

        let error =
            JobTempDir::create(&link_root, &Uuid::new_v4().hyphenated().to_string()).unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert_eq!(fs::read_dir(&real_root).unwrap().count(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_identity_replacement_at_the_same_root_path() {
        let container = tempfile::tempdir().unwrap();
        let root = container.path().join("installer-root");
        fs::create_dir(&root).unwrap();
        let job_directory =
            JobTempDir::create(&root, &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let moved = container.path().join("moved-root");
        fs::rename(&root, &moved).unwrap();
        fs::create_dir(&root).unwrap();

        let error = job_directory
            .validate_artifact_path(&job_directory.part_path(ArtifactKind::Msix))
            .unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert!(moved.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_a_reparse_like_direct_child_instead_of_following_it() {
        use std::os::unix::fs::symlink;

        let container = tempfile::tempdir().unwrap();
        let parent = TrustedDirectory::capture(container.path()).unwrap();
        let target = container.path().join("target");
        fs::create_dir(&target).unwrap();
        let child = container.path().join("cache");
        symlink(&target, &child).unwrap();

        let error = match open_direct_child_directory(&parent, "cache") {
            Err(error) => error,
            Ok(_) => panic!("a symlink direct child must be rejected"),
        };

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    }

    #[test]
    fn cleanup_removes_only_fixed_artifacts_and_the_empty_job_directory() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        for kind in [ArtifactKind::Msix, ArtifactKind::Dmg] {
            fs::write(job_directory.part_path(kind), b"partial").unwrap();
            fs::write(job_directory.final_path(kind), b"complete").unwrap();
        }
        let path = job_directory.path().to_path_buf();

        job_directory.cleanup().unwrap();
        // Windows removes a delete-pending directory when its last handle is
        // closed. The capability intentionally retains that handle until drop.
        drop(job_directory);

        assert!(!path.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn cleanup_fails_closed_when_the_job_directory_contains_an_unknown_entry() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let unknown_path = job_directory.path().join("unrecognized");
        fs::write(&unknown_path, b"do not recursively remove").unwrap();

        let error = job_directory.cleanup().unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert!(unknown_path.exists());
    }

    #[test]
    fn windows_policy_treats_non_msix_names_as_unknown_content() {
        let root = tempfile::tempdir().unwrap();
        let trusted_root = TrustedDirectory::capture(root.path()).unwrap();
        let job_directory = JobTempDir::create_with_ancestors(
            vec![trusted_root],
            &Uuid::new_v4().hyphenated().to_string(),
            ArtifactPolicy::WindowsMsixOnly,
        )
        .unwrap();
        let msix = job_directory.final_path(ArtifactKind::Msix);
        let dmg = job_directory.final_path(ArtifactKind::Dmg);
        fs::write(&msix, b"owned Windows package").unwrap();
        fs::write(&dmg, b"unknown on Windows").unwrap();

        assert!(job_directory.validate_artifact_path(&dmg).is_err());
        let error = job_directory.cleanup().unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert!(!msix.exists());
        assert!(dmg.exists());
    }

    #[test]
    fn stale_cleanup_removes_only_expired_canonical_job_directories() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        fs::write(job_directory.final_path(ArtifactKind::Msix), b"complete").unwrap();
        let path = job_directory.path().to_path_buf();
        let unknown = root.path().join("not-a-job-directory");
        fs::create_dir(&unknown).unwrap();
        // Stale cleanup models a prior process's abandoned directory. Do not
        // keep the creator's non-delete-sharing handle alive during the scan.
        drop(job_directory);

        let removed = JobTempDir::cleanup_stale_under(
            root.path(),
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now() + STALE_JOB_DIRECTORY_AGE,
        )
        .unwrap();

        assert_eq!(removed, 1);
        assert!(!path.exists());
        assert!(unknown.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn stale_cleanup_refuses_a_same_name_replacement_after_age_capture() {
        let root = tempfile::tempdir().unwrap();
        let job_id = Uuid::new_v4().hyphenated().to_string();
        let job_directory = JobTempDir::create(root.path(), &job_id).unwrap();
        fs::write(
            job_directory.final_path(ArtifactKind::Msix),
            b"captured old job",
        )
        .unwrap();
        let candidate = job_directory.path().to_path_buf();
        let displaced = root.path().join("displaced-old-job");
        let trusted_root = TrustedDirectory::capture(root.path()).unwrap();
        let replacement_performed = std::cell::Cell::new(false);

        let removed = JobTempDir::cleanup_stale_with_ancestors_after_capture(
            vec![trusted_root],
            ArtifactPolicy::CrossPlatform,
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now() + STALE_JOB_DIRECTORY_AGE,
            |captured| {
                if replacement_performed.replace(true) {
                    return;
                }
                assert_eq!(captured, candidate);
                fs::rename(captured, &displaced).unwrap();
                fs::create_dir(captured).unwrap();
                fs::write(captured.join("installer.msix"), b"replacement job").unwrap();
            },
        )
        .unwrap();

        assert!(replacement_performed.get());
        assert_eq!(removed, 0);
        assert_eq!(
            fs::read(candidate.join("installer.msix")).unwrap(),
            b"replacement job"
        );
        assert_eq!(
            fs::read(displaced.join("installer.msix")).unwrap(),
            b"captured old job"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_job_guard_revalidates_and_blocks_path_rename() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let displaced = root.path().join("displaced-job");

        job_directory.revalidate().unwrap();
        assert!(fs::rename(job_directory.path(), &displaced).is_err());
        job_directory.revalidate().unwrap();
        assert!(job_directory.path().is_dir());
        assert!(!displaced.exists());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_stale_cleanup_keeps_the_captured_directory_bound_during_cleanup() {
        let root = tempfile::tempdir().unwrap();
        let job_id = Uuid::new_v4().hyphenated().to_string();
        let job_directory = JobTempDir::create(root.path(), &job_id).unwrap();
        fs::write(
            job_directory.final_path(ArtifactKind::Msix),
            b"captured old job",
        )
        .unwrap();
        let candidate = job_directory.path().to_path_buf();
        drop(job_directory);

        let trusted_root = TrustedDirectory::capture(root.path()).unwrap();
        let displaced = root.path().join("displaced-old-job");
        let replacement_attempted = std::cell::Cell::new(false);
        let removed = JobTempDir::cleanup_stale_with_ancestors_after_capture(
            vec![trusted_root],
            ArtifactPolicy::CrossPlatform,
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now() + STALE_JOB_DIRECTORY_AGE,
            |captured| {
                replacement_attempted.set(true);
                assert_eq!(captured, candidate);
                assert!(fs::rename(captured, &displaced).is_err());
            },
        )
        .unwrap();

        assert!(replacement_attempted.get());
        assert_eq!(removed, 1);
        assert!(!candidate.exists());
        assert!(!displaced.exists());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cleanup_refuses_a_reparse_leaf_at_a_fixed_name() {
        use std::os::windows::fs::symlink_file;

        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("outside-target.msix");
        fs::write(&target, b"outside bytes").unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let leaf = job_directory.final_path(ArtifactKind::Msix);
        symlink_file(&target, &leaf).unwrap();

        let error = job_directory.cleanup().unwrap_err();

        assert_eq!(error.code(), InstallerErrorCode::InternalError);
        assert!(leaf.exists());
        assert_eq!(fs::read(target).unwrap(), b"outside bytes");
    }

    #[test]
    fn stale_cleanup_keeps_fresh_job_directories_and_future_timestamps() {
        let root = tempfile::tempdir().unwrap();
        let job_directory =
            JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap();
        let path = job_directory.path().to_path_buf();

        let removed = JobTempDir::cleanup_stale_under(
            root.path(),
            STALE_JOB_DIRECTORY_AGE,
            SystemTime::now(),
        )
        .unwrap();

        assert_eq!(removed, 0);
        assert!(path.exists());
        assert!(!is_stale(
            SystemTime::now() + Duration::from_secs(1),
            SystemTime::now(),
            STALE_JOB_DIRECTORY_AGE,
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn fixed_executable_fixture_matches_the_helper_install_layout() {
        let install_root = tempfile::tempdir().unwrap();
        let executable = install_root.path().join("fyagent.exe");
        fs::write(&executable, b"fixture executable").unwrap();
        let frozen = FrozenInstallRoot::capture(executable.clone()).unwrap();
        let job_id = CanonicalJobId::parse("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let cache = ensure_direct_child_directory(&frozen.install_root, CACHE_DIRECTORY).unwrap();
        let installer = ensure_direct_child_directory(&cache, CODEX_INSTALLER_DIRECTORY).unwrap();
        let directory = JobTempDir::create_with_ancestors(
            vec![frozen.install_root.clone(), cache, installer],
            job_id.as_str(),
            ArtifactPolicy::WindowsMsixOnly,
        )
        .unwrap();
        let helper_layout = derive_install_layout(&executable, &job_id).unwrap();

        assert_eq!(
            directory
                .final_path(ArtifactKind::Msix)
                .strip_prefix(&frozen.install_root.path)
                .unwrap(),
            helper_layout
                .installer_path()
                .strip_prefix(helper_layout.install_root())
                .unwrap()
        );
    }
}
