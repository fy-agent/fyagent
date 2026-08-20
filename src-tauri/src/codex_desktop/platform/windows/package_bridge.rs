//! Handle-pinned ProgramData bridge for one current-user MSIX deployment.
//!
//! The bridge is deliberately separate from install-root staging. The parent
//! copies only from an already verified source handle into a BA-owned namespace
//! that the frozen Shell user may read but cannot rebind. Every path component
//! below the resolved volume root is opened relative to a held parent handle.

use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    mem::{align_of, offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{FileExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf},
};

use fyagent_user_helper::{
    BridgeOperationId, PackageBridgeControl, PinnedPackageIdentity, BRIDGE_OPERATION_ID_BYTES,
    INSTALLER_FILE_NAME, PACKAGE_BRIDGE_PART_FILE_NAME, PACKAGE_BRIDGE_ROOT_DIRECTORY,
    PACKAGE_BRIDGE_VERSION_DIRECTORY,
};
use sha2::{Digest, Sha256};
use windows::{
    core::{BOOL, HRESULT, PCWSTR, PWSTR},
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FileRenameInformation, NtCreateFile, NtSetInformationFile, FILE_CREATE,
            FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_IF,
            FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION, FILE_SYNCHRONOUS_IO_NONALERT,
            NTCREATEFILE_CREATE_DISPOSITION, NTCREATEFILE_CREATE_OPTIONS,
        },
    },
    Win32::{
        Foundation::{
            CloseHandle, ERROR_NO_MORE_FILES, HANDLE, HLOCAL, HWND, INVALID_HANDLE_VALUE,
            OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, UNICODE_STRING,
        },
        Security::{
            AccessCheck, AclSizeInformation,
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
                GetSecurityInfo, SDDL_REVISION_1, SE_FILE_OBJECT,
            },
            CheckTokenMembership, CreateWellKnownSid, DuplicateToken, EqualSid, GetAce,
            GetAclInformation, GetLengthSid, GetSecurityDescriptorControl, GetTokenInformation,
            IsValidSid, IsWellKnownSid, SecurityImpersonation, TokenUser, WinAuthenticatedUserSid,
            WinBuiltinAdministratorsSid, WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACL_REVISION,
            ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, GENERIC_MAPPING,
            GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PRIVILEGE_SET,
            PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE, SE_DACL_AUTO_INHERITED,
            SE_DACL_AUTO_INHERIT_REQ, SE_DACL_DEFAULTED, SE_DACL_PRESENT, SE_DACL_PROTECTED,
            SE_GROUP_DEFAULTED, SE_OWNER_DEFAULTED, TOKEN_DUPLICATE, TOKEN_QUERY, TOKEN_USER,
        },
        Storage::FileSystem::{
            FileDispositionInfo, FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo,
            FileStandardInfo, FlushFileBuffers, GetDiskFreeSpaceExW, GetDriveTypeW,
            GetFileInformationByHandle, GetFileInformationByHandleEx, GetVolumeInformationW,
            GetVolumePathNameW, SetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, DELETE,
            FILE_ACCESS_RIGHTS, FILE_ALL_ACCESS, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT, FILE_DELETE_CHILD,
            FILE_DISPOSITION_INFO, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
            FILE_ID_BOTH_DIR_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_MODE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, FILE_TRAVERSE,
            FILE_WRITE_ATTRIBUTES, FILE_WRITE_EA, READ_CONTROL, SYNCHRONIZE, WRITE_DAC,
            WRITE_OWNER,
        },
        System::{
            Com::CoTaskMemFree,
            Threading::{OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION},
            WindowsProgramming::DRIVE_FIXED,
            IO::IO_STATUS_BLOCK,
        },
        UI::{
            Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG},
            WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId},
        },
    },
};

use crate::{
    codex_desktop::error::{InstallerError, InstallerErrorCode},
    windows_runtime::is_canonical_sid,
};

