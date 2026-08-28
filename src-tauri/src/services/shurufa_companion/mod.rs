mod device_settings;
mod input;
mod network;
mod profile;
mod runtime;
mod serial;
mod target;
mod windows_foreground_restore;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use device_settings::{DeviceSettings, DeviceSettingsStore};
use network::NetworkStatus;
use profile::{ProfileDraft, ProfileStore, ProfileTarget};
use runtime::{
    RuntimeController, RuntimeMode, RuntimeStatus, WindowsInputDispatcher, WindowsModifierState,
};
use serial::{AsrAdmission, AsrDone, AsrOutcome, SerialPortSource};
use target::{ForegroundProbe, WindowsForegroundProbe};
use windows_foreground_restore::WindowsForegroundTargetRestorer;

pub type CompanionProfile = ProfileDraft;
pub type CompanionTarget = ProfileTarget;
pub type CompanionDeviceSettings = DeviceSettings;
pub type CompanionNetwork = NetworkStatus;
pub type CompanionRuntime = RuntimeStatus;
pub type CompanionAsrAdmission = AsrAdmission;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSnapshot {
    pub ports: Vec<String>,
    pub profile: Option<CompanionProfile>,
    pub device: CompanionDeviceSettings,
    pub runtime: CompanionRuntime,
    pub last_asr_seq: Option<u32>,
    pub last_asr_admission: CompanionAsrAdmission,
    pub last_asr_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompanionApplyDeviceConfig {
    pub port: String,
    pub baud: u32,
    pub settings: CompanionDeviceSettings,
}

#[derive(Default)]
struct CompanionInner {
    runtime: RuntimeController,
    last_asr_seq: Option<u32>,
    last_asr_admission: AsrAdmission,
    last_asr_error: Option<String>,
}

pub struct CompanionState {
    inner: Arc<Mutex<CompanionInner>>,
    stop: Arc<AtomicBool>,
}

impl CompanionState {
    pub fn new(app: AppHandle) -> Self {
        let inner = Arc::new(Mutex::new(CompanionInner::default()));
        let stop = Arc::new(AtomicBool::new(false));
        spawn_pump(inner.clone(), stop.clone(), app);
        Self { inner, stop }
    }

    pub fn list_ports() -> Result<Vec<String>, String> {
        SerialPortSource::available_ports().map_err(|_| "serial ports are unavailable".to_owned())
    }

    pub fn snapshot(&self) -> Result<CompanionSnapshot, String> {
        let ports = SerialPortSource::available_ports().unwrap_or_default();
        let profile = load_profile_from_store(&profile_store())?;
        let device = load_device_from_store(&device_store())?;
        let inner = self.lock_inner()?;
        Ok(build_snapshot(&inner, ports, profile, device))
    }

    pub fn capture_target(&self) -> Result<CompanionTarget, String> {
        self.lock_inner()?
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        std::thread::sleep(Duration::from_secs(3));
        let runtime_guard = self.lock_inner()?;
        runtime_guard
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let identity = WindowsForegroundProbe
            .foreground_identity()
            .map_err(|_| "foreground target is unavailable".to_owned())?
            .ok_or_else(|| "no foreground target is available".to_owned())?;
        drop(runtime_guard);
        Ok(CompanionTarget {
            process_name: identity.process_name,
            process_path: identity.process_path,
        })
    }

    pub fn save_profile(&self, draft: CompanionProfile) -> Result<CompanionProfile, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let saved = save_profile_to_store(&profile_store(), draft)?;
        inner
            .runtime
            .set_profile(saved.clone())
            .map_err(|error| error.to_string())?;
        Ok(saved)
    }

    pub fn start_dry_run(&self) -> Result<CompanionRuntime, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let profile = load_profile_into_runtime(&profile_store(), &mut inner.runtime)?;
        start_shortcut(
            &mut inner.runtime,
            &profile.serial.port,
            profile.serial.baud,
            RuntimeMode::DryRun,
        )
    }

    pub fn enable_live(&self) -> Result<CompanionRuntime, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let profile = load_profile_into_runtime(&profile_store(), &mut inner.runtime)?;
        start_shortcut(
            &mut inner.runtime,
            &profile.serial.port,
            profile.serial.baud,
            RuntimeMode::Live,
        )
    }

    pub fn stop(&self) -> Result<CompanionRuntime, String> {
        Ok(self.lock_inner()?.runtime.stop())
    }

    pub fn save_device_settings(
        draft: CompanionDeviceSettings,
    ) -> Result<CompanionDeviceSettings, String> {
        device_store()
            .save(draft)
            .map_err(|error| error.to_string())
    }

    pub fn apply_device_config(
        &self,
        request: CompanionApplyDeviceConfig,
    ) -> Result<CompanionNetwork, String> {
        if request.port.trim().is_empty() || request.baud == 0 {
            return Err("a serial port is required".to_owned());
        }
        request
            .settings
            .validate()
            .map_err(|error| error.to_string())?;
        let saved = device_store()
            .save(request.settings)
            .map_err(|error| error.to_string())?;
        let mut inner = self.lock_inner()?;
        ensure_device_source(&mut inner.runtime, &request.port, request.baud)?;
        inner
            .runtime
            .apply_config(&saved)
            .map_err(|error| error.to_string())
    }

    fn lock_inner(&self) -> Result<MutexGuard<'_, CompanionInner>, String> {
        self.inner
            .lock()
            .map_err(|_| "runtime state is unavailable".to_owned())
    }
}

