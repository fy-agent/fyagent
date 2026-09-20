use super::*;
use crate::{database::Database, provider::Provider};
use serde_json::{json, Value};

fn fixture() -> String {
    include_str!("../../../../tests/fixtures/configPackDtoContract.v1.json").to_string()
}
fn pack(name: &str) -> ConfigPack {
    let mut pack = ConfigPack::parse(fixture().as_bytes()).unwrap();
    pack.providers.truncate(1);
    pack.providers[0].name = name.into();
    pack
}
fn seeded(db: &Database, name: &str, active: bool) -> Provider {
    let p = &pack(name).providers[0];
    let mut provider = Provider::with_id(name.into(), name.into(), p.settings().unwrap(), None);
    provider.category = Some("custom".into());
    db.save_provider("codex", &provider).unwrap();
    if active {
        db.set_current_provider("codex", &provider.id).unwrap();
    }
    provider
}
fn import(service: &mut ConfigPackService, db: &Database, text: &str) -> ImportResult {
    let p = service.preview_import(db, text, vec![], vec![]).unwrap();
    service.apply(db, &p.preview_id, &p.digest, &[]).unwrap()
}
#[test]
fn config_pack_strict_schema_blocks_secrets_paths_execution_and_limits() {
    let base: Value = serde_json::from_str(&fixture()).unwrap();
    for field in [
        "apiKey",
        "auth",
        "credentialRef",
        "secretRef",
        "env",
        "command",
        "hooks",
        "path",
        "__proto__",
        "active",
    ] {
        let mut bad = base.clone();
        bad["providers"][0][field] = json!("credential-or-command");
        assert!(
            ConfigPack::parse(bad.to_string().as_bytes()).is_err(),
            "{field}"
        );
    }
    for endpoint in [
        "file:///Users/me/private.json",
        "https://user:password@example.com/v1",
        "https://api.example.com/v1?token=value",
        "https://api.example.com/#token",
        "http://localhost:9999",
        "http://127.0.0.1/v1",
        "http://192.168.0.1/v1",
        "https://api.example.com/%2e%2e/secret",
        "https://api.example.com/../config",
        "https://api.example.com/sk-secret",
    ] {
        let mut bad = base.clone();
        bad["providers"][0]["endpoint"] = json!(endpoint);
        assert!(
            ConfigPack::parse(bad.to_string().as_bytes()).is_err(),
            "{endpoint}"
        );
    }
    for name in [
        "/Users/me/config",
        r"C:\Users\me",
        "../escape",
        "Bearer token",
        "sk-secret",
        "~/.config",
    ] {
        let mut bad = base.clone();
        bad["providers"][0]["name"] = json!(name);
        assert!(
            ConfigPack::parse(bad.to_string().as_bytes()).is_err(),
            "{name}"
        );
    }
    let mut bad = base.clone();
    bad["providers"][0]["model"] = json!("./local-model");
    assert!(ConfigPack::parse(bad.to_string().as_bytes()).is_err());
    let mut bad = base.clone();
    bad["providers"][0]["wireApi"] = json!("custom");
    assert!(ConfigPack::parse(bad.to_string().as_bytes()).is_err());
    let mut bad = base.clone();
    bad["providers"][1]
        .as_object_mut()
        .unwrap()
        .remove("wireApi");
    assert!(ConfigPack::parse(bad.to_string().as_bytes()).is_err());
    let mut bad = base.clone();
    bad["format"] = json!("fyagent-config-pack/v99");
    assert_eq!(
        ConfigPack::parse(bad.to_string().as_bytes()).unwrap_err(),
        PackError::UnsupportedVersion
    );
    assert_eq!(
        ConfigPack::parse(&vec![b' '; MAX_BYTES + 1]).unwrap_err(),
        PackError::TooLarge
    );
    let duplicate =
        r#"{"format":"fyagent-config-pack/v1","format":"fyagent-config-pack/v1","providers":[]}"#;
    assert!(ConfigPack::parse(duplicate.as_bytes()).is_err());
    let too_many = ConfigPack {
        format: FORMAT.into(),
        providers: (0..33)
            .map(|n| pack(&format!("Item {n}")).providers.remove(0))
            .collect(),
    };
    assert!(too_many.validate().is_err());
}
#[test]
fn config_pack_export_is_whitelisted_and_selective() {
    let db = Database::memory().unwrap();
    let mut provider = seeded(&db, "Exported", false);
    provider.settings_config["auth"] = json!({"OPENAI_API_KEY": "private-value"});
    provider.settings_config["credentialRef"] = json!("native-reference");
    provider.settings_config["path"] = json!("/Users/private/device");
    provider.notes = Some("never export notes".into());
    // Deliberately seed a malformed legacy row, bypassing the credential-aware
    // production facade. This must never admit a real credential reference.
    db.conn
        .lock()
        .unwrap()
        .execute(
            "UPDATE providers SET settings_config=?1, notes=?2 WHERE id=?3 AND app_type='codex'",
            rusqlite::params![
                provider.settings_config.to_string(),
                provider.notes,
                provider.id
            ],
        )
        .unwrap();
    seeded(&db, "Not selected", false);
    let mut service = ConfigPackService::default();
    let candidates = ConfigPackService::candidates(&db).unwrap();
    let id = candidates
        .entries
        .iter()
        .find(|e| e.provider.name == "Exported")
        .unwrap()
        .selection_id
        .clone();
    let output = service.preview_export(&db, vec![id]).unwrap();
    for forbidden in [
        "private-value",
        "native-reference",
        "/Users/",
        "never export",
        "Not selected",
        "credentialRef",
        "OPENAI_API_KEY",
    ] {
        assert!(!output.text.contains(forbidden), "{forbidden}");
    }
    let parsed = ConfigPack::parse(output.text.as_bytes()).unwrap();
    assert_eq!(parsed.providers.len(), 1);
    db.conn
        .lock()
        .unwrap()
        .execute(
            "UPDATE providers SET name='private-value' WHERE id=?1 AND app_type='codex'",
            [&provider.id],
        )
        .unwrap();
    assert!(!ConfigPackService::candidates(&db)
        .unwrap()
        .entries
        .iter()
        .any(|p| p.provider.name == "private-value"));
    let mut settings = pack("Header secret").providers[0].settings().unwrap();
    settings["config"] = json!(format!(
        "{}\nhttp_headers = {{ Authorization = \"header-private-value\" }}\n",
        settings["config"].as_str().unwrap()
    ));
    assert!(project(PackApp::Codex, "header-private-value", &settings).is_none());
}
#[test]
fn config_pack_import_roundtrip_never_activates_and_returns_persisted_fields() {
    let db = Database::memory().unwrap();
    let mut service = ConfigPackService::default();
    let before_claude = db.get_current_provider("claude").unwrap();
    let before_codex = db.get_current_provider("codex").unwrap();
    let preview = service
        .preview_import(&db, &fixture(), vec![], vec![])
        .unwrap();
    assert!(preview
        .entries
        .iter()
        .all(|e| e.credentials_required && e.action == ImportAction::Add));
    assert!(!db
        .get_all_providers("codex")
        .unwrap()
        .values()
        .any(|p| p.name == "Team Codex"));
    let result = service
        .apply(&db, &preview.preview_id, &preview.digest, &[])
        .unwrap();
    assert_eq!(
        result.providers,
        ConfigPack::parse(fixture().as_bytes()).unwrap().providers
    );
    assert_eq!(db.get_current_provider("claude").unwrap(), before_claude);
    assert_eq!(db.get_current_provider("codex").unwrap(), before_codex);
    for app in ["claude", "codex"] {
        let providers = db.get_all_providers(app).unwrap();
        let p = providers
            .values()
            .find(|p| p.name.starts_with("Team "))
            .unwrap();
        assert_eq!(p.category.as_deref(), Some(DRAFT_CATEGORY));
        assert!(!p.in_failover_queue);
        assert!(!p.settings_config.to_string().contains("KEY"));
    }
    assert!(matches!(
        service.apply(&db, &preview.preview_id, &preview.digest, &[]),
        Err(PackError::StalePreview)
    ));
}
#[test]
fn config_pack_conflicts_default_to_skip_and_rename_preserves_existing() {
    let db = Database::memory().unwrap();
    seeded(&db, "Conflict", true);
    let mut service = ConfigPackService::default();
    let text = pack("Conflict").text().unwrap();
    let preview = service.preview_import(&db, &text, vec![], vec![]).unwrap();
    assert_eq!(preview.entries[0].action, ImportAction::Skip);
    assert!(!preview.entries[0].can_overwrite);
    let overwrite = vec![ImportChoice {
        action: ImportAction::Overwrite,
        name: None,
    }];
    assert!(matches!(
        service.preview_import(&db, &text, overwrite, vec![]),
        Err(PackError::Conflict)
    ));
    let rename = vec![ImportChoice {
        action: ImportAction::Rename,
        name: Some("Renamed".into()),
    }];
    let p = service.preview_import(&db, &text, rename, vec![]).unwrap();
    assert_eq!(p.entries[0].provider.name, "Renamed");
    let result = service.apply(&db, &p.preview_id, &p.digest, &[]).unwrap();
    assert_eq!(result.providers[0].name, "Renamed");
    assert_eq!(
        db.get_current_provider("codex").unwrap().as_deref(),
        Some("Conflict")
    );
}
#[test]
fn config_pack_only_clean_unselected_pack_drafts_can_be_overwritten() {
    let db = Database::memory().unwrap();
    let mut service = ConfigPackService::default();
    import(&mut service, &db, &pack("Draft").text().unwrap());
    let mut changed = pack("Draft");
    changed.providers[0].model = "updated-model".into();
    let preview = service
        .preview_import(
            &db,
            &changed.text().unwrap(),
            vec![ImportChoice {
                action: ImportAction::Overwrite,
                name: None,
            }],
            vec![],
        )
        .unwrap();
    assert_eq!(
        preview.entries[0].existing.as_ref().unwrap().model,
        "example-model"
    );
    assert_eq!(
        service
            .apply(&db, &preview.preview_id, &preview.digest, &[])
            .unwrap()
            .providers[0]
            .model,
        "updated-model"
    );
    let id = db
        .get_all_providers("codex")
        .unwrap()
        .values()
        .find(|p| p.name == "Draft")
        .unwrap()
        .id
        .clone();
    let local = vec![format!("codex:{id}")];
    let p = service
        .preview_import(&db, &changed.text().unwrap(), vec![], local)
        .unwrap();
    assert!(!p.entries[0].can_overwrite);
    db.conn
        .lock()
        .unwrap()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS provider_credentials (
          credential_id TEXT PRIMARY KEY, provider_id TEXT NOT NULL,
          secret_ref TEXT UNIQUE NOT NULL, secret_version TEXT NOT NULL, status TEXT NOT NULL)",
        )
        .unwrap();
    db.conn.lock().unwrap().execute(
        "INSERT INTO provider_credentials(credential_id,provider_id,secret_ref,secret_version,status) VALUES ('local',?1,'opaque','1','ready')",
        [&id],
    ).unwrap();
    let p = service
        .preview_import(&db, &changed.text().unwrap(), vec![], vec![])
        .unwrap();
    assert!(!p.entries[0].can_overwrite);
}
#[test]
fn config_pack_preview_digest_and_drift_fail_before_writes() {
    let db = Database::memory().unwrap();
    let mut service = ConfigPackService::default();
    let p = service
        .preview_import(&db, &fixture(), vec![], vec![])
        .unwrap();
    assert!(matches!(
        service.apply(&db, &p.preview_id, &"0".repeat(64), &[]),
        Err(PackError::InvalidPreview)
    ));
    seeded(&db, "Concurrent change", false);
    assert!(matches!(
        service.apply(&db, &p.preview_id, &p.digest, &[]),
        Err(PackError::StalePreview)
    ));
    assert!(!db
        .get_all_providers("codex")
        .unwrap()
        .values()
        .any(|p| p.name == "Team Codex"));
    let p = service
        .preview_import(&db, &fixture(), vec![], vec![])
        .unwrap();
    assert!(matches!(
        service.apply(&db, &p.preview_id, &p.digest, &["codex:changed".into()]),
        Err(PackError::StalePreview)
    ));
}
#[test]
fn config_pack_late_write_failure_and_readback_failure_roll_back_all_rows() {
    for trigger in [
        "CREATE TRIGGER config_pack_fault BEFORE INSERT ON providers WHEN NEW.name='Team Claude' BEGIN SELECT RAISE(ABORT,'fixture'); END;",
        "CREATE TRIGGER config_pack_fault AFTER INSERT ON providers WHEN NEW.name='Team Claude' BEGIN UPDATE providers SET name='Changed by trigger' WHERE id=NEW.id; END;",
    ] {
        let db = Database::memory().unwrap();
        seeded(&db, "Preserved", true);
        let original = db.config_pack_inventory(&[]).unwrap().revision;
        db.conn.lock().unwrap().execute_batch(trigger).unwrap();
        let mut service = ConfigPackService::default();
        let p = service.preview_import(&db, &fixture(), vec![], vec![]).unwrap();
        assert!(service.apply(&db, &p.preview_id, &p.digest, &[]).is_err());
        assert_eq!(db.config_pack_inventory(&[]).unwrap().revision, original);
        assert_eq!(db.get_current_provider("codex").unwrap().as_deref(), Some("Preserved"));
    }
}
#[test]
fn config_pack_preview_capacity_cancel_and_expiry() {
    let db = Database::memory().unwrap();
    let mut service = ConfigPackService::default();
    for _ in 0..16 {
        service
            .preview_import(&db, &fixture(), vec![], vec![])
            .unwrap();
    }
    assert!(matches!(
        service.preview_import(&db, &fixture(), vec![], vec![]),
        Err(PackError::Busy)
    ));
    let id = service.imports.keys().next().unwrap().clone();
    service.cancel(&id);
    let p = service
        .preview_import(&db, &fixture(), vec![], vec![])
        .unwrap();
    service.imports.get_mut(&p.preview_id).unwrap().created = Instant::now() - TTL;
    assert!(matches!(
        service.apply(&db, &p.preview_id, &p.digest, &[]),
        Err(PackError::StalePreview)
    ));
}
#[test]
fn config_pack_files_are_bounded_regular_json_and_export_never_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let path = root.join("connections.fyagent-config.json");
    export_file(&path, fixture().as_bytes()).unwrap();
    assert_eq!(
        ConfigPack::parse(read_file(&path).unwrap().as_bytes())
            .unwrap()
            .providers
            .len(),
        2
    );
    let original = std::fs::read(&path).unwrap();
    assert!(matches!(
        export_file(&path, fixture().as_bytes()),
        Err(PackError::Conflict)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(read_file(&root.join("../escape.json")).is_err());
    let directory = root.join("directory.json");
    std::fs::create_dir(&directory).unwrap();
    assert!(matches!(
        read_file(&directory),
        Err(PackError::UnsafeContent)
    ));
    let oversized = root.join("oversized.json");
    std::fs::write(&oversized, vec![b' '; MAX_BYTES + 1]).unwrap();
    assert!(matches!(read_file(&oversized), Err(PackError::TooLarge)));
    #[cfg(target_os = "macos")]
    {
        let link = root.join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(matches!(read_file(&link), Err(PackError::UnsafeContent)));
    }
}
