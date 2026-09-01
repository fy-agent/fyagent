use super::*;
use once_cell::sync::Lazy;
use regex::Regex;

/// 获取单个工具的版本信息（内部实现）
pub(super) async fn get_single_tool_version_impl(tool: &str) -> ToolVersion {
    debug_assert!(
        VALID_TOOLS.contains(&tool),
        "unexpected tool name in get_single_tool_version_impl: {tool}"
    );

    let client = crate::proxy::http_client::get();

    #[cfg(target_os = "windows")]
    let probe = scan_cli_version(tool);

    #[cfg(target_os = "macos")]
    let probe = match try_get_version(tool) {
        ShellProbe::NotFound(_) => scan_cli_version(tool),
        found => found,
    };

    let (local_version, local_error, installed_but_broken) = match probe {
        ShellProbe::Found(v) => (Some(v), None, false),
        ShellProbe::FoundButFailed(e) => (None, Some(e), true),
        ShellProbe::NotFound(e) => (None, Some(e), false),
    };

    let local = local_version.as_deref();
    let mut distribution_owner = None;
    let mut latest_source = None;
    let latest_version = match tool {
        "claude" => {
            fetch_npm_latest_for_tool(&client, "@anthropic-ai/claude-code", tool, local).await
        }
        "codex" => fetch_npm_latest_for_tool(&client, "@openai/codex", tool, local).await,
        "gemini" => fetch_npm_latest_for_tool(&client, "@google/gemini-cli", tool, local).await,
        "grok" => {
            let (latest, owner) = fetch_grok_latest_with_owner(&client, local).await;
            distribution_owner = owner.clone();
            latest_source = owner;
            latest
        }
        "opencode" => {
            if let Some(version) =
                fetch_npm_latest_for_tool(&client, "opencode-ai", tool, local).await
            {
                Some(version)
            } else {
                fetch_github_latest_version(&client, "anomalyco/opencode").await
            }
        }
        "openclaw" => fetch_npm_latest_for_tool(&client, "openclaw", tool, local).await,
        "hermes" => fetch_pypi_latest_version(&client, "hermes-agent").await,
        _ => None,
    };

    ToolVersion {
        name: tool.to_string(),
        version: local_version,
        latest_version,
        error: if tool == "grok" {
            super::grok::last_grok_lifecycle_error().or(local_error)
        } else {
            local_error
        },
        installed_but_broken,
        distribution_owner,
        latest_source,
    }
}

pub(super) async fn fetch_grok_latest_with_owner(
    client: &reqwest::Client,
    local: Option<&str>,
) -> (Option<String>, Option<String>) {
    #[cfg(target_os = "macos")]
    {
        let observation = super::grok::observe_installed_grok_owner();
        let owner = super::grok::owner_observation_wire(observation).map(str::to_string);
        let latest = match observation {
            super::grok::GrokOwnerObservation::NativeInternal => {
                super::grok::native_latest_from_update_check(local)
            }
            super::grok::GrokOwnerObservation::OfficialNpm => {
                fetch_npm_latest_for_tool(client, "@xai-official/grok", "grok", local).await
            }
            super::grok::GrokOwnerObservation::Ambiguous
            | super::grok::GrokOwnerObservation::Absent => None,
        };
        (latest, owner)
    }
    #[cfg(not(target_os = "macos"))]
    {
        (
            fetch_npm_latest_for_tool(client, "@xai-official/grok", "grok", local).await,
            None,
        )
    }
}

pub(super) fn elevated_windows_tool_version_unavailable(tool: &str) -> ToolVersion {
    ToolVersion {
        name: tool.to_string(),
        version: None,
        latest_version: None,
        error: Some(ELEVATED_WINDOWS_CLI_BOUNDARY_MESSAGE.to_string()),
        installed_but_broken: false,
        distribution_owner: None,
        latest_source: None,
    }
}

fn npm_prerelease_tags(tool: &str) -> &'static [&'static str] {
    match tool {
        "claude" => &["next"],
        _ => &[],
    }
}

fn parse_semver(v: &str) -> Option<([u64; 3], Vec<String>)> {
    let core_and_pre = v.trim().split('+').next().unwrap_or("");
    let (core, pre) = match core_and_pre.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (core_and_pre, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let pre_segments = pre
        .map(|p| p.split('.').map(|s| s.to_string()).collect())
        .unwrap_or_default();
    Some(([major, minor, patch], pre_segments))
}

pub(super) fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (ac, ap) = parse_semver(a)?;
    let (bc, bp) = parse_semver(b)?;
    for i in 0..3 {
        match ac[i].cmp(&bc[i]) {
            Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    match (ap.is_empty(), bp.is_empty()) {
        (true, true) => return Some(Ordering::Equal),
        (true, false) => return Some(Ordering::Greater),
        (false, true) => return Some(Ordering::Less),
        (false, false) => {}
    }
    for (x, y) in ap.iter().zip(bp.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(xv), Ok(yv)) => xv.cmp(&yv),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.as_str().cmp(y.as_str()),
        };
        if ord != Ordering::Equal {
            return Some(ord);
        }
    }
    Some(ap.len().cmp(&bp.len()))
}

