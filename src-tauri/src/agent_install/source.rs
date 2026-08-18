//! Source / license layer (#25).

use serde::{Deserialize, Serialize};

use super::types::{AgentId, InstallMode, LayerState, LicenseScope, PackageSourceKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceLayerState {
    pub agent_id: AgentId,
    pub source_state: LayerState,
    pub official_landing_url: Option<String>,
    pub legal_entity: Option<String>,
    pub license_url: Option<String>,
    pub license_scope: LicenseScope,
    pub package_source_kind: PackageSourceKind,
    pub cache_allowed: bool,
    pub redistribution_allowed: Option<bool>,
    pub install_mode: InstallMode,
    pub evidence_url: Option<String>,
    pub checked_at: String,
    pub written_permission_needed: bool,
}

impl SourceLayerState {
    pub fn distribution_gate(&self) -> LayerState {
        match self.redistribution_allowed {
            Some(true) => LayerState::Ok,
            Some(false) | None => LayerState::Fail,
        }
    }

    pub fn package_install_blocked(&self) -> bool {
        matches!(self.source_state, LayerState::Fail | LayerState::Unknown)
            || !matches!(self.distribution_gate(), LayerState::Ok)
            || matches!(self.install_mode, InstallMode::OfficialGuide)
    }

    pub fn guide_allowed(&self) -> bool {
        self.official_landing_url
            .as_deref()
            .is_some_and(|url| url.starts_with("https://"))
    }

    pub fn renderer_download_url(&self) -> Option<&str> {
        None
    }
}
