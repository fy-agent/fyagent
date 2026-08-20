//! Windows Explorer-backed interactive-user launcher.
//!
//! Microsoft documents the Explorer `ShellExecute` route specifically for
//! starting an unelevated process from an elevated process. This adapter only
//! invokes that route and returns an unavailable error when the Explorer COM
//! objects cannot be acquired. It has no elevated fallback.

use std::{
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    time::Duration,
};

use windows::{
    core::{Interface, BSTR},
    Win32::{
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_LOCAL_SERVER,
                COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
            },
            Variant::VARIANT,
        },
        UI::{
            Shell::{
                IShellBrowser, IShellDispatch2, IShellFolderViewDual, IShellWindows,
                IUnknown_QueryService, SID_STopLevelBrowser, ShellWindows, SVGIO_BACKGROUND,
                SWC_DESKTOP, SWFO_NEEDDISPATCH,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
};

use crate::platform::process_launch::{
    InteractiveUserLauncher, ProcessLaunchError, UserHelperLaunchOutcome,
};
use fyagent_user_helper::{CanonicalJobId, PipeNonce, INSTALL_ACTION};

const USER_HELPER_LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
static USER_HELPER_LAUNCH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

pub(crate) struct ExplorerInteractiveUserLauncher;

impl InteractiveUserLauncher for ExplorerInteractiveUserLauncher {
    fn open_http_url(&self, url: &str) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(url.to_owned())
    }

    fn open_directory(&self, directory: &Path) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(directory.to_string_lossy().to_string())
    }

    fn open_terminal_script(&self, script: &Path) -> Result<(), ProcessLaunchError> {
        launch_from_explorer(script.to_string_lossy().to_string())
    }

    fn open_trusted_windows_app_aumid(&self, aumid: &str) -> Result<(), ProcessLaunchError> {
        // `shell:AppsFolder\<AUMID>` is Explorer's application namespace form.
        // The common launcher has already accepted only the strict AUMID
        // grammar; this adapter adds no executable, argument, or fallback.
        launch_from_explorer(format!(r"shell:AppsFolder\{aumid}"))
    }

    fn launch_fyagent_user_helper(
        &self,
        job_id: &CanonicalJobId,
        pipe_nonce: &PipeNonce,
    ) -> Result<(), ProcessLaunchError> {
        match self.begin_fyagent_user_helper_launch(job_id, pipe_nonce) {
            UserHelperLaunchOutcome::Confirmed => Ok(()),
            UserHelperLaunchOutcome::MayHaveLaunched => {
                Err(ProcessLaunchError::InteractiveUserUnavailable)
            }
            UserHelperLaunchOutcome::NotInvoked(error) => Err(error),
        }
    }

    fn begin_fyagent_user_helper_launch(
        &self,
        job_id: &CanonicalJobId,
        pipe_nonce: &PipeNonce,
    ) -> UserHelperLaunchOutcome {
        let helper = match crate::platform::process_launch::fixed_user_helper_path() {
            Ok(helper) => helper,
            Err(error) => return UserHelperLaunchOutcome::NotInvoked(error),
        };
        let arguments = format!(
            "{INSTALL_ACTION} --job-id {job_id} --pipe {}",
            pipe_nonce.as_str()
        );
        launch_path_from_explorer_with_arguments(helper, arguments)
    }
}

/// Runs the COM automation call on a fresh STA thread. A fresh apartment keeps
/// the proxy independent from the Tauri runtime worker's COM mode and lets us
/// balance successful initialization with `CoUninitialize` on the same thread.
fn launch_from_explorer(target: String) -> Result<(), ProcessLaunchError> {
    launch_from_explorer_optional_arguments(target, None)
}

fn launch_path_from_explorer_with_arguments(
    target: PathBuf,
    arguments: String,
) -> UserHelperLaunchOutcome {
    let launch_slot = match UserHelperLaunchSlot::acquire() {
        Ok(slot) => slot,
        Err(error) => return UserHelperLaunchOutcome::NotInvoked(error),
    };
    let (sender, receiver) = mpsc::sync_channel(1);
    if std::thread::Builder::new()
        .name("fyagent-user-helper-launch".to_owned())
        .spawn(move || {
            let _launch_slot = launch_slot;
            let result = launch_path_from_explorer_sta(&target, &arguments);
            let _ = sender.send(result);
        })
        .is_err()
    {
        return UserHelperLaunchOutcome::NotInvoked(ProcessLaunchError::InteractiveUserUnavailable);
    }

    match receiver.recv_timeout(USER_HELPER_LAUNCH_TIMEOUT) {
        Ok(UserHelperStaOutcome::NotInvoked(error)) => UserHelperLaunchOutcome::NotInvoked(error),
        Ok(UserHelperStaOutcome::Invoked(Ok(()))) => UserHelperLaunchOutcome::Confirmed,
        Ok(UserHelperStaOutcome::Invoked(Err(_)))
        | Err(RecvTimeoutError::Timeout)
        | Err(RecvTimeoutError::Disconnected) => UserHelperLaunchOutcome::MayHaveLaunched,
    }
}

