//! Immutable, native-owned generations. Never follows links or overwrites user files.
//! Publication is the project row CAS; failed publication may leave an unreferenced
//! generation, but cannot expose partial content or overwrite a previous generation.
use super::{domain::MAX_CONTEXT_BYTES, project_error, validate_id};
use crate::error::AppError;
use std::path::{Path, PathBuf};

pub(super) fn directory(root: &Path, project: &str, generation: &str) -> Result<PathBuf, AppError> {
    validate_id(project)?;
    validate_id(generation)?;
    Ok(root.join(project).join(generation))
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::{
        ffi::CString,
        fs::File,
        io::{Read, Write},
        os::fd::{AsRawFd, FromRawFd},
        path::Component,
    };
    fn name(s: &std::ffi::OsStr) -> Result<CString, AppError> {
        use std::os::unix::ffi::OsStrExt;
        CString::new(s.as_bytes()).map_err(|_| project_error("path_rejected"))
    }
    fn child(parent: &File, s: &std::ffi::OsStr, create: bool) -> Result<File, AppError> {
        let c = name(s)?;
        if create {
            // mkdirat never traverses a symlink leaf. EEXIST still requires nofollow open.
            let rc = unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) };
            if rc != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                return Err(project_error("path_rejected"));
            }
        }
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(project_error("path_rejected"));
        }
        // SAFETY: successful openat returns a newly owned descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    fn root_dir(path: &Path, create: bool) -> Result<File, AppError> {
        if !path.is_absolute() {
            return Err(project_error("path_rejected"));
        }
        let mut dir = File::open("/").map_err(|_| project_error("path_rejected"))?;
        for part in path.components() {
            match part {
                Component::RootDir => {}
                Component::Normal(s) => {
                    dir = child(&dir, s, create)?;
                }
                _ => return Err(project_error("path_rejected")),
            }
        }
        Ok(dir)
    }
    fn generation(root: &Path, p: &str, g: &str, create: bool) -> Result<File, AppError> {
        validate_id(p)?;
        validate_id(g)?;
        let root = root_dir(root, create)?;
        let project = child(&root, std::ffi::OsStr::new(p), create)?;
        if create {
            let c = CString::new(g).map_err(|_| project_error("path_rejected"))?;
            if unsafe { libc::mkdirat(project.as_raw_fd(), c.as_ptr(), 0o700) } != 0 {
                return Err(project_error("path_rejected"));
            }
        }
        child(&project, std::ffi::OsStr::new(g), false)
    }
    fn read_file(dir: &File, filename: &str) -> Result<String, AppError> {
        let c = CString::new(filename).map_err(|_| project_error("path_rejected"))?;
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(project_error("context_unavailable"));
        }
        let f = unsafe { File::from_raw_fd(fd) };
        let metadata = f
            .metadata()
            .map_err(|_| project_error("context_unavailable"))?;
        if !metadata.is_file() || metadata.len() > MAX_CONTEXT_BYTES as u64 {
            return Err(project_error("path_rejected"));
        }
        let mut content = String::new();
        f.take((MAX_CONTEXT_BYTES + 1) as u64)
            .read_to_string(&mut content)
            .map_err(|_| project_error("context_unavailable"))?;
        if content.len() > MAX_CONTEXT_BYTES {
            return Err(project_error("invalid_request"));
        }
        Ok(content)
    }
    pub(super) fn read(root: &Path, p: &str, g: &str) -> Result<String, AppError> {
        read_file(&generation(root, p, g, false)?, "context.md")
    }
    pub(super) fn write(root: &Path, p: &str, g: &str, content: &str) -> Result<(), AppError> {
        if content.len() > MAX_CONTEXT_BYTES {
            return Err(project_error("invalid_request"));
        }
        let dir = generation(root, p, g, true)?;
        write_file(&dir, "context.md", content)
    }
    fn write_file(dir: &File, filename: &str, content: &str) -> Result<(), AppError> {
        let c = CString::new(filename).map_err(|_| project_error("path_rejected"))?;
        let fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                c.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(project_error("context_unavailable"));
        }
        let mut f = unsafe { File::from_raw_fd(fd) };
        f.write_all(content.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|_| project_error("context_unavailable"))?;
        dir.sync_all()
            .map_err(|_| project_error("context_unavailable"))?;
        if read_file(dir, filename)? != content {
            return Err(project_error("context_unavailable"));
        }
        Ok(())
    }
    pub(super) fn prepare_codex(
        root: &Path,
        p: &str,
        g: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let dir = generation(root, p, g, false)?;
        let home = child(&dir, std::ffi::OsStr::new("home"), true)?;
        let codex = child(&home, std::ffi::OsStr::new(".codex"), true)?;
        child(&codex, std::ffi::OsStr::new("skills"), true)?;
        child(&dir, std::ffi::OsStr::new("state"), true)?;
        let work = child(&dir, std::ffi::OsStr::new("workspace"), true)?;
        write_file(&codex, "config.toml", CODEX_CONFIG)?;
        write_file(&codex, "project.config.toml", CODEX_CONFIG)?;
        write_file(&work, "AGENTS.md", content)?;
        write_file(
            &dir,
            "CODEX-README.md",
            &codex_instructions(&directory(root, p, g)?),
        )?;
        verify_codex(root, p, g, content)
    }
    pub(super) fn verify_codex(
        root: &Path,
        p: &str,
        g: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let dir = generation(root, p, g, false)?;
        let home = child(&dir, std::ffi::OsStr::new("home"), false)?;
        let codex = child(&home, std::ffi::OsStr::new(".codex"), false)?;
        child(&dir, std::ffi::OsStr::new("state"), false)?;
        child(&codex, std::ffi::OsStr::new("skills"), false)?;
        let work = child(&dir, std::ffi::OsStr::new("workspace"), false)?;
        if read_file(&codex, "config.toml")? != CODEX_CONFIG
            || read_file(&codex, "project.config.toml")? != CODEX_CONFIG
            || read_file(&work, "AGENTS.md")? != content
            || read_file(&dir, "CODEX-README.md")? != codex_instructions(&directory(root, p, g)?)
        {
            return Err(project_error("context_changed"));
        }
        Ok(())
    }
}

