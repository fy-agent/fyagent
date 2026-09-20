use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) const MAX_BYTES: usize = 64 * 1024;
pub(crate) const MAX_ENTRIES: usize = 32;
pub(crate) const FORMAT: &str = "fyagent-config-pack/v1";
pub(crate) const DRAFT_CATEGORY: &str = "config-pack-draft";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PackApp {
    Claude,
    Codex,
}
impl PackApp {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WireApi {
    Responses,
    Chat,
}
impl WireApi {
    fn as_str(self) -> &'static str {
        match self {
            Self::Responses => "responses",
            Self::Chat => "chat",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortableProvider {
    pub app: PackApp,
    pub name: String,
    pub endpoint: String,
    pub model: String,
    #[serde(deserialize_with = "required_wire_api")]
    pub wire_api: Option<WireApi>,
}
fn required_wire_api<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<WireApi>, D::Error> {
    Option::<WireApi>::deserialize(deserializer)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfigPack {
    pub format: String,
    pub providers: Vec<PortableProvider>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PackError {
    InvalidPack,
    TooLarge,
    UnsafeContent,
    UnsupportedVersion,
    Conflict,
    StalePreview,
    InvalidPreview,
    StorageUnavailable,
    WriteFailed,
    ReadbackFailed,
    FileUnavailable,
    Busy,
}
pub(crate) type Result<T> = std::result::Result<T, PackError>;

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sensitive_text(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    [
        "sk-",
        "sk_",
        "bearer ",
        "secretref",
        "secret_ref",
        "credentialref",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "password",
        "-----begin",
        "eyj",
    ]
    .iter()
    .any(|part| lower.contains(part))
}
pub(crate) fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.trim() == s
        && !sensitive_text(s)
        && s.chars()
            .all(|c| c.is_alphanumeric() || " ._-()".contains(c))
        && !s.contains("..")
}
fn valid_model(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !sensitive_text(s)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:/".contains(&b))
        && !s.starts_with('/')
        && !s.split('/').any(|part| part == ".")
        && !s.contains("..")
        && !s.contains("://")
        && !s.contains(":/")
}
fn valid_endpoint(s: &str) -> bool {
    if s.len() > 2048
        || sensitive_text(s)
        || s.contains(['%', '\\', '$', '~'])
        || s.chars().any(char::is_whitespace)
    {
        return false;
    }
    let Ok(url) = url::Url::parse(s) else {
        return false;
    };
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || s.split('/').any(|p| p == ".." || p == ".")
    {
        return false;
    }
    match url.host() {
        Some(url::Host::Domain(host)) => {
            host.contains('.')
                && host != "localhost"
                && ![".localhost", ".local", ".internal"]
                    .iter()
                    .any(|s| host.ends_with(s))
        }
        Some(url::Host::Ipv4(ip)) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_unspecified()
                && !ip.is_broadcast()
                && !ip.is_multicast()
        }
        Some(url::Host::Ipv6(ip)) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unique_local()
                && !ip.is_unicast_link_local()
                && !ip.is_multicast()
                && ip.to_ipv4().is_none()
        }
        None => false,
    }
}
impl PortableProvider {
    pub(crate) fn validate(&self) -> Result<()> {
        if !valid_name(&self.name) || !valid_model(&self.model) || !valid_endpoint(&self.endpoint) {
            return Err(PackError::UnsafeContent);
        }
        if (self.app == PackApp::Codex) != self.wire_api.is_some() {
            return Err(PackError::InvalidPack);
        }
        Ok(())
    }
    // One closed projection. There is deliberately no Provider/metadata/credential input.
    pub(crate) fn settings(&self) -> Result<Value> {
        self.validate()?;
        match self.app {
            PackApp::Claude => Ok(json!({"env": {
                "ANTHROPIC_BASE_URL": self.endpoint,
                "ANTHROPIC_MODEL": self.model
            }})),
            PackApp::Codex => {
                let q = |s: &str| toml::Value::String(s.to_owned()).to_string();
                let wire = self.wire_api.ok_or(PackError::InvalidPack)?;
                Ok(json!({"auth": {}, "config": format!(
                    "model_provider = \"portable\"\nmodel = {}\n\n[model_providers.portable]\nname = \"Portable connection\"\nbase_url = {}\nwire_api = {}\nrequires_openai_auth = true\n",
                    q(&self.model), q(&self.endpoint), q(wire.as_str())
                )}))
            }
        }
    }
}
impl ConfigPack {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err(PackError::TooLarge);
        }
        let pack: Self = serde_json::from_slice(bytes).map_err(|_| PackError::InvalidPack)?;
        pack.validate()?;
        Ok(pack)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.format != FORMAT {
            return Err(PackError::UnsupportedVersion);
        }
        if self.providers.is_empty() || self.providers.len() > MAX_ENTRIES {
            return Err(PackError::InvalidPack);
        }
        let mut seen = HashSet::new();
        for p in &self.providers {
            p.validate()?;
            if !seen.insert((p.app, &p.name)) {
                return Err(PackError::Conflict);
            }
        }
        Ok(())
    }
    pub(crate) fn text(&self) -> Result<String> {
        self.validate()?;
        let text = serde_json::to_string_pretty(self).map_err(|_| PackError::InvalidPack)?;
        if text.len() > MAX_BYTES {
            return Err(PackError::TooLarge);
        }
        Ok(text)
    }
}

