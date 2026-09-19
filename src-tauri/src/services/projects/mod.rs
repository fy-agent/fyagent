pub(crate) mod domain;
mod files;
mod resources;
#[cfg(test)]
mod tests;

use crate::{database::Database, error::AppError};
use domain::*;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc};

pub(crate) fn project_error(code: &str) -> AppError {
    AppError::InvalidInput(format!("projects_{code}"))
}
pub(crate) fn validate_id(id: &str) -> Result<(), AppError> {
    let v = uuid::Uuid::parse_str(id).map_err(|_| project_error("invalid_request"))?;
    if v.get_version_num() != 4 || v.to_string() != id {
        return Err(project_error("invalid_request"));
    }
    Ok(())
}
fn name(value: &str) -> Result<String, AppError> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > 160 || v.chars().any(char::is_control) {
        return Err(project_error("invalid_request"));
    }
    Ok(v.to_string())
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Kit authority checks the immutable native library, never a renderer assertion.
pub(crate) trait ProjectKitReader: Send + Sync {
    fn confirm(&self, kit: &KitBinding) -> Result<(), AppError>;
}
pub(crate) struct UnavailableKitReader;
impl ProjectKitReader for UnavailableKitReader {
    fn confirm(&self, _: &KitBinding) -> Result<(), AppError> {
        Err(project_error("kit_owner_unavailable"))
    }
}

