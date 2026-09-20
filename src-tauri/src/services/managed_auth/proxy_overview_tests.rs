use super::*;
use crate::services::provider::{BindManagedProxyRequest, ProviderService};
use crate::services::secret::MemorySecretBackend;
use crate::store::AppState;
use serial_test::serial;

struct TestHome {
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
        crate::settings::reload_settings().unwrap();
    }
}

fn fixture() -> (TestHome, ManagedAuthService<MemorySecretBackend>, AppState) {
    let directory = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("FYAGENT_TEST_HOME");
    std::env::set_var("FYAGENT_TEST_HOME", directory.path());
    crate::settings::reload_settings().unwrap();
    let db = Arc::new(Database::memory().unwrap());
    let auth = ManagedAuthService::new(
        db.clone(),
        SecretService::new(MemorySecretBackend::new()),
        directory.path().join("vault-meta"),
    );
    let state = AppState::new(db);
    (
        TestHome {
            _directory: directory,
            previous,
        },
        auth,
        state,
    )
}

fn seed(
    auth: &ManagedAuthService<MemorySecretBackend>,
    provider: ManagedAuthProvider,
    legacy: &str,
    subject: &str,
) -> CredentialRecord {
    auth.provision_legacy_credential(LegacyCredentialInput {
        migration_id: None,
        provider,
        purpose: CredentialPurpose::ProxyUpstream,
        consumer: Some(ManagedAuthConsumer::FyagentProxy),
        legacy_account_id: legacy.into(),
        provider_subject: subject.into(),
        provider_tenant: String::new(),
        login: format!("{subject}@example.test"),
        display_name: None,
        avatar_url: None,
        access_token: Some(Zeroizing::new(format!("synthetic-access-{legacy}"))),
        refresh_token: Some(Zeroizing::new(format!("synthetic-refresh-{legacy}"))),
        id_token: None,
        desired_status: CredentialStatus::Ready,
        refresh_owner: RefreshOwner::Fyagent,
        authenticated_at: 1_700_000_000,
        make_default: true,
    })
    .unwrap()
}

#[test]
#[serial]
fn subscription_overview_observes_nondefault_and_default_accounts_independently() {
    for provider in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        let (_home, auth, state) = fixture();
        let first = seed(&auth, provider, "first", "first");
        let second = seed(&auth, provider, "second", "second");
        let rows = auth.repository.list_all_credentials().unwrap();
        assert!(
            !rows
                .iter()
                .find(|row| row.credential.credential_id == first.credential_id)
                .unwrap()
                .is_default
        );
        assert!(
            rows.iter()
                .find(|row| row.credential.credential_id == second.credential_id)
                .unwrap()
                .is_default
        );

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _entered = runtime.enter();
        let mut global = runtime
            .block_on(state.db.get_global_proxy_config())
            .unwrap();
        global.listen_address = "127.0.0.1".into();
        global.listen_port = 0;
        runtime
            .block_on(state.db.update_global_proxy_config(global))
            .unwrap();
        for (app, account) in [("claude", &first), ("grokbuild", &second)] {
            ProviderService::bind_managed_proxy(
                &state,
                &auth,
                BindManagedProxyRequest {
                    app: app.into(),
                    account_id: account.identity_id.clone(),
                    model_id: if provider == ManagedAuthProvider::Openai {
                        "gpt-5-codex"
                    } else {
                        "grok-build"
                    }
                    .into(),
                },
            )
            .unwrap();
        }

        // This is the production overview builder and the real ProxyService
        // observer, with only the Tauri AppHandle lookup supplied directly.
        let overview = auth
            .overview_inner_with_proxy_observer(|kind, account, is_default| {
                state
                    .proxy_service
                    .observe_managed_account_route(kind, account, is_default)
            })
            .unwrap();
        for credential in [&first, &second] {
            let connections: Vec<_> = overview
                .connections
                .iter()
                .filter(|connection| {
                    connection.consumer == ManagedAuthConsumer::FyagentProxy
                        && connection.account_id.as_deref() == Some(credential.identity_id.as_str())
                })
                .collect();
            assert_eq!(connections.len(), 1);
            assert_eq!(
                connections[0].auth_status,
                ManagedAuthConnectionState::Connected
            );
            assert_eq!(
                connections[0].request_mode,
                ManagedAuthRequestMode::OfficialSubscription
            );
            assert_eq!(connections[0].target_id, None);
            let account = overview
                .accounts
                .iter()
                .find(|account| account.account_id == credential.identity_id)
                .unwrap();
            assert_eq!(account.connected_consumer_count, 1);
            let preview = auth
                .preview_account_removal(&account.account_id, &account.revision)
                .unwrap();
            assert_eq!(
                preview
                    .disconnects
                    .iter()
                    .filter(|impact| impact.consumer == ManagedAuthConsumer::FyagentProxy)
                    .count(),
                1
            );
        }
        let serialized = serde_json::to_string(&overview).unwrap();
        for secret in ["synthetic-access-", "synthetic-refresh-", "mcred1:"] {
            assert!(!serialized.contains(secret));
        }
        runtime
            .block_on(state.proxy_service.stop_with_restore())
            .unwrap();
        let stopped = auth
            .overview_inner_with_proxy_observer(|kind, account, is_default| {
                state
                    .proxy_service
                    .observe_managed_account_route(kind, account, is_default)
            })
            .unwrap();
        assert!(stopped
            .connections
            .iter()
            .filter(|connection| connection.consumer == ManagedAuthConsumer::FyagentProxy)
            .all(|connection| {
                connection.auth_status == ManagedAuthConnectionState::Disconnected
                    && connection.request_mode == ManagedAuthRequestMode::None
                    && connection.request_provider_label.is_none()
            }));
    }
}

