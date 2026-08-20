//! Windows handle-pinned storage for WorkBuddy's fixed credential document.
//!
//! A formal FyAgent build is elevated, so every component below the frozen
//! Explorer user's profile is attacker-controlled input.  This module never
//! turns a validated path back into a path-based operation: ancestors are held,
//! leaves are opened or created relative to the pinned `.workbuddy` handle, and
//! commits rename an already-open temporary file through that same handle.

use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::AsRawHandle, io::FromRawHandle},
    path::{Component, Path, PathBuf},
};

use uuid::Uuid;
use windows::{
    core::PWSTR,
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FileRenameInformationEx, FileStreamInformation, NtCreateFile, NtQueryInformationFile,
            NtSetInformationFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE,
            FILE_OPEN, FILE_OPEN_IF, FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION,
            FILE_RENAME_POSIX_SEMANTICS, FILE_RENAME_REPLACE_IF_EXISTS, FILE_STREAM_INFORMATION,
            FILE_SYNCHRONOUS_IO_NONALERT, NTCREATEFILE_CREATE_DISPOSITION,
            NTCREATEFILE_CREATE_OPTIONS,
        },
    },
    Win32::{
        Foundation::{
            LocalFree, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE,
            OBJ_DONT_REPARSE, STATUS_BUFFER_OVERFLOW, STATUS_BUFFER_TOO_SMALL,
            STATUS_INFO_LENGTH_MISMATCH, STATUS_NO_SUCH_FILE, STATUS_OBJECT_NAME_NOT_FOUND,
            STATUS_OBJECT_PATH_NOT_FOUND, UNICODE_STRING,
        },
        Security::{
            Authorization::{GetSecurityInfo, SetSecurityInfo, SE_FILE_OBJECT},
            GetSecurityDescriptorControl, DACL_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SE_DACL_PROTECTED,
            UNPROTECTED_DACL_SECURITY_INFORMATION,
        },
        Storage::FileSystem::{
            FileStandardInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
            BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ACCESS_RIGHTS, FILE_ADD_FILE,
            FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_OFFLINE,
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SPARSE_FILE, FILE_DELETE_CHILD,
            FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            FILE_STANDARD_INFO, FILE_TRAVERSE, SYNCHRONIZE, WRITE_DAC,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

use super::document::MAX_CONFIG_BYTES;

const MODELS_FILE_NAME: &str = "models.json";
const BACKUP_FILE_NAME: &str = "models.json.backup";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObjectKind {
    Directory,
    RegularFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    volume_serial: u64,
    file_index: u64,
    size: u64,
    links: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StableIdentity {
    volume_serial: u64,
    file_index: u64,
}

impl FileIdentity {
    fn stable(self) -> StableIdentity {
        StableIdentity {
            volume_serial: self.volume_serial,
            file_index: self.file_index,
        }
    }
}

struct HeldDirectory {
    file: File,
    identity: StableIdentity,
}

impl HeldDirectory {
    fn capture(file: File) -> io::Result<Self> {
        let identity = file_identity(&file, ObjectKind::Directory)?.stable();
        Ok(Self { file, identity })
    }

    fn recheck(&self) -> io::Result<()> {
        // Child creation/removal can legitimately change directory size and
        // link metadata.  Only the volume/file ID is a stable namespace
        // identity; file_identity still revalidates type and unsafe attrs.
        if file_identity(&self.file, ObjectKind::Directory)?.stable() != self.identity {
            return Err(integrity_error());
        }
        Ok(())
    }
}

struct HeldLeaf {
    file: File,
    identity: FileIdentity,
}

impl HeldLeaf {
    fn capture(file: File) -> io::Result<Self> {
        let identity = file_identity(&file, ObjectKind::RegularFile)?;
        Ok(Self { file, identity })
    }

    fn recheck(&self) -> io::Result<()> {
        if file_identity(&self.file, ObjectKind::RegularFile)? != self.identity {
            return Err(integrity_error());
        }
        Ok(())
    }
}

/// One operation-scoped namespace capability.  Holding every ancestor prevents
/// a rename/swap of the profile or `.workbuddy` component during the operation.
pub(super) struct WindowsWorkBuddyStorage {
    _ancestors: Vec<HeldDirectory>,
    ancestor_names: Vec<OsString>,
    directory: HeldDirectory,
    production_context: bool,
}

/// A primary-file preimage plus the no-write handle that freezes its contents
/// for the duration of a save.  For a missing primary, the final rename uses
/// non-replacing semantics so a raced creation fails closed.
pub(super) struct WindowsModelsSnapshot {
    bytes: Option<Vec<u8>>,
    leaf: Option<HeldLeaf>,
}

impl WindowsModelsSnapshot {
    pub(super) fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WindowsCommitError {
    Concurrent,
    Backup,
    Primary,
}

enum TargetExpectation<'a> {
    Discover,
    Existing(&'a HeldLeaf),
    Missing,
}

impl WindowsWorkBuddyStorage {
    pub(super) fn open(home: &Path, create_directory: bool) -> io::Result<Self> {
        let production_context = production_context_for(home)?;
        revalidate_production_context(production_context)?;
        let (ancestors, ancestor_names) = open_absolute_ancestor_chain(home)?;
        let home_directory = ancestors.last().ok_or_else(integrity_error)?;
        let workbuddy = open_relative(
            &home_directory.file,
            OsStr::new(".workbuddy"),
            (FILE_GENERIC_READ
                | FILE_TRAVERSE
                | FILE_READ_ATTRIBUTES
                | FILE_ADD_FILE
                | FILE_DELETE_CHILD
                | SYNCHRONIZE)
                .0,
            (FILE_SHARE_READ | FILE_SHARE_WRITE).0,
            if create_directory {
                RelativeDisposition::OpenIf
            } else {
                RelativeDisposition::Open
            },
            FILE_ATTRIBUTE_DIRECTORY.0,
            (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        );

        let directory = match workbuddy {
            Ok(file) => HeldDirectory::capture(file)?,
            Err(RelativeOpenError::NotFound) if !create_directory => {
                return Err(io::Error::from(io::ErrorKind::NotFound));
            }
            Err(_) => return Err(integrity_error()),
        };
        if directory.identity.volume_serial != home_directory.identity.volume_serial {
            return Err(integrity_error());
        }
        home_directory.recheck()?;
        directory.recheck()?;
        revalidate_production_context(production_context)?;

        // Keep the root-to-home chain alive. `relative_components` is consumed
        // by the opener and retained only as a sanity proof that `home` was not
        // the volume root itself.
        if ancestor_names.is_empty() {
            return Err(integrity_error());
        }
        Ok(Self {
            _ancestors: ancestors,
            ancestor_names,
            directory,
            production_context,
        })
    }

    pub(super) fn read_models(&self) -> io::Result<Option<Vec<u8>>> {
        self.read_named(MODELS_FILE_NAME)
    }

    pub(super) fn snapshot_models(&self) -> io::Result<WindowsModelsSnapshot> {
        self.recheck()?;
        let Some(mut leaf) = self.open_leaf(MODELS_FILE_NAME, LeafAccess::Read)? else {
            return Ok(WindowsModelsSnapshot {
                bytes: None,
                leaf: None,
            });
        };
        let bytes = read_bounded(&mut leaf.file)?;
        leaf.recheck()?;
        self.recheck()?;
        Ok(WindowsModelsSnapshot {
            bytes: Some(bytes),
            leaf: Some(leaf),
        })
    }

    pub(super) fn backup_exists(&self) -> io::Result<bool> {
        Ok(self
            .open_leaf(BACKUP_FILE_NAME, LeafAccess::Read)?
            .is_some())
    }

    /// Backup-first commit: both backup and primary temporary leaves are
    /// created relative to the same pinned directory.  The primary rename is
    /// attempted only after the backup rename succeeds.
    pub(super) fn commit(
        &self,
        snapshot: &mut WindowsModelsSnapshot,
        replacement: &[u8],
    ) -> Result<(), WindowsCommitError> {
        self.recheck().map_err(|_| WindowsCommitError::Backup)?;
        if !self
            .snapshot_matches(snapshot)
            .map_err(|_| WindowsCommitError::Concurrent)?
        {
            return Err(WindowsCommitError::Concurrent);
        }
        if let Some(original) = snapshot.bytes() {
            self.write_named_atomically(BACKUP_FILE_NAME, original, TargetExpectation::Discover)
                .map_err(|_| WindowsCommitError::Backup)?;
        }
        self.recheck().map_err(|_| WindowsCommitError::Primary)?;
        if !self
            .snapshot_matches(snapshot)
            .map_err(|_| WindowsCommitError::Primary)?
        {
            return Err(WindowsCommitError::Primary);
        }
        let primary_expectation = match snapshot.leaf.as_ref() {
            Some(leaf) => TargetExpectation::Existing(leaf),
            None => TargetExpectation::Missing,
        };
        self.write_named_atomically(MODELS_FILE_NAME, replacement, primary_expectation)
            .map_err(|_| WindowsCommitError::Primary)
    }

    fn read_named(&self, name: &str) -> io::Result<Option<Vec<u8>>> {
        self.recheck()?;
        let Some(mut leaf) = self.open_leaf(name, LeafAccess::Read)? else {
            return Ok(None);
        };
        let bytes = read_bounded(&mut leaf.file)?;
        leaf.recheck()?;
        self.recheck()?;
        Ok(Some(bytes))
    }

    pub(super) fn snapshot_matches(
        &self,
        snapshot: &mut WindowsModelsSnapshot,
    ) -> io::Result<bool> {
        self.recheck()?;
        let matches = match (&mut snapshot.leaf, snapshot.bytes.as_deref()) {
            (Some(held), Some(expected)) => {
                held.recheck()?;
                let namespace_leaf = self.open_leaf(MODELS_FILE_NAME, LeafAccess::Read)?;
                let Some(namespace_leaf) = namespace_leaf else {
                    return Ok(false);
                };
                if namespace_leaf.identity != held.identity {
                    return Ok(false);
                }
                let bytes = read_bounded(&mut held.file)?;
                held.recheck()?;
                bytes == expected
            }
            (None, None) => self
                .open_leaf(MODELS_FILE_NAME, LeafAccess::Read)?
                .is_none(),
            _ => false,
        };
        self.recheck()?;
        Ok(matches)
    }

    fn open_leaf(&self, name: &str, access: LeafAccess) -> io::Result<Option<HeldLeaf>> {
        self.recheck()?;
        let (desired_access, share_access, disposition) = match access {
            LeafAccess::Read => (
                (FILE_GENERIC_READ | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
                (FILE_SHARE_READ | FILE_SHARE_DELETE).0,
                RelativeDisposition::Open,
            ),
        };
        let file = match open_relative(
            &self.directory.file,
            OsStr::new(name),
            desired_access,
            share_access,
            disposition,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        ) {
            Ok(file) => file,
            Err(RelativeOpenError::NotFound) => return Ok(None),
            Err(_) => return Err(integrity_error()),
        };
        let leaf = HeldLeaf::capture(file)?;
        if leaf.identity.volume_serial != self.directory.identity.volume_serial {
            return Err(integrity_error());
        }
        self.recheck()?;
        Ok(Some(leaf))
    }

    fn write_named_atomically(
        &self,
        target_name: &str,
        data: &[u8],
        expectation: TargetExpectation<'_>,
    ) -> io::Result<()> {
        let discovered;
        let existing = match expectation {
            TargetExpectation::Discover => {
                discovered = self.open_leaf(target_name, LeafAccess::Read)?;
                discovered.as_ref()
            }
            TargetExpectation::Existing(existing) => Some(existing),
            TargetExpectation::Missing => {
                if self.open_leaf(target_name, LeafAccess::Read)?.is_some() {
                    return Err(integrity_error());
                }
                None
            }
        };
        self.recheck()?;
        revalidate_production_context(self.production_context)?;

        let temp_name = format!(".{target_name}.tmp.{}", Uuid::new_v4());
        let file = open_relative(
            &self.directory.file,
            OsStr::new(&temp_name),
            (FILE_GENERIC_READ
                | FILE_GENERIC_WRITE
                | FILE_READ_ATTRIBUTES
                | DELETE
                | SYNCHRONIZE
                | WRITE_DAC)
                .0,
            0,
            RelativeDisposition::Create,
            FILE_ATTRIBUTE_NORMAL.0,
            (FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        )
        .map_err(|_| integrity_error())?;
        #[cfg(test)]
        eprintln!("workbuddy native test trace: temporary leaf created");
        let mut temp = HeldLeaf::capture(file)?;
        if temp.identity.volume_serial != self.directory.identity.volume_serial {
            return Err(integrity_error());
        }

        let result = (|| {
            temp.file.seek(SeekFrom::Start(0))?;
            temp.file.write_all(data)?;
            temp.file.flush()?;
            temp.file.sync_all()?;
            #[cfg(test)]
            eprintln!("workbuddy native test trace: temporary leaf synced");
            temp.identity = file_identity(&temp.file, ObjectKind::RegularFile)?;
            self.recheck()?;
            if let Some(existing) = existing {
                existing.recheck()?;
                copy_dacl(&existing.file, &temp.file)?;
            }
            match existing {
                Some(expected) => {
                    let Some(current) = self.open_leaf(target_name, LeafAccess::Read)? else {
                        return Err(integrity_error());
                    };
                    if current.identity != expected.identity {
                        return Err(integrity_error());
                    }
                }
                None => {
                    if self.open_leaf(target_name, LeafAccess::Read)?.is_some() {
                        return Err(integrity_error());
                    }
                }
            }
            revalidate_production_context(self.production_context)?;
            if let Some(existing) = existing {
                existing.recheck()?;
            }
            let target_existed = existing.is_some();
            #[cfg(test)]
            eprintln!("workbuddy native test trace: starting handle rename");
            rename_by_handle(&self.directory, &temp, target_name, target_existed)?;
            #[cfg(test)]
            eprintln!("workbuddy native test trace: handle rename committed");
            Ok(())
        })();

        if result.is_err() {
            // Delete by the already-open handle.  Never resolve the temporary
            // name again in an attacker-controlled namespace.
            let _ = mark_delete_by_handle(&temp.file);
        }
        result
    }

    fn recheck(&self) -> io::Result<()> {
        for ancestor in &self._ancestors {
            ancestor.recheck()?;
        }
        for (index, name) in self.ancestor_names.iter().enumerate() {
            let parent = self._ancestors.get(index).ok_or_else(integrity_error)?;
            let expected = self._ancestors.get(index + 1).ok_or_else(integrity_error)?;
            let current = open_relative_directory(&parent.file, name)?;
            if current.identity != expected.identity {
                return Err(integrity_error());
            }
        }
        self.directory.recheck()?;
        let home = self._ancestors.last().ok_or_else(integrity_error)?;
        let current = open_relative_directory(&home.file, OsStr::new(".workbuddy"))?;
        if current.identity != self.directory.identity {
            return Err(integrity_error());
        }
        revalidate_production_context(self.production_context)
    }
}

#[derive(Clone, Copy)]
enum LeafAccess {
    Read,
}

#[derive(Clone, Copy)]
enum RelativeDisposition {
    Open,
    Create,
    OpenIf,
}

enum RelativeOpenError {
    NotFound,
    Rejected,
}

#[cfg(test)]
fn production_context_for(_home: &Path) -> io::Result<bool> {
    Ok(false)
}

#[cfg(not(test))]
fn production_context_for(home: &Path) -> io::Result<bool> {
    #[cfg(feature = "test-hooks")]
    if let Some(test_home) = std::env::var_os("FYAGENT_TEST_HOME") {
        let test_home = PathBuf::from(test_home);
        if test_home.is_absolute() && test_home == home {
            return Ok(false);
        }
        return Err(integrity_error());
    }

    let context = crate::windows_runtime::require_interactive_user_context();
    if home != context.user_profile() {
        return Err(integrity_error());
    }
    Ok(true)
}

fn revalidate_production_context(production_context: bool) -> io::Result<()> {
    if production_context
        && !crate::windows_runtime::revalidate_interactive_user_context(
            crate::windows_runtime::require_interactive_user_context(),
        )
    {
        return Err(integrity_error());
    }
    Ok(())
}

fn open_absolute_ancestor_chain(home: &Path) -> io::Result<(Vec<HeldDirectory>, Vec<OsString>)> {
    if !home.is_absolute() || home.as_os_str().is_empty() {
        return Err(integrity_error());
    }
    let mut components = home.components();
    let prefix = match components.next() {
        Some(Component::Prefix(prefix)) => prefix.as_os_str(),
        _ => return Err(integrity_error()),
    };
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(integrity_error());
    }

    let mut root = PathBuf::from(prefix);
    root.push(Path::new(r"\"));
    let root = open_absolute_directory_no_follow(&root)?;
    let root = HeldDirectory::capture(root)?;
    let volume = root.identity.volume_serial;
    let mut held = vec![root];
    let mut names = Vec::new();
    let components = components.collect::<Vec<_>>();
    let last_index = components
        .len()
        .checked_sub(1)
        .ok_or_else(integrity_error)?;
    for (index, component) in components.into_iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(integrity_error());
        };
        let parent = held.last().expect("the volume root is held");
        parent.recheck()?;
        let file = open_relative(
            &parent.file,
            name,
            (FILE_GENERIC_READ
                | FILE_TRAVERSE
                | FILE_READ_ATTRIBUTES
                | SYNCHRONIZE
                | if index == last_index {
                    FILE_ADD_SUBDIRECTORY
                } else {
                    FILE_ACCESS_RIGHTS(0)
                })
            .0,
            (FILE_SHARE_READ | FILE_SHARE_WRITE).0,
            RelativeDisposition::Open,
            FILE_ATTRIBUTE_DIRECTORY.0,
            (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
        )
        .map_err(|_| integrity_error())?;
        let next = HeldDirectory::capture(file)?;
        if next.identity.volume_serial != volume {
            return Err(integrity_error());
        }
        parent.recheck()?;
        held.push(next);
        names.push(name.to_os_string());
    }
    Ok((held, names))
}

fn open_relative_directory(parent: &File, name: &OsStr) -> io::Result<HeldDirectory> {
    let file = open_relative(
        parent,
        name,
        (FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
        (FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0,
        RelativeDisposition::Open,
        FILE_ATTRIBUTE_DIRECTORY.0,
        (FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0,
    )
    .map_err(|_| integrity_error())?;
    HeldDirectory::capture(file)
}

fn open_absolute_directory_no_follow(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    let mut options = std::fs::OpenOptions::new();
    options
        .access_mode((FILE_GENERIC_READ | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0);
    options.open(path).map_err(|_| integrity_error())
}

#[allow(clippy::too_many_arguments)]
fn open_relative(
    parent: &File,
    name: &OsStr,
    desired_access: u32,
    share_access: u32,
    disposition: RelativeDisposition,
    attributes: u32,
    create_options: u32,
) -> Result<File, RelativeOpenError> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(component)) if component == name)
        || components.next().is_some()
    {
        return Err(RelativeOpenError::Rejected);
    }
    let mut wide = name.encode_wide().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(RelativeOpenError::Rejected);
    }
    let byte_length = wide
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|length| u16::try_from(length).ok())
        .ok_or(RelativeOpenError::Rejected)?;
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
        SecurityDescriptor: std::ptr::null(),
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
    if status.is_err() {
        return if matches!(
            status,
            STATUS_NO_SUCH_FILE | STATUS_OBJECT_NAME_NOT_FOUND | STATUS_OBJECT_PATH_NOT_FOUND
        ) {
            Err(RelativeOpenError::NotFound)
        } else {
            Err(RelativeOpenError::Rejected)
        };
    }
    if handle.0.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(RelativeOpenError::Rejected);
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

fn file_identity(file: &File, expected: ObjectKind) -> io::Result<FileIdentity> {
    let mut basic = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut basic) }
        .map_err(|_| integrity_error())?;
    let directory = basic.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let unsafe_attributes = unsafe_file_attributes(expected);
    if basic.dwFileAttributes & unsafe_attributes != 0
        || match expected {
            ObjectKind::Directory => !directory,
            ObjectKind::RegularFile => directory,
        }
    {
        return Err(integrity_error());
    }
    if expected == ObjectKind::RegularFile {
        reject_non_default_streams(file)?;
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
    .map_err(|_| integrity_error())?;
    let size = u64::try_from(standard.EndOfFile).map_err(|_| integrity_error())?;
    let basic_size = (u64::from(basic.nFileSizeHigh) << 32) | u64::from(basic.nFileSizeLow);
    if standard.DeletePending
        || standard.Directory != directory
        || standard.NumberOfLinks != basic.nNumberOfLinks
        || size != basic_size
        || (expected == ObjectKind::RegularFile && basic.nNumberOfLinks != 1)
    {
        return Err(integrity_error());
    }
    Ok(FileIdentity {
        volume_serial: u64::from(basic.dwVolumeSerialNumber),
        file_index: (u64::from(basic.nFileIndexHigh) << 32) | u64::from(basic.nFileIndexLow),
        size,
        links: basic.nNumberOfLinks,
    })
}

fn unsafe_file_attributes(expected: ObjectKind) -> u32 {
    let mut attributes = FILE_ATTRIBUTE_REPARSE_POINT.0
        | FILE_ATTRIBUTE_OFFLINE.0
        | FILE_ATTRIBUTE_RECALL_ON_OPEN.0
        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0;
    if expected == ObjectKind::RegularFile {
        // Replacing these files with an ordinary temporary leaf would either
        // expose plaintext or silently discard filesystem-managed data.
        attributes |=
            FILE_ATTRIBUTE_ENCRYPTED.0 | FILE_ATTRIBUTE_COMPRESSED.0 | FILE_ATTRIBUTE_SPARSE_FILE.0;
    }
    attributes
}

fn reject_non_default_streams(file: &File) -> io::Result<()> {
    const MAX_STREAM_INFORMATION_BYTES: usize = 64 * 1024;
    let default_stream = "::$DATA".encode_utf16().collect::<Vec<_>>();
    let mut byte_capacity = 4096usize;

    loop {
        // u64 backing keeps each FILE_STREAM_INFORMATION entry suitably
        // aligned while still allowing a variable-sized native result.
        let mut storage = vec![0u64; byte_capacity.div_ceil(size_of::<u64>())];
        let mut io_status = IO_STATUS_BLOCK::default();
        let status = unsafe {
            NtQueryInformationFile(
                HANDLE(file.as_raw_handle()),
                &mut io_status,
                storage.as_mut_ptr().cast(),
                byte_capacity as u32,
                FileStreamInformation,
            )
        };
        if matches!(
            status,
            STATUS_BUFFER_OVERFLOW | STATUS_BUFFER_TOO_SMALL | STATUS_INFO_LENGTH_MISMATCH
        ) {
            byte_capacity = byte_capacity
                .checked_mul(2)
                .filter(|size| *size <= MAX_STREAM_INFORMATION_BYTES)
                .ok_or_else(integrity_error)?;
            continue;
        }
        if status.is_err() {
            return Err(integrity_error());
        }

        let bytes = storage.as_ptr().cast::<u8>();
        let header_size = std::mem::offset_of!(FILE_STREAM_INFORMATION, StreamName);
        let mut offset = 0usize;
        let mut entries = 0usize;
        loop {
            if offset
                .checked_add(header_size)
                .is_none_or(|end| end > byte_capacity)
            {
                return Err(integrity_error());
            }
            let entry = unsafe { &*bytes.add(offset).cast::<FILE_STREAM_INFORMATION>() };
            let name_bytes =
                usize::try_from(entry.StreamNameLength).map_err(|_| integrity_error())?;
            if name_bytes % size_of::<u16>() != 0
                || offset
                    .checked_add(header_size)
                    .and_then(|start| start.checked_add(name_bytes))
                    .is_none_or(|end| end > byte_capacity)
            {
                return Err(integrity_error());
            }
            let name = unsafe {
                std::slice::from_raw_parts(
                    bytes.add(offset + header_size).cast::<u16>(),
                    name_bytes / size_of::<u16>(),
                )
            };
            entries += 1;
            if entries != 1 || name != default_stream {
                return Err(integrity_error());
            }
            if entry.NextEntryOffset == 0 {
                return Ok(());
            }
            let next = usize::try_from(entry.NextEntryOffset).map_err(|_| integrity_error())?;
            if next < header_size || next % size_of::<u64>() != 0 {
                return Err(integrity_error());
            }
            offset = offset.checked_add(next).ok_or_else(integrity_error)?;
        }
    }
}

fn read_bounded(file: &mut File) -> io::Result<Vec<u8>> {
    let identity = file_identity(file, ObjectKind::RegularFile)?;
    if identity.size > MAX_CONFIG_BYTES {
        return Err(integrity_error());
    }
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::with_capacity(identity.size as usize);
    Read::by_ref(file)
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != identity.size || bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(integrity_error());
    }
    Ok(bytes)
}

fn rename_by_handle(
    directory: &HeldDirectory,
    temp: &HeldLeaf,
    final_name: &str,
    target_existed: bool,
) -> io::Result<()> {
    directory.recheck()?;
    temp.recheck()?;
    let wide = final_name.encode_utf16().collect::<Vec<_>>();
    if wide.is_empty() || wide.contains(&0) {
        return Err(integrity_error());
    }
    let name_bytes = wide
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(integrity_error)?;
    let buffer_size = size_of::<FILE_RENAME_INFORMATION>()
        .checked_add(name_bytes)
        .ok_or_else(integrity_error)?;
    let word_size = size_of::<usize>();
    let mut storage = vec![0_usize; buffer_size.div_ceil(word_size)];
    let information = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io_status = IO_STATUS_BLOCK::default();
    let status = unsafe {
        (*information).Anonymous.Flags = if target_existed {
            FILE_RENAME_REPLACE_IF_EXISTS | FILE_RENAME_POSIX_SEMANTICS
        } else {
            0
        };
        (*information).RootDirectory = HANDLE(directory.file.as_raw_handle());
        (*information).FileNameLength = u32::try_from(name_bytes).map_err(|_| integrity_error())?;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            (*information).FileName.as_mut_ptr(),
            wide.len(),
        );
        NtSetInformationFile(
            HANDLE(temp.file.as_raw_handle()),
            &mut io_status,
            information.cast(),
            u32::try_from(buffer_size).map_err(|_| integrity_error())?,
            FileRenameInformationEx,
        )
    };
    if status.is_err() {
        #[cfg(test)]
        eprintln!(
            "workbuddy native test trace: handle rename NTSTATUS 0x{:08X}",
            status.0 as u32
        );
        return Err(integrity_error());
    }
    // The successful rename is the commit point.  All security checks that can
    // fail are deliberately before it so callers never receive an unwritten
    // failure after the namespace has already changed.
    Ok(())
}

struct OwnedSecurityDescriptor(PSECURITY_DESCRIPTOR);

impl Drop for OwnedSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0 .0.is_null() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

/// `ReplaceFileW` preserved an existing credential file's ACL. The
/// handle-relative replacement applies the already-open target's DACL to the
/// temporary handle before the atomic rename, without reopening either path.
fn copy_dacl(source: &File, destination: &File) -> io::Result<()> {
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            HANDLE(source.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0 || descriptor.0.is_null() {
        return Err(integrity_error());
    }
    let descriptor = OwnedSecurityDescriptor(descriptor);
    let mut control = 0_u16;
    let mut revision = 0_u32;
    unsafe { GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) }
        .map_err(|_| integrity_error())?;
    let protection = if control & SE_DACL_PROTECTED.0 != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    let status = unsafe {
        SetSecurityInfo(
            HANDLE(destination.as_raw_handle()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | protection,
            None,
            None,
            Some(dacl),
            None,
        )
    };
    if status.0 != 0 {
        return Err(integrity_error());
    }
    drop(descriptor);
    Ok(())
}

fn mark_delete_by_handle(file: &File) -> io::Result<()> {
    use windows::Win32::Storage::FileSystem::{
        FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO,
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
    .map_err(|_| integrity_error())
}

fn integrity_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "WorkBuddy configuration storage is unavailable",
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, OpenOptions},
        io::{Seek, SeekFrom, Write},
        os::windows::fs::OpenOptionsExt,
        path::Path,
        process::{Command, Stdio},
        sync::{Arc, Barrier},
        thread,
    };

    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_SPARSE_FILE,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE,
    };

    use super::{unsafe_file_attributes, ObjectKind, WindowsCommitError, WindowsWorkBuddyStorage};

    fn make_junction(link: &Path, target: &Path) {
        let status = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(Stdio::null())
            .status()
            .expect("cmd.exe must be available for the native junction test");
        assert!(status.success(), "directory junction creation failed");
    }

    fn assert_no_temp_leaves(directory: &Path) {
        if !directory.is_dir() {
            return;
        }
        let names = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            names.iter().all(|name| !name.contains(".tmp.")),
            "temporary WorkBuddy leaves must not remain: {names:?}"
        );
    }

    #[test]
    fn normal_create_replace_and_backup_use_the_pinned_directory() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        fs::create_dir(&home).unwrap();

        let storage = WindowsWorkBuddyStorage::open(&home, true).unwrap();
        let mut missing = storage.snapshot_models().unwrap();
        assert!(missing.bytes().is_none());
        storage.commit(&mut missing, b"first").unwrap();
        drop(storage);

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        let mut first = storage.snapshot_models().unwrap();
        assert_eq!(first.bytes(), Some(b"first".as_slice()));
        storage.commit(&mut first, b"second").unwrap();
        drop(storage);
        assert_eq!(
            fs::read(home.join(".workbuddy/models.json")).unwrap(),
            b"second"
        );
        assert_eq!(
            fs::read(home.join(".workbuddy/models.json.backup")).unwrap(),
            b"first"
        );

        // Exercise replacement of both an existing backup and an existing
        // primary while their no-delete handles remain open.
        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        let mut second = storage.snapshot_models().unwrap();
        storage.commit(&mut second, b"third").unwrap();
        drop(storage);
        assert_eq!(
            fs::read(home.join(".workbuddy/models.json")).unwrap(),
            b"third"
        );
        assert_eq!(
            fs::read(home.join(".workbuddy/models.json.backup")).unwrap(),
            b"second"
        );
        assert_no_temp_leaves(&home.join(".workbuddy"));
    }

    #[test]
    fn snapshot_blocks_writers_but_still_allows_atomic_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        fs::create_dir_all(&workbuddy).unwrap();
        let models = workbuddy.join("models.json");
        fs::write(&models, b"original").unwrap();

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        let mut snapshot = storage.snapshot_models().unwrap();
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .access_mode((FILE_GENERIC_READ | FILE_GENERIC_WRITE).0)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0);
        assert!(options.open(&models).is_err());

        storage.commit(&mut snapshot, b"replacement").unwrap();
        assert_eq!(fs::read(&models).unwrap(), b"replacement");
        assert_eq!(
            fs::read(workbuddy.join("models.json.backup")).unwrap(),
            b"original"
        );
        assert_no_temp_leaves(&workbuddy);
    }

    #[test]
    fn credential_file_attributes_that_cannot_be_preserved_are_unsafe() {
        let attributes = unsafe_file_attributes(ObjectKind::RegularFile);
        assert_ne!(attributes & FILE_ATTRIBUTE_ENCRYPTED.0, 0);
        assert_ne!(attributes & FILE_ATTRIBUTE_COMPRESSED.0, 0);
        assert_ne!(attributes & FILE_ATTRIBUTE_SPARSE_FILE.0, 0);
    }

    #[test]
    fn alternate_data_stream_is_rejected_before_backup_or_temp_creation() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        fs::create_dir_all(&workbuddy).unwrap();
        let models = workbuddy.join("models.json");
        fs::write(&models, b"primary").unwrap();
        let stream = models.as_os_str().to_os_string();
        let mut stream = stream.into_string().unwrap();
        stream.push_str(":secret");
        fs::write(stream, b"hidden").unwrap();

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        assert!(storage.snapshot_models().is_err());
        assert_eq!(fs::read(&models).unwrap(), b"primary");
        assert!(!workbuddy.join("models.json.backup").exists());
        assert_no_temp_leaves(&workbuddy);
    }

    #[test]
    fn parent_junction_is_rejected_without_touching_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let target = temp.path().join("protected-target");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel"), b"unchanged").unwrap();
        make_junction(&home.join(".workbuddy"), &target);

        let error = match WindowsWorkBuddyStorage::open(&home, true) {
            Ok(_) => panic!("a parent junction must be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            "WorkBuddy configuration storage is unavailable"
        );
        assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"unchanged");
        assert!(!target.join("models.json").exists());
        assert!(!target.join("models.json.backup").exists());
        assert_no_temp_leaves(&target);
    }

    #[test]
    fn leaf_reparse_points_are_rejected_without_target_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        let models_target = temp.path().join("models-target");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&workbuddy).unwrap();
        fs::create_dir(&models_target).unwrap();
        fs::write(models_target.join("sentinel"), b"unchanged").unwrap();
        make_junction(&workbuddy.join("models.json"), &models_target);

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        let error = storage.read_models().unwrap_err();
        assert_eq!(
            error.to_string(),
            "WorkBuddy configuration storage is unavailable"
        );
        assert_eq!(
            fs::read(models_target.join("sentinel")).unwrap(),
            b"unchanged"
        );
        assert!(!models_target.join("models.json.backup").exists());
        assert_no_temp_leaves(&models_target);
    }

    #[test]
    fn backup_leaf_reparse_fails_before_primary_commit() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        let backup_target = temp.path().join("backup-target");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&workbuddy).unwrap();
        fs::create_dir(&backup_target).unwrap();
        fs::write(workbuddy.join("models.json"), b"primary").unwrap();
        fs::write(backup_target.join("sentinel"), b"unchanged").unwrap();
        make_junction(&workbuddy.join("models.json.backup"), &backup_target);

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        let mut snapshot = storage.snapshot_models().unwrap();
        assert_eq!(
            storage.commit(&mut snapshot, b"replacement"),
            Err(WindowsCommitError::Backup)
        );
        assert_eq!(fs::read(workbuddy.join("models.json")).unwrap(), b"primary");
        assert_eq!(
            fs::read(backup_target.join("sentinel")).unwrap(),
            b"unchanged"
        );
        assert_no_temp_leaves(&workbuddy);
        assert_no_temp_leaves(&backup_target);
    }

    #[test]
    fn preexisting_same_size_writer_is_rejected_before_backup_or_temp_creation() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&workbuddy).unwrap();
        let models = workbuddy.join("models.json");
        fs::write(&models, b"AAAA").unwrap();

        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .access_mode((FILE_GENERIC_READ | FILE_GENERIC_WRITE).0)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0);
        let mut attacker = options.open(&models).unwrap();
        let ready = Arc::new(Barrier::new(2));
        let write_now = Arc::new(Barrier::new(2));
        let attacker_ready = Arc::clone(&ready);
        let attacker_write = Arc::clone(&write_now);
        let attack = thread::spawn(move || {
            attacker_ready.wait();
            attacker_write.wait();
            attacker.seek(SeekFrom::Start(0)).unwrap();
            attacker.write_all(b"BBBB").unwrap();
            attacker.flush().unwrap();
            attacker.sync_all().unwrap();
        });

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        ready.wait();
        assert!(storage.snapshot_models().is_err());
        write_now.wait();
        attack.join().unwrap();
        assert_eq!(fs::read(&models).unwrap(), b"BBBB");
        assert!(!workbuddy.join("models.json.backup").exists());
        assert_no_temp_leaves(&workbuddy);
    }

    #[test]
    fn pinned_directory_denies_namespace_swap_and_keeps_the_primary_transactional() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("profile");
        let workbuddy = home.join(".workbuddy");
        let moved = home.join(".workbuddy-moved");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&workbuddy).unwrap();
        fs::write(workbuddy.join("models.json"), b"primary").unwrap();

        let storage = WindowsWorkBuddyStorage::open(&home, false).unwrap();
        assert!(
            fs::rename(&workbuddy, &moved).is_err(),
            "a pinned directory must not remain renameable"
        );

        let mut snapshot = storage.snapshot_models().unwrap();
        storage.commit(&mut snapshot, b"replacement").unwrap();
        assert_eq!(
            fs::read(workbuddy.join("models.json")).unwrap(),
            b"replacement"
        );
        assert_eq!(
            fs::read(workbuddy.join("models.json.backup")).unwrap(),
            b"primary"
        );
        assert!(!moved.exists());
        assert_no_temp_leaves(&workbuddy);
    }
}
