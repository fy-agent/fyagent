use std::collections::HashMap;

use serde_json::Value;

use crate::error::AppError;
use crate::provider::UniversalProvider;
use crate::store::AppState;

use super::ProviderService;

impl ProviderService {
    pub fn list_universal(
        state: &AppState,
    ) -> Result<HashMap<String, UniversalProvider>, AppError> {
        state.db.get_all_universal_providers()
    }

    pub fn get_universal(
        state: &AppState,
        id: &str,
    ) -> Result<Option<UniversalProvider>, AppError> {
        state.db.get_universal_provider(id)
    }

    pub fn upsert_universal(
        state: &AppState,
        provider: UniversalProvider,
    ) -> Result<bool, AppError> {
        state.db.save_universal_provider(&provider)?;
        Ok(true)
    }

    pub fn delete_universal(state: &AppState, id: &str) -> Result<bool, AppError> {
        let provider = state.db.get_universal_provider(id)?;
        state.db.delete_universal_provider(id)?;

        if let Some(provider) = provider {
            for (enabled, app, child_id) in [
                (
                    provider.apps.claude,
                    "claude",
                    format!("universal-claude-{id}"),
                ),
                (
                    provider.apps.codex,
                    "codex",
                    format!("universal-codex-{id}"),
                ),
                (
                    provider.apps.gemini,
                    "gemini",
                    format!("universal-gemini-{id}"),
                ),
            ] {
                if enabled {
                    let _ = state.db.delete_provider(app, &child_id);
                }
            }
        }

        Ok(true)
    }

    pub fn sync_universal_to_apps(state: &AppState, id: &str) -> Result<bool, AppError> {
        let provider = state
            .db
            .get_universal_provider(id)?
            .ok_or_else(|| AppError::Message(format!("统一供应商 {id} 不存在")))?;

        sync_projection(
            state,
            "claude",
            format!("universal-claude-{id}"),
            provider.to_claude_provider(),
        )?;
        sync_projection(
            state,
            "codex",
            format!("universal-codex-{id}"),
            provider.to_codex_provider(),
        )?;
        sync_projection(
            state,
            "gemini",
            format!("universal-gemini-{id}"),
            provider.to_gemini_provider(),
        )?;

        Ok(true)
    }
}

fn sync_projection(
    state: &AppState,
    app: &str,
    child_id: String,
    projected: Option<crate::provider::Provider>,
) -> Result<(), AppError> {
    let Some(mut projected) = projected else {
        let _ = state.db.delete_provider(app, &child_id);
        return Ok(());
    };

    if let Some(existing) = state.db.get_provider_by_id(&projected.id, app)? {
        let mut merged = existing.settings_config.clone();
        merge_json(&mut merged, &projected.settings_config);
        projected.settings_config = merged;
        // Existing cards own their app-specific behavior and placement.
        projected.meta = existing.meta;
        projected.created_at = existing.created_at;
        projected.sort_index = existing.sort_index;
    }
    state.db.save_provider(app, &projected)
}

