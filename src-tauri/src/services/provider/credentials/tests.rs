use super::*;
use crate::provider::sanitize_provider_for_export;
use crate::services::secret::{
    MemoryFailureMode, MemorySecretBackend, SecretBackend, SecretService,
};

const CANARY: &str = "fixture-provider-secret-canary-2026";

fn fixture() -> Provider {
    Provider::with_id(
        "fixture-codex".into(),
        "Fixture".into(),
        json!({
            "auth": {"OPENAI_API_KEY": CANARY},
            "config": format!("# keep this comment\nmodel_provider = 'custom'\nmodel = 'fixture-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\nexperimental_bearer_token = '{CANARY}'\n")
        }),
        None,
    )
}

fn database() -> (Database, MemorySecretBackend) {
    let mut db = Database::memory().unwrap();
    let backend = MemorySecretBackend::new();
    db.provider_secrets = SecretService::new(Box::new(backend.clone()));
    (db, backend)
}

const USAGE_KEY: &str = "fixture-usage-api-key-canary";
const USAGE_TOKEN: &str = "fixture-usage-access-token-canary";
const ACCESS_ID: &str = "fixture-control-access-id-canary";
const ACCESS_SECRET: &str = "fixture-control-access-secret-canary";

fn usage_fixture() -> Provider {
    let mut provider = fixture();
    provider.meta = Some(crate::provider::ProviderMeta {
        usage_script: Some(
            serde_json::from_value(json!({
                "enabled": true, "language": "javascript", "code": "({request:{}})",
                "baseUrl": "https://usage.example.invalid/api", "templateType": "token_plan",
                "apiKey": USAGE_KEY, "accessToken": USAGE_TOKEN,
                "accessKeyId": ACCESS_ID, "secretAccessKey": ACCESS_SECRET,
                "codingPlanProvider": "volcengine"
            }))
            .unwrap(),
        ),
        ..Default::default()
    });
    provider
}

fn usage(provider: &Provider) -> &crate::provider::UsageScript {
    provider
        .meta
        .as_ref()
        .unwrap()
        .usage_script
        .as_ref()
        .unwrap()
}

fn usage_mut(provider: &mut Provider) -> &mut crate::provider::UsageScript {
    provider
        .meta
        .as_mut()
        .unwrap()
        .usage_script
        .as_mut()
        .unwrap()
}

fn saved(db: &Database) -> Provider {
    db.get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap()
}

