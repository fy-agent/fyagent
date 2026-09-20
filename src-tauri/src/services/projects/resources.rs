use super::{domain::*, ProjectsService};
use crate::{
    error::AppError,
    services::managed_auth::{CredentialStatus, ManagedAuthConsumer},
};

impl ProjectsService {
    pub(crate) fn resource_options(&self) -> Result<Vec<ResourceOption>, AppError> {
        let mut out = Vec::new();
        // Read the resource owners only. No filesystem discovery, material, or enable calls.
        for agent in [
            "claude",
            "claude-desktop",
            "codex",
            "gemini",
            "grokbuild",
            "opencode",
            "openclaw",
            "hermes",
        ] {
            for p in self.db.get_all_providers(agent)?.values() {
                out.push(ResourceOption {
                    kind: ResourceKind::Provider,
                    agent_id: agent.into(),
                    raw_id: p.id.clone(),
                    label: p.name.clone(),
                    version: self.db.project_resource_version(
                        ResourceKind::Provider,
                        agent,
                        &p.id,
                    )?,
                });
            }
            for p in self.db.get_prompts(agent)?.values() {
                out.push(ResourceOption {
                    kind: ResourceKind::Prompt,
                    agent_id: agent.into(),
                    raw_id: p.id.clone(),
                    label: p.name.clone(),
                    version: self.db.project_resource_version(
                        ResourceKind::Prompt,
                        agent,
                        &p.id,
                    )?,
                });
            }
        }
        let mcp = self.db.get_all_mcp_servers()?;
        let skills = self.db.get_all_installed_skills()?;
        for agent in [
            "claude",
            "codex",
            "gemini",
            "grokbuild",
            "opencode",
            "hermes",
            "workbuddy",
            "qoderwork",
            "trae-work",
        ] {
            for m in mcp.values() {
                out.push(ResourceOption {
                    kind: ResourceKind::Mcp,
                    agent_id: agent.into(),
                    raw_id: m.id.clone(),
                    label: m.name.clone(),
                    version: self
                        .db
                        .project_resource_version(ResourceKind::Mcp, agent, &m.id)?,
                });
            }
            for s in skills.values() {
                out.push(ResourceOption {
                    kind: ResourceKind::Skill,
                    agent_id: agent.into(),
                    raw_id: s.id.clone(),
                    label: s.name.clone(),
                    version: self
                        .db
                        .project_resource_version(ResourceKind::Skill, agent, &s.id)?,
                });
            }
        }
        // Generations cover DB mutations only; skill files remain independently unverifiable.
        out.sort_by(|a, b| {
            a.agent_id
                .cmp(&b.agent_id)
                .then(a.label.cmp(&b.label))
                .then(a.raw_id.cmp(&b.raw_id))
        });
        Ok(out)
    }
    pub(crate) fn credential_options(&self) -> Result<Vec<CredentialOption>, AppError> {
        self.db.managed_auth_list_all_credentials().map(|rows| {
            rows.into_iter()
                .map(|r| {
                    let c = r.credential;
                    let consumer = match c.consumer {
                        Some(ManagedAuthConsumer::Codex) => "codex",
                        Some(ManagedAuthConsumer::Grokbuild) => "grokbuild",
                        Some(ManagedAuthConsumer::Opencode) => "opencode",
                        Some(ManagedAuthConsumer::FyagentProxy) => "fyagent_proxy",
                        None => "unavailable",
                    };
                    CredentialOption {
                        credential_id: c.credential_id,
                        label: r.identity.login,
                        purpose: c.purpose.as_str().into(),
                        consumer: consumer.into(),
                        generation: c.generation,
                        state: match c.status {
                            CredentialStatus::Ready if c.consumer.is_some() => {
                                ObservationState::Unverifiable
                            }
                            CredentialStatus::Revoked => ObservationState::Revoked,
                            _ => ObservationState::Unavailable,
                        },
                    }
                })
                .collect()
        })
    }
}