// These masks are protocol facts shared with the helper's exact ACL verifier.
// They intentionally contain no inheritance flags.
const ADMINISTRATORS_FULL_MASK: u32 = 0x001f_01ff;
const DIRECTORY_READ_TRAVERSE_MASK: u32 = 0x0012_00a9;
const DIRECTORY_TRAVERSE_ONLY_MASK: u32 = 0x0012_00a0;
const FILE_READ_MASK: u32 = 0x0012_0089;
const ACCESS_ALLOWED_ACE_TYPE_VALUE: u8 = 0;
const SECURITY_DESCRIPTOR_REVISION_VALUE: u32 = 1;
const FILE_PERSISTENT_ACLS_FLAG: u32 = 0x0000_0008;
const MAXIMUM_ALLOWED_MASK: u32 = 0x0200_0000;
const COPY_BUFFER_BYTES: usize = 128 * 1024;
const ACCESS_CHECK_BUFFER_BYTES: usize = 4 * 1024;
const DIRECTORY_ENUMERATION_BUFFER_BYTES: usize = 64 * 1024;
const MAX_ORPHAN_DIRECTORY_ENTRIES: usize = 256;
const MAX_ORPHAN_ENUMERATION_BATCHES: usize = 16;
const MAX_OPERATION_LEAF_ENTRIES: usize = 8;
const MAX_OPERATION_ENUMERATION_BATCHES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DescriptorKind {
    StableDirectory,
    OperationDirectory,
    PackageLeaf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedPrincipal {
    Administrators,
    System,
    AuthenticatedUsers,
    ShellUser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExpectedAce {
    principal: ExpectedPrincipal,
    mask: u32,
}

const STABLE_DIRECTORY_ACES: [ExpectedAce; 3] = [
    ExpectedAce {
        principal: ExpectedPrincipal::Administrators,
        mask: ADMINISTRATORS_FULL_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::System,
        mask: DIRECTORY_READ_TRAVERSE_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::AuthenticatedUsers,
        mask: DIRECTORY_TRAVERSE_ONLY_MASK,
    },
];

const OPERATION_DIRECTORY_ACES: [ExpectedAce; 3] = [
    ExpectedAce {
        principal: ExpectedPrincipal::Administrators,
        mask: ADMINISTRATORS_FULL_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::System,
        mask: DIRECTORY_READ_TRAVERSE_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::ShellUser,
        mask: DIRECTORY_READ_TRAVERSE_MASK,
    },
];

const PACKAGE_LEAF_ACES: [ExpectedAce; 3] = [
    ExpectedAce {
        principal: ExpectedPrincipal::Administrators,
        mask: ADMINISTRATORS_FULL_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::System,
        mask: FILE_READ_MASK,
    },
    ExpectedAce {
        principal: ExpectedPrincipal::ShellUser,
        mask: FILE_READ_MASK,
    },
];

impl DescriptorKind {
    const fn expected_aces(self) -> &'static [ExpectedAce] {
        match self {
            Self::StableDirectory => &STABLE_DIRECTORY_ACES,
            Self::OperationDirectory => &OPERATION_DIRECTORY_ACES,
            Self::PackageLeaf => &PACKAGE_LEAF_ACES,
        }
    }
}

/// Returns the real CommonApplicationData directory used for bridge-volume
/// capacity probing. There is deliberately no environment or drive fallback.
pub(super) fn program_data_bridge_probe_path() -> Result<PathBuf, InstallerError> {
    resolve_program_data_path()
}

/// One sealed bridge capability.
///
/// Ordinary drop only closes handles. It never performs pathname cleanup. A
/// caller with indeterminate post-admission state must retain this value for
/// the process lifetime; a settled caller may invoke [`Self::cleanup`].
pub(super) struct ProtectedPackageBridge {
    shell_sid: String,
    anchor: ProgramDataAnchor,
    root: HeldObject,
    version: HeldObject,
    operation: HeldObject,
    final_file: Option<File>,
    final_identity: NativeFileIdentity,
    expected_sha256: [u8; 32],
    control: PackageBridgeControl,
}

impl ProtectedPackageBridge {
    pub(super) fn create(
        shell_sid: &str,
        source_file: &mut File,
        expected_size: u64,
        expected_sha256: &str,
    ) -> Result<Self, InstallerError> {
        if expected_size == 0 || !is_canonical_sid(shell_sid) {
            return Err(bridge_integrity_error(
                "the package bridge input identity was invalid",
            ));
        }
        let expected_sha256 = decode_sha256(expected_sha256).ok_or_else(|| {
            bridge_integrity_error("the package bridge checksum was not canonical")
        })?;
        let shell_sid_binary = OwnedSid::parse(shell_sid)?;

        let source_before = native_file_identity(source_file, NativeObjectKind::RegularFile)?;
        if source_before.size != expected_size {
            return Err(bridge_integrity_error(
                "the verified package source size changed before bridging",
            ));
        }

        let anchor = ProgramDataAnchor::open(shell_sid_binary.as_sid(), expected_size)?;
        let stable_descriptor =
            OwnedSecurityDescriptor::for_kind(DescriptorKind::StableDirectory, shell_sid)?;
        let operation_descriptor =
            OwnedSecurityDescriptor::for_kind(DescriptorKind::OperationDirectory, shell_sid)?;
        let leaf_descriptor =
            OwnedSecurityDescriptor::for_kind(DescriptorKind::PackageLeaf, shell_sid)?;

        let root = create_or_open_directory(
            anchor.program_data_file(),
            PACKAGE_BRIDGE_ROOT_DIRECTORY,
            RelativeDisposition::OpenIf,
            &stable_descriptor,
        )?;
        verify_exact_descriptor(
            &root.file,
            DescriptorKind::StableDirectory,
            shell_sid_binary.as_sid(),
        )?;
        anchor.recheck(shell_sid_binary.as_sid())?;

        let version = create_or_open_directory(
            &root.file,
            PACKAGE_BRIDGE_VERSION_DIRECTORY,
            RelativeDisposition::OpenIf,
            &stable_descriptor,
        )?;
        verify_exact_descriptor(
            &version.file,
            DescriptorKind::StableDirectory,
            shell_sid_binary.as_sid(),
        )?;
        root.recheck()?;

        cleanup_bounded_orphans(&version.file, shell_sid_binary.as_sid());

        let operation_id = generate_operation_id()?;
        let operation_name = operation_id.directory_name();
        let operation = create_or_open_directory(
            &version.file,
            &operation_name,
            RelativeDisposition::Create,
            &operation_descriptor,
        )?;
        verify_exact_descriptor(
            &operation.file,
            DescriptorKind::OperationDirectory,
            shell_sid_binary.as_sid(),
        )?;
        version.recheck()?;

        let part = create_package_leaf(
            &operation.file,
            PACKAGE_BRIDGE_PART_FILE_NAME,
            &leaf_descriptor,
        )?;
        verify_exact_descriptor(
            &part,
            DescriptorKind::PackageLeaf,
            shell_sid_binary.as_sid(),
        )?;

        let copied_sha256 = copy_exact_from_source(source_file, &part, expected_size)?;
        if copied_sha256 != expected_sha256 {
            return Err(bridge_checksum_error(
                "the package bridge copy checksum did not match",
            ));
        }
        unsafe { FlushFileBuffers(HANDLE(part.as_raw_handle())) }
            .map_err(|_| bridge_error("the package bridge copy could not be flushed"))?;

        let part_identity = native_file_identity(&part, NativeObjectKind::RegularFile)?;
        if part_identity.size != expected_size || part_identity.number_of_links != 1 {
            return Err(bridge_integrity_error(
                "the package bridge partial file identity was invalid",
            ));
        }
        rename_leaf_without_replacement(&part, INSTALLER_FILE_NAME)?;
        if native_file_identity(&part, NativeObjectKind::RegularFile)? != part_identity {
            return Err(bridge_integrity_error(
                "the package bridge file identity changed during finalization",
            ));
        }
        drop(part);

        let final_file = open_final_package_leaf(&operation.file)?;
        let final_identity = native_file_identity(&final_file, NativeObjectKind::RegularFile)?;
        if final_identity != part_identity
            || final_identity.volume_serial != anchor.volume_serial
            || final_identity.size != expected_size
            || final_identity.number_of_links != 1
        {
            return Err(bridge_integrity_error(
                "the sealed package bridge identity was invalid",
            ));
        }
        verify_exact_descriptor(
            &final_file,
            DescriptorKind::PackageLeaf,
            shell_sid_binary.as_sid(),
        )?;
        if hash_exact_file(&final_file, expected_size)? != expected_sha256 {
            return Err(bridge_checksum_error(
                "the sealed package bridge checksum did not match",
            ));
        }

        let source_after = native_file_identity(source_file, NativeObjectKind::RegularFile)?;
        if source_after != source_before {
            return Err(bridge_integrity_error(
                "the verified package source identity changed while bridging",
            ));
        }

        let package_identity = PinnedPackageIdentity::new(
            final_identity.volume_serial,
            final_identity.file_index,
            final_identity.size,
        );
        let control = PackageBridgeControl::new(operation_id, package_identity)
            .map_err(|_| bridge_integrity_error("the package bridge control was invalid"))?;
        let bridge = Self {
            shell_sid: shell_sid.to_owned(),
            anchor,
            root,
            version,
            operation,
            final_file: Some(final_file),
            final_identity,
            expected_sha256,
            control,
        };
        bridge.recheck()?;
        Ok(bridge)
    }

    pub(super) const fn control(&self) -> PackageBridgeControl {
        self.control
    }

    pub(super) const fn identity(&self) -> PinnedPackageIdentity {
        self.control.package()
    }

    pub(super) fn recheck(&self) -> Result<(), InstallerError> {
        if !is_canonical_sid(&self.shell_sid) {
            return Err(bridge_integrity_error(
                "the package bridge Shell identity was invalid",
            ));
        }
        // Keep the raw PSID local to each native call sequence. The bridge may
        // move into the process-lifetime quarantine, while LocalAlloc pointers
        // do not carry a Rust Send proof even though the rendered SID is stable.
        let shell_sid_binary = OwnedSid::parse(&self.shell_sid)?;
        self.anchor.recheck(shell_sid_binary.as_sid())?;
        self.root.recheck()?;
        self.version.recheck()?;
        self.operation.recheck()?;
        verify_exact_descriptor(
            &self.root.file,
            DescriptorKind::StableDirectory,
            shell_sid_binary.as_sid(),
        )?;
        verify_exact_descriptor(
            &self.version.file,
            DescriptorKind::StableDirectory,
            shell_sid_binary.as_sid(),
        )?;
        verify_exact_descriptor(
            &self.operation.file,
            DescriptorKind::OperationDirectory,
            shell_sid_binary.as_sid(),
        )?;

        let final_file = self.final_file.as_ref().ok_or_else(|| {
            bridge_integrity_error("the sealed package bridge handle was unavailable")
        })?;
        let identity = native_file_identity(final_file, NativeObjectKind::RegularFile)?;
        if identity != self.final_identity || identity.number_of_links != 1 {
            return Err(bridge_integrity_error(
                "the sealed package bridge identity changed",
            ));
        }
        verify_exact_descriptor(
            final_file,
            DescriptorKind::PackageLeaf,
            shell_sid_binary.as_sid(),
        )?;
        if hash_exact_file(final_file, identity.size)? != self.expected_sha256 {
            return Err(bridge_checksum_error(
                "the sealed package bridge checksum changed",
            ));
        }
        if self.identity()
            != PinnedPackageIdentity::new(
                identity.volume_serial,
                identity.file_index,
                identity.size,
            )
        {
            return Err(bridge_integrity_error(
                "the package bridge control identity changed",
            ));
        }
        Ok(())
    }

    /// Removes only the exact sealed leaf and empty operation directory through
    /// their held parent capabilities. Stable root/version directories remain.
    pub(super) fn cleanup(mut self) -> Result<(), InstallerError> {
        self.recheck()?;
        let shell_sid_binary = OwnedSid::parse(&self.shell_sid)?;
        drop(self.final_file.take());

        let delete_file = open_relative(
            &self.operation.file,
            INSTALLER_FILE_NAME,
            (DELETE | READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
            FILE_SHARE_READ.0,
            RelativeDisposition::Open,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
            None,
        )?;
        if native_file_identity(&delete_file, NativeObjectKind::RegularFile)? != self.final_identity
        {
            return Err(bridge_integrity_error(
                "the package bridge cleanup leaf identity changed",
            ));
        }
        verify_exact_descriptor(
            &delete_file,
            DescriptorKind::PackageLeaf,
            shell_sid_binary.as_sid(),
        )?;
        mark_handle_for_deletion(&delete_file)?;
        drop(delete_file);

        self.operation.recheck()?;
        let operation_identity = self.operation.identity;
        drop(self.operation);
        let operation_name = self.control.operation_id().directory_name();
        let delete_operation = open_relative(
            &self.version.file,
            &operation_name,
            (FILE_GENERIC_READ | FILE_TRAVERSE | DELETE).0,
            FILE_SHARE_READ.0,
            RelativeDisposition::Open,
            FILE_ATTRIBUTE_DIRECTORY.0,
            (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
            None,
        )?;
        if native_file_identity(&delete_operation, NativeObjectKind::Directory)?
            != operation_identity
        {
            return Err(bridge_integrity_error(
                "the package bridge cleanup directory identity changed",
            ));
        }
        verify_exact_descriptor(
            &delete_operation,
            DescriptorKind::OperationDirectory,
            shell_sid_binary.as_sid(),
        )?;
        mark_handle_for_deletion(&delete_operation)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeObjectKind {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NativeFileIdentity {
    volume_serial: u64,
    file_index: u64,
    size: u64,
    number_of_links: u32,
}

struct HeldObject {
    file: File,
    identity: NativeFileIdentity,
    kind: NativeObjectKind,
}

impl HeldObject {
    fn capture(file: File, kind: NativeObjectKind) -> Result<Self, InstallerError> {
        let identity = native_file_identity(&file, kind)?;
        Ok(Self {
            file,
            identity,
            kind,
        })
    }

    fn recheck(&self) -> Result<(), InstallerError> {
        if native_file_identity(&self.file, self.kind)? != self.identity {
            return Err(bridge_integrity_error(
                "a held package bridge ancestor identity changed",
            ));
        }
        Ok(())
    }
}

struct ProgramDataAnchor {
    held_directories: Vec<HeldObject>,
    volume_serial: u64,
}

impl ProgramDataAnchor {
    fn open(shell_sid: PSID, expected_size: u64) -> Result<Self, InstallerError> {
        let program_data = resolve_program_data_path()?;
        let volume = probe_program_data_volume(&program_data)?;
        ensure_program_data_capacity(&program_data, expected_size)?;

        let volume_root = open_absolute_directory_no_follow(&volume.root)?;
        let volume_root = HeldObject::capture(volume_root, NativeObjectKind::Directory)?;
        if volume_root.identity.volume_serial != volume.serial {
            return Err(bridge_integrity_error(
                "the ProgramData volume identity was inconsistent",
            ));
        }
        let mut held_directories = vec![volume_root];
        let relative = program_data.strip_prefix(&volume.root).map_err(|_| {
            bridge_integrity_error("the ProgramData path was outside its resolved volume")
        })?;
        let mut component_count = 0_usize;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(bridge_integrity_error(
                    "the ProgramData path contained an unsafe component",
                ));
            };
            component_count += 1;
            let file = open_relative(
                &held_directories
                    .last()
                    .expect("volume root is always held")
                    .file,
                name,
                (READ_CONTROL | FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE).0,
                (FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0,
                RelativeDisposition::Open,
                FILE_ATTRIBUTE_DIRECTORY.0,
                (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
                None,
            )?;
            let held = HeldObject::capture(file, NativeObjectKind::Directory)?;
            if held.identity.volume_serial != volume.serial {
                return Err(bridge_integrity_error(
                    "a ProgramData ancestor crossed the resolved volume",
                ));
            }
            held_directories.push(held);
        }
        if component_count == 0 {
            return Err(bridge_integrity_error(
                "the ProgramData path did not identify a directory below its volume root",
            ));
        }

        let anchor = Self {
            held_directories,
            volume_serial: volume.serial,
        };
        anchor.recheck(shell_sid)?;
        Ok(anchor)
    }

    fn program_data_file(&self) -> &File {
        &self
            .held_directories
            .last()
            .expect("validated ProgramData anchor")
            .file
    }

    fn recheck(&self, shell_sid: PSID) -> Result<(), InstallerError> {
        let token = shell_access_check_token(shell_sid)?;
        for held in &self.held_directories {
            held.recheck()?;
            if held.identity.volume_serial != self.volume_serial {
                return Err(bridge_integrity_error(
                    "a ProgramData ancestor volume identity changed",
                ));
            }

            let granted = effective_file_access(&held.file, token.raw())?;
            // Creating unrelated ProgramData siblings is a normal Windows
            // capability and cannot rebind an exact existing child. Deleting
            // a fixed child, deleting/mutating the held component itself, or
            // rewriting its descriptor/owner would invalidate the namespace.
            let dangerous = FILE_DELETE_CHILD.0
                | DELETE.0
                | WRITE_DAC.0
                | WRITE_OWNER.0
                | FILE_WRITE_EA.0
                | FILE_WRITE_ATTRIBUTES.0;
            let granted_dangerous = granted & dangerous;
            // A non-administrator Alice must not be able to delete or re-ACL
            // C:\ or ProgramData. Built-in Administrator / UAC-disabled Explorer
            // tokens already have those OS rights; fail-closed would block install
            // without reducing capability. Exact child ACLs still apply.
            if ancestor_mutation_rejected(
                granted_dangerous,
                token_is_local_administrator(token.raw())?,
            ) {
                return Err(bridge_integrity_error(
                    "the Shell user could mutate a ProgramData bridge ancestor",
                ));
            }
        }
        Ok(())
    }
}

struct ProgramDataVolume {
    root: PathBuf,
    serial: u64,
}

fn resolve_program_data_path() -> Result<PathBuf, InstallerError> {
    let raw = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KNOWN_FOLDER_FLAG(0), None) }
        .map_err(|_| bridge_error("the ProgramData known folder could not be resolved"))?;
    if raw.is_null() {
        return Err(bridge_error("the ProgramData known folder was unavailable"));
    }
    let mut length = 0_usize;
    unsafe {
        while *raw.0.add(length) != 0 {
            length += 1;
        }
    }
    let value = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(raw.0, length)
    }));
    unsafe { CoTaskMemFree(Some(raw.0.cast())) };
    if !value.is_absolute() || value.as_os_str().is_empty() {
        return Err(bridge_error(
            "the ProgramData known folder path was invalid",
        ));
    }
    Ok(value)
}

fn probe_program_data_volume(program_data: &Path) -> Result<ProgramDataVolume, InstallerError> {
    let program_data = wide_null(program_data.as_os_str())?;
    let mut root = vec![0_u16; 32_768];
    unsafe { GetVolumePathNameW(PCWSTR(program_data.as_ptr()), &mut root) }
        .map_err(|_| bridge_error("the ProgramData volume path could not be resolved"))?;
    let root_length = root
        .iter()
        .position(|value| *value == 0)
        .ok_or_else(|| bridge_error("the ProgramData volume path was not null terminated"))?;
    if root_length == 0 {
        return Err(bridge_error("the ProgramData volume path was empty"));
    }
    root.truncate(root_length + 1);
    if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != DRIVE_FIXED {
        return Err(bridge_integrity_error(
            "the ProgramData bridge requires a local fixed volume",
        ));
    }

    let mut serial = 0_u32;
    let mut flags = 0_u32;
    let mut filesystem = vec![0_u16; 64];
    unsafe {
        GetVolumeInformationW(
            PCWSTR(root.as_ptr()),
            None,
            Some(&mut serial),
            None,
            Some(&mut flags),
            Some(&mut filesystem),
        )
    }
    .map_err(|_| bridge_error("the ProgramData volume information could not be queried"))?;
    let filesystem_length = filesystem
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(filesystem.len());
    let filesystem = OsString::from_wide(&filesystem[..filesystem_length]);
    if !filesystem.to_string_lossy().eq_ignore_ascii_case("NTFS")
        || flags & FILE_PERSISTENT_ACLS_FLAG == 0
    {
        return Err(bridge_integrity_error(
            "the ProgramData bridge requires NTFS persistent ACLs",
        ));
    }

    Ok(ProgramDataVolume {
        root: PathBuf::from(OsString::from_wide(&root[..root_length])),
        serial: u64::from(serial),
    })
}

fn ensure_program_data_capacity(
    program_data: &Path,
    expected_size: u64,
) -> Result<(), InstallerError> {
    let program_data = wide_null(program_data.as_os_str())?;
    let mut available = 0_u64;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(program_data.as_ptr()),
            Some(&mut available),
            None,
            None,
        )
    }
    .map_err(|_| bridge_error("the ProgramData free space could not be queried"))?;
    if available < expected_size {
        return Err(
            InstallerError::new(InstallerErrorCode::InsufficientDiskSpace)
                .with_retryable(true)
                .with_diagnostic_message("the ProgramData volume did not have enough free space"),
        );
    }
    Ok(())
}