#[test]
fn auxiliary_credentials_share_native_lifecycle_and_never_enter_dto_or_export() {
    let (db, _) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let stored = saved(&db);
    let projected = ProviderCredentials::renderer_projection(&stored);
    assert!(binding(&projected).is_none());
    assert_eq!(usage(&projected).api_key.as_deref(), Some(material::MASK));
    assert_eq!(usage(&projected).base_url, usage(&stored).base_url);
    let native = ProviderCredentials::resolve(&db, "codex", &stored).unwrap();
    assert_eq!(native.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
    assert_eq!(usage(&native).api_key.as_deref(), Some(USAGE_KEY));
    assert_eq!(usage(&native).access_token.as_deref(), Some(USAGE_TOKEN));
    assert_eq!(usage(&native).access_key_id.as_deref(), Some(ACCESS_ID));
    assert_eq!(
        usage(&native).secret_access_key.as_deref(),
        Some(ACCESS_SECRET)
    );
    for output in [
        serde_json::to_string(&stored).unwrap(),
        serde_json::to_string(&projected).unwrap(),
        format!("{native:?}"),
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        for secret in [CANARY, USAGE_KEY, USAGE_TOKEN, ACCESS_ID, ACCESS_SECRET] {
            assert!(!output.contains(secret));
        }
    }
    assert_eq!(db.provider_credential_records().unwrap().len(), 1);
}

#[test]
fn auxiliary_masks_retain_then_explicit_empty_fields_and_script_removal_revoke() {
    let (db, backend) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let original = saved(&db);
    let record = ProviderCredentials::record(&db, &original).unwrap();
    let mut edit = ProviderCredentials::renderer_projection(&original);
    db.save_provider("codex", &edit).unwrap();
    assert_eq!(binding(&saved(&db)), binding(&original));
    usage_mut(&mut edit).api_key = Some("fixture-replaced-usage-key".into());
    db.save_provider("codex", &edit).unwrap();
    let replaced = saved(&db);
    assert_ne!(binding(&replaced), binding(&original));
    let native = ProviderCredentials::resolve(&db, "codex", &replaced).unwrap();
    assert_eq!(
        usage(&native).api_key.as_deref(),
        Some("fixture-replaced-usage-key")
    );
    assert_eq!(usage(&native).access_token.as_deref(), Some(USAGE_TOKEN));
    assert!(
        backend.contains(record.handle.secret_ref()),
        "rollback still owns the old version"
    );

    let mut edit = ProviderCredentials::renderer_projection(&replaced);
    usage_mut(&mut edit).api_key = None;
    usage_mut(&mut edit).access_token = Some(String::new());
    db.save_provider("codex", &edit).unwrap();
    let cleared = saved(&db);
    let native = ProviderCredentials::resolve(&db, "codex", &cleared).unwrap();
    assert!(usage(&native).api_key.is_none());
    assert!(usage(&native).access_token.is_none());
    assert_eq!(usage(&native).access_key_id.as_deref(), Some(ACCESS_ID));

    let mut edit = ProviderCredentials::renderer_projection(&cleared);
    edit.meta.as_mut().unwrap().usage_script = None;
    db.save_provider("codex", &edit).unwrap();
    let cleared = saved(&db);
    let native = ProviderCredentials::resolve(&db, "codex", &cleared).unwrap();
    assert!(native.meta.as_ref().unwrap().usage_script.is_none());
    assert_eq!(native.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(!backend.contains(record.handle.secret_ref()));
}

#[test]
fn auxiliary_masks_cannot_cross_owners_targets_or_scripts() {
    let (db, _) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let original = saved(&db);
    for kind in ["owner", "endpoint", "script", "vendor", "user"] {
        let mut edit = ProviderCredentials::renderer_projection(&original);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!(CANARY);
        match kind {
            "owner" => edit.id = "another-provider".into(),
            "endpoint" => {
                usage_mut(&mut edit).base_url = Some("https://other.example.invalid".into())
            }
            "script" => usage_mut(&mut edit).code.push_str(";({request:{}})"),
            "vendor" => usage_mut(&mut edit).coding_plan_provider = Some("other".into()),
            "user" => usage_mut(&mut edit).user_id = Some("other-user".into()),
            _ => unreachable!(),
        }
        assert!(
            ProviderCredentials::merge_edit(&db, "codex", &edit).is_err(),
            "{kind}"
        );
        assert!(db.save_provider("codex", &edit).is_err(), "{kind}");
        assert_eq!(binding(&saved(&db)), binding(&original));
    }
}

#[test]
fn blank_inference_edits_cannot_redirect_credentials_but_fresh_capture_can() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let original = saved(&db);
    for kind in ["endpoint", "protocol", "auth-role"] {
        let mut edit = ProviderCredentials::renderer_projection(&original);
        let config = edit.settings_config["config"].as_str().unwrap();
        let changed = match kind {
            "endpoint" => config.replace(
                "https://example.invalid/v1",
                "https://other.example.invalid/v1",
            ),
            "protocol" => config.replace("wire_api = 'responses'", "wire_api = 'chat'"),
            "auth-role" => format!("{config}requires_openai_auth = true\n"),
            _ => unreachable!(),
        };
        edit.settings_config["config"] = json!(changed);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!("********");
        assert!(
            ProviderCredentials::merge_edit(&db, "codex", &edit).is_err(),
            "{kind}"
        );
        assert!(db.save_provider("codex", &edit).is_err(), "{kind}");
        assert_eq!(binding(&saved(&db)), binding(&original));
    }
    let mut edit = ProviderCredentials::renderer_projection(&original);
    edit.settings_config["config"] =
        json!(edit.settings_config["config"].as_str().unwrap().replace(
            "https://example.invalid/v1",
            "https://other.example.invalid/v1"
        ));
    edit.settings_config["auth"]["OPENAI_API_KEY"] = json!(CANARY);
    db.save_provider("codex", &edit).unwrap();
    assert_ne!(binding(&saved(&db)), binding(&original));
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &saved(&db))
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn current_row_target_tampering_cannot_reauthorize_retained_material() {
    let (db, _) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let original = saved(&db);
    for kind in ["inference", "usage"] {
        let mut tampered = original.clone();
        if kind == "inference" {
            tampered.settings_config["config"] = json!(tampered.settings_config["config"]
                .as_str()
                .unwrap()
                .replace(
                    "https://example.invalid/v1",
                    "https://other.example.invalid/v1"
                ));
        } else {
            usage_mut(&mut tampered).base_url = Some("https://other.example.invalid/usage".into());
        }
        db.save_provider_record("codex", &tampered).unwrap();
        let edit = ProviderCredentials::renderer_projection(&tampered);
        assert!(ProviderCredentials::resolve(&db, "codex", &tampered).is_err());
        assert!(ProviderCredentials::merge_edit(&db, "codex", &edit).is_err());
        assert!(db.save_provider("codex", &edit).is_err());
        assert_eq!(db.provider_credential_records().unwrap().len(), 1);
        db.save_provider_record("codex", &original).unwrap();
        assert!(ProviderCredentials::resolve(&db, "codex", &original).is_ok());
    }
}

#[test]
#[serial_test::serial]
fn common_config_keeps_legal_settings_but_redirect_requires_fresh_authorization() {
    super::super::tests::with_test_home(|state, _| {
        let mut provider = fixture();
        provider.meta = Some(crate::provider::ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        state
            .db
            .set_config_snippet("codex", Some("[tui]\nnotifications = false\n".into()))
            .unwrap();
        state.db.save_provider("codex", &provider).unwrap();
        let original = saved(&state.db);
        super::super::live::write_live_with_common_config(
            &state.db,
            &crate::app_config::AppType::Codex,
            &original,
        )
        .unwrap();
        let path = crate::codex_config::get_codex_config_path();
        let before = std::fs::read(&path).unwrap();
        assert!(String::from_utf8_lossy(&before).contains("notifications = false"));
        for changed in [
            "[model_providers.custom]\nbase_url = 'https://redirect.example.invalid/v1'\n",
            "[model_providers.custom]\nwire_api = 'chat'\n",
            "[model_providers.custom]\nrequires_openai_auth = true\n",
        ] {
            state
                .db
                .set_config_snippet("codex", Some(changed.into()))
                .unwrap();
            assert!(ProviderCredentials::resolve(&state.db, "codex", &original).is_err());
            let edit = ProviderCredentials::renderer_projection(&original);
            assert!(ProviderCredentials::merge_edit(&state.db, "codex", &edit).is_err());
            assert!(state.db.save_provider("codex", &edit).is_err());
            assert!(super::super::live::write_live_with_common_config(
                &state.db,
                &crate::app_config::AppType::Codex,
                &original
            )
            .is_err());
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
        state
            .db
            .set_config_snippet(
                "codex",
                Some(
                    "[model_providers.custom]\nbase_url = 'https://redirect.example.invalid/v1'\n"
                        .into(),
                ),
            )
            .unwrap();
        state
            .db
            .save_provider("codex", &provider)
            .expect("fresh capture authorizes new effective endpoint");
        let updated = saved(&state.db);
        assert_ne!(binding(&updated), binding(&original));
        super::super::live::write_live_with_common_config(
            &state.db,
            &crate::app_config::AppType::Codex,
            &updated,
        )
        .unwrap();
        let after = std::fs::read_to_string(path).unwrap();
        assert!(after.contains("https://redirect.example.invalid/v1"));
        assert!(after.contains(CANARY));
    });
}

#[test]
fn enabling_common_redirect_cannot_retain_inference_or_implicit_usage_keys() {
    let (db, _) = database();
    let mut provider = usage_fixture();
    provider.meta.as_mut().unwrap().common_config_enabled = Some(false);
    usage_mut(&mut provider).base_url = None;
    db.set_config_snippet(
        "codex",
        Some("[model_providers.custom]\nbase_url = 'https://redirect.example.invalid/v1'\n".into()),
    )
    .unwrap();
    db.save_provider("codex", &provider).unwrap();
    let original = saved(&db);
    let mut edit = ProviderCredentials::renderer_projection(&original);
    edit.meta.as_mut().unwrap().common_config_enabled = Some(true);
    assert!(ProviderCredentials::merge_edit(&db, "codex", &edit).is_err());
    // A fresh inference key alone does not authorize forwarding retained
    // auxiliary credentials to a different implicit usage destination.
    edit.settings_config["auth"]["OPENAI_API_KEY"] = json!(CANARY);
    assert!(db.save_provider("codex", &edit).is_err());
    assert_eq!(binding(&saved(&db)), binding(&original));
    assert!(ProviderCredentials::resolve(&db, "codex", &original).is_ok());
}

#[test]
#[serial_test::serial]
fn native_draft_cannot_redirect_after_merge_or_outlive_its_source_reference() {
    super::super::tests::with_test_home(|state, _| {
        let mut provider = fixture();
        provider.meta = Some(crate::provider::ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        let safe_snippet = "[tui]\nnotifications = false\n";
        state
            .db
            .set_config_snippet("codex", Some(safe_snippet.into()))
            .unwrap();
        state.db.save_provider("codex", &provider).unwrap();
        let original = saved(&state.db);
        let edit = ProviderCredentials::renderer_projection(&original);
        let draft = ProviderCredentials::merge_edit(&state.db, "codex", &edit).unwrap();
        assert!(draft
            .meta
            .as_ref()
            .unwrap()
            .native_credential_draft
            .is_some());
        assert_eq!(draft.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
        super::super::live::write_live_with_common_config(
            &state.db,
            &crate::app_config::AppType::Codex,
            &draft,
        )
        .unwrap();
        let path = crate::codex_config::get_codex_config_path();
        let before = std::fs::read(&path).unwrap();

        state
            .db
            .set_config_snippet(
                "codex",
                Some(
                    "[model_providers.custom]\nbase_url = 'https://redirect.example.invalid/v1'\n"
                        .into(),
                ),
            )
            .unwrap();
        assert!(ProviderCredentials::merge_edit(&state.db, "codex", &draft).is_err());
        assert!(state.db.save_provider("codex", &draft).is_err());
        assert!(super::super::live::write_live_with_common_config(
            &state.db,
            &crate::app_config::AppType::Codex,
            &draft
        )
        .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(binding(&saved(&state.db)), binding(&original));

        state
            .db
            .set_config_snippet("codex", Some(safe_snippet.into()))
            .unwrap();
        let mut replacement = edit.clone();
        replacement.settings_config["auth"]["OPENAI_API_KEY"] =
            json!("fixture-newly-authorized-secret");
        state.db.save_provider("codex", &replacement).unwrap();
        let error = ProviderCredentials::resolve(&state.db, "codex", &draft).unwrap_err();
        assert_eq!(error.to_string(), "provider_secret_revoked");
        assert!(state.db.save_provider("codex", &draft).is_err());
    });
}

#[test]
fn serialized_provider_cannot_forge_or_export_native_draft_authority() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let edit = ProviderCredentials::renderer_projection(&saved(&db));
    let draft = ProviderCredentials::merge_edit(&db, "codex", &edit).unwrap();
    let serialized = serde_json::to_string(&draft).unwrap();
    assert!(!serialized.contains("native_credential_draft"));
    assert!(!serialized.contains("target_fingerprint"));
    let mut forged = serde_json::to_value(&edit).unwrap();
    forged["meta"] = json!({
        "native_credential_draft": {"provider_id":"fixture-codex", "source_ref":binding(&saved(&db)), "target_fingerprint": [0]},
        "nativeCredentialDraft": {"providerId":"fixture-codex"}
    });
    let mut forged: Provider = serde_json::from_value(forged).unwrap();
    assert!(forged
        .meta
        .as_ref()
        .unwrap()
        .native_credential_draft
        .is_none());
    forged.settings_config["config"] =
        json!(forged.settings_config["config"].as_str().unwrap().replace(
            "https://example.invalid/v1",
            "https://redirect.example.invalid/v1"
        ));
    assert!(ProviderCredentials::merge_edit(&db, "codex", &forged).is_err());
}

#[test]
fn unknown_or_corrupt_native_bundle_never_becomes_a_key_or_plaintext_fallback() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let stored = saved(&db);
    let record = ProviderCredentials::record(&db, &stored).unwrap();
    let material = CredentialMaterial::capture(&fixture(), &fixture(), CANARY, None).unwrap();
    let valid = material.encode().unwrap();
    let mut unknown_field = String::from_utf8(valid.to_vec()).unwrap();
    let pos = unknown_field.rfind('}').unwrap();
    unknown_field.insert_str(pos, ",\"unknown\":true");
    for bytes in [
        b"fyagent-provider-credential:2\n{}".to_vec(),
        b"fyagent-provider-credential:1\n{".to_vec(),
        b"fyagent-provider-credential:1\n{\"inference_key\":\"x\"}".to_vec(),
        unknown_field.into_bytes(),
        b"{\"future_bundle\":true}".to_vec(),
    ] {
        backend.delete(record.handle.secret_ref()).unwrap();
        backend
            .create_new(
                record.handle.secret_ref(),
                &SecretMaterial::from_native_input(bytes, PURPOSE).unwrap(),
            )
            .unwrap();
        let error = ProviderCredentials::resolve(&db, "codex", &stored)
            .err()
            .unwrap()
            .to_string();
        assert_eq!(error, "provider_secret_invalid");
        assert!(db.save_provider("codex", &fixture()).is_err());
        assert_eq!(binding(&saved(&db)), binding(&stored));
    }
    backend.delete(record.handle.secret_ref()).unwrap();
    backend
        .create_new(
            record.handle.secret_ref(),
            &SecretMaterial::from_native_input(CANARY.as_bytes().to_vec(), PURPOSE).unwrap(),
        )
        .unwrap();
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &stored)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn auxiliary_migration_failure_and_db_readback_failure_preserve_complete_previous_material() {
    let (db, backend) = database();
    let original = usage_fixture();
    db.save_provider_record("codex", &original).unwrap();
    backend.set_mode(MemoryFailureMode::Locked);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    assert_eq!(
        serde_json::to_value(saved(&db)).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    backend.set_mode(MemoryFailureMode::Healthy);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 1);
    let original = saved(&db);
    db.conn.lock().unwrap().execute_batch("CREATE TRIGGER tamper_usage AFTER UPDATE ON providers WHEN NEW.name = 'Reject' BEGIN UPDATE providers SET meta = '{}' WHERE id = NEW.id; END;").unwrap();
    let mut edit = usage_fixture();
    edit.name = "Reject".into();
    usage_mut(&mut edit).api_key = Some("fixture-edited-usage-secret".into());
    assert_eq!(
        db.save_provider("codex", &edit).unwrap_err().to_string(),
        "provider_secret_persistence_failed"
    );
    let restored = saved(&db);
    assert_eq!(binding(&restored), binding(&original));
    let native = ProviderCredentials::resolve(&db, "codex", &restored).unwrap();
    assert_eq!(usage(&native).api_key.as_deref(), Some(USAGE_KEY));
    assert_eq!(
        usage(&native).secret_access_key.as_deref(),
        Some(ACCESS_SECRET)
    );
}

#[test]
#[serial_test::serial]
fn actual_usage_query_materializes_auxiliary_fields_before_script_validation() {
    super::super::tests::with_test_home(|state, _| {
        let mut provider = usage_fixture();
        let script = usage_mut(&mut provider);
        script.template_type = Some("custom".into());
        // Correct material reaches URL validation; empty/masked/wrong fields
        // fail JS evaluation first. Both paths stop before all network I/O.
        script.code = format!("(function() {{ if ('{{{{apiKey}}}}'.length !== {} || '{{{{accessToken}}}}'.length !== {}) throw new Error('fixture missing native material'); return {{request:{{url:'http://example.invalid',method:'GET'}}}}; }})()", USAGE_KEY.len(), USAGE_TOKEN.len());
        state.db.save_provider("codex", &provider).unwrap();
        let result = futures::executor::block_on(super::super::usage::query_usage(
            state,
            crate::app_config::AppType::Codex,
            &provider.id,
        ))
        .unwrap();
        assert!(!result.success);
        assert!(
            result.error.as_deref().unwrap().contains("HTTPS"),
            "{result:?}"
        );
        assert!(!serde_json::to_string(&result).unwrap().contains(USAGE_KEY));
    });
}

#[test]
fn save_uses_reference_and_native_resolution_without_dto_or_debug_leak() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert!(binding(&stored).unwrap().starts_with("pc_"));
    assert!(!serde_json::to_string(&stored).unwrap().contains(CANARY));
    assert!(!format!("{:?}", fixture()).contains(CANARY));
    let native = ProviderCredentials::resolve(&db, "codex", &stored).unwrap();
    assert_eq!(native.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
    let public = sanitize_provider_for_export(&native);
    let public = serde_json::to_string(&public).unwrap();
    assert!(!public.contains(CANARY));
    assert!(!public.contains("credentialRef"));
    assert!(stored.settings_config["config"]
        .as_str()
        .unwrap()
        .contains("# keep this comment"));
}

#[test]
fn blank_and_masked_edits_retain_the_same_credential() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let original = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    for mask in ["", "********", "••••••", "[REDACTED]"] {
        let mut edit = sanitize_provider_for_export(&original);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!(mask);
        edit.name = "Edited".into();
        let merged = ProviderCredentials::merge_edit(&db, "codex", &edit).unwrap();
        assert_eq!(merged.settings_config["auth"]["OPENAI_API_KEY"], CANARY);
        db.save_provider("codex", &merged).unwrap();
        let saved = db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert_eq!(binding(&saved), binding(&original));
        assert_eq!(saved.name, "Edited");
    }
}

#[test]
fn built_in_image_marker_is_preserved_but_arbitrary_headers_are_rejected() {
    let (db, _) = database();
    let mut provider = fixture();
    let base = provider.settings_config["config"]
        .as_str()
        .unwrap()
        .to_string();
    let name = crate::codex_config::CODEX_IMAGE_EXTENSION_HEADER;
    let marker = crate::codex_config::CODEX_IMAGE_EXTENSION_VALUE;
    provider.settings_config["config"] = json!(format!(
        "{base}http_headers = {{ '{name}' = '{marker}' }}\n"
    ));
    db.save_provider("codex", &provider).unwrap();
    let stored = db
        .get_provider_by_id(&provider.id, "codex")
        .unwrap()
        .unwrap();
    assert!(stored.settings_config["config"]
        .as_str()
        .unwrap()
        .contains(marker));
    assert!(!serde_json::to_string(&stored).unwrap().contains(CANARY));
    for headers in [
        format!("'{name}' = '{CANARY}'"),
        format!("'{name}' = '{marker}', Authorization = '{CANARY}'"),
    ] {
        provider.settings_config["config"] =
            json!(format!("{base}http_headers = {{ {headers} }}\n"));
        assert!(db.save_provider("codex", &provider).is_err());
        assert_eq!(
            db.get_provider_by_id(&provider.id, "codex")
                .unwrap()
                .unwrap()
                .settings_config,
            stored.settings_config
        );
    }
}

#[test]
fn rotation_retains_old_key_for_rollback_then_revokes_after_commit() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let original = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let old = ProviderCredentials::record(&db, &original).unwrap();
    let mut replacement = fixture();
    replacement.settings_config["config"] = json!("model = 'fixture-model'");
    replacement.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-new-secret");
    db.save_provider("codex", &replacement).unwrap();
    let updated = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert_ne!(binding(&original), binding(&updated));
    assert!(backend.contains(old.handle.secret_ref()));
    db.save_provider_record("codex", &original).unwrap();
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &original)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
    db.save_provider_record("codex", &updated).unwrap();
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(!backend.contains(old.handle.secret_ref()));
    assert!(ProviderCredentials::resolve(&db, "codex", &original)
        .unwrap_err()
        .to_string()
        .contains("revoked"));
}

#[test]
fn locked_missing_and_foreign_refs_never_fall_back_to_inline_material() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let mut stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &stored).unwrap();
    stored.settings_config["auth"]["OPENAI_API_KEY"] = json!("do-not-fall-back");
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("locked"));
    backend.set_mode(MemoryFailureMode::Healthy);
    backend.delete(record.handle.secret_ref()).unwrap();
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("missing"));
    stored.id = "another-provider".into();
    assert!(ProviderCredentials::resolve(&db, "codex", &stored).is_err());
    assert!(db.save_provider("codex", &stored).is_err());
}

