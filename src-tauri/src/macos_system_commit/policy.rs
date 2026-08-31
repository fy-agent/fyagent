//! Frozen known-application product/slot table for the privileged helper ABI.
//!
//! Integers are the C ABI / XPC wire values from
//! `research/implementation-seam.md`. Display names and path strings never
//! travel. Bundle IDs must stay in lockstep with:
//! - Codex: [`crate::codex_desktop::platform::MACOS_CODEX_STABLE_IDENTITY`]
//! - other desktop products: `agent_install/desktop.rs` `DESKTOP_PRODUCTS`
//!   (`macos_bundle_id_for`)

use crate::agent_install::AgentReasonCode;

/// Protocol product integers. Unknown values are rejected before mutation.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownSystemProduct {
    /// Codex / new ChatGPT desktop. Stable identity is `com.openai.codex`.
    CodexDesktop = 1,
    /// OpenCode Desktop. Owner: `agent_install/desktop.rs`.
    OpenCodeDesktop = 2,
    /// QoderWork CN. Owner: `agent_install/desktop.rs`.
    QoderWork = 3,
    /// TRAE Work CN. Owner: `agent_install/desktop.rs`.
    TraeWork = 4,
    /// WorkBuddy. Owner: `agent_install/desktop.rs`.
    WorkBuddy = 5,
}

/// One product-local target slot. The helper maps this to a fixed
/// `/Applications` basename; callers never send that basename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductSlotPolicy {
    pub product: KnownSystemProduct,
    pub slot: u32,
    pub bundle_id: &'static str,
    pub basename: &'static str,
    pub existing_only: bool,
}

const CODEX_BUNDLE_ID: &str = "com.openai.codex";
const OPENCODE_BUNDLE_ID: &str = "ai.opencode.desktop";
const QODERWORK_BUNDLE_ID: &str = "com.qoder.work.cn";
const TRAEWORK_BUNDLE_ID: &str = "cn.trae.solo.app";
const WORKBUDDY_BUNDLE_ID: &str = "com.workbuddy.workbuddy";

impl KnownSystemProduct {
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn from_u32(value: u32) -> Result<Self, AgentReasonCode> {
        match value {
            1 => Ok(Self::CodexDesktop),
            2 => Ok(Self::OpenCodeDesktop),
            3 => Ok(Self::QoderWork),
            4 => Ok(Self::TraeWork),
            5 => Ok(Self::WorkBuddy),
            _ => Err(AgentReasonCode::TargetSlotInvalid),
        }
    }
}