pub(super) fn write(root: &Path, p: &str, g: &str, content: &str) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        native::write(root, p, g, content)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (root, p, g, content, MAX_CONTEXT_BYTES);
        Err(project_error("platform_unavailable"))
    }
}
pub(super) fn read(root: &Path, p: &str, g: &str) -> Result<String, AppError> {
    #[cfg(target_os = "macos")]
    {
        native::read(root, p, g)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (root, p, g, MAX_CONTEXT_BYTES);
        Err(project_error("platform_unavailable"))
    }
}

const CODEX_CONFIG: &str = "# FyAgent project preparation. No credentials, providers or MCP are copied.\napproval_policy = \"never\"\nsandbox_mode = \"read-only\"\n";

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
}
pub(super) fn codex_instructions(dir: &Path) -> String {
    let home = shell_quote(&dir.join("home"));
    let codex = shell_quote(&dir.join("home/.codex"));
    let state = shell_quote(&dir.join("state"));
    let work = shell_quote(&dir.join("workspace"));
    format!("# Codex 项目准备\n\n已生成独立工作目录、空认证目录与只读配置。没有复制账号、MCP、Skill 或全局配置；未启动 Agent。\n\n本机实验：codex-cli 0.154.0（macOS）。CLI 升级后请重新检查。先在终端确认受信 Codex CLI 路径，再运行只读检查：\n\n```sh\nCODEX_BIN=$(command -v codex)\nNODE_BIN=$(command -v node)\nPROJECT_PATH=\"$(dirname \"$NODE_BIN\"):/usr/bin:/bin:/usr/sbin:/sbin\"\nenv -i PATH=\"$PROJECT_PATH\" HOME={home} CODEX_HOME={codex} CODEX_SQLITE_HOME={state} \"$CODEX_BIN\" --profile project --cd {work} doctor --summary --no-color\nenv -i PATH=\"$PROJECT_PATH\" HOME={home} CODEX_HOME={codex} CODEX_SQLITE_HOME={state} \"$CODEX_BIN\" --profile project --cd {work} login status\n```\n\n首次检查应显示未登录、无 MCP。若结果不同，请停止并检查，不要沿用全局账号。完成本项目单独认证和权限核对后，可使用相同环境与工作目录调用 Codex；本产品尚未代为启动。\n\n系统管理配置、企业策略及祖先目录的规则/Skill 发现仍可能参与解析，必须另行核对。此目录边界不是操作系统沙箱，也没有证明推理、工具或客户验收通过。\n")
}
pub(super) fn prepare_codex(root: &Path, p: &str, g: &str, content: &str) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        native::prepare_codex(root, p, g, content)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (root, p, g, content);
        Err(project_error("platform_unavailable"))
    }
}
pub(super) fn verify_codex(root: &Path, p: &str, g: &str, content: &str) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        native::verify_codex(root, p, g, content)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (root, p, g, content, CODEX_CONFIG);
        Err(project_error("platform_unavailable"))
    }
}
