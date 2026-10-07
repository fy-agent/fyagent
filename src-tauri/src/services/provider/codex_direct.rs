//! 写 Codex 的客户端文件：`config.toml` 只替换关键字段和独有字段，其余字节不碰；
//! 模型目录和配置在同一个操作里提交，崩溃后按 pending 前滚或丢弃。
//! FyAgent 请求来源切换保持原生 `auth.json`、账号标记和登录暂存不变。
//!
//! 写 Codex live 的入口（切换、新增第一个供应商、编辑当前供应商、同步、统一供应商、
//! 进入 / 退出代理）都走这里。不回填、不合并通用配置片段、不补回 MCP：这些设置本来就
//! 留在 live 里。
//!
//! 分三步：
//! 1. [`prepare_official_rows`]：需要时只读原生登录，准备官方模型目录；
//! 2. [`plan`]：在内存里算出 `config.toml` 的补丁、模型目录和代理契约，行有问题就在这里
//!    报错，什么都不写；
//! 3. [`run`]：拿写锁，只读原生登录以确定路由表的 `requires_openai_auth`，提交配置。

use serde_json::Value;
use toml_edit::{Item, Table, Value as TomlValue};

use crate::app_config::AppType;
use crate::codex_config::{
    codex_config_auth_store_mode, codex_disables_web_search, get_codex_auth_path,
    get_codex_config_path, get_codex_model_catalog_path, plan_codex_model_catalog,
    CodexAuthStoreMode,
};
use crate::codex_config::{
    plan_codex_stack_catalog, CodexCatalogRow, CodexStackCatalogMember, CodexStackRoute,
};
use crate::config::sorted_json_bytes;
use crate::database::Database;
use crate::error::AppError;
use crate::live::engine::{digest, read_current, LiveFile};
use crate::live::patch::toml::{value_text, TomlDocPatch, TomlSteps};
use crate::live::patch::WholeFile;
use crate::live::project::codex::{
    official_mirror_table, proxy_route_table, requires_openai_auth, row_catalog_pointer,
    without_row_catalog, CodexConfigPatch, CodexProjection, KnownTable, Route, RouteAuth,
    RouteWrite, RowInput, ROUTE_ID, WEB_SEARCH_DISABLED,
};
use crate::mode::contract::CONTRACT_VERSION;
use crate::mode::operation::{AppWrite, FileChange, OperationReport};
use crate::mode::stack::Member;
use crate::mode::state::{Contract, PendingTarget};
use crate::provider::Provider;
use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
use crate::services::subscription::CodexKeychainLogin;
use std::sync::Arc;

use super::codex_official_models::{self, NativeRows, OfficialLogin};

fn app() -> &'static str {
    AppType::Codex.as_str()
}

/// 官方卡：`category == "official"`，或按 `is_codex_official_provider` 认出来的（早期
/// 绑定托管账号时没存 category 的卡）。
pub(crate) fn is_official(provider: &Provider) -> bool {
    !provider.uses_subscription_proxy()
        && (provider.category.as_deref() == Some("official")
            || crate::proxy::providers::is_codex_official_provider(provider))
}

/// 本地代理给 Codex 的地址（带 `/v1`），不需要代理在运行：官方直连时写休眠表用。
pub(crate) fn configured_proxy_base_url(db: &Database) -> String {
    let (address, port) = db.get_proxy_listen_sync();
    let port = if port == 0 {
        crate::proxy::types::ProxyConfig::default().listen_port
    } else {
        port
    };
    format!(
        "{}/v1",
        crate::services::proxy::proxy_origin(&address, port)
    )
}

/// `config_text` 是不是代理的官方路由（指向本地代理，但不带占位 Key）。
pub(crate) fn routes_official_to_proxy(db: &Database, config_text: &str) -> bool {
    let (address, port) = db.get_proxy_listen_sync();
    crate::codex_config::codex_config_routes_official_to_proxy(config_text, |url| {
        is_proxy_base_url(url, &address, port)
    })
}