pub(crate) struct ProjectsService {
    db: Arc<Database>,
    root: PathBuf,
    kits: Arc<dyn ProjectKitReader>,
}
impl ProjectsService {
    pub(crate) fn new(db: Arc<Database>, root: PathBuf, kits: Arc<dyn ProjectKitReader>) -> Self {
        Self { db, root, kits }
    }
    pub(crate) fn customers(&self) -> Result<Vec<Customer>, AppError> {
        self.db.projects_list_customers()
    }
    pub(crate) fn create_customer(&self, label: &str) -> Result<Customer, AppError> {
        let c = Customer {
            customer_id: uuid::Uuid::new_v4().to_string(),
            name: name(label)?,
            revision: 0,
            archived: false,
        };
        self.db.projects_save_customer(&c, None)?;
        Ok(c)
    }
    pub(crate) fn update_customer(
        &self,
        id: &str,
        expected: i64,
        label: &str,
        archived: bool,
    ) -> Result<Customer, AppError> {
        validate_id(id)?;
        if !(0..MAX_REVISION).contains(&expected) {
            return Err(project_error("revision_conflict"));
        }
        let c = Customer {
            customer_id: id.into(),
            name: name(label)?,
            revision: expected + 1,
            archived,
        };
        self.db.projects_save_customer(&c, Some(expected))?;
        Ok(c)
    }
    pub(crate) fn list(&self) -> Result<Vec<Project>, AppError> {
        self.db.projects_list()
    }
    pub(crate) fn get(&self, id: &str) -> Result<Project, AppError> {
        validate_id(id)?;
        self.db.projects_get(id)
    }
    pub(crate) fn create(&self, customer_id: &str, label: &str) -> Result<Project, AppError> {
        validate_id(customer_id)?;
        let p = Project {
            project_id: uuid::Uuid::new_v4().to_string(),
            customer_id: customer_id.into(),
            name: name(label)?,
            project_revision: 0,
            archived: false,
            resources: vec![],
            credentials: vec![],
            kit: None,
            context_generation: None,
            codex_prepared: false,
            created_at: now(),
            updated_at: now(),
        };
        self.db.projects_insert(&p)?;
        Ok(p)
    }
    fn mutation(&self, r: &ProjectMutation) -> Result<Project, AppError> {
        let p = self.get(&r.project_id)?;
        if p.archived {
            return Err(project_error("archived"));
        }
        if p.project_revision != r.expected_revision
            || !(0..MAX_REVISION).contains(&r.expected_revision)
        {
            return Err(project_error("revision_conflict"));
        }
        Ok(p)
    }
    fn save(&self, mut p: Project, expected: i64) -> Result<Project, AppError> {
        p.project_revision = expected + 1;
        p.updated_at = now();
        self.db.projects_cas(&p, expected)?;
        Ok(p)
    }
    pub(crate) fn update(
        &self,
        r: &ProjectMutation,
        label: &str,
        archived: bool,
    ) -> Result<Project, AppError> {
        let mut p = self.mutation(r)?;
        p.name = name(label)?;
        p.archived = archived;
        self.save(p, r.expected_revision)
    }
    pub(crate) fn bind_resource(
        &self,
        r: &ProjectMutation,
        kind: ResourceKind,
        agent_id: &str,
        raw_id: &str,
        model: Option<String>,
    ) -> Result<Project, AppError> {
        let mut p = self.mutation(r)?;
        if model
            .as_ref()
            .is_some_and(|m| m.is_empty() || m.len() > 256 || m.chars().any(char::is_control))
        {
            return Err(project_error("invalid_request"));
        }
        let source = self
            .resource_options()?
            .into_iter()
            .find(|x| x.kind == kind && x.agent_id == agent_id && x.raw_id == raw_id)
            .ok_or_else(|| project_error("resource_missing"))?;
        p.resources
            .retain(|x| !(x.kind == kind && x.agent_id == agent_id && x.raw_id == raw_id));
        if p.resources.len() >= 128 {
            return Err(project_error("invalid_request"));
        }
        p.resources.push(ResourceBinding {
            kind,
            agent_id: agent_id.into(),
            raw_id: raw_id.into(),
            model,
            pinned_version: source.version,
        });
        self.save(p, r.expected_revision)
    }
    pub(crate) fn remove_resource(
        &self,
        r: &ProjectMutation,
        kind: ResourceKind,
        agent_id: &str,
        raw_id: &str,
    ) -> Result<Project, AppError> {
        let mut p = self.mutation(r)?;
        p.resources
            .retain(|x| !(x.kind == kind && x.agent_id == agent_id && x.raw_id == raw_id));
        self.save(p, r.expected_revision)
    }
    pub(crate) fn bind_credential(
        &self,
        r: &ProjectMutation,
        id: &str,
        purpose: &str,
        consumer: &str,
    ) -> Result<Project, AppError> {
        let mut p = self.mutation(r)?;
        let source = self
            .credential_options()?
            .into_iter()
            .find(|x| x.credential_id == id && x.purpose == purpose && x.consumer == consumer)
            .ok_or_else(|| project_error("credential_unavailable"))?;
        if source.state != ObservationState::Unverifiable {
            return Err(project_error("credential_unavailable"));
        }
        p.credentials.retain(|x| x.credential_id != id);
        if p.credentials.len() >= 32 {
            return Err(project_error("invalid_request"));
        }
        p.credentials.push(CredentialBinding {
            credential_id: id.into(),
            purpose: purpose.into(),
            consumer: consumer.into(),
            pinned_generation: source.generation,
        });
        self.save(p, r.expected_revision)
    }
    pub(crate) fn remove_credential(
        &self,
        r: &ProjectMutation,
        id: &str,
    ) -> Result<Project, AppError> {
        let mut p = self.mutation(r)?;
        p.credentials.retain(|x| x.credential_id != id);
        self.save(p, r.expected_revision)
    }
    pub(crate) fn context(&self, id: &str) -> Result<ProjectContext, AppError> {
        let p = self.get(id)?;
        if let Some(g) = &p.context_generation {
            let content = files::read(&self.root, id, g)?;
            if format!("{:x}", Sha256::digest(content.as_bytes()))
                != self.db.projects_context_digest(id, g)?
            {
                return Err(project_error("context_changed"));
            }
            let codex_instructions = if p.codex_prepared {
                files::verify_codex(&self.root, id, g, &content)?;
                Some(files::codex_instructions(&files::directory(
                    &self.root, id, g,
                )?))
            } else {
                None
            };
            Ok(ProjectContext {
                project_id: id.into(),
                project_revision: p.project_revision,
                content,
                directory: Some(
                    files::directory(&self.root, id, g)?
                        .to_string_lossy()
                        .into_owned(),
                ),
                state: ContextState::Materialized,
                codex_instructions,
            })
        } else {
            Ok(ProjectContext {
                project_id: id.into(),
                project_revision: p.project_revision,
                content: String::new(),
                directory: None,
                state: ContextState::NotCreated,
                codex_instructions: None,
            })
        }
    }
    pub(crate) fn write_context(
        &self,
        r: &ProjectMutation,
        content: &str,
    ) -> Result<ProjectContext, AppError> {
        let mut p = self.mutation(r)?;
        if content.len() > MAX_CONTEXT_BYTES || content.contains('\0') {
            return Err(project_error("invalid_request"));
        }
        if p.context_generation.is_some() {
            self.context(&p.project_id)?;
        }
        let generation = uuid::Uuid::new_v4().to_string();
        files::write(&self.root, &p.project_id, &generation, content)?;
        p.context_generation = Some(generation);
        p.codex_prepared = false;
        p.project_revision = r.expected_revision + 1;
        p.updated_at = now();
        self.db.projects_publish_context(
            &p,
            r.expected_revision,
            &format!("{:x}", Sha256::digest(content.as_bytes())),
        )?;
        self.context(&r.project_id)
    }
    pub(crate) fn prepare_codex(&self, r: &ProjectMutation) -> Result<ProjectContext, AppError> {
        let mut p = self.mutation(r)?;
        let context = self.context(&p.project_id)?;
        if context.state != ContextState::Materialized {
            return Err(project_error("context_required"));
        }
        let generation = uuid::Uuid::new_v4().to_string();
        files::write(&self.root, &p.project_id, &generation, &context.content)?;
        files::prepare_codex(&self.root, &p.project_id, &generation, &context.content)?;
        p.context_generation = Some(generation);
        p.codex_prepared = true;
        p.project_revision = r.expected_revision + 1;
        p.updated_at = now();
        self.db.projects_publish_context(
            &p,
            r.expected_revision,
            &format!("{:x}", Sha256::digest(context.content.as_bytes())),
        )?;
        self.context(&p.project_id)
    }
    pub(crate) fn bind_delivery_kit(&self, r: &BindKitRequest) -> Result<Project, AppError> {
        validate_id(&r.project_id)?;
        validate_id(&r.binding_intent_id)?;
        if r.kit_id.is_empty()
            || r.kit_id.len() > 160
            || r.kit_version.is_empty()
            || r.kit_version.len() > 80
            || r.manifest_digest.len() != 64
            || !r
                .manifest_digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(project_error("invalid_request"));
        }
        self.kits.confirm(&KitBinding {
            kit_id: r.kit_id.clone(),
            kit_version: r.kit_version.clone(),
            manifest_digest: r.manifest_digest.clone(),
        })?;
        self.db.projects_bind_kit(r)
    }
    /// Read only. Never calls reconcile/overview, probes remote services or reads vault material.
    pub(crate) fn dependency_snapshot(
        &self,
        id: &str,
    ) -> Result<ProjectDependencySnapshot, AppError> {
        let p = self.get(id)?;
        let options = self.resource_options()?;
        let credentials = self.credential_options()?;
        let resources = p
            .resources
            .iter()
            .map(|b| {
                let source = options
                    .iter()
                    .find(|x| x.kind == b.kind && x.agent_id == b.agent_id && x.raw_id == b.raw_id);
                let observed = source.and_then(|s| s.version.clone());
                let state = match source {
                    None => ObservationState::Missing,
                    Some(_) => match (&b.pinned_version, &observed) {
                        (Some(a), Some(b)) if a == b => ObservationState::Matched,
                        (Some(_), Some(_)) => ObservationState::Drifted,
                        _ => ObservationState::Unverifiable,
                    },
                };
                ResourceDependency {
                    resource: b.clone(),
                    observed_version: observed,
                    state,
                }
            })
            .collect();
        let credentials = p
            .credentials
            .iter()
            .map(|b| {
                let c = credentials
                    .iter()
                    .find(|c| c.credential_id == b.credential_id);
                let state = match c {
                    None => ObservationState::Missing,
                    Some(c) if c.purpose != b.purpose || c.consumer != b.consumer => {
                        ObservationState::PurposeMismatch
                    }
                    Some(c) if c.state != ObservationState::Unverifiable => c.state.clone(),
                    Some(c) if c.generation != b.pinned_generation => ObservationState::Drifted,
                    _ => ObservationState::Unverifiable,
                };
                CredentialDependency {
                    binding: b.clone(),
                    observed_generation: c.map(|c| c.generation),
                    state,
                }
            })
            .collect();
        let context_state = match self.context(id) {
            Ok(c) => c.state,
            Err(_) => ContextState::Unavailable,
        };
        // A concurrent change cannot produce a snapshot falsely tied to the previous revision.
        if self.get(id)?.project_revision != p.project_revision {
            return Err(project_error("revision_conflict"));
        }
        let kit_state = match &p.kit {
            None => ObservationState::Missing,
            Some(kit) => {
                if self.kits.confirm(kit).is_ok() {
                    ObservationState::Matched
                } else {
                    ObservationState::Unavailable
                }
            }
        };
        Ok(ProjectDependencySnapshot {
            project_id: p.project_id,
            project_revision: p.project_revision,
            archived: p.archived,
            kit: p.kit,
            kit_state,
            resources,
            credentials,
            context_generation: p.context_generation,
            context_state,
            observed_at: now(),
            runtime_available: false,
        })
    }
}
