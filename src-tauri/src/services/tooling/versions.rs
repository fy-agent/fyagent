use super::grok_npm::NpmVersionAuthority;
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
    let probe = match probe_path_default_version(tool) {
        ShellProbe::NotFound(_) => scan_cli_version(tool),
        found => found,
    };

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
    let (latest_version, latest_authority) = match tool {
        "claude" => {
            fetch_npm_latest_with_authority(&client, "@anthropic-ai/claude-code", tool, local).await
        }
        "codex" => fetch_npm_latest_with_authority(&client, "@openai/codex", tool, local).await,
        "gemini" => {
            fetch_npm_latest_with_authority(&client, "@google/gemini-cli", tool, local).await
        }
        "grok" => {
            let (latest, owner, authority) = fetch_grok_latest_with_owner(&client, local).await;
            distribution_owner = owner.clone();
            latest_source = owner;
            (latest, authority)
        }
        "opencode" => fetch_opencode_cli_latest(&client, local).await,
        "openclaw" => fetch_npm_latest_with_authority(&client, "openclaw", tool, local).await,
        "hermes" => {
            let latest = drop_latest_behind_local(fetch_hermes_latest_version().await, local);
            if latest.is_some() {
                latest_source = Some(HERMES_LATEST_SOURCE.to_string());
            }
            (latest, None)
        }
        _ => (None, None),
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
        latest_authority: latest_authority.map(|authority| authority.wire().to_string()),
    }
}

/// Grok latest version, observed owner and npm version authority. A native
/// updater result has no npm authority.
pub(super) async fn fetch_grok_latest_with_owner(
    client: &reqwest::Client,
    local: Option<&str>,
) -> (Option<String>, Option<String>, Option<NpmVersionAuthority>) {
    #[cfg(target_os = "macos")]
    {
        let observation = super::grok::observe_installed_grok_owner();
        let owner = super::grok::owner_observation_wire(observation).map(str::to_string);
        let (latest, authority) = match observation {
            super::grok::GrokOwnerObservation::NativeInternal => {
                (super::grok::native_latest_from_update_check(local), None)
            }
            super::grok::GrokOwnerObservation::OfficialNpm
            | super::grok::GrokOwnerObservation::Absent => {
                fetch_npm_latest_with_authority(client, "@xai-official/grok", "grok", local).await
            }
            super::grok::GrokOwnerObservation::Ambiguous => (None, None),
        };
        (latest, owner, authority)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let (latest, authority) =
            fetch_npm_latest_with_authority(client, "@xai-official/grok", "grok", local).await;
        (latest, None, authority)
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
        latest_authority: None,
    }
}

fn npm_prerelease_tags(tool: &str) -> &'static [&'static str] {
    match tool {
        "claude" => &["next"],
        _ => &[],
    }
}