// Known credentials can be arbitrary strings; exclude a projected field if it
// accidentally embeds one, in addition to the portable shape's lexical checks.
fn collect_secrets(value: &Value, sensitive: bool, out: &mut Vec<String>) {
    match value {
        Value::String(s) if sensitive && !s.is_empty() => out.push(s.clone()),
        Value::Object(map) => {
            for (key, v) in map {
                let key = key.to_ascii_lowercase();
                collect_secrets(
                    v,
                    sensitive
                        || [
                            "key",
                            "token",
                            "secret",
                            "credential",
                            "password",
                            "authorization",
                        ]
                        .iter()
                        .any(|s| key.contains(s)),
                    out,
                );
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_secrets(value, sensitive, out);
            }
        }
        _ => {}
    }
}
pub(crate) fn project(app: PackApp, name: &str, settings: &Value) -> Option<PortableProvider> {
    let mut secrets = Vec::new();
    collect_secrets(settings, false, &mut secrets);
    let p = match app {
        PackApp::Claude => PortableProvider {
            app,
            name: name.into(),
            endpoint: settings
                .pointer("/env/ANTHROPIC_BASE_URL")?
                .as_str()?
                .into(),
            model: settings.pointer("/env/ANTHROPIC_MODEL")?.as_str()?.into(),
            wire_api: None,
        },
        PackApp::Codex => {
            let table: toml::Value = toml::from_str(settings.get("config")?.as_str()?).ok()?;
            let selected = table.get("model_provider")?.as_str()?;
            let provider = table.get("model_providers")?.get(selected)?;
            // Header/env key names and literal tokens are never projected.
            let toml_value = serde_json::to_value(&table).ok()?;
            collect_secrets(&toml_value, false, &mut secrets);
            PortableProvider {
                app,
                name: name.into(),
                endpoint: provider.get("base_url")?.as_str()?.into(),
                model: table.get("model")?.as_str()?.into(),
                wire_api: Some(
                    match provider.get("wire_api").and_then(toml::Value::as_str) {
                        Some("responses") | None => WireApi::Responses,
                        Some("chat") => WireApi::Chat,
                        _ => return None,
                    },
                ),
            }
        }
    };
    p.validate().ok()?;
    let fields = [&p.name, &p.endpoint, &p.model];
    if secrets
        .iter()
        .any(|secret| fields.iter().any(|field| field.contains(secret)))
    {
        return None;
    }
    Some(p)
}

#[derive(Clone)]
pub(crate) struct PackStoredProvider {
    pub id: String,
    pub app: PackApp,
    pub name: String,
    pub selection_id: String,
    pub portable: Option<PortableProvider>,
    pub overwrite_allowed: bool,
}
pub(crate) struct PackInventory {
    pub revision: String,
    pub providers: Vec<PackStoredProvider>,
}
#[derive(Clone)]
pub(crate) struct PackDraftWrite {
    pub id: String,
    pub provider: PortableProvider,
    pub overwrite: bool,
}
