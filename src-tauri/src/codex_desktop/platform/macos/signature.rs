//! Admission checks for the downloaded Codex application, independent of its basename.

use std::path::Path;

use super::{command, stable_bundle_id, CommandOutput, CommandRunner};
use crate::codex_desktop::error::{InstallerError, InstallerErrorCode};

const TEAM_ID: &str = "2DC432GLL2";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CodesignIdentity<'a> {
    identifier: &'a str,
    team_identifier: &'a str,
}

fn rejected(message: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageSignatureInvalid)
        .with_diagnostic_message(message)
}

pub(super) fn parse_codesign_display(output: &str) -> Result<CodesignIdentity<'_>, InstallerError> {
    let mut identifier = None;
    let mut team_identifier = None;
    for line in output.lines() {
        let (slot, value) = if let Some(value) = line.strip_prefix("Identifier=") {
            (&mut identifier, value)
        } else if let Some(value) = line.strip_prefix("TeamIdentifier=") {
            (&mut team_identifier, value)
        } else {
            continue;
        };
        if slot.replace(value).is_some() {
            return Err(rejected("签名身份字段重复，已拒绝安装"));
        }
    }
    let identifier = identifier
        .filter(|value| !value.is_empty())
        .ok_or_else(|| rejected("签名缺少程序编号 Identifier，已拒绝安装"))?;
    let team_identifier = team_identifier
        .filter(|value| !value.is_empty() && *value != "not set")
        .ok_or_else(|| rejected("签名缺少有效团队编号 TeamIdentifier，已拒绝安装"))?;
    Ok(CodesignIdentity {
        identifier,
        team_identifier,
    })
}

pub(super) fn verify_bundle_signature(
    runner: &dyn CommandRunner,
    bundle: &Path,
) -> Result<(), InstallerError> {
    let display = run_check(
        runner,
        bundle,
        "codesign",
        &["--display", "--verbose=4"],
        "无法读取应用签名（codesign --display 失败），已拒绝安装",
    )?;
    let output = std::str::from_utf8(display.stderr())
        .map_err(|_| rejected("应用签名输出不是有效 UTF-8，已拒绝安装"))?;
    let identity = parse_codesign_display(output)?;
    if identity.identifier != stable_bundle_id() {
        return Err(rejected(
            "签名程序编号不匹配：必须为 com.openai.codex，已拒绝安装",
        ));
    }
    if identity.team_identifier != TEAM_ID {
        return Err(rejected(
            "签名团队编号不匹配：必须为 2DC432GLL2，已拒绝安装",
        ));
    }
    run_check(
        runner,
        bundle,
        "codesign",
        &["--verify", "--deep", "--strict"],
        "应用签名完整性校验失败（codesign --verify），已拒绝安装",
    )?;
    run_check(
        runner,
        bundle,
        "spctl",
        &["--assess", "--type", "execute", "--verbose=2"],
        "macOS 安全评估未通过（spctl），请检查签名、公证或吊销状态，已拒绝安装",
    )?;
    Ok(())
}