fn open_absolute_directory_no_follow(path: &Path) -> Result<File, InstallerError> {
    let mut options = OpenOptions::new();
    options
        .access_mode((READ_CONTROL | FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE).0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0);
    options
        .open(path)
        .map_err(|_| bridge_error("the ProgramData volume root could not be opened safely"))
}

#[derive(Clone, Copy)]
enum RelativeDisposition {
    Open,
    Create,
    OpenIf,
}

#[allow(clippy::too_many_arguments)]
fn open_relative(
    parent: &File,
    name: impl AsRef<OsStr>,
    desired_access: u32,
    share_access: u32,
    disposition: RelativeDisposition,
    attributes: u32,
    create_options: u32,
    security: Option<&OwnedSecurityDescriptor>,
) -> Result<File, InstallerError> {
    let name = name.as_ref();
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(component)) if component == name)
        || components.next().is_some()
    {
        return Err(bridge_integrity_error(
            "the package bridge relative name was invalid",
        ));
    }
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(bridge_integrity_error(
            "the package bridge relative name was invalid",
        ));
    }
    let byte_length = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|value| u16::try_from(value).ok())
        .ok_or_else(|| bridge_integrity_error("the package bridge relative name was too long"))?;
    let object_name = UNICODE_STRING {
        Length: byte_length,
        MaximumLength: byte_length,
        Buffer: PWSTR(wide.as_mut_ptr()),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &object_name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        SecurityDescriptor: security
            .map(OwnedSecurityDescriptor::as_nt_descriptor)
            .unwrap_or(std::ptr::null()),
        SecurityQualityOfService: std::ptr::null(),
    };
    let disposition: NTCREATEFILE_CREATE_DISPOSITION = match disposition {
        RelativeDisposition::Open => FILE_OPEN,
        RelativeDisposition::Create => FILE_CREATE,
        RelativeDisposition::OpenIf => FILE_OPEN_IF,
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
            FILE_FLAGS_AND_ATTRIBUTES(attributes),
            FILE_SHARE_MODE(share_access),
            disposition,
            NTCREATEFILE_CREATE_OPTIONS(create_options),
            None,
            0,
        )
    };
    if status.is_err() || handle.0.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(bridge_integrity_error(
            "the package bridge object could not be opened or created safely",
        ));
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

