//! The caller's deadline never releases capacity owned by an unfinished read.
use super::{collect, configuration, AgentHealthSnapshot};
use crate::{
    services::{
        external_agents::AgentCatalogId,
        managed_auth::{ManagedAuthOverview, ManagedAuthService},
        secret::SecretBackend,
    },
    store::AppState,
};
use std::{
    future::Future,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::sync::Semaphore;

static HEALTH_READS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(1)));
const CALLER_DEADLINE: Duration = Duration::from_secs(8);

pub(crate) async fn read<B: SecretBackend + 'static>(
    agent: AgentCatalogId,
    state: AppState,
    service: Arc<ManagedAuthService<B>>,
) -> Result<AgentHealthSnapshot, &'static str> {
    run_admitted(&HEALTH_READS, CALLER_DEADLINE, async move {
        let overview = read_overview(agent, move || service.observe_overview()).await;
        collect(agent, &state, overview).await
    })
    .await
}

async fn read_overview(
    agent: AgentCatalogId,
    read: impl FnOnce() -> ManagedAuthOverview + Send + 'static,
) -> ManagedAuthOverview {
    if configuration::app_type(agent).is_none() {
        return ManagedAuthOverview::unavailable();
    }
    tauri::async_runtime::spawn_blocking(read)
        .await
        .unwrap_or_else(|_| ManagedAuthOverview::unavailable())
}

async fn run_admitted<T: Send + 'static>(
    capacity: &Arc<Semaphore>,
    deadline: Duration,
    read: impl Future<Output = T> + Send + 'static,
) -> Result<T, &'static str> {
    let permit = capacity
        .clone()
        .try_acquire_owned()
        .map_err(|_| "health_busy")?;
    // Dropping this JoinHandle on timeout (or caller cancellation) detaches the
    // coordinator. It continues awaiting every started read and owns the permit
    // until actual completion. Inner readers must not detach their own work.
    let operation = tokio::spawn(async move {
        let _permit = permit;
        read.await
    });
    tokio::time::timeout(deadline, operation)
        .await
        .map_err(|_| "health_timeout")?
        .map_err(|_| "health_read_failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn health_vendor_checks_skip_managed_auth_overview() {
        let reads = Arc::new(AtomicUsize::new(0));
        for agent in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
        ] {
            let reader_calls = reads.clone();
            read_overview(agent, move || {
                reader_calls.fetch_add(1, Ordering::SeqCst);
                ManagedAuthOverview::unavailable()
            })
            .await;
        }
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn health_configuration_checks_retain_managed_auth_overview() {
        let reads = Arc::new(AtomicUsize::new(0));
        for agent in [
            AgentCatalogId::Codex,
            AgentCatalogId::ClaudeCode,
            AgentCatalogId::GrokBuild,
            AgentCatalogId::OpenCode,
        ] {
            let mut expected = ManagedAuthOverview::unavailable();
            expected.checked_at = "2026-09-20T00:00:00.000Z".into();
            expected.providers[0].available = true;
            let result = expected.clone();
            let reader_calls = reads.clone();
            let observed = read_overview(agent, move || {
                reader_calls.fetch_add(1, Ordering::SeqCst);
                result
            })
            .await;
            assert_eq!(
                serde_json::to_value(observed).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
        assert_eq!(reads.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn health_failed_overview_reader_retains_unavailable_fallback() {
        for agent in [
            AgentCatalogId::Codex,
            AgentCatalogId::ClaudeCode,
            AgentCatalogId::GrokBuild,
            AgentCatalogId::OpenCode,
        ] {
            let observed = read_overview(agent, || panic!("failed overview reader")).await;
            let mut expected = ManagedAuthOverview::unavailable();
            expected.checked_at.clone_from(&observed.checked_at);
            assert_eq!(
                serde_json::to_value(observed).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_timeout_keeps_admission_until_blocking_read_actually_finishes() {
        let capacity = Arc::new(Semaphore::new(1));
        let (release, blocked) = std::sync::mpsc::channel();
        let (started, observed_start) = tokio::sync::oneshot::channel();
        let first_capacity = capacity.clone();
        let first = tokio::spawn(async move {
            run_admitted(&first_capacity, Duration::from_millis(20), async move {
                tokio::task::spawn_blocking(move || {
                    started.send(()).unwrap();
                    blocked.recv().unwrap();
                    7
                })
                .await
                .unwrap()
            })
            .await
        });
        observed_start.await.unwrap();
        assert_eq!(first.await.unwrap(), Err("health_timeout"));
        assert_eq!(capacity.available_permits(), 0);
        let dispatched = Arc::new(AtomicUsize::new(0));
        let retry_dispatched = dispatched.clone();
        assert_eq!(
            run_admitted(&capacity, Duration::from_secs(1), async move {
                retry_dispatched.fetch_add(1, Ordering::SeqCst);
                8
            })
            .await,
            Err("health_busy")
        );
        assert_eq!(dispatched.load(Ordering::SeqCst), 0);
        release.send(()).unwrap();
        // Wait for the real reader/coordinator to release, not a timing sleep.
        let returned = tokio::time::timeout(Duration::from_secs(2), capacity.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(returned);
        assert_eq!(
            run_admitted(&capacity, Duration::from_secs(1), async { 9 }).await,
            Ok(9)
        );
        assert_eq!(capacity.available_permits(), 1);
    }
}
