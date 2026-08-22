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

    use super::merge_json;

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