fn create_or_open_directory(
    parent: &File,
    name: &str,
    disposition: RelativeDisposition,
    security: &OwnedSecurityDescriptor,
) -> Result<HeldObject, InstallerError> {
    let file = open_relative(
        parent,
        name,
        (FILE_GENERIC_READ | FILE_TRAVERSE).0,
        FILE_SHARE_READ.0,
        disposition,
        FILE_ATTRIBUTE_DIRECTORY.0,
        (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        Some(security),
    )?;
    HeldObject::capture(file, NativeObjectKind::Directory)
}

fn create_package_leaf(
    operation: &File,
    name: &str,
    security: &OwnedSecurityDescriptor,
) -> Result<File, InstallerError> {
    let file = open_relative(
        operation,
        name,
        (FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE | READ_CONTROL | SYNCHRONIZE).0,
        0,
        RelativeDisposition::Create,
        FILE_ATTRIBUTE_NORMAL.0,
        (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        Some(security),
    )?;
    native_file_identity(&file, NativeObjectKind::RegularFile)?;
    Ok(file)
}

fn open_final_package_leaf(operation: &File) -> Result<File, InstallerError> {
    let file = open_relative(
        operation,
        INSTALLER_FILE_NAME,
        FILE_GENERIC_READ.0,
        FILE_SHARE_READ.0,
        RelativeDisposition::Open,
        FILE_ATTRIBUTE_NORMAL.0,
        (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        None,
    )?;
    native_file_identity(&file, NativeObjectKind::RegularFile)?;
    Ok(file)
}

fn native_file_identity(
    file: &File,
    kind: NativeObjectKind,
) -> Result<NativeFileIdentity, InstallerError> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| bridge_integrity_error("a package bridge object identity was unavailable"))?;
    let is_directory = information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let is_reparse = information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0;
    let is_remote_or_recalled = information.dwFileAttributes
        & (FILE_ATTRIBUTE_OFFLINE.0
            | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0)
        != 0;
    if is_reparse
        || is_remote_or_recalled
        || match kind {
            NativeObjectKind::Directory => !is_directory,
            NativeObjectKind::RegularFile => is_directory,
        }
    {
        return Err(bridge_integrity_error(
            "a package bridge object was a reparse point or unexpected type",
        ));
    }

    let mut standard = FILE_STANDARD_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileStandardInfo,
            (&mut standard as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )
    }
    .map_err(|_| {
        bridge_integrity_error("a package bridge object's standard identity was unavailable")
    })?;
    let standard_size = u64::try_from(standard.EndOfFile)
        .map_err(|_| bridge_integrity_error("a package bridge object reported a negative size"))?;
    let basic_size =
        (u64::from(information.nFileSizeHigh) << 32) | u64::from(information.nFileSizeLow);
    if standard.DeletePending
        || standard.Directory != is_directory
        || standard.NumberOfLinks != information.nNumberOfLinks
        || standard_size != basic_size
    {
        return Err(bridge_integrity_error(
            "a package bridge object was delete-pending or internally inconsistent",
        ));
    }
    Ok(NativeFileIdentity {
        volume_serial: u64::from(information.dwVolumeSerialNumber),
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        size: standard_size,
        number_of_links: information.nNumberOfLinks,
    })
}

