use super::*;
use serde_json::json;
use serial_test::serial;

struct TestHomeGuard(
    Option<std::ffi::OsString>,
    Option<crate::settings::AppSettings>,
);
impl TestHomeGuard {
    fn set(home: &std::path::Path) -> Self {
        #[cfg(target_os = "windows")]
        crate::initialize_windows_user_context().expect("Windows test user context");
        let mut guard = Self(std::env::var_os("FYAGENT_TEST_HOME"), None);
        std::env::set_var("FYAGENT_TEST_HOME", home);
        guard.1 = Some(crate::settings::get_settings());
        crate::settings::replace_settings_in_memory_for_test(Default::default());
        assert_eq!(crate::config::get_home_dir(), home);
        guard
    }
}
impl Drop for TestHomeGuard {
    fn drop(&mut self) {
        if let Some(settings) = self.1.take() {
            crate::settings::replace_settings_in_memory_for_test(settings);
        }
        match self.0.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
    }
}

fn write_config(home: &std::path::Path, value: &Value) {
    let dir = home.join(".config").join("opencode");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("opencode.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

fn read_config(home: &std::path::Path) -> Value {
    serde_json::from_slice(
        &std::fs::read(home.join(".config").join("opencode").join("opencode.json")).unwrap(),
    )
    .unwrap()
}

fn seeded_config() -> Value {
    json!({
        "theme": "keep-me",
        "provider": {
            "gateway": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "Gateway",
                "extra": { "keep": true },
                "options": {
                    "baseURL": "https://old.example.test/v1",
                    "apiKey": "OPENCODE-SECRET"
                },
                "models": {
                    "existing-model": { "name": "Existing", "limit": { "context": 8 } }
                }
            }
        }
    })
}

fn save_request(
    revision: Option<String>,
    selected: &[&str],
    removed: &[&str],
) -> SaveOpenCodeModelsRequest {
    SaveOpenCodeModelsRequest {
        provider_id: Some("gateway".into()),
        provider_name: "Gateway".into(),
        base_url: "https://gateway.example.test/v1".into(),
        api_key: "USER-OPENCODE-KEY".into(),
        selected_model_ids: selected.iter().map(|id| (*id).to_string()).collect(),
        removed_model_ids: removed.iter().map(|id| (*id).to_string()).collect(),
        expected_revision: revision,
        overwrite_token: None,
    }
}

#[test]
#[serial]
fn config_reliability_builtin_snapshot_is_read_only_and_save_never_injects_npm() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let config = json!({"provider":{"builtin":{"options":{"apiKey":"FIXTURE-SECRET"},"models":{"builtin-model":{}},"extra":{"keep":true}}}});
    write_config(temp.path(), &config);
    let path = temp.path().join(".config/opencode/opencode.json");
    let before = std::fs::read(&path).unwrap();
    let snapshot = get_opencode_model_snapshot().unwrap();
    assert!(!snapshot.providers[0].editable);
    assert_eq!(snapshot.providers[0].model_ids, vec!["builtin-model"]);
    assert!(!serde_json::to_string(&snapshot)
        .unwrap()
        .contains("FIXTURE-SECRET"));
    let mut request = save_request(snapshot.revision, &["new-model"], &[]);
    request.provider_id = Some("builtin".into());
    let error = save_opencode_models(request).unwrap_err();
    assert_eq!(error.code, OpenCodeModelsErrorCode::ConfigUnavailable);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!path.with_extension("json.backup").exists());
}

