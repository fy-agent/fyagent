//! HTTP代理服务器
//!
//! 基于Axum的HTTP服务器，处理代理请求
//!
//! Uses a manual hyper HTTP/1.1 accept loop with `preserve_header_case(true)` so
//! that the original header-name casing from the CLI client is captured in a
//! `HeaderCaseMap` extension.  This map is later forwarded to the upstream via
//! the hyper-based HTTP client, producing wire-level header casing identical to
//! a direct (non-proxied) CLI request.

use super::{
    failover_switch::FailoverSwitchManager,
    handlers,
    log_codes::srv as log_srv,
    provider_router::ProviderRouter,
    providers::{codex_chat_history::CodexChatHistoryStore, gemini_shadow::GeminiShadowStore},
    types::*,
    ProxyError,
};
use crate::database::Database;
use axum::{
    extract::DefaultBodyLimit,
    routing::{any, get, post},
    Router,
};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;

pub(crate) fn validate_listen_address(address: &str) -> Result<std::net::IpAddr, ProxyError> {
    address
        .parse()
        .map_err(|_| ProxyError::ConfigError("代理监听地址必须是有效 IP 地址".to_string()))
}

async fn authenticate_lan_request(
    axum::extract::State(state): axum::extract::State<ProxyState>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;

    let loopback = request.extensions().get::<axum::extract::ConnectInfo<SocketAddr>>().is_some_and(|peer| {
        peer.0.ip().is_loopback()
            || matches!(peer.0.ip(), std::net::IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback()))
    });
    let access_key = state.access_key.read().await.clone();
    let expected = match access_key {
        Ok(Some(key)) if !key.is_empty() => key,
        Err(_) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "局域网代理访问密钥读取失败",
            )
                .into_response();
        }
        Ok(_) if loopback => return next.run(request).await,
        _ => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "局域网代理访问密钥未配置",
            )
                .into_response()
        }
    };
    let headers = request.headers();
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, key)| key.trim());
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let digest = Sha256::digest(expected.as_bytes());
    let matches = |key: Option<&str>| {
        key.is_some_and(|key| bool::from(digest.ct_eq(&Sha256::digest(key.as_bytes()))))
    };
    let bearer_matches = matches(bearer);
    let api_key_matches = matches(api_key);
    if !loopback && !(bearer_matches | api_key_matches) {
        return ProxyError::AuthError("局域网代理访问密钥缺失或无效".into()).into_response();
    }
    if bearer_matches {
        request
            .headers_mut()
            .remove(axum::http::header::AUTHORIZATION);
    }
    if api_key_matches {
        request.headers_mut().remove("x-api-key");
    }
    next.run(request).await
}

/// 代理服务器状态（共享）
#[derive(Clone)]
pub struct ProxyState {
    pub db: Arc<Database>,
    access_key: Arc<RwLock<Result<Option<String>, String>>>,
    pub config: Arc<RwLock<ProxyConfig>>,
    pub status: Arc<RwLock<ProxyStatus>>,
    pub start_time: Arc<RwLock<Option<std::time::Instant>>>,
    /// 每个应用类型当前使用的 provider (app_type -> (provider_id, provider_name))
    pub current_providers: Arc<RwLock<std::collections::HashMap<String, (String, String)>>>,
    /// 共享的 ProviderRouter（持有熔断器状态，跨请求保持）
    pub provider_router: Arc<ProviderRouter>,
    /// Gemini Native shadow state，用于 thoughtSignature / tool call 回放
    pub gemini_shadow: Arc<GeminiShadowStore>,
    /// Codex Chat bridge history，用于恢复 previous_response_id 指向的 tool call
    pub codex_chat_history: Arc<CodexChatHistoryStore>,
    /// AppHandle，用于发射事件和更新托盘菜单
    pub app_handle: Option<tauri::AppHandle>,
    /// 故障转移切换管理器
    pub failover_manager: Arc<FailoverSwitchManager>,
}

/// 代理HTTP服务器
pub struct ProxyServer {
    config: ProxyConfig,
    state: ProxyState,
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    /// 服务器任务句柄，用于等待服务器实际关闭
    server_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

#[cfg(test)]
impl ProxyState {
    /// 测试用：用这个数据库、其余都是默认值的状态。
    pub(crate) fn for_test(db: Arc<Database>) -> Self {
        ProxyServer::new(ProxyConfig::default(), db, None).state
    }
}

impl ProxyServer {
    pub fn new(
        config: ProxyConfig,
        db: Arc<Database>,
        app_handle: Option<tauri::AppHandle>,
    ) -> Self {
        // 创建共享的 ProviderRouter（熔断器状态将跨所有请求保持）
        let provider_router = Arc::new(ProviderRouter::new(db.clone()));
        // 创建故障转移切换管理器
        let failover_manager = Arc::new(FailoverSwitchManager::new());

        let access_key = db
            .get_setting("proxy_access_key")
            .map_err(|error| error.to_string());
        let state = ProxyState {
            access_key: Arc::new(RwLock::new(access_key)),
            db,
            config: Arc::new(RwLock::new(config.clone())),
            status: Arc::new(RwLock::new(ProxyStatus::default())),
            start_time: Arc::new(RwLock::new(None)),
            current_providers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            provider_router,
            gemini_shadow: Arc::new(GeminiShadowStore::default()),
            codex_chat_history: Arc::new(CodexChatHistoryStore::default()),
            app_handle,
            failover_manager,
        };

        Self {
            config,
            state,
            shutdown_tx: Arc::new(RwLock::new(None)),
            server_handle: Arc::new(RwLock::new(None)),
        }
    }

    #[cfg(test)]
    pub(crate) async fn cached_access_key_for_test(&self) -> Option<String> {
        self.state.access_key.read().await.as_ref().unwrap().clone()
    }

    #[cfg(test)]
    pub(crate) fn authenticate_router_for_test(&self, router: Router) -> Router {
        router.layer(axum::middleware::from_fn_with_state(
            self.state.clone(),
            authenticate_lan_request,
        ))
    }

    pub(crate) async fn update_access_key(&self, key: String) {
        *self.state.access_key.write().await = Ok(Some(key));
    }

    pub async fn start(&self) -> Result<ProxyServerInfo, ProxyError> {
        // 检查是否已在运行
        if self.shutdown_tx.read().await.is_some() {
            return Err(ProxyError::AlreadyRunning);
        }

        let ip = validate_listen_address(&self.config.listen_address).map_err(|error| {
            log::error!("代理启动被拒绝: {error}");
            error
        })?;
        if !ip.is_loopback()
            && self
                .state
                .access_key
                .read()
                .await
                .as_ref()
                .map_err(|_| ProxyError::ConfigError("局域网代理访问密钥读取失败".into()))?
                .as_ref()
                .is_none_or(|key| key.is_empty())
        {
            return Err(ProxyError::ConfigError(
                "局域网监听需要访问密钥，请先保存代理监听设置或重新生成密钥".into(),
            ));
        }
        let addr = SocketAddr::new(ip, self.config.listen_port);

        // 创建关闭通道
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // 构建路由
        let app = self.build_router(super::content_encoding::MAX_REQUEST_BODY_BYTES);

        // 绑定监听器
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| ProxyError::BindFailed(e.to_string()))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| ProxyError::BindFailed(e.to_string()))?;
        let actual_port = local_addr.port();

        log::info!("[{}] 代理服务器启动于 {local_addr}", log_srv::STARTED);

        // 更新全局代理端口，用于系统代理检测
        crate::proxy::http_client::set_proxy_port(actual_port);

        // 保存关闭句柄
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // 更新状态
        let mut status = self.state.status.write().await;
        status.running = true;
        status.address = self.config.listen_address.clone();
        status.port = actual_port;
        drop(status);

        // 记录启动时间
        *self.state.start_time.write().await = Some(std::time::Instant::now());

        // 启动服务器 — 使用手动 hyper HTTP/1.1 accept loop
        // 开启 preserve_header_case 以捕获客户端请求头的原始大小写
        let state = self.state.clone();
        let handle = tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (stream, remote_addr) = match result {
                            Ok(v) => v,
                            Err(e) => {
                                log::error!("[{SRV}] accept 失败: {e}", SRV = log_srv::ACCEPT_ERR);
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                continue;
                            }
                        };

                        let app = app.clone();
                        tokio::spawn(async move {
                            // Peek raw TCP bytes to capture original header casing
                            // before hyper parses (and lowercases) the header names.
                            let original_cases = {
                                let mut peek_buf = vec![0u8; 8192];
                                match stream.peek(&mut peek_buf).await {
                                    Ok(n) => {
                                        let cases = super::hyper_client::OriginalHeaderCases::from_raw_bytes(&peek_buf[..n]);
                                        log::debug!(
                                            "[ProxyServer] Peeked {} bytes, captured {} header casings",
                                            n, cases.cases.len()
                                        );
                                        cases
                                    }
                                    Err(e) => {
                                        log::debug!("[ProxyServer] peek failed (non-fatal): {e}");
                                        super::hyper_client::OriginalHeaderCases::default()
                                    }
                                }
                            };

                            // service_fn 将 axum Router（tower::Service）桥接到 hyper
                            let service = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                                let mut router = app.clone();
                                let cases = original_cases.clone();
                                async move {
                                    // 将 hyper::body::Incoming 转为 axum::body::Body，保留 extensions
                                    let (mut parts, body) = req.into_parts();

                                    // Insert our own header case map alongside hyper's internal one
                                    parts.extensions.insert(cases);
                                    parts.extensions.insert(axum::extract::ConnectInfo(remote_addr));

                                    let body = axum::body::Body::new(body);
                                    let axum_req = http::Request::from_parts(parts, body);
                                    <Router as tower::Service<http::Request<axum::body::Body>>>::call(&mut router, axum_req).await
                                }
                            });

