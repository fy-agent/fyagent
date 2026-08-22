use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::app_config::InstalledSkill;

use super::{DiscoverableSkill, DiscoverableSkillsPage, SkillRepo};

const DEFAULT_LIMIT: usize = 20;
pub(super) const MAX_LIMIT: usize = 50;
const CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Repository discovery installation-state filter. The command layer rejects
/// unknown values instead of silently widening them to `all`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillDiscoveryStatus {
    All,
    Installed,
    Uninstalled,
}

impl SkillDiscoveryStatus {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "all" => Ok(Self::All),
            "installed" => Ok(Self::Installed),
            "uninstalled" => Ok(Self::Uninstalled),
            _ => Err("无效的安装状态筛选".to_string()),
        }
    }
}

/// Service-level paging request. IPC commands keep their existing flat wire
/// fields and translate into this value before entering the service.
pub struct DiscoverAvailablePageRequest<'a> {
    pub query: &'a str,
    pub repo: Option<&'a str>,
    pub status: SkillDiscoveryStatus,
    pub limit: usize,
    pub offset: usize,
}

struct CacheEntry {
    fingerprint: String,
    skills: Vec<DiscoverableSkill>,
    fetched_at: Instant,
}

pub(super) struct DiscoveryCache {
    entry: Mutex<Option<CacheEntry>>,
}

impl DiscoveryCache {
    pub(super) fn new() -> Self {
        Self {
            entry: Mutex::new(None),
        }
    }

    pub(super) fn invalidate(&self) {
        *self.lock() = None;
    }

    pub(super) fn get(&self, fingerprint: &str) -> Option<Vec<DiscoverableSkill>> {
        let cache = self.lock();
        let entry = cache.as_ref()?;
        if entry.fingerprint != fingerprint || entry.fetched_at.elapsed() >= CACHE_TTL {
            return None;
        }
        Some(entry.skills.clone())
    }

    pub(super) fn store(&self, fingerprint: String, skills: Vec<DiscoverableSkill>) {
        *self.lock() = Some(CacheEntry {
            fingerprint,
            skills,
            fetched_at: Instant::now(),
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<CacheEntry>> {
        self.entry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub(super) fn clamp_discovery_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_LIMIT
    } else {
        limit.min(MAX_LIMIT)
    }
}

pub(super) fn discovery_fingerprint(repos: &[SkillRepo]) -> String {
    let mut keys: Vec<String> = repos
        .iter()
        .map(|repo| format!("{}/{}/{}", repo.owner, repo.name, repo.branch))
        .collect();
    keys.sort();
    keys.join("\n")
}

pub(super) fn directory_tail(directory: &str) -> String {
    directory
        .split(['/', '\\'])
        .rfind(|part| !part.is_empty())
        .unwrap_or("")
        .to_lowercase()
}

pub(super) fn is_discoverable_installed(
    skill: &DiscoverableSkill,
    installed: &[InstalledSkill],
) -> bool {
    let tail = directory_tail(&skill.directory);
    installed.iter().any(|item| {
        directory_tail(&item.directory) == tail
            && item.repo_owner.as_deref().unwrap_or("").to_lowercase()
                == skill.repo_owner.to_lowercase()
            && item.repo_name.as_deref().unwrap_or("").to_lowercase()
                == skill.repo_name.to_lowercase()
    })
}

pub(super) fn filter_discoverable_skills(
    skills: &[DiscoverableSkill],
    installed: &[InstalledSkill],
    query: &str,
    repo: Option<&str>,
    status: SkillDiscoveryStatus,
) -> Vec<DiscoverableSkill> {
    let query = query.trim().to_lowercase();
    skills
        .iter()
        .filter(|skill| {
            if !query.is_empty() {
                let haystack = format!(
                    "{} {} {}/{}",
                    skill.name, skill.description, skill.repo_owner, skill.repo_name
                )
                .to_lowercase();
                if !haystack.contains(&query) {
                    return false;
                }
            }
            if let Some(repo_key) = repo {
                if format!("{}/{}", skill.repo_owner, skill.repo_name) != repo_key {
                    return false;
                }
            }
            match status {
                SkillDiscoveryStatus::All => true,
                SkillDiscoveryStatus::Installed => is_discoverable_installed(skill, installed),
                SkillDiscoveryStatus::Uninstalled => !is_discoverable_installed(skill, installed),
            }
        })
        .cloned()
        .collect()
}

pub(super) fn paginate_discoverable_skills(
    skills: &[DiscoverableSkill],
    limit: usize,
    offset: usize,
) -> DiscoverableSkillsPage {
    DiscoverableSkillsPage {
        skills: skills.iter().skip(offset).take(limit).cloned().collect(),
        total_count: skills.len(),
    }
}