#[test]
#[serial]
fn config_reliability_exact_target_and_explicit_create_preserve_builtin() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let builtin = json!({"models":{"builtin-model":{}},"vendor":{"keep":true}});
    write_config(temp.path(), &json!({"provider":{"builtin":builtin}}));
    let mut create = save_request(
        get_opencode_model_snapshot().unwrap().revision,
        &["custom-model"],
        &[],
    );
    create.provider_id = None;
    assert!(matches!(
        save_opencode_models(create).unwrap(),
        SaveOpenCodeModelsOutcome::Saved { .. }
    ));
    let after_create = read_config(temp.path());
    assert_eq!(after_create["provider"]["builtin"], builtin);
    assert_eq!(after_create["provider"]["gateway"]["npm"], DEFAULT_NPM);

    let mut targeted = save_request(
        get_opencode_model_snapshot().unwrap().revision,
        &["second-model"],
        &[],
    );
    targeted.provider_name = "Renamed Display Name".into();
    assert!(matches!(
        save_opencode_models(targeted).unwrap(),
        SaveOpenCodeModelsOutcome::Saved { .. }
    ));
    let after_edit = read_config(temp.path());
    assert_eq!(after_edit["provider"].as_object().unwrap().len(), 2);
    assert!(after_edit["provider"]["gateway"]["models"]
        .get("second-model")
        .is_some());
    assert_eq!(after_edit["provider"]["builtin"], builtin);

    for target in [None, Some("missing".into())] {
        let mut rejected = save_request(
            get_opencode_model_snapshot().unwrap().revision,
            &["third-model"],
            &[],
        );
        rejected.provider_id = target;
        rejected.provider_name = "Builtin".into();
        assert!(save_opencode_models(rejected).is_err());
        assert_eq!(read_config(temp.path()), after_edit);
    }
}

#[test]
#[serial]
fn config_reliability_overwrite_token_is_bound_to_exact_provider() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let mut config = seeded_config();
    config["provider"]["other"] = config["provider"]["gateway"].clone();
    write_config(temp.path(), &config);
    let mut request = save_request(
        get_opencode_model_snapshot().unwrap().revision,
        &["existing-model"],
        &[],
    );
    let SaveOpenCodeModelsOutcome::OverwriteConfirmationRequired { token, .. } =
        save_opencode_models(request.clone()).unwrap()
    else {
        panic!("expected confirmation")
    };
    request.provider_id = Some("other".into());
    request.overwrite_token = Some(token);
    assert_eq!(
        save_opencode_models(request).unwrap_err().code,
        OpenCodeModelsErrorCode::OverwriteTokenInvalid
    );
    assert_eq!(read_config(temp.path()), config);
}

#[test]
#[serial]
fn snapshot_is_secret_free_and_preserves_ids() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let snapshot = get_opencode_model_snapshot().unwrap();
    assert_eq!(snapshot.providers.len(), 1);
    assert_eq!(snapshot.providers[0].id, "gateway");
    assert_eq!(snapshot.providers[0].name, "Gateway");
    assert_eq!(snapshot.providers[0].model_ids, vec!["existing-model"]);
    assert!(snapshot.revision.is_some());
    let debug = format!("{snapshot:?}");
    let json = serde_json::to_string(&snapshot).unwrap();
    for secret in ["OPENCODE-SECRET", "USER-OPENCODE-KEY"] {
        assert!(!debug.contains(secret));
        assert!(!json.contains(secret));
    }
    assert!(!json.contains("apiKey"));
}

#[test]
#[serial]
fn save_adds_model_keeps_unknown_fields_and_never_returns_secrets() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let original_bytes = std::fs::read(
        temp.path()
            .join(".config")
            .join("opencode")
            .join("opencode.json"),
    )
    .unwrap();
    let revision = get_opencode_model_snapshot().unwrap().revision;
    let outcome = save_opencode_models(save_request(revision, &["new-model"], &[])).unwrap();
    match outcome {
        SaveOpenCodeModelsOutcome::Saved {
            created_entries,
            updated_entries,
            model_count,
            ..
        } => {
            assert_eq!(created_entries, 1);
            assert_eq!(updated_entries, 0);
            assert_eq!(model_count, 2);
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
    let config = read_config(temp.path());
    assert_eq!(config["theme"], "keep-me");
    assert_eq!(config["provider"]["gateway"]["extra"]["keep"], true);
    assert_eq!(
        config["provider"]["gateway"]["models"]["existing-model"]["limit"]["context"],
        8
    );
    assert_eq!(
        config["provider"]["gateway"]["models"]["new-model"]["name"],
        "new-model"
    );
    assert_eq!(
        config["provider"]["gateway"]["options"]["apiKey"],
        "USER-OPENCODE-KEY"
    );
    let snapshot = get_opencode_model_snapshot().unwrap();
    let debug = format!("{snapshot:?}");
    assert!(!debug.contains("USER-OPENCODE-KEY"));
    assert!(!debug.contains("OPENCODE-SECRET"));
    assert_eq!(
        std::fs::read(
            temp.path()
                .join(".config")
                .join("opencode")
                .join("opencode.json.backup"),
        )
        .unwrap(),
        original_bytes
    );
}

#[test]
#[serial]
fn first_save_creates_no_fake_backup_without_a_preimage() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let mut request = save_request(None, &["new-model"], &[]);
    request.provider_id = None;
    let outcome = save_opencode_models(request).unwrap();
    assert!(matches!(outcome, SaveOpenCodeModelsOutcome::Saved { .. }));
    let dir = temp.path().join(".config").join("opencode");
    assert!(dir.join("opencode.json").exists());
    assert!(!dir.join("opencode.json.backup").exists());
}

