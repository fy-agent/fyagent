//! Strict, shape-preserving WorkBuddy JSON document handling.
//!
//! This module is the only owner of the untyped `models.json` contract. It
//! validates an entire existing document before a mutation is considered and
//! keeps the original `serde_json::Value` tree so unknown fields, object-key
//! order, model order, and duplicate historical entries survive a save.

use std::collections::HashSet;

use serde_json::{Map, Value};

use super::{
    credential_matches_model_id,
    error::{WorkBuddyError, WorkBuddyErrorCode},
    types::WorkBuddyConfigFormat,
};

pub(crate) const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct WorkBuddyDocument {
    root: Value,
    format: WorkBuddyConfigFormat,
}

impl WorkBuddyDocument {
    pub(crate) fn missing() -> Self {
        let mut root = Map::new();
        root.insert("models".to_string(), Value::Array(Vec::new()));
        Self {
            root: Value::Object(root),
            format: WorkBuddyConfigFormat::Missing,
        }
    }

    pub(crate) fn parse(mut root: Value) -> Result<Self, WorkBuddyError> {
        let format = match &mut root {
            Value::Array(_) => WorkBuddyConfigFormat::LegacyArray,
            Value::Object(object) => {
                // A missing models field is explicitly an empty collection.
                // Insert it only in memory; it reaches disk solely after a
                // later successful save, preserving read-only behavior.
                let models = object
                    .entry("models".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if !models.is_array() {
                    return Err(WorkBuddyError::new(
                        WorkBuddyErrorCode::ConfigModelsNotArray,
                    ));
                }
                WorkBuddyConfigFormat::ObjectRoot
            }
            _ => {
                return Err(WorkBuddyError::new(
                    WorkBuddyErrorCode::ConfigRootUnsupported,
                ))
            }
        };

        let document = Self { root, format };
        document.validate_models()?;
        Ok(document)
    }

    pub(crate) const fn format(&self) -> WorkBuddyConfigFormat {
        self.format
    }

    pub(crate) fn unique_model_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for entry in self.models() {
            let id = entry
                .get("id")
                .and_then(Value::as_str)
                .expect("validated model entries always have a string id")
                .trim();
            if seen.insert(id.to_owned()) {
                ids.push(id.to_owned());
            }
        }
        ids
    }

    pub(crate) fn existing_target_ids(&self, target_ids: &[String]) -> Vec<String> {
        let existing = self
            .models()
            .iter()
            .filter_map(|entry| entry.get("id").and_then(Value::as_str))
            .map(str::trim)
            .collect::<HashSet<_>>();

        target_ids
            .iter()
            .filter(|id| existing.contains(id.as_str()))
            .cloned()
            .collect()
    }

    pub(crate) fn models(&self) -> &Vec<Value> {
        match &self.root {
            Value::Array(models) => models,
            Value::Object(root) => root
                .get("models")
                .and_then(Value::as_array)
                .expect("document validation guarantees an object-root models array"),
            _ => unreachable!("validated document roots are arrays or objects"),
        }
    }

    pub(crate) fn models_mut(&mut self) -> &mut Vec<Value> {
        match &mut self.root {
            Value::Array(models) => models,
            Value::Object(root) => root
                .get_mut("models")
                .and_then(Value::as_array_mut)
                .expect("document validation guarantees an object-root models array"),
            _ => unreachable!("validated document roots are arrays or objects"),
        }
    }

