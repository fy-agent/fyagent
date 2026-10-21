//! OMP observation through the authenticated ordinary-user helper only.
pub(crate) use fyagent_user_helper::OmpProbeResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OmpProbeUnavailable {
    InteractiveUserUnavailable,
    HelperUnavailable,
}

impl std::fmt::Display for OmpProbeUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InteractiveUserUnavailable => "OMP probe requires an available interactive user",
            Self::HelperUnavailable => "OMP probe could not be verified by the current-user helper",
        })
    }
}

impl std::error::Error for OmpProbeUnavailable {}

pub(crate) async fn probe_omp_cli_as_user() -> Result<OmpProbeResult, OmpProbeUnavailable> {
    let context = crate::windows_runtime::interactive_user_context()
        .ok_or(OmpProbeUnavailable::InteractiveUserUnavailable)?;
    tokio::task::spawn_blocking(move || {
        crate::codex_desktop::platform::windows::run_omp_cli_probe(context)
            .map_err(|_| OmpProbeUnavailable::HelperUnavailable)
    })
    .await
    .map_err(|_| OmpProbeUnavailable::HelperUnavailable)?
}