#[test]
#[serial]
fn updating_existing_model_requires_overwrite_token() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let revision = get_opencode_model_snapshot().unwrap().revision;
    let first =
        save_opencode_models(save_request(revision.clone(), &["existing-model"], &[])).unwrap();
    let SaveOpenCodeModelsOutcome::OverwriteConfirmationRequired {
        token,
        existing_ids,
    } = first
    else {
        panic!("expected overwrite: {first:?}");
    };
    assert_eq!(existing_ids, vec!["existing-model"]);
    let mut confirmed = save_request(revision, &["existing-model"], &[]);
    confirmed.overwrite_token = Some(token);
    match save_opencode_models(confirmed).unwrap() {
        SaveOpenCodeModelsOutcome::Saved {
            updated_entries, ..
        } => assert_eq!(updated_entries, 1),
        other => panic!("unexpected outcome: {other:?}"),
    }
}

#[test]
#[serial]
fn stale_revision_is_concurrent_modification() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let outcome =
        save_opencode_models(save_request(Some("stale".into()), &["new-model"], &[])).unwrap();
    assert_eq!(outcome, SaveOpenCodeModelsOutcome::ConcurrentModification);
}

fn fetch_request(revision: Option<String>) -> FetchOpenCodeModelsRequest {
    FetchOpenCodeModelsRequest {
        provider_id: Some("gateway".into()),
        expected_revision: revision,
        base_url: String::new(),
        api_key: String::new(),
        allow_no_api_key: false,
    }
}

async fn model_server(body: String) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut buffer = [0u8; 1024];
            let size = stream.read(&mut buffer).await.unwrap();
            assert!(size > 0 && request.len() < 16 * 1024);
            request.extend_from_slice(&buffer[..size]);
            if request.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(request).unwrap()
    });
    (url, server)
}

fn empty_draft_save(revision: Option<String>) -> SaveOpenCodeModelsRequest {
    let mut request = save_request(revision, &["grok-4.7"], &[]);
    request.base_url.clear();
    request.api_key.clear();
    request
}

#[tokio::test]
#[serial]
async fn refresh_saved_connection_fetches_latest_and_saves_only_selection() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let (url, server) = model_server(
        json!({"data": [
            {"id": "grok-4.7", "owned_by": "xai"},
            {"id": "other-latest", "owned_by": "vendor"}
        ]})
        .to_string(),
    )
    .await;
    let mut config = seeded_config();
    config["provider"]["gateway"]["options"]["baseURL"] = json!(url);
    config["provider"]["neighbor"] = json!({"models": {"builtin-model": {"keep": 9}}});
    config["model"] = json!("gateway/existing-model");
    write_config(temp.path(), &config);
    let path = temp.path().join(".config/opencode/opencode.json");
    let preimage = std::fs::read(&path).unwrap();
    let snapshot = get_opencode_model_snapshot().unwrap();
    let fetched = fetch_opencode_provider_models(fetch_request(snapshot.revision.clone()))
        .await
        .unwrap();
    assert_eq!(
        fetched
            .models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        vec!["grok-4.7", "other-latest"]
    );
    let received = server.await.unwrap();
    assert!(received.starts_with("GET /v1/models HTTP/1.1\r\n"));
    assert!(received
        .to_ascii_lowercase()
        .contains("authorization: bearer opencode-secret\r\n"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        preimage,
        "fetch is read-only"
    );
    let outcome = save_opencode_models(empty_draft_save(snapshot.revision)).unwrap();
    let SaveOpenCodeModelsOutcome::Saved {
        revision,
        model_count,
        created_entries,
        ..
    } = &outcome
    else {
        panic!("expected saved outcome: {outcome:?}");
    };
    assert_eq!((*model_count, *created_entries), (2, 1));
    let after = read_config(temp.path());
    let mut expected = config;
    expected["provider"]["gateway"]["models"]["grok-4.7"] = json!({"name": "grok-4.7"});
    assert_eq!(after, expected, "only the selected model is added");
    assert_eq!(
        std::fs::read(path.with_file_name("opencode.json.backup")).unwrap(),
        preimage
    );
    let reread = get_opencode_model_snapshot().unwrap();
    assert_eq!(reread.revision.as_ref(), Some(revision));
    assert_eq!(
        reread.selected_model.as_deref(),
        Some("gateway/existing-model")
    );
    for public in [
        serde_json::to_string(&fetched).unwrap(),
        serde_json::to_string(&outcome).unwrap(),
        serde_json::to_string(&reread).unwrap(),
    ] {
        assert!(!public.contains("OPENCODE-SECRET"));
        assert!(!public.contains(&url));
        assert!(!public.contains("apiKey"));
    }
}