#[test]
#[serial]
fn subscription_proxy_slots_remove_superseded_and_orphaned_rows_only() {
    let (_home, auth, _state) = fixture();
    let credential = seed(&auth, ManagedAuthProvider::Openai, "first", "first");
    auth.upsert_proxy_connections().unwrap();
    let current = auth
        .repository
        .list_connections()
        .unwrap()
        .into_iter()
        .find(|row| row.consumer == ManagedAuthConsumer::FyagentProxy)
        .unwrap();
    let mut legacy = current.clone();
    legacy.provider_slot = "openai".into();
    legacy.connection_id = stable_connection_id(legacy.consumer, "", &legacy.provider_slot);
    auth.repository.upsert_connection(&legacy).unwrap();
    let mut orphan = current.clone();
    orphan.provider_slot = format!("openai:mcred1:{}", "f".repeat(32));
    orphan.connection_id = stable_connection_id(orphan.consumer, "", &orphan.provider_slot);
    orphan.credential_id = None;
    auth.repository.upsert_connection(&orphan).unwrap();

    // Unrecognized user slots, lifecycle-target slots, and native consumers
    // cannot be pruned by this subscription owner's migration.
    let mut unrelated = current.clone();
    unrelated.provider_slot = "openai:user-defined".into();
    unrelated.connection_id =
        stable_connection_id(unrelated.consumer, "", &unrelated.provider_slot);
    unrelated.credential_id = None;
    auth.repository.upsert_connection(&unrelated).unwrap();
    let mut targeted = legacy.clone();
    targeted.target_id = "other-target".into();
    targeted.connection_id = stable_connection_id(
        targeted.consumer,
        &targeted.target_id,
        &targeted.provider_slot,
    );
    targeted.credential_id = None;
    auth.repository.upsert_connection(&targeted).unwrap();
    let mut native = legacy.clone();
    native.consumer = ManagedAuthConsumer::Opencode;
    native.connection_id = stable_connection_id(native.consumer, "", &native.provider_slot);
    native.credential_id = None;
    auth.repository.upsert_connection(&native).unwrap();

    let observed = auth
        .observe_overview_inner_with_proxy_observer(|_, _, _| Some(true))
        .unwrap();
    for retained in [&current, &unrelated, &targeted] {
        assert!(observed
            .connections
            .iter()
            .any(|row| row.connection_id == retained.connection_id));
    }
    for removed in [&legacy, &orphan] {
        assert!(!observed
            .connections
            .iter()
            .any(|row| row.connection_id == removed.connection_id));
        assert!(
            auth.repository
                .list_connections()
                .unwrap()
                .iter()
                .any(|row| row.connection_id == removed.connection_id),
            "Health must not persist reconciliation"
        );
    }
    auth.upsert_proxy_connections().unwrap();
    let rows = auth.repository.list_connections().unwrap();
    for retained in [&current, &unrelated, &targeted, &native] {
        assert!(rows
            .iter()
            .any(|row| row.connection_id == retained.connection_id));
    }
    for removed in [&legacy, &orphan] {
        assert!(!rows
            .iter()
            .any(|row| row.connection_id == removed.connection_id));
    }
    assert_eq!(
        rows.iter()
            .filter(|row| row.credential_id.as_deref() == Some(credential.credential_id.as_str()))
            .count(),
        1
    );
    auth.remove_credential_record(&credential).unwrap();
    auth.upsert_proxy_connections().unwrap();
    assert!(!auth
        .repository
        .list_connections()
        .unwrap()
        .iter()
        .any(|row| row.connection_id == current.connection_id));
}

