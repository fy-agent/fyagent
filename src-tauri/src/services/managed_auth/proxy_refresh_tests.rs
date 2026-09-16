//! Refresh policy tests use the real vault/CAS owner and an injected grant
//! exchange. They never call an authorization server or use a real account.

use super::*;
use crate::services::secret::MemorySecretBackend;
use std::sync::atomic::{AtomicUsize, Ordering};

fn fixture() -> (
    ManagedAuthService<MemorySecretBackend>,
    CredentialRecord,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let service = ManagedAuthService::new(
        Arc::new(Database::memory().unwrap()),
        SecretService::new(MemorySecretBackend::new()),
        dir.path().into(),
    );
    let credential = service.provision_legacy_credential(input()).unwrap();
    (service, credential, dir)
}

fn input() -> LegacyCredentialInput {
    LegacyCredentialInput {
        migration_id: None,
        provider: ManagedAuthProvider::Openai,
        purpose: CredentialPurpose::ProxyUpstream,
        consumer: Some(ManagedAuthConsumer::FyagentProxy),
        legacy_account_id: "refresh-fixture".into(),
        provider_subject: "fixture-subject".into(),
        provider_tenant: String::new(),
        login: "fixture@example.test".into(),
        display_name: None,
        avatar_url: None,
        access_token: None,
        refresh_token: Some(Zeroizing::new("synthetic-refresh".into())),
        id_token: None,
        desired_status: CredentialStatus::Ready,
        refresh_owner: RefreshOwner::Fyagent,
        authenticated_at: 1_700_000_000,
        make_default: true,
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn grant(access: &str) -> RefreshedGrant {
    RefreshedGrant {
        access_token: access.into(),
        refresh_token: Some("synthetic-rotated-refresh".into()),
        id_token: None,
        expires_in: Some(3600),
    }
}

#[test]
fn concurrent_expired_requests_share_one_refresh_and_preserve_lineage() {
    let (service, credential, _dir) = fixture();
    let calls = AtomicUsize::new(0);
    let calls = &calls;
    runtime().block_on(async {
        let resolve = || {
            service.resolve_credential_access_with(
                credential.clone(),
                None,
                |provider, token| async move {
                    assert_eq!(provider, ManagedAuthProvider::Openai);
                    assert_eq!(token.as_str(), "synthetic-refresh");
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    Ok(grant("synthetic-access"))
                },
            )
        };
        let (first, second, third) = tokio::join!(resolve(), resolve(), resolve());
        for material in [first.unwrap(), second.unwrap(), third.unwrap()] {
            assert_eq!(material.access_token(), "synthetic-access");
            assert_eq!(material.routing_subject(), Some("fixture-subject"));
            assert_eq!(material.credential.generation, credential.generation + 1);
            assert!(same_proxy_lineage(&credential, &material.credential));
            assert!(!format!("{material:?}").contains("synthetic-access"));
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn forced_refresh_compares_rejected_token_and_coalesces_concurrent_401s() {
    let (service, credential, _dir) = fixture();
    let calls = AtomicUsize::new(0);
    let calls = &calls;
    runtime().block_on(async {
        let rejected = service
            .resolve_credential_access_with(credential, None, |_, _| async {
                Ok(grant("old-access"))
            })
            .await
            .unwrap();
        let retry = || {
            service.resolve_credential_access_with(
                rejected.credential.clone(),
                Some(rejected.access_token()),
                |_, token| async move {
                    assert_eq!(token.as_str(), "synthetic-rotated-refresh");
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    Ok(grant("fresh-access"))
                },
            )
        };
        let (first, second) = tokio::join!(retry(), retry());
        for material in [first.unwrap(), second.unwrap()] {
            assert_eq!(material.access_token(), "fresh-access");
            assert_eq!(
                material.credential.generation,
                rejected.credential.generation + 1
            );
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn terminal_refresh_requires_reauth_but_transient_failure_keeps_session() {
    for terminal in [false, true] {
        let (service, credential, _dir) = fixture();
        let result = runtime().block_on(service.resolve_credential_access_with(
            credential.clone(),
            None,
            |_, _| async move {
                Err(if terminal {
                    ProxyRefreshFailure::Terminal
                } else {
                    ProxyRefreshFailure::Retriable
                })
            },
        ));
        if terminal {
            assert!(matches!(result, Err(ManagedAuthCoreError::Conflict)));
        } else {
            assert!(matches!(result, Err(ManagedAuthCoreError::Io)));
        }
        let current = service
            .repository
            .get_credential(&credential.credential_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            current.status,
            if terminal {
                CredentialStatus::RequiresReauth
            } else {
                CredentialStatus::Ready
            }
        );
        assert_eq!(current.generation, credential.generation);
        assert_eq!(
            service
                .readback_bundle(&current.secret_handle)
                .unwrap()
                .refresh_token(),
            Some("synthetic-refresh")
        );
    }
}

#[test]
fn deleted_credential_cannot_be_resurrected_by_inflight_refresh() {
    let (service, credential, _dir) = fixture();
    let result = runtime().block_on(service.resolve_credential_access_with(
        credential.clone(),
        None,
        |_, _| async {
            service.remove_credential_record(&credential).unwrap();
            Ok(grant("must-not-be-returned"))
        },
    ));
    assert!(matches!(result, Err(ManagedAuthCoreError::NotFound)));
    assert!(service
        .repository
        .get_credential(&credential.credential_id)
        .unwrap()
        .is_none());
}

#[test]
fn native_owner_change_during_refresh_rejects_success_and_terminal_failure() {
    for terminal in [false, true] {
        let (service, credential, _dir) = fixture();
        let result = runtime().block_on(service.resolve_credential_access_with(
            credential.clone(),
            None,
            |_, _| async {
                assert!(service
                    .repository
                    .transfer_refresh_owner(
                        &credential.credential_id,
                        credential.generation,
                        RefreshOwner::Fyagent,
                        RefreshOwner::CodexNative,
                        chrono::Utc::now().timestamp(),
                    )
                    .unwrap());
                if terminal {
                    Err(ProxyRefreshFailure::Terminal)
                } else {
                    Ok(grant("must-not-be-returned"))
                }
            },
        ));
        assert!(matches!(result, Err(ManagedAuthCoreError::Stale)));
        let current = service
            .repository
            .get_credential(&credential.credential_id)
            .unwrap()
            .unwrap();
        assert_eq!(current.refresh_owner, RefreshOwner::CodexNative);
        assert_eq!(current.status, CredentialStatus::Ready);
        assert_eq!(current.generation, credential.generation);
    }
}

#[test]
fn new_generation_during_http_is_not_used_as_a_fallback() {
    let (service, credential, _dir) = fixture();
    let result = runtime().block_on(service.resolve_credential_access_with(
        credential.clone(),
        None,
        |_, _| async {
            let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
                credential_id: credential.credential_id.clone(),
                provider: credential.provider,
                generation: credential.generation + 1,
                access_token: Some("other-access".into()),
                refresh_token: Some("other-refresh".into()),
                id_token: None,
                token_type: Some("Bearer".into()),
                granted_scopes: Vec::new(),
                issued_at: None,
                expires_at: Some(chrono::Utc::now().timestamp() + 3600),
            })
            .unwrap();
            // Simulate an authoritative external generation change while the
            // exchange is in flight; normal callers use the same lock.
            assert!(service
                .replace_bundle_cas_locked(
                    &credential.credential_id,
                    credential.generation,
                    RefreshOwner::Fyagent,
                    bundle,
                )
                .unwrap());
            Ok(grant("stale-access"))
        },
    ));
    assert!(matches!(result, Err(ManagedAuthCoreError::Stale)));
    let current = service
        .repository
        .get_credential(&credential.credential_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        service
            .readback_bundle(&current.secret_handle)
            .unwrap()
            .access_token(),
        Some("other-access")
    );
}

#[test]
fn relogin_with_same_account_and_generation_rejects_queued_old_request() {
    let (service, credential, _dir) = fixture();
    service.remove_credential_record(&credential).unwrap();
    let replacement = service.provision_legacy_credential(input()).unwrap();
    assert_eq!(replacement.credential_id, credential.credential_id);
    assert_eq!(replacement.generation, credential.generation);
    assert_ne!(
        replacement.secret_handle.secret_ref(),
        credential.secret_handle.secret_ref()
    );
    let result = runtime().block_on(service.resolve_credential_access_with(
        credential,
        None,
        |_, _| async { panic!("stale lineage must not refresh") },
    ));
    assert!(matches!(result, Err(ManagedAuthCoreError::Stale)));
}

#[test]
fn current_status_revocation_is_checked_before_cached_access_can_escape() {
    let (service, credential, _dir) = fixture();
    runtime().block_on(async {
        let material = service
            .resolve_credential_access_with(credential, None, |_, _| async {
                Ok(grant("cached-access"))
            })
            .await
            .unwrap();
        service
            .repository
            .set_status(
                &material.credential.credential_id,
                CredentialStatus::Revoked,
                chrono::Utc::now().timestamp(),
            )
            .unwrap();
        let result = service
            .resolve_credential_access_with(material.credential, None, |_, _| async {
                panic!("revoked session must not refresh")
            })
            .await;
        assert!(matches!(result, Err(ManagedAuthCoreError::Conflict)));
    });
}
