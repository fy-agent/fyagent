use serde_json::json;

use fyagent_lib::{
    get_claude_settings_path, read_json_file, write_codex_live_atomic, AppError, AppType, McpApps,
    McpServer, MultiAppConfig, Provider, ProviderMeta, ProviderService,
};

#[path = "support.rs"]
mod support;
use support::{
    create_credential_test_state as create_test_state,
    create_credential_test_state_with_config as create_test_state_with_config,
    enable_codex_official_auth_preservation, ensure_test_home, reset_test_fs, test_mutex,
};

fn sanitize_provider_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

#[test]
fn migrate_legacy_common_config_usage_marks_historical_provider_enabled() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "legacy-provider".to_string();
        manager.providers.insert(
            "legacy-provider".to_string(),
            Provider::with_id(
                "legacy-provider".to_string(),
                "Legacy".to_string(),
                json!({
                    "includeCoAuthoredBy": false,
                    "env": {
                        "ANTHROPIC_API_KEY": "legacy-key"
                    }
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{ "includeCoAuthoredBy": false }"#.to_string()),
        )
        .expect("set common config snippet");

    ProviderService::migrate_legacy_common_config_usage_if_needed(&state, AppType::Claude)
        .expect("migrate legacy common config");

    let providers = state
        .db
        .get_all_providers(AppType::Claude.as_str())
        .expect("get providers after migration");
    let provider = providers
        .get("legacy-provider")
        .expect("legacy provider exists");

    assert_eq!(
        provider
            .meta
            .as_ref()
            .and_then(|meta| meta.common_config_enabled),
        Some(true),
        "historical provider should be explicitly marked as using common config"
    );
    assert!(
        provider
            .settings_config
            .get("includeCoAuthoredBy")
            .is_none(),
        "common config fields should be stripped from provider storage after migration"
    );
    assert_eq!(
        provider
            .settings_config
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_API_KEY"))
            .and_then(|v| v.as_str()),
        Some("legacy-key"),
        "provider-specific auth should remain untouched"
    );
}

#[test]
fn first_run_codex_import_switch_away_and_back_preserves_model_reasoning_effort() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let live_config = r#"model_provider = "relay"
model = "gpt-5-codex"
model_reasoning_effort = "high"

[model_providers.relay]
name = "relay"
base_url = "https://relay.example/v1"
wire_api = "responses"

[projects."/Users/me/repo"]
trust_level = "trusted"
"#;
    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-live" }), Some(live_config))
        .expect("seed first-run Codex live config");

    let state = create_test_state().expect("create test state");
    // 与首启行快照走同一入口，包含 common snippet 提取和行内共享字段剥离。
    assert!(
        fyagent_lib::import_default_config_test_hook(&state, AppType::Codex)
            .expect("import first-run Codex live config"),
        "first-run import should create the default provider"
    );
    state
        .db
        .init_default_official_providers()
        .expect("seed official providers after first-run import");
    let other = Provider::with_id(
        "other-codex".to_string(),
        "Other Codex".to_string(),
        json!({
            "auth": { "OPENAI_API_KEY": "sk-other" },
            "config": r#"model_provider = "other"
model = "gpt-other"
model_reasoning_effort = "low"

[model_providers.other]
name = "other"
base_url = "https://other.example/v1"
wire_api = "responses"
"#
        }),
        None,
    );
    state
        .db
        .save_provider(AppType::Codex.as_str(), &other)
        .expect("save other Codex provider");

    ProviderService::switch(&state, AppType::Codex, &other.id).expect("switch away from default");
    let config_text =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    let config: toml_edit::DocumentMut = config_text.parse().expect("parse config.toml");
    assert_eq!(config["model"].as_str(), Some("gpt-other"));
    assert_eq!(config["model_reasoning_effort"].as_str(), Some("low"));

    ProviderService::switch(&state, AppType::Codex, "default")
        .expect("switch back to imported default");
    let config_text = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switching back");
    let config: toml_edit::DocumentMut = config_text.parse().expect("parse restored config.toml");
    assert_eq!(config["model"].as_str(), Some("gpt-5-codex"));
    assert_eq!(
        config
            .get("model_reasoning_effort")
            .and_then(|item| item.as_str()),
        Some("high"),
        "imported reasoning effort must be written back to config.toml, got: {config_text}"
    );

    let stored = state
        .db
        .get_provider_by_id("default", AppType::Codex.as_str())
        .expect("query imported default")
        .expect("imported default exists");
    let row_config = stored
        .settings_config
        .get("config")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let row_doc: toml_edit::DocumentMut = row_config.parse().expect("parse stored Codex config");
    assert_eq!(
        row_doc
            .get("model_reasoning_effort")
            .and_then(|item| item.as_str()),
        Some("high"),
        "reasoning effort must stay on the imported row, got: {row_config}"
    );
    let snippet = state
        .db
        .get_config_snippet(AppType::Codex.as_str())
        .expect("read Codex common snippet")
        .unwrap_or_default();
    assert!(
        !snippet.contains("model_reasoning_effort"),
        "reasoning effort must stay out of the common snippet, got: {snippet}"
    );
}

#[test]
fn provider_service_switch_codex_updates_live_and_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let legacy_auth = json!({ "OPENAI_API_KEY": "legacy-key" });
    let legacy_config = r#"model_provider = "legacy"
model = "fixture-model"
[model_providers.legacy]
name = "Legacy"
base_url = "https://legacy.example.invalid/v1"
wire_api = "responses"
[mcp_servers.legacy]
type = "stdio"
command = "echo"
"#;
    write_codex_live_atomic(&legacy_auth, Some(legacy_config))
        .expect("seed existing codex live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();
        manager.providers.insert(
            "old-provider".to_string(),
            Provider::with_id(
                "old-provider".to_string(),
                "Legacy".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "stale"},
                    "config": "stale-config"
                }),
                None,
            ),
        );
        manager.providers.insert(
            "new-provider".to_string(),
            Provider::with_id(
                "new-provider".to_string(),
                "Latest".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "fresh-key"},
                    "config": r#"model_provider = "latest"
model = "fixture-model"
[model_providers.latest]
name = "Latest"
base_url = "https://latest.example.invalid/v1"
wire_api = "responses"
[mcp_servers.latest]
type = "stdio"
command = "say"
"#
                }),
                None,
            ),
        );
    }

    // 使用新的统一 MCP 结构（v3.7.0+）
    let servers = initial_config
        .mcp
        .servers
        .get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    fyagent_lib::McpService::sync_enabled_for_app(&state, &AppType::Codex)
        .expect("seed MCP independently of source switching");

    ProviderService::switch(&state, AppType::Codex, "new-provider")
        .expect("switch provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value.get("OPENAI_API_KEY").and_then(|v| v.as_str()),
        Some("legacy-key"),
        "Codex provider switching should preserve the existing live auth.json"
    );

    let config_text =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        config_text.contains("mcp_servers.echo-server"),
        "config.toml should contain synced MCP servers"
    );
    assert!(
        config_text.contains("experimental_bearer_token"),
        "config.toml should carry the selected provider API key"
    );

    let current_id = state
        .db
        .get_current_provider(AppType::Codex.as_str())
        .expect("read current provider after switch");
    assert_eq!(
        current_id.as_deref(),
        Some("new-provider"),
        "current provider updated"
    );

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switch");

    let new_provider = providers.get("new-provider").expect("new provider exists");
    let new_config_text = new_provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // provider 存储的是原始配置，不包含 MCP 同步后的内容
    assert!(
        new_config_text.contains("mcp_servers.latest"),
        "provider config should contain original MCP servers"
    );
    // live 文件额外包含同步的 MCP 服务器
    assert!(
        config_text.contains("mcp_servers.echo-server"),
        "live config should include synced MCP servers"
    );

    let legacy = providers
        .get("old-provider")
        .expect("legacy provider still exists");
    assert_eq!(
        legacy.settings_config,
        initial_config
            .get_manager(&AppType::Codex)
            .unwrap()
            .providers["old-provider"]
            .settings_config,
        "switching must not backfill or migrate the previous saved source"
    );
    let before = std::fs::read(fyagent_lib::get_codex_config_path()).unwrap();
    ProviderService::switch(&state, AppType::Codex, "old-provider")
        .expect_err("the unchanged malformed saved source must still be rejected");
    assert_eq!(
        std::fs::read(fyagent_lib::get_codex_config_path()).unwrap(),
        before
    );
    assert_eq!(
        state.db.get_current_provider("codex").unwrap().as_deref(),
        Some("new-provider")
    );
}

#[test]
fn provider_service_switch_codex_preserves_user_model_provider_id_after_migration() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let legacy_auth = json!({ "OPENAI_API_KEY": "rightcode-key" });
    let legacy_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&legacy_auth, Some(legacy_config))
        .expect("seed existing codex live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();
        manager.providers.insert(
            "old-provider".to_string(),
            Provider::with_id(
                "old-provider".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "stale"},
                    "config": legacy_config
                }),
                None,
            ),
        );
        manager.providers.insert(
            "new-provider".to_string(),
            Provider::with_id(
                "new-provider".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "fresh-key"},
                    "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "new-provider")
        .expect("switch provider should succeed");

    let config_text =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    let parsed: toml::Value = toml::from_str(&config_text).expect("parse config.toml");

    assert_eq!(
        parsed.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "4.0.4 projects third-party routes into the custom table"
    );

    let model_providers = parsed
        .get("model_providers")
        .and_then(|v| v.as_table())
        .expect("model_providers table exists");
    assert!(
        model_providers.get("aihubmix").is_none(),
        "the retired source table must not survive normalization"
    );
    assert_eq!(
        model_providers
            .get("custom")
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str()),
        Some("https://aihubmix.example/v1"),
        "selected provider id should point at the newly selected supplier endpoint"
    );

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switch");
    let new_config_text = providers
        .get("new-provider")
        .expect("new provider exists")
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        new_config_text.contains("[model_providers.aihubmix]"),
        "stored provider template should remain provider-specific"
    );
}

#[test]
fn provider_service_switch_codex_preserves_oauth_and_backfills_api_key_from_live_token() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "oauth-token",
            "account_id": "acct-1"
        }
    });
    let legacy_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&live_auth, Some(legacy_config))
        .expect("seed existing Codex OAuth live config");

    let bridge_provider = Provider::with_id(
        "bridge-provider".to_string(),
        "Bridge Provider".to_string(),
        json!({
            "auth": {"OPENAI_API_KEY": "bridge-key"},
            "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "legacy-provider".to_string();
        manager.providers.insert(
            "legacy-provider".to_string(),
            Provider::with_id(
                "legacy-provider".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "rightcode-key"},
                    "config": legacy_config
                }),
                None,
            ),
        );
        manager
            .providers
            .insert("bridge-provider".to_string(), bridge_provider);
        manager.providers.insert(
            "plain-provider".to_string(),
            Provider::with_id(
                "plain-provider".to_string(),
                "Plain Provider".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "plain-key"},
                    "config": r#"model_provider = "plain"
model = "gpt-5.4"

[model_providers.plain]
name = "Plain"
base_url = "https://plain.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");
    let saved = state
        .db
        .get_provider_by_id("bridge-provider", "codex")
        .unwrap()
        .unwrap();
    state
        .db
        .save_provider("codex", &saved)
        .expect("admit fixture credential before selecting a source");

    ProviderService::switch(&state, AppType::Codex, "bridge-provider")
        .expect("switch to bridge provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value.get("auth_mode").and_then(|v| v.as_str()),
        Some("chatgpt")
    );
    assert!(
        auth_value
            .get("OPENAI_API_KEY")
            .is_some_and(|v| v.is_null()),
        "provider switching should keep OPENAI_API_KEY null in live auth.json"
    );
    assert_eq!(
        auth_value
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("oauth-token"),
        "existing ChatGPT OAuth token should be preserved"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    let parsed_live: toml::Value = toml::from_str(&live_config).expect("parse live config");
    assert_eq!(
        parsed_live
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("experimental_bearer_token"))
            .and_then(|v| v.as_str()),
        Some("bridge-key"),
        "third-party key should be injected into the selected live provider table"
    );
    assert_eq!(
        parsed_live
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("requires_openai_auth"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    ProviderService::switch(&state, AppType::Codex, "plain-provider")
        .expect("switch away should backfill bridge provider");

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers");
    let stored_bridge = providers
        .get("bridge-provider")
        .expect("bridge provider exists after backfill");
    assert_eq!(
        stored_bridge
            .settings_config
            .pointer("/auth/OPENAI_API_KEY")
            .and_then(|v| v.as_str()),
        None,
        "source selection retains the native credential reference"
    );
    assert!(
        stored_bridge
            .settings_config
            .pointer("/auth/tokens")
            .is_none(),
        "backfill should not persist ChatGPT OAuth tokens into provider storage"
    );
    assert!(stored_bridge.settings_config.get("credentialRef").is_some());
    assert!(!stored_bridge
        .settings_config
        .to_string()
        .contains("bridge-key"));
    ProviderService::switch(&state, AppType::Codex, "bridge-provider")
        .expect("resolve migrated key on switch back");
    assert!(
        std::fs::read_to_string(fyagent_lib::get_codex_config_path())
            .unwrap()
            .contains("bridge-key")
    );
    assert!(
        !stored_bridge
            .settings_config
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("experimental_bearer_token"),
        "stored provider config should stay clean; bridge token is generated only for live config"
    );
}

#[tokio::test(flavor = "current_thread")]
#[allow(
    clippy::await_holding_lock,
    reason = "this integration-style test must serialize global test HOME and settings mutations across async takeover calls"
)]
async fn codex_official_to_deepseek_then_takeover_enters_and_restores_proxy_managed_live_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let oauth_auth = json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "access_token": "oauth-access",
            "id_token": "oauth-id"
        }
    });
    let official_config = r#"model_provider = "openai"
model = "gpt-5"

[model_providers.openai]
name = "OpenAI"
wire_api = "responses"
"#;
    write_codex_live_atomic(&oauth_auth, Some(official_config))
        .expect("seed official Codex OAuth live config");

    let deepseek_provider_config = r#"model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
"#;
    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "official-provider".to_string();

        let mut official_provider = Provider::with_id(
            "official-provider".to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": oauth_auth,
                "config": official_config
            }),
            None,
        );
        official_provider.category = Some("official".to_string());
        manager
            .providers
            .insert("official-provider".to_string(), official_provider);

        let mut deepseek_provider = Provider::with_id(
            "deepseek-provider".to_string(),
            "DeepSeek".to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": "deepseek-key"},
                "config": deepseek_provider_config
            }),
            None,
        );
        deepseek_provider.category = Some("custom".to_string());
        manager
            .providers
            .insert("deepseek-provider".to_string(), deepseek_provider);
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    let mut proxy_config = state.db.get_proxy_config().await.expect("get proxy config");
    proxy_config.listen_port = 0;
    state
        .db
        .update_proxy_config(proxy_config)
        .await
        .expect("use ephemeral proxy port");

    ProviderService::switch(&state, AppType::Codex, "deepseek-provider")
        .expect("switch from official subscription to DeepSeek");

    let auth_after_switch: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth after switch");
    assert_eq!(
        auth_after_switch, oauth_auth,
        "normal provider switch with Codex preservation enabled must keep OAuth auth.json"
    );

    let config_after_switch =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config");
    assert!(
        config_after_switch.contains("https://api.deepseek.com/v1"),
        "normal switch should write the DeepSeek endpoint before takeover"
    );
    assert!(
        config_after_switch.contains("deepseek-key"),
        "normal switch should inject the DeepSeek key into config.toml"
    );

    state
        .proxy_service
        .set_takeover_for_app("codex", true)
        .await
        .expect("enable Codex takeover");
    let proxy_status = state
        .proxy_service
        .get_status()
        .await
        .expect("read proxy status after takeover");
    let codex_proxy_base_url = format!("http://127.0.0.1:{}/v1", proxy_status.port);

    let auth_after_takeover: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth after takeover");
    assert_eq!(
        auth_after_takeover, oauth_auth,
        "enabling takeover must not rewrite Codex OAuth auth.json"
    );

    let config_after_takeover =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config");
    assert!(
        config_after_takeover.contains(&codex_proxy_base_url),
        "enabling takeover should point Codex config.toml at the local proxy"
    );
    assert!(
        config_after_takeover.contains("PROXY_MANAGED"),
        "enabling takeover should move the proxy placeholder into config.toml"
    );
    assert!(
        !config_after_takeover.contains("https://api.deepseek.com/v1"),
        "takeover live config should not keep the upstream DeepSeek endpoint"
    );

    let backup = state
        .db
        .get_live_backup("codex")
        .await
        .expect("read Codex backup")
        .expect("backup exists after takeover");
    let backup_value: serde_json::Value =
        serde_json::from_str(&backup.original_config).expect("parse backup");
    let backup_config = backup_value
        .get("config")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert!(
        backup_config.contains("https://api.deepseek.com/v1")
            && backup_config.contains("deepseek-key"),
        "takeover backup should remain the restorable DeepSeek config"
    );

    state
        .proxy_service
        .set_takeover_for_app("codex", false)
        .await
        .expect("disable Codex takeover");

    let restored_auth: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read restored auth");
    assert_eq!(
        restored_auth, oauth_auth,
        "disabling takeover should restore without replacing OAuth auth.json"
    );

    let restored_config = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read restored config");
    assert!(
        restored_config.contains("https://api.deepseek.com/v1")
            && restored_config.contains("deepseek-key"),
        "disabling takeover should restore the selected DeepSeek live config"
    );
    assert!(
        !restored_config.contains("PROXY_MANAGED"),
        "restored live config must not keep the proxy placeholder"
    );
}

