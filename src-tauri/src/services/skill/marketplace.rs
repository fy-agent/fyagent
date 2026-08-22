use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::app_config::{InstalledSkill, SkillTargetId};
use crate::database::Database;
use crate::error::format_skill_error;

use super::{SkillService, ZipInstallProvenance, SKILL_DISCOVERY_MAX_LIMIT};

// skills.sh API types -------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct SkillsShApiResponse {
    pub query: String,
    #[serde(rename = "searchType")]
    #[allow(dead_code)]
    pub search_type: String,
    pub skills: Vec<SkillsShApiSkill>,
    pub count: usize,
    #[allow(dead_code)]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillsShApiSkill {
    pub id: String,
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShSearchResult {
    pub skills: Vec<SkillsShDiscoverableSkill>,
    pub total_count: usize,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsShDiscoverableSkill {
    pub key: String,
    pub name: String,
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub installs: u64,
    pub readme_url: Option<String>,
}

// SkillHub -----------------------------------------------------------------

const SKILLHUB_API_ORIGIN: &str = "https://api.skillhub.cn";
const SKILLHUB_LIST_PATH: &str = "/api/skills";
const SKILLHUB_DOWNLOAD_PATH: &str = "/api/v1/download";
const SKILLHUB_PUBLIC_SKILL_PREFIX: &str = "https://skillhub.cn/skills/";
pub const SKILLHUB_MARKET_OWNER: &str = "skillhub.cn";
const SKILLHUB_QUERY_MAX_CHARS: usize = 200;
const SKILLHUB_DEFAULT_PAGE_SIZE: usize = 21;
const SKILLHUB_OFFICIAL_CATEGORIES: &[(&str, &str)] = &[
    ("office-efficiency", "办公效率"),
    ("content-creation", "内容创作"),
    ("dev-programming", "开发编程"),
    ("data-analysis", "数据分析"),
    ("design-media", "设计多媒体"),
    ("ai-agent", "AI Agent"),
    ("knowledge-management", "知识管理"),
    ("business-ops", "商业运营"),
    ("education", "教育学习"),
    ("professional", "行业专业"),
    ("it-ops-security", "IT 运维与安全"),
    ("life-service", "生活服务"),
];

#[derive(Debug, Clone, Deserialize)]
struct SkillHubListApiResponse {
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: SkillHubListApiData,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SkillHubListApiData {
    #[serde(default)]
    total: usize,
    #[serde(default)]
    skills: Vec<SkillHubApiSkill>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillHubApiSkill {
    slug: String,
    #[serde(default, alias = "displayName")]
    display_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, alias = "descriptionZh")]
    description_zh: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default, alias = "ownerName")]
    owner_name: Option<String>,
    #[serde(default)]
    downloads: Option<u64>,
    #[serde(default)]
    installs: Option<u64>,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubSearchResult {
    pub skills: Vec<SkillHubDiscoverableSkill>,
    pub total_count: usize,
    pub query: String,
    #[serde(default)]
    pub categories: Vec<SkillHubCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubCategory {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillHubDiscoverableSkill {
    pub key: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub directory: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub repo_branch: String,
    pub version: Option<String>,
    pub owner_name: Option<String>,
    pub installs: Option<u64>,
    pub downloads: Option<u64>,
    pub homepage_url: String,
    pub readme_url: Option<String>,
    pub category: Option<String>,
}

fn clamp_skillhub_page_size(limit: usize) -> usize {
    if limit == 0 {
        SKILLHUB_DEFAULT_PAGE_SIZE
    } else {
        limit.min(SKILL_DISCOVERY_MAX_LIMIT)
    }
}

fn official_skillhub_categories() -> Vec<SkillHubCategory> {
    SKILLHUB_OFFICIAL_CATEGORIES
        .iter()
        .map(|(key, name)| SkillHubCategory {
            key: (*key).to_string(),
            name: (*name).to_string(),
        })
        .collect()
}

pub(super) async fn search_skills_sh(
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult> {
    let client = crate::proxy::http_client::get();
    let url = url::Url::parse_with_params(
        "https://skills.sh/api/search",
        &[
            ("q", query),
            ("limit", &limit.to_string()),
            ("offset", &offset.to_string()),
        ],
    )?;
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json::<SkillsShApiResponse>()
        .await?;

    let skills = resp
        .skills
        .into_iter()
        .filter_map(|skill| {
            let parts: Vec<&str> = skill.source.splitn(2, '/').collect();
            if parts.len() != 2 {
                return None;
            }
            let (owner, repo) = (parts[0].to_string(), parts[1].to_string());
            if SkillService::validate_repo_ref(&owner, &repo, "main").is_err() {
                return None;
            }
            Some(SkillsShDiscoverableSkill {
                key: skill.id,
                name: skill.name,
                directory: skill.skill_id,
                repo_owner: owner.clone(),
                repo_name: repo.clone(),
                repo_branch: "main".to_string(),
                installs: skill.installs,
                readme_url: Some(format!("https://github.com/{owner}/{repo}")),
            })
        })
        .collect();

    Ok(SkillsShSearchResult {
        skills,
        total_count: resp.count,
        query: resp.query,
    })
}

pub(super) fn is_valid_skillhub_slug(slug: &str) -> bool {
    if slug.is_empty() || slug.len() > 128 || slug == "." || slug == ".." {
        return false;
    }
    let mut chars = slug.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn clamp_skillhub_query(query: &str) -> String {
    query.chars().take(SKILLHUB_QUERY_MAX_CHARS).collect()
}

fn first_nonempty(values: &[Option<String>]) -> Option<String> {
    values.iter().find_map(|value| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub(super) fn skillhub_homepage_url(slug: &str) -> Option<String> {
    if !is_valid_skillhub_slug(slug) {
        return None;
    }
    Some(format!("{SKILLHUB_PUBLIC_SKILL_PREFIX}{slug}"))
}

fn assert_skillhub_api_url(url: &url::Url, expected_path: &str) -> Result<()> {
    if url.scheme() != "https"
        || url.host_str() != Some("api.skillhub.cn")
        || url.path() != expected_path
    {
        return Err(anyhow!(format_skill_error(
            "INVALID_SKILLHUB_URL",
            &[("url", url.as_str())],
            Some("checkNetwork"),
        )));
    }
    Ok(())
}

pub(super) fn normalize_skillhub_category(raw: Option<&str>) -> Option<&'static str> {
    let value = raw.map(str::trim).filter(|text| !text.is_empty())?;
    let compact = value.to_ascii_lowercase().replace('_', "-");
    SKILLHUB_OFFICIAL_CATEGORIES
        .iter()
        .find(|(key, name)| compact == *key || value == *name)
        .map(|(key, _)| *key)
}

pub(super) fn skillhub_list_url(
    query: &str,
    category: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<url::Url> {
    let page = page.max(1);
    let page_size = clamp_skillhub_page_size(page_size);
    let category_key = normalize_skillhub_category(category);
    let sort_by = if query.is_empty() && category_key.is_some() {
        "downloads"
    } else {
        "score"
    };
    let page_s = page.to_string();
    let page_size_s = page_size.to_string();
    let mut params: Vec<(&str, &str)> = vec![
        ("page", page_s.as_str()),
        ("pageSize", page_size_s.as_str()),
        ("sortBy", sort_by),
    ];
    if !query.is_empty() {
        params.push(("keyword", query));
    }
    if let Some(key) = category_key {
        params.push(("category", key));
    }
    let url = url::Url::parse_with_params(
        &format!("{SKILLHUB_API_ORIGIN}{SKILLHUB_LIST_PATH}"),
        &params,
    )?;
    assert_skillhub_api_url(&url, SKILLHUB_LIST_PATH)?;
    Ok(url)
}

pub(super) fn skillhub_download_url(slug: &str) -> Result<url::Url> {
    if !is_valid_skillhub_slug(slug) {
        return Err(anyhow!(format_skill_error(
            "INVALID_SKILLHUB_SLUG",
            &[("slug", slug)],
            Some("checkNetwork"),
        )));
    }
    let url = url::Url::parse_with_params(
        &format!("{SKILLHUB_API_ORIGIN}{SKILLHUB_DOWNLOAD_PATH}"),
        &[("slug", slug)],
    )?;
    assert_skillhub_api_url(&url, SKILLHUB_DOWNLOAD_PATH)?;
    let slug_param = url
        .query_pairs()
        .find(|(key, _)| key == "slug")
        .map(|(_, value)| value.into_owned());
    if slug_param.as_deref() != Some(slug) {
        return Err(anyhow!(format_skill_error(
            "INVALID_SKILLHUB_SLUG",
            &[("slug", slug)],
            Some("checkNetwork"),
        )));
    }
    Ok(url)
}

fn map_skillhub_item(item: SkillHubApiSkill) -> Option<SkillHubDiscoverableSkill> {
    if !is_valid_skillhub_slug(&item.slug) {
        return None;
    }
    let homepage = skillhub_homepage_url(&item.slug)?;
    let name = first_nonempty(&[item.display_name.clone(), item.name.clone()])
        .unwrap_or_else(|| item.slug.clone());
    let description = first_nonempty(&[
        item.description_zh.clone(),
        item.description.clone(),
        item.summary.clone(),
    ])
    .unwrap_or_default();
    Some(SkillHubDiscoverableSkill {
        key: format!("skillhub:{}", item.slug),
        slug: item.slug.clone(),
        name,
        description,
        directory: item.slug.clone(),
        repo_owner: SKILLHUB_MARKET_OWNER.to_string(),
        repo_name: item.slug.clone(),
        repo_branch: item
            .version
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "skillhub".to_string()),
        version: first_nonempty(&[item.version]),
        owner_name: first_nonempty(&[item.owner_name]),
        installs: item.installs,
        downloads: item.downloads,
        homepage_url: homepage.clone(),
        readme_url: Some(homepage),
        category: normalize_skillhub_category(item.category.as_deref()).map(str::to_string),
    })
}

fn dedupe_skillhub_by_slug(
    skills: Vec<SkillHubDiscoverableSkill>,
) -> Vec<SkillHubDiscoverableSkill> {
    let mut seen = HashSet::new();
    skills
        .into_iter()
        .filter(|skill| seen.insert(skill.slug.clone()))
        .collect()
}

pub(super) async fn search_skillhub(
    query: &str,
    limit: usize,
    offset: usize,
    category: Option<&str>,
) -> Result<SkillHubSearchResult> {
    let query = clamp_skillhub_query(query);
    let page_size = clamp_skillhub_page_size(limit);
    let page = offset / page_size.max(1) + 1;
    let url = skillhub_list_url(&query, category, page, page_size)?;
    let client = crate::proxy::http_client::get();
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?
        .json::<SkillHubListApiResponse>()
        .await?;
    if resp.code != 0 {
        return Err(anyhow!(format_skill_error(
            "SKILLHUB_LIST_FAILED",
            &[("code", &resp.code.to_string()), ("message", &resp.message)],
            Some("checkNetwork"),
        )));
    }
    let skills = dedupe_skillhub_by_slug(
        resp.data
            .skills
            .into_iter()
            .filter_map(map_skillhub_item)
            .collect(),
    );
    Ok(SkillHubSearchResult {
        skills,
        total_count: resp.data.total,
        query,
        categories: official_skillhub_categories(),
    })
}

pub(super) async fn install_skillhub(
    db: &Arc<Database>,
    slug: &str,
    current_app: &SkillTargetId,
) -> Result<Vec<InstalledSkill>> {
    let url = skillhub_download_url(slug)?;
    let bytes = SkillService::download_bounded_bytes(url, Duration::from_secs(60)).await?;
    let temp_root = crate::config::get_user_temp_dir();
    fs::create_dir_all(&temp_root)?;
    let mut tmp = tempfile::Builder::new()
        .prefix("skillhub-")
        .suffix(".zip")
        .tempfile_in(&temp_root)?;
    tmp.write_all(&bytes)?;
    tmp.flush()?;
    let homepage = skillhub_homepage_url(slug);
    let provenance = ZipInstallProvenance {
        id: format!("skillhub:{slug}"),
        repo_owner: SKILLHUB_MARKET_OWNER.to_string(),
        repo_name: slug.to_string(),
        repo_branch: "skillhub".to_string(),
        readme_url: homepage,
    };
    SkillService::install_from_zip_with_provenance(db, tmp.path(), current_app, Some(&provenance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_zh_description_and_official_list_envelope() {
        let raw = serde_json::json!({
            "code": 0,
            "data": {
                "total": 2,
                "skills": [
                    {
                        "slug": "tencent-docs",
                        "displayName": "腾讯文档",
                        "description_zh": "中文介绍",
                        "description": "English intro",
                        "version": "1.0.41",
                        "owner_name": "tencent-adm",
                        "installs": 8107,
                        "category": "office-efficiency"
                    },
                    {
                        "slug": "../evil",
                        "name": "skip me"
                    },
                    {
                        "slug": "summarize",
                        "name": "Summarize",
                        "summary": "摘要",
                        "category": "开发编程"
                    }
                ]
            }
        });
        let parsed: SkillHubListApiResponse = serde_json::from_value(raw).expect("parse");
        assert_eq!(parsed.data.total, 2);
        let skills: Vec<_> = parsed
            .data
            .skills
            .into_iter()
            .filter_map(map_skillhub_item)
            .collect();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "腾讯文档");
        assert_eq!(skills[0].description, "中文介绍");
        assert_eq!(skills[0].repo_owner, SKILLHUB_MARKET_OWNER);
        assert_eq!(skills[0].category.as_deref(), Some("office-efficiency"));
        assert_eq!(
            skills[0].homepage_url,
            "https://skillhub.cn/skills/tencent-docs"
        );
        assert_eq!(skills[1].description, "摘要");
        assert_eq!(skills[1].category.as_deref(), Some("dev-programming"));
    }

    #[test]
    fn page_size_defaults_to_21_and_clamps_page_size_only() {
        assert_eq!(clamp_skillhub_page_size(0), 21);
        assert_eq!(clamp_skillhub_page_size(21), 21);
        assert_eq!(clamp_skillhub_page_size(50), 50);
        assert_eq!(clamp_skillhub_page_size(100), 50);
        let page_two = skillhub_list_url("", None, 2, 0).expect("default size");
        assert!(page_two
            .query_pairs()
            .any(|(key, value)| key == "pageSize" && value == "21"));
        assert!(page_two
            .query_pairs()
            .any(|(key, value)| key == "page" && value == "2"));
        assert_eq!(official_skillhub_categories().len(), 12);
    }
}
