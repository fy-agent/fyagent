use indexmap::IndexMap;
use std::collections::HashMap;

use crate::app_config::{AppType, McpServer, McpTargetId};
use crate::error::AppError;
use crate::mcp;
use crate::store::AppState;

/// MCP 相关业务逻辑（v3.7.0 统一结构）
pub struct McpService;

impl McpService {
    /// 获取所有 MCP 服务器（统一结构）
    pub fn get_all_servers(state: &AppState) -> Result<IndexMap<String, McpServer>, AppError> {
        state.db.get_all_mcp_servers()
    }

    /// 添加或更新 MCP 服务器
    pub fn upsert_server(state: &AppState, server: McpServer) -> Result<(), AppError> {
        // Codex MCP and Provider settings share config.toml. Serialize every
        // read-modify-write with Provider switching/quick setup.
        let _codex_guard = futures::executor::block_on(
            state
                .proxy_service
                .lock_switch_for_app(AppType::Codex.as_str()),
        );
        // 读取旧状态：用于处理“编辑时取消勾选某个应用”的场景（需要从对应 live 配置中移除）
        let prev_apps = state
            .db
            .get_all_mcp_servers()?
            .get(&server.id)
            .map(|s| s.apps.clone())
            .unwrap_or_default();

        // 处理禁用：若旧版本启用但新版本取消，则需要从该应用的 live 配置移除
        if prev_apps.claude && !server.apps.claude {
            Self::disable_server_for_app(state, &server.id, &AppType::Claude)?;
        }
        if prev_apps.codex && !server.apps.codex {
            Self::disable_server_for_app(state, &server.id, &AppType::Codex)?;
        }
        if prev_apps.gemini && !server.apps.gemini {
            Self::disable_server_for_app(state, &server.id, &AppType::Gemini)?;
        }
        if prev_apps.grokbuild && !server.apps.grokbuild {
            Self::disable_server_for_app(state, &server.id, &AppType::GrokBuild)?;
        }
        if prev_apps.opencode && !server.apps.opencode {
            Self::disable_server_for_app(state, &server.id, &AppType::OpenCode)?;
        }
        if prev_apps.hermes && !server.apps.hermes {
            Self::disable_server_for_app(state, &server.id, &AppType::Hermes)?;
        }
        if prev_apps.workbuddy && !server.apps.workbuddy {
            Self::disable_server_for_target(state, &server.id, McpTargetId::WorkBuddy)?;
        }
        if prev_apps.qoderwork && !server.apps.qoderwork {
            Self::disable_server_for_target(state, &server.id, McpTargetId::QoderWork)?;
        }
        if prev_apps.trae_work && !server.apps.trae_work {
            Self::disable_server_for_target(state, &server.id, McpTargetId::TraeWork)?;
        }

        // 安全相关的取消分配必须先在 live 配置生效，才能提交数据库状态；
        // 否则清理失败后，界面会显示已关闭，但 Agent 仍会加载旧命令。
        state.db.save_mcp_server(&server)?;

        // 同步到各个启用的应用
        Self::sync_server_to_apps(state, &server)?;

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn delete_server(state: &AppState, id: &str) -> Result<bool, AppError> {
        let _codex_guard = futures::executor::block_on(
            state
                .proxy_service
                .lock_switch_for_app(AppType::Codex.as_str()),
        );
        let server = state.db.get_all_mcp_servers()?.shift_remove(id);

        if let Some(server) = server {
            // 从所有应用的 live 配置中移除
            Self::remove_server_from_all_apps(state, id, &server)?;
            // 只有所有 live 清理都成功，才删除可重试的权威记录。
            state.db.delete_mcp_server(id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 切换指定应用的启用状态
    pub fn toggle_app(
        state: &AppState,
        server_id: &str,
        app: AppType,
        enabled: bool,
    ) -> Result<(), AppError> {
        match McpTargetId::try_from(&app) {
            Ok(target) => Self::toggle_target(state, server_id, target, enabled),
            Err(_) => Ok(()),
        }
    }

    pub fn toggle_target(
        state: &AppState,
        server_id: &str,
        target: McpTargetId,
        enabled: bool,
    ) -> Result<(), AppError> {
        let lock_id = target.as_str();
        let _guard = futures::executor::block_on(state.proxy_service.lock_switch_for_app(lock_id));
        if enabled {
            if let Some(server) = state
                .db
                .update_mcp_server_target_enabled(server_id, &target, true)?
            {
                Self::sync_server_to_target(&server, &target)?;
            }
        } else if state.db.get_all_mcp_servers()?.contains_key(server_id) {
            Self::disable_server_for_target(state, server_id, target)?;
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到所有启用的应用
    fn sync_server_to_apps(_state: &AppState, server: &McpServer) -> Result<(), AppError> {
        for target in server.apps.enabled_targets() {
            Self::sync_server_to_target(server, &target)?;
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到指定应用
    fn sync_server_to_app(
        _state: &AppState,
        server: &McpServer,
        app: &AppType,
    ) -> Result<(), AppError> {
        if let Ok(target) = McpTargetId::try_from(app) {
            Self::sync_server_to_target(server, &target)?;
        }
        Ok(())
    }

    fn sync_server_to_target(server: &McpServer, target: &McpTargetId) -> Result<(), AppError> {
        match target {
            McpTargetId::Claude => {
                mcp::sync_single_server_to_claude(&Default::default(), &server.id, &server.server)?;
            }
            McpTargetId::Codex => {
                mcp::sync_single_server_to_codex(&Default::default(), &server.id, &server.server)?;
            }
            McpTargetId::Gemini => {
                mcp::sync_single_server_to_gemini(&Default::default(), &server.id, &server.server)?;
            }
            McpTargetId::GrokBuild => {
                mcp::sync_single_server_to_grokbuild(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            McpTargetId::OpenCode => {
                mcp::sync_single_server_to_opencode(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            McpTargetId::Hermes => {
                mcp::sync_single_server_to_hermes(&Default::default(), &server.id, &server.server)?;
            }
            McpTargetId::WorkBuddy => {
                mcp::sync_single_server_to_workbuddy(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            McpTargetId::QoderWork => {
                mcp::sync_single_server_to_qoderwork(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
            McpTargetId::TraeWork => {
                mcp::sync_single_server_to_traework(
                    &Default::default(),
                    &server.id,
                    &server.server,
                )?;
            }
        }
        Ok(())
    }

    /// 从所有曾启用过该服务器的应用中移除
    fn remove_server_from_all_apps(
        state: &AppState,
        id: &str,
        server: &McpServer,
    ) -> Result<(), AppError> {
        for target in server.apps.enabled_targets() {
            Self::disable_server_for_target(state, id, target)?;
        }
        Ok(())
    }

    fn disable_server_for_app(state: &AppState, id: &str, app: &AppType) -> Result<(), AppError> {
        if let Ok(target) = McpTargetId::try_from(app) {
            Self::disable_server_for_target(state, id, target)?;
        }
        Ok(())
    }

    fn disable_server_for_target(
        state: &AppState,
        id: &str,
        target: McpTargetId,
    ) -> Result<(), AppError> {
        Self::remove_server_from_target(id, &target)?;
        state
            .db
            .update_mcp_server_target_enabled(id, &target, false)?;
        Ok(())
    }

    fn remove_server_from_target(id: &str, target: &McpTargetId) -> Result<(), AppError> {
        match target {
            McpTargetId::Claude => mcp::remove_server_from_claude(id)?,
            McpTargetId::Codex => mcp::remove_server_from_codex(id)?,
            McpTargetId::Gemini => mcp::remove_server_from_gemini(id)?,
            McpTargetId::GrokBuild => mcp::remove_server_from_grokbuild(id)?,
            McpTargetId::OpenCode => mcp::remove_server_from_opencode(id)?,
            McpTargetId::Hermes => mcp::remove_server_from_hermes(id)?,
            McpTargetId::WorkBuddy => mcp::remove_server_from_workbuddy(id)?,
            McpTargetId::QoderWork => mcp::remove_server_from_qoderwork(id)?,
            McpTargetId::TraeWork => mcp::remove_server_from_traework(id)?,
        }
        Ok(())
    }

    /// Persist one application's imported servers without treating a shared ID
    /// as proof that executable specs are equivalent. A conflicting command,
    /// argument, environment, header, or URL must remain scoped to its source
    /// application until the user resolves it explicitly.
    fn persist_imported_servers(
        state: &AppState,
        config: &crate::app_config::MultiAppConfig,
        app: &AppType,
    ) -> Result<usize, AppError> {
        let target = McpTargetId::try_from(app)?;
        Self::persist_imported_servers_for_target(state, config, target)
    }

    fn persist_imported_servers_for_target(
        state: &AppState,
        config: &crate::app_config::MultiAppConfig,
        target: McpTargetId,
    ) -> Result<usize, AppError> {
        let Some(servers) = &config.mcp.servers else {
            return Ok(0);
        };
        let imported = servers.values().cloned().collect::<Vec<_>>();
        state
            .db
            .import_mcp_servers_atomically_for_target(&imported, &target)
    }

    /// 手动同步所有启用的 MCP 服务器到对应的应用。
    ///
    /// Best-effort：单个应用投影失败（如 ~/.claude.json 坏 JSON）不阻断
    /// 其余应用——各应用的 live 文件互相独立，一处损坏没有理由让其他
    /// 应用的 MCP 状态陈旧。全部跑完后若有失败，聚合成一个错误上报，
    /// 保留调用方的可见性。
    pub fn sync_all_enabled(state: &AppState) -> Result<(), AppError> {
        Self::sync_all_enabled_with_locking(state, true)
    }

    pub(crate) fn sync_all_enabled_inner(state: &AppState) -> Result<(), AppError> {
        Self::sync_all_enabled_with_locking(state, false)
    }

    fn sync_all_enabled_with_locking(
        state: &AppState,
        lock_each_app: bool,
    ) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        let mut failures: Vec<String> = Vec::new();
        for target in McpTargetId::all() {
            let _guard = lock_each_app.then(|| {
                futures::executor::block_on(
                    state.proxy_service.lock_switch_for_app(target.as_str()),
                )
            });
            if let Err(err) = Self::project_servers_to_target(&servers, &target) {
                log::warn!("同步 MCP 到 {target:?} 失败: {err}");
                failures.push(format!("{}: {err}", target.as_str()));
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Message(format!(
                "部分应用 MCP 同步失败: {}",
                failures.join("; ")
            )))
        }
    }

    /// 只把启用状态投影到单个应用。某个应用的 live 被整体重写后用它做
    /// 定向重投影，避免把无关应用的失败面（如 ~/.claude.json 坏 JSON）
    /// 牵连进目标应用的关键路径。
    pub fn sync_enabled_for_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
        let _guard =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app(app.as_str()));
        Self::sync_enabled_for_app_inner(state, app)
    }

    pub(crate) fn sync_enabled_for_app_inner(
        state: &AppState,
        app: &AppType,
    ) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;
        match McpTargetId::try_from(app) {
            Ok(target) => Self::project_servers_to_target(&servers, &target),
            Err(_) => Ok(()),
        }
    }

    fn project_servers_to_target(
        servers: &IndexMap<String, McpServer>,
        target: &McpTargetId,
    ) -> Result<(), AppError> {
        for server in servers.values() {
            if server.apps.is_enabled_for_target(target) {
                Self::sync_server_to_target(server, target)?;
            } else {
                Self::remove_server_from_target(&server.id, target)?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // 兼容层：支持旧的 v3.6.x 命令（已废弃，将在 v4.0 移除）
    // ========================================================================

    /// [已废弃] 获取指定应用的 MCP 服务器（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use get_all_servers instead")]
    pub fn get_servers(
        state: &AppState,
        app: AppType,
    ) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let all_servers = Self::get_all_servers(state)?;
        let mut result = HashMap::new();

        for (id, server) in all_servers {
            if server.apps.is_enabled_for(&app) {
                result.insert(id, server.server);
            }
        }

        Ok(result)
    }

    /// [已废弃] 设置 MCP 服务器在指定应用的启用状态（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use toggle_app instead")]
    pub fn set_enabled(
        state: &AppState,
        app: AppType,
        id: &str,
        enabled: bool,
    ) -> Result<bool, AppError> {
        Self::toggle_app(state, id, app, enabled)?;
        Ok(true)
    }

    /// [已废弃] 同步启用的 MCP 到指定应用（兼容旧 API）
    #[deprecated(since = "3.7.0", note = "Use sync_all_enabled instead")]
    pub fn sync_enabled(state: &AppState, app: AppType) -> Result<(), AppError> {
        let servers = Self::get_all_servers(state)?;

        for server in servers.values() {
            if server.apps.is_enabled_for(&app) {
                Self::sync_server_to_app(state, server, &app)?;
            }
        }

        Ok(())
    }

    /// 从 Claude 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_claude(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        crate::mcp::import_from_claude(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::Claude)
    }

    /// 从 Codex 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_codex(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        crate::mcp::import_from_codex(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::Codex)
    }

    /// 从 Gemini 导入 MCP（v3.7.0 已更新为统一结构）
    pub fn import_from_gemini(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp.rs）
        crate::mcp::import_from_gemini(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::Gemini)
    }

    /// 从 Grok Build 的 `[mcp_servers]` 导入 MCP。
    pub fn import_from_grokbuild(state: &AppState) -> Result<usize, AppError> {
        let mut temp_config = crate::app_config::MultiAppConfig::default();
        crate::mcp::import_from_grokbuild(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::GrokBuild)
    }

    /// 从 OpenCode 导入 MCP（v3.9.2+ 新增）
    pub fn import_from_opencode(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用原有的导入逻辑（从 mcp/opencode.rs）
        crate::mcp::import_from_opencode(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::OpenCode)
    }

    /// 从 Hermes 导入 MCP
    pub fn import_from_hermes(state: &AppState) -> Result<usize, AppError> {
        // 创建临时 MultiAppConfig 用于导入
        let mut temp_config = crate::app_config::MultiAppConfig::default();

        // 调用导入逻辑（从 mcp/hermes.rs）
        crate::mcp::import_from_hermes(&mut temp_config)?;
        Self::persist_imported_servers(state, &temp_config, &AppType::Hermes)
    }

    /// 从 WorkBuddy 导入 MCP。WorkBuddy 不是 AppType。
    pub fn import_from_workbuddy(state: &AppState) -> Result<usize, AppError> {
        let mut temp_config = crate::app_config::MultiAppConfig::default();
        crate::mcp::import_from_workbuddy(&mut temp_config)?;
        Self::persist_imported_servers_for_target(state, &temp_config, McpTargetId::WorkBuddy)
    }

    pub fn import_from_qoderwork(state: &AppState) -> Result<usize, AppError> {
        let mut temp_config = crate::app_config::MultiAppConfig::default();
        crate::mcp::import_from_qoderwork(&mut temp_config)?;
        Self::persist_imported_servers_for_target(state, &temp_config, McpTargetId::QoderWork)
    }

    pub fn import_from_traework(state: &AppState) -> Result<usize, AppError> {
        let mut temp_config = crate::app_config::MultiAppConfig::default();
        crate::mcp::import_from_traework(&mut temp_config)?;
        Self::persist_imported_servers_for_target(state, &temp_config, McpTargetId::TraeWork)
    }

    /// 从所有支持 MCP 的应用导入服务器，返回新导入的数量。
    ///
    /// Best-effort：单个应用导入失败（如坏 config.toml）不阻断其余应用；
    /// 全部跑完后若有失败，聚合成一个错误上报——历史实现逐应用
    /// `unwrap_or(0)` 吞错，坏文件只会表现为"导入成功 0 个"，用户
    /// 无从得知哪个应用出了问题。
    pub fn import_from_all_apps(state: &AppState) -> Result<usize, AppError> {
        let mut total = 0;
        let mut failures: Vec<String> = Vec::new();

        let results: [(&str, Result<usize, AppError>); 9] = [
            ("claude", Self::import_from_claude(state)),
            ("codex", Self::import_from_codex(state)),
            ("gemini", Self::import_from_gemini(state)),
            ("grokbuild", Self::import_from_grokbuild(state)),
            ("opencode", Self::import_from_opencode(state)),
            ("hermes", Self::import_from_hermes(state)),
            ("workbuddy", Self::import_from_workbuddy(state)),
            ("qoderwork", Self::import_from_qoderwork(state)),
            ("trae-work", Self::import_from_traework(state)),
        ];
        for (app, result) in results {
            match result {
                Ok(count) => total += count,
                Err(err) => {
                    log::warn!("从 {app} 导入 MCP 失败: {err}");
                    failures.push(format!("{app}: {err}"));
                }
            }
        }

        if failures.is_empty() {
            Ok(total)
        } else {
            Err(AppError::Message(format!(
                "已导入 {total} 个，部分应用导入失败: {}",
                failures.join("; ")
            )))
        }
    }
}
