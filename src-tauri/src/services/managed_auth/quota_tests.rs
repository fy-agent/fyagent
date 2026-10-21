//! Exact-account quota observation. Injected query callbacks never call an
//! authorization server, CLI auth file, or default-account resolver.

use super::*;
use crate::database::Database;
use crate::services::secret::{MemorySecretBackend, SecretService};
use crate::services::subscription::{
    CredentialStatus as SubscriptionCredentialStatus, QuotaTier, SubscriptionQuota,
};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn service() -> (ManagedAuthService<MemorySecretBackend>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let service = ManagedAuthService::new(
        Arc::new(Database::memory().unwrap()),
        SecretService::new(MemorySecretBackend::new()),
        dir.path().into(),
    );
    (service, dir)
}

fn input(
    legacy: &str,
    subject: &str,
    provider: ManagedAuthProvider,
    purpose: CredentialPurpose,
    consumer: Option<ManagedAuthConsumer>,
    owner: RefreshOwner,
    make_default: bool,
) -> LegacyCredentialInput {
    LegacyCredentialInput {
        migration_id: None,
        provider,
        purpose,
        consumer,
        legacy_account_id: legacy.into(),
        provider_subject: subject.into(),
        provider_tenant: String::new(),
        login: format!("{legacy}@example.test"),
        display_name: None,
        avatar_url: None,
        access_token: None,
        refresh_token: Some(Zeroizing::new(format!("quota-refresh-{legacy}"))),
        id_token: None,
        desired_status: CredentialStatus::Ready,
        refresh_owner: owner,
        authenticated_at: 1_700_000_000,
        make_default,
    }
}

fn install_access(
    service: &ManagedAuthService<MemorySecretBackend>,
    credential: &CredentialRecord,
    legacy: &str,
    expires_at: i64,
) -> CredentialRecord {
    let bundle = ManagedAuthSecretBundle::new(ManagedAuthSecretBundleParts {
        credential_id: credential.credential_id.clone(),
        provider: credential.provider,
        generation: credential.generation + 1,
        access_token: Some(format!("quota-access-{legacy}")),
        refresh_token: Some(format!("quota-refresh-{legacy}")),
        id_token: None,
        token_type: Some("Bearer".into()),
        granted_scopes: vec![],
        issued_at: Some(chrono::Utc::now().timestamp()),
        expires_at: Some(expires_at),
    })
    .unwrap();
    assert!(service
        .replace_bundle_cas(
            &credential.credential_id,
            credential.generation,
            credential.refresh_owner,
            bundle,
        )
        .unwrap());
    service
        .repository
        .get_credential(&credential.credential_id)
        .unwrap()
        .unwrap()
}

fn seed_proxy(
    service: &ManagedAuthService<MemorySecretBackend>,
    legacy: &str,
    subject: &str,
    provider: ManagedAuthProvider,
    make_default: bool,
) -> CredentialRecord {
    let credential = service
        .provision_legacy_credential(input(
            legacy,
            subject,
            provider,
            CredentialPurpose::ProxyUpstream,
            Some(ManagedAuthConsumer::FyagentProxy),
            RefreshOwner::Fyagent,
            make_default,
        ))
        .unwrap();
    install_access(
        service,
        &credential,
        legacy,
        chrono::Utc::now().timestamp() + 3600,
    )
}

fn seed_native(
    service: &ManagedAuthService<MemorySecretBackend>,
    legacy: &str,
    subject: &str,
    provider: ManagedAuthProvider,
    (purpose, consumer, owner): (CredentialPurpose, ManagedAuthConsumer, RefreshOwner),
    expires_at: i64,
) -> CredentialRecord {
    let credential = service
        .provision_legacy_credential(input(
            legacy,
            subject,
            provider,
            purpose,
            Some(consumer),
            owner,
            false,
        ))
        .unwrap();
    install_access(service, &credential, legacy, expires_at)
}

fn success_quota() -> SubscriptionQuota {
    SubscriptionQuota {
        tool: "codex_oauth".into(),
        credential_status: SubscriptionCredentialStatus::Valid,
        credential_message: Some("credential_message sentinel".into()),
        success: true,
        tiers: vec![QuotaTier {
            name: "five_hour".into(),
            utilization: 12.0,
            resets_at: Some("2026-09-09T15:00:00+00:00".into()),
            used_value_usd: None,
            max_value_usd: None,
        }],
        extra_usage: None,
        reset_credits: None,
        credits_balance: None,
        error: Some("raw quota.error access_token=sk-leak".into()),
        queried_at: Some(1),
    }
}

