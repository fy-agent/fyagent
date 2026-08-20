// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

fn main() {
    // Resolve Explorer's immutable Shell-user authority before the panic hook,
    // Tauri, or any user-path lookup. The elevated
    // process account is never a fallback for this boundary.
    #[cfg(target_os = "windows")]
    if let Err(code) = fyagent_lib::initialize_windows_user_context() {
        eprintln!("{code}");
        std::process::exit(1);
    }

    fyagent_lib::run();
}
