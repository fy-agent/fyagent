//! Durable project verification. Cross-domain authority enters only through
//! native readers, never renderer-supplied fingerprints or machine outcomes.
mod domain;
mod export;
mod types;
use crate::database::Database;
use chrono::{Duration, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
pub(crate) use types::*;

/// The project owner resolves a saved binding, not an arbitrary draft URL.
/// Secrets are process-local and deliberately neither Debug nor Serialize.
pub(crate) struct SavedModel {
    pub app: super::model_probe::ModelProbeApp,
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
}

impl Drop for SavedModel {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.api_key.zeroize();
    }
}

pub(crate) struct KitValidationReceipt {
    pub kit: KitIdentity,
    pub passed: bool,
    pub validator_version: u32,
    pub fixture: KitFixture,
    pub sample: SampleReceipt,
}

pub(crate) trait ProjectDependencyReader: Send + Sync {
    fn read(&self, project_id: &str) -> VerificationResult<ProjectDependencySnapshot>;
    fn saved_model(&self, project_id: &str) -> VerificationResult<SavedModel>;
    fn validate_kit(
        &self,
        project_id: &str,
        fixture: KitFixture,
    ) -> VerificationResult<KitValidationReceipt>;
}

/// Explicit integration boundary until composition injects the project/kit
/// owners. It cannot claim a successful read or execute a production fixture.
#[cfg(test)]
pub(crate) struct UnavailableProjectReader;
#[cfg(test)]
impl ProjectDependencyReader for UnavailableProjectReader {
    fn read(&self, _: &str) -> VerificationResult<ProjectDependencySnapshot> {
        Err("project_reader_unavailable")
    }
    fn saved_model(&self, _: &str) -> VerificationResult<SavedModel> {
        Err("saved_model_unavailable")
    }
    fn validate_kit(&self, _: &str, _: KitFixture) -> VerificationResult<KitValidationReceipt> {
        Err("kit_validator_unavailable")
    }
}

pub(crate) struct VerificationService {
    db: Arc<Database>,
    reader: Arc<dyn ProjectDependencyReader>,
    running: Mutex<HashMap<String, Arc<tokio::sync::Notify>>>,
}

impl VerificationService {
    pub(crate) fn new(db: Arc<Database>, reader: Arc<dyn ProjectDependencyReader>) -> Self {
        Self {
            db,
            reader,
            running: Mutex::new(HashMap::new()),
        }
    }

    fn dependency(&self, project: &str) -> VerificationResult<ProjectDependencySnapshot> {
        if !domain::valid_id(project) {
            return Err("invalid_project");
        }
        let snapshot = self.reader.read(project)?;
        if snapshot.project_id != project
            || !domain::valid_dependency(&snapshot)
            || !snapshot.active
        {
            return Err("project_unavailable");
        }
        Ok(snapshot)
    }

    fn expected(
        &self,
        project: &str,
        revision: u64,
    ) -> VerificationResult<ProjectDependencySnapshot> {
        let snapshot = self.dependency(project)?;
        if snapshot.project_revision != revision {
            return Err("project_changed");
        }
        Ok(snapshot)
    }

    pub(crate) fn snapshot(&self, project: &str) -> VerificationResult<VerificationSnapshot> {
        if !domain::valid_id(project) {
            return Err("invalid_project");
        }
        let now = Utc::now();
        let dependency = self.dependency(project).ok();
        let (records, revoked) = self
            .db
            .verification_records(project)
            .map_err(|_| "evidence_read_failed")?;
        let (handoff_revision, handoff) = self
            .db
            .verification_handoff(project)
            .map_err(|_| "handoff_read_failed")?;
        if records.iter().any(|e| !domain::valid_record(e, project)) {
            return Err("invalid_evidence");
        }
        domain::validate_handoff(&handoff)?;
        Ok(VerificationSnapshot {
            schema_version: 1,
            project_id: project.into(),
            project_revision: dependency.as_ref().map(|s| s.project_revision),
            available: dependency.is_some(),
            checked_at: domain::stamp(now),
            evidence: domain::views(&records, dependency.as_ref(), &revoked, now),
            handoff,
            handoff_revision,
        })
    }