fn leak_text(value: &ManagedAuthAccountQuota) -> String {
    serde_json::to_string(value).unwrap().to_ascii_lowercase()
}

#[test]
fn quota_uses_exact_account_and_ignores_default_sibling() {
    let (service, _dir) = service();
    let default_account = seed_proxy(
        &service,
        "alpha",
        "quota-subject-alpha",
        ManagedAuthProvider::Openai,
        true,
    );
    let selected = seed_proxy(
        &service,
        "beta",
        "quota-subject-beta",
        ManagedAuthProvider::Openai,
        false,
    );
    assert_ne!(default_account.identity_id, selected.identity_id);
    let seen = Mutex::new(Vec::new());
    let quota = runtime()
        .block_on(
            service.get_account_quota_with(&selected.identity_id, |access| {
                seen.lock().unwrap().push((
                    access.provider(),
                    access.access_token().to_string(),
                    access.routing_subject().map(str::to_string),
                ));
                async { Ok(success_quota()) }
            }),
        )
        .expect("selected quota");
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, ManagedAuthProvider::Openai);
    assert_eq!(seen[0].1, "quota-access-beta");
    assert_eq!(seen[0].2.as_deref(), Some("quota-subject-beta"));
    assert_eq!(quota.account_id, selected.identity_id);
    assert_ne!(quota.account_id, default_account.identity_id);
    assert_eq!(quota.provider, ManagedAuthProvider::Openai);
    assert_eq!(quota.status, ManagedAuthQuotaStatus::Available);
    assert_eq!(quota.reason_code, None);
    assert_eq!(quota.windows[0].window_id, "five_hour");
    assert_eq!(quota.windows[0].remaining_percent, 88);
    assert_eq!(
        quota.windows[0].resets_at.as_deref(),
        Some("2026-09-09T15:00:00Z")
    );
}

#[test]
fn quota_admits_ready_native_access_for_exact_account() {
    let (service, _dir) = service();
    let default_proxy = seed_proxy(
        &service,
        "alpha",
        "quota-subject-alpha",
        ManagedAuthProvider::Openai,
        true,
    );
    let native = seed_native(
        &service,
        "native",
        "quota-subject-native",
        ManagedAuthProvider::Openai,
        (
            CredentialPurpose::CodexNative,
            ManagedAuthConsumer::Codex,
            RefreshOwner::CodexNative,
        ),
        chrono::Utc::now().timestamp() + 3600,
    );
    let opencode = seed_native(
        &service,
        "opencode",
        "quota-subject-opencode",
        ManagedAuthProvider::Xai,
        (
            CredentialPurpose::OpencodeProvider,
            ManagedAuthConsumer::Opencode,
            RefreshOwner::Opencode,
        ),
        chrono::Utc::now().timestamp() + 3600,
    );
    let seen = Mutex::new(Vec::new());
    let quota = runtime()
        .block_on(
            service.get_account_quota_with(&native.identity_id, |access| {
                seen.lock().unwrap().push((
                    access.provider(),
                    access.access_token().to_string(),
                    access.routing_subject().map(str::to_string),
                ));
                async { Ok(success_quota()) }
            }),
        )
        .expect("native quota");
    let opencode_quota = runtime()
        .block_on(
            service.get_account_quota_with(&opencode.identity_id, |access| {
                seen.lock().unwrap().push((
                    access.provider(),
                    access.access_token().to_string(),
                    access.routing_subject().map(str::to_string),
                ));
                async { Ok(success_quota()) }
            }),
        )
        .expect("opencode quota");
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].1, "quota-access-native");
    assert_eq!(seen[0].2.as_deref(), Some("quota-subject-native"));
    assert_eq!(seen[1].1, "quota-access-opencode");
    assert_eq!(quota.account_id, native.identity_id);
    assert_ne!(quota.account_id, default_proxy.identity_id);
    assert_eq!(opencode_quota.account_id, opencode.identity_id);

    let rejected = runtime()
        .block_on(
            service.get_account_quota_with(&native.identity_id, |_| async {
                Ok(SubscriptionQuota::error(
                    "codex_oauth",
                    SubscriptionCredentialStatus::Expired,
                    "upstream rejected the token".into(),
                ))
            }),
        )
        .unwrap();
    assert_eq!(
        rejected.status,
        ManagedAuthQuotaStatus::NativeRefreshRequired
    );
    let current = service
        .repository
        .get_credential(&native.credential_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.refresh_owner, RefreshOwner::CodexNative);
    assert_eq!(current.generation, native.generation);
    assert_eq!(current.status, CredentialStatus::Ready);
    assert_eq!(quota.status, ManagedAuthQuotaStatus::Available);
}

