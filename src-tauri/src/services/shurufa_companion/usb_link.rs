pub const USB_VID: u16 = 0x303A;
pub const USB_PID: u16 = 0x82D0;
pub const USB_LINK_ID: &str = "usb:ventured";
pub const USB_LINK_BAUD: u32 = 115200;

const HID_REPORT_LEN: usize = 64;
const HID_PAYLOAD_MAX: usize = 63;

#[cfg(target_os = "windows")]
mod host {
    use hidapi::{HidApi, HidDevice};

    use super::{
        hid_pack, hid_read_is_idle, hid_unpack, HID_PAYLOAD_MAX, HID_REPORT_LEN, USB_PID, USB_VID,
    };
    use crate::services::shurufa_companion::serial::{
        AcceptAction, BoundedLineBuffer, EventSource, LinkDecoder, SerialError, SerialEvent,
    };

    pub struct UsbLinkSource {
        _api: HidApi,
        device: HidDevice,
        buffered: BoundedLineBuffer,
        decoder: LinkDecoder,
    }

    impl UsbLinkSource {
        pub fn present() -> bool {
            HidApi::new()
                .ok()
                .map(|api| {
                    api.device_list().any(|device| {
                        device.vendor_id() == USB_VID && device.product_id() == USB_PID
                    })
                })
                .unwrap_or(false)
        }

        pub fn open() -> Result<Self, SerialError> {
            let api = HidApi::new().map_err(|_| SerialError::Unavailable)?;
            let device = api
                .open(USB_VID, USB_PID)
                .map_err(|_| SerialError::Unavailable)?;
            let _ = device.set_blocking_mode(false);
            Ok(Self {
                _api: api,
                device,
                buffered: BoundedLineBuffer::default(),
                decoder: LinkDecoder::default(),
            })
        }
    }

    impl EventSource for UsbLinkSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            // Drain a bounded burst like VentureD so split VKEY_PING/REC/INPUT
            // frames are not dropped, but stop after 24 reads so Apply/capture
            // can still take the companion mutex.
            const MAX_READS: u8 = 24;
            let mut reads = 0_u8;
            loop {
                if let Some(line) = self.buffered.push(&[]) {
                    match self.decoder.accept_decoded(&line) {
                        AcceptAction::Input(event) => return Ok(Some(event)),
                        AcceptAction::Yield => return Ok(None),
                        AcceptAction::Continue => continue,
                    }
                }
                if reads >= MAX_READS {
                    return Ok(None);
                }
                reads += 1;
                let mut report = [0_u8; HID_REPORT_LEN + 1];
                match self.device.read_timeout(&mut report, 10) {
                    Ok(0) => return Ok(None),
                    Ok(count) => {
                        let mut payload = [0_u8; HID_PAYLOAD_MAX];
                        let n = hid_unpack(&report[..count], &mut payload);
                        if n == 0 {
                            continue;
                        }
                        let Some(line) = self.buffered.push(&payload[..n]) else {
                            continue;
                        };
                        match self.decoder.accept_decoded(&line) {
                            AcceptAction::Input(event) => return Ok(Some(event)),
                            AcceptAction::Yield => return Ok(None),
                            AcceptAction::Continue => continue,
                        }
                    }
                    Err(error) if hid_read_is_idle(&error.to_string()) => return Ok(None),
                    Err(_) => return Err(SerialError::Read),
                }
            }
        }

        fn write_line(&mut self, line: &str) -> Result<(), SerialError> {
            let mut bytes = line.as_bytes().to_vec();
            if !line.ends_with('\n') {
                bytes.push(b'\n');
            }
            let mut rest = bytes.as_slice();
            while !rest.is_empty() {
                let mut report = [0_u8; HID_REPORT_LEN];
                let n = hid_pack(&mut report, rest);
                if n == 0 {
                    return Err(SerialError::Write);
                }
                write_hid_report(&self.device, &report)?;
                rest = &rest[n..];
            }
            Ok(())
        }

        fn last_network_status(
            &self,
        ) -> Option<crate::services::shurufa_companion::network::NetworkStatus> {
            self.decoder.last_network_status()
        }

        fn take_asr_outcome(&mut self) -> crate::services::shurufa_companion::serial::AsrOutcome {
            self.decoder.take_asr_outcome()
        }

        fn close(&mut self) {
            self.buffered.clear();
        }
    }

    fn write_hid_report(
        device: &HidDevice,
        report: &[u8; HID_REPORT_LEN],
    ) -> Result<(), SerialError> {
        let mut framed = [0_u8; HID_REPORT_LEN + 1];
        framed[1..].copy_from_slice(report);
        if device.write(&framed).is_ok() {
            return Ok(());
        }
        device
            .write(report)
            .map(|_| ())
            .map_err(|_| SerialError::Write)
    }
}

#[cfg(not(target_os = "windows"))]
mod host {
    use crate::services::shurufa_companion::serial::{EventSource, SerialError, SerialEvent};

