//! Read-only projection of the same ownership proof used by proxy exit.

use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyRestorePreview {
    pub app: String,
    pub enabled: bool,
    pub can_restore: bool,
    pub targets: Vec<ProxyRestoreTarget>,
}

#[derive(Debug, Serialize)]
pub struct ProxyRestoreTarget {
    pub path: String,
    pub exists: bool,
}

impl ProxyService {
    pub async fn get_restore_preview(&self, app: &str) -> Result<ProxyRestorePreview, String> {
        let target = match app {
            "claude" => AppType::Claude,
            "codex" => AppType::Codex,
            "grokbuild" => AppType::GrokBuild,
            _ => return Err("Unsupported proxy restore target".into()),
        };
        let _guard = self.switch_locks.lock_for_app(app).await;
        let enabled = self
            .db
            .get_proxy_config_for_app(app)
            .await
            .map_err(|_| "Proxy restore status unavailable")?
            .enabled;
        let mut preview = ProxyRestorePreview {
            app: app.to_owned(),
            enabled,
            can_restore: false,
            targets: Vec::new(),
        };
        if !enabled {
            return Ok(preview);
        }
        // A corrupt/missing backup or a changed file is not a grant to restore.
        // Do not return backup content, credentials, or unfiltered native errors.
        if let Ok(targets) = self.verified_restore_preview_targets(&target) {
            preview.can_restore = true;
            preview.targets = targets;
        } else if self.db.get_live_backup_sync(app).ok().flatten().is_some() {
            // Conflict metadata uses the closed native path list only. Never
            // trust the conflicting backup or treat disclosure as restore authority.
            if let Ok(targets) = Self::closed_restore_preview_targets(&target) {
                preview.targets = targets;
            }
        }
        Ok(preview)
    }

    fn verified_restore_preview_targets(
        &self,
        app: &AppType,
    ) -> Result<Vec<ProxyRestoreTarget>, String> {
        let saved = self
            .db
            .get_live_backup_sync(app.as_str())
            .map_err(|_| "Proxy restore backup unavailable")?
            .ok_or("Proxy restore backup missing")?;
        let original: Value = serde_json::from_str(&saved.original_config)
            .map_err(|_| "Proxy restore backup unavailable")?;
        let paths = match self.managed_restore_preview_paths(app, &original)? {
            Some(paths) => paths,
            None => self.legacy_restore_preview_paths(app, &original)?,
        };
        Self::preview_targets(&paths)
    }

    fn closed_restore_preview_targets(app: &AppType) -> Result<Vec<ProxyRestoreTarget>, String> {
        Self::preview_targets(&Self::closed_restore_preview_paths(app)?)
    }

    fn preview_targets(paths: &[std::path::PathBuf]) -> Result<Vec<ProxyRestoreTarget>, String> {
        paths
            .iter()
            .map(|path| {
                let target = crate::config::file_write_target(path)
                    .map_err(|_| "Proxy restore target unavailable")?;
                Ok(ProxyRestoreTarget {
                    path: target.path,
                    exists: target.exists,
                })
            })
            .collect()
    }

    /// The ordinary selector repairs stale preferences. A preview must not.
    pub(super) fn current_provider_for_restore_preview(
        &self,
        app: &AppType,
    ) -> Result<Option<Provider>, String> {
        let selected = crate::settings::get_current_provider(app)
            .map(|id| Ok(Some(id)))
            .unwrap_or_else(|| self.db.get_current_provider(app.as_str()))
            .map_err(|_| "Proxy restore selection unavailable")?;
        let Some(id) = selected else { return Ok(None) };
        self.db
            .get_provider_by_id(&id, app.as_str())
            .map_err(|_| "Proxy restore provider unavailable")?
            .ok_or_else(|| "Proxy restore provider unavailable".to_owned())
            .map(Some)
    }
}