/// `url`（去掉末尾 `/`）是不是本地代理给 Codex 的地址。端口配成 0 时代理用系统分配的
/// 端口，只核对主机和路径。
fn is_proxy_base_url(url: &str, address: &str, port: u16) -> bool {
    let origin = crate::services::proxy::proxy_origin(address, port);
    if port != 0 {
        return url == format!("{origin}/v1");
    }
    // `http://127.0.0.1:0` → `http://127.0.0.1:`
    let host = origin.strip_suffix('0').unwrap_or(&origin);
    url.strip_prefix(host)
        .and_then(|rest| rest.strip_suffix("/v1"))
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}

/// 写成什么样。
#[derive(Clone, Copy)]
pub(crate) enum Target<'a> {
    /// 直连：这个供应商（`None`：没有直连供应商，只清掉关键字段）。
    Direct(Option<&'a Provider>),
    /// 代理契约：路由供应商；`base_url` 是本地代理给 Codex 的地址（带 `/v1`）；`stack` 是
    /// 发布的 Stack 供应商（不含路由那家），为空时和没有 Stack 模型逐字节一致。
    Proxy {
        route: &'a Provider,
        base_url: &'a str,
        stack: &'a [Member],
    },
}

/// live 现在是谁写进去的：据此清理上一个来源拥有的独有配置字段。
#[derive(Clone, Copy)]
pub(crate) enum Owner<'a> {
    Provider(&'a Provider),
    /// 代理契约记录的独有配置字段。
    Contract {
        contract: &'a Contract,
    },
    None,
}

/// 拿写锁之前只读准备的官方模型行和原生钥匙串登录。
#[derive(Default)]
pub(crate) struct Prepared {
    /// 官方做路由、又发布了 Stack 模型时目录里的官方行（[`prepare_official_rows`]）。
    native: Option<NativeRows>,
    /// 钥匙串里 Codex 的登录，在拿锁之前读好（见 [`prepare_official_rows`]）：钥匙串不归
    /// 写锁管，`security` 还可能弹出授权对话框，不能让写锁等着用户点。锁里没有就当读不出。
    keychain: Option<CodexKeychainLogin>,
}

fn target_provider<'a>(target: &Target<'a>) -> Option<&'a Provider> {
    match target {
        Target::Direct(provider) => *provider,
        Target::Proxy { route, .. } => Some(route),
    }
}

/// 官方做路由、又发布了 Stack 模型时，按操作之后 Codex 会用的登录取官方模型行。可能联网，
/// 所以在拿写锁之前做；登录是按未加锁读到的内容预测的，拿锁后在
/// [`run_with_edits`] 里按真实输入再核对一次。
///
/// 读钥匙串（`security` 可能等用户点授权框）和取官方行（跑 Codex 子进程、联网最多
/// 10 秒）放到阻塞线程池里做，不占异步运行时的工作线程。
pub(crate) async fn prepare_official_rows(
    db: &Database,
    owner: &Owner<'_>,
    target: &Target<'_>,
    prepared: &mut Prepared,
) -> Result<(), AppError> {
    let Target::Proxy { route, stack, .. } = target else {
        return Ok(());
    };
    if !needs_official_rows(route, stack) {
        return Ok(());
    }
    if matches!(
        codex_config_auth_store_mode(&read_config_text()),
        CodexAuthStoreMode::Keyring | CodexAuthStoreMode::Auto
    ) {
        prepared.keychain = Some(off_runtime(codex_official_models::keychain_login).await?);
    }
    let login = predicted_official_login(db, owner, target, prepared)?;
    let rows = off_runtime(move || codex_official_models::rows_for_switch(login.as_ref())).await?;
    // 取官方行最多要 10 秒，这期间 Codex 可能在钥匙串里换了号。取完再读一次，拿锁后按
    // 这次读到的核对（见 `run_with_edits`），换了号就停下。
    if rows.identity().is_some() && prepared.keychain.is_some() {
        prepared.keychain = Some(off_runtime(codex_official_models::keychain_login).await?);
    }
    prepared.native = Some(rows);
    Ok(())
}

/// 放到阻塞线程池里做，不占异步运行时的工作线程。
pub(crate) async fn off_runtime<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Message(format!("后台线程异常退出: {error}")))
}

/// 官方做路由、发布了 Stack 模型：目录里要写全官方模型。
pub(crate) fn needs_official_rows(route: &Provider, stack: &[Member]) -> bool {
    !stack.is_empty() && is_official(route)
}

