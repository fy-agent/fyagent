use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::input::InputId;
use super::network::{NetworkState, NetworkStatus};

pub const MAX_LINE_BYTES: usize = 1024;
const PREFIX: &str = "VKEY_INPUT/1 ";
const NET_PREFIX: &str = "VKEY_NET/1 ";
const LOG_PREFIX: &str = "VKEY_LOG/1 ";
const PING_PREFIX: &str = "VKEY_PING/1 ";
const REC_PREFIX: &str = "VKEY_REC/1 ";
const ASR_PREFIX: &str = "VKEY_ASR/1 ";
const SENSOR_PREFIX: &str = "VKEY_SENSOR/1 ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialEvent {
    pub sequence: u32,
    pub input: InputId,
    pub gap_missed: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AsrAdmission {
    #[default]
    None,
    Start,
    Fail,
    Empty,
    Admitted,
    Duplicate,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrDone {
    pub seq: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AsrOutcome {
    pub admission: AsrAdmission,
    pub seq: Option<u32>,
    pub done: Option<AsrDone>,
}

pub trait EventSource: Send {
    fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError>;
    fn write_line(&mut self, _line: &str) -> Result<(), SerialError> {
        Err(SerialError::Unavailable)
    }
    fn last_network_status(&self) -> Option<NetworkStatus> {
        None
    }
    fn take_asr_outcome(&mut self) -> AsrOutcome {
        AsrOutcome::default()
    }
    fn close(&mut self) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialError {
    Unavailable,
    Read,
    Write,
}
impl Display for SerialError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "serial source is unavailable",
            Self::Read => "serial source read failed",
            Self::Write => "serial source write failed",
        })
    }
}
impl std::error::Error for SerialError {}

/// Shared VKEY decoder for COM and Board C HID. Sequence + ASR admission live
/// here so both transports cannot drift into a second decoder.
#[derive(Default)]
pub(crate) struct LinkDecoder {
    tracker: SequenceTracker,
    net_tracker: SequenceTracker,
    asr_tracker: SequenceTracker,
    last_network: Option<NetworkStatus>,
    last_asr: AsrOutcome,
}

/// The production COM source is retained for tests and leftover fixtures.
/// Formal Board C traffic uses `UsbLinkSource` and this same `LinkDecoder`.
pub struct SerialPortSource {
    port: Box<dyn serialport::SerialPort>,
    buffered: BoundedLineBuffer,
    decoder: LinkDecoder,
}

impl SerialPortSource {
    pub fn available_ports() -> Result<Vec<String>, SerialError> {
        serialport::available_ports()
            .map(|ports| ports.into_iter().map(|port| port.port_name).collect())
            .map_err(|_| SerialError::Unavailable)
    }

    pub fn open(port_name: &str, baud: u32) -> Result<Self, SerialError> {
        if port_name.trim().is_empty() || baud == 0 {
            return Err(SerialError::Unavailable);
        }
        let port = serialport::new(port_name, baud)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|_| SerialError::Unavailable)?;
        Ok(Self {
            port,
            buffered: BoundedLineBuffer::default(),
            decoder: LinkDecoder::default(),
        })
    }
}

impl EventSource for SerialPortSource {
    fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
        // One hardware read per tick so a noisy device or a hung COM reset
        // cannot keep the companion mutex for the whole Wi-Fi join.
        let mut did_read = false;
        loop {
            if let Some(line) = self.buffered.push(&[]) {
                match self.decoder.accept_decoded(&line) {
                    AcceptAction::Input(event) => return Ok(Some(event)),
                    AcceptAction::Yield => return Ok(None),
                    AcceptAction::Continue => continue,
                }
            }
            if did_read {
                return Ok(None);
            }
            did_read = true;
            let mut bytes = [0_u8; 128];
            match self.port.read(&mut bytes) {
                Ok(0) => return Ok(None),
                Ok(count) => {
                    let Some(line) = self.buffered.push(&bytes[..count]) else {
                        return Ok(None);
                    };
                    match self.decoder.accept_decoded(&line) {
                        AcceptAction::Input(event) => return Ok(Some(event)),
                        AcceptAction::Yield => return Ok(None),
                        AcceptAction::Continue => continue,
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                Err(_) => return Err(SerialError::Read),
            }
        }
    }

    fn write_line(&mut self, line: &str) -> Result<(), SerialError> {
        self.port
            .write_all(line.as_bytes())
            .map_err(|_| SerialError::Write)?;
        if !line.ends_with('\n') {
            self.port.write_all(b"\n").map_err(|_| SerialError::Write)?;
        }
        self.port.flush().map_err(|_| SerialError::Write)
    }

    fn last_network_status(&self) -> Option<NetworkStatus> {
        self.decoder.last_network_status()
    }

    fn take_asr_outcome(&mut self) -> AsrOutcome {
        self.decoder.take_asr_outcome()
    }

    fn close(&mut self) {
        let _ = self.port.clear(serialport::ClearBuffer::All);
        self.buffered.clear();
    }
}

#[derive(Debug)]
pub(crate) enum AcceptAction {
    Input(SerialEvent),
    Yield,
    Continue,
}

impl LinkDecoder {
    pub(crate) fn last_network_status(&self) -> Option<NetworkStatus> {
        self.last_network.clone()
    }

