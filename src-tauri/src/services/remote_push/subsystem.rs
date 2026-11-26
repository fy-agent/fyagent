//! Windows-host manual configuration sync to a local WSL distribution.
//!
//! Reuses `remote_push::render` and the audited POSIX atomic shell transaction
//! from `remote_push::transport` over `wsl.exe -d <distro> -- sh -s` stdin so:
//! - Secrets never appear in command-line arguments or environment variables.
//! - Files are written via POSIX `sh` with `umask 077`, `chmod 600` on secret
//!   files, symlink/directory refusal, pre-overwrite backups, and full rollback.
//! - Unmanaged/official-only clients (e.g. TRAE/QoderWork) and private OAuth
//!   or `agent.db` credentials are never exported.

use serde::{Deserialize, Serialize};

use crate::store::AppState;

use super::{
    render::{render_remote_push_bundle_with_mapping, RemotePathMapping, RemotePushSelection},
    transport::{
        build_atomic_push_script_at, build_probe_existing_script, parse_probe_output,
        parse_push_apply_output, RemotePushApplyResult, RemotePushFileAction,
        RemotePushFilePreview,
    },
    RemotePushPreviewResult,
};

const WSL_BACKUP_DIR_NAME: &str = ".fyagent-wsl-backups";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslDistroEntry {
    pub name: String,
    pub state: String,
    pub version: u8,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslEnvironmentStatus {
    pub supported: bool,
    pub available: bool,
    pub distros: Vec<WslDistroEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WslSyncRequest {
    pub distro: String,
    pub selection: RemotePushSelection,
    #[serde(default)]
    pub path_mapping: Option<RemotePathMapping>,
}

pub fn validate_wsl_distro_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err("请选择有效的 WSL 发行版名称".to_string());
    }
    if trimmed.starts_with('-')
        || trimmed.contains("..")
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err("WSL 发行版名称包含非法字符".to_string());
    }
    Ok(trimmed.to_string())
}

/// Decodes raw stdout from `wsl.exe --list --verbose` (typically UTF-16LE on
/// Windows, occasionally UTF-8 under overridden console code pages).
#[cfg(any(target_os = "windows", test))]
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    // UTF-16LE BOM or alternating NUL bytes typical of Windows console UTF-16LE output.
    let looks_utf16le = bytes.starts_with(&[0xFF, 0xFE])
        || (bytes.len() >= 4
            && bytes.len() % 2 == 0
            && bytes
                .iter()
                .skip(1)
                .step_by(2)
                .take(16)
                .filter(|&&b| b == 0)
                .count()
                >= 4);

    if looks_utf16le {
        let start = if bytes.starts_with(&[0xFF, 0xFE]) {
            2
        } else {
            0
        };
        let units: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&units)
            .trim_start_matches('\u{FEFF}')
            .to_string()
    } else {
        String::from_utf8_lossy(bytes)
            .trim_start_matches('\u{FEFF}')
            .replace('\0', "")
    }
}

#[cfg(any(target_os = "windows", test))]
pub fn parse_wsl_list_verbose(bytes: &[u8]) -> Vec<WslDistroEntry> {
    let text = decode_wsl_output(bytes);
    let mut distros = Vec::new();
    let mut header_skipped = false;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !header_skipped {
            let upper = trimmed.to_ascii_uppercase();
            if upper.contains("NAME") && upper.contains("STATE") && upper.contains("VERSION") {
                header_skipped = true;
                continue;
            }
        }
        let is_default = trimmed.starts_with('*');
        let line_body = trimmed.trim_start_matches('*').trim();
        let tokens: Vec<&str> = line_body.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }
        let Ok(version) = tokens[tokens.len() - 1].parse::<u8>() else {
            continue;
        };
        let state = tokens[tokens.len() - 2].to_string();
        let name = tokens[..tokens.len() - 2].join(" ");
        if validate_wsl_distro_name(&name).is_ok() {
            distros.push(WslDistroEntry {
                name,
                state,
                version,
                is_default,
            });
        }
    }

    distros
}

