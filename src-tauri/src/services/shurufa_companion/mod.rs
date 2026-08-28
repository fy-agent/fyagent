mod device_settings;
mod input;
mod network;
mod profile;
mod runtime;
mod serial;
mod target;
mod usb_link;
mod windows_foreground_restore;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use device_settings::{DeviceSettings, DeviceSettingsStore};
use network::NetworkStatus;
use profile::{ProfileDraft, ProfileStore, ProfileTarget};
use runtime::{
    RuntimeController, RuntimeMode, RuntimeStatus, WindowsInputDispatcher, WindowsModifierState,
};
use serial::{AsrAdmission, AsrDone, AsrOutcome};
use target::{ForegroundProbe, WindowsForegroundProbe};
use usb_link::{UsbLinkSource, USB_LINK_BAUD, USB_LINK_ID};
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
    cached_ports: Vec<String>,
    cached_profile: Option<ProfileDraft>,
    cached_device: DeviceSettings,
}

#[derive(Clone)]
pub struct CompanionIo {
    inner: Arc<Mutex<CompanionInner>>,
    pause_pump: Arc<AtomicBool>,
    last_snapshot: Arc<Mutex<CompanionSnapshot>>,
}

pub struct CompanionState {
    io: CompanionIo,
    stop: Arc<AtomicBool>,
}

impl CompanionState {
    pub fn new(app: AppHandle) -> Self {
        let mut inner = CompanionInner {
            cached_profile: load_profile_from_store(&profile_store()).ok().flatten(),
            cached_device: load_device_from_store(&device_store()).unwrap_or_default(),
            ..CompanionInner::default()
        };
        if let Some(mut profile) = inner.cached_profile.clone() {
            normalize_link(&mut profile);
            inner.cached_profile = Some(profile.clone());
            let _ = inner.runtime.set_profile(profile);
        }
        let last_snapshot = snapshot_from_inner(&inner);
        let io = CompanionIo {
            inner: Arc::new(Mutex::new(inner)),
            pause_pump: Arc::new(AtomicBool::new(false)),
            last_snapshot: Arc::new(Mutex::new(last_snapshot)),
        };
        let stop = Arc::new(AtomicBool::new(false));
        spawn_pump(io.clone(), stop.clone(), app);
        Self { io, stop }
    }

    pub fn io(&self) -> CompanionIo {
        self.io.clone()
    }

    pub fn snapshot(&self) -> Result<CompanionSnapshot, String> {
        self.io.snapshot()
    }
}

impl CompanionIo {
    pub fn list_ports(&self) -> Result<Vec<String>, String> {
        let ports = usb_presence_ports();
        if let Ok(mut inner) = self.inner.lock() {
            inner.cached_ports = ports.clone();
            self.publish_snapshot(&inner);
        }
        Ok(ports)
    }

    pub fn snapshot(&self) -> Result<CompanionSnapshot, String> {
        match self.inner.try_lock() {
            Ok(inner) => Ok(self.publish_snapshot(&inner)),
            Err(TryLockError::WouldBlock) => Ok(self
                .last_snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()),
            Err(TryLockError::Poisoned(_)) => Err("runtime state is unavailable".to_owned()),
        }
    }

    pub fn capture_target(&self) -> Result<CompanionTarget, String> {
        {
            let inner =
                self.lock_inner_timeout(Duration::from_secs(2), "foreground capture timed out")?;
            inner
                .runtime
                .ensure_stopped()
                .map_err(|error| error.to_string())?;
        }
        std::thread::sleep(Duration::from_secs(3));
        let runtime_guard =
            self.lock_inner_timeout(Duration::from_secs(2), "foreground capture timed out")?;
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

    pub fn save_profile(&self, mut draft: CompanionProfile) -> Result<CompanionProfile, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        normalize_link(&mut draft);
        let saved = save_profile_to_store(&profile_store(), draft)?;
        inner
            .runtime
            .set_profile(saved.clone())
            .map_err(|error| error.to_string())?;
        inner.cached_profile = Some(saved.clone());
        self.publish_snapshot(&inner);
        Ok(saved)
    }

