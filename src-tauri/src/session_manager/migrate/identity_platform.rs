//! OS-owned identity primitives. Raw machine identifiers and user IDs never
//! leave this module; callers persist only domain-separated digests.

use std::fs::{File, OpenOptions};
#[cfg(target_os = "macos")]
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Owns the directory whose lock protects the identity transaction. macOS
/// child I/O is descriptor-relative; Windows pins the canonical ancestor chain
/// against rename/delete before retaining the existing atomic config writer.
pub(super) struct IdentityDirectory {
    #[cfg(target_os = "macos")]
    file: File,
    path: PathBuf,
    instance: String,
    #[cfg(target_os = "windows")]
    _ancestors: Vec<File>,
}

impl IdentityDirectory {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let path = path.canonicalize()?;
        #[cfg(target_os = "windows")]
        let ancestors = {
            use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
            use windows::Win32::Storage::FileSystem::{
                FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            let mut pinned = Vec::new();
            for component in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
                let file = OpenOptions::new()
                    .read(true)
                    .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
                    .open(component)?;
                let metadata = file.metadata()?;
                if !metadata.is_dir()
                    || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
                {
                    return Err(identity_unavailable());
                }
                pinned.push(file);
            }
            pinned
        };
        #[cfg(target_os = "windows")]
        let file = ancestors
            .last()
            .ok_or_else(identity_unavailable)?
            .try_clone()?;
        #[cfg(target_os = "macos")]
        let file = File::open(&path)?;
        if !file.metadata()?.is_dir() {
            return Err(identity_unavailable());
        }
        let instance = file_instance_from_handle(&file)?;
        let directory = Self {
            #[cfg(target_os = "macos")]
            file,
            path,
            instance,
            #[cfg(target_os = "windows")]
            _ancestors: ancestors,
        };
        directory.check_current()?;
        Ok(directory)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub(super) fn open(_: &Path) -> io::Result<Self> {
        Err(identity_unavailable())
    }

    pub(super) fn instance(&self) -> &str {
        &self.instance
    }

    pub(super) fn check_current(&self) -> io::Result<()> {
        if file_instance(&self.path)? != self.instance {
            return Err(identity_unavailable());
        }
        Ok(())
    }

    pub(super) fn open_lock(&self, name: &str) -> io::Result<File> {
        self.check_current()?;
        self.open_child(name, true, false)
    }

    pub(super) fn check_locked(&self, name: &str, lock: &IdentityLock) -> io::Result<()> {
        self.check_current()?;
        let current = self.open_child(name, false, false)?;
        if file_instance_from_handle(&current)? != file_instance_from_handle(&lock.file)? {
            return Err(identity_unavailable());
        }
        Ok(())
    }

    pub(super) fn read_child(&self, name: &str, limit: u64) -> io::Result<Option<Vec<u8>>> {
        self.check_current()?;
        let file = match self.open_child(name, false, false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "identity state exceeds its limit",
            ));
        }
        Ok(Some(bytes))
    }

    pub(super) fn write_child(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.check_current()?;
        #[cfg(target_os = "macos")]
        {
            // config::atomic_write only accepts paths. A directory replacement
            // could redirect it after a check, so this private metadata writer
            // uses the held directory fd for BOTH staging and atomic publication.
            let temporary = format!(".identity-{}.tmp", uuid::Uuid::new_v4().simple());
            let mut file = self.open_child(&temporary, true, true)?;
            let result = (|| {
                file.write_all(bytes)?;
                file.sync_all()?;
                self.check_current()?;
                unix_child::rename(&self.file, &temporary, name)?;
                self.file.sync_all()
            })();
            if result.is_err() {
                let _ = unix_child::unlink(&self.file, &temporary);
            }
            result
        }
        #[cfg(target_os = "windows")]
        {
            crate::config::atomic_write(&self.path.join(name), bytes)
                .map_err(|_| io::Error::other("identity state could not be committed"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (name, bytes);
            Err(identity_unavailable())
        }
    }

    fn open_child(&self, name: &str, create: bool, exclusive: bool) -> io::Result<File> {
        if name.is_empty() || name.contains(['/', '\\', '\0']) || matches!(name, "." | "..") {
            return Err(identity_unavailable());
        }
        #[cfg(target_os = "macos")]
        let file = unix_child::open(&self.file, name, create, exclusive)?;
        #[cfg(target_os = "windows")]
        let file = {
            use std::os::windows::fs::OpenOptionsExt;
            use windows::Win32::Storage::FileSystem::{
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            let mut options = OpenOptions::new();
            options
                .read(true)
                .write(create)
                .create(create)
                .create_new(exclusive)
                .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
            options.open(self.path.join(name))?
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return Err(identity_unavailable());
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                return Err(identity_unavailable());
            }
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::MetadataExt;
                use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
                if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
                    return Err(identity_unavailable());
                }
            }
            Ok(file)
        }
    }
}

