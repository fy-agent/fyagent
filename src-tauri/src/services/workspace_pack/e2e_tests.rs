//! Real end-to-end integration tests for Wave 1 + Wave 2.
//!
//! Exercises live loopback HTTP probing, cross-app cloning, key rotation,
//! old-PC -> `.fyagent-pack` (<=100 MiB) -> brand-new-PC restore, all inside
//! completely isolated temporary `HOME` directories and in-memory SQLite DBs.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{
    app_config::{AppType, InstalledSkill, McpApps, McpServer, SkillApps},
    database::Database,
    prompt::Prompt,
    services::{
        mcp::McpService,
        prompt::PromptService,
        provider::{
            batch_setup::{
                apply_batch_provider_setup, BatchProviderApplyRequest, BatchSetupTargetId,
                BatchSetupTargetSpec, BatchTargetStatus,
            },
            custom_templates::{
                list_custom_gateway_templates, save_custom_gateway_template,
                SaveCustomGatewayTemplateRequest,
            },
            model_overview::{
                apply_api_key_rotation, apply_clone_model_config, get_model_health_overview,
                preview_api_key_rotation, preview_clone_model_config, ApiKeyRotationApplyRequest,
                ApiKeyRotationPreviewRequest, CloneModelConfigRequest, ModelCredentialSource,
            },
        },
        provider_api::ApiProtocol,
        saved_model_probe::{probe_saved_model, SavedModelProbeCategory, SavedModelProbeRequest},
        skill::SkillService,
        workspace_pack::{
            apply_workspace_pack_import, export_workspace_pack, inspect_workspace_pack,
            preview_workspace_pack_export,
            schema::{
                decode_pack_container, encode_pack_container_from_json,
                serialize_payload_with_sha256, SessionProjectionMode, HARD_LIMIT_BYTES, PACK_MAGIC,
            },
            WorkspacePackExportOptions, WorkspacePackExportRequest, WorkspacePackImportRequest,
            WorkspacePackInspectRequest,
        },
    },
    store::AppState,
};

struct IsolatedHomeGuard {
    _dir: tempfile::TempDir,
    path: PathBuf,
    prev_test_home: Option<String>,
    prev_home: Option<String>,
}

