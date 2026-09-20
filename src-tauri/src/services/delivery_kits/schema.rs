use std::collections::{BTreeMap, HashSet};

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) const MAX_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const SCHEMA: &str = "fyagent-delivery-kit/v1";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KitError {
    InvalidPackage,
    UnsupportedSchema,
    IncompatibleHost,
    UnsafeContent,
    ContentConflict,
    NotFound,
    PreviewExpired,
    InvalidPreview,
    LibraryUnavailable,
    WriteFailed,
    ReadbackFailed,
    UnsupportedValidator,
    Busy,
}
pub(crate) type Result<T> = std::result::Result<T, KitError>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Manifest {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub title: String,
    pub summary: String,
    pub scenario: Scenario,
    pub compatibility: Compatibility,
    pub provenance: Provenance,
    pub inputs: Vec<Input>,
    pub connections: Vec<Connection>,
    pub credential_slots: Vec<CredentialSlot>,
    pub permissions: Vec<Permission>,
    pub prompts: Vec<ContentRef>,
    pub skills: Vec<ContentRef>,
    pub resources: Vec<Resource>,
    pub fixtures: Vec<Fixture>,
    pub checks: Vec<Check>,
    pub handoff: String,
    pub rollback: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Scenario {
    WeeklyReport,
    KnowledgeSupport,
    BusinessQuery,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Compatibility {
    pub min_fy_agent_version: String,
    pub required_capabilities: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Provenance {
    pub publisher_label: String,
    pub license: String,
    pub source_preset_ids: Vec<String>,
    pub source_recipe_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Input {
    pub id: String,
    pub label: String,
    pub description: String,
    pub required: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Connection {
    pub id: String,
    pub recipe_id: String,
    pub purpose: String,
    pub permission_ids: Vec<String>,
    pub credential_slot_ids: Vec<String>,
    pub optional: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CredentialSlot {
    pub id: String,
    pub purpose: String,
    pub required: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Permission {
    pub id: String,
    pub resource_class: ResourceClass,
    pub access: ReadAccess,
    pub rationale: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResourceClass {
    BusinessTable,
    KnowledgeDocuments,
    Database,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReadAccess {
    Read,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContentRef {
    pub id: String,
    pub resource_id: String,
    pub version: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Resource {
    pub id: String,
    pub media_type: MediaType,
    pub text: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) enum MediaType {
    #[serde(rename = "text/markdown")]
    Markdown,
    #[serde(rename = "application/json")]
    Json,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Fixture {
    pub id: String,
    pub input_resource_id: String,
    pub expected_resource_id: String,
    pub kind: FixtureKind,
    pub provenance: Synthetic,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FixtureKind {
    Positive,
    Negative,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Synthetic {
    Synthetic,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Check {
    pub id: String,
    pub validator: Validator,
    pub fixture_ids: Vec<String>,
    pub description: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Validator {
    WeeklyReportV1,
    ManualReview,
}

// Parsing through Value normally loses duplicate members. Reject them before typed decoding,
// including duplicates nested inside resource JSON. Errors never include the input.
struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bounded JSON")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<UniqueValue>()? {
                    values.push(v.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut map = serde_json::Map::new();
                while let Some((key, v)) = a.next_entry::<String, UniqueValue>()? {
                    if map.contains_key(&key)
                        || matches!(key.as_str(), "__proto__" | "constructor" | "prototype")
                    {
                        return Err(de::Error::custom("invalid member"));
                    }
                    map.insert(key, v.0);
                }
                Ok(UniqueValue(Value::Object(map)))
            }
        }
        d.deserialize_any(UniqueVisitor)
    }
}
pub(crate) fn strict_json(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_BYTES {
        return Err(KitError::InvalidPackage);
    }
    let value: UniqueValue = serde_json::from_slice(bytes).map_err(|_| KitError::InvalidPackage)?;
    fn depth(v: &Value, n: usize) -> bool {
        n <= 32
            && match v {
                Value::Array(a) => a.iter().all(|x| depth(x, n + 1)),
                Value::Object(m) => m.values().all(|x| depth(x, n + 1)),
                Value::String(s) => s.len() <= 256 * 1024,
                _ => true,
            }
    }
    if !depth(&value.0, 0) {
        return Err(KitError::InvalidPackage);
    }
    Ok(value.0)
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn canonical(value: &impl Serialize) -> Result<Vec<u8>> {
    fn sorted(v: Value) -> Value {
        match v {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, sorted(v)))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.into_iter().map(sorted).collect()),
            v => v,
        }
    }
    serde_json::to_vec(&sorted(
        serde_json::to_value(value).map_err(|_| KitError::InvalidPackage)?,
    ))
    .map_err(|_| KitError::InvalidPackage)
}
pub(crate) fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn ids<'a>(values: impl Iterator<Item = &'a str>) -> Result<HashSet<&'a str>> {
    let mut set = HashSet::new();
    for s in values {
        if !valid_id(s) || !set.insert(s) || set.len() > 100 {
            return Err(KitError::InvalidPackage);
        }
    }
    Ok(set)
}
fn version(s: &str) -> bool {
    s.len() <= 32 && semver::Version::parse(s).is_ok_and(|v| v.pre.is_empty() && v.build.is_empty())
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Manifest> {
    let value = strict_json(bytes)?;
    if value.get("schemaVersion").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(KitError::UnsupportedSchema);
    }
    let m: Manifest = serde_json::from_value(value).map_err(|_| KitError::InvalidPackage)?;
    if !valid_id(&m.id) || !version(&m.version) || !version(&m.compatibility.min_fy_agent_version) {
        return Err(KitError::InvalidPackage);
    }
    let encoded = canonical(&m)?;
    let text = std::str::from_utf8(&encoded).map_err(|_| KitError::InvalidPackage)?;
    // This is a rejection aid, never a certification that arbitrary prose has been anonymized.
    let lower = text.to_ascii_lowercase();
    if [
        "sec_",
        "sv_",
        "sk-",
        "-----begin",
        "/users/",
        "bearer ",
        "api_key=",
        "apikey=",
        "password=",
        "secretref",
        "file://",
        "c:\\\\",
    ]
    .iter()
    .any(|s| lower.contains(s))
        || text.contains("\\u0000")
    {
        return Err(KitError::UnsafeContent);
    }
    if m.title.is_empty()
        || m.title.chars().count() > 100
        || m.summary.chars().count() > 1000
        || m.resources.is_empty()
        || m.resources.len() > 100
        || m.fixtures.is_empty()
        || m.checks.is_empty()
        || m.prompts.is_empty()
        || m.skills.is_empty()
    {
        return Err(KitError::InvalidPackage);
    }
    ids(m.inputs.iter().map(|x| x.id.as_str()))?;
    ids(m
        .compatibility
        .required_capabilities
        .iter()
        .map(String::as_str))?;
    ids(m.provenance.source_preset_ids.iter().map(String::as_str))?;
    ids(m.provenance.source_recipe_ids.iter().map(String::as_str))?;
    ids(m.connections.iter().map(|x| x.id.as_str()))?;
    ids(m.prompts.iter().map(|x| x.id.as_str()))?;
    ids(m.skills.iter().map(|x| x.id.as_str()))?;
    ids(m.checks.iter().map(|x| x.id.as_str()))?;
    let resources = ids(m.resources.iter().map(|x| x.id.as_str()))?;
    let fixtures = ids(m.fixtures.iter().map(|x| x.id.as_str()))?;
    let slots = ids(m.credential_slots.iter().map(|x| x.id.as_str()))?;
    let permissions = ids(m.permissions.iter().map(|x| x.id.as_str()))?;
    for r in &m.resources {
        if r.text.len() > 256 * 1024 || r.sha256 != digest(r.text.as_bytes()) {
            return Err(KitError::InvalidPackage);
        }
        if r.media_type == MediaType::Json {
            strict_json(r.text.as_bytes())?;
        }
    }
    for r in m.prompts.iter().chain(&m.skills) {
        if !resources.contains(r.resource_id.as_str())
            || !version(&r.version)
            || m.resources
                .iter()
                .find(|x| x.id == r.resource_id)
                .is_none_or(|x| x.media_type != MediaType::Markdown)
        {
            return Err(KitError::InvalidPackage);
        }
    }
    if !resources.contains(m.handoff.as_str()) || !resources.contains(m.rollback.as_str()) {
        return Err(KitError::InvalidPackage);
    }
    for f in &m.fixtures {
        if !resources.contains(f.input_resource_id.as_str())
            || !resources.contains(f.expected_resource_id.as_str())
        {
            return Err(KitError::InvalidPackage);
        }
    }
    for c in &m.checks {
        ids(c.fixture_ids.iter().map(String::as_str))?;
        if c.fixture_ids.is_empty()
            || c.fixture_ids
                .iter()
                .any(|id| !fixtures.contains(id.as_str()))
        {
            return Err(KitError::InvalidPackage);
        }
    }
    for c in &m.connections {
        ids(c.permission_ids.iter().map(String::as_str))?;
        ids(c.credential_slot_ids.iter().map(String::as_str))?;
        if !valid_id(&c.recipe_id)
            || c.permission_ids
                .iter()
                .any(|id| !permissions.contains(id.as_str()))
            || c.credential_slot_ids
                .iter()
                .any(|id| !slots.contains(id.as_str()))
        {
            return Err(KitError::InvalidPackage);
        }
    }
    Ok(m)
}
pub(crate) fn compatible(m: &Manifest) -> bool {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"));
    let minimum = semver::Version::parse(&m.compatibility.min_fy_agent_version);
    matches!((current,minimum),(Ok(a),Ok(b)) if a>=b)
        && m.compatibility
            .required_capabilities
            .iter()
            .all(|c| matches!(c.as_str(), "text-kit-v1" | "weekly-report-v1"))
}
