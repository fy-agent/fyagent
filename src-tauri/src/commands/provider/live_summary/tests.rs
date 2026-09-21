use super::*;
use crate::commands::provider::build_provider_public_summary_result;
use crate::database::Database;
use crate::provider::Provider;
use serde_json::{json, Value};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};

struct TestHome(Option<std::ffi::OsString>);

impl TestHome {
    fn set(path: &Path) -> Self {
        let previous = std::env::var_os("FYAGENT_TEST_HOME");
        std::env::set_var("FYAGENT_TEST_HOME", path);
        crate::settings::reload_settings().unwrap();
        Self(previous)
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
        crate::settings::reload_settings().unwrap();
    }
}

fn setup() -> (tempfile::TempDir, TestHome, PathBuf, Database) {
    let home = tempfile::tempdir().unwrap();
    let guard = TestHome::set(home.path());
    let path = crate::codex_config::get_codex_config_path();
    assert!(path.starts_with(home.path()));
    let db = Database::memory().unwrap();
    let provider = Provider::with_id(
        "saved-source".into(),
        "Saved source".into(),
        json!({ "config": "model = 'saved-model'\nmodel_provider = 'saved'\n[model_providers.saved]\nbase_url = 'https://saved.example.test/v1'\nwire_api = 'responses'" }),
        None,
    );
    db.save_provider("codex", &provider).unwrap();
    db.set_current_provider("codex", &provider.id).unwrap();
    (home, guard, path, db)
}

