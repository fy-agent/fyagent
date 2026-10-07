use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Emitter};

use super::auto_sync::{AutoSyncController, SuppressionGuard};
use crate::error::AppError;
use crate::services::webdav_sync;
use crate::settings::{self, WebDavSyncSettings};

static AUTO_SYNC: AutoSyncController = AutoSyncController::new();

pub(crate) struct AutoSyncSuppressionGuard {
    _guard: SuppressionGuard<'static>,
}

impl AutoSyncSuppressionGuard {
    pub fn new() -> Self {
        Self {
            _guard: AUTO_SYNC.suppress(),
        }
    }
}

impl Drop for AutoSyncSuppressionGuard {
    fn drop(&mut self) {
        let _ =
            AUTO_SYNC_SUPPRESS_DEPTH.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                Some(value.saturating_sub(1))
            });
    }
}

pub(crate) fn is_auto_sync_suppressed() -> bool {
    AUTO_SYNC_SUPPRESS_DEPTH.load(Ordering::SeqCst) > 0
}

pub fn should_trigger_for_table(table: &str) -> bool {
    should_trigger_auto_sync_for_table(table)
}

pub(crate) fn enqueue_change_signal(tx: &Sender<String>, table: &str) -> bool {
    match tx.try_send(table.to_string()) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) | Err(TrySendError::Closed(_)) => false,
    }
}

pub(crate) fn auto_sync_wait_duration(started_at: Instant, now: Instant) -> Option<Duration> {
    let max_wait = Duration::from_millis(MAX_AUTO_SYNC_WAIT_MS);
    let debounce = Duration::from_millis(AUTO_SYNC_DEBOUNCE_MS);
    let elapsed = now.saturating_duration_since(started_at);
    if elapsed >= max_wait {
        return None;
    }
    Some(debounce.min(max_wait - elapsed))
}

fn should_run_auto_sync(settings: Option<&WebDavSyncSettings>) -> bool {
    settings.is_some_and(|sync| sync.enabled && sync.auto_sync)
}

fn emit_auto_sync_status_updated(app: &AppHandle, status: &str, error: Option<&str>) {
    let mut payload = json!({ "source": "auto", "status": status });
    if let Some(error) = error {
        payload["error"] = json!(error);
    }
    if let Err(err) = app.emit("webdav-sync-status-updated", payload) {
        log::debug!("[WebDAV] failed to emit sync status update event: {err}");
    }
}

async fn run_auto_sync_upload(
    db: &crate::database::Database,
    app: &AppHandle,
) -> Result<(), AppError> {
    let mut settings = settings::get_webdav_sync_settings();
    if !should_run_auto_sync(settings.as_ref()) {
        return Ok(());
    }
    let Some(mut sync_settings) = settings.take() else {
        return Ok(());
    };
    match webdav_sync::run_with_sync_lock(webdav_sync::upload(db, &mut sync_settings)).await {
        Ok(_) => {
            emit_auto_sync_status_updated(app, "success", None);
            Ok(())
        }
        Err(err) => {
            sync_settings.status.last_error = Some(err.to_string());
            sync_settings.status.last_error_source = Some("auto".to_string());
            let _ = settings::update_webdav_sync_status(sync_settings.status.clone());
            emit_auto_sync_status_updated(app, "error", Some(&err.to_string()));
            Err(err)
        }
    }
}

pub fn notify_db_changed(table: &str) {
    AUTO_SYNC.notify(table);
}

pub fn start_worker(db: Arc<crate::database::Database>, app: AppHandle) {
    AUTO_SYNC.start("WebDAV", move || {
        let db = db.clone();
        let app = app.clone();
        async move { run_auto_sync_upload(&db, &app).await }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_upload_requires_both_webdav_settings_flags() {
        assert!(!should_run_auto_sync(None));
        for enabled in [false, true] {
            for auto_sync in [false, true] {
                let settings = WebDavSyncSettings {
                    enabled,
                    auto_sync,
                    ..Default::default()
                };
                assert_eq!(should_run_auto_sync(Some(&settings)), enabled && auto_sync);
            }
        }
    }
}