#[test]
fn provider_service_switch_codex_default_preserves_official_auth() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    // Official ChatGPT login is a hard invariant: third-party switches are
    // config-only even when the legacy preserve field is absent/false.
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "official-oauth-token",
            "account_id": "acct-1"
        }
    });
    let legacy_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&live_auth, Some(legacy_config))
        .expect("seed existing Codex OAuth live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "legacy-provider".to_string();
        manager.providers.insert(
            "legacy-provider".to_string(),
            Provider::with_id(
                "legacy-provider".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "rightcode-key"},
                    "config": legacy_config
                }),
                None,
            ),
        );
        manager.providers.insert(
            "third-party".to_string(),
            Provider::with_id(
                "third-party".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "third-party-key"},
                    "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "third-party")
        .expect("switch to third-party provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value, live_auth,
        "third-party switches must keep the official ChatGPT login in auth.json"
    );
    assert_eq!(
        auth_value
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("official-oauth-token"),
        "official OAuth tokens must survive a third-party provider switch"
    );
    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        live_config.contains("experimental_bearer_token = \"third-party-key\""),
        "the third-party key must be injected as the provider-scoped bearer token; got:\n{live_config}"
    );
}

#[test]
fn provider_service_switch_codex_default_injects_bearer_token_into_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    // Preservation stays OFF (default). Since Codex 0.149 (openai/codex#39214)
    // custom providers no longer inherit ambient auth, so third-party switches
    // are config-only on every path: the key travels as a provider-scoped
    // `experimental_bearer_token` and auth.json is removed.
    let _home = ensure_test_home();

    let third_party_config = r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = false
"#;

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "third-party".to_string(),
            Provider::with_id(
                "third-party".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "third-party-key"},
                    "config": third_party_config
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "third-party")
        .expect("switch to third-party provider should succeed");

    assert!(
        !fyagent_lib::get_codex_auth_path().exists(),
        "third-party switches are config-only: no auth.json is written"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        live_config.contains("experimental_bearer_token = \"third-party-key\""),
        "default switch must inject the API key into config.toml so Codex >= 0.149 \
         custom providers authenticate (openai/codex#39214); got:\n{live_config}"
    );
}

#[test]
fn provider_service_switch_codex_preserved_login_rejects_empty_third_party_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    // Preservation ON + third-party provider with an empty config: auth.json is
    // not written, and an empty config.toml has no provider table to carry the
    // bearer token, so the API key has nowhere to land while the official
    // OAuth login stays live — Codex would silently fall back to the official
    // provider and bill the ChatGPT account. The switch must be refused, as it
    // was before the bearer-token injection change.
    let _home = ensure_test_home();
    enable_codex_official_auth_preservation();

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "empty-config".to_string(),
            Provider::with_id(
                "empty-config".to_string(),
                "EmptyConfig".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "third-party-key"},
                    "config": ""
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    let err = ProviderService::switch(&state, AppType::Codex, "empty-config").expect_err(
        "switching to an empty-config third-party provider with preservation on must fail",
    );
    assert!(
        err.to_string().contains("config.toml"),
        "error should explain the missing config.toml, got: {err}"
    );
}

#[test]
fn provider_service_switch_codex_preserved_login_normalizes_legacy_reroute_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    // Preservation ON + a legacy-shape third-party config (top-level
    // openai_base_url rerouting the built-in `openai` provider): the shape
    // has no provider table to carry the bearer token — since 0.149 the
    // built-in provider would keep using the preserved official OAuth from
    // auth.json and send it to the third-party base URL. The switch must
    // normalize the config into a cc-switch-owned custom table with the key
    // injected, leaving the official login untouched.
    let _home = ensure_test_home();
    enable_codex_official_auth_preservation();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "official-oauth-token",
            "account_id": "acct-1"
        }
    });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed official OAuth live config");

    let legacy_shape_config = r#"model_provider = "openai"
model = "gpt-5.4"
openai_base_url = "https://relay.example/v1"
"#;

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "legacy-shape".to_string(),
            Provider::with_id(
                "legacy-shape".to_string(),
                "LegacyShape".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "third-party-key"},
                    "config": legacy_shape_config
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "legacy-shape")
        .expect("legacy reroute shape must be normalized, not rejected");

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        !live_config.contains("openai_base_url"),
        "the top-level reroute must be rewritten away; got:\n{live_config}"
    );
    assert!(
        live_config.contains("[model_providers.custom]")
            && live_config.contains("base_url = \"https://relay.example/v1\"")
            && live_config.contains("experimental_bearer_token = \"third-party-key\""),
        "routing and key must move into the custom provider table; got:\n{live_config}"
    );

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value, live_auth,
        "third-party switches must keep the official ChatGPT login in auth.json"
    );
    assert_eq!(
        auth_value
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("official-oauth-token"),
        "official OAuth tokens must survive a third-party provider switch"
    );
}

#[test]
fn provider_service_switch_codex_projects_bearer_token_when_openai_auth_disabled() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let image_config = r#"model_provider = "custom"
model = "gpt-5.4"

[model_providers.custom]
name = "Gateway"
base_url = "https://gateway.example/v1"
wire_api = "responses"
requires_openai_auth = false
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
"#;
    write_codex_live_atomic(
        &json!({ "OPENAI_API_KEY": "stale-key" }),
        Some("model = \"stale\"\n"),
    )
    .expect("seed live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "image-provider".to_string(),
            Provider::with_id(
                "image-provider".to_string(),
                "Gateway".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "image-mode-key"},
                    "config": image_config
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "image-provider")
        .expect("switch to image-mode provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value.get("OPENAI_API_KEY").and_then(|v| v.as_str()),
        Some("stale-key"),
        "image mode must not overwrite live auth.json"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        live_config.contains("experimental_bearer_token = \"image-mode-key\""),
        "requires_openai_auth=false must project the API key onto experimental_bearer_token, got:\n{live_config}"
    );
}

#[test]
fn provider_service_switch_codex_supports_official_login_provider_without_auth_write() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "official-oauth-token",
            "account_id": "acct-official"
        }
    });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed official OAuth live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "legacy-provider".to_string();
        manager.providers.insert(
            "legacy-provider".to_string(),
            Provider::with_id(
                "legacy-provider".to_string(),
                "Legacy".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "legacy-key"},
                    "config": r#"model_provider = "legacy"

[model_providers.legacy]
name = "Legacy"
base_url = "https://legacy.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
        let official_provider = Provider::with_id(
            "codex-official".to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": {},
                "config": ""
            }),
            None,
        );
        manager
            .providers
            .insert("codex-official".to_string(), official_provider);
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "codex-official")
        .expect("switch to official provider should succeed without API key");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value.get("auth_mode").and_then(|v| v.as_str()),
        Some("chatgpt")
    );
    assert!(
        auth_value
            .get("OPENAI_API_KEY")
            .is_some_and(|v| v.is_null()),
        "official provider switching should keep OPENAI_API_KEY null"
    );
    assert_eq!(
        auth_value
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("official-oauth-token"),
        "official provider should preserve the existing ChatGPT OAuth token"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        !live_config.contains("experimental_bearer_token"),
        "official login provider has no API key to inject"
    );
}

#[test]
fn provider_service_switch_codex_official_clears_stale_third_party_auth() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    // preservation stays OFF (default): switching to the third-party provider
    // wrote its key into live auth.json, and that residue is what this test
    // expects the official switch to clean up.
    let _home = ensure_test_home();

    let third_party_config = r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    // Live key intentionally differs from the DB row so the assertion below
    // proves the backfill preserved the live copy before it was deleted.
    let live_auth = json!({ "OPENAI_API_KEY": "stale-live-key" });
    write_codex_live_atomic(&live_auth, Some(third_party_config))
        .expect("seed third-party live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "third-party".to_string();
        manager.providers.insert(
            "third-party".to_string(),
            Provider::with_id(
                "third-party".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "old-db-key"},
                    "config": third_party_config
                }),
                None,
            ),
        );
        let mut official_provider = Provider::with_id(
            "official-provider".to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": {},
                "config": ""
            }),
            None,
        );
        official_provider.category = Some("official".to_string());
        manager
            .providers
            .insert("official-provider".to_string(), official_provider);
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");
    let saved = state
        .db
        .get_provider_by_id("third-party", "codex")
        .unwrap()
        .unwrap();
    state
        .db
        .save_provider("codex", &saved)
        .expect("admit fixture credential before selecting a source");

    ProviderService::switch(&state, AppType::Codex, "official-provider")
        .expect("switch to official provider should succeed");

    assert!(
        fyagent_lib::get_codex_auth_path().exists(),
        "source selection preserves native auth.json; explicit account management owns login changes"
    );

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switch");
    let outgoing = providers
        .get("third-party")
        .expect("third-party provider exists");
    assert!(outgoing
        .settings_config
        .pointer("/auth/OPENAI_API_KEY")
        .is_none());
    assert!(outgoing.settings_config["credentialRef"].as_str().is_some());
    assert!(!serde_json::to_string(outgoing)
        .unwrap()
        .contains("stale-live-key"));

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        !live_config.contains("experimental_bearer_token"),
        "official provider has no API key to inject"
    );
    ProviderService::switch(&state, AppType::Codex, "third-party")
        .expect("restore outgoing source from native credentials");
    let restored = std::fs::read_to_string(fyagent_lib::get_codex_config_path()).unwrap();
    assert!(restored.contains("old-db-key"));
}

#[test]
fn provider_service_reswitch_current_official_keeps_live_auth() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    // Re-selecting the already-current provider performs no backfill, so the
    // cleanup must not run either: without a fresh DB copy of whatever sits
    // in live auth.json, deleting it would destroy the only copy.
    let live_auth = json!({ "OPENAI_API_KEY": "residue-key" });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "official-provider".to_string();
        let mut official_provider = Provider::with_id(
            "official-provider".to_string(),
            "OpenAI Official".to_string(),
            json!({
                "auth": {},
                "config": ""
            }),
            None,
        );
        official_provider.category = Some("official".to_string());
        manager
            .providers
            .insert("official-provider".to_string(), official_provider);
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "official-provider")
        .expect("re-switch to current official provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("auth.json must survive");
    assert_eq!(
        auth_value.get("OPENAI_API_KEY").and_then(|v| v.as_str()),
        Some("residue-key"),
        "no backfill happened, so live auth.json must be left untouched"
    );
}

#[test]
fn read_codex_live_settings_tolerates_missing_auth_when_config_file_exists() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    assert!(
        fyagent_lib::read_codex_live_settings().is_err(),
        "both files missing is still 'no live install'"
    );

    // auth.json deleted + empty config.toml is the exact state the official
    // switch cleanup leaves behind; it must stay readable or the next
    // backfill / hot switch would treat Codex as uninstalled.
    let config_path = fyagent_lib::get_codex_config_path();
    std::fs::create_dir_all(config_path.parent().expect("codex dir")).expect("create codex dir");
    std::fs::write(&config_path, "").expect("write empty config.toml");

    let live = fyagent_lib::read_codex_live_settings()
        .expect("config file present but empty must be readable");
    assert_eq!(live.get("auth"), Some(&json!({})));
    assert_eq!(live.get("config").and_then(|v| v.as_str()), Some(""));
}

#[test]
fn reapply_codex_official_live_resyncs_mcp_servers() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
    });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed official live auth");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        let official = Provider::with_id(
            "codex-official".to_string(),
            "Official".to_string(),
            json!({
                "auth": {
                    "auth_mode": "chatgpt",
                    "OPENAI_API_KEY": null,
                    "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
                },
                "config": ""
            }),
            None,
        );
        manager
            .providers
            .insert("codex-official".to_string(), official);
    }
    let servers = initial_config
        .mcp
        .servers
        .get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    fyagent_lib::McpService::sync_enabled_for_app(&state, &AppType::Codex)
        .expect("seed MCP independently of source switching");

    ProviderService::switch(&state, AppType::Codex, "codex-official")
        .expect("switch to official provider");
    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "switch should preserve existing MCP servers"
    );

    // 统一会话开关变更触发的 reapply 会整体重写 live config.toml（有意设计），
    // 写完必须重新投影 DB 里启用的 MCP，否则用户的 MCP 会静默失效。
    let reapplied =
        fyagent_lib::reapply_current_codex_official_live(&state).expect("reapply official live");
    assert!(
        reapplied,
        "current provider is official, reapply should run"
    );

    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after reapply");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "reapply must re-project enabled MCP servers after the full live rewrite, got: {live}"
    );
}

/// reapply 走到 MCP 投影时 live 已按新开关状态落盘、开关事实上已生效：
/// ① 投影失败若上抛，save_settings 会回滚开关设置，制造"设置=旧值、
/// live=新桶"的会话分裂——必须降级为警告而不是失败；
/// ② 投影必须只针对 Codex：sync_all_enabled 按 AppType::all() 顺序短路，
/// Claude 排在 Codex 前面，损坏的 ~/.claude.json 会在轮到 Codex 之前
/// 报错——若吞错了事，刚被整体重写清掉的 [mcp_servers] 就无人补回，
/// Codex MCP 静默消失。这里用坏 JSON 的 ~/.claude.json 复现该场景。
#[test]
fn reapply_codex_official_live_projects_mcp_despite_broken_claude_json() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
    });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed official live auth");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        let mut official = Provider::with_id(
            "official-provider".to_string(),
            "Official".to_string(),
            json!({
                "auth": {
                    "auth_mode": "chatgpt",
                    "OPENAI_API_KEY": null,
                    "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
                },
                "config": ""
            }),
            None,
        );
        official.category = Some("official".to_string());
        manager
            .providers
            .insert("official-provider".to_string(), official);
    }
    let servers = initial_config
        .mcp
        .servers
        .get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    // 先切换建立"当前=官方"的前置状态，再破坏 claude 文件，
    // 让损坏只作用于被测的 reapply 路径。
    ProviderService::switch(&state, AppType::Codex, "official-provider")
        .expect("switch to official provider");

    // 破坏 ~/.claude.json：坏 JSON 能通过 should_sync_claude_mcp 门控
    // （文件存在即过），但 read_mcp_servers_map 解析必然报错。
    // 注意 codex-only 的服务器也会触发 claude 的 remove 分支读该文件。
    let claude_json = fyagent_lib::get_claude_mcp_path();
    std::fs::write(&claude_json, "{ not valid json").expect("seed broken claude json");

    let reapplied = fyagent_lib::reapply_current_codex_official_live(&state)
        .expect("MCP projection failure must degrade to a warning, not fail the toggle");
    assert!(
        reapplied,
        "current provider is official, reapply should run"
    );

    // 定向投影不碰 claude：Codex 的 MCP 投影必须完成，不能被
    // 无关应用的损坏文件阻断后静默丢失。
    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after reapply");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "codex MCP projection must not be blocked by a broken ~/.claude.json, got: {live}"
    );

    // 顺带锁定：定向投影不应把手改坏的 ~/.claude.json 触碰或"修复"。
    let claude_after = std::fs::read_to_string(&claude_json).expect("read claude json");
    assert_eq!(
        claude_after, "{ not valid json",
        "codex-only projection must not touch claude's live file"
    );
}