/// 按未加锁读到的内容预测这次操作之后 Codex 会用的登录（能取官方列表的才返回）。钥匙串
/// 没预先读过（[`prepare_official_rows`] 之外的调用方）就在用到时读，调用方要在阻塞线程里。
pub(crate) fn predicted_official_login(
    db: &Database,
    owner: &Owner<'_>,
    target: &Target<'_>,
    prepared: &Prepared,
) -> Result<Option<OfficialLogin>, AppError> {
    // 来源切换只读原生登录；仍校验目标行，保证预测和写入采用同一配置。
    plan(db, owner, target, prepared)?;
    let live = read_current(&get_codex_auth_path())
        .ok()
        .flatten()
        .map(|bytes| serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null));
    let keychain = || {
        prepared
            .keychain
            .clone()
            .unwrap_or_else(codex_official_models::keychain_login)
    };
    Ok(login_after(live.as_ref(), &read_config_text(), keychain)
        .as_ref()
        .and_then(OfficialLogin::of))
}

/// 操作之后 Codex 实际会用的登录，按 `cli_auth_credentials_store`：file 看操作之后的
/// `auth.json`；keyring 看系统钥匙串（CC Switch 不改它）；auto 钥匙串里有就用它，确定
/// 没有才同 file（读不出钥匙串时 Codex 可能用着另一个登录，`auth.json` 不能顶替）；
/// ephemeral 和认不出的没有。`keychain` 只在要看钥匙串时调用。
fn login_after(
    live: Option<&Value>,
    config_text: &str,
    keychain: impl FnOnce() -> CodexKeychainLogin,
) -> Option<Value> {
    match codex_config_auth_store_mode(config_text) {
        CodexAuthStoreMode::File => live.cloned(),
        CodexAuthStoreMode::Keyring => match keychain() {
            CodexKeychainLogin::Found(login) => Some(login),
            CodexKeychainLogin::Missing | CodexKeychainLogin::Unknown => None,
        },
        CodexAuthStoreMode::Auto => match keychain() {
            CodexKeychainLogin::Found(login) => Some(login),
            CodexKeychainLogin::Missing => live.cloned(),
            CodexKeychainLogin::Unknown => None,
        },
        CodexAuthStoreMode::Ephemeral | CodexAuthStoreMode::Unknown => None,
    }
}