#[test]
fn quota_expired_native_access_does_not_refresh() {
    let (service, _dir) = service();
    let native = seed_native(
        &service,
        "stale-native",
        "quota-subject-stale",
        ManagedAuthProvider::Openai,
        (
            CredentialPurpose::CodexNative,
            ManagedAuthConsumer::Codex,
            RefreshOwner::CodexNative,
        ),
        chrono::Utc::now().timestamp() - 120,
    );
    let queried = Arc::new(Mutex::new(false));
    let quota = runtime()
        .block_on(service.get_account_quota_with(&native.identity_id, |_| {
            *queried.lock().unwrap() = true;
            async { Ok(success_quota()) }
        }))
        .expect("expired native");
    assert!(!*queried.lock().unwrap());
    assert_eq!(quota.account_id, native.identity_id);
    assert_eq!(quota.status, ManagedAuthQuotaStatus::NativeRefreshRequired);
    assert_eq!(quota.reason_code, None);
    assert!(quota.windows.is_empty());
    let current = service
        .repository
        .get_credential(&native.credential_id)
        .unwrap()
        .unwrap();
    assert_eq!(current.status, CredentialStatus::Ready);
    assert_eq!(current.refresh_owner, RefreshOwner::CodexNative);
    let text = leak_text(&quota);
    assert!(!text.contains("quota-access-stale-native"));
    assert!(!text.contains("quota-refresh-stale-native"));
}

#[test]
fn quota_rejects_unsupported_provider() {
    let (service, _dir) = service();
    let copilot = service
        .provision_legacy_credential(input(
            "copilot",
            "quota-subject-copilot",
            ManagedAuthProvider::GithubCopilot,
            CredentialPurpose::Copilot,
            Some(ManagedAuthConsumer::FyagentProxy),
            RefreshOwner::Fyagent,
            true,
        ))
        .unwrap();
    let queried = Arc::new(Mutex::new(false));
    let copilot_error = runtime()
        .block_on(service.get_account_quota_with(&copilot.identity_id, |_| {
            *queried.lock().unwrap() = true;
            async { Ok(success_quota()) }
        }))
        .expect_err("copilot");
    assert_eq!(
        copilot_error.reason_code,
        ManagedAuthReasonCode::ProviderNotSupported
    );
    assert!(!*queried.lock().unwrap());
}

#[test]
fn quota_dto_omits_raw_secret_and_upstream_error() {
    let (service, _dir) = service();
    let selected = seed_proxy(
        &service,
        "gamma",
        "quota-subject-gamma",
        ManagedAuthProvider::Xai,
        true,
    );
    let quota = runtime()
        .block_on(
            service.get_account_quota_with(&selected.identity_id, |access| {
                assert_eq!(access.provider(), ManagedAuthProvider::Xai);
                assert_eq!(access.access_token(), "quota-access-gamma");
                async {
                    Ok(SubscriptionQuota::error(
                        "xai_oauth",
                        SubscriptionCredentialStatus::Valid,
                        "raw quota.error refresh_token=rt-secret secret_ref=vault".into(),
                    ))
                }
            }),
        )
        .expect("mapped failure");
    assert_eq!(quota.account_id, selected.identity_id);
    assert_eq!(quota.status, ManagedAuthQuotaStatus::Unavailable);
    assert_eq!(
        quota.reason_code,
        Some(ManagedAuthReasonCode::ObserverUnavailable)
    );
    assert!(quota.windows.is_empty());
    let text = leak_text(&quota);
    for forbidden in [
        "access_token",
        "refresh_token",
        "secret_ref",
        "secretref",
        "quota-access-gamma",
        "quota-refresh-gamma",
        "rt-secret",
        "credential_message",
        "raw quota.error",
        "xai_oauth",
    ] {
        assert!(!text.contains(forbidden), "{forbidden} leaked: {text}");
    }
}

