//! Windows installation evidence and product policy.
//!
//! Registry/App Paths/MSIX records are discovery evidence only. A path becomes
//! actionable only after the native inspector proves a regular, non-reparse
//! executable, a closed product identity, a trusted Authenticode chain, the
//! reviewed signer subject, and a stable file identity. Renderer input never
//! reaches this module.

use std::path::PathBuf;

#[cfg(any(target_os = "windows", test))]
use std::path::Path;

use super::desktop::{DesktopInstallationEvidence, DesktopProduct};

#[cfg(any(target_os = "windows", test))]
use super::types::{
    AgentReasonCode, InstallationEvidenceCode, InstallationOwner, InstallationPackageKind,
    InstallationScope,
};

#[cfg(target_os = "windows")]
const MAX_REGISTRY_CHILDREN: usize = 4096;
#[cfg(any(target_os = "windows", test))]
const MAX_REGISTRY_STRING_CHARS: usize = 32 * 1024;

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WindowsEvidenceSource {
    KnownPath,
    AppPaths,
    Uninstall,
}

#[cfg(any(target_os = "windows", test))]
impl WindowsEvidenceSource {
    fn code(self) -> InstallationEvidenceCode {
        match self {
            Self::KnownPath => InstallationEvidenceCode::KnownPath,
            Self::AppPaths => InstallationEvidenceCode::AppPathsRegistration,
            Self::Uninstall => InstallationEvidenceCode::UninstallRegistration,
        }
    }

    fn retains_missing_observation(self) -> bool {
        !matches!(self, Self::KnownPath)
    }
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsPathHint {
    path: PathBuf,
    source: WindowsEvidenceSource,
    registration_scope: InstallationScope,
    registration_version: Option<String>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectedWindowsExecutable {
    stable_file_key: String,
    product_version: Option<String>,
    machine: u16,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsInspectionFailure {
    Missing,
    UnsafeFileShape,
    ProductIdentityMismatch,
    SignatureInvalid,
    SignerMismatch,
    ArchitectureUnsupported,
    IdentityChanged,
    Unavailable,
}

#[cfg(any(target_os = "windows", test))]
trait WindowsExecutableInspector {
    fn inspect(
        &self,
        path: &Path,
        policy: &WindowsProductPolicy,
    ) -> Result<InspectedWindowsExecutable, WindowsInspectionFailure>;
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy)]
struct WindowsProductPolicy {
    product_names: &'static [&'static str],
    signer_subjects: &'static [&'static str],
}

#[cfg(any(target_os = "windows", test))]
fn product_policy(product: &DesktopProduct) -> WindowsProductPolicy {
    let signer_subjects: &[&str] = match product.agent_id {
        crate::services::external_agents::AgentCatalogId::QoderWork => {
            &["Alibaba Cloud Computing Co., Ltd."]
        }
        crate::services::external_agents::AgentCatalogId::TraeWork => &["北京引力弹弓科技有限公司"],
        crate::services::external_agents::AgentCatalogId::WorkBuddy => {
            &["Tencent Technology (Shenzhen) Company Limited"]
        }
        _ => &[],
    };
    WindowsProductPolicy {
        product_names: product.windows_product_names,
        signer_subjects,
    }
}

