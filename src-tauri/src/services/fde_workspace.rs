//! Composition of the project, package and evidence owners.
use super::{delivery_kits as kits, projects, verification as evidence};
use crate::{app_config::AppType, database::Database, error::AppError};
use projects::domain::{ContextState, KitBinding, ObservationState, ResourceKind};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) struct FdeServices {
    pub projects: Arc<projects::ProjectsService>,
    pub kits: Arc<Mutex<kits::KitLibrary>>,
    pub verification: Arc<evidence::VerificationService>,
}

pub(crate) fn compose(db: Arc<Database>, root: PathBuf) -> FdeServices {
    let kits = Arc::new(Mutex::new(kits::KitLibrary::new(
        root.join("delivery-kits"),
    )));
    let projects = Arc::new(projects::ProjectsService::new(
        db.clone(),
        root.join("projects"),
        Arc::new(KitReader(kits.clone())),
    ));
    let verification = Arc::new(evidence::VerificationService::new(
        db.clone(),
        Arc::new(DependencyReader {
            db,
            projects: projects.clone(),
            kits: kits.clone(),
        }),
    ));
    FdeServices {
        projects,
        kits,
        verification,
    }
}

struct KitReader(Arc<Mutex<kits::KitLibrary>>);
fn kit_identity(kit: &KitBinding) -> kits::KitIdentity {
    kits::KitIdentity {
        kit_id: kit.kit_id.clone(),
        kit_version: kit.kit_version.clone(),
        manifest_digest: kit.manifest_digest.clone(),
    }
}
impl projects::ProjectKitReader for KitReader {
    fn confirm(&self, kit: &KitBinding) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| projects::project_error("kit_owner_unavailable"))?
            .confirm_identity(&kit_identity(kit))
            .map_err(|_| projects::project_error("kit_owner_unavailable"))
    }
}

struct DependencyReader {
    db: Arc<Database>,
    projects: Arc<projects::ProjectsService>,
    kits: Arc<Mutex<kits::KitLibrary>>,
}
impl evidence::ProjectDependencyReader for DependencyReader {
    fn read(&self, id: &str) -> evidence::VerificationResult<evidence::ProjectDependencySnapshot> {
        let snapshot = self
            .projects
            .dependency_snapshot(id)
            .map_err(|_| "project_reader_unavailable")?;
        // Only owner-provided identities and generations enter this digest. No
        // content, credentials or volatile observation timestamps are hashed.
        let resources: Vec<_> = snapshot
            .resources
            .iter()
            .map(|r| {
                (
                    &r.resource.kind,
                    &r.resource.agent_id,
                    &r.resource.raw_id,
                    &r.resource.model,
                    &r.resource.pinned_version,
                    &r.observed_version,
                    &r.state,
                )
            })
            .collect();
        let dependency_bytes = serde_json::to_vec(&(
            resources,
            &snapshot.credentials,
            &snapshot.context_generation,
            &snapshot.context_state,
            &snapshot.kit_state,
        ))
        .map_err(|_| "project_reader_unavailable")?;
        let providers: Vec<_> = snapshot
            .resources
            .iter()
            .filter(|r| r.resource.kind == ResourceKind::Provider)
            .collect();
        let credential_generation =
            if providers.len() == 1 && providers[0].state == ObservationState::Matched {
                providers[0].observed_version.clone()
            } else {
                None
            };
        Ok(evidence::ProjectDependencySnapshot {
            project_id: snapshot.project_id,
            project_revision: u64::try_from(snapshot.project_revision)
                .map_err(|_| "project_reader_unavailable")?,
            active: !snapshot.archived,
            kit: snapshot.kit.map(|k| evidence::KitIdentity {
                kit_id: k.kit_id,
                kit_version: k.kit_version,
                manifest_digest: k.manifest_digest,
            }),
            resource_revision: format!("{:x}", Sha256::digest(&dependency_bytes)),
            credential_generation,
            configuration_saved: snapshot.context_state != ContextState::Unavailable
                && snapshot
                    .resources
                    .iter()
                    .all(|r| r.state == ObservationState::Matched)
                && snapshot
                    .credentials
                    .iter()
                    .all(|r| r.state == ObservationState::Matched),
            projection: match snapshot.context_state {
                ContextState::Materialized => evidence::ProjectionState::Confirmed,
                ContextState::Unavailable => evidence::ProjectionState::Different,
                ContextState::NotCreated => evidence::ProjectionState::Unavailable,
            },
        })
    }