#[test]
fn quota_dto_binds_requested_account_id() {
    let (service, _dir) = service();
    let first = seed_proxy(
        &service,
        "one",
        "quota-subject-one",
        ManagedAuthProvider::Openai,
        true,
    );
    let second = seed_proxy(
        &service,
        "two",
        "quota-subject-two",
        ManagedAuthProvider::Openai,
        false,
    );
    let quota = runtime()
        .block_on(
            service.get_account_quota_with(&second.identity_id, |access| {
                assert_eq!(access.access_token(), "quota-access-two");
                assert_eq!(access.routing_subject(), Some("quota-subject-two"));
                async { Ok(success_quota()) }
            }),
        )
        .expect("bound");
    assert_eq!(quota.account_id, second.identity_id);
    assert_ne!(quota.account_id, first.identity_id);
    let value = serde_json::to_value(&quota).unwrap();
    let keys: std::collections::BTreeSet<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "accountId",
            "checkedAt",
            "contractVersion",
            "provider",
            "reasonCode",
            "status",
            "windows",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
}

#[test]
fn openai_models_use_exact_account_credential() {
    use crate::services::codex_oauth_models::ListedModelsError;

    let (service, _dir) = service();
    let default_account = seed_proxy(
        &service,
        "models-default",
        "models-subject-default",
        ManagedAuthProvider::Openai,
        true,
    );
    let selected = seed_proxy(
        &service,
        "models-selected",
        "models-subject-selected",
        ManagedAuthProvider::Openai,
        false,
    );
    let seen = Mutex::new(Vec::new());
    let result = runtime()
        .block_on(
            service.fetch_openai_models_with(&selected.identity_id, |token, subject| {
                seen.lock().unwrap().push((token.to_string(), subject));
                async { Ok(vec!["gpt-6-sol".to_string(), "gpt-5.5".to_string()]) }
            }),
        )
        .expect("models");
    assert_eq!(
        seen.into_inner().unwrap(),
        vec![(
            "quota-access-models-selected".to_string(),
            "models-subject-selected".to_string()
        )]
    );
    assert_eq!(result.account_id, selected.identity_id);
    assert_ne!(result.account_id, default_account.identity_id);
    assert_eq!(result.models, vec!["gpt-6-sol", "gpt-5.5"]);
    let value = serde_json::to_value(&result).unwrap();
    let keys: std::collections::BTreeSet<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        ["accountId", "contractVersion", "fetchedAt", "models"]
            .into_iter()
            .map(str::to_string)
            .collect()
    );
    let text = value.to_string();
    assert!(!text.contains("quota-access"));
    assert!(!text.contains("quota-refresh"));

    for (error, reason) in [
        (
            ListedModelsError::Unauthorized,
            ManagedAuthReasonCode::RequiresReauth,
        ),
        (
            ListedModelsError::Unavailable,
            ManagedAuthReasonCode::ObserverUnavailable,
        ),
        (
            ListedModelsError::InvalidResponse,
            ManagedAuthReasonCode::InvalidResponse,
        ),
    ] {
        let failure = runtime()
            .block_on(
                service.fetch_openai_models_with(&selected.identity_id, |_, _| async move {
                    Err(error)
                }),
            )
            .expect_err("mapped failure");
        assert_eq!(failure.reason_code, reason);
    }
}

#[test]
fn openai_models_refuse_other_providers_and_unknown_accounts() {
    let (service, _dir) = service();
    let xai = seed_proxy(
        &service,
        "models-xai",
        "models-subject-xai",
        ManagedAuthProvider::Xai,
        true,
    );
    let native_only = seed_native(
        &service,
        "models-native",
        "models-subject-native",
        ManagedAuthProvider::Openai,
        (
            CredentialPurpose::CodexNative,
            ManagedAuthConsumer::Codex,
            RefreshOwner::CodexNative,
        ),
        chrono::Utc::now().timestamp() + 3600,
    );
    let queried = Arc::new(Mutex::new(false));
    for account in [
        xai.identity_id.as_str(),
        native_only.identity_id.as_str(),
        "ma1:ffffffffffffffffffffffffffffffff",
    ] {
        let error = runtime()
            .block_on(service.fetch_openai_models_with(account, |_, _| {
                *queried.lock().unwrap() = true;
                async { Ok(vec!["gpt-5.5".to_string()]) }
            }))
            .expect_err("refused");
        assert_eq!(
            error.reason_code,
            ManagedAuthReasonCode::ConnectionUnavailable,
            "{account}"
        );
    }
    assert!(!*queried.lock().unwrap());
}

