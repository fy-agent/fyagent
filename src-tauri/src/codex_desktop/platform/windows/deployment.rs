//! Narrow Windows PackageManager inventory, AUMID, and disk-space boundaries.
//!
//! Package deployment belongs exclusively to the unelevated user helper. The
//! main runtime keeps only exact-user inventory and launch operations and must
//! not call PackageManager add, stage, or provisioning APIs.

use std::{fmt, path::Path};

use crate::codex_desktop::{
    error::{InstallerError, InstallerErrorCode},
    types::{CpuArchitecture, PlatformVersion},
};
use crate::windows_runtime::InteractiveUserContext;

/// Immutable, redacted proof that a package-manager result belongs to the
/// exact interactive-user context supplied to the operation. The optional
/// wrapper on inventories and receipts is intentional: fakes can model a
/// missing proof and callers must reject it before reaching another side
/// effect.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WindowsUserContextEvidence {
    canonical_sid: String,
    process_session_id: u32,
    shell_session_id: u32,
}

impl WindowsUserContextEvidence {
    fn bound_to(context: &InteractiveUserContext) -> Self {
        Self {
            canonical_sid: context.canonical_sid().to_owned(),
            process_session_id: context.process_session_id(),
            shell_session_id: context.shell_session_id(),
        }
    }

    pub(crate) fn belongs_to(&self, context: &InteractiveUserContext) -> bool {
        self.canonical_sid == context.canonical_sid()
            && self.process_session_id == context.process_session_id()
            && self.shell_session_id == context.shell_session_id()
    }

    #[cfg(test)]
    pub(crate) fn for_test(context: &InteractiveUserContext) -> Self {
        Self::bound_to(context)
    }
}

impl fmt::Debug for WindowsUserContextEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsUserContextEvidence")
            .field("canonical_sid", &"<redacted>")
            .field("process_session_id", &self.process_session_id)
            .field("shell_session_id", &self.shell_session_id)
            .finish()
    }
}

/// Exact explicit-SID/Main inventory returned by the ordinary facade.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WindowsPackageInventory {
    context_evidence: Option<WindowsUserContextEvidence>,
    records: Vec<WindowsPackageRecord>,
}

impl WindowsPackageInventory {
    fn bound_to(context: &InteractiveUserContext, records: Vec<WindowsPackageRecord>) -> Self {
        Self {
            context_evidence: Some(WindowsUserContextEvidence::bound_to(context)),
            records,
        }
    }

    pub(crate) fn context_evidence(&self) -> Option<&WindowsUserContextEvidence> {
        self.context_evidence.as_ref()
    }

    pub(crate) fn records(&self) -> &[WindowsPackageRecord] {
        &self.records
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        context_evidence: Option<WindowsUserContextEvidence>,
        records: Vec<WindowsPackageRecord>,
    ) -> Self {
        Self {
            context_evidence,
            records,
        }
    }
}

impl fmt::Debug for WindowsPackageInventory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsPackageInventory")
            .field("context_evidence", &self.context_evidence)
            .field("record_count", &self.records.len())
            .finish()
    }
}

/// Context-bound completion receipt for an ordinary deployment or launch.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WindowsUserOperationReceipt {
    context_evidence: Option<WindowsUserContextEvidence>,
}

impl WindowsUserOperationReceipt {
    fn bound_to(context: &InteractiveUserContext) -> Self {
        Self {
            context_evidence: Some(WindowsUserContextEvidence::bound_to(context)),
        }
    }

    pub(crate) fn context_evidence(&self) -> Option<&WindowsUserContextEvidence> {
        self.context_evidence.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn for_test(context_evidence: Option<WindowsUserContextEvidence>) -> Self {
        Self { context_evidence }
    }
}

impl fmt::Debug for WindowsUserOperationReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsUserOperationReceipt")
            .field("context_evidence", &self.context_evidence)
            .finish()
    }
}