#[test]
fn migration_failure_preserves_legacy_bytes_then_retries_idempotently() {
    let (db, backend) = database();
    let legacy = fixture();
    db.save_provider_record("codex", &legacy).unwrap();
    backend.set_mode(MemoryFailureMode::Locked);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    let untouched = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert_eq!(untouched.settings_config, legacy.settings_config);
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &untouched)
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
    backend.set_mode(MemoryFailureMode::Healthy);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 1);
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    let migrated = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert!(!migrated.settings_config.to_string().contains(CANARY));
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &migrated)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn failed_database_commit_does_not_destroy_previous_key_or_config() {
    let (db, _) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let previous = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    db.conn.lock().unwrap().execute_batch("CREATE TRIGGER reject_provider_edit BEFORE UPDATE ON providers WHEN NEW.name = 'Reject' BEGIN SELECT RAISE(ABORT, 'fixture-private-error'); END;").unwrap();
    let mut replacement = fixture();
    replacement.name = "Reject".into();
    replacement.settings_config["config"] = json!("model = 'fixture-model'");
    replacement.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-replacement-secret");
    let error = db
        .save_provider("codex", &replacement)
        .unwrap_err()
        .to_string();
    assert!(!error.contains("fixture-private-error"));
    let restored = db
        .get_provider_by_id(&previous.id, "codex")
        .unwrap()
        .unwrap();
    assert_eq!(restored.settings_config, previous.settings_config);
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &restored)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn deletion_revokes_locally_even_when_backend_cleanup_must_retry() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let stored = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &stored).unwrap();
    db.delete_provider("codex", &stored.id).unwrap();
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(ProviderCredentials::cleanup(&db).is_err());
    assert!(ProviderCredentials::resolve(&db, "codex", &stored)
        .unwrap_err()
        .to_string()
        .contains("revoked"));
    backend.set_mode(MemoryFailureMode::Healthy);
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(!backend.contains(record.handle.secret_ref()));
}

