//! xAI native Responses compatibility for Codex clients.
//!
//! Adapted from CC Switch commit ef97ef95e717b1b5c47357c5a28ed78387138417
//! (MIT, Copyright (c) 2025 Jason Young; see LICENSES/MIT-CC-SWITCH.txt).
//!
//! Request compatibility runs after namespace flattening so lifted function
//! tools survive the xAI tool whitelist. It normalizes unsupported schemas,
//! collaboration messages and model aliases. Response compatibility restores
//! namespaces and integer arguments while preserving partial argument deltas.
//! The provider gate limits these rules to native Responses on xAI upstreams.

use std::collections::{HashMap, HashSet};

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde_json::{json, Map, Value};

use super::transform_codex_responses_namespace::{restore_sse_event_namespaces, NamespacedName};
use crate::proxy::sse::{append_utf8_safe, strip_sse_field, take_sse_block};

/// Codex plugin-private fields removed recursively at any nesting depth.
const RECURSIVE_UNSUPPORTED_FIELDS: &[&str] = &["external_web_access"];

/// Top-level request fields xAI rejects regardless of model.
const TOP_LEVEL_UNSUPPORTED_FIELDS: &[&str] = &["prompt_cache_retention", "safety_identifier"];

/// Top-level sampling fields rejected specifically by grok-4.5.
const GROK_45_UNSUPPORTED_FIELDS: &[&str] = &[
    "presence_penalty",
    "presencePenalty",
    "frequency_penalty",
    "frequencyPenalty",
    "stop",
];

/// Tool `type` values xAI's Responses schema accepts. Sourced from xAI's own
/// serde error enumeration (which is more complete than sub2api's hand-copied
/// list — it includes `image_generation`). Any other `type` is a Codex/OpenAI
/// private carrier (`tool_search`, a stray `namespace`, `custom`, …) that the
/// strict parser would reject, so it is dropped.
const XAI_SUPPORTED_TOOL_TYPES: &[&str] = &[
    "function",
    "web_search",
    "x_search",
    "image_generation",
    "collections_search",
    "file_search",
    "code_execution",
    "code_interpreter",
    "mcp",
    "shell",
];

/// Strip xAI-unsupported fields and tools from a native Codex Responses request
/// body in place. Returns whether anything changed. Deterministic and
/// idempotent: running it twice on the same body changes nothing the second
/// time.
pub(crate) fn sanitize_xai_responses_request(body: &mut Value) -> bool {
    if !body.is_object() {
        return false;
    }

    let mut changed = false;

    // 1. Top-level fields xAI rejects for every model.
    for field in TOP_LEVEL_UNSUPPORTED_FIELDS {
        changed |= remove_top_level_field(body, field);
    }

    // 2. grok-4.5 additionally rejects these sampling knobs.
    if request_targets_grok_45(body) {
        for field in GROK_45_UNSUPPORTED_FIELDS {
            changed |= remove_top_level_field(body, field);
        }
    }

    // 3. Codex plugin-private flags buried at any depth (e.g. inside tools or
    //    tool parameter schemas).
    for field in RECURSIVE_UNSUPPORTED_FIELDS {
        changed |= remove_field_recursive(body, field);
    }

    // 4. Lift the `additional_tools` input carrier (Responses Lite private
    //    shape) up to top-level `tools` so the supported ones survive.
    changed |= promote_additional_tools(body);

    // 5. Drop `content: null` on reasoning input items — xAI's untagged enum
    //    deserializer refuses a present-but-null content field.
    changed |= strip_null_reasoning_content(body);

    // 6. Whitelist the tool types and clean a now-dangling `tool_choice`.
    changed |= filter_unsupported_tools(body);

    // xAI requires a compatible object root for function parameters.
    changed |= normalize_xai_function_tool_parameter_schemas(body);

    changed
}

/// Whether the request's (possibly provider-prefixed) model resolves to
/// grok-4.5. Mirrors sub2api's suffix match: `foo/grok-4.5` counts.
fn request_targets_grok_45(body: &Value) -> bool {
    let Some(model) = body.get("model").and_then(Value::as_str) else {
        return false;
    };
    let mut model = model.trim();
    if let Some(idx) = model.rfind('/') {
        model = model[idx + 1..].trim();
    }
    model.eq_ignore_ascii_case("grok-4.5")
}

fn remove_top_level_field(body: &mut Value, field: &str) -> bool {
    body.as_object_mut()
        .and_then(|obj| obj.remove(field))
        .is_some()
}

/// Delete every occurrence of `field` in the tree, at any depth.
fn remove_field_recursive(value: &mut Value, field: &str) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = map.remove(field).is_some();
            for child in map.values_mut() {
                changed |= remove_field_recursive(child, field);
            }
            changed
        }
        Value::Array(items) => {
            let mut changed = false;
            for child in items.iter_mut() {
                changed |= remove_field_recursive(child, field);
            }
            changed
        }
        _ => false,
    }
}

fn is_additional_tools_item(item: &Value) -> bool {
    item.get("type").and_then(Value::as_str).map(str::trim) == Some("additional_tools")
}

/// Promote any `additional_tools` carrier items from `input` into top-level
/// `tools`, preserving top-level order and appending carrier tools in order,
/// de-duplicated. The carrier items themselves are removed from `input`.
fn promote_additional_tools(body: &mut Value) -> bool {
    // Clone `input` up front so the later mutable write-back to `body` doesn't
    // collide with the read borrow. Only pays the clone on the rare carrier path.
    let input_items: Vec<Value> = match body.get("input").and_then(Value::as_array) {
        Some(arr) if arr.iter().any(is_additional_tools_item) => arr.clone(),
        _ => return false,
    };

    // Seed merged tools + dedup keys from the existing top-level tools.
    let mut merged: Vec<Value> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        for tool in tools {
            seen.insert(tool_dedup_key(tool));
            merged.push(tool.clone());
        }
    }

    let mut filtered_input: Vec<Value> = Vec::with_capacity(input_items.len());
    let mut promoted = false;
    for item in input_items {
        if is_additional_tools_item(&item) {
            if let Some(carrier_tools) = item.get("tools").and_then(Value::as_array) {
                for tool in carrier_tools {
                    if seen.insert(tool_dedup_key(tool)) {
                        merged.push(tool.clone());
                        promoted = true;
                    }
                }
            }
            continue; // carrier item dropped regardless of dedup outcome
        }
        filtered_input.push(item);
    }

    if let Some(obj) = body.as_object_mut() {
        obj.insert("input".to_string(), Value::Array(filtered_input));
        if promoted {
            obj.insert("tools".to_string(), Value::Array(merged));
        }
    }
    // We reached here only because a carrier existed, so `input` changed.
    true
}

/// Stable dedup key for a tool: `(type, name)`, `(mcp, server_label)`, or the
/// serialized tool as a last resort. Mirrors sub2api's `grokResponsesToolDedupKey`.
fn tool_dedup_key(tool: &Value) -> String {
    let tool_type = tool
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if !tool_type.is_empty() {
        if let Some(name) = tool.get("name").and_then(Value::as_str) {
            let name = name.trim();
            if !name.is_empty() {
                return format!("type:{tool_type}\u{0}name:{name}");
            }
        }
        if tool_type == "mcp" {
            if let Some(label) = tool.get("server_label").and_then(Value::as_str) {
                let label = label.trim();
                if !label.is_empty() {
                    return format!("type:mcp\u{0}server_label:{label}");
                }
            }
        }
    }
    format!("json:{tool}")
}

fn strip_null_reasoning_content(body: &mut Value) -> bool {
    let Some(input) = body.get_mut("input").and_then(Value::as_array_mut) else {
        return false;
    };
    let mut changed = false;
    for item in input.iter_mut() {
        if item.get("type").and_then(Value::as_str).map(str::trim) != Some("reasoning") {
            continue;
        }
        if let Some(obj) = item.as_object_mut() {
            if matches!(obj.get("content"), Some(Value::Null)) {
                obj.remove("content");
                changed = true;
            }
        }
    }
    changed
}

