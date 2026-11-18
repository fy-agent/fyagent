use crate::error::AppError;
use serde::Deserialize;
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};

use super::{OmpModelInput, OmpProviderPatch};

pub(super) fn parse_yaml_strict(content: &str, label: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Ok(());
    }
    for doc in serde_yaml::Deserializer::from_str(content) {
        YamlValue::deserialize(doc)
            .map_err(|e| AppError::Config(format!("{label} YAML parse failed: {e}")))?;
    }
    Ok(())
}

fn parse_single_root_mapping(content: &str, label: &str) -> Result<YamlMapping, AppError> {
    if content.trim().is_empty() {
        return Ok(YamlMapping::new());
    }
    let mut docs = Vec::new();
    for doc in serde_yaml::Deserializer::from_str(content) {
        let val = YamlValue::deserialize(doc)
            .map_err(|e| AppError::Config(format!("{label} YAML parse failed: {e}")))?;
        docs.push(val);
    }
    if docs.len() > 1 {
        return Err(AppError::InvalidInput(format!(
            "{label} must contain a single YAML document"
        )));
    }
    match docs.into_iter().next().unwrap_or(YamlValue::Null) {
        YamlValue::Null => Ok(YamlMapping::new()),
        YamlValue::Mapping(map) => Ok(map),
        YamlValue::Tagged(tagged) => match tagged.value {
            YamlValue::Null => Ok(YamlMapping::new()),
            YamlValue::Mapping(map) => Ok(map),
            _ => Err(AppError::InvalidInput(format!(
                "{label} root must be a mapping"
            ))),
        },
        _ => Err(AppError::InvalidInput(format!(
            "{label} root must be a mapping"
        ))),
    }
}

pub(super) fn patch_provider_in_yaml(
    content: &str,
    patch: &OmpProviderPatch,
) -> Result<String, AppError> {
    let mut root = parse_single_root_mapping(content, "omp models")?;
    validate_and_merge_provider_ast(&mut root, patch)?;

    let output = if can_block_patch_models(content, &patch.id) {
        block_patch_provider(content, patch).unwrap_or_else(|| {
            serialize_mapping_preserving_header(content, &root, "omp models").unwrap_or_default()
        })
    } else {
        serialize_mapping_preserving_header(content, &root, "omp models")?
    };

    parse_yaml_strict(&output, "patched omp models")?;
    Ok(output)
}

pub(super) fn patch_default_model_in_yaml(
    content: &str,
    default_model: &str,
) -> Result<String, AppError> {
    let mut root = parse_single_root_mapping(content, "omp config")?;
    validate_and_merge_default_role_ast(&mut root, default_model)?;

    let output = if can_block_patch_config(content) {
        block_patch_default_role(content, default_model).unwrap_or_else(|| {
            serialize_mapping_preserving_header(content, &root, "omp config").unwrap_or_default()
        })
    } else {
        serialize_mapping_preserving_header(content, &root, "omp config")?
    };

    parse_yaml_strict(&output, "patched omp config")?;
    Ok(output)
}

