//! Import observations contain only stable source identities and counts.

use serde::Serialize;

use crate::app_config::{McpServer, McpTargetId};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerView {
    #[serde(flatten)]
    pub server: McpServer,
    pub sources: Vec<McpTargetId>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpImportCounts {
    pub added: usize,
    pub assignment_changed: usize,
    pub unchanged: usize,
    pub disabled_skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportSourceResult {
    pub source: McpTargetId,
    #[serde(flatten)]
    pub counts: McpImportCounts,
    // Closed code only; native paths, executable values and secrets stay native.
    pub failure_code: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportReport {
    pub contract_version: u8,
    pub sources: Vec<McpImportSourceResult>,
}
