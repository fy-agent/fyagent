# Research: Current FyAgent OpenCode Auth Seams

- **Query**: Freeze current FyAgent OpenCode auth observation, CLI, Desktop inventory/target, and any `auth.json` reads; note where PATH CLI is required.
- **Scope**: internal
- **Date**: 2026-09-03

## Findings

### Files Found

| File Path | Description |
|---|---|
| `src-tauri/src/agent_install/auth_actions.rs` | OpenCode observe/connect/logout adapter; runs `opencode auth *` via Tooling PATH discovery |
| `src-tauri/src/agent_install/auth_sessions.rs` | Session state machine; OpenCode provider-set verify; Desktop inventory binding is **not** admitted for OpenCode |
| `src-tauri/src/agent_install/cli.rs` | Catalog → Tooling map still includes `OpenCode → "opencode"` |
| `src-tauri/src/agent_install/lifecycle_policy.rs` | Product lifecycle is Desktop-only; CLI surface is `SurfaceNotSupported` |
| `src-tauri/src/agent_install/desktop.rs` | Desktop identity: bundle `ai.opencode.desktop`, Windows `@opencode-aidesktop/OpenCode.exe` |
| `src-tauri/src/agent_install/inventory.rs` | Opaque `i1:` / `c1:` / `d1:` inventory; location labels for install roots, not data dir |
| `src-tauri/src/agent_install/types.rs` | Auth DTO, opaque `p1:` provider IDs, inventory/target validators |
| `src-tauri/src/commands/agent_auth.rs` | Tauri façade for observation/session |
| `src-tauri/src/services/tooling/discovery.rs` | `run_detected_tool_command_*` locates the `opencode` binary then executes it |
| `src-tauri/src/opencode_config.rs` | Config dir `~/.config/opencode` and data dir `~/.local/share/opencode`; **does not read `auth.json`** |
| `src-tauri/src/config.rs` | Frozen user home (`FYAGENT_TEST_HOME` / Windows Shell user) used by data-dir helpers |
| `src-tauri/src/windows_runtime/mod.rs` | Windows Explorer-user `user_home_dir` / `user_local_app_data_dir` |
| `src-tauri/src/commands/managed_auth.rs` | Connection/login mutations currently return `unavailable` |
| `src-tauri/src/services/managed_auth/core.rs` | Enums already include `purpose=opencode_provider`, `consumer=opencode`, `refresh_owner=opencode` |
| `src/v2/pages/agents/AgentAuthStatusPanel.tsx` | OpenCode detail panel routes mutations to `/auth?consumer=opencode` |
| `src/v2/shared/features/managed-auth.ts` | Closed consumer `opencode`; `pendingRestart` / `restart` action enums |
| `src/v2/pages/auth/presentation.ts` | User copy “由 OpenCode 自动续期” |
| `.trellis/spec/backend/external-agent-auth.md` | Agent-auth façade contract: CLI observer, no vendor token-file reads |
| `.trellis/spec/frontend/v2-agent-auth.md` | V2 Agent Auth UI; OpenCode is a managed-consumer summary |
| `.trellis/spec/backend/managed-auth.md` | Managed Auth owner; OpenCode Desktop projection is a later slice |

### Code Patterns

#### 1. Observation and connect still require a PATH `opencode` CLI

`observe_agent_auth` for OpenCode is a blocking `opencode auth list` through Tooling discovery:

```97:105:src-tauri/src/agent_install/auth_actions.rs
        AgentCatalogId::OpenCode => tokio::task::spawn_blocking(observe_opencode_providers)
            .await
            .unwrap_or_else(|_| {
                unavailable_observation(
                    AgentCatalogId::OpenCode,
                    AgentAuthOwnership::ProviderOwned,
                    AgentAuthReasonCode::AuthObserverUnavailable,
                )
            }),
```

```227:236:src-tauri/src/agent_install/auth_actions.rs
fn observe_opencode_providers() -> AgentAuthObservationDto {
    let output = match run_bounded(OPENCODE_TOOL_ID, &["auth", "list"]) {
        Ok(output) => output,
        Err(reason) => {
            return unavailable_observation(
                AgentCatalogId::OpenCode,
                AgentAuthOwnership::ProviderOwned,
                reason,
            )
        }
    };
```

Connect/logout launch the same Tooling binary:

```147:162:src-tauri/src/agent_install/auth_actions.rs
        (AgentCatalogId::OpenCode, AgentAuthIntent::ConnectProvider) => {
            launch_closed_cli(
                OPENCODE_TOOL_ID,
                "opencode auth login",
                "opencode_auth_login",
            )?;
            Ok(AuthLaunchDisposition::AwaitingVerification)
        }
        (AgentCatalogId::OpenCode, AgentAuthIntent::Logout) => {
            launch_closed_cli(
                OPENCODE_TOOL_ID,
                "opencode auth logout",
                "opencode_auth_logout",
            )?;
            Ok(AuthLaunchDisposition::AwaitingVerification)
        }
```