    /// Apply the object-root-only `availableModels` rule before serializing.
    /// Missing and empty fields intentionally remain untouched. A populated
    /// field must be a string list; malformed values are never repaired.
    pub(crate) fn update_available_models(
        &mut self,
        target_ids: &[String],
    ) -> Result<(), WorkBuddyError> {
        if self.format != WorkBuddyConfigFormat::ObjectRoot {
            return Ok(());
        }

        let Value::Object(root) = &mut self.root else {
            unreachable!("an object-root document has an object root");
        };
        let Some(value) = root.get_mut("availableModels") else {
            return Ok(());
        };
        let Value::Array(available_models) = value else {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry));
        };
        if available_models.is_empty() {
            return Ok(());
        }
        if !available_models.iter().all(Value::is_string) {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry));
        }

        let mut present = available_models
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        for target_id in target_ids {
            if present.insert(target_id.clone()) {
                available_models.push(Value::String(target_id.clone()));
            }
        }
        Ok(())
    }

    pub(crate) fn remove_models(&mut self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let remove = ids.iter().map(String::as_str).collect::<HashSet<_>>();
        self.models_mut().retain(|entry| {
            entry
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(|id| !remove.contains(id))
                .unwrap_or(true)
        });
    }

    /// Drop removed IDs from a populated object-root `availableModels` list.
    /// Missing and empty fields stay untouched; malformed values still fail.
    pub(crate) fn prune_available_models(
        &mut self,
        removed_ids: &[String],
    ) -> Result<(), WorkBuddyError> {
        if removed_ids.is_empty() || self.format != WorkBuddyConfigFormat::ObjectRoot {
            return Ok(());
        }

        let Value::Object(root) = &mut self.root else {
            unreachable!("an object-root document has an object root");
        };
        let Some(value) = root.get_mut("availableModels") else {
            return Ok(());
        };
        let Value::Array(available_models) = value else {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry));
        };
        if available_models.is_empty() {
            return Ok(());
        }
        if !available_models.iter().all(Value::is_string) {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry));
        }

        let remove = removed_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        available_models
            .retain(|value| value.as_str().is_some_and(|id| !remove.contains(id.trim())));
        Ok(())
    }

    pub(crate) fn serialize(&self) -> Result<Vec<u8>, WorkBuddyError> {
        let mut serialized = serde_json::to_vec_pretty(&self.root)
            .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed))?;
        serialized.push(b'\n');
        if serialized.len() as u64 > MAX_CONFIG_BYTES {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigWriteFailed));
        }
        Ok(serialized)
    }

    fn validate_models(&self) -> Result<(), WorkBuddyError> {
        let mut ids = Vec::new();
        let mut credentials = Vec::new();
        for (index, entry) in self.models().iter().enumerate() {
            let model = entry.as_object().ok_or_else(|| {
                WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry)
                    .with_invalid_entry_index(index)
            })?;
            let id = model
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry)
                        .with_invalid_entry_index(index)
                })?;
            ids.push(id);
            if let Some(api_key) = model
                .get("apiKey")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                credentials.push(api_key);
            }
        }

        if ids.iter().any(|id| {
            credentials
                .iter()
                .any(|credential| credential_matches_model_id(credential, id))
        }) {
            return Err(WorkBuddyError::new(WorkBuddyErrorCode::ConfigInvalidEntry));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_root_missing_models_is_accepted_without_losing_unknown_top_level_fields() {
        let document = WorkBuddyDocument::parse(serde_json::json!({
            "theme": "dark",
            "future": { "enabled": true }
        }))
        .unwrap();

        assert_eq!(document.format(), WorkBuddyConfigFormat::ObjectRoot);
        assert!(document.models().is_empty());
        let serialized: Value = serde_json::from_slice(&document.serialize().unwrap()).unwrap();
        assert_eq!(serialized["theme"], "dark");
        assert_eq!(serialized["future"], serde_json::json!({ "enabled": true }));
        assert_eq!(serialized["models"], serde_json::json!([]));
    }

    #[test]
    fn unique_ids_trim_and_deduplicate_without_changing_case_or_first_order() {
        let document = WorkBuddyDocument::parse(serde_json::json!([
            { "id": " model-a " },
            { "id": "model-a" },
            { "id": "Model-A" }
        ]))
        .unwrap();

        assert_eq!(document.unique_model_ids(), ["model-a", "Model-A"]);
        assert_eq!(
            document.existing_target_ids(&[
                "model-a".to_string(),
                "Model-A".to_string(),
                "model-b".to_string(),
            ]),
            ["model-a", "Model-A"]
        );
    }

    #[test]
    fn rejects_any_model_id_that_matches_any_trimmed_document_api_key() {
        let credential = "TEST-SECRET-DOCUMENT-KEY";
        let error = WorkBuddyDocument::parse(serde_json::json!([
            { "id": "safe-model", "apiKey": format!(" {credential} ") },
            { "id": format!(" {credential} "), "apiKey": "other-key" }
        ]))
        .unwrap_err();

        let serialized = serde_json::to_string(&error.to_dto()).unwrap();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigInvalidEntry);
        assert!(!serialized.contains(credential));
    }

    #[test]
    fn malformed_root_models_and_entries_are_rejected_without_repair() {
        assert_eq!(
            WorkBuddyDocument::parse(Value::String("bad root".to_string()))
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::ConfigRootUnsupported
        );
        assert_eq!(
            WorkBuddyDocument::parse(serde_json::json!({ "models": {} }))
                .unwrap_err()
                .code(),
            WorkBuddyErrorCode::ConfigModelsNotArray
        );
        let error = WorkBuddyDocument::parse(serde_json::json!({
            "models": [{ "id": "valid" }, 1]
        }))
        .unwrap_err();
        assert_eq!(error.code(), WorkBuddyErrorCode::ConfigInvalidEntry);
        assert_eq!(error.to_dto().details.invalid_entry_index, Some(1));
    }

    #[test]
    fn populated_available_models_appends_only_missing_target_ids() {
        let mut document = WorkBuddyDocument::parse(serde_json::json!({
            "models": [],
            "availableModels": ["kept", "model-a"]
        }))
        .unwrap();

        document
            .update_available_models(&["model-a".to_string(), "model-b".to_string()])
            .unwrap();
        let serialized: Value = serde_json::from_slice(&document.serialize().unwrap()).unwrap();
        assert_eq!(
            serialized["availableModels"],
            serde_json::json!(["kept", "model-a", "model-b"])
        );
    }

    #[test]
    fn remove_models_preserves_remaining_order_and_prunes_available_models() {
        let mut document = WorkBuddyDocument::parse(serde_json::json!({
            "models": [
                { "id": "keep-a", "name": "A" },
                { "id": "drop-me", "name": "B" },
                { "id": "keep-b", "name": "C" }
            ],
            "availableModels": ["keep-a", "drop-me", "keep-b", "other"]
        }))
        .unwrap();

        document.remove_models(&["drop-me".to_string()]);
        document
            .prune_available_models(&["drop-me".to_string()])
            .unwrap();
        let serialized: Value = serde_json::from_slice(&document.serialize().unwrap()).unwrap();
        assert_eq!(
            serialized["models"],
            serde_json::json!([
                { "id": "keep-a", "name": "A" },
                { "id": "keep-b", "name": "C" }
            ])
        );
        assert_eq!(
            serialized["availableModels"],
            serde_json::json!(["keep-a", "keep-b", "other"])
        );
    }
}