/// Keep only whitelisted tool types and drop a `tool_choice` that now points at
/// a removed or unsupported tool.
fn filter_unsupported_tools(body: &mut Value) -> bool {
    let Some(tools) = body.get("tools").and_then(Value::as_array) else {
        return false;
    };
    let original_len = tools.len();
    let filtered: Vec<Value> = tools
        .iter()
        .filter(|tool| {
            let t = tool
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            XAI_SUPPORTED_TOOL_TYPES.contains(&t)
        })
        .cloned()
        .collect();

    let mut changed = false;
    if filtered.len() != original_len {
        if let Some(obj) = body.as_object_mut() {
            if filtered.is_empty() {
                obj.remove("tools");
            } else {
                obj.insert("tools".to_string(), Value::Array(filtered.clone()));
            }
        }
        changed = true;
    }

    if body.get("tool_choice").is_some() && should_drop_tool_choice(body, &filtered) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("tool_choice");
        }
        changed = true;
    }

    changed
}

/// Whether `tool_choice` should be dropped given the surviving `tools`. String
/// choices (`"auto"`, `"none"`, `"required"`) are always kept; object choices
/// are dropped when they reference an unsupported type or a function name that
/// no longer exists.
fn should_drop_tool_choice(body: &Value, tools: &[Value]) -> bool {
    let Some(tool_choice) = body.get("tool_choice") else {
        return false;
    };
    if tools.is_empty() {
        return true;
    }
    let Some(choice) = tool_choice.as_object() else {
        return false; // "auto"/"none"/"required" string choices stay
    };
    let choice_type = choice
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if choice_type.is_empty() {
        return false;
    }
    if !XAI_SUPPORTED_TOOL_TYPES.contains(&choice_type) {
        return true;
    }
    if choice_type == "function" {
        let choice_name = choice
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| {
                choice
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
            })
            .unwrap_or("")
            .trim();
        if choice_name.is_empty() {
            return false;
        }
        let exists = tools.iter().any(|tool| {
            let t = tool
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let name = tool
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| {
                    tool.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(Value::as_str)
                })
                .unwrap_or("")
                .trim();
            t == "function" && name == choice_name
        });
        return !exists;
    }
    false
}

/// Whether a Responses function tool declares parameters that xAI's strict
/// validator rejects before sampling begins.
fn xai_function_parameters_need_simplification(params: &Value) -> bool {
    match params {
        Value::Null => true,
        Value::Object(obj) if obj.is_empty() => true,
        Value::Object(obj) => {
            match obj.get("type") {
                None | Some(Value::Null) => return true,
                Some(Value::String(type_name)) if type_name != "object" => return true,
                _ => {}
            }

            for union_key in ["oneOf", "anyOf"] {
                let Some(branches) = obj.get(union_key).and_then(Value::as_array) else {
                    continue;
                };
                if branches.is_empty() {
                    continue;
                }
                if branches
                    .iter()
                    .any(|branch| branch.get("type").and_then(Value::as_str) != Some("object"))
                {
                    return true;
                }
            }

            false
        }
        _ => true,
    }
}

/// Collapse a root-level `oneOf`/`anyOf` union into a plain object schema.
fn flatten_union_branches_to_object(branches: &[Value]) -> Value {
    let object_branches: Vec<&Value> = branches
        .iter()
        .filter(|branch| branch.get("type").and_then(Value::as_str) == Some("object"))
        .collect();

    if object_branches.len() == 1 {
        let mut result = object_branches[0].clone();
        if let Some(obj) = result.as_object_mut() {
            obj.insert("type".to_string(), json!("object"));
            obj.entry("properties".to_string())
                .or_insert_with(|| json!({}));
        }
        return result;
    }

    if !object_branches.is_empty() {
        let mut merged_properties = Map::new();
        let mut merged_required = Vec::new();
        for branch in object_branches {
            if let Some(properties) = branch.get("properties").and_then(Value::as_object) {
                for (key, value) in properties {
                    merged_properties
                        .entry(key.clone())
                        .or_insert_with(|| value.clone());
                }
            }
            if let Some(required) = branch.get("required").and_then(Value::as_array) {
                for item in required {
                    if !merged_required.iter().any(|existing| existing == item) {
                        merged_required.push(item.clone());
                    }
                }
            }
        }

        let mut result = json!({
            "type": "object",
            "properties": Value::Object(merged_properties),
        });
        if !merged_required.is_empty() {
            result["required"] = Value::Array(merged_required);
        }
        return result;
    }

    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true
    })
}

/// Rewrite a function tool's JSON Schema parameters into an xAI-compatible root
/// object schema.
fn simplify_xai_function_parameters(params: Option<&Value>) -> Value {
    match params {
        None | Some(Value::Null) => {
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        }
        Some(Value::Object(obj)) if obj.is_empty() => {
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        }
        Some(Value::Object(obj)) => {
            for union_key in ["oneOf", "anyOf"] {
                if let Some(branches) = obj.get(union_key).and_then(Value::as_array) {
                    if branches
                        .iter()
                        .any(|branch| branch.get("type").and_then(Value::as_str) != Some("object"))
                    {
                        return flatten_union_branches_to_object(branches);
                    }
                }
            }

            let mut result = Value::Object(obj.clone());
            if let Some(obj) = result.as_object_mut() {
                match obj.get("type").and_then(Value::as_str) {
                    Some("object") => {}
                    _ => {
                        obj.insert("type".to_string(), json!("object"));
                        obj.entry("properties".to_string())
                            .or_insert_with(|| json!({}));
                    }
                }
            }
            result
        }
        _ => json!({"type": "object", "properties": {}, "additionalProperties": true}),
    }
}

fn function_tool_name(tool: &Value) -> &str {
    tool.get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            tool.get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
        })
        .unwrap_or("")
        .trim()
}

fn is_automation_update_tool(name: &str) -> bool {
    name == "codex_app__automation_update"
        || name == "mcp__codex_app__automation_update"
        || name.ends_with("__automation_update")
}

fn rewrite_function_tool_parameters(tool: &mut Value, params: Option<&Value>) -> bool {
    let simplified = simplify_xai_function_parameters(params);
    if params == Some(&simplified) {
        return false;
    }

    if let Some(obj) = tool.as_object_mut() {
        if obj.contains_key("parameters") {
            obj.insert("parameters".to_string(), simplified);
            return true;
        }
        if let Some(function) = obj.get_mut("function").and_then(Value::as_object_mut) {
            function.insert("parameters".to_string(), simplified);
            return true;
        }
    }

    false
}

fn xai_safe_empty_object_schema() -> Value {
    json!({"type": "object", "properties": {}, "additionalProperties": true})
}

fn normalize_xai_function_tool_parameters(tool: &mut Value) -> bool {
    if tool.get("type").and_then(Value::as_str) != Some("function") {
        return false;
    }

    // Codex Desktop always injects automation_update with a root oneOf/anyOf
    // that includes a non-object (null) branch. xAI rejects the whole turn
    // (farion1231/cc-switch#6815). Keep the tool callable, but force a plain
    // object root the way CLIProxyAPI does.
    if is_automation_update_tool(function_tool_name(tool)) {
        let safe = xai_safe_empty_object_schema();
        let needs_rewrite = {
            let current = tool.get("parameters").or_else(|| {
                tool.get("function")
                    .and_then(|function| function.get("parameters"))
            });
            current != Some(&safe)
        };
        let mut changed = needs_rewrite;
        if let Some(obj) = tool.as_object_mut() {
            if needs_rewrite {
                if obj.contains_key("parameters") || obj.get("function").is_none() {
                    obj.insert("parameters".to_string(), safe);
                } else if let Some(function) =
                    obj.get_mut("function").and_then(Value::as_object_mut)
                {
                    function.insert("parameters".to_string(), safe);
                }
            }
            if obj.get("strict") == Some(&json!(true)) {
                obj.insert("strict".to_string(), json!(false));
                changed = true;
            }
            if let Some(function) = obj.get_mut("function").and_then(Value::as_object_mut) {
                if function.get("strict") == Some(&json!(true)) {
                    function.insert("strict".to_string(), json!(false));
                    changed = true;
                }
            }
        }
        return changed;
    }

    let params = tool
        .get("parameters")
        .or_else(|| {
            tool.get("function")
                .and_then(|function| function.get("parameters"))
        })
        .cloned();

    let changed = match params.as_ref() {
        Some(params) if xai_function_parameters_need_simplification(params) => {
            rewrite_function_tool_parameters(tool, Some(params))
        }
        None => rewrite_function_tool_parameters(tool, None),
        _ => false,
    };

    changed
}

