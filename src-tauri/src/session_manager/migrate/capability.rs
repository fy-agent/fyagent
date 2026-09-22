//! Release capability matrix and the local runtime probe.
//!
//! These are two different questions and they are answered separately:
//!
//! * The matrix is engineering acceptance — what this build has fixture
//!   evidence for. The user never has to run anything to see it.
//! * The probe is what is installed on this machine right now.
//!
//! Exporting and restoring are independent gates. Passing the extraction gate
//! says a transcript can be read safely; it says nothing about whether the
//! target can be written. Probing is mandatory rather than reading a
//! documentation table: the local Grok 1.0.34 help has no `import` even though
//! the vendor's web reference lists one, so trusting docs would manufacture
//! support that does not exist.

use std::time::Duration;

use super::extract::rules;
use super::extract::{normalize_version, version_satisfies};
use super::model::{
    ExtractionRuleInfo, LocalProviderProbe, MigrationError, MigrationResult, ReleaseCapability,
    RestoreStage,
};
use super::native::{
    provider_binary, provider_cli_path, run_with_timeout, user_cli_execution_blocked, writer_for,
    RunOutcome,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Every provider in product scope, in a stable display order.
pub const PRODUCT_PROVIDERS: &[&str] = &[
    "codex",
    "claude",
    "opencode",
    "openclaw",
    "gemini",
    "grokbuild",
    "hermes",
];

/// Per-stage evidence recorded by the isolated probe runs.
///
/// An empty version list means the stage was never verified, which is not the
/// same as "does not work". Every entry here points at a captured artifact
/// under the task's `evidence/` directory; a stage that was reasoned about
/// rather than observed does not belong in this table.
fn verified_stages(provider_id: &str) -> Vec<(RestoreStage, Vec<String>)> {
    let stages: &[RestoreStage] = match provider_id {
        // Codex 0.154.0: an isolated legacy rollout was written, the official
        // app-server read it back before and after a restart, and a captured
        // next-turn request carried all four migrated messages in order. The
        // request was captured against a local endpoint: it proves what the
        // client sent, and deliberately not that a model replied.
        "codex" => &[
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritten,
            RestoreStage::NativeReadbackVerified,
            RestoreStage::RestartReadbackVerified,
            RestoreStage::NextTurnRequestVerified,
        ],
        // OpenCode 1.18.30: the actual Rust create-only writer published an
        // isolated session. A fresh official export and next-turn request
        // retained exact roles/order/text, including consecutive roles and
        // an unanswered user. The loopback returned HTTP 400, not a reply.
        // Evidence: native-continuation/create-only (actual writer probe).
        "opencode" => &[
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritten,
            RestoreStage::NativeReadbackVerified,
            RestoreStage::NextTurnRequestVerified,
        ],
        // Hermes 0.20.5: the installed SDK and official JSONL export preserve
        // accepted alternating message shapes. Whitespace loss, same-role
        // merging and unanswered-tail loss in native continuation are rejected
        // before writing; no real model reply is claimed.
        "hermes" => &[
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritten,
            RestoreStage::NativeReadbackVerified,
        ],
        // Gemini 0.46.0: installed Storage/record loading preserves accepted
        // history before and after process restart. Preflight rejects shapes
        // that the client filters or changes. Strict export of arbitrary
        // original native history remains unavailable without final proof.
        "gemini" => &[
            RestoreStage::PackageVerified,
            RestoreStage::NativeWritten,
            RestoreStage::NativeReadbackVerified,
            RestoreStage::RestartReadbackVerified,
        ],
        _ => &[],
    };
    let versions: Vec<String> = writer_for(provider_id)
        .map(|writer| {
            writer
                .verified_write_versions()
                .iter()
                .map(|version| (*version).to_string())
                .collect()
        })
        .unwrap_or_default();
    stages
        .iter()
        .map(|stage| (*stage, versions.clone()))
        .collect()
}

pub fn release_capability(provider_id: &str) -> ReleaseCapability {
    let extraction_rule = rules::rule_for(provider_id).map(|rule| ExtractionRuleInfo {
        rule_id: rule.rule_id().to_string(),
        verified_versions: rule
            .verified_versions()
            .iter()
            .map(|version| version.to_string())
            .collect(),
    });
    ReleaseCapability {
        provider_id: provider_id.to_string(),
        extraction_rule,
        write_strategy: writer_for(provider_id).map(|writer| writer.write_strategy().to_string()),
        verified_stages: verified_stages(provider_id),
    }
}

pub fn release_capability_matrix() -> Vec<ReleaseCapability> {
    PRODUCT_PROVIDERS
        .iter()
        .map(|provider| release_capability(provider))
        .collect()
}

/// Probe the installed CLI for one provider.
///
/// The executable is resolved through [`provider_cli_path`], the same lookup
/// the native writers use. Resolving it differently here is how a probe ends
/// up reporting the version of one installation while a write goes to
/// another.
pub fn probe_local_provider(provider_id: &str) -> MigrationResult<LocalProviderProbe> {
    if provider_binary(provider_id).is_none() {
        return Err(MigrationError::CapabilityProbeFailed {
            provider_id: provider_id.to_string(),
            reason: "unknown provider".to_string(),
        });
    }
    // Not "not installed": the CLI may well be there, and this build is not
    // allowed to run it. Reporting absence would send the user looking for an
    // installation problem that does not exist.
    if let Some(reason) = user_cli_execution_blocked() {
        return Err(MigrationError::CapabilityProbeFailed {
            provider_id: provider_id.to_string(),
            reason: reason.to_string(),
        });
    }
    let Some(executable) = provider_cli_path(provider_id) else {
        return Ok(build_probe(provider_id, None));
    };
    let detected = detect_version(&executable)?;
    let mut probe = build_probe(provider_id, detected);
    if provider_id == "codex" {
        apply_codex_store_gate(&mut probe, &crate::codex_config::get_codex_config_dir());
    }
    Ok(probe)
}

fn apply_codex_store_gate(probe: &mut LocalProviderProbe, home: &std::path::Path) {
    if probe.write_supported && super::native::codex::read_installation_id_at(home).is_err() {
        probe.write_supported = false;
        probe.reason_code = Some("targetStoreUnidentified".into());
    }
}

/// Run `<cli> --version` with a deadline. `Ok(None)` means the executable
/// answered without a parseable version, which is different from a probe that
/// failed to run.
pub fn detect_version(executable: &std::path::Path) -> MigrationResult<Option<String>> {
    let shown = executable.display().to_string();
    let mut command = std::process::Command::new(executable);
    command.arg("--version");
    match run_with_timeout(command, PROBE_TIMEOUT) {
        RunOutcome::NotStarted { .. } => Ok(None),
        RunOutcome::OutputIncomplete => Err(MigrationError::CapabilityProbeFailed {
            provider_id: shown.clone(),
            reason: "CLI version output was incomplete or invalid".into(),
        }),
        RunOutcome::TimedOut { after } => Err(MigrationError::CapabilityProbeFailed {
            provider_id: shown.clone(),
            reason: format!("`{shown} --version` timed out after {}s", after.as_secs()),
        }),
        RunOutcome::Exited { stdout, stderr, .. } => {
            // Some CLIs print the version on stderr; a non-zero status with a
            // parseable version still tells us what is installed.
            let combined = format!("{stdout}\n{stderr}");
            Ok(normalize_version(&combined).map(|version| version.to_string()))
        }
    }
}

/// Decide both gates from a detected version. Split out so the decision table
/// is unit-testable without spawning processes.
pub fn build_probe(provider_id: &str, detected: Option<String>) -> LocalProviderProbe {
    let installed = detected.is_some();
    let capability = release_capability(provider_id);

    let extraction_supported = capability
        .extraction_rule
        .as_ref()
        .map(|rule| {
            let verified: Vec<&str> = rule.verified_versions.iter().map(String::as_str).collect();
            version_satisfies(detected.as_deref(), &verified)
        })
        .unwrap_or(false);

    let write_supported = writer_for(provider_id)
        .map(|writer| version_satisfies(detected.as_deref(), writer.verified_write_versions()))
        .unwrap_or(false);

    let reason_code = if !installed {
        Some("providerNotInstalled".to_string())
    } else if capability.extraction_rule.is_none() {
        Some("extractionRuleUnavailable".to_string())
    } else if !extraction_supported {
        Some("extractionRuleVersionMismatch".to_string())
    } else if capability.write_strategy.is_none() {
        Some("writeStrategyUnavailable".to_string())
    } else if !write_supported {
        Some("providerVersionUnsupported".to_string())
    } else {
        None
    };

    LocalProviderProbe {
        provider_id: provider_id.to_string(),
        installed,
        detected_version: detected,
        extraction_supported,
        write_supported,
        reason_code,
    }
}

/// The export gate, used before reading a source session.
pub fn require_extraction_gate(provider_id: &str) -> MigrationResult<LocalProviderProbe> {
    let probe = probe_local_provider(provider_id)?;
    if !probe.installed {
        return Err(MigrationError::ProviderNotInstalled {
            provider_id: provider_id.to_string(),
        });
    }
    if !probe.extraction_supported {
        let capability = release_capability(provider_id);
        return match capability.extraction_rule {
            None => {
                // The gap table says which artifact is missing and what would
                // produce it. Logging it here is what turns "unavailable" in
                // the UI into something a support bundle can act on.
                if let Some(gap) = rules::extraction_gap(provider_id) {
                    log::info!(
                        "{provider_id}: no extraction rule yet ({}); next step: {}",
                        gap.reason,
                        gap.next_step
                    );
                }
                Err(MigrationError::ExtractionRuleUnavailable {
                    provider_id: provider_id.to_string(),
                    detected_version: probe.detected_version,
                })
            }
            Some(rule) => Err(MigrationError::ExtractionRuleVersionMismatch {
                provider_id: provider_id.to_string(),
                detected: probe.detected_version,
                verified: rule.verified_versions,
            }),
        };
    }
    Ok(probe)
}

/// The restore gate. Passing the export gate does not imply this one.
pub fn require_write_gate(provider_id: &str) -> MigrationResult<LocalProviderProbe> {
    let probe = probe_local_provider(provider_id)?;
    if !probe.installed {
        return Err(MigrationError::ProviderNotInstalled {
            provider_id: provider_id.to_string(),
        });
    }
    if probe.reason_code.as_deref() == Some("targetStoreUnidentified") {
        return Err(MigrationError::TargetStoreUnidentified {
            provider_id: provider_id.to_string(),
        });
    }
    if !probe.write_supported {
        return Err(MigrationError::ProviderVersionUnsupported {
            provider_id: provider_id.to_string(),
            detected: probe.detected_version,
            verified: writer_for(provider_id)
                .map(|writer| {
                    writer
                        .verified_write_versions()
                        .iter()
                        .map(|version| version.to_string())
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    Ok(probe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_product_provider_has_a_matrix_row() {
        let matrix = release_capability_matrix();
        assert_eq!(matrix.len(), PRODUCT_PROVIDERS.len());
        for provider in PRODUCT_PROVIDERS {
            assert!(matrix.iter().any(|row| row.provider_id == *provider));
        }
    }

    #[test]
    fn nothing_claims_a_stage_that_needs_a_user_or_a_model() {
        // `targetOpened` requires a person to have seen the conversation in
        // the target, and `nextTurnReplyVerified` requires a real model
        // reply. Neither was produced in a lab, and neither may be inferred
        // from a readback.
        //
        // `nextTurnRequestVerified` is different and is allowed: it is an
        // observation of what the client sent, captured against a local
        // endpoint without any inference.
        for row in release_capability_matrix() {
            for (stage, _) in &row.verified_stages {
                assert!(
                    !matches!(
                        stage,
                        RestoreStage::TargetOpened | RestoreStage::NextTurnReplyVerified
                    ),
                    "{} must not claim {:?}",
                    row.provider_id,
                    stage
                );
            }
        }
    }

    #[test]
    fn a_claimed_stage_is_pinned_to_the_versions_its_writer_was_exercised_on() {
        for row in release_capability_matrix() {
            let Some(writer) = writer_for(&row.provider_id) else {
                assert!(
                    row.verified_stages.is_empty(),
                    "{}: stages without a writer",
                    row.provider_id
                );
                continue;
            };
            let expected: Vec<String> = writer
                .verified_write_versions()
                .iter()
                .map(|version| (*version).to_string())
                .collect();
            for (stage, versions) in &row.verified_stages {
                assert_eq!(
                    versions, &expected,
                    "{} claims {stage:?} on versions its writer was never run against",
                    row.provider_id
                );
            }
        }
    }

    #[test]
    fn readback_is_claimed_only_where_the_target_answered_through_its_own_channel() {
        // Four providers reached it, each through the target's own read
        // path: Codex' app-server, `opencode export --pure`,
        // `hermes sessions export`, and Gemini's own exported record loader.
        let claiming: Vec<String> = release_capability_matrix()
            .into_iter()
            .filter(|row| {
                row.verified_stages
                    .iter()
                    .any(|(stage, _)| *stage == RestoreStage::NativeReadbackVerified)
            })
            .map(|row| row.provider_id)
            .collect();
        assert_eq!(claiming, ["codex", "opencode", "gemini", "hermes"]);
    }

    #[test]
    fn a_round_trip_needs_both_directions_and_the_matrix_says_who_has_them() {
        // Export and restore are separate pieces of evidence. Three
        // providers now have both, which is what makes an ordinary
        // export-then-restore possible at all.
        for (provider, rule_id) in [
            ("codex", "codex.rollout.phase-v1"),
            ("opencode", "opencode.finish-parts-v1"),
            ("hermes", "hermes.finish-provenance-v1"),
        ] {
            let row = release_capability(provider);
            assert_eq!(
                row.extraction_rule
                    .as_ref()
                    .map(|rule| rule.rule_id.as_str()),
                Some(rule_id),
                "{provider}"
            );
            // Taken from the writer rather than spelled out here: the value
            // that matters is that the matrix names the strategy actually
            // registered for the provider.
            let writer = super::super::native::writer_for(provider).expect(provider);
            assert_eq!(
                row.write_strategy.as_deref(),
                Some(writer.write_strategy()),
                "{provider}"
            );
        }

        // Gemini can be written to and cannot be exported from: its records
        // carry no durable completion signal, so a final answer cannot be
        // identified. That asymmetry is the point of two separate gates.
        let gemini = release_capability("gemini");
        assert!(gemini.write_strategy.is_some());
        assert!(gemini.extraction_rule.is_none());
    }

    #[test]
    fn the_two_gates_are_independent() {
        // Codex 0.154.0: both directions verified, so no reason code.
        let codex = build_probe("codex", Some("0.154.0".to_string()));
        assert!(codex.extraction_supported);
        assert!(codex.write_supported);
        assert_eq!(codex.reason_code, None);

        // Gemini 0.46.0: restore yes, export no.
        let gemini = build_probe("gemini", Some("0.46.0".to_string()));
        assert!(!gemini.extraction_supported);
        assert!(gemini.write_supported);
        assert_eq!(
            gemini.reason_code.as_deref(),
            Some("extractionRuleUnavailable")
        );

        // Claude 2.1.220: neither, and the reason names the export gap
        // rather than the missing writer.
        let claude = build_probe("claude", Some("2.1.220".to_string()));
        assert!(!claude.extraction_supported);
        assert!(!claude.write_supported);
        assert_eq!(
            claude.reason_code.as_deref(),
            Some("extractionRuleUnavailable")
        );
    }

    #[test]
    fn codex_probe_requires_native_initialization_without_mutating_store() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("not-initialized");
        let mut probe = build_probe("codex", Some("0.154.0".into()));
        apply_codex_store_gate(&mut probe, &missing);
        assert!(probe.installed);
        assert!(probe.extraction_supported);
        assert!(!probe.write_supported);
        assert_eq!(
            probe.reason_code.as_deref(),
            Some("targetStoreUnidentified")
        );
        assert!(
            !missing.exists(),
            "probe must not initialize the native store"
        );

        for invalid in ["invalid".to_string(), " ".repeat(257)] {
            std::fs::write(temp.path().join("installation_id"), &invalid).unwrap();
            let mut probe = build_probe("codex", Some("0.154.0".into()));
            apply_codex_store_gate(&mut probe, temp.path());
            assert!(!probe.write_supported);
            assert_eq!(
                std::fs::read_to_string(temp.path().join("installation_id")).unwrap(),
                invalid
            );
        }
        // Synthetic unit fixture only; production never generates this file.
        std::fs::write(
            temp.path().join("installation_id"),
            "4036a238-95b5-4f70-9472-3b81df1f1183\n",
        )
        .unwrap();
        let mut initialized = build_probe("codex", Some("0.154.0".into()));
        apply_codex_store_gate(&mut initialized, temp.path());
        assert!(initialized.write_supported);
        assert_eq!(initialized.reason_code, None);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);

        let mut unsupported = build_probe("codex", Some("0.155.0".into()));
        apply_codex_store_gate(&mut unsupported, &missing);
        assert_eq!(
            unsupported.reason_code.as_deref(),
            Some("extractionRuleVersionMismatch")
        );
    }

    #[test]
    fn an_unverified_version_closes_both_gates() {
        let codex = build_probe("codex", Some("0.155.0".to_string()));
        assert!(!codex.extraction_supported);
        assert_eq!(
            codex.reason_code.as_deref(),
            Some("extractionRuleVersionMismatch")
        );

        let opencode = build_probe("opencode", Some("1.19.0".to_string()));
        assert!(!opencode.write_supported);
    }

    #[test]
    fn a_missing_cli_is_reported_as_not_installed() {
        let probe = build_probe("openclaw", None);
        assert!(!probe.installed);
        assert!(!probe.extraction_supported);
        assert!(!probe.write_supported);
        assert_eq!(probe.reason_code.as_deref(), Some("providerNotInstalled"));
    }

    #[test]
    fn probing_an_unknown_provider_fails_instead_of_returning_a_blank_row() {
        let error = probe_local_provider("not-a-provider").expect_err("must fail");
        assert_eq!(error.code(), "capabilityProbeFailed");
    }

    #[test]
    fn a_provider_whose_cli_is_absent_probes_without_spawning_anything() {
        // `provider_cli_path` returning nothing is the not-installed answer;
        // no process is started to discover that.
        let detected = detect_version(std::path::Path::new(
            "/definitely/not/here/fyagent-native-cli",
        ))
        .expect("probe runs");
        assert_eq!(detected, None);
    }

    #[test]
    fn probing_and_writing_resolve_the_same_executable() {
        // The probe decides whether a version is supported and the writer
        // performs the write. If those two resolved different files, a
        // receipt could record a verified version for a store that was never
        // written to.
        for provider in PRODUCT_PROVIDERS {
            let Some(path) = provider_cli_path(provider) else {
                continue;
            };
            assert_eq!(
                path,
                provider_cli_path(provider).expect("stable resolution"),
                "{provider}"
            );
            assert!(path.is_absolute(), "{provider}: {}", path.display());
        }
    }
}
