//! Crate-private macOS system-commit port.
//!
//! Renderer IPC stays on existing Agent/Codex closed actions. This module owns
//! the helper-facing product/slot policy, C ABI, and production-disabled
//! adapter. Production `/Applications` actions stay `authorization_required`
//! until formal signed/notarized HIL.

#![cfg_attr(not(test), allow(dead_code))]

mod ffi;
mod policy;
mod port;
mod types;

#[cfg(test)]
mod fake;

pub(crate) use port::system_scope_rejection;

#[cfg(test)]
pub(crate) use policy::{resolve_slot, KnownSystemProduct};
#[cfg(test)]
pub(crate) use port::production_enabled;