#[cfg(target_os = "windows")]
pub fn list_wsl_distros() -> WslEnvironmentStatus {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = Command::new("wsl.exe")
        .args(["--list", "--verbose"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let distros = parse_wsl_list_verbose(&out.stdout);
            let message = if distros.is_empty() {
                Some("已检测到 WSL，但尚未安装任何发行版。".to_string())
            } else {
                None
            };
            WslEnvironmentStatus {
                supported: true,
                available: true,
                distros,
                message,
            }
        }
        Ok(out) => {
            let hint = decode_wsl_output(&out.stderr);
            let first = hint
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or("未检测到可用的 WSL 发行版，请确认已在 Windows 启用 WSL。");
            WslEnvironmentStatus {
                supported: true,
                available: false,
                distros: Vec::new(),
                message: Some(first.to_string()),
            }
        }
        Err(_) => WslEnvironmentStatus {
            supported: true,
            available: false,
            distros: Vec::new(),
            message: Some("本机未找到 wsl.exe，请先安装并初始化 WSL。".to_string()),
        },
    }
}

#[cfg(target_os = "macos")]
pub fn list_wsl_distros() -> WslEnvironmentStatus {
    WslEnvironmentStatus {
        supported: false,
        available: false,
        distros: Vec::new(),
        message: Some(
            "WSL 同步仅在 Windows 电脑可用；在 macOS 上请使用「打包迁移包」或「推送到 SSH 远端」。"
                .to_string(),
        ),
    }
}