`launch_closed_cli` first probes `--version` via `ensure_tool_available`, then opens a terminal running the literal command string. `run_bounded` / `ensure_tool_available` call `run_detected_tool_command_with_timeout_and_output_limit` (`auth_actions.rs:395-419`), which in `discovery.rs:183` locates the default tool binary (`locate_default_tool`) from PATH / Tooling inventory. If that binary is absent, observation is `AuthObserverUnavailable`.

This is the PATH CLI requirement: **Desktop installed, CLI missing ⇒ OpenCode Agent Auth observation is unavailable**, even though lifecycle policy is Desktop-only.

Legacy readiness still keys OpenCode auth on CLI detection:

```111:131:src-tauri/src/agent_install/auth_actions.rs
pub fn observe_auth_state(
    agent_id: AgentCatalogId,
    cli_detected: bool,
    cli_unavailable: bool,
) -> AgentAuthState {
    if cli_unavailable {
        return AgentAuthState::Unavailable;
    }
    match agent_id {
        AgentCatalogId::ClaudeCode if cli_detected => {
            legacy_state_from_observation(&observe_claude_account())
        }
        AgentCatalogId::OpenCode if cli_detected => AgentAuthState::ProviderConnectionRequired,
        // ... OpenCode with cli_detected=false falls through to Unknown
```

`mod.rs:167-171` feeds `observation.detected` (CLI probe) into that function for readiness DTOs.

#### 2. CLI stdout is parsed; `auth.json` path is stripped, not opened

`parse_opencode_auth_list` expects Clack chrome with a `Credentials <path>` header, provider rows of the form `<label> <api|oauth|wellknown>`, and a trailing `N credentials` count (`auth_actions.rs:303-360`). The header path is ignored. Labels become opaque `p1:` SHA-256 IDs (`auth_actions.rs:386-393`). Tests assert the home path and credential type never leave the DTO (`auth_actions.rs:598-614`).

Production Rust does **not** `fs::read` OpenCode `auth.json`. The only occurrence of that path in this adapter is a **fixture string** in the unit test (`auth_actions.rs:599`).

Grep of `src-tauri` for OpenCode `auth.json` reads: no production reader. Adjacent OpenCode file owners:

- `opencode_config.rs` reads/writes `opencode.json` (models/MCP), not credentials.
- `get_opencode_data_dir()` (`opencode_config.rs:99-118`) resolves `XDG_DATA_HOME/opencode` on macOS, else `{home}/.local/share/opencode`. Windows passes `data_home=None`, so it always uses `{frozen_home}/.local/share/opencode`.
- `session_usage_opencode.rs` reads `opencode.db` under that data dir (usage), not `auth.json`.

#### 3. Lifecycle vs Auth split: Desktop inventory exists; Auth does not bind it

Lifecycle:

```77:83:src-tauri/src/agent_install/lifecycle_policy.rs
const OPENCODE_DESKTOP: AgentLifecyclePolicy = AgentLifecyclePolicy {
    surfaces: &[AgentSurface::Desktop],
    install: true,
    update: true,
    launch: true,
    managed_desktop_source: Some(ManagedDesktopSourceId::OpenCodeDesktop),
};
```

Tests freeze CLI lifecycle as unsupported (`lifecycle_policy.rs:330-348`, `mod.rs:1588-1601`). Catalog still maps OpenCode to Tooling id `"opencode"` for CLI observation (`cli.rs:8-14`).

Desktop inventory identity (`desktop.rs:77-83`):

- macOS bundle id `ai.opencode.desktop`
- Windows relative exes `@opencode-aidesktop/OpenCode.exe`, `OpenCode/OpenCode.exe`

Inventory DTO fields are opaque (`types.rs:507-518`, validators `638-660`): `inventoryId` `i1:`, candidate `c1:`, destination `d1:`, revision `r1:`. `location_label` is a display string such as `~/Applications/OpenCode.app` or `%LOCALAPPDATA%\Programs\OpenCode` (`inventory.rs:1106-1126`). **No data-directory or `auth.json` path is on the inventory DTO.**

Auth session target binding is limited to QoderWork / TRAE Work / WorkBuddy. OpenCode with any inventory triplet is rejected:

```454:469:src-tauri/src/agent_install/auth_sessions.rs
async fn validate_auth_target(
    request: &StartAgentAuthSessionRequest,
    state: &AppState,
) -> Result<AuthLaunchTarget, AgentAuthReasonCode> {
    let has_binding = request.inventory_id.is_some()
        || request.target_id.is_some()
        || request.expected_target_revision.is_some();
    if !matches!(
        request.agent_id,
        AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy
    ) {
        return if has_binding {
            Err(AgentAuthReasonCode::TargetChanged)
        } else {
            Ok(AuthLaunchTarget::None)
        };
    }
```

OpenCode connect/logout therefore always uses `AuthLaunchTarget::None` → `launch_auth_action` → PATH CLI (`auth_sessions.rs:624-641`).

Session verification for OpenCode is before/after provider-id set drift (`auth_sessions.rs:661-673`). Connect requires a new provider id; logout requires the selected `p1:` id to disappear. Connect request must have `provider_id=None`; logout must have a valid opaque id (`auth_sessions.rs:396-398`).

#### 4. V2 UI already treats OpenCode as a managed-auth consumer, while native mutations are unavailable

`AgentAuthStatusPanel.tsx:39-45` maps `opencode → consumer=opencode`. Detail mode does not start Agent Auth sessions; it navigates to `/auth?consumer=opencode&view=connections` (`AgentAuthStatusPanel.tsx:290-305`). Compact/summary still shows the Agent Auth observation (which still comes from CLI `auth list`).

Managed Auth IPC:

- `managed_auth_get_overview` is live (`commands/managed_auth.rs:16-18`).
- `managed_auth_start_login`, session commands, and `managed_auth_apply_connection_action` validate then return `unavailable` (`commands/managed_auth.rs:21-108`).

Domain enums already name the OpenCode consumer/purpose (`services/managed_auth/core.rs:13-74`). There is no `services/managed_auth/consumers/opencode.rs` in this tree.

#### 5. User-context path resolution already exists (install/config), unused by Auth

Windows home used by `get_opencode_data_dir`:

```23:41:src-tauri/src/config.rs
pub fn get_home_dir() -> PathBuf {
    // ...
    #[cfg(target_os = "windows")]
    {
        crate::windows_runtime::user_home_dir()
    }
    #[cfg(target_os = "macos")]
    dirs::home_dir().unwrap_or_else(|| {
```

```414:418:src-tauri/src/windows_runtime/mod.rs
pub(crate) fn user_home_dir() -> PathBuf {
    require_interactive_user_context()
        .user_profile()
        .to_path_buf()
}
```

Desktop install scan uses the same frozen user (`desktop.rs:294` user Applications; `desktop.rs:355` `user_local_app_data_dir().join("Programs")`). That scan answers “which OpenCode.app/exe”, not “where is `auth.json`”. Official OpenCode stores credentials under `Global.Path.data`, which is per-user XDG data, not per-install binary path.

#### 6. Spec dual authority (Agent Auth vs Managed Auth)

`.trellis/spec/backend/external-agent-auth.md:107-109` currently forbids the Agent Auth façade from reading vendor token files. `.trellis/spec/backend/managed-auth.md:24-25` states OpenCode Desktop projection is a later slice of Managed Auth, not of `agent_install` auth. Current OpenCode observe/connect still lives in the Agent Auth façade and uses CLI rather than store projection.

### Related Specs

- `.trellis/spec/backend/external-agent-auth.md` — OpenCode is provider-owned; observe via official provider list; opaque `p1:` IDs.
- `.trellis/spec/frontend/v2-agent-auth.md` — OpenCode panel is a summary + `/auth` deep link.
- `.trellis/spec/frontend/v2-managed-auth.md` — closed consumer `opencode`; pending restart is a connection field.
- `.trellis/spec/backend/managed-auth.md` — `refresh_owner` CHECK includes `opencode`; login/connection commands currently unavailable.
- `.trellis/spec/backend/external-agent-lifecycle.md` — Desktop identity and inventory for OpenCode.

### External References

None for this file. Official store/schema is in `official-opencode-desktop-auth.md`.

## Caveats / Not Found

- No FyAgent production reader/writer of OpenCode `auth.json` exists in this tree.
- No FyAgent code scans Desktop sidecar ports or `OPENCODE_SERVER_PASSWORD`.
- `services/managed_auth/consumers/opencode.rs` is not present; only enums and unavailable IPC exist.
- Whether a user-installed Desktop app sets `XDG_DATA_HOME` for its sidecar is not proven by this repo; current FyAgent Windows data-dir helper ignores `XDG_DATA_HOME`.
- Agent Auth `p1:` IDs hash the **display label**, not the official provider id (`openai`). Official store keys are provider IDs. These namespaces are not interchangeable.