fn merge_json(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_json(base_value, patch_value),
                    None => {
                        base_map.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
        (base_value, patch_value) => *base_value = patch_value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::sync::Arc;

    use super::*;
    use crate::database::Database;
    use crate::provider::{Provider, ProviderMeta};
    use crate::services::provider::ProviderCredentials;

    fn universal_fixture() -> UniversalProvider {
        let mut provider = UniversalProvider::new(
            "shared".into(),
            "Shared updated name".into(),
            "newapi".into(),
            "https://shared.example".into(),
            "shared-fixture-key".into(),
        );
        provider.apps.claude = true;
        provider.apps.codex = true;
        provider.apps.gemini = true;
        provider.created_at = Some(900);
        provider.sort_index = Some(90);
        provider.website_url = Some("https://shared.example/about".into());
        provider.notes = Some("Shared updated notes".into());
        provider.icon = Some("shared-icon".into());
        provider.icon_color = Some("#112233".into());
        provider.meta = Some(ProviderMeta {
            common_config_enabled: Some(true),
            endpoint_auto_select: Some(false),
            ..Default::default()
        });
        provider
    }

    fn child_projections(provider: &UniversalProvider) -> [(&str, Provider); 3] {
        [
            ("claude", provider.to_claude_provider().unwrap()),
            ("codex", provider.to_codex_provider().unwrap()),
            ("gemini", provider.to_gemini_provider().unwrap()),
        ]
    }

    #[test]
    fn compat_config_universal_sync_preserves_existing_child_metadata() {
        let state = AppState::new(Arc::new(Database::memory().unwrap()));
        let mut parent = universal_fixture();
        let mut previous = parent.clone();
        previous.name = "Old child name".into();
        previous.base_url = "https://previous.example".into();
        previous.api_key = "previous-fixture-key".into();
        previous.website_url = None;
        previous.notes = Some("Old child notes".into());
        previous.icon = None;
        previous.icon_color = None;
        for (app, mut child) in child_projections(&previous) {
            child.created_at = Some(123);
            child.sort_index = Some(4);
            child.in_failover_queue = true;
            child.settings_config["childOnly"] = json!({"keep": app});
            child.meta = Some(ProviderMeta {
                common_config_enabled: Some(false),
                endpoint_auto_select: Some(true),
                usage_script: Some(
                    serde_json::from_value(json!({
                        "enabled": true,
                        "language": "javascript",
                        "code": "return { remaining: 12 };"
                    }))
                    .unwrap(),
                ),
                ..Default::default()
            });
            state.db.save_provider(app, &child).unwrap();
            state.db.set_current_provider(app, &child.id).unwrap();
        }
        // Neither a different parent metadata object nor an absent one may
        // replace app-owned usage scripts and independent card settings.
        for keep_parent_meta in [true, false] {
            if !keep_parent_meta {
                parent.meta = None;
            }
            state.db.save_universal_provider(&parent).unwrap();
            ProviderService::sync_universal_to_apps(&state, &parent.id).unwrap();
            for (app, projected) in child_projections(&parent) {
                let child = state
                    .db
                    .get_provider_by_id(&projected.id, app)
                    .unwrap()
                    .unwrap();
                assert_eq!(child.created_at, Some(123), "{app}");
                assert_eq!(child.sort_index, Some(4), "{app}");
                let meta = child.meta.as_ref().unwrap();
                assert_eq!(meta.common_config_enabled, Some(false), "{app}");
                assert_eq!(meta.endpoint_auto_select, Some(true), "{app}");
                assert_eq!(
                    meta.usage_script.as_ref().unwrap().code,
                    "return { remaining: 12 };"
                );
                assert_eq!(child.name, projected.name);
                assert_eq!(child.website_url, projected.website_url);
                assert_eq!(child.category, projected.category);
                assert_eq!(child.notes, projected.notes);
                assert_eq!(child.icon, projected.icon);
                assert_eq!(child.icon_color, projected.icon_color);
                assert_eq!(child.settings_config["childOnly"], json!({"keep": app}));
                assert_projected_settings_preserved(app, &child, &projected, &state.db);
                assert!(child.in_failover_queue);
                assert_eq!(
                    state.db.get_current_provider(app).unwrap().as_deref(),
                    Some(child.id.as_str())
                );
            }
        }
    }

    #[test]
    fn compat_config_universal_sync_initializes_new_child_metadata() {
        let state = AppState::new(Arc::new(Database::memory().unwrap()));
        let parent = universal_fixture();
        state.db.save_universal_provider(&parent).unwrap();
        ProviderService::sync_universal_to_apps(&state, &parent.id).unwrap();
        for (app, projected) in child_projections(&parent) {
            let child = state
                .db
                .get_provider_by_id(&projected.id, app)
                .unwrap()
                .unwrap();
            assert_eq!(child.id, projected.id);
            assert_eq!(child.name, projected.name);
            assert_eq!(child.website_url, projected.website_url);
            assert_eq!(child.category, projected.category);
            assert_eq!(child.notes, projected.notes);
            assert_eq!(child.icon, projected.icon);
            assert_eq!(child.icon_color, projected.icon_color);
            assert_eq!(child.created_at, projected.created_at);
            assert_eq!(child.sort_index, projected.sort_index);
            assert_eq!(
                serde_json::to_value(&child.meta).unwrap(),
                serde_json::to_value(&projected.meta).unwrap()
            );
            assert_eq!(child.in_failover_queue, projected.in_failover_queue);
            assert_projected_settings_preserved(app, &child, &projected, &state.db);
        }
    }

    fn assert_projected_settings_preserved(
        app: &str,
        child: &Provider,
        projected: &Provider,
        db: &Database,
    ) {
        for (key, value) in projected.settings_config.as_object().unwrap() {
            if app == "codex" && key == "auth" {
                assert_eq!(&child.settings_config[key], &json!({}), "{app}/{key}");
                continue;
            }
            assert_eq!(&child.settings_config[key], value, "{app}/{key}");
        }
        if app != "codex" {
            return;
        }
        let reference = child.settings_config["credentialRef"]
            .as_str()
            .expect("Codex public child stores SecretRef");
        assert!(reference.starts_with("pc_"), "{app} {reference}");
        let public = serde_json::to_string(child).unwrap();
        for secret in ["shared-fixture-key", "previous-fixture-key"] {
            assert!(!public.contains(secret), "{app} leaked {secret}");
        }
        assert_eq!(
            ProviderCredentials::resolve(db, "codex", child)
                .unwrap()
                .settings_config["auth"]["OPENAI_API_KEY"],
            projected.settings_config["auth"]["OPENAI_API_KEY"]
        );
    }

    #[test]
    fn merge_json_preserves_unknown_nested_fields_and_overrides_patch_values() {
        let mut base = json!({
            "options": {
                "baseURL": "https://old.example",
                "unknown": true
            },
            "models": ["legacy"]
        });
        let patch = json!({
            "options": {
                "baseURL": "https://new.example",
                "apiKey": "secret"
            },
            "models": ["new"]
        });

        merge_json(&mut base, &patch);

        assert_eq!(
            base,
            json!({
                "options": {
                    "baseURL": "https://new.example",
                    "unknown": true,
                    "apiKey": "secret"
                },
                "models": ["new"]
            })
        );
    }
}
