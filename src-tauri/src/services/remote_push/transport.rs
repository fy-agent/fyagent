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
    cmd.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
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
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("通过 SSH 发送数据失败：{e}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("等待 SSH 执行完成失败：{e}"))?;
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stdout, stderr))
}

fn single_quote_posix(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
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

/// Checks which candidate relative paths already exist under remote `$HOME`.
pub fn probe_remote_existing_paths(
    target: &ValidatedSshTarget,
    rel_paths: &[&str],
) -> Result<(Option<String>, HashSet<String>), String> {
    let mut script = String::from("set -e\nprintf 'FY_HOME:%s\\n' \"$HOME\"\n");
    for rel in rel_paths {
        let quoted = single_quote_posix(rel);
        script.push_str(&format!(
            "if [ -e \"$HOME/\"{} ]; then printf 'FY_EXISTS:%s\\n' {}; fi\n",
            quoted, quoted
        ));
    }
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

    let mut home = None;
    let mut existing = HashSet::new();
    for line in stdout.lines() {
        if let Some(h) = line.strip_prefix("FY_HOME:") {
            home = Some(h.trim().to_string());
        } else if let Some(p) = line.strip_prefix("FY_EXISTS:") {
            existing.insert(p.trim().to_string());
        }
    }
    Ok((home, existing))
}

/// Builds the zero-dependency POSIX shell script streamed over SSH stdin.
pub(crate) fn build_atomic_push_script(backup_stamp: &str, files: &[RenderedRemoteFile]) -> String {
    let mut script = String::from(
        "set -eu\n\
         if printf 'QQ==' | base64 -d >/dev/null 2>&1; then\n\
           fy_b64() { base64 -d; }\n\
         elif printf 'QQ==' | base64 -D >/dev/null 2>&1; then\n\
           fy_b64() { base64 -D; }\n\
         else\n\
           printf 'FY_ERR_NO_B64\\n'\n\
           exit 1\n\
         fi\n\
         BACKUP_DIR=\"$HOME/.fyagent-remote-backups/",
    );
    script.push_str(backup_stamp);
    script.push_str("\"\nmkdir -p \"$BACKUP_DIR\"\nprintf 'FY_BACKUP:%s\\n' \"$BACKUP_DIR\"\n");

    for (idx, file) in files.iter().enumerate() {
        let rel_q = single_quote_posix(&file.remote_rel_path);
        let b64 = BASE64.encode(file.content.as_slice());
        let marker = format!("FY_EOF_{idx}_{backup_stamp}");

        script.push_str(&format!(
            "REL={rel_q}\n\
             DEST=\"$HOME/$REL\"\n\
             DIR=\"$(dirname \"$DEST\")\"\n\
             mkdir -p \"$DIR\"\n\
             ACTION=\"created\"\n\
             if [ -f \"$DEST\" ]; then\n\
               ACTION=\"updated\"\n\
               BK_DEST=\"$BACKUP_DIR/$REL\"\n\
               mkdir -p \"$(dirname \"$BK_DEST\")\"\n\
               cp -p \"$DEST\" \"$BK_DEST\"\n\
             fi\n\
             TMP=\"$DEST.fyagent.tmp.$$\"\n\
             if fy_b64 > \"$TMP\" <<'{marker}'\n\
             {b64}\n\
             {marker}\n\
             then\n\
               chmod {mode} \"$TMP\"\n\
               mv -f \"$TMP\" \"$DEST\"\n\
               printf 'FY_OK:%s:%s\\n' \"$ACTION\" \"$REL\"\n\
             else\n\
               rm -f \"$TMP\"\n\
               printf 'FY_ERR:%s\\n' \"$REL\"\n\
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

pub fn apply_ssh_push(
    target: &ValidatedSshTarget,
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
    let script = build_atomic_push_script(&backup_stamp, files);
    let (code, stdout, stderr) = run_ssh_stdin(target, &script)?;

    let mut backup_dir = format!("~/.fyagent-remote-backups/{backup_stamp}");
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
                remote_path: format!("~/{}", f.remote_rel_path),
                ok: true,
                action: action.clone(),
                message: if action == "updated" {
                    "已备份原文件并更新生效".to_string()
                } else {
                    "已创建并生效".to_string()
                },
            },
            _ => RemotePushFileOutcome {
                remote_path: format!("~/{}", f.remote_rel_path),
                ok: false,
                action: "failed".to_string(),
                message: err_hint.to_string(),
            },
        })
        .collect();

    let all_ok = code == 0 && outcomes.iter().all(|o| o.ok);
    Ok(RemotePushApplyResult {
        ok: all_ok,
        backup_dir,
        outcomes,
        warnings,
    })
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
                content: Zeroizing::new(br#"{"OPENAI_API_KEY":"sk-new-value"}"#.to_vec()),
            },
            RenderedRemoteFile {
                remote_rel_path: ".claude/CLAUDE.md".to_string(),
                category: RemoteFileCategory::GlobalPrompt,
                mode: "644",
                contains_secret: false,
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
}