fn run_check(
    runner: &dyn CommandRunner,
    bundle: &Path,
    program: &'static str,
    arguments: &[&str],
    message: &str,
) -> Result<CommandOutput, InstallerError> {
    let invocation = command(
        program,
        arguments
            .iter()
            .map(std::ffi::OsString::from)
            .chain(std::iter::once(bundle.as_os_str().to_owned())),
    );
    let output = runner.run(&invocation).map_err(|_| rejected(message))?;
    if !output.is_success() {
        return Err(rejected(message));
    }
    Ok(output)
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::{
        test_support::FakeRunner, CommandInvocation, CommandRunnerError, CommandRunnerErrorKind,
    };
    use super::*;

    pub(in crate::codex_desktop::platform::macos) fn queue_valid_signature(runner: &FakeRunner) {
        runner.queue_success_stderr(
            "codesign",
            display(stable_bundle_id(), TEAM_ID, "OpenAI OpCo, LLC"),
        );
        runner.queue_success("codesign", Vec::new());
        runner.queue_success("spctl", Vec::new());
    }

    fn display(id: &str, team: &str, authority: &str) -> String {
        format!("Identifier={id}\nAuthority={authority}\nTeamIdentifier={team}\n")
    }

    #[test]
    fn both_basenames_and_any_authority_pass_with_exact_commands() {
        for basename in ["Codex.app", "ChatGPT.app"] {
            for authority in ["OpenAI, L.L.C.", "OpenAI OpCo, LLC", "任意名称"] {
                let runner = FakeRunner::new();
                runner.queue_success_stderr(
                    "codesign",
                    display(stable_bundle_id(), TEAM_ID, authority),
                );
                runner.queue_success("codesign", Vec::new());
                runner.queue_success("spctl", Vec::new());
                let path = Path::new("/Volumes/Codex").join(basename);
                verify_bundle_signature(&runner, &path).unwrap();
                runner.assert_drained();
                let calls = runner.invocations();
                for (call, expected) in calls.iter().zip([
                    vec!["--display", "--verbose=4"],
                    vec!["--verify", "--deep", "--strict"],
                    vec!["--assess", "--type", "execute", "--verbose=2"],
                ]) {
                    let mut args: Vec<std::ffi::OsString> =
                        expected.into_iter().map(Into::into).collect();
                    args.push(path.as_os_str().to_owned());
                    assert_eq!(call.arguments(), args);
                }
            }
        }
    }

    #[test]
    fn invalid_identity_is_rejected_before_verification() {
        for output in [
            display(stable_bundle_id(), "WRONG", "ignored"),
            display("com.openai.chat", TEAM_ID, "ignored"),
            display("com.openai.codex.beta", TEAM_ID, "ignored"),
            display(stable_bundle_id(), "not set", "ignored"),
            "Identifier=com.openai.codex\n".into(),
            "TeamIdentifier=2DC432GLL2\n".into(),
            display("", TEAM_ID, "ignored"),
            display(stable_bundle_id(), "", "ignored"),
            format!(
                "{}Identifier=com.openai.codex\n",
                display(stable_bundle_id(), TEAM_ID, "ignored")
            ),
        ] {
            let runner = FakeRunner::new();
            runner.queue_success_stderr("codesign", output);
            let error = verify_bundle_signature(&runner, Path::new("/Volumes/Codex/Codex.app"))
                .unwrap_err();
            assert_eq!(error.code(), InstallerErrorCode::PackageSignatureInvalid);
            assert!(error
                .to_dto()
                .details
                .redacted_message
                .unwrap()
                .contains("已拒绝安装"));
            runner.assert_drained();
        }
    }

    #[test]
    fn every_nonzero_command_exit_is_rejected() {
        for failed_step in 0..3 {
            let runner = FakeRunner::new();
            if failed_step > 0 {
                runner.queue_success_stderr(
                    "codesign",
                    display(stable_bundle_id(), TEAM_ID, "ignored"),
                );
            }
            if failed_step > 1 {
                runner.queue_success("codesign", Vec::new());
            }
            runner.queue_failure(
                if failed_step == 2 {
                    "spctl"
                } else {
                    "codesign"
                },
                Some(1),
                "denied",
            );
            assert!(
                verify_bundle_signature(&runner, Path::new("/Volumes/Codex/Codex.app")).is_err()
            );
            runner.assert_drained();
        }
    }

    #[test]
    fn runner_failures_are_rejected() {
        struct FailedRunner(CommandRunnerErrorKind);
        impl CommandRunner for FailedRunner {
            fn run(&self, _: &CommandInvocation) -> Result<CommandOutput, CommandRunnerError> {
                Err(CommandRunnerError { kind: self.0 })
            }
        }
        for kind in [
            CommandRunnerErrorKind::Spawn,
            CommandRunnerErrorKind::Timeout,
            CommandRunnerErrorKind::Io,
        ] {
            assert!(verify_bundle_signature(&FailedRunner(kind), Path::new("/Codex.app")).is_err());
        }
    }
}