    pub(crate) fn take_asr_outcome(&mut self) -> AsrOutcome {
        std::mem::take(&mut self.last_asr)
    }

    pub(crate) fn accept_decoded(&mut self, line: &[u8]) -> AcceptAction {
        let Some(decoded) = decode_record(line) else {
            return AcceptAction::Continue;
        };
        match decoded {
            DecodedLine::Input(event) => {
                let outcome = self.tracker.observe(event.sequence);
                match accept_sequence(event, outcome) {
                    Some(event) => AcceptAction::Input(event),
                    None => AcceptAction::Continue,
                }
            }
            DecodedLine::Network { sequence, status } => {
                if !matches!(
                    self.net_tracker.observe(sequence),
                    SequenceOutcome::DuplicateOrBackward
                ) {
                    self.last_network = Some(merge_network(self.last_network.as_ref(), status));
                }
                AcceptAction::Continue
            }
            DecodedLine::Log { message } => {
                if let Some(current) = self.last_network.as_mut() {
                    current.last_log = Some(message);
                } else {
                    self.last_network = Some(NetworkStatus {
                        last_log: Some(message),
                        ..NetworkStatus::default()
                    });
                }
                AcceptAction::Continue
            }
            DecodedLine::Ping {
                host,
                ok,
                ms,
                lost,
                sent,
            } => {
                let mut status = self.last_network.clone().unwrap_or_default();
                status.ping_host = Some(host);
                status.ping_ok = Some(ok);
                status.ping_ms = Some(ms);
                status.ping_lost = Some(lost);
                status.ping_sent = Some(sent);
                self.last_network = Some(status);
                AcceptAction::Continue
            }
            DecodedLine::Rec {
                state,
                ms,
                samples,
                rms,
                peak,
                silence,
                reason,
            } => {
                let mut status = self.last_network.clone().unwrap_or_default();
                status.rec_state = Some(state);
                status.rec_ms = Some(ms);
                status.rec_samples = Some(samples);
                status.rec_rms = Some(rms);
                status.rec_peak = Some(peak);
                status.rec_silence = silence;
                status.rec_reason = reason;
                self.last_network = Some(status);
                AcceptAction::Continue
            }
            DecodedLine::Asr {
                sequence,
                state,
                text,
                reason,
            } => {
                let outcome =
                    admit_asr_record(&mut self.asr_tracker, sequence, &state, text.clone());
                self.last_asr = outcome.clone();
                if outcome.admission != AsrAdmission::Duplicate {
                    self.last_network =
                        Some(apply_asr(self.last_network.clone(), state, text, reason));
                }
                if outcome.done.is_some() {
                    AcceptAction::Yield
                } else {
                    AcceptAction::Continue
                }
            }
            DecodedLine::Sensor {
                pir,
                dist_mm,
                state,
            } => {
                let mut status = self.last_network.clone().unwrap_or_default();
                status.pir = Some(pir);
                if dist_mm.is_some() {
                    status.tof_mm = dist_mm;
                }
                status.sensor_state = Some(state);
                self.last_network = Some(status);
                AcceptAction::Continue
            }
        }
    }
}

fn accept_sequence(mut event: SerialEvent, outcome: SequenceOutcome) -> Option<SerialEvent> {
    match outcome {
        SequenceOutcome::DuplicateOrBackward => None,
        SequenceOutcome::Accepted => Some(event),
        SequenceOutcome::Gap { missed } => {
            event.gap_missed = Some(missed);
            Some(event)
        }
    }
}

#[derive(Default)]
pub struct BoundedLineBuffer {
    buffered: Vec<u8>,
    ready: VecDeque<Vec<u8>>,
    discarding_overlong: bool,
}
impl BoundedLineBuffer {
    pub fn push(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        for byte in bytes {
            if self.discarding_overlong {
                if *byte == b'\n' {
                    self.discarding_overlong = false;
                }
                continue;
            }
            if self.buffered.len() == MAX_LINE_BYTES {
                self.buffered.clear();
                self.discarding_overlong = *byte != b'\n';
                continue;
            }
            self.buffered.push(*byte);
            if *byte == b'\n' {
                self.ready.push_back(std::mem::take(&mut self.buffered));
            }
        }
        self.ready.pop_front()
    }
    pub fn clear(&mut self) {
        self.buffered.clear();
        self.ready.clear();
        self.discarding_overlong = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceOutcome {
    Accepted,
    Gap { missed: u32 },
    DuplicateOrBackward,
}
#[derive(Default)]
pub struct SequenceTracker {
    last: Option<u32>,
}
impl SequenceTracker {
    pub fn observe(&mut self, sequence: u32) -> SequenceOutcome {
        let outcome = match self.last {
            None => SequenceOutcome::Accepted,
            Some(last) if sequence <= last => SequenceOutcome::DuplicateOrBackward,
            Some(last) if sequence > last.saturating_add(1) => SequenceOutcome::Gap {
                missed: sequence - last - 1,
            },
            Some(_) => SequenceOutcome::Accepted,
        };
        if !matches!(outcome, SequenceOutcome::DuplicateOrBackward) {
            self.last = Some(sequence);
        }
        outcome
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    seq: u32,
    input: InputId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NetRecord {
    seq: u32,
    state: NetworkState,
    ssid: String,
    ip: String,
    rssi: i32,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LogRecord {
    #[allow(dead_code)]
    seq: u32,
    msg: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PingRecord {
    #[allow(dead_code)]
    seq: u32,
    host: String,
    ok: bool,
    ms: u32,
    lost: u32,
    sent: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AsrRecord {
    seq: u32,
    state: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SensorRecord {
    #[allow(dead_code)]
    seq: u32,
    pir: bool,
    #[serde(default)]
    dist_mm: Option<u32>,
    state: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecRecord {
    #[allow(dead_code)]
    seq: u32,
    state: String,
    ms: u32,
    samples: u32,
    rms: u32,
    peak: u32,
    #[serde(default)]
    silence: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedLine {
    Input(SerialEvent),
    Network {
        sequence: u32,
        status: NetworkStatus,
    },
    Log {
        message: String,
    },
    Ping {
        host: String,
        ok: bool,
        ms: u32,
        lost: u32,
        sent: u32,
    },
    Rec {
        state: String,
        ms: u32,
        samples: u32,
        rms: u32,
        peak: u32,
        silence: Option<bool>,
        reason: Option<String>,
    },
    Asr {
        sequence: u32,
        state: String,
        text: Option<String>,
        reason: Option<String>,
    },
    Sensor {
        pir: bool,
        dist_mm: Option<u32>,
        state: String,
    },
}

fn merge_network(previous: Option<&NetworkStatus>, mut status: NetworkStatus) -> NetworkStatus {
    if let Some(previous) = previous {
        status.ping_host = previous.ping_host.clone();
        status.ping_ok = previous.ping_ok;
        status.ping_ms = previous.ping_ms;
        status.ping_lost = previous.ping_lost;
        status.ping_sent = previous.ping_sent;
        status.last_log = previous.last_log.clone();
        status.rec_state = previous.rec_state.clone();
        status.rec_ms = previous.rec_ms;
        status.rec_samples = previous.rec_samples;
        status.rec_rms = previous.rec_rms;
        status.rec_peak = previous.rec_peak;
        status.rec_silence = previous.rec_silence;
        status.rec_reason = previous.rec_reason.clone();
        status.asr_state = previous.asr_state.clone();
        status.asr_text = previous.asr_text.clone();
        status.asr_reason = previous.asr_reason.clone();
        status.pir = previous.pir;
        status.tof_mm = previous.tof_mm;
        status.sensor_state = previous.sensor_state.clone();
        let extra = u32::from(status.state == NetworkState::Connected);
        status.beats = Some(previous.beats.unwrap_or(0).saturating_add(extra));
    } else if status.state == NetworkState::Connected {
        status.beats = Some(1);
    }
    status
}

pub fn decode_record(raw: &[u8]) -> Option<DecodedLine> {
    if raw.len() > MAX_LINE_BYTES {
        return None;
    }
    let line = std::str::from_utf8(raw)
        .ok()?
        .trim_end_matches(['\r', '\n']);
    if let Some(json) = line.strip_prefix(PREFIX) {
        let record = serde_json::from_str::<Record>(json).ok()?;
        return Some(DecodedLine::Input(SerialEvent {
            sequence: record.seq,
            input: record.input,
            gap_missed: None,
        }));
    }
    if let Some(json) = line.strip_prefix(NET_PREFIX) {
        let record = serde_json::from_str::<NetRecord>(json).ok()?;
        return Some(DecodedLine::Network {
            sequence: record.seq,
            status: NetworkStatus {
                state: record.state,
                ssid: record.ssid,
                ip: record.ip,
                rssi: (record.rssi != 0).then_some(record.rssi),
                reason: record.reason.filter(|reason| !reason.is_empty()),
                ..NetworkStatus::default()
            },
        });
    }
    if let Some(json) = line.strip_prefix(LOG_PREFIX) {
        let record = serde_json::from_str::<LogRecord>(json).ok()?;
        return Some(DecodedLine::Log {
            message: record.msg,
        });
    }
    if let Some(json) = line.strip_prefix(PING_PREFIX) {
        let record = serde_json::from_str::<PingRecord>(json).ok()?;
        return Some(DecodedLine::Ping {
            host: record.host,
            ok: record.ok,
            ms: record.ms,
            lost: record.lost,
            sent: record.sent,
        });
    }
    if let Some(json) = line.strip_prefix(REC_PREFIX) {
        let record = serde_json::from_str::<RecRecord>(json).ok()?;
        return Some(DecodedLine::Rec {
            state: record.state,
            ms: record.ms,
            samples: record.samples,
            rms: record.rms,
            peak: record.peak,
            silence: record.silence,
            reason: record.reason.filter(|reason| !reason.is_empty()),
        });
    }
    if let Some(json) = line.strip_prefix(ASR_PREFIX) {
        let record = serde_json::from_str::<AsrRecord>(json).ok()?;
        return Some(DecodedLine::Asr {
            sequence: record.seq,
            state: record.state,
            text: record
                .text
                .map(|text| text.trim().to_string())
                .filter(|text| !text.is_empty()),
            reason: record.reason.filter(|reason| !reason.is_empty()),
        });
    }
    if let Some(json) = line.strip_prefix(SENSOR_PREFIX) {
        let record = serde_json::from_str::<SensorRecord>(json).ok()?;
        return Some(DecodedLine::Sensor {
            pir: record.pir,
            dist_mm: record.dist_mm,
            state: record.state,
        });
    }
    None
}

fn apply_asr(
    previous: Option<NetworkStatus>,
    state: String,
    text: Option<String>,
    reason: Option<String>,
) -> NetworkStatus {
    let mut status = previous.unwrap_or_default();
    status.asr_state = Some(state);
    if text.is_some() {
        status.asr_text = text;
    }
    status.asr_reason = reason;
    status
}

pub fn admit_asr_record(
    tracker: &mut SequenceTracker,
    seq: u32,
    state: &str,
    text: Option<String>,
) -> AsrOutcome {
    if matches!(tracker.observe(seq), SequenceOutcome::DuplicateOrBackward) {
        return AsrOutcome {
            admission: AsrAdmission::Duplicate,
            seq: Some(seq),
            done: None,
        };
    }
    match state {
        "START" => AsrOutcome {
            admission: AsrAdmission::Start,
            seq: Some(seq),
            done: None,
        },
        "FAIL" => AsrOutcome {
            admission: AsrAdmission::Fail,
            seq: Some(seq),
            done: None,
        },
        "DONE" => match text
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            Some(text) => AsrOutcome {
                admission: AsrAdmission::Admitted,
                seq: Some(seq),
                done: Some(AsrDone { seq, text }),
            },
            None => AsrOutcome {
                admission: AsrAdmission::Empty,
                seq: Some(seq),
                done: None,
            },
        },
        _ => AsrOutcome {
            admission: AsrAdmission::None,
            seq: Some(seq),
            done: None,
        },
    }
}

#[cfg(test)]
pub fn decode_line(raw: &[u8]) -> Option<SerialEvent> {
    match decode_record(raw)? {
        DecodedLine::Input(event) => Some(event),
        DecodedLine::Network { .. }
        | DecodedLine::Log { .. }
        | DecodedLine::Ping { .. }
        | DecodedLine::Rec { .. }
        | DecodedLine::Asr { .. }
        | DecodedLine::Sensor { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_decoder_rejects_malformed_overlong_and_unknown_fields() {
        assert_eq!(
            decode_line(br#"VKEY_INPUT/1 {"seq":1,"input":"ENCODER_CW"}"#)
                .unwrap()
                .input,
            InputId::EncoderCw
        );
        assert_eq!(
            decode_line(br#"VKEY_INPUT/1 {"seq":4,"input":"BUTTON_A"}"#)
                .unwrap()
                .input,
            InputId::ButtonA
        );
        assert_eq!(
            decode_line(br#"VKEY_INPUT/1 {"seq":5,"input":"BUTTON_B"}"#)
                .unwrap()
                .input,
            InputId::ButtonB
        );
        assert!(
            decode_line(br#"VKEY_INPUT/1 {"seq":1,"input":"ENCODER_CW","extra":true}"#).is_none()
        );
        assert!(decode_line(&vec![b'x'; MAX_LINE_BYTES + 1]).is_none());
        assert!(decode_line(b"VKEY_INPUT/2 {}").is_none());
        assert!(decode_line(
            br#"VKEY_NET/1 {"seq":1,"state":"CONNECTED","ssid":"cafe","ip":"10.0.0.8","rssi":-40}"#
        )
        .is_none());
        let DecodedLine::Network { status, .. } = decode_record(
            br#"VKEY_NET/1 {"seq":1,"state":"CONNECTED","ssid":"cafe","ip":"10.0.0.8","rssi":-40}"#,
        )
        .unwrap() else {
            panic!("expected network record");
        };
        assert_eq!(status.state, NetworkState::Connected);
        assert_eq!(status.ip, "10.0.0.8");
        assert_eq!(status.rssi, Some(-40));
        assert_eq!(status.reason, None);
        let DecodedLine::Network { status, .. } = decode_record(
            br#"VKEY_NET/1 {"seq":2,"state":"FAILED","ssid":"Home-5G","ip":"","rssi":0,"reason":"BAND"}"#,
        )
        .unwrap() else {
            panic!("expected failed network record");
        };
        assert_eq!(status.state, NetworkState::Failed);
        assert_eq!(status.reason.as_deref(), Some("BAND"));
        let DecodedLine::Ping { host, ok, ms, .. } = decode_record(
            br#"VKEY_PING/1 {"seq":4,"host":"8.8.8.8","ok":true,"ms":18,"lost":0,"sent":3}"#,
        )
        .unwrap() else {
            panic!("expected ping record");
        };
        assert_eq!(host, "8.8.8.8");
        assert!(ok);
        assert_eq!(ms, 18);
        let DecodedLine::Rec {
            state,
            rms,
            silence,
            ..
        } = decode_record(
            br#"VKEY_REC/1 {"seq":3,"state":"DONE","ms":1200,"samples":19200,"rms":800,"peak":9000,"silence":false}"#,
        )
        .unwrap() else {
            panic!("expected rec record");
        };
        assert_eq!(state, "DONE");
        assert_eq!(rms, 800);
        assert_eq!(silence, Some(false));
        assert!(decode_line(
            br#"VKEY_REC/1 {"seq":3,"state":"DONE","ms":1200,"samples":19200,"rms":800,"peak":9000,"silence":false}"#
        )
        .is_none());
        let DecodedLine::Asr { state, text, .. } =
            decode_record(r#"VKEY_ASR/1 {"seq":2,"state":"DONE","text":"hello asr"}"#.as_bytes())
                .unwrap()
        else {
            panic!("expected asr record");
        };
        assert_eq!(state, "DONE");
        assert_eq!(text.as_deref(), Some("hello asr"));
        let DecodedLine::Asr { state, reason, .. } =
            decode_record(r#"VKEY_ASR/1 {"seq":3,"state":"FAIL","reason":"CANCEL"}"#.as_bytes())
                .unwrap()
        else {
            panic!("expected asr cancel");
        };
        assert_eq!(state, "FAIL");
        assert_eq!(reason.as_deref(), Some("CANCEL"));
        let DecodedLine::Sensor {
            pir,
            dist_mm,
            state,
        } = decode_record(br#"VKEY_SENSOR/1 {"seq":1,"pir":true,"state":"OK","distMm":312}"#)
            .unwrap()
        else {
            panic!("expected sensor record");
        };
        assert!(pir);
        assert_eq!(dist_mm, Some(312));
        assert_eq!(state, "OK");
        assert!(decode_line(br#"VKEY_SENSOR/1 {"seq":1,"pir":false,"state":"TOF"}"#).is_none());
    }

    #[test]
    fn asr_start_and_cancel_keep_previous_text() {
        let done = apply_asr(None, "DONE".into(), Some("今天天气不错".into()), None);
        assert_eq!(done.asr_text.as_deref(), Some("今天天气不错"));
        let start = apply_asr(Some(done), "START".into(), None, None);
        assert_eq!(start.asr_state.as_deref(), Some("START"));
        assert_eq!(start.asr_text.as_deref(), Some("今天天气不错"));
        let fail = apply_asr(Some(start), "FAIL".into(), None, Some("CANCEL".into()));
        assert_eq!(fail.asr_state.as_deref(), Some("FAIL"));
        assert_eq!(fail.asr_reason.as_deref(), Some("CANCEL"));
        assert_eq!(fail.asr_text.as_deref(), Some("今天天气不错"));
    }

    #[test]
    fn merge_network_preserves_sensor_telemetry() {
        let previous = NetworkStatus {
            pir: Some(true),
            tof_mm: Some(312),
            sensor_state: Some("OK".into()),
            asr_text: Some("keep me".into()),
            ..NetworkStatus::default()
        };
        let merged = merge_network(
            Some(&previous),
            NetworkStatus {
                state: NetworkState::Connected,
                ssid: "cafe".into(),
                ip: "10.0.0.8".into(),
                ..NetworkStatus::default()
            },
        );
        assert_eq!(merged.pir, Some(true));
        assert_eq!(merged.tof_mm, Some(312));
        assert_eq!(merged.sensor_state.as_deref(), Some("OK"));
        assert_eq!(merged.asr_text.as_deref(), Some("keep me"));
    }
    #[test]
    fn tracker_reports_gaps_and_rejects_duplicates() {
        let mut tracker = SequenceTracker::default();
        assert_eq!(tracker.observe(4), SequenceOutcome::Accepted);
        assert_eq!(tracker.observe(7), SequenceOutcome::Gap { missed: 2 });
        assert_eq!(tracker.observe(7), SequenceOutcome::DuplicateOrBackward);
    }
    #[test]
    fn sequence_gap_is_attached_to_the_current_valid_event() {
        let event = SerialEvent {
            sequence: 7,
            input: InputId::EncoderCw,
            gap_missed: None,
        };
        assert_eq!(
            accept_sequence(event.clone(), SequenceOutcome::DuplicateOrBackward),
            None
        );
        assert_eq!(
            accept_sequence(event, SequenceOutcome::Gap { missed: 2 })
                .unwrap()
                .gap_missed,
            Some(2)
        );
    }
    #[test]
    fn bounded_buffer_resynchronizes_after_overlong_line_and_preserves_following_line() {
        let mut buffer = BoundedLineBuffer::default();
        let mut bytes = vec![b'x'; MAX_LINE_BYTES + 1];
        bytes.extend_from_slice(b"\nVKEY_INPUT/1 {\"seq\":2,\"input\":\"ENCODER_PRESS\"}\n");
        let line = buffer.push(&bytes).unwrap();
        assert_eq!(decode_line(&line).unwrap().input, InputId::EncoderPress);
    }

    #[test]
    fn bounded_buffer_returns_every_complete_line_from_one_read_chunk() {
        let mut buffer = BoundedLineBuffer::default();
        let first = buffer
            .push(b"VKEY_INPUT/1 {\"seq\":1,\"input\":\"ENCODER_CW\"}\nVKEY_INPUT/1 {\"seq\":2,\"input\":\"ENCODER_CCW\"}\n")
            .unwrap();
        let second = buffer.push(&[]).unwrap();
        assert_eq!(decode_line(&first).unwrap().sequence, 1);
        assert_eq!(decode_line(&second).unwrap().sequence, 2);
        assert!(buffer.push(&[]).is_none());
    }

    #[test]
    fn asr_admission_is_exactly_once_per_sequence() {
        let mut tracker = SequenceTracker::default();
        let first = admit_asr_record(&mut tracker, 2, "DONE", Some("hello".into()));
        assert_eq!(first.admission, AsrAdmission::Admitted);
        assert_eq!(
            first.done.as_ref().map(|done| done.text.as_str()),
            Some("hello")
        );
        let duplicate = admit_asr_record(&mut tracker, 2, "DONE", Some("hello".into()));
        assert_eq!(duplicate.admission, AsrAdmission::Duplicate);
        assert!(duplicate.done.is_none());
        let backward = admit_asr_record(&mut tracker, 1, "DONE", Some("older".into()));
        assert_eq!(backward.admission, AsrAdmission::Duplicate);
        let same_text = admit_asr_record(&mut tracker, 3, "DONE", Some("hello".into()));
        assert_eq!(same_text.admission, AsrAdmission::Admitted);
        assert_eq!(same_text.done.as_ref().map(|done| done.seq), Some(3));
    }

    #[test]
    fn asr_start_fail_and_empty_done_do_not_admit_agent() {
        let mut tracker = SequenceTracker::default();
        assert_eq!(
            admit_asr_record(&mut tracker, 1, "START", None).admission,
            AsrAdmission::Start
        );
        assert_eq!(
            admit_asr_record(&mut tracker, 2, "FAIL", None).admission,
            AsrAdmission::Fail
        );
        assert_eq!(
            admit_asr_record(&mut tracker, 3, "DONE", None).admission,
            AsrAdmission::Empty
        );
        assert_eq!(
            admit_asr_record(&mut tracker, 4, "DONE", Some(String::new())).admission,
            AsrAdmission::Empty
        );
        assert_eq!(
            admit_asr_record(&mut tracker, 5, "DONE", Some("   \t".into())).admission,
            AsrAdmission::Empty
        );
        let cancel = admit_asr_record(&mut tracker, 6, "FAIL", Some("CANCEL".into()));
        assert_eq!(cancel.admission, AsrAdmission::Fail);
        assert!(cancel.done.is_none());
    }

    #[test]
    fn decoder_keeps_asr_sequence_and_ignores_invalid_overlong_lines() {
        let DecodedLine::Asr {
            sequence,
            state,
            text,
            ..
        } = decode_record(r#"VKEY_ASR/1 {"seq":7,"state":"DONE","text":"hello asr"}"#.as_bytes())
            .unwrap()
        else {
            panic!("expected asr record");
        };
        assert_eq!(sequence, 7);
        assert_eq!(state, "DONE");
        assert_eq!(text.as_deref(), Some("hello asr"));
        let DecodedLine::Asr {
            text: whitespace, ..
        } = decode_record(r#"VKEY_ASR/1 {"seq":8,"state":"DONE","text":"   "}"#.as_bytes())
            .unwrap()
        else {
            panic!("expected whitespace asr record");
        };
        assert_eq!(whitespace, None);
        assert!(decode_record(&vec![b'x'; MAX_LINE_BYTES + 1]).is_none());
        assert!(decode_record(b"not-a-protocol-line\n").is_none());
        assert!(decode_record(br#"VKEY_ASR/1 {"seq":1,"state":"DONE","extra":true}"#).is_none());
    }
}
