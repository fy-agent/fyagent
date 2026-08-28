use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::input::{Chord, InputId};
use super::target::Target;

pub const PROFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDraft {
    pub version: u32,
    pub revision: Option<String>,
    pub serial: ProfileSerial,
    pub target: Option<ProfileTarget>,
    pub mappings: Vec<MappingDraft>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSerial {
    pub port: String,
    pub baud: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileTarget {
    pub process_name: String,
    pub process_path: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MappingDraft {
    pub input: InputId,
    pub display_name: String,
    pub keys: Vec<String>,
}
impl ProfileDraft {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.version != PROFILE_VERSION
            || self.serial.port.trim().is_empty()
            || self.serial.baud == 0
            || self.target.is_none()
            || self.mappings.len() != InputId::ALL.len()
        {
            return Err(ProfileError::Invalid);
        }
        Target::new(
            self.target.as_ref().unwrap().process_name.clone(),
            self.target.as_ref().unwrap().process_path.clone(),
        )
        .map_err(|_| ProfileError::Invalid)?;
        let mut inputs = HashSet::new();
        let mut chords = HashSet::new();
        for mapping in &self.mappings {
            if !inputs.insert(mapping.input) || name_invalid(&mapping.display_name) {
                return Err(ProfileError::Invalid);
            }
            let chord = Chord::parse(&mapping.keys).map_err(|_| ProfileError::Invalid)?;
            if !chords.insert(chord.canonical()) {
                return Err(ProfileError::DuplicateChord);
            }
        }
        if inputs.len() != InputId::ALL.len() {
            return Err(ProfileError::Invalid);
        }
        Ok(())
    }
    pub fn with_computed_revision(mut self) -> Self {
        self.revision = Some(format!(
            "sha256:{:x}",
            Sha256::digest(
                serde_json::to_vec(&self.without_revision()).expect("serializable profile")
            )
        ));
        self
    }
    fn without_revision(&self) -> Self {
        let mut copy = self.clone();
        copy.revision = None;
        copy
    }
    pub fn mapping_for(&self, input: InputId) -> Option<&MappingDraft> {
        self.mappings.iter().find(|mapping| mapping.input == input)
    }
}
fn name_invalid(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.is_empty() || trimmed.chars().count() > 40 || trimmed.chars().any(char::is_control)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    Invalid,
    DuplicateChord,
    Stale,
}
impl Display for ProfileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "profile is invalid",
            Self::DuplicateChord => "shortcut chord is duplicated",
            Self::Stale => "profile revision is stale",
        })
    }
}
impl std::error::Error for ProfileError {}

