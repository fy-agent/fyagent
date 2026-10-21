//! Portable session logic for the `codex-models` helper action.
//!
//! The helper talks to the current user's `codex app-server` over stdio:
//! `initialize` → `initialized` → `model/list` with cursor pagination. This
//! module owns every request shape, response check, and bound so the native
//! runtime only moves bytes. All limits are chosen so a complete catalog fits
//! one bounded helper protocol frame.

use std::collections::HashSet;
use std::fmt;

use serde_json::{json, Value};

/// Models requested per `model/list` page.
pub const CODEX_MODEL_PAGE_SIZE: u32 = 32;
/// Pages read before the catalog is reported as truncated.
pub const MAX_CODEX_MODEL_PAGES: usize = 4;
/// Models carried in one catalog. Anything beyond is reported as truncated.
pub const MAX_CODEX_MODELS: usize = 64;
pub const MAX_CODEX_MODEL_ID_BYTES: usize = 96;
pub const MAX_CODEX_MODEL_NAME_BYTES: usize = 96;
pub const MAX_CODEX_REASONING_EFFORTS: usize = 8;
pub const MAX_CODEX_REASONING_EFFORT_BYTES: usize = 16;
pub const MAX_CODEX_CLI_VERSION_BYTES: usize = 64;
/// Longest stdout line the helper buffers from app-server.
pub const MAX_CODEX_RPC_LINE_BYTES: usize = 1 << 20;
pub const MAX_CODEX_CURSOR_BYTES: usize = 512;
/// Non-matching stdout lines tolerated while waiting for one response.
pub const MAX_CODEX_RPC_SKIPPED_LINES: usize = 256;

pub const CODEX_INITIALIZE_REQUEST_ID: u64 = 1;
const METHOD_NOT_FOUND: i64 = -32601;

const _: () = assert!(MAX_CODEX_MODEL_PAGES * CODEX_MODEL_PAGE_SIZE as usize >= MAX_CODEX_MODELS);
const _: () = assert!(MAX_CODEX_MODELS <= u8::MAX as usize);
const _: () = assert!(MAX_CODEX_REASONING_EFFORTS <= u8::MAX as usize);
const _: () = assert!(MAX_CODEX_MODEL_ID_BYTES <= u8::MAX as usize);
const _: () = assert!(MAX_CODEX_MODEL_NAME_BYTES <= u8::MAX as usize);
const _: () = assert!(MAX_CODEX_CLI_VERSION_BYTES <= u8::MAX as usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexModelEntry {
    pub id: String,
    pub display_name: Option<String>,
    pub is_default: bool,
    pub reasoning_efforts: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodexModelCatalog {
    /// Version parsed from the app-server `initialize` user agent.
    pub cli_version: Option<String>,
    pub models: Vec<CodexModelEntry>,
    /// The server offered more models or pages than the helper carries.
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexCatalogError {
    /// app-server answered the request with a JSON-RPC error.
    RequestFailed,
    /// app-server does not implement `model/list`.
    MethodUnsupported,
    /// The response broke the documented shape or a helper bound.
    InvalidResponse,
}

impl fmt::Display for CodexCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RequestFailed => "Codex app-server rejected the request",
            Self::MethodUnsupported => "Codex app-server does not support model/list",
            Self::InvalidResponse => "Codex app-server returned an invalid model catalog",
        })
    }
}

impl std::error::Error for CodexCatalogError {}

fn line(message: Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&message).expect("static JSON-RPC shape serializes");
    bytes.push(b'\n');
    bytes
}

pub fn initialize_request(client_version: &str) -> Vec<u8> {
    line(json!({
        "id": CODEX_INITIALIZE_REQUEST_ID,
        "method": "initialize",
        "params": {
            "clientInfo": { "name": "fyagent", "version": client_version },
        },
    }))
}

pub fn initialized_notification() -> Vec<u8> {
    line(json!({ "method": "initialized", "params": {} }))
}

/// Page `index` (0-based) of `model/list`. Request IDs follow initialize.
pub fn model_list_request(index: usize, cursor: Option<&str>) -> Vec<u8> {
    let mut params = json!({
        "includeHidden": false,
        "limit": CODEX_MODEL_PAGE_SIZE,
    });
    if let Some(cursor) = cursor {
        params["cursor"] = Value::String(cursor.to_owned());
    }
    line(json!({
        "id": model_list_request_id(index),
        "method": "model/list",
        "params": params,
    }))
}

