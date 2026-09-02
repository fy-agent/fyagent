//! Crate-private Agent lifecycle product/surface/action policy.
//!
//! This owner answers legal surfaces, default surface, action admission, and
//! closed source kind. Runtime evidence still lives in inventory/readiness.

use super::types::{AgentActionId, AgentInstallState, AgentReasonCode, AgentSurface};
use crate::services::external_agents::AgentCatalogId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManagedDesktopSourceId {
    QoderWork,
    TraeWork,
    WorkBuddy,
    ClaudeDesktop,
    OpenCodeDesktop,
    CodexDesktopDedicated,
    GrokCliTooling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentLifecyclePolicy {
    pub surfaces: &'static [AgentSurface],
    pub install: bool,
    pub update: bool,
    pub launch: bool,
    pub managed_desktop_source: Option<ManagedDesktopSourceId>,
}

const QODERWORK: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: false,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::QoderWork),
};

const TRAEWORK: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: false,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::TraeWork),
};

const WORKBUDDY: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: false,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::WorkBuddy),
};

const GROK_CLI: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Cli],
    install: true,
    update: true,
    launch: false,
    managed_desktop_source: Some(ManagedDesktopSourceId::GrokCliTooling),
};

const CODEX_DESKTOP: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: true,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::CodexDesktopDedicated),
};

const CLAUDE_DESKTOP: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: true,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::ClaudeDesktop),
};

const OPENCODE_DESKTOP: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: true,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::OpenCodeDesktop),
};

fn policy_for_product(agent_id: AgentCatalogId) -> &'static AgentLifecyclePolicy {
    match agent_id {
        AgentCatalogId::QoderWork => &QODERWORK,
        AgentCatalogId::TraeWork => &TRAEWORK,
        AgentCatalogId::WorkBuddy => &WORKBUDDY,
        AgentCatalogId::GrokBuild => &GROK_CLI,
        AgentCatalogId::Codex => &CODEX_DESKTOP,
        AgentCatalogId::ClaudeCode => &CLAUDE_DESKTOP,
        AgentCatalogId::OpenCode => &OPENCODE_DESKTOP,
    }
}

pub(crate) fn legal_surfaces(agent_id: AgentCatalogId) -> &'static [AgentSurface] {
    policy_for_product(agent_id).surfaces
}

pub(crate) fn default_surface(agent_id: AgentCatalogId) -> AgentSurface {
    legal_surfaces(agent_id)[0]
}

pub(crate) fn lifecycle_policy(
    agent_id: AgentCatalogId,
    surface: AgentSurface,
) -> Result<&'static AgentLifecyclePolicy, AgentReasonCode> {
    let policy = policy_for_product(agent_id);
    if policy.surfaces.contains(&surface) {
        Ok(policy)
    } else {
        Err(AgentReasonCode::SurfaceNotSupported)
    }
}

/// Returns whether the closed lifecycle action is admitted by product policy.
/// An illegal surface yields `SurfaceNotSupported`; a legal surface with a
/// disabled action yields `Ok(false)`.
pub(crate) fn action_supported(
    agent_id: AgentCatalogId,
    surface: AgentSurface,
    action: AgentActionId,
) -> Result<bool, AgentReasonCode> {
    let policy = lifecycle_policy(agent_id, surface)?;
    Ok(match action {
        AgentActionId::Install => policy.install,
        AgentActionId::Update => policy.update,
        AgentActionId::Launch => policy.launch,
        AgentActionId::AuthLogin
        | AgentActionId::AuthLogout
        | AgentActionId::AuthConnectProvider => false,
    })
}

/// Admits a closed lifecycle action on a legal surface.
/// Disabled product actions fail closed with `ActionNotSupported`.
pub(crate) fn admit_action(
    agent_id: AgentCatalogId,
    surface: AgentSurface,
    action: AgentActionId,
) -> Result<(), AgentReasonCode> {
    if action_supported(agent_id, surface, action)? {
        Ok(())
    } else {
        Err(AgentReasonCode::ActionNotSupported)
    }
}

