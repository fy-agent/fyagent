// Prevents a console window from flashing when Explorer launches the packaged
// helper. Debug builds keep a console for local diagnostics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use fyagent_user_helper::parse_cli_args;
#[cfg(target_os = "windows")]
use fyagent_user_helper::SETTLED_FAILURE_EXIT_CODE;

#[cfg(target_os = "windows")]
mod windows;

const USAGE: &str = "usage: fyagent-user-helper.exe codex-msix-install --job-id <lowercase-uuid> --pipe <64-lowercase-hex>";

fn main() -> ExitCode {
    let request = match parse_cli_args(std::env::args_os().skip(1)) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("fyagent-user-helper: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    #[cfg(target_os = "windows")]
    {
        match windows::run_install(&request) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("fyagent-user-helper: {error}");
                ExitCode::from(SETTLED_FAILURE_EXIT_CODE)
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = request;
        eprintln!("fyagent-user-helper: this helper is available only on Windows");
        ExitCode::from(1)
    }
}