/// Current-user package facts obtained from PackageManager, not from a path,
/// process name, or executable scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowsPackageRecord {
    pub(crate) identity_name: String,
    pub(crate) publisher: String,
    pub(crate) family_name: String,
    pub(crate) version: PlatformVersion,
    pub(crate) architecture: CpuArchitecture,
    pub(crate) display_name: Option<String>,
    pub(crate) application_ids: Vec<String>,
}

impl WindowsPackageRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity_name: impl Into<String>,
        publisher: impl Into<String>,
        family_name: impl Into<String>,
        version: PlatformVersion,
        architecture: CpuArchitecture,
        display_name: Option<String>,
        application_ids: Vec<String>,
    ) -> Self {
        Self {
            identity_name: identity_name.into(),
            publisher: publisher.into(),
            family_name: family_name.into(),
            version,
            architecture,
            display_name,
            application_ids,
        }
    }
}

/// The only system boundary the ordinary Windows adapter needs. Every method
/// is explicitly bound to the one frozen interactive-user context. Inventory
/// is the exact SID/Main capability; an all-users query is deliberately absent
/// and must never be added as a fallback.
pub(crate) trait WindowsPackageManager: Send + Sync {
    fn packages_for_user(
        &self,
        context: &InteractiveUserContext,
    ) -> Result<WindowsPackageInventory, WindowsNativeError>;

    /// Launches an already verified app identity. The system implementation
    /// delegates this to the interactive user's Explorer shell rather than
    /// activating the app from the elevated FyAgent process.
    fn launch_aumid(
        &self,
        context: &InteractiveUserContext,
        aumid: &str,
    ) -> Result<WindowsUserOperationReceipt, WindowsNativeError>;
}

/// A sanitized native failure. Raw system text is intentionally not retained:
/// it can contain deployment paths, policy details, or user-specific data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WindowsNativeError {
    hresult: Option<i32>,
    context_mismatch: bool,
}

impl WindowsNativeError {
    pub(crate) const fn from_hresult(hresult: i32) -> Self {
        Self {
            hresult: Some(hresult),
            context_mismatch: false,
        }
    }

    pub(crate) const fn unavailable() -> Self {
        Self {
            hresult: None,
            context_mismatch: false,
        }
    }

    pub(crate) const fn context_mismatch() -> Self {
        Self {
            hresult: None,
            context_mismatch: true,
        }
    }

    pub(crate) const fn hresult(self) -> Option<i32> {
        self.hresult
    }

    pub(crate) const fn is_context_mismatch(self) -> bool {
        self.context_mismatch
    }
}

/// Converts a PackageManager failure into a stable installer error. Only
/// documented HRESULTs with a clear recovery action receive a specialized
/// code; all other deployment failures remain generic and retain the numeric
/// HRESULT for diagnostics.
pub(crate) fn deployment_error(error: WindowsNativeError) -> InstallerError {
    if error.is_context_mismatch() {
        return interactive_context_error();
    }
    let code = error
        .hresult()
        .map(map_deployment_hresult)
        .unwrap_or(InstallerErrorCode::WindowsDeploymentFailed);
    let mut installer_error = InstallerError::new(code).with_diagnostic_message(match code {
        InstallerErrorCode::WindowsPackageInUse => {
            "Windows reported that the package is currently in use"
        }
        InstallerErrorCode::WindowsDeploymentBlocked => {
            "Windows policy or deployment settings blocked the package"
        }
        InstallerErrorCode::WindowsDependencyMissing => {
            "Windows reported an unsatisfied package dependency"
        }
        InstallerErrorCode::PackageSignatureInvalid => {
            "Windows rejected the package signature or certificate trust"
        }
        InstallerErrorCode::PackageParseFailed => "Windows rejected malformed MSIX package data",
        InstallerErrorCode::MetadataChanged => "Windows rejected an older package version",
        _ => "Windows PackageManager deployment failed",
    });
    if let Some(hresult) = error.hresult() {
        installer_error = installer_error.with_platform_error_code(format_hresult(hresult));
    }
    installer_error
}