                            if hyper::server::conn::http1::Builder::new()
                                .preserve_header_case(true)
                                .serve_connection(TokioIo::new(stream), service)
                                .await
                                .is_err()
                            {
                                // Connection reset / broken pipe 等在代理场景下很常见，debug 级别
                                log::debug!("[{SRV}] connection error: transport error", SRV = log_srv::CONN_ERR);
                            }
                        });
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }

            // 服务器停止后更新状态
            state.status.write().await.running = false;
            *state.start_time.write().await = None;
        });

        // 保存服务器任务句柄
        *self.server_handle.write().await = Some(handle);

        Ok(ProxyServerInfo {
            address: self.config.listen_address.clone(),
            port: actual_port,
            started_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn stop(&self) -> Result<(), ProxyError> {
        // 1. 发送关闭信号
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(());
        } else {
            return Err(ProxyError::NotRunning);
        }

        // 2. 等待服务器任务结束（带 5 秒超时保护）
        if let Some(handle) = self.server_handle.write().await.take() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), handle).await {
                Ok(Ok(())) => {
                    log::info!("[{}] 代理服务器已完全停止", log_srv::STOPPED);
                    Ok(())
                }
                Ok(Err(e)) => {
                    log::warn!("[{}] 代理服务器任务异常终止: {e}", log_srv::TASK_ERROR);
                    Err(ProxyError::StopFailed(e.to_string()))
                }
                Err(_) => {
                    log::warn!(
                        "[{}] 代理服务器停止超时（5秒），强制继续",
                        log_srv::STOP_TIMEOUT
                    );
                    Err(ProxyError::StopTimeout)
                }
            }
        } else {
            Ok(())
        }
    }

    pub async fn get_status(&self) -> ProxyStatus {
        let mut status = self.state.status.read().await.clone();

        // 计算运行时间
        if let Some(start) = *self.state.start_time.read().await {
            status.uptime_seconds = start.elapsed().as_secs();
        }

        // 从 current_providers HashMap 获取每个应用类型当前正在使用的 provider
        let current_providers = self.state.current_providers.read().await;
        status.active_targets = current_providers
            .iter()
            .map(|(app_type, (provider_id, provider_name))| ActiveTarget {
                app_type: app_type.clone(),
                provider_id: provider_id.clone(),
                provider_name: provider_name.clone(),
            })
            .collect();

        status
    }

    /// Non-blocking listener evidence for synchronous account observations.
    /// A contended state is unknown, never a guessed running value. This does
    /// not include the separately owned presentation-only active-target map.
    pub(crate) fn try_listener_status(&self) -> Option<ProxyStatus> {
        self.state
            .status
            .try_read()
            .ok()
            .map(|status| status.clone())
    }

    /// 更新某个应用类型当前“目标供应商”（用于 UI 展示 active_targets）
    ///
    /// 注意：这不代表该供应商一定已经处理过请求，而是用于“热切换/启用故障转移立即切 P1”
    /// 等场景下，让 UI 能立刻反映最新目标。
    pub async fn set_active_target(&self, app_type: &str, provider_id: &str, provider_name: &str) {
        let mut current_providers = self.state.current_providers.write().await;
        current_providers.insert(
            app_type.to_string(),
            (provider_id.to_string(), provider_name.to_string()),
        );
    }

    fn build_router(&self, body_limit: usize) -> Router {
        Router::new()
            // 健康检查
            .route("/health", get(handlers::health_check))
            .route("/status", get(handlers::get_status))
            // Claude API (支持带前缀和不带前缀两种格式)
            .route("/v1/messages", post(handlers::handle_messages))
            .route("/claude/v1/messages", post(handlers::handle_messages))
            // Claude Desktop 3P 本地 gateway（独立 provider namespace）
            .route(
                "/claude-desktop/v1/models",
                get(handlers::handle_claude_desktop_models),
            )
            .route(
                "/claude-desktop/v1/messages",
                post(handlers::handle_claude_desktop_messages),
            )
            // OpenAI Chat Completions API (Codex CLI，支持带前缀和不带前缀)
            .route("/chat/completions", post(handlers::handle_chat_completions))
            .route(
                "/v1/chat/completions",
                post(handlers::handle_chat_completions),
            )
            .route(
                "/v1/v1/chat/completions",
                post(handlers::handle_chat_completions),
            )
            .route(
                "/codex/v1/chat/completions",
                post(handlers::handle_chat_completions),
            )
            // OpenAI Models API (Codex CLI reachability check)
            .route("/models", get(handlers::handle_models))
            .route("/v1/models", get(handlers::handle_models))
            // OpenAI Responses API (Codex CLI，支持带前缀和不带前缀)。GET 是 WebSocket
            // 握手：Codex 内置的 openai（代理的官方路由）会先试 WebSocket。
            .route(
                "/responses",
                post(handlers::handle_responses).get(handlers::handle_responses_websocket),
            )
            .route(
                "/v1/responses",
                post(handlers::handle_responses).get(handlers::handle_responses_websocket),
            )
            .route(
                "/v1/v1/responses",
                post(handlers::handle_responses).get(handlers::handle_responses_websocket),
            )
            .route(
                "/codex/v1/responses",
                post(handlers::handle_responses).get(handlers::handle_responses_websocket),
            )
            // Grok Build uses the Responses protocol but has an independent
            // provider namespace and failover queue.
            .route(
                "/grokbuild/v1/responses",
                post(handlers::handle_grokbuild_responses),
            )
            .route(
                "/opencode/v1/responses",
                post(handlers::handle_opencode_responses),
            )
            // OpenAI Responses Compact API (Codex CLI 远程压缩，透传)
            .route(
                "/responses/compact",
                post(handlers::handle_responses_compact),
            )
            .route(
                "/v1/responses/compact",
                post(handlers::handle_responses_compact),
            )
            .route(
                "/v1/v1/responses/compact",
                post(handlers::handle_responses_compact),
            )
            .route(
                "/codex/v1/responses/compact",
                post(handlers::handle_responses_compact),
            )
            .route(
                "/grokbuild/v1/responses/compact",
                post(handlers::handle_grokbuild_responses_compact),
            )
            // Codex standalone Alpha Search API. All local aliases normalize to
            // the selected provider's canonical sibling `/alpha/search` route.
            .route("/alpha/search", post(handlers::handle_alpha_search))
            .route("/v1/alpha/search", post(handlers::handle_alpha_search))
            .route("/v1/v1/alpha/search", post(handlers::handle_alpha_search))
            .route(
                "/codex/v1/alpha/search",
                post(handlers::handle_alpha_search),
            )
            // Codex built-in ImageGen still calls the legacy OpenAI Images API.
            .route(
                "/images/generations",
                post(handlers::handle_images_generations),
            )
            .route(
                "/v1/images/generations",
                post(handlers::handle_images_generations),
            )
            .route(
                "/v1/v1/images/generations",
                post(handlers::handle_images_generations),
            )
            .route(
                "/codex/v1/images/generations",
                post(handlers::handle_images_generations),
            )
            // Codex ImageGen posts to `/images/edits` when it references existing images.
            .route("/images/edits", post(handlers::handle_images_edits))
            .route("/v1/images/edits", post(handlers::handle_images_edits))
            .route("/v1/v1/images/edits", post(handlers::handle_images_edits))
            .route(
                "/codex/v1/images/edits",
                post(handlers::handle_images_edits),
            )
            // Gemini API (支持带前缀和不带前缀)
            //
            // 用 `any(..)` 覆盖所有 HTTP 方法：除了 POST `:generateContent` /
            // `:streamGenerateContent` / `:countTokens` 之外，Gemini SDK / CLI 还会发
            // GET `/models`、GET `/models/<id>` 等只读端点。如果只挂 POST，这些 GET
            // 请求会在路由层 404，绕过本地代理的统计、整流和故障转移。
            .route("/v1beta/*path", any(handlers::handle_gemini))
            .route("/gemini/v1beta/*path", any(handlers::handle_gemini))
            // Gemini 的 GA 版本也叫 /v1，给原 SDK 留一条出口
            .route("/gemini/v1/*path", any(handlers::handle_gemini))
            // 手动读取入口通过 RequestExt::with_limited_body 执行此 200 MiB 限额
            .layer(DefaultBodyLimit::max(body_limit))
            .layer(axum::middleware::from_fn_with_state(
                self.state.clone(),
                authenticate_lan_request,
            ))
            .with_state(self.state.clone())
    }

    /// 在不重启服务的情况下更新运行时配置
    pub async fn apply_runtime_config(&self, config: &ProxyConfig) {
        *self.state.config.write().await = config.clone();
    }

    /// 热更新熔断器配置
    ///
    /// 将新配置应用到所有已创建的熔断器实例
    pub async fn update_circuit_breaker_configs(
        &self,
        config: super::circuit_breaker::CircuitBreakerConfig,
    ) {
        self.state.provider_router.update_all_configs(config).await;
    }

    pub async fn update_circuit_breaker_config_for_app(
        &self,
        app_type: &str,
        config: super::circuit_breaker::CircuitBreakerConfig,
    ) {
        self.state
            .provider_router
            .update_app_configs(app_type, config)
            .await;
    }

    /// 重置指定 Provider 的熔断器
    pub async fn reset_provider_circuit_breaker(&self, provider_id: &str, app_type: &str) {
        self.state
            .provider_router
            .reset_provider_breaker(provider_id, app_type)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{Provider, ProviderMeta};
    use crate::AppError;
    use axum::http::{header, HeaderMap, StatusCode};
    use serde_json::{json, Value};
    use tokio::sync::Mutex;

    #[test]
    fn listener_policy_accepts_ip_literals() {
        for address in ["127.0.0.1", "::1", "0.0.0.0", "::", "192.168.1.2"] {
            assert!(validate_listen_address(address).is_ok());
        }
        assert!(validate_listen_address("localhost").is_err());
    }

    #[tokio::test]
    async fn saved_non_loopback_config_without_key_cannot_start_listener() {
        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_address: "0.0.0.0".into(),
                listen_port: 0,
                ..Default::default()
            },
            Arc::new(Database::memory().unwrap()),
            None,
        );
        assert!(proxy
            .start()
            .await
            .unwrap_err()
            .to_string()
            .contains("访问密钥"));
        assert!(!proxy.state.status.read().await.running);
        assert!(proxy.shutdown_tx.read().await.is_none());
    }

    async fn lan_test_request(
        proxy: &ProxyServer,
        path: &str,
        peer: &str,
        auth: Option<(&str, &str)>,
    ) -> axum::http::StatusCode {
        let mut request =
            axum::http::Request::builder()
                .uri(path)
                .extension(axum::extract::ConnectInfo(
                    peer.parse::<SocketAddr>().unwrap(),
                ));
        if let Some((name, value)) = auth {
            request = request.header(name, value);
        }
        let request = request.body(axum::body::Body::empty()).unwrap();
        let mut app = proxy.build_router(32);
        use tower::Service;
        app.call(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn lan_routes_require_key_and_rotation_revokes_old_key() {
        let db = Arc::new(Database::memory().unwrap());
        let old = db.regenerate_proxy_access_key().unwrap();
        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_address: "0.0.0.0".into(),
                ..Default::default()
            },
            db.clone(),
            None,
        );
        for path in [
            "/health",
            "/status",
            "/v1/messages",
            "/v1/models",
            "/v1/responses",
            "/v1/chat/completions",
            "/v1/responses/compact",
            "/v1/images/generations",
            "/v1/images/edits",
            "/v1/alpha/search",
            "/grokbuild/v1/responses",
            "/opencode/v1/responses",
            "/claude-desktop/v1/models",
            "/v1beta/models",
            "/gemini/v1/models",
        ] {
            for auth in [
                None,
                Some(("x-api-key", "wrong")),
                Some(("authorization", "Bearer wrong")),
            ] {
                assert_eq!(
                    lan_test_request(&proxy, path, "192.168.1.2:1234", auth).await,
                    StatusCode::UNAUTHORIZED,
                    "{path}"
                );
            }
        }
        for (name, value) in [
            ("x-api-key", old.clone()),
            ("authorization", format!("Bearer {old}")),
        ] {
            assert_eq!(
                lan_test_request(&proxy, "/health", "192.168.1.2:1234", Some((name, &value))).await,
                StatusCode::OK
            );
        }
        for peer in ["127.0.0.1:1234", "[::1]:1234", "[::ffff:127.0.0.1]:1234"] {
            assert_eq!(
                lan_test_request(&proxy, "/health", peer, None).await,
                StatusCode::OK
            );
        }
        let new = db.regenerate_proxy_access_key().unwrap();
        proxy.update_access_key(new.clone()).await;
        assert_ne!(new, old);
        assert_eq!(
            lan_test_request(
                &proxy,
                "/health",
                "192.168.1.2:1234",
                Some(("x-api-key", &old))
            )
            .await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            lan_test_request(
                &proxy,
                "/health",
                "192.168.1.2:1234",
                Some(("x-api-key", &new))
            )
            .await,
            StatusCode::OK
        );
        db.set_setting("proxy_access_key", "").unwrap();
        proxy.update_access_key(String::new()).await;
        assert_eq!(
            lan_test_request(&proxy, "/health", "192.168.1.2:1234", None).await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            lan_test_request(&proxy, "/health", "127.0.0.1:1234", None).await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn lan_key_cache_does_not_read_database_per_request() {
        let db = Arc::new(Database::memory().unwrap());
        let key = db.regenerate_proxy_access_key().unwrap();
        let proxy = ProxyServer::new(ProxyConfig::default(), db.clone(), None);
        db.conn
            .lock()
            .unwrap()
            .execute_batch("DROP TABLE settings;")
            .unwrap();
        assert_eq!(
            lan_test_request(
                &proxy,
                "/health",
                "192.168.1.2:1234",
                Some(("x-api-key", &key))
            )
            .await,
            StatusCode::OK
        );
        let failed = ProxyServer::new(ProxyConfig::default(), db, None);
        for peer in ["192.168.1.2:1234", "127.0.0.1:1234"] {
            let request = axum::http::Request::builder()
                .uri("/health")
                .extension(axum::extract::ConnectInfo(
                    peer.parse::<SocketAddr>().unwrap(),
                ))
                .body(axum::body::Body::empty())
                .unwrap();
            use tower::Service;
            let response = failed.build_router(32).call(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let body = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), "局域网代理访问密钥读取失败".as_bytes());
        }
    }

    struct AuthTestLogger(std::sync::Mutex<Option<(std::thread::ThreadId, Vec<String>)>>);
    static AUTH_TEST_LOGGER: AuthTestLogger = AuthTestLogger(std::sync::Mutex::new(None));
    static INIT_LOGGER: std::sync::Once = std::sync::Once::new();

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn copilot_migration_warning_hides_exchange_and_refresh_details() {
        use super::super::providers::copilot_auth::{log_copilot_auth_warning, CopilotAuthError};

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const SECRET: &str = "fake-copilot-key/+=private";
        let detail = format!("500 Internal Server Error: Authorization: Bearer {SECRET}");
        let exchange_error = CopilotAuthError::CopilotTokenFetchFailed(detail.clone());
        let original = format!("Copilot 令牌获取失败: {detail}");
        assert_eq!(exchange_error.to_string(), original);
        // The legacy migration calls this same warning function before AuthError wrapping.
        log_copilot_auth_warning("迁移时验证 Copilot 订阅失败", &exchange_error);
        assert_eq!(exchange_error.to_string(), original);
        for error in [
            CopilotAuthError::NetworkError(detail.clone()),
            CopilotAuthError::ParseError(detail.clone()),
            CopilotAuthError::IoError(detail),
        ] {
            log_copilot_auth_warning("加载存储失败", &error);
            log_copilot_auth_warning("迁移失败，旧 token 可能已失效", &error);
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(logs.len(), 7);
        assert_eq!(
            logs[0],
            "[CopilotAuth] 迁移时验证 Copilot 订阅失败: Copilot token exchange failed"
        );
        for (index, category) in ["network error", "parse error", "IO error"]
            .iter()
            .enumerate()
        {
            assert_eq!(
                logs[1 + index * 2],
                format!("[CopilotAuth] 加载存储失败: {category}")
            );
            assert_eq!(
                logs[2 + index * 2],
                format!("[CopilotAuth] 迁移失败，旧 token 可能已失效: {category}")
            );
        }
        assert!(logs
            .iter()
            .all(|line| !line.contains(SECRET) && !line.contains("Authorization")));
    }

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn conversion_logs_hide_usage_and_unknown_finish_reason_details() {
        use super::super::providers::{
            transform::openai_to_anthropic,
            transform_responses::build_anthropic_usage_from_responses,
        };

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const SECRET: &str = "fake-key";
        // This usage builder is shared by non-streaming and streaming Responses conversion.
        for (input, output) in [(1, 0), (0, 2)] {
            let usage = json!({
                "input_tokens": input,
                "output_tokens": output,
                "Authorization": format!("Bearer {SECRET}")
            });
            assert_eq!(
                build_anthropic_usage_from_responses(Some(&usage)),
                json!({"input_tokens": input, "output_tokens": output})
            );
        }
        let response = json!({
            "id": "response-test", "model": "test-model",
            "choices": [{
                "message": {"content": "unchanged answer"},
                "finish_reason": format!("Authorization: Bearer {SECRET}")
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2}
        });
        assert_eq!(
            openai_to_anthropic(response).unwrap(),
            json!({
                "id": "response-test", "type": "message", "role": "assistant",
                "content": [{"type": "text", "text": "unchanged answer"}],
                "model": "test-model", "stop_reason": "end_turn", "stop_sequence": null,
                "usage": {"input_tokens": 3, "output_tokens": 2}
            })
        );
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(
            logs,
            vec![
                "[Responses] Partial usage object: input_tokens=1, output_tokens=0",
                "[Responses] Partial usage object: input_tokens=0, output_tokens=2",
                "[Claude/OpenAI] Unknown finish_reason in non-streaming response: unknown",
            ]
        );
        assert!(logs
            .iter()
            .all(|line| !line.contains(SECRET) && !line.contains("Authorization")));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn upstream_finish_reason_logs_hide_credentials_and_preserve_output() {
        use super::super::providers::{
            streaming::create_anthropic_sse_stream, transform_gemini::gemini_to_anthropic,
        };
        use bytes::Bytes;
        use futures::StreamExt;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        let input = format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({
                "id":"message-test", "model":DETAIL,
                "choices":[{"delta":{"content":"unchanged answer"}, "finish_reason":DETAIL}],
                "usage":{"prompt_tokens":3,"completion_tokens":2}
            })
        );
        let upstream = futures::stream::iter(vec![Ok::<_, std::io::Error>(Bytes::from(input))]);
        let converted = create_anthropic_sse_stream(upstream)
            .collect::<Vec<_>>()
            .await;
        let events: Vec<Value> = converted
            .iter()
            .flat_map(|chunk| {
                String::from_utf8(chunk.as_ref().unwrap().to_vec())
                    .unwrap()
                    .lines()
                    .filter_map(|line| line.strip_prefix("data: "))
                    .map(|data| serde_json::from_str(data).unwrap())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(events
            .iter()
            .any(|event| event.pointer("/message/model") == Some(&json!(DETAIL))));
        assert!(events
            .iter()
            .any(|event| event.pointer("/delta/text") == Some(&json!("unchanged answer"))));
        assert!(events
            .iter()
            .any(
                |event| event.pointer("/delta/stop_reason") == Some(&json!("end_turn"))
                    && event["usage"]["output_tokens"] == 2
            ));
        assert_eq!(events.last().unwrap(), &json!({"type":"message_stop"}));
        assert_eq!(gemini_to_anthropic(json!({
                "responseId":"message-test", "modelVersion":DETAIL,
                "candidates":[{"content":{"parts":[{"text":"unchanged answer"}]},"finishReason":DETAIL}],
                "usageMetadata":{"promptTokenCount":3,"totalTokenCount":5}
            })).unwrap(), json!({
                "id":"message-test","type":"message","role":"assistant",
                "content":[{"type":"text","text":"unchanged answer"}], "model":DETAIL,
                "stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":2}
            }));
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert!(logs
            .iter()
            .any(|line| line == "[Claude/OpenRouter] Unknown finish_reason in streaming: unknown"));
        assert!(logs
            .iter()
            .any(|line| line
                == "[Claude/Gemini] Unknown Gemini finishReason unknown, using end_turn"));
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn usage_parser_logs_hide_upstream_json_and_preserve_tokens() {
        use super::super::usage::parser::TokenUsage;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        assert!(TokenUsage::from_codex_response(&json!({DETAIL:DETAIL})).is_none());
        assert!(TokenUsage::from_codex_response(
            &json!({"usage":{"input_tokens":1,"Authorization":DETAIL}})
        )
        .is_none());
        assert!(
            TokenUsage::from_codex_response_auto(&json!({"usage":{"Authorization":DETAIL}}))
                .is_none()
        );
        let codex = TokenUsage::from_codex_response_auto(&json!({
            "id":DETAIL,"model":DETAIL,"usage":{"input_tokens":3,"output_tokens":2,
            "input_tokens_details":{"cached_tokens":1,"cache_write_tokens":2},"Authorization":DETAIL}
        }))
        .unwrap();
        let events = [json!({"id":DETAIL,"model":DETAIL,"usage":{
            "prompt_tokens":3,"completion_tokens":2,
            "prompt_tokens_details":{"cached_tokens":1,"cache_write_tokens":2},"Authorization":DETAIL
        }})];
        let openai = TokenUsage::from_openai_stream_events(&events).unwrap();
        for usage in [codex, openai] {
            assert_eq!(usage.message_id.as_deref(), Some(DETAIL));
            assert_eq!(
                serde_json::to_value(usage).unwrap(),
                json!({
                    "input_tokens":3,"output_tokens":2,"cache_read_tokens":1,
                    "cache_creation_tokens":2,"model":DETAIL
                })
            );
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(
            logs,
            vec![
                "[Codex] 响应中没有 usage 字段",
                "[Codex] usage 字段缺少 input_tokens 或 output_tokens",
                "[Codex] 无法识别响应格式",
                "[Codex] 检测到 Codex 格式 (input_tokens)",
                "[Codex] 解析 OpenAI 流式事件，共 1 个事件",
                "[Codex] 找到非空 usage 字段",
            ]
        );
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn responses_event_logs_use_allowlist_and_preserve_stream() {
        use super::super::providers::streaming_responses::create_anthropic_sse_stream_from_responses;
        use bytes::Bytes;
        use futures::StreamExt;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        let created = format!(
            "data: {}\n\n",
            json!({"type":"response.created","response":{"id":"message-test","model":"test-model"}})
        );
        let tail = format!(
            "data: {}\n\ndata: {}\n\n",
            json!({
                "type":"response.output_text.delta","delta":"unchanged answer","output_index":0,"content_index":0
            }),
            json!({"type":"response.completed","response":{
                "id":"message-test","model":"test-model","status":"completed",
                "output":[{"type":"message","content":[{"type":"output_text","text":"unchanged answer"}]}],
                "usage":{"input_tokens":3,"output_tokens":2}
            }})
        );
        let baseline = format!("{created}{tail}");
        let malicious = format!(
            "{created}event: {DETAIL}\ndata: {{}}\n\ndata: {}\n\n{tail}",
            json!({"type":DETAIL})
        );
        let mut outputs = Vec::new();
        for input in [baseline, malicious] {
            let stream = futures::stream::iter(vec![Ok::<_, std::io::Error>(Bytes::from(input))]);
            let chunks = create_anthropic_sse_stream_from_responses(stream)
                .collect::<Vec<_>>()
                .await;
            let output: Vec<u8> = chunks
                .into_iter()
                .flat_map(|chunk| chunk.unwrap().to_vec())
                .collect();
            outputs.push(String::from_utf8(output).unwrap());
        }
        assert_eq!(outputs[0], outputs[1]);
        assert!(outputs[0].contains("unchanged answer"));
        assert!(outputs[0].contains("message_stop"));
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(
            logs.iter()
                .filter(|line| line.as_str() == "[Claude/Responses] <<< SSE event: unknown")
                .count(),
            2
        );
        assert!(logs
            .iter()
            .any(|line| line == "[Claude/Responses] <<< SSE event: response.created"));
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn incomplete_tool_logs_hide_names_and_preserve_conversion() {
        use super::super::providers::transform_responses::responses_to_anthropic;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        for arguments in ["[", "[]"] {
            assert_eq!(responses_to_anthropic(json!({
                    "id":"message-test","model":DETAIL,"status":"incomplete",
                    "output":[{"type":"function_call","call_id":DETAIL,"name":DETAIL,"arguments":arguments}],
                    "usage":{"input_tokens":3,"output_tokens":2}
                })).unwrap(), json!({
                    "id":"message-test","type":"message","role":"assistant","model":DETAIL,
                    "content":[{"type":"tool_use","id":DETAIL,"name":DETAIL,"input":{}}],
                    "stop_reason":"max_tokens","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":2}
                }));
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(logs, vec![
                "[Responses] Replacing incomplete function_call arguments with an empty object: invalid JSON",
                "[Responses] Replacing incomplete function_call non-object arguments with an empty object",
            ]);
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn stream_error_logs_hide_details_and_preserve_client_errors() {
        use super::super::providers::{
            streaming::create_anthropic_sse_stream,
            streaming_responses::create_anthropic_sse_stream_from_responses,
        };
        use bytes::Bytes;
        use futures::StreamExt;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        for responses in [false, true] {
            let stream =
                futures::stream::iter(vec![Err::<Bytes, _>(std::io::Error::other(DETAIL))]);
            let chunks = if responses {
                create_anthropic_sse_stream_from_responses(stream)
                    .collect::<Vec<_>>()
                    .await
            } else {
                create_anthropic_sse_stream(stream)
                    .collect::<Vec<_>>()
                    .await
            };
            let output: Vec<u8> = chunks
                .into_iter()
                .flat_map(|chunk| chunk.unwrap().to_vec())
                .collect();
            let events: Vec<Value> = String::from_utf8(output)
                .unwrap()
                .lines()
                .filter_map(|line| line.strip_prefix("data: "))
                .map(|data| serde_json::from_str(data).unwrap())
                .collect();
            assert!(events.iter().any(|event| event["type"] == "error"
                && event["error"]["message"] == format!("Stream error: {DETAIL}")));
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert!(logs
            .iter()
            .any(|line| line == "Stream error: upstream stream failed"));
        assert!(logs
            .iter()
            .any(|line| line == "Responses stream error: upstream stream failed"));
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn private_key_and_encoding_logs_hide_credentials_and_preserve_results() {
        use super::super::{
            body_filter::filter_private_params_with_whitelist,
            content_encoding::decompress_body_with_limit,
        };

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        let private_key = format!("_{DETAIL}");
        assert_eq!(
            filter_private_params_with_whitelist(
                json!({private_key:DETAIL,"answer":"unchanged"}),
                &[]
            ),
            json!({"answer":"unchanged"})
        );
        assert!(decompress_body_with_limit(DETAIL, b"unchanged body", 1024)
            .unwrap()
            .is_none());
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(
            logs,
            vec![
                "[BodyFilter] 过滤私有参数: count=1",
                "不支持的 content-encoding: unknown，跳过解压",
            ]
        );
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }

    #[test]
    #[serial_test::serial(proxy_log_capture)]
    fn gemini_rectification_logs_hide_tool_names_and_preserve_arguments() {
        use super::super::providers::transform_gemini::{
            extract_anthropic_tool_schema_hints, rectify_tool_call_parts,
        };

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const DETAIL: &str = "Authorization: Bearer fake-key";
        let hints = extract_anthropic_tool_schema_hints(&json!({"tools":[{
            "name":DETAIL,"input_schema":{"type":"object","properties":{"skill":{"type":"string"}},"required":["skill"]}
        }]}));
        let mut parts = vec![json!({"functionCall":{"name":DETAIL,"args":{"name":DETAIL}}})];
        rectify_tool_call_parts(&mut parts, Some(&hints));
        assert_eq!(
            parts,
            vec![json!({"functionCall":{"name":DETAIL,"args":{"skill":DETAIL}}})]
        );
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(logs, vec!["[Claude/Gemini] Rectified tool args"]);
        assert!(logs
            .iter()
            .all(|line| !line.contains("Authorization") && !line.contains("fake-key")));
    }
    impl log::Log for AuthTestLogger {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }
        fn log(&self, record: &log::Record<'_>) {
            if let Some((thread, records)) = self.0.lock().unwrap().as_mut() {
                if *thread == std::thread::current().id() {
                    records.push(record.args().to_string());
                }
            }
        }
        fn flush(&self) {}
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn copilot_token_exchange_failure_logs_and_usage_hide_secrets() {
        use super::super::{
            error_mapper::{get_error_message, get_log_error_message, map_proxy_error_to_status},
            forwarder::log_retryable_failure,
            providers::copilot_auth::CopilotAuthError,
            usage::logger::UsageLogger,
        };
        use axum::response::IntoResponse;

        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        const SECRET: &str = "fake-copilot-key/+=private";
        const SUMMARY: &str = "认证失败: authentication failed";
        // Reproduce the HTTP 500 token-exchange error and the legacy auth wrapper.
        // The exchange endpoint is fixed HTTPS and the legacy path needs AppHandle.
        let exchange_error = CopilotAuthError::CopilotTokenFetchFailed(format!(
            "{}: Authorization: Bearer {SECRET}",
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        let detail = format!("GitHub Copilot 认证失败: {exchange_error}");
        let error = ProxyError::AuthError(detail.clone());
        assert_eq!(get_error_message(&error), format!("认证失败: {detail}"));
        assert_eq!(get_log_error_message(&error), SUMMARY);
        for total_providers in [1, 2] {
            // Exercise the same warning emission used by forward_with_retry.
            log_retryable_failure("claude", "Copilot", 1, total_providers, &error);
        }

        let db = Database::memory().unwrap();
        UsageLogger::new(&db)
            .log_error_with_context(
                "copilot-auth-failure".into(),
                "copilot".into(),
                "claude".into(),
                "test".into(),
                map_proxy_error_to_status(&error),
                get_log_error_message(&error),
                0,
                false,
                None,
                None,
            )
            .unwrap();
        let (status, stored): (u16, String) = db.conn.lock().unwrap().query_row(
            "SELECT status_code, error_message FROM proxy_request_logs WHERE request_id = 'copilot-auth-failure'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(status, 401);
        assert_eq!(stored, SUMMARY);
        assert!(!stored.contains(SECRET));

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let client: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            client,
            json!({"error": {
                "type": "proxy_error", "message": format!("认证失败: {detail}")
            }})
        );
        assert!(String::from_utf8_lossy(&bytes).contains(SECRET));
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(logs.iter().filter(|line| line.contains(SUMMARY)).count(), 2);
        assert!(logs.iter().any(
            |line| line == &format!("[claude] [FWD-003] Provider Copilot 请求失败: {SUMMARY}")
        ));
        assert!(logs.iter().any(|line| line
            == &format!(
                "[claude] [FWD-001] Provider Copilot 失败，继续尝试下一个 (1/2): {SUMMARY}"
            )));
        assert!(logs
            .iter()
            .all(|line| !line.contains(SECRET) && !line.contains("Authorization")));
    }

    #[tokio::test]
    async fn lan_listener_accepts_key_config_and_loopback_without_key() {
        let db = Arc::new(Database::memory().unwrap());
        db.regenerate_proxy_access_key().unwrap();
        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_address: "0.0.0.0".into(),
                listen_port: 0,
                ..Default::default()
            },
            db,
            None,
        );
        let info = proxy.start().await.unwrap();
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://127.0.0.1:{}/health", info.port))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        proxy.stop().await.unwrap();
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn lan_auth_strips_access_keys_before_handler_logging() {
        async fn inspect(request: axum::extract::Request) -> StatusCode {
            assert!(!request.headers().contains_key("authorization"));
            assert!(!request.headers().contains_key("x-api-key"));
            log::debug!("LAN handler headers: count={}", request.headers().len());
            StatusCode::OK
        }
        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        let db = Arc::new(Database::memory().unwrap());
        let key = db.regenerate_proxy_access_key().unwrap();
        let state = ProxyState::for_test(db);
        let mut app =
            Router::new()
                .route("/", get(inspect))
                .layer(axum::middleware::from_fn_with_state(
                    state,
                    authenticate_lan_request,
                ));
        let request = axum::http::Request::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(
                "192.168.1.2:1234".parse::<SocketAddr>().unwrap(),
            ))
            .header("authorization", format!("Bearer {key}"))
            .header("x-api-key", &key)
            .body(axum::body::Body::empty())
            .unwrap();
        use tower::Service;
        assert_eq!(app.call(request).await.unwrap().status(), StatusCode::OK);
        let request = axum::http::Request::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(
                "192.168.1.2:1234".parse::<SocketAddr>().unwrap(),
            ))
            .header("authorization", format!("Bearer wrong-{key}"))
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&body).contains(&key));
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert!(!logs.is_empty());
        assert!(logs.iter().all(|line| !line.contains(&key)));
    }

    #[tokio::test]
    async fn loopback_strips_only_matching_lan_credentials() {
        async fn inspect(request: axum::extract::Request) -> axum::Json<Value> {
            axum::Json(json!({
                "authorization": request.headers().get(header::AUTHORIZATION).map(|v| v.to_str().unwrap()),
                "x-api-key": request.headers().get("x-api-key").map(|v| v.to_str().unwrap()),
            }))
        }
        let db = Arc::new(Database::memory().unwrap());
        let key = db.regenerate_proxy_access_key().unwrap();
        let state = ProxyState::for_test(db.clone());
        let access_key = state.access_key.clone();
        let mut app =
            Router::new()
                .route("/", get(inspect))
                .layer(axum::middleware::from_fn_with_state(
                    state,
                    authenticate_lan_request,
                ));
        use tower::Service;
        for peer in ["127.0.0.1:1234", "[::1]:1234", "[::ffff:127.0.0.1]:1234"] {
            for (authorization, api_key, expected_auth, expected_api_key) in [
                (format!("Bearer {key}"), key.clone(), None, None),
                (
                    format!("Bearer {key}"),
                    "business-api-key".into(),
                    None,
                    Some("business-api-key"),
                ),
                (
                    "Bearer business-token".into(),
                    key.clone(),
                    Some("Bearer business-token"),
                    None,
                ),
                (
                    "Bearer business-token".into(),
                    "business-api-key".into(),
                    Some("Bearer business-token"),
                    Some("business-api-key"),
                ),
            ] {
                let request = axum::http::Request::builder()
                    .uri("/")
                    .extension(axum::extract::ConnectInfo(
                        peer.parse::<SocketAddr>().unwrap(),
                    ))
                    .header(header::AUTHORIZATION, authorization)
                    .header("x-api-key", api_key)
                    .body(axum::body::Body::empty())
                    .unwrap();
                let response = app.call(request).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                let bytes = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                let headers: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(headers["authorization"], json!(expected_auth));
                assert_eq!(headers["x-api-key"], json!(expected_api_key));
            }
        }
        for (auth, api, expected_auth, expected_api) in [
            (
                format!("Bearer {key}"),
                "business-api-key".to_string(),
                None,
                Some("business-api-key"),
            ),
            (
                "Bearer business-token".to_string(),
                key.clone(),
                Some("Bearer business-token"),
                None,
            ),
        ] {
            let request = axum::http::Request::builder()
                .uri("/")
                .extension(axum::extract::ConnectInfo(
                    "192.168.1.2:1234".parse::<SocketAddr>().unwrap(),
                ))
                .header(header::AUTHORIZATION, auth)
                .header("x-api-key", api)
                .body(axum::body::Body::empty())
                .unwrap();
            let response = app.call(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            let headers: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(headers["authorization"], json!(expected_auth));
            assert_eq!(headers["x-api-key"], json!(expected_api));
        }
        db.set_setting("proxy_access_key", "").unwrap();
        *access_key.write().await = Ok(None);
        let request = axum::http::Request::builder()
            .uri("/")
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
            ))
            .header(header::AUTHORIZATION, "Bearer business-token")
            .header("x-api-key", "business-api-key")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({
                "authorization": "Bearer business-token", "x-api-key": "business-api-key"
            })
        );
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn responses_sse_failure_does_not_leak_to_health_or_logs() {
        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        let _ = rustls::crypto::ring::default_provider().install_default();
        const SECRET: &str = "fake-sse-key/+=private";
        for (payload, media_retry) in [
            (
                json!({"type": "response.failed", "response": {"status": "failed", "error": {
                    "type": SECRET, "message": format!("Authorization: Bearer {SECRET} {}", "x".repeat(4096))
                }}}),
                false,
            ),
            (
                json!({"type": "error", "code": SECRET, "message": SECRET}),
                false,
            ),
            (
                json!({"type": "response.failed", "response": {"status": "cancelled", "error": SECRET}}),
                false,
            ),
            (
                json!({"type": "error", "code": SECRET, "message": SECRET}),
                true,
            ),
        ] {
            let sse = format!(
                "event: {}\ndata: {payload}\n\n",
                payload["type"].as_str().unwrap()
            );
            let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let seen = attempts.clone();
            let mock = Router::new().route(
                "/v1/responses",
                post(move || {
                    let sse = sse.clone();
                    let attempt = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if media_retry && attempt == 0 {
                            return (
                                StatusCode::BAD_REQUEST,
                                [(header::CONTENT_TYPE, "application/json")],
                                json!({"error": {"message": "unsupported image input"}})
                                    .to_string(),
                            );
                        }
                        (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "text/event-stream")],
                            sse,
                        )
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .unwrap();
            let addr = listener.local_addr().unwrap();
            let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
            let db = Arc::new(Database::memory().unwrap());
            let mut provider = Provider::with_id(
                "sse-safe-error-test".into(),
                "Test".into(),
                json!({
                    "env": {"ANTHROPIC_BASE_URL": format!("http://{addr}"), "ANTHROPIC_API_KEY": "upstream-key"}
                }),
                None,
            );
            provider.meta = Some(ProviderMeta {
                api_format: Some("openai_responses".into()),
                ..Default::default()
            });
            db.save_provider("claude", &provider).unwrap();
            db.set_current_provider("claude", &provider.id).unwrap();
            let proxy = ProxyServer::new(ProxyConfig::default(), db.clone(), None);
            let mut app = proxy.build_router(4096);
            let content = if media_retry {
                json!([
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AA=="}},
                    {"type": "text", "text": "test"}
                ])
            } else {
                json!("test")
            };
            let request = axum::http::Request::builder()
                .method("POST")
                .uri("/v1/messages")
                .extension(axum::extract::ConnectInfo(
                    "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({"model": "test", "max_tokens": 16, "stream": true,
                    "messages": [{"role": "user", "content": content}]})
                    .to_string(),
                ))
                .unwrap();
            use tower::Service;
            let response = app.call(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let health = db
                .get_provider_health(&provider.id, "claude")
                .await
                .unwrap();
            assert_eq!(
                attempts.load(std::sync::atomic::Ordering::SeqCst),
                if media_retry { 2 } else { 1 }
            );
            if media_retry {
                assert_eq!(health.consecutive_failures, 0);
                assert!(health.last_error.is_none());
            } else {
                assert_eq!(health.consecutive_failures, 1);
                let error = health.last_error.unwrap();
                assert_eq!(error, "请求/响应转换错误: transformation failed");
                assert!(!error.contains(SECRET));
                assert!(error.chars().count() <= 256);
            }
            assert!(!proxy
                .state
                .status
                .read()
                .await
                .last_error
                .as_ref()
                .unwrap()
                .contains(SECRET));
            task.abort();
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert!(
            logs.iter()
                .filter(|line| line.contains("transformation failed"))
                .count()
                >= 3
        );
        assert!(logs.iter().any(|line| {
            line.contains(
            "[Media] Unsupported-image retry still failed: 请求/响应转换错误: transformation failed"
        )
        }));
        assert!(logs.iter().all(|line| !line.contains(SECRET)));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn non_streaming_sse_aggregation_failure_logs_hide_upstream_secrets() {
        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        let _ = rustls::crypto::ring::default_provider().install_default();
        const SECRET: &str = "fake-non-stream-sse-key/+=private";
        // The mislabeled SSE body reaches the stream:false aggregation fallback.
        let mock = Router::new().route(
            "/v1/responses",
            post(|axum::Json(body): axum::Json<Value>| async move {
                assert_eq!(body["stream"], false);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(
                        "event: response.failed\ndata: {}\n\n",
                        json!({"response": {"error": {"message": SECRET}}})
                    ),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
        let db = Arc::new(Database::memory().unwrap());
        let mut provider = Provider::with_id(
            "non-stream-sse".into(),
            "Test".into(),
            json!({
                "env": {"ANTHROPIC_BASE_URL": format!("http://{addr}"), "ANTHROPIC_API_KEY": "upstream-key"}
            }),
            None,
        );
        provider.meta = Some(ProviderMeta {
            api_format: Some("openai_responses".into()),
            ..Default::default()
        });
        db.save_provider("claude", &provider).unwrap();
        db.set_current_provider("claude", &provider.id).unwrap();
        let proxy = ProxyServer::new(ProxyConfig::default(), db, None);
        let mut app = proxy.build_router(4096);
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(
                json!({"model": "test", "max_tokens": 16, "stream": false,
                "messages": [{"role": "user", "content": "test"}]})
                .to_string(),
            ))
            .unwrap();
        use tower::Service;
        let response = app.call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains(SECRET));
        task.abort();
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert!(logs.iter().any(|line| line.contains(
            "[Claude] SSE 聚合兜底失败: 请求/响应转换错误: transformation failed, body_bytes="
        )));
        assert!(logs.iter().all(|line| !line.contains(SECRET)));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn non_streaming_sse_terminal_error_logs_hide_upstream_secrets() {
        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        let _ = rustls::crypto::ring::default_provider().install_default();
        const SECRET: &str = "fake-non-stream-terminal-sse-key/+=private";

        for status in ["incomplete", "completed"] {
            // Mislabeled HTTP 200 SSE reaches the non-stream aggregation fallback,
            // then fails in response conversion after retaining the terminal error.
            let mock = Router::new().route(
                "/v1/responses",
                post(move |axum::Json(body): axum::Json<Value>| async move {
                    assert_eq!(body["stream"], false);
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/json")],
                        format!(
                            "event: response.{status}\ndata: {}\n\n",
                            json!({"type": format!("response.{status}"), "response": {
                                "id": "resp_test", "status": status, "output": [],
                                "error": {"message": format!("{status}: Authorization: Bearer {SECRET}")}
                            }})
                        ),
                    )
                }),
            );
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .unwrap();
            let addr = listener.local_addr().unwrap();
            let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
            let db = Arc::new(Database::memory().unwrap());
            let mut provider = Provider::with_id(
                "non-stream-terminal-sse".into(),
                "Test".into(),
                json!({
                    "env": {"ANTHROPIC_BASE_URL": format!("http://{addr}"), "ANTHROPIC_API_KEY": "upstream-key"}
                }),
                None,
            );
            provider.meta = Some(ProviderMeta {
                api_format: Some("openai_responses".into()),
                ..Default::default()
            });
            db.save_provider("claude", &provider).unwrap();
            db.set_current_provider("claude", &provider.id).unwrap();
            let proxy = ProxyServer::new(ProxyConfig::default(), db, None);
            let mut app = proxy.build_router(4096);
            let request = axum::http::Request::builder()
                .method("POST")
                .uri("/v1/messages")
                .extension(axum::extract::ConnectInfo(
                    "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
                ))
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    json!({"model": "test", "max_tokens": 16, "stream": false,
                    "messages": [{"role": "user", "content": "test"}]})
                    .to_string(),
                ))
                .unwrap();
            use tower::Service;
            let response = app.call(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            let client: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                client,
                json!({"error": {"type": "proxy_error", "message": format!(
                    "格式转换错误: Responses upstream returned an error envelope: {status}: Authorization: Bearer {SECRET}"
                )}})
            );
            task.abort();
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        assert_eq!(
            logs.iter()
                .filter(|line| line
                    .contains("[Claude] 转换响应失败: 请求/响应转换错误: transformation failed"))
                .count(),
            2
        );
        assert!(logs.iter().all(|line| !line.contains(SECRET)));
    }

    #[tokio::test]
    #[serial_test::serial(proxy_log_capture)]
    async fn json_2xx_errors_retry_thinking_and_budget_without_leaking_to_logs_or_storage() {
        INIT_LOGGER.call_once(|| log::set_logger(&AUTH_TEST_LOGGER).unwrap());
        let previous_level = log::max_level();
        log::set_max_level(log::LevelFilter::Trace);
        *AUTH_TEST_LOGGER.0.lock().unwrap() = Some((std::thread::current().id(), Vec::new()));
        let _ = rustls::crypto::ring::default_provider().install_default();
        const SECRET: &str = "fake-json-retry-key/+=private";
        for budget in [false, true] {
            for retry_succeeds in [false, true] {
                let error_message = if budget {
                    "thinking.budget_tokens: Input should be greater than or equal to 1024"
                } else {
                    "messages.1.content.0: Invalid `signature` in `thinking` block"
                };
                let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let seen = attempts.clone();
                let mock = Router::new().route("/v1/responses", post(move || {
                    let attempt = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        let body = if retry_succeeds && attempt > 0 {
                            json!({"id": "resp_test", "object": "response", "status": "completed", "model": "test",
                                "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "ok"}]}],
                                "usage": {"input_tokens": 1, "output_tokens": 1}})
                        } else {
                            json!({"status": "failed", "error": {"type": SECRET, "code": SECRET,
                                "message": format!("{error_message}; Authorization: Bearer {SECRET}")}})
                        };
                        (StatusCode::OK, axum::Json(body))
                    }
                }));
                let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                    .await
                    .unwrap();
                let addr = listener.local_addr().unwrap();
                let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
                let db = Arc::new(Database::memory().unwrap());
                let mut provider = Provider::with_id(
                    "json-retry".into(),
                    "Test".into(),
                    json!({
                        "env": {"ANTHROPIC_BASE_URL": format!("http://{addr}"), "ANTHROPIC_API_KEY": "upstream-key"}
                    }),
                    None,
                );
                provider.meta = Some(ProviderMeta {
                    api_format: Some("openai_responses".into()),
                    ..Default::default()
                });
                db.save_provider("claude", &provider).unwrap();
                db.set_current_provider("claude", &provider.id).unwrap();
                let proxy = ProxyServer::new(ProxyConfig::default(), db.clone(), None);
                let mut app = proxy.build_router(4096);
                let mut body = json!({"model": "test", "max_tokens": 1000, "stream": false, "messages": [
                    {"role": "user", "content": "test"},
                    {"role": "assistant", "content": [{"type": "thinking", "thinking": "t", "signature": "sig"}, {"type": "text", "text": "a"}]},
                    {"role": "user", "content": "again"}
                ]});
                if budget {
                    body["thinking"] = json!({"type": "enabled", "budget_tokens": 500});
                }
                let request = axum::http::Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .extension(axum::extract::ConnectInfo(
                        "127.0.0.1:1234".parse::<SocketAddr>().unwrap(),
                    ))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(body.to_string()))
                    .unwrap();
                use tower::Service;
                let response = app.call(request).await.unwrap();
                assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
                assert_eq!(
                    response.status(),
                    if retry_succeeds {
                        StatusCode::OK
                    } else {
                        StatusCode::UNPROCESSABLE_ENTITY
                    }
                );
                let bytes = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                if !retry_succeeds {
                    let client: Value = serde_json::from_slice(&bytes).unwrap();
                    let message = client["error"]["message"].as_str().unwrap();
                    assert!(message.contains(error_message));
                    assert!(message.contains(SECRET));
                    {
                        let conn = db.conn.lock().unwrap();
                        let (status, stored): (u16, String) = conn
                            .query_row(
                                "SELECT status_code, error_message FROM proxy_request_logs",
                                [],
                                |row| Ok((row.get(0)?, row.get(1)?)),
                            )
                            .unwrap();
                        assert_eq!(status, 422);
                        assert_eq!(stored, "请求/响应转换错误: transformation failed");
                        assert!(!stored.contains(SECRET));
                    }
                    assert!(!proxy
                        .state
                        .status
                        .read()
                        .await
                        .last_error
                        .as_ref()
                        .unwrap()
                        .contains(SECRET));
                    let health = db
                        .get_provider_health(&provider.id, "claude")
                        .await
                        .unwrap();
                    assert_eq!(health.consecutive_failures, 0);
                    assert!(health.last_error.is_none());
                }
                task.abort();
            }
        }
        let (_, logs) = AUTH_TEST_LOGGER.0.lock().unwrap().take().unwrap();
        log::set_max_level(previous_level);
        for code in [
            "[RECT-001]",
            "[RECT-002]",
            "[RECT-003]",
            "[RECT-010]",
            "[RECT-011]",
            "[RECT-012]",
        ] {
            assert!(
                logs.iter().any(|line| line.contains(code)),
                "missing retry evidence: {code}"
            );
        }
        assert!(logs.iter().all(|line| !line.contains(SECRET)));
    }

    #[tokio::test]
    async fn manual_body_routes_reject_fixed_and_chunked_overflow_before_routing() {
        let proxy = ProxyServer::new(
            ProxyConfig::default(),
            Arc::new(Database::memory().unwrap()),
            None,
        );
        let app = proxy.build_router(32);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for path in [
            "/v1/messages",
            "/v1/chat/completions",
            "/v1/responses",
            "/v1/images/generations",
            "/v1/responses/compact",
            "/v1beta/models/test:generateContent",
        ] {
            for chunked in [false, true] {
                let body = if chunked {
                    reqwest::Body::wrap_stream(futures::stream::iter([
                        Ok::<_, std::io::Error>(bytes::Bytes::from_static(
                            b"                                ",
                        )),
                        Ok(bytes::Bytes::from_static(b"x")),
                    ]))
                } else {
                    reqwest::Body::from("x".repeat(33))
                };
                let response = client
                    .post(format!("http://{addr}{path}"))
                    .body(body)
                    .send()
                    .await
                    .unwrap();
                assert_eq!(
                    response.status(),
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "{path}, chunked={chunked}"
                );
                assert!(response
                    .text()
                    .await
                    .unwrap()
                    .contains("请求体超过大小上限"));
            }
        }
        // No providers exist: reaching selection would return 503, not 413.
        assert_eq!(proxy.state.status.read().await.total_requests, 0);
        assert!(proxy.state.current_providers.read().await.is_empty());
        // At and below the boundary, valid JSON reaches normal provider selection.
        for length in [31, 32] {
            let response = client
                .post(format!("http://{addr}/v1/messages"))
                .body(format!("{{}}{}", " ".repeat(length - 2)))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        }
        task.abort();
    }

    #[tokio::test]
    async fn upstream_error_body_is_kept_for_client_but_not_usage_storage() {
        for status in [StatusCode::UNAUTHORIZED, StatusCode::INTERNAL_SERVER_ERROR] {
            let secret_body = json!({"error": {"message": format!(
                "Authorization: Bearer fake-key/+= refreshed-token/+ query=fake-key%2F%2B%3D private-prompt {}",
                "x".repeat(4096)
            )}});
            let response_body = secret_body.clone();
            let mock = Router::new().route(
                "/v1/messages",
                post(move || {
                    let body = response_body.clone();
                    async move { (status, axum::Json(body)) }
                }),
            );
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .unwrap();
            let addr = listener.local_addr().unwrap();
            let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap() });
            let db = Arc::new(Database::memory().unwrap());
            let provider = Provider::with_id(
                "safe-error-test".into(),
                "Test".into(),
                json!({
                    "env": {"ANTHROPIC_BASE_URL": format!("http://{addr}"),
                        "ANTHROPIC_API_KEY": "fake-key/+="}
                }),
                None,
            );
            db.save_provider("claude", &provider).unwrap();
            db.set_current_provider("claude", &provider.id).unwrap();
            let proxy = ProxyServer::new(
                ProxyConfig {
                    listen_port: 0,
                    max_retries: 0,
                    non_streaming_timeout: 10,
                    ..Default::default()
                },
                db.clone(),
                None,
            );
            let info = proxy.start().await.unwrap();
            let response = reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .post(format!(
                    "http://127.0.0.1:{}/v1/messages",
                    info.port
                ))
                .json(&json!({"model": "claude-test", "max_tokens": 16, "messages": [{"role": "user", "content": "test"}]}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.json::<Value>().await.unwrap(), secret_body);
            {
                let conn = db.conn.lock().unwrap();
                let (stored_status, message): (u16, String) = conn
                    .query_row(
                        "SELECT status_code, error_message FROM proxy_request_logs",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .unwrap();
                assert_eq!(stored_status, status.as_u16());
                assert!(message.contains(&status.as_u16().to_string()));
                assert!(message.len() <= 256);
                for secret in [
                    "fake-key",
                    "refreshed-token",
                    "Authorization",
                    "private-prompt",
                    "%2F%2B%3D",
                ] {
                    assert!(!message.contains(secret), "stored error leaked {secret}");
                }
            }
            proxy.stop().await.unwrap();
            task.abort();
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        path_and_query: String,
        authorization: Option<String>,
        body: Value,
    }

    /// Codex 内置的 openai 先用 WebSocket 连 Responses：握手回 426 它才立刻改走 HTTP。
    #[tokio::test]
    async fn responses_websocket_handshake_is_answered_with_upgrade_required() {
        let db = Arc::new(Database::memory().expect("memory database"));
        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_port: 0,
                ..ProxyConfig::default()
            },
            db,
            None,
        );
        let proxy_info = proxy.start().await.expect("start test proxy");
        let client = reqwest::Client::new();
        for path in [
            "/responses",
            "/v1/responses",
            "/v1/v1/responses",
            "/codex/v1/responses",
        ] {
            let response = client
                .get(format!("http://127.0.0.1:{}{path}", proxy_info.port))
                .header(header::CONNECTION, "Upgrade")
                .header(header::UPGRADE, "websocket")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                .send()
                .await
                .expect("websocket handshake");
            assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED, "{path}");
        }
        proxy.stop().await.expect("stop test proxy");
    }

    /// A base URL pasted as a complete endpoint with the full-URL switch left off
    /// must derive the sibling standalone endpoint instead of having the
    /// standalone path appended to it.
    #[tokio::test]
    async fn codex_standalone_endpoints_derive_from_pasted_full_base_url() {
        let captured = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
        let capture_handler = {
            let captured = captured.clone();
            move |request: axum::extract::Request| {
                let captured = captured.clone();
                async move {
                    let (parts, _body) = request.into_parts();
                    captured.lock().await.push(CapturedRequest {
                        path_and_query: parts
                            .uri
                            .path_and_query()
                            .map(|value| value.as_str().to_string())
                            .unwrap_or_else(|| parts.uri.path().to_string()),
                        authorization: parts
                            .headers
                            .get(header::AUTHORIZATION)
                            .and_then(|value| value.to_str().ok())
                            .map(ToString::to_string),
                        body: Value::Null,
                    });

                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/json")],
                        r#"{"created":1,"data":[{"b64_json":"aW1hZ2U="}],"usage":{"input_tokens":7,"output_tokens":11,"total_tokens":18}}"#,
                    )
                }
            }
        };
        let mock_app = Router::new()
            .route("/v1/images/generations", post(capture_handler.clone()))
            .route("/v1/images/edits", post(capture_handler.clone()))
            .route("/Gateway/v1/images/edits", post(capture_handler.clone()))
            .route("/v1/alpha/search", post(capture_handler));
        let mock_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock upstream");
        let mock_addr = mock_listener.local_addr().expect("mock upstream address");
        let mock_handle = tokio::spawn(async move {
            axum::serve(mock_listener, mock_app)
                .await
                .expect("serve mock upstream");
        });

        let db = Arc::new(Database::memory().expect("memory database"));
        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_port: 0,
                non_streaming_timeout: 10,
                ..ProxyConfig::default()
            },
            db.clone(),
            None,
        );
        let proxy_info = proxy.start().await.expect("start test proxy");
        let client = reqwest::Client::new();

        let cases = [
            (
                "pasted-mixed-case-chat-completions",
                format!("http://{mock_addr}/v1/Chat/Completions?api-version=CaseValue"),
                "/v1/images/edits",
                "/v1/images/edits?api-version=CaseValue&client_version=0.145.0",
            ),
            (
                "pasted-mixed-case-images-generations",
                format!("http://{mock_addr}/Gateway/v1/Images/Generations/?api-version=CaseValue#fragment"),
                "/v1/images/edits",
                "/Gateway/v1/images/edits?api-version=CaseValue&client_version=0.145.0",
            ),
            (
                "pasted-mixed-case-images-edits",
                format!("http://{mock_addr}/v1/Images/Edits/"),
                "/v1/images/generations",
                "/v1/images/generations?client_version=0.145.0",
            ),
            (
                "pasted-mixed-case-responses-compact",
                format!("http://{mock_addr}/v1/Responses/Compact/"),
                "/v1/alpha/search",
                "/v1/alpha/search?client_version=0.145.0",
            ),
            (
                "pasted-chat-completions",
                format!("http://{mock_addr}/v1/chat/completions"),
                "/v1/images/generations",
                "/v1/images/generations?client_version=0.145.0",
            ),
            (
                "pasted-chat-completions",
                format!("http://{mock_addr}/v1/chat/completions"),
                "/v1/images/edits",
                "/v1/images/edits?client_version=0.145.0",
            ),
            (
                "pasted-images-generations",
                format!("http://{mock_addr}/v1/images/generations?api-version=test"),
                "/v1/images/edits",
                "/v1/images/edits?api-version=test&client_version=0.145.0",
            ),
            (
                "pasted-responses",
                format!("http://{mock_addr}/v1/responses"),
                "/v1/alpha/search",
                "/v1/alpha/search?client_version=0.145.0",
            ),
        ];

        for (provider_id, base_url, local_path, expected_upstream) in cases {
            let provider = Provider::with_id(
                provider_id.to_string(),
                provider_id.to_string(),
                json!({
                    "base_url": base_url,
                    "auth": {"OPENAI_API_KEY": "upstream-secret"}
                }),
                None,
            );
            db.save_provider("codex", &provider)
                .expect("save pasted base URL provider");
            db.set_current_provider("codex", &provider.id)
                .expect("select pasted base URL provider");

            let response = client
                .post(format!(
                    "http://127.0.0.1:{}{local_path}?client_version=0.145.0",
                    proxy_info.port
                ))
                .header(header::AUTHORIZATION, "Bearer client-secret")
                .json(&json!({"model": "gpt-image-1", "prompt": "pasted base URL"}))
                .send()
                .await
                .expect("send images request");

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{local_path} via {base_url}"
            );
            let request = captured
                .lock()
                .await
                .pop()
                .expect("upstream request captured");
            assert_eq!(
                request.path_and_query, expected_upstream,
                "{local_path} via {base_url}"
            );
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer upstream-secret")
            );
        }

        proxy.stop().await.expect("stop test proxy");
        mock_handle.abort();
    }

    #[tokio::test]
    async fn codex_images_generation_aliases_forward_and_record_usage() {
        let captured = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
        let mock_app = Router::new().route(
            "/v1/images/generations",
            post({
                let captured = captured.clone();
                move |request: axum::extract::Request| {
                    let captured = captured.clone();
                    async move {
                        let (parts, body) = request.into_parts();
                        let body = axum::body::to_bytes(body, 1024 * 1024)
                            .await
                            .expect("read mock request body");
                        captured.lock().await.push(CapturedRequest {
                            path_and_query: parts
                                .uri
                                .path_and_query()
                                .map(|value| value.as_str().to_string())
                                .unwrap_or_else(|| parts.uri.path().to_string()),
                            authorization: parts
                                .headers
                                .get(header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok())
                                .map(ToString::to_string),
                            body: serde_json::from_slice(&body).expect("parse mock request body"),
                        });

                        (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "application/json")],
                            r#"{"created":1,"data":[{"b64_json":"aW1hZ2U="}],"usage":{"input_tokens":7,"output_tokens":11,"total_tokens":18}}"#,
                        )
                    }
                }
            }),
        );
        let mock_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock upstream");
        let mock_addr = mock_listener.local_addr().expect("mock upstream address");
        let mock_handle = tokio::spawn(async move {
            axum::serve(mock_listener, mock_app)
                .await
                .expect("serve mock upstream");
        });

        let db = Arc::new(Database::memory().expect("memory database"));
        let provider = Provider::with_id(
            "images-api-upstream".to_string(),
            "Images API Upstream".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1"),
                "auth": {"OPENAI_API_KEY": "upstream-secret"}
            }),
            None,
        );
        db.save_provider("codex", &provider)
            .expect("save test provider");
        db.set_current_provider("codex", &provider.id)
            .expect("select test provider");

        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_port: 0,
                enable_logging: true,
                non_streaming_timeout: 10,
                ..ProxyConfig::default()
            },
            db.clone(),
            None,
        );
        let proxy_info = proxy.start().await.expect("start test proxy");
        let client = reqwest::Client::new();
        let aliases = [
            "/images/generations",
            "/v1/images/generations",
            "/v1/v1/images/generations",
            "/codex/v1/images/generations",
        ];

        for (index, path) in aliases.iter().enumerate() {
            let response = client
                .post(format!(
                    "http://127.0.0.1:{}{}?client_version=0.145.0",
                    proxy_info.port, path
                ))
                .header(header::AUTHORIZATION, "Bearer client-secret")
                .json(&json!({
                    "model": "gpt-image-1",
                    "prompt": format!("image generation alias {index}")
                }))
                .send()
                .await
                .expect("send images request");

            assert_eq!(response.status(), StatusCode::OK, "alias {path}");
            assert_eq!(
                response.text().await.expect("read proxy response"),
                r#"{"created":1,"data":[{"b64_json":"aW1hZ2U="}],"usage":{"input_tokens":7,"output_tokens":11,"total_tokens":18}}"#,
                "alias {path}"
            );
        }

        let (log_count, input_tokens, output_tokens): (i64, i64, i64) =
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                loop {
                    let totals = {
                        let conn = crate::database::lock_conn!(db.conn);
                        match conn.query_row(
                            "SELECT COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0)
                             FROM proxy_request_logs WHERE provider_id = ?1",
                            ["images-api-upstream"],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        ) {
                            Ok(totals) => totals,
                            Err(error) => panic!("query images usage logs: {error}"),
                        }
                    };

                    if totals.0 == aliases.len() as i64 {
                        break Ok::<_, AppError>(totals);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("images usage logs were not recorded")
            .expect("read images usage logs");
        assert_eq!(log_count, aliases.len() as i64);
        assert_eq!(input_tokens, 7 * aliases.len() as i64);
        assert_eq!(output_tokens, 11 * aliases.len() as i64);

        let mut full_url_provider = Provider::with_id(
            "images-api-full-url".to_string(),
            "Images API Full URL".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1/responses?api-version=test"),
                "auth": {"OPENAI_API_KEY": "full-url-secret"}
            }),
            None,
        );
        full_url_provider.meta = Some(ProviderMeta {
            is_full_url: Some(true),
            ..ProviderMeta::default()
        });
        db.save_provider("codex", &full_url_provider)
            .expect("save full URL images provider");
        db.set_current_provider("codex", &full_url_provider.id)
            .expect("select full URL images provider");

        let response = client
            .post(format!(
                "http://127.0.0.1:{}/v1/images/generations?client_version=0.145.0",
                proxy_info.port
            ))
            .header(header::AUTHORIZATION, "Bearer client-secret")
            .json(&json!({
                "model": "gpt-image-1",
                "prompt": "image generation full URL"
            }))
            .send()
            .await
            .expect("send full URL images request");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.text().await.expect("read full URL image response"),
            r#"{"created":1,"data":[{"b64_json":"aW1hZ2U="}],"usage":{"input_tokens":7,"output_tokens":11,"total_tokens":18}}"#
        );

        let captured = captured.lock().await;
        assert_eq!(captured.len(), aliases.len() + 1);
        for (index, request) in captured.iter().take(aliases.len()).enumerate() {
            assert_eq!(
                request.path_and_query,
                "/v1/images/generations?client_version=0.145.0"
            );
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer upstream-secret")
            );
            assert_eq!(request.body["model"], "gpt-image-1");
            assert_eq!(
                request.body["prompt"],
                format!("image generation alias {index}")
            );
        }

        let full_url_request = captured.last().expect("full URL image request captured");
        assert_eq!(
            full_url_request.path_and_query,
            "/v1/images/generations?api-version=test&client_version=0.145.0"
        );
        assert_eq!(
            full_url_request.authorization.as_deref(),
            Some("Bearer full-url-secret")
        );
        assert_eq!(full_url_request.body["model"], "gpt-image-1");
        assert_eq!(full_url_request.body["prompt"], "image generation full URL");

        proxy.stop().await.expect("stop test proxy");
        mock_handle.abort();
    }

    #[tokio::test]
    async fn codex_images_edit_aliases_forward_and_record_usage() {
        // Real Images API responses carry `input_tokens_details` with text/image
        // splits; the shared Codex usage parser must tolerate them.
        const UPSTREAM_BODY: &str = r#"{"created":1,"data":[{"b64_json":"ZWRpdA=="}],"usage":{"input_tokens":13,"output_tokens":17,"total_tokens":30,"input_tokens_details":{"text_tokens":5,"image_tokens":8}}}"#;
        const IMAGE_DATA_URL: &str = "data:image/png;base64,Zm9v";

        let captured = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
        let mock_app = Router::new().route(
            "/v1/images/edits",
            post({
                let captured = captured.clone();
                move |request: axum::extract::Request| {
                    let captured = captured.clone();
                    async move {
                        let (parts, body) = request.into_parts();
                        let body = axum::body::to_bytes(body, 1024 * 1024)
                            .await
                            .expect("read mock request body");
                        captured.lock().await.push(CapturedRequest {
                            path_and_query: parts
                                .uri
                                .path_and_query()
                                .map(|value| value.as_str().to_string())
                                .unwrap_or_else(|| parts.uri.path().to_string()),
                            authorization: parts
                                .headers
                                .get(header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok())
                                .map(ToString::to_string),
                            body: serde_json::from_slice(&body).expect("parse mock request body"),
                        });

                        (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "application/json")],
                            UPSTREAM_BODY,
                        )
                    }
                }
            }),
        );
        let mock_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock upstream");
        let mock_addr = mock_listener.local_addr().expect("mock upstream address");
        let mock_handle = tokio::spawn(async move {
            axum::serve(mock_listener, mock_app)
                .await
                .expect("serve mock upstream");
        });

        let db = Arc::new(Database::memory().expect("memory database"));
        let provider = Provider::with_id(
            "images-edit-upstream".to_string(),
            "Images Edit Upstream".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1"),
                "auth": {"OPENAI_API_KEY": "upstream-secret"}
            }),
            None,
        );
        db.save_provider("codex", &provider)
            .expect("save test provider");
        db.set_current_provider("codex", &provider.id)
            .expect("select test provider");

        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_port: 0,
                enable_logging: true,
                non_streaming_timeout: 10,
                ..ProxyConfig::default()
            },
            db.clone(),
            None,
        );
        let proxy_info = proxy.start().await.expect("start test proxy");
        let client = reqwest::Client::new();
        let aliases = [
            "/images/edits",
            "/v1/images/edits",
            "/v1/v1/images/edits",
            "/codex/v1/images/edits",
        ];

        for (index, path) in aliases.iter().enumerate() {
            let response = client
                .post(format!(
                    "http://127.0.0.1:{}{}?client_version=0.145.0",
                    proxy_info.port, path
                ))
                .header(header::AUTHORIZATION, "Bearer client-secret")
                .json(&json!({
                    "model": "gpt-image-2",
                    "prompt": format!("image edit alias {index}"),
                    "images": [{"image_url": IMAGE_DATA_URL}]
                }))
                .send()
                .await
                .expect("send images edit request");

            assert_eq!(response.status(), StatusCode::OK, "alias {path}");
            assert_eq!(
                response.text().await.expect("read proxy response"),
                UPSTREAM_BODY,
                "alias {path}"
            );
        }

        let (log_count, input_tokens, output_tokens): (i64, i64, i64) =
            tokio::time::timeout(std::time::Duration::from_secs(2), async {
                loop {
                    let totals = {
                        let conn = crate::database::lock_conn!(db.conn);
                        match conn.query_row(
                            "SELECT COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0)
                             FROM proxy_request_logs WHERE provider_id = ?1",
                            ["images-edit-upstream"],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        ) {
                            Ok(totals) => totals,
                            Err(error) => panic!("query images edit usage logs: {error}"),
                        }
                    };

                    if totals.0 == aliases.len() as i64 {
                        break Ok::<_, AppError>(totals);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("images edit usage logs were not recorded")
            .expect("read images edit usage logs");
        assert_eq!(log_count, aliases.len() as i64);
        assert_eq!(input_tokens, 13 * aliases.len() as i64);
        assert_eq!(output_tokens, 17 * aliases.len() as i64);

        // A full URL pasted for the sibling generations route must derive
        // `/images/edits` instead of posting edit payloads to generations.
        let mut full_url_provider = Provider::with_id(
            "images-edit-full-url".to_string(),
            "Images Edit Full URL".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1/images/generations?api-version=test"),
                "auth": {"OPENAI_API_KEY": "full-url-secret"}
            }),
            None,
        );
        full_url_provider.meta = Some(ProviderMeta {
            is_full_url: Some(true),
            ..ProviderMeta::default()
        });
        db.save_provider("codex", &full_url_provider)
            .expect("save full URL images edit provider");
        db.set_current_provider("codex", &full_url_provider.id)
            .expect("select full URL images edit provider");

        let response = client
            .post(format!(
                "http://127.0.0.1:{}/v1/images/edits?client_version=0.145.0",
                proxy_info.port
            ))
            .header(header::AUTHORIZATION, "Bearer client-secret")
            .json(&json!({
                "model": "gpt-image-2",
                "prompt": "image edit full URL",
                "images": [{"image_url": IMAGE_DATA_URL}]
            }))
            .send()
            .await
            .expect("send full URL images edit request");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .text()
                .await
                .expect("read full URL image edit response"),
            UPSTREAM_BODY
        );

        let captured = captured.lock().await;
        assert_eq!(captured.len(), aliases.len() + 1);
        for (index, request) in captured.iter().take(aliases.len()).enumerate() {
            assert_eq!(
                request.path_and_query,
                "/v1/images/edits?client_version=0.145.0"
            );
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer upstream-secret")
            );
            assert_eq!(request.body["model"], "gpt-image-2");
            assert_eq!(request.body["prompt"], format!("image edit alias {index}"));
            assert_eq!(request.body["images"][0]["image_url"], IMAGE_DATA_URL);
        }

        let full_url_request = captured
            .last()
            .expect("full URL image edit request captured");
        assert_eq!(
            full_url_request.path_and_query,
            "/v1/images/edits?api-version=test&client_version=0.145.0"
        );
        assert_eq!(
            full_url_request.authorization.as_deref(),
            Some("Bearer full-url-secret")
        );
        assert_eq!(full_url_request.body["model"], "gpt-image-2");
        assert_eq!(full_url_request.body["prompt"], "image edit full URL");
        assert_eq!(
            full_url_request.body["images"][0]["image_url"],
            IMAGE_DATA_URL
        );

        proxy.stop().await.expect("stop test proxy");
        mock_handle.abort();
    }

    #[tokio::test]
    async fn alpha_search_routes_forward_to_canonical_upstream() {
        let captured = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
        let mock_app = Router::new().route(
            "/v1/alpha/search",
            post({
                let captured = captured.clone();
                move |request: axum::extract::Request| {
                    let captured = captured.clone();
                    async move {
                        let (parts, body) = request.into_parts();
                        let body = axum::body::to_bytes(body, 1024 * 1024)
                            .await
                            .expect("read mock request body");
                        captured.lock().await.push(CapturedRequest {
                            path_and_query: parts
                                .uri
                                .path_and_query()
                                .map(|value| value.as_str().to_string())
                                .unwrap_or_else(|| parts.uri.path().to_string()),
                            authorization: parts
                                .headers
                                .get(header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok())
                                .map(ToString::to_string),
                            body: serde_json::from_slice(&body).expect("parse mock request body"),
                        });

                        let mut headers = HeaderMap::new();
                        headers.insert(
                            header::CONTENT_TYPE,
                            "application/json".parse().expect("content type"),
                        );
                        headers.insert(
                            "x-upstream-request-id",
                            "search-1".parse().expect("request id"),
                        );
                        (
                            StatusCode::ACCEPTED,
                            headers,
                            r#"{"encrypted_output":"ciphertext"}"#,
                        )
                    }
                }
            }),
        );
        let mock_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock upstream");
        let mock_addr = mock_listener.local_addr().expect("mock upstream address");
        let mock_handle = tokio::spawn(async move {
            axum::serve(mock_listener, mock_app)
                .await
                .expect("serve mock upstream");
        });

        let db = Arc::new(Database::memory().expect("memory database"));
        let provider = Provider::with_id(
            "alpha-search-upstream".to_string(),
            "Alpha Search Upstream".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1"),
                "auth": {"OPENAI_API_KEY": "upstream-secret"}
            }),
            None,
        );
        db.save_provider("codex", &provider)
            .expect("save test provider");
        db.set_current_provider("codex", &provider.id)
            .expect("select test provider");

        let proxy = ProxyServer::new(
            ProxyConfig {
                listen_port: 0,
                enable_logging: false,
                non_streaming_timeout: 10,
                ..ProxyConfig::default()
            },
            db.clone(),
            None,
        );
        let proxy_info = proxy.start().await.expect("start test proxy");
        let client = reqwest::Client::new();
        let aliases = [
            "/alpha/search",
            "/v1/alpha/search",
            "/v1/v1/alpha/search",
            "/codex/v1/alpha/search",
        ];

        for (index, path) in aliases.iter().enumerate() {
            let response = client
                .post(format!(
                    "http://127.0.0.1:{}{}?client_version=0.144.6",
                    proxy_info.port, path
                ))
                .header(header::AUTHORIZATION, "Bearer client-secret")
                .json(&json!({
                    "id": format!("search-{index}"),
                    "model": "gpt-5.6-sol",
                    "commands": {"search_query": [{"q": "test"}]}
                }))
                .send()
                .await
                .expect("send alpha search request");

            assert_eq!(response.status(), StatusCode::ACCEPTED, "alias {path}");
            assert_eq!(
                response
                    .headers()
                    .get("x-upstream-request-id")
                    .and_then(|value| value.to_str().ok()),
                Some("search-1"),
                "alias {path}"
            );
            assert_eq!(
                response.text().await.expect("read proxy response"),
                r#"{"encrypted_output":"ciphertext"}"#,
                "alias {path}"
            );
        }

        // Full-URL providers were the known flaw in the original PR: without a
        // sibling-endpoint rewrite, this request would be posted back to
        // `/v1/responses` instead of `/v1/alpha/search`.
        let mut full_url_provider = Provider::with_id(
            "alpha-search-full-url".to_string(),
            "Alpha Search Full URL".to_string(),
            json!({
                "base_url": format!("http://{mock_addr}/v1/responses?api-version=test"),
                "auth": {"OPENAI_API_KEY": "full-url-secret"}
            }),
            None,
        );
        full_url_provider.meta = Some(ProviderMeta {
            is_full_url: Some(true),
            ..ProviderMeta::default()
        });
        db.save_provider("codex", &full_url_provider)
            .expect("save full URL provider");
        db.set_current_provider("codex", &full_url_provider.id)
            .expect("select full URL provider");

        let response = client
            .post(format!(
                "http://127.0.0.1:{}/v1/alpha/search?client_version=0.144.6",
                proxy_info.port
            ))
            .header(header::AUTHORIZATION, "Bearer client-secret")
            .json(&json!({
                "id": "search-full-url",
                "model": "gpt-5.6-sol",
                "commands": {"search_query": [{"q": "full URL"}]}
            }))
            .send()
            .await
            .expect("send full URL alpha search request");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response.text().await.expect("read full URL response"),
            r#"{"encrypted_output":"ciphertext"}"#
        );

        proxy.stop().await.expect("stop test proxy");
        mock_handle.abort();

        let captured = captured.lock().await;
        assert_eq!(captured.len(), aliases.len() + 1);
        for (index, request) in captured.iter().take(aliases.len()).enumerate() {
            assert_eq!(
                request.path_and_query,
                "/v1/alpha/search?client_version=0.144.6"
            );
            assert_eq!(
                request.authorization.as_deref(),
                Some("Bearer upstream-secret")
            );
            assert_eq!(request.body["id"], format!("search-{index}"));
            assert_eq!(request.body["model"], "gpt-5.6-sol");
            assert_eq!(request.body["commands"]["search_query"][0]["q"], "test");
        }

        let full_url_request = captured.last().expect("full URL request captured");
        assert_eq!(
            full_url_request.path_and_query,
            "/v1/alpha/search?api-version=test&client_version=0.144.6"
        );
        assert_eq!(
            full_url_request.authorization.as_deref(),
            Some("Bearer full-url-secret")
        );
        assert_eq!(full_url_request.body["id"], "search-full-url");
        assert_eq!(
            full_url_request.body["commands"]["search_query"][0]["q"],
            "full URL"
        );
    }
}
