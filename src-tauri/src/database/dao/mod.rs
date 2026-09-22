//! Data Access Object layer
//!
//! Database access operations for each domain

pub(crate) mod change_plan;
mod config_pack;
pub mod failover;
pub(crate) mod health;
pub(crate) mod managed_auth;
pub mod mcp;
pub mod profiles;
pub mod prompts;
pub mod providers;
pub mod providers_seed;
pub mod proxy;
pub(crate) mod session_restore;
pub mod settings;
pub mod skills;
pub mod stream_check;
pub mod universal_providers;
pub mod usage_rollup;

// 所有 DAO 方法都通过 Database impl 提供，无需单独导出
// 导出 FailoverQueueItem / Profile 供外部使用
pub use failover::FailoverQueueItem;
pub use profiles::Profile;

pub(super) mod provider_credentials;
