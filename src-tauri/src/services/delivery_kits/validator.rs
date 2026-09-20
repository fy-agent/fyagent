use super::schema::{
    canonical, digest, strict_json, FixtureKind, KitError, Manifest, Result, Scenario, Validator,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WeeklyInput {
    current_period: String,
    previous_period: String,
    currency: String,
    target_minor: i64,
    rows: Vec<Row>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Row {
    row_id: String,
    period: String,
    currency: String,
    revenue_minor: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BusinessCode {
    Ok,
    InvalidInput,
    DuplicateRow,
    PeriodMismatch,
    CurrencyMismatch,
    ZeroPrevious,
    MissingPeriod,
    InvalidAmount,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Metrics {
    pub current_minor: i64,
    pub previous_minor: i64,
    pub growth_bps: i64,
    pub target_bps: i64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaseResult {
    pub fixture_id: String,
    pub input_digest: String,
    pub code: BusinessCode,
    pub metrics: Option<Metrics>,
    pub source_row_ids: Vec<String>,
    pub matches_expectation: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DemoResult {
    pub kit_id: String,
    pub kit_version: String,
    pub manifest_digest: String,
    pub source_class: &'static str,
    pub validator_version: &'static str,
    pub checked_at: String,
    pub cases: Vec<CaseResult>,
}

fn evaluate(bytes: &[u8]) -> (BusinessCode, Option<Metrics>, Vec<String>) {
    let Ok(value) = strict_json(bytes) else {
        return (BusinessCode::InvalidInput, None, vec![]);
    };
    let Ok(input) = serde_json::from_value::<WeeklyInput>(value) else {
        return (BusinessCode::InvalidInput, None, vec![]);
    };
    let fail = |code| (code, None, vec![]);
    if input.rows.is_empty()
        || input.rows.len() > 1000
        || input.current_period == input.previous_period
        || input.current_period.is_empty()
        || input.previous_period.is_empty()
        || input.currency.len() != 3
    {
        return fail(BusinessCode::InvalidInput);
    }
    if input.target_minor <= 0 || input.target_minor > 1_000_000_000_000 {
        return fail(BusinessCode::InvalidAmount);
    }
    let (mut current, mut previous) = (0i64, 0i64);
    let (mut current_rows, mut previous_rows) = (0, 0);
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    for row in input.rows {
        if !super::schema::valid_id(&row.row_id) {
            return fail(BusinessCode::InvalidInput);
        }
        if !seen.insert(row.row_id.clone()) {
            return fail(BusinessCode::DuplicateRow);
        }
        if row.currency != input.currency {
            return fail(BusinessCode::CurrencyMismatch);
        }
        if row.revenue_minor < 0 || row.revenue_minor > 1_000_000_000_000 {
            return fail(BusinessCode::InvalidAmount);
        }
        if row.period == input.current_period {
            current += row.revenue_minor;
            current_rows += 1;
        } else if row.period == input.previous_period {
            previous += row.revenue_minor;
            previous_rows += 1;
        } else {
            return fail(BusinessCode::PeriodMismatch);
        }
        sources.push(row.row_id);
    }
    if current_rows == 0 || previous_rows == 0 {
        return fail(BusinessCode::MissingPeriod);
    }
    if previous == 0 {
        return fail(BusinessCode::ZeroPrevious);
    }
    // Bounded integer inputs and i128 intermediate arithmetic avoid overflow and float drift.
    let ratio = |n: i64, d: i64| -> Option<i64> {
        let n = i128::from(n) * 10000;
        let rounded = (n + if n >= 0 {
            i128::from(d) / 2
        } else {
            -i128::from(d) / 2
        }) / i128::from(d);
        if rounded.abs() > 9_007_199_254_740_991 {
            return None;
        }
        i64::try_from(rounded).ok()
    };
    let (Some(growth_bps), Some(target_bps)) = (
        ratio(current - previous, previous),
        ratio(current, input.target_minor),
    ) else {
        return fail(BusinessCode::InvalidAmount);
    };
    (
        BusinessCode::Ok,
        Some(Metrics {
            current_minor: current,
            previous_minor: previous,
            growth_bps,
            target_bps,
        }),
        sources,
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Expected {
    code: BusinessCode,
    current_minor: Option<i64>,
    previous_minor: Option<i64>,
    growth_bps: Option<i64>,
    target_bps: Option<i64>,
}

pub(crate) fn run(m: &Manifest) -> Result<DemoResult> {
    if m.scenario != Scenario::WeeklyReport {
        return Err(KitError::UnsupportedValidator);
    }
    let check = m
        .checks
        .iter()
        .find(|x| x.validator == Validator::WeeklyReportV1)
        .ok_or(KitError::UnsupportedValidator)?;
    let mut cases = Vec::new();
    for id in &check.fixture_ids {
        let f = m
            .fixtures
            .iter()
            .find(|x| &x.id == id)
            .ok_or(KitError::InvalidPackage)?;
        let resource = |id: &str| {
            m.resources
                .iter()
                .find(|x| x.id == id)
                .ok_or(KitError::InvalidPackage)
        };
        let input = resource(&f.input_resource_id)?;
        let expected = resource(&f.expected_resource_id)?;
        let e: Expected = serde_json::from_value(strict_json(expected.text.as_bytes())?)
            .map_err(|_| KitError::InvalidPackage)?;
        let (code, metrics, sources) = evaluate(input.text.as_bytes());
        let matches = code == e.code
            && match &metrics {
                Some(x) => {
                    f.kind == FixtureKind::Positive
                        && e.current_minor == Some(x.current_minor)
                        && e.previous_minor == Some(x.previous_minor)
                        && e.growth_bps == Some(x.growth_bps)
                        && e.target_bps == Some(x.target_bps)
                }
                None => {
                    f.kind == FixtureKind::Negative
                        && e.current_minor.is_none()
                        && e.previous_minor.is_none()
                        && e.growth_bps.is_none()
                        && e.target_bps.is_none()
                }
            };
        cases.push(CaseResult {
            fixture_id: id.clone(),
            input_digest: digest(input.text.as_bytes()),
            code,
            metrics,
            source_row_ids: sources,
            matches_expectation: matches,
        });
    }
    Ok(DemoResult {
        kit_id: m.id.clone(),
        kit_version: m.version.clone(),
        manifest_digest: digest(&canonical(m)?),
        source_class: "local_fixture",
        validator_version: "weekly-report/v1",
        checked_at: chrono::Utc::now().to_rfc3339(),
        cases,
    })
}
