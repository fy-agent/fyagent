//! Process-local agent-install service.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::agent_install::contract::{catalog_entries, default_contract, official_landing_url};
use crate::agent_install::error::AgentInstallError;
use crate::agent_install::plan::{
    create_snapshot, reconfirm_snapshot, start_install, PlanLayerState, PlanSnapshot,
    StartAgentInstallRequest,
};
use crate::agent_install::preflight::{observe_preflight, MachineFacts, PreflightLayerState};
use crate::agent_install::probe::{probe_health, HealthProbeResult};
use crate::agent_install::types::{AgentId, InstallContract};

pub struct AgentInstallService {
    now: String,
    snapshots: Mutex<HashMap<String, PlanSnapshot>>,
    facts: MachineFacts,
}

impl Default for AgentInstallService {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentInstallService {
    pub fn new() -> Self {
        Self {
            now: current_timestamp(),
            snapshots: Mutex::new(HashMap::new()),
            facts: MachineFacts::host(),
        }
    }

    pub fn list_catalog(&self) -> Vec<crate::agent_install::source::SourceLayerState> {
        catalog_entries()
    }

    pub fn get_contract(&self, agent_id: &str) -> Result<InstallContract, AgentInstallError> {
        let id = AgentId::parse(agent_id)?;
        default_contract(id, &self.now)
    }

    pub fn refresh_preflight(
        &self,
        agent_id: &str,
    ) -> Result<PreflightLayerState, AgentInstallError> {
        let id = AgentId::parse(agent_id)?;
        Ok(observe_preflight(id, &self.facts, &self.now))
    }

    pub fn create_plan(&self, agent_id: &str) -> Result<PlanLayerState, AgentInstallError> {
        let id = AgentId::parse(agent_id)?;
        let contract = default_contract(id, &self.now)?;
        let snapshot = create_snapshot(&contract, &self.now);
        let layer = snapshot.as_layer();
        self.store(snapshot);
        Ok(layer)
    }

    pub fn reconfirm_plan(&self, snapshot_id: &str) -> Result<PlanLayerState, AgentInstallError> {
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let current = snapshots
            .remove(snapshot_id)
            .ok_or_else(AgentInstallError::snapshot_stale)?;
        let next = reconfirm_snapshot(&current, &self.now);
        let layer = next.as_layer();
        snapshots.insert(next.snapshot_id.clone(), next);
        Ok(layer)
    }

    pub fn start_install(
        &self,
        request: StartAgentInstallRequest,
    ) -> Result<PlanLayerState, AgentInstallError> {
        let snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let snapshot = snapshots
            .get(&request.snapshot_id)
            .ok_or_else(AgentInstallError::snapshot_stale)?;
        start_install(snapshot, &request)
    }

    pub fn official_guide_url(&self, agent_id: &str) -> Result<String, AgentInstallError> {
        official_landing_url(AgentId::parse(agent_id)?)
    }

    pub fn probe(&self, agent_id: &str) -> Result<HealthProbeResult, AgentInstallError> {
        let id = AgentId::parse(agent_id)?;
        Ok(probe_health(id, &self.now))
    }

    fn store(&self, snapshot: PlanSnapshot) {
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        snapshots.insert(snapshot.snapshot_id.clone(), snapshot);
    }
}

fn current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}