/// 切换供应商与 reapply 是同一类场景：live 整体重写后只需重投影本应用
/// 的 MCP。历史实现走全量 sync_all_enabled + `?`，损坏的 ~/.claude.json
/// 会让"切 Codex"直接报切换失败——而此时 DB is_current 与 live 都已
/// 落盘，切换事实上成功，报错只制造分裂假象。
#[test]
fn switch_codex_projects_mcp_despite_broken_claude_json() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-old" }), Some(""))
        .expect("seed codex live");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "p".to_string(),
            Provider::with_id(
                "p".to_string(),
                "P".to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": "sk-p" },
                    "config": "model = \"gpt-5.5\"\n"
                }),
                None,
            ),
        );
    }
    let servers = config.mcp.servers.get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&config).expect("create test state");

    // 坏 JSON 能通过 should_sync_claude_mcp 门控（文件存在即过），
    // 但 read_mcp_servers_map 解析必然报错；codex-only 服务器也会
    // 触发 claude 的 remove 分支去读这个文件。
    let claude_json = fyagent_lib::get_claude_mcp_path();
    std::fs::write(&claude_json, "{ not valid json").expect("seed broken claude json");

    fyagent_lib::McpService::sync_enabled_for_app(&state, &AppType::Codex)
        .expect("seed MCP independently of source switching");

    ProviderService::switch(&state, AppType::Codex, "p")
        .expect("broken ~/.claude.json must not fail an unrelated codex switch");

    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "switch must preserve Codex MCP, got: {live}"
    );

    let claude_after = std::fs::read_to_string(&claude_json).expect("read claude json");
    assert_eq!(
        claude_after, "{ not valid json",
        "codex-only projection must not touch claude's live file"
    );
}

/// sync_all_enabled 的全量语义（配置导入 / 云同步恢复）：单个应用的
/// live 损坏不阻断其余应用的投影，但失败必须聚合上报——调用方需要
/// 知道结果不完整。历史实现按 AppType::all() 顺序 `?` 短路，Claude
/// 排在 Codex 前面，一份坏 ~/.claude.json 会让所有后续应用的 MCP
/// 状态永远陈旧。
#[test]
fn sync_all_enabled_reports_broken_app_but_projects_the_rest() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk" }), Some("")).expect("seed codex live");

    let mut config = MultiAppConfig::default();
    let servers = config.mcp.servers.get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&config).expect("create test state");

    let claude_json = fyagent_lib::get_claude_mcp_path();
    std::fs::write(&claude_json, "{ not valid json").expect("seed broken claude json");

    let err = fyagent_lib::McpService::sync_all_enabled(&state)
        .expect_err("broken claude live must surface as an aggregated error");
    let message = err.to_string();
    assert!(
        message.contains("claude"),
        "aggregated error should name the failing app, got: {message}"
    );

    // Claude 的失败不能阻断 Codex：best-effort 必须继续投影其余应用。
    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after sync_all_enabled");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "codex projection must proceed despite the broken claude file, got: {live}"
    );
}

#[test]
fn provider_service_official_source_switch_preserves_live_auth_json() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let live_auth_a = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "official-a-live-token",
            "account_id": "acct-a"
        }
    });
    write_codex_live_atomic(
        &live_auth_a,
        Some("cli_auth_credentials_store = \"file\"\n"),
    )
    .expect("seed official account A live auth");

    let mut official_a = Provider::with_id(
        "official-a".to_string(),
        "Official A".to_string(),
        json!({
            "auth": {
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": {
                    "access_token": "stale-a-token",
                    "account_id": "acct-a"
                }
            },
            "config": ""
        }),
        None,
    );
    official_a.category = Some("official".to_string());

    let mut official_b = Provider::with_id(
        "official-b".to_string(),
        "Official B".to_string(),
        json!({
            "auth": {
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": {
                    "access_token": "official-b-token",
                    "account_id": "acct-b"
                }
            },
            "config": ""
        }),
        None,
    );
    official_b.category = Some("official".to_string());

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "official-a".to_string();
        manager
            .providers
            .insert("official-a".to_string(), official_a);
        manager
            .providers
            .insert("official-b".to_string(), official_b);
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "official-b")
        .expect("switch official request source without changing login identity");
    let auth_b: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth B");
    assert_eq!(
        auth_b
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("official-a-live-token"),
        "source selection must not replace the official account from a saved Provider"
    );

    ProviderService::switch(&state, AppType::Codex, "official-a")
        .expect("switch back to official account A should use backfilled live auth");
    let auth_a: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth A");
    assert_eq!(
        auth_a
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("official-a-live-token"),
        "backfill should preserve account A's latest live token for later official switches"
    );
}

#[test]
fn provider_service_switch_codex_backfill_keeps_provider_specific_model_provider_id() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let legacy_auth = json!({ "OPENAI_API_KEY": "rightcode-key" });
    let provider_a_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&legacy_auth, Some(provider_a_config))
        .expect("seed existing codex live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "provider-a".to_string();
        manager.providers.insert(
            "provider-a".to_string(),
            Provider::with_id(
                "provider-a".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "rightcode-key"},
                    "config": provider_a_config
                }),
                None,
            ),
        );
        manager.providers.insert(
            "provider-b".to_string(),
            Provider::with_id(
                "provider-b".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "aihubmix-key"},
                    "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"
profile = "work"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true

[profiles.work]
model_provider = "aihubmix"
model = "gpt-5.4"
"#
                }),
                None,
            ),
        );
        manager.providers.insert(
            "provider-c".to_string(),
            Provider::with_id(
                "provider-c".to_string(),
                "Vendor C".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "vendor-c-key"},
                    "config": r#"model_provider = "vendor_c"
model = "gpt-5.4"

[model_providers.vendor_c]
name = "Vendor C"
base_url = "https://vendor-c.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "provider-b")
        .expect("switch to provider b should succeed");
    ProviderService::switch(&state, AppType::Codex, "provider-c")
        .expect("switch to provider c should succeed");

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switches");
    let provider_b_config = providers
        .get("provider-b")
        .expect("provider b exists")
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .expect("provider b config");
    let parsed: toml::Value = toml::from_str(provider_b_config).expect("parse provider b config");

    assert_eq!(
        parsed.get("model_provider").and_then(|v| v.as_str()),
        Some("aihubmix"),
        "backfill should restore provider b's storage-specific model_provider id"
    );
    assert!(
        parsed
            .get("model_providers")
            .and_then(|v| v.get("aihubmix"))
            .is_some(),
        "provider b should keep its own model_providers table after backfill"
    );
    assert_eq!(
        parsed
            .get("profiles")
            .and_then(|v| v.get("work"))
            .and_then(|v| v.get("model_provider"))
            .and_then(|v| v.as_str()),
        Some("aihubmix"),
        "source selection leaves the stored snapshot unchanged; it does not backfill"
    );
}

#[test]
fn sync_current_provider_for_app_keeps_live_takeover_and_updates_restore_backup() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "current-provider".to_string();

        let mut provider = Provider::with_id(
            "current-provider".to_string(),
            "Current".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "real-token",
                    "ANTHROPIC_BASE_URL": "https://claude.example"
                }
            }),
            None,
        );
        provider.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });

        manager
            .providers
            .insert("current-provider".to_string(), provider);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{ "includeCoAuthoredBy": false }"#.to_string()),
        )
        .expect("set common config snippet");

    let taken_over_live = json!({
        "env": {
            "ANTHROPIC_BASE_URL": "http://127.0.0.1:5000",
            "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
        }
    });
    let settings_path = get_claude_settings_path();
    std::fs::create_dir_all(settings_path.parent().expect("settings dir")).expect("create dir");
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&taken_over_live).expect("serialize taken over live"),
    )
    .expect("write taken over live");

    futures::executor::block_on(state.db.save_live_backup("claude", "{\"env\":{}}"))
        .expect("seed live backup");

    let mut proxy_config = futures::executor::block_on(state.db.get_proxy_config_for_app("claude"))
        .expect("get proxy config");
    proxy_config.enabled = true;
    futures::executor::block_on(state.db.update_proxy_config_for_app(proxy_config))
        .expect("enable takeover");

    ProviderService::sync_current_provider_for_app(&state, AppType::Claude)
        .expect("sync current provider should succeed");

    let live_after: serde_json::Value =
        read_json_file(&settings_path).expect("read live settings after sync");
    assert_eq!(
        live_after, taken_over_live,
        "sync should not overwrite live config while takeover is active"
    );

    let backup = futures::executor::block_on(state.db.get_live_backup("claude"))
        .expect("get live backup")
        .expect("backup exists");
    let backup_value: serde_json::Value =
        serde_json::from_str(&backup.original_config).expect("parse backup value");

    assert_eq!(
        backup_value
            .get("includeCoAuthoredBy")
            .and_then(|v| v.as_bool()),
        Some(false),
        "restore backup should receive the updated effective config"
    );
    assert_eq!(
        backup_value
            .get("env")
            .and_then(|v| v.get("ANTHROPIC_AUTH_TOKEN"))
            .and_then(|v| v.as_str()),
        Some("real-token"),
        "restore backup should preserve the provider token rather than proxy placeholder"
    );
}

#[test]
fn switch_codex_provider_with_takeover_live_but_stopped_proxy_keeps_proxy_live_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let oauth_auth = json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "access_token": "oauth-access",
            "id_token": "oauth-id"
        }
    });
    let old_provider_config = r#"model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
experimental_bearer_token = "old-key"
"#;
    let proxy_live_config = r#"model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "http://127.0.0.1:15721/v1"
wire_api = "responses"
experimental_bearer_token = "PROXY_MANAGED"
"#;
    write_codex_live_atomic(&oauth_auth, Some(proxy_live_config))
        .expect("seed taken-over Codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();

        let mut old_provider = Provider::with_id(
            "old-provider".to_string(),
            "DeepSeek Old".to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": "old-key"},
                "config": old_provider_config
            }),
            None,
        );
        old_provider.category = Some("custom".to_string());
        manager
            .providers
            .insert("old-provider".to_string(), old_provider);

        let mut new_provider = Provider::with_id(
            "new-provider".to_string(),
            "DeepSeek New".to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": "new-key"},
                "config": r#"model_provider = "deepseek-new"
model = "deepseek-reasoner"

[model_providers.deepseek-new]
name = "DeepSeek New"
base_url = "https://new.deepseek.example/v1"
wire_api = "responses"
"#
            }),
            None,
        );
        new_provider.category = Some("custom".to_string());
        manager
            .providers
            .insert("new-provider".to_string(), new_provider);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    futures::executor::block_on(
        state.db.save_live_backup(
            "codex",
            &serde_json::to_string(&json!({
                "auth": oauth_auth,
                "config": old_provider_config
            }))
            .expect("serialize backup"),
        ),
    )
    .expect("seed Codex live backup");

    assert!(
        !futures::executor::block_on(state.proxy_service.is_running()),
        "fixture keeps the proxy server stopped"
    );

    ProviderService::switch(&state, AppType::Codex, "new-provider")
        .expect("switch should update takeover backup instead of writing normal live config");

    let auth_after: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_after, oauth_auth,
        "provider switch during takeover ownership must not rewrite Codex OAuth auth"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        live_config.contains("http://127.0.0.1:15721/v1"),
        "live config should remain pointed at the local proxy"
    );
    assert!(
        live_config.contains("PROXY_MANAGED"),
        "live config should keep the proxy bearer placeholder"
    );
    assert!(
        live_config.contains(r#"model_provider = "deepseek-new""#)
            && live_config.contains(r#"name = "DeepSeek New""#),
        "live config should update the Codex-visible provider label during takeover"
    );
    assert!(
        !live_config.contains("https://new.deepseek.example/v1"),
        "normal provider base_url must not overwrite taken-over live config"
    );

    let backup = futures::executor::block_on(state.db.get_live_backup("codex"))
        .expect("get Codex backup")
        .expect("backup exists");
    let backup_value: serde_json::Value =
        serde_json::from_str(&backup.original_config).expect("parse backup");
    assert_eq!(
        backup_value.get("auth"),
        Some(&auth_after),
        "restore backup should preserve the official OAuth auth"
    );
    let backup_config = backup_value
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        backup_config.contains("new-key") && backup_config.contains("deepseek-new"),
        "restore backup should be rebuilt from the newly selected provider"
    );

    let current = state
        .db
        .get_current_provider(AppType::Codex.as_str())
        .expect("get current provider");
    assert_eq!(current.as_deref(), Some("new-provider"));
}

#[test]
fn explicitly_cleared_common_snippet_is_not_auto_extracted() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let state = create_test_state().expect("create test state");
    state
        .db
        .set_config_snippet_cleared(AppType::Claude.as_str(), true)
        .expect("mark snippet explicitly cleared");

    assert!(
        !state
            .db
            .should_auto_extract_config_snippet(AppType::Claude.as_str())
            .expect("check auto-extract eligibility"),
        "explicitly cleared snippets should block auto-extraction"
    );

    state
        .db
        .set_config_snippet(AppType::Claude.as_str(), Some("{}".to_string()))
        .expect("set snippet");
    state
        .db
        .set_config_snippet_cleared(AppType::Claude.as_str(), false)
        .expect("clear explicit-empty marker");

    assert!(
        !state
            .db
            .should_auto_extract_config_snippet(AppType::Claude.as_str())
            .expect("check auto-extract after snippet saved"),
        "existing snippets should also block auto-extraction"
    );
}

#[test]
fn legacy_common_config_migration_flag_roundtrip() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let state = create_test_state().expect("create test state");

    assert!(
        !state
            .db
            .is_legacy_common_config_migrated()
            .expect("initial migration flag"),
        "migration flag should default to false"
    );

    state
        .db
        .set_legacy_common_config_migrated(true)
        .expect("set migration flag");
    assert!(
        state
            .db
            .is_legacy_common_config_migrated()
            .expect("read migration flag"),
        "migration flag should persist once set"
    );

    state
        .db
        .set_legacy_common_config_migrated(false)
        .expect("clear migration flag");
    assert!(
        !state
            .db
            .is_legacy_common_config_migrated()
            .expect("read migration flag after clear"),
        "migration flag should be removable for tests/debugging"
    );
}

#[test]
fn switch_packycode_gemini_updates_security_selected_type() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Gemini)
            .expect("gemini manager");
        manager.current = "packy-gemini".to_string();
        manager.providers.insert(
            "packy-gemini".to_string(),
            Provider::with_id(
                "packy-gemini".to_string(),
                "PackyCode".to_string(),
                json!({
                    "env": {
                        "GEMINI_API_KEY": "pk-key",
                        "GOOGLE_GEMINI_BASE_URL": "https://www.packyapi.com"
                    }
                }),
                Some("https://www.packyapi.com".to_string()),
            ),
        );
    }

    let state = create_test_state_with_config(&config).expect("create test state");

    ProviderService::switch(&state, AppType::Gemini, "packy-gemini")
        .expect("switching to PackyCode Gemini should succeed");

    // Gemini security settings are written to ~/.gemini/settings.json, not ~/.fyagent/settings.json
    let settings_path = home.join(".gemini").join("settings.json");
    assert!(
        settings_path.exists(),
        "Gemini settings.json should exist at {}",
        settings_path.display()
    );
    let raw = std::fs::read_to_string(&settings_path).expect("read gemini settings.json");
    let value: serde_json::Value =
        serde_json::from_str(&raw).expect("parse gemini settings.json after switch");

    assert_eq!(
        value
            .pointer("/security/auth/selectedType")
            .and_then(|v| v.as_str()),
        Some("gemini-api-key"),
        "PackyCode Gemini should set security.auth.selectedType"
    );
}