#[cfg(target_os = "macos")]
mod unix_child {
    use super::*;
    use libc as sys;
    use std::ffi::CString;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    fn name(value: &str) -> io::Result<CString> {
        CString::new(value).map_err(|_| identity_unavailable())
    }
    pub(super) fn open(
        directory: &File,
        filename: &str,
        create: bool,
        exclusive: bool,
    ) -> io::Result<File> {
        let filename = name(filename)?;
        let mut flags = sys::O_CLOEXEC
            | sys::O_NOFOLLOW
            | if create {
                sys::O_RDWR | sys::O_CREAT | sys::O_EXCL
            } else {
                sys::O_RDONLY
            };
        if exclusive {
            flags |= sys::O_EXCL;
        }
        // SAFETY: live directory fd, NUL-terminated basename, fixed flags/mode.
        let fd = loop {
            let fd =
                unsafe { sys::openat(directory.as_raw_fd(), filename.as_ptr(), flags, 0o600u32) };
            if fd >= 0 {
                break fd;
            }
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if create
                && !exclusive
                && flags & sys::O_CREAT != 0
                && error.kind() == io::ErrorKind::AlreadyExists
            {
                // Simultaneous O_CREAT|O_NOFOLLOW opens on macOS can report
                // ENOENT while another caller creates the file. Elect the
                // creator atomically, then open its existing file without
                // another create lookup. O_NOFOLLOW and the transaction's
                // directory/lock identity checks remain in force.
                flags &= !(sys::O_CREAT | sys::O_EXCL);
                continue;
            }
            return Err(error);
        };
        // SAFETY: successful openat transfers a new owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    pub(super) fn rename(directory: &File, from: &str, to: &str) -> io::Result<()> {
        let from = name(from)?;
        let to = name(to)?;
        // SAFETY: both basenames are anchored to the same live directory fd.
        if unsafe {
            sys::renameat(
                directory.as_raw_fd(),
                from.as_ptr(),
                directory.as_raw_fd(),
                to.as_ptr(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub(super) fn unlink(directory: &File, filename: &str) -> io::Result<()> {
        let filename = name(filename)?;
        // SAFETY: this fixed temporary basename belongs to this directory fd.
        if unsafe { sys::unlinkat(directory.as_raw_fd(), filename.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

/// Same flock/RAII ownership pattern as managed_auth's GrokAuthLock, with
/// actual Windows byte-range locking. The auth-specific owner cannot be reused:
/// it reports auth errors and intentionally has no Windows lock implementation.
pub(super) struct IdentityLock {
    file: File,
}

impl IdentityLock {
    pub(super) fn acquire(file: File) -> io::Result<Self> {
        set_lock(&file, true)?;
        Ok(Self { file })
    }
}

impl Drop for IdentityLock {
    fn drop(&mut self) {
        let _ = set_lock(&self.file, false);
    }
}

#[cfg(target_os = "macos")]
fn set_lock(file: &File, acquire: bool) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;
    let operation = if acquire {
        libc::LOCK_EX
    } else {
        libc::LOCK_UN
    };
    loop {
        // SAFETY: File owns a live descriptor and operation is a fixed flock flag.
        if unsafe { libc::flock(file.as_raw_fd(), operation) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(target_os = "windows")]
fn set_lock(file: &File, acquire: bool) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{LockFileEx, UnlockFileEx, LOCKFILE_EXCLUSIVE_LOCK},
        System::IO::OVERLAPPED,
    };
    let mut offset = OVERLAPPED::default();
    // Lock byte zero, including when the lock file is empty. A synchronous
    // handle and no FAIL_IMMEDIATELY flag give blocking cross-process exclusion.
    // SAFETY: The handle is live and the initialized offset lives through each call.
    let result = unsafe {
        if acquire {
            LockFileEx(
                HANDLE(file.as_raw_handle()),
                LOCKFILE_EXCLUSIVE_LOCK,
                None,
                1,
                0,
                &mut offset,
            )
        } else {
            UnlockFileEx(HANDLE(file.as_raw_handle()), None, 1, 0, &mut offset)
        }
    };
    result.map_err(|_| io::Error::other("migration identity file lock failed"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn set_lock(_: &File, _: bool) -> io::Result<()> {
    Err(identity_unavailable())
}

#[cfg(not(any(test, feature = "test-hooks")))]
pub(super) fn machine_user_fingerprint() -> io::Result<String> {
    machine_user_fingerprint_native()
}

#[cfg(all(target_os = "macos", not(any(test, feature = "test-hooks"))))]
fn machine_user_fingerprint_native() -> io::Result<String> {
    use std::process::{Command, Stdio};
    // A fixed Apple system binary and fixed arguments, never a PATH lookup or
    // shell command. The platform UUID is not a credential.
    let output = Command::new("/usr/sbin/ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() || output.stdout.len() > 128 * 1024 {
        return Err(identity_unavailable());
    }
    let text = std::str::from_utf8(&output.stdout).map_err(|_| identity_unavailable())?;
    let mut values = text.lines().filter_map(|line| {
        line.trim()
            .strip_prefix("\"IOPlatformUUID\" = ")
            .and_then(|raw| raw.strip_prefix('"'))
            .and_then(|raw| raw.strip_suffix('"'))
    });
    let machine = uuid::Uuid::parse_str(values.next().ok_or_else(identity_unavailable)?)
        .map_err(|_| identity_unavailable())?;
    if machine.is_nil() || values.next().is_some() {
        return Err(identity_unavailable());
    }
    // SAFETY: geteuid has no arguments and returns the process's effective UID.
    let user = unsafe { libc::geteuid() }.to_string();
    Ok(super::domain_hash(
        "fyagent.machine-user.v1",
        &["macos", &machine.to_string(), &user],
    ))
}

#[cfg(all(target_os = "windows", not(any(test, feature = "test-hooks"))))]
fn machine_user_fingerprint_native() -> io::Result<String> {
    use winreg::{
        enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY},
        RegKey,
    };
    let machine_key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Cryptography",
        KEY_READ | KEY_WOW64_64KEY,
    )?;
    let machine: String = machine_key.get_value("MachineGuid")?;
    let machine = uuid::Uuid::parse_str(machine.trim()).map_err(|_| identity_unavailable())?;
    if machine.is_nil() {
        return Err(identity_unavailable());
    }
    // FyAgent release is elevated. Its frozen Explorer user owns application
    // state; the UAC administrator's process SID is not the right identity.
    let user =
        crate::windows_runtime::interactive_user_context().ok_or_else(identity_unavailable)?;
    if !crate::windows_runtime::revalidate_interactive_user_context(user) {
        return Err(identity_unavailable());
    }
    Ok(super::domain_hash(
        "fyagent.machine-user.v1",
        &["windows", &machine.to_string(), user.canonical_sid()],
    ))
}

#[cfg(all(
    not(any(target_os = "macos", target_os = "windows")),
    not(any(test, feature = "test-hooks"))
))]
fn machine_user_fingerprint_native() -> io::Result<String> {
    Err(identity_unavailable())
}

fn identity_unavailable() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "OS identity is unavailable")
}

/// Open and identify one existing regular file or directory. Do not use
/// modification time or length: appending native history must keep the ID.
pub(super) fn file_instance(path: &Path) -> io::Result<String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
        options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(identity_unavailable());
    }
    file_instance_from_handle(&file)
}

#[cfg(target_os = "macos")]
fn file_instance_from_handle(file: &File) -> io::Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    let created = metadata
        .created()?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| identity_unavailable())?;
    if metadata.ino() == 0 {
        return Err(identity_unavailable());
    }
    Ok(format!(
        "unix:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        created.as_secs(),
        created.subsec_nanos()
    ))
}

