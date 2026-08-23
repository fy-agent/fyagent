const DISPLAY_NAME_LIMIT: usize = 80;

/// Project a provider-supplied display name without consulting the host
/// filesystem. Every platform therefore produces the same safe summary.
pub(crate) fn sanitize_display_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().any(char::is_control)
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(['/', '\\'])
        || has_drive_prefix(trimmed)
        || trimmed
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
    {
        return "Provider".to_string();
    }

    trimmed.chars().take(DISPLAY_NAME_LIMIT).collect()
}

fn has_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

pub(crate) fn is_safe_opaque_id(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 || !value.is_ascii() {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphanumeric() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::{is_safe_opaque_id, sanitize_display_name};

    #[test]
    fn display_name_sanitization_is_pure_and_platform_neutral() {
        for rejected in [
            "",
            "   ",
            "/Users/alice/key",
            "\\\\server\\share",
            "C:\\secret",
            "z:relative",
            "folder/name",
            "folder\\name",
            "file:///tmp/secret",
            "FiLe:C:\\secret",
            "line\nbreak",
            "nul\0byte",
        ] {
            assert_eq!(sanitize_display_name(rejected), "Provider", "{rejected:?}");
        }
        assert_eq!(sanitize_display_name("  安全供应商 🦀  "), "安全供应商 🦀");
        assert_eq!(sanitize_display_name(&"界".repeat(81)).chars().count(), 80);
    }

    #[test]
    fn opaque_ids_exclude_paths_controls_and_unbounded_values() {
        for accepted in ["codex-official", "provider_1", "p.1", "_internal"] {
            assert!(is_safe_opaque_id(accepted), "{accepted}");
        }
        for rejected in [
            "",
            "-leading",
            "C:\\secret",
            "/tmp/provider",
            "provider/child",
            "provider secret",
            "供应商",
        ] {
            assert!(!is_safe_opaque_id(rejected), "{rejected:?}");
        }
    }
}