fn normalize_xai_function_tool_parameter_schemas(body: &mut Value) -> bool {
    let Some(tools) = body.get_mut("tools").and_then(Value::as_array_mut) else {
        return false;
    };

    let mut changed = false;
    for tool in tools.iter_mut() {
        changed |= normalize_xai_function_tool_parameters(tool);
    }
    changed
}

/// Apply model selection before model-specific field normalization.
pub(crate) fn apply_xai_native_responses_request_compat(
    body: &mut Value,
    provider_id: &str,
    upstream_model: Option<&str>,
    settings: &Value,
) {
    let mut changed = false;
    if let Some(upstream_model) = upstream_model {
        let allowed = collect_xai_catalog_model_ids(settings);
        changed |= rewrite_xai_unknown_request_model(body, upstream_model, &allowed).is_some();
    }
    changed |= sanitize_xai_responses_request(body);
    changed |= rewrite_xai_agent_message_input_items(body);
    if changed {
        log::debug!("[Codex] Applied xAI native Responses compatibility (provider={provider_id})");
    }
}

/// Rewrite Codex multi-agent v2 `agent_message` items into ordinary `message`
/// items. xAI's Responses `ModelInput` enum has no `agent_message` variant, so
/// a native passthrough 422s (`input[N]: unknown item type "agent_message"`)
/// before the child agent can run.
///
/// Walk the whole request body, not only the top-level `input` array: Codex
/// may nest the same item under later collaboration turns. Keep this out of
/// [`sanitize_xai_responses_request`]: it is a structural rewrite, not a field
/// deletion. Routed Grok sessions currently put plaintext task bodies in
/// `encrypted_content` parts; flatten those to `input_text`.
fn rewrite_xai_agent_message_input_items(body: &mut Value) -> bool {
    rewrite_agent_message_value(body)
}

fn rewrite_agent_message_value(value: &mut Value) -> bool {
    if rewrite_agent_message_item(value) {
        return true;
    }
    match value {
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= rewrite_agent_message_value(item);
            }
            changed
        }
        Value::Object(obj) => {
            let mut changed = false;
            for child in obj.values_mut() {
                changed |= rewrite_agent_message_value(child);
            }
            changed
        }
        _ => false,
    }
}

/// Remap a request `model` that xAI will not serve onto the provider's
/// configured model (the live main-agent model). Catalog `model`/`slug`/`id`
/// values are preserved so a user who picked `grok-4.5` is not forced onto
/// `grok-4.6`. Unknown OpenAI role SKUs such as `gpt-5.6-sol` are rewritten.
///
/// Returns `Some((from, to))` when the field changed. Missing or empty
/// `model` is filled with `upstream_model`. An empty upstream model is a
/// no-op so we never invent a name.
fn rewrite_xai_unknown_request_model(
    body: &mut Value,
    upstream_model: &str,
    allowed_models: &HashSet<String>,
) -> Option<(String, String)> {
    let upstream = upstream_model.trim();
    if upstream.is_empty() {
        return None;
    }

    let obj = body.as_object_mut()?;
    let request = obj
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    if !request.is_empty() && request_model_is_allowed(&request, upstream, allowed_models) {
        return None;
    }

    obj.insert("model".to_string(), Value::String(upstream.to_string()));
    Some((request, upstream.to_string()))
}

/// Collect catalog model identifiers from a Codex provider's settings.
/// Grok catalogs store the live id on `slug`; other providers may use
/// `model` or `id`. All three are accepted so a catalog hit is not missed.
fn collect_xai_catalog_model_ids(settings: &Value) -> HashSet<String> {
    let mut ids = HashSet::new();
    let Some(models) = settings
        .get("modelCatalog")
        .and_then(|catalog| catalog.get("models"))
        .and_then(Value::as_array)
    else {
        return ids;
    };
    for entry in models {
        for key in ["model", "slug", "id"] {
            if let Some(id) = entry
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
            {
                ids.insert(id.to_string());
            }
        }
    }
    ids
}

fn request_model_is_allowed(
    request: &str,
    upstream: &str,
    allowed_models: &HashSet<String>,
) -> bool {
    request.eq_ignore_ascii_case(upstream)
        || allowed_models
            .iter()
            .any(|id| id.eq_ignore_ascii_case(request))
}

fn json_type(value: &Value) -> Option<&str> {
    value.get("type").and_then(Value::as_str).map(str::trim)
}

fn rewrite_agent_message_item(item: &mut Value) -> bool {
    if json_type(item) != Some("agent_message") {
        return false;
    }

    let id = item.get("id").cloned();
    let content = flatten_agent_message_content(item.get("content"));
    let mut message = json!({
        "type": "message",
        "role": "user",
        "content": content,
    });
    if let Some(id) = id {
        message["id"] = id;
    }
    *item = message;
    true
}

fn flatten_agent_message_content(content: Option<&Value>) -> Vec<Value> {
    match content {
        Some(Value::Array(parts)) => parts.iter().filter_map(part_to_input_text).collect(),
        Some(Value::String(text)) if !text.is_empty() => vec![input_text_part(text)],
        _ => Vec::new(),
    }
}

fn part_to_input_text(part: &Value) -> Option<Value> {
    let text = if json_type(part) == Some("encrypted_content") {
        part.get("encrypted_content")
            .or_else(|| part.get("text"))
            .and_then(Value::as_str)
    } else {
        part.get("text").and_then(Value::as_str)
    }?;
    if text.is_empty() {
        None
    } else {
        Some(input_text_part(text))
    }
}

fn input_text_part(text: &str) -> Value {
    json!({ "type": "input_text", "text": text })
}

/// Rewrite whole-number JSON floats (`92116.0`) to integers (`92116`) on
/// completed function-call argument payloads. Grok emits JSON Number floats for
/// integer tool fields; Codex Desktop then fails local serde (`expected i32` /
/// `expected u64`) and never runs the tool.
///
/// Applies only to `response.function_call_arguments.done` and completed
/// `function_call` items. SSE `*.delta` fragments are left untouched because
/// they are not complete JSON. Parse/rewrite failures pass the original bytes
/// through so Codex still surfaces the error — never replace arguments with
/// `{}` or otherwise swallow the failure.
pub(crate) fn normalize_xai_function_call_integer_arguments(value: &mut Value) -> bool {
    normalize_xai_function_call_integer_arguments_value(value)
}

fn normalize_xai_function_call_integer_arguments_value(value: &mut Value) -> bool {
    match value {
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= normalize_xai_function_call_integer_arguments_value(item);
            }
            changed
        }
        Value::Object(obj) => {
            let event_type = obj
                .get("type")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if event_type.as_deref() == Some("response.function_call_arguments.delta") {
                return false;
            }

            let mut changed = false;
            if event_type.as_deref() == Some("response.function_call_arguments.done")
                || event_type.as_deref() == Some("function_call")
            {
                changed |= normalize_function_call_arguments_field(obj);
            }
            for child in obj.values_mut() {
                changed |= normalize_xai_function_call_integer_arguments_value(child);
            }
            changed
        }
        _ => false,
    }
}

fn normalize_function_call_arguments_field(obj: &mut Map<String, Value>) -> bool {
    match obj.get_mut("arguments") {
        Some(Value::String(arguments)) => match rewrite_whole_float_arguments_json(arguments) {
            Ok(Some(rewritten)) => {
                *arguments = rewritten;
                true
            }
            Ok(None) => false,
            Err(error) => {
                log::debug!(
                    "[Codex] xAI function_call arguments were not rewritten; passing through unchanged: {error}"
                );
                false
            }
        },
        // Parsed numeric values no longer retain the original decimal token.
        _ => false,
    }
}

