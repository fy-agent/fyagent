use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

pub const DEFAULT_CLOUD_MODEL: &str = "XingChenAGI/XingChenASR-V3.2-Ultra";
#[allow(dead_code)]
pub const CLOUD_MODELS: [&str; 1] = ["XingChenAGI/XingChenASR-V3.2-Ultra"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkState {
    #[default]
    Unknown,
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkStatus {
    pub state: NetworkState,
    pub ssid: String,
    pub ip: String,
    pub rssi: Option<i32>,
    pub reason: Option<String>,
    #[serde(default)]
    pub ping_host: Option<String>,
    #[serde(default)]
    pub ping_ok: Option<bool>,
    #[serde(default)]
    pub ping_ms: Option<u32>,
    #[serde(default)]
    pub ping_lost: Option<u32>,
    #[serde(default)]
    pub ping_sent: Option<u32>,
    #[serde(default)]
    pub last_log: Option<String>,
    #[serde(default)]
    pub beats: Option<u32>,
    #[serde(default)]
    pub rec_state: Option<String>,
    #[serde(default)]
    pub rec_ms: Option<u32>,
    #[serde(default)]
    pub rec_samples: Option<u32>,
    #[serde(default)]
    pub rec_rms: Option<u32>,
    #[serde(default)]
    pub rec_peak: Option<u32>,
    #[serde(default)]
    pub rec_silence: Option<bool>,
    #[serde(default)]
    pub rec_reason: Option<String>,
    #[serde(default)]
    pub asr_state: Option<String>,
    #[serde(default)]
    pub asr_text: Option<String>,
    #[serde(default)]
    pub asr_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfigRecord {
    pub seq: u32,
    pub ssid: String,
    pub password: String,
    pub api_key: String,
    pub model: String,
}

impl DeviceConfigRecord {
    pub fn line(&self) -> Result<String, NetworkError> {
        let json = serde_json::to_string(self).map_err(|_| NetworkError::Invalid)?;
        Ok(format!("VKEY_CONFIG/1 {json}\n"))
    }
}

pub fn model_allowed(model: &str) -> bool {
    let trimmed = model.trim();
    trimmed.chars().count() <= 64 && !trimmed.chars().any(char::is_control)
}

pub fn ssid_looks_5g(ssid: &str) -> bool {
    let bytes = ssid.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'5' {
            let mut cursor = index + 1;
            while cursor < bytes.len() && bytes[cursor] == b' ' {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor].eq_ignore_ascii_case(&b'g') {
                cursor += 1;
                if cursor + 1 < bytes.len()
                    && bytes[cursor].eq_ignore_ascii_case(&b'h')
                    && bytes[cursor + 1].eq_ignore_ascii_case(&b'z')
                {
                    cursor += 2;
                }
                if bytes
                    .get(cursor)
                    .is_none_or(|next| !next.is_ascii_alphanumeric())
                {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    Invalid,
    Unavailable,
}
impl Display for NetworkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "device settings are invalid",
            Self::Unavailable => "device link is unavailable",
        })
    }
}
impl std::error::Error for NetworkError {}

#[cfg(test)]
mod tests {
    use super::ssid_looks_5g;

    #[test]
    fn ssid_5g_heuristic_matches_common_hotspot_names() {
        assert!(ssid_looks_5g("Home-5G"));
        assert!(ssid_looks_5g("office_5ghz"));
        assert!(ssid_looks_5g("5G-office"));
        assert!(!ssid_looks_5g("5guys"));
        assert!(!ssid_looks_5g("cafe"));
        assert!(!ssid_looks_5g("channel5"));
    }

    #[test]
    fn config_line_keeps_empty_cloud_fields() {
        let line = super::DeviceConfigRecord {
            seq: 1,
            ssid: "cafe".into(),
            password: "secret".into(),
            api_key: String::new(),
            model: String::new(),
        }
        .line()
        .unwrap();
        assert!(line.contains("\"apiKey\":\"\""));
        assert!(line.contains("\"model\":\"\""));
        assert!(super::model_allowed(""));
        assert!(super::model_allowed("XingChenAGI/XingChenASR-V3.2-Ultra"));
        assert!(super::model_allowed("whisper"));
        assert!(!super::model_allowed(&"x".repeat(65)));
    }
}
