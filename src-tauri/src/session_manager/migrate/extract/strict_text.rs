//! Block-level strict text extraction.
//!
//! Runs parallel to [`crate::session_manager::providers::utils::extract_text`]
//! and deliberately does not reuse it: that function is the display contract
//! and synthesizes `[Tool: name]` text plus inlined tool results, which must
//! never reach a migration package. The existing readers keep their behavior
//! unchanged.
//!
//! This is a block projection only, not a final-answer classifier. A caller
//! must establish role, provenance and finality before using its text, and
//! reject unknown block types rather than silently accepting a partial body.

use serde_json::Value;

use crate::session_manager::migrate::model::OmittedCounts;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrictText {
    /// Kept fragments concatenated in source order, without invented separators.
    pub text: String,
    pub omitted: OmittedCounts,
    /// Block types that were neither kept nor recognized. A rule may turn this
    /// into an `Indeterminate` verdict instead of silently dropping content.
    pub unknown_types: Vec<String>,
    /// True when at least one kept fragment came from a recognized text block.
    pub has_text_block: bool,
}

const TOOL_TYPES: &[&str] = &[
    "tool_use",
    "tool_call",
    "tool-call",
    "function_call",
    "tool_result",
    "tool-result",
    "function_call_output",
    "tool-invocation",
    "local_shell_call",
    "local_shell_call_output",
    "custom_tool_call",
    "custom_tool_call_output",
    "web_search_call",
];

const REASONING_TYPES: &[&str] = &["thinking", "reasoning", "redacted_thinking", "thought"];

const ATTACHMENT_TYPES: &[&str] = &[
    "image",
    "input_image",
    "file",
    "input_file",
    "document",
    "attachment",
    "audio",
    "input_audio",
];

const TEXT_TYPES: &[&str] = &["text", "input_text", "output_text", "summary_text"];

pub fn extract_text_strict(content: &Value) -> StrictText {
    let mut out = StrictText::default();
    let mut fragments: Vec<String> = Vec::new();
    visit(content, &mut out, &mut fragments);
    out.text = fragments.concat();
    out
}

fn visit(content: &Value, out: &mut StrictText, fragments: &mut Vec<String>) {
    match content {
        // A bare string body carries no block structure to inspect. Keep it
        // verbatim; the provider rule decides whether that role is allowed to
        // contribute a message at all.
        Value::String(text) => {
            if !text.is_empty() {
                fragments.push(text.clone());
            }
        }
        Value::Array(items) => {
            for item in items {
                visit_block(item, out, fragments);
            }
        }
        Value::Object(_) => visit_block(content, out, fragments),
        Value::Null => {}
        other => {
            out.omitted.unknown_blocks += 1;
            out.unknown_types.push(kind_name(other).to_string());
        }
    }
}

fn visit_block(item: &Value, out: &mut StrictText, fragments: &mut Vec<String>) {
    let Some(object) = item.as_object() else {
        if let Value::String(text) = item {
            if !text.is_empty() {
                fragments.push(text.clone());
            }
            return;
        }
        out.omitted.unknown_blocks += 1;
        out.unknown_types.push(kind_name(item).to_string());
        return;
    };

    let block_type = object.get("type").and_then(Value::as_str);
    match block_type {
        Some(kind) if TEXT_TYPES.contains(&kind) => {
            out.has_text_block = true;
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                if !text.is_empty() {
                    fragments.push(text.to_string());
                }
            } else {
                // A recognized text block with no string payload is not
                // "empty text", it is a shape we do not understand.
                out.omitted.unknown_blocks += 1;
                out.unknown_types.push(format!("{kind}:no-text"));
            }
        }
        Some(kind) if TOOL_TYPES.contains(&kind) => out.omitted.tool_events += 1,
        Some(kind) if REASONING_TYPES.contains(&kind) => out.omitted.reasoning_blocks += 1,
        Some(kind) if ATTACHMENT_TYPES.contains(&kind) => out.omitted.attachments += 1,
        Some(kind) => {
            out.omitted.unknown_blocks += 1;
            out.unknown_types.push(kind.to_string());
        }
        None => {
            // Typeless object with a string `text` is the shape the existing
            // readers already accept; anything else stays unknown.
            match object.get("text").and_then(Value::as_str) {
                Some(text) => {
                    out.has_text_block = true;
                    if !text.is_empty() {
                        fragments.push(text.to_string());
                    }
                }
                None => {
                    out.omitted.unknown_blocks += 1;
                    out.unknown_types.push("untyped".to_string());
                }
            }
        }
    }
}

fn kind_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_text_blocks_and_drops_tool_blocks_from_the_same_message() {
        let content = json!([
            {"type": "text", "text": "Here is the plan."},
            {"type": "tool_use", "name": "shell", "input": {"cmd": "ls"}},
            {"type": "text", "text": "Done."}
        ]);
        let strict = extract_text_strict(&content);
        assert_eq!(strict.text, "Here is the plan.Done.");
        assert_eq!(strict.omitted.tool_events, 1);
        assert_eq!(strict.omitted.unknown_blocks, 0);
    }

    #[test]
    fn never_inlines_tool_results() {
        // `utils::extract_text` recursively inlines tool_result content; a
        // migration package must not carry it at all.
        let content = json!([
            {"type": "tool_result", "content": [{"type": "text", "text": "SECRET OUTPUT"}]}
        ]);
        let strict = extract_text_strict(&content);
        assert_eq!(strict.text, "");
        assert!(!strict.text.contains("SECRET"));
        assert_eq!(strict.omitted.tool_events, 1);
    }

    #[test]
    fn never_synthesizes_a_tool_name_marker() {
        let content = json!([{"type": "tool_use", "name": "shell"}]);
        let strict = extract_text_strict(&content);
        assert!(!strict.text.contains("[Tool:"));
        assert!(strict.text.is_empty());
    }

    #[test]
    fn counts_reasoning_and_attachments_separately() {
        let content = json!([
            {"type": "thinking", "thinking": "hidden"},
            {"type": "reasoning", "summary": []},
            {"type": "image", "source": {}},
            {"type": "input_file", "path": "a.pdf"},
            {"type": "output_text", "text": "visible"}
        ]);
        let strict = extract_text_strict(&content);
        assert_eq!(strict.text, "visible");
        assert_eq!(strict.omitted.reasoning_blocks, 2);
        assert_eq!(strict.omitted.attachments, 2);
    }

    #[test]
    fn unknown_block_types_are_reported_not_hidden() {
        let content = json!([{"type": "step-start"}, {"type": "text", "text": "hi"}]);
        let strict = extract_text_strict(&content);
        assert_eq!(strict.text, "hi");
        assert_eq!(strict.omitted.unknown_blocks, 1);
        assert_eq!(strict.unknown_types, vec!["step-start".to_string()]);
    }

    #[test]
    fn plain_string_content_is_kept_verbatim() {
        let strict = extract_text_strict(&json!("  spaced  \n\n body  "));
        assert_eq!(strict.text, "  spaced  \n\n body  ");
        assert_eq!(strict.omitted, OmittedCounts::default());
    }

    #[test]
    fn text_is_never_trimmed_or_reflowed() {
        let content = json!([{"type": "text", "text": "line1\n\n\n   line2   "}]);
        assert_eq!(extract_text_strict(&content).text, "line1\n\n\n   line2   ");
    }
}