/// live 的 `config.toml`（读不出时为空）。
pub(crate) fn read_config_text() -> String {
    read_current(&get_codex_config_path())
        .ok()
        .flatten()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

fn project(provider: &Provider) -> Result<CodexProjection, AppError> {
    if let Some(config) = provider
        .settings_config
        .get("config")
        .and_then(Value::as_str)
    {
        crate::codex_config::preflight_codex_provider_table_conflicts(config)?;
    }
    CodexProjection::of(&RowInput {
        settings: &provider.settings_config,
        official: is_official(provider),
        proxy_injected_oauth: provider.uses_proxy_injected_oauth(),
    })
}

/// 这个供应商的独有字段，含 `web_search`（需要时为 `"disabled"`）。
fn exclusive_of(provider: &Provider, projection: &CodexProjection) -> Vec<(String, TomlValue)> {
    let mut exclusive = projection.exclusive.clone();
    let profile = crate::proxy::providers::resolve_codex_catalog_tool_profile(provider);
    if codex_disables_web_search(
        &provider.settings_config,
        &projection.catalog_input_text(),
        profile,
    ) {
        exclusive.retain(|(key, _)| key != "web_search");
        exclusive.push((
            "web_search".to_string(),
            TomlValue::from(WEB_SEARCH_DISABLED),
        ));
    }
    exclusive
}

/// live 现在对应的那一家带进来的独有字段：切走时值还相同就删。
pub(crate) fn outgoing_exclusive(owner: &Owner<'_>) -> Vec<(String, TomlValue)> {
    match owner {
        Owner::Provider(provider) => match project(provider) {
            Ok(projection) => exclusive_of(provider, &projection),
            Err(err) => {
                log::warn!(
                    "无法投影 Codex 供应商 {} 的独有字段，切走时不清理它们: {err}",
                    provider.id
                );
                Vec::new()
            }
        },
        Owner::Contract { contract, .. } => contract
            .exclusive
            .iter()
            .filter_map(|(key, value)| {
                let literal = value.as_str()?.parse::<TomlValue>().ok()?;
                Some((key.clone(), literal))
            })
            .collect(),
        Owner::None => Vec::new(),
    }
}

/// 数据库里 Codex 行能证明的旧路由表；用于精准清理历史配置。
struct RowFacts {
    retired: Vec<KnownTable>,
}

fn row_facts(db: &Database) -> Result<RowFacts, AppError> {
    let mut facts = RowFacts {
        retired: Vec::new(),
    };
    for provider in db.get_all_providers(app())?.values() {
        if is_official(provider) {
            continue;
        }
        let Some(doc) = provider
            .settings_config
            .get("config")
            .and_then(Value::as_str)
            .and_then(|text| text.parse::<toml_edit::DocumentMut>().ok())
        else {
            continue;
        };
        let providers = doc.get("model_providers").and_then(Item::as_table_like);
        let base_url_of = |id: &str| {
            providers
                .and_then(|table| table.get(id))
                .and_then(Item::as_table_like)
                .and_then(|table| table.get("base_url"))
                .and_then(Item::as_str)
                .map(|url| url.trim().to_string())
        };
        // 旧版整份写入时，路由表用的是行自己的 id（custom 是 CC Switch 现在写的，不算）。
        let selector = doc
            .get("model_provider")
            .and_then(Item::as_str)
            .map(str::trim)
            .filter(|id| !id.is_empty() && *id != ROUTE_ID);
        if let Some((id, base_url)) = selector.and_then(|id| Some((id, base_url_of(id)?))) {
            facts.retired.push(KnownTable {
                id: id.to_string(),
                base_url,
            });
        }
        if let Some(base_url) = doc
            .get("openai_base_url")
            .and_then(Item::as_str)
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
        {
            facts.retired.push(KnownTable {
                id: "cc-switch".to_string(),
                base_url,
            });
        }
    }
    Ok(facts)
}

/// 在内存里算好的一次 Codex 写入。
pub(crate) struct Planned {
    config: CodexConfigPatch,
    /// 第三方路由表的凭据来源：写入时按盘上有没有登录定 `requires_openai_auth`。
    stamp: Option<RouteAuth>,
    catalog: Option<Vec<u8>>,
    /// 目录里的官方行取自哪个登录的官方列表：拿写锁后和实际的目标登录核对。
    native_identity: Option<String>,
    /// 代理契约（直连时也算，没有用处）。
    pub contract: Contract,
}

impl Planned {
    /// `config.toml` 的补丁（编辑器显示用：在内存里对 live 做一次切换投影）。
    pub(crate) fn config(&self) -> &CodexConfigPatch {
        &self.config
    }
}

/// 算出写入内容；行有问题（比如会把官方登录发给第三方）就在这里报错，什么都不写。
pub(crate) fn plan(
    db: &Database,
    owner: &Owner<'_>,
    target: &Target<'_>,
    prepared: &Prepared,
) -> Result<Planned, AppError> {
    let facts = row_facts(db)?;
    let provider = target_provider(target);
    let projection = provider.map(project).transpose()?;

    let (top, nested, mut exclusive) = match (&projection, provider) {
        (Some(projection), Some(provider)) => (
            projection.top.clone(),
            projection.nested.clone(),
            exclusive_of(provider, projection),
        ),
        _ => (Vec::new(), Vec::new(), Vec::new()),
    };

    let official = provider.is_some_and(is_official);
    let (route, stamp) = match (target, &projection) {
        (Target::Direct(None), _) | (_, None) => (RouteWrite::Default, None),
        (Target::Direct(Some(_provider)), Some(projection)) => match &projection.route {
            Route::Official if crate::settings::unify_codex_session_history() => {
                (RouteWrite::OfficialMirror, None)
            }
            Route::Official => (
                RouteWrite::Official {
                    dormant_base_url: configured_proxy_base_url(db),
                },
                None,
            ),
            Route::Custom { table, auth: kind } => (RouteWrite::Custom(table.clone()), Some(*kind)),
            Route::BuiltIn { id, table } => (
                RouteWrite::BuiltIn {
                    id: id.clone(),
                    table: table.clone(),
                },
                None,
            ),
            Route::Default => (RouteWrite::Default, None),
        },
        (Target::Proxy { base_url, .. }, Some(_)) => {
            if official {
                (
                    RouteWrite::OfficialProxy {
                        base_url: base_url.to_string(),
                        unified: crate::settings::unify_codex_session_history(),
                    },
                    None,
                )
            } else {
                (
                    RouteWrite::Custom(proxy_route_table(ROUTE_ID, base_url, false)),
                    Some(RouteAuth::Bearer),
                )
            }
        }
    };

    let stack = match target {
        Target::Proxy { stack, .. } => *stack,
        Target::Direct(_) => &[],
    };
    let stack_catalog = match (provider, &projection) {
        (Some(provider), Some(projection)) => stack_catalog(provider, projection, stack, prepared)?,
        _ => None,
    };
    let catalog = match (stack_catalog, provider, &projection) {
        (Some(catalog), _, _) => {
            // 窗口类全局键会覆盖目录里的每一行，改由各家写进自己的行。
            exclusive.retain(|(key, _)| !STACK_SUNK_WINDOW_KEYS.contains(&key.as_str()));
            Some(catalog)
        }
        (None, Some(provider), Some(projection)) => {
            plan_codex_model_catalog(
                &provider.settings_config,
                &projection.catalog_input_text(),
                crate::proxy::providers::resolve_codex_catalog_tool_profile(provider),
            )?
            .catalog
        }
        _ => None,
    };
    let catalog = catalog
        .map(|catalog| sorted_json_bytes(&catalog))
        .transpose()?;

    let config = CodexConfigPatch {
        top,
        nested,
        exclusive,
        outgoing: outgoing_exclusive(owner),
        route,
        catalog: catalog.is_some(),
        retired: facts.retired,
    };
    let contract = contract_of(target, &config, catalog.as_deref());
    Ok(Planned {
        config,
        stamp,
        catalog,
        native_identity: prepared
            .native
            .as_ref()
            .and_then(NativeRows::identity)
            .map(str::to_string),
        contract,
    })
}

/// 能不能加进 Stack：配置要能解析。之后才坏掉的成员在目录里跳过（见 [`stack_catalog`]）。
pub(crate) fn check_stack_member(provider: &Provider) -> Result<(), AppError> {
    project(provider).map(|_| ()).map_err(|error| {
        AppError::Message(format!(
            "「{name}」的配置有问题，不能加入聚合 (The configuration of \"{name}\" is invalid, so it cannot join the aggregation): {error}",
            name = provider.name
        ))
    })
}

/// 路由那家的行指定了自己管理的模型目录文件（`model_catalog_json`）：Codex 只读那个
/// 文件，Stack 模型合并不进去。
pub(crate) fn route_owns_catalog(route: &Provider) -> bool {
    project(route).is_ok_and(|projection| row_catalog_pointer(&projection.top).is_some())
}

/// 去掉路由那家行里自己指定的模型目录指针之后的 `settings_config`；行里没有时为 `None`。
pub(crate) fn settings_without_row_catalog(route: &Provider) -> Option<Value> {
    let text = without_row_catalog(route.settings_config.get("config")?.as_str()?)?;
    let mut settings = route.settings_config.clone();
    settings
        .as_object_mut()?
        .insert("config".to_string(), Value::String(text));
    Some(settings)
}

/// 发布了 Stack 模型时不写进 `config.toml` 的全局键：Codex 拿它们覆盖目录里的每一行。
const STACK_SUNK_WINDOW_KEYS: &[&str] = &["model_context_window", "model_auto_compact_token_limit"];

/// 发布了 Stack 模型时的合并目录（路由那家的行在前，Stack 里的在后）；没有要发布的 Stack 模型，
/// 或者这次发布不了时为 `None`，目录照原来的规则算。
fn stack_catalog(
    route: &Provider,
    projection: &CodexProjection,
    stack: &[Member],
    prepared: &Prepared,
) -> Result<Option<serde_json::Value>, AppError> {
    if stack.is_empty() {
        return Ok(None);
    }
    // 路由那家的行指定了自己管理的目录文件：Codex 只读那个文件，合并不进去（界面上
    // 由 `route_owns_catalog` 给出提示）。
    if row_catalog_pointer(&projection.top).is_some() {
        log::warn!(
            "Codex 路由供应商 {} 使用自己的模型目录文件，Stack 模型不发布",
            route.id
        );
        return Ok(None);
    }
    let route_text = projection.catalog_input_text();
    let route_row = if is_official(route) {
        // 写了目录之后 Codex 只认文件里的模型：拿不到官方行时不能写出只有 Stack 模型的目录。
        match prepared.native.as_ref().and_then(NativeRows::rows) {
            Some(rows) => CodexStackRoute::Official {
                native: rows.to_vec(),
                config_text: &route_text,
            },
            None => {
                if prepared.native.is_some() {
                    log::warn!("读取不到 Codex 模型列表，Stack 模型暂不发布");
                }
                return Ok(None);
            }
        }
    } else {
        CodexStackRoute::ThirdParty(CodexCatalogRow {
            settings: &route.settings_config,
            config_text: &route_text,
            profile: crate::proxy::providers::resolve_codex_catalog_tool_profile(route),
        })
    };

    // 一家的配置坏了（比如云同步带来的行解析不了）只跳过这一家：报错会让进出代理、换
    // 路由、启动时接上这些 Codex 写入全部失败。
    let inputs: Vec<_> = stack
        .iter()
        .filter_map(|member| match project(&member.provider) {
            Ok(projection) => Some((
                member,
                projection.catalog_input_text(),
                crate::proxy::providers::resolve_codex_catalog_tool_profile(&member.provider),
            )),
            Err(error) => {
                log::warn!(
                    "Stack 模型「{}」的配置有问题，这次不发布它: {error}",
                    member.provider.name
                );
                None
            }
        })
        .collect();
    let members: Vec<CodexStackCatalogMember<'_>> = inputs
        .iter()
        .map(|(member, config_text, profile)| CodexStackCatalogMember {
            key: &member.key,
            provider_name: &member.provider.name,
            row: CodexCatalogRow {
                settings: &member.provider.settings_config,
                config_text,
                profile: *profile,
            },
        })
        .collect();
    plan_codex_stack_catalog(route_row, &members).map(Some)
}

fn table_text(table: &Table) -> String {
    let mut table = table.clone();
    table.remove("requires_openai_auth");
    let mut doc = toml_edit::DocumentMut::new();
    doc.insert("t", Item::Table(table));
    doc.to_string()
}

/// 代理契约：路由供应商在客户端那一侧的全部要求。摘要相同，换路由时客户端文件就不读
/// 也不写。requires_openai_auth 跟着原生登录走，不算进契约；来源切换不绑定账号。
fn contract_of(target: &Target<'_>, config: &CodexConfigPatch, catalog: Option<&[u8]>) -> Contract {
    let base_url = match target {
        Target::Proxy { base_url, .. } => *base_url,
        Target::Direct(_) => "",
    };
    let (selector, table) = match &config.route {
        RouteWrite::Custom(table) => (ROUTE_ID, table_text(table)),
        RouteWrite::OfficialProxy {
            base_url,
            unified: true,
        } => (
            ROUTE_ID,
            table_text(&official_mirror_table(Some(base_url), false)),
        ),
        // 不写选路，改道写在顶层（地址已经在 `url` 里）。
        RouteWrite::OfficialProxy { unified: false, .. } => ("", "openai_base_url".to_string()),
        _ => ("", String::new()),
    };
    let pairs = |entries: &[(String, TomlValue)]| -> Vec<Value> {
        let mut pairs: Vec<Value> = entries
            .iter()
            .map(|(key, value)| serde_json::json!([key, value_text(value)]))
            .collect();
        pairs.sort_by_key(|pair| pair[0].as_str().unwrap_or_default().to_string());
        pairs
    };
    let nested: Vec<Value> = config
        .nested
        .iter()
        .map(|(path, value)| serde_json::json!([path.join("."), value_text(value)]))
        .collect();
    let parts = serde_json::json!({
        "app": "codex",
        "version": CONTRACT_VERSION,
        "url": base_url,
        "top": pairs(&config.top),
        "nested": nested,
        "exclusive": pairs(&config.exclusive),
        "selector": selector,
        "table": table,
        "catalog": digest(catalog),
        "managed": null,
        "login": null,
    });
    let key = digest(Some(
        &serde_json::to_vec(&parts).expect("contract parts serialize"),
    ))
    .expect("digest");
    Contract {
        version: CONTRACT_VERSION,
        key,
        exclusive: config
            .exclusive
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value_text(value))))
            .collect(),
    }
}

