// The production app graph is deliberately absent from this target. The
// path-included module keeps the domain and runtime's pure transport tests
// independently compilable while normal library and Windows checks validate
// the complete production graph. This test-only copy intentionally exercises
// only a subset of that graph.
#[cfg(target_os = "windows")]
mod platform {
    pub(crate) mod process_launch {
        use std::path::PathBuf;

        use fyagent_user_helper::{CanonicalJobId, PipeNonce, UserHelperAction};

        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum UserHelperLaunchOutcome {
            Confirmed,
            MayHaveLaunched,
            NotInvoked(&'static str),
        }

        /// The path-included Windows deployment module names the production
        /// crate-root launcher. This isolated domain-test crate deliberately
        /// omits that graph, so any accidental launch attempt must fail closed;
        /// library and native Windows jobs validate the real implementation.
        pub(crate) fn launch_trusted_windows_app_aumid_as_user(_aumid: &str) -> Result<(), String> {
            Err("isolated Codex desktop domain tests cannot launch Windows apps".to_owned())
        }

        pub(crate) fn fixed_user_helper_path() -> Result<PathBuf, &'static str> {
            Err("isolated Codex desktop domain tests cannot resolve the user helper")
        }

        pub(crate) fn launch_fyagent_user_helper_as_user(
            _action: UserHelperAction,
            _job_id: &CanonicalJobId,
            _pipe_nonce: &PipeNonce,
        ) -> UserHelperLaunchOutcome {
            UserHelperLaunchOutcome::NotInvoked(
                "isolated Codex desktop domain tests cannot launch the user helper",
            )
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code, unused_imports, clippy::enum_variant_names)]
#[path = "../src/windows_runtime/mod.rs"]
mod windows_runtime;

#[allow(dead_code)]
#[path = "../src/codex_desktop/mod.rs"]
mod codex_desktop;