#[test]
fn first_run_gemini_import_switch_away_and_back_preserves_gemini_model() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let gemini_dir = home.join(".gemini");
    std::fs::create_dir_all(&gemini_dir).expect("create Gemini config directory");
    let env_path = gemini_dir.join(".env");
    std::fs::write(
        &env_path,
        "GEMINI_API_KEY=g-live\nGOOGLE_GEMINI_BASE_URL=https://relay.example\nGEMINI_MODEL=gemini-2.5-pro\n",
    )
    .expect("seed first-run Gemini .env");
    std::fs::write(
        gemini_dir.join("settings.json"),
        r#"{
  "security": { "auth": { "selectedType": "gemini-api-key" } },
  "ui": { "theme": "GitHub" }
}"#,
    )
    .expect("seed first-run Gemini settings");

    let state = create_test_state().expect("create test state");
    // 不能只调用 service 导入：回归还需要覆盖 common snippet 提取和行内剥离。
    assert!(
        fyagent_lib::import_default_config_test_hook(&state, AppType::Gemini)
            .expect("import first-run Gemini live config"),
        "first-run import should create the default provider"
    );
    state
        .db
        .init_default_official_providers()
        .expect("seed official providers after first-run import");
    let other = Provider::with_id(
        "other-gemini".to_string(),
        "Other Gemini".to_string(),
        json!({
            "env": {
                "GEMINI_API_KEY": "g-other",
                "GOOGLE_GEMINI_BASE_URL": "https://other.example",
                "GEMINI_MODEL": "gemini-2.5-flash"
            }
        }),
        None,
    );
    state
        .db
        .save_provider(AppType::Gemini.as_str(), &other)
        .expect("save other Gemini provider");

    ProviderService::switch(&state, AppType::Gemini, &other.id).expect("switch away from default");
    let env_content =
        std::fs::read_to_string(&env_path).expect("read Gemini .env after switching away");
    assert_eq!(
        env_content
            .lines()
            .filter_map(|line| line.strip_prefix("GEMINI_MODEL="))
            .collect::<Vec<_>>(),
        vec!["gemini-2.5-flash"],
        "switching away must replace the original live model"
    );

    ProviderService::switch(&state, AppType::Gemini, "default")
        .expect("switch back to imported default");
    let env_content =
        std::fs::read_to_string(&env_path).expect("read Gemini .env after switching back");
    assert_eq!(
        env_content
            .lines()
            .filter_map(|line| line.strip_prefix("GEMINI_MODEL="))
            .collect::<Vec<_>>(),
        vec!["gemini-2.5-pro"],
        "imported GEMINI_MODEL must be written back to .env, got: {env_content}"
    );
}

#[test]
fn switch_google_official_gemini_preserves_env_vars() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Gemini)
            .expect("gemini manager");
        manager.current = "google-official".to_string();
        let provider = Provider::with_id(
            "google-official".to_string(),
            "Google".to_string(),
            json!({
                "env": {
                    "GEMINI_MODEL": "gemini-2.5-pro"
                }
            }),
            Some("https://ai.google.dev".to_string()),
        );
        manager
            .providers
            .insert("google-official".to_string(), provider);
    }

    let state = create_test_state_with_config(&config).expect("create test state");

    ProviderService::switch(&state, AppType::Gemini, "google-official")
        .expect("switching to Google official Gemini should succeed");

    // Verify env vars are preserved in ~/.gemini/.env
    let env_path = home.join(".gemini").join(".env");
    assert!(
        env_path.exists(),
        "Gemini .env should exist at {}",
        env_path.display()
    );
    let env_content = std::fs::read_to_string(&env_path).expect("read gemini .env");
    assert!(
        env_content.contains("GEMINI_MODEL=gemini-2.5-pro"),
        "GEMINI_MODEL should be preserved in .env, got: {env_content}"
    );

    // Verify OAuth security flag is still set correctly
    let gemini_settings = home.join(".gemini").join("settings.json");
    let gemini_raw = std::fs::read_to_string(&gemini_settings).expect("read gemini settings");
    let gemini_value: serde_json::Value =
        serde_json::from_str(&gemini_raw).expect("parse gemini settings");
    assert_eq!(
        gemini_value
            .pointer("/security/auth/selectedType")
            .and_then(|v| v.as_str()),
        Some("oauth-personal"),
        "OAuth security flag should still be set"
    );
}

#[test]
fn provider_service_switch_claude_updates_live_and_state() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("create claude settings dir");
    }
    let legacy_live = json!({
        "env": {
            "ANTHROPIC_API_KEY": "legacy-key"
        },
        "workspace": {
            "path": "/tmp/workspace"
        }
    });
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&legacy_live).expect("serialize legacy live"),
    )
    .expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "old-provider".to_string();
        manager.providers.insert(
            "old-provider".to_string(),
            Provider::with_id(
                "old-provider".to_string(),
                "Legacy Claude".to_string(),
                json!({
                    "env": { "ANTHROPIC_API_KEY": "stale-key" }
                }),
                None,
            ),
        );
        manager.providers.insert(
            "new-provider".to_string(),
            Provider::with_id(
                "new-provider".to_string(),
                "Fresh Claude".to_string(),
                json!({
                    "env": { "ANTHROPIC_API_KEY": "fresh-key" },
                    "workspace": { "path": "/tmp/new-workspace" }
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&config).expect("create test state");

    ProviderService::switch(&state, AppType::Claude, "new-provider")
        .expect("switch provider should succeed");

    let live_after: serde_json::Value =
        read_json_file(&settings_path).expect("read claude live settings");
    assert_eq!(
        live_after
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_API_KEY"))
            .and_then(|key| key.as_str()),
        Some("fresh-key"),
        "live settings.json should reflect new provider auth"
    );

    let providers = state
        .db
        .get_all_providers(AppType::Claude.as_str())
        .expect("get all providers");
    let current_id = state
        .db
        .get_current_provider(AppType::Claude.as_str())
        .expect("get current provider");
    assert_eq!(
        current_id.as_deref(),
        Some("new-provider"),
        "current provider updated"
    );

    let legacy_provider = providers
        .get("old-provider")
        .expect("legacy provider still exists");
    assert_eq!(
        legacy_provider.settings_config,
        json!({"env": {"ANTHROPIC_API_KEY": "stale-key"}}),
        "source switching leaves the saved row unchanged"
    );
}

/// 切换只替换 Claude 的关键字段；用户 live 偏好原样保留，旧通用片段不回填。
#[test]
fn switch_claude_syncs_new_shared_keys_from_live_into_common_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("create claude settings dir");
    }
    // A 的 live = A 私有密钥（含非 Anthropic 的 OpenRouter 凭据）+ 已共享的 theme
    // + 用户刚在应用内新增的 enableAllProjectMcpServers
    let live = json!({
        "env": { "ANTHROPIC_API_KEY": "a-key", "OPENROUTER_API_KEY": "sk-or-leak" },
        "theme": "dark",
        "enableAllProjectMcpServers": true
    });
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&live).expect("serialize live"),
    )
    .expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "a".to_string();
        let mut provider_a = Provider::with_id(
            "a".to_string(),
            "A".to_string(),
            // 即使 A 的保存行里也有这个非托管凭据，切换仍保留 live 原值。
            json!({ "env": {
                "ANTHROPIC_API_KEY": "a-key",
                "OPENROUTER_API_KEY": "sk-or-leak"
            } }),
            None,
        );
        provider_a.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("a".to_string(), provider_a);
        let mut provider_b = Provider::with_id(
            "b".to_string(),
            "B".to_string(),
            json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
            None,
        );
        provider_b.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("b".to_string(), provider_b);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{"theme":"dark"}"#.to_string()),
        )
        .expect("seed common config snippet");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch should succeed");

    // 片段应捕获到新增键，并保留已有共享键，且绝不含密钥
    let snippet = state
        .db
        .get_config_snippet(AppType::Claude.as_str())
        .expect("read snippet")
        .expect("snippet present");
    let snippet_value: serde_json::Value =
        serde_json::from_str(&snippet).expect("snippet is valid JSON");
    assert_eq!(
        snippet_value.get("enableAllProjectMcpServers"),
        None,
        "live edits stay in live and are not captured into the saved common snippet"
    );
    assert_eq!(
        snippet_value.get("theme").and_then(|v| v.as_str()),
        Some("dark"),
        "previously shared key should be preserved"
    );
    assert!(
        snippet_value
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_API_KEY"))
            .is_none(),
        "secrets must never leak into the shared snippet"
    );
    assert!(
        snippet_value
            .get("env")
            .and_then(|env| env.get("OPENROUTER_API_KEY"))
            .is_none(),
        "non-Anthropic Claude credentials must never leak into the shared snippet"
    );

    // 新增键应通过通用配置带到 B 的 live
    let live_after: serde_json::Value =
        read_json_file(&settings_path).expect("read live after switch");
    assert_eq!(
        live_after.get("enableAllProjectMcpServers"),
        Some(&json!(true)),
        "shared key should propagate to the next provider's live config"
    );
    // William 2026-10-08 选择 A：非托管凭据留在本机 live，不进入共享片段。
    assert_eq!(
        live_after.pointer("/env/OPENROUTER_API_KEY"),
        Some(&json!("sk-or-leak")),
        "unmanaged credential must retain its original live value"
    );
    assert_eq!(
        live_after
            .get("env")
            .and_then(|env| env.get("ANTHROPIC_API_KEY"))
            .and_then(|v| v.as_str()),
        Some("b-key"),
        "live should reflect new provider's own auth"
    );
}

/// 用户在应用内删掉一个已共享的键后，切换应把删除同步进通用配置，
/// 且不会在切到下一个供应商时被重新注入（否则会"删不掉"）。
#[test]
fn switch_claude_syncs_deletions_from_live_into_common_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("create claude settings dir");
    }
    // live 里 theme 还在，但用户已删掉 enableAllProjectMcpServers
    let live = json!({
        "env": { "ANTHROPIC_API_KEY": "a-key" },
        "theme": "dark"
    });
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&live).expect("serialize live"),
    )
    .expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "a".to_string();
        let mut provider_a = Provider::with_id(
            "a".to_string(),
            "A".to_string(),
            json!({ "env": { "ANTHROPIC_API_KEY": "a-key" } }),
            None,
        );
        provider_a.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("a".to_string(), provider_a);
        let mut provider_b = Provider::with_id(
            "b".to_string(),
            "B".to_string(),
            json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
            None,
        );
        provider_b.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("b".to_string(), provider_b);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    // 片段里仍残留 enableAllProjectMcpServers（上次共享的）
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{"theme":"dark","enableAllProjectMcpServers":true}"#.to_string()),
        )
        .expect("seed common config snippet");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch should succeed");

    let snippet = state
        .db
        .get_config_snippet(AppType::Claude.as_str())
        .expect("read snippet")
        .expect("snippet present");
    let snippet_value: serde_json::Value =
        serde_json::from_str(&snippet).expect("snippet is valid JSON");
    assert!(
        snippet_value.get("enableAllProjectMcpServers") == Some(&json!(true)),
        "source selection leaves the saved snippet unchanged"
    );
    assert_eq!(
        snippet_value.get("theme").and_then(|v| v.as_str()),
        Some("dark"),
        "untouched shared key should remain"
    );

    // 切到 B 后 live 不应再出现被删除的键
    let live_after: serde_json::Value =
        read_json_file(&settings_path).expect("read live after switch");
    assert!(
        live_after.get("enableAllProjectMcpServers").is_none(),
        "deleted shared key must not be re-injected into the next provider"
    );
}

/// Codex 切换保留 live 用户偏好与 MCP，但不回填片段或旧供应商；来源关键字段按目标投影。
#[test]
fn switch_codex_syncs_shared_keys_from_live_into_common_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    // A 激活状态下的 live：A 专属路由 + 已共享的 [tui] + 用户刚加的
    // hide_agent_reasoning + fyagent 注入产物 + MCP 同步投影
    // + 顶层 wire_api（无 model_provider 时的 fallback 写法，属 A 的路由语义）
    // + 历史错误格式 [mcp.servers]（sync_all_enabled 清不掉的孤儿形态）
    let live_config = r#"model = "gpt-5.5"
model_provider = "aprov"
wire_api = "chat"
experimental_bearer_token = "sk-a-live-secret"
model_catalog_json = "fyagent-model-catalog.json"
web_search = "disabled"
hide_agent_reasoning = true

[tui]
notifications = true

[model_providers.aprov]
name = "A Prov"
base_url = "https://a.example/v1"
wire_api = "responses"

[mcp_servers.echo]
type = "stdio"
command = "echo"

[mcp.servers.ghost-legacy]
command = "ghost-cmd"
"#;
    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-a" }), Some(live_config))
        .expect("seed codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "a".to_string();
        let mut provider_a = Provider::with_id(
            "a".to_string(),
            "A".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-a" },
                "config": "model = \"gpt-5.5\"\nmodel_provider = \"aprov\"\n\n[model_providers.aprov]\nname = \"A Prov\"\nbase_url = \"https://a.example/v1\"\nwire_api = \"responses\"\n"
            }),
            None,
        );
        provider_a.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("a".to_string(), provider_a);
        let mut provider_b = Provider::with_id(
            "b".to_string(),
            "B".to_string(),
            json!({
                "auth": { "OPENAI_API_KEY": "sk-b" },
                "config": "model = \"gpt-5.5\"\nmodel_provider = \"custom\"\n\n[model_providers.custom]\nname = \"B Prov\"\nbase_url = \"https://b.example/v1\"\nwire_api = \"responses\"\n"
            }),
            None,
        );
        provider_b.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("b".to_string(), provider_b);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Codex.as_str(),
            Some("[tui]\nnotifications = true\n".to_string()),
        )
        .expect("seed codex common config snippet");

    ProviderService::switch(&state, AppType::Codex, "b").expect("switch should succeed");

    // 片段：捕获新增共享键、保留既有共享键；专属字段/密钥/注入产物一律不进
    let snippet = state
        .db
        .get_config_snippet(AppType::Codex.as_str())
        .expect("read snippet")
        .expect("snippet present");
    assert!(
        !snippet.contains("hide_agent_reasoning = true"),
        "live edits must not be captured into the saved snippet, got: {snippet}"
    );
    assert!(
        snippet.contains("notifications = true"),
        "previously shared key should be preserved, got: {snippet}"
    );
    for forbidden in [
        "experimental_bearer_token",
        "sk-a-live-secret",
        "model_catalog_json",
        "web_search",
        "mcp_servers",
        "model_providers",
        "model_provider",
        "wire_api",
        "ghost-legacy",
    ] {
        assert!(
            !snippet.contains(forbidden),
            "'{forbidden}' must never enter the shared snippet, got: {snippet}"
        );
    }

    // B 的 live：共享键传递到位，A 的密钥/投影不得跟过来
    let live_after = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(
        live_after.contains("hide_agent_reasoning = true"),
        "shared key should propagate to the next provider's live, got: {live_after}"
    );
    assert!(
        live_after.contains("model_provider = \"custom\""),
        "live should be provider B's own config, got: {live_after}"
    );
    assert!(
        !live_after.contains("sk-a-live-secret"),
        "provider A's bearer token must not leak into B's live, got: {live_after}"
    );
    assert!(
        live_after.contains("mcp_servers.echo"),
        "source switching must preserve the user's existing unmanaged MCP, got: {live_after}"
    );
    assert!(
        live_after.contains("ghost-legacy"),
        "unrelated legacy user tables are not owned by request-source selection, got: {live_after}"
    );
    assert!(
        !live_after.contains("wire_api = \"chat\""),
        "provider A's top-level wire_api must not rewrite B's protocol, got: {live_after}"
    );

    // A 的存储配置：保持原样，不从 live 回填。
    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switch");
    let stored_a = providers.get("a").expect("provider a exists");
    let stored_a_config = stored_a
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        stored_a_config.contains("model_provider = \"aprov\""),
        "the previous provider keeps its saved routing, got: {stored_a_config}"
    );
    assert_eq!(
        stored_a.settings_config,
        config.get_manager(&AppType::Codex).unwrap().providers["a"].settings_config,
        "source selection must not backfill the previous saved provider"
    );
}

/// 删掉的共享键要同步进通用配置，且不会在下一家被重新注入
#[test]
fn switch_codex_syncs_deletions_from_live_into_common_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    // 片段里有两个共享键，但用户已在 live 里删掉 disable_response_storage
    let live_config = r#"model_provider = "aprov"

[tui]
notifications = true

[model_providers.aprov]
name = "A Prov"
base_url = "https://a.example/v1"
wire_api = "responses"
"#;
    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-a" }), Some(live_config))
        .expect("seed codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "a".to_string();
        for (id, name, prov_key) in [("a", "A", "aprov"), ("b", "B", "bprov")] {
            let mut provider = Provider::with_id(
                id.to_string(),
                name.to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": format!("sk-{id}") },
                    "config": format!("model_provider = \"{prov_key}\"\n\n[model_providers.{prov_key}]\nname = \"{name} Prov\"\nbase_url = \"https://{id}.example/v1\"\nwire_api = \"responses\"\n")
                }),
                None,
            );
            provider.meta = Some(ProviderMeta {
                common_config_enabled: Some(true),
                ..Default::default()
            });
            manager.providers.insert(id.to_string(), provider);
        }
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Codex.as_str(),
            Some("disable_response_storage = true\n\n[tui]\nnotifications = true\n".to_string()),
        )
        .expect("seed codex common config snippet");

    ProviderService::switch(&state, AppType::Codex, "b").expect("switch should succeed");

    let snippet = state
        .db
        .get_config_snippet(AppType::Codex.as_str())
        .expect("read snippet")
        .expect("snippet present");
    assert!(
        !snippet.contains("disable_response_storage"),
        "deleted shared key must be removed from the common snippet, got: {snippet}"
    );
    assert!(
        snippet.contains("notifications = true"),
        "kept shared key should remain in the snippet, got: {snippet}"
    );

    let live_after = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(
        !live_after.contains("disable_response_storage"),
        "deleted shared key must not be re-injected into the next provider, got: {live_after}"
    );
    assert!(
        live_after.contains("notifications = true"),
        "kept shared key should propagate to the next provider, got: {live_after}"
    );
}