#[tokio::test]
#[serial]
async fn refresh_wrong_target_stale_revision_and_changed_endpoint_have_no_effects() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let mut config = seeded_config();
    config["provider"]["gateway"]["options"]["baseURL"] = json!(url);
    write_config(temp.path(), &config);
    let path = temp.path().join(".config/opencode/opencode.json");
    let before = std::fs::read(&path).unwrap();
    let revision = get_opencode_model_snapshot().unwrap().revision;
    let mut wrong = fetch_request(revision.clone());
    wrong.provider_id = Some("missing".into());
    let mut missing_revision = fetch_request(None);
    missing_revision.api_key = "FRESH-KEY".into();
    let stale = fetch_request(Some("stale".into()));
    let mut changed = fetch_request(revision.clone());
    changed.base_url = format!("{url}/changed");
    changed.allow_no_api_key = true;
    for (request, expected) in [
        (wrong, OpenCodeModelsErrorCode::ConfigUnavailable),
        (
            missing_revision,
            OpenCodeModelsErrorCode::ConcurrentModification,
        ),
        (stale, OpenCodeModelsErrorCode::ConcurrentModification),
        (changed, OpenCodeModelsErrorCode::InvalidRequest),
    ] {
        let failure = fetch_opencode_provider_models(request).await.unwrap_err();
        assert_eq!(failure.code, expected);
        assert!(!serde_json::to_string(&failure)
            .unwrap()
            .contains("OPENCODE-SECRET"));
    }
    let mut changed_save = empty_draft_save(revision.clone());
    changed_save.base_url = format!("{url}/changed");
    assert_eq!(
        save_opencode_models(changed_save).unwrap_err().code,
        OpenCodeModelsErrorCode::InvalidRequest
    );
    assert_eq!(
        save_opencode_models(empty_draft_save(Some("stale".into()))).unwrap(),
        SaveOpenCodeModelsOutcome::ConcurrentModification
    );
    let mut wrong_save = empty_draft_save(revision);
    wrong_save.provider_id = Some("missing".into());
    assert!(save_opencode_models(wrong_save).is_err());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!path.with_file_name("opencode.json.backup").exists());
    assert!(!path.with_file_name("opencode.json.fyagent.backup").exists());
}