fn copy_exact_from_source(
    source: &File,
    destination: &File,
    expected_size: u64,
) -> Result<[u8; 32], InstallerError> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < expected_size {
        let remaining = expected_size - offset;
        let requested = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("bounded by the copy buffer length");
        let read = source
            .seek_read(&mut buffer[..requested], offset)
            .map_err(|_| bridge_error("the verified package source could not be read"))?;
        if read == 0 {
            return Err(bridge_checksum_error(
                "the verified package source ended before its expected size",
            ));
        }
        write_all_at(destination, &buffer[..read], offset)?;
        hasher.update(&buffer[..read]);
        offset = offset
            .checked_add(read as u64)
            .ok_or_else(|| bridge_integrity_error("the package bridge copy length overflowed"))?;
    }
    let mut trailing = [0_u8; 1];
    if source
        .seek_read(&mut trailing, expected_size)
        .map_err(|_| bridge_error("the verified package source EOF could not be checked"))?
        != 0
    {
        return Err(bridge_checksum_error(
            "the verified package source exceeded its expected size",
        ));
    }
    Ok(hasher.finalize().into())
}

fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> Result<(), InstallerError> {
    while !bytes.is_empty() {
        let written = file
            .seek_write(bytes, offset)
            .map_err(|_| bridge_error("the package bridge copy could not be written"))?;
        if written == 0 {
            return Err(bridge_error(
                "the package bridge copy produced a short write",
            ));
        }
        offset = offset
            .checked_add(written as u64)
            .ok_or_else(|| bridge_integrity_error("the package bridge write offset overflowed"))?;
        bytes = &bytes[written..];
    }
    Ok(())
}

fn hash_exact_file(file: &File, expected_size: u64) -> Result<[u8; 32], InstallerError> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < expected_size {
        let remaining = expected_size - offset;
        let requested = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("bounded by the hash buffer length");
        let read = file
            .seek_read(&mut buffer[..requested], offset)
            .map_err(|_| bridge_error("the sealed package bridge could not be read"))?;
        if read == 0 {
            return Err(bridge_checksum_error(
                "the sealed package bridge ended before its expected size",
            ));
        }
        hasher.update(&buffer[..read]);
        offset = offset
            .checked_add(read as u64)
            .ok_or_else(|| bridge_integrity_error("the package bridge hash length overflowed"))?;
    }
    let mut trailing = [0_u8; 1];
    if file
        .seek_read(&mut trailing, expected_size)
        .map_err(|_| bridge_error("the sealed package bridge EOF could not be checked"))?
        != 0
    {
        return Err(bridge_checksum_error(
            "the sealed package bridge exceeded its expected size",
        ));
    }
    Ok(hasher.finalize().into())
}

fn rename_leaf_without_replacement(file: &File, final_name: &str) -> Result<(), InstallerError> {
    let wide = final_name.encode_utf16().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(bridge_integrity_error(
            "the package bridge final name was invalid",
        ));
    }
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| bridge_integrity_error("the package bridge rename length overflowed"))?;
    // The variable-length buffer starts with the complete inline structure.
    // Using only the FileName offset omits its inline WCHAR and trailing ABI
    // alignment, which NtSetInformationFile rejects on Windows.
    let buffer_size = rename_information_buffer_size(name_bytes)
        .ok_or_else(|| bridge_integrity_error("the package bridge rename buffer overflowed"))?;
    let word_size = size_of::<usize>();
    let mut storage = vec![0_usize; buffer_size.div_ceil(word_size)];
    let information = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        (*information).Anonymous.ReplaceIfExists = false;
        // A native simple name with a null root renames this already-pinned
        // source handle within its current directory. Passing the long-lived
        // operation handle as RootDirectory would require a second directory
        // open whose write sharing conflicts with the capability pin.
        (*information).RootDirectory = HANDLE::default();
        (*information).FileNameLength = u32::try_from(name_bytes)
            .map_err(|_| bridge_integrity_error("the package bridge final name was too long"))?;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            (*information).FileName.as_mut_ptr(),
            wide.len(),
        );
        NtSetInformationFile(
            HANDLE(file.as_raw_handle()),
            &mut io_status,
            information.cast(),
            u32::try_from(buffer_size).map_err(|_| {
                bridge_integrity_error("the package bridge rename buffer was too large")
            })?,
            FileRenameInformation,
        )
    };
    if status.is_err() {
        return Err(bridge_integrity_error(
            "the package bridge partial file could not be finalized",
        )
        .with_platform_error_code(format!("NTSTATUS 0x{:08X}", status.0 as u32)));
    }
    Ok(())
}

fn rename_information_buffer_size(name_bytes: usize) -> Option<usize> {
    size_of::<FILE_RENAME_INFORMATION>().checked_add(name_bytes)
}

fn mark_handle_for_deletion(file: &File) -> Result<(), InstallerError> {
    let information = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileDispositionInfo,
            (&raw const information).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(|_| bridge_error("the known package bridge object could not be deleted"))
}

fn cleanup_bounded_orphans(version: &File, shell_sid: PSID) {
    let Ok(names) = enumerate_directory_names(
        version,
        MAX_ORPHAN_DIRECTORY_ENTRIES,
        MAX_ORPHAN_ENUMERATION_BATCHES,
    ) else {
        return;
    };
    for name in names {
        if is_canonical_operation_directory_name(name.as_os_str()) {
            // Opportunistic cleanup never affects admission. An inaccessible,
            // live, drifted, or nonempty candidate is preserved verbatim.
            let _ = cleanup_one_orphan(version, name.as_os_str(), shell_sid);
        }
    }
}