pub const fn model_list_request_id(index: usize) -> u64 {
    CODEX_INITIALIZE_REQUEST_ID + 1 + index as u64
}

/// Outcome of one stdout line while waiting for response `expected_id`.
#[derive(Debug, PartialEq)]
pub enum RpcLine {
    /// A notification, log, or response to another ID. Keep reading.
    Unrelated,
    Result(Value),
}

/// Classifies one app-server stdout line. Error text never leaves this
/// function; only the error class does.
pub fn classify_response_line(line: &[u8], expected_id: u64) -> Result<RpcLine, CodexCatalogError> {
    if line.len() > MAX_CODEX_RPC_LINE_BYTES {
        return Err(CodexCatalogError::InvalidResponse);
    }
    let Ok(message) = serde_json::from_slice::<Value>(line) else {
        return Ok(RpcLine::Unrelated);
    };
    if message.get("id").and_then(Value::as_u64) != Some(expected_id) {
        return Ok(RpcLine::Unrelated);
    }
    if let Some(error) = message.get("error") {
        return Err(
            if error.get("code").and_then(Value::as_i64) == Some(METHOD_NOT_FOUND) {
                CodexCatalogError::MethodUnsupported
            } else {
                CodexCatalogError::RequestFailed
            },
        );
    }
    message
        .get("result")
        .cloned()
        .map(RpcLine::Result)
        .ok_or(CodexCatalogError::InvalidResponse)
}

/// Extracts the CLI version from the `initialize` result's `userAgent`
/// (`<originator>/<version> (...)`), including originators containing spaces.
/// An unrecognized agent yields `None`.
pub fn cli_version_from_initialize(result: &Value) -> Option<String> {
    let agent = result.get("userAgent")?.as_str()?;
    let product = agent.split('(').next()?.trim();
    let version = product.split_once('/')?.1.split_whitespace().next()?;
    is_cli_version(version).then(|| version.to_owned())
}

pub fn is_cli_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODEX_CLI_VERSION_BYTES
        && value.as_bytes()[0].is_ascii_digit()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

pub fn is_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODEX_MODEL_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
        })
}

pub fn is_display_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_CODEX_MODEL_NAME_BYTES
        && !value.chars().any(char::is_control)
}