#[tokio::test]
#[serial]
async fn refresh_rejects_readonly_managed_native_and_invalid_connections() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let mut invalids = Vec::new();
    let mut builtin = seeded_config();
    builtin["provider"]["gateway"]
        .as_object_mut()
        .unwrap()
        .remove("npm");
    invalids.push(builtin);
    let mut native = seeded_config();
    native["providers"] =
        json!({"gateway": {"package": "@ai-sdk/openai", "settings": {"apiKey": "NATIVE-SECRET"}}});
    invalids.push(native);
    let mut managed = seeded_config();
    managed["provider"]["fyagent-openai-opencode-proxy"] = json!({"npm": DEFAULT_NPM});
    invalids.push(managed);
    let mut invalid_key = seeded_config();
    invalid_key["provider"]["gateway"]["options"]["apiKey"] = json!({"value": "SECRET"});
    invalids.push(invalid_key);
    let mut invalid_options = seeded_config();
    invalid_options["provider"]["gateway"]["options"] = json!([]);
    invalids.push(invalid_options);
    for mut config in invalids {
        if config["provider"]["gateway"]["options"].is_object() {
            config["provider"]["gateway"]["options"]["baseURL"] = json!(url);
        }
        write_config(temp.path(), &config);
        let revision = get_opencode_model_snapshot().unwrap().revision;
        assert_eq!(
            fetch_opencode_provider_models(fetch_request(revision.clone()))
                .await
                .unwrap_err()
                .code,
            OpenCodeModelsErrorCode::ConfigUnavailable
        );
        assert_eq!(
            save_opencode_models(empty_draft_save(revision))
                .unwrap_err()
                .code,
            OpenCodeModelsErrorCode::ConfigUnavailable
        );
        assert_eq!(read_config(temp.path()), config);
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test]
#[serial]
async fn refresh_reference_credentials_are_not_sent_or_rewritten() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    for reference in ["{env:OPENCODE_API_KEY}", "{file:~/.credentials/key}"] {
        let mut config = seeded_config();
        config["provider"]["gateway"]["options"]["baseURL"] = json!(url);
        config["provider"]["gateway"]["options"]["apiKey"] = json!(reference);
        write_config(temp.path(), &config);
        let revision = get_opencode_model_snapshot().unwrap().revision;
        let mut request = fetch_request(revision.clone());
        request.allow_no_api_key = true;
        let failure = fetch_opencode_provider_models(request).await.unwrap_err();
        assert_eq!(failure.code, OpenCodeModelsErrorCode::InvalidRequest);
        assert!(!serde_json::to_string(&failure).unwrap().contains(reference));
        assert!(matches!(
            save_opencode_models(empty_draft_save(revision)).unwrap(),
            SaveOpenCodeModelsOutcome::Saved { .. }
        ));
        assert_eq!(
            read_config(temp.path())["provider"]["gateway"]["options"]["apiKey"],
            reference
        );
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test]
#[serial]
async fn refresh_explicit_connection_and_optional_auth_keep_original_modes() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    for existing in [false, true] {
        for with_key in [false, true] {
            let (url, server) =
                model_server(json!({"data": [{"id": "grok-4.7"}]}).to_string()).await;
            let request = if existing {
                let mut config = seeded_config();
                config["provider"]["gateway"]["options"] = json!({"baseURL": url});
                write_config(temp.path(), &config);
                let mut request = fetch_request(get_opencode_model_snapshot().unwrap().revision);
                if with_key {
                    request.api_key = "FRESH-KEY".into();
                }
                request.allow_no_api_key = !with_key;
                request
            } else {
                FetchOpenCodeModelsRequest {
                    provider_id: None,
                    expected_revision: None,
                    base_url: url,
                    api_key: if with_key {
                        "FRESH-KEY".into()
                    } else {
                        String::new()
                    },
                    allow_no_api_key: !with_key,
                }
            };
            assert_eq!(
                fetch_opencode_provider_models(request)
                    .await
                    .unwrap()
                    .models[0]
                    .id,
                "grok-4.7"
            );
            let received = server.await.unwrap().to_ascii_lowercase();
            assert_eq!(received.contains("authorization:"), with_key);
            if with_key {
                assert!(received.contains("authorization: bearer fresh-key\r\n"));
            }
            assert!(!received.contains("opencode-secret"));
        }
    }
}

#[tokio::test]
#[serial]
async fn refresh_changed_endpoint_uses_only_fresh_key_and_redacts_echoed_secrets() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let (url, server) =
        model_server(json!({"data": [{"id": "grok-4.7", "owned_by": "FRESH-KEY"}]}).to_string())
            .await;
    let mut request = fetch_request(get_opencode_model_snapshot().unwrap().revision);
    request.base_url = url;
    request.api_key = "FRESH-KEY".into();
    let failure = fetch_opencode_provider_models(request).await.unwrap_err();
    assert_eq!(failure.code, OpenCodeModelsErrorCode::CredentialCollision);
    assert!(!serde_json::to_string(&failure)
        .unwrap()
        .contains("FRESH-KEY"));
    let received = server.await.unwrap();
    assert!(received.contains("Bearer FRESH-KEY"));
    assert!(!received.contains("OPENCODE-SECRET"));
}