fn cleanup_one_orphan(
    version: &File,
    operation_name: &OsStr,
    shell_sid: PSID,
) -> Result<(), InstallerError> {
    let operation = open_relative(
        version,
        operation_name,
        (FILE_GENERIC_READ | FILE_TRAVERSE | DELETE).0,
        FILE_SHARE_READ.0,
        RelativeDisposition::Open,
        FILE_ATTRIBUTE_DIRECTORY.0,
        (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        None,
    )?;
    let operation_identity = native_file_identity(&operation, NativeObjectKind::Directory)?;
    verify_exact_descriptor(&operation, DescriptorKind::OperationDirectory, shell_sid)?;

    let names = enumerate_directory_names(
        &operation,
        MAX_OPERATION_LEAF_ENTRIES,
        MAX_OPERATION_ENUMERATION_BATCHES,
    )?;
    let mut leaves = Vec::with_capacity(2);
    let mut saw_part = false;
    let mut saw_final = false;
    for name in names {
        let name_ref = name.as_os_str();
        if name_ref == OsStr::new(".") || name_ref == OsStr::new("..") {
            continue;
        }
        let is_part = name_ref == OsStr::new(PACKAGE_BRIDGE_PART_FILE_NAME);
        let is_final = name_ref == OsStr::new(INSTALLER_FILE_NAME);
        if (!is_part && !is_final) || (is_part && saw_part) || (is_final && saw_final) {
            return Err(bridge_integrity_error(
                "an orphan package bridge directory contained unknown content",
            ));
        }
        saw_part |= is_part;
        saw_final |= is_final;
        let leaf = open_relative(
            &operation,
            name_ref,
            (FILE_GENERIC_READ | DELETE).0,
            FILE_SHARE_READ.0,
            RelativeDisposition::Open,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
            None,
        )?;
        let identity = native_file_identity(&leaf, NativeObjectKind::RegularFile)?;
        if identity.number_of_links != 1 {
            return Err(bridge_integrity_error(
                "an orphan package bridge leaf had multiple links",
            ));
        }
        verify_exact_descriptor(&leaf, DescriptorKind::PackageLeaf, shell_sid)?;
        leaves.push(leaf);
    }

    if native_file_identity(&operation, NativeObjectKind::Directory)? != operation_identity {
        return Err(bridge_integrity_error(
            "an orphan package bridge directory changed during inspection",
        ));
    }
    for leaf in &leaves {
        mark_handle_for_deletion(leaf)?;
    }
    drop(leaves);
    if native_file_identity(&operation, NativeObjectKind::Directory)? != operation_identity {
        return Err(bridge_integrity_error(
            "an orphan package bridge directory changed during cleanup",
        ));
    }
    mark_handle_for_deletion(&operation)
}

fn enumerate_directory_names(
    directory: &File,
    max_entries: usize,
    max_batches: usize,
) -> Result<Vec<OsString>, InstallerError> {
    let mut names = Vec::new();
    let word_size = size_of::<usize>();
    let mut storage = vec![0_usize; DIRECTORY_ENUMERATION_BUFFER_BYTES.div_ceil(word_size)];
    let buffer_bytes = storage.len() * word_size;
    let mut restart = true;
    for _ in 0..max_batches {
        let class = if restart {
            FileIdBothDirectoryRestartInfo
        } else {
            FileIdBothDirectoryInfo
        };
        restart = false;
        match unsafe {
            GetFileInformationByHandleEx(
                HANDLE(directory.as_raw_handle()),
                class,
                storage.as_mut_ptr().cast(),
                buffer_bytes as u32,
            )
        } {
            Ok(()) => {}
            Err(error) if error.code() == HRESULT::from_win32(ERROR_NO_MORE_FILES.0) => {
                return Ok(names);
            }
            Err(_) => {
                return Err(bridge_integrity_error(
                    "a package bridge directory could not be enumerated safely",
                ));
            }
        }

        let header_bytes = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
        let mut offset = 0_usize;
        loop {
            if offset
                .checked_add(header_bytes)
                .is_none_or(|end| end > buffer_bytes)
            {
                return Err(bridge_integrity_error(
                    "a package bridge directory record was truncated",
                ));
            }
            let information = unsafe {
                &*storage
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>()
            };
            let name_bytes = information.FileNameLength as usize;
            if name_bytes == 0 || name_bytes % size_of::<u16>() != 0 {
                return Err(bridge_integrity_error(
                    "a package bridge directory name record was invalid",
                ));
            }
            let record_bytes = header_bytes.checked_add(name_bytes).ok_or_else(|| {
                bridge_integrity_error("a package bridge directory record overflowed")
            })?;
            if offset
                .checked_add(record_bytes)
                .is_none_or(|end| end > buffer_bytes)
            {
                return Err(bridge_integrity_error(
                    "a package bridge directory name was truncated",
                ));
            }
            let name = OsString::from_wide(unsafe {
                std::slice::from_raw_parts(
                    information.FileName.as_ptr(),
                    name_bytes / size_of::<u16>(),
                )
            });
            names.push(name);
            if names.len() > max_entries {
                return Err(bridge_integrity_error(
                    "a package bridge directory exceeded its cleanup bound",
                ));
            }
            if information.NextEntryOffset == 0 {
                break;
            }
            let next = information.NextEntryOffset as usize;
            if next < record_bytes || next % align_of::<FILE_ID_BOTH_DIR_INFO>() != 0 {
                return Err(bridge_integrity_error(
                    "a package bridge directory record offset was invalid",
                ));
            }
            offset = offset.checked_add(next).ok_or_else(|| {
                bridge_integrity_error("a package bridge directory offset overflowed")
            })?;
        }
    }
    Err(bridge_integrity_error(
        "a package bridge directory exceeded its enumeration bound",
    ))
}

fn is_canonical_operation_directory_name(value: &OsStr) -> bool {
    let Some(value) = value.to_str() else {
        return false;
    };
    value.len() == BRIDGE_OPERATION_ID_BYTES * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn generate_operation_id() -> Result<BridgeOperationId, InstallerError> {
    use windows::Win32::Security::Cryptography::{
        BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    };

    let mut bytes = [0_u8; BRIDGE_OPERATION_ID_BYTES];
    let status = unsafe { BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.0 < 0 {
        return Err(bridge_error(
            "the package bridge operation ID could not be generated",
        ));
    }
    BridgeOperationId::new(bytes)
        .map_err(|_| bridge_error("the package bridge operation ID was invalid"))
}

struct OwnedSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl OwnedSecurityDescriptor {
    fn for_kind(kind: DescriptorKind, shell_sid: &str) -> Result<Self, InstallerError> {
        let sddl = match kind {
            DescriptorKind::StableDirectory => format!(
                "O:BAG:BAD:P(A;;0x{ADMINISTRATORS_FULL_MASK:08x};;;BA)(A;;0x{DIRECTORY_READ_TRAVERSE_MASK:08x};;;SY)(A;;0x{DIRECTORY_TRAVERSE_ONLY_MASK:08x};;;AU)"
            ),
            DescriptorKind::OperationDirectory => format!(
                "O:BAG:BAD:P(A;;0x{ADMINISTRATORS_FULL_MASK:08x};;;BA)(A;;0x{DIRECTORY_READ_TRAVERSE_MASK:08x};;;SY)(A;;0x{DIRECTORY_READ_TRAVERSE_MASK:08x};;;{shell_sid})"
            ),
            DescriptorKind::PackageLeaf => format!(
                "O:BAG:BAD:P(A;;0x{ADMINISTRATORS_FULL_MASK:08x};;;BA)(A;;0x{FILE_READ_MASK:08x};;;SY)(A;;0x{FILE_READ_MASK:08x};;;{shell_sid})"
            ),
        };
        let sddl = wide_null(OsStr::new(&sddl))?;
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| bridge_error("the package bridge security descriptor was invalid"))?;
        if descriptor.0.is_null() {
            return Err(bridge_error(
                "the package bridge security descriptor was unavailable",
            ));
        }
        Ok(Self(descriptor))
    }

    fn as_nt_descriptor(&self) -> *const windows::Win32::Security::SECURITY_DESCRIPTOR {
        self.0 .0.cast()
    }
}

impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0 .0.is_null() {
            unsafe {
                let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

struct OwnedSid(PSID);

impl OwnedSid {
    fn parse(value: &str) -> Result<Self, InstallerError> {
        let value = wide_null(OsStr::new(value))?;
        let mut sid = PSID::default();
        unsafe { ConvertStringSidToSidW(PCWSTR(value.as_ptr()), &mut sid) }
            .map_err(|_| bridge_integrity_error("the package bridge Shell SID was invalid"))?;
        if sid.0.is_null() || !unsafe { IsValidSid(sid) }.as_bool() {
            return Err(bridge_integrity_error(
                "the package bridge Shell SID was invalid",
            ));
        }
        Ok(Self(sid))
    }

    const fn as_sid(&self) -> PSID {
        self.0
    }
}

impl Drop for OwnedSid {
    fn drop(&mut self) {
        if !self.0 .0.is_null() {
            unsafe {
                let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

struct QueriedSecurityDescriptor {
    descriptor: OwnedSecurityDescriptor,
    owner: PSID,
    group: PSID,
    dacl: *mut windows::Win32::Security::ACL,
}

fn query_security_descriptor(file: &File) -> Result<QueriedSecurityDescriptor, InstallerError> {
    let mut owner = PSID::default();
    let mut group = PSID::default();
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            Some(&mut group),
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0 || descriptor.0.is_null() || owner.0.is_null() || group.0.is_null() {
        return Err(bridge_integrity_error(
            "a package bridge security descriptor could not be queried",
        ));
    }
    Ok(QueriedSecurityDescriptor {
        descriptor: OwnedSecurityDescriptor(descriptor),
        owner,
        group,
        dacl,
    })
}

fn verify_exact_descriptor(
    file: &File,
    kind: DescriptorKind,
    shell_sid: PSID,
) -> Result<(), InstallerError> {
    let security = query_security_descriptor(file)?;
    if !unsafe { IsWellKnownSid(security.owner, WinBuiltinAdministratorsSid) }.as_bool()
        || !unsafe { IsWellKnownSid(security.group, WinBuiltinAdministratorsSid) }.as_bool()
        || security.dacl.is_null()
    {
        return Err(bridge_integrity_error(
            "the package bridge owner, group, or DACL was invalid",
        ));
    }
    let mut control = 0_u16;
    let mut revision = 0_u32;
    unsafe { GetSecurityDescriptorControl(security.descriptor.0, &mut control, &mut revision) }
        .map_err(|_| bridge_integrity_error("the package bridge DACL control was unavailable"))?;
    let forbidden_control = SE_OWNER_DEFAULTED.0
        | SE_GROUP_DEFAULTED.0
        | SE_DACL_DEFAULTED.0
        | SE_DACL_AUTO_INHERIT_REQ.0
        | SE_DACL_AUTO_INHERITED.0;
    if revision != SECURITY_DESCRIPTOR_REVISION_VALUE
        || control & SE_DACL_PRESENT.0 == 0
        || control & SE_DACL_PROTECTED.0 == 0
        || control & forbidden_control != 0
    {
        return Err(bridge_integrity_error(
            "the package bridge DACL was not present and protected",
        ));
    }

    let mut information = ACL_SIZE_INFORMATION::default();
    unsafe {
        GetAclInformation(
            security.dacl,
            (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    }
    .map_err(|_| bridge_integrity_error("the package bridge ACL could not be inspected"))?;
    let expected = kind.expected_aces();
    let acl = unsafe { &*security.dacl };
    if u32::from(acl.AclRevision) != ACL_REVISION.0
        || acl.Sbz1 != 0
        || acl.Sbz2 != 0
        || information.AceCount != expected.len() as u32
    {
        return Err(bridge_integrity_error(
            "the package bridge ACL header or ACE count was invalid",
        ));
    }

    for (index, expected) in expected.iter().enumerate() {
        let mut raw_ace = std::ptr::null_mut();
        unsafe { GetAce(security.dacl, index as u32, &mut raw_ace) }
            .map_err(|_| bridge_integrity_error("a package bridge ACE was unavailable"))?;
        if raw_ace.is_null() {
            return Err(bridge_integrity_error(
                "a package bridge ACE was unavailable",
            ));
        }
        let ace = unsafe { &*raw_ace.cast::<ACCESS_ALLOWED_ACE>() };
        if ace.Header.AceType != ACCESS_ALLOWED_ACE_TYPE_VALUE
            || ace.Header.AceFlags != 0
            || ace.Mask != expected.mask
        {
            return Err(bridge_integrity_error(
                "a package bridge ACE type, flags, or mask was invalid",
            ));
        }
        let sid_offset = offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        let ace_size = usize::from(ace.Header.AceSize);
        let minimum_sid_bytes = 8_usize;
        if sid_offset
            .checked_add(minimum_sid_bytes)
            .is_none_or(|length| length > ace_size)
        {
            return Err(bridge_integrity_error(
                "a package bridge ACE SID was truncated",
            ));
        }
        let sid = PSID((&raw const ace.SidStart).cast_mut().cast());
        let sid_header = unsafe { std::slice::from_raw_parts(sid.0.cast::<u8>(), 2) };
        let encoded_sid_length = minimum_sid_bytes
            .checked_add(usize::from(sid_header[1]).saturating_mul(size_of::<u32>()))
            .ok_or_else(|| bridge_integrity_error("a package bridge ACE SID overflowed"))?;
        if sid_offset
            .checked_add(encoded_sid_length)
            .is_none_or(|length| length > ace_size)
        {
            return Err(bridge_integrity_error(
                "a package bridge ACE SID exceeded its record",
            ));
        }
        if !unsafe { IsValidSid(sid) }.as_bool() {
            return Err(bridge_integrity_error(
                "a package bridge ACE SID was invalid",
            ));
        }
        let sid_length = unsafe { GetLengthSid(sid) } as usize;
        if sid_offset
            .checked_add(sid_length)
            .is_none_or(|length| length != ace_size)
            || sid_length != encoded_sid_length
        {
            return Err(bridge_integrity_error(
                "a package bridge ACE SID exceeded its record",
            ));
        }
        let principal_matches = match expected.principal {
            ExpectedPrincipal::Administrators => {
                unsafe { IsWellKnownSid(sid, WinBuiltinAdministratorsSid) }.as_bool()
            }
            ExpectedPrincipal::System => {
                unsafe { IsWellKnownSid(sid, WinLocalSystemSid) }.as_bool()
            }
            ExpectedPrincipal::AuthenticatedUsers => {
                unsafe { IsWellKnownSid(sid, WinAuthenticatedUserSid) }.as_bool()
            }
            ExpectedPrincipal::ShellUser => unsafe { EqualSid(sid, shell_sid) }.is_ok(),
        };
        if !principal_matches {
            return Err(bridge_integrity_error(
                "a package bridge ACE principal was invalid",
            ));
        }
    }
    Ok(())
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    const fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

fn shell_access_check_token(expected_sid: PSID) -> Result<OwnedHandle, InstallerError> {
    let shell_window = unsafe { GetShellWindow() };
    if shell_window == HWND::default() {
        return Err(bridge_integrity_error(
            "the Explorer Shell token was unavailable for bridge admission",
        ));
    }
    let mut shell_pid = 0_u32;
    unsafe { GetWindowThreadProcessId(shell_window, Some(&mut shell_pid)) };
    if shell_pid == 0 {
        return Err(bridge_integrity_error(
            "the Explorer Shell process was unavailable for bridge admission",
        ));
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, shell_pid) }
        .map_err(|_| bridge_integrity_error("the Explorer Shell process could not be opened"))?;
    let process = OwnedHandle(process);
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process.raw(), TOKEN_QUERY | TOKEN_DUPLICATE, &mut token) }
        .map_err(|_| bridge_integrity_error("the Explorer Shell token could not be opened"))?;
    if token.is_invalid() {
        return Err(bridge_integrity_error(
            "the Explorer Shell token was invalid",
        ));
    }
    let token = OwnedHandle(token);

    let mut required = 0_u32;
    let _ = unsafe { GetTokenInformation(token.raw(), TokenUser, None, 0, &mut required) };
    if required == 0 {
        return Err(bridge_integrity_error(
            "the Explorer Shell token SID was unavailable",
        ));
    }
    let word_size = size_of::<usize>();
    let mut token_user = vec![0_usize; (required as usize).div_ceil(word_size)];
    unsafe {
        GetTokenInformation(
            token.raw(),
            TokenUser,
            Some(token_user.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(|_| bridge_integrity_error("the Explorer Shell token SID could not be read"))?;
    let token_user = unsafe { &*token_user.as_ptr().cast::<TOKEN_USER>() };
    if unsafe { EqualSid(token_user.User.Sid, expected_sid) }.is_err() {
        return Err(bridge_integrity_error(
            "the Explorer Shell SID changed before bridge admission",
        ));
    }

    let mut duplicate = HANDLE::default();
    unsafe { DuplicateToken(token.raw(), SecurityImpersonation, &mut duplicate) }
        .map_err(|_| bridge_integrity_error("the Explorer Shell token could not be duplicated"))?;
    if duplicate.is_invalid() {
        return Err(bridge_integrity_error(
            "the Explorer Shell access-check token was invalid",
        ));
    }
    Ok(OwnedHandle(duplicate))
}

fn effective_file_access(file: &File, token: HANDLE) -> Result<u32, InstallerError> {
    let security = query_security_descriptor(file)?;
    let mapping = GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ.0,
        GenericWrite: FILE_GENERIC_WRITE.0,
        GenericExecute: windows::Win32::Storage::FileSystem::FILE_GENERIC_EXECUTE.0,
        GenericAll: FILE_ALL_ACCESS.0,
    };
    let word_size = size_of::<usize>();
    let mut privilege_storage = vec![0_usize; ACCESS_CHECK_BUFFER_BYTES.div_ceil(word_size)];
    let mut privilege_length = (privilege_storage.len() * word_size) as u32;
    let mut granted = 0_u32;
    let mut access = BOOL::default();
    unsafe {
        AccessCheck(
            security.descriptor.0,
            token,
            MAXIMUM_ALLOWED_MASK,
            &mapping,
            Some(privilege_storage.as_mut_ptr().cast::<PRIVILEGE_SET>()),
            &mut privilege_length,
            &mut granted,
            &mut access,
        )
    }
    .map_err(|_| bridge_integrity_error("the Shell-user ancestor access check failed"))?;
    if !access.as_bool() {
        return Err(bridge_integrity_error(
            "the Shell-user ancestor access check was indeterminate",
        ));
    }
    Ok(granted)
}

fn ancestor_mutation_rejected(granted_dangerous: u32, token_is_administrator: bool) -> bool {
    granted_dangerous != 0 && !token_is_administrator
}

fn token_is_local_administrator(token: HANDLE) -> Result<bool, InstallerError> {
    let word_size = size_of::<usize>();
    let mut administrators = vec![0_usize; (SECURITY_MAX_SID_SIZE as usize).div_ceil(word_size)];
    let mut administrators_len = SECURITY_MAX_SID_SIZE;
    let administrators_sid = PSID(administrators.as_mut_ptr().cast());
    unsafe {
        CreateWellKnownSid(
            WinBuiltinAdministratorsSid,
            None,
            Some(administrators_sid),
            &mut administrators_len,
        )
    }
    .map_err(|_| {
        bridge_integrity_error("the Administrators SID could not be created for ancestor access")
    })?;
    let mut member = BOOL::default();
    unsafe { CheckTokenMembership(Some(token), administrators_sid, &mut member) }.map_err(
        |_| {
            bridge_integrity_error(
                "the Explorer Shell Administrators membership could not be queried",
            )
        },
    )?;
    Ok(member.as_bool())
}

fn decode_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut decoded = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        decoded[index] = (hex_nibble(chunk[0])? << 4) | hex_nibble(chunk[1])?;
    }
    Some(decoded)
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn wide_null(value: &OsStr) -> Result<Vec<u16>, InstallerError> {
    let mut wide = value.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(bridge_integrity_error(
            "a package bridge Windows string was invalid",
        ));
    }
    wide.push(0);
    Ok(wide)
}

fn bridge_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::WindowsDeploymentFailed)
        .with_retryable(false)
        .with_diagnostic_message(message)
}

fn bridge_integrity_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
        .with_retryable(false)
        .with_diagnostic_message(message)
}

fn bridge_checksum_error(message: &'static str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::ChecksumMismatch)
        .with_retryable(false)
        .with_diagnostic_message(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_buffer_includes_the_complete_inline_structure() {
        let name_bytes = INSTALLER_FILE_NAME.encode_utf16().count() * size_of::<u16>();
        let buffer_size = rename_information_buffer_size(name_bytes).unwrap();

        assert_eq!(
            buffer_size,
            size_of::<FILE_RENAME_INFORMATION>() + name_bytes
        );
        assert!(buffer_size > offset_of!(FILE_RENAME_INFORMATION, FileName) + name_bytes);
    }

    #[test]
    fn fixed_layout_names_are_single_safe_components() {
        for name in [
            PACKAGE_BRIDGE_ROOT_DIRECTORY,
            PACKAGE_BRIDGE_VERSION_DIRECTORY,
            PACKAGE_BRIDGE_PART_FILE_NAME,
            INSTALLER_FILE_NAME,
        ] {
            let components = Path::new(name).components().collect::<Vec<_>>();
            assert_eq!(components.len(), 1);
            assert!(matches!(components[0], Component::Normal(_)));
        }
        assert_ne!(PACKAGE_BRIDGE_PART_FILE_NAME, INSTALLER_FILE_NAME);
    }

    #[test]
    fn sha256_decoder_accepts_only_canonical_lowercase_hex() {
        let value = "0123456789abcdef".repeat(4);
        let decoded = decode_sha256(&value).expect("canonical checksum");
        assert_eq!(
            &decoded[..8],
            &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
        );
        assert!(decode_sha256(&value.to_ascii_uppercase()).is_none());
        assert!(decode_sha256(&value[..63]).is_none());
        assert!(decode_sha256(&format!("{}g", &value[..63])).is_none());
    }

    #[test]
    fn orphan_cleanup_admits_only_canonical_operation_components() {
        let canonical = "0123456789abcdef".repeat(4);
        assert!(is_canonical_operation_directory_name(OsStr::new(
            &canonical
        )));
        assert!(!is_canonical_operation_directory_name(OsStr::new(
            &canonical.to_ascii_uppercase()
        )));
        assert!(!is_canonical_operation_directory_name(OsStr::new(
            &canonical[..63]
        )));
        assert!(!is_canonical_operation_directory_name(OsStr::new(
            &format!("{}g", &canonical[..63])
        )));
        assert!(!is_canonical_operation_directory_name(OsStr::new(
            "../0123456789abcdef"
        )));
        assert!(!is_canonical_operation_directory_name(OsStr::new(
            &"0".repeat(BRIDGE_OPERATION_ID_BYTES * 2)
        )));
    }

    #[test]
    fn stable_traverse_ace_cannot_list_create_write_or_delete() {
        let mask = DIRECTORY_TRAVERSE_ONLY_MASK;
        assert_ne!(mask & FILE_TRAVERSE.0, 0);
        assert_eq!(
            mask & windows::Win32::Storage::FileSystem::FILE_LIST_DIRECTORY.0,
            0
        );
        assert_eq!(
            mask & windows::Win32::Storage::FileSystem::FILE_ADD_FILE.0,
            0
        );
        assert_eq!(
            mask & windows::Win32::Storage::FileSystem::FILE_ADD_SUBDIRECTORY.0,
            0
        );
        assert_eq!(mask & FILE_DELETE_CHILD.0, 0);
        assert_eq!(mask & DELETE.0, 0);
        assert_eq!(mask & WRITE_DAC.0, 0);
        assert_eq!(mask & WRITE_OWNER.0, 0);
    }

    #[test]
    fn every_exact_descriptor_is_allow_only_and_has_three_aces() {
        for kind in [
            DescriptorKind::StableDirectory,
            DescriptorKind::OperationDirectory,
            DescriptorKind::PackageLeaf,
        ] {
            let aces = kind.expected_aces();
            assert_eq!(aces.len(), 3);
            assert_eq!(aces[0].principal, ExpectedPrincipal::Administrators);
            assert_eq!(aces[0].mask, ADMINISTRATORS_FULL_MASK);
        }

        let forbidden_mutation = windows::Win32::Storage::FileSystem::FILE_WRITE_DATA.0
            | windows::Win32::Storage::FileSystem::FILE_APPEND_DATA.0
            | FILE_WRITE_EA.0
            | FILE_WRITE_ATTRIBUTES.0
            | FILE_DELETE_CHILD.0
            | DELETE.0
            | WRITE_DAC.0
            | WRITE_OWNER.0;
        assert_eq!(OPERATION_DIRECTORY_ACES[2].mask & forbidden_mutation, 0);
        assert_eq!(PACKAGE_LEAF_ACES[2].mask & forbidden_mutation, 0);
    }

    #[test]
    fn os_owned_ancestors_fail_closed_only_for_a_non_administrator_shell() {
        let dangerous = FILE_DELETE_CHILD.0
            | DELETE.0
            | WRITE_DAC.0
            | WRITE_OWNER.0
            | FILE_WRITE_EA.0
            | FILE_WRITE_ATTRIBUTES.0;
        assert!(ancestor_mutation_rejected(dangerous, false));
        assert!(!ancestor_mutation_rejected(dangerous, true));
        assert!(!ancestor_mutation_rejected(0, false));
        assert!(!ancestor_mutation_rejected(0, true));
    }
}
