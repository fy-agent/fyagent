use super::types::*;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub(crate) fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id)
}

/// Deliberately structured labels, never raw logs, URLs, paths or documents.
pub(crate) fn safe_label(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.trim().is_empty()
        && value == value.trim()
        && value.chars().count() <= 160
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || " _-（）()，、".contains(c))
        && ![
            "secretref",
            "sk-",
            "bearer",
            "api_key",
            "apikey",
            "password",
            "token",
        ]
        .iter()
        .any(|s| lower.contains(s))
}

pub(crate) fn safe_reference(value: &str) -> bool {
    if safe_label(value) {
        return true;
    }
    if value.len() > 2048
        || value.trim() != value
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains(['\\', '%', '<', '>', '"', '[', ']'])
    {
        return false;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    let lower = value.to_ascii_lowercase();
    url.scheme() == "https"
        && value.starts_with("https://")
        && url
            .host_str()
            .is_some_and(|host| host.contains('.') && host != "localhost")
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && ![
            "secretref",
            "sk-",
            "bearer",
            "api_key",
            "apikey",
            "password",
            "token",
        ]
        .iter()
        .any(|s| lower.contains(s))
}

pub(crate) fn valid_sample(s: &SampleReceipt) -> bool {
    s.input_digest.len() == 64
        && s.input_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && s.validator == "weekly-report/v1"
        && s.source_row_ids.len() <= 6
        && s.source_row_ids.iter().all(|id| {
            matches!(
                id.as_str(),
                "row-1" | "row-2" | "row-3" | "row-4" | "row-5" | "row-6"
            )
        })
        && s.source_row_ids.iter().collect::<HashSet<_>>().len() == s.source_row_ids.len()
        && (s.code == SampleCode::Ok) == s.metrics.is_some()
        && s.metrics.as_ref().is_none_or(|m| {
            [
                m.current_minor,
                m.previous_minor,
                m.growth_bps,
                m.target_bps,
            ]
            .iter()
            .all(|v| (-9_007_199_254_740_991..=9_007_199_254_740_991).contains(v))
        })
}

pub(crate) fn time(value: &str) -> VerificationResult<DateTime<Utc>> {
    if !value.ends_with('Z') {
        return Err("invalid_time");
    }
    DateTime::parse_from_rfc3339(value)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| "invalid_time")
}

pub(crate) fn stamp(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub(crate) fn valid_dependency(snapshot: &ProjectDependencySnapshot) -> bool {
    valid_id(&snapshot.project_id)
        && snapshot.project_revision <= 9_007_199_254_740_991
        && !snapshot.resource_revision.is_empty()
        && snapshot.resource_revision.len() <= 256
        && snapshot
            .credential_generation
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.len() <= 256)
        && snapshot.kit.as_ref().is_none_or(|kit| {
            [&kit.kit_id, &kit.kit_version].iter().all(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c))
            }) && kit.manifest_digest.len() == 64
                && kit
                    .manifest_digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

