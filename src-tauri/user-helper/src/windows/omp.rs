//! Fixed OMP version observation executed only after ordinary-user admission.
use super::*;
use fyagent_user_helper::omp::{
    omp_candidates, parse_omp_version_output, OmpProbeResult, OMP_PROBE_TIMEOUT_SECS,
};

pub(super) fn execute() -> Result<OmpProbeResult, HelperErrorCode> {
    // Resolve known folders inside this helper process, never from the elevated host.
    let local = known_user_folder(&FOLDERID_LocalAppData)?;
    let profile = known_user_folder(&FOLDERID_Profile)?;
    let paths = interactive_path_directories().unwrap_or_default();
    let candidates = omp_candidates(Some(&local), Some(&profile), paths, true);
    let Some(candidate) = candidates.into_iter().find(|path| path.is_file()) else {
        return Ok(OmpProbeResult::not_detected());
    };
    validate_ordinary_dos_path(&candidate).map_err(|_| HelperErrorCode::ToolOwnerMismatch)?;
    let path = candidate
        .to_str()
        .ok_or(HelperErrorCode::ToolOwnerMismatch)?;
    // cmd.exe expands these characters even inside quotes; never admit them in a shim path.
    if path.chars().any(char::is_control)
        || (is_windows_command_script(&candidate)
            && path
                .chars()
                .any(|c| matches!(c, '%' | '!' | '^' | '"' | '&' | '|' | '<' | '>')))
    {
        return Err(HelperErrorCode::ToolOwnerMismatch);
    }
    let (output, exit_code) = run_grok_binary(
        &candidate,
        &["--version"],
        Duration::from_secs(OMP_PROBE_TIMEOUT_SECS),
    )?;
    if exit_code != 0 {
        return Err(HelperErrorCode::ToolExecutionFailed);
    }
    let version = parse_omp_version_output(&output).ok_or(HelperErrorCode::ToolExecutionFailed)?;
    let result = OmpProbeResult {
        detected: true,
        version: Some(version),
        executable_path: Some(path.to_owned()),
    };
    if !result.is_valid() {
        return Err(HelperErrorCode::ToolExecutionFailed);
    }
    Ok(result)
}

pub(super) fn run_session(
    controls: &ParentControls,
    channel: Arc<PipeChannel>,
) -> Result<(), HelperRunError> {
    // Reuse the existing fixed no-plan control and authenticated admission handshake.
    match channel.read_grok_npm_plan() {
        Ok(None) => {}
        _ => {
            channel.send_prestart_error(HelperErrorCode::ParentAdmissionFailed)?;
            return Err(HelperRunError::OperationFailed(
                HelperErrorCode::ParentAdmissionFailed,
            ));
        }
    }
    channel.send_started(TOOL_OPERATION_STARTED_IDENTITY)?;
    if let Err(code) = controls.wait_for_admission(ADMISSION_TIMEOUT) {
        channel.send_terminal(HelperMessage::error(code))?;
        return Err(HelperRunError::OperationFailed(code));
    }
    channel.mark_admitted()?;
    match execute() {
        Ok(result) => channel.send_terminal(HelperMessage::OmpProbeResult(result)),
        Err(code) => {
            channel.send_terminal(HelperMessage::error(code))?;
            Err(HelperRunError::OperationFailed(code))
        }
    }
}