pub(super) fn compare_semver(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let a = semver::Version::parse(a.trim()).ok()?;
    let b = semver::Version::parse(b.trim()).ok()?;
    // Version's total ordering includes build metadata; upgrade precedence must not.
    Some(a.cmp_precedence(&b))
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

pub(super) fn npm_dist_tags_url(package: &str) -> String {
    format!(
        "https://registry.npmjs.org/-/package/{}/dist-tags",
        package.replace('/', "%2f")
    )
}

pub(super) fn github_release_version_from_json(json: &serde_json::Value) -> Option<String> {
    let from_name = json
        .get("name")
        .and_then(|v| v.as_str())
        .and_then(|name| release_display_version(&extract_version(name)));
    from_name.or_else(|| {
        json.get("tag_name")
            .and_then(|v| v.as_str())
            .and_then(|tag| release_display_version(tag.strip_prefix('v').unwrap_or(tag)))
    })
}

pub(super) fn drop_latest_behind_local(
    latest: Option<String>,
    local_version: Option<&str>,
) -> Option<String> {
    let latest = latest?;
    let local_leads = local_version
        .and_then(|local| compare_semver(local, &latest))
        .is_some_and(|ord| ord == std::cmp::Ordering::Greater);
    (!local_leads).then_some(latest)
}

fn release_display_version(candidate: &str) -> Option<String> {
    semver::Version::parse(candidate)
        .ok()
        .filter(|version| version.major < 1000)
        .map(|_| candidate.to_string())
}

const LATEST_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

async fn fetch_npm_dist_tags(
    client: &reqwest::Client,
    package: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let resp = client
        .get(npm_dist_tags_url(package))
        .timeout(LATEST_PROBE_TIMEOUT)
        .send()
        .await
        .ok()?;
    resp.json::<serde_json::Map<String, serde_json::Value>>()
        .await
        .ok()
}

#[allow(dead_code)]
pub(super) async fn fetch_npm_latest_for_package(
    client: &reqwest::Client,
    package: &str,
) -> Option<String> {
    fetch_npm_latest_for_tool(client, package, "", None).await
}

#[allow(dead_code)]
pub(super) async fn fetch_npm_latest_for_tool(
    client: &reqwest::Client,
    package: &str,
    tool: &str,
    local_version: Option<&str>,
) -> Option<String> {
    fetch_npm_latest_with_authority(client, package, tool, local_version)
        .await
        .0
}

/// npmjs dist-tags are the official authority. When they are unavailable the
/// reviewed registry chain may still answer, but that answer is labelled
/// `MirrorFallback`: a mirror can lag or carry a stale `latest` tag, so its
/// value is never presented as the confirmed official latest.
pub(super) async fn fetch_npm_latest_with_authority(
    client: &reqwest::Client,
    package: &str,
    tool: &str,
    local_version: Option<&str>,
) -> (Option<String>, Option<NpmVersionAuthority>) {
    let from_dist_tags = fetch_npm_dist_tags(client, package)
        .await
        .and_then(|dist_tags| {
            pick_latest_version(&dist_tags, npm_prerelease_tags(tool), local_version)
        });
    match from_dist_tags {
        Some(version) => (Some(version), Some(NpmVersionAuthority::Official)),
        None => match super::grok_npm::fetch_published_version(package).await {
            Some((version, authority)) => (Some(version), Some(authority)),
            None => (None, None),
        },
    }
}

/// OpenCode publishes two official CLI lines from `anomalyco/opencode`:
/// the legacy `opencode-ai` package (1.x, also the GitHub Releases line) and
/// the v2 `@opencode/cli` package. Neither package publishes the other's
/// major line, so the observed local major selects the matching channel for
/// display only. No installation is migrated; an unknown or absent local CLI
/// keeps the legacy lookup.
const OPENCODE_LEGACY_NPM_PACKAGE: &str = "opencode-ai";
const OPENCODE_V2_NPM_PACKAGE: &str = "@opencode/cli";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenCodeCliLine {
    Legacy,
    V2,
}

fn opencode_cli_line(local: Option<&str>) -> OpenCodeCliLine {
    match local.and_then(|value| semver::Version::parse(value.trim()).ok()) {
        Some(version) if version.major >= 2 => OpenCodeCliLine::V2,
        _ => OpenCodeCliLine::Legacy,
    }
}

async fn fetch_opencode_cli_latest(
    client: &reqwest::Client,
    local: Option<&str>,
) -> (Option<String>, Option<NpmVersionAuthority>) {
    match opencode_cli_line(local) {
        OpenCodeCliLine::V2 => {
            // GitHub Releases carry the legacy line only; never use them as a
            // v2 fallback.
            fetch_npm_latest_with_authority(client, OPENCODE_V2_NPM_PACKAGE, "opencode", local)
                .await
        }
        OpenCodeCliLine::Legacy => {
            match fetch_npm_latest_with_authority(
                client,
                OPENCODE_LEGACY_NPM_PACKAGE,
                "opencode",
                local,
            )
            .await
            {
                (Some(version), authority) => (Some(version), authority),
                (None, _) => (
                    fetch_github_latest_version(client, FIXED_GITHUB_OPENCODE_REPO).await,
                    None,
                ),
            }
        }
    }
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
    github_release_version_from_json(&json)
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

// Hermes official releases use calendar tags (for example `v2026.9.24`) while
// `hermes --version` reports the product version from that tag's
// `pyproject.toml` (for example `0.21.5`). The official installers track the
// source release, and PyPI is not an official install channel and can lag, so
// the comparable latest is resolved as: fixed official repository latest
// stable release -> exact tag -> that tag's `[project].version`. The tag,
// release title, mutable `main` and PyPI are never substituted; any failure is
// an unknown latest.
const HERMES_LATEST_SOURCE: &str = "official_release";
const HERMES_RELEASE_API: &str =
    "https://api.github.com/repos/NousResearch/hermes-agent/releases/latest";
const HERMES_RELEASE_PAGE_PREFIX: &str =
    "https://github.com/NousResearch/hermes-agent/releases/tag/";
const HERMES_PROJECT_RAW_PREFIX: &str =
    "https://raw.githubusercontent.com/NousResearch/hermes-agent/";
const HERMES_MAX_TAG_BYTES: usize = 64;

/// Stable Hermes release tags are dotted numeric (calendar) versions such as
/// `v2026.9.24`. Zero-padded components (`v2026.10.01`) are accepted; any
/// other character, including prerelease/build suffixes and path syntax, is
/// rejected before the tag is placed into the fixed raw URL.
fn hermes_stable_tag(tag: &str) -> Option<&str> {
    if tag.is_empty() || tag.len() > HERMES_MAX_TAG_BYTES {
        return None;
    }
    let components: Vec<&str> = tag.strip_prefix('v').unwrap_or(tag).split('.').collect();
    let well_formed = components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && component.len() <= 8
                && component.bytes().all(|byte| byte.is_ascii_digit())
        });
    well_formed.then_some(tag)
}

