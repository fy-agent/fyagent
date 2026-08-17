use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::schema::{STATE_MAX_BYTES, sha256_hex};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

pub fn ensure_private_dir(path: &Path) -> io::Result<()> {
    if path.exists() {
        // Fresh injected TempDir roots are typically 0755. Tighten to 0700
        // only when the directory is still empty (no compiled state yet).
        if dir_is_uninitialized(path) {
            tighten_dir_mode(path)?;
        }
        verify_dir(path)?;
        return Ok(());
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)?;
    verify_dir(path)
}

fn dir_is_uninitialized(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(entries) => entries.filter_map(Result::ok).all(|entry| {
            let name = entry.file_name();
            name == *".DS_Store" || name == *"store.lock"
        }),
        Err(_) => false,
    }
}

#[cfg(unix)]
fn tighten_dir_mode(path: &Path) -> io::Result<()> {
    let mut perms = fs::symlink_metadata(path)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn tighten_dir_mode(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn verify_dir(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink dir refused"));
    }
    if !meta.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a directory"));
    }
    let mode = meta.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "directory group/world bits set",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_dir(path: &Path) -> io::Result<()> {
    if !path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a directory"));
    }
    Ok(())
}

pub fn open_lock_file(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        opts.mode(0o600);
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let file = opts.open(path)?;
    #[cfg(unix)]
    {
        let meta = file.metadata()?;
        if meta.permissions().mode() & 0o177 != 0o100 {
            // regular file expected; group/world bits must be clear
            if !meta.file_type().is_file() || meta.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "lock file mode refused",
                ));
            }
        }
        let rc = unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&file), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "store.lock already held",
            ));
        }
    }
    Ok(file)
}

pub fn temp_state_path(root: &Path) -> PathBuf {
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    root.join(format!(".tmp-state-{nonce}.json"))
}

pub fn write_atomic_json(root: &Path, dest: &Path, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > STATE_MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "state.json exceeds 4 MiB"));
    }
    let tmp = temp_state_path(root);
    write_private_file(&tmp, bytes)?;
    fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        opts.mode(0o600);
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    let mut file = opts.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn read_limited(path: &Path, max: usize) -> io::Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink refused"));
    }
    if !meta.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a regular file"));
    }
    if meta.len() as usize > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file exceeds bound"));
    }
    fs::read(path)
}

pub fn hex32_from_bytes(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}
