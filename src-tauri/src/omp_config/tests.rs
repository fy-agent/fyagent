use super::*;
use serial_test::serial;
use std::fs;

fn paths() -> OmpResolvedPaths {
    resolve_omp_paths().expect("resolve profile")
}

fn example_provider_patch() -> OmpProviderPatch {
    OmpProviderPatch {
        id: "fyagent-example".to_string(),
        base_url: "https://gateway.example.com/v1".to_string(),
        api: "openai-completions".to_string(),
        api_key: Some("FYAGENT_OMP_EXAMPLE_KEY".to_string()),
        auth: Some("apiKey".to_string()),
        models: vec![OmpModelInput {
            id: "model-from-provider".to_string(),
            name: Some("Example model".to_string()),
        }],
    }
}

fn plan_from_read(
    config: &OmpConfigRead,
    provider: OmpProviderPatch,
    default_model: Option<String>,
) -> OmpSavePlan {
    OmpSavePlan {
        provider,
        default_model,
        expected_models_revision: config.models_revision.clone(),
        expected_config_revision: config.config_revision.clone(),
    }
}

fn preview_and_apply(plan: OmpSavePlan) -> OmpSaveResult {
    let preview = preview_omp_save(&plan).expect("preview save");
    apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })
    .expect("apply save")
}

#[test]
#[serial]
fn read_empty_config_when_no_files_exist() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    assert_eq!(config.profile, DEFAULT_PROFILE);
    assert!(config.providers.is_empty());
    assert_eq!(config.default_model, None);
}

#[test]
fn new_collections_preserve_valid_flow_yaml_and_existing_provider_fields() {
    for original in [
        "{}\n",
        "providers: {}\n",
        "providers: {keep: {api: openai-responses}}\n",
        "providers: {fyagent-example: {models: []}}\n",
        "providers:\n  fyagent-example: {models: []}\n",
    ] {
        let output = patch_provider_in_yaml(original, &example_provider_patch()).unwrap();
        let summaries = parse_providers_from_yaml(output.as_bytes()).unwrap();
        let provider = summaries
            .iter()
            .find(|provider| provider.id == "fyagent-example")
            .unwrap();
        assert_eq!(provider.models[0].id, "model-from-provider");
        if original.contains("keep:") {
            assert_eq!(
                summaries
                    .iter()
                    .find(|provider| provider.id == "keep")
                    .unwrap()
                    .api
                    .as_deref(),
                Some("openai-responses")
            );
        }
    }
    for original in ["{}\n", "modelRoles: {}\n", "{other: keep}\n"] {
        let output =
            patch_default_model_in_yaml(original, "fyagent-example/model-from-provider").unwrap();
        assert_eq!(
            parse_default_model_from_bytes(output.as_bytes())
                .unwrap()
                .as_deref(),
            Some("fyagent-example/model-from-provider")
        );
        if original.contains("other:") {
            assert!(output.contains("other: keep"));
        }
    }
}

#[test]
#[serial]
fn read_providers_redacts_secret_fields() {
    let _env = test_support::TestOmpEnv::new();
    let models_path = paths().models_path;
    fs::create_dir_all(models_path.parent().unwrap()).unwrap();
    fs::write(
            &models_path,
            "providers:\n  demo:\n    baseUrl: https://gateway.example.com/v1\n    api: openai-completions\n    apiKey: SECRET_TOKEN\n    headers:\n      Authorization: Bearer also-secret\n    models:\n      - id: org/model\n        name: Demo\n",
        )
        .unwrap();

    let config = read_omp_config().expect("read omp config");
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("SECRET_TOKEN"));
    assert!(!json.contains("Authorization"));
    assert!(!json.contains("also-secret"));
    assert!(config.providers[0].has_api_key);
    assert_eq!(config.providers[0].models[0].id, "org/model");
}

#[test]
#[serial]
fn yaml_precedence_yml_over_yaml_and_legacy_json_preserved() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    fs::write(
        paths.models_path.with_extension("yml"),
        "providers:\n  from-yml:\n    baseUrl: https://yml.test\n",
    )
    .unwrap();
    fs::write(
        paths.models_path.with_extension("yaml"),
        "providers:\n  from-yaml:\n    baseUrl: https://yaml.test\n",
    )
    .unwrap();
    fs::write(paths.legacy_json_path, r#"{"providers":{}}"#).unwrap();

    let config = read_omp_config().expect("read omp config");
    assert_eq!(config.providers[0].id, "from-yml");
    assert!(Path::new(&config.models_path).ends_with("models.yml"));
}