#[test]
fn ordinary_and_sync_exports_scrub_legacy_and_current_provider_secrets() {
    let (db, _) = database();
    db.save_provider_record("codex", &fixture()).unwrap();
    for exported in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!exported.contains(CANARY));
        assert!(db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap()
            .settings_config
            .to_string()
            .contains(CANARY));
    }
    ProviderCredentials::migrate_legacy(&db).unwrap();
    let record = db.provider_credential_records().unwrap().remove(0);
    for exported in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!exported.contains(CANARY));
        assert!(!exported.contains(record.handle.secret_ref().as_str()));
        assert!(!exported.contains(&record.id));
    }
}

#[test]
#[serial_test::serial]
fn provider_credentials_native_create_edit_switch_and_delete_round_trip() {
    use crate::app_config::AppType;
    use crate::services::ProviderService;
    super::super::tests::with_test_home(|state, _home| {
        crate::settings::set_current_provider(&AppType::Codex, None).unwrap();
        let auth_path = crate::codex_config::get_codex_auth_path();
        std::fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
        let auth_bytes = b"fixture consumer-owned auth bytes, untouched even if malformed";
        std::fs::write(&auth_path, auth_bytes).unwrap();
        let config_path = crate::codex_config::get_codex_config_path();
        std::fs::write(
            &config_path,
            "# user comment\nmodel = 'previous'\n[features]\nuser_choice = true\n",
        )
        .unwrap();

        let mut first = fixture();
        first.id = super::super::QUICK_SETUP_CODEX_PROVIDER_ID.into();
        let result = ProviderService::apply_quick_setup(state, AppType::Codex, first).unwrap();
        assert!(result.live_config_changed);
        let saved = state
            .db
            .get_provider_by_id(super::super::QUICK_SETUP_CODEX_PROVIDER_ID, "codex")
            .unwrap()
            .unwrap();
        assert!(binding(&saved).is_some());
        assert!(!saved.settings_config.to_string().contains(CANARY));
        let live = std::fs::read_to_string(&config_path).unwrap();
        assert!(live.contains(CANARY));
        assert!(live.contains("user_choice"));
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);

        let mut edit = sanitize_provider_for_export(&saved);
        edit.settings_config["auth"]["OPENAI_API_KEY"] = json!("");
        edit.settings_config["config"] = json!("model_provider = 'custom'\nmodel = 'changed-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n");
        ProviderService::update(state, AppType::Codex, None, edit).unwrap();
        let updated = state
            .db
            .get_provider_by_id(&saved.id, "codex")
            .unwrap()
            .unwrap();
        assert_eq!(binding(&saved), binding(&updated));
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains("changed-model"));
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains(CANARY));

        let mut second = fixture();
        second.id = "fixture-second".into();
        second.settings_config["auth"]["OPENAI_API_KEY"] = json!("fixture-second-secret");
        second.settings_config["config"] = json!("model_provider = 'second'\nmodel = 'second-model'\n[model_providers.second]\nname = 'Second'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n");
        ProviderService::add_draft(state, AppType::Codex, second.clone()).unwrap();
        ProviderService::switch(state, AppType::Codex, &second.id).unwrap();
        assert!(std::fs::read_to_string(&config_path)
            .unwrap()
            .contains("fixture-second-secret"));
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth_bytes);
        ProviderService::delete(state, AppType::Codex, &second.id).unwrap();
        assert!(state
            .db
            .get_provider_by_id(&second.id, "codex")
            .unwrap()
            .is_none());
        assert!(state
            .db
            .provider_credential_records()
            .unwrap()
            .iter()
            .filter(|record| record.provider_id == second.id)
            .all(|record| record.status == "deleted"));
    });
}