fn write_codex_login(service: &ManagedAuthService<MemorySecretBackend>, subject: &str) {
    use crate::services::managed_auth::consumers::codex::CodexChatGptAuthDocument;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
    let payload = URL_SAFE_NO_PAD.encode(format!(
        r#"{{"chatgpt_account_id":"{subject}","email":"fixture@example.test"}}"#
    ));
    let token = format!("{header}.{payload}.sig");
    let auth = CodexChatGptAuthDocument::from_tokens(
        &token,
        &token,
        "native-refresh-secret",
        Some(subject),
        Some(1_700_000_000),
    )
    .unwrap()
    .serialize_bytes()
    .unwrap();
    let home = service.codex_home();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("auth.json"), auth).unwrap();
}

fn native_catalog() -> fyagent_user_helper::CodexModelCatalog {
    fyagent_user_helper::CodexModelCatalog {
        cli_version: Some("0.160.1".into()),
        models: vec![
            fyagent_user_helper::CodexModelEntry {
                id: "gpt-6-sol".into(),
                display_name: Some("GPT-6 Sol".into()),
                is_default: true,
                reasoning_efforts: vec!["low".into(), "high".into()],
            },
            fyagent_user_helper::CodexModelEntry {
                id: "gpt-5.5".into(),
                display_name: None,
                is_default: false,
                reasoning_efforts: vec![],
            },
        ],
        truncated: false,
    }
}