/// 执行一次 Codex 来源写入：拿写锁，只提交配置和模型目录，最后落定 pending。
pub(crate) fn run(
    db: &Database,
    op: &str,
    planned: Planned,
    prepared: &Prepared,
    pending: PendingTarget,
) -> Result<OperationReport, AppError> {
    run_with_edits(db, op, planned, prepared, pending, None)
}

/// 同 [`run`]，另把编辑器里对全局设置的改动在同一次 `config.toml` 写入里应用。
pub(crate) fn run_with_edits(
    db: &Database,
    op: &str,
    planned: Planned,
    prepared: &Prepared,
    pending: PendingTarget,
    edits: Option<&super::editor_toml::TomlEdits>,
) -> Result<OperationReport, AppError> {
    // 先完成上次配置事务，再观察原生登录；来源切换不写登录、标记或暂存。
    let write = AppWrite::begin(db, app())?;
    let store = &write.store;
    let live_auth = read_current(&get_codex_auth_path())
        .ok()
        .flatten()
        .map(|bytes| serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null));
    let config_text = read_config_text();
    // 目录里的官方行是按拿锁前预测的登录取的：实际的目标登录换了人（Codex 恰好重新登录、
    // settle 补完了上一次操作），就停下，什么都不写。不能换成别的来源：目录已经算进契约。
    // 钥匙串用取完官方行后读到的那次（见 `prepare_official_rows`），不在锁里读。
    if let Some(expected) = &planned.native_identity {
        let keychain = || {
            prepared
                .keychain
                .clone()
                .unwrap_or(CodexKeychainLogin::Unknown)
        };
        let actual = login_after(live_auth.as_ref(), &config_text, keychain)
            .as_ref()
            .and_then(OfficialLogin::of)
            .map(|login| login.identity);
        if actual.as_deref() != Some(expected.as_str()) {
            return Err(AppError::localized(
                "codex.official_models_login_changed",
                "Codex 的登录在这次操作期间变了，官方模型列表对不上新的登录。本次没有写入任何文件，请重试",
                "The Codex login changed during this operation, so the official model list no longer matches it. Nothing was written; please try again",
            ));
        }
    }
    // Codex 把登录存在哪由 `cli_auth_credentials_store` 决定：只存 auth.json 时看它；
    // 存在系统钥匙串（keyring、auto）或认不出时按登录保留处理；
    // ephemeral 从不落盘，当成没登录。这里仅决定路由表字段，不改原生登录。
    let login = match codex_config_auth_store_mode(&config_text) {
        CodexAuthStoreMode::File => live_auth
            .as_ref()
            .is_some_and(crate::codex_config::codex_auth_has_openai_account_material),
        CodexAuthStoreMode::Ephemeral => false,
        CodexAuthStoreMode::Keyring | CodexAuthStoreMode::Auto | CodexAuthStoreMode::Unknown => {
            true
        }
    };
    let mut config = planned.config;
    if let (Some(kind), RouteWrite::Custom(table)) = (planned.stamp, &mut config.route) {
        if matches!(kind, RouteAuth::Bearer | RouteAuth::EnvKey) {
            table.insert(
                "requires_openai_auth",
                toml_edit::value(requires_openai_auth(kind, login)),
            );
        }
    }

    let catalog_patch = planned.catalog.map(WholeFile::Write);
    // 先应用编辑器里的全局改动（有的话），再换关键字段。
    let mut config_steps: Vec<&dyn TomlDocPatch> = Vec::new();
    if let Some(edits) = edits {
        config_steps.push(edits);
    }
    config_steps.push(&config);
    let config_patch = TomlSteps(config_steps);

    let mut changes: Vec<FileChange<'_>> = Vec::new();
    changes.push(FileChange {
        file: LiveFile::private(get_codex_config_path()),
        patch: &config_patch,
    });
    if let Some(patch) = &catalog_patch {
        changes.push(FileChange {
            file: LiveFile::shared(get_codex_model_catalog_path()),
            patch,
        });
    }
    let report = write.run(op, &changes, pending);
    // 按磁盘上的实际内容记新启动的 Codex 会读到的目录：失败时可能已经发布了一部分。
    super::codex_client_catalog::observe(store);
    report
}