/// 片段同步后切换失败，必须恢复数据库里的原始片段。
#[test]
fn switch_codex_failure_restores_common_config_snippet() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    // 片段里有两个共享键，但用户已在 live 里删掉 disable_response_storage
    let live_config = r#"model_provider = "aprov"

[tui]
notifications = true

[model_providers.aprov]
name = "A Prov"
base_url = "https://a.example/v1"
wire_api = "responses"
"#;
    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-a" }), Some(live_config))
        .expect("seed codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "a".to_string();
        for (id, name, prov_key) in [("a", "A", "aprov"), ("b", "B", "bprov")] {
            let mut provider = Provider::with_id(
                id.to_string(),
                name.to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": format!("sk-{id}") },
                    "config": format!("model_provider = \"{prov_key}\"\n\n[model_providers.{prov_key}]\nname = \"{name} Prov\"\nbase_url = \"https://{id}.example/v1\"\nwire_api = \"responses\"\n")
                }),
                None,
            );
            provider.meta = Some(ProviderMeta {
                common_config_enabled: Some(true),
                ..Default::default()
            });
            manager.providers.insert(id.to_string(), provider);
        }
    }

    // 触发器和切换操作必须使用同一个磁盘数据库；CI 的 test-hooks 保持凭据在内存中。
    let state = support::create_golden_test_state().expect("create test state");
    state.db.migrate_from_json(&config).expect("seed providers");
    state
        .db
        .set_config_snippet(
            AppType::Codex.as_str(),
            Some("disable_response_storage = true\n\n[tui]\nnotifications = true\n".to_string()),
        )
        .expect("seed codex common config snippet");

    let original_snippet = state
        .db
        .get_config_snippet(AppType::Codex.as_str())
        .expect("read original snippet");
    let db = rusqlite::Connection::open(home.join(".fyagent/fyagent.db")).expect("open db");
    db.execute_batch(
        "CREATE TRIGGER reject_codex_switch_current
         BEFORE UPDATE OF is_current ON providers
         WHEN NEW.app_type = 'codex' AND NEW.id = 'b' AND NEW.is_current = 1
         BEGIN
           SELECT CASE WHEN instr(
             (SELECT value FROM settings WHERE key = 'common_config_codex'),
             'disable_response_storage'
           ) = 0 THEN RAISE(ABORT, 'injected Codex switch failure')
           ELSE RAISE(ABORT, 'snippet synchronization did not run') END;
         END;",
    )
    .expect("install switch failure trigger");

    let error = ProviderService::switch(&state, AppType::Codex, "b")
        .expect_err("switch must fail after snippet synchronization");
    db.execute_batch("DROP TRIGGER reject_codex_switch_current;")
        .expect("drop switch failure trigger");
    assert!(
        error.to_string().contains("injected Codex switch failure"),
        "expected the injected switch failure, got: {error}"
    );
    assert_eq!(
        state
            .db
            .get_config_snippet(AppType::Codex.as_str())
            .expect("read snippet after failed switch"),
        original_snippet,
        "failed switch must restore the exact original common config snippet"
    );
    assert_eq!(
        fyagent_lib::mode::operation::settle(&state.db, "codex").expect("recover published switch"),
        Some(fyagent_lib::mode::operation::RecoveryOutcome::RolledForward)
    );
    assert_eq!(
        state.db.get_current_provider("codex").unwrap().as_deref(),
        Some("b")
    );
    let recovered_snippet = state.db.get_config_snippet("codex").unwrap().unwrap();
    assert!(!recovered_snippet.contains("disable_response_storage"));
    assert!(recovered_snippet.contains("notifications = true"));
    let live = std::fs::read_to_string(get_codex_config_path()).unwrap();
    assert!(live.contains("bprov"));
    assert!(!live.contains("disable_response_storage"));
    assert_eq!(
        fyagent_lib::mode::operation::settle(&state.db, "codex").unwrap(),
        None
    );
}

/// 未勾选"写入通用配置"的供应商，其 live 改动不应自动污染通用配置片段。
#[test]
fn switch_claude_does_not_sync_common_config_for_opted_out_provider() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("create claude settings dir");
    }
    let live = json!({
        "env": { "ANTHROPIC_API_KEY": "a-key" },
        "providerSpecific": "x"
    });
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&live).expect("serialize live"),
    )
    .expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "a".to_string();
        // A 未勾选通用配置（meta = None）
        manager.providers.insert(
            "a".to_string(),
            Provider::with_id(
                "a".to_string(),
                "A".to_string(),
                json!({ "env": { "ANTHROPIC_API_KEY": "a-key" } }),
                None,
            ),
        );
        manager.providers.insert(
            "b".to_string(),
            Provider::with_id(
                "b".to_string(),
                "B".to_string(),
                json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{"theme":"dark"}"#.to_string()),
        )
        .expect("seed common config snippet");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch should succeed");

    let snippet = state
        .db
        .get_config_snippet(AppType::Claude.as_str())
        .expect("read snippet")
        .expect("snippet present");
    let snippet_value: serde_json::Value =
        serde_json::from_str(&snippet).expect("snippet is valid JSON");
    assert!(
        snippet_value.get("providerSpecific").is_none(),
        "opted-out provider's live changes must not pollute the shared snippet"
    );
    assert_eq!(
        snippet_value.get("theme").and_then(|v| v.as_str()),
        Some("dark"),
        "snippet should stay unchanged for opted-out providers"
    );
}

/// 用户显式清空过通用配置（_cleared）后，切换不应把片段重新塞回来。
#[test]
fn switch_claude_respects_explicitly_cleared_common_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).expect("create claude settings dir");
    }
    let live = json!({
        "env": { "ANTHROPIC_API_KEY": "a-key" },
        "theme": "dark"
    });
    std::fs::write(
        &settings_path,
        serde_json::to_string_pretty(&live).expect("serialize live"),
    )
    .expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "a".to_string();
        let mut provider_a = Provider::with_id(
            "a".to_string(),
            "A".to_string(),
            json!({ "env": { "ANTHROPIC_API_KEY": "a-key" } }),
            None,
        );
        provider_a.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("a".to_string(), provider_a);
        let mut provider_b = Provider::with_id(
            "b".to_string(),
            "B".to_string(),
            json!({ "env": { "ANTHROPIC_API_KEY": "b-key" } }),
            None,
        );
        provider_b.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert("b".to_string(), provider_b);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet_cleared(AppType::Claude.as_str(), true)
        .expect("mark snippet cleared");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch should succeed");

    assert!(
        state
            .db
            .get_config_snippet(AppType::Claude.as_str())
            .expect("read snippet")
            .is_none(),
        "explicitly cleared snippet must not be resurrected by switch-away sync"
    );
}

#[test]
fn provider_service_switch_missing_provider_returns_error() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let state = create_test_state().expect("create test state");

    let err = ProviderService::switch(&state, AppType::Claude, "missing")
        .expect_err("switching missing provider should fail");
    match err {
        AppError::Message(msg) => {
            assert!(
                msg.contains("不存在") || msg.contains("not found"),
                "expected provider not found message, got {msg}"
            );
        }
        other => panic!("expected Message error for provider not found, got {other:?}"),
    }
}

#[test]
fn provider_service_switch_codex_missing_auth_returns_error() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "invalid".to_string(),
            Provider::with_id(
                "invalid".to_string(),
                "Broken Codex".to_string(),
                json!({
                    "config": "[mcp_servers.test]\ncommand = \"noop\""
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&config).expect("create test state");

    let err = ProviderService::switch(&state, AppType::Codex, "invalid")
        .expect_err("switching should fail without auth");
    match err {
        AppError::Config(msg) => assert!(
            msg.contains("auth"),
            "expected auth related message, got {msg}"
        ),
        other => panic!("expected config error, got {other:?}"),
    }
}

#[test]
fn provider_service_delete_codex_removes_provider_and_files() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "keep".to_string();
        manager.providers.insert(
            "keep".to_string(),
            Provider::with_id(
                "keep".to_string(),
                "Keep".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "keep-key"},
                    "config": ""
                }),
                None,
            ),
        );
        manager.providers.insert(
            "to-delete".to_string(),
            Provider::with_id(
                "to-delete".to_string(),
                "DeleteCodex".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "delete-key"},
                    "config": ""
                }),
                None,
            ),
        );
    }

    let sanitized = sanitize_provider_name("DeleteCodex");
    let codex_dir = home.join(".codex");
    std::fs::create_dir_all(&codex_dir).expect("create codex dir");
    let auth_path = codex_dir.join(format!("auth-{sanitized}.json"));
    let cfg_path = codex_dir.join(format!("config-{sanitized}.toml"));
    std::fs::write(&auth_path, "{}").expect("seed auth file");
    std::fs::write(&cfg_path, "base_url = \"https://example\"").expect("seed config file");

    let app_state = create_test_state_with_config(&config).expect("create test state");

    ProviderService::delete(&app_state, AppType::Codex, "to-delete")
        .expect("delete provider should succeed");

    let providers = app_state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("get all providers");
    assert!(
        !providers.contains_key("to-delete"),
        "provider entry should be removed"
    );
    // v3.7.0+ 不再使用供应商特定文件（如 auth-*.json, config-*.toml）
    // 删除供应商只影响数据库记录，不清理这些旧格式文件
}

#[test]
fn provider_service_delete_claude_removes_provider_files() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "keep".to_string();
        manager.providers.insert(
            "keep".to_string(),
            Provider::with_id(
                "keep".to_string(),
                "Keep".to_string(),
                json!({
                    "env": { "ANTHROPIC_API_KEY": "keep-key" }
                }),
                None,
            ),
        );
        manager.providers.insert(
            "delete".to_string(),
            Provider::with_id(
                "delete".to_string(),
                "DeleteClaude".to_string(),
                json!({
                    "env": { "ANTHROPIC_API_KEY": "delete-key" }
                }),
                None,
            ),
        );
    }

    let sanitized = sanitize_provider_name("DeleteClaude");
    let claude_dir = home.join(".claude");
    std::fs::create_dir_all(&claude_dir).expect("create claude dir");
    let by_name = claude_dir.join(format!("settings-{sanitized}.json"));
    let by_id = claude_dir.join("settings-delete.json");
    std::fs::write(&by_name, "{}").expect("seed settings by name");
    std::fs::write(&by_id, "{}").expect("seed settings by id");

    let app_state = create_test_state_with_config(&config).expect("create test state");

    ProviderService::delete(&app_state, AppType::Claude, "delete").expect("delete claude provider");

    let providers = app_state
        .db
        .get_all_providers(AppType::Claude.as_str())
        .expect("get all providers");
    assert!(
        !providers.contains_key("delete"),
        "claude provider should be removed"
    );
    // v3.7.0+ 不再使用供应商特定文件（如 settings-*.json）
    // 删除供应商只影响数据库记录，不清理这些旧格式文件
}

#[test]
fn provider_service_delete_current_provider_returns_error() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "keep".to_string();
        manager.providers.insert(
            "keep".to_string(),
            Provider::with_id(
                "keep".to_string(),
                "Keep".to_string(),
                json!({
                    "env": { "ANTHROPIC_API_KEY": "keep-key" }
                }),
                None,
            ),
        );
    }

    let app_state = create_test_state_with_config(&config).expect("create test state");

    let err = ProviderService::delete(&app_state, AppType::Claude, "keep")
        .expect_err("deleting current provider should fail");
    match err {
        AppError::Localized { zh, .. } => assert!(
            zh.contains("不能删除当前正在使用的供应商")
                || zh.contains("无法删除当前正在使用的供应商"),
            "unexpected message: {zh}"
        ),
        AppError::Config(msg) => assert!(
            msg.contains("不能删除当前正在使用的供应商")
                || msg.contains("无法删除当前正在使用的供应商"),
            "unexpected message: {msg}"
        ),
        AppError::Message(msg) => assert!(
            msg.contains("不能删除当前正在使用的供应商")
                || msg.contains("无法删除当前正在使用的供应商"),
            "unexpected message: {msg}"
        ),
        other => panic!("expected Config/Message error, got {other:?}"),
    }
}

#[test]
fn recover_from_crash_without_owned_proof_preserves_placeholder() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    // 接管态 Claude Live，且 DB 中无备份（模拟切换 app_config_dir 后新库首启的场景）
    let taken_over_live = json!({
        "env": {
            "ANTHROPIC_BASE_URL": "http://127.0.0.1:15721",
            "ANTHROPIC_AUTH_TOKEN": "PROXY_MANAGED"
        }
    });
    let settings_path = get_claude_settings_path();
    std::fs::create_dir_all(settings_path.parent().expect("settings dir")).expect("create dir");
    let seeded = serde_json::to_string_pretty(&taken_over_live).expect("serialize taken over live");
    std::fs::write(&settings_path, &seeded).expect("write taken over live");

    let state = create_test_state().expect("create test state");

    // 模拟历史异常：接管态 Live 已被导入成 current provider（SSOT 被污染）
    let provider = Provider::with_id(
        "default".to_string(),
        "default".to_string(),
        taken_over_live.clone(),
        None,
    );
    state
        .db
        .save_provider(AppType::Claude.as_str(), &provider)
        .expect("save placeholder provider");
    state
        .db
        .set_current_provider(AppType::Claude.as_str(), "default")
        .expect("set current provider");

    let before = std::fs::read(&settings_path).expect("read seeded live");
    let error = futures::executor::block_on(state.proxy_service.recover_from_crash())
        .expect_err("unowned placeholder must not be rewritten");
    assert!(
        error.contains("无法确认代理配置仍可安全恢复"),
        "crash recovery without backup/receipt must report conflict: {error}"
    );
    assert_eq!(
        std::fs::read(&settings_path).expect("reread live"),
        before,
        "conflict must preserve the unproven placeholder bytes"
    );

    let live_after: serde_json::Value =
        read_json_file(&settings_path).expect("read live settings after recovery");
    let env = live_after.get("env").cloned().unwrap_or_else(|| json!({}));
    assert_eq!(
        env.get("ANTHROPIC_AUTH_TOKEN").and_then(|v| v.as_str()),
        Some("PROXY_MANAGED"),
        "recovery must not manufacture an original by deleting the placeholder"
    );
    assert_eq!(
        env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()),
        Some("http://127.0.0.1:15721"),
        "recovery must not drop an unproven local proxy URL"
    );
}

#[test]
fn provider_service_switch_codex_writes_every_third_party_route_as_custom() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let legacy_auth = json!({ "OPENAI_API_KEY": "rightcode-key" });
    let legacy_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&legacy_auth, Some(legacy_config))
        .expect("seed existing codex live config");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();
        manager.providers.insert(
            "old-provider".to_string(),
            Provider::with_id(
                "old-provider".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "stale"},
                    "config": legacy_config
                }),
                None,
            ),
        );
        manager.providers.insert(
            "new-provider".to_string(),
            Provider::with_id(
                "new-provider".to_string(),
                "AiHubMix".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "fresh-key"},
                    "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    ProviderService::switch(&state, AppType::Codex, "new-provider")
        .expect("switch provider should succeed");

    let config_text =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    let parsed: toml::Value = toml::from_str(&config_text).expect("parse config.toml");

    assert_eq!(
        parsed.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "every third-party route is written as the custom table"
    );

    let model_providers = parsed
        .get("model_providers")
        .and_then(|v| v.as_table())
        .expect("model_providers table exists");
    assert_eq!(
        model_providers
            .get("custom")
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str()),
        Some("https://aihubmix.example/v1"),
        "the custom table points at the newly selected supplier endpoint"
    );
    assert!(
        model_providers.get("aihubmix").is_none(),
        "the row's own table id is not written"
    );
    assert!(
        model_providers.get("rightcode").is_none(),
        "the old version's table for old-provider (same id and address as its row) is retired"
    );

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers after switch");
    let new_config_text = providers
        .get("new-provider")
        .expect("new provider exists")
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        new_config_text.contains("[model_providers.aihubmix]"),
        "stored provider template stays as it was"
    );
}

