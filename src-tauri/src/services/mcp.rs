use indexmap::IndexMap;
use std::collections::HashMap;

use crate::app_config::{AppType, McpServer, McpTargetId};
use crate::error::AppError;
use crate::mcp;
use crate::mcp::{
    McpImportCounts, McpImportReport, McpImportSourceResult, McpProjectionFailure, McpServerView,
};
use crate::store::AppState;

/// MCP 相关业务逻辑（v3.7.0 统一结构）
pub struct McpService;

/// 「重新同步到各应用」里单个应用的结果
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAppSyncOutcome {
    pub app: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl McpService {
    /// 由 FyAgent 管理 MCP 的应用（Claude Desktop、OpenClaw 不支持）
    pub fn live_sync_apps() -> Vec<AppType> {
        AppType::all()
            .filter(|app| !matches!(app, AppType::OpenClaw | AppType::ClaudeDesktop))
            .collect()
    }

    /// 解析「重新同步」的目标应用：缺省或空列表＝全部受管应用；
    /// 不认识或不支持 MCP 的应用直接报错，不静默跳过。
    pub fn resync_targets(apps: Option<&[String]>) -> Result<Vec<AppType>, AppError> {
        let managed = Self::live_sync_apps();
        let Some(apps) = apps.filter(|apps| !apps.is_empty()) else {
            return Ok(managed);
        };
        let mut targets = Vec::new();
        for raw in apps {
            let app = <AppType as std::str::FromStr>::from_str(raw)?;
            if !managed.contains(&app) {
                return Err(AppError::Message(format!(
                    "{} 不支持由 FyAgent 管理 MCP",
                    app.as_str()
                )));
            }
            if !targets.contains(&app) {
                targets.push(app);
            }
        }
        Ok(targets)
    }

    /// 把数据库里的启用状态重新投影到一个应用的 live 配置，结果按应用报告。
    /// 调用方负责先拿这个应用的切换锁。
    pub fn resync_app(state: &AppState, app: &AppType) -> McpAppSyncOutcome {
        match Self::sync_enabled_for_app_inner(state, app) {
            Ok(()) => McpAppSyncOutcome {
                app: app.as_str().to_string(),
                ok: true,
                error: None,
            },
            Err(err) => {
                log::warn!("重新同步 MCP 到 {app:?} 失败: {err}");
                McpAppSyncOutcome {
                    app: app.as_str().to_string(),
                    ok: false,
                    error: Some(err.to_string()),
                }
            }
        }
    }

    /// 获取所有 MCP 服务器（统一结构）
    pub fn get_all_servers(state: &AppState) -> Result<IndexMap<String, McpServer>, AppError> {
        state.db.get_all_mcp_servers()
    }

    pub fn get_server_views(state: &AppState) -> Result<IndexMap<String, McpServerView>, AppError> {
        let sources = state.db.get_mcp_import_sources()?;
        Ok(Self::get_all_servers(state)?
            .into_iter()
            .map(|(id, server)| {
                let view = McpServerView {
                    sources: sources.get(&id).cloned().unwrap_or_default(),
                    server,
                };
                (id, view)
            })
            .collect())
    }

    /// 添加或更新 MCP 服务器
    pub fn upsert_server(state: &AppState, server: McpServer) -> Result<(), AppError> {
        // Validate even library-only entries, before any DB or live-file mutation.
        mcp::validate_server_spec(&server.server)?;
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
            .get_mcp_server(&server.id)?
            .map(|s| s.apps)
            .unwrap_or_default();

        // 处理禁用：若旧版本启用但新版本取消，则需要从该应用的 live 配置移除
        for target in McpTargetId::all() {
            if prev_apps.is_enabled_for_target(&target)
                && !server.apps.is_enabled_for_target(&target)
            {
                Self::disable_server_for_target(state, &server.id, target)?;
            }
        }

        // 安全相关的取消分配必须先在 live 配置生效，才能提交数据库状态；
        // 否则清理失败后，界面会显示已关闭，但 Agent 仍会加载旧命令。
        state.db.save_mcp_server(&server)?;

        // 同步到各个启用的应用
        Self::sync_server_to_apps(&server)?;

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn delete_server(state: &AppState, id: &str) -> Result<bool, AppError> {
        let _codex_guard = futures::executor::block_on(
            state
                .proxy_service
                .lock_switch_for_app(AppType::Codex.as_str()),
        );
        let server = state.db.get_mcp_server(id)?;

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
        } else if state.db.get_mcp_server(server_id)?.is_some() {
            Self::disable_server_for_target(state, server_id, target)?;
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到所有启用的应用
    fn sync_server_to_apps(server: &McpServer) -> Result<(), AppError> {
        for target in server.apps.enabled_targets() {
            Self::sync_server_to_target(server, &target)?;
        }

        Ok(())
    }

    /// 将 MCP 服务器同步到指定应用
    fn sync_server_to_app(server: &McpServer, app: &AppType) -> Result<(), AppError> {
        if let Ok(target) = McpTargetId::try_from(app) {
            Self::sync_server_to_target(server, &target)?;
        }
        Ok(())
    }

    fn sync_server_to_target(server: &McpServer, target: &McpTargetId) -> Result<(), AppError> {
        mcp::validate_server_spec(&server.server)?;
        match target {
            McpTargetId::Claude => {
                mcp::sync_single_server_to_claude(&server.id, &server.server)?;
            }
            McpTargetId::Codex => {
                mcp::sync_single_server_to_codex(&server.id, &server.server)?;
            }
            McpTargetId::Gemini => {
                mcp::sync_single_server_to_gemini(&server.id, &server.server)?;
            }
            McpTargetId::GrokBuild => {
                mcp::sync_single_server_to_grokbuild(&server.id, &server.server)?;
            }
            McpTargetId::OpenCode => {
                mcp::sync_single_server_to_opencode(&server.id, &server.server)?;
            }
            McpTargetId::Hermes => {
                mcp::sync_single_server_to_hermes(&server.id, &server.server)?;
            }
            McpTargetId::WorkBuddy => {
                mcp::sync_single_server_to_workbuddy(&server.id, &server.server)?;
            }
            McpTargetId::QoderWork => {
                mcp::sync_single_server_to_qoderwork(&server.id, &server.server)?;
            }
            McpTargetId::TraeWork => {
                mcp::sync_single_server_to_traework(&server.id, &server.server)?;
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

    fn import_source(state: &AppState, target: McpTargetId) -> Result<McpImportCounts, AppError> {
        let _guard =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app(target.as_str()));
        let mut config = crate::app_config::MultiAppConfig::default();
        match target {
            McpTargetId::Claude => mcp::import_from_claude(&mut config)?,
            McpTargetId::Codex => mcp::import_from_codex(&mut config)?,
            McpTargetId::Gemini => mcp::import_from_gemini(&mut config)?,
            McpTargetId::GrokBuild => mcp::import_from_grokbuild(&mut config)?,
            McpTargetId::OpenCode => mcp::import_from_opencode(&mut config)?,
            McpTargetId::Hermes => mcp::import_from_hermes(&mut config)?,
            McpTargetId::WorkBuddy => mcp::import_from_workbuddy(&mut config)?,
            McpTargetId::QoderWork => mcp::import_from_qoderwork(&mut config)?,
            McpTargetId::TraeWork => mcp::import_from_traework(&mut config)?,
        };
        let imported = config
            .mcp
            .servers
            .unwrap_or_default()
            .into_values()
            .collect::<Vec<_>>();
        state.db.import_mcp_servers_with_report(&imported, &target)
    }

    /// Each selected source keeps its own atomic DAO transaction. Failures do
    /// not suppress successful independent sources or expose raw diagnostics.
    /// Project accepted targets only, after all source transactions settle.
    pub fn import_from_sources(
        state: &AppState,
        sources: Vec<McpTargetId>,
    ) -> Result<McpImportReport, AppError> {
        if sources.is_empty()
            || sources.len() > McpTargetId::all().count()
            || sources
                .iter()
                .enumerate()
                .any(|(index, source)| sources[..index].contains(source))
        {
            return Err(AppError::McpValidation(
                "请选择不重复的 MCP 导入来源".into(),
            ));
        }
        let mut results = Vec::new();
        for source in sources {
            let (counts, failure_code) = match Self::import_source(state, source) {
                Ok(counts) => (counts, None),
                Err(_) => {
                    log::warn!(
                        "MCP import from {} failed; source transaction not accepted",
                        source.as_str()
                    );
                    (McpImportCounts::default(), Some("source_failed"))
                }
            };
            results.push(McpImportSourceResult {
                source,
                counts,
                failure_code,
            });
        }
        let mut projection_failures = Vec::new();
        for result in &results {
            if result.failure_code.is_some() {
                continue;
            }
            let target = result.source;
            let _guard = futures::executor::block_on(
                state.proxy_service.lock_switch_for_app(target.as_str()),
            );
            // A read failure after accepted transactions must not erase their
            // counts or prevent other targets from being attempted.
            let failures = match Self::get_all_servers(state) {
                Ok(servers) => Self::project_servers_to_target_failures(&servers, &target),
                Err(error) => vec![(None, error)],
            };
            for (server_id, error) in failures {
                let reason = match error {
                    AppError::Io { .. } | AppError::IoContext { .. } => "io_failed",
                    AppError::Json { .. }
                    | AppError::Toml { .. }
                    | AppError::Config(_)
                    | AppError::McpValidation(_) => "invalid_config",
                    _ => "projection_failed",
                };
                projection_failures.push(McpProjectionFailure {
                    target,
                    server_id,
                    reason,
                });
            }
        }
        Ok(McpImportReport {
            contract_version: 1,
            sources: results,
            projection_failed: projection_failures.len(),
            projection_failures,
        })
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
        if *target == McpTargetId::Claude {
            return crate::claude_mcp::sync_collection(servers);
        }
        let mut failures: IndexMap<String, Vec<String>> = IndexMap::new();
        for (server_id, error) in Self::project_servers_to_target_failures(servers, target) {
            failures
                .entry(error.to_string())
                .or_default()
                .push(server_id.expect("non-Claude projections report a server ID"));
        }
        if failures.is_empty() {
            return Ok(());
        }
        let failed_count: usize = failures.values().map(Vec::len).sum();
        Err(AppError::Message(format!(
            "{failed_count} 个 MCP 条目写入失败: {}",
            failures
                .into_iter()
                .map(|(error, ids)| format!("{}: {} ({error})", target.as_str(), ids.join(", ")))
                .collect::<Vec<_>>()
                .join("; "),
        )))
    }

    // Retain individual errors for import reporting; the sync compatibility
    // wrapper above keeps its existing aggregated native error contract.
    fn project_servers_to_target_failures(
        servers: &IndexMap<String, McpServer>,
        target: &McpTargetId,
    ) -> Vec<(Option<String>, AppError)> {
        if *target == McpTargetId::Claude {
            return crate::claude_mcp::sync_collection(servers)
                .err()
                .map(|error| vec![(None, error)])
                .unwrap_or_default();
        }
        let mut failures = Vec::new();
        for server in servers.values() {
            let result = if server.apps.is_enabled_for_target(target) {
                Self::sync_server_to_target(server, target)
            } else {
                Self::remove_server_from_target(&server.id, target)
            };
            if let Err(error) = result {
                failures.push((Some(server.id.clone()), error));
            }
        }
        failures
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
                Self::sync_server_to_app(server, &app)?;
            }
        }

        Ok(())
    }

    pub fn import_from_traework(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::TraeWork).map(|counts| counts.added)
    }

    pub fn import_from_gemini(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::Gemini).map(|counts| counts.added)
    }

    pub fn import_from_codex(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::Codex).map(|counts| counts.added)
    }

    pub fn import_from_workbuddy(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::WorkBuddy).map(|counts| counts.added)
    }

    pub fn import_from_opencode(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::OpenCode).map(|counts| counts.added)
    }

    pub fn import_from_hermes(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::Hermes).map(|counts| counts.added)
    }

    pub fn import_from_claude(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::Claude).map(|counts| counts.added)
    }

    pub fn import_from_grokbuild(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::GrokBuild).map(|counts| counts.added)
    }

    pub fn import_from_qoderwork(state: &AppState) -> Result<usize, AppError> {
        Self::import_source(state, McpTargetId::QoderWork).map(|counts| counts.added)
    }

    /// Compatibility return value remains the number of newly added rows.
    pub fn import_from_all_apps(state: &AppState) -> Result<usize, AppError> {
        let report = Self::import_from_sources(state, McpTargetId::all().collect())?;
        let total = report
            .sources
            .iter()
            .map(|source| source.counts.added)
            .sum();
        let failed = report
            .sources
            .iter()
            .filter(|source| source.failure_code.is_some())
            .map(|source| source.source.as_str())
            .collect::<Vec<_>>();
        if report.projection_failed > 0 {
            return Err(AppError::Message(format!(
                "已导入 {total} 个，来源导入失败 {} 个，工具配置写入失败 {} 项",
                failed.len(),
                report.projection_failed,
            )));
        }
        if failed.is_empty() {
            Ok(total)
        } else {
            Err(AppError::Message(format!(
                "已导入 {total} 个，部分应用导入失败: {}",
                failed.join(", ")
            )))
        }
    }
}