    pub struct UsbLinkSource;

    impl UsbLinkSource {
        pub fn present() -> bool {
            false
        }

        pub fn open() -> Result<Self, SerialError> {
            Err(SerialError::Unavailable)
        }
    }

    impl EventSource for UsbLinkSource {
        fn poll_event(&mut self) -> Result<Option<SerialEvent>, SerialError> {
            Err(SerialError::Unavailable)
        }
    }
}

pub use host::UsbLinkSource;

#[allow(dead_code)]
fn hid_read_is_disconnect(message: &str) -> bool {
    message.contains("not connected")
        || message.contains("device not found")
        || message.contains("no such device")
        || message.contains("access denied")
        || message.contains("permission")
        || message.contains("broken pipe")
        || message.contains("device has been removed")
        || message.contains("device disconnected")
        || message.contains("0x0000048f")
        || message.contains("0x48f")
        || message.contains("设备没有连接")
        || message.contains("设备未连接")
}

#[allow(dead_code)]
fn hid_read_is_idle(message: &str) -> bool {
    let message = message.trim().to_ascii_lowercase();
    if hid_read_is_disconnect(&message) {
        return false;
    }
    message.is_empty()
        || message.contains("hid error")
        || message.contains("hidapi error")
        || message.contains("could not get error message")
        || message.contains("timed out")
        || message.contains("timeout")
        || message.contains("would block")
        || message.contains("overlapped")
}

fn hid_pack(report: &mut [u8; HID_REPORT_LEN], src: &[u8]) -> usize {
    if src.is_empty() {
        return 0;
    }
    let chunk = src.len().min(HID_PAYLOAD_MAX);
    report.fill(0);
    report[0] = chunk as u8;
    report[1..1 + chunk].copy_from_slice(&src[..chunk]);
    chunk
}

fn hid_unpack(report: &[u8], dst: &mut [u8]) -> usize {
    if report.is_empty() || dst.is_empty() {
        return 0;
    }
    let mut offset = 0;
    if report.len() >= 2 && report[0] == 0 {
        offset = 1;
    }
    if offset >= report.len() {
        return 0;
    }
    let payload = report[offset] as usize;
    if payload == 0 || payload > HID_PAYLOAD_MAX {
        return 0;
    }
    if offset + 1 + payload > report.len() {
        return 0;
    }
    let n = payload.min(dst.len());
    dst[..n].copy_from_slice(&report[offset + 1..offset + 1 + n]);
    n
}

#[cfg(test)]
use crate::services::shurufa_companion::serial::{AcceptAction, BoundedLineBuffer, LinkDecoder};