#[test]
#[serial]
fn legacy_json_only_is_detected_and_not_created_over() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    fs::write(&paths.legacy_json_path, r#"{"providers":{}}"#).unwrap();

    let config = read_omp_config().expect("read omp config");
    assert_eq!(config.models_file_status, OmpFileStatus::LegacyJsonOnly);
    let plan = plan_from_read(&config, example_provider_patch(), None);
    assert!(preview_omp_save(&plan).is_err());
    assert!(!paths.models_path.exists());
}

#[test]
#[serial]
fn lossless_patch_preserves_comments_unknowns_quotes_hash_strings_and_other_nodes() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let original = r#"# root comment
unknownRoot: keep
providers:
  # demo comment
  demo:
    baseUrl: "https://old.example.com/v1" # url comment
    api: openai-completions
    apiKey: "!command vault read # not a comment"
    headers:
      Authorization: "Bearer should-stay-secret"
    transport: mystery
    models:
      - id: "org/model-a"
        name: "Old # Name"
        reasoning:
          effort: high
      - id: org/model-b
        name: Keep B
  other:
    baseUrl: https://other.example.com/v1
"#;
    fs::write(&paths.models_path, original).unwrap();
    let config = read_omp_config().expect("read config");
    let plan = plan_from_read(
        &config,
        OmpProviderPatch {
            id: "demo".to_string(),
            base_url: "https://new.example.com/v1".to_string(),
            api: "openai-completions".to_string(),
            api_key: None,
            auth: None,
            models: vec![OmpModelInput {
                id: "org/model-a".to_string(),
                name: Some("New # Name".to_string()),
            }],
        },
        None,
    );

    let preview = preview_omp_save(&plan).expect("preview");
    assert!(!serde_json::to_string(&preview)
        .unwrap()
        .contains("Bearer should-stay-secret"));
    apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })
    .expect("apply");

    let saved = fs::read_to_string(&paths.models_path).unwrap();
    assert!(saved.contains("# root comment"));
    assert!(saved.contains("unknownRoot: keep"));
    assert!(saved.contains("# demo comment"));
    assert!(saved.contains("# url comment"));
    assert!(saved.contains("transport: mystery"));
    assert!(saved.contains("Authorization: \"Bearer should-stay-secret\""));
    assert!(saved.contains("other:"));
    assert!(saved.contains("reasoning:"));
    assert!(saved.contains("org/model-b"));
    assert!(saved.contains("!command vault read # not a comment"));
    assert!(saved.contains("https://new.example.com/v1"));
    assert!(saved.contains("New # Name"));
}

#[test]
#[serial]
fn invalid_yaml_rejected_without_file_changes() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let invalid = "providers: [unterminated";
    fs::write(&paths.models_path, invalid).unwrap();

    assert!(read_omp_config().is_err());
    let plan = OmpSavePlan {
        provider: example_provider_patch(),
        default_model: None,
        expected_models_revision: revision(invalid.as_bytes()),
        expected_config_revision: MISSING_REVISION.to_string(),
    };
    assert!(preview_omp_save(&plan).is_err());
    assert!(fs::read_to_string(&paths.models_path)
        .unwrap()
        .contains("unterminated"));
}

#[test]
#[serial]
fn save_new_provider_with_default_role() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    let result = preview_and_apply(plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    ));
    assert!(result.models_written);
    assert!(result.config_written);
    assert_eq!(result.provider.id, "fyagent-example");
    assert_eq!(
        result.default_model.as_deref(),
        Some("fyagent-example/model-from-provider")
    );
}

#[test]
#[serial]
fn only_provider_save_leaves_config_bytes_unchanged() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let config_bytes = b"modelRoles:\n  default: old/model\n";
    fs::write(&paths.config_path, config_bytes).unwrap();
    let config = read_omp_config().expect("read omp config");

    let result = preview_and_apply(plan_from_read(&config, example_provider_patch(), None));
    assert!(!result.config_written);
    assert_eq!(fs::read(&paths.config_path).unwrap(), config_bytes);
}

#[test]
#[serial]
fn slash_model_id_and_command_key_are_quoted_and_read_correctly() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    let mut patch = example_provider_patch();
    patch.api_key = Some("!command vault read secret/key".to_string());
    patch.models = vec![OmpModelInput {
        id: "org/model/variant".to_string(),
        name: Some("Variant".to_string()),
    }];
    let result = preview_and_apply(plan_from_read(&config, patch, None));
    assert!(result.provider.has_command_key);
    assert_eq!(result.provider.models[0].id, "org/model/variant");
    let saved = fs::read_to_string(paths().models_path).unwrap();
    assert!(
        saved.contains("'!command vault read secret/key'")
            || saved.contains("\"!command vault read secret/key\"")
    );
}