fn parse_hermes_release_tag(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > MAX_GITHUB_LATEST_BYTES {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(body).ok()?;
    // Missing status fields are not evidence of a stable release.
    if json.get("draft")?.as_bool()? || json.get("prerelease")?.as_bool()? {
        return None;
    }
    let tag = hermes_stable_tag(json.get("tag_name")?.as_str()?)?;
    // The response may not turn a different repository into Hermes evidence.
    let expected_page = format!("{HERMES_RELEASE_PAGE_PREFIX}{tag}");
    if json.get("html_url")?.as_str()? != expected_page {
        return None;
    }
    Some(tag.to_string())
}

fn hermes_project_url(tag: &str) -> Option<String> {
    let tag = hermes_stable_tag(tag)?;
    // No response-provided URL, title, archive or command is followed.
    Some(format!("{HERMES_PROJECT_RAW_PREFIX}{tag}/pyproject.toml"))
}

fn parse_hermes_project_version(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > MAX_GITHUB_LATEST_BYTES {
        return None;
    }
    let document: toml::Value = toml::from_str(std::str::from_utf8(body).ok()?).ok()?;
    let project = document.get("project")?.as_table()?;
    if project.get("name")?.as_str()? != "hermes-agent" {
        return None;
    }
    let raw_version = project.get("version")?.as_str()?;
    if raw_version.len() > HERMES_MAX_TAG_BYTES {
        return None;
    }
    let version = semver::Version::parse(raw_version).ok()?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Bounded metadata GET. Non-success status (including an unfollowed
/// redirect), an oversized declared length, a streamed body over the limit,
/// or a transport/timeout error all yield `None`.
async fn fetch_bounded_metadata(
    client: &reqwest::Client,
    url: &str,
    accept: &str,
    max_bytes: usize,
) -> Option<Vec<u8>> {
    let mut response = client
        .get(url)
        .header("User-Agent", "fyagent")
        .header("Accept", accept)
        .send()
        .await
        .ok()?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > max_bytes as u64)
    {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len().checked_add(chunk.len())? > max_bytes {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    Some(body)
}

async fn fetch_hermes_latest_with_client(client: &reqwest::Client) -> Option<String> {
    let release = fetch_bounded_metadata(
        client,
        HERMES_RELEASE_API,
        "application/vnd.github+json",
        MAX_GITHUB_LATEST_BYTES,
    )
    .await?;
    let tag = parse_hermes_release_tag(&release)?;
    let project = fetch_bounded_metadata(
        client,
        &hermes_project_url(&tag)?,
        "text/plain",
        MAX_GITHUB_LATEST_BYTES,
    )
    .await?;
    parse_hermes_project_version(&project)
}

async fn fetch_hermes_latest_version() -> Option<String> {
    // Same installer-proxy, HTTPS-only, no-redirect, 20 s metadata client as
    // the npm owner; no general-purpose client headers are inherited.
    let client = super::grok_npm::metadata_client()?;
    fetch_hermes_latest_with_client(&client).await
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
    fn semver_precedence_matches_the_standard_and_ignores_build_metadata() {
        use std::cmp::Ordering;
        let versions = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in versions.windows(2) {
            assert_eq!(compare_semver(pair[0], pair[1]), Some(Ordering::Less));
            assert_eq!(compare_semver(pair[1], pair[0]), Some(Ordering::Greater));
        }
        assert_eq!(
            compare_semver(" 1.2.3+build.9 ", "1.2.3+build.1"),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_semver("1.0.0-99999999999999999999", "1.0.0-100000000000000000000"),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn semver_rejects_invalid_versions_in_either_operand() {
        for invalid in [
            "",
            "1.0",
            "1.2.3.4",
            "01.2.3",
            "1.2.3-",
            "1.2.3+",
            "1.2.3-01",
            "1.2.3-a..b",
            "1.2.3-a_b",
            "1.2.3+bad+metadata",
            "v1.2.3",
        ] {
            assert_eq!(compare_semver(invalid, "1.2.3"), None, "{invalid}");
            assert_eq!(compare_semver("1.2.3", invalid), None, "{invalid}");
        }
    }

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

    const HERMES_STABLE_RELEASE: &[u8] = br#"{"tag_name":"v2026.9.24","name":"Hermes Agent v0.21.5","draft":false,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.24"}"#;

    #[test]
    fn hermes_release_parser_accepts_only_fixed_repo_stable_tags() {
        assert_eq!(
            parse_hermes_release_tag(HERMES_STABLE_RELEASE).as_deref(),
            Some("v2026.9.24")
        );
        for body in [
            // draft / prerelease / missing status fields
            br#"{"tag_name":"v2026.9.24","draft":true,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.24"}"#.as_slice(),
            br#"{"tag_name":"v2026.9.24","draft":false,"prerelease":true,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.24"}"#.as_slice(),
            br#"{"tag_name":"v2026.9.24","html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.24"}"#.as_slice(),
            // foreign repository / mismatched page
            br#"{"tag_name":"v2026.9.24","draft":false,"prerelease":false,"html_url":"https://github.com/evil/hermes-agent/releases/tag/v2026.9.24"}"#.as_slice(),
            br#"{"tag_name":"v2026.9.24","draft":false,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.23"}"#.as_slice(),
            // prerelease / path-like / non-semver tags
            br#"{"tag_name":"v2026.9.24-rc.1","draft":false,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/v2026.9.24-rc.1"}"#.as_slice(),
            br#"{"tag_name":"../main","draft":false,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/../main"}"#.as_slice(),
            br#"{"tag_name":"main","draft":false,"prerelease":false,"html_url":"https://github.com/NousResearch/hermes-agent/releases/tag/main"}"#.as_slice(),
            b"".as_slice(),
            b"not json".as_slice(),
        ] {
            assert_eq!(parse_hermes_release_tag(body), None, "{body:?}");
        }
        assert_eq!(
            parse_hermes_release_tag(&vec![b' '; MAX_GITHUB_LATEST_BYTES + 1]),
            None
        );
    }

    #[test]
    fn hermes_project_url_is_fixed_and_tag_scoped() {
        assert_eq!(
            hermes_project_url("v2026.9.24").as_deref(),
            Some("https://raw.githubusercontent.com/NousResearch/hermes-agent/v2026.9.24/pyproject.toml")
        );
        assert_eq!(hermes_project_url("main"), None);
        assert_eq!(hermes_project_url("v1.2.3/../../x"), None);
        assert_eq!(hermes_project_url("v1.2.3-beta"), None);
        assert_eq!(
            hermes_project_url("v2026.10.01").as_deref(),
            Some("https://raw.githubusercontent.com/NousResearch/hermes-agent/v2026.10.01/pyproject.toml")
        );
        assert_eq!(hermes_project_url("v2026.9"), None);
        assert_eq!(hermes_project_url("v2026.9.24.1"), None);
        assert_eq!(hermes_project_url("v2026..24"), None);
        assert_eq!(hermes_project_url("v2026.9.24+build"), None);
    }

    #[test]
    fn hermes_project_version_comes_from_project_table_not_tag_or_title() {
        let project = br#"
[build-system]
requires = ["setuptools"]

[project]
name = "hermes-agent"
version = "0.21.5"
"#;
        assert_eq!(
            parse_hermes_project_version(project).as_deref(),
            Some("0.21.5")
        );
        for body in [
            br#"[project]
name = "other"
version = "0.21.5"
"#
            .as_slice(),
            br#"[project]
name = "hermes-agent"
version = "0.22.0rc1"
"#
            .as_slice(),
            br#"[project]
name = "hermes-agent"
version = "0.22.0-beta.1"
"#
            .as_slice(),
            br#"[project]
name = "hermes-agent"
dynamic = ["version"]
"#
            .as_slice(),
            br#"[tool.poetry]
name = "hermes-agent"
version = "0.21.5"
"#
            .as_slice(),
            b"not = [toml".as_slice(),
            b"".as_slice(),
        ] {
            assert_eq!(parse_hermes_project_version(body), None, "{body:?}");
        }
        assert_eq!(HERMES_LATEST_SOURCE, "official_release");
    }

    #[test]
    fn opencode_cli_line_follows_the_observed_local_major() {
        assert_eq!(opencode_cli_line(None), OpenCodeCliLine::Legacy);
        assert_eq!(opencode_cli_line(Some("")), OpenCodeCliLine::Legacy);
        assert_eq!(opencode_cli_line(Some("garbage")), OpenCodeCliLine::Legacy);
        assert_eq!(opencode_cli_line(Some("1.18.33")), OpenCodeCliLine::Legacy);
        assert_eq!(opencode_cli_line(Some("0.9.0")), OpenCodeCliLine::Legacy);
        assert_eq!(opencode_cli_line(Some("2.0.19")), OpenCodeCliLine::V2);
        assert_eq!(opencode_cli_line(Some(" 2.1.0 ")), OpenCodeCliLine::V2);
        assert_eq!(opencode_cli_line(Some("3.0.0-beta.1")), OpenCodeCliLine::V2);
        assert_eq!(OPENCODE_LEGACY_NPM_PACKAGE, "opencode-ai");
        assert_eq!(OPENCODE_V2_NPM_PACKAGE, "@opencode/cli");
    }

    mod bounded_metadata {
        use super::*;
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        enum Reply {
            Raw(Vec<u8>),
            Hang,
        }

        /// One-shot local HTTP server; returns its base URL and the raw
        /// request head it received.
        async fn serve_once(reply: Reply) -> (String, tokio::task::JoinHandle<String>) {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let handle = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut head = Vec::new();
                let mut buffer = [0u8; 1024];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = socket.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    head.extend_from_slice(&buffer[..read]);
                }
                match reply {
                    Reply::Raw(bytes) => {
                        let _ = socket.write_all(&bytes).await;
                        let _ = socket.shutdown().await;
                    }
                    Reply::Hang => tokio::time::sleep(Duration::from_secs(5)).await,
                }
                String::from_utf8_lossy(&head).into_owned()
            });
            (format!("http://{address}/metadata"), handle)
        }

        fn plain_test_client() -> reqwest::Client {
            // Mirrors the production metadata policy except HTTPS-only, so a
            // loopback fixture can exercise status, size and timeout paths.
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_millis(400))
                .no_proxy()
                .build()
                .unwrap()
        }

        fn response(status: &str, headers: &str, body: &[u8]) -> Reply {
            let mut bytes =
                format!("HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n").into_bytes();
            bytes.extend_from_slice(body);
            Reply::Raw(bytes)
        }

        #[tokio::test]
        async fn success_returns_body_and_sends_accept_header() {
            let (url, server) =
                serve_once(response("200 OK", "Content-Length: 5\r\n", b"hello")).await;
            let body = fetch_bounded_metadata(
                &plain_test_client(),
                &url,
                "application/vnd.github+json",
                16,
            )
            .await;
            assert_eq!(body.as_deref(), Some(b"hello".as_slice()));
            let head = server.await.unwrap().to_ascii_lowercase();
            assert!(head.contains("accept: application/vnd.github+json"));
            assert!(head.contains("user-agent: fyagent"));
        }

        #[tokio::test]
        async fn non_success_status_is_unknown() {
            let (url, _server) =
                serve_once(response("404 Not Found", "Content-Length: 2\r\n", b"no")).await;
            assert_eq!(
                fetch_bounded_metadata(&plain_test_client(), &url, "text/plain", 16).await,
                None
            );
        }

        #[tokio::test]
        async fn redirect_is_not_followed() {
            let (url, _server) = serve_once(response(
                "302 Found",
                "Location: https://evil.example/pyproject.toml\r\nContent-Length: 0\r\n",
                b"",
            ))
            .await;
            assert_eq!(
                fetch_bounded_metadata(&plain_test_client(), &url, "text/plain", 16).await,
                None
            );
        }

        #[tokio::test]
        async fn declared_oversize_is_rejected() {
            let (url, _server) =
                serve_once(response("200 OK", "Content-Length: 17\r\n", &[b'a'; 17])).await;
            assert_eq!(
                fetch_bounded_metadata(&plain_test_client(), &url, "text/plain", 16).await,
                None
            );
        }

        #[tokio::test]
        async fn streamed_oversize_without_length_is_rejected() {
            let (url, _server) = serve_once(response("200 OK", "", &[b'a'; 64])).await;
            assert_eq!(
                fetch_bounded_metadata(&plain_test_client(), &url, "text/plain", 16).await,
                None
            );
        }

        #[tokio::test]
        async fn timeout_is_unknown() {
            let (url, _server) = serve_once(Reply::Hang).await;
            assert_eq!(
                fetch_bounded_metadata(&plain_test_client(), &url, "text/plain", 16).await,
                None
            );
        }

        #[tokio::test]
        async fn production_metadata_policy_refuses_plain_http() {
            // Same builder as production minus the ambient proxy adapter.
            let client = super::super::super::grok_npm::metadata_client_builder()
                .no_proxy()
                .build()
                .unwrap();
            let (url, _server) =
                serve_once(response("200 OK", "Content-Length: 5\r\n", b"hello")).await;
            assert_eq!(
                fetch_bounded_metadata(&client, &url, "text/plain", 16).await,
                None
            );
        }
    }
}
