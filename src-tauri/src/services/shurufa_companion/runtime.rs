use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use super::device_settings::DeviceSettings;
use super::input::Chord;
use super::network::{ssid_looks_5g, DeviceConfigRecord, NetworkState, NetworkStatus};
use super::profile::ProfileDraft;
use super::serial::{AsrOutcome, EventSource, SerialError, SerialEvent};
#[cfg(test)]
use super::target::WindowsForegroundProbe;
use super::target::{
    evaluate_target, ForegroundProbe, ForegroundRestoreOutcome, ForegroundTargetRestorer, Target,
    TargetDecision,
};
#[cfg(test)]
use super::windows_foreground_restore::WindowsForegroundTargetRestorer;

pub trait InputDispatcher {
    fn dispatch(&mut self, chord: &Chord) -> Result<(), RuntimeError>;
}
pub trait ModifierState {
    fn all_keys_clear(&self, chord: &Chord) -> bool;
}

/// Windows-specific effects are isolated here. They are created only by a live
/// polling command after a profile and serial runtime were explicitly started.
pub struct WindowsModifierState;
pub struct WindowsInputDispatcher;

#[cfg(target_os = "windows")]
fn virtual_key(token: &str) -> Option<u16> {
    match token {
        "CTRL" => Some(0x11),
        "ALT" => Some(0x12),
        "SHIFT" => Some(0x10),
        "ENTER" => Some(0x0d),
        "TAB" => Some(0x09),
        "ESC" => Some(0x1b),
        "SPACE" => Some(0x20),
        "[" => Some(0xdb),
        "]" => Some(0xdd),
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_uppercase() => {
            Some(u16::from(value.as_bytes()[0]))
        }
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_digit() => {
            Some(u16::from(value.as_bytes()[0]))
        }
        value => value
            .strip_prefix('F')
            .and_then(|number| number.parse::<u16>().ok())
            .filter(|number| (1..=24).contains(number))
            .map(|number| 0x70 + number - 1),
    }
}

#[cfg(target_os = "windows")]
impl ModifierState for WindowsModifierState {
    fn all_keys_clear(&self, chord: &Chord) -> bool {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetAsyncKeyState(virtual_key: i32) -> i16;
        }
        keyboard_state_is_clear(chord, |key| unsafe {
            (GetAsyncKeyState(i32::from(key)) as u16 & 0x8000) != 0
        })
    }
}
#[cfg(not(target_os = "windows"))]
impl ModifierState for WindowsModifierState {
    fn all_keys_clear(&self, _: &Chord) -> bool {
        false
    }
}

