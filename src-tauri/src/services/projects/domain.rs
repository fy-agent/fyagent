use serde::{Deserialize, Serialize};

pub const MAX_CONTEXT_BYTES: usize = 128 * 1024;
pub const MAX_REVISION: i64 = 9_007_199_254_740_990;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Customer {
    pub customer_id: String,
    pub name: String,
    pub revision: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Provider,
    Mcp,
    Skill,
    Prompt,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceBinding {
    pub kind: ResourceKind,
    pub agent_id: String,
    pub raw_id: String,
    pub model: Option<String>,
    pub pinned_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialBinding {
    pub credential_id: String,
    pub purpose: String,
    pub consumer: String,
    pub pinned_generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KitBinding {
    pub kit_id: String,
    pub kit_version: String,
    pub manifest_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Project {
    pub project_id: String,
    pub customer_id: String,
    pub name: String,
    pub project_revision: i64,
    pub archived: bool,
    pub resources: Vec<ResourceBinding>,
    pub credentials: Vec<CredentialBinding>,
    pub kit: Option<KitBinding>,
    pub context_generation: Option<String>,
    pub codex_prepared: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectMutation {
    pub project_id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindKitRequest {
    pub project_id: String,
    pub expected_revision: i64,
    pub kit_id: String,
    pub kit_version: String,
    pub manifest_digest: String,
    pub binding_intent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectContext {
    pub project_id: String,
    pub project_revision: i64,
    pub content: String,
    pub directory: Option<String>,
    pub state: ContextState,
    pub codex_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContextState {
    NotCreated,
    Materialized,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationState {
    Unverifiable,
    Matched,
    Drifted,
    Missing,
    Unavailable,
    Revoked,
    PurposeMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceOption {
    pub kind: ResourceKind,
    pub agent_id: String,
    pub raw_id: String,
    pub label: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialOption {
    pub credential_id: String,
    pub label: String,
    pub purpose: String,
    pub consumer: String,
    pub generation: u64,
    pub state: ObservationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDependency {
    pub resource: ResourceBinding,
    pub observed_version: Option<String>,
    pub state: ObservationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialDependency {
    pub binding: CredentialBinding,
    pub observed_generation: Option<u64>,
    pub state: ObservationState,
}

/// Local native contract. Device references must be allowlisted away by exporters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDependencySnapshot {
    pub project_id: String,
    pub project_revision: i64,
    pub archived: bool,
    pub kit: Option<KitBinding>,
    pub kit_state: ObservationState,
    pub resources: Vec<ResourceDependency>,
    pub credentials: Vec<CredentialDependency>,
    pub context_generation: Option<String>,
    pub context_state: ContextState,
    pub observed_at: String,
    pub runtime_available: bool,
}