fn validate_and_merge_provider_ast(
    root: &mut YamlMapping,
    patch: &OmpProviderPatch,
) -> Result<(), AppError> {
    let providers_key = YamlValue::String("providers".to_string());
    if !root.contains_key(&providers_key) {
        root.insert(providers_key.clone(), YamlValue::Mapping(YamlMapping::new()));
    }
    let providers = root
        .get_mut(&providers_key)
        .and_then(YamlValue::as_mapping_mut)
        .ok_or_else(|| {
            AppError::InvalidInput("omp models providers node must be a mapping".to_string())
        })?;

    let id_key = YamlValue::String(patch.id.clone());
    if !providers.contains_key(&id_key) {
        providers.insert(id_key.clone(), YamlValue::Mapping(YamlMapping::new()));
    }
    let provider = providers
        .get_mut(&id_key)
        .and_then(YamlValue::as_mapping_mut)
        .ok_or_else(|| {
            AppError::InvalidInput(format!(
                "omp provider '{}' must be a mapping before it can be edited",
                patch.id
            ))
        })?;

    provider.insert(
        YamlValue::String("baseUrl".to_string()),
        YamlValue::String(patch.base_url.clone()),
    );
    provider.insert(
        YamlValue::String("api".to_string()),
        YamlValue::String(patch.api.clone()),
    );
    if let Some(auth) = &patch.auth {
        provider.insert(
            YamlValue::String("auth".to_string()),
            YamlValue::String(auth.clone()),
        );
    }
    if let Some(api_key) = &patch.api_key {
        provider.insert(
            YamlValue::String("apiKey".to_string()),
            YamlValue::String(api_key.clone()),
        );
    }

    if !patch.models.is_empty() {
        let models_key = YamlValue::String("models".to_string());
        if !provider.contains_key(&models_key) {
            provider.insert(models_key.clone(), YamlValue::Sequence(Vec::new()));
        }
        let seq = provider
            .get_mut(&models_key)
            .and_then(YamlValue::as_sequence_mut)
            .ok_or_else(|| {
                AppError::InvalidInput("omp provider models node must be a sequence".to_string())
            })?;

        for input in &patch.models {
            let existing = seq.iter_mut().find(|item| {
                item.as_mapping()
                    .and_then(|m| m.get(YamlValue::String("id".to_string())))
                    .and_then(YamlValue::as_str)
                    == Some(input.id.as_str())
            });
            match existing {
                Some(item) => {
                    let map = item.as_mapping_mut().ok_or_else(|| {
                        AppError::Config(format!("omp model '{}' is not a mapping", input.id))
                    })?;
                    if let Some(name) = &input.name {
                        map.insert(
                            YamlValue::String("name".to_string()),
                            YamlValue::String(name.clone()),
                        );
                    }
                }
                None => {
                    let mut map = YamlMapping::new();
                    map.insert(
                        YamlValue::String("id".to_string()),
                        YamlValue::String(input.id.clone()),
                    );
                    if let Some(name) = &input.name {
                        map.insert(
                            YamlValue::String("name".to_string()),
                            YamlValue::String(name.clone()),
                        );
                    }
                    seq.push(YamlValue::Mapping(map));
                }
            }
        }
    }
    Ok(())
}

fn validate_and_merge_default_role_ast(
    root: &mut YamlMapping,
    default_model: &str,
) -> Result<(), AppError> {
    let roles_key = YamlValue::String("modelRoles".to_string());
    if !root.contains_key(&roles_key) {
        root.insert(roles_key.clone(), YamlValue::Mapping(YamlMapping::new()));
    }
    let roles = root
        .get_mut(&roles_key)
        .and_then(YamlValue::as_mapping_mut)
        .ok_or_else(|| {
            AppError::InvalidInput("omp config modelRoles node must be a mapping".to_string())
        })?;
    roles.insert(
        YamlValue::String("default".to_string()),
        YamlValue::String(default_model.to_string()),
    );
    Ok(())
}

fn serialize_mapping_preserving_header(
    original: &str,
    root: &YamlMapping,
    label: &str,
) -> Result<String, AppError> {
    let mut header = String::new();
    for line in original.lines() {
        if is_blank_or_comment(line) {
            header.push_str(line);
            header.push('\n');
        } else {
            break;
        }
    }
    let body = serde_yaml::to_string(&YamlValue::Mapping(root.clone()))
        .map_err(|e| AppError::Config(format!("{label} YAML serialize failed: {e}")))?;
    Ok(format!("{header}{body}"))
}

// ── Block-style comment-preserving line patcher ────────────────────────────

fn is_flow_root(content: &str) -> bool {
    for line in content.lines() {
        let trimmed = strip_unquoted_comment(line).trim();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        return trimmed.starts_with('{') || trimmed.starts_with('[');
    }
    false
}

