mod files;
mod schema;
#[cfg(test)]
mod tests;

use crate::database::Database;
pub(crate) use files::{export_file, read_file};
pub(crate) use schema::{
    digest, project, ConfigPack, PackApp, PackDraftWrite, PackError, PackInventory,
    PackStoredProvider, PortableProvider, Result, DRAFT_CATEGORY, FORMAT, MAX_BYTES, MAX_ENTRIES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use uuid::Uuid;

const TTL: Duration = Duration::from_secs(600);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Candidate {
    pub selection_id: String,
    pub provider: PortableProvider,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Candidates {
    pub entries: Vec<Candidate>,
    pub excluded: usize,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportAction {
    Add,
    Skip,
    Rename,
    Overwrite,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportChoice {
    pub action: ImportAction,
    pub name: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewEntry {
    pub provider: PortableProvider,
    pub action: ImportAction,
    pub conflict: bool,
    pub can_overwrite: bool,
    pub existing: Option<PortableProvider>,
    pub credentials_required: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportPreview {
    pub preview_id: String,
    pub digest: String,
    pub entries: Vec<PreviewEntry>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportPreview {
    pub export_id: String,
    pub digest: String,
    pub text: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportResult {
    pub providers: Vec<PortableProvider>,
    pub skipped: usize,
}
struct PendingImport {
    preview: ImportPreview,
    revision: String,
    local_current: Vec<String>,
    writes: Vec<PackDraftWrite>,
    created: Instant,
}
struct PendingExport {
    preview: ExportPreview,
    created: Instant,
}
#[derive(Default)]
pub(crate) struct ConfigPackService {
    imports: HashMap<String, PendingImport>,
    exports: HashMap<String, PendingExport>,
}
impl ConfigPackService {
    fn current_selections() -> Vec<String> {
        [
            crate::app_config::AppType::Claude,
            crate::app_config::AppType::Codex,
        ]
        .iter()
        .filter_map(|app| {
            crate::settings::get_current_provider(app).map(|id| format!("{}:{id}", app.as_str()))
        })
        .collect()
    }
    pub(crate) fn preview_for_state(
        &mut self,
        state: &crate::store::AppState,
        text: &str,
        choices: Vec<ImportChoice>,
    ) -> Result<ImportPreview> {
        self.preview_import(&state.db, text, choices, Self::current_selections())
    }
    pub(crate) fn apply_for_state(
        &mut self,
        state: &crate::store::AppState,
        id: &str,
        expected_digest: &str,
    ) -> Result<ImportResult> {
        let _claude =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app("claude"));
        let _codex = futures::executor::block_on(state.proxy_service.lock_switch_for_app("codex"));
        self.apply(&state.db, id, expected_digest, &Self::current_selections())
    }
    fn prune(&mut self) {
        self.imports.retain(|_, p| p.created.elapsed() < TTL);
        self.exports.retain(|_, p| p.created.elapsed() < TTL);
    }
    fn reserve(&mut self) -> Result<()> {
        self.prune();
        if self.imports.len() + self.exports.len() >= 16 {
            return Err(PackError::Busy);
        }
        Ok(())
    }
    pub(crate) fn candidates(db: &Database) -> Result<Candidates> {
        let inventory = db.config_pack_inventory(&[])?;
        let mut excluded = 0;
        let entries = inventory
            .providers
            .into_iter()
            .filter_map(|p| {
                if let Some(provider) = p.portable {
                    Some(Candidate {
                        selection_id: p.selection_id,
                        provider,
                    })
                } else {
                    excluded += 1;
                    None
                }
            })
            .collect();
        Ok(Candidates { entries, excluded })
    }
    pub(crate) fn preview_export(
        &mut self,
        db: &Database,
        selection: Vec<String>,
    ) -> Result<ExportPreview> {
        self.reserve()?;
        if selection.is_empty()
            || selection.len() > MAX_ENTRIES
            || selection.iter().collect::<HashSet<_>>().len() != selection.len()
        {
            return Err(PackError::InvalidPack);
        }
        let candidates = Self::candidates(db)?;
        let providers = selection
            .iter()
            .map(|id| {
                candidates
                    .entries
                    .iter()
                    .find(|c| &c.selection_id == id)
                    .map(|c| c.provider.clone())
                    .ok_or(PackError::Conflict)
            })
            .collect::<Result<Vec<_>>>()?;
        let text = ConfigPack {
            format: FORMAT.into(),
            providers,
        }
        .text()?;
        let preview = ExportPreview {
            export_id: Uuid::new_v4().to_string(),
            digest: digest(text.as_bytes()),
            text,
        };
        self.exports.insert(
            preview.export_id.clone(),
            PendingExport {
                preview: preview.clone(),
                created: Instant::now(),
            },
        );
        Ok(preview)
    }
    pub(crate) fn export_bytes(&mut self, id: &str, expected_digest: &str) -> Result<Vec<u8>> {
        self.prune();
        let p = self.exports.get(id).ok_or(PackError::StalePreview)?;
        if p.preview.digest != expected_digest {
            return Err(PackError::InvalidPreview);
        }
        Ok(p.preview.text.as_bytes().to_vec())
    }
    pub(crate) fn preview_import(
        &mut self,
        db: &Database,
        text: &str,
        choices: Vec<ImportChoice>,
        local_current: Vec<String>,
    ) -> Result<ImportPreview> {
        self.reserve()?;
        let pack = ConfigPack::parse(text.as_bytes())?;
        if !choices.is_empty() && choices.len() != pack.providers.len() {
            return Err(PackError::InvalidPack);
        }
        let inventory = db.config_pack_inventory(&local_current)?;
        let mut entries = Vec::new();
        let mut writes = Vec::new();
        let mut selected_names = HashSet::new();
        for (index, mut provider) in pack.providers.into_iter().enumerate() {
            let matching = inventory
                .providers
                .iter()
                .filter(|p| p.app == provider.app && p.name == provider.name)
                .collect::<Vec<_>>();
            let conflict = !matching.is_empty();
            let can_overwrite = matching.len() == 1 && matching[0].overwrite_allowed;
            let default = ImportChoice {
                action: if conflict {
                    ImportAction::Skip
                } else {
                    ImportAction::Add
                },
                name: None,
            };
            let choice = choices.get(index).unwrap_or(&default);
            let existing = if can_overwrite {
                matching[0].portable.clone()
            } else {
                None
            };
            let id = match choice.action {
                ImportAction::Skip => None,
                ImportAction::Add if !conflict && choice.name.is_none() => {
                    Some(Uuid::new_v4().to_string())
                }
                ImportAction::Rename => {
                    provider.name = choice.name.clone().ok_or(PackError::InvalidPack)?;
                    provider.validate()?;
                    if inventory
                        .providers
                        .iter()
                        .any(|p| p.app == provider.app && p.name == provider.name)
                    {
                        return Err(PackError::Conflict);
                    }
                    Some(Uuid::new_v4().to_string())
                }
                ImportAction::Overwrite if can_overwrite && choice.name.is_none() => {
                    Some(matching[0].id.clone())
                }
                _ => return Err(PackError::Conflict),
            };
            if choice.action != ImportAction::Rename && choice.name.is_some() {
                return Err(PackError::InvalidPack);
            }
            if let Some(id) = id {
                if !selected_names.insert((provider.app, provider.name.clone())) {
                    return Err(PackError::Conflict);
                }
                writes.push(PackDraftWrite {
                    id,
                    provider: provider.clone(),
                    overwrite: choice.action == ImportAction::Overwrite,
                });
            }
            entries.push(PreviewEntry {
                provider,
                action: choice.action,
                conflict,
                can_overwrite,
                existing,
                credentials_required: true,
            });
        }
        let preview_id = Uuid::new_v4().to_string();
        let encoded = serde_json::to_string(&entries).map_err(|_| PackError::InvalidPack)?;
        let preview = ImportPreview {
            digest: digest(format!("{preview_id}:{}:{encoded}", inventory.revision).as_bytes()),
            preview_id,
            entries,
        };
        self.imports.insert(
            preview.preview_id.clone(),
            PendingImport {
                preview: preview.clone(),
                revision: inventory.revision,
                local_current,
                writes,
                created: Instant::now(),
            },
        );
        Ok(preview)
    }
    pub(crate) fn apply(
        &mut self,
        db: &Database,
        id: &str,
        expected_digest: &str,
        local_current: &[String],
    ) -> Result<ImportResult> {
        self.prune();
        let pending = self.imports.get(id).ok_or(PackError::StalePreview)?;
        if pending.preview.digest != expected_digest {
            return Err(PackError::InvalidPreview);
        }
        if pending.local_current != local_current {
            return Err(PackError::StalePreview);
        }
        // Consume before mutation. Lost responses cannot replay an import. The
        // ordinary candidate list is the recovery/readback entry after reopening.
        let pending = self.imports.remove(id).ok_or(PackError::StalePreview)?;
        let providers =
            db.insert_portable_provider_drafts(&pending.revision, local_current, &pending.writes)?;
        Ok(ImportResult {
            skipped: pending.preview.entries.len() - providers.len(),
            providers,
        })
    }
    pub(crate) fn cancel(&mut self, id: &str) {
        self.imports.remove(id);
        self.exports.remove(id);
    }
}
