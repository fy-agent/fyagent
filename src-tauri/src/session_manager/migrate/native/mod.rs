//! Native write, readback and reconciliation interface for one provider.
//!
//! Every provider-specific write path implements [`NativeSessionWriter`]. The
//! orchestration in [`super::restore`] owns the receipt row and the ordering
//! rules; a writer only performs the native call and reports what it can
//! actually prove.
//!
//! Two rules shape this interface:
//!
//! 1. The native write is a subprocess or an external protocol call, so it is
//!    never inside the local SQLite transaction. There is no cross-process
//!    exactly-once. A writer that cannot prove the outcome must say so instead
//!    of guessing.
//! 2. When the target can hand back an ID before content exists, the ID must
//!    reach the receipt first. [`NativeWriteContext::record_native_id`] exists
//!    for exactly that ordering and must be awaited before the content call.

// `native/codex.rs` is coordinator-owned; see
// `.trellis/tasks/09-22-session-cross-device-recovery/implementation/codex-integration-contract.md`.
pub mod codex;
pub mod gemini;
pub mod hermes;
pub mod opencode;

use std::path::PathBuf;

use super::model::{MigratableSession, MigrationError, MigrationResult};

/// Everything a writer is allowed to see. There is no command string and no
/// path taken from the package: the target directory was chosen by the user
/// through the platform dialog and canonicalized by the caller.
#[derive(Debug, Clone)]
pub struct NativeRestoreInput {
    /// Final-only transcript, verbatim.
    pub session: MigratableSession,
    /// Existing absolute directory on this machine.
    pub target_workspace: PathBuf,
    pub target_store_id: String,
    /// Pre-allocated reconciliation anchor. A writer that can persist it in a
    /// field the target keeps should do so and report it through
    /// [`NativeWriteOutcome::Written`]'s companion [`NonceCarrier`].
    ///
    /// Unread by every writer in this build, because all four preallocate the
    /// native ID instead (see [`ReconciliationModel::IdBeforeContent`]). It
    /// stays on the input, and on the receipt row, so a target that mints its
    /// own ID mid-write can still be made reconcilable without changing the
    /// receipt schema.
    #[allow(dead_code)]
    pub target_native_nonce: String,
    /// Reuse an ID already persisted by a proven-side-effect-free attempt.
    pub target_native_id: Option<String>,
}

/// How the nonce was placed, if at all. Reconciliation needs to know whether
/// looking for it is meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceCarrier {
    /// The target returned its own ID, so the nonce was not needed.
    NotUsed,
    /// Stored in the session slug/identifier space.
    ///
    /// Unused in this build. The Hermes foreign importer did plant the nonce,
    /// but in `origin_json`, where the installed CLI's own marker search does
    /// not look; that writer now preallocates an ID instead. The variant
    /// remains so a writer that can only plant a marker has a truthful way to
    /// say where it put it.
    #[allow(dead_code)]
    SessionSlug,
}

#[derive(Debug, Clone)]
pub enum NativeWriteOutcome {
    /// The native call returned success. `target_native_id` stays `None` when
    /// the target genuinely returned no ID; it is never invented.
    Written {
        target_native_id: Option<String>,
        nonce_carrier: NonceCarrier,
    },
    /// Positive evidence that nothing was written: the executable is missing,
    /// startup failed, argument parsing rejected the call before any IO, or
    /// the protocol dropped before the first write method. Only this outcome
    /// may become `failed` and allow an idempotent retry.
    ProvenNoSideEffect { error: MigrationError },
    /// Timeout, kill, unparseable stderr, unknown exit code, or any other
    /// outcome that cannot rule out a partial write. Becomes
    /// `needsReconciliation`; a second native write is forbidden.
    Unresolved { error: MigrationError },
}

/// What the target's own read channel says. Scanning provider files from disk
/// is explicitly not a substitute: it proves a file exists, not that the user
/// can see the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadbackVerdict {
    /// The target listed the session through its own API and the final-only
    /// projection matches.
    Visible { observed_digest: String },
    /// The target answered and the projection differs.
    Mismatch { observed: String },
    /// The target answered that it has no such session.
    NotVisible,
    /// The target has no working read channel in this version, so absence is
    /// not evidence either way. This is a release-blocking gap, not a pending
    /// state that will resolve by retrying.
    Blocked { reason: String },
    /// Several candidates. No automatic action is allowed.
    ///
    /// No writer in this build can produce it — both read channels answer
    /// about one ID — but reconciliation handles it, so a writer that queries
    /// by anything looser has somewhere truthful to report.
    #[allow(dead_code)]
    Ambiguous { candidates: Vec<String> },
}