    pub(crate) fn record_manual(
        &self,
        request: ManualRequest,
    ) -> VerificationResult<VerificationSnapshot> {
        let now = Utc::now();
        domain::validate_manual(&request, now)?;
        let dependency = self.expected(&request.project_id, request.expected_revision)?;
        let snapshot = self.snapshot(&request.project_id)?;
        let mut real_basis = request.external_basis.is_some();
        for id in &request.basis_evidence_ids {
            let basis = snapshot
                .evidence
                .iter()
                .find(|e| &e.id == id)
                .ok_or("invalid_basis")?;
            if basis.outcome != Outcome::Passed || basis.validity != Validity::Current {
                return Err("basis_not_current");
            }
            if domain::time(&basis.observed_at)? > domain::time(&request.observed_at)? {
                return Err("basis_after_acceptance");
            }
            real_basis |= domain::has_real_basis(id, &snapshot.evidence, &mut HashSet::new());
        }
        if request.stage == Stage::CustomerAccepted && !real_basis {
            return Err("real_acceptance_basis_required");
        }
        let observed = domain::time(&request.observed_at)?;
        let expiry = matches!(
            request.stage,
            Stage::AuthenticationAvailable | Stage::ToolCallable
        )
        .then(|| domain::stamp(observed + Duration::minutes(15)));
        let record = Evidence {
            id: uuid::Uuid::new_v4().to_string(),
            dependencies: dependency.clone(),
            stage: request.stage,
            outcome: request.outcome,
            source_class: SourceClass::ManualRecord,
            checker_id: "external_manual_record".into(),
            fixture: None,
            sample: None,
            checker_version: 1,
            app_version: env!("CARGO_PKG_VERSION").into(),
            observed_at: request.observed_at,
            recorded_at: domain::stamp(now),
            expires_at: expiry,
            reason_code: "manual_record".into(),
            basis_evidence_ids: request.basis_evidence_ids,
            manual: Some(ManualDetails {
                person: request.person,
                role: request.role,
                scope: request.scope,
                external_basis: request.external_basis,
            }),
            invalidated_during_run: false,
        };
        if self.dependency(&request.project_id)? != dependency {
            return Err("project_changed");
        }
        self.db
            .verification_append(&record)
            .map_err(|_| "evidence_persist_failed")?;
        self.snapshot(&request.project_id)
    }

    pub(crate) fn revoke(
        &self,
        request: RevokeRequest,
    ) -> VerificationResult<VerificationSnapshot> {
        if !domain::valid_id(&request.project_id) || !domain::valid_id(&request.evidence_id) {
            return Err("invalid_request");
        }
        if !self
            .db
            .verification_revoke(
                &request.project_id,
                &request.evidence_id,
                &domain::stamp(Utc::now()),
            )
            .map_err(|_| "revoke_failed")?
        {
            return Err("evidence_not_found");
        }
        self.snapshot(&request.project_id)
    }

    pub(crate) fn save_handoff(
        &self,
        request: SaveHandoffRequest,
    ) -> VerificationResult<VerificationSnapshot> {
        self.expected(&request.project_id, request.expected_revision)?;
        domain::validate_handoff(&request.handoff)?;
        self.db
            .verification_save_handoff(
                &request.project_id,
                request.handoff_revision,
                &request.handoff,
            )
            .map_err(|_| "handoff_conflict")?;
        self.snapshot(&request.project_id)
    }

