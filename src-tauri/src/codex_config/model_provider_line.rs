//! Top-level `model_provider` comment/uncomment.
//!
//! Official ChatGPT login uses Codex's built-in openai route, which is the
//! default when the top-level key is absent. Unofficial routing keeps the
//! user's `[model_providers.*]` tables and only toggles the selector line so
//! the previous provider id can be restored.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineAction {
    Comment,
    Uncomment,
}

/// Comment the first top-level `model_provider = …` assignment.
/// Returns `None` when the live document already has no active selector.
pub(crate) fn comment_top_level_model_provider(config: &str) -> Option<String> {
    rewrite_top_level_model_provider_line(config, LineAction::Comment)
}

/// Uncomment the first top-level `#model_provider = …` assignment.
/// Returns `None` when no commented selector exists before the first table.
pub(crate) fn uncomment_top_level_model_provider(config: &str) -> Option<String> {
    rewrite_top_level_model_provider_line(config, LineAction::Uncomment)
}

pub(crate) fn top_level_model_provider_is_active(config: &str) -> bool {
    scan_top_level_model_provider_line(config).is_some_and(|active| active)
}

fn rewrite_top_level_model_provider_line(config: &str, action: LineAction) -> Option<String> {
    let mut in_table = false;
    let mut changed = false;
    let mut out = String::with_capacity(config.len().saturating_add(1));
    for (content, ending) in lines_with_endings(config) {
        let trimmed = content.trim_start();
        if !in_table && is_table_header(trimmed) {
            in_table = true;
        }
        if !changed && !in_table {
            if let Some(rewritten) = rewrite_line(content, action) {
                out.push_str(&rewritten);
                out.push_str(ending);
                changed = true;
                continue;
            }
        }
        out.push_str(content);
        out.push_str(ending);
    }
    changed.then_some(out)
}

fn scan_top_level_model_provider_line(config: &str) -> Option<bool> {
    let mut in_table = false;
    for (content, _) in lines_with_endings(config) {
        let trimmed = content.trim_start();
        if !in_table && is_table_header(trimmed) {
            in_table = true;
        }
        if in_table {
            continue;
        }
        if is_active_assignment(trimmed) {
            return Some(true);
        }
        if uncommented_assignment(trimmed).is_some() {
            return Some(false);
        }
    }
    None
}

fn is_table_header(trimmed: &str) -> bool {
    !trimmed.starts_with('#') && trimmed.starts_with('[')
}

fn rewrite_line(content: &str, action: LineAction) -> Option<String> {
    let indent_len = content.len() - content.trim_start().len();
    let indent = &content[..indent_len];
    let trimmed = content.trim_start();
    match action {
        LineAction::Comment => {
            if is_active_assignment(trimmed) {
                Some(format!("{indent}#{trimmed}"))
            } else {
                None
            }
        }
        LineAction::Uncomment => {
            uncommented_assignment(trimmed).map(|assignment| format!("{indent}{assignment}"))
        }
    }
}

fn is_active_assignment(trimmed: &str) -> bool {
    let Some(after) = trimmed.strip_prefix("model_provider") else {
        return false;
    };
    after.trim_start().starts_with('=')
}

fn uncommented_assignment(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix('#')?;
    let assignment = rest.trim_start();
    is_active_assignment(assignment).then_some(assignment)
}

fn lines_with_endings(config: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut remaining = config;
    std::iter::from_fn(move || {
        if remaining.is_empty() {
            return None;
        }
        let split = remaining
            .find('\n')
            .map_or(remaining.len(), |index| index + 1);
        let chunk = &remaining[..split];
        remaining = &remaining[split..];
        let ending = if chunk.ends_with("\r\n") {
            "\r\n"
        } else if chunk.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        Some((&chunk[..chunk.len() - ending.len()], ending))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_only_the_top_level_selector_and_preserves_provider_tables() {
        let input = "model = \"gpt\"\nmodel_provider = \"OpenAI\"\n\n[model_providers.OpenAI]\nbase_url = \"https://example.test/v1\"\n";
        let output = comment_top_level_model_provider(input).unwrap();
        assert_eq!(
            output,
            "model = \"gpt\"\n#model_provider = \"OpenAI\"\n\n[model_providers.OpenAI]\nbase_url = \"https://example.test/v1\"\n"
        );
        assert!(uncomment_top_level_model_provider(&output).is_some());
        assert!(comment_top_level_model_provider(&output).is_none());
        assert!(!top_level_model_provider_is_active(&output));
    }

    #[test]
    fn uncomments_hash_without_space_and_keeps_crlf() {
        let input = "#model_provider = \"OpenAI\"\r\nmodel = \"gpt\"\r\n";
        let output = uncomment_top_level_model_provider(input).unwrap();
        assert_eq!(output, "model_provider = \"OpenAI\"\r\nmodel = \"gpt\"\r\n");
        assert!(top_level_model_provider_is_active(&output));
        assert_eq!(
            comment_top_level_model_provider(&output).as_deref(),
            Some(input)
        );
    }

    #[test]
    fn ignores_selector_lines_inside_tables() {
        let input = "[model_providers.OpenAI]\nmodel_provider = \"ignored\"\n";
        assert!(comment_top_level_model_provider(input).is_none());
        assert!(uncomment_top_level_model_provider(
            "[model_providers.OpenAI]\n#model_provider = \"ignored\"\n"
        )
        .is_none());
    }

    #[test]
    fn comments_user_codex_openai_selector_without_touching_the_table() {
        let input = concat!(
            "model_provider = \"OpenAI\"\n",
            "model = \"gpt-5.6-luna\"\n",
            "\n",
            "[model_providers.OpenAI]\n",
            "name = \"OpenAI\"\n",
            "base_url = \"https://example.test/v1\"\n",
        );
        let output = comment_top_level_model_provider(input).unwrap();
        assert!(output.starts_with("#model_provider = \"OpenAI\"\n"));
        assert!(output.contains("[model_providers.OpenAI]"));
        assert!(output.contains("base_url = \"https://example.test/v1\""));
        assert_eq!(
            uncomment_top_level_model_provider(&output).as_deref(),
            Some(input)
        );
    }
}