    pub fn start_dry_run(&self) -> Result<CompanionRuntime, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let profile = load_profile_into_runtime(&profile_store(), &mut inner.runtime)?;
        inner.cached_profile = Some(profile);
        let status = start_shortcut(&mut inner.runtime, RuntimeMode::DryRun)?;
        if inner.runtime.has_source() {
            inner.cached_ports = vec![USB_LINK_ID.to_owned()];
        }
        self.publish_snapshot(&inner);
        Ok(status)
    }

    pub fn enable_live(&self) -> Result<CompanionRuntime, String> {
        let mut inner = self.lock_inner()?;
        inner
            .runtime
            .ensure_stopped()
            .map_err(|error| error.to_string())?;
        let profile = load_profile_into_runtime(&profile_store(), &mut inner.runtime)?;
        inner.cached_profile = Some(profile);
        let status = start_shortcut(&mut inner.runtime, RuntimeMode::Live)?;
        if inner.runtime.has_source() {
            inner.cached_ports = vec![USB_LINK_ID.to_owned()];
        }
        self.publish_snapshot(&inner);
        Ok(status)
    }

    pub fn stop(&self) -> Result<CompanionRuntime, String> {
        let mut inner = self.lock_inner()?;
        let status = inner.runtime.stop();
        self.publish_snapshot(&inner);
        Ok(status)
    }

    pub fn save_device_settings(
        &self,
        draft: CompanionDeviceSettings,
    ) -> Result<CompanionDeviceSettings, String> {
        let saved = device_store()
            .save(draft)
            .map_err(|error| error.to_string())?;
        if let Ok(mut inner) = self.inner.lock() {
            inner.cached_device = saved.clone();
            self.publish_snapshot(&inner);
        }
        Ok(saved)
    }

    pub fn apply_device_config(
        &self,
        request: CompanionApplyDeviceConfig,
    ) -> Result<CompanionNetwork, String> {
        request
            .settings
            .validate()
            .map_err(|error| error.to_string())?;
        let saved = device_store()
            .save(request.settings)
            .map_err(|error| error.to_string())?;
        self.pause_pump.store(true, Ordering::SeqCst);
        let result = (|| {
            let mut inner = self
                .lock_inner_timeout(Duration::from_secs(2), "serial pump is busy; retry apply")?;
            inner.cached_device = saved.clone();
            ensure_device_source(&mut inner.runtime)?;
            inner.cached_ports = vec![USB_LINK_ID.to_owned()];
            let network = inner
                .runtime
                .apply_config(&saved)
                .map_err(|error| error.to_string())?;
            self.publish_snapshot(&inner);
            Ok(network)
        })();
        self.pause_pump.store(false, Ordering::SeqCst);
        result
    }

    fn lock_inner(&self) -> Result<MutexGuard<'_, CompanionInner>, String> {
        self.inner
            .lock()
            .map_err(|_| "runtime state is unavailable".to_owned())
    }

    fn lock_inner_timeout(
        &self,
        timeout: Duration,
        busy: &str,
    ) -> Result<MutexGuard<'_, CompanionInner>, String> {
        let started = Instant::now();
        loop {
            match self.inner.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(TryLockError::WouldBlock) if started.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(busy.to_owned());
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err("runtime state is unavailable".to_owned());
                }
            }
        }
    }

    fn publish_snapshot(&self, inner: &CompanionInner) -> CompanionSnapshot {
        let snapshot = snapshot_from_inner(inner);
        *self
            .last_snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = snapshot.clone();
        snapshot
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

fn normalize_link(draft: &mut ProfileDraft) {
    draft.serial.port = USB_LINK_ID.to_owned();
    draft.serial.baud = USB_LINK_BAUD;
}

fn usb_presence_ports() -> Vec<String> {
    if UsbLinkSource::present() {
        vec![USB_LINK_ID.to_owned()]
    } else {
        Vec::new()
    }
}

fn load_profile_from_store(store: &ProfileStore) -> Result<Option<ProfileDraft>, String> {
    let mut profile = store
        .load()
        .map_err(|_| "saved profile is invalid".to_owned())?;
    if let Some(draft) = profile.as_mut() {
        normalize_link(draft);
    }
    Ok(profile)
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
    let mut profile = store
        .load()
        .map_err(|_| "profile is unavailable".to_owned())?
        .ok_or_else(|| "a saved profile is required".to_owned())?;
    normalize_link(&mut profile);
    runtime
        .set_profile(profile.clone())
        .map_err(|error| error.to_string())?;
    Ok(profile)
}

fn ensure_device_source(runtime: &mut RuntimeController) -> Result<(), String> {
    if runtime.source_matches(USB_LINK_ID, USB_LINK_BAUD) {
        return Ok(());
    }
    runtime
        .ensure_stopped()
        .map_err(|error| error.to_string())?;
    runtime.close_source();
    let source = UsbLinkSource::open().map_err(|_| "未插入 Board C USB 设备".to_owned())?;
    runtime.attach_source(USB_LINK_ID.to_owned(), USB_LINK_BAUD, Box::new(source));
    Ok(())
}

fn start_shortcut(
    runtime: &mut RuntimeController,
    mode: RuntimeMode,
) -> Result<RuntimeStatus, String> {
    ensure_device_source(runtime)?;
    runtime
        .start_existing(mode)
        .map_err(|error| error.to_string())
}

fn try_attach_usb(inner: &mut CompanionInner) -> bool {
    if inner.runtime.source_matches(USB_LINK_ID, USB_LINK_BAUD) {
        inner.cached_ports = vec![USB_LINK_ID.to_owned()];
        return true;
    }
    if inner.runtime.has_source() {
        return true;
    }
    match UsbLinkSource::open() {
        Ok(source) => {
            inner
                .runtime
                .attach_source(USB_LINK_ID.to_owned(), USB_LINK_BAUD, Box::new(source));
            inner.cached_ports = vec![USB_LINK_ID.to_owned()];
            true
        }
        Err(_) => {
            inner.cached_ports.clear();
            false
        }
    }
}

fn snapshot_from_inner(inner: &CompanionInner) -> CompanionSnapshot {
    CompanionSnapshot {
        ports: inner.cached_ports.clone(),
        profile: inner.cached_profile.clone(),
        device: inner.cached_device.clone(),
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

fn spawn_pump(io: CompanionIo, stop: Arc<AtomicBool>, app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("shurufa-companion-pump".into())
        .spawn(move || {
            let probe = WindowsForegroundProbe;
            let restorer = WindowsForegroundTargetRestorer;
            let modifiers = WindowsModifierState;
            while !stop.load(Ordering::Relaxed) {
                if io.pause_pump.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                let mut dispatcher = WindowsInputDispatcher;
                let asr_done = {
                    let mut guard = match io.inner.lock() {
                        Ok(guard) => guard,
                        Err(_) => break,
                    };
                    if !guard.runtime.has_source() && !try_attach_usb(&mut guard) {
                        io.publish_snapshot(&guard);
                        drop(guard);
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                    match guard
                        .runtime
                        .pump_once(&probe, &restorer, &modifiers, &mut dispatcher)
                    {
                        Ok(outcome) => {
                            apply_asr_outcome(&mut guard, &outcome.asr);
                            if !guard.runtime.has_source() {
                                guard.cached_ports.clear();
                            } else {
                                guard.cached_ports = vec![USB_LINK_ID.to_owned()];
                            }
                            io.publish_snapshot(&guard);
                            outcome.asr.done
                        }
                        Err(_) => None,
                    }
                };
                if let Some(done) = asr_done {
                    spawn_ingest(app.clone(), io.inner.clone(), done);
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
    use crate::services::shurufa_companion::usb_link::{USB_LINK_BAUD, USB_LINK_ID};
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
                MappingDraft {
                    input: InputId::ButtonA,
                    display_name: "Key A".into(),
                    keys: vec!["CTRL".into(), "1".into()],
                },
                MappingDraft {
                    input: InputId::ButtonB,
                    display_name: "Key B".into(),
                    keys: vec!["CTRL".into(), "2".into()],
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
    fn snapshot_does_not_block_when_runtime_lock_is_held() {
        let mut inner = CompanionInner::default();
        inner.cached_ports = vec![USB_LINK_ID.into()];
        let published = snapshot_from_inner(&inner);
        let io = CompanionIo {
            inner: Arc::new(Mutex::new(inner)),
            pause_pump: Arc::new(AtomicBool::new(false)),
            last_snapshot: Arc::new(Mutex::new(published)),
        };
        let _hold = io.inner.lock().unwrap();
        let snapshot = io.snapshot().unwrap();
        assert_eq!(snapshot.ports, vec![USB_LINK_ID.to_owned()]);
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
        inner.cached_ports = vec![USB_LINK_ID.into()];
        let snapshot = snapshot_from_inner(&inner);
        assert_eq!(snapshot.ports, vec![USB_LINK_ID.to_owned()]);
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
        let snapshot = snapshot_from_inner(&inner);
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
        assert_eq!(loaded.revision, saved.revision);
        assert_eq!(loaded.mappings, saved.mappings);
        assert_eq!(loaded.serial.port, USB_LINK_ID);
        assert_eq!(loaded.serial.baud, USB_LINK_BAUD);
        let mut restarted = RuntimeController::default();
        restarted.set_profile(loaded).unwrap();
        assert_eq!(restarted.status().state, RuntimeMode::Stopped);
        assert!(!restarted.status().live_enabled);
    }

    #[test]
    fn legacy_com_profile_hydrates_usb_and_save_upgrades_five_mappings() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let mut legacy = draft();
        legacy.serial.port = "COM3".into();
        legacy
            .mappings
            .retain(|mapping| InputId::LEGACY.contains(&mapping.input));
        let legacy = legacy.with_computed_revision();
        std::fs::write(store.path(), serde_json::to_vec(&legacy).unwrap()).unwrap();

        let loaded = load_profile_from_store(&store).unwrap().unwrap();
        assert_eq!(loaded.serial.port, USB_LINK_ID);
        assert_eq!(loaded.serial.baud, USB_LINK_BAUD);
        assert_eq!(loaded.revision, legacy.revision);
        assert_eq!(loaded.mappings.len(), 3);

        let mut upgraded = draft();
        upgraded.revision = loaded.revision.clone();
        normalize_link(&mut upgraded);
        let saved = save_profile_to_store(&store, upgraded).unwrap();
        assert_eq!(saved.serial.port, USB_LINK_ID);
        assert_eq!(saved.serial.baud, USB_LINK_BAUD);
        assert_eq!(saved.mappings.len(), 5);
        assert_ne!(saved.revision, legacy.revision);
        assert!(directory.path().join("profile.json.bak").exists());
        let stale = draft();
        assert_eq!(
            save_profile_to_store(&store, stale).unwrap_err(),
            "profile revision is stale"
        );
    }
}