fn read_summary(db: &Database) -> Value {
    serde_json::to_value(
        build_provider_public_summary_result(
            &AppType::Codex,
            db.get_all_providers("codex").unwrap(),
            db.get_current_provider("codex").unwrap().unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn assert_saved_available(summary: &Value) {
    assert_eq!(summary["currentId"], "saved-source");
    assert_eq!(summary["providers"]["saved-source"]["name"], "Saved source");
    assert_eq!(
        summary["providers"]["saved-source"]["connection"]["modelId"],
        "saved-model"
    );
}

#[test]
#[serial]
fn actual_external_file_differs_from_saved_provider_without_any_write() {
    let (_home, _guard, path, db) = setup();
    let content = "model = 'external-model'\nmodel_provider = 'external'\n[model_providers.external]\nbase_url = 'https://external.example.test/v1'\nwire_api = 'chat'\nexperimental_bearer_token = 'fixture-live-secret'\n";
    write(&path, content);
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(
        summary["live"],
        json!({
            "target": "codex", "state": "configured", "exists": true,
            "connection": { "baseUrl": "https://external.example.test/v1", "modelId": "external-model", "protocol": "chat" }
        })
    );
    assert!(!summary.to_string().contains("fixture-live-secret"));
    assert!(!summary.to_string().contains("experimental_bearer_token"));
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    assert!(!path.with_file_name("config.toml.fyagent.backup").exists());
    assert_eq!(db.get_all_providers("codex").unwrap().len(), 1);
}

#[test]
#[serial]
fn missing_empty_and_invalid_files_keep_saved_summary() {
    let (_home, _guard, path, db) = setup();
    let missing = read_summary(&db);
    assert_saved_available(&missing);
    assert_eq!(
        missing["live"],
        json!({ "target": "codex", "state": "missing", "exists": false, "connection": null })
    );
    for (content, state) in [
        ("# user has no model config\n", "not_configured"),
        ("model = [", "unreadable"),
    ] {
        write(&path, content);
        let summary = read_summary(&db);
        assert_saved_available(&summary);
        assert_eq!(summary["live"]["state"], state);
        assert_eq!(summary["live"]["exists"], true);
        assert_eq!(summary["live"]["connection"], Value::Null);
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
}

#[test]
#[serial]
fn target_metadata_failure_is_explicit_without_losing_saved_sources() {
    let (_home, _guard, path, db) = setup();
    fs::create_dir_all(&path).unwrap();
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(summary["writeTargets"], json!([]));
    assert_eq!(
        summary["live"],
        json!({ "target": "codex", "state": "unreadable", "exists": null, "connection": null })
    );
    assert!(path.is_dir());
}

#[test]
#[serial]
fn malformed_auth_is_unreadable_without_echoing_reader_error() {
    let (_home, _guard, path, db) = setup();
    write(&path, "model = 'external-model'\n");
    let auth_path = crate::codex_config::get_codex_auth_path();
    write(&auth_path, "{ fixture-private-auth");
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(summary["live"]["state"], "unreadable");
    assert!(!summary.to_string().contains("fixture-private-auth"));
    assert_eq!(
        fs::read_to_string(auth_path).unwrap(),
        "{ fixture-private-auth"
    );
}

#[test]
#[serial]
fn live_auth_and_header_collisions_cannot_enter_public_fields() {
    let (_home, _guard, path, db) = setup();
    let auth_path = crate::codex_config::get_codex_auth_path();
    write(&auth_path, r#"{"OPENAI_API_KEY":"fixture-secret"}"#);
    for content in [
        "model = 'fixture-secret'\n",
        "model = 'external-model'\nmodel_provider = 'external'\n[model_providers.external]\nbase_url = 'https://external.example.test/%66ixture-secret'\n",
        "model = 'header-secret'\nmodel_provider = 'external'\n[model_providers.external.http_headers]\nAuthorization = 'Bearer header-secret'\n",
    ] {
        write(&path, content);
        let summary = read_summary(&db);
        assert_saved_available(&summary);
        assert_eq!(summary["live"]["state"], "unreadable");
        assert_eq!(summary["live"]["connection"], Value::Null);
        for secret in ["fixture-secret", "header-secret", "%66ixture-secret"] {
            assert!(!summary.to_string().contains(secret));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
}

#[test]
fn explicit_profile_and_unspecified_values_are_not_guessed() {
    let settings = json!({"config": "model = 'root-model'\nprofile = 'work'\n[profiles.work]\nmodel = 'profile-model'\nmodel_provider = 'other'\n[model_providers.other]\nbase_url = 'https://profile.example.test/v1'\n"});
    let connection =
        serde_json::to_value(project(&AppType::Codex, &settings, &[]).unwrap()).unwrap();
    assert_eq!(
        connection,
        json!({"baseUrl": "https://profile.example.test/v1", "modelId": "profile-model", "protocol": null})
    );
    let partial = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "model = 'only-model'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        partial,
        json!({"baseUrl": null, "modelId": "only-model", "protocol": null})
    );
    assert!(project(&AppType::Codex, &json!({"config": "model_provider = 'custom'\n[model_providers.custom]\nwire_api = 'responses'"}), &[]).unwrap().is_none());
    let top_level = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "model = 'm'\nbase_url = 'https://gateway.example/v1'\nwire_api = 'responses'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        top_level,
        json!({"baseUrl": "https://gateway.example/v1", "modelId": "m", "protocol": "responses"})
    );
    let base_only = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "base_url = 'https://gateway.example/v1'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        base_only,
        json!({"baseUrl": "https://gateway.example/v1", "modelId": null, "protocol": null})
    );
    let selected_wins = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "model = 'm'\nbase_url = 'https://top.example/v1'\nwire_api = 'chat'\nmodel_provider = 'active'\n[model_providers.active]\nbase_url = 'https://active.example/v1'\nwire_api = 'responses'\n[profiles.idle]\nmodel = 'idle'\nbase_url = 'https://idle.example/v1'\nwire_api = 'chat'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        selected_wins,
        json!({"baseUrl": "https://active.example/v1", "modelId": "m", "protocol": "responses"})
    );
    let profile_over_root = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "model = 'root-model'\nbase_url = 'https://root.example/v1'\nwire_api = 'chat'\nprofile = 'work'\n[profiles.work]\nmodel = 'profile-model'\nbase_url = 'https://profile.example/v1'\nwire_api = 'responses'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        profile_over_root,
        json!({"baseUrl": "https://profile.example/v1", "modelId": "profile-model", "protocol": "responses"})
    );
    let provider_over_profile = serde_json::to_value(
        project(
            &AppType::Codex,
            &json!({"config": "base_url = 'https://root.example/v1'\nprofile = 'work'\n[profiles.work]\nmodel = 'profile-model'\nmodel_provider = 'active'\nbase_url = 'https://profile.example/v1'\n[model_providers.active]\nbase_url = 'https://active.example/v1'\nwire_api = 'responses'"}),
            &[],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        provider_over_profile,
        json!({"baseUrl": "https://active.example/v1", "modelId": "profile-model", "protocol": "responses"})
    );
}

#[test]
fn unsafe_public_values_and_inherited_credentials_fail_closed() {
    for config in [
        "model = 42".to_string(),
        "model = 'secret-from-saved-provider'".to_string(),
        format!("model = '{}'", "x".repeat(257)),
        "model = 'external'\nmodel_provider = 'p'\n[model_providers.p]\nwire_api = 'unknown'".to_string(),
        "model = 'external'\nmodel_provider = 'p'\n[model_providers.p]\nbase_url = 'https://user:secret@example.test/v1'".to_string(),
        "model = 'external'\nmodel_provider = 'p'\n[model_providers.p]\nbase_url = 'https://example.test/v1?api_key=secret'".to_string(),
        "base_url = 42".to_string(),
        "base_url = ['https://gateway.example/v1']".to_string(),
        "profile = 'work'\n[profiles.work]\nbase_url = 42".to_string(),
        "model_provider = 'p'\n[model_providers.p]\nbase_url = 42".to_string(),
        "wire_api = ['responses']".to_string(),
        "profile = 'work'\n[profiles.work]\nwire_api = 1".to_string(),
        "model_provider = 'p'\n[model_providers.p]\nwire_api = []".to_string(),
    ] {
        assert!(project(&AppType::Codex, &json!({"config": config}), &["secret-from-saved-provider".into()]).is_err());
    }
}

#[test]
fn other_provider_targets_have_closed_partial_projections() {
    let claude = project(&AppType::Claude, &json!({"env": {"ANTHROPIC_MODEL": "claude-model", "ANTHROPIC_AUTH_TOKEN": "fixture-token"}}), &[]).unwrap();
    assert_eq!(
        serde_json::to_value(claude).unwrap(),
        json!({"baseUrl": null, "modelId": "claude-model", "protocol": "anthropic"})
    );
    let grok = project(&AppType::GrokBuild, &json!({"config": "[models]\ndefault = 'main'\n[model.main]\nmodel = 'grok-model'\nbase_url = 'https://api.example.test/v1'\napi_backend = 'responses'\napi_key = 'fixture-token'"}), &[]).unwrap();
    assert_eq!(
        serde_json::to_value(grok).unwrap(),
        json!({"baseUrl": "https://api.example.test/v1", "modelId": "grok-model", "protocol": "responses"})
    );
}

#[test]
#[serial]
fn top_level_and_selected_provider_routes_are_read_from_actual_files() {
    let (_home, _guard, path, db) = setup();
    write(
        &path,
        "model = 'm'\nbase_url = 'https://gateway.example/v1'\nwire_api = 'responses'\n",
    );
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(
        summary["live"],
        json!({
            "target": "codex", "state": "configured", "exists": true,
            "connection": { "baseUrl": "https://gateway.example/v1", "modelId": "m", "protocol": "responses" }
        })
    );
    write(&path, "base_url = 'https://gateway.example/v1'\n");
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(
        summary["live"],
        json!({
            "target": "codex", "state": "configured", "exists": true,
            "connection": { "baseUrl": "https://gateway.example/v1", "modelId": null, "protocol": null }
        })
    );
    write(
        &path,
        "model = 'm'\nbase_url = 'https://top.example/v1'\nwire_api = 'chat'\nmodel_provider = 'active'\n[model_providers.active]\nbase_url = 'https://active.example/v1'\nwire_api = 'responses'\n[profiles.idle]\nmodel = 'idle'\nbase_url = 'https://idle.example/v1'\n",
    );
    let summary = read_summary(&db);
    assert_eq!(
        summary["live"]["connection"],
        json!({ "baseUrl": "https://active.example/v1", "modelId": "m", "protocol": "responses" })
    );
    assert!(fs::read_to_string(&path).unwrap().contains("idle.example"));
    write(
        &path,
        "model = 'root-model'\nbase_url = 'https://root.example/v1'\nwire_api = 'chat'\nprofile = 'work'\n[profiles.work]\nmodel = 'profile-model'\nbase_url = 'https://profile.example/v1'\nwire_api = 'responses'\n",
    );
    let summary = read_summary(&db);
    assert_saved_available(&summary);
    assert_eq!(
        summary["live"]["connection"],
        json!({ "baseUrl": "https://profile.example/v1", "modelId": "profile-model", "protocol": "responses" })
    );
    write(
        &path,
        "base_url = 'https://root.example/v1'\nprofile = 'work'\n[profiles.work]\nmodel = 'profile-model'\nmodel_provider = 'active'\nbase_url = 'https://profile.example/v1'\n[model_providers.active]\nbase_url = 'https://active.example/v1'\nwire_api = 'responses'\n",
    );
    let summary = read_summary(&db);
    assert_eq!(
        summary["live"]["connection"],
        json!({ "baseUrl": "https://active.example/v1", "modelId": "profile-model", "protocol": "responses" })
    );
}

#[test]
#[serial]
fn wrong_route_field_types_make_actual_files_unreadable() {
    let (_home, _guard, path, db) = setup();
    for content in [
        "base_url = 42\n",
        "base_url = ['https://gateway.example/v1']\n",
        "profile = 'work'\n[profiles.work]\nbase_url = 42\n",
        "model_provider = 'p'\n[model_providers.p]\nbase_url = 42\n",
    ] {
        write(&path, content);
        let summary = read_summary(&db);
        assert_saved_available(&summary);
        assert_eq!(summary["live"]["state"], "unreadable");
        assert_eq!(summary["live"]["connection"], Value::Null);
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
    }
}