#[test]
#[serial]
fn default_model_update_preserves_other_roles() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    fs::write(&paths.config_path, "modelRoles:\n  default: old-provider/old-model\n  thinking: other/thinking-model\n# User comment\n").unwrap();
    fs::write(&paths.models_path, "providers:\n  fyagent-example:\n    baseUrl: https://gateway.example.com/v1\n    api: openai-completions\n    apiKey: KEY\n    models:\n      - id: model-from-provider\n").unwrap();
    let config = read_omp_config().expect("read omp config");
    let result = preview_and_apply(plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    ));
    assert!(result.config_written);
    let saved = fs::read_to_string(&paths.config_path).unwrap();
    assert!(saved.contains("thinking: other/thinking-model"));
    assert!(saved.contains("fyagent-example/model-from-provider"));
    assert!(saved.contains("# User comment"));
}

#[test]
#[serial]
fn config_conflict_before_write_preserves_models() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let models_original = "providers:\n  other:\n    baseUrl: https://other.example.com\n";
    fs::write(&paths.models_path, models_original).unwrap();
    fs::write(&paths.config_path, "modelRoles:\n  default: old/model\n").unwrap();
    let config = read_omp_config().expect("read omp config");
    fs::write(
        &paths.config_path,
        "modelRoles:\n  default: externally/changed\n",
    )
    .unwrap();

    let err = preview_omp_save(&plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    ))
    .expect_err("conflict");
    assert!(matches!(err, AppError::Conflict(_)));
    assert_eq!(
        fs::read_to_string(&paths.models_path).unwrap(),
        models_original
    );
}

#[test]
#[serial]
fn config_write_failure_restores_models_preimage() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let models_original = "providers:\n  other:\n    baseUrl: https://other.example.com\n";
    fs::write(&paths.models_path, models_original).unwrap();
    let config = read_omp_config().expect("read omp config");
    let plan = plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    );
    let preview = preview_omp_save(&plan).expect("preview");
    test_support::arm_config_write_fault();
    let err = apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })
    .expect_err("config write failure is reported");
    assert!(err.to_string().contains("models.yml was restored"));
    assert_eq!(
        fs::read_to_string(&paths.models_path).unwrap(),
        models_original
    );
}

#[test]
#[serial]
fn config_write_failure_preserves_external_models_edit() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    fs::write(
        &paths.models_path,
        "providers:\n  other:\n    baseUrl: https://other.example.com\n",
    )
    .unwrap();
    let config = read_omp_config().expect("read omp config");
    let plan = plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    );
    let preview = preview_omp_save(&plan).expect("preview");
    let external = b"providers:\n  external:\n    baseUrl: https://external.example.com\n".to_vec();
    test_support::arm_config_write_fault_with_external_models(external.clone());
    let err = apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })
    .expect_err("external rollback conflict is reported");
    assert!(err.to_string().contains("external changes were preserved"));
    assert_eq!(fs::read(&paths.models_path).unwrap(), external);
}

#[test]
#[serial]
fn preview_identity_rejects_profile_switch() {
    let dir = tempfile::tempdir().unwrap();
    let _env = test_support::TestOmpEnv::with_input(dir, None, None, None, None, None);
    let config = read_omp_config().expect("read omp config");
    let plan = plan_from_read(&config, example_provider_patch(), None);
    let preview = preview_omp_save(&plan).expect("preview");
    drop(_env);

    let dir2 = tempfile::tempdir().unwrap();
    let _env2 = test_support::TestOmpEnv::with_input(dir2, None, Some("named"), None, None, None);
    let err = apply_omp_save(&OmpApplyConfirmation {
        plan_id: preview.plan_id,
        plan_digest: preview.plan_digest,
    })
    .expect_err("profile switch rejected");
    assert!(matches!(err, AppError::Conflict(_)));
}

#[test]
#[serial]
fn resolver_uses_config_root_name_and_named_profile_ignores_agent_override() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let input = OmpResolverInput {
        home: home.clone(),
        cwd: home.join("working-directory"),
        cli_profile: None,
        omp_profile: Some("team".to_string()),
        pi_profile: None,
        pi_config_dir: Some("omp-root".to_string()),
        pi_coding_agent_dir: Some(home.join("custom-agent").to_string_lossy().to_string()),
    };
    let paths = resolve_omp_paths_from_input(input).expect("resolve");
    assert_eq!(
        paths.agent_dir,
        home.join("omp-root")
            .join("profiles")
            .join("team")
            .join("agent")
    );
}

