//! Stable, privacy-safe agent-install errors. Renderer branches on `code`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentInstallErrorCode {
    AgentUnknown,
    SourceBlocked,
    IntegrityFailed,
    IntegrityUnknown,
    PreflightBlocked,
    SnapshotStale,
    SnapshotNotFound,
    GuideUnavailable,
    JobAlreadyRunning,
    JobNotFound,
    InternalError,
}

#[derive(Debug, Clone, Error)]
#[error("{code:?}")]
pub struct AgentInstallError {
    pub code: AgentInstallErrorCode,
}

impl AgentInstallError {
    pub const fn new(code: AgentInstallErrorCode) -> Self {
        Self { code }
    }

    pub const fn unknown_agent() -> Self {
        Self::new(AgentInstallErrorCode::AgentUnknown)
    }

    pub const fn source_blocked() -> Self {
        Self::new(AgentInstallErrorCode::SourceBlocked)
    }

    pub const fn snapshot_stale() -> Self {
        Self::new(AgentInstallErrorCode::SnapshotStale)
    }

    pub const fn guide_unavailable() -> Self {
        Self::new(AgentInstallErrorCode::GuideUnavailable)
    }

    pub fn to_dto(&self) -> AgentInstallErrorDto {
        AgentInstallErrorDto { code: self.code }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstallErrorDto {
    pub code: AgentInstallErrorCode,
}

impl From<AgentInstallError> for AgentInstallErrorDto {
    fn from(error: AgentInstallError) -> Self {
        error.to_dto()
    }
}