#[test]
fn explicit_replacement_repairs_missing_material_without_inline_fallback() {
    let (db, backend) = database();
    db.save_provider("codex", &fixture()).unwrap();
    let old = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    let record = ProviderCredentials::record(&db, &old).unwrap();
    backend.delete(record.handle.secret_ref()).unwrap();
    assert!(ProviderCredentials::resolve(&db, "codex", &old).is_err());
    db.save_provider("codex", &fixture()).unwrap();
    let repaired = db
        .get_provider_by_id("fixture-codex", "codex")
        .unwrap()
        .unwrap();
    assert_ne!(binding(&old), binding(&repaired));
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &repaired)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
}

#[test]
fn interrupted_admitted_create_reuses_verified_material_and_preserves_legacy_until_retry() {
    let (db, backend) = database();
    let legacy = fixture();
    db.save_provider_record("codex", &legacy).unwrap();
    // Simulate interruption after native create but before readback/DB cutover.
    let record = ProviderCredentialRecord {
        id: "pc_interrupted".into(),
        provider_id: legacy.id.clone(),
        handle: db.provider_secrets.reserve(),
        status: "pending".into(),
    };
    db.admit_provider_credential(&record).unwrap();
    backend
        .create_new(
            record.handle.secret_ref(),
            &SecretMaterial::from_native_input(CANARY.as_bytes().to_vec(), PURPOSE).unwrap(),
        )
        .unwrap();
    ProviderCredentials::cleanup(&db).unwrap();
    assert!(backend.contains(record.handle.secret_ref()));
    assert_eq!(
        db.get_provider_by_id(&legacy.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
    db.save_provider("codex", &legacy).unwrap();
    let saved = db.get_provider_by_id(&legacy.id, "codex").unwrap().unwrap();
    assert_eq!(binding(&saved), Some("pc_interrupted"));
    assert_eq!(db.provider_credential_records().unwrap().len(), 1);
}

#[test]
fn conflicting_inline_credentials_fail_without_erasing_legacy() {
    let (db, _) = database();
    let mut legacy = fixture();
    legacy.settings_config["env"] = json!({"OPENAI_API_KEY": "other-fixture-account"});
    db.save_provider_record("codex", &legacy).unwrap();
    assert_eq!(ProviderCredentials::migrate_legacy(&db).unwrap(), 0);
    assert_eq!(
        db.get_provider_by_id(&legacy.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        legacy.settings_config
    );
}

#[test]
fn all_provider_export_shapes_and_snapshot_copies_are_secret_free() {
    let (db, _) = database();
    let shapes = [
        (
            "claude",
            json!({"env":{"ANTHROPIC_AUTH_TOKEN":CANARY,"ANTHROPIC_BASE_URL":"https://example.invalid"},"headers":{"Authorization":CANARY}}),
        ),
        ("gemini", json!({"env":{"GEMINI_API_KEY":CANARY}})),
        (
            "opencode",
            json!({"options":{"apiKey":CANARY,"headers":{"Authorization":CANARY},"baseURL":"https://example.invalid"}}),
        ),
        (
            "openclaw",
            json!({"apiKey":CANARY,"baseUrl":"https://example.invalid","models":[{"id":"fixture-model"}]}),
        ),
        ("hermes", json!({"api_key":CANARY,"model":"fixture-model"})),
        (
            "grokbuild",
            json!({"config":format!("# {CANARY}\napi_key = '{CANARY}'\n[api]\nbase_url = 'https://example.invalid'\n")}),
        ),
        (
            "claude-desktop",
            json!({"unknownExtension":CANARY,"auth":{"accessToken":CANARY}}),
        ),
    ];
    for (app, shape) in shapes {
        let mut provider =
            Provider::with_id(format!("fixture-{app}"), "Fixture".into(), shape, None);
        provider.notes = Some(format!("embedded {CANARY}"));
        db.save_provider(app, &provider).unwrap();
        db.set_current_provider(app, &provider.id).unwrap();
    }
    db.set_setting(
        "universal_providers",
        &json!({"fixture":{"apiKey":CANARY}}).to_string(),
    )
    .unwrap();
    db.set_setting("common_config_codex", &format!("token='{CANARY}'"))
        .unwrap();
    {
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO profiles(id,name,payload) VALUES('profile','Profile',?1)",
            [CANARY],
        )
        .unwrap();
        conn.execute_batch("CREATE TABLE credential_spy(value TEXT); CREATE TRIGGER credential_spy_copy BEFORE UPDATE ON providers BEGIN INSERT INTO credential_spy(value) VALUES(OLD.settings_config); END;").unwrap();
    }
    for export in [
        db.export_sql_string().unwrap(),
        db.export_sql_string_for_sync().unwrap(),
    ] {
        assert!(!export.contains(CANARY));
        assert!(!export.contains("unknownExtension"));
    }
    // Public projections do not mutate the underlying private backup source.
    assert!(db
        .get_provider_by_id("fixture-claude", "claude")
        .unwrap()
        .unwrap()
        .settings_config
        .to_string()
        .contains(CANARY));
}

#[test]
#[serial_test::serial]
fn sync_import_preserves_local_credential_route_without_grafting_to_remote_endpoint() {
    super::super::tests::with_test_home(|state, _home| {
        state.db.save_provider("codex", &fixture()).unwrap();
        let before = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        let remote = Database::memory().unwrap();
        let mut remote_provider = fixture();
        remote_provider.settings_config["config"] =
            json!("model='remote-model'\nbase_url='https://other.invalid'\n");
        remote.save_provider("codex", &remote_provider).unwrap();
        state
            .db
            .import_sql_string_for_sync(&remote.export_sql_string_for_sync().unwrap())
            .unwrap();
        let after = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert_eq!(after.settings_config, before.settings_config);
        assert_eq!(
            ProviderCredentials::resolve(&state.db, "codex", &after)
                .unwrap()
                .settings_config["auth"]["OPENAI_API_KEY"],
            CANARY
        );
    });
}

#[test]
#[serial_test::serial]
fn imported_portable_provider_is_a_credentialless_inactive_draft() {
    super::super::tests::with_test_home(|state, _home| {
        let remote = Database::memory().unwrap();
        remote.save_provider("codex", &fixture()).unwrap();
        remote
            .set_current_provider("codex", "fixture-codex")
            .unwrap();
        let config_path = crate::codex_config::get_codex_config_path();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, "model='user-choice'\n").unwrap();
        let before = std::fs::read(&config_path).unwrap();
        state
            .db
            .import_sql_string(&remote.export_sql_string().unwrap())
            .unwrap();
        let draft = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert!(binding(&draft).is_none());
        assert!(key(&draft).is_none());
        assert!(state.db.get_current_provider("codex").unwrap().is_none());
        assert_eq!(std::fs::read(&config_path).unwrap(), before);
    });
}

#[test]
#[serial_test::serial]
fn missing_target_credential_does_not_change_current_provider_or_live_file() {
    use crate::app_config::AppType;
    use crate::services::ProviderService;
    super::super::tests::with_test_home(|state, _home| {
        let mut first = fixture();
        first.id = super::super::QUICK_SETUP_CODEX_PROVIDER_ID.into();
        ProviderService::apply_quick_setup(state, AppType::Codex, first).unwrap();
        let mut second = fixture();
        second.id = "other-target".into();
        ProviderService::add_draft(state, AppType::Codex, second).unwrap();
        let second = state
            .db
            .get_provider_by_id("other-target", "codex")
            .unwrap()
            .unwrap();
        let record = ProviderCredentials::record(&state.db, &second).unwrap();
        state.db.provider_secrets.delete(&record.handle).unwrap();
        let live_before = std::fs::read(crate::codex_config::get_codex_config_path()).unwrap();
        assert!(ProviderService::switch(state, AppType::Codex, "other-target").is_err());
        assert_eq!(
            state.db.get_current_provider("codex").unwrap().as_deref(),
            Some(super::super::QUICK_SETUP_CODEX_PROVIDER_ID)
        );
        assert_eq!(
            std::fs::read(crate::codex_config::get_codex_config_path()).unwrap(),
            live_before
        );
    });
}

#[test]
fn api_key_persistence_cannot_be_bypassed_by_an_official_category_label() {
    let (db, _) = database();
    let mut provider = fixture();
    provider.category = Some("official".into());
    db.save_provider("codex", &provider).unwrap();
    let stored = db
        .get_provider_by_id(&provider.id, "codex")
        .unwrap()
        .unwrap();
    assert!(binding(&stored).is_some());
    assert!(!stored.settings_config.to_string().contains(CANARY));
}

#[test]
#[serial_test::serial]
fn proxy_router_receives_native_material_while_database_remains_reference_only() {
    super::super::tests::with_test_home(|state, _home| {
        state.db.save_provider("codex", &fixture()).unwrap();
        state
            .db
            .set_current_provider("codex", "fixture-codex")
            .unwrap();
        crate::settings::set_current_provider(
            &crate::app_config::AppType::Codex,
            Some("fixture-codex"),
        )
        .unwrap();
        let router = crate::proxy::provider_router::ProviderRouter::new(state.db.clone());
        let providers = futures::executor::block_on(router.select_providers("codex")).unwrap();
        assert_eq!(
            providers[0].settings_config["auth"]["OPENAI_API_KEY"],
            CANARY
        );
        let stored = state
            .db
            .get_provider_by_id("fixture-codex", "codex")
            .unwrap()
            .unwrap();
        assert!(!stored.settings_config.to_string().contains(CANARY));
        let record = ProviderCredentials::record(&state.db, &stored).unwrap();
        state.db.provider_secrets.delete(&record.handle).unwrap();
        assert!(futures::executor::block_on(router.select_providers("codex")).is_err());
    });
}

#[test]
fn blank_legacy_edit_does_not_erase_the_only_copy_when_migration_is_locked() {
    let (db, backend) = database();
    db.save_provider_record("codex", &fixture()).unwrap();
    let mut edit = fixture();
    edit.settings_config["auth"]["OPENAI_API_KEY"] = json!("");
    edit.settings_config["config"] = json!("model='changed-model'");
    backend.set_mode(MemoryFailureMode::Locked);
    assert!(db.save_provider("codex", &edit).is_err());
    assert_eq!(
        db.get_provider_by_id(&edit.id, "codex")
            .unwrap()
            .unwrap()
            .settings_config,
        fixture().settings_config
    );
    backend.set_mode(MemoryFailureMode::Healthy);
    db.save_provider("codex", &edit).unwrap();
    let saved = db.get_provider_by_id(&edit.id, "codex").unwrap().unwrap();
    assert_eq!(
        ProviderCredentials::resolve(&db, "codex", &saved)
            .unwrap()
            .settings_config["auth"]["OPENAI_API_KEY"],
        CANARY
    );
    let mut new_mask = edit;
    new_mask.id = "new-mask".into();
    new_mask.settings_config["auth"]["OPENAI_API_KEY"] = json!("********");
    assert!(db.save_provider("codex", &new_mask).is_err());
}

#[test]
fn usage_test_admission_rejects_changed_targets_before_using_saved_secrets() {
    let (db, _) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let stored = saved(&db);
    let script = usage(&stored).code.clone();
    let admitted = ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &script,
        Some(""),
        None,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(admitted.api_key, USAGE_KEY);
    assert_eq!(admitted.base_url, "https://usage.example.invalid/api");
    assert!(ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &(script.clone() + ";0"),
        Some(""),
        None,
        None,
        None,
        None,
    )
    .is_err());
    assert!(ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &script,
        Some(""),
        Some("https://changed.example.invalid/v1"),
        None,
        None,
        None,
    )
    .is_err());
    let fresh = ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &(script + ";1"),
        Some("fixture-fresh-usage-key"),
        Some("https://changed.example.invalid/v1"),
        None,
        Some("other-user"),
        Some("custom"),
    )
    .unwrap();
    assert_eq!(fresh.api_key, "fixture-fresh-usage-key");
    assert_eq!(fresh.base_url, "https://changed.example.invalid/v1");
}