pub struct ProfileStore {
    path: PathBuf,
}
impl ProfileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn load(&self) -> Result<Option<ProfileDraft>, ProfileError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let profile = serde_json::from_slice::<ProfileDraft>(
            &fs::read(&self.path).map_err(|_| ProfileError::Invalid)?,
        )
        .map_err(|_| ProfileError::Invalid)?;
        profile.validate()?;
        if profile.revision.is_none()
            || profile.clone().with_computed_revision().revision != profile.revision
        {
            return Err(ProfileError::Invalid);
        }
        Ok(Some(profile))
    }
    pub fn save(
        &self,
        draft: ProfileDraft,
        expected_revision: Option<&str>,
    ) -> Result<ProfileDraft, ProfileError> {
        draft.validate()?;
        let previous = self.load()?;
        if previous
            .as_ref()
            .and_then(|profile| profile.revision.as_deref())
            != expected_revision
        {
            return Err(ProfileError::Stale);
        }
        let saved = draft.with_computed_revision();
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|_| ProfileError::Invalid)?;
        if previous.as_ref() == Some(&saved) {
            return Ok(saved);
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| ProfileError::Invalid)?;
        }
        if let Some(existing) = self.load()? {
            fs::write(
                self.path.with_extension("json.bak"),
                serde_json::to_vec_pretty(&existing).map_err(|_| ProfileError::Invalid)?,
            )
            .map_err(|_| ProfileError::Invalid)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|_| ProfileError::Invalid)?;
        fs::rename(temporary, &self.path).map_err(|_| ProfileError::Invalid)?;
        Ok(saved)
    }
    #[cfg(test)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn draft() -> ProfileDraft {
        ProfileDraft {
            version: PROFILE_VERSION,
            revision: None,
            serial: ProfileSerial {
                port: "fixture".into(),
                baud: 115200,
            },
            target: Some(ProfileTarget {
                process_name: "Fixture.exe".into(),
                process_path: r"C:\Fixture.exe".into(),
            }),
            mappings: vec![
                MappingDraft {
                    input: InputId::EncoderCw,
                    display_name: "Previous".into(),
                    keys: vec!["CTRL".into(), "TAB".into()],
                },
                MappingDraft {
                    input: InputId::EncoderCcw,
                    display_name: "Next".into(),
                    keys: vec!["CTRL".into(), "SHIFT".into(), "TAB".into()],
                },
                MappingDraft {
                    input: InputId::EncoderPress,
                    display_name: "Confirm".into(),
                    keys: vec!["ENTER".into()],
                },
            ],
        }
    }
    #[test]
    fn save_is_revision_aware_and_keeps_backup_on_change() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let first = store.save(draft(), None).unwrap();
        assert!(store.path().exists());
        let mut changed = first.clone();
        changed.mappings[0].display_name = "Earlier".into();
        let second = store.save(changed, first.revision.as_deref()).unwrap();
        assert_ne!(first.revision, second.revision);
        assert!(directory.path().join("profile.json.bak").exists());
        assert_eq!(
            store.save(second.clone(), Some("wrong")),
            Err(ProfileError::Stale)
        );
    }
    #[test]
    fn duplicate_chord_is_rejected() {
        let mut profile = draft();
        profile.mappings[1].keys = vec!["TAB".into(), "CTRL".into()];
        assert_eq!(profile.validate(), Err(ProfileError::DuplicateChord));
    }
    #[test]
    fn load_rejects_unknown_fields_and_wrong_contract_version() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let mut flattened = serde_json::to_value(draft()).unwrap();
        let serial = flattened.as_object_mut().unwrap().remove("serial").unwrap();
        flattened
            .as_object_mut()
            .unwrap()
            .insert("serialPort".into(), serial["port"].clone());
        flattened
            .as_object_mut()
            .unwrap()
            .insert("baud".into(), serial["baud"].clone());
        fs::write(store.path(), serde_json::to_vec(&flattened).unwrap()).unwrap();
        assert_eq!(store.load(), Err(ProfileError::Invalid));
        let mut wrong = draft();
        wrong.version = 2;
        fs::write(store.path(), serde_json::to_vec(&wrong).unwrap()).unwrap();
        assert_eq!(store.load(), Err(ProfileError::Invalid));
    }
    #[test]
    fn saved_json_uses_only_the_nested_serial_contract() {
        let serialized = serde_json::to_value(draft()).unwrap();
        assert_eq!(
            serialized["serial"],
            serde_json::json!({"port":"fixture","baud":115200})
        );
        assert!(serialized.get("serialPort").is_none());
        assert!(serialized.get("baud").is_none());
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let mut unknown_serial = serde_json::to_value(draft()).unwrap();
        unknown_serial["serial"]["extra"] = serde_json::json!(true);
        fs::write(store.path(), serde_json::to_vec(&unknown_serial).unwrap()).unwrap();
        assert_eq!(store.load(), Err(ProfileError::Invalid));
    }

    #[test]
    fn load_rejects_content_that_does_not_match_its_revision() {
        let directory = tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("profile.json"));
        let mut saved = store.save(draft(), None).unwrap();
        saved.mappings[0].display_name = "Tampered".into();
        fs::write(store.path(), serde_json::to_vec(&saved).unwrap()).unwrap();
        assert_eq!(store.load(), Err(ProfileError::Invalid));
    }
}