/// How a writer's crash window can be resolved afterwards.
///
/// This is the fact reconciliation needs and cannot derive: "no ID on the row"
/// means different things depending on when the target mints its ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationModel {
    /// The ID is minted locally or returned before any content exists, and is
    /// persisted through [`NativeWriteContext::record_native_id`] first. A row
    /// with no ID therefore proves no content was written.
    IdBeforeContent,
    /// The target mints the ID itself during the write, so a crashed attempt
    /// can only be found by the nonce the writer planted.
    ///
    /// Unused in this build: every registered writer preallocates its ID.
    #[allow(dead_code)]
    NonceLookup,
    /// Neither holds. A crashed attempt cannot be resolved automatically and
    /// stays unresolved instead of being guessed at.
    ///
    /// Unused in this build: a writer is only released once its crash window
    /// is resolvable. It exists so that constraint is a choice a writer has to
    /// declare, rather than something it can omit.
    #[allow(dead_code)]
    Unresolvable,
}

/// Receipt ordering hook handed to a writer.
pub trait NativeWriteContext {
    /// Persist the native ID on the receipt row before producing any content
    /// side effect. A crash between the ID being minted and this call landing
    /// leaves an unlocatable thread, which is why it is the writer's
    /// responsibility to call it at the earliest possible moment.
    fn record_native_id(&self, native_id: &str) -> MigrationResult<()>;
}

pub trait NativeSessionWriter: Send + Sync {
    fn provider_id(&self) -> &'static str;

    /// Identifier of the verified write path, e.g. `opencode.import.pure-v1`.
    /// Surfaces in the release capability matrix.
    fn write_strategy(&self) -> &'static str;

    /// Provider versions the write path has been exercised against. The local
    /// probe gates restoring on this, independently of the export gate.
    fn verified_write_versions(&self) -> &'static [&'static str];

    /// Identity of the target store on this machine. Returning
    /// `targetStoreUnidentified` blocks every native write for the provider:
    /// a path alone cannot prove the store instance was not replaced.
    fn resolve_target_store_id(&self) -> MigrationResult<String>;

    fn restore(
        &self,
        input: &NativeRestoreInput,
        context: &dyn NativeWriteContext,
    ) -> NativeWriteOutcome;

    /// Ask the target's own read channel whether the session is visible.
    ///
    /// Only `expected.content_digest` and `expected.origin` are guaranteed to
    /// be populated. Verification can be re-run long after the write, at which
    /// point the only surviving local record is the receipt, and a receipt
    /// deliberately stores no message bodies — see
    /// [`super::restore::expectation_from_receipt`]. A writer therefore
    /// compares digests of its own projection, never message text.
    fn verify_readback(
        &self,
        native_id: &str,
        expected: &MigratableSession,
    ) -> MigrationResult<ReadbackVerdict>;

    /// Reconciliation path for writes that returned no ID. Returns every
    /// candidate; the caller refuses to act on more than one.
    fn locate_by_nonce(&self, nonce: &str) -> MigrationResult<Vec<String>>;

    /// What a missing native ID proves for this writer.
    ///
    /// The default is [`ReconciliationModel::IdBeforeContent`], which is the
    /// only model that makes the crash window resolvable without a nonce, and
    /// it is a real obligation: a writer keeping the default must persist the
    /// ID through [`NativeWriteContext::record_native_id`] before producing
    /// any content side effect.
    fn reconciliation_model(&self) -> ReconciliationModel {
        ReconciliationModel::IdBeforeContent
    }

    /// Argument the target CLI needs to reopen a restored session. Built from
    /// the ID the target returned and validated against the existing resume
    /// allowlist, never from a string carried inside a package.
    fn resume_argument(&self, native_id: &str) -> Option<String> {
        crate::session_manager::terminal::session_resume_argument(native_id)
    }
}

/// Providers with a native writer in this build.
pub fn writer_for(provider_id: &str) -> Option<&'static dyn NativeSessionWriter> {
    match provider_id {
        "codex" => Some(&codex::CodexWriter),
        "opencode" => Some(&opencode::OpenCodeWriter),
        "hermes" => Some(&hermes::HermesWriter),
        "gemini" => Some(&gemini::GeminiWriter),
        _ => None,
    }
}