    fn saved_model(&self, id: &str) -> evidence::VerificationResult<evidence::SavedModel> {
        let snapshot = self
            .projects
            .dependency_snapshot(id)
            .map_err(|_| "saved_model_unavailable")?;
        let bindings: Vec<_> = snapshot
            .resources
            .iter()
            .filter(|r| r.resource.kind == ResourceKind::Provider)
            .collect();
        if snapshot.archived
            || bindings.len() != 1
            || bindings[0].state != ObservationState::Matched
        {
            return Err("saved_model_unavailable");
        }
        let binding = &bindings[0].resource;
        let model_id = binding
            .model
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .ok_or("saved_model_unavailable")?
            .clone();
        let (app, probe) = match binding.agent_id.as_str() {
            "claude" => (AppType::Claude, super::model_probe::ModelProbeApp::Claude),
            "codex" => (AppType::Codex, super::model_probe::ModelProbeApp::Codex),
            "grokbuild" => (
                AppType::GrokBuild,
                super::model_probe::ModelProbeApp::GrokBuild,
            ),
            "opencode" => (
                AppType::OpenCode,
                super::model_probe::ModelProbeApp::OpenCode,
            ),
            _ => return Err("saved_model_unavailable"),
        };
        let provider = self
            .db
            .get_provider_by_id(&binding.raw_id, &binding.agent_id)
            .map_err(|_| "saved_model_unavailable")?
            .ok_or("saved_model_unavailable")?;
        // This checker owns a saved API-key request. Subscription credentials
        // belong to the local proxy, and its marker is never upstream auth.
        // An unsupported source must not become a vendor/network failure.
        if provider.uses_subscription_proxy() {
            return Err("saved_model_unavailable");
        }
        let (api_key, base_url) =
            super::provider::ProviderService::extract_credentials(&provider, &app)
                .map_err(|_| "saved_model_unavailable")?;
        if api_key.is_empty() || base_url.is_empty() {
            return Err("saved_model_unavailable");
        }
        Ok(evidence::SavedModel {
            app: probe,
            base_url,
            api_key,
            model_id,
        })
    }

    fn validate_kit(
        &self,
        id: &str,
        fixture: evidence::KitFixture,
    ) -> evidence::VerificationResult<evidence::KitValidationReceipt> {
        let project = self
            .projects
            .get(id)
            .map_err(|_| "kit_validator_unavailable")?;
        let kit = project.kit.as_ref().ok_or("kit_validator_unavailable")?;
        let result = self
            .kits
            .lock()
            .map_err(|_| "kit_validator_unavailable")?
            .run(&kit_identity(kit))
            .map_err(|_| "kit_validator_unavailable")?;
        let selected = match fixture {
            evidence::KitFixture::Baseline => "baseline",
            evidence::KitFixture::MissingField => "missing-field",
        };
        let case = result
            .cases
            .iter()
            .find(|case| case.fixture_id == selected)
            .ok_or("kit_validator_unavailable")?;
        let code: evidence::SampleCode = serde_json::from_value(
            serde_json::to_value(&case.code).map_err(|_| "kit_validator_unavailable")?,
        )
        .map_err(|_| "kit_validator_unavailable")?;
        Ok(evidence::KitValidationReceipt {
            kit: evidence::KitIdentity {
                kit_id: result.kit_id,
                kit_version: result.kit_version,
                manifest_digest: result.manifest_digest,
            },
            passed: code == evidence::SampleCode::Ok && case.matches_expectation,
            validator_version: 1,
            fixture,
            sample: evidence::SampleReceipt {
                input_digest: case.input_digest.clone(),
                code,
                matches_expectation: case.matches_expectation,
                metrics: case.metrics.as_ref().map(|m| evidence::SampleMetrics {
                    current_minor: m.current_minor,
                    previous_minor: m.previous_minor,
                    growth_bps: m.growth_bps,
                    target_bps: m.target_bps,
                }),
                source_row_ids: case.source_row_ids.clone(),
                validator: result.validator_version.into(),
            },
        })
    }
}

#[cfg(test)]
mod tests;