#[cfg(target_os = "windows")]
pub(super) fn discover_windows_installations(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Vec<DesktopInstallationEvidence> {
    discover_windows_inventory(product, roots).installations
}

#[cfg(not(target_os = "windows"))]
pub(super) fn discover_windows_installations(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> Vec<DesktopInstallationEvidence> {
    // Platform-neutral tests keep their existing fake-PE fixture. The shipped
    // Windows build never uses the UTF-16 window scanner.
    super::desktop::discover_windows_known_path_installations(product, roots)
}

#[cfg(target_os = "windows")]
pub(super) struct WindowsInstallationDiscovery {
    pub(super) installations: Vec<DesktopInstallationEvidence>,
    pub(super) complete: bool,
}

#[cfg(target_os = "windows")]
pub(super) fn discover_windows_inventory(
    product: &DesktopProduct,
    roots: &[PathBuf],
) -> WindowsInstallationDiscovery {
    native::discover(product, roots)
}

#[cfg(target_os = "windows")]
pub(super) fn verify_windows_installer(
    product: &DesktopProduct,
    path: &Path,
    architecture: super::sources::AgentArch,
) -> Result<Option<String>, AgentReasonCode> {
    native::verify_installer(path, &product_policy(product), architecture)
        .map(|inspection| inspection.product_version)
        .map_err(inspection_reason)
}

#[cfg(any(target_os = "windows", test))]
fn normalize_hints(
    product: &DesktopProduct,
    roots: &[PathBuf],
    hints: Vec<WindowsPathHint>,
    inspector: &dyn WindowsExecutableInspector,
) -> Vec<DesktopInstallationEvidence> {
    let policy = product_policy(product);
    let mut evidence = Vec::new();
    for hint in deduplicate_hints(hints) {
        let scope = scope_for_path(&hint.path, roots).unwrap_or(match hint.source {
            WindowsEvidenceSource::KnownPath => hint.registration_scope,
            WindowsEvidenceSource::AppPaths | WindowsEvidenceSource::Uninstall => {
                InstallationScope::Custom
            }
        });
        match inspector.inspect(&hint.path, &policy) {
            Ok(inspection) => evidence.push(DesktopInstallationEvidence {
                stable_key: inspection.stable_file_key,
                path: hint.path,
                scope,
                package_kind: InstallationPackageKind::Exe,
                local_version: inspection.product_version.or(hint.registration_version),
                owner: InstallationOwner::VendorInstaller,
                launch_eligible: true,
                update_eligible: true,
                reason_codes: Vec::new(),
                evidence_codes: vec![hint.source.code(), InstallationEvidenceCode::FileIdentity],
            }),
            Err(WindowsInspectionFailure::Missing)
                if !hint.source.retains_missing_observation() => {}
            Err(failure) => evidence.push(DesktopInstallationEvidence {
                stable_key: format!(
                    "windows-observation:{:?}:{}",
                    hint.source,
                    hint.path.to_string_lossy()
                ),
                path: hint.path,
                scope,
                package_kind: InstallationPackageKind::Exe,
                local_version: hint.registration_version,
                owner: InstallationOwner::VendorInstaller,
                launch_eligible: false,
                update_eligible: false,
                reason_codes: vec![inspection_reason(failure)],
                evidence_codes: vec![hint.source.code()],
            }),
        }
    }
    evidence
}

#[cfg(any(target_os = "windows", test))]
fn deduplicate_hints(hints: Vec<WindowsPathHint>) -> Vec<WindowsPathHint> {
    let mut result = Vec::new();
    for hint in hints {
        if let Some(existing) = result.iter_mut().find(|existing: &&mut WindowsPathHint| {
            existing.path == hint.path
                && existing.source == hint.source
                && existing.registration_scope == hint.registration_scope
        }) {
            if existing.registration_version.is_none() {
                existing.registration_version = hint.registration_version;
            }
        } else {
            result.push(hint);
        }
    }
    result
}

#[cfg(any(target_os = "windows", test))]
fn scope_for_path(path: &Path, roots: &[PathBuf]) -> Option<InstallationScope> {
    roots.iter().enumerate().find_map(|(index, root)| {
        path.starts_with(root).then_some(if index == 0 {
            InstallationScope::CurrentUser
        } else {
            InstallationScope::AllUsers
        })
    })
}

#[cfg(any(target_os = "windows", test))]
fn inspection_reason(failure: WindowsInspectionFailure) -> AgentReasonCode {
    match failure {
        WindowsInspectionFailure::Missing => AgentReasonCode::TargetNotExecutable,
        WindowsInspectionFailure::ProductIdentityMismatch
        | WindowsInspectionFailure::IdentityChanged => AgentReasonCode::CandidateConflict,
        WindowsInspectionFailure::ArchitectureUnsupported => AgentReasonCode::PlatformUnsupported,
        WindowsInspectionFailure::UnsafeFileShape
        | WindowsInspectionFailure::SignatureInvalid
        | WindowsInspectionFailure::SignerMismatch
        | WindowsInspectionFailure::Unavailable => AgentReasonCode::SourceNotVerified,
    }
}

#[cfg(any(target_os = "windows", test))]
fn bounded_registry_string(value: String) -> Option<String> {
    let trimmed = value.trim().trim_matches('"').trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_REGISTRY_STRING_CHARS
        || trimmed.contains('\0')
        || trimmed.chars().any(char::is_control)
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(any(target_os = "windows", test))]
fn display_icon_path(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    let raw = if let Some(remainder) = trimmed.strip_prefix('"') {
        let end = remainder.find('"')?;
        &remainder[..end]
    } else if let Some((path, index)) = trimmed.rsplit_once(',') {
        if index.trim().parse::<i32>().is_ok() {
            path.trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    bounded_registry_string(raw.to_string()).map(PathBuf::from)
}

#[cfg(target_os = "windows")]
mod native {
    use std::{
        collections::HashSet,
        ffi::{c_void, OsStr},
        mem::size_of,
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
        ptr,
    };

    use windows::{
        core::{GUID, PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, HANDLE, HWND, INVALID_HANDLE_VALUE},
            Security::{
                Cryptography::{
                    CertCloseStore, CertFreeCertificateContext, CertGetNameStringW, CryptMsgClose,
                    CryptMsgGetAndVerifySigner, CryptMsgGetParam, CryptQueryObject, CERT_CONTEXT,
                    CERT_NAME_SIMPLE_DISPLAY_TYPE, CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                    CERT_QUERY_FORMAT_FLAG_BINARY, CERT_QUERY_OBJECT_FILE, CMSG_SIGNER_COUNT_PARAM,
                    HCERTSTORE,
                },
                WinTrust::{
                    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA,
                    WINTRUST_DATA_0, WINTRUST_FILE_INFO, WTD_CHOICE_FILE,
                    WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT, WTD_REVOKE_WHOLECHAIN,
                    WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UICONTEXT_EXECUTE,
                    WTD_UI_NONE,
                },
            },
            Storage::FileSystem::{
                CreateFileW, GetFileInformationByHandle, GetFileVersionInfoSizeW,
                GetFileVersionInfoW, VerQueryValueW, BY_HANDLE_FILE_INFORMATION,
                FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
                FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
                OPEN_EXISTING,
            },
            System::SystemInformation::{IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_ARM64},
        },
    };
    use winreg::RegKey;

    use super::{
        bounded_registry_string, display_icon_path, normalize_hints, InspectedWindowsExecutable,
        WindowsEvidenceSource, WindowsExecutableInspector, WindowsInspectionFailure,
        WindowsInstallationDiscovery, WindowsPathHint, WindowsProductPolicy, MAX_REGISTRY_CHILDREN,
    };
    use crate::{
        agent_install::{desktop::DesktopProduct, sources::AgentArch, types::InstallationScope},
        windows_runtime::{
            open_inventory_child_read, open_machine_inventory_parent,
            open_shell_user_inventory_parent, require_interactive_user_context,
            revalidate_interactive_user_context, MachineRegistryLocation, RegistryView,
            ShellUserRegistryLocation,
        },
    };

    pub(super) fn discover(
        product: &DesktopProduct,
        roots: &[PathBuf],
    ) -> WindowsInstallationDiscovery {
        let context = require_interactive_user_context();
        let mut hints = known_path_hints(product, roots);
        let (registry_hints, registry_complete) = registry_hints(product);
        hints.extend(registry_hints);
        if !revalidate_interactive_user_context(context) {
            return WindowsInstallationDiscovery {
                installations: Vec::new(),
                complete: false,
            };
        }
        WindowsInstallationDiscovery {
            installations: normalize_hints(product, roots, hints, &NativeInspector),
            complete: registry_complete,
        }
    }

    fn known_path_hints(product: &DesktopProduct, roots: &[PathBuf]) -> Vec<WindowsPathHint> {
        let mut hints = Vec::new();
        for (index, root) in roots.iter().enumerate() {
            let scope = if index == 0 {
                InstallationScope::CurrentUser
            } else {
                InstallationScope::AllUsers
            };
            for relative in product.windows_relative_exes {
                hints.push(WindowsPathHint {
                    path: root.join(relative),
                    source: WindowsEvidenceSource::KnownPath,
                    registration_scope: scope,
                    registration_version: None,
                });
            }
        }
        hints
    }

    fn registry_hints(product: &DesktopProduct) -> (Vec<WindowsPathHint>, bool) {
        let mut hints = Vec::new();
        let mut complete = true;
        for view in [RegistryView::Registry64, RegistryView::Registry32] {
            complete &= collect_optional_parent(
                open_shell_user_inventory_parent(ShellUserRegistryLocation::AppPaths, view),
                |parent| {
                    collect_app_paths(
                        parent,
                        view,
                        product,
                        InstallationScope::CurrentUser,
                        &mut hints,
                    )
                },
            );
            complete &= collect_optional_parent(
                open_machine_inventory_parent(MachineRegistryLocation::AppPaths, view),
                |parent| {
                    collect_app_paths(
                        parent,
                        view,
                        product,
                        InstallationScope::AllUsers,
                        &mut hints,
                    )
                },
            );
            complete &= collect_optional_parent(
                open_shell_user_inventory_parent(ShellUserRegistryLocation::Uninstall, view),
                |parent| {
                    collect_uninstall(
                        parent,
                        view,
                        product,
                        InstallationScope::CurrentUser,
                        &mut hints,
                    )
                },
            );
            complete &= collect_optional_parent(
                open_machine_inventory_parent(MachineRegistryLocation::Uninstall, view),
                |parent| {
                    collect_uninstall(
                        parent,
                        view,
                        product,
                        InstallationScope::AllUsers,
                        &mut hints,
                    )
                },
            );
        }
        (hints, complete)
    }

    fn collect_optional_parent(
        parent: std::io::Result<RegKey>,
        collect: impl FnOnce(&RegKey) -> bool,
    ) -> bool {
        match parent {
            Ok(parent) => collect(&parent),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
        }
    }

    fn collect_app_paths(
        parent: &RegKey,
        view: RegistryView,
        product: &DesktopProduct,
        scope: InstallationScope,
        hints: &mut Vec<WindowsPathHint>,
    ) -> bool {
        let mut complete = true;
        let mut names = HashSet::new();
        for relative in product.windows_relative_exes {
            if let Some(name) = Path::new(relative)
                .file_name()
                .and_then(|name| name.to_str())
            {
                names.insert(name.to_string());
            }
        }
        for name in names {
            let key = match open_inventory_child_read(parent, &name, view) {
                Ok(key) => key,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            let Ok(value) = key.get_value::<String, _>("") else {
                continue;
            };
            let Some(path) = executable_registry_path(&value) else {
                continue;
            };
            hints.push(WindowsPathHint {
                path,
                source: WindowsEvidenceSource::AppPaths,
                registration_scope: scope,
                registration_version: None,
            });
        }
        complete
    }

    fn collect_uninstall(
        parent: &RegKey,
        view: RegistryView,
        product: &DesktopProduct,
        scope: InstallationScope,
        hints: &mut Vec<WindowsPathHint>,
    ) -> bool {
        let mut complete = true;
        for (count, name) in parent.enum_keys().enumerate() {
            if count >= MAX_REGISTRY_CHILDREN {
                complete = false;
                break;
            }
            let name = match name {
                Ok(name) => name,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            let key = match open_inventory_child_read(parent, &name, view) {
                Ok(key) => key,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            let Ok(display_name) = key.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let Some(display_name) = bounded_registry_string(display_name) else {
                continue;
            };
            if !product
                .windows_product_names
                .iter()
                .any(|expected| display_name.eq_ignore_ascii_case(expected))
            {
                continue;
            }
            let version = key
                .get_value::<String, _>("DisplayVersion")
                .ok()
                .and_then(bounded_registry_string);
            if let Ok(icon) = key.get_value::<String, _>("DisplayIcon") {
                if let Some(path) = display_icon_path(&icon).and_then(validate_executable_path) {
                    hints.push(WindowsPathHint {
                        path,
                        source: WindowsEvidenceSource::Uninstall,
                        registration_scope: scope,
                        registration_version: version.clone(),
                    });
                }
            }
            if let Ok(location) = key.get_value::<String, _>("InstallLocation") {
                let Some(location) = bounded_registry_string(location).map(PathBuf::from) else {
                    continue;
                };
                for relative in product.windows_relative_exes {
                    let relative = Path::new(relative);
                    for candidate in [
                        relative.file_name().map(|name| location.join(name)),
                        Some(location.join(relative)),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        if let Some(path) = validate_executable_path(candidate) {
                            hints.push(WindowsPathHint {
                                path,
                                source: WindowsEvidenceSource::Uninstall,
                                registration_scope: scope,
                                registration_version: version.clone(),
                            });
                        }
                    }
                }
            }
        }
        complete
    }

    fn executable_registry_path(value: &str) -> Option<PathBuf> {
        bounded_registry_string(value.to_string())
            .map(PathBuf::from)
            .and_then(validate_executable_path)
    }

    fn validate_executable_path(path: PathBuf) -> Option<PathBuf> {
        if path.is_absolute()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            && !path.to_string_lossy().contains('%')
        {
            Some(path)
        } else {
            None
        }
    }

    struct NativeInspector;

    impl WindowsExecutableInspector for NativeInspector {
        fn inspect(
            &self,
            path: &Path,
            policy: &WindowsProductPolicy,
        ) -> Result<InspectedWindowsExecutable, WindowsInspectionFailure> {
            inspect_executable(path, policy)
        }
    }

    fn inspect_executable(
        path: &Path,
        policy: &WindowsProductPolicy,
    ) -> Result<InspectedWindowsExecutable, WindowsInspectionFailure> {
        let before = file_identity(path)?;
        let version = version_resource(path)?;
        if !policy
            .product_names
            .iter()
            .any(|expected| version.product_name.eq_ignore_ascii_case(expected))
        {
            return Err(WindowsInspectionFailure::ProductIdentityMismatch);
        }
        let machine = pe_machine(path)?;
        if machine != IMAGE_FILE_MACHINE_AMD64.0 && machine != IMAGE_FILE_MACHINE_ARM64.0 {
            return Err(WindowsInspectionFailure::ArchitectureUnsupported);
        }
        let wide = wide_path(path)?;
        if !verify_authenticode(&wide) {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }
        let signer_subject = signer_subject(&wide)?;
        if !policy
            .signer_subjects
            .iter()
            .any(|expected| signer_subject == *expected)
        {
            return Err(WindowsInspectionFailure::SignerMismatch);
        }
        let after = file_identity(path)?;
        if before != after {
            return Err(WindowsInspectionFailure::IdentityChanged);
        }
        Ok(InspectedWindowsExecutable {
            stable_file_key: format!(
                "windows-file:{:08x}:{:016x}",
                before.volume_serial, before.file_index
            ),
            product_version: version.product_version,
            machine,
        })
    }

    pub(super) fn verify_installer(
        path: &Path,
        policy: &WindowsProductPolicy,
        architecture: AgentArch,
    ) -> Result<InspectedWindowsExecutable, WindowsInspectionFailure> {
        let inspection = inspect_executable(path, policy)?;
        let expected_machine = match architecture {
            AgentArch::X86_64 => IMAGE_FILE_MACHINE_AMD64.0,
            AgentArch::Aarch64 => IMAGE_FILE_MACHINE_ARM64.0,
        };
        if inspection.machine != expected_machine {
            return Err(WindowsInspectionFailure::ArchitectureUnsupported);
        }
        Ok(inspection)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct FileIdentity {
        volume_serial: u32,
        file_index: u64,
    }

    fn file_identity(path: &Path) -> Result<FileIdentity, WindowsInspectionFailure> {
        let wide = wide_path(path)?;
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
                None,
            )
        }
        .map_err(|_| {
            if path.exists() {
                WindowsInspectionFailure::UnsafeFileShape
            } else {
                WindowsInspectionFailure::Missing
            }
        })?;
        if handle == INVALID_HANDLE_VALUE {
            return Err(WindowsInspectionFailure::Missing);
        }
        let owned = OwnedHandle(handle);
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(owned.0, &mut info) }
            .map_err(|_| WindowsInspectionFailure::Unavailable)?;
        if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        Ok(FileIdentity {
            volume_serial: info.dwVolumeSerialNumber,
            file_index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        })
    }

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    struct VersionResource {
        product_name: String,
        product_version: Option<String>,
    }

    fn version_resource(path: &Path) -> Result<VersionResource, WindowsInspectionFailure> {
        let wide = wide_path(path)?;
        let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None) };
        if size == 0 || size > 16 * 1024 * 1024 {
            return Err(WindowsInspectionFailure::ProductIdentityMismatch);
        }
        let mut bytes = vec![0_u8; size as usize];
        unsafe {
            GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, bytes.as_mut_ptr().cast())
        }
        .map_err(|_| WindowsInspectionFailure::ProductIdentityMismatch)?;
        let translations = query_translation(&bytes).unwrap_or(vec![(0x0409, 0x04b0)]);
        let product_name = query_version_string(&bytes, &translations, "ProductName")
            .or_else(|| query_version_string(&bytes, &translations, "FileDescription"))
            .ok_or(WindowsInspectionFailure::ProductIdentityMismatch)?;
        let product_version = query_version_string(&bytes, &translations, "ProductVersion")
            .or_else(|| query_version_string(&bytes, &translations, "FileVersion"));
        Ok(VersionResource {
            product_name,
            product_version,
        })
    }

    fn query_translation(bytes: &[u8]) -> Option<Vec<(u16, u16)>> {
        let query = wide_string("\\VarFileInfo\\Translation");
        let mut pointer = ptr::null_mut();
        let mut length = 0_u32;
        let queried = unsafe {
            VerQueryValueW(
                bytes.as_ptr().cast(),
                PCWSTR(query.as_ptr()),
                &mut pointer,
                &mut length,
            )
        };
        if !queried.as_bool() {
            return None;
        }
        if pointer.is_null() || !(4..=256).contains(&length) || length % 4 != 0 {
            return None;
        }
        let pairs =
            unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), length as usize / 2) };
        Some(
            pairs
                .chunks_exact(2)
                .take(16)
                .map(|pair| (pair[0], pair[1]))
                .collect(),
        )
    }

    fn query_version_string(
        bytes: &[u8],
        translations: &[(u16, u16)],
        name: &str,
    ) -> Option<String> {
        translations.iter().find_map(|(language, codepage)| {
            let query = wide_string(&format!(
                "\\StringFileInfo\\{language:04x}{codepage:04x}\\{name}"
            ));
            let mut pointer = ptr::null_mut();
            let mut length = 0_u32;
            let queried = unsafe {
                VerQueryValueW(
                    bytes.as_ptr().cast(),
                    PCWSTR(query.as_ptr()),
                    &mut pointer,
                    &mut length,
                )
            };
            if !queried.as_bool() {
                return None;
            }
            if pointer.is_null() || length == 0 || length > 1024 {
                return None;
            }
            let units =
                unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), length as usize) };
            let end = units
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(units.len());
            bounded_registry_string(String::from_utf16(&units[..end]).ok()?)
        })
    }

    fn pe_machine(path: &Path) -> Result<u16, WindowsInspectionFailure> {
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).map_err(|_| WindowsInspectionFailure::Missing)?;
        let mut dos = [0_u8; 64];
        file.read_exact(&mut dos)
            .map_err(|_| WindowsInspectionFailure::UnsafeFileShape)?;
        if &dos[..2] != b"MZ" {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        let pe_offset = u32::from_le_bytes(dos[0x3c..0x40].try_into().unwrap()) as u64;
        if pe_offset > 16 * 1024 * 1024 {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        file.seek(SeekFrom::Start(pe_offset))
            .map_err(|_| WindowsInspectionFailure::UnsafeFileShape)?;
        let mut header = [0_u8; 6];
        file.read_exact(&mut header)
            .map_err(|_| WindowsInspectionFailure::UnsafeFileShape)?;
        if &header[..4] != b"PE\0\0" {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        Ok(u16::from_le_bytes([header[4], header[5]]))
    }

    fn verify_authenticode(wide_path: &[u16]) -> bool {
        let mut file = WINTRUST_FILE_INFO {
            cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(wide_path.as_ptr()),
            hFile: HANDLE::default(),
            pgKnownSubject: ptr::null_mut(),
        };
        let mut data = WINTRUST_DATA {
            cbStruct: size_of::<WINTRUST_DATA>() as u32,
            pPolicyCallbackData: ptr::null_mut(),
            pSIPClientData: ptr::null_mut(),
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
            dwStateAction: WTD_STATEACTION_VERIFY,
            hWVTStateData: HANDLE::default(),
            pwszURLReference: PWSTR::null(),
            dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
            dwUIContext: WTD_UICONTEXT_EXECUTE,
            pSignatureSettings: ptr::null_mut(),
        };
        let mut action: GUID = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = unsafe {
            WinVerifyTrust(
                HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast::<c_void>(),
            )
        };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = unsafe {
            WinVerifyTrust(
                HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast::<c_void>(),
            )
        };
        status == 0
    }

    fn signer_subject(wide_path: &[u16]) -> Result<String, WindowsInspectionFailure> {
        let mut raw_store = HCERTSTORE::default();
        let mut raw_message = ptr::null_mut();
        unsafe {
            CryptQueryObject(
                CERT_QUERY_OBJECT_FILE,
                wide_path.as_ptr().cast(),
                CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                None,
                None,
                None,
                Some(&mut raw_store),
                Some(&mut raw_message),
                None,
            )
        }
        .map_err(|_| WindowsInspectionFailure::SignatureInvalid)?;
        let store = OwnedCertStore(raw_store);
        let message = OwnedCryptMessage(raw_message);
        if store.0.is_invalid() || message.0.is_null() {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }

        let mut signer_count = 0_u32;
        let mut signer_count_bytes = size_of::<u32>() as u32;
        unsafe {
            CryptMsgGetParam(
                message.0,
                CMSG_SIGNER_COUNT_PARAM,
                0,
                Some((&mut signer_count as *mut u32).cast()),
                &mut signer_count_bytes,
            )
        }
        .map_err(|_| WindowsInspectionFailure::SignatureInvalid)?;
        if signer_count != 1 || signer_count_bytes != size_of::<u32>() as u32 {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }

        let stores = [store.0];
        let mut raw_signer = ptr::null_mut::<CERT_CONTEXT>();
        unsafe {
            CryptMsgGetAndVerifySigner(message.0, Some(&stores), 0, Some(&mut raw_signer), None)
        }
        .map_err(|_| WindowsInspectionFailure::SignatureInvalid)?;
        let signer = OwnedCertContext(raw_signer);
        if signer.0.is_null() {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }
        certificate_simple_name(signer.0)
    }

    fn certificate_simple_name(
        context: *const CERT_CONTEXT,
    ) -> Result<String, WindowsInspectionFailure> {
        let required =
            unsafe { CertGetNameStringW(context, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, None) };
        if required <= 1 || required > 2048 {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }
        let mut buffer = vec![0_u16; required as usize];
        let written = unsafe {
            CertGetNameStringW(
                context,
                CERT_NAME_SIMPLE_DISPLAY_TYPE,
                0,
                None,
                Some(&mut buffer),
            )
        };
        if written <= 1 || written != required {
            return Err(WindowsInspectionFailure::SignatureInvalid);
        }
        let value = String::from_utf16(&buffer[..written as usize - 1])
            .map_err(|_| WindowsInspectionFailure::SignatureInvalid)?;
        bounded_registry_string(value).ok_or(WindowsInspectionFailure::SignatureInvalid)
    }

    struct OwnedCertStore(HCERTSTORE);

    impl Drop for OwnedCertStore {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                let _ = unsafe { CertCloseStore(Some(self.0), 0) };
            }
        }
    }

    struct OwnedCryptMessage(*mut c_void);

    impl Drop for OwnedCryptMessage {
        fn drop(&mut self) {
            if !self.0.is_null() {
                let _ = unsafe { CryptMsgClose(Some(self.0)) };
            }
        }
    }

    struct OwnedCertContext(*mut CERT_CONTEXT);

    impl Drop for OwnedCertContext {
        fn drop(&mut self) {
            if !self.0.is_null() {
                let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
            }
        }
    }

    fn wide_path(path: &Path) -> Result<Vec<u16>, WindowsInspectionFailure> {
        if !path.is_absolute() || path.as_os_str().is_empty() {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        let mut wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if wide.len() > 32 * 1024 || wide.contains(&0) {
            return Err(WindowsInspectionFailure::UnsafeFileShape);
        }
        wide.push(0);
        Ok(wide)
    }

    fn wide_string(value: &str) -> Vec<u16> {
        OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::*;
    use crate::services::external_agents::AgentCatalogId;

    struct FakeInspector {
        outcomes: HashMap<PathBuf, Result<InspectedWindowsExecutable, WindowsInspectionFailure>>,
    }

    impl WindowsExecutableInspector for FakeInspector {
        fn inspect(
            &self,
            path: &Path,
            _policy: &WindowsProductPolicy,
        ) -> Result<InspectedWindowsExecutable, WindowsInspectionFailure> {
            self.outcomes
                .get(path)
                .cloned()
                .unwrap_or(Err(WindowsInspectionFailure::Missing))
        }
    }

    fn product() -> DesktopProduct {
        DesktopProduct {
            agent_id: AgentCatalogId::WorkBuddy,
            macos_bundle_id: "com.workbuddy.workbuddy",
            windows_product_names: &["WorkBuddy"],
            windows_relative_exes: &["WorkBuddy/WorkBuddy.exe"],
        }
    }

    #[test]
    fn trusted_file_identity_merges_registry_and_known_path_evidence_later() {
        let user_root = PathBuf::from("C:/Users/test/AppData/Local/Programs");
        let path = user_root.join("WorkBuddy/WorkBuddy.exe");
        let inspector = FakeInspector {
            outcomes: HashMap::from([(
                path.clone(),
                Ok(InspectedWindowsExecutable {
                    stable_file_key: "windows-file:1:2".into(),
                    product_version: Some("5.3.14".into()),
                    machine: 0x8664,
                }),
            )]),
        };
        let rows = normalize_hints(
            &product(),
            std::slice::from_ref(&user_root),
            vec![
                WindowsPathHint {
                    path: path.clone(),
                    source: WindowsEvidenceSource::KnownPath,
                    registration_scope: InstallationScope::CurrentUser,
                    registration_version: None,
                },
                WindowsPathHint {
                    path,
                    source: WindowsEvidenceSource::AppPaths,
                    registration_scope: InstallationScope::CurrentUser,
                    registration_version: Some("5.3.14".into()),
                },
            ],
            &inspector,
        );
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.stable_key == "windows-file:1:2"));
        assert!(rows
            .iter()
            .all(|row| row.launch_eligible && row.update_eligible));
    }

    #[test]
    fn stale_registration_remains_visible_but_known_path_absence_does_not() {
        let path = PathBuf::from("C:/Missing/WorkBuddy.exe");
        let inspector = FakeInspector {
            outcomes: HashMap::new(),
        };
        let rows = normalize_hints(
            &product(),
            &[],
            vec![
                WindowsPathHint {
                    path: path.clone(),
                    source: WindowsEvidenceSource::KnownPath,
                    registration_scope: InstallationScope::CurrentUser,
                    registration_version: None,
                },
                WindowsPathHint {
                    path,
                    source: WindowsEvidenceSource::Uninstall,
                    registration_scope: InstallationScope::AllUsers,
                    registration_version: Some("5.3.14".into()),
                },
            ],
            &inspector,
        );
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].launch_eligible);
        assert_eq!(rows[0].reason_codes, [AgentReasonCode::TargetNotExecutable]);
        assert_eq!(rows[0].scope, InstallationScope::Custom);
    }

    #[test]
    fn registry_discovery_outside_known_roots_is_custom_not_the_registry_hive_scope() {
        let path = PathBuf::from("D:/Apps/WorkBuddy/WorkBuddy.exe");
        let inspector = FakeInspector {
            outcomes: HashMap::from([(
                path.clone(),
                Ok(InspectedWindowsExecutable {
                    stable_file_key: "windows-file:9:9".into(),
                    product_version: Some("5.3.14".into()),
                    machine: 0x8664,
                }),
            )]),
        };
        let rows = normalize_hints(
            &product(),
            &[
                PathBuf::from("C:/Users/test/AppData/Local/Programs"),
                PathBuf::from("C:/Program Files"),
            ],
            vec![WindowsPathHint {
                path,
                source: WindowsEvidenceSource::Uninstall,
                registration_scope: InstallationScope::AllUsers,
                registration_version: Some("5.3.14".into()),
            }],
            &inspector,
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scope, InstallationScope::Custom);
        assert!(rows[0].launch_eligible);
    }

    #[test]
    fn signer_or_product_mismatch_never_becomes_actionable() {
        for failure in [
            WindowsInspectionFailure::ProductIdentityMismatch,
            WindowsInspectionFailure::SignatureInvalid,
            WindowsInspectionFailure::SignerMismatch,
            WindowsInspectionFailure::ArchitectureUnsupported,
            WindowsInspectionFailure::IdentityChanged,
            WindowsInspectionFailure::UnsafeFileShape,
            WindowsInspectionFailure::Unavailable,
        ] {
            let path = PathBuf::from("C:/Programs/WorkBuddy.exe");
            let inspector = FakeInspector {
                outcomes: HashMap::from([(path.clone(), Err(failure))]),
            };
            let rows = normalize_hints(
                &product(),
                &[],
                vec![WindowsPathHint {
                    path,
                    source: WindowsEvidenceSource::AppPaths,
                    registration_scope: InstallationScope::CurrentUser,
                    registration_version: None,
                }],
                &inspector,
            );
            assert_eq!(rows.len(), 1);
            assert!(!rows[0].launch_eligible);
            assert!(!rows[0].update_eligible);
        }
    }

    #[test]
    fn display_icon_parser_never_treats_arguments_as_a_path() {
        assert_eq!(
            display_icon_path(r#""C:\Apps\WorkBuddy.exe",0"#),
            Some(PathBuf::from(r"C:\Apps\WorkBuddy.exe"))
        );
        assert_eq!(
            display_icon_path(r"C:\Apps\WorkBuddy.exe,12"),
            Some(PathBuf::from(r"C:\Apps\WorkBuddy.exe"))
        );
        assert!(display_icon_path("\"C:\\Apps\\WorkBuddy.exe -flag").is_none());
    }

    #[test]
    fn current_products_have_reviewed_signer_subjects() {
        for agent_id in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
        ] {
            let product = crate::agent_install::desktop::desktop_product(agent_id)
                .expect("every managed desktop agent has one closed product policy");
            let policy = product_policy(product);
            assert!(!policy.product_names.is_empty());
            assert!(!policy.signer_subjects.is_empty());
            if agent_id == AgentCatalogId::TraeWork {
                assert!(policy.product_names.contains(&"TraeWork CN"));
            }
        }
    }
}