#[cfg(target_os = "windows")]
fn run_wsl_stdin(distro: &str, script: &str) -> Result<(i32, String, String), String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = Command::new("wsl.exe");
    cmd.creation_flags(CREATE_NO_WINDOW)
        .args(["-d", distro, "--", "sh", "-s"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 wsl.exe（请确认本机已安装 WSL）：{e}"))?;
    let mut stdin = child.stdin.take();

    let (write_res, output_res) = std::thread::scope(|s| {
        let writer = s.spawn(move || -> Result<(), String> {
            if let Some(mut pipe) = stdin.take() {
                pipe.write_all(script.as_bytes())
                    .map_err(|e| format!("向 WSL 发送同步脚本失败：{e}"))?;
            }
            Ok(())
        });
        let output = child
            .wait_with_output()
            .map_err(|e| format!("等待 WSL 执行完成失败：{e}"));
        let write = writer
            .join()
            .unwrap_or_else(|_| Err("WSL 写入线程意外中止".to_string()));
        (write, output)
    });

    let output = output_res?;
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if code == 0 {
        write_res?;
    }
    Ok((code, stdout, stderr))
}

#[cfg(target_os = "macos")]
fn run_wsl_stdin(_distro: &str, _script: &str) -> Result<(i32, String, String), String> {
    Err("WSL 同步仅在 Windows 系统可用".to_string())
}

pub fn preview_wsl_config_sync(
    state: &AppState,
    request: WslSyncRequest,
) -> Result<RemotePushPreviewResult, String> {
    let distro = validate_wsl_distro_name(&request.distro)?;
    let bundle = render_remote_push_bundle_with_mapping(
        state,
        &request.selection,
        request.path_mapping.as_ref(),
        WSL_BACKUP_DIR_NAME,
    )?;
    let mut warnings = bundle.warnings;
    let display_prefix = bundle.display_prefix;
    let target_root = bundle.target_root;

    let rel_paths: Vec<&str> = bundle
        .files
        .iter()
        .map(|f| f.remote_rel_path.as_str())
        .collect();
    let (reachable, remote_home, existing_set) = if rel_paths.is_empty() {
        (true, None, std::collections::HashSet::new())
    } else {
        let probe_script = build_probe_existing_script(&rel_paths, target_root.as_deref());
        match run_wsl_stdin(&distro, &probe_script) {
            Ok((0, stdout, _)) => {
                let (home, existing) = parse_probe_output(&stdout);
                (true, home, existing)
            }
            Ok((_, _, stderr)) => {
                let hint = stderr
                    .lines()
                    .next()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("WSL 发行版未响应");
                warnings.push(format!(
                    "无法探测 WSL 发行版「{distro}」现有文件：{hint}（仍可查看待同步文件清单）"
                ));
                (false, None, std::collections::HashSet::new())
            }
            Err(err) => {
                warnings.push(format!("{err}（仍可查看待同步文件清单）"));
                (false, None, std::collections::HashSet::new())
            }
        }
    };

    let files: Vec<RemotePushFilePreview> = bundle
        .files
        .iter()
        .map(|f| {
            let action = if !reachable {
                RemotePushFileAction::Unknown
            } else if existing_set.contains(&f.remote_rel_path) {
                RemotePushFileAction::Update
            } else {
                RemotePushFileAction::Create
            };
            RemotePushFilePreview {
                remote_path: format!("{display_prefix}{}", f.remote_rel_path),
                category: f.category,
                bytes: f.content.len(),
                mode: f.mode.to_string(),
                contains_secret: f.contains_secret,
                action,
                summary: f.summary.clone(),
            }
        })
        .collect();

    let total_bytes = files.iter().map(|f| f.bytes).sum();

    Ok(RemotePushPreviewResult {
        destination: format!("WSL:{distro}"),
        remote_home,
        reachable,
        files,
        total_bytes,
        warnings,
    })
}

pub fn apply_wsl_config_sync(
    state: &AppState,
    request: WslSyncRequest,
) -> Result<RemotePushApplyResult, String> {
    let distro = validate_wsl_distro_name(&request.distro)?;
    let bundle = render_remote_push_bundle_with_mapping(
        state,
        &request.selection,
        request.path_mapping.as_ref(),
        WSL_BACKUP_DIR_NAME,
    )?;
    if bundle.files.is_empty() {
        return Err(
            "当前没有可同步的配置文件，请先在本地配置至少 1 项模型、MCP、提示词或 Skills"
                .to_string(),
        );
    }

    let backup_stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let script = build_atomic_push_script_at(
        WSL_BACKUP_DIR_NAME,
        &backup_stamp,
        bundle.target_root.as_deref(),
        &bundle.files,
    );
    let (code, stdout, stderr) = run_wsl_stdin(&distro, &script)?;
    let fallback_backup = format!("~/{WSL_BACKUP_DIR_NAME}/{backup_stamp}");
    Ok(parse_push_apply_output(
        code,
        &stdout,
        &stderr,
        fallback_backup,
        &bundle.display_prefix,
        &bundle.files,
        bundle.warnings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::remote_push::render::{RemoteFileCategory, RenderedRemoteFile};
    use zeroize::Zeroizing;

    fn encode_utf16le_with_bom(text: &str) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out
    }

    #[test]
    fn parses_utf16le_and_utf8_wsl_list_verbose_output() {
        let sample = "  NAME            STATE           VERSION\r\n* DevEnv-24       Running         2\r\n  WorkBox_2       Stopped         2\r\n";
        let utf16_bytes = encode_utf16le_with_bom(sample);
        let parsed = parse_wsl_list_verbose(&utf16_bytes);
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0],
            WslDistroEntry {
                name: "DevEnv-24".to_string(),
                state: "Running".to_string(),
                version: 2,
                is_default: true,
            }
        );
        assert_eq!(
            parsed[1],
            WslDistroEntry {
                name: "WorkBox_2".to_string(),
                state: "Stopped".to_string(),
                version: 2,
                is_default: false,
            }
        );

        let parsed_utf8 = parse_wsl_list_verbose(sample.as_bytes());
        assert_eq!(parsed_utf8, parsed);
    }

    #[test]
    fn rejects_invalid_wsl_distro_names() {
        assert!(validate_wsl_distro_name("DevEnv-24.04").is_ok());
        assert!(validate_wsl_distro_name("").is_err());
        assert!(validate_wsl_distro_name("-d").is_err());
        assert!(validate_wsl_distro_name("foo;rm -rf /").is_err());
        assert!(validate_wsl_distro_name("../escape").is_err());
    }

    #[test]
    fn wsl_atomic_script_uses_dedicated_backup_dir_and_hides_secrets() {
        let files = vec![RenderedRemoteFile {
            remote_rel_path: ".codex/auth.json".to_string(),
            category: RemoteFileCategory::ModelConfig,
            mode: "600",
            contains_secret: true,
            summary: "Codex API 凭据（密钥已隐藏，chmod 600）".to_string(),
            content: Zeroizing::new(br#"{"OPENAI_API_KEY":"sk-wsl-secret"}"#.to_vec()),
        }];
        let script = build_atomic_push_script_at(
            WSL_BACKUP_DIR_NAME,
            "20261125T180000Z",
            Some("~/custom-workspace"),
            &files,
        );
        assert!(script.contains(".fyagent-wsl-backups/20261125T180000Z"));
        assert!(script.contains("TARGET_ROOT=\"$HOME\"/'custom-workspace'"));
        assert!(script.contains("chmod 600"));
        assert!(!script.contains("sk-wsl-secret"));
    }
}