#[test]
fn provider_service_switch_codex_preserves_oauth_and_keeps_rows_untouched() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": {
            "access_token": "oauth-token",
            "account_id": "acct-1"
        }
    });
    let legacy_config = r#"model_provider = "rightcode"
model = "gpt-5.4"

[model_providers.rightcode]
name = "RightCode"
base_url = "https://rightcode.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#;
    write_codex_live_atomic(&live_auth, Some(legacy_config))
        .expect("seed existing Codex OAuth live config");

    let bridge_provider = Provider::with_id(
        "bridge-provider".to_string(),
        "Bridge Provider".to_string(),
        json!({
            "auth": {"OPENAI_API_KEY": "bridge-key"},
            "config": r#"model_provider = "aihubmix"
model = "gpt-5.4"

[model_providers.aihubmix]
name = "AiHubMix"
base_url = "https://aihubmix.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
        }),
        None,
    );

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "legacy-provider".to_string();
        manager.providers.insert(
            "legacy-provider".to_string(),
            Provider::with_id(
                "legacy-provider".to_string(),
                "RightCode".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "rightcode-key"},
                    "config": legacy_config
                }),
                None,
            ),
        );
        manager
            .providers
            .insert("bridge-provider".to_string(), bridge_provider);
        manager.providers.insert(
            "plain-provider".to_string(),
            Provider::with_id(
                "plain-provider".to_string(),
                "Plain Provider".to_string(),
                json!({
                    "auth": {"OPENAI_API_KEY": "plain-key"},
                    "config": r#"model_provider = "plain"
model = "gpt-5.4"

[model_providers.plain]
name = "Plain"
base_url = "https://plain.example/v1"
wire_api = "responses"
requires_openai_auth = true
"#
                }),
                None,
            ),
        );
    }

    let state = create_test_state_with_config(&initial_config).expect("create test state");
    let saved = state
        .db
        .get_provider_by_id("bridge-provider", "codex")
        .unwrap()
        .unwrap();
    state
        .db
        .save_provider("codex", &saved)
        .expect("admit fixture credential before selecting a source");
    let bridge_before = state
        .db
        .get_provider_by_id("bridge-provider", AppType::Codex.as_str())
        .expect("read bridge row")
        .expect("bridge row");

    ProviderService::switch(&state, AppType::Codex, "bridge-provider")
        .expect("switch to bridge provider should succeed");

    let auth_value: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_value.get("auth_mode").and_then(|v| v.as_str()),
        Some("chatgpt")
    );
    assert!(
        auth_value
            .get("OPENAI_API_KEY")
            .is_some_and(|v| v.is_null()),
        "provider switching should keep OPENAI_API_KEY null in live auth.json"
    );
    assert_eq!(
        auth_value
            .pointer("/tokens/access_token")
            .and_then(|v| v.as_str()),
        Some("oauth-token"),
        "existing ChatGPT OAuth token should be preserved"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    let parsed_live: toml::Value = toml::from_str(&live_config).expect("parse live config");
    assert_eq!(
        parsed_live.get("model_provider").and_then(|v| v.as_str()),
        Some("custom"),
        "third-party routes are always written as the custom table"
    );
    assert_eq!(
        parsed_live
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("experimental_bearer_token"))
            .and_then(|v| v.as_str()),
        Some("bridge-key"),
        "third-party key should be injected into the selected live provider table"
    );
    assert_eq!(
        parsed_live
            .get("model_providers")
            .and_then(|v| v.get("custom"))
            .and_then(|v| v.get("requires_openai_auth"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    ProviderService::switch(&state, AppType::Codex, "plain-provider")
        .expect("switch away should backfill bridge provider");

    let providers = state
        .db
        .get_all_providers(AppType::Codex.as_str())
        .expect("read providers");
    let stored_bridge = providers
        .get("bridge-provider")
        .expect("bridge provider exists after backfill");
    assert_eq!(
        stored_bridge.settings_config, bridge_before.settings_config,
        "switching must preserve the stored provider row"
    );
    assert_eq!(
        stored_bridge
            .settings_config
            .pointer("/auth/OPENAI_API_KEY")
            .and_then(|v| v.as_str()),
        None,
        "source selection retains the native credential reference"
    );
    assert!(
        parsed_live
            .get("model_providers")
            .and_then(|v| v.get("rightcode"))
            .is_none(),
        "the table an old version wrote for legacy-provider is retired: {live_config}"
    );
    assert!(stored_bridge.settings_config.get("credentialRef").is_some());
    assert!(!stored_bridge
        .settings_config
        .to_string()
        .contains("bridge-key"));
    ProviderService::switch(&state, AppType::Codex, "bridge-provider")
        .expect("resolve migrated key on switch back");
    assert!(
        std::fs::read_to_string(fyagent_lib::get_codex_config_path())
            .unwrap()
            .contains("bridge-key")
    );
    assert!(
        !stored_bridge
            .settings_config
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .contains("experimental_bearer_token"),
        "stored provider config should stay clean; bridge token is generated only for live config"
    );
}

#[test]
fn reapply_codex_official_live_rewrites_only_the_session_routing() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let live_auth = json!({
        "auth_mode": "chatgpt",
        "OPENAI_API_KEY": null,
        "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
    });
    write_codex_live_atomic(&live_auth, Some("")).expect("seed official live auth");

    let mut initial_config = MultiAppConfig::default();
    {
        let manager = initial_config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        let official = Provider::with_id(
            "codex-official".to_string(),
            "Official".to_string(),
            json!({
                "auth": {
                    "auth_mode": "chatgpt",
                    "OPENAI_API_KEY": null,
                    "tokens": { "access_token": "official-oauth-token", "account_id": "acct" }
                },
                "config": ""
            }),
            None,
        );
        manager
            .providers
            .insert("codex-official".to_string(), official);
    }
    let servers = initial_config
        .mcp
        .servers
        .get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&initial_config).expect("create test state");

    fyagent_lib::McpService::sync_enabled_for_app(&state, &AppType::Codex)
        .expect("seed MCP independently of source switching");

    ProviderService::switch(&state, AppType::Codex, "codex-official")
        .expect("switch to official provider");
    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "switch should preserve existing MCP servers"
    );

    // 统一会话开关变更触发的 reapply 会整体重写 live config.toml（有意设计），
    // 写完必须重新投影 DB 里启用的 MCP，否则用户的 MCP 会静默失效。
    let reapplied =
        fyagent_lib::reapply_current_codex_official_live(&state).expect("reapply official live");
    assert!(
        reapplied,
        "current provider is official, reapply should run"
    );

    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after reapply");
    assert!(
        live.contains("mcp_servers.echo-server"),
        "reapply must re-project enabled MCP servers after the full live rewrite, got: {live}"
    );
}

#[test]
fn switch_codex_ignores_a_broken_claude_json() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-old" }), Some(""))
        .expect("seed codex live");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.providers.insert(
            "p".to_string(),
            Provider::with_id(
                "p".to_string(),
                "P".to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": "sk-p" },
                    "config": "model = \"gpt-5.5\"\n"
                }),
                None,
            ),
        );
    }
    let servers = config.mcp.servers.get_or_insert_with(Default::default);
    servers.insert(
        "echo-server".into(),
        McpServer {
            id: "echo-server".into(),
            name: "Echo Server".into(),
            server: json!({
                "type": "stdio",
                "command": "echo"
            }),
            apps: McpApps {
                claude: false,
                codex: true,
                gemini: false,
                grokbuild: false,
                opencode: false,
                hermes: false,
                workbuddy: false,
                qoderwork: false,
                trae_work: false,
            },
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        },
    );

    let state = create_test_state_with_config(&config).expect("create test state");

    // 坏 JSON 能通过 should_sync_claude_mcp 门控（文件存在即过），
    // 但 read_mcp_servers_map 解析必然报错；codex-only 服务器也会
    // 触发 claude 的 remove 分支去读这个文件。
    let claude_json = fyagent_lib::get_claude_mcp_path();
    std::fs::write(&claude_json, "{ not valid json").expect("seed broken claude json");

    ProviderService::switch(&state, AppType::Codex, "p")
        .expect("broken ~/.claude.json must not fail an unrelated codex switch");

    let live = std::fs::read_to_string(fyagent_lib::get_codex_config_path())
        .expect("read config.toml after switch");
    assert!(live.contains("gpt-5.5"), "{live}");

    let claude_after = std::fs::read_to_string(&claude_json).expect("read claude json");
    assert_eq!(
        claude_after, "{ not valid json",
        "a Codex switch must not touch claude's live file"
    );
}

#[test]
fn sync_current_provider_for_app_leaves_the_proxy_contract_alone() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = "current-provider".to_string();

        let mut provider = Provider::with_id(
            "current-provider".to_string(),
            "Current".to_string(),
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "real-token",
                    "ANTHROPIC_BASE_URL": "https://claude.example"
                }
            }),
            None,
        );
        provider.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });

        manager
            .providers
            .insert("current-provider".to_string(), provider);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Claude.as_str(),
            Some(r#"{ "includeCoAuthoredBy": false }"#.to_string()),
        )
        .expect("set common config snippet");

    let settings_path = get_claude_settings_path();
    std::fs::create_dir_all(settings_path.parent().expect("settings dir")).expect("create dir");
    std::fs::write(
        &settings_path,
        r#"{"env":{"ANTHROPIC_BASE_URL":"https://claude.example","ANTHROPIC_AUTH_TOKEN":"real-token"}}"#,
    )
    .expect("seed live settings");

    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");
    rt.block_on(async {
        let mut proxy_config = state.db.get_proxy_config().await.expect("get proxy config");
        proxy_config.listen_port = 0;
        state
            .db
            .update_proxy_config(proxy_config)
            .await
            .expect("use ephemeral proxy port");
        fyagent_lib::mode::controller::enter(&state, &AppType::Claude, false)
            .await
            .expect("enter routing mode");
    });
    let contract_bytes = std::fs::read(&settings_path).expect("read proxy contract");

    ProviderService::sync_current_provider_for_app(&state, AppType::Claude)
        .expect("sync current provider should succeed");

    assert_eq!(
        std::fs::read(&settings_path).expect("read live settings after sync"),
        contract_bytes,
        "routing mode: syncing the routed provider must not rewrite live with its direct projection"
    );
    rt.block_on(fyagent_lib::mode::controller::exit(
        &state,
        &AppType::Claude,
    ))
    .expect("leave routing mode");
}

#[test]
fn switch_codex_in_direct_mode_replaces_leftover_proxy_placeholders() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    enable_codex_official_auth_preservation();
    let _home = ensure_test_home();

    let oauth_auth = json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "access_token": "oauth-access",
            "id_token": "oauth-id"
        }
    });
    let old_provider_config = r#"model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/v1"
wire_api = "responses"
experimental_bearer_token = "old-key"
"#;
    let proxy_live_config = r#"model_provider = "deepseek"
model = "deepseek-chat"

[model_providers.deepseek]
name = "DeepSeek"
base_url = "http://127.0.0.1:15721/v1"
wire_api = "responses"
experimental_bearer_token = "PROXY_MANAGED"
"#;
    write_codex_live_atomic(&oauth_auth, Some(proxy_live_config))
        .expect("seed taken-over Codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "old-provider".to_string();

        let mut old_provider = Provider::with_id(
            "old-provider".to_string(),
            "DeepSeek Old".to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": "old-key"},
                "config": old_provider_config
            }),
            None,
        );
        old_provider.category = Some("custom".to_string());
        manager
            .providers
            .insert("old-provider".to_string(), old_provider);

        let mut new_provider = Provider::with_id(
            "new-provider".to_string(),
            "DeepSeek New".to_string(),
            json!({
                "auth": {"OPENAI_API_KEY": "new-key"},
                "config": r#"model_provider = "deepseek-new"
model = "deepseek-reasoner"

[model_providers.deepseek-new]
name = "DeepSeek New"
base_url = "https://new.deepseek.example/v1"
wire_api = "responses"
"#
            }),
            None,
        );
        new_provider.category = Some("custom".to_string());
        manager
            .providers
            .insert("new-provider".to_string(), new_provider);
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    assert!(!fyagent_lib::mode::current::is_proxy(&AppType::Codex));

    ProviderService::switch(&state, AppType::Codex, "new-provider")
        .expect("switch in direct mode writes the new provider");

    let auth_after: serde_json::Value =
        read_json_file(&fyagent_lib::get_codex_auth_path()).expect("read auth.json");
    assert_eq!(
        auth_after, oauth_auth,
        "preserving the official login keeps OAuth auth.json"
    );

    let live_config =
        std::fs::read_to_string(fyagent_lib::get_codex_config_path()).expect("read config.toml");
    assert!(
        live_config.contains("https://new.deepseek.example/v1")
            && !live_config.contains("PROXY_MANAGED"),
        "a direct switch replaces leftover proxy placeholders: {live_config}"
    );
    let old_row = state
        .db
        .get_provider_by_id("old-provider", AppType::Codex.as_str())
        .expect("read old provider")
        .expect("old provider exists");
    assert!(
        !old_row
            .settings_config
            .to_string()
            .contains("PROXY_MANAGED"),
        "leftover placeholders must not be backfilled into the outgoing provider"
    );
}

fn seed_claude_switch_state(
    providers: &[(&str, serde_json::Value)],
    current: &str,
    live: &str,
) -> fyagent_lib::AppState {
    let settings_path = get_claude_settings_path();
    std::fs::create_dir_all(settings_path.parent().expect("settings dir"))
        .expect("create claude settings dir");
    std::fs::write(&settings_path, live).expect("seed claude live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        manager.current = current.to_string();
        for (id, settings) in providers {
            let mut provider =
                Provider::with_id(id.to_string(), id.to_string(), settings.clone(), None);
            provider.meta = Some(ProviderMeta {
                common_config_enabled: Some(true),
                ..Default::default()
            });
            manager.providers.insert(id.to_string(), provider);
        }
    }
    support::create_test_state_with_config(&config).expect("create test state")
}

fn claude_row(state: &fyagent_lib::AppState, id: &str) -> serde_json::Value {
    state
        .db
        .get_provider_by_id(id, AppType::Claude.as_str())
        .expect("read provider")
        .expect("provider exists")
        .settings_config
}

fn claude_live_text() -> String {
    std::fs::read_to_string(get_claude_settings_path()).expect("read claude live")
}

fn claude_live() -> serde_json::Value {
    read_json_file(&get_claude_settings_path()).expect("read claude live")
}

#[test]
fn switch_claude_only_replaces_key_fields() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let live = r#"{
  "env": {
    "ANTHROPIC_BASE_URL": "https://a.example",
    "OPENROUTER_API_KEY": "sk-or-user",
    "ANTHROPIC_AUTH_TOKEN": "sk-a",
    "CLAUDE_CODE_USE_POWERSHELL_TOOL": "1"
  },
  "permissions": {
    "allow": [
      "Bash(git status)"
    ]
  },
  "hooks": {
    "Stop": []
  },
  "enabledPlugins": {
    "example@marketplace": true
  },
  "statusLine": {
    "type": "command",
    "command": "~/.claude/statusline.sh"
  }
}
"#;
    let a = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://a.example",
        "ANTHROPIC_AUTH_TOKEN": "sk-a"
    }});
    let b = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://b.example",
        "ANTHROPIC_AUTH_TOKEN": "sk-b"
    }});
    let state = seed_claude_switch_state(&[("a", a.clone()), ("b", b.clone())], "a", live);
    let snippet = r#"{"theme":"dark"}"#;
    state
        .db
        .set_config_snippet(AppType::Claude.as_str(), Some(snippet.to_string()))
        .expect("seed snippet");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch to b");
    let after = claude_live_text();
    assert_eq!(
        after,
        live.replace("https://a.example", "https://b.example")
            .replace("sk-a", "sk-b"),
        "only the key fields change, in place"
    );
    assert_eq!(
        claude_row(&state, "a"),
        a,
        "no backfill into the previous row"
    );
    assert_eq!(
        state
            .db
            .get_config_snippet(AppType::Claude.as_str())
            .expect("read snippet")
            .as_deref(),
        Some(snippet),
        "the frozen snippet is neither read nor written"
    );
    assert!(
        claude_live().get("theme").is_none(),
        "the snippet is not merged into live"
    );

    ProviderService::switch(&state, AppType::Claude, "a").expect("switch back to a");
    assert_eq!(claude_live_text(), live, "A→B→A is byte-identical");
}

