use crate::codex_desktop_runtime::production_service;
use crate::database::Database;
use crate::secret::OpenedDeviceLocalSecretStore;
use crate::services::{CodexDesktopService, ProxyService, UsageCache};
use std::sync::Arc;

/// 全局应用状态
pub struct AppState {
    pub db: Arc<Database>,
    pub proxy_service: ProxyService,
    pub usage_cache: Arc<UsageCache>,
    /// Process-local installer state. Its factory is inert: no metadata or
    /// package I/O is performed while constructing ordinary application state.
    pub codex_desktop_service: Arc<CodexDesktopService>,
    /// Device-local secret store opened at startup. `new` leaves this unset so
    /// existing test callsites keep compiling; production setup uses
    /// `new_with_secret_store` after `SecretBootstrap::open`.
    pub(crate) secret_store: Option<OpenedDeviceLocalSecretStore>,
    /// Test-only InMemory backend hold. Production AppState never carries this.
    #[cfg(test)]
    pub(crate) secret_in_memory_backend: Option<crate::secret::InMemorySecretBackend>,
    /// Test-only process-local capture-intent registry. Production never carries this.
    #[cfg(test)]
    pub(crate) secret_capture_registry: Option<crate::secret::capture::SecretCaptureIntentRegistry>,
}

impl AppState {
    /// 创建新的应用状态
    pub fn new(db: Arc<Database>) -> Self {
        let proxy_service = ProxyService::new(db.clone());

        Self {
            db,
            proxy_service,
            usage_cache: Arc::new(UsageCache::new()),
            codex_desktop_service: Arc::new(production_service()),
            secret_store: None,
            #[cfg(test)]
            secret_in_memory_backend: None,
            #[cfg(test)]
            secret_capture_registry: None,
        }
    }

    pub(crate) fn new_with_secret_store(
        db: Arc<Database>,
        secret_store: OpenedDeviceLocalSecretStore,
    ) -> Self {
        let mut state = Self::new(db);
        state.secret_store = Some(secret_store);
        state
    }

    #[cfg(test)]
    pub(crate) fn attach_in_memory_secret_backend(
        &mut self,
        backend: crate::secret::InMemorySecretBackend,
    ) {
        self.secret_in_memory_backend = Some(backend);
    }

    #[cfg(test)]
    pub(crate) fn attach_secret_capture_registry(
        &mut self,
        registry: crate::secret::capture::SecretCaptureIntentRegistry,
    ) {
        self.secret_capture_registry = Some(registry);
    }
}