impl IsolatedHomeGuard {
    fn new(prefix: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("create isolated temp home");
        let path = fs::canonicalize(dir.path()).unwrap_or_else(|_| dir.path().to_path_buf());
        let prev_test_home = std::env::var("FYAGENT_TEST_HOME").ok();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("FYAGENT_TEST_HOME", &path);
        std::env::set_var("HOME", &path);
        Self {
            _dir: dir,
            path,
            prev_test_home,
            prev_home,
        }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for IsolatedHomeGuard {
    fn drop(&mut self) {
        match &self.prev_test_home {
            Some(v) => std::env::set_var("FYAGENT_TEST_HOME", v),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
        match &self.prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}

async fn spawn_mock_gateway(
    expected_key: Arc<Mutex<String>>,
) -> (String, tokio::task::JoinHandle<()>) {
    async fn handle_probe(
        State(expected): State<Arc<Mutex<String>>>,
        headers: HeaderMap,
    ) -> (StatusCode, Json<Value>) {
        let want = expected.lock().unwrap().clone();
        let bearer = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(str::trim);
        let x_api = headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(str::trim);
        if bearer == Some(want.as_str()) || x_api == Some(want.as_str()) {
            (
                StatusCode::OK,
                Json(json!({
                    "id": "resp_ok",
                    "choices": [{ "message": { "content": "ok" } }],
                    "content": [{ "type": "text", "text": "ok" }],
                })),
            )
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": { "message": "invalid api key" } })),
            )
        }
    }

    let app = Router::new()
        .route("/v1/chat/completions", post(handle_probe))
        .route("/v1/responses", post(handle_probe))
        .route("/v1/messages", post(handle_probe))
        .with_state(expected_key);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback gateway");
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://127.0.0.1:{}/v1", addr.port()), handle)
}

#[tokio::test]
#[serial_test::serial]
async fn wave1_e2e_health_probe_clone_and_key_rotation_against_live_loopback_gateway() {
    let home = IsolatedHomeGuard::new("fyagent-e2e-wave1-");
    let state = AppState::new(Arc::new(Database::memory().expect("in-memory db")));

    let active_key = Arc::new(Mutex::new("sk-initial-e2e-key".to_string()));
    let (base_url, server_handle) = spawn_mock_gateway(Arc::clone(&active_key)).await;

    // 1. Batch-setup Codex with the initial key.
    let setup_res = apply_batch_provider_setup(
        &state,
        BatchProviderApplyRequest {
            api_key: "sk-initial-e2e-key".to_string(),
            targets: vec![BatchSetupTargetSpec {
                target: BatchSetupTargetId::Codex,
                provider_id: "e2e-gw".to_string(),
                provider_name: "E2E Gateway".to_string(),
                base_url: base_url.clone(),
                model_id: "gpt-5.4-mini".to_string(),
                extra_model_ids: Vec::new(),
                protocol: Some(ApiProtocol::Responses),
            }],
        },
    )
    .expect("setup codex");
    assert_eq!(setup_res.outcomes[0].status, BatchTargetStatus::Applied);

    // 2. Verify overview reports configured Codex without leaking any secret key.
    let overview = get_model_health_overview(&state).expect("health overview");
    let overview_json = serde_json::to_string(&overview).unwrap();
    assert!(!overview_json.contains("sk-initial-e2e-key"));
    let codex_snap = overview
        .targets
        .iter()
        .find(|t| t.target == BatchSetupTargetId::Codex)
        .unwrap();
    assert!(codex_snap.configured);
    assert_eq!(codex_snap.credential_source, ModelCredentialSource::ApiKey);
    assert!(codex_snap.can_probe && codex_snap.can_clone_source && codex_snap.can_rotate_key);

    // 3. Real HTTP probe on Codex succeeds against live loopback server.
    let probe1 = probe_saved_model(
        &state,
        SavedModelProbeRequest {
            target: BatchSetupTargetId::Codex,
            provider_id: codex_snap.provider_id.clone(),
            model_id: "gpt-5.4-mini".to_string(),
        },
    )
    .await
    .expect("probe codex");
    assert!(probe1.ok, "probe failed: {:?}", probe1);
    assert_eq!(probe1.category, SavedModelProbeCategory::Ok);

    // 4. Clone Codex -> OpenCode & omp; verify files landed and live probes pass.
    let clone_req = CloneModelConfigRequest {
        source_target: BatchSetupTargetId::Codex,
        destination_targets: vec![BatchSetupTargetId::Opencode, BatchSetupTargetId::Omp],
        model_id_override: None,
        base_url_overrides: Default::default(),
    };
    let clone_preview =
        preview_clone_model_config(&state, clone_req.clone()).expect("preview clone");
    assert_eq!(clone_preview.items.len(), 2);

    let clone_apply = apply_clone_model_config(&state, clone_req).expect("apply clone");
    assert!(clone_apply
        .outcomes
        .iter()
        .all(|o| o.status == BatchTargetStatus::Applied));
    assert!(home.path().join(".config/opencode/opencode.json").is_file());
    assert!(home.path().join(".omp/agent/models.yml").is_file());

    // Fetch refreshed overview to grab exact provider IDs for OpenCode and omp.
    let overview_after_clone = get_model_health_overview(&state).expect("overview after clone");
    let omp_snap = overview_after_clone
        .targets
        .iter()
        .find(|t| t.target == BatchSetupTargetId::Omp)
        .unwrap();
    assert!(omp_snap.configured && omp_snap.provider_id.is_some());

    let omp_probe = probe_saved_model(
        &state,
        SavedModelProbeRequest {
            target: BatchSetupTargetId::Omp,
            provider_id: omp_snap.provider_id.clone(),
            model_id: "gpt-5.4-mini".to_string(),
        },
    )
    .await
    .expect("probe cloned omp");
    assert!(omp_probe.ok, "omp probe failed: {:?}", omp_probe);

    // Inject a custom advanced TOML setting on the active Codex provider to verify
    // API key rotation updates IN PLACE without wiping custom settings or provider ID.
    let orig_codex_id = codex_snap.provider_id.clone().expect("codex provider id");
    {
        let mut stored = state
            .db
            .get_provider_by_id(&orig_codex_id, AppType::Codex.as_str())
            .unwrap()
            .expect("stored codex provider");
        let existing_cfg = stored.settings_config["config"]
            .as_str()
            .unwrap_or("")
            .to_string();
        stored.settings_config["config"] =
            Value::String(format!("{existing_cfg}\nsupports_websockets = true\n"));
        state
            .db
            .save_provider(AppType::Codex.as_str(), &stored)
            .expect("save custom codex setting");
    }

    // 5. Simulate gateway rotating API key: old probe must now fail with InvalidKey (401).
    *active_key.lock().unwrap() = "sk-rotated-e2e-key".to_string();
    let stale_probe = probe_saved_model(
        &state,
        SavedModelProbeRequest {
            target: BatchSetupTargetId::Codex,
            provider_id: Some(orig_codex_id.clone()),
            model_id: "gpt-5.4-mini".to_string(),
        },
    )
    .await
    .expect("probe with stale key");
    assert!(!stale_probe.ok);
    assert_eq!(stale_probe.category, SavedModelProbeCategory::InvalidKey);
    assert_eq!(stale_probe.http_status, Some(401));

    // 6. One-click key rotation across Codex, OpenCode, omp restores 200 OK everywhere
    // without wiping Codex custom settings or duplicating `fyagent-fyagent-` in omp.
    let rot_targets = vec![
        BatchSetupTargetId::Codex,
        BatchSetupTargetId::Opencode,
        BatchSetupTargetId::Omp,
    ];
    let rot_preview = preview_api_key_rotation(
        &state,
        ApiKeyRotationPreviewRequest {
            targets: rot_targets.clone(),
        },
    )
    .expect("preview rotation");
    assert_eq!(rot_preview.items.len(), 3);
    assert!(rot_preview.items.iter().all(|item| item.ready));

    let rot_apply = apply_api_key_rotation(
        &state,
        ApiKeyRotationApplyRequest {
            api_key: "  sk-rotated-e2e-key  ".to_string(),
            targets: rot_targets,
        },
    )
    .expect("apply rotation");
    assert!(rot_apply
        .outcomes
        .iter()
        .all(|o| o.status == BatchTargetStatus::Applied));

    // Verify Codex kept original provider ID + custom `supports_websockets = true`.
    let rotated_codex_id = state
        .db
        .get_current_provider(AppType::Codex.as_str())
        .unwrap()
        .expect("current codex id");
    assert_eq!(rotated_codex_id, orig_codex_id);
    let rotated_codex_toml =
        fs::read_to_string(home.path().join(".codex/config.toml")).expect("codex config.toml");
    assert!(rotated_codex_toml.contains("supports_websockets = true"));

    // Verify omp has ZERO `fyagent-fyagent-` duplicate prefix and ZERO leaked old key.
    let rotated_omp_yml =
        fs::read_to_string(home.path().join(".omp/agent/models.yml")).expect("omp models.yml");
    assert!(!rotated_omp_yml.contains("fyagent-fyagent-"));
    assert!(!rotated_omp_yml.contains("sk-initial-e2e-key"));
    assert!(rotated_omp_yml.contains("sk-rotated-e2e-key"));

    let overview_after_rot = get_model_health_overview(&state).expect("overview after rotation");
    for target in [
        BatchSetupTargetId::Codex,
        BatchSetupTargetId::Opencode,
        BatchSetupTargetId::Omp,
    ] {
        let snap = overview_after_rot
            .targets
            .iter()
            .find(|t| t.target == target)
            .unwrap();
        let ok_probe = probe_saved_model(
            &state,
            SavedModelProbeRequest {
                target,
                provider_id: snap.provider_id.clone(),
                model_id: "gpt-5.4-mini".to_string(),
            },
        )
        .await
        .expect("probe after rotation");
        assert!(
            ok_probe.ok,
            "target {target:?} failed after rotation: {ok_probe:?}"
        );
    }

    server_handle.abort();
}

#[test]
#[serial_test::serial]
fn wave2_e2e_old_pc_to_encrypted_pack_under_100mib_to_fresh_new_pc() {
    let pack_dir = tempfile::tempdir().expect("pack exchange dir");
    let pack_path = pack_dir.path().join("portable-hardware.fyagent-pack");

    // ── Phase 1: Old PC setup & export ──
    {
        let _old_home = IsolatedHomeGuard::new("fyagent-e2e-oldpc-");
        let old_state = AppState::new(Arc::new(Database::memory().expect("old pc db")));

        // Models (Codex + omp)
        let setup = apply_batch_provider_setup(
            &old_state,
            BatchProviderApplyRequest {
                api_key: "sk-old-pc-secret-999".to_string(),
                targets: vec![
                    BatchSetupTargetSpec {
                        target: BatchSetupTargetId::Codex,
                        provider_id: "corp-gw".to_string(),
                        provider_name: "Corp Gateway".to_string(),
                        base_url: "https://gateway.corp.example/v1".to_string(),
                        model_id: "gpt-5.4".to_string(),
                        extra_model_ids: Vec::new(),
                        protocol: Some(ApiProtocol::Responses),
                    },
                    BatchSetupTargetSpec {
                        target: BatchSetupTargetId::Omp,
                        provider_id: "corp-gw".to_string(),
                        provider_name: "Corp Gateway".to_string(),
                        base_url: "https://gateway.corp.example/v1".to_string(),
                        model_id: "gpt-5.4".to_string(),
                        extra_model_ids: Vec::new(),
                        protocol: Some(ApiProtocol::Chat),
                    },
                ],
            },
        )
        .expect("old pc model setup");
        assert!(setup
            .outcomes
            .iter()
            .all(|o| o.status == BatchTargetStatus::Applied));

        // Custom gateway template
        save_custom_gateway_template(
            &old_state.db,
            SaveCustomGatewayTemplateRequest {
                id: None,
                name: "内部聚合网关".to_string(),
                summary: "团队共享".to_string(),
                base_url: "https://gateway.corp.example/v1".to_string(),
                anthropic_base_url: Some("https://gateway.corp.example/anthropic".to_string()),
                default_model_id: "gpt-5.4".to_string(),
                codex_protocol: Some(ApiProtocol::Responses),
                api_key_url: None,
            },
        )
        .expect("save custom template");

        // MCP server with sensitive env token (must be encrypted, NEVER plaintext in pack!)
        McpService::upsert_server(
            &old_state,
            McpServer {
                id: "fs-mcp".to_string(),
                name: "Filesystem MCP".to_string(),
                server: json!({
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
                    "env": {
                        "GITHUB_TOKEN": "ghp_e2e_secret_token_999"
                    }
                }),
                apps: McpApps {
                    claude: true,
                    codex: true,
                    ..Default::default()
                },
                description: Some("Local FS tools".to_string()),
                homepage: None,
                docs: None,
                tags: vec!["dev".to_string()],
            },
        )
        .expect("upsert mcp");

        // Global prompt
        PromptService::upsert_prompt(
            &old_state,
            AppType::Claude,
            "p-main",
            Prompt {
                id: "p-main".to_string(),
                name: "全局规范".to_string(),
                content: "# Rules\nAlways write concise Rust.".to_string(),
                description: None,
                enabled: true,
                created_at: None,
                updated_at: None,
            },
        )
        .expect("upsert prompt");

        // Text skill + ignored `.git` + oversized binary blob (must NOT enter pack!)
        let ssot = SkillService::get_ssot_dir().expect("ssot dir");
        let skill_dir = ssot.join("my-deploy-skill");
        fs::create_dir_all(skill_dir.join(".git")).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "# Deploy Skill\nRun tests first.\n",
        )
        .unwrap();
        fs::write(
            skill_dir.join(".git/config"),
            "[core]\nrepositoryformatversion = 0\n",
        )
        .unwrap();
        fs::write(skill_dir.join("huge.bin"), vec![0u8; 320 * 1024]).unwrap();
        old_state
            .db
            .save_skill(&InstalledSkill {
                id: "local:my-deploy-skill".to_string(),
                name: "My Deploy Skill".to_string(),
                description: Some("Portable skill".to_string()),
                directory: "my-deploy-skill".to_string(),
                repo_owner: None,
                repo_name: None,
                repo_branch: None,
                readme_url: None,
                apps: SkillApps {
                    claude: true,
                    codex: true,
                    ..Default::default()
                },
                installed_at: 1763550000,
                content_hash: None,
                updated_at: 1763550000,
                path: None,
            })
            .expect("save skill");

        // Preview budget and export encrypted `.fyagent-pack`
        let options = WorkspacePackExportOptions {
            include_models: true,
            include_custom_templates: true,
            include_mcp: true,
            include_prompts: true,
            include_skills: true,
            include_sessions: true,
            include_api_keys: true,
            session_mode: SessionProjectionMode::FullDialogue,
            session_max_age_days: Some(90),
            session_max_count: Some(50),
        };
        let preview =
            preview_workspace_pack_export(&old_state, options.clone()).expect("preview export");
        assert!(preview.breakdown.within_hard_limit);
        assert_eq!(preview.counts.models, 2);
        assert_eq!(preview.counts.custom_templates, 1);
        assert_eq!(preview.counts.mcp_servers, 1);
        assert_eq!(preview.counts.prompts, 1);
        assert_eq!(preview.counts.skills, 1);
        assert_eq!(preview.api_key_count, 2);

        let outcome = export_workspace_pack(
            &old_state,
            WorkspacePackExportRequest {
                target_path: pack_path.to_string_lossy().to_string(),
                options,
                passphrase: Some("migrate-secret-2026".to_string()),
            },
        )
        .expect("export pack");
        assert!(outcome.compressed_bytes <= HARD_LIMIT_BYTES);
        assert!(outcome.compressed_bytes < 512 * 1024);
        assert!(outcome.has_encrypted_secrets);
    }

    // Verify raw file on disk never leaks plaintext API keys, MCP env tokens, or huge binary blobs.
    let raw_pack = fs::read(&pack_path).expect("read pack bytes");
    assert_eq!(&raw_pack[0..8], PACK_MAGIC);
    let raw_pack_lossy = String::from_utf8_lossy(&raw_pack);
    assert!(!raw_pack_lossy.contains("sk-old-pc-secret-999"));
    assert!(!raw_pack_lossy.contains("ghp_e2e_secret_token_999"));
    assert!(raw_pack.len() < 100 * 1024);

    // Tampering test: modifying unencrypted model base_url + recomputing payload_sha256
    // MUST fail AES-256-GCM AAD decryption when inspected with the real passphrase!
    {
        let mut decoded = decode_pack_container(&raw_pack).expect("decode real pack");
        decoded.payload.models[0].base_url = "https://evil.attacker.invalid/v1".to_string();
        let (evil_json, evil_sha) = serialize_payload_with_sha256(&decoded.payload).unwrap();
        let mut evil_manifest = decoded.manifest.clone();
        let evil_bytes = encode_pack_container_from_json(
            &mut evil_manifest,
            &evil_json,
            &evil_sha,
            decoded.secrets.as_ref(),
        )
        .unwrap();
        let evil_path = pack_dir.path().join("tampered.fyagent-pack");
        fs::write(&evil_path, evil_bytes).unwrap();
        assert!(inspect_workspace_pack(WorkspacePackInspectRequest {
            path: evil_path.to_string_lossy().to_string(),
            passphrase: Some("migrate-secret-2026".to_string()),
        })
        .is_err());
    }

    // ── Phase 2: Brand-new empty PC inspect, unlock & import ──
    {
        let new_home = IsolatedHomeGuard::new("fyagent-e2e-newpc-");
        let new_state = AppState::new(Arc::new(Database::memory().expect("new pc db")));

        // Without passphrase: locked!
        let locked = inspect_workspace_pack(WorkspacePackInspectRequest {
            path: pack_path.to_string_lossy().to_string(),
            passphrase: None,
        })
        .expect("inspect locked");
        assert!(locked.secrets_locked);
        assert!(locked.decrypted_api_key_targets.is_empty());

        // Wrong passphrase: rejected!
        assert!(inspect_workspace_pack(WorkspacePackInspectRequest {
            path: pack_path.to_string_lossy().to_string(),
            passphrase: Some("wrong-passphrase".to_string()),
        })
        .is_err());

        // Right passphrase: unlocked!
        let unlocked = inspect_workspace_pack(WorkspacePackInspectRequest {
            path: pack_path.to_string_lossy().to_string(),
            passphrase: Some("migrate-secret-2026".to_string()),
        })
        .expect("inspect unlocked");
        assert!(!unlocked.secrets_locked);
        assert_eq!(unlocked.decrypted_api_key_targets.len(), 2);

        // Apply import on brand-new PC!
        let imported = apply_workspace_pack_import(
            &new_state,
            WorkspacePackImportRequest {
                path: pack_path.to_string_lossy().to_string(),
                passphrase: Some("migrate-secret-2026".to_string()),
                import_models: true,
                import_custom_templates: true,
                import_mcp: true,
                import_prompts: true,
                import_skills: true,
                import_sessions: true,
            },
        )
        .expect("apply import");
        assert_eq!(imported.models_applied, 2);
        assert_eq!(imported.models_skipped, 0);
        assert_eq!(imported.templates_imported, 1);
        assert_eq!(imported.mcp_imported, 1);
        assert_eq!(imported.prompts_imported, 1);
        assert_eq!(imported.skills_imported, 1);

        // Read back real files & SQLite state on the brand-new PC!
        let codex_cfg = fs::read_to_string(new_home.path().join(".codex/config.toml"))
            .expect("new pc codex config.toml");
        assert!(codex_cfg.contains("https://gateway.corp.example/v1"));
        assert!(codex_cfg.contains("gpt-5.4"));
        assert!(codex_cfg.contains("fs-mcp"));
        assert!(codex_cfg.contains("ghp_e2e_secret_token_999"));
        let codex_auth = fs::read_to_string(new_home.path().join(".codex/auth.json")).ok();
        assert!(
            codex_cfg.contains("sk-old-pc-secret-999")
                || codex_auth
                    .as_deref()
                    .is_some_and(|a| a.contains("sk-old-pc-secret-999")),
            "new pc Codex config or auth must carry migrated API key"
        );
        let omp_yml = fs::read_to_string(new_home.path().join(".omp/agent/models.yml"))
            .expect("new pc omp yml");
        assert!(omp_yml.contains("sk-old-pc-secret-999"));
        assert!(omp_yml.contains("https://gateway.corp.example/v1"));
        assert!(!omp_yml.contains("fyagent-fyagent-"));

        let new_templates = list_custom_gateway_templates(&new_state.db).unwrap();
        assert_eq!(new_templates.len(), 1);
        assert_eq!(new_templates[0].name, "内部聚合网关");

        // MCP restored with decrypted GITHUB_TOKEN AND synced live to ~/.claude.json on fresh PC!
        let new_mcp = McpService::get_all_servers(&new_state).unwrap();
        let restored_mcp = new_mcp.get("fs-mcp").expect("fs-mcp in db");
        assert_eq!(
            restored_mcp.server["env"]["GITHUB_TOKEN"].as_str(),
            Some("ghp_e2e_secret_token_999")
        );
        let claude_json = fs::read_to_string(new_home.path().join(".claude.json"))
            .expect("fresh pc ~/.claude.json written for enabled MCP");
        assert!(claude_json.contains("ghp_e2e_secret_token_999"));

        let new_prompts = PromptService::get_prompts(&new_state, AppType::Claude).unwrap();
        assert!(new_prompts.contains_key("p-main"));

        // Skill restored cleanly (NO `.fyagent.undo.json` sidecar) AND synced into Claude/Codex skill dirs!
        let new_ssot = SkillService::get_ssot_dir().unwrap();
        let restored_skill_dir = new_ssot.join("my-deploy-skill");
        assert_eq!(
            fs::read_to_string(restored_skill_dir.join("SKILL.md")).unwrap(),
            "# Deploy Skill\nRun tests first.\n"
        );
        assert!(!restored_skill_dir.join(".git/config").exists());
        assert!(!restored_skill_dir.join("huge.bin").exists());
        let sidecars: Vec<_> = fs::read_dir(&restored_skill_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".fyagent."))
            .collect();
        assert!(
            sidecars.is_empty(),
            "skill dir must not contain .fyagent undo/backup sidecars"
        );
        assert_eq!(
            fs::read_to_string(
                new_home
                    .path()
                    .join(".claude/skills/my-deploy-skill/SKILL.md")
            )
            .expect("synced to claude skills dir"),
            "# Deploy Skill\nRun tests first.\n"
        );
        assert_eq!(
            fs::read_to_string(
                new_home
                    .path()
                    .join(".codex/skills/my-deploy-skill/SKILL.md")
            )
            .expect("synced to codex skills dir"),
            "# Deploy Skill\nRun tests first.\n"
        );
    }

    // ── Phase 3: Keyless import (without passphrase) saves models as reusable templates ──
    {
        let _keyless_home = IsolatedHomeGuard::new("fyagent-e2e-keyless-");
        let keyless_state = AppState::new(Arc::new(Database::memory().expect("keyless pc db")));
        let res = apply_workspace_pack_import(
            &keyless_state,
            WorkspacePackImportRequest {
                path: pack_path.to_string_lossy().to_string(),
                passphrase: None,
                import_models: true,
                import_custom_templates: true,
                import_mcp: true,
                import_prompts: false,
                import_skills: false,
                import_sessions: false,
            },
        )
        .expect("keyless import");
        assert_eq!(res.models_applied, 0);
        assert_eq!(res.models_skipped, 2);
        // 1 original custom template + 2 auto-preserved keyless model templates = 3!
        assert_eq!(res.templates_imported, 3);
        let saved_templates = list_custom_gateway_templates(&keyless_state.db).unwrap();
        assert_eq!(saved_templates.len(), 3);
        // Without passphrase, MCP env secret must NOT be populated!
        let keyless_mcp = McpService::get_all_servers(&keyless_state).unwrap();
        assert!(keyless_mcp["fs-mcp"].server.get("env").is_none());
    }
}