/// Verified application launch failures are not package deployment results.
/// Preserve an HRESULT if present, but always expose the stable launch-specific
/// code.
pub(crate) fn launch_error(error: WindowsNativeError) -> InstallerError {
    if error.is_context_mismatch() {
        return interactive_context_error();
    }
    let mut installer_error = InstallerError::new(InstallerErrorCode::LaunchFailed)
        .with_diagnostic_message("Windows could not launch the verified application identity");
    if let Some(hresult) = error.hresult() {
        installer_error = installer_error.with_platform_error_code(format_hresult(hresult));
    }
    installer_error
}

/// Stable fail-closed result for a missing, changed, or wrong-owner ordinary
/// Windows user proof. SID values are intentionally absent from diagnostics.
pub(crate) fn interactive_context_error() -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
        .with_diagnostic_message("the Windows interactive-user context is unavailable or changed")
}

pub(crate) fn verify_context_evidence(
    context: &InteractiveUserContext,
    evidence: Option<&WindowsUserContextEvidence>,
) -> Result<(), InstallerError> {
    match evidence {
        Some(evidence) if evidence.belongs_to(context) => Ok(()),
        _ => Err(interactive_context_error()),
    }
}

fn format_hresult(hresult: i32) -> String {
    format!("0x{:08X}", hresult as u32)
}

fn map_deployment_hresult(hresult: i32) -> InstallerErrorCode {
    match hresult as u32 {
        // ERROR_PACKAGES_IN_USE is retryable after closing the target app.
        0x8007_3D02 => InstallerErrorCode::WindowsPackageInUse,
        // ERROR_INSTALL_PACKAGE_DOWNGRADE requires fresh release metadata;
        // retrying the same older package or closing the app cannot fix it.
        0x8007_3D06 => InstallerErrorCode::MetadataChanged,
        // Deployment blocked by machine/profile/volume policy, or by the
        // legacy sideloading policy failure.
        0x8007_3CFF | 0x8007_3D01 | 0x8007_3D19 | 0x8007_3D21 | 0x8007_3D22 | 0x8007_3D23
        | 0x8007_0005 => InstallerErrorCode::WindowsDeploymentBlocked,
        // ERROR_INSTALL_RESOLVE_DEPENDENCY_FAILED and
        // ERROR_INSTALL_PREREQUISITE_FAILED.
        0x8007_3CF3 | 0x8007_3CFD => InstallerErrorCode::WindowsDependencyMissing,
        // Trust failures reported by the deployment platform. The `CF0` case
        // also covers a package that cannot be opened because its signature
        // and manifest publisher cannot be validated.
        0x8007_3CF0 | 0x800B_0100 | 0x800B_0109 | 0x800B_010A | 0x800B_0004 => {
            InstallerErrorCode::PackageSignatureInvalid
        }
        // Malformed manifest/block-map/corrupt package data is not a retryable
        // deployment result and should not be presented as a signature issue.
        0x8008_0204..=0x8008_0207 => InstallerErrorCode::PackageParseFailed,
        _ => InstallerErrorCode::WindowsDeploymentFailed,
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default)]
pub struct SystemWindowsPackageManager;

#[cfg(target_os = "windows")]
impl WindowsPackageManager for SystemWindowsPackageManager {
    fn packages_for_user(
        &self,
        context: &InteractiveUserContext,
    ) -> Result<WindowsPackageInventory, WindowsNativeError> {
        native::packages_for_user_main(context)
    }

    fn launch_aumid(
        &self,
        context: &InteractiveUserContext,
        aumid: &str,
    ) -> Result<WindowsUserOperationReceipt, WindowsNativeError> {
        native::launch_aumid(context, aumid)
    }
}

#[cfg(target_os = "windows")]
pub struct SystemWindowsDiskSpaceProbe {
    volumes: std::sync::Mutex<
        std::collections::HashMap<crate::codex_desktop::verify::VolumeKey, std::path::PathBuf>,
    >,
}