#[test]
#[serial]
fn refresh_jsonc_selected_config_and_blank_draft_preserve_existing_content() {
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    write_config(temp.path(), &seeded_config());
    let json_path = temp.path().join(".config/opencode/opencode.json");
    let json_bytes = std::fs::read(&json_path).unwrap();
    let jsonc_path = json_path.with_extension("jsonc");
    let source = format!(
        "// Keep this comment\n{}\n",
        String::from_utf8(json_bytes.clone()).unwrap()
    );
    std::fs::write(&jsonc_path, &source).unwrap();
    let snapshot = get_opencode_model_snapshot().unwrap();
    assert!(snapshot.path.ends_with("opencode.jsonc"));
    assert!(matches!(
        save_opencode_models(empty_draft_save(snapshot.revision)).unwrap(),
        SaveOpenCodeModelsOutcome::Saved { .. }
    ));
    let saved = std::fs::read_to_string(&jsonc_path).unwrap();
    assert!(saved.starts_with("// Keep this comment\n"));
    assert!(saved.contains("OPENCODE-SECRET"));
    assert_eq!(std::fs::read(json_path).unwrap(), json_bytes);
    assert_eq!(
        std::fs::read(jsonc_path.with_file_name("opencode.json.backup")).unwrap(),
        source.as_bytes()
    );
    assert!(get_opencode_model_snapshot().unwrap().providers[0]
        .model_ids
        .contains(&"grok-4.7".to_owned()));
}

#[test]
fn refresh_fetch_dto_accepts_legacy_request_and_rejects_unknown_authority() {
    let request: FetchOpenCodeModelsRequest = serde_json::from_value(json!({
        "baseUrl": "https://example.test/v1", "apiKey": "SECRET", "allowNoApiKey": false
    }))
    .unwrap();
    assert_eq!(request.provider_id, None);
    assert_eq!(request.expected_revision, None);
    assert!(!format!("{request:?}").contains("SECRET"));
    assert!(serde_json::from_value::<FetchOpenCodeModelsRequest>(json!({
        "baseUrl": "", "apiKey": "", "allowNoApiKey": false, "configPath": "/arbitrary"
    }))
    .is_err());
    for invalid in [
        "",
        "file:///tmp/key",
        "https://user:secret@example.test/v1",
        "https://example.test/v1?key=secret",
        "https://example.test/v1#secret",
    ] {
        assert!(resolve_connection(None, invalid, "SECRET").is_err());
    }
}

#[tokio::test]
#[serial]
async fn refresh_saved_credential_never_follows_same_origin_redirect() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let temp = tempfile::TempDir::new().unwrap();
    let _home = TestHomeGuard::set(temp.path());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let mut config = seeded_config();
    config["provider"]["gateway"]["options"]["baseURL"] = json!(url);
    write_config(temp.path(), &config);
    let request = fetch_request(get_opencode_model_snapshot().unwrap().revision);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        loop {
            let mut buffer = [0u8; 1024];
            let size = stream.read(&mut buffer).await.unwrap();
            assert!(size > 0 && received.len() < 16 * 1024);
            received.extend_from_slice(&buffer[..size]);
            if received.windows(4).any(|part| part == b"\r\n\r\n") {
                break;
            }
        }
        stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: /credential-trap\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await.unwrap();
        drop(stream);
        (listener, String::from_utf8(received).unwrap())
    });
    let failure = tokio::time::timeout(
        Duration::from_secs(5),
        fetch_opencode_provider_models(request),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(failure.code, OpenCodeModelsErrorCode::FetchFailed);
    assert!(!serde_json::to_string(&failure)
        .unwrap()
        .contains("OPENCODE-SECRET"));
    let (listener, received) = server.await.unwrap();
    assert!(received.starts_with("GET /v1/models HTTP/1.1\r\n"));
    assert!(received.contains("Bearer OPENCODE-SECRET"));
    assert_eq!(
        listener.into_std().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(read_config(temp.path()), config);
}
