//! Six-agent G1 install domain. Distinct from Codex Desktop MSIX.

pub mod contract;
pub mod error;
pub mod gate;
pub mod integrity;
pub mod plan;
pub mod preflight;
pub mod probe;
pub mod registry;
pub mod source;
pub mod types;

pub use error::{AgentInstallError, AgentInstallErrorCode, AgentInstallErrorDto};
pub use registry::{first_wave_ids, registry};
pub use types::{AgentId, InstallContract};