#[cfg(test)]
fn feed_hid_payload(
    decoder: &mut LinkDecoder,
    buffered: &mut BoundedLineBuffer,
    src: &[u8],
) -> AcceptAction {
    let mut rest = src;
    let mut last = AcceptAction::Continue;
    while !rest.is_empty() {
        let mut report = [0_u8; HID_REPORT_LEN];
        let packed = hid_pack(&mut report, rest);
        assert_ne!(packed, 0);
        rest = &rest[packed..];
        let mut payload = [0_u8; HID_PAYLOAD_MAX];
        let n = hid_unpack(&report, &mut payload);
        if n == 0 {
            continue;
        }
        if let Some(line) = buffered.push(&payload[..n]) {
            last = decoder.accept_decoded(&line);
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::shurufa_companion::input::InputId;
    use crate::services::shurufa_companion::network::NetworkState;
    use crate::services::shurufa_companion::serial::{admit_asr_record, AsrAdmission};

    #[test]
    fn hid_frame_matches_teensy_style_length_prefix() {
        let src = b"VKEY_INPUT/1 {\"seq\":1,\"input\":\"ENCODER_CW\"}\n";
        let mut report = [0_u8; HID_REPORT_LEN];
        let n = hid_pack(&mut report, src);
        assert_eq!(n, src.len());
        assert_eq!(report[0] as usize, src.len());
        let mut out = [0_u8; HID_PAYLOAD_MAX];
        assert_eq!(hid_unpack(&report, &mut out), src.len());
        assert_eq!(&out[..src.len()], src);

        let mut prefixed = [0_u8; HID_REPORT_LEN + 1];
        prefixed[1..].copy_from_slice(&report);
        assert_eq!(hid_unpack(&prefixed, &mut out), src.len());
        assert_eq!(&out[..src.len()], src);
        assert_eq!(hid_unpack(&[0, 3, b'a'], &mut out), 0);
    }

    #[test]
    fn hid_idle_read_errors_do_not_drop_the_link() {
        assert!(hid_read_is_idle(""));
        assert!(hid_read_is_idle("Hid error"));
        assert!(hid_read_is_idle(
            "hidapi error: (could not get error message)",
        ));
        assert!(hid_read_is_idle("Operation timed out"));
        assert!(hid_read_is_idle("Would block"));
        assert!(!hid_read_is_idle("device not connected"));
        assert!(!hid_read_is_idle(
            "hidapi error: The device has been removed.",
        ));
        assert!(!hid_read_is_idle("Access denied"));
        assert!(!hid_read_is_idle(
            "hidapi error: ReadFile: (0x0000048F) 设备没有连接。",
        ));
    }

    #[test]
    fn hid_pack_caps_payload_at_63_and_rejects_illegal_lengths() {
        let src = [0x41_u8; 80];
        let mut report = [0_u8; HID_REPORT_LEN];
        assert_eq!(hid_pack(&mut report, &src), HID_PAYLOAD_MAX);
        assert_eq!(report[0] as usize, HID_PAYLOAD_MAX);
        assert_eq!(hid_pack(&mut report, &[]), 0);

        let mut out = [0_u8; HID_PAYLOAD_MAX];
        assert_eq!(hid_unpack(&[], &mut out), 0);
        assert_eq!(hid_unpack(&[0], &mut out), 0);
        assert_eq!(hid_unpack(&[64, 1], &mut out), 0);
        assert_eq!(hid_unpack(&[0, 0, 1], &mut out), 0);
        let mut too_long = [0_u8; 2];
        too_long[0] = 2;
        assert_eq!(hid_unpack(&too_long, &mut out), 0);
    }

    #[test]
    fn hid_framed_link_decoder_keeps_input_net_sensor_and_asr_admission() {
        let mut decoder = LinkDecoder::default();
        let mut buffered = BoundedLineBuffer::default();

        let input = feed_hid_payload(
            &mut decoder,
            &mut buffered,
            br#"VKEY_INPUT/1 {"seq":1,"input":"ENCODER_CW"}
"#,
        );
        match input {
            AcceptAction::Input(event) => {
                assert_eq!(event.sequence, 1);
                assert_eq!(event.input, InputId::EncoderCw);
            }
            other => panic!("expected input, got {other:?}"),
        }

        assert!(matches!(
            feed_hid_payload(
                &mut decoder,
                &mut buffered,
                br#"VKEY_NET/1 {"seq":1,"state":"CONNECTED","ssid":"cafe","ip":"10.0.0.8","rssi":-40}
"#,
            ),
            AcceptAction::Continue
        ));
        assert!(matches!(
            feed_hid_payload(
                &mut decoder,
                &mut buffered,
                br#"VKEY_REC/1 {"seq":3,"state":"DONE","ms":1200,"samples":19200,"rms":800,"peak":9000,"silence":false}
"#,
            ),
            AcceptAction::Continue
        ));
        assert!(matches!(
            feed_hid_payload(
                &mut decoder,
                &mut buffered,
                br#"VKEY_SENSOR/1 {"seq":1,"pir":true,"state":"OK","distMm":312}
"#,
            ),
            AcceptAction::Continue
        ));

        let asr = feed_hid_payload(
            &mut decoder,
            &mut buffered,
            br#"VKEY_ASR/1 {"seq":2,"state":"DONE","text":"hello asr"}
"#,
        );
        assert!(matches!(asr, AcceptAction::Yield));
        let outcome = decoder.take_asr_outcome();
        assert_eq!(outcome.admission, AsrAdmission::Admitted);
        assert_eq!(
            outcome.done.as_ref().map(|done| done.text.as_str()),
            Some("hello asr")
        );

        let duplicate = feed_hid_payload(
            &mut decoder,
            &mut buffered,
            br#"VKEY_ASR/1 {"seq":2,"state":"DONE","text":"hello asr"}
"#,
        );
        assert!(matches!(duplicate, AcceptAction::Continue));
        assert_eq!(
            decoder.take_asr_outcome().admission,
            AsrAdmission::Duplicate
        );

        let network = decoder.last_network_status().expect("network status");
        assert_eq!(network.state, NetworkState::Connected);
        assert_eq!(network.ip, "10.0.0.8");
        assert_eq!(network.rec_state.as_deref(), Some("DONE"));
        assert_eq!(network.pir, Some(true));
        assert_eq!(network.tof_mm, Some(312));
        assert_eq!(network.asr_text.as_deref(), Some("hello asr"));
    }

    #[test]
    fn hid_framed_duplicate_asr_seq_does_not_admit_again() {
        let mut tracker = crate::services::shurufa_companion::serial::SequenceTracker::default();
        let first = admit_asr_record(&mut tracker, 4, "DONE", Some("once".into()));
        assert_eq!(first.admission, AsrAdmission::Admitted);
        let replay = admit_asr_record(&mut tracker, 4, "DONE", Some("once".into()));
        assert_eq!(replay.admission, AsrAdmission::Duplicate);
        assert!(replay.done.is_none());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_usb_link_is_fail_closed() {
        assert!(!UsbLinkSource::present());
        assert!(UsbLinkSource::open().is_err());
    }
}