pub(super) fn pick_latest_version(
    dist_tags: &serde_json::Map<String, serde_json::Value>,
    prerelease_tags: &[&str],
    local_version: Option<&str>,
) -> Option<String> {
    use std::cmp::Ordering;
    let latest = dist_tags.get("latest").and_then(|v| v.as_str())?;
    let local_ahead = local_version
        .and_then(|local| compare_semver(local, latest))
        .map(|ord| ord == Ordering::Greater)
        .unwrap_or(false);
    if prerelease_tags.is_empty() || !local_ahead {
        return Some(latest.to_string());
    }

    let mut best = latest.to_string();
    for tag in prerelease_tags {
        if let Some(candidate) = dist_tags.get(*tag).and_then(|v| v.as_str()) {
            if compare_semver(candidate, &best) == Some(Ordering::Greater) {
                best = candidate.to_string();
            }
        }
    }
    Some(best)
}

async fn fetch_npm_dist_tags(
    client: &reqwest::Client,
    package: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let url = format!("https://registry.npmjs.org/{package}");
    let resp = client.get(&url).send().await.ok()?;
    let json = resp.json::<serde_json::Value>().await.ok()?;
    json.get("dist-tags")?.as_object().cloned()
}

#[allow(dead_code)]
pub(super) async fn fetch_npm_latest_for_package(
    client: &reqwest::Client,
    package: &str,
) -> Option<String> {
    fetch_npm_latest_for_tool(client, package, "", None).await
}

async fn fetch_npm_latest_for_tool(
    client: &reqwest::Client,
    package: &str,
    tool: &str,
    local_version: Option<&str>,
) -> Option<String> {
    let dist_tags = fetch_npm_dist_tags(client, package).await?;
    pick_latest_version(&dist_tags, npm_prerelease_tags(tool), local_version)
}

pub(crate) const FIXED_GITHUB_OPENCODE_REPO: &str = "anomalyco/opencode";
const MAX_GITHUB_LATEST_BYTES: usize = 1024 * 1024;

pub(crate) fn github_latest_release_url(repo: &str) -> Option<String> {
    if repo != FIXED_GITHUB_OPENCODE_REPO {
        return None;
    }
    Some(format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ))
}

pub(crate) fn parse_github_latest_release_tag(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > MAX_GITHUB_LATEST_BYTES {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(body).ok()?;
    if json.get("draft").and_then(|value| value.as_bool()) == Some(true)
        || json.get("prerelease").and_then(|value| value.as_bool()) == Some(true)
    {
        return None;
    }
    let tag = json.get("tag_name")?.as_str()?;
    if tag.is_empty() || tag.len() > 64 {
        return None;
    }
    Some(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

pub(crate) async fn fetch_github_latest_version(
    client: &reqwest::Client,
    repo: &str,
) -> Option<String> {
    let url = github_latest_release_url(repo)?;
    let resp = client
        .get(&url)
        .header("User-Agent", "fyagent")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    parse_github_latest_release_tag(&bytes)
}

async fn fetch_pypi_latest_version(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{package}/json");
    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json.get("info")
                    .and_then(|info| info.get("version"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

static VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+\.\d+\.\d+(-[\w.]+)?").expect("Invalid version regex"));

pub(super) fn extract_version(raw: &str) -> String {
    VERSION_RE
        .find(raw)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_latest_tag_parser_is_fixed_repo_and_rejects_drafts() {
        assert_eq!(
            parse_github_latest_release_tag(br#"{"tag_name":"v1.2.3"}"#).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            parse_github_latest_release_tag(br#"{"tag_name":"1.2.3"}"#).as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            parse_github_latest_release_tag(br#"{"tag_name":"v1.2.3","draft":true}"#),
            None
        );
        assert_eq!(
            parse_github_latest_release_tag(br#"{"tag_name":"v1.2.3","prerelease":true}"#),
            None
        );
        assert_eq!(parse_github_latest_release_tag(&[]), None);
        assert_eq!(
            parse_github_latest_release_tag(&vec![b'{'; MAX_GITHUB_LATEST_BYTES + 1]),
            None
        );
        assert_eq!(
            github_latest_release_url(FIXED_GITHUB_OPENCODE_REPO).as_deref(),
            Some("https://api.github.com/repos/anomalyco/opencode/releases/latest")
        );
        assert_eq!(github_latest_release_url("some/other"), None);
    }
}
