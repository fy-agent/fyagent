use crate::codex_desktop_runtime::production_service;
use crate::database::Database;
use crate::services::{CodexDesktopService, ProxyService, UsageCache};
use std::sync::Arc;

/// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub(crate) projects: Arc<crate::services::projects::ProjectsService>,
    pub(crate) delivery_kits: Arc<std::sync::Mutex<crate::services::delivery_kits::KitLibrary>>,
    pub(crate) verification: Arc<crate::services::verification::VerificationService>,
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub usage_cache: Arc<UsageCache>,
    /// Process-local installer state. Its factory is inert: no metadata or
    /// package I/O is performed while constructing ordinary application state.
    pub codex_desktop_service: Arc<CodexDesktopService>,
    pub agent_action_jobs: Arc<crate::agent_install::AgentActionJobStore>,
    pub agent_auth_sessions: Arc<crate::agent_install::AgentAuthSessionStore>,
    pub agent_installation_inventory: Arc<crate::agent_install::AgentInstallationInventoryStore>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let proxy_service = ProxyService::new(db.clone());
        let fde = crate::services::fde_workspace::compose(
            db.clone(),
            crate::config::get_app_config_dir(),
        );

        Self {
            projects: fde.projects,
            delivery_kits: fde.kits,
            verification: fde.verification,
            db,
            proxy_service,
            usage_cache: Arc::new(UsageCache::new()),
            codex_desktop_service: Arc::new(production_service()),
            agent_action_jobs: Arc::new(crate::agent_install::AgentActionJobStore::new()),
            agent_auth_sessions: Arc::new(crate::agent_install::AgentAuthSessionStore::new()),
            agent_installation_inventory: Arc::new(
                crate::agent_install::AgentInstallationInventoryStore::new(),
            ),
        }
    }
}