fn token_only_fixture() -> Provider {
    let mut provider = fixture();
    provider.settings_config = json!({
        "auth": {},
        "config": "model_provider = 'custom'\nmodel = 'fixture-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n"
    });
    provider.meta = Some(crate::provider::ProviderMeta {
        usage_script: Some(
            serde_json::from_value(json!({
                "enabled": true, "language": "javascript", "code": "({request:{}})",
                "baseUrl": "https://usage.example.invalid/api", "templateType": "newapi",
                "accessToken": USAGE_TOKEN
            }))
            .unwrap(),
        ),
        ..Default::default()
    });
    provider
}

#[test]
fn usage_test_admission_allows_token_only_same_target_and_rejects_changes() {
    let (db, _) = database();
    db.save_provider("codex", &token_only_fixture()).unwrap();
    let stored = saved(&db);
    let script = usage(&stored).code.clone();
    let admitted = ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &script,
        Some(""),
        None,
        Some("********"),
        None,
        None,
    )
    .unwrap();
    assert_eq!(admitted.api_key, "");
    assert_eq!(admitted.access_token.as_deref(), Some(USAGE_TOKEN));
    assert!(ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &script,
        Some(""),
        Some("https://changed.example.invalid/v1"),
        Some("********"),
        None,
        None,
    )
    .is_err());
    let fresh = ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &(script + ";1"),
        Some(""),
        Some("https://changed.example.invalid/v1"),
        Some("fixture-fresh-access-token"),
        None,
        Some("newapi"),
    )
    .unwrap();
    assert_eq!(fresh.api_key, "");
    assert_eq!(
        fresh.access_token.as_deref(),
        Some("fixture-fresh-access-token")
    );
}

#[test]
fn fresh_usage_token_does_not_inherit_a_saved_api_key() {
    let (db, _) = database();
    db.save_provider("codex", &usage_fixture()).unwrap();
    let stored = saved(&db);
    let fresh = ProviderCredentials::admit_usage_test(
        &db,
        "codex",
        &stored,
        &(usage(&stored).code.clone() + ";fresh"),
        Some(""),
        Some("https://changed.example.invalid/v1"),
        Some("fixture-fresh-access-token"),
        None,
        Some("custom"),
    )
    .unwrap();
    assert_eq!(fresh.api_key, "");
    assert_eq!(
        fresh.access_token.as_deref(),
        Some("fixture-fresh-access-token")
    );
}