/// 直连写入：`plan` → `run`；不准备或切换原生账号。
pub(crate) fn write_direct(
    db: &Database,
    _manager: &Arc<CodexOAuthManager>,
    op: &str,
    owner: Owner<'_>,
    target: Option<&Provider>,
    pending: PendingTarget,
) -> Result<OperationReport, AppError> {
    let resolved = target
        .map(|provider| super::ProviderCredentials::resolve(db, "codex", provider))
        .transpose()?;
    let target = Target::Direct(resolved.as_ref());
    let prepared = Prepared::default();
    let planned = plan(db, &owner, &target, &prepared)?;
    run(db, op, planned, &prepared, pending)
}

/// 只校验，不写：切换前用它挡住会被拒绝的目标（行有问题时指针不能先动）。
pub(crate) fn preflight(db: &Database, provider: &Provider) -> Result<(), AppError> {
    let resolved = super::ProviderCredentials::resolve(db, "codex", provider)?;
    let target = Target::Direct(Some(&resolved));
    plan(db, &Owner::None, &target, &Prepared::default()).map(|_| ())
}

#[cfg(test)]
mod tests {
    #[test]
    fn preflight_rejects_a_foreign_credential_binding_before_projection() {
        let db = crate::database::Database::memory().unwrap();
        let provider = crate::provider::Provider::with_id(
            "credential-target".into(),
            "Credential target".into(),
            serde_json::json!({
                "auth": {}, "config": "",
                "credentialRef": "pc_00000000-0000-4000-8000-000000000000"
            }),
            None,
        );
        assert!(super::preflight(&db, &provider).is_err());
        assert!(db.get_current_provider("codex").unwrap().is_none());
        assert!(db.get_all_providers("codex").unwrap().is_empty());
    }

