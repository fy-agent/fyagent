//! Exact local proxy records only; no imported usage or global request counters.
use super::types::{
    HealthAction as Action, HealthCheck, HealthCheckId as Id, HealthCheckState as State,
    HealthReasonCode as Reason, HealthSeverity,
};
use crate::database::Database;
use chrono::{DateTime, SecondsFormat};
pub(super) fn request_check(
    db: &Database,
    app: &str,
    provider: Option<&str>,
    at: &str,
) -> HealthCheck {
    let mut check = HealthCheck::new(
        Id::LastRequest,
        State::Unknown,
        Reason::RequestNotRecorded,
        Some(Action::ModelTest),
        at,
    );
    check.severity = HealthSeverity::Info;
    match db.latest_health_request(app, provider) {
        Ok(Some(evidence)) => {
            check.state = if (200..300).contains(&evidence.status_code) {
                State::Ok
            } else {
                State::Attention
            };
            check.reason_code = if check.state == State::Ok {
                Reason::RequestSucceeded
            } else {
                Reason::RequestFailed
            };
            check.severity = if check.state == State::Ok {
                HealthSeverity::Info
            } else {
                HealthSeverity::Warning
            };
            check.evidence_at = DateTime::from_timestamp(evidence.created_at, 0)
                .map(|time| time.to_rfc3339_opts(SecondsFormat::Millis, true));
        }
        Ok(None) => {}
        Err(_) => {
            check.reason_code = Reason::ReadFailed;
        }
    }
    check
}