/// Outcome of a bounded subprocess run, kept separate from the exit code so
/// callers cannot accidentally treat "we gave up waiting" as "it failed".
#[derive(Debug)]
pub(crate) enum RunOutcome {
    /// The process exited on its own.
    Exited {
        status: Option<i32>,
        stdout: String,
        stderr: String,
    },
    /// The executable could not be started. Nothing ran, so nothing was
    /// written.
    ///
    /// Only ever returned for a failed spawn. Anything that goes wrong after
    /// the child exists is [`RunOutcome::TimedOut`] or an exit status,
    /// because at that point a partial write can no longer be ruled out.
    NotStarted {
        /// Diagnostic only. Writers deliberately never surface it — a CLI's
        /// own output can carry credentials — so it survives in logs and in
        /// test failure messages through `Debug`.
        #[allow(dead_code)]
        reason: String,
    },
    /// Output was truncated, unreadable, invalid UTF-8, or did not close.
    /// The process ran, so callers must preserve the uncertain write outcome.
    OutputIncomplete,
    /// The deadline passed, or waiting failed, and the process group was
    /// killed. A partial write cannot be ruled out.
    TimedOut { after: std::time::Duration },
}

/// Version probes and status replies need only a small bounded capture.
const MAX_CAPTURED_OUTPUT: usize = 1024 * 1024;
/// A legal 32 MiB transcript can expand sixfold when JSON escapes its text.
pub(crate) const MAX_NATIVE_READBACK_OUTPUT: usize = 256 * 1024 * 1024;

pub(crate) fn run_with_timeout(
    command: std::process::Command,
    timeout: std::time::Duration,
) -> RunOutcome {
    run_with_output_limit(command, timeout, MAX_CAPTURED_OUTPUT)
}

/// Drain both pipes while bounding memory and wall time. Only a complete,
/// valid UTF-8 capture may produce `Exited`; partial output is never proof.
pub(crate) fn run_with_output_limit(
    mut command: std::process::Command,
    timeout: std::time::Duration,
    stdout_limit: usize,
) -> RunOutcome {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::mpsc;

    if let Some(reason) = user_cli_execution_blocked() {
        return RunOutcome::NotStarted {
            reason: reason.into(),
        };
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return RunOutcome::NotStarted {
                reason: error.to_string(),
            }
        }
    };
    let child_id = child.id();

    fn drain(
        pipe: Option<impl Read + Send + 'static>,
        cap: usize,
    ) -> mpsc::Receiver<Result<String, ()>> {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut kept = Vec::new();
            let mut complete = true;
            if let Some(mut pipe) = pipe {
                let mut chunk = [0u8; 8192];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            let room = cap.saturating_sub(kept.len());
                            if read > room {
                                complete = false;
                            }
                            kept.extend_from_slice(&chunk[..read.min(room)]);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => {
                            complete = false;
                            break;
                        }
                    }
                }
            } else {
                complete = false;
            }
            let result = if complete {
                String::from_utf8(kept).map_err(|_| ())
            } else {
                Err(())
            };
            let _ = sender.send(result);
        });
        receiver
    }
    let stdout_rx = drain(child.stdout.take(), stdout_limit);
    let stderr_rx = drain(child.stderr.take(), MAX_CAPTURED_OUTPUT);
    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25))
            }
            _ => break None,
        }
    };
    if status.is_none() {
        kill_process_group(&mut child, child_id);
    }
    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let stdout =
        stdout_rx.recv_timeout(drain_deadline.saturating_duration_since(std::time::Instant::now()));
    let stderr =
        stderr_rx.recv_timeout(drain_deadline.saturating_duration_since(std::time::Instant::now()));
    match (status, stdout, stderr) {
        (Some(status), Ok(Ok(stdout)), Ok(Ok(stderr))) => RunOutcome::Exited {
            status: status.code(),
            stdout,
            stderr,
        },
        (None, _, _) => RunOutcome::TimedOut { after: timeout },
        _ => {
            // A wrapper can exit while its child still owns either pipe.
            kill_process_group(&mut child, child_id);
            RunOutcome::OutputIncomplete
        }
    }
}

fn kill_process_group(child: &mut std::process::Child, child_id: u32) {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: this subprocess was started in its own process group.
        unsafe {
            libc::kill(-(child_id as libc::pid_t), libc::SIGKILL);
        }
    }
    #[cfg(target_os = "windows")]
    let _ = child_id;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = child_id;
    let _ = child.kill();
    let _ = child.wait();
}