#[test]
fn explicit_blank_omp_profile_selects_default_before_legacy_profile() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["", " ", "default"] {
        let input = OmpResolverInput {
            home: dir.path().to_path_buf(),
            cwd: dir.path().join("project"),
            cli_profile: None,
            omp_profile: Some(profile.to_string()),
            pi_profile: Some("work".to_string()),
            pi_config_dir: None,
            pi_coding_agent_dir: None,
        };
        let paths = resolve_omp_paths_from_input(input).unwrap();
        assert_eq!(paths.profile, "default");
        assert_eq!(paths.agent_dir, dir.path().join(".omp/agent"));
    }
}

#[test]
fn relative_agent_override_resolves_from_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().join("project");
    let input = OmpResolverInput {
        home: dir.path().join("home"),
        cwd: cwd.clone(),
        cli_profile: None,
        omp_profile: None,
        pi_profile: None,
        pi_config_dir: None,
        pi_coding_agent_dir: Some("state".to_string()),
    };
    assert_eq!(
        resolve_omp_paths_from_input(input).unwrap().agent_dir,
        cwd.join("state")
    );
}

#[test]
#[serial]
fn saving_explicit_default_profile_preserves_legacy_named_profile() {
    let dir = tempfile::tempdir().unwrap();
    let other_path = dir.path().join(".omp/profiles/work/agent/models.yml");
    fs::create_dir_all(other_path.parent().unwrap()).unwrap();
    fs::write(&other_path, "# other profile\nproviders: {}\n").unwrap();
    let _env = test_support::TestOmpEnv::with_input(dir, None, Some(" "), Some("work"), None, None);
    let config = read_omp_config().unwrap();
    preview_and_apply(plan_from_read(&config, example_provider_patch(), None));
    assert_eq!(
        fs::read_to_string(&other_path).unwrap(),
        "# other profile\nproviders: {}\n"
    );
    assert_eq!(
        read_omp_config().unwrap().providers[0].id,
        "fyagent-example"
    );
}

#[test]
#[serial]
fn preview_rejects_oversized_output_before_any_write_or_backup() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(&paths.agent_dir).unwrap();
    let original = format!(
        "# {}\nproviders: {{}}\n",
        "x".repeat(MAX_OMP_FILE_BYTES as usize - 80)
    );
    fs::write(&paths.models_path, &original).unwrap();
    let config = read_omp_config().unwrap();
    let error =
        preview_omp_save(&plan_from_read(&config, example_provider_patch(), None)).unwrap_err();
    assert!(error.to_string().contains("1 MiB"), "{error}");
    assert_eq!(fs::read_to_string(&paths.models_path).unwrap(), original);
    assert_eq!(fs::read_dir(&paths.agent_dir).unwrap().count(), 1);
}

#[test]
#[serial]
fn preview_rejects_oversized_default_role_before_writing_models() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(&paths.agent_dir).unwrap();
    let original = format!(
        "# {}\nmodelRoles: {{}}\n",
        "x".repeat(MAX_OMP_FILE_BYTES as usize - 40)
    );
    fs::write(&paths.config_path, &original).unwrap();
    let config = read_omp_config().unwrap();
    let error = preview_omp_save(&plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    ))
    .unwrap_err();
    assert!(error.to_string().contains("1 MiB"), "{error}");
    assert!(!paths.models_path.exists());
    assert_eq!(fs::read_to_string(&paths.config_path).unwrap(), original);
    assert_eq!(fs::read_dir(&paths.agent_dir).unwrap().count(), 1);
}

#[test]
#[serial]
fn public_revisions_are_hmac_not_plain_sha256() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    let bytes = b"providers: {}\n";
    fs::write(&paths.models_path, bytes).unwrap();

    let config = read_omp_config().expect("read omp config");
    assert_ne!(config.models_revision, content_hash(bytes));
}

#[test]
#[serial]
fn new_api_key_provider_requires_key_unless_auth_allows_empty() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    let mut patch = example_provider_patch();
    patch.api_key = None;
    patch.auth = None;
    let err = preview_omp_save(&plan_from_read(&config, patch.clone(), None))
        .expect_err("missing api key rejected");
    assert!(err.to_string().contains("requires a non-empty apiKey"));

    patch.auth = Some("none".to_string());
    preview_omp_save(&plan_from_read(&config, patch, None)).expect("auth none may omit key");
}