struct UserHelperLaunchSlot;

impl UserHelperLaunchSlot {
    fn acquire() -> Result<Self, ProcessLaunchError> {
        USER_HELPER_LAUNCH_IN_FLIGHT
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
        Ok(Self)
    }
}

impl Drop for UserHelperLaunchSlot {
    fn drop(&mut self) {
        USER_HELPER_LAUNCH_IN_FLIGHT.store(false, Ordering::Release);
    }
}

fn launch_from_explorer_optional_arguments(
    target: String,
    arguments: Option<String>,
) -> Result<(), ProcessLaunchError> {
    std::thread::Builder::new()
        .name("fyagent-explorer-launch".to_owned())
        .spawn(move || launch_from_explorer_sta(&target, arguments.as_deref()))
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?
        .join()
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?
}

fn launch_from_explorer_sta(
    target: &str,
    arguments: Option<&str>,
) -> Result<(), ProcessLaunchError> {
    launch_from_explorer_sta_bstr(BSTR::from(target), arguments)
}

fn launch_path_from_explorer_sta(target: &Path, arguments: &str) -> UserHelperStaOutcome {
    let target = target.as_os_str().encode_wide().collect::<Vec<_>>();
    launch_user_helper_from_explorer_sta_bstr(BSTR::from_wide(&target), arguments)
}

enum UserHelperStaOutcome {
    NotInvoked(ProcessLaunchError),
    Invoked(Result<(), ProcessLaunchError>),
}

fn launch_user_helper_from_explorer_sta_bstr(
    target: BSTR,
    arguments: &str,
) -> UserHelperStaOutcome {
    let initialized =
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if initialized.is_err() {
        return UserHelperStaOutcome::NotInvoked(ProcessLaunchError::InteractiveUserUnavailable);
    }

    let dispatch = explorer_shell_dispatch();

    let outcome = match dispatch {
        Err(error) => UserHelperStaOutcome::NotInvoked(error),
        Ok(shell_dispatch) => {
            let empty = VARIANT::default();
            let arguments = VARIANT::from(arguments);
            // From this instruction onward, an HRESULT failure cannot prove
            // the helper was not started: Explorer/Alice can perform the side
            // effect before reporting an error.
            let result =
                unsafe { shell_dispatch.ShellExecute(&target, &arguments, &empty, &empty, &empty) }
                    .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable);
            UserHelperStaOutcome::Invoked(result)
        }
    };
    unsafe { CoUninitialize() };
    outcome
}

fn launch_from_explorer_sta_bstr(
    target: BSTR,
    arguments: Option<&str>,
) -> Result<(), ProcessLaunchError> {
    let initialized =
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    if initialized.is_err() {
        return Err(ProcessLaunchError::InteractiveUserUnavailable);
    }

    let result = (|| {
        let shell_dispatch = explorer_shell_dispatch()?;
        let empty = VARIANT::default();
        let arguments = arguments.map(VARIANT::from).unwrap_or_default();
        let show = VARIANT::from(SW_SHOWNORMAL.0);
        unsafe { shell_dispatch.ShellExecute(&target, &arguments, &empty, &empty, &show) }
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)
    })();

    unsafe { CoUninitialize() };
    result
}

/// Obtains Explorer's `IShellDispatch2` through the desktop shell view.
///
/// Enumerating only open folder windows makes link opening fail whenever the
/// user has no File Explorer window open. The desktop view is the stable
/// Explorer-owned object in Microsoft's ExecInExplorer sample and preserves
/// the unelevated launch boundary.
fn explorer_shell_dispatch() -> Result<IShellDispatch2, ProcessLaunchError> {
    let shell_windows: IShellWindows =
        unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER) }
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let empty = VARIANT::default();
    let mut desktop_window = 0;
    let desktop_dispatch = unsafe {
        shell_windows.FindWindowSW(
            &empty,
            &empty,
            SWC_DESKTOP,
            &mut desktop_window,
            SWFO_NEEDDISPATCH,
        )
    }
    .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let shell_browser: IShellBrowser =
        unsafe { IUnknown_QueryService(&desktop_dispatch, &SID_STopLevelBrowser) }
            .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let shell_view = unsafe { shell_browser.QueryActiveShellView() }
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let background_dispatch: IDispatch = unsafe { shell_view.GetItemObject(SVGIO_BACKGROUND) }
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let folder_view: IShellFolderViewDual = background_dispatch
        .cast()
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    let application = unsafe { folder_view.Application() }
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)?;
    application
        .cast()
        .map_err(|_| ProcessLaunchError::InteractiveUserUnavailable)
}