fn rewrite_whole_float_arguments_json(
    arguments: &str,
) -> Result<Option<String>, serde_json::Error> {
    // Validate structure without rebuilding numbers through an f64-backed Value.
    // Only replaced numeric tokens change; all other bytes remain original.
    let _: serde::de::IgnoredAny = serde_json::from_str(arguments)?;
    let bytes = arguments.as_bytes();
    let mut cursor = 0;
    let mut copied_until = 0;
    let mut rewritten = String::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => {
                cursor += 1;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'\\' => cursor += 2,
                        b'"' => {
                            cursor += 1;
                            break;
                        }
                        _ => cursor += 1,
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = cursor;
                cursor += 1;
                while cursor < bytes.len()
                    && matches!(
                        bytes[cursor],
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'
                    )
                {
                    cursor += 1;
                }
                if let Some(integer) = exact_decimal_integer(&arguments[start..cursor]) {
                    rewritten.push_str(&arguments[copied_until..start]);
                    rewritten.push_str(&integer);
                    copied_until = cursor;
                }
            }
            _ => cursor += 1,
        }
    }
    if copied_until == 0 {
        return Ok(None);
    }
    rewritten.push_str(&arguments[copied_until..]);
    Ok(Some(rewritten))
}

/// Convert a validated decimal/exponent JSON number only when its exact value
/// is an integer in i64/u64 range. Decimal digits, including insignificant zeros,
/// determine integrality; binary floating-point rounding is never involved.
fn exact_decimal_integer(token: &str) -> Option<String> {
    if !token.contains(['.', 'e', 'E']) {
        return None;
    }
    let negative = token.starts_with('-');
    let unsigned = token.strip_prefix('-').unwrap_or(token);
    let (coefficient, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => (
            &unsigned[..index],
            unsigned[index + 1..].parse::<i64>().ok()?,
        ),
        None => (unsigned, 0),
    };
    let fraction_digits = coefficient
        .find('.')
        .map_or(0, |index| coefficient.len() - index - 1);
    let digits: String = coefficient.chars().filter(|ch| *ch != '.').collect();
    let significant = digits.trim_start_matches('0');
    if significant.is_empty() {
        return Some("0".into());
    }
    let shift = exponent.checked_sub(i64::try_from(fraction_digits).ok()?)?;
    let integer_digits = if shift < 0 {
        let removed = usize::try_from(shift.checked_neg()?).ok()?;
        let end = significant.len().checked_sub(removed)?;
        if !significant.as_bytes()[end..]
            .iter()
            .all(|digit| *digit == b'0')
        {
            return None;
        }
        significant[..end].to_string()
    } else {
        let added = usize::try_from(shift).ok()?;
        if significant.len().checked_add(added)? > 20 {
            return None;
        }
        let mut integer = significant.to_string();
        integer.extend(std::iter::repeat_n('0', added));
        integer
    };
    let magnitude = integer_digits.parse::<u64>().ok()?;
    if negative {
        if magnitude > i64::MIN.unsigned_abs() {
            return None;
        }
        Some(format!("-{magnitude}"))
    } else {
        Some(magnitude.to_string())
    }
}

/// Wrap a native Responses SSE byte stream: restore flattened namespace names
/// and rewrite completed function-call argument JSON. Delta fragments that are
/// not complete JSON pass through unchanged.
pub(crate) fn create_xai_native_responses_sse_stream<E>(
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
    restore_map: HashMap<String, NamespacedName>,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send
where
    E: std::error::Error + Send + 'static,
{
    async_stream::stream! {
        let mut buffer = String::new();
        let mut utf8_remainder: Vec<u8> = Vec::new();

        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    append_utf8_safe(&mut buffer, &mut utf8_remainder, &bytes);
                    while let Some(block) = take_sse_block(&mut buffer) {
                        if block.trim().is_empty() {
                            continue;
                        }
                        yield Ok(rewrite_xai_native_sse_block(&block, &restore_map));
                    }
                }
                Err(e) => {
                    yield Err(std::io::Error::other(e.to_string()));
                    return;
                }
            }
        }

        if !utf8_remainder.is_empty() {
            buffer.push_str(&String::from_utf8_lossy(&utf8_remainder));
        }
        let tail = std::mem::take(&mut buffer);
        if !tail.trim().is_empty() {
            yield Ok(rewrite_xai_native_sse_block(&tail, &restore_map));
        }
    }
}