#[test]
#[serial]
fn cst_patch_rejects_named_nodes_with_wrong_type() {
    let _env = test_support::TestOmpEnv::new();
    let paths = paths();
    fs::create_dir_all(paths.agent_dir).unwrap();
    fs::write(&paths.models_path, "providers: []\n").unwrap();
    let config = read_omp_config().expect("read omp config");
    let err = preview_omp_save(&plan_from_read(&config, example_provider_patch(), None))
        .expect_err("providers sequence rejected");
    assert!(err.to_string().contains("providers node must be a mapping"));

    fs::write(&paths.models_path, "providers: {}\n").unwrap();
    fs::write(&paths.config_path, "modelRoles: []\n").unwrap();
    let config = read_omp_config().expect("read omp config");
    let err = preview_omp_save(&plan_from_read(
        &config,
        example_provider_patch(),
        Some("fyagent-example/model-from-provider".to_string()),
    ))
    .expect_err("modelRoles sequence rejected");
    assert!(
        err.to_string()
            .contains("modelRoles node must be a mapping"),
        "{err}"
    );
}

#[test]
fn omp_version_parser_accepts_official_slash_output() {
    assert_eq!(
        fyagent_user_helper::omp::parse_omp_version_output("omp/18.12.5\n").as_deref(),
        Some("18.12.5")
    );
    assert_eq!(
        fyagent_user_helper::omp::parse_omp_version_output("omp v18.12.5\n").as_deref(),
        Some("18.12.5")
    );
}

#[test]
fn omp_candidate_resolver_prioritizes_windows_install_path_and_bun_entry() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().to_path_buf();
    let local = home.join("AppData/Local");
    let path_dir = home.join("Tools");
    let candidates = fyagent_user_helper::omp::omp_candidates(
        Some(&local),
        Some(&home),
        vec![path_dir.clone()],
        true,
    );
    assert_eq!(candidates[0], local.join("omp").join("omp.exe"));
    assert!(candidates.contains(&path_dir.join("omp.exe")));
    assert!(candidates.contains(&home.join(".bun").join("bin").join("omp.cmd")));
}

#[test]
#[serial]
fn production_preview_apply_generates_official_cli_fixture() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    let result = preview_and_apply(plan_from_read(
        &config,
        OmpProviderPatch {
            id: "fyagent-issue377-fixture".to_string(),
            base_url: "http://127.0.0.1:1/v1".to_string(),
            api: "openai-completions".to_string(),
            api_key: Some("test-only-not-a-secret".to_string()),
            auth: Some("apiKey".to_string()),
            models: vec![OmpModelInput {
                id: "issue377-fixture".to_string(),
                name: Some("Issue 377 fixture".to_string()),
            }],
        },
        Some("fyagent-issue377-fixture/issue377-fixture".to_string()),
    ));
    assert_eq!(result.provider.id, "fyagent-issue377-fixture");
    assert_eq!(
        result.default_model.as_deref(),
        Some("fyagent-issue377-fixture/issue377-fixture")
    );

    let paths = paths();
    let models = fs::read_to_string(&paths.models_path).unwrap();
    let config = fs::read_to_string(&paths.config_path).unwrap();
    assert!(models.contains("fyagent-issue377-fixture"));
    assert!(models.contains("http://127.0.0.1:1/v1"));
    assert!(config.contains("fyagent-issue377-fixture/issue377-fixture"));

    if let Some(proof_dir) = std::env::var_os("FYAGENT_OMP_PROOF_DIR") {
        let proof_dir = PathBuf::from(proof_dir);
        fs::create_dir_all(&proof_dir).unwrap();
        assert!(
            fs::read_dir(&proof_dir).unwrap().next().is_none(),
            "FYAGENT_OMP_PROOF_DIR must be empty before proof files are copied"
        );
        fs::copy(&paths.models_path, proof_dir.join("models.yml")).unwrap();
        fs::copy(&paths.config_path, proof_dir.join("config.yml")).unwrap();
    }
}

#[test]
#[serial]
fn invalid_public_dto_is_rejected() {
    let _env = test_support::TestOmpEnv::new();
    let config = read_omp_config().expect("read omp config");
    let mut patch = example_provider_patch();
    patch.id = "bad/id".to_string();
    let err = preview_omp_save(&plan_from_read(&config, patch, None)).expect_err("invalid id");
    assert!(matches!(err, AppError::InvalidInput(_)));
}