    #[test]
    fn desired_projection_rejects_conflicts_in_an_idle_provider_table() {
        let provider = crate::provider::Provider::with_id(
            "invalid-target".into(),
            "Invalid target".into(),
            serde_json::json!({ "auth": {}, "config":
                "[model_providers.idle]\nname = 'Idle'\nauth = {}\nenv_key = 'KEY'\n" }),
            None,
        );
        assert!(super::project(&provider).is_err());
    }

    #[test]
    fn explicit_subscription_is_not_an_official_live_projection() {
        let mut provider = crate::provider::Provider::with_id(
            crate::database::CODEX_OFFICIAL_PROVIDER_ID.to_string(),
            "Bound subscription".to_string(),
            serde_json::json!({ "auth": {}, "config": "" }),
            None,
        );
        provider.category = Some("official".to_string());
        for kind in ["codex_oauth", "xai_oauth"] {
            provider.meta = Some(crate::provider::ProviderMeta {
                provider_type: Some(kind.to_string()),
                ..Default::default()
            });
            assert!(!super::is_official(&provider));
        }
    }

    use super::is_proxy_base_url;

    #[test]
    fn the_proxy_base_url_matches_the_listen_address() {
        assert!(is_proxy_base_url(
            "http://127.0.0.1:15721/v1",
            "127.0.0.1",
            15721
        ));
        assert!(is_proxy_base_url(
            "http://127.0.0.1:15721/v1",
            "0.0.0.0",
            15721
        ));
        assert!(!is_proxy_base_url(
            "http://127.0.0.1:10531/v1",
            "127.0.0.1",
            15721
        ));
        assert!(is_proxy_base_url("http://[::1]:15721/v1", "::", 15721));
        // 端口 0：系统分配的端口，只核对主机和路径。
        assert!(is_proxy_base_url(
            "http://127.0.0.1:54321/v1",
            "127.0.0.1",
            0
        ));
        assert!(!is_proxy_base_url("http://127.0.0.1:/v1", "127.0.0.1", 0));
        assert!(!is_proxy_base_url(
            "http://127.0.0.1:54321/v2",
            "127.0.0.1",
            0
        ));
        assert!(!is_proxy_base_url(
            "http://10.0.0.1:54321/v1",
            "127.0.0.1",
            0
        ));
    }
}