fn rewrite_xai_native_sse_block(
    block: &str,
    restore_map: &HashMap<String, NamespacedName>,
) -> Bytes {
    let mut data_parts: Vec<&str> = Vec::new();
    for line in block.lines() {
        if let Some(data) = strip_sse_field(line, "data") {
            data_parts.push(data);
        }
    }

    if data_parts.is_empty() {
        return Bytes::from(format!("{block}\n\n"));
    }

    let data = data_parts.join("\n");
    if data.trim() == "[DONE]" {
        return Bytes::from(format!("{block}\n\n"));
    }

    let mut event: Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return Bytes::from(format!("{block}\n\n")),
    };

    let mut changed = restore_sse_event_namespaces(&mut event, restore_map);
    changed |= normalize_xai_function_call_integer_arguments(&mut event);
    if !changed {
        return Bytes::from(format!("{block}\n\n"));
    }

    let Ok(restored) = serde_json::to_string(&event) else {
        return Bytes::from(format!("{block}\n\n"));
    };
    // Replace the data field group while preserving cursor, retry, comments,
    // extension fields, and each non-data line's original position and ending.
    let mut out = String::new();
    let mut data_written = false;
    for line in block.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if strip_sse_field(content, "data").is_some() {
            if !data_written {
                out.push_str("data: ");
                out.push_str(&restored);
                out.push_str(&line[content.len()..]);
                data_written = true;
            }
        } else {
            out.push_str(line);
        }
    }
    let newline = if block.contains("\r\n") { "\r\n" } else { "\n" };
    if !out.ends_with('\n') {
        out.push_str(newline);
    }
    out.push_str(newline);
    Bytes::from(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn strips_external_web_access_recursively() {
        let mut body = json!({
            "model": "grok-4.5",
            "external_web_access": true,
            "tools": [
                {"type": "function", "name": "f", "external_web_access": true,
                 "parameters": {"type": "object", "q": {"external_web_access": true}}}
            ],
            "metadata": {"external_web_access": false}
        });
        assert!(sanitize_xai_responses_request(&mut body));
        let s = body.to_string();
        assert!(!s.contains("external_web_access"), "left over: {s}");
    }

    #[test]
    fn strips_top_level_unsupported_fields() {
        let mut body = json!({
            "model": "grok-4.5",
            "prompt_cache_retention": "24h",
            "safety_identifier": "abc"
        });
        assert!(sanitize_xai_responses_request(&mut body));
        assert!(body.get("prompt_cache_retention").is_none());
        assert!(body.get("safety_identifier").is_none());
    }

    #[test]
    fn strips_grok_45_only_sampling_fields() {
        let mut body = json!({
            "model": "grok-4.5",
            "presence_penalty": 0.1,
            "frequency_penalty": 0.2,
            "stop": ["x"]
        });
        assert!(sanitize_xai_responses_request(&mut body));
        assert!(body.get("presence_penalty").is_none());
        assert!(body.get("frequency_penalty").is_none());
        assert!(body.get("stop").is_none());
    }

    #[test]
    fn keeps_sampling_fields_for_non_grok_45() {
        let mut body = json!({
            "model": "grok-4-fast",
            "presence_penalty": 0.1,
            "stop": ["x"]
        });
        // No unsupported fields present, so no change and knobs preserved.
        assert!(!sanitize_xai_responses_request(&mut body));
        assert_eq!(body.get("presence_penalty"), Some(&json!(0.1)));
        assert_eq!(body.get("stop"), Some(&json!(["x"])));
    }

    #[test]
    fn matches_grok_45_with_provider_prefix() {
        let mut body = json!({"model": "xai/grok-4.5", "stop": ["x"]});
        assert!(sanitize_xai_responses_request(&mut body));
        assert!(body.get("stop").is_none());
    }

    #[test]
    fn promotes_additional_tools_dedup() {
        let mut body = json!({
            "model": "grok-4.5",
            "tools": [{"type": "function", "name": "kept"}],
            "input": [
                {"type": "message", "role": "user", "content": "hi"},
                {"type": "additional_tools", "tools": [
                    {"type": "function", "name": "kept"},
                    {"type": "function", "name": "extra"}
                ]}
            ]
        });
        assert!(sanitize_xai_responses_request(&mut body));
        // carrier removed from input
        let input = body.get("input").unwrap().as_array().unwrap();
        assert_eq!(input.len(), 1);
        assert!(input.iter().all(|i| !is_additional_tools_item(i)));
        // extra promoted, kept not duplicated
        let tools = body.get("tools").unwrap().as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t.get("name").and_then(Value::as_str).unwrap())
            .collect();
        assert_eq!(names, vec!["kept", "extra"]);
    }

    #[test]
    fn strips_null_reasoning_content() {
        let mut body = json!({
            "model": "grok-4.5",
            "input": [
                {"type": "reasoning", "content": null, "id": "r1"},
                {"type": "reasoning", "content": [{"text": "keep"}], "id": "r2"}
            ]
        });
        assert!(sanitize_xai_responses_request(&mut body));
        let input = body.get("input").unwrap().as_array().unwrap();
        assert!(input[0].get("content").is_none());
        assert!(input[1].get("content").is_some());
    }

    #[test]
    fn filters_unsupported_tool_types() {
        let mut body = json!({
            "model": "grok-4.5",
            "tools": [
                {"type": "function", "name": "f"},
                {"type": "tool_search"},
                {"type": "custom", "name": "c"},
                {"type": "mcp", "server_label": "s"}
            ]
        });
        assert!(sanitize_xai_responses_request(&mut body));
        let types: Vec<&str> = body
            .get("tools")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.get("type").and_then(Value::as_str).unwrap())
            .collect();
        assert_eq!(types, vec!["function", "mcp"]);
    }

    #[test]
    fn drops_dangling_function_tool_choice() {
        let mut body = json!({
            "model": "grok-4.5",
            "tools": [{"type": "tool_search"}],
            "tool_choice": {"type": "function", "name": "gone"}
        });
        assert!(sanitize_xai_responses_request(&mut body));
        // tool_search filtered → no tools → tool_choice dropped
        assert!(body.get("tools").is_none());
        assert!(body.get("tool_choice").is_none());
    }

    #[test]
    fn keeps_valid_function_tool_choice() {
        let mut body = json!({
            "model": "grok-4.5",
            "tools": [{"type": "function", "name": "run"}],
            "tool_choice": {"type": "function", "name": "run"}
        });
        assert!(!sanitize_xai_responses_request(&mut body));
        assert_eq!(
            body.get("tool_choice").unwrap(),
            &json!({"type": "function", "name": "run"})
        );
    }

    #[test]
    fn keeps_string_tool_choice() {
        let mut body = json!({
            "model": "grok-4.5",
            "tools": [{"type": "function", "name": "run"}],
            "tool_choice": "auto"
        });
        assert!(!sanitize_xai_responses_request(&mut body));
        assert_eq!(body.get("tool_choice").unwrap(), &json!("auto"));
    }

    #[test]
    fn noop_on_clean_request() {
        let mut body = json!({
            "model": "grok-4.5",
            "input": [{"type": "message", "role": "user", "content": "hi"}],
            "tools": [{"type": "function", "name": "f"}]
        });
        assert!(!sanitize_xai_responses_request(&mut body));
    }

    #[test]
    fn idempotent_second_pass() {
        let mut body = json!({
            "model": "grok-4.5",
            "external_web_access": true,
            "prompt_cache_retention": "24h",
            "tools": [{"type": "function", "name": "f"}, {"type": "tool_search"}]
        });
        assert!(sanitize_xai_responses_request(&mut body));
        // second pass finds nothing left to change
        assert!(!sanitize_xai_responses_request(&mut body));
    }

    #[test]
    fn simplifies_flattened_automation_update_one_of_null_root() {
        let mut body = json!({
            "model": "grok-4.6",
            "tools": [{
                "type": "function",
                "name": "mcp__codex_app__automation_update",
                "strict": true,
                "parameters": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {"action": {"type": "string"}},
                            "required": ["action"]
                        },
                        {"type": "null"}
                    ]
                }
            }]
        });

        assert!(sanitize_xai_responses_request(&mut body));

        let tool = &body["tools"][0];
        assert_eq!(tool["strict"], json!(false));
        assert_eq!(
            tool["parameters"],
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        );
    }

    #[test]
    fn simplifies_null_tool_parameters_without_touching_valid_tools() {
        let mut body = json!({
            "model": "grok-4.6",
            "tools": [
                {
                    "type": "function",
                    "name": "codex_app__automation_update",
                    "parameters": null
                },
                {
                    "type": "function",
                    "name": "echo_tool",
                    "parameters": {
                        "type": "object",
                        "properties": {"message": {"type": "string"}}
                    }
                }
            ]
        });

        assert!(sanitize_xai_responses_request(&mut body));

        assert_eq!(
            body["tools"][0]["parameters"],
            json!({"type": "object", "properties": {}, "additionalProperties": true})
        );
        assert_eq!(
            body["tools"][1]["parameters"],
            json!({
                "type": "object",
                "properties": {"message": {"type": "string"}}
            })
        );
    }

    #[test]
    fn automation_update_schema_normalization_is_idempotent() {
        let mut body = json!({
            "model": "grok-4.6",
            "tools": [{
                "type": "function",
                "name": "mcp__codex_app__automation_update",
                "parameters": {
                    "oneOf": [
                        {"type": "object", "properties": {"action": {"type": "string"}}},
                        {"type": "null"}
                    ]
                }
            }]
        });

        assert!(sanitize_xai_responses_request(&mut body));
        assert!(!sanitize_xai_responses_request(&mut body));
    }

    #[test]
    fn whole_floats_92116_and_120000_become_integers() {
        let rewritten = rewrite_whole_float_arguments_json(
            r#"{"session_id":92116.0,"yield_time_ms":120000.0,"wait":1.5}"#,
        )
        .unwrap()
        .unwrap();
        let value: Value = serde_json::from_str(&rewritten).unwrap();
        assert_eq!(value["session_id"].as_i64(), Some(92116));
        assert_eq!(value["yield_time_ms"].as_u64(), Some(120000));
        assert_eq!(value["wait"].as_f64(), Some(1.5));
        assert!(value["wait"].as_i64().is_none());

        let encoded = serde_json::to_string(&value).unwrap();
        assert!(encoded.contains(r#""session_id":92116"#));
        assert!(encoded.contains(r#""yield_time_ms":120000"#));
        assert!(!encoded.contains("92116.0"));
        assert!(!encoded.contains("120000.0"));
        assert!(encoded.contains("1.5"));
    }

    #[test]
    fn xai_integer_precision_preserves_original_decimal_values() {
        let cases = [
            (r#"{"id":9007199254740993.0}"#, r#"{"id":9007199254740993}"#),
            (
                r#"{"value":1.0000000000000001}"#,
                r#"{"value":1.0000000000000001}"#,
            ),
            (
                r#"{"id":-9223372036854775809.0}"#,
                r#"{"id":-9223372036854775809.0}"#,
            ),
            (
                r#"{"id":18446744073709551615.0,"count":1.0}"#,
                r#"{"id":18446744073709551615,"count":1}"#,
            ),
            (r#"{"id":92116.0}"#, r#"{"id":92116}"#),
            (
                r#"{"value":1.0000000000000001,"count":1.0}"#,
                r#"{"value":1.0000000000000001,"count":1}"#,
            ),
            (
                r#"{"id":-9223372036854775809.0,"count":1.0}"#,
                r#"{"id":-9223372036854775809.0,"count":1}"#,
            ),
            (
                r#"{"id":18446744073709551616.0,"count":1.0}"#,
                r#"{"id":18446744073709551616.0,"count":1}"#,
            ),
        ];
        let actual: Vec<_> = cases
            .iter()
            .map(|(input, _)| {
                rewrite_whole_float_arguments_json(input)
                    .unwrap()
                    .unwrap_or_else(|| input.to_string())
            })
            .collect();
        let expected: Vec<_> = cases.iter().map(|(_, expected)| *expected).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn xai_integer_precision_handles_decimal_exponents_and_limits_exactly() {
        for (token, expected) in [
            ("9007199254740993e0", Some("9007199254740993")),
            ("90071992547409930e-1", Some("9007199254740993")),
            ("184467440737095516150e-1", Some("18446744073709551615")),
            ("-92233720368547758080e-1", Some("-9223372036854775808")),
            ("1.0000000000000001e16", Some("10000000000000001")),
            ("1.0000000000000001e15", None),
            ("0.0001e4", Some("1")),
            ("0.0001e3", None),
            ("0.1000e1", Some("1")),
            ("1e+0", Some("1")),
            ("1e-1", None),
            ("0.0", Some("0")),
            ("-0.0", Some("0")),
            ("-9223372036854775809e0", None),
            ("18446744073709551616e0", None),
            ("1e9223372036854775807", None),
            ("1.0e-9223372036854775808", None),
            ("1e999999999999999999999999", None),
            ("9007199254740993", None),
        ] {
            assert_eq!(exact_decimal_integer(token).as_deref(), expected, "{token}");
        }
    }

    #[test]
    fn xai_integer_precision_preserves_strings_whitespace_and_non_target_tokens() {
        let input = r#" { "count": 1.0, "nested": [1.0000000000000001, 2.0], "text": "说明 \"1.0\" \\ 9007199254740993.0", "id":18446744073709551616 } "#;
        let expected = r#" { "count": 1, "nested": [1.0000000000000001, 2], "text": "说明 \"1.0\" \\ 9007199254740993.0", "id":18446744073709551616 } "#;
        let rewritten = rewrite_whole_float_arguments_json(input).unwrap().unwrap();
        assert_eq!(rewritten, expected);
        assert!(rewrite_whole_float_arguments_json(&rewritten)
            .unwrap()
            .is_none());
    }

    #[test]
    fn xai_integer_precision_preserves_extreme_exponents_in_mixed_arguments() {
        let input =
            r#"{"huge":1e999999999999999999999999,"tiny":1e-999999999999999999999999,"count":1.0}"#;
        assert_eq!(
            rewrite_whole_float_arguments_json(input).unwrap().unwrap(),
            r#"{"huge":1e999999999999999999999999,"tiny":1e-999999999999999999999999,"count":1}"#
        );
    }

    #[test]
    fn xai_integer_precision_passes_invalid_number_tokens_through() {
        for invalid in ["01", "1.", "1e+", "+1", "NaN", "--1"] {
            let arguments = format!("{{\"count\":1.0,\"invalid\":{invalid}}}");
            let mut event = json!({
                "type": "response.function_call_arguments.done",
                "arguments": arguments
            });
            assert!(!normalize_xai_function_call_integer_arguments(&mut event));
            assert_eq!(event["arguments"], arguments);
        }
    }

    #[test]
    fn xai_integer_precision_leaves_parsed_non_string_arguments_unchanged() {
        let mut event = json!({
            "type": "response.function_call_arguments.done",
            "arguments": {"id": 9007199254740993.0, "count": 1.0}
        });
        let original = event.clone();
        assert!(!normalize_xai_function_call_integer_arguments(&mut event));
        assert_eq!(event, original);
    }

    #[test]
    fn xai_integer_precision_sse_done_preserves_non_target_number_tokens() {
        let event = json!({
            "type": "response.function_call_arguments.done",
            "arguments": r#"{"id":9007199254740993.0,"fraction":1.0000000000000001,"outside":-9223372036854775809.0,"count":1.0}"#
        });
        let block = format!("event: response.function_call_arguments.done\ndata: {event}");
        let output = rewrite_xai_native_sse_block(&block, &HashMap::new());
        let output = std::str::from_utf8(&output).unwrap();
        let data = output
            .lines()
            .find_map(|line| strip_sse_field(line, "data"))
            .unwrap();
        let rewritten: Value = serde_json::from_str(data).unwrap();
        assert_eq!(
            rewritten["arguments"],
            r#"{"id":9007199254740993,"fraction":1.0000000000000001,"outside":-9223372036854775809.0,"count":1}"#
        );
    }

    #[test]
    fn function_call_arguments_done_rewrites_whole_floats_recursively() {
        let mut event = json!({
            "type": "response.function_call_arguments.done",
            "item_id": "fc_exec",
            "arguments": r#"{"session_id":92116.0,"yield_time_ms":120000.0,"nested":{"n":92116.0},"arr":[120000.0,1.5]}"#
        });

        assert!(normalize_xai_function_call_integer_arguments(&mut event));
        let arguments: Value = serde_json::from_str(event["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["session_id"].as_i64(), Some(92116));
        assert_eq!(arguments["yield_time_ms"].as_u64(), Some(120000));
        assert_eq!(arguments["nested"]["n"].as_i64(), Some(92116));
        assert_eq!(arguments["arr"][0].as_u64(), Some(120000));
        assert_eq!(arguments["arr"][1].as_f64(), Some(1.5));
        assert!(arguments["arr"][1].as_i64().is_none());
    }

    #[test]
    fn completed_function_call_item_rewrites_whole_float_arguments() {
        let mut body = json!({
            "output": [{
                "type": "function_call",
                "name": "write_stdin",
                "arguments": r#"{"session_id":92116.0,"yield_time_ms":120000.0}"#
            }]
        });

        assert!(normalize_xai_function_call_integer_arguments(&mut body));
        let arguments: Value =
            serde_json::from_str(body["output"][0]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["session_id"].as_i64(), Some(92116));
        assert_eq!(arguments["yield_time_ms"].as_u64(), Some(120000));
    }

    #[test]
    fn function_call_argument_deltas_are_not_rewritten() {
        let mut event = json!({
            "type": "response.function_call_arguments.delta",
            "delta": r#"{"session_id":92116.0"#,
            "item": {
                "type": "function_call",
                "arguments": r#"{"session_id":92116.0}"#
            }
        });
        let original = event.clone();
        assert!(!normalize_xai_function_call_integer_arguments(&mut event));
        assert_eq!(event, original);
    }

    #[test]
    fn invalid_function_call_arguments_pass_through() {
        let mut event = json!({
            "type": "response.function_call_arguments.done",
            "arguments": r#"{"session_id":92116.0"#
        });
        assert!(!normalize_xai_function_call_integer_arguments(&mut event));
        assert_eq!(event["arguments"], r#"{"session_id":92116.0"#);
    }

    #[test]
    fn sse_done_event_rewrites_whole_floats_but_delta_bytes_stay_intact() {
        let done = concat!(
            "event: response.function_call_arguments.done\n",
            r#"data: {"type":"response.function_call_arguments.done","arguments":"{\"session_id\":92116.0,\"yield_time_ms\":120000.0}"}"#,
        );
        let rewritten = rewrite_xai_native_sse_block(done, &HashMap::new());
        let rewritten = String::from_utf8(rewritten.to_vec()).unwrap();
        let data = rewritten
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        let event: Value = serde_json::from_str(data).unwrap();
        let arguments: Value = serde_json::from_str(event["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["session_id"].as_i64(), Some(92116));
        assert_eq!(arguments["yield_time_ms"].as_u64(), Some(120000));
        assert!(!rewritten.contains("92116.0"));
        assert!(!rewritten.contains("120000.0"));

        let delta = concat!(
            "event: response.function_call_arguments.delta\n",
            r#"data: {"type":"response.function_call_arguments.delta","delta":"{\"session_id\":92116.0"}"#,
        );
        let passed = rewrite_xai_native_sse_block(delta, &HashMap::new());
        assert_eq!(
            String::from_utf8(passed.to_vec()).unwrap(),
            format!("{delta}\n\n")
        );
    }

    #[test]
    fn xai_sse_metadata_preserves_non_data_lines_for_integer_rewrites() {
        let original = json!({
            "type": "response.function_call_arguments.done",
            "arguments": "{\"count\":2.0}"
        });
        let rewritten = json!({
            "type": "response.function_call_arguments.done",
            "arguments": "{\"count\":2}"
        });
        for newline in ["\n", "\r\n"] {
            let prefix = [
                "id: cursor-42",
                "retry: 5000",
                ": vendor-comment",
                "event:  response.function_call_arguments.done",
                "x-vendor: retained",
            ]
            .join(newline);
            let block = format!("{prefix}{newline}data: {original}{newline}: after-data");
            let output = rewrite_xai_native_sse_block(&block, &HashMap::new());
            assert_eq!(
                std::str::from_utf8(&output).unwrap(),
                format!(
                    "{prefix}{newline}data: {rewritten}{newline}: after-data{newline}{newline}"
                )
            );
        }
    }

    #[test]
    fn xai_sse_metadata_preserves_interleaved_fields_with_multiple_data_lines() {
        let block = concat!(
            "event: response.function_call_arguments.done\r\n",
            "data: {\"type\":\"response.function_call_arguments.done\",\r\n",
            "id: cursor-43\r\n",
            ": between-data\r\n",
            "retry: 4500\r\n",
            r#"data: "arguments":"{\"count\":2.0}"}"#,
            "\r\n",
            "x-vendor: retained"
        );
        let rewritten = json!({
            "type": "response.function_call_arguments.done",
            "arguments": "{\"count\":2}"
        });
        let output = rewrite_xai_native_sse_block(block, &HashMap::new());
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            format!(
                "event: response.function_call_arguments.done\r\ndata: {rewritten}\r\nid: cursor-43\r\n: between-data\r\nretry: 4500\r\nx-vendor: retained\r\n\r\n"
            )
        );
    }

    #[test]
    fn xai_sse_metadata_preserves_fields_for_namespace_only_rewrites() {
        let original = json!({"type": "response.output_item.done", "item": {
            "type": "function_call", "name": "mcp__files____read", "arguments": "{}"
        }});
        let rewritten = json!({"type": "response.output_item.done", "item": {
            "type": "function_call", "name": "read", "arguments": "{}", "namespace": "mcp__files__"
        }});
        let map = HashMap::from([(
            "mcp__files____read".into(),
            NamespacedName {
                namespace: "mcp__files__".into(),
                name: "read".into(),
            },
        )]);
        let block =
            format!("id: namespace-cursor\nretry: 3000\n: namespace-comment\ndata: {original}");
        let output = rewrite_xai_native_sse_block(&block, &map);
        assert_eq!(
            std::str::from_utf8(&output).unwrap(),
            format!(
                "id: namespace-cursor\nretry: 3000\n: namespace-comment\ndata: {rewritten}\n\n"
            )
        );
    }

    #[test]
    fn xai_sse_metadata_preserves_unchanged_frames() {
        for data in [
            r#"{"type":"response.function_call_arguments.delta","delta":"{\"count\":2.0"}"#,
            r#"{"type":"response.function_call_arguments.done","arguments":"{\"count\":2}"}"#,
            "not-json",
            "[DONE]",
        ] {
            let block =
                format!("id: unchanged-cursor\nretry: 1500\n: untouched-comment\ndata: {data}");
            let output = rewrite_xai_native_sse_block(&block, &HashMap::new());
            assert_eq!(
                std::str::from_utf8(&output).unwrap(),
                format!("{block}\n\n")
            );
        }
    }

    #[tokio::test]
    async fn xai_sse_metadata_preserves_complete_trailing_frames() {
        let original = json!({
            "type": "response.function_call_arguments.done",
            "arguments": "{\"count\":2.0,\"label\":\"说明\"}"
        });
        let rewritten = json!({
            "type": "response.function_call_arguments.done",
            "arguments": "{\"count\":2,\"label\":\"说明\"}"
        });
        for (newline, tail_ending) in [("\n", ""), ("\r\n", "\r\n")] {
            let prefix =
                format!("id: tail-cursor{newline}: tail-comment{newline}retry: 2500{newline}");
            let wire = format!(": keepalive\n\n{prefix}data: {original}{tail_ending}");
            let chunks: Vec<_> = wire
                .bytes()
                .map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])))
                .collect();
            let output = create_xai_native_responses_sse_stream(
                futures::stream::iter(chunks),
                HashMap::new(),
            );
            futures::pin_mut!(output);
            assert_eq!(
                output.next().await.unwrap().unwrap(),
                Bytes::from_static(b": keepalive\n\n")
            );
            let tail = output.next().await.unwrap().unwrap();
            assert_eq!(
                std::str::from_utf8(&tail).unwrap(),
                format!("{prefix}data: {rewritten}{newline}{newline}")
            );
            assert!(output.next().await.is_none());
        }
    }

    #[test]
    fn rewrites_agent_message_new_task_with_encrypted_content_part() {
        let mut body = json!({
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "review"}]},
                {
                    "type": "agent_message",
                    "id": "amsg_child",
                    "author": "/root",
                    "recipient": "/root/code_review",
                    "content": [
                        {"type": "input_text", "text": "Message Type: NEW_TASK\nPayload:\n"},
                        {"type": "encrypted_content", "encrypted_content": "You are a Senior Code Reviewer."}
                    ]
                }
            ]
        });

        assert!(!sanitize_xai_responses_request(&mut body));
        assert!(rewrite_xai_agent_message_input_items(&mut body));
        assert_eq!(body["input"][0]["type"], "message");
        let item = &body["input"][1];
        assert_eq!(item["type"], "message");
        assert_eq!(item["role"], "user");
        assert_eq!(item["id"], "amsg_child");
        assert_eq!(item["content"][0]["type"], "input_text");
        assert_eq!(
            item["content"][0]["text"],
            "Message Type: NEW_TASK\nPayload:\n"
        );
        assert_eq!(
            item["content"][1]["text"],
            "You are a Senior Code Reviewer."
        );
        assert!(item.get("author").is_none());
        assert!(!rewrite_xai_agent_message_input_items(&mut body));
    }

    #[test]
    fn rewrites_agent_message_final_answer_without_dropping_neighbors() {
        let mut body = json!({
            "input": [
                {
                    "type": "function_call",
                    "name": "wait_agent",
                    "arguments": "{\"timeout_ms\":180000}"
                },
                {
                    "type": "agent_message",
                    "id": "amsg_parent",
                    "author": "/root/code_review",
                    "recipient": "/root",
                    "content": [{
                        "type": "input_text",
                        "text": "Message Type: FINAL_ANSWER\nPayload:\nAgent errored: unexpected status 422"
                    }]
                }
            ]
        });

        assert!(rewrite_xai_agent_message_input_items(&mut body));
        assert_eq!(body["input"][0]["type"], "function_call");
        assert_eq!(body["input"][0]["name"], "wait_agent");
        assert_eq!(body["input"][1]["type"], "message");
        assert_eq!(body["input"][1]["role"], "user");
        assert!(body["input"][1]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("FINAL_ANSWER"));
    }

    #[test]
    fn leaves_ordinary_messages_unchanged() {
        let mut body = json!({
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            }]
        });
        let original = body.clone();
        assert!(!rewrite_xai_agent_message_input_items(&mut body));
        assert_eq!(body, original);
    }

    #[test]
    fn rewrites_agent_message_at_input_index_matching_xai_422() {
        let mut body = json!({
            "model": "grok-4.6",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "go"}]},
                {"type": "function_call", "name": "spawn_agent", "arguments": "{}"},
                {"type": "function_call_output", "call_id": "c1", "output": "ok"},
                {"type": "reasoning", "summary": []},
                {
                    "type": "agent_message",
                    "id": "amsg_child",
                    "author": "/root",
                    "recipient": "/root/review_uncommitted",
                    "content": [
                        {"type": "input_text", "text": "Message Type: NEW_TASK\nPayload:\n"},
                        {"type": "encrypted_content", "encrypted_content": "Review the diff."}
                    ]
                }
            ]
        });

        assert!(rewrite_xai_agent_message_input_items(&mut body));
        assert_eq!(body["input"][4]["type"], "message");
        assert_eq!(body["input"][4]["role"], "user");
        assert_eq!(body["input"][4]["content"][1]["text"], "Review the diff.");
        assert!(body["input"][4].get("author").is_none());
        assert_eq!(body["input"][0]["type"], "message");
        assert_eq!(body["input"][1]["type"], "function_call");
    }

    #[test]
    fn rewrites_nested_agent_message_inside_object_wrapper() {
        let mut body = json!({
            "input": {
                "items": [{
                    "type": "agent_message",
                    "content": [{"type": "input_text", "text": "nested"}]
                }]
            }
        });
        assert!(rewrite_xai_agent_message_input_items(&mut body));
        assert_eq!(body["input"]["items"][0]["type"], "message");
        assert_eq!(body["input"]["items"][0]["role"], "user");
    }

    #[test]
    fn request_compat_rewrites_agent_message_and_unknown_model_together() {
        let mut body = json!({
            "model": "gpt-5.6-sol",
            "input": [{
                "type": "agent_message",
                "content": [{"type": "input_text", "text": "hi"}]
            }]
        });
        let settings = json!({
            "modelCatalog": {"models": [{"slug": "grok-4.6"}, {"slug": "grok-4.5"}]}
        });
        apply_xai_native_responses_request_compat(&mut body, "grok", Some("grok-4.6"), &settings);
        assert_eq!(body["model"], "grok-4.6");
        assert_eq!(body["input"][0]["type"], "message");
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["input"][0]["content"][0]["text"], "hi");
    }

    #[test]
    fn remaps_unknown_openai_role_model_to_upstream() {
        let allowed = collect_xai_catalog_model_ids(&json!({
            "modelCatalog": {
                "models": [
                    {"slug": "grok-4.6"},
                    {"slug": "grok-4.5"}
                ]
            }
        }));
        let mut body = json!({"model": "gpt-5.6-sol", "input": []});
        assert_eq!(
            rewrite_xai_unknown_request_model(&mut body, "grok-4.6", &allowed),
            Some(("gpt-5.6-sol".to_string(), "grok-4.6".to_string()))
        );
        assert_eq!(body["model"], "grok-4.6");
    }

    #[test]
    fn preserves_catalog_slug_instead_of_forcing_upstream() {
        let allowed = collect_xai_catalog_model_ids(&json!({
            "modelCatalog": {
                "models": [
                    {"slug": "grok-4.6"},
                    {"slug": "grok-4.5"}
                ]
            }
        }));
        let mut body = json!({"model": "grok-4.5"});
        assert_eq!(
            rewrite_xai_unknown_request_model(&mut body, "grok-4.6", &allowed),
            None
        );
        assert_eq!(body["model"], "grok-4.5");
    }

    #[test]
    fn fills_missing_model_with_upstream() {
        let allowed = HashSet::new();
        let mut body = json!({"input": []});
        assert_eq!(
            rewrite_xai_unknown_request_model(&mut body, "grok-4.6", &allowed),
            Some(("".to_string(), "grok-4.6".to_string()))
        );
        assert_eq!(body["model"], "grok-4.6");
    }

    #[test]
    fn leaves_model_unchanged_when_upstream_is_empty() {
        let allowed = HashSet::new();
        let mut body = json!({"model": "gpt-5.6-sol"});
        assert_eq!(
            rewrite_xai_unknown_request_model(&mut body, "  ", &allowed),
            None
        );
        assert_eq!(body["model"], "gpt-5.6-sol");
    }
    #[test]
    fn xai_native_responses_schema_regression() {
        let mut body = json!({
            "model": "grok-4.6",
            "tools": [{
                "type": "function",
                "name": "mcp__codex_app__automation_update",
                "strict": true,
                "parameters": {"oneOf": [
                    {"type": "object", "properties": {"name": {"type": "string"}}},
                    {"type": "null"}
                ]}
            }]
        });

        assert!(sanitize_xai_responses_request(&mut body));
        assert_eq!(body["tools"][0]["parameters"]["type"], "object");
        assert!(body["tools"][0]["parameters"].get("oneOf").is_none());
        assert_eq!(body["tools"][0]["strict"], false);
    }

    #[test]
    fn nested_function_parameters_are_rewritten_in_place() {
        let mut body = json!({"tools": [{
            "type": "function",
            "function": {"name": "run", "parameters": null}
        }]});
        assert!(sanitize_xai_responses_request(&mut body));
        assert!(body["tools"][0].get("parameters").is_none());
        assert_eq!(body["tools"][0]["function"]["parameters"]["type"], "object");
        assert!(!sanitize_xai_responses_request(&mut body));
    }

    #[test]
    fn model_alias_is_resolved_before_model_specific_fields() {
        let mut body = json!({"model": "gpt-5.6-sol", "stop": ["end"], "presence_penalty": 0.5});
        apply_xai_native_responses_request_compat(
            &mut body,
            "fixture",
            Some("grok-4.5"),
            &json!({}),
        );
        assert_eq!(body["model"], "grok-4.5");
        assert!(body.get("stop").is_none());
        assert!(body.get("presence_penalty").is_none());
        let once = body.clone();
        apply_xai_native_responses_request_compat(
            &mut body,
            "fixture",
            Some("grok-4.5"),
            &json!({}),
        );
        assert_eq!(body, once);
    }

    #[test]
    fn whole_float_integer_bounds_do_not_saturate() {
        let input = "[18446744073709551616.0,18446744073709549568.0,-9223372036854775808.0,-9223372036854777856.0,1.25]";
        let rewritten = rewrite_whole_float_arguments_json(input).unwrap().unwrap();
        assert_eq!(rewritten, "[18446744073709551616.0,18446744073709549568,-9223372036854775808,-9223372036854777856.0,1.25]");
        assert!(rewrite_whole_float_arguments_json(&rewritten)
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn native_sse_handles_split_utf8_crlf_and_trailing_done() {
        let delta = concat!(
            "event: response.function_call_arguments.delta\r\n",
            "data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"count\\\":2.0\"}\r\n\r\n"
        );
        let item = json!({"type": "response.output_item.done", "item": {
            "type": "function_call", "name": "mcp__files____read", "call_id": "call-1",
            "arguments": "{\"count\":2.0,\"label\":\"说明\"}"
        }});
        let wire =
            format!("{delta}event: response.output_item.done\r\ndata: {item}\r\n\r\ndata: [DONE]");
        let chunks: Vec<_> = wire
            .bytes()
            .map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])))
            .collect();
        let map = HashMap::from([(
            "mcp__files____read".into(),
            NamespacedName {
                namespace: "mcp__files__".into(),
                name: "read".into(),
            },
        )]);
        let output = create_xai_native_responses_sse_stream(futures::stream::iter(chunks), map);
        futures::pin_mut!(output);
        let mut collected = String::new();
        while let Some(chunk) = output.next().await {
            collected.push_str(std::str::from_utf8(&chunk.unwrap()).unwrap());
        }
        assert!(collected.contains(r#""delta":"{\"count\":2.0""#));
        assert!(collected.contains("说明"));
        let events: Vec<Value> = collected
            .lines()
            .filter_map(|line| strip_sse_field(line, "data"))
            .filter_map(|data| serde_json::from_str(data).ok())
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1]["item"]["name"], "read");
        assert_eq!(events[1]["item"]["namespace"], "mcp__files__");
        let arguments: Value =
            serde_json::from_str(events[1]["item"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(arguments["count"].as_u64(), Some(2));
        assert_eq!(arguments["label"], "说明");
        assert!(collected.ends_with("data: [DONE]\n\n"));
    }

    #[tokio::test]
    async fn native_sse_preserves_malformed_event_and_upstream_failure() {
        let wire = "event: response.output_item.done\ndata: not-json\n\n";
        let chunks = vec![
            Ok(Bytes::from_static(wire.as_bytes())),
            Err(std::io::Error::other("fixture transport failure")),
        ];
        let output =
            create_xai_native_responses_sse_stream(futures::stream::iter(chunks), HashMap::new());
        futures::pin_mut!(output);
        assert_eq!(
            output.next().await.unwrap().unwrap(),
            Bytes::from_static(wire.as_bytes())
        );
        assert_eq!(
            output.next().await.unwrap().unwrap_err().to_string(),
            "fixture transport failure"
        );
        assert!(output.next().await.is_none());
    }
}