pub(crate) fn valid_record(e: &Evidence, project: &str) -> bool {
    const REASONS: &[&str] = &[
        "manual_record",
        "configuration_not_saved",
        "saved_projection_confirmed",
        "saved_projection_different",
        "saved_projection_unavailable",
        "saved_model_unavailable",
        "credential_generation_unavailable",
        "model_identity_confirmed",
        "model_identity_unconfirmed",
        "model_request_failed",
        "local_sample_checked",
        "kit_validator_unavailable",
        "check_cancelled",
    ];
    valid_id(&e.id)
        && e.sample.as_ref().is_none_or(valid_sample)
        && (e.checker_id == "kit_validator") == e.fixture.is_some()
        && (e.sample.is_none() || e.fixture.is_some())
        && e.dependencies.project_id == project
        && valid_dependency(&e.dependencies)
        && [
            "saved_configuration_readback",
            "saved_model_probe",
            "kit_validator",
            "external_manual_record",
        ]
        .contains(&e.checker_id.as_str())
        && e.app_version.len() <= 128
        && e.app_version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c))
        && REASONS.contains(&e.reason_code.as_str())
        && time(&e.observed_at).is_ok()
        && time(&e.recorded_at).is_ok()
        && e.expires_at.as_ref().is_none_or(|t| time(t).is_ok())
        && e.basis_evidence_ids.len() <= 32
        && e.basis_evidence_ids.iter().all(|id| valid_id(id))
        && (e.source_class == SourceClass::ManualRecord) == e.manual.is_some()
        && (e.stage != Stage::CustomerAccepted || e.source_class == SourceClass::ManualRecord)
        && e.manual.as_ref().is_none_or(|m| {
            [&m.person, &m.role, &m.scope].iter().all(|s| safe_label(s))
                && m.external_basis
                    .as_ref()
                    .is_none_or(|b| safe_reference(&b.reference) && safe_label(&b.issuer))
        })
}

pub(crate) fn validate_manual(
    request: &ManualRequest,
    now: DateTime<Utc>,
) -> VerificationResult<()> {
    if !valid_id(&request.project_id)
        || ![&request.person, &request.role, &request.scope]
            .iter()
            .all(|s| safe_label(s))
        || request.basis_evidence_ids.len() > 32
        || request.basis_evidence_ids.iter().any(|id| !valid_id(id))
        || request
            .basis_evidence_ids
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != request.basis_evidence_ids.len()
        || time(&request.observed_at)? > now
    {
        return Err("invalid_request");
    }
    if let Some(basis) = &request.external_basis {
        if !safe_reference(&basis.reference) || !safe_label(&basis.issuer) {
            return Err("invalid_external_basis");
        }
    }
    if request.external_basis.is_none() && request.basis_evidence_ids.is_empty() {
        return Err("basis_required");
    }
    Ok(())
}

pub(crate) fn views(
    records: &[Evidence],
    current: Option<&ProjectDependencySnapshot>,
    revoked: &HashSet<String>,
    now: DateTime<Utc>,
) -> Vec<EvidenceView> {
    let superseded: HashSet<_> = records
        .iter()
        .enumerate()
        .filter_map(|(index, e)| {
            (e.outcome == Outcome::Passed
                && records[index + 1..].iter().any(|later| {
                    later.outcome != Outcome::Passed
                        && later.stage == e.stage
                        && later.checker_id == e.checker_id
                        && later.fixture == e.fixture
                        && later.dependencies == e.dependencies
                }))
            .then_some(e.id.as_str())
        })
        .collect();
    let by_id: HashMap<_, _> = records.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut memo: HashMap<_, _> = superseded
        .into_iter()
        .filter(|id| !revoked.contains(*id))
        .map(|id| (id, Validity::Stale))
        .collect();
    records
        .iter()
        .map(|e| {
            let validity = validity(
                e,
                current,
                revoked,
                now,
                &by_id,
                &mut HashSet::new(),
                &mut memo,
            );
            EvidenceView {
                id: e.id.clone(),
                project_revision: e.dependencies.project_revision,
                kit: e.dependencies.kit.clone(),
                stage: e.stage,
                outcome: e.outcome,
                validity,
                source_class: e.source_class,
                checker_id: e.checker_id.clone(),
                fixture: e.fixture,
                sample: e.sample.clone(),
                checker_version: e.checker_version,
                app_version: e.app_version.clone(),
                observed_at: e.observed_at.clone(),
                recorded_at: e.recorded_at.clone(),
                expires_at: e.expires_at.clone(),
                reason_code: if validity == Validity::Current {
                    e.reason_code.clone()
                } else {
                    match validity {
                        Validity::Stale => "dependencies_changed",
                        Validity::Revoked => "revoked",
                        _ => "dependency_unverifiable",
                    }
                    .into()
                },
                basis_evidence_ids: e.basis_evidence_ids.clone(),
                manual: e.manual.clone(),
            }
        })
        .collect()
}