/// Locate an installed CLI without guessing at an absolute path.
pub(crate) fn resolve_cli(provider_id: &str, binary: &str, extra: &[PathBuf]) -> Option<PathBuf> {
    for candidate in extra {
        if candidate.is_absolute() && candidate.is_file() {
            return Some(candidate.clone());
        }
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_absolute() && candidate.is_file() {
            return Some(candidate);
        }
    }
    log::debug!("{provider_id}: no `{binary}` on PATH");
    None
}

/// Where a provider's own installer puts its launcher, beyond `PATH`.
///
/// A desktop app inherits the launcher's environment, not a login shell's, so
/// `PATH` alone routinely misses a CLI the user can run in a terminal. These
/// are the locations the product already treats as that provider's home.
fn installer_locations(provider_id: &str, binary: &str) -> Vec<PathBuf> {
    let home = crate::config::get_home_dir();
    if home.as_os_str().is_empty() {
        return Vec::new();
    }
    let mut directories = vec![home.join(".local/bin"), home.join(".npm-global/bin")];
    match provider_id {
        "codex" => directories.push(home.join(".codex/bin")),
        "opencode" => directories.push(home.join(".opencode/bin")),
        "hermes" => directories.push(home.join(".hermes/bin")),
        "grokbuild" => directories.push(home.join(".grok/bin")),
        _ => {}
    }
    directories
        .iter()
        .flat_map(|dir| executable_candidates(dir, binary))
        .collect()
}

/// The one executable this feature probes and writes through for a provider.
///
/// Version detection and the native write have to agree on a single file.
/// Resolving them separately is how a probe reports a supported version while
/// the write lands in a different installation, which then produces a receipt
/// describing a store that was never touched.
///
/// `PATH` wins over the installer locations. A user with two installations
/// gets the one their shell would run, and an unsupported version there is
/// reported as unsupported — quietly reaching past it to an older copy that
/// happens to be verified would restore into a store the user does not use.
///
/// When the product home is overridden, only launchers inside it are eligible:
/// a fixture that stands up a fake CLI must not end up running the developer's
/// own installation against a temporary store.
///
/// Returns `None` when the provider has no CLI in this product, or nothing is
/// installed. Callers turn that into `providerNotInstalled`; it never means
/// "blocked", which [`user_cli_execution_blocked`] answers separately.
pub(crate) fn provider_cli_path(provider_id: &str) -> Option<PathBuf> {
    let binary = provider_binary(provider_id)?;
    let from_path: Vec<PathBuf> = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .flat_map(|dir| executable_candidates(&dir, binary))
        .collect();
    let installed = installer_locations(provider_id, binary);
    let ordered: Vec<PathBuf> = match overridden_home() {
        Some(home) => installed
            .into_iter()
            .chain(from_path)
            .filter(|candidate| candidate.starts_with(&home))
            .collect(),
        None => from_path.into_iter().chain(installed).collect(),
    };
    for candidate in ordered {
        // A relative `PATH` entry resolves against the working directory,
        // which differs between the probe and a write with a target cwd.
        if candidate.is_absolute() && candidate.is_file() {
            return Some(candidate);
        }
    }
    log::debug!("{provider_id}: no `{binary}` executable found");
    None
}

/// The home this build was told to treat as the user's, if it was told one.
///
/// Mirrors the override in [`crate::config::get_home_dir`], including its
/// platform gate, so executable lookup stays inside whatever home the rest of
/// the product is reading and writing.
fn overridden_home() -> Option<PathBuf> {
    #[cfg(any(target_os = "macos", test, feature = "test-hooks"))]
    {
        let home = std::env::var("FYAGENT_TEST_HOME").ok()?;
        let home = home.trim();
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    None
}

/// Filenames one launcher can have in a directory.
fn executable_candidates(dir: &std::path::Path, binary: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            dir.join(format!("{binary}.cmd")),
            dir.join(format!("{binary}.exe")),
            dir.join(binary),
        ]
    }
    #[cfg(target_os = "macos")]
    {
        vec![dir.join(binary)]
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (dir, binary);
        Vec::new()
    }
}

/// Launcher name per provider. `grokbuild` ships as `grok`, which is why this
/// mapping exists rather than using the provider ID directly.
pub(crate) fn provider_binary(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "codex" => Some("codex"),
        "claude" => Some("claude"),
        "opencode" => Some("opencode"),
        "openclaw" => Some("openclaw"),
        "gemini" => Some("gemini"),
        "grokbuild" => Some("grok"),
        "hermes" => Some("hermes"),
        _ => None,
    }
}