#[test]
fn codex_native_models_are_read_only_for_the_live_matching_login() {
    use super::super::codex_native_models::{CodexNativeAccountBinding, CodexNativeCatalogStatus};

    let (service, _dir) = service();
    let selected = seed_proxy(
        &service,
        "native-selected",
        "acct-selected",
        ManagedAuthProvider::Openai,
        false,
    );
    write_codex_login(&service, "acct-selected");
    let result = runtime()
        .block_on(
            service.get_codex_native_models_with(&selected.identity_id, true, || async {
                Ok(native_catalog())
            }),
        )
        .expect("catalog");
    assert_eq!(result.account_id, selected.identity_id);
    assert_eq!(result.account_binding, CodexNativeAccountBinding::Matched);
    assert_eq!(
        result.native_account_id.as_deref(),
        Some(selected.identity_id.as_str())
    );
    assert_eq!(result.catalog_status, CodexNativeCatalogStatus::Ready);
    assert_eq!(result.cli_version.as_deref(), Some("0.160.1"));
    assert_eq!(result.source, "codex_app_server");
    assert_eq!(
        result.revision,
        crate::services::managed_auth::consumers::codex::observe_codex_home(&service.codex_home())
            .connection_revision()
    );
    assert_eq!(
        result
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        ["gpt-6-sol", "gpt-5.5"]
    );
    let value = serde_json::to_value(&result).unwrap();
    let keys: std::collections::BTreeSet<_> = value.as_object().unwrap().keys().cloned().collect();
    assert_eq!(
        keys,
        [
            "accountBinding",
            "accountId",
            "bindingDetail",
            "catalogStatus",
            "checkedAt",
            "cliVersion",
            "contractVersion",
            "failure",
            "models",
            "nativeAccountId",
            "revision",
            "source",
            "truncated",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
    assert_eq!(
        value["models"][0],
        serde_json::json!({
            "id": "gpt-6-sol",
            "displayName": "GPT-6 Sol",
            "isDefault": true,
            "reasoningEfforts": ["low", "high"],
        })
    );
    let text = value.to_string();
    assert!(!text.contains("native-refresh-secret"));
    assert!(!text.contains("acct-selected"));
}

#[test]
fn codex_native_models_never_read_for_another_or_undecided_login() {
    use super::super::codex_native_models::{
        CodexNativeAccountBinding, CodexNativeBindingDetail, CodexNativeCatalogStatus,
    };

    let (service, _dir) = service();
    let selected = seed_proxy(
        &service,
        "native-selected",
        "acct-selected",
        ManagedAuthProvider::Openai,
        false,
    );
    let other = seed_proxy(
        &service,
        "native-other",
        "acct-other",
        ManagedAuthProvider::Openai,
        true,
    );
    let read = Arc::new(Mutex::new(0));
    let run = |cli_reads_managed_home: bool| {
        let read = read.clone();
        runtime()
            .block_on(service.get_codex_native_models_with(
                &selected.identity_id,
                cli_reads_managed_home,
                move || {
                    *read.lock().unwrap() += 1;
                    async { Ok(native_catalog()) }
                },
            ))
            .expect("attribution")
    };

    let signed_out = run(true);
    assert_eq!(
        signed_out.account_binding,
        CodexNativeAccountBinding::Unknown
    );
    assert_eq!(
        signed_out.binding_detail,
        Some(CodexNativeBindingDetail::SignedOut)
    );

    write_codex_login(&service, "acct-other");
    let mismatch = run(true);
    assert_eq!(
        mismatch.account_binding,
        CodexNativeAccountBinding::Mismatch
    );
    assert_eq!(
        mismatch.native_account_id.as_deref(),
        Some(other.identity_id.as_str())
    );

    write_codex_login(&service, "acct-selected");
    let custom_home = run(false);
    assert_eq!(
        custom_home.account_binding,
        CodexNativeAccountBinding::Unknown
    );
    assert_eq!(
        custom_home.binding_detail,
        Some(CodexNativeBindingDetail::CustomCodexHome)
    );

    for result in [signed_out, mismatch, custom_home] {
        assert_eq!(result.catalog_status, CodexNativeCatalogStatus::NotRead);
        assert!(result.models.is_empty());
        assert_eq!(result.cli_version, None);
    }
    assert_eq!(*read.lock().unwrap(), 0);
}

#[test]
fn codex_native_models_drop_a_catalog_read_across_a_login_change() {
    use super::super::codex_native_models::{
        CodexNativeAccountBinding, CodexNativeCatalogFailure, CodexNativeCatalogStatus,
    };

    let (service, _dir) = service();
    let selected = seed_proxy(
        &service,
        "native-selected",
        "acct-selected",
        ManagedAuthProvider::Openai,
        false,
    );
    write_codex_login(&service, "acct-selected");
    let changed = runtime()
        .block_on(
            service.get_codex_native_models_with(&selected.identity_id, true, || {
                write_codex_login(&service, "acct-switched");
                async { Ok(native_catalog()) }
            }),
        )
        .expect("result");
    assert_eq!(changed.account_binding, CodexNativeAccountBinding::Matched);
    assert_eq!(changed.catalog_status, CodexNativeCatalogStatus::Failed);
    assert_eq!(
        changed.failure,
        Some(CodexNativeCatalogFailure::ChangedDuringRead)
    );
    assert!(changed.models.is_empty());

    write_codex_login(&service, "acct-selected");
    let failed = runtime()
        .block_on(
            service.get_codex_native_models_with(&selected.identity_id, true, || async {
                Err(CodexNativeCatalogFailure::TimedOut)
            }),
        )
        .expect("result");
    assert_eq!(failed.catalog_status, CodexNativeCatalogStatus::Failed);
    assert_eq!(failed.failure, Some(CodexNativeCatalogFailure::TimedOut));
}

#[test]
fn codex_native_models_refuse_other_providers_and_unknown_accounts() {
    let (service, _dir) = service();
    let xai = seed_proxy(
        &service,
        "native-xai",
        "acct-xai",
        ManagedAuthProvider::Xai,
        true,
    );
    write_codex_login(&service, "acct-xai");
    for account in [
        xai.identity_id.as_str(),
        "ma1:ffffffffffffffffffffffffffffffff",
    ] {
        let error = runtime()
            .block_on(
                service
                    .get_codex_native_models_with(account, true, || async { Ok(native_catalog()) }),
            )
            .expect_err("refused");
        assert_eq!(
            error.reason_code,
            ManagedAuthReasonCode::ConnectionUnavailable,
            "{account}"
        );
    }
}

#[test]
fn quota_nonfinite_windows_are_not_success() {
    let mut quota = success_quota();
    quota.tiers[0].utilization = f64::NAN;
    let mapped = map_subscription_quota(
        "ma1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ManagedAuthProvider::Openai,
        Ok(quota),
    );
    assert_eq!(mapped.status, ManagedAuthQuotaStatus::Unavailable);
    assert_eq!(
        mapped.reason_code,
        Some(ManagedAuthReasonCode::ObserverUnavailable)
    );
    assert!(mapped.windows.is_empty());
}