#[cfg(target_os = "windows")]
impl SystemWindowsDiskSpaceProbe {
    pub fn new() -> Self {
        Self {
            volumes: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(target_os = "windows")]
impl Default for SystemWindowsDiskSpaceProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
impl crate::codex_desktop::verify::DiskSpaceProbe for SystemWindowsDiskSpaceProbe {
    fn volume_key(
        &self,
        path: &Path,
    ) -> Result<
        crate::codex_desktop::verify::VolumeKey,
        crate::codex_desktop::verify::DiskSpaceProbeError,
    > {
        let volume_path = native::volume_root_for(path)
            .map_err(|_| crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)?;
        let key = crate::codex_desktop::verify::VolumeKey::new(volume_path.to_string_lossy())?;
        self.volumes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(key.clone(), volume_path);
        Ok(key)
    }

    fn available_bytes(
        &self,
        volume: &crate::codex_desktop::verify::VolumeKey,
    ) -> Result<u64, crate::codex_desktop::verify::DiskSpaceProbeError> {
        let path = self
            .volumes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(volume)
            .cloned()
            .ok_or(crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)?;
        native::available_bytes(&path)
            .map_err(|_| crate::codex_desktop::verify::DiskSpaceProbeError::Unavailable)
    }
}

#[cfg(target_os = "windows")]
mod native {
    use std::{
        ffi::OsString,
        os::windows::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    };

    #[cfg(test)]
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL},
            Security::{
                Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser, TOKEN_QUERY,
                TOKEN_USER,
            },
            System::Threading::{GetCurrentProcess, OpenProcessToken},
        },
    };
    use windows::{
        core::{HSTRING, PCWSTR},
        Management::Deployment::{PackageManager, PackageTypes},
        System::ProcessorArchitecture,
        Win32::{
            Storage::FileSystem::{GetDiskFreeSpaceExW, GetVolumePathNameW},
            System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        },
    };

    use super::{
        WindowsNativeError, WindowsPackageInventory, WindowsPackageRecord,
        WindowsUserOperationReceipt,
    };
    use crate::codex_desktop::types::{CpuArchitecture, PlatformVersion};
    use crate::windows_runtime::{revalidate_interactive_user_context, InteractiveUserContext};

    pub(super) fn packages_for_user_main(
        context: &InteractiveUserContext,
    ) -> Result<WindowsPackageInventory, WindowsNativeError> {
        require_current_context(context)?;
        let records = packages_for_user_sid_main(context.canonical_sid())?;
        require_current_context(context)?;
        Ok(WindowsPackageInventory::bound_to(context, records))
    }

    pub(super) fn packages_for_user_sid_main(
        canonical_sid: &str,
    ) -> Result<Vec<WindowsPackageRecord>, WindowsNativeError> {
        let _apartment = WinRtApartment::initialize()?;
        let package_manager = PackageManager::new().map_err(WindowsNativeError::from_windows)?;
        let mut records = Vec::new();

        let packages = package_manager
            .FindPackagesByUserSecurityIdWithPackageTypes(
                &HSTRING::from(canonical_sid),
                PackageTypes::Main,
            )
            .map_err(WindowsNativeError::from_windows)?;
        let iterator = packages.First().map_err(WindowsNativeError::from_windows)?;
        while iterator
            .HasCurrent()
            .map_err(WindowsNativeError::from_windows)?
        {
            let package = iterator
                .Current()
                .map_err(WindowsNativeError::from_windows)?;
            let package_id = package.Id().map_err(WindowsNativeError::from_windows)?;
            let identity_name = package_id
                .Name()
                .map_err(WindowsNativeError::from_windows)?
                .to_string();
            let version = package_id
                .Version()
                .map_err(WindowsNativeError::from_windows)?;
            let architecture = map_architecture(
                package_id
                    .Architecture()
                    .map_err(WindowsNativeError::from_windows)?,
            );
            let family_name = package_id
                .FamilyName()
                .map_err(WindowsNativeError::from_windows)?
                .to_string();
            let app_entries = package
                .GetAppListEntriesAsync()
                .map_err(WindowsNativeError::from_windows)?
                .get()
                .map_err(WindowsNativeError::from_windows)?;
            let mut application_ids = Vec::new();
            let app_count = app_entries
                .Size()
                .map_err(WindowsNativeError::from_windows)?;
            for index in 0..app_count {
                let entry = app_entries
                    .GetAt(index)
                    .map_err(WindowsNativeError::from_windows)?;
                let aumid = entry
                    .AppUserModelId()
                    .map_err(WindowsNativeError::from_windows)?
                    .to_string();
                let Some((aumid_family, application_id)) = aumid.split_once('!') else {
                    return Err(WindowsNativeError::unavailable());
                };
                if aumid_family != family_name || application_id.is_empty() {
                    return Err(WindowsNativeError::unavailable());
                }
                application_ids.push(application_id.to_owned());
            }

            let display_name = package
                .DisplayName()
                .ok()
                .map(|value| value.to_string())
                .filter(|value| !value.trim().is_empty());
            records.push(WindowsPackageRecord::new(
                identity_name,
                package_id
                    .Publisher()
                    .map_err(WindowsNativeError::from_windows)?
                    .to_string(),
                family_name,
                PlatformVersion::WindowsMsix {
                    major: version.Major,
                    minor: version.Minor,
                    build: version.Build,
                    revision: version.Revision,
                },
                architecture,
                display_name,
                application_ids,
            ));
            iterator
                .MoveNext()
                .map_err(WindowsNativeError::from_windows)?;
        }
        Ok(records)
    }

    pub(super) fn launch_aumid(
        context: &InteractiveUserContext,
        aumid: &str,
    ) -> Result<WindowsUserOperationReceipt, WindowsNativeError> {
        require_current_context(context)?;
        crate::platform::process_launch::launch_trusted_windows_app_aumid_as_user(aumid)
            .map_err(|_| WindowsNativeError::unavailable())?;
        require_current_context(context)?;
        Ok(WindowsUserOperationReceipt::bound_to(context))
    }

    fn require_current_context(context: &InteractiveUserContext) -> Result<(), WindowsNativeError> {
        revalidate_interactive_user_context(context)
            .then_some(())
            .ok_or_else(WindowsNativeError::context_mismatch)
    }

    #[cfg(test)]
    pub(super) fn current_process_sid_for_test() -> Result<String, WindowsNativeError> {
        struct OwnedToken(HANDLE);

        impl Drop for OwnedToken {
            fn drop(&mut self) {
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }

        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(WindowsNativeError::from_windows)?;
        if token.is_invalid() {
            return Err(WindowsNativeError::unavailable());
        }
        let token = OwnedToken(token);
        let mut required = 0_u32;
        let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut required) };
        if required < std::mem::size_of::<TOKEN_USER>() as u32 {
            return Err(WindowsNativeError::unavailable());
        }
        // TOKEN_USER contains pointer-aligned fields. A byte vector does not
        // guarantee that alignment before the Win32 buffer is cast back.
        let mut buffer = vec![0_usize; (required as usize).div_ceil(std::mem::size_of::<usize>())];
        unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                required,
                &mut required,
            )
        }
        .map_err(WindowsNativeError::from_windows)?;
        let token_user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
        if token_user.User.Sid.is_invalid() {
            return Err(WindowsNativeError::unavailable());
        }
        let mut string_sid = PWSTR::null();
        unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut string_sid) }
            .map_err(WindowsNativeError::from_windows)?;
        if string_sid.is_null() {
            return Err(WindowsNativeError::unavailable());
        }
        let value =
            unsafe { string_sid.to_string() }.map_err(|_| WindowsNativeError::unavailable());
        unsafe {
            let _ = LocalFree(Some(HLOCAL(string_sid.0.cast())));
        }
        value
    }

    pub(super) fn volume_root_for(path: &Path) -> Result<PathBuf, WindowsNativeError> {
        let path = wide_path(path)?;
        let mut volume = vec![0_u16; 32_768];
        unsafe { GetVolumePathNameW(PCWSTR(path.as_ptr()), &mut volume) }
            .map_err(WindowsNativeError::from_windows)?;
        let length = volume
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(volume.len());
        if length == 0 || length == volume.len() {
            return Err(WindowsNativeError::unavailable());
        }
        Ok(PathBuf::from(OsString::from_wide(&volume[..length])))
    }

    pub(super) fn available_bytes(path: &Path) -> Result<u64, WindowsNativeError> {
        let path = wide_path(path)?;
        let mut available = 0_u64;
        unsafe { GetDiskFreeSpaceExW(PCWSTR(path.as_ptr()), Some(&mut available), None, None) }
            .map_err(WindowsNativeError::from_windows)?;
        Ok(available)
    }

    fn wide_path(path: &Path) -> Result<Vec<u16>, WindowsNativeError> {
        if path.as_os_str().is_empty() {
            return Err(WindowsNativeError::unavailable());
        }
        Ok(path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect())
    }

    fn map_architecture(architecture: ProcessorArchitecture) -> CpuArchitecture {
        match architecture {
            ProcessorArchitecture::X64 => CpuArchitecture::X86_64,
            ProcessorArchitecture::Arm64 => CpuArchitecture::Aarch64,
            _ => CpuArchitecture::Unsupported,
        }
    }

    struct WinRtApartment;

    impl WinRtApartment {
        fn initialize() -> Result<Self, WindowsNativeError> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
                .map_err(WindowsNativeError::from_windows)?;
            Ok(Self)
        }
    }

    impl Drop for WinRtApartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    impl WindowsNativeError {
        fn from_windows(error: windows::core::Error) -> Self {
            Self::from_hresult(error.code().0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_deployment_hresult_values_to_stable_errors() {
        let cases = [
            (
                0x8007_3D02_u32 as i32,
                InstallerErrorCode::WindowsPackageInUse,
            ),
            (0x8007_3D06_u32 as i32, InstallerErrorCode::MetadataChanged),
            (
                0x8007_3D01_u32 as i32,
                InstallerErrorCode::WindowsDeploymentBlocked,
            ),
            (
                0x8007_3CF3_u32 as i32,
                InstallerErrorCode::WindowsDependencyMissing,
            ),
            (
                0x800B_0100_u32 as i32,
                InstallerErrorCode::PackageSignatureInvalid,
            ),
            (
                0x8008_0205_u32 as i32,
                InstallerErrorCode::PackageParseFailed,
            ),
            (
                0x8123_4567_u32 as i32,
                InstallerErrorCode::WindowsDeploymentFailed,
            ),
        ];
        for (hresult, expected) in cases {
            let error = deployment_error(WindowsNativeError::from_hresult(hresult));
            assert_eq!(error.code(), expected);
            assert_eq!(
                error.to_dto().details.platform_error_code,
                Some(format!("0x{:08X}", hresult as u32))
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn native_explicit_sid_main_query_smoke() {
        let sid = native::current_process_sid_for_test()
            .expect("the Windows test process token must expose its SID");
        // The runner does not need Codex, Store access, network access, or a
        // second account. An empty exact-SID/Main inventory is a valid smoke
        // result; the assertion is that the locked WinRT overload completed
        // and propagated the native error from an invalid explicit SID.
        let records = native::packages_for_user_sid_main(&sid)
            .expect("the explicit-SID/Main PackageManager binding must be callable");
        assert!(records
            .iter()
            .all(|record| !record.identity_name.is_empty()));
        let error = native::packages_for_user_sid_main("not-a-windows-sid")
            .expect_err("PackageManager must reject a malformed explicit SID");
        assert!(error.hresult().is_some());
    }
}
