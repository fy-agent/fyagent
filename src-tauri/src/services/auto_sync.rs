//! Shared scheduling mechanics, not a cloud transport or an event bus.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::Instant;

use crate::error::AppError;

const DEBOUNCE: Duration = Duration::from_secs(1);
const MAX_WAIT: Duration = Duration::from_secs(10);

pub(super) struct AutoSyncController {
    sender: OnceLock<Sender<String>>,
    suppression_depth: AtomicUsize,
}

impl AutoSyncController {
    pub const fn new() -> Self {
        Self {
            sender: OnceLock::new(),
            suppression_depth: AtomicUsize::new(0),
        }
    }

    pub fn suppress(&self) -> SuppressionGuard<'_> {
        self.suppression_depth.fetch_add(1, Ordering::SeqCst);
        SuppressionGuard(self)
    }

    /// Called inside SQLite's update hook. Never block or reenter the database.
    pub fn notify(&self, table: &str) {
        if self.suppression_depth.load(Ordering::SeqCst) > 0 || !should_trigger(table) {
            return;
        }
        if let Some(sender) = self.sender.get() {
            // One pending dirty hint is sufficient, including during an upload.
            let _ = sender.try_send(table.to_owned());
        }
    }

    fn subscribe_once(&self) -> Option<Receiver<String>> {
        let (sender, receiver) = mpsc::channel(1);
        self.sender.set(sender).ok()?;
        Some(receiver)
    }

    pub fn start<F, Fut>(&'static self, label: &'static str, upload: F)
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let Some(receiver) = self.subscribe_once() else {
            return;
        };
        tauri::async_runtime::spawn(run_worker(receiver, label, upload));
    }
}

async fn run_worker<F, Fut>(mut receiver: Receiver<String>, label: &str, mut upload: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    while let Some((table, count)) = next_batch(&mut receiver).await {
        log::debug!("[{label}][AutoSync] Triggered by table={table}, merged_changes={count}");
        if let Err(error) = upload().await {
            log::warn!("[{label}][AutoSync] Upload failed: {error}");
        }
    }
}

pub(super) struct SuppressionGuard<'a>(&'a AutoSyncController);

impl Drop for SuppressionGuard<'_> {
    fn drop(&mut self) {
        self.0.suppression_depth.fetch_sub(1, Ordering::SeqCst);
    }
}

fn should_trigger(table: &str) -> bool {
    matches!(
        table.trim().to_ascii_lowercase().as_str(),
        "providers"
            | "provider_endpoints"
            | "mcp_servers"
            | "prompts"
            | "skills"
            | "skill_repos"
            | "settings"
            | "proxy_config"
    )
}

async fn next_batch(receiver: &mut Receiver<String>) -> Option<(String, usize)> {
    let first = receiver.recv().await?;
    let deadline = Instant::now() + MAX_WAIT;
    let mut count = 1;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(DEBOUNCE.min(remaining), receiver.recv()).await {
            Ok(Some(_)) => count += 1,
            Ok(None) => return None,
            Err(_) => break,
        }
    }
    Some((first, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn writes_during_a_failed_upload_are_coalesced_into_one_later_upload() {
        let controller = AutoSyncController::new();
        let receiver = controller.subscribe_once().unwrap();
        let (entered, mut observations) = mpsc::unbounded_channel();
        let mut calls = 0;
        let worker = tokio::spawn(run_worker(receiver, "test", move || {
            calls += 1;
            let call = calls;
            let entered = entered.clone();
            async move {
                entered.send((call, Instant::now())).unwrap();
                tokio::time::sleep(Duration::from_secs(2)).await;
                if call == 1 {
                    Err(AppError::Config("test upload failure".into()))
                } else {
                    Ok(())
                }
            }
        }));
        controller.notify("providers");
        let (first, started) = observations.recv().await.unwrap();
        assert_eq!(first, 1);
        for _ in 0..10 {
            controller.notify("settings");
        }
        let (second, later) = observations.recv().await.unwrap();
        assert_eq!(second, 2);
        assert_eq!(later.duration_since(started), Duration::from_secs(3));
        tokio::time::advance(Duration::from_secs(20)).await;
        tokio::task::yield_now().await;
        assert!(observations.try_recv().is_err());
        worker.abort();
    }

    #[test]
    fn only_configuration_tables_trigger_sync() {
        for table in [
            "providers",
            "provider_endpoints",
            "mcp_servers",
            "prompts",
            "skills",
            "skill_repos",
            "settings",
            "proxy_config",
            " PROVIDERS ",
        ] {
            assert!(should_trigger(table), "{table}");
        }
        for table in [
            "proxy_request_logs",
            "provider_health",
            "managed_auth_credentials",
            "",
        ] {
            assert!(!should_trigger(table), "{table}");
        }
    }

    #[test]
    fn independent_nested_suppression_and_capacity_one_preserve_dirty_semantics() {
        let first = AutoSyncController::new();
        let second = AutoSyncController::new();
        first.notify("providers"); // No receiver yet; initialization is not replayed.
        let mut a = first.subscribe_once().unwrap();
        let mut b = second.subscribe_once().unwrap();
        assert!(first.subscribe_once().is_none());
        let outer = first.suppress();
        {
            let _inner = first.suppress();
            first.notify("providers");
            second.notify("providers");
            assert!(a.try_recv().is_err());
            assert_eq!(b.try_recv().unwrap(), "providers");
        }
        first.notify("settings");
        assert!(a.try_recv().is_err());
        drop(outer);
        first.notify("providers");
        first.notify("settings");
        assert_eq!(a.try_recv().unwrap(), "providers");
        assert!(a.try_recv().is_err());
        // A later write (e.g. while the prior upload runs) remains pending.
        first.notify("settings");
        assert_eq!(a.try_recv().unwrap(), "settings");
        drop(a);
        first.notify("providers"); // Closed receiver is harmless/nonblocking.
    }

    #[tokio::test(start_paused = true)]
    async fn quiet_batch_flushes_after_one_second() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send("providers".to_owned()).await.unwrap();
        let started = Instant::now();
        assert_eq!(
            next_batch(&mut receiver).await,
            Some(("providers".into(), 1))
        );
        assert_eq!(started.elapsed(), DEBOUNCE);
    }

    #[tokio::test(start_paused = true)]
    async fn continuous_changes_cannot_starve_the_ten_second_flush() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send("providers".to_owned()).await.unwrap();
        let producer = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if sender.send("settings".into()).await.is_err() {
                    break;
                }
            }
        });
        let started = Instant::now();
        let batch = next_batch(&mut receiver).await.unwrap();
        assert_eq!(batch.0, "providers");
        assert!(batch.1 > 1);
        assert_eq!(started.elapsed(), MAX_WAIT);
        producer.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn a_closed_channel_stops_without_starting_an_upload() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send("providers".into()).await.unwrap();
        drop(sender);
        assert_eq!(next_batch(&mut receiver).await, None);
    }
}
