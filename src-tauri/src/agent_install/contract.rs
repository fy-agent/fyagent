//! Assemble a four-layer contract without collapsing states.

use super::gate::package_install_allowed;
use super::integrity::IntegrityLayerState;
use super::plan::PlanLayerState;
use super::preflight::PreflightLayerState;
use super::registry::source_layer_for;
use super::source::SourceLayerState;
use super::types::{AgentId, InstallContract, InstallMode};

pub fn catalog_entries() -> Vec<SourceLayerState> {
    super::registry::registry().collect()
}

pub fn contract_for(
    agent_id: AgentId,
    package: IntegrityLayerState,
    environment: PreflightLayerState,
    plan: PlanLayerState,
    updated_at: String,
) -> Result<InstallContract, super::error::AgentInstallError> {
    let catalog = source_layer_for(agent_id)?;
    let guide_allowed = catalog.guide_allowed();
    let install_allowed = package_install_allowed(&catalog, &package, &environment, &plan);
    Ok(InstallContract::new(
        agent_id,
        catalog,
        package,
        environment,
        plan,
        updated_at,
        install_allowed,
        guide_allowed,
    ))
}

pub fn default_contract(
    agent_id: AgentId,
    now: &str,
) -> Result<InstallContract, super::error::AgentInstallError> {
    let catalog = source_layer_for(agent_id)?;
    let package = match catalog.install_mode {
        InstallMode::PackageManager => IntegrityLayerState::package_manager_warn(now),
        InstallMode::OfficialGuide | InstallMode::NativeVerified => {
            IntegrityLayerState::unknown(now)
        }
    };
    contract_for(
        agent_id,
        package,
        PreflightLayerState::unknown(now),
        PlanLayerState::absent(now),
        now.to_owned(),
    )
}

pub fn official_landing_url(agent_id: AgentId) -> Result<String, super::error::AgentInstallError> {
    let source = source_layer_for(agent_id)?;
    source
        .official_landing_url
        .filter(|url| url.starts_with("https://"))
        .ok_or_else(super::error::AgentInstallError::guide_unavailable)
}