/// Whether desktop readiness should resolve remote source metadata.
pub(crate) fn should_resolve_desktop_source(
    policy: &AgentLifecyclePolicy,
    install_state: AgentInstallState,
) -> bool {
    match install_state {
        AgentInstallState::NotInstalled => policy.install,
        AgentInstallState::Installed | AgentInstallState::InstalledNotRunnable => policy.update,
        AgentInstallState::Unknown | AgentInstallState::Unavailable => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_AGENTS: [AgentCatalogId; 7] = [
        AgentCatalogId::QoderWork,
        AgentCatalogId::TraeWork,
        AgentCatalogId::WorkBuddy,
        AgentCatalogId::GrokBuild,
        AgentCatalogId::Codex,
        AgentCatalogId::ClaudeCode,
        AgentCatalogId::OpenCode,
    ];

    const ALL_SURFACES: [AgentSurface; 2] = [AgentSurface::Cli, AgentSurface::Desktop];

    const LIFECYCLE_ACTIONS: [AgentActionId; 3] = [
        AgentActionId::Install,
        AgentActionId::Update,
        AgentActionId::Launch,
    ];

    fn expected_policy(
        agent_id: AgentCatalogId,
    ) -> (
        &'static [AgentSurface],
        bool,
        bool,
        bool,
        ManagedDesktopSourceId,
    ) {
        match agent_id {
            AgentCatalogId::QoderWork => (
                &[AgentSurface::Desktop],
                true,
                false,
                true,
                ManagedDesktopSourceId::QoderWork,
            ),
            AgentCatalogId::TraeWork => (
                &[AgentSurface::Desktop],
                true,
                false,
                true,
                ManagedDesktopSourceId::TraeWork,
            ),
            AgentCatalogId::WorkBuddy => (
                &[AgentSurface::Desktop],
                true,
                false,
                true,
                ManagedDesktopSourceId::WorkBuddy,
            ),
            AgentCatalogId::GrokBuild => (
                &[AgentSurface::Cli],
                true,
                true,
                false,
                ManagedDesktopSourceId::GrokCliTooling,
            ),
            AgentCatalogId::Codex => (
                &[AgentSurface::Desktop],
                true,
                true,
                true,
                ManagedDesktopSourceId::CodexDesktopDedicated,
            ),
            AgentCatalogId::ClaudeCode => (
                &[AgentSurface::Desktop],
                true,
                true,
                true,
                ManagedDesktopSourceId::ClaudeDesktop,
            ),
            AgentCatalogId::OpenCode => (
                &[AgentSurface::Desktop],
                true,
                true,
                true,
                ManagedDesktopSourceId::OpenCodeDesktop,
            ),
        }
    }

    #[test]
    fn product_surface_action_matrix_is_exhaustive_and_closed() {
        for agent_id in ALL_AGENTS {
            let (surfaces, install, update, launch, source) = expected_policy(agent_id);
            assert_eq!(legal_surfaces(agent_id), surfaces);
            assert_eq!(default_surface(agent_id), surfaces[0]);

            for surface in ALL_SURFACES {
                if surfaces.contains(&surface) {
                    let policy = lifecycle_policy(agent_id, surface).expect("legal surface");
                    assert_eq!(policy.install, install);
                    assert_eq!(policy.update, update);
                    assert_eq!(policy.launch, launch);
                    assert_eq!(policy.managed_desktop_source, Some(source));
                    assert_eq!(
                        action_supported(agent_id, surface, AgentActionId::Install).unwrap(),
                        install
                    );
                    assert_eq!(
                        action_supported(agent_id, surface, AgentActionId::Update).unwrap(),
                        update
                    );
                    assert_eq!(
                        action_supported(agent_id, surface, AgentActionId::Launch).unwrap(),
                        launch
                    );
                    for auth in [
                        AgentActionId::AuthLogin,
                        AgentActionId::AuthLogout,
                        AgentActionId::AuthConnectProvider,
                    ] {
                        assert!(!action_supported(agent_id, surface, auth).unwrap());
                    }
                } else {
                    assert_eq!(
                        lifecycle_policy(agent_id, surface),
                        Err(AgentReasonCode::SurfaceNotSupported)
                    );
                    for action in LIFECYCLE_ACTIONS {
                        assert_eq!(
                            action_supported(agent_id, surface, action),
                            Err(AgentReasonCode::SurfaceNotSupported)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn managed_desktop_products_admit_install_update_and_launch_on_desktop_only() {
        for agent_id in [
            AgentCatalogId::QoderWork,
            AgentCatalogId::TraeWork,
            AgentCatalogId::WorkBuddy,
        ] {
            assert_eq!(
                action_supported(agent_id, AgentSurface::Desktop, AgentActionId::Update),
                Ok(false)
            );
            assert_eq!(
                admit_action(agent_id, AgentSurface::Desktop, AgentActionId::Update),
                Err(AgentReasonCode::ActionNotSupported)
            );
            assert_eq!(
                action_supported(agent_id, AgentSurface::Desktop, AgentActionId::Install),
                Ok(true)
            );
            assert_eq!(
                admit_action(agent_id, AgentSurface::Desktop, AgentActionId::Install),
                Ok(())
            );
            assert_eq!(
                action_supported(agent_id, AgentSurface::Cli, AgentActionId::Update),
                Err(AgentReasonCode::SurfaceNotSupported)
            );
            assert_eq!(
                admit_action(agent_id, AgentSurface::Cli, AgentActionId::Update),
                Err(AgentReasonCode::SurfaceNotSupported)
            );
        }
    }

    #[test]
    fn claude_and_opencode_reject_cli_and_admit_desktop_lifecycle_actions() {
        for agent_id in [AgentCatalogId::ClaudeCode, AgentCatalogId::OpenCode] {
            assert_eq!(legal_surfaces(agent_id), &[AgentSurface::Desktop]);
            assert_eq!(default_surface(agent_id), AgentSurface::Desktop);
            assert_eq!(
                lifecycle_policy(agent_id, AgentSurface::Cli),
                Err(AgentReasonCode::SurfaceNotSupported)
            );
            for action in LIFECYCLE_ACTIONS {
                assert_eq!(
                    action_supported(agent_id, AgentSurface::Desktop, action),
                    Ok(true)
                );
                assert_eq!(
                    action_supported(agent_id, AgentSurface::Cli, action),
                    Err(AgentReasonCode::SurfaceNotSupported)
                );
            }
        }
    }

    #[test]
    fn grok_preserves_cli_install_update_without_launch_and_codex_stays_dedicated() {
        assert_eq!(
            action_supported(
                AgentCatalogId::GrokBuild,
                AgentSurface::Cli,
                AgentActionId::Launch
            ),
            Ok(false)
        );
        assert_eq!(
            lifecycle_policy(AgentCatalogId::GrokBuild, AgentSurface::Desktop),
            Err(AgentReasonCode::SurfaceNotSupported)
        );
        let codex = lifecycle_policy(AgentCatalogId::Codex, AgentSurface::Desktop).unwrap();
        assert_eq!(
            codex.managed_desktop_source,
            Some(ManagedDesktopSourceId::CodexDesktopDedicated)
        );
        assert!(codex.install && codex.update && codex.launch);
        assert_eq!(
            lifecycle_policy(AgentCatalogId::Codex, AgentSurface::Cli),
            Err(AgentReasonCode::SurfaceNotSupported)
        );
        assert_eq!(
            admit_action(
                AgentCatalogId::GrokBuild,
                AgentSurface::Cli,
                AgentActionId::Launch
            ),
            Err(AgentReasonCode::ActionNotSupported)
        );
    }

    #[test]
    fn desktop_source_resolve_follows_install_state_and_update_policy() {
        let domestic = lifecycle_policy(AgentCatalogId::QoderWork, AgentSurface::Desktop).unwrap();
        assert!(should_resolve_desktop_source(
            domestic,
            AgentInstallState::NotInstalled
        ));
        assert!(!should_resolve_desktop_source(
            domestic,
            AgentInstallState::Installed
        ));
        assert!(!should_resolve_desktop_source(
            domestic,
            AgentInstallState::InstalledNotRunnable
        ));
        assert!(!should_resolve_desktop_source(
            domestic,
            AgentInstallState::Unknown
        ));

        let opencode = lifecycle_policy(AgentCatalogId::OpenCode, AgentSurface::Desktop).unwrap();
        assert!(should_resolve_desktop_source(
            opencode,
            AgentInstallState::NotInstalled
        ));
        assert!(should_resolve_desktop_source(
            opencode,
            AgentInstallState::Installed
        ));
        assert!(!should_resolve_desktop_source(
            opencode,
            AgentInstallState::Unknown
        ));
    }
}