    pub(crate) fn cancel(&self, project: &str) -> VerificationResult<bool> {
        if !domain::valid_id(project) {
            return Err("invalid_project");
        }
        let running = self.running.lock().map_err(|_| "check_unavailable")?;
        if let Some(notification) = running.get(project) {
            notification.notify_one();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn preview(&self, project: &str) -> VerificationResult<HandoffPreview> {
        export::preview(self.snapshot(project)?)
    }

    /// Destination is selected by the native dialog, never supplied over IPC.
    pub(crate) fn export_to(
        &self,
        project: &str,
        path: &std::path::Path,
        markdown: bool,
    ) -> VerificationResult<()> {
        let preview = self.preview(project)?;
        let content = if markdown {
            preview.markdown
        } else {
            preview.json
        };
        crate::config::atomic_write_private(path, content.as_bytes()).map_err(|_| "export_failed")
    }

    pub(crate) async fn run(
        self: &Arc<Self>,
        request: RunRequest,
    ) -> VerificationResult<VerificationSnapshot> {
        if !domain::valid_id(&request.project_id)
            || !domain::valid_id(&request.run_id)
            || (request.checker == Checker::KitValidator) != request.fixture.is_some()
        {
            return Err("invalid_request");
        }
        let cancellation = Arc::new(tokio::sync::Notify::new());
        {
            let mut running = self.running.lock().map_err(|_| "check_unavailable")?;
            if running.contains_key(&request.project_id) {
                return Err("check_busy");
            }
            running.insert(request.project_id.clone(), cancellation.clone());
        }
        struct Guard<'a>(&'a Mutex<HashMap<String, Arc<tokio::sync::Notify>>>, String);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Ok(mut set) = self.0.lock() {
                    set.remove(&self.1);
                }
            }
        }
        let _guard = Guard(&self.running, request.project_id.clone());
        let checker_id = match request.checker {
            Checker::SavedConfigurationReadback => "saved_configuration_readback",
            Checker::SavedModelProbe => "saved_model_probe",
            Checker::KitValidator => "kit_validator",
        };
        let this = self.clone();
        let run_id = request.run_id.clone();
        let existing = tokio::task::spawn_blocking(move || this.db.verification_find(&run_id))
            .await
            .map_err(|_| "evidence_read_failed")?
            .map_err(|_| "evidence_read_failed")?;
        if let Some(existing) = existing {
            if existing.dependencies.project_id != request.project_id
                || existing.dependencies.project_revision != request.expected_revision
                || existing.checker_id != checker_id
                || existing.fixture != request.fixture
            {
                return Err("run_id_conflict");
            }
            return self.snapshot(&request.project_id);
        }
        let this = self.clone();
        let project = request.project_id.clone();
        let dependency =
            tokio::task::spawn_blocking(move || this.expected(&project, request.expected_revision))
                .await
                .map_err(|_| "check_unavailable")??;
        let now = Utc::now();
        let mut sample = None;
        let execute = async {
            Ok::<_, &'static str>(match request.checker {
                Checker::SavedConfigurationReadback => (
                    Stage::ConfigurationSaved,
                    SourceClass::NativeLocal,
                    if dependency.configuration_saved {
                        Outcome::Passed
                    } else {
                        Outcome::Failed
                    },
                    if !dependency.configuration_saved {
                        "configuration_not_saved"
                    } else {
                        match dependency.projection {
                            ProjectionState::Confirmed => "saved_projection_confirmed",
                            ProjectionState::Different => "saved_projection_different",
                            ProjectionState::Unavailable => "saved_projection_unavailable",
                        }
                    },
                    1,
                ),
                Checker::SavedModelProbe => {
                    let reader = self.reader.clone();
                    let project = request.project_id.clone();
                    let model = tokio::task::spawn_blocking(move || reader.saved_model(&project))
                        .await
                        .map_err(|_| "saved_model_unavailable")?;
                    match model {
                        Err(_) => (
                            Stage::ToolCallable,
                            SourceClass::NativeRemote,
                            Outcome::Unsupported,
                            "saved_model_unavailable",
                            1,
                        ),
                        Ok(model) => {
                            // A missing credential epoch is not a license to issue a request with untracked credentials.
                            if dependency.credential_generation.is_none() {
                                (
                                    Stage::ToolCallable,
                                    SourceClass::NativeRemote,
                                    Outcome::Unknown,
                                    "credential_generation_unavailable",
                                    1,
                                )
                            } else {
                                let result = super::model_probe::probe_saved_identity(
                                    model.app,
                                    &model.base_url,
                                    &model.api_key,
                                    &model.model_id,
                                )
                                .await;
                                match result {
                                    super::model_probe::IdentityProbe::Matched => (
                                        Stage::ToolCallable,
                                        SourceClass::NativeRemote,
                                        Outcome::Passed,
                                        "model_identity_confirmed",
                                        1,
                                    ),
                                    super::model_probe::IdentityProbe::Unconfirmed => (
                                        Stage::ToolCallable,
                                        SourceClass::NativeRemote,
                                        Outcome::Unknown,
                                        "model_identity_unconfirmed",
                                        1,
                                    ),
                                    super::model_probe::IdentityProbe::Failed => (
                                        Stage::ToolCallable,
                                        SourceClass::NativeRemote,
                                        Outcome::Failed,
                                        "model_request_failed",
                                        1,
                                    ),
                                }
                            }
                        }
                    }
                }
                Checker::KitValidator => {
                    let reader = self.reader.clone();
                    let project = request.project_id.clone();
                    let selected = request.fixture.ok_or("invalid_request")?;
                    let receipt = tokio::task::spawn_blocking(move || {
                        reader.validate_kit(&project, selected)
                    })
                    .await
                    .map_err(|_| "kit_validator_unavailable")?;
                    match receipt {
                        Ok(receipt)
                            if dependency.kit.as_ref() == Some(&receipt.kit)
                                && receipt.fixture == selected
                                && receipt.passed
                                    == (receipt.sample.code == SampleCode::Ok
                                        && receipt.sample.matches_expectation)
                                && domain::valid_sample(&receipt.sample) =>
                        {
                            sample = Some(receipt.sample);
                            (
                                Stage::SamplePassed,
                                SourceClass::LocalFixture,
                                if receipt.passed {
                                    Outcome::Passed
                                } else {
                                    Outcome::Failed
                                },
                                "local_sample_checked",
                                receipt.validator_version,
                            )
                        }
                        _ => (
                            Stage::SamplePassed,
                            SourceClass::LocalFixture,
                            Outcome::Unsupported,
                            "kit_validator_unavailable",
                            1,
                        ),
                    }
                }
            })
        };
        let (stage, source, outcome, reason, checker_version) = tokio::select! {
            result=execute => result?,
            _=cancellation.notified() => {
                let (stage,source)=match request.checker {
                    Checker::SavedConfigurationReadback=>(Stage::ConfigurationSaved,SourceClass::NativeLocal),
                    Checker::SavedModelProbe=>(Stage::ToolCallable,SourceClass::NativeRemote),
                    Checker::KitValidator=>(Stage::SamplePassed,SourceClass::LocalFixture),
                };
                (stage,source,Outcome::Cancelled,"check_cancelled",1)
            }
        };
        let observed = Utc::now();
        let this = self.clone();
        let project = request.project_id.clone();
        tokio::task::spawn_blocking(move || {
            let invalidated = this.dependency(&project).as_ref() != Ok(&dependency);
            let record = Evidence {
                id: request.run_id,
                dependencies: dependency,
                stage,
                outcome,
                source_class: source,
                checker_id: checker_id.into(),
                fixture: request.fixture,
                sample,
                checker_version,
                app_version: env!("CARGO_PKG_VERSION").into(),
                observed_at: domain::stamp(observed),
                recorded_at: domain::stamp(Utc::now()),
                expires_at: matches!(stage, Stage::AuthenticationAvailable | Stage::ToolCallable)
                    .then(|| domain::stamp(observed + Duration::minutes(15))),
                reason_code: reason.into(),
                basis_evidence_ids: vec![],
                manual: None,
                invalidated_during_run: invalidated || observed < now,
            };
            this.db
                .verification_append(&record)
                .map_err(|_| "evidence_persist_failed")?;
            this.snapshot(&project)
        })
        .await
        .map_err(|_| "evidence_persist_failed")?
    }
}

#[cfg(test)]
mod tests;