impl Drop for CompanionState {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn companion_dir() -> std::path::PathBuf {
    crate::config::get_app_config_dir().join("shurufacli/companion")
}

fn profile_store() -> ProfileStore {
    ProfileStore::new(companion_dir().join("profile.json"))
}

fn device_store() -> DeviceSettingsStore {
    DeviceSettingsStore::new(companion_dir().join("device.json"))
}

fn load_profile_from_store(store: &ProfileStore) -> Result<Option<ProfileDraft>, String> {
    store
        .load()
        .map_err(|_| "saved profile is invalid".to_owned())
}

fn load_device_from_store(store: &DeviceSettingsStore) -> Result<DeviceSettings, String> {
    Ok(store
        .load()
        .map_err(|_| "saved device settings are invalid".to_owned())?
        .unwrap_or_default())
}

fn save_profile_to_store(
    store: &ProfileStore,
    draft: ProfileDraft,
) -> Result<ProfileDraft, String> {
    let expected = draft.revision.clone();
    store
        .save(draft, expected.as_deref())
        .map_err(|error| error.to_string())
}

fn load_profile_into_runtime(
    store: &ProfileStore,
    runtime: &mut RuntimeController,
) -> Result<ProfileDraft, String> {
    let profile = store
        .load()
        .map_err(|_| "profile is unavailable".to_owned())?
        .ok_or_else(|| "a saved profile is required".to_owned())?;
    runtime
        .set_profile(profile.clone())
        .map_err(|error| error.to_string())?;
    Ok(profile)
}

fn ensure_device_source(
    runtime: &mut RuntimeController,
    port: &str,
    baud: u32,
) -> Result<(), String> {
    if runtime.source_matches(port, baud) {
        return Ok(());
    }
    runtime
        .ensure_stopped()
        .map_err(|error| error.to_string())?;
    runtime.close_source();
    let source = SerialPortSource::open(port, baud)
        .map_err(|_| "selected serial port could not be opened".to_owned())?;
    runtime.attach_source(port.to_owned(), baud, Box::new(source));
    Ok(())
}

fn start_shortcut(
    runtime: &mut RuntimeController,
    port: &str,
    baud: u32,
    mode: RuntimeMode,
) -> Result<RuntimeStatus, String> {
    if !runtime.source_matches(port, baud) {
        runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        runtime.close_source();
        let source = SerialPortSource::open(port, baud)
            .map_err(|_| "selected serial port could not be opened".to_owned())?;
        runtime.attach_source(port.to_owned(), baud, Box::new(source));
    }
    runtime
        .start_existing(mode)
        .map_err(|error| error.to_string())
}

fn build_snapshot(
    inner: &CompanionInner,
    ports: Vec<String>,
    profile: Option<ProfileDraft>,
    device: DeviceSettings,
) -> CompanionSnapshot {
    CompanionSnapshot {
        ports,
        profile,
        device,
        runtime: inner.runtime.status(),
        last_asr_seq: inner.last_asr_seq,
        last_asr_admission: inner.last_asr_admission,
        last_asr_error: inner.last_asr_error.clone(),
    }
}

fn apply_ingest_failure(inner: &mut CompanionInner, message: String) {
    if message == "正在生成中，请稍后再试" {
        inner.last_asr_admission = AsrAdmission::Busy;
    }
    inner.last_asr_error = Some(message);
}

fn apply_asr_outcome(inner: &mut CompanionInner, asr: &AsrOutcome) {
    if asr.admission == AsrAdmission::None {
        return;
    }
    inner.last_asr_admission = asr.admission;
    inner.last_asr_seq = asr.seq.or(inner.last_asr_seq);
    if asr.admission == AsrAdmission::Admitted {
        inner.last_asr_error = None;
    }
}

fn spawn_ingest(app: AppHandle, inner: Arc<Mutex<CompanionInner>>, done: AsrDone) {
    tauri::async_runtime::spawn(async move {
        if let Err(message) = crate::commands::shurufa::run_ingest_text(app, done.text, true).await
        {
            log::warn!("shurufa companion asr ingest failed: {message}");
            if let Ok(mut guard) = inner.lock() {
                apply_ingest_failure(&mut guard, message);
            }
        }
    });
}

fn spawn_pump(inner: Arc<Mutex<CompanionInner>>, stop: Arc<AtomicBool>, app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("shurufa-companion-pump".into())
        .spawn(move || {
            let probe = WindowsForegroundProbe;
            let restorer = WindowsForegroundTargetRestorer;
            let modifiers = WindowsModifierState;
            while !stop.load(Ordering::Relaxed) {
                let mut dispatcher = WindowsInputDispatcher;
                let asr_done = {
                    let mut guard = match inner.lock() {
                        Ok(guard) => guard,
                        Err(_) => break,
                    };
                    if !guard.runtime.has_source() {
                        drop(guard);
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    match guard
                        .runtime
                        .pump_once(&probe, &restorer, &modifiers, &mut dispatcher)
                    {
                        Ok(outcome) => {
                            apply_asr_outcome(&mut guard, &outcome.asr);
                            outcome.asr.done
                        }
                        Err(_) => None,
                    }
                };
                if let Some(done) = asr_done {
                    spawn_ingest(app.clone(), inner.clone(), done);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::shurufa_companion::input::InputId;
    use crate::services::shurufa_companion::profile::{
        MappingDraft, ProfileSerial, PROFILE_VERSION,
    };
    use crate::services::shurufa_companion::serial::EventSource;
    use crate::services::shurufa_companion::serial::SerialError;
    use crate::services::shurufa_companion::serial::SerialEvent;
    use std::sync::atomic::AtomicU32;
    use tempfile::tempdir;

    struct CountingSource {
        polls: Arc<AtomicU32>,
    }

    impl EventSource for CountingSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            self.polls.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }
    }

    fn draft() -> ProfileDraft {
        ProfileDraft {
            version: PROFILE_VERSION,
            revision: None,
            serial: ProfileSerial {
                port: "fixture".into(),
                baud: 115200,
            },
            target: Some(ProfileTarget {
                process_name: "Fixture.exe".into(),
                process_path: r"C:\Fixture.exe".into(),
            }),
            mappings: vec![
                MappingDraft {
                    input: InputId::EncoderCw,
                    display_name: "Previous".into(),
                    keys: vec!["CTRL".into(), "TAB".into()],
                },
                MappingDraft {
                    input: InputId::EncoderCcw,
                    display_name: "Next".into(),
                    keys: vec!["CTRL".into(), "SHIFT".into(), "TAB".into()],
                },
                MappingDraft {
                    input: InputId::EncoderPress,
                    display_name: "Confirm".into(),
                    keys: vec!["ENTER".into()],
                },
            ],
        }
    }

    #[test]
    fn companion_files_live_under_shurufacli_companion() {
        let profile = companion_dir().join("profile.json");
        let device = companion_dir().join("device.json");
        assert_eq!(
            profile.file_name().and_then(|name| name.to_str()),
            Some("profile.json")
        );
        assert_eq!(
            device.file_name().and_then(|name| name.to_str()),
            Some("device.json")
        );
        assert_eq!(
            profile.parent().and_then(|path| path.file_name()),
            Some(std::ffi::OsStr::new("companion"))
        );
    }

    #[test]
    fn snapshot_does_not_consume_serial() {
        let polls = Arc::new(AtomicU32::new(0));
        let mut inner = CompanionInner::default();
        inner.runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(CountingSource {
                polls: polls.clone(),
            }),
        );
        inner.last_asr_seq = Some(4);
        inner.last_asr_admission = AsrAdmission::Admitted;
        let snapshot = build_snapshot(&inner, vec!["COM3".into()], None, DeviceSettings::default());
        assert_eq!(snapshot.ports, vec!["COM3".to_owned()]);
        assert_eq!(snapshot.last_asr_seq, Some(4));
        assert_eq!(snapshot.last_asr_admission, AsrAdmission::Admitted);
        assert_eq!(snapshot.runtime.state, RuntimeMode::Stopped);
        assert_eq!(polls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn one_admitted_done_updates_snapshot_once() {
        let mut inner = CompanionInner::default();
        apply_asr_outcome(
            &mut inner,
            &AsrOutcome {
                admission: AsrAdmission::Admitted,
                seq: Some(2),
                done: Some(AsrDone {
                    seq: 2,
                    text: "hello".into(),
                }),
            },
        );
        apply_asr_outcome(
            &mut inner,
            &AsrOutcome {
                admission: AsrAdmission::Duplicate,
                seq: Some(2),
                done: None,
            },
        );
        assert_eq!(inner.last_asr_seq, Some(2));
        assert_eq!(inner.last_asr_admission, AsrAdmission::Duplicate);
        let snapshot = build_snapshot(&inner, Vec::new(), None, DeviceSettings::default());
        assert_eq!(snapshot.last_asr_seq, Some(2));
        assert_eq!(snapshot.last_asr_admission, AsrAdmission::Duplicate);
    }

    #[test]
    fn ingest_busy_error_marks_admission_without_a_second_turn() {
        let mut inner = CompanionInner::default();
        apply_asr_outcome(
            &mut inner,
            &AsrOutcome {
                admission: AsrAdmission::Admitted,
                seq: Some(3),
                done: Some(AsrDone {
                    seq: 3,
                    text: "hello".into(),
                }),
            },
        );
        apply_ingest_failure(&mut inner, "正在生成中，请稍后再试".into());
        assert_eq!(inner.last_asr_seq, Some(3));
        assert_eq!(inner.last_asr_admission, AsrAdmission::Busy);
        assert_eq!(
            inner.last_asr_error.as_deref(),
            Some("正在生成中，请稍后再试")
        );
    }

    #[test]
    fn save_command_core_persists_to_an_injected_temporary_store() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let saved = save_profile_to_store(&store, draft()).unwrap();
        assert!(store.path().exists());
        assert_eq!(store.load().unwrap(), Some(saved));
    }

    #[test]
    fn saved_profile_round_trips_into_a_restarted_live_off_runtime() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let saved = save_profile_to_store(&store, draft()).unwrap();
        let loaded = load_profile_from_store(&store).unwrap().unwrap();
        assert_eq!(loaded, saved);
        let mut restarted = RuntimeController::default();
        restarted.set_profile(loaded).unwrap();
        assert_eq!(restarted.status().state, RuntimeMode::Stopped);
        assert!(!restarted.status().live_enabled);
    }
}
