use super::{ConfigPack, PackError, Result, MAX_BYTES};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path},
};

fn check_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path.extension().and_then(|x| x.to_str()) != Some("json")
    {
        return Err(PackError::UnsafeContent);
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(PackError::UnsafeContent);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(PackError::UnsafeContent);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && ancestor == path => {}
            Err(_) => return Err(PackError::FileUnavailable),
        }
    }
    Ok(())
}
pub(crate) fn read_file(path: &Path) -> Result<String> {
    check_path(path)?;
    // Reject devices/directories/FIFOs before open as well as on the acquired
    // handle. This avoids blocking on a selected special file on dev hosts.
    let selected = fs::symlink_metadata(path).map_err(|_| PackError::FileUnavailable)?;
    if !selected.is_file() {
        return Err(PackError::UnsafeContent);
    }
    if selected.len() > MAX_BYTES as u64 {
        return Err(PackError::TooLarge);
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(|_| PackError::FileUnavailable)?;
    let metadata = file.metadata().map_err(|_| PackError::FileUnavailable)?;
    if !metadata.is_file() {
        return Err(PackError::UnsafeContent);
    }
    if metadata.len() > MAX_BYTES as u64 {
        return Err(PackError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PackError::FileUnavailable)?;
    // Strictly validate before exposing any imported text to a renderer.
    ConfigPack::parse(&bytes)?.text()
}
pub(crate) fn export_file(path: &Path, bytes: &[u8]) -> Result<()> {
    ConfigPack::parse(bytes)?;
    check_path(path)?;
    if !path
        .file_name()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x.ends_with(".fyagent-config.json"))
    {
        return Err(PackError::UnsafeContent);
    }
    if path.exists() {
        return Err(PackError::Conflict);
    }
    let parent = path.parent().ok_or(PackError::UnsafeContent)?;
    let mut stage = tempfile::NamedTempFile::new_in(parent).map_err(|_| PackError::WriteFailed)?;
    stage
        .write_all(bytes)
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| PackError::WriteFailed)?;
    // Create-only publication never replaces a preexisting file. Application
    // config writes/undo remain owned by config/recovery and Change Plan.
    stage
        .persist_noclobber(path)
        .map_err(|_| PackError::WriteFailed)?;
    if ConfigPack::parse(bytes)?.text()? != read_file(path)? {
        return Err(PackError::ReadbackFailed);
    }
    Ok(())
}