#[test]
fn switch_claude_moves_compat_switches_with_their_provider() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let deepseek = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://api.deepseek.example/anthropic",
        "ANTHROPIC_AUTH_TOKEN": "sk-ds",
        "CLAUDE_CODE_DISABLE_ARTIFACT": "1"
    }});
    let state = seed_claude_switch_state(
        &[
            ("deepseek", deepseek.clone()),
            ("claude-official", json!({ "env": {} })),
        ],
        "deepseek",
        &serde_json::to_string_pretty(&deepseek).expect("serialize"),
    );

    ProviderService::switch(&state, AppType::Claude, "claude-official").expect("to official");
    assert_eq!(claude_live(), json!({ "env": {} }));

    ProviderService::switch(&state, AppType::Claude, "deepseek").expect("back to deepseek");
    assert_eq!(claude_live(), deepseek);

    // 用户在 live 里手动改成了 0：不是 CC Switch 写的，切走时保留。
    let mut edited = claude_live();
    edited["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("0");
    std::fs::write(get_claude_settings_path(), edited.to_string()).expect("edit live");
    ProviderService::switch(&state, AppType::Claude, "claude-official").expect("to official");
    assert_eq!(
        claude_live(),
        json!({ "env": { "CLAUDE_CODE_DISABLE_ARTIFACT": "0" } })
    );
}

#[test]
fn switch_claude_window_values_follow_the_provider() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let qwen = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://qwen.example",
        "ANTHROPIC_AUTH_TOKEN": "sk-qwen",
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "983616"
    }});
    let kimi = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
        "ANTHROPIC_AUTH_TOKEN": "sk-kimi",
        "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "262144"
    }});
    let old_kimi = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://api.kimi.com/coding/",
        "ANTHROPIC_AUTH_TOKEN": "sk-old-kimi"
    }});
    let state = seed_claude_switch_state(
        &[
            ("qwen", qwen.clone()),
            ("kimi", kimi),
            ("old-kimi", old_kimi),
            ("claude-official", json!({ "env": {} })),
        ],
        "qwen",
        &serde_json::to_string_pretty(&qwen).expect("serialize"),
    );

    ProviderService::switch(&state, AppType::Claude, "kimi").expect("to kimi");
    assert_eq!(
        claude_live()["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
        json!("262144")
    );
    ProviderService::switch(&state, AppType::Claude, "claude-official").expect("to official");
    assert_eq!(claude_live(), json!({ "env": {} }));

    // 旧版切到早期的 Kimi 行时注入的默认值：上一家的行里查不到。
    ProviderService::switch(&state, AppType::Claude, "old-kimi").expect("to old kimi");
    let mut live = claude_live();
    live["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"] = json!("262144");
    live["env"]["CLAUDE_CODE_AUTO_COMPACT_WINDOW"] = json!("262144");
    std::fs::write(get_claude_settings_path(), live.to_string()).expect("seed injected");
    ProviderService::switch(&state, AppType::Claude, "claude-official").expect("to official");
    assert_eq!(claude_live(), json!({ "env": {} }));
}

#[test]
fn switch_claude_refuses_a_settings_file_it_cannot_parse() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let broken = "{\n  \"env\": {\n    \"ANTHROPIC_BASE_URL\": \"https://a.example\",\n}\n";
    let state = seed_claude_switch_state(
        &[
            (
                "a",
                json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example" } }),
            ),
            (
                "b",
                json!({ "env": { "ANTHROPIC_BASE_URL": "https://b.example" } }),
            ),
        ],
        "a",
        broken,
    );
    let path = get_claude_settings_path();
    let before = std::fs::metadata(&path)
        .expect("stat")
        .modified()
        .expect("mtime");

    let err = ProviderService::switch(&state, AppType::Claude, "b").expect_err("must refuse");
    assert!(err.to_string().contains("第 4 行"), "{err}");
    assert_eq!(claude_live_text(), broken);
    assert_eq!(
        std::fs::metadata(&path)
            .expect("stat")
            .modified()
            .expect("mtime"),
        before
    );
    assert_eq!(
        state
            .db
            .get_current_provider(AppType::Claude.as_str())
            .expect("current")
            .as_deref(),
        Some("a")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn switch_writes_credential_files_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let home = ensure_test_home();
    for dir in [".claude", ".codex", ".grok"] {
        std::fs::create_dir_all(home.join(dir)).expect("create client dir");
    }
    // 已有的 settings.json 是 0644：替换写入后也收紧。
    let claude_settings = get_claude_settings_path();
    std::fs::write(&claude_settings, "{}").expect("seed claude settings");
    std::fs::set_permissions(&claude_settings, std::fs::Permissions::from_mode(0o644))
        .expect("chmod 644");

    let mut config = MultiAppConfig::default();
    {
        let claude = config
            .get_manager_mut(&AppType::Claude)
            .expect("claude manager");
        claude.current = "claude-a".to_string();
        for (id, key) in [("claude-a", "sk-a"), ("claude-b", "sk-b")] {
            claude.providers.insert(
                id.to_string(),
                Provider::with_id(
                    id.to_string(),
                    id.to_string(),
                    json!({ "env": { "ANTHROPIC_AUTH_TOKEN": key } }),
                    None,
                ),
            );
        }
    }
    {
        let codex = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        codex.current = "codex-official".to_string();
        let mut official = Provider::with_id(
            "codex-official".to_string(),
            "OpenAI".to_string(),
            json!({ "auth": { "OPENAI_API_KEY": "sk-official" }, "config": "" }),
            None,
        );
        official.category = Some("official".to_string());
        codex
            .providers
            .insert("codex-official".to_string(), official);
        codex.providers.insert(
            "codex-relay".to_string(),
            Provider::with_id(
                "codex-relay".to_string(),
                "Relay".to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": "sk-relay" },
                    "config": "model_provider = \"custom\"\n\n[model_providers.custom]\nname = \"Relay\"\nbase_url = \"https://relay.example/v1\"\nwire_api = \"responses\"\n"
                }),
                None,
            ),
        );
    }
    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .save_provider(
            AppType::GrokBuild.as_str(),
            &Provider::with_id(
                "grok-relay".to_string(),
                "Relay".to_string(),
                json!({ "config": "[models]\ndefault = \"grok-4.5\"\n\n[model.\"grok-4.5\"]\nmodel = \"grok-4.5\"\nbase_url = \"https://relay.example/v1\"\nname = \"Relay\"\napi_key = \"xai-relay\"\napi_backend = \"responses\"\ncontext_window = 500000\n" }),
                None,
            ),
        )
        .expect("save grok provider");

    let mode = |path: &std::path::Path| {
        std::fs::metadata(path)
            .unwrap_or_else(|e| panic!("stat {}: {e}", path.display()))
            .permissions()
            .mode()
            & 0o777
    };

    ProviderService::switch(&state, AppType::Codex, "codex-official").expect("codex official");
    assert!(
        !fyagent_lib::get_codex_auth_path().exists(),
        "source switching must not create auth.json"
    );
    ProviderService::switch(&state, AppType::Codex, "codex-relay").expect("codex relay");
    assert_eq!(mode(&fyagent_lib::get_codex_config_path()), 0o600);
    ProviderService::switch(&state, AppType::Claude, "claude-b").expect("claude b");
    assert_eq!(mode(&claude_settings), 0o600);
    ProviderService::switch(&state, AppType::GrokBuild, "grok-relay").expect("grok relay");
    assert_eq!(mode(&fyagent_lib::get_grok_config_path()), 0o600);
}

fn editor_save(base: &serde_json::Value, on_conflict: &str) -> fyagent_lib::EditorSave {
    serde_json::from_value(json!({ "base": base, "onConflict": on_conflict })).expect("editor save")
}

fn open_claude_editor(state: &fyagent_lib::AppState, id: &str) -> (Provider, serde_json::Value) {
    let row = state
        .db
        .get_provider_by_id(id, AppType::Claude.as_str())
        .expect("read provider")
        .expect("provider exists");
    let view = ProviderService::editor_view(state, AppType::Claude, &row.settings_config, None)
        .expect("editor view");
    (row, view.settings)
}

fn save_claude_editor(
    state: &fyagent_lib::AppState,
    row: &Provider,
    base: &serde_json::Value,
    edited: serde_json::Value,
    on_conflict: &str,
) -> Result<bool, AppError> {
    let mut provider = row.clone();
    provider.settings_config = edited;
    ProviderService::update_from_editor(
        state,
        AppType::Claude,
        None,
        provider,
        Some(editor_save(base, on_conflict)),
    )
}

#[test]
fn claude_editor_global_settings_go_to_live() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let a = json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example" } });
    let b = json!({ "env": { "ANTHROPIC_BASE_URL": "https://b.example" } });
    let state = seed_claude_switch_state(
        &[("a", a.clone()), ("b", b.clone())],
        "a",
        &serde_json::to_string_pretty(&a).expect("serialize"),
    );

    let (row, base) = open_claude_editor(&state, "b");
    assert_eq!(base, b, "b's key fields over the live file");
    let mut edited = base.clone();
    edited["hooks"] =
        json!({ "Stop": [{ "hooks": [{ "type": "command", "command": "say done" }] }] });
    edited["alwaysThinkingEnabled"] = json!(false);
    save_claude_editor(&state, &row, &base, edited, "refuse").expect("save");

    let live = claude_live();
    assert_eq!(
        live["env"]["ANTHROPIC_BASE_URL"],
        json!("https://a.example")
    );
    assert_eq!(live["alwaysThinkingEnabled"], json!(false));
    assert!(live.get("hooks").is_some());
    assert_eq!(
        claude_row(&state, "b"),
        b,
        "global settings stay out of the row"
    );

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch");
    let live = claude_live();
    assert_eq!(
        live["env"]["ANTHROPIC_BASE_URL"],
        json!("https://b.example")
    );
    assert!(live.get("hooks").is_some(), "the hook survives the switch");
}

#[test]
fn claude_editor_provider_fields_go_to_the_row() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let deepseek = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://api.deepseek.example/anthropic",
        "CLAUDE_CODE_DISABLE_ARTIFACT": "1"
    }});
    let qwen = json!({ "env": { "ANTHROPIC_BASE_URL": "https://qwen.example" } });
    let state = seed_claude_switch_state(
        &[("deepseek", deepseek.clone()), ("qwen", qwen)],
        "deepseek",
        &serde_json::to_string_pretty(&deepseek).expect("serialize"),
    );

    // 当前带着禁用 Artifact，编辑另一家：不显示这个键。
    let (row, base) = open_claude_editor(&state, "qwen");
    assert!(base["env"].get("CLAUDE_CODE_DISABLE_ARTIFACT").is_none());
    let mut edited = base.clone();
    edited["env"]["ANTHROPIC_BASE_URL"] = json!("https://qwen-new.example");
    edited["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"] = json!("983616");
    save_claude_editor(&state, &row, &base, edited, "refuse").expect("save qwen");
    assert_eq!(
        claude_live(),
        deepseek,
        "a non-current edit leaves live alone"
    );
    assert_eq!(
        claude_row(&state, "qwen"),
        json!({ "env": {
            "ANTHROPIC_BASE_URL": "https://qwen-new.example",
            "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "983616"
        }})
    );

    let (row, base) = open_claude_editor(&state, "deepseek");
    let mut edited = base.clone();
    edited["env"]
        .as_object_mut()
        .expect("env")
        .remove("CLAUDE_CODE_DISABLE_ARTIFACT");
    save_claude_editor(&state, &row, &base, edited, "refuse").expect("save deepseek");
    assert_eq!(
        claude_live(),
        json!({ "env": { "ANTHROPIC_BASE_URL": "https://api.deepseek.example/anthropic" } })
    );

    ProviderService::switch(&state, AppType::Claude, "qwen").expect("switch");
    assert_eq!(
        claude_live()["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"],
        json!("983616")
    );
}

#[test]
fn claude_editor_lists_row_fields_that_never_reach_live() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let imported = json!({ "env": {
        "ANTHROPIC_BASE_URL": "https://relay.example",
        "API_TIMEOUT_MS": "3000000"
    }});
    let state = seed_claude_switch_state(
        &[("relay", imported.clone())],
        "relay",
        r#"{ "env": { "ANTHROPIC_BASE_URL": "https://relay.example" } }"#,
    );

    let view =
        ProviderService::editor_view(&state, AppType::Claude, &imported, None).expect("view");
    assert_eq!(
        serde_json::to_value(&view.inactive).expect("serialize"),
        json!([{ "path": ["env", "API_TIMEOUT_MS"], "value": "3000000" }])
    );

    let row = state
        .db
        .get_provider_by_id("relay", AppType::Claude.as_str())
        .expect("read")
        .expect("row");
    let mut edited = view.settings.clone();
    edited["env"]["API_TIMEOUT_MS"] = json!("3000000");
    save_claude_editor(&state, &row, &view.settings, edited, "refuse").expect("save");
    assert_eq!(claude_live()["env"]["API_TIMEOUT_MS"], json!("3000000"));
    assert_eq!(claude_row(&state, "relay"), imported);
}

#[test]
fn claude_editor_refuses_to_silently_overwrite_external_edits() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let a = json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example" } });
    let state = seed_claude_switch_state(
        &[("a", a.clone())],
        "a",
        r#"{ "env": { "ANTHROPIC_BASE_URL": "https://a.example" }, "x": 1 }"#,
    );
    let (row, base) = open_claude_editor(&state, "a");

    let mut external = claude_live();
    external["x"] = json!(3);
    std::fs::write(get_claude_settings_path(), external.to_string()).expect("external edit");
    let untouched = claude_live_text();

    let mut edited = base.clone();
    edited["x"] = json!(2);
    edited["y"] = json!("mine");
    let err =
        save_claude_editor(&state, &row, &base, edited.clone(), "refuse").expect_err("conflict");
    let detail: serde_json::Value =
        serde_json::from_str(&err.to_string()).expect("structured error");
    assert_eq!(detail["code"], json!("LIVE_EDIT_CONFLICT"));
    assert_eq!(detail["keys"], json!(["x"]));
    assert_eq!(claude_live_text(), untouched, "nothing written");

    save_claude_editor(&state, &row, &base, edited.clone(), "keepTheirs").expect("theirs");
    assert_eq!(claude_live()["x"], json!(3));
    assert_eq!(claude_live()["y"], json!("mine"));

    save_claude_editor(&state, &row, &base, edited, "keepMine").expect("mine");
    assert_eq!(claude_live()["x"], json!(2));
}

#[test]
fn claude_editor_first_provider_keeps_the_existing_settings() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let settings_path = get_claude_settings_path();
    std::fs::create_dir_all(settings_path.parent().expect("dir")).expect("mkdir");
    let existing = r#"{
  "permissions": {
    "allow": [
      "Bash(git status)"
    ]
  },
  "model": "opus"
}"#;
    std::fs::write(&settings_path, existing).expect("seed live");
    let state = create_test_state().expect("state");

    let base = ProviderService::editor_view(&state, AppType::Claude, &json!({}), None)
        .expect("view")
        .settings;
    assert_eq!(
        base,
        json!({ "permissions": { "allow": ["Bash(git status)"] } }),
        "a new provider starts from live without the key fields"
    );
    let mut edited = base.clone();
    edited["env"] = json!({
        "ANTHROPIC_BASE_URL": "https://relay.example",
        "ANTHROPIC_AUTH_TOKEN": "sk-relay"
    });
    let provider = Provider::with_id("relay".into(), "Relay".into(), edited, None);
    ProviderService::add_from_editor(
        &state,
        AppType::Claude,
        provider,
        true,
        Some(editor_save(&base, "refuse")),
    )
    .expect("add");

    assert_eq!(
        claude_live(),
        json!({
            "permissions": { "allow": ["Bash(git status)"] },
            "env": {
                "ANTHROPIC_BASE_URL": "https://relay.example",
                "ANTHROPIC_AUTH_TOKEN": "sk-relay"
            }
        })
    );
    let row = state
        .db
        .get_provider_by_id("relay", AppType::Claude.as_str())
        .expect("read")
        .expect("row");
    assert_eq!(
        row.settings_config,
        json!({ "env": {
            "ANTHROPIC_BASE_URL": "https://relay.example",
            "ANTHROPIC_AUTH_TOKEN": "sk-relay"
        }})
    );
    assert_eq!(
        row.meta.and_then(|meta| meta.common_config_enabled),
        Some(true)
    );
    assert_eq!(
        state
            .db
            .get_current_provider(AppType::Claude.as_str())
            .expect("current")
            .as_deref(),
        Some("relay")
    );
}