fn validity<'a>(
    e: &'a Evidence,
    current: Option<&ProjectDependencySnapshot>,
    revoked: &HashSet<String>,
    now: DateTime<Utc>,
    by_id: &HashMap<&str, &'a Evidence>,
    visiting: &mut HashSet<&'a str>,
    memo: &mut HashMap<&'a str, Validity>,
) -> Validity {
    if let Some(result) = memo.get(e.id.as_str()) {
        return *result;
    }
    if visiting.len() >= 128 || !visiting.insert(&e.id) {
        return Validity::Unverifiable;
    }
    let result = calculate_validity(e, current, revoked, now, by_id, visiting, memo);
    visiting.remove(e.id.as_str());
    memo.insert(e.id.as_str(), result);
    result
}

fn calculate_validity<'a>(
    e: &'a Evidence,
    current: Option<&ProjectDependencySnapshot>,
    revoked: &HashSet<String>,
    now: DateTime<Utc>,
    by_id: &HashMap<&str, &'a Evidence>,
    visiting: &mut HashSet<&'a str>,
    memo: &mut HashMap<&'a str, Validity>,
) -> Validity {
    if revoked.contains(&e.id) {
        return Validity::Revoked;
    }
    let Some(current) = current else {
        return Validity::Unverifiable;
    };
    if !current.active || &e.dependencies != current || e.invalidated_during_run {
        return Validity::Stale;
    }
    if time(&e.observed_at).map_or(true, |t| t > now)
        || time(&e.recorded_at).map_or(true, |t| t > now)
    {
        return Validity::Unverifiable;
    }
    if e.expires_at
        .as_ref()
        .is_some_and(|v| time(v).map_or(true, |t| t <= now))
    {
        return Validity::Stale;
    }
    // None means the native owner cannot track credential changes, not that a
    // credential is absent. Even a genuine external manual fact remains stored
    // but cannot be presented as current authority under an unknown dependency.
    if e.stage != Stage::ConfigurationSaved
        && current.credential_generation.is_none()
        && e.source_class != SourceClass::LocalFixture
    {
        return Validity::Unverifiable;
    }
    for id in &e.basis_evidence_ids {
        let Some(basis) = by_id.get(id.as_str()) else {
            return Validity::Unverifiable;
        };
        if basis.dependencies.project_id != e.dependencies.project_id
            || basis.outcome != Outcome::Passed
        {
            return Validity::Stale;
        }
        if validity(basis, Some(current), revoked, now, by_id, visiting, memo) != Validity::Current
        {
            return Validity::Stale;
        }
    }
    Validity::Current
}

pub(crate) fn validate_handoff(notes: &HandoffNotes) -> VerificationResult<()> {
    if notes.items.len() > 100
        || notes
            .items
            .iter()
            .any(|i| !safe_label(&i.title) || i.owner.as_ref().is_some_and(|s| !safe_label(s)))
    {
        return Err("invalid_handoff");
    }
    Ok(())
}

pub(crate) fn has_real_basis(
    id: &str,
    evidence: &[EvidenceView],
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(id.into()) {
        return false;
    }
    let Some(record) = evidence.iter().find(|e| e.id == id) else {
        return false;
    };
    if record.validity != Validity::Current || record.outcome != Outcome::Passed {
        return false;
    }
    match record.source_class {
        SourceClass::LocalFixture => false,
        SourceClass::NativeRemote => true,
        SourceClass::NativeLocal => false,
        SourceClass::ManualRecord => {
            record
                .manual
                .as_ref()
                .is_some_and(|m| m.external_basis.is_some())
                || record
                    .basis_evidence_ids
                    .iter()
                    .any(|id| has_real_basis(id, evidence, visited))
        }
    }
}