/// Resolve a product-local slot. Unknown product/slot combinations fail closed.
pub fn resolve_slot(
    product: KnownSystemProduct,
    slot: u32,
) -> Result<ProductSlotPolicy, AgentReasonCode> {
    let policy = match (product, slot) {
        (KnownSystemProduct::CodexDesktop, 1) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: CODEX_BUNDLE_ID,
            basename: "ChatGPT.app",
            existing_only: false,
        },
        (KnownSystemProduct::CodexDesktop, 2) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: CODEX_BUNDLE_ID,
            basename: "Codex.app",
            existing_only: true,
        },
        (KnownSystemProduct::OpenCodeDesktop, 1) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: OPENCODE_BUNDLE_ID,
            basename: "OpenCode.app",
            existing_only: false,
        },
        (KnownSystemProduct::QoderWork, 1) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: QODERWORK_BUNDLE_ID,
            basename: "QoderWork CN.app",
            existing_only: false,
        },
        (KnownSystemProduct::TraeWork, 1) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: TRAEWORK_BUNDLE_ID,
            basename: "TRAE SOLO CN.app",
            existing_only: false,
        },
        (KnownSystemProduct::WorkBuddy, 1) => ProductSlotPolicy {
            product,
            slot,
            bundle_id: WORKBUDDY_BUNDLE_ID,
            basename: "WorkBuddy.app",
            existing_only: false,
        },
        _ => return Err(AgentReasonCode::TargetSlotInvalid),
    };
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_desktop::platform::MACOS_CODEX_STABLE_IDENTITY;

    #[test]
    fn frozen_product_integers_match_seam() {
        assert_eq!(KnownSystemProduct::CodexDesktop.as_u32(), 1);
        assert_eq!(KnownSystemProduct::OpenCodeDesktop.as_u32(), 2);
        assert_eq!(KnownSystemProduct::QoderWork.as_u32(), 3);
        assert_eq!(KnownSystemProduct::TraeWork.as_u32(), 4);
        assert_eq!(KnownSystemProduct::WorkBuddy.as_u32(), 5);
        assert_eq!(
            KnownSystemProduct::from_u32(1).expect("codex"),
            KnownSystemProduct::CodexDesktop
        );
        assert!(KnownSystemProduct::from_u32(0).is_err());
        assert!(KnownSystemProduct::from_u32(6).is_err());
    }

    #[test]
    fn unknown_product_or_slot_is_rejected() {
        assert_eq!(
            KnownSystemProduct::from_u32(99),
            Err(AgentReasonCode::TargetSlotInvalid)
        );
        assert_eq!(
            resolve_slot(KnownSystemProduct::CodexDesktop, 0).map(|policy| policy.basename),
            Err(AgentReasonCode::TargetSlotInvalid)
        );
        assert_eq!(
            resolve_slot(KnownSystemProduct::CodexDesktop, 3).map(|policy| policy.basename),
            Err(AgentReasonCode::TargetSlotInvalid)
        );
        assert_eq!(
            resolve_slot(KnownSystemProduct::QoderWork, 2).map(|policy| policy.basename),
            Err(AgentReasonCode::TargetSlotInvalid)
        );
        assert_eq!(
            resolve_slot(KnownSystemProduct::OpenCodeDesktop, 2).map(|policy| policy.basename),
            Err(AgentReasonCode::TargetSlotInvalid)
        );
    }

    #[test]
    fn slot_table_matches_seam_basenames_and_bundle_ids() {
        let chatgpt = resolve_slot(KnownSystemProduct::CodexDesktop, 1).expect("chatgpt slot");
        assert_eq!(chatgpt.bundle_id, CODEX_BUNDLE_ID);
        assert_eq!(chatgpt.basename, "ChatGPT.app");
        assert!(!chatgpt.existing_only);

        let codex = resolve_slot(KnownSystemProduct::CodexDesktop, 2).expect("codex slot");
        assert_eq!(codex.bundle_id, CODEX_BUNDLE_ID);
        assert_eq!(codex.basename, "Codex.app");
        assert!(codex.existing_only);

        let opencode = resolve_slot(KnownSystemProduct::OpenCodeDesktop, 1).expect("opencode");
        assert_eq!(opencode.bundle_id, OPENCODE_BUNDLE_ID);
        assert_eq!(opencode.basename, "OpenCode.app");
        assert!(!opencode.existing_only);

        let qoder = resolve_slot(KnownSystemProduct::QoderWork, 1).expect("qoder");
        assert_eq!(qoder.bundle_id, QODERWORK_BUNDLE_ID);
        assert_eq!(qoder.basename, "QoderWork CN.app");

        let trae = resolve_slot(KnownSystemProduct::TraeWork, 1).expect("trae");
        assert_eq!(trae.bundle_id, TRAEWORK_BUNDLE_ID);
        assert_eq!(trae.basename, "TRAE SOLO CN.app");

        let workbuddy = resolve_slot(KnownSystemProduct::WorkBuddy, 1).expect("workbuddy");
        assert_eq!(workbuddy.bundle_id, WORKBUDDY_BUNDLE_ID);
        assert_eq!(workbuddy.basename, "WorkBuddy.app");
    }

    #[test]
    fn bundle_ids_match_codex_stable_identity_and_desktop_owner() {
        assert_eq!(CODEX_BUNDLE_ID, MACOS_CODEX_STABLE_IDENTITY);
        assert_eq!(
            resolve_slot(KnownSystemProduct::CodexDesktop, 1)
                .expect("chatgpt")
                .bundle_id,
            MACOS_CODEX_STABLE_IDENTITY
        );
        // Must stay in lockstep with agent_install/desktop.rs DESKTOP_PRODUCTS.
        assert_eq!(OPENCODE_BUNDLE_ID, "ai.opencode.desktop");
        assert_eq!(QODERWORK_BUNDLE_ID, "com.qoder.work.cn");
        assert_eq!(TRAEWORK_BUNDLE_ID, "cn.trae.solo.app");
        assert_eq!(WORKBUDDY_BUNDLE_ID, "com.workbuddy.workbuddy");
    }

    #[test]
    fn policy_table_does_not_carry_path_strings_on_the_product_enum() {
        for product in [
            KnownSystemProduct::CodexDesktop,
            KnownSystemProduct::OpenCodeDesktop,
            KnownSystemProduct::QoderWork,
            KnownSystemProduct::TraeWork,
            KnownSystemProduct::WorkBuddy,
        ] {
            let debug = format!("{product:?}");
            assert!(!debug.contains('/'), "product debug leaked a path: {debug}");
            assert!(!debug.contains("Applications"));
        }
    }
}
