pub(super) fn is_local_proxy_url(url: &str) -> bool {
    let url = url.trim();
    if !url.starts_with("http://") {
        return false;
    }
    let rest = &url["http://".len()..];
    rest.starts_with("127.0.0.1")
        || rest.starts_with("localhost")
        || rest.starts_with("0.0.0.0")
        || rest.starts_with("[::1]")
        || rest.starts_with("[::]")
        || rest.starts_with("::1")
        || rest.starts_with("::")
}

pub(super) fn proxy_urls_match(actual: &str, expected: &str) -> bool {
    actual.trim().trim_end_matches('/') == expected.trim().trim_end_matches('/')
}

pub(super) fn codex_config_has_base_url_matching(
    config_text: &str,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    let Ok(doc) = toml::from_str::<toml::Value>(config_text) else {
        return false;
    };

    let active_provider = doc
        .get("model_provider")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty());

    if let Some(provider_id) = active_provider {
        if doc
            .get("model_providers")
            .and_then(|value| value.get(provider_id))
            .and_then(|value| value.get("base_url"))
            .and_then(|value| value.as_str())
            .is_some_and(&predicate)
        {
            return true;
        }
    }

    doc.get("base_url")
        .and_then(|value| value.as_str())
        .is_some_and(predicate)
}

#[cfg(test)]
mod tests {
    use super::{codex_config_has_base_url_matching, is_local_proxy_url, proxy_urls_match};

    #[test]
    fn local_proxy_url_recognizes_loopback_only() {
        for url in [
            "http://127.0.0.1:1234",
            "http://localhost:1234/",
            "http://[::1]:1234",
        ] {
            assert!(is_local_proxy_url(url), "expected local proxy URL: {url}");
        }
        assert!(!is_local_proxy_url("https://127.0.0.1:1234"));
        assert!(!is_local_proxy_url("http://example.com"));
    }

    #[test]
    fn proxy_url_match_ignores_surrounding_space_and_trailing_slash() {
        assert!(proxy_urls_match(
            " http://127.0.0.1:1234/ ",
            "http://127.0.0.1:1234"
        ));
    }

    #[test]
    fn codex_base_url_match_checks_active_provider_then_top_level() {
        let active = r#"
model_provider = "custom"
[model_providers.custom]
base_url = "http://127.0.0.1:1234/v1"
"#;
        assert!(codex_config_has_base_url_matching(active, |url| {
            url == "http://127.0.0.1:1234/v1"
        }));

        let top_level = r#"base_url = "http://localhost:4321/v1""#;
        assert!(codex_config_has_base_url_matching(top_level, |url| {
            url == "http://localhost:4321/v1"
        }));
        assert!(!codex_config_has_base_url_matching("not = [toml", |_| true));
    }
}