#[cfg(target_os = "windows")]
fn file_instance_from_handle(file: &File) -> io::Result<String> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: File owns a live handle and the output points to initialized storage.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(|_| identity_unavailable())?;
    let index =
        (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow);
    let created = (u64::from(information.ftCreationTime.dwHighDateTime) << 32)
        | u64::from(information.ftCreationTime.dwLowDateTime);
    if index == 0 || created == 0 {
        return Err(identity_unavailable());
    }
    Ok(format!(
        "windows:{}:{index}:{created}",
        information.dwVolumeSerialNumber
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn file_instance_from_handle(_: &File) -> io::Result<String> {
    Err(identity_unavailable())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn concurrent_lock_creation_opens_one_file_instance() {
        // Exercise the cold-create race independently of the lock-protected
        // JSON transaction, and retain every OS error before dropping the fixture.
        for _ in 0..32 {
            let state = tempfile::tempdir().unwrap();
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(12));
            let workers = (0..12)
                .map(|_| {
                    let directory = IdentityDirectory::open(state.path()).unwrap();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        let file = directory.open_lock("identity.lock")?;
                        file_instance_from_handle(&file)
                    })
                })
                .collect::<Vec<_>>();
            let results = workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .collect::<Vec<_>>();
            assert!(results.iter().all(Result::is_ok), "{results:?}");
            let instances = results.into_iter().map(Result::unwrap).collect::<Vec<_>>();
            assert!(instances.iter().all(|instance| instance == &instances[0]));
        }
    }

    #[test]
    fn atomic_publication_after_directory_swap_stays_in_locked_directory() {
        let base = tempfile::tempdir().unwrap();
        let path = base.path().join("identity");
        let moved = base.path().join("previous");
        std::fs::create_dir(&path).unwrap();
        let directory = IdentityDirectory::open(&path).unwrap();
        let mut staged = directory.open_child("temporary", true, true).unwrap();
        staged.write_all(b"owned state").unwrap();
        staged.sync_all().unwrap();
        // Model the exact final-check -> rename window, after staging succeeded.
        directory.check_current().unwrap();
        std::fs::rename(&path, &moved).unwrap();
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("install-identity.json"), b"replacement state").unwrap();
        unix_child::rename(&directory.file, "temporary", "install-identity.json").unwrap();
        assert_eq!(
            std::fs::read(path.join("install-identity.json")).unwrap(),
            b"replacement state"
        );
        assert_eq!(
            std::fs::read(moved.join("install-identity.json")).unwrap(),
            b"owned state"
        );
        assert!(directory.check_current().is_err());
    }
}