#[test]
fn claude_editor_leaves_exclusive_fields_from_live_to_the_user() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let a = json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example" } });
    let b = json!({ "env": { "ANTHROPIC_BASE_URL": "https://b.example" } });
    let state = seed_claude_switch_state(
        &[("a", a.clone()), ("b", b)],
        "a",
        r#"{ "env": { "ANTHROPIC_BASE_URL": "https://a.example", "ENABLE_TOOL_SEARCH": "true", "DISABLE_INTERLEAVED_THINKING": "1" } }"#,
    );

    // 只改名、配置原样保存。
    let (mut row, base) = open_claude_editor(&state, "a");
    assert_eq!(base["env"]["ENABLE_TOOL_SEARCH"], json!("true"));
    row.name = "renamed".into();
    save_claude_editor(&state, &row, &base, base.clone(), "refuse").expect("save as is");
    assert_eq!(
        claude_row(&state, "a"),
        a,
        "live's exclusive fields stay out of the row"
    );

    // 删掉一个从 live 带进来的，再加一个供应商自己的。
    let (row, base) = open_claude_editor(&state, "a");
    let mut edited = base.clone();
    edited["env"]
        .as_object_mut()
        .unwrap()
        .remove("DISABLE_INTERLEAVED_THINKING");
    edited["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("1");
    save_claude_editor(&state, &row, &base, edited, "refuse").expect("save edits");
    let live = claude_live();
    assert!(
        live["env"].get("DISABLE_INTERLEAVED_THINKING").is_none(),
        "{live}"
    );
    assert_eq!(live["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], json!("1"));
    let a_row = claude_row(&state, "a");
    assert_eq!(a_row["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], json!("1"));
    assert!(a_row["env"].get("ENABLE_TOOL_SEARCH").is_none(), "{a_row}");

    ProviderService::switch(&state, AppType::Claude, "b").expect("switch to b");
    let live = claude_live();
    assert_eq!(live["env"]["ENABLE_TOOL_SEARCH"], json!("true"), "{live}");
    assert!(
        live["env"].get("CLAUDE_CODE_DISABLE_ARTIFACT").is_none(),
        "{live}"
    );
}

#[test]
fn claude_editor_keeps_a_legacy_bedrock_key_where_older_versions_read_it() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let legacy = json!({
        "apiKey": "bedrock-key",
        "env": { "CLAUDE_CODE_USE_BEDROCK": "1", "AWS_REGION": "us-east-1" }
    });
    let state = seed_claude_switch_state(&[("bedrock", legacy.clone())], "bedrock", "{}");
    ProviderService::switch(&state, AppType::Claude, "bedrock").expect("switch");

    let (row, base) = open_claude_editor(&state, "bedrock");
    assert_eq!(
        base["env"]["AWS_BEARER_TOKEN_BEDROCK"],
        json!("bedrock-key")
    );
    save_claude_editor(&state, &row, &base, base.clone(), "refuse").expect("save as is");
    assert_eq!(claude_row(&state, "bedrock"), legacy);

    let (row, base) = open_claude_editor(&state, "bedrock");
    let mut edited = base.clone();
    edited["env"]["AWS_BEARER_TOKEN_BEDROCK"] = json!("rotated-key");
    save_claude_editor(&state, &row, &base, edited, "refuse").expect("rotate the key");
    let stored = claude_row(&state, "bedrock");
    assert_eq!(stored["apiKey"], json!("rotated-key"), "{stored}");
    assert!(stored["env"].get("AWS_BEARER_TOKEN_BEDROCK").is_none());
    assert_eq!(
        claude_live()["env"]["AWS_BEARER_TOKEN_BEDROCK"],
        json!("rotated-key")
    );
}

#[test]
fn gemini_editor_recognizes_a_legacy_google_card_like_switching_does() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let home = ensure_test_home();

    let state = create_test_state().expect("create test state");
    let mut google = Provider::with_id(
        "google".to_string(),
        "Google".to_string(),
        json!({ "env": {} }),
        None,
    );
    google.category = None;
    state
        .db
        .save_provider(AppType::Gemini.as_str(), &google)
        .expect("save legacy google card");

    let category = ProviderService::editor_category(&state, &AppType::Gemini, Some("google"), None)
        .expect("category");
    assert_eq!(category.as_deref(), Some("official"));
    let shown = ProviderService::editor_view(
        &state,
        AppType::Gemini,
        &google.settings_config,
        category.as_deref(),
    )
    .expect("view")
    .settings;
    ProviderService::switch(&state, AppType::Gemini, "google").expect("switch");
    let live = read_json_file::<serde_json::Value>(&home.join(".gemini/settings.json"))
        .expect("read gemini settings");
    assert_eq!(
        shown.pointer("/config/security/auth/selectedType"),
        live.pointer("/security/auth/selectedType"),
    );
    assert_eq!(
        live.pointer("/security/auth/selectedType"),
        Some(&json!("oauth-personal"))
    );

    assert_eq!(
        ProviderService::editor_category(&state, &AppType::Gemini, None, Some("third".into()))
            .expect("category for a draft"),
        Some("third".to_string())
    );
}

#[test]
fn claude_add_dialog_keeps_the_users_exclusive_fields_out_of_the_new_row() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let a = json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example" } });
    let state = seed_claude_switch_state(
        &[("a", a)],
        "a",
        r#"{ "env": { "ANTHROPIC_BASE_URL": "https://a.example", "ENABLE_TOOL_SEARCH": "true" } }"#,
    );
    let base = ProviderService::editor_view(&state, AppType::Claude, &json!({}), None)
        .expect("view")
        .settings;
    let mut edited = base.clone();
    edited["env"]["ANTHROPIC_BASE_URL"] = json!("https://relay.example");
    edited["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("1");
    let provider = Provider::with_id("relay".into(), "Relay".into(), edited, None);
    ProviderService::add_from_editor(
        &state,
        AppType::Claude,
        provider,
        true,
        Some(editor_save(&base, "refuse")),
    )
    .expect("add");

    let row = claude_row(&state, "relay");
    assert_eq!(row["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], json!("1"));
    assert!(row["env"].get("ENABLE_TOOL_SEARCH").is_none(), "{row}");
}

#[test]
fn claude_editor_never_leaves_the_row_and_live_apart() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let home = ensure_test_home();

    let a = json!({ "env": { "ANTHROPIC_BASE_URL": "https://a.example", "ANTHROPIC_AUTH_TOKEN": "sk-old" } });
    let state = seed_claude_switch_state(
        &[("a", a.clone())],
        "a",
        r#"{ "env": { "ANTHROPIC_BASE_URL": "https://a.example", "ANTHROPIC_AUTH_TOKEN": "sk-old" }, "x": 1 }"#,
    );
    let before = claude_live_text();

    // 行存不进去。
    let (row, base) = open_claude_editor(&state, "a");
    let mut edited = base.clone();
    edited["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("sk-new");
    let db = rusqlite::Connection::open(home.join(".fyagent/fyagent.db")).expect("open db");
    db.execute_batch(
        "CREATE TRIGGER fail_edit BEFORE UPDATE OF settings_config ON providers \
         BEGIN SELECT RAISE(ABORT, 'injected save failure'); END;",
    )
    .expect("trigger");
    save_claude_editor(&state, &row, &base, edited.clone(), "refuse").expect_err("save fails");
    db.execute_batch("DROP TRIGGER fail_edit;")
        .expect("drop trigger");
    assert_eq!(claude_live_text(), before, "live untouched");
    assert_eq!(claude_row(&state, "a"), a);

    // live 写不进去（外部改了同一个全局键）。
    std::fs::write(
        get_claude_settings_path(),
        before.replace("\"x\": 1", "\"x\": 3"),
    )
    .expect("external edit");
    edited["x"] = json!(2);
    save_claude_editor(&state, &row, &base, edited, "refuse").expect_err("conflict");
    assert_eq!(claude_row(&state, "a"), a, "the saved row is taken back");
    assert_eq!(
        claude_live()["env"]["ANTHROPIC_AUTH_TOKEN"],
        json!("sk-old")
    );
}

#[test]
fn source_switch_preserves_native_auth_marker_and_stash_bytes_with_preservation_disabled() {
    let _guard = test_mutex().lock().expect("test mutex");
    reset_test_fs();
    let home = ensure_test_home();
    fyagent_lib::update_settings(fyagent_lib::AppSettings {
        preserve_codex_official_auth_on_switch: false,
        ..Default::default()
    })
    .expect("disable legacy preservation preference");

    let mut config = MultiAppConfig::default();
    let manager = config.get_manager_mut(&AppType::Codex).unwrap();
    for id in ["source-a", "source-b"] {
        manager.providers.insert(id.into(), Provider::with_id(
            id.into(), id.into(), json!({
                "auth": {"OPENAI_API_KEY": format!("key-{id}")},
                "config": format!("model_provider = \"custom\"\nmodel = \"test-model\"\n[model_providers.custom]\nname = \"{id}\"\nbase_url = \"https://{id}.example/v1\"\nwire_api = \"responses\"\n")
            }), None,
        ));
    }
    let state = create_test_state_with_config(&config).expect("state");
    let auth_path = fyagent_lib::get_codex_auth_path();
    std::fs::create_dir_all(auth_path.parent().unwrap()).unwrap();
    let auth = b"{\n  \"tokens\": {\"access_token\":\"native-token\",\"account_id\":\"native-account\"},\n  \"unknown\": true\n}\n";
    std::fs::write(&auth_path, auth).unwrap();
    let device = home.join(".fyagent");
    std::fs::create_dir_all(&device).unwrap();
    let marker = device.join("codex_managed_oauth_live_auth.json");
    let stash = device.join("codex-login-stash.json");
    let marker_bytes = b"{\n  \"retained_fixture\": true\n}\n";
    let stash_bytes = b"{\n  \"logins\": {}, \"last\": null\n}\n";
    std::fs::write(&marker, marker_bytes).unwrap();
    std::fs::write(&stash, stash_bytes).unwrap();
    std::fs::write(
        fyagent_lib::get_codex_config_path(),
        "# user settings\n[mcp_servers.keep]\ncommand = \"echo\"\n",
    )
    .unwrap();

    for id in ["source-a", "source-b"] {
        ProviderService::switch(&state, AppType::Codex, id).expect("switch source");
        assert_eq!(std::fs::read(&auth_path).unwrap(), auth);
        assert_eq!(std::fs::read(&marker).unwrap(), marker_bytes);
        assert_eq!(std::fs::read(&stash).unwrap(), stash_bytes);
        let written = std::fs::read_to_string(fyagent_lib::get_codex_config_path()).unwrap();
        assert!(
            written.contains(&format!("https://{id}.example/v1")),
            "{written}"
        );
        assert!(written.contains("[mcp_servers.keep]"), "{written}");
    }
}

#[test]
fn switch_claude_preserves_credential_missing_from_previous_row() {
    assert_claude_switch_preserves_unmanaged_credential(None);
}

#[test]
fn switch_claude_preserves_credential_differing_from_previous_row() {
    assert_claude_switch_preserves_unmanaged_credential(Some("saved-old-key"));
}

fn assert_claude_switch_preserves_unmanaged_credential(saved_key: Option<&str>) {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let path = get_claude_settings_path();
    std::fs::create_dir_all(path.parent().unwrap()).expect("create claude dir");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "env": {"ANTHROPIC_API_KEY": "a-key", "OPENROUTER_API_KEY": "sk-or-leak"},
            "theme": "dark"
        }))
        .unwrap(),
    )
    .expect("seed live");
    let mut config = MultiAppConfig::default();
    let manager = config.get_manager_mut(&AppType::Claude).unwrap();
    manager.current = "a".into();
    let mut saved = json!({"env": {"ANTHROPIC_API_KEY": "a-key"}});
    if let Some(key) = saved_key {
        saved["env"]["OPENROUTER_API_KEY"] = json!(key);
    }
    for (id, settings) in [
        ("a", saved),
        ("b", json!({"env": {"ANTHROPIC_API_KEY": "b-key"}})),
    ] {
        let mut provider = Provider::with_id(id.into(), id.into(), settings, None);
        provider.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            ..Default::default()
        });
        manager.providers.insert(id.into(), provider);
    }
    let state = create_test_state_with_config(&config).expect("create state");
    state
        .db
        .set_config_snippet("claude", Some(r#"{"theme":"dark"}"#.into()))
        .unwrap();
    ProviderService::switch(&state, AppType::Claude, "b").expect("switch");
    let live: serde_json::Value = read_json_file(&path).unwrap();
    let snippet = state.db.get_config_snippet("claude").unwrap().unwrap();
    let shared: serde_json::Value = serde_json::from_str(&snippet).unwrap();
    assert_eq!(
        live.pointer("/env/OPENROUTER_API_KEY"),
        Some(&json!("sk-or-leak"))
    );
    assert!(shared.pointer("/env/OPENROUTER_API_KEY").is_none());
    assert!(live.to_string().contains("sk-or-leak"));
    assert!(!snippet.contains("sk-or-leak"));
    assert_eq!(live["env"]["ANTHROPIC_API_KEY"], "b-key");
    assert_eq!(
        state.db.get_current_provider("claude").unwrap().as_deref(),
        Some("b")
    );
}

#[test]
fn switch_codex_prepublication_failure_restores_common_config_snippet() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    // 片段里有两个共享键，但用户已在 live 里删掉 disable_response_storage
    let live_config = r#"model_provider = "aprov"

[tui]
notifications = true

[model_providers.aprov]
name = "A Prov"
base_url = "https://a.example/v1"
wire_api = "responses"
"#;
    write_codex_live_atomic(&json!({ "OPENAI_API_KEY": "sk-a" }), Some(live_config))
        .expect("seed codex live config");

    let mut config = MultiAppConfig::default();
    {
        let manager = config
            .get_manager_mut(&AppType::Codex)
            .expect("codex manager");
        manager.current = "a".to_string();
        for (id, name, prov_key) in [("a", "A", "aprov"), ("b", "B", "bprov")] {
            let mut provider = Provider::with_id(
                id.to_string(),
                name.to_string(),
                json!({
                    "auth": { "OPENAI_API_KEY": format!("sk-{id}") },
                    "config": format!("model_provider = \"{prov_key}\"\n\n[model_providers.{prov_key}]\nname = \"{name} Prov\"\nbase_url = \"https://{id}.example/v1\"\nwire_api = \"responses\"\n")
                }),
                None,
            );
            provider.meta = Some(ProviderMeta {
                common_config_enabled: Some(true),
                ..Default::default()
            });
            manager.providers.insert(id.to_string(), provider);
        }
    }

    let state = create_test_state_with_config(&config).expect("create test state");
    state
        .db
        .set_config_snippet(
            AppType::Codex.as_str(),
            Some("disable_response_storage = true\n\n[tui]\nnotifications = true\n".to_string()),
        )
        .expect("seed codex common config snippet");

    let original_snippet = state
        .db
        .get_config_snippet(AppType::Codex.as_str())
        .expect("read original snippet");
    let before = std::fs::read(get_codex_config_path()).unwrap();
    // 阻塞首写备份目录：计划和片段同步已完成，但第一个文件尚未发布。
    let store = fyagent_lib::live::engine::DeviceStore::for_device();
    let backup_dir = store.first_write_backup_dir();
    std::fs::create_dir_all(backup_dir.parent().unwrap()).unwrap();
    std::fs::write(&backup_dir, b"block backup directory").unwrap();
    ProviderService::switch(&state, AppType::Codex, "b").expect_err("backup must fail");
    let pending = fyagent_lib::mode::state::pending(&store, "codex")
        .unwrap()
        .unwrap();
    assert!(!pending.published);
    assert!(!pending
        .target
        .common_config_snippet
        .as_ref()
        .unwrap()
        .contains("disable_response_storage"));
    assert_eq!(
        state.db.get_config_snippet("codex").unwrap(),
        original_snippet
    );
    assert_eq!(std::fs::read(get_codex_config_path()).unwrap(), before);
    assert_eq!(
        state.db.get_current_provider("codex").unwrap().as_deref(),
        Some("a")
    );
    assert_eq!(
        fyagent_lib::mode::operation::settle(&state.db, "codex").unwrap(),
        Some(fyagent_lib::mode::operation::RecoveryOutcome::Discarded)
    );
    assert_eq!(
        state.db.get_config_snippet("codex").unwrap(),
        original_snippet
    );
    assert_eq!(std::fs::read(get_codex_config_path()).unwrap(), before);
}
