//! Extraction rule registry.
//!
//! A provider appears here only once a rule has been proven against a real
//! artifact and pinned to concrete versions. Providers without a rule are not
//! given a stub that answers "unsupported": they are listed in
//! [`extraction_gap`] with the specific missing evidence and the next step, so
//! the gap stays visible instead of looking like a product decision.

pub mod codex;
pub mod hermes;
pub mod opencode;

use super::ExtractionRule;

pub fn rule_for(provider_id: &str) -> Option<&'static dyn ExtractionRule> {
    match provider_id {
        "codex" => Some(&codex::CodexRolloutPhaseV1),
        "opencode" => Some(&opencode::OpenCodeFinishV1),
        "hermes" => Some(&hermes::HermesFinishV1),
        _ => None,
    }
}

/// Why a provider has no extraction rule yet, and what would produce one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionGap {
    pub provider_id: &'static str,
    /// The candidate completion signal that the format is already known to
    /// carry, or the fact that no such field is known.
    pub candidate_signal: &'static str,
    /// Why the candidate is not yet a rule.
    pub reason: &'static str,
    /// The concrete artifact that would unblock it.
    pub next_step: &'static str,
}

/// Every provider in product scope that cannot export yet.
///
/// These are "no reader field / no fixture" gaps, not "the format has no such
/// information". Verification order is not a permanent product limit.
pub const EXTRACTION_GAPS: &[ExtractionGap] = &[
    ExtractionGap {
        provider_id: "claude",
        candidate_signal:
            "`message.stop_reason`, plus `isSidechain`/`parentUuid` to drop subagents",
        reason: "the reader never reads stop_reason; only the format is known, with no artifact \
                 showing which values appear on a completed turn",
        next_step: "capture an isolated Claude Code 2.1.220 transcript covering a completed turn, \
                    an interrupted turn and a subagent branch",
    },
    ExtractionGap {
        provider_id: "openclaw",
        candidate_signal: "none known",
        reason: "no completion field has been identified in the on-disk format",
        next_step: "capture an isolated OpenClaw 2026.7.1-2 session and look for a per-message \
                    terminal marker; sessionId also changes on daily/idle reset",
    },
    ExtractionGap {
        provider_id: "gemini",
        candidate_signal: "native records omit finishReason",
        reason: "installed Gemini 0.46.0 persists structurally identical gemini records before checking \
                 a missing stream finish reason; no-tool text alone cannot prove final completion",
        next_step: "verify an isolated version with durable per-turn completion provenance; \
                    see implementation/gemini-native-report.md and its missing-finish counterexample",
    },
    ExtractionGap {
        provider_id: "grokbuild",
        candidate_signal: "none known",
        reason: "the authoritative recovery log is `updates.jsonl` (an ACP event sequence) while \
                 the reader reads `chat_history.jsonl`, a different view; filtering the event \
                 sequence would break it",
        next_step: "capture an isolated Grok Build 1.0.34 session and decide which of the two \
                    files can yield a final-answer marker without breaking the event sequence",
    },
];

pub fn extraction_gap(provider_id: &str) -> Option<&'static ExtractionGap> {
    EXTRACTION_GAPS
        .iter()
        .find(|gap| gap.provider_id == provider_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRODUCT_PROVIDERS: &[&str] = &[
        "codex",
        "claude",
        "opencode",
        "openclaw",
        "gemini",
        "grokbuild",
        "hermes",
    ];

    #[test]
    fn every_product_provider_either_has_a_rule_or_a_named_gap() {
        // All seven stay in product scope. A provider must never be silently
        // absent from both lists.
        for provider in PRODUCT_PROVIDERS {
            let has_rule = rule_for(provider).is_some();
            let has_gap = extraction_gap(provider).is_some();
            assert!(
                has_rule ^ has_gap,
                "{provider}: rule={has_rule} gap={has_gap}"
            );
        }
    }

    #[test]
    fn gaps_state_a_concrete_next_step() {
        for gap in EXTRACTION_GAPS {
            assert!(!gap.reason.is_empty(), "{}", gap.provider_id);
            assert!(
                gap.next_step.contains("isolated") || gap.next_step.contains("PRAGMA"),
                "{} next step must name the artifact to capture",
                gap.provider_id
            );
        }
    }

    #[test]
    fn registered_rules_are_version_pinned() {
        for provider in PRODUCT_PROVIDERS {
            let Some(rule) = rule_for(provider) else {
                continue;
            };
            assert_eq!(rule.provider_id(), *provider);
            assert!(
                !rule.verified_versions().is_empty(),
                "{provider}: a registered rule without verified versions would open the export \
                 gate on unproven builds"
            );
        }
    }
}
