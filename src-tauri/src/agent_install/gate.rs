//! Combined install-allowed gate. Four layers stay independent.

use super::integrity::IntegrityLayerState;
use super::plan::PlanLayerState;
use super::preflight::PreflightLayerState;
use super::source::SourceLayerState;
use super::types::LayerState;

pub fn package_install_allowed(
    source: &SourceLayerState,
    integrity: &IntegrityLayerState,
    preflight: &PreflightLayerState,
    plan: &PlanLayerState,
) -> bool {
    if source.package_install_blocked() {
        return false;
    }
    match integrity.integrity_state {
        LayerState::Fail | LayerState::Unknown => return false,
        LayerState::Ok | LayerState::Warn => {}
    }
    match preflight.preflight_state {
        LayerState::Fail | LayerState::Unknown => return false,
        LayerState::Ok | LayerState::Warn => {}
    }
    !plan.snapshot_stale
}
