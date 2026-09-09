//! Metadata projection stays beside the WorkBuddy document/storage owners.

use super::{
    config::{current_paths, load_readonly_config_at, LoadedConfig},
    error::{WorkBuddyError, WorkBuddyErrorCode},
    url::{normalize_workbuddy_base_url, reject_url_credential_collision},
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WorkBuddyHealthMetadata {
    pub exists: bool,
    pub model_count: usize,
    pub entry_count: usize,
    pub credentials_present: usize,
    pub endpoints_present: usize,
    pub endpoints_valid: usize,
}

pub(crate) async fn health_metadata() -> Result<WorkBuddyHealthMetadata, WorkBuddyError> {
    let paths = current_paths();
    tokio::task::spawn_blocking(move || load_readonly_config_at(&paths).map(project))
        .await
        .map_err(|_| WorkBuddyError::new(WorkBuddyErrorCode::InternalError))?
}

fn project(loaded: LoadedConfig) -> WorkBuddyHealthMetadata {
    let mut observation = WorkBuddyHealthMetadata {
        exists: loaded.exists,
        model_count: loaded.document.unique_model_ids().len(),
        entry_count: loaded.document.models().len(),
        credentials_present: 0,
        endpoints_present: 0,
        endpoints_valid: 0,
    };
    for model in loaded.document.models() {
        let key = model
            .get("apiKey")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        if !key.is_empty()
            && !key.starts_with("{env:")
            && !key.starts_with("{file:")
            && key != "PROXY_MANAGED"
        {
            observation.credentials_present += 1;
        }
        if let Some(endpoint) = model
            .get("url")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            observation.endpoints_present += 1;
            if normalize_workbuddy_base_url(endpoint)
                .and_then(|url| reject_url_credential_collision(&url, key))
                .is_ok()
            {
                observation.endpoints_valid += 1;
            }
        }
    }
    observation
}

#[cfg(test)]
mod tests {
    use super::super::config::WorkBuddyPaths;
    use super::*;

    #[test]
    fn health_metadata_reads_existing_fields_without_creating_or_changing_files() {
        let temp = tempfile::tempdir().unwrap();
        let paths = WorkBuddyPaths::from_home(temp.path());
        let missing = project(load_readonly_config_at(&paths).unwrap());
        assert!(!missing.exists);
        assert!(!paths.directory.exists());
        std::fs::create_dir_all(&paths.directory).unwrap();
        let bytes = br#"{"models":[{"id":"first","url":"https://gateway.example.test/v1","apiKey":"fixture-private-key"},{"id":"second","url":"file:///private/fixture","apiKey":""},{"id":"third"}]}"#;
        std::fs::write(&paths.models, bytes).unwrap();
        let facts = project(load_readonly_config_at(&paths).unwrap());
        assert_eq!(
            facts,
            WorkBuddyHealthMetadata {
                exists: true,
                model_count: 3,
                entry_count: 3,
                credentials_present: 1,
                endpoints_present: 2,
                endpoints_valid: 1
            }
        );
        assert_eq!(std::fs::read(&paths.models).unwrap(), bytes);
        assert!(!paths.backup.exists());
        let debug = format!("{facts:?}");
        for sensitive in [
            "fixture-private-key",
            "gateway.example",
            "/private/fixture",
            "first",
            "second",
        ] {
            assert!(!debug.contains(sensitive));
        }
    }

    #[test]
    fn health_metadata_rejects_bad_shapes_oversize_and_credential_collisions() {
        let temp = tempfile::tempdir().unwrap();
        let paths = WorkBuddyPaths::from_home(temp.path());
        std::fs::create_dir_all(&paths.directory).unwrap();
        for contents in [
            r#"{"models":{}}"#,
            r#"[{"id":"fixture-private-key","apiKey":"fixture-private-key"}]"#,
            "not json",
        ] {
            std::fs::write(&paths.models, contents).unwrap();
            assert!(load_readonly_config_at(&paths).is_err());
            assert_eq!(std::fs::read_to_string(&paths.models).unwrap(), contents);
        }
        std::fs::write(
            &paths.models,
            vec![b' '; super::super::document::MAX_CONFIG_BYTES as usize + 1],
        )
        .unwrap();
        assert!(load_readonly_config_at(&paths).is_err());
        assert!(!paths.backup.exists());
    }
}