/// Why user-owned CLIs may not be executed at all in this build.
///
/// The formal Windows release runs elevated, and `services::tooling` already
/// refuses to start any user-owned CLI there: a process this app starts would
/// inherit administrator rights the user never granted that tool. Migration
/// executes the same CLIs — including `--version` — so it observes the same
/// boundary, and it observes it before spawning rather than reporting a
/// failure afterwards.
///
/// This is an integration gap waiting on the ordinary-user helper, not a
/// decision that Windows cannot migrate sessions.
pub(crate) fn user_cli_execution_blocked() -> Option<&'static str> {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Some("Session migration requires a supported desktop platform")
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        #[cfg(target_os = "windows")]
        if crate::windows_runtime::formal_windows_build() {
            return Some(
                "user-owned CLIs cannot be executed from the elevated Windows release; \
             the ordinary-user execution helper is required",
            );
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unregistered_providers_have_no_writer() {
        // These three have no fixture-proven write path in this build. The
        // absence must stay visible instead of being answered with a writer
        // that returns "unsupported".
        for provider in ["claude", "openclaw", "grokbuild"] {
            assert!(writer_for(provider).is_none(), "{provider}");
        }
    }

    #[test]
    fn registered_writers_report_their_own_provider_id() {
        for provider in ["codex", "opencode", "hermes", "gemini"] {
            let writer = writer_for(provider).expect("writer");
            assert_eq!(writer.provider_id(), provider);
            assert!(!writer.write_strategy().is_empty());
            assert!(
                !writer.verified_write_versions().is_empty(),
                "{provider}: a registered writer without verified versions would open the \
                 restore gate on unproven builds"
            );
        }
    }

    #[test]
    fn every_writer_resolves_through_the_shared_executable_lookup() {
        // Probing one executable and writing through another is how a
        // receipt ends up describing a store nobody touched. Registration in
        // this table is what keeps both sides on one file.
        for provider in ["codex", "opencode", "hermes", "gemini"] {
            assert!(
                provider_binary(provider).is_some(),
                "{provider}: registered writer with no launcher name"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_hung_child_is_killed_and_reported_as_timed_out() {
        if user_cli_execution_blocked().is_some() {
            return;
        }
        // A shell that outlives its own child is the shape that used to hang
        // the caller: killing the wrapper alone leaves the sleeper holding
        // the pipe open. Absolute paths only — the test runner's `PATH` is
        // not the shell's.
        let mut command = std::process::Command::new("/bin/sh");
        command.args(["-c", "/bin/sleep 30 & wait"]);
        let started = std::time::Instant::now();
        let outcome = run_with_timeout(command, std::time::Duration::from_millis(300));
        assert!(
            matches!(outcome, RunOutcome::TimedOut { .. }),
            "{outcome:?}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "the deadline did not bound the call"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn output_limits_never_certify_a_truncated_success() {
        let make = || {
            let mut command = std::process::Command::new("/bin/sh");
            command.args(["-c", "s=fyagent; i=0; while [ $i -lt 18 ]; do s=\"$s$s\"; i=$((i+1)); done; printf '%s' \"$s\""]);
            command
        };
        assert!(matches!(
            run_with_timeout(make(), std::time::Duration::from_secs(20)),
            RunOutcome::OutputIncomplete
        ));
        match run_with_output_limit(
            make(),
            std::time::Duration::from_secs(20),
            MAX_NATIVE_READBACK_OUTPUT,
        ) {
            RunOutcome::Exited { status, stdout, .. } => {
                assert_eq!(status, Some(0));
                assert_eq!(stdout.len(), 7 * (1 << 18));
            }
            other => panic!("{other:?}"),
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn invalid_utf8_or_an_inherited_open_pipe_cannot_confirm_success() {
        for script in ["printf '\\377'", "/bin/sleep 30 & exit 0"] {
            let mut command = std::process::Command::new("/bin/sh");
            command.args(["-c", script]);
            let start = std::time::Instant::now();
            assert!(matches!(
                run_with_timeout(command, std::time::Duration::from_secs(5)),
                RunOutcome::OutputIncomplete
            ));
            assert!(start.elapsed() < std::time::Duration::from_secs(5));
        }
    }

    #[test]
    fn a_missing_executable_is_the_only_way_to_get_not_started() {
        match run_with_timeout(
            std::process::Command::new("/definitely/not/here/fyagent-native"),
            std::time::Duration::from_secs(5),
        ) {
            RunOutcome::NotStarted { .. } => {}
            other => panic!("{other:?}"),
        }
    }
}