#[test]
#[serial]
fn subscription_overview_counts_one_consumer_for_multiple_credentials_on_one_account() {
    let (_home, auth, _state) = fixture();
    let first = seed(
        &auth,
        ManagedAuthProvider::Xai,
        "first-session",
        "one-account",
    );
    let second = seed(
        &auth,
        ManagedAuthProvider::Xai,
        "second-session",
        "one-account",
    );
    assert_eq!(first.identity_id, second.identity_id);
    assert_ne!(first.credential_id, second.credential_id);
    let overview = auth
        .overview_inner_with_proxy_observer(|_, _, _| Some(true))
        .unwrap();
    assert_eq!(overview.accounts.len(), 1);
    assert_eq!(overview.accounts[0].connected_consumer_count, 1);
    let proxy: Vec<_> = overview
        .connections
        .iter()
        .filter(|row| row.consumer == ManagedAuthConsumer::FyagentProxy)
        .collect();
    assert_eq!(proxy.len(), 2);
    assert_ne!(proxy[0].connection_id, proxy[1].connection_id);
    assert!(proxy
        .iter()
        .all(|connection| connection.account_id.as_deref() == Some(first.identity_id.as_str())));
}

#[test]
#[serial]
fn release_integration_health_observes_all_proxy_accounts_without_persisting_slots() {
    for provider in [ManagedAuthProvider::Openai, ManagedAuthProvider::Xai] {
        let (_home, auth, state) = fixture();
        let first = seed(&auth, provider, "first", "first");
        let second = seed(&auth, provider, "second", "second");
        assert!(auth.repository.list_connections().unwrap().is_empty());
        let before = state.db.export_sql_string().unwrap();
        for _ in 0..2 {
            let mut observed = Vec::new();
            let overview = auth
                .observe_overview_inner_with_proxy_observer(|_, account, is_default| {
                    observed.push((account.to_owned(), is_default));
                    Some(true)
                })
                .unwrap();
            assert!(observed.contains(&("first".into(), false)));
            assert!(observed.contains(&("second".into(), true)));
            for credential in [&first, &second] {
                let account = overview
                    .accounts
                    .iter()
                    .find(|row| row.account_id == credential.identity_id)
                    .unwrap();
                assert_eq!(account.connected_consumer_count, 1);
                assert!(overview
                    .connections
                    .iter()
                    .any(
                        |row| row.account_id.as_deref() == Some(credential.identity_id.as_str())
                            && row.consumer == ManagedAuthConsumer::FyagentProxy
                            && row.auth_status == ManagedAuthConnectionState::Connected
                    ));
            }
        }
        assert!(auth.repository.list_connections().unwrap().is_empty());
        // Export timestamps are presentation metadata, not persisted data.
        let rows = |value: &str| {
            value
                .lines()
                .filter(|line| !line.starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(rows(&state.db.export_sql_string().unwrap()), rows(&before));
    }
}