pub fn is_reasoning_effort(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODEX_REASONING_EFFORT_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

/// Next step after one accepted `model/list` page.
#[derive(Debug, PartialEq, Eq)]
pub enum CatalogStep {
    /// Request the page with this cursor (index = pages read so far).
    Next(String),
    Done,
}

/// Accumulates `model/list` pages under the helper bounds.
#[derive(Debug, Default)]
pub struct CodexCatalogBuilder {
    catalog: CodexModelCatalog,
    pages: usize,
    seen_ids: HashSet<String>,
    seen_cursors: HashSet<String>,
}

impl CodexCatalogBuilder {
    pub fn new(cli_version: Option<String>) -> Self {
        Self {
            catalog: CodexModelCatalog {
                cli_version,
                ..CodexModelCatalog::default()
            },
            ..Self::default()
        }
    }

    pub fn pages_read(&self) -> usize {
        self.pages
    }

    pub fn accept_page(&mut self, result: &Value) -> Result<CatalogStep, CodexCatalogError> {
        if self.pages == MAX_CODEX_MODEL_PAGES {
            return Err(CodexCatalogError::InvalidResponse);
        }
        self.pages += 1;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .ok_or(CodexCatalogError::InvalidResponse)?;
        let next_cursor = match result.get("nextCursor") {
            None | Some(Value::Null) => None,
            Some(Value::String(cursor))
                if !cursor.is_empty() && cursor.len() <= MAX_CODEX_CURSOR_BYTES =>
            {
                Some(cursor.clone())
            }
            Some(_) => return Err(CodexCatalogError::InvalidResponse),
        };

        for item in data {
            let Some(entry) = parse_model(item)? else {
                continue;
            };
            if !self.seen_ids.insert(entry.id.clone()) {
                continue;
            }
            if self.catalog.models.len() == MAX_CODEX_MODELS {
                self.catalog.truncated = true;
                return Ok(CatalogStep::Done);
            }
            let is_default =
                entry.is_default && !self.catalog.models.iter().any(|model| model.is_default);
            self.catalog.models.push(CodexModelEntry {
                is_default,
                ..entry
            });
        }

        let Some(cursor) = next_cursor else {
            return Ok(CatalogStep::Done);
        };
        if !self.seen_cursors.insert(cursor.clone()) {
            return Err(CodexCatalogError::InvalidResponse);
        }
        if self.pages == MAX_CODEX_MODEL_PAGES || self.catalog.models.len() == MAX_CODEX_MODELS {
            self.catalog.truncated = true;
            return Ok(CatalogStep::Done);
        }
        Ok(CatalogStep::Next(cursor))
    }

    pub fn finish(self) -> CodexModelCatalog {
        self.catalog
    }
}

/// `Ok(None)` skips a hidden entry. A missing or malformed `id` rejects the
/// whole page; optional presentation fields degrade instead.
fn parse_model(item: &Value) -> Result<Option<CodexModelEntry>, CodexCatalogError> {
    let object = item.as_object().ok_or(CodexCatalogError::InvalidResponse)?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| is_model_id(id))
        .ok_or(CodexCatalogError::InvalidResponse)?;
    if object.get("hidden").and_then(Value::as_bool) == Some(true) {
        return Ok(None);
    }
    let display_name = object
        .get("displayName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| is_display_name(name))
        .map(str::to_owned);
    let is_default = object.get("isDefault").and_then(Value::as_bool) == Some(true);
    let mut reasoning_efforts = Vec::new();
    if let Some(efforts) = object
        .get("supportedReasoningEfforts")
        .and_then(Value::as_array)
    {
        for effort in efforts {
            let value = effort
                .as_str()
                .or_else(|| effort.get("reasoningEffort").and_then(Value::as_str));
            if let Some(value) = value.filter(|value| is_reasoning_effort(value)) {
                if reasoning_efforts.len() < MAX_CODEX_REASONING_EFFORTS
                    && !reasoning_efforts.iter().any(|known| known == value)
                {
                    reasoning_efforts.push(value.to_owned());
                }
            }
        }
    }
    Ok(Some(CodexModelEntry {
        id: id.to_owned(),
        display_name,
        is_default,
        reasoning_efforts,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str) -> Value {
        json!({
            "id": id,
            "model": id,
            "displayName": id.to_uppercase(),
            "hidden": false,
            "isDefault": false,
            "supportedReasoningEfforts": [
                { "reasoningEffort": "low", "description": "" },
                { "reasoningEffort": "high", "description": "" },
            ],
        })
    }

    fn page(models: &[Value], next: Option<&str>) -> Value {
        json!({ "data": models, "nextCursor": next })
    }

    fn parse_line(bytes: &[u8]) -> Value {
        assert_eq!(bytes.last(), Some(&b'\n'));
        serde_json::from_slice(&bytes[..bytes.len() - 1]).unwrap()
    }

    #[test]
    fn request_lines_follow_the_documented_handshake_and_pagination() {
        let initialize = parse_line(&initialize_request("1.2.3"));
        assert_eq!(initialize["id"], 1);
        assert_eq!(initialize["method"], "initialize");
        assert_eq!(initialize["params"]["clientInfo"]["name"], "fyagent");
        assert_eq!(initialize["params"]["clientInfo"]["version"], "1.2.3");

        let initialized = parse_line(&initialized_notification());
        assert_eq!(initialized["method"], "initialized");
        assert!(initialized.get("id").is_none());

        let first = parse_line(&model_list_request(0, None));
        assert_eq!(first["id"], 2);
        assert_eq!(first["method"], "model/list");
        assert_eq!(first["params"]["includeHidden"], false);
        assert_eq!(first["params"]["limit"], CODEX_MODEL_PAGE_SIZE);
        assert!(first["params"].get("cursor").is_none());

        let second = parse_line(&model_list_request(1, Some("c1")));
        assert_eq!(second["id"], 3);
        assert_eq!(second["params"]["cursor"], "c1");
    }

    #[test]
    fn response_lines_skip_unrelated_traffic_and_classify_errors() {
        assert_eq!(
            classify_response_line(b"not json", 2).unwrap(),
            RpcLine::Unrelated
        );
        assert_eq!(
            classify_response_line(br#"{"method":"account/updated","params":{}}"#, 2).unwrap(),
            RpcLine::Unrelated
        );
        assert_eq!(
            classify_response_line(br#"{"id":1,"result":{}}"#, 2).unwrap(),
            RpcLine::Unrelated
        );
        assert_eq!(
            classify_response_line(br#"{"id":2,"result":{"data":[]}}"#, 2).unwrap(),
            RpcLine::Result(json!({"data":[]}))
        );
        assert_eq!(
            classify_response_line(
                br#"{"id":2,"error":{"code":-32601,"message":"C:\\Users\\secret"}}"#,
                2
            )
            .unwrap_err(),
            CodexCatalogError::MethodUnsupported
        );
        assert_eq!(
            classify_response_line(br#"{"id":2,"error":{"code":-32000,"message":"x"}}"#, 2)
                .unwrap_err(),
            CodexCatalogError::RequestFailed
        );
        assert_eq!(
            classify_response_line(br#"{"id":2}"#, 2).unwrap_err(),
            CodexCatalogError::InvalidResponse
        );
        let oversized = vec![b' '; MAX_CODEX_RPC_LINE_BYTES + 1];
        assert_eq!(
            classify_response_line(&oversized, 2).unwrap_err(),
            CodexCatalogError::InvalidResponse
        );
    }

    #[test]
    fn cli_version_comes_only_from_a_well_formed_user_agent() {
        for (agent, expected) in [
            (
                "Codex Desktop/0.160.1 (Windows 10.0.22635; x86_64) fyagent/0.4.14",
                Some("0.160.1"),
            ),
            (
                "fyagent/0.160.1 (Windows 10.0.22635; x86_64) xterm",
                Some("0.160.1"),
            ),
            (
                "codex_cli_rs/0.163.0-alpha.11 (Windows)",
                Some("0.163.0-alpha.11"),
            ),
            ("fyagent (Windows)", None),
            ("fyagent (Windows/10.0.22635)", None),
            ("fyagent/ (Windows)", None),
            ("fyagent/v1 (Windows)", None),
            ("fyagent/1.0;rm (Windows)", None),
        ] {
            assert_eq!(
                cli_version_from_initialize(&json!({ "userAgent": agent })).as_deref(),
                expected,
                "{agent}"
            );
        }
        assert_eq!(cli_version_from_initialize(&json!({})), None);
        let long = format!("x/1{}", "0".repeat(MAX_CODEX_CLI_VERSION_BYTES));
        assert_eq!(
            cli_version_from_initialize(&json!({ "userAgent": long })),
            None
        );
    }

    #[test]
    fn single_page_keeps_order_default_hidden_filter_and_efforts() {
        let mut default = model("gpt-a");
        default["isDefault"] = json!(true);
        let mut hidden = model("gpt-hidden");
        hidden["hidden"] = json!(true);
        let mut second_default = model("gpt-b");
        second_default["isDefault"] = json!(true);
        second_default["supportedReasoningEfforts"] = json!([
            "low",
            "low",
            "LOUD",
            "x".repeat(MAX_CODEX_REASONING_EFFORT_BYTES + 1)
        ]);
        let mut plain = model("gpt-c");
        plain["displayName"] = json!("bad\nname");

        let mut builder = CodexCatalogBuilder::new(Some("0.160.1".into()));
        assert_eq!(
            builder
                .accept_page(&page(&[default, hidden, second_default, plain], None))
                .unwrap(),
            CatalogStep::Done
        );
        let catalog = builder.finish();
        assert_eq!(catalog.cli_version.as_deref(), Some("0.160.1"));
        assert!(!catalog.truncated);
        let ids: Vec<_> = catalog
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect();
        assert_eq!(ids, ["gpt-a", "gpt-b", "gpt-c"]);
        assert!(catalog.models[0].is_default);
        assert!(
            !catalog.models[1].is_default,
            "only the first default is kept"
        );
        assert_eq!(catalog.models[0].display_name.as_deref(), Some("GPT-A"));
        assert_eq!(catalog.models[0].reasoning_efforts, ["low", "high"]);
        assert_eq!(catalog.models[1].reasoning_efforts, ["low"]);
        assert_eq!(catalog.models[2].display_name, None);
    }

    #[test]
    fn multiple_pages_follow_cursors_and_drop_repeated_ids() {
        let mut builder = CodexCatalogBuilder::new(None);
        assert_eq!(
            builder
                .accept_page(&page(&[model("a"), model("b")], Some("c1")))
                .unwrap(),
            CatalogStep::Next("c1".into())
        );
        assert_eq!(
            builder
                .accept_page(&page(&[model("b"), model("c")], None))
                .unwrap(),
            CatalogStep::Done
        );
        assert_eq!(builder.pages_read(), 2);
        let ids: Vec<_> = builder.finish().models.into_iter().map(|m| m.id).collect();
        assert_eq!(ids, ["a", "b", "c"]);
    }

    #[test]
    fn repeated_cursor_is_rejected_instead_of_looping() {
        let mut builder = CodexCatalogBuilder::new(None);
        builder
            .accept_page(&page(&[model("a")], Some("loop")))
            .unwrap();
        assert_eq!(
            builder
                .accept_page(&page(&[model("b")], Some("loop")))
                .unwrap_err(),
            CodexCatalogError::InvalidResponse
        );
    }

    #[test]
    fn page_and_model_bounds_end_with_an_explicit_truncation() {
        let mut builder = CodexCatalogBuilder::new(None);
        for index in 0..MAX_CODEX_MODEL_PAGES {
            let step = builder
                .accept_page(&page(
                    &[model(&format!("m{index}"))],
                    Some(&format!("c{index}")),
                ))
                .unwrap();
            if index + 1 < MAX_CODEX_MODEL_PAGES {
                assert_eq!(step, CatalogStep::Next(format!("c{index}")));
            } else {
                assert_eq!(step, CatalogStep::Done);
            }
        }
        let catalog = builder.finish();
        assert!(catalog.truncated);
        assert_eq!(catalog.models.len(), MAX_CODEX_MODEL_PAGES);

        let many: Vec<_> = (0..MAX_CODEX_MODELS + 3)
            .map(|index| model(&format!("m{index}")))
            .collect();
        let mut builder = CodexCatalogBuilder::new(None);
        assert_eq!(
            builder.accept_page(&page(&many, None)).unwrap(),
            CatalogStep::Done
        );
        let catalog = builder.finish();
        assert!(catalog.truncated);
        assert_eq!(catalog.models.len(), MAX_CODEX_MODELS);

        let exact: Vec<_> = (0..MAX_CODEX_MODELS)
            .map(|index| model(&format!("m{index}")))
            .collect();
        let mut builder = CodexCatalogBuilder::new(None);
        builder.accept_page(&page(&exact, None)).unwrap();
        assert!(!builder.finish().truncated);
    }

    #[test]
    fn malformed_pages_are_rejected() {
        for bad in [
            json!({}),
            json!({ "data": {} }),
            json!({ "data": [1] }),
            json!({ "data": [{ "displayName": "no id" }] }),
            json!({ "data": [{ "id": "" }] }),
            json!({ "data": [{ "id": "has space" }] }),
            json!({ "data": [{ "id": "x".repeat(MAX_CODEX_MODEL_ID_BYTES + 1) }] }),
            json!({ "data": [], "nextCursor": 7 }),
            json!({ "data": [], "nextCursor": "" }),
            json!({ "data": [], "nextCursor": "c".repeat(MAX_CODEX_CURSOR_BYTES + 1) }),
        ] {
            let mut builder = CodexCatalogBuilder::new(None);
            assert_eq!(
                builder.accept_page(&bad).unwrap_err(),
                CodexCatalogError::InvalidResponse,
                "{bad}"
            );
        }
    }

    #[test]
    fn empty_catalog_is_a_valid_result() {
        let mut builder = CodexCatalogBuilder::new(None);
        assert_eq!(
            builder.accept_page(&page(&[], None)).unwrap(),
            CatalogStep::Done
        );
        assert_eq!(builder.finish(), CodexModelCatalog::default());
    }
}
