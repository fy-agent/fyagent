//! Crate-private macOS system-commit port.
//!
//! Renderer IPC stays on existing Agent/Codex closed actions. This module owns
//! the helper-facing product/slot policy, C ABI, and signed runtime adapter.
//! Unsigned/ordinary builds stay disabled. Signed development enables the real
//! path; a Developer ID build enables it only when an explicit formal HIL
//! candidate selects the reviewed compile-time mode. Standard Release bundles
//! may carry the signed helper/client while keeping root transactions off.

#![cfg_attr(not(test), allow(dead_code))]

mod ffi;
mod policy;
mod port;
mod types;

#[cfg(test)]
mod fake;

pub(crate) use policy::{product_for_agent, resolve_slot};
pub(crate) use port::{
    production_enabled, production_port, system_scope_rejection, MacSystemCommitPort,
};
pub(crate) use types::{
    AuthorizedSystemCommit, SystemCommitAction, SystemCommitOutcome, UserIntent,
};

#[cfg(test)]
pub(crate) use policy::KnownSystemProduct;