#[cfg(target_os = "windows")]
impl InputDispatcher for WindowsInputDispatcher {
    fn dispatch(&mut self, chord: &Chord) -> Result<(), RuntimeError> {
        if !WindowsModifierState.all_keys_clear(chord) {
            return Err(RuntimeError::DirtyModifiers);
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct KeyboardInput {
            virtual_key: u16,
            scan_code: u16,
            flags: u32,
            time: u32,
            extra_info: usize,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct MouseInput {
            dx: i32,
            dy: i32,
            mouse_data: u32,
            flags: u32,
            time: u32,
            extra_info: usize,
        }
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct HardwareInput {
            message: u32,
            parameter_low: u16,
            parameter_high: u16,
        }
        #[repr(C)]
        union InputData {
            mouse: MouseInput,
            keyboard: KeyboardInput,
            hardware: HardwareInput,
        }
        #[repr(C)]
        struct Input {
            input_type: u32,
            data: InputData,
        }
        #[link(name = "user32")]
        unsafe extern "system" {
            fn SendInput(input_count: u32, inputs: *const Input, input_size: i32) -> u32;
        }
        const INPUT_KEYBOARD: u32 = 1;
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        let keys = chord
            .0
            .iter()
            .map(|token| virtual_key(token).ok_or(RuntimeError::DispatchRejected))
            .collect::<Result<Vec<_>, _>>()?;
        let mut inputs = Vec::with_capacity(keys.len() * 2);
        for key in &keys {
            inputs.push(Input {
                input_type: INPUT_KEYBOARD,
                data: InputData {
                    keyboard: KeyboardInput {
                        virtual_key: *key,
                        scan_code: 0,
                        flags: 0,
                        time: 0,
                        extra_info: 0,
                    },
                },
            });
        }
        for key in keys.iter().rev() {
            inputs.push(Input {
                input_type: INPUT_KEYBOARD,
                data: InputData {
                    keyboard: KeyboardInput {
                        virtual_key: *key,
                        scan_code: 0,
                        flags: KEYEVENTF_KEYUP,
                        time: 0,
                        extra_info: 0,
                    },
                },
            });
        }
        let expected = inputs.len() as u32;
        let sent = unsafe {
            SendInput(
                expected,
                inputs.as_ptr(),
                std::mem::size_of::<Input>() as i32,
            )
        };
        if sent == expected {
            Ok(())
        } else {
            Err(RuntimeError::DispatchRejected)
        }
    }
}

#[cfg(target_os = "windows")]
fn keyboard_state_is_clear(chord: &Chord, mut key_is_down: impl FnMut(u16) -> bool) -> bool {
    // Unrelated held modifiers would alter the configured chord, so the guard
    // checks all supported modifiers plus every configured primary key.
    ["CTRL", "ALT", "SHIFT"]
        .into_iter()
        .chain(
            chord
                .0
                .iter()
                .map(String::as_str)
                .filter(|token| !matches!(*token, "CTRL" | "ALT" | "SHIFT")),
        )
        .filter_map(virtual_key)
        .all(|key| !key_is_down(key))
}
#[cfg(not(target_os = "windows"))]
impl InputDispatcher for WindowsInputDispatcher {
    fn dispatch(&mut self, _: &Chord) -> Result<(), RuntimeError> {
        Err(RuntimeError::DispatchRejected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeMode {
    Stopped,
    DryRun,
    Live,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub state: RuntimeMode,
    pub live_enabled: bool,
    pub last_event: String,
    pub gap_missed: Option<u32>,
    #[serde(default)]
    pub network: NetworkStatus,
}
impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            state: RuntimeMode::Stopped,
            live_enabled: false,
            last_event: "No event yet.".into(),
            gap_missed: None,
            network: NetworkStatus::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PumpOutcome {
    pub status: RuntimeStatus,
    pub asr: AsrOutcome,
}

#[derive(Default)]
pub struct RuntimeController {
    status: RuntimeStatus,
    profile: Option<ProfileDraft>,
    source: Option<Box<dyn EventSource>>,
    open_port: Option<(String, u32)>,
    config_seq: u32,
}
impl RuntimeController {
    pub fn ensure_stopped(&self) -> Result<(), RuntimeError> {
        if self.status.state == RuntimeMode::Stopped {
            Ok(())
        } else {
            Err(RuntimeError::AlreadyRunning)
        }
    }

    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    pub fn source_matches(&self, port: &str, baud: u32) -> bool {
        self.open_port.as_ref() == Some(&(port.to_owned(), baud))
    }

    pub fn attach_source(&mut self, port: String, baud: u32, source: Box<dyn EventSource>) {
        self.source = Some(source);
        self.open_port = Some((port, baud));
    }

    pub fn close_source(&mut self) {
        if let Some(source) = self.source.as_mut() {
            source.close();
        }
        self.source = None;
        self.open_port = None;
    }

    pub fn start_existing(&mut self, mode: RuntimeMode) -> Result<RuntimeStatus, RuntimeError> {
        self.start(mode, None)
    }

    pub fn set_profile(&mut self, profile: ProfileDraft) -> Result<(), RuntimeError> {
        self.ensure_stopped()?;
        profile
            .validate()
            .map_err(|_| RuntimeError::InvalidProfile)?;
        if profile.revision.is_none() {
            return Err(RuntimeError::InvalidProfile);
        }
        self.profile = Some(profile);
        Ok(())
    }
    #[cfg(test)]
    pub fn start_dry_run(
        &mut self,
        source: Box<dyn EventSource>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        self.start(RuntimeMode::DryRun, Some(source))
    }
    #[cfg(test)]
    pub fn enable_live_for_run(
        &mut self,
        source: Box<dyn EventSource>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        self.start(RuntimeMode::Live, Some(source))
    }
    fn start(
        &mut self,
        mode: RuntimeMode,
        source: Option<Box<dyn EventSource>>,
    ) -> Result<RuntimeStatus, RuntimeError> {
        self.ensure_stopped()?;
        if self.profile.is_none() {
            return Err(RuntimeError::ProfileRequired);
        }
        if let Some(source) = source {
            if let Some(previous) = self.source.as_mut() {
                previous.close();
            }
            self.source = Some(source);
        } else if self.source.is_none() {
            return Err(RuntimeError::Serial);
        }
        let network = self.status.network.clone();
        self.status = RuntimeStatus {
            state: mode,
            live_enabled: mode == RuntimeMode::Live,
            last_event: match mode {
                RuntimeMode::DryRun => "Dry-run started. No dispatcher constructed.".into(),
                RuntimeMode::Live => "Live enabled for this process only.".into(),
                RuntimeMode::Stopped => unreachable!(),
            },
            gap_missed: None,
            network,
        };
        Ok(self.status.clone())
    }
    #[cfg(test)]
    pub fn poll_dry_run(&mut self) -> Result<RuntimeStatus, RuntimeError> {
        if self.status.state != RuntimeMode::DryRun {
            return Err(RuntimeError::Stopped);
        }
        let probe = WindowsForegroundProbe;
        let restorer = WindowsForegroundTargetRestorer;
        let modifiers = WindowsModifierState;
        let mut dispatcher = WindowsInputDispatcher;
        Ok(self
            .pump_once(&probe, &restorer, &modifiers, &mut dispatcher)?
            .status)
    }
    #[cfg(test)]
    pub fn poll_live<P, R, M, D>(
        &mut self,
        probe: &P,
        restorer: &R,
        modifiers: &M,
        dispatcher: &mut D,
    ) -> Result<RuntimeStatus, RuntimeError>
    where
        P: ForegroundProbe,
        R: ForegroundTargetRestorer,
        M: ModifierState,
        D: InputDispatcher,
    {
        if self.status.state != RuntimeMode::Live || !self.status.live_enabled {
            return Err(RuntimeError::Stopped);
        }
        Ok(self
            .pump_once(probe, restorer, modifiers, dispatcher)?
            .status)
    }

    pub fn pump_once<P, R, M, D>(
        &mut self,
        probe: &P,
        restorer: &R,
        modifiers: &M,
        dispatcher: &mut D,
    ) -> Result<PumpOutcome, RuntimeError>
    where
        P: ForegroundProbe,
        R: ForegroundTargetRestorer,
        M: ModifierState,
        D: InputDispatcher,
    {
        if self.source.is_none() {
            return Ok(PumpOutcome {
                status: self.status.clone(),
                asr: AsrOutcome::default(),
            });
        }
        let event = match self.next_event() {
            Ok(event) => event,
            Err(RuntimeError::Serial) => {
                return Ok(PumpOutcome {
                    status: self.stop_with_last_event(RuntimeError::Serial.to_string()),
                    asr: AsrOutcome::default(),
                });
            }
            Err(error) => return Err(error),
        };
        let asr = self
            .source
            .as_mut()
            .map(|source| source.take_asr_outcome())
            .unwrap_or_default();
        if let Some(event) = event {
            match self.status.state {
                RuntimeMode::DryRun => {
                    let chord = resolve_event(
                        &event,
                        self.profile.as_ref().ok_or(RuntimeError::ProfileRequired)?,
                    )?;
                    self.status.gap_missed = event.gap_missed;
                    self.status.last_event = gap_prefix(
                        event.gap_missed,
                        format!("{} → {} · dry-run", event.input, chord.canonical()),
                    );
                }
                RuntimeMode::Live if self.status.live_enabled => {
                    let profile = self.profile.as_ref().ok_or(RuntimeError::ProfileRequired)?;
                    let saved_target = profile
                        .target
                        .as_ref()
                        .ok_or(RuntimeError::ProfileRequired)?;
                    let target = Target::new(
                        saved_target.process_name.clone(),
                        saved_target.process_path.clone(),
                    )
                    .map_err(|_| RuntimeError::InvalidProfile)?;
                    self.status.gap_missed = event.gap_missed;
                    self.status.last_event = gap_prefix(
                        event.gap_missed,
                        match handle_live_event(
                            &event, profile, &target, probe, restorer, modifiers, dispatcher,
                        ) {
                            Ok(event) => event,
                            Err(error) => format!("{} · rejected", error),
                        },
                    );
                }
                RuntimeMode::Stopped | RuntimeMode::Live => {}
            }
        }
        self.refresh_network();
        Ok(PumpOutcome {
            status: self.status.clone(),
            asr,
        })
    }

    fn next_event(&mut self) -> Result<Option<SerialEvent>, RuntimeError> {
        self.source
            .as_mut()
            .ok_or(RuntimeError::Stopped)?
            .poll_event()
            .map_err(|_| RuntimeError::Serial)
    }
    pub fn stop(&mut self) -> RuntimeStatus {
        let network = self.status.network.clone();
        self.status = RuntimeStatus::default();
        self.status.network = network;
        self.status.clone()
    }
    fn stop_with_last_event(&mut self, last_event: String) -> RuntimeStatus {
        self.close_source();
        self.stop();
        self.status.last_event = last_event;
        self.status.clone()
    }
    pub fn status(&self) -> RuntimeStatus {
        self.status.clone()
    }
    pub fn apply_config(
        &mut self,
        settings: &DeviceSettings,
    ) -> Result<NetworkStatus, RuntimeError> {
        let source = self.source.as_mut().ok_or(RuntimeError::Serial)?;
        self.config_seq = self.config_seq.saturating_add(1);
        let line = DeviceConfigRecord {
            seq: self.config_seq,
            ssid: settings.ssid.clone(),
            password: settings.password.clone(),
            api_key: settings.api_key.clone(),
            model: settings.model.clone(),
        }
        .line()
        .map_err(|_| RuntimeError::InvalidProfile)?;
        source.write_line(&line).map_err(|_| RuntimeError::Serial)?;
        let looks_5g = ssid_looks_5g(&settings.ssid);
        self.status.network = NetworkStatus {
            state: if looks_5g {
                NetworkState::Failed
            } else {
                NetworkState::Connecting
            },
            ssid: settings.ssid.clone(),
            reason: looks_5g.then(|| "BAND".to_owned()),
            ..NetworkStatus::default()
        };
        Ok(self.status.network.clone())
    }
    fn refresh_network(&mut self) {
        if let Some(source) = self.source.as_ref() {
            if let Some(network) = source.last_network_status() {
                self.status.network = network;
            }
        }
    }
}
pub fn resolve_event(event: &SerialEvent, profile: &ProfileDraft) -> Result<Chord, RuntimeError> {
    profile
        .mapping_for(event.input)
        .ok_or(RuntimeError::Unmapped)
        .and_then(|mapping| Chord::parse(&mapping.keys).map_err(|_| RuntimeError::InvalidProfile))
}

fn gap_prefix(gap_missed: Option<u32>, event: String) -> String {
    gap_missed.map_or(event.clone(), |missed| {
        format!("SERIAL_GAP/{missed}: {event}")
    })
}
pub fn handle_live_event<P, R, M, D>(
    event: &SerialEvent,
    profile: &ProfileDraft,
    target: &Target,
    probe: &P,
    restorer: &R,
    modifiers: &M,
    dispatcher: &mut D,
) -> Result<String, RuntimeError>
where
    P: ForegroundProbe,
    R: ForegroundTargetRestorer,
    M: ModifierState,
    D: InputDispatcher,
{
    let chord = resolve_event(event, profile)?;
    if evaluate_target(probe, target) != TargetDecision::Ready {
        match restorer.restore_saved_target(target) {
            ForegroundRestoreOutcome::Unchanged | ForegroundRestoreOutcome::Restored => {}
            ForegroundRestoreOutcome::Missing => return Err(RuntimeError::RestoreMissing),
            ForegroundRestoreOutcome::Rejected => return Err(RuntimeError::RestoreRejected),
        }
    }
    if evaluate_target(probe, target) != TargetDecision::Ready {
        return Err(RuntimeError::WrongForeground);
    }
    if !modifiers.all_keys_clear(&chord) {
        return Err(RuntimeError::DirtyModifiers);
    }
    dispatcher.dispatch(&chord)?;
    Ok(format!(
        "{} → {} · dispatched",
        event.input,
        chord.canonical()
    ))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    Stopped,
    AlreadyRunning,
    ProfileRequired,
    Unmapped,
    InvalidProfile,
    Serial,
    WrongForeground,
    RestoreMissing,
    RestoreRejected,
    DirtyModifiers,
    DispatchRejected,
}
impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Stopped => "runtime is stopped",
            Self::AlreadyRunning => "stop the active runtime before changing configuration",
            Self::ProfileRequired => "a valid saved profile is required",
            Self::Unmapped => "input is unmapped",
            Self::InvalidProfile => "profile mapping is invalid",
            Self::Serial => "serial input stopped",
            Self::WrongForeground => "foreground target did not match",
            Self::RestoreMissing => "foreground restore target is missing",
            Self::RestoreRejected => "foreground restore was rejected",
            Self::DirtyModifiers => "keyboard state is not clear",
            Self::DispatchRejected => "input dispatch rejected",
        })
    }
}
impl std::error::Error for RuntimeError {}
impl From<SerialError> for RuntimeError {
    fn from(_: SerialError) -> Self {
        Self::Serial
    }
}

#[cfg(test)]
mod tests {
    use super::super::input::InputId;
    use super::super::profile::PROFILE_VERSION;
    use super::super::profile::{MappingDraft, ProfileSerial, ProfileTarget};
    use super::super::serial::{AsrAdmission, AsrDone};
    use super::super::target::{
        ForegroundIdentity, ForegroundRestoreOutcome, ForegroundTargetRestorer,
        NoopForegroundRestorer, Target, TargetError,
    };
    use super::*;
    use std::cell::Cell;
    use std::collections::VecDeque;
    struct Source(VecDeque<Result<Option<SerialEvent>, SerialError>>);
    impl EventSource for Source {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            self.0.pop_front().unwrap_or(Ok(None))
        }
    }
    struct ReadyProbe;
    impl ForegroundProbe for ReadyProbe {
        fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError> {
            Ok(Some(ForegroundIdentity {
                process_name: "Fixture.exe".into(),
                process_path: r"C:\Fixture.exe".into(),
            }))
        }
    }
    struct DirtyModifiers;
    impl ModifierState for DirtyModifiers {
        fn all_keys_clear(&self, _: &Chord) -> bool {
            false
        }
    }
    struct ClearModifiers;
    impl ModifierState for ClearModifiers {
        fn all_keys_clear(&self, _: &Chord) -> bool {
            true
        }
    }
    struct Dispatcher(Cell<u8>);
    impl InputDispatcher for Dispatcher {
        fn dispatch(&mut self, _: &Chord) -> Result<(), RuntimeError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
    }
    struct SwitchableProbe {
        ready: Cell<bool>,
    }
    impl ForegroundProbe for SwitchableProbe {
        fn foreground_identity(&self) -> Result<Option<ForegroundIdentity>, TargetError> {
            Ok(Some(if self.ready.get() {
                ForegroundIdentity {
                    process_name: "Fixture.exe".into(),
                    process_path: r"C:\Fixture.exe".into(),
                }
            } else {
                ForegroundIdentity {
                    process_name: "Other.exe".into(),
                    process_path: r"C:\Other.exe".into(),
                }
            }))
        }
    }
    struct RecordingRestorer<'a> {
        calls: Cell<u8>,
        outcome: ForegroundRestoreOutcome,
        probe: &'a SwitchableProbe,
    }
    impl ForegroundTargetRestorer for RecordingRestorer<'_> {
        fn restore_saved_target(&self, _target: &Target) -> ForegroundRestoreOutcome {
            self.calls.set(self.calls.get() + 1);
            if self.outcome == ForegroundRestoreOutcome::Restored {
                self.probe.ready.set(true);
            }
            self.outcome
        }
    }
    fn profile() -> ProfileDraft {
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
            mappings: InputId::ALL
                .into_iter()
                .map(|input| MappingDraft {
                    input,
                    display_name: input.to_string(),
                    keys: if input == InputId::EncoderCw {
                        vec!["CTRL".into(), "TAB".into()]
                    } else if input == InputId::EncoderCcw {
                        vec!["CTRL".into(), "SHIFT".into(), "TAB".into()]
                    } else {
                        vec!["ENTER".into()]
                    },
                })
                .collect(),
        }
        .with_computed_revision()
    }
    fn event_source() -> Box<dyn EventSource> {
        Box::new(Source(VecDeque::from([Ok(Some(SerialEvent {
            sequence: 1,
            input: InputId::EncoderCw,
            gap_missed: None,
        }))])))
    }
    #[test]
    fn live_cannot_start_without_a_valid_saved_profile() {
        let mut runtime = RuntimeController::default();
        assert_eq!(
            runtime.enable_live_for_run(event_source()),
            Err(RuntimeError::ProfileRequired)
        );
    }
    #[test]
    fn dry_run_never_constructs_or_calls_a_dispatcher() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.start_dry_run(event_source()).unwrap();
        let status = runtime.poll_dry_run().unwrap();
        assert!(status.last_event.contains("dry-run"));
    }
    #[test]
    fn live_rechecks_guards_before_dispatch() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.enable_live_for_run(event_source()).unwrap();
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = runtime
            .poll_live(
                &ReadyProbe,
                &NoopForegroundRestorer,
                &DirtyModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(status.state, RuntimeMode::Live);
        assert_eq!(status.last_event, "keyboard state is not clear · rejected");
        assert_eq!(dispatcher.0.get(), 0);
    }
    #[test]
    fn live_dispatches_once_after_all_guards_pass() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.enable_live_for_run(event_source()).unwrap();
        let mut dispatcher = Dispatcher(Cell::new(0));
        runtime
            .poll_live(
                &ReadyProbe,
                &NoopForegroundRestorer,
                &ClearModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(dispatcher.0.get(), 1);
    }
    #[test]
    fn dry_run_reports_a_forward_gap_but_processes_the_valid_event() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        let source = Box::new(Source(VecDeque::from([Ok(Some(SerialEvent {
            sequence: 4,
            input: InputId::EncoderCw,
            gap_missed: Some(2),
        }))])));
        runtime.start_dry_run(source).unwrap();
        let status = runtime.poll_dry_run().unwrap();
        assert_eq!(status.gap_missed, Some(2));
        assert_eq!(
            status.last_event,
            "SERIAL_GAP/2: ENCODER_CW → CTRL+TAB · dry-run"
        );
    }

    #[test]
    fn running_runtime_rejects_profile_changes_and_repeated_start() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.start_dry_run(event_source()).unwrap();
        assert_eq!(
            runtime.set_profile(profile()),
            Err(RuntimeError::AlreadyRunning)
        );
        assert_eq!(
            runtime.start_dry_run(event_source()),
            Err(RuntimeError::AlreadyRunning)
        );
    }

    #[test]
    fn serial_read_error_stops_and_clears_live_permission() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime
            .enable_live_for_run(Box::new(Source(VecDeque::from([Err(SerialError::Read)]))))
            .unwrap();
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = runtime
            .poll_live(
                &ReadyProbe,
                &NoopForegroundRestorer,
                &ClearModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(status.state, RuntimeMode::Stopped);
        assert!(!status.live_enabled);
        assert_eq!(status.last_event, "serial input stopped");
        assert_eq!(dispatcher.0.get(), 0);
    }

    #[test]
    fn resolve_event_rejects_an_unmapped_input() {
        let event = SerialEvent {
            sequence: 1,
            input: InputId::EncoderPress,
            gap_missed: None,
        };
        let mut incomplete = profile();
        incomplete
            .mappings
            .retain(|mapping| mapping.input != InputId::EncoderPress);
        assert_eq!(
            resolve_event(&event, &incomplete),
            Err(RuntimeError::Unmapped)
        );
    }

    fn live_with_restore(
        probe: &SwitchableProbe,
        restorer: &RecordingRestorer<'_>,
        dispatcher: &mut Dispatcher,
    ) -> RuntimeStatus {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.enable_live_for_run(event_source()).unwrap();
        runtime
            .poll_live(probe, restorer, &ClearModifiers, dispatcher)
            .unwrap()
    }

    #[test]
    fn restore_missing_rejects_with_zero_dispatch() {
        let probe = SwitchableProbe {
            ready: Cell::new(false),
        };
        let restorer = RecordingRestorer {
            calls: Cell::new(0),
            outcome: ForegroundRestoreOutcome::Missing,
            probe: &probe,
        };
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = live_with_restore(&probe, &restorer, &mut dispatcher);
        assert_eq!(
            status.last_event,
            "foreground restore target is missing · rejected"
        );
        assert_eq!(restorer.calls.get(), 1);
        assert_eq!(dispatcher.0.get(), 0);
    }

    #[test]
    fn restore_rejected_rejects_with_zero_dispatch() {
        let probe = SwitchableProbe {
            ready: Cell::new(false),
        };
        let restorer = RecordingRestorer {
            calls: Cell::new(0),
            outcome: ForegroundRestoreOutcome::Rejected,
            probe: &probe,
        };
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = live_with_restore(&probe, &restorer, &mut dispatcher);
        assert_eq!(
            status.last_event,
            "foreground restore was rejected · rejected"
        );
        assert_eq!(restorer.calls.get(), 1);
        assert_eq!(dispatcher.0.get(), 0);
    }

    #[test]
    fn restore_then_exact_recheck_dispatches_once() {
        let probe = SwitchableProbe {
            ready: Cell::new(false),
        };
        let restorer = RecordingRestorer {
            calls: Cell::new(0),
            outcome: ForegroundRestoreOutcome::Restored,
            probe: &probe,
        };
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = live_with_restore(&probe, &restorer, &mut dispatcher);
        assert!(status.last_event.contains("dispatched"));
        assert_eq!(restorer.calls.get(), 1);
        assert_eq!(dispatcher.0.get(), 1);
    }

    #[test]
    fn already_matching_foreground_leaves_restore_unchanged_and_dispatches() {
        let probe = SwitchableProbe {
            ready: Cell::new(true),
        };
        let restorer = RecordingRestorer {
            calls: Cell::new(0),
            outcome: ForegroundRestoreOutcome::Restored,
            probe: &probe,
        };
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = live_with_restore(&probe, &restorer, &mut dispatcher);
        assert!(status.last_event.contains("dispatched"));
        assert_eq!(restorer.calls.get(), 0);
        assert_eq!(dispatcher.0.get(), 1);
    }

    #[test]
    fn restore_success_without_exact_recheck_never_dispatches() {
        let probe = SwitchableProbe {
            ready: Cell::new(false),
        };
        struct RestoredWithoutFocus;
        impl ForegroundTargetRestorer for RestoredWithoutFocus {
            fn restore_saved_target(&self, _target: &Target) -> ForegroundRestoreOutcome {
                ForegroundRestoreOutcome::Restored
            }
        }
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime.enable_live_for_run(event_source()).unwrap();
        let mut dispatcher = Dispatcher(Cell::new(0));
        let status = runtime
            .poll_live(
                &probe,
                &RestoredWithoutFocus,
                &ClearModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(
            status.last_event,
            "foreground target did not match · rejected"
        );
        assert_eq!(dispatcher.0.get(), 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn keyboard_guard_checks_unconfigured_modifiers_too() {
        let chord = Chord::parse(&["CTRL".into(), "TAB".into()]).unwrap();
        assert!(!keyboard_state_is_clear(&chord, |key| key == 0x12));
    }

    struct ConfigSource {
        written: String,
        network: Option<NetworkStatus>,
    }
    impl EventSource for ConfigSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            Ok(None)
        }
        fn write_line(&mut self, line: &str) -> Result<(), SerialError> {
            self.written = line.to_owned();
            self.network = Some(NetworkStatus {
                state: NetworkState::Connecting,
                ssid: "cafe".into(),
                ..NetworkStatus::default()
            });
            Ok(())
        }
        fn last_network_status(&self) -> Option<NetworkStatus> {
            self.network.clone()
        }
    }

    #[test]
    fn apply_config_writes_device_link_line_without_changing_shortcut_mode() {
        let mut runtime = RuntimeController::default();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(ConfigSource {
                written: String::new(),
                network: None,
            }),
        );
        let status = runtime
            .apply_config(&DeviceSettings {
                version: 1,
                ssid: "cafe".into(),
                password: "secret".into(),
                api_key: "sk-demo".into(),
                model: "XingChenAGI/XingChenASR-V3.2-Ultra".into(),
            })
            .unwrap();
        assert_eq!(status.state, NetworkState::Connecting);
        assert_eq!(runtime.status().state, RuntimeMode::Stopped);
        assert_eq!(status.ssid, "cafe");
    }

    #[test]
    fn apply_config_allows_empty_cloud_fields_for_wifi_only() {
        let mut runtime = RuntimeController::default();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(ConfigSource {
                written: String::new(),
                network: None,
            }),
        );
        let status = runtime
            .apply_config(&DeviceSettings {
                version: 1,
                ssid: "cafe".into(),
                password: "secret".into(),
                api_key: String::new(),
                model: String::new(),
            })
            .unwrap();
        assert_eq!(status.state, NetworkState::Connecting);
        assert_eq!(status.ssid, "cafe");
    }

    #[test]
    fn apply_config_fails_closed_when_ssid_looks_like_5g() {
        let mut runtime = RuntimeController::default();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(ConfigSource {
                written: String::new(),
                network: None,
            }),
        );
        let status = runtime
            .apply_config(&DeviceSettings {
                version: 1,
                ssid: "Home-5G".into(),
                password: "secret".into(),
                api_key: "sk-demo".into(),
                model: "XingChenAGI/XingChenASR-V3.2-Ultra".into(),
            })
            .unwrap();
        assert_eq!(status.state, NetworkState::Failed);
        assert_eq!(status.reason.as_deref(), Some("BAND"));
        assert_eq!(status.ssid, "Home-5G");
    }

    struct CountingSource {
        polls: std::sync::Arc<std::sync::atomic::AtomicU32>,
    }
    impl EventSource for CountingSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            self.polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(None)
        }
    }

    struct AsrSource {
        asr: Option<AsrOutcome>,
    }
    impl EventSource for AsrSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            Ok(None)
        }
        fn take_asr_outcome(&mut self) -> AsrOutcome {
            self.asr.take().unwrap_or_default()
        }
    }

    #[test]
    fn status_snapshot_does_not_consume_serial() {
        let polls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let mut runtime = RuntimeController::default();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(CountingSource {
                polls: polls.clone(),
            }),
        );
        let _ = runtime.status();
        assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn stopped_pump_admits_one_asr_done_without_dispatch() {
        let mut runtime = RuntimeController::default();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(AsrSource {
                asr: Some(AsrOutcome {
                    admission: AsrAdmission::Admitted,
                    seq: Some(2),
                    done: Some(AsrDone {
                        seq: 2,
                        text: "hello".into(),
                    }),
                }),
            }),
        );
        let mut dispatcher = Dispatcher(Cell::new(0));
        let outcome = runtime
            .pump_once(
                &ReadyProbe,
                &NoopForegroundRestorer,
                &ClearModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(outcome.asr.admission, AsrAdmission::Admitted);
        assert_eq!(
            outcome.asr.done.as_ref().map(|done| done.text.as_str()),
            Some("hello")
        );
        assert_eq!(runtime.status().state, RuntimeMode::Stopped);
        assert_eq!(dispatcher.0.get(), 0);
    }

    #[test]
    fn serial_error_during_stopped_pump_clears_live_and_closes_source() {
        let mut runtime = RuntimeController::default();
        runtime.set_profile(profile()).unwrap();
        runtime
            .enable_live_for_run(Box::new(Source(VecDeque::from([Err(SerialError::Read)]))))
            .unwrap();
        runtime.stop();
        runtime.attach_source(
            "fixture".into(),
            115200,
            Box::new(Source(VecDeque::from([Err(SerialError::Read)]))),
        );
        let mut dispatcher = Dispatcher(Cell::new(0));
        let outcome = runtime
            .pump_once(
                &ReadyProbe,
                &NoopForegroundRestorer,
                &ClearModifiers,
                &mut dispatcher,
            )
            .unwrap();
        assert_eq!(outcome.status.state, RuntimeMode::Stopped);
        assert!(!outcome.status.live_enabled);
        assert_eq!(outcome.status.last_event, "serial input stopped");
        assert!(!runtime.has_source());
        assert_eq!(dispatcher.0.get(), 0);
    }
}
