//! Zero-dependency POSIX SSH transport for agentless config pushing.
//!
//! Invokes the local system `ssh` binary in non-interactive `BatchMode=yes`
//! mode (reusing `~/.ssh/config`, `ssh-agent`, and configured key files).
//! All file payloads and API keys travel exclusively through `ssh` stdin —
//! NEVER on command-line arguments or environment variables.

use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Serialize;

use super::{
    render::{RemoteFileCategory, RenderedRemoteFile},
    ssh_config::ValidatedSshTarget,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConnectionTestResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_home: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RemotePushFileAction {
    Create,
    Update,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePushFilePreview {
    pub remote_path: String,
    pub category: RemoteFileCategory,
    pub bytes: usize,
    pub mode: String,
    pub contains_secret: bool,
    pub action: RemotePushFileAction,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePushFileOutcome {
    pub remote_path: String,
    pub ok: bool,
    pub action: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePushApplyResult {
    pub ok: bool,
    pub backup_dir: String,
    pub outcomes: Vec<RemotePushFileOutcome>,
    pub warnings: Vec<String>,
}

fn build_base_ssh_command(target: &ValidatedSshTarget) -> Command {
    let mut cmd = Command::new("ssh");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        "-o",
        "ServerAliveInterval=10",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-T",
    ]);
    if let Some(port) = target.port {
        cmd.arg("-p").arg(port.to_string());
    }
    if let Some(identity) = target.identity_file.as_deref() {
        cmd.arg("-i").arg(identity);
    }
    cmd.arg("--").arg(&target.destination);
    cmd
}

fn run_ssh_stdin(
    target: &ValidatedSshTarget,
    script: &str,
) -> Result<(i32, String, String), String> {
    let mut cmd = build_base_ssh_command(target);
    cmd.arg("sh -s")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动本机 ssh 命令（请确认已安装 OpenSSH）：{e}"))?;
    let mut stdin = child.stdin.take();

    // Write stdin on a dedicated scoped thread while the main thread drains
    // stdout/stderr via `wait_with_output()`, preventing bidirectional pipe deadlocks
    // on large multi-file pushes.
    let (write_res, output_res) = std::thread::scope(|s| {
        let writer = s.spawn(move || -> Result<(), String> {
            if let Some(mut pipe) = stdin.take() {
                pipe.write_all(script.as_bytes())
                    .map_err(|e| format!("通过 SSH 发送数据失败：{e}"))?;
            }
            Ok(())
        });
        let output = child
            .wait_with_output()
            .map_err(|e| format!("等待 SSH 执行完成失败：{e}"));
        let write = writer
            .join()
            .unwrap_or_else(|_| Err("SSH 写入线程意外中止".to_string()));
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

fn encode_base64_wrapped(bytes: &[u8]) -> String {
    let raw = BASE64.encode(bytes);
    if raw.len() <= 76 {
        return raw;
    }
    let mut out = String::with_capacity(raw.len() + raw.len() / 76 + 1);
    for (idx, chunk) in raw.as_bytes().chunks(76).enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if let Ok(s) = std::str::from_utf8(chunk) {
            out.push_str(s);
        }
    }
    out
}

fn single_quote_posix(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

fn shell_target_root_expr(target_root: Option<&str>) -> String {
    match target_root {
        None => "\"$HOME\"".to_string(),
        Some(raw) => {
            if let Some(sub) = raw.strip_prefix("~/") {
                format!("\"$HOME\"/{}", single_quote_posix(sub))
            } else {
                single_quote_posix(raw)
            }
        }
    }
}

pub(crate) fn build_probe_existing_script(rel_paths: &[&str], target_root: Option<&str>) -> String {
    let root_expr = shell_target_root_expr(target_root);
    let mut script =
        format!("set -e\nTARGET_ROOT={root_expr}\nprintf 'FY_HOME:%s\\n' \"$TARGET_ROOT\"\n");
    for rel in rel_paths {
        let quoted = single_quote_posix(rel);
        script.push_str(&format!(
            "if [ -e \"$TARGET_ROOT/\"{} ]; then printf 'FY_EXISTS:%s\\n' {}; fi\n",
            quoted, quoted
        ));
    }
    script
}

pub(crate) fn parse_probe_output(stdout: &str) -> (Option<String>, HashSet<String>) {
    let mut home = None;
    let mut existing = HashSet::new();
    for line in stdout.lines() {
        if let Some(h) = line.strip_prefix("FY_HOME:") {
            home = Some(h.trim().to_string());
        } else if let Some(p) = line.strip_prefix("FY_EXISTS:") {
            existing.insert(p.trim().to_string());
        }
    }
    (home, existing)
}

pub fn test_ssh_connection(target: &ValidatedSshTarget) -> SshConnectionTestResult {
    let probe_script = r#"
set -e
if ! command -v base64 >/dev/null 2>&1; then
  printf 'FY_SSH_NO_BASE64\n'
  exit 2
fi
printf 'FY_SSH_OK:%s\n' "$HOME"
"#;
    match run_ssh_stdin(target, probe_script) {
        Ok((0, stdout, _)) => {
            let remote_home = stdout
                .lines()
                .find_map(|line| line.strip_prefix("FY_SSH_OK:"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            SshConnectionTestResult {
                ok: true,
                remote_home,
                message: "SSH 连通正常，远端支持无客户端直接写入配置".to_string(),
            }
        }
        Ok((_, stdout, _stderr)) if stdout.contains("FY_SSH_NO_BASE64") => {
            SshConnectionTestResult {
                ok: false,
                remote_home: None,
                message: "远端主机缺少 base64 命令，无法安全传输配置文件".to_string(),
            }
        }
        Ok((code, _, stderr)) => {
            let hint = stderr
                .lines()
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("请确认 SSH 公钥已授权（BatchMode 免密登录）");
            SshConnectionTestResult {
                ok: false,
                remote_home: None,
                message: format!("SSH 连接失败（退出码 {code}）：{hint}"),
            }
        }
        Err(err) => SshConnectionTestResult {
            ok: false,
            remote_home: None,
            message: err,
        },
    }
}

/// Checks which candidate relative paths already exist under the effective target root.
pub fn probe_remote_existing_paths_at(
    target: &ValidatedSshTarget,
    rel_paths: &[&str],
    target_root: Option<&str>,
) -> Result<(Option<String>, HashSet<String>), String> {
    let script = build_probe_existing_script(rel_paths, target_root);
    let (code, stdout, stderr) = run_ssh_stdin(target, &script)?;
    if code != 0 {
        let hint = stderr
            .lines()
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("SSH 探测失败");
        return Err(format!("无法探测远端文件状态：{hint}"));
    }

    Ok(parse_probe_output(&stdout))
}

/// Builds the zero-dependency POSIX shell script streamed over remote shell stdin.
///
/// Security invariants:
/// - `umask 077` is enabled before creating backups or temp files so secret-bearing
///   payloads are NEVER world- or group-readable even for a millisecond.
/// - Refuses to overwrite remote symlinks (`-L`) or directories (`-d`).
/// - Wraps base64 lines at 76 columns to stay strictly within POSIX `LINE_MAX`.
/// - Rolls back prior files in the batch from `$BACKUP_DIR` if any later file fails.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_atomic_push_script(backup_stamp: &str, files: &[RenderedRemoteFile]) -> String {
    build_atomic_push_script_at(".fyagent-remote-backups", backup_stamp, None, files)
}

pub(crate) fn build_atomic_push_script_at(
    backup_dir_name: &str,
    backup_stamp: &str,
    target_root: Option<&str>,
    files: &[RenderedRemoteFile],
) -> String {
    let root_expr = shell_target_root_expr(target_root);
    let mut script = format!(
        "set -eu\n\
         umask 077\n\
         if printf 'QQ==' | base64 -d >/dev/null 2>&1; then\n\
           fy_b64() {{ base64 -d; }}\n\
         elif printf 'QQ==' | base64 -D >/dev/null 2>&1; then\n\
           fy_b64() {{ base64 -D; }}\n\
         else\n\
           printf 'FY_ERR_NO_B64\\n'\n\
           exit 1\n\
         fi\n\
         TARGET_ROOT={root_expr}\n\
         mkdir -p \"$TARGET_ROOT\"\n\
         BACKUP_DIR=\"$HOME/{backup_dir_name}/{backup_stamp}\"\n\
         mkdir -p \"$BACKUP_DIR\"\n\
         chmod 700 \"$BACKUP_DIR\"\n\
         printf 'FY_BACKUP:%s\\n' \"$BACKUP_DIR\"\n\
         CREATED_LOG=\"$BACKUP_DIR/.fy_created.log\"\n\
         UPDATED_LOG=\"$BACKUP_DIR/.fy_updated.log\"\n\
         : > \"$CREATED_LOG\"\n\
         : > \"$UPDATED_LOG\"\n\
         fy_rollback() {{\n\
           if [ -f \"$UPDATED_LOG\" ]; then\n\
             while IFS= read -r _r; do\n\
               [ -n \"$_r\" ] && [ -f \"$BACKUP_DIR/$_r\" ] && cp -p \"$BACKUP_DIR/$_r\" \"$TARGET_ROOT/$_r\" || true\n\
             done < \"$UPDATED_LOG\"\n\
           fi\n\
           if [ -f \"$CREATED_LOG\" ]; then\n\
             while IFS= read -r _r; do\n\
               [ -n \"$_r\" ] && rm -f \"$TARGET_ROOT/$_r\" || true\n\
             done < \"$CREATED_LOG\"\n\
           fi\n\
         }}\n"
    );

    for (idx, file) in files.iter().enumerate() {
        let rel_q = single_quote_posix(&file.remote_rel_path);
        let b64 = encode_base64_wrapped(file.content.as_slice());
        let marker = format!("FY_EOF_{idx}_{backup_stamp}");

        script.push_str(&format!(
            "REL={rel_q}\n\
             DEST=\"$TARGET_ROOT/$REL\"\n\
             DIR=\"$(dirname \"$DEST\")\"\n\
             if [ -L \"$DIR\" ] || [ -L \"$DEST\" ] || [ -d \"$DEST\" ]; then\n\
               printf 'FY_ERR:%s\\n' \"$REL\"\n\
               printf 'Refusing to overwrite symlink or directory: %s\\n' \"$REL\" >&2\n\
               fy_rollback\n\
               exit 1\n\
             fi\n\
             mkdir -p \"$DIR\"\n\
             ACTION=\"created\"\n\
             if [ -f \"$DEST\" ]; then\n\
               ACTION=\"updated\"\n\
               BK_DEST=\"$BACKUP_DIR/$REL\"\n\
               mkdir -p \"$(dirname \"$BK_DEST\")\"\n\
               cp -p \"$DEST\" \"$BK_DEST\"\n\
             fi\n\
             TMP=\"$DEST.fyagent.tmp.$$\"\n\
             rm -f \"$TMP\"\n\
             if fy_b64 > \"$TMP\" <<'{marker}'\n\
{b64}\n\
{marker}\n\
             then\n\
               chmod {mode} \"$TMP\"\n\
               mv -f \"$TMP\" \"$DEST\"\n\
               if [ \"$ACTION\" = \"updated\" ]; then\n\
                 printf '%s\\n' \"$REL\" >> \"$UPDATED_LOG\"\n\
               else\n\
                 printf '%s\\n' \"$REL\" >> \"$CREATED_LOG\"\n\
               fi\n\
               printf 'FY_OK:%s:%s\\n' \"$ACTION\" \"$REL\"\n\
             else\n\
               rm -f \"$TMP\"\n\
               printf 'FY_ERR:%s\\n' \"$REL\"\n\
               fy_rollback\n\
               exit 1\n\
             fi\n",
            rel_q = rel_q,
            mode = file.mode,
            marker = marker,
            b64 = b64,
        ));
    }

    script
}

pub(crate) fn parse_push_apply_output(
    code: i32,
    stdout: &str,
    stderr: &str,
    fallback_backup_dir: String,
    display_prefix: &str,
    files: &[RenderedRemoteFile],
    warnings: Vec<String>,
) -> RemotePushApplyResult {
    let mut backup_dir = fallback_backup_dir;
    let mut status_map = std::collections::HashMap::<String, (bool, String)>::new();

    for line in stdout.lines() {
        if let Some(dir) = line.strip_prefix("FY_BACKUP:") {
            backup_dir = dir.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("FY_OK:") {
            if let Some((action, rel)) = rest.split_once(':') {
                status_map.insert(rel.to_string(), (true, action.to_string()));
            }
        } else if let Some(rel) = line.strip_prefix("FY_ERR:") {
            status_map.insert(rel.trim().to_string(), (false, "failed".to_string()));
        }
    }

    let err_hint = stderr
        .lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("远端写入中断");

    let outcomes: Vec<RemotePushFileOutcome> = files
        .iter()
        .map(|f| match status_map.get(&f.remote_rel_path) {
            Some((true, action)) => RemotePushFileOutcome {
                remote_path: format!("{display_prefix}{}", f.remote_rel_path),
                ok: true,
                action: action.clone(),
                message: if action == "updated" {
                    "已备份原文件并更新生效".to_string()
                } else {
                    "已创建并生效".to_string()
                },
            },
            _ => RemotePushFileOutcome {
                remote_path: format!("{display_prefix}{}", f.remote_rel_path),
                ok: false,
                action: "failed".to_string(),
                message: err_hint.to_string(),
            },
        })
        .collect();

    let all_ok = code == 0 && outcomes.iter().all(|o| o.ok);
    RemotePushApplyResult {
        ok: all_ok,
        backup_dir,
        outcomes,
        warnings,
    }
}

pub fn apply_ssh_push_at(
    target: &ValidatedSshTarget,
    target_root: Option<&str>,
    display_prefix: &str,
    files: &[RenderedRemoteFile],
    warnings: Vec<String>,
) -> Result<RemotePushApplyResult, String> {
    if files.is_empty() {
        return Err(
            "当前没有可推送的配置文件，请先在本地配置至少 1 项模型、MCP、提示词或 Skills"
                .to_string(),
        );
    }
    let backup_stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let script =
        build_atomic_push_script_at(".fyagent-remote-backups", &backup_stamp, target_root, files);
    let (code, stdout, stderr) = run_ssh_stdin(target, &script)?;
    Ok(parse_push_apply_output(
        code,
        &stdout,
        &stderr,
        format!("~/.fyagent-remote-backups/{backup_stamp}"),
        display_prefix,
        files,
        warnings,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    #[test]
    fn atomic_push_script_encodes_secrets_in_heredoc_and_sets_600_mode() {
        let files = vec![RenderedRemoteFile {
            remote_rel_path: ".codex/auth.json".to_string(),
            category: RemoteFileCategory::ModelConfig,
            mode: "600",
            contains_secret: true,
            summary: "Codex API 凭据".to_string(),
            content: Zeroizing::new(br#"{"OPENAI_API_KEY":"sk-ssh-test"}"#.to_vec()),
        }];
        let script = build_atomic_push_script("20261119T140000Z", &files);
        assert!(script.contains("chmod 600"));
        assert!(script.contains("BACKUP_DIR=\"$HOME/.fyagent-remote-backups/20261119T140000Z\""));
        assert!(!script.contains("sk-ssh-test"));
        assert!(script.contains("mv -f \"$TMP\" \"$DEST\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn atomic_push_script_executes_cleanly_in_posix_sh_with_backup_and_600_mode() {
        let temp = tempfile::tempdir().expect("temp home");
        let existing_dir = temp.path().join(".codex");
        std::fs::create_dir_all(&existing_dir).unwrap();
        std::fs::write(
            existing_dir.join("auth.json"),
            r#"{"OPENAI_API_KEY":"old"}"#,
        )
        .unwrap();

        let files = vec![
            RenderedRemoteFile {
                remote_rel_path: ".codex/auth.json".to_string(),
                category: RemoteFileCategory::ModelConfig,
                mode: "600",
                contains_secret: true,
                summary: "Codex API 凭据".to_string(),
                content: Zeroizing::new(br#"{"OPENAI_API_KEY":"sk-new-value"}"#.to_vec()),
            },
            RenderedRemoteFile {
                remote_rel_path: ".claude/CLAUDE.md".to_string(),
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
                summary: "Claude Code 全局提示词".to_string(),
                content: Zeroizing::new(b"# Global Rule\nUse clean Rust.\n".to_vec()),
            },
        ];

        let mut script = build_atomic_push_script("20261119T140000Z", &files);
        script.push_str("printf 'FY_MODE:%s\\n' \"$(stat -f '%Lp' \"$HOME/.codex/auth.json\")\"\n");
        let mut child = Command::new("sh")
            .arg("-s")
            .env("HOME", temp.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("FY_OK:updated:.codex/auth.json"));
        assert!(stdout.contains("FY_OK:created:.claude/CLAUDE.md"));
        assert!(stdout.contains("FY_MODE:600"));

        let written = std::fs::read_to_string(temp.path().join(".codex/auth.json")).unwrap();
        assert_eq!(written, r#"{"OPENAI_API_KEY":"sk-new-value"}"#);

        let backup = std::fs::read_to_string(
            temp.path()
                .join(".fyagent-remote-backups/20261119T140000Z/.codex/auth.json"),
        )
        .unwrap();
        assert_eq!(backup, r#"{"OPENAI_API_KEY":"old"}"#);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn atomic_push_script_rejects_symlinks_and_rolls_back_earlier_files() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("temp home");
        let codex_dir = temp.path().join(".codex");
        let claude_dir = temp.path().join(".claude");
        std::fs::create_dir_all(&codex_dir).unwrap();
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(codex_dir.join("auth.json"), r#"{"OPENAI_API_KEY":"orig"}"#).unwrap();

        let outside = temp.path().join("outside-target.txt");
        std::fs::write(&outside, "DO_NOT_TOUCH").unwrap();
        symlink(&outside, claude_dir.join("CLAUDE.md")).unwrap();

        let files = vec![
            RenderedRemoteFile {
                remote_rel_path: ".codex/auth.json".to_string(),
                category: RemoteFileCategory::ModelConfig,
                mode: "600",
                contains_secret: true,
                summary: "Codex API 凭据".to_string(),
                content: Zeroizing::new(br#"{"OPENAI_API_KEY":"updated-before-fail"}"#.to_vec()),
            },
            RenderedRemoteFile {
                remote_rel_path: ".claude/CLAUDE.md".to_string(),
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
                summary: "Claude Code 全局提示词".to_string(),
                content: Zeroizing::new(b"# Hijack attempt\n".to_vec()),
            },
        ];

        let script = build_atomic_push_script("20261119T140100Z", &files);
        assert!(script.contains("umask 077"));
        let mut child = Command::new("sh")
            .arg("-s")
            .env("HOME", temp.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success(), "push over symlink must fail");

        // Outside symlink target MUST remain untouched.
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "DO_NOT_TOUCH");
        // Earlier updated file MUST be automatically rolled back to original contents!
        assert_eq!(
            std::fs::read_to_string(codex_dir.join("auth.json")).unwrap(),
            r#"{"OPENAI_API_KEY":"orig"}"#
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn real_loopback_sshd_agentless_push_e2e_verifies_backup_and_600_permissions() {
        use std::net::TcpListener;
        use std::path::Path;
        use std::time::Duration;

        if !Path::new("/usr/sbin/sshd").is_file() {
            return;
        }

        // Bind a random free loopback port and release it for sshd.
        let listener = TcpListener::bind("127.0.0.1:0").expect("pick free port");
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let temp = tempfile::tempdir().expect("sshd temp workspace");
        let remote_home = temp.path().join("remote_home");
        let local_home = temp.path().join("local_home");
        std::fs::create_dir_all(remote_home.join(".codex")).unwrap();
        std::fs::create_dir_all(local_home.join(".ssh")).unwrap();
        std::fs::write(
            remote_home.join(".codex/auth.json"),
            r#"{"OPENAI_API_KEY":"legacy-remote-key"}"#,
        )
        .unwrap();

        let host_key = temp.path().join("host_key");
        let client_key = temp.path().join("client_key");
        let auth_keys = temp.path().join("authorized_keys");
        for path in [&host_key, &client_key] {
            let status = Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(path)
                .status()
                .expect("run ssh-keygen");
            assert!(status.success());
        }
        std::fs::copy(temp.path().join("client_key.pub"), &auth_keys).unwrap();

        let sshd_config = temp.path().join("sshd_config");
        std::fs::write(
            &sshd_config,
            format!(
                "ListenAddress 127.0.0.1:{port}\n\
                 HostKey {}\n\
                 AuthorizedKeysFile {}\n\
                 PidFile none\n\
                 StrictModes no\n\
                 UsePAM no\n\
                 PasswordAuthentication no\n\
                 KbdInteractiveAuthentication no\n\
                 PubkeyAuthentication yes\n\
                 ForceCommand /usr/bin/env HOME={} /bin/sh -c \"$SSH_ORIGINAL_COMMAND\"\n",
                host_key.display(),
                auth_keys.display(),
                remote_home.display(),
            ),
        )
        .unwrap();

        struct ChildGuard(std::process::Child);
        impl Drop for ChildGuard {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        let _sshd = ChildGuard(
            Command::new("/usr/sbin/sshd")
                .args(["-D", "-e", "-f"])
                .arg(&sshd_config)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn ephemeral sshd"),
        );

        // Isolate client known_hosts inside local_home so real user ~/.ssh is untouched.
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &local_home);
        struct RestoreHome(Option<String>);
        impl Drop for RestoreHome {
            fn drop(&mut self) {
                match &self.0 {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
        let _restore = RestoreHome(prev_home);

        // Wait up to 2s for sshd to accept TCP connections.
        for _ in 0..20 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        let target = ValidatedSshTarget {
            destination: "127.0.0.1".to_string(),
            port: Some(port),
            identity_file: Some(client_key.to_string_lossy().to_string()),
        };

        // 1. Live SSH connectivity probe
        let conn = test_ssh_connection(&target);
        assert!(conn.ok, "ssh test failed: {:?}", conn);
        assert_eq!(
            conn.remote_home.as_deref(),
            Some(remote_home.to_string_lossy().as_ref())
        );

        // 2. Live remote path existence probe
        let (_, existing) = probe_remote_existing_paths_at(
            &target,
            &[".codex/auth.json", ".claude/settings.json"],
            None,
        )
        .expect("probe remote paths");
        assert!(existing.contains(".codex/auth.json"));
        assert!(!existing.contains(".claude/settings.json"));

        // 3. Live agentless push over real TCP SSH stdin
        let files = vec![
            RenderedRemoteFile {
                remote_rel_path: ".codex/auth.json".to_string(),
                category: RemoteFileCategory::ModelConfig,
                mode: "600",
                contains_secret: true,
                summary: "Codex API 凭据".to_string(),
                content: Zeroizing::new(br#"{"OPENAI_API_KEY":"sk-real-ssh-push"}"#.to_vec()),
            },
            RenderedRemoteFile {
                remote_rel_path: ".claude/settings.json".to_string(),
                category: RemoteFileCategory::ModelConfig,
                mode: "600",
                contains_secret: true,
                summary: "Claude Code 模型配置".to_string(),
                content: Zeroizing::new(
                    br#"{"env":{"ANTHROPIC_AUTH_TOKEN":"sk-real-ssh-push"}}"#.to_vec(),
                ),
            },
            RenderedRemoteFile {
                remote_rel_path: ".claude/CLAUDE.md".to_string(),
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
                summary: "Claude Code 全局提示词".to_string(),
                content: Zeroizing::new(b"# Remote Rules\nShip clean code.\n".to_vec()),
            },
        ];
        let applied =
            apply_ssh_push_at(&target, None, "~/", &files, Vec::new()).expect("apply ssh push");
        assert!(applied.ok, "apply failed: {:?}", applied);
        assert_eq!(applied.outcomes.len(), 3);
        assert_eq!(applied.outcomes[0].action, "updated");
        assert_eq!(applied.outcomes[1].action, "created");
        assert_eq!(applied.outcomes[2].action, "created");

        // 4. Verify real files & automatic backup landed on isolated remote_home!
        assert_eq!(
            std::fs::read_to_string(remote_home.join(".codex/auth.json")).unwrap(),
            r#"{"OPENAI_API_KEY":"sk-real-ssh-push"}"#
        );
        assert_eq!(
            std::fs::read_to_string(remote_home.join(".claude/CLAUDE.md")).unwrap(),
            "# Remote Rules\nShip clean code.\n"
        );
        let backup_auth = Path::new(&applied.backup_dir).join(".codex/auth.json");
        assert_eq!(
            std::fs::read_to_string(&backup_auth).expect("backup auth.json"),
            r#"{"OPENAI_API_KEY":"legacy-remote-key"}"#
        );
        let stat_out = Command::new("stat")
            .args(["-f", "%Lp"])
            .arg(remote_home.join(".claude/settings.json"))
            .output()
            .expect("stat remote secret file");
        assert_eq!(String::from_utf8_lossy(&stat_out.stdout).trim(), "600");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn atomic_push_script_supports_custom_target_root_and_backups() {
        let temp = tempfile::tempdir().expect("temp home");
        let existing_dir = temp.path().join("work-env").join(".claude-alt");
        std::fs::create_dir_all(&existing_dir).unwrap();
        std::fs::write(existing_dir.join("CLAUDE.md"), "# Old Rule\n").unwrap();

        let files = vec![RenderedRemoteFile {
            remote_rel_path: ".claude-alt/CLAUDE.md".to_string(),
            category: RemoteFileCategory::GlobalPrompt,
            mode: "644",
            contains_secret: false,
            summary: "Claude Code 全局提示词".to_string(),
            content: Zeroizing::new(b"# Custom Root Rule\n".to_vec()),
        }];

        let script = build_atomic_push_script_at(
            ".fyagent-remote-backups",
            "20261125T180500Z",
            Some("~/work-env"),
            &files,
        );
        let mut child = Command::new("sh")
            .arg("-s")
            .env("HOME", temp.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(existing_dir.join("CLAUDE.md")).unwrap(),
            "# Custom Root Rule\n"
        );
        assert_eq!(
            std::fs::read_to_string(
                temp.path()
                    .join(".fyagent-remote-backups/20261125T180500Z/.claude-alt/CLAUDE.md")
            )
            .unwrap(),
            "# Old Rule\n"
        );
    }
}