fn can_block_patch_models(content: &str, provider_id: &str) -> bool {
    if content.trim().is_empty() || is_flow_root(content) {
        return false;
    }
    let lines: Vec<&str> = content.lines().collect();
    if let Some((p_idx, p_end, p_indent)) = find_mapping_block(&lines, 0, lines.len(), 0, "providers")
    {
        if has_inline_flow_value(lines[p_idx], "providers") {
            return false;
        }
        if let Some(child_indent) = detect_child_indent(&lines, p_idx + 1, p_end, p_indent) {
            if let Some((prov_idx, prov_end, _)) =
                find_mapping_block(&lines, p_idx + 1, p_end, child_indent, provider_id)
            {
                if has_inline_flow_value(lines[prov_idx], provider_id) {
                    return false;
                }
                if let Some(prop_indent) =
                    detect_child_indent(&lines, prov_idx + 1, prov_end, child_indent)
                {
                    if let Some((m_idx, _, _)) =
                        find_mapping_block(&lines, prov_idx + 1, prov_end, prop_indent, "models")
                    {
                        if has_inline_flow_value(lines[m_idx], "models") {
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

fn can_block_patch_config(content: &str) -> bool {
    if content.trim().is_empty() || is_flow_root(content) {
        return false;
    }
    let lines: Vec<&str> = content.lines().collect();
    if let Some((r_idx, _, _)) = find_mapping_block(&lines, 0, lines.len(), 0, "modelRoles") {
        if has_inline_flow_value(lines[r_idx], "modelRoles") {
            return false;
        }
    }
    true
}

fn has_inline_flow_value(line: &str, key: &str) -> bool {
    let (_, after) = match split_key_line(line) {
        Some((k, rest)) if k == key => (k, rest),
        _ => return false,
    };
    let val = strip_unquoted_comment(after).trim();
    !val.is_empty()
}

fn line_indent(line: &str) -> usize {
    line.bytes().take_while(|b| *b == b' ').count()
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn unquote_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn split_key_line(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("- ") || trimmed == "-"
    {
        return None;
    }
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (idx, ch) in trimmed.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_double && ch == '\\' {
            escaped = true;
            continue;
        }
        if !in_double && ch == '\'' {
            in_single = !in_single;
            continue;
        }
        if !in_single && ch == '"' {
            in_double = !in_double;
            continue;
        }
        if !in_single && !in_double && ch == ':' {
            let rest = &trimmed[idx + 1..];
            if rest.is_empty() || rest.starts_with([' ', '\t', '\r']) {
                return Some((unquote_key(&trimmed[..idx]), rest));
            }
        }
    }
    None
}

/// Splits `value_and_comment` into `(value_part, trailing_comment_with_leading_space)`.
fn split_trailing_comment(rest: &str) -> (&str, &str) {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut prev_whitespace = true;
    for (idx, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            prev_whitespace = false;
            continue;
        }
        if in_double && ch == '\\' {
            escaped = true;
            continue;
        }
        if !in_double && ch == '\'' {
            in_single = !in_single;
            prev_whitespace = false;
            continue;
        }
        if !in_single && ch == '"' {
            in_double = !in_double;
            prev_whitespace = false;
            continue;
        }
        if !in_single && !in_double && ch == '#' && prev_whitespace {
            let ws_start = rest[..idx]
                .rfind(|c: char| c != ' ' && c != '\t')
                .map_or(0, |p| p + rest[p..].chars().next().map_or(1, char::len_utf8));
            return (&rest[..ws_start], &rest[ws_start..]);
        }
        prev_whitespace = ch == ' ' || ch == '\t';
    }
    (rest, "")
}

fn strip_unquoted_comment(line: &str) -> &str {
    split_trailing_comment(line).0
}

fn format_yaml_scalar(value: &str) -> String {
    let needs_quotes = value.is_empty()
        || value.starts_with(['!', '&', '*', '{', '[', '|', '>', '%', '@', '`', ' ', '\t', '#', '-', '?', ':'])
        || value.ends_with([' ', '\t'])
        || value.contains(['#', ':', '"', '\'', '\n', '\r'])
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "true" | "false" | "null" | "~" | "yes" | "no" | "on" | "off"
        );
    if !needs_quotes {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_yaml_key(key: &str) -> String {
    if key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !key.is_empty()
    {
        key.to_string()
    } else {
        format_yaml_scalar(key)
    }
}

fn find_mapping_block(
    lines: &[&str],
    start: usize,
    end: usize,
    indent: usize,
    target_key: &str,
) -> Option<(usize, usize, usize)> {
    let mut key_idx = None;
    for idx in start..end {
        let line = lines[idx];
        if is_blank_or_comment(line) {
            continue;
        }
        let cur_indent = line_indent(line);
        if cur_indent < indent {
            break;
        }
        if cur_indent == indent {
            if let Some((k, _)) = split_key_line(line) {
                if key_idx.is_some() {
                    return Some((key_idx.unwrap(), idx, indent));
                }
                if k == target_key {
                    key_idx = Some(idx);
                }
            } else if key_idx.is_some() {
                return Some((key_idx.unwrap(), idx, indent));
            }
        }
    }
    key_idx.map(|k| (k, end, indent))
}

/// Trims trailing blank/comment lines that belong to the next sibling so insertions stay inside the block.
fn trim_block_end(lines: &[&str], block_start: usize, raw_end: usize) -> usize {
    let mut end = raw_end;
    while end > block_start + 1 && is_blank_or_comment(lines[end - 1]) {
        end -= 1;
    }
    end
}

fn detect_child_indent(lines: &[&str], start: usize, end: usize, parent_indent: usize) -> Option<usize> {
    for idx in start..end {
        let line = lines[idx];
        if is_blank_or_comment(line) {
            continue;
        }
        let ind = line_indent(line);
        if ind > parent_indent {
            return Some(ind);
        }
        break;
    }
    None
}

fn upsert_scalar_in_block(
    lines: &mut Vec<String>,
    block_start: usize,
    block_end: &mut usize,
    prop_indent: usize,
    key: &str,
    value: &str,
    insert_before_key: Option<&str>,
) {
    let formatted_val = format_yaml_scalar(value);
    for idx in (block_start + 1)..*block_end {
        if is_blank_or_comment(&lines[idx]) {
            continue;
        }
        if line_indent(&lines[idx]) == prop_indent {
            if let Some((k, rest)) = split_key_line(&lines[idx]) {
                if k == key {
                    let (_, comment) = split_trailing_comment(rest);
                    let raw_key_prefix = &lines[idx][..lines[idx].find(':').unwrap() + 1];
                    lines[idx] = format!("{raw_key_prefix} {formatted_val}{comment}");
                    return;
                }
            }
        }
    }

    let mut insert_at = trim_block_end(
        &lines.iter().map(String::as_str).collect::<Vec<_>>(),
        block_start,
        *block_end,
    );
    if let Some(before) = insert_before_key {
        for idx in (block_start + 1)..*block_end {
            if !is_blank_or_comment(&lines[idx]) && line_indent(&lines[idx]) == prop_indent {
                if let Some((k, _)) = split_key_line(&lines[idx]) {
                    if k == before {
                        insert_at = idx;
                        break;
                    }
                }
            }
        }
    }
    let pad = " ".repeat(prop_indent);
    lines.insert(insert_at, format!("{pad}{key}: {formatted_val}"));
    *block_end += 1;
}

fn render_provider_lines(indent: usize, patch: &OmpProviderPatch) -> Vec<String> {
    let p_pad = " ".repeat(indent);
    let f_pad = " ".repeat(indent + 2);
    let m_pad = " ".repeat(indent + 4);
    let mut out = Vec::new();
    out.push(format!("{p_pad}{}:", format_yaml_key(&patch.id)));
    out.push(format!("{f_pad}baseUrl: {}", format_yaml_scalar(&patch.base_url)));
    out.push(format!("{f_pad}api: {}", format_yaml_scalar(&patch.api)));
    if let Some(auth) = &patch.auth {
        out.push(format!("{f_pad}auth: {}", format_yaml_scalar(auth)));
    }
    if let Some(api_key) = &patch.api_key {
        out.push(format!("{f_pad}apiKey: {}", format_yaml_scalar(api_key)));
    }
    if !patch.models.is_empty() {
        out.push(format!("{f_pad}models:"));
        for m in &patch.models {
            out.push(format!("{m_pad}- id: {}", format_yaml_scalar(&m.id)));
            if let Some(name) = &m.name {
                out.push(format!("{m_pad}  name: {}", format_yaml_scalar(name)));
            }
        }
    }
    out
}

fn block_patch_provider(content: &str, patch: &OmpProviderPatch) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
    let refs = || lines.iter().map(String::as_str).collect::<Vec<_>>();

    let r = refs();
    let Some((p_idx, p_raw_end, _)) = find_mapping_block(&r, 0, r.len(), 0, "providers") else {
        if !lines.is_empty() && !lines.last().unwrap().is_empty() {
            // keep tight or separated cleanly
        }
        lines.push("providers:".to_string());
        lines.extend(render_provider_lines(2, patch));
        return Some(join_lines(&lines));
    };

    let child_indent = detect_child_indent(&r, p_idx + 1, p_raw_end, 0).unwrap_or(2);
    let Some((prov_idx, mut prov_end, _)) =
        find_mapping_block(&r, p_idx + 1, p_raw_end, child_indent, &patch.id)
    else {
        let insert_at = trim_block_end(&r, p_idx, p_raw_end);
        let rendered = render_provider_lines(child_indent, patch);
        lines.splice(insert_at..insert_at, rendered);
        return Some(join_lines(&lines));
    };

    let prop_indent = detect_child_indent(&r, prov_idx + 1, prov_end, child_indent)
        .unwrap_or(child_indent + 2);

    upsert_scalar_in_block(
        &mut lines,
        prov_idx,
        &mut prov_end,
        prop_indent,
        "baseUrl",
        &patch.base_url,
        Some("models"),
    );
    upsert_scalar_in_block(
        &mut lines,
        prov_idx,
        &mut prov_end,
        prop_indent,
        "api",
        &patch.api,
        Some("models"),
    );
    if let Some(auth) = &patch.auth {
        upsert_scalar_in_block(
            &mut lines,
            prov_idx,
            &mut prov_end,
            prop_indent,
            "auth",
            auth,
            Some("models"),
        );
    }
    if let Some(api_key) = &patch.api_key {
        upsert_scalar_in_block(
            &mut lines,
            prov_idx,
            &mut prov_end,
            prop_indent,
            "apiKey",
            api_key,
            Some("models"),
        );
    }

    if !patch.models.is_empty() {
        patch_models_in_block(&mut lines, prov_idx, &mut prov_end, prop_indent, &patch.models)?;
    }

    Some(join_lines(&lines))
}

fn patch_models_in_block(
    lines: &mut Vec<String>,
    prov_idx: usize,
    prov_end: &mut usize,
    prop_indent: usize,
    models: &[OmpModelInput],
) -> Option<()> {
    let r = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let Some((m_idx, m_raw_end, _)) =
        find_mapping_block(&r, prov_idx + 1, *prov_end, prop_indent, "models")
    else {
        let insert_at = trim_block_end(&r, prov_idx, *prov_end);
        let f_pad = " ".repeat(prop_indent);
        let m_pad = " ".repeat(prop_indent + 2);
        let mut added = vec![format!("{f_pad}models:")];
        for m in models {
            added.push(format!("{m_pad}- id: {}", format_yaml_scalar(&m.id)));
            if let Some(name) = &m.name {
                added.push(format!("{m_pad}  name: {}", format_yaml_scalar(name)));
            }
        }
        let count = added.len();
        lines.splice(insert_at..insert_at, added);
        *prov_end += count;
        return Some(());
    };

    let mut m_end = m_raw_end;
    let seq_indent = find_seq_dash_indent(lines, m_idx + 1, m_end, prop_indent).unwrap_or(prop_indent + 2);

    for model in models {
        if let Some((item_start, item_end, item_prop_indent)) =
            find_model_seq_item(lines, m_idx + 1, m_end, seq_indent, &model.id)
        {
            if let Some(name) = &model.name {
                let formatted_name = format_yaml_scalar(name);
                let mut updated = false;
                for idx in item_start..item_end {
                    let check_line = if idx == item_start {
                        let trimmed = lines[idx].trim_start();
                        trimmed.strip_prefix("- ").unwrap_or("")
                    } else {
                        lines[idx].trim_start()
                    };
                    if idx > item_start && line_indent(&lines[idx]) != item_prop_indent {
                        continue;
                    }
                    if let Some((k, rest)) = split_key_line(check_line) {
                        if k == "name" {
                            let (_, comment) = split_trailing_comment(rest);
                            if idx == item_start {
                                let dash_pad = " ".repeat(seq_indent);
                                lines[idx] = format!("{dash_pad}- name: {formatted_name}{comment}");
                            } else {
                                let pad = " ".repeat(item_prop_indent);
                                lines[idx] = format!("{pad}name: {formatted_name}{comment}");
                            }
                            updated = true;
                            break;
                        }
                    }
                }
                if !updated {
                    let pad = " ".repeat(item_prop_indent);
                    lines.insert(item_start + 1, format!("{pad}name: {formatted_name}"));
                    m_end += 1;
                    *prov_end += 1;
                }
            }
        } else {
            let r_now = lines.iter().map(String::as_str).collect::<Vec<_>>();
            let insert_at = trim_block_end(&r_now, m_idx, m_end);
            let dash_pad = " ".repeat(seq_indent);
            let prop_pad = " ".repeat(seq_indent + 2);
            let mut added = vec![format!("{dash_pad}- id: {}", format_yaml_scalar(&model.id))];
            if let Some(name) = &model.name {
                added.push(format!("{prop_pad}name: {}", format_yaml_scalar(name)));
            }
            let count = added.len();
            lines.splice(insert_at..insert_at, added);
            m_end += count;
            *prov_end += count;
        }
    }
    Some(())
}

fn find_seq_dash_indent(
    lines: &[String],
    start: usize,
    end: usize,
    parent_indent: usize,
) -> Option<usize> {
    for idx in start..end {
        if is_blank_or_comment(&lines[idx]) {
            continue;
        }
        let ind = line_indent(&lines[idx]);
        if ind >= parent_indent && lines[idx].trim_start().starts_with('-') {
            return Some(ind);
        }
    }
    None
}

fn find_model_seq_item(
    lines: &[String],
    start: usize,
    end: usize,
    seq_indent: usize,
    target_id: &str,
) -> Option<(usize, usize, usize)> {
    let mut items: Vec<(usize, usize)> = Vec::new();
    let mut current_start: Option<usize> = None;

    for idx in start..end {
        if is_blank_or_comment(&lines[idx]) {
            continue;
        }
        let ind = line_indent(&lines[idx]);
        let trimmed = lines[idx].trim_start();
        if ind == seq_indent && (trimmed == "-" || trimmed.starts_with("- ")) {
            if let Some(s) = current_start {
                items.push((s, idx));
            }
            current_start = Some(idx);
        } else if ind < seq_indent {
            break;
        }
    }
    if let Some(s) = current_start {
        items.push((s, end));
    }

    for (i_start, i_end) in items {
        let item_prop_indent = seq_indent + 2;
        for idx in i_start..i_end {
            let candidate = if idx == i_start {
                let trimmed = lines[idx].trim_start();
                trimmed.strip_prefix("- ").unwrap_or("")
            } else {
                if is_blank_or_comment(&lines[idx]) || line_indent(&lines[idx]) != item_prop_indent
                {
                    continue;
                }
                lines[idx].trim_start()
            };
            if let Some((k, rest)) = split_key_line(candidate) {
                if k == "id" {
                    let (val_part, _) = split_trailing_comment(rest);
                    if unquote_key(val_part) == target_id {
                        return Some((i_start, i_end, item_prop_indent));
                    }
                }
            }
        }
    }
    None
}

fn block_patch_default_role(content: &str, default_model: &str) -> Option<String> {
    let mut lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
    let r = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let formatted = format_yaml_scalar(default_model);

    let Some((r_idx, mut r_end, _)) = find_mapping_block(&r, 0, r.len(), 0, "modelRoles") else {
        lines.push("modelRoles:".to_string());
        lines.push(format!("  default: {formatted}"));
        return Some(join_lines(&lines));
    };

    let prop_indent = detect_child_indent(&r, r_idx + 1, r_end, 0).unwrap_or(2);
    upsert_scalar_in_block(
        &mut lines,
        r_idx,
        &mut r_end,
        prop_indent,
        "default",
        default_model,
        None,
    );
    Some(join_lines(&lines))
}

fn join_lines(lines: &[String]) -> String {
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}
