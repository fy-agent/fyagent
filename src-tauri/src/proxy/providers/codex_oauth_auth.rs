//! Codex OAuth Authentication Module
//!
//! 实现 OpenAI ChatGPT Plus/Pro 订阅的 OAuth Device Code 流程。
//! 支持多账号管理，每个 Provider 可关联不同的 ChatGPT 账号。
//!
//! ## 认证流程
//! 1. 启动 Device Code 流程，获取 device_auth_id 和 user_code
//! 2. 用户在浏览器中完成 ChatGPT 授权
//! 3. 轮询获取 authorization_code 和 code_verifier（注意：verifier 由服务端返回）
//! 4. 使用 code + verifier 换取 access_token + refresh_token + id_token
//! 5. 自动刷新 access_token（到期前 60 秒）
//!
//! ## 多账号支持
//! - 每个 ChatGPT 账号独立存储 refresh_token
//! - Provider 通过 meta.authBinding 关联账号（auth_provider = "codex_oauth"）
//! - HashMap 键是 FyAgent `credential_id`；`chatgpt_account_id` 只做上游路由元数据

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::copilot_auth::{GitHubAccount, GitHubDeviceCodeResponse};

/// OpenAI OAuth 客户端 ID（OpenCode 使用，与官方 Codex CLI 相同）
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// Device Code 启动 URL
const DEVICE_AUTH_USERCODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";

/// Device Code 轮询 URL
const DEVICE_AUTH_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";

/// OAuth Token URL（用于 code 换 token 和 refresh token）
const OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";

/// Device Code 验证 URL（向用户展示）
const DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";

/// Device Code 流程的 redirect_uri（OpenAI 服务端约定）
const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";

/// Token 刷新提前量（毫秒）
const TOKEN_REFRESH_BUFFER_MS: i64 = 60_000;

/// Device Code 默认有效时长（秒），OpenAI 文档约定 15 分钟
const DEVICE_CODE_DEFAULT_EXPIRES_IN: u64 = 900;

/// 轮询间隔安全余量（秒）
const POLLING_SAFETY_MARGIN_SECS: u64 = 3;

/// User-Agent
const CODEX_USER_AGENT: &str = "fyagent-codex-oauth";
const OAUTH_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

async fn send_bounded(
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, CodexOAuthError> {
    tokio::time::timeout(OAUTH_HTTP_TIMEOUT, request.send())
        .await
        .map_err(|_| CodexOAuthError::NetworkError("OAuth 请求超时".to_string()))?
        .map_err(CodexOAuthError::from)
}

/// Codex OAuth 错误
#[derive(Debug, thiserror::Error)]
pub enum CodexOAuthError {
    #[error("等待用户授权中")]
    AuthorizationPending,

    #[error("用户拒绝授权")]
    AccessDenied,

    #[error("Device Code 已过期")]
    ExpiredToken,

    #[error("OAuth Token 获取失败: {0}")]
    TokenFetchFailed(String),

    #[error("Refresh Token 失效或已过期")]
    RefreshTokenInvalid,

    #[error("网络错误: {0}")]
    NetworkError(String),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("账号不存在: {0}")]
    AccountNotFound(String),

    #[error("登录已取消")]
    Cancelled,
}

impl From<reqwest::Error> for CodexOAuthError {
    fn from(err: reqwest::Error) -> Self {
        CodexOAuthError::NetworkError(err.to_string())
    }
}

impl From<std::io::Error> for CodexOAuthError {
    fn from(err: std::io::Error) -> Self {
        CodexOAuthError::IoError(err.to_string())
    }
}

/// OpenAI Device Code 响应
#[derive(Debug, Clone, Deserialize)]
struct DeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// OpenAI Device Code 轮询响应（成功）
#[derive(Debug, Clone, Deserialize)]
struct DevicePollSuccess {
    authorization_code: String,
    code_verifier: String,
}

/// OAuth Token 响应
#[derive(Clone, Deserialize)]
pub(crate) struct OAuthTokenResponse {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    #[serde(default)]
    pub(crate) id_token: Option<String>,
    #[serde(default)]
    pub(crate) expires_in: Option<i64>,
}

/// 解析后的 JWT claims（仅关心 chatgpt_account_id 等字段）
#[derive(Debug, Clone, Default, Deserialize)]
struct IdTokenClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    organizations: Vec<OrgClaim>,
    #[serde(default, rename = "https://api.openai.com/auth")]
    openai_auth: Option<OpenAiAuthClaim>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OrgClaim {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAiAuthClaim {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
}

/// 缓存的 access_token（含过期时间）
#[derive(Debug, Clone)]
struct CachedAccessToken {
    token: String,
    /// 过期时间戳（毫秒）
    expires_at_ms: i64,
}

impl CachedAccessToken {
    fn is_expiring_soon(&self) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        self.expires_at_ms - now < TOKEN_REFRESH_BUFFER_MS
    }
}

/// 进行中的 Device Code 条目，带过期时间以便清理放弃的登录流程
#[derive(Debug, Clone)]
struct PendingDeviceCode {
    user_code: String,
    /// Unix 毫秒时间戳，超时后可清理
    expires_at_ms: i64,
}

/// 持久化的账号数据。HashMap 键是 `credential_id`，不是 ChatGPT workspace id。
#[derive(Clone, Serialize, Deserialize)]
struct CodexAccountData {
    pub credential_id: String,
    pub chatgpt_account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub refresh_token: String,
    pub authenticated_at: i64,
}

impl std::fmt::Debug for CodexAccountData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexAccountData")
            .field("credential_id", &self.credential_id)
            .field("chatgpt_account_id", &self.chatgpt_account_id)
            .field("email", &self.email)
            .field("refresh_token", &"<redacted>")
            .field("authenticated_at", &self.authenticated_at)
            .finish()
    }
}

/// 公开的账号信息（返回给前端，复用 GitHubAccount 结构）
impl From<&CodexAccountData> for GitHubAccount {
    fn from(data: &CodexAccountData) -> Self {
        GitHubAccount {
            id: data.credential_id.clone(),
            login: data
                .email
                .clone()
                .unwrap_or_else(|| format!("ChatGPT ({})", data.chatgpt_account_id)),
            avatar_url: None,
            authenticated_at: data.authenticated_at,
            github_domain: "github.com".to_string(),
            chatgpt_account_id: Some(data.chatgpt_account_id.clone()),
        }
    }
}

const CODEX_OAUTH_STORE_VERSION: u32 = 2;

/// 持久化存储结构（v2）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CodexOAuthStore {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    accounts: HashMap<String, CodexAccountData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyCodexAccountData {
    account_id: String,
    #[serde(default)]
    email: Option<String>,
    refresh_token: String,
    authenticated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyCodexOAuthStore {
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    #[serde(default)]
    accounts: HashMap<String, LegacyCodexAccountData>,
    #[serde(default)]
    default_account_id: Option<String>,
}

/// Codex OAuth 认证管理器（多账号）
pub struct CodexOAuthManager {
    accounts: Arc<RwLock<HashMap<String, CodexAccountData>>>,
    default_account_id: Arc<RwLock<Option<String>>>,
    /// 内存缓存的 access_token（不持久化）
    access_tokens: Arc<RwLock<HashMap<String, CachedAccessToken>>>,
    /// 每个账号的刷新锁
    refresh_locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
    /// 进行中的 Device Code 流程：device_auth_id -> {user_code, expires_at_ms}
    /// 过期条目会在 start_device_flow 时被清理，防止放弃的登录流程导致无界增长
    pending_device_codes: Arc<RwLock<HashMap<String, PendingDeviceCode>>>,
    login_lock: Mutex<()>,
    storage_path: PathBuf,
    store_loaded: bool,
    json_store_sealed: AtomicBool,
}

impl CodexOAuthManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let storage_path = data_dir.join("codex_oauth_auth.json");

        let mut manager = Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            default_account_id: Arc::new(RwLock::new(None)),
            access_tokens: Arc::new(RwLock::new(HashMap::new())),
            refresh_locks: Arc::new(RwLock::new(HashMap::new())),
            pending_device_codes: Arc::new(RwLock::new(HashMap::new())),
            login_lock: Mutex::new(()),
            storage_path,
            store_loaded: false,
            json_store_sealed: AtomicBool::new(false),
        };

        match manager.load_from_disk_sync() {
            Ok(()) => manager.store_loaded = true,
            Err(e) => log::warn!("[CodexOAuth] 加载存储失败: {e}"),
        }

        manager
    }

    pub fn store_loaded(&self) -> bool {
        self.store_loaded
    }

    pub fn seal_json_store(&self) {
        self.json_store_sealed.store(true, Ordering::SeqCst);
    }

    // ==================== 设备码流程 ====================

    /// 启动 Device Code 流程
    ///
    /// 返回 GitHubDeviceCodeResponse 复用现有前端结构，但字段含义对应 OpenAI 的字段：
    /// - device_code = device_auth_id
    /// - user_code = user_code
    /// - verification_uri = https://auth.openai.com/codex/device
    pub async fn start_device_flow(&self) -> Result<GitHubDeviceCodeResponse, CodexOAuthError> {
        log::info!("[CodexOAuth] 启动 Device Code 流程");

        let response = send_bounded(
            crate::proxy::http_client::get()
                .post(DEVICE_AUTH_USERCODE_URL)
                .header("Content-Type", "application/json")
                .header("User-Agent", CODEX_USER_AGENT)
                .json(&serde_json::json!({ "client_id": CODEX_CLIENT_ID })),
        )
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CodexOAuthError::NetworkError(format!(
                "Device Code 请求失败 ({status})"
            )));
        }

        let device: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;

        let interval = parse_interval(device.interval.as_ref());
        let expires_in = device.expires_in.unwrap_or(DEVICE_CODE_DEFAULT_EXPIRES_IN);
        let expires_at_ms = chrono::Utc::now().timestamp_millis() + (expires_in as i64) * 1000;

        // 记录 device_auth_id -> 用户码映射；同时清理所有已过期的条目，
        // 避免用户放弃登录流程导致 HashMap 无界增长
        {
            let mut pending = self.pending_device_codes.write().await;
            let now_ms = chrono::Utc::now().timestamp_millis();
            pending.retain(|_, entry| entry.expires_at_ms > now_ms);
            pending.insert(
                device.device_auth_id.clone(),
                PendingDeviceCode {
                    user_code: device.user_code.clone(),
                    expires_at_ms,
                },
            );
        }

        log::info!(
            "[CodexOAuth] 获取 Device Code 成功，user_code: {}",
            device.user_code
        );

        Ok(GitHubDeviceCodeResponse {
            device_code: device.device_auth_id,
            user_code: device.user_code,
            verification_uri: DEVICE_VERIFICATION_URL.to_string(),
            expires_in,
            interval,
        })
    }

    /// 轮询 Device Code 状态
    ///
    /// 接收 device_code（即 device_auth_id），返回 Some(account) 表示授权成功
    pub async fn poll_for_token(
        &self,
        device_code: &str,
    ) -> Result<Option<GitHubAccount>, CodexOAuthError> {
        let entry = {
            let pending = self.pending_device_codes.read().await;
            pending.get(device_code).cloned()
        };

        let entry = entry.ok_or_else(|| {
            CodexOAuthError::TokenFetchFailed(
                "未找到对应的 user_code，请重新启动登录流程".to_string(),
            )
        })?;

        if entry.expires_at_ms <= chrono::Utc::now().timestamp_millis() {
            let mut pending = self.pending_device_codes.write().await;
            pending.remove(device_code);
            return Err(CodexOAuthError::ExpiredToken);
        }

        let user_code = entry.user_code;

        log::debug!("[CodexOAuth] 轮询 Device Code");

        let poll_response = send_bounded(
            crate::proxy::http_client::get()
                .post(DEVICE_AUTH_TOKEN_URL)
                .header("Content-Type", "application/json")
                .header("User-Agent", CODEX_USER_AGENT)
                .json(&serde_json::json!({
                    "device_auth_id": device_code,
                    "user_code": user_code,
                })),
        )
        .await?;

        let status = poll_response.status();

        // 403/404 表示用户未完成授权，继续轮询
        if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND {
            return Err(CodexOAuthError::AuthorizationPending);
        }

        if status == reqwest::StatusCode::GONE {
            return Err(CodexOAuthError::ExpiredToken);
        }

        if !status.is_success() {
            return Err(CodexOAuthError::TokenFetchFailed(format!(
                "OAuth 轮询失败 ({status})"
            )));
        }

        let success: DevicePollSuccess = poll_response
            .json()
            .await
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;

        log::info!("[CodexOAuth] 用户已授权，正在换取 OAuth Token");

        // 用 authorization_code + code_verifier 换 token
        let tokens = self
            .exchange_code_for_tokens(&success.authorization_code, &success.code_verifier)
            .await?;

        {
            let mut pending = self.pending_device_codes.write().await;
            if pending.remove(device_code).is_none() {
                return Err(CodexOAuthError::Cancelled);
            }
        }

        let refresh_token = tokens.refresh_token.clone().ok_or_else(|| {
            CodexOAuthError::TokenFetchFailed("响应缺少 refresh_token".to_string())
        })?;

        let (chatgpt_account_id, email) = extract_identity_from_tokens(&tokens);
        let chatgpt_account_id = chatgpt_account_id.ok_or_else(|| {
            CodexOAuthError::ParseError("无法从 token 中提取 chatgpt_account_id".to_string())
        })?;

        let account = self
            .add_account_internal(
                chatgpt_account_id,
                refresh_token,
                email,
                tokens.access_token.clone(),
                tokens.expires_in,
            )
            .await?;

        Ok(Some(account))
    }

    /// 用 authorization_code + code_verifier 换取 tokens
    async fn exchange_code_for_tokens(
        &self,
        code: &str,
        code_verifier: &str,
    ) -> Result<OAuthTokenResponse, CodexOAuthError> {
        let response = send_bounded(
            crate::proxy::http_client::get()
                .post(OAUTH_TOKEN_URL)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", CODEX_USER_AGENT)
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code),
                    ("redirect_uri", DEVICE_REDIRECT_URI),
                    ("client_id", CODEX_CLIENT_ID),
                    ("code_verifier", code_verifier),
                ]),
        )
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CodexOAuthError::TokenFetchFailed(format!(
                "Token 交换失败 ({status})"
            )));
        }

        response
            .json()
            .await
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))
    }

    /// Refresh an OpenAI grant. Callers must already own the unique refresh
    /// lease for the credential lineage.
    pub(crate) async fn refresh_with_token(
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, CodexOAuthError> {
        let response = send_bounded(
            crate::proxy::http_client::get()
                .post(OAUTH_TOKEN_URL)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("User-Agent", CODEX_USER_AGENT)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token),
                    ("client_id", CODEX_CLIENT_ID),
                    ("scope", "openid profile email"),
                ]),
        )
        .await?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(CodexOAuthError::RefreshTokenInvalid);
        }

        if !status.is_success() {
            return Err(CodexOAuthError::TokenFetchFailed(format!(
                "Refresh 失败 ({status})"
            )));
        }

        response
            .json()
            .await
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))
    }

    // ==================== Token 获取（含自动刷新） ====================

    /// 获取指定账号的有效 access_token（必要时自动刷新）
    pub async fn get_valid_token_for_account(
        &self,
        account_id: &str,
    ) -> Result<String, CodexOAuthError> {
        // 先检查缓存
        {
            let tokens = self.access_tokens.read().await;
            if let Some(cached) = tokens.get(account_id) {
                if !cached.is_expiring_soon() {
                    return Ok(cached.token.clone());
                }
            }
        }

        log::info!("[CodexOAuth] 账号 {account_id} 的 access_token 需要刷新");

        let refresh_lock = self.get_refresh_lock(account_id).await;
        let _guard = refresh_lock.lock().await;

        // double-check
        {
            let tokens = self.access_tokens.read().await;
            if let Some(cached) = tokens.get(account_id) {
                if !cached.is_expiring_soon() {
                    return Ok(cached.token.clone());
                }
            }
        }

        let refresh_token = {
            let accounts = self.accounts.read().await;
            accounts
                .get(account_id)
                .map(|a| a.refresh_token.clone())
                .ok_or_else(|| CodexOAuthError::AccountNotFound(account_id.to_string()))?
        };

        let new_tokens = Self::refresh_with_token(&refresh_token).await?;

        // 如果服务端返回了新的 refresh_token，更新存储
        if let Some(new_refresh) = new_tokens.refresh_token.clone() {
            if new_refresh != refresh_token {
                let mut accounts = self.accounts.write().await;
                if let Some(account) = accounts.get_mut(account_id) {
                    account.refresh_token = new_refresh;
                }
                drop(accounts);
                self.save_to_disk().await?;
            }
        }

        let access_token = new_tokens.access_token.clone();
        let expires_at_ms = compute_expires_at_ms(new_tokens.expires_in);

        {
            let mut tokens = self.access_tokens.write().await;
            tokens.insert(
                account_id.to_string(),
                CachedAccessToken {
                    token: access_token.clone(),
                    expires_at_ms,
                },
            );
        }

        Ok(access_token)
    }

    /// 获取默认账号的有效 token
    pub async fn get_valid_token(&self) -> Result<String, CodexOAuthError> {
        match self.resolve_default_account_id().await {
            Some(id) => self.get_valid_token_for_account(&id).await,
            None => Err(CodexOAuthError::AccountNotFound(
                "无可用的 ChatGPT 账号".to_string(),
            )),
        }
    }

    /// 获取默认账号 ID（热路径使用，避免克隆整个账号 HashMap）
    pub async fn default_account_id(&self) -> Option<String> {
        self.resolve_default_account_id().await
    }

    pub async fn chatgpt_account_id_for(&self, credential_id: &str) -> Option<String> {
        self.accounts
            .read()
            .await
            .get(credential_id)
            .map(|account| account.chatgpt_account_id.clone())
    }

    pub async fn cancel_pending_login(
        &self,
        device_code: Option<&str>,
    ) -> Result<(), CodexOAuthError> {
        let mut pending = self.pending_device_codes.write().await;
        if let Some(device_code) = device_code {
            pending.remove(device_code);
        } else {
            pending.clear();
        }
        Ok(())
    }

    // ==================== 多账号管理 ====================

    pub async fn list_accounts(&self) -> Vec<GitHubAccount> {
        let accounts = self.accounts.read().await.clone();
        let default_id = self.resolve_default_account_id().await;
        Self::sorted_accounts(&accounts, default_id.as_deref())
    }

    pub async fn remove_account(&self, account_id: &str) -> Result<(), CodexOAuthError> {
        log::info!("[CodexOAuth] 移除账号: {account_id}");

        {
            let mut accounts = self.accounts.write().await;
            if accounts.remove(account_id).is_none() {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
        }

        {
            let mut tokens = self.access_tokens.write().await;
            tokens.remove(account_id);
        }
        {
            let mut locks = self.refresh_locks.write().await;
            locks.remove(account_id);
        }

        {
            let accounts = self.accounts.read().await;
            let mut default = self.default_account_id.write().await;
            if default.as_deref() == Some(account_id) {
                *default = Self::fallback_default_account_id(&accounts);
            }
        }

        self.save_to_disk().await?;
        Ok(())
    }

    pub async fn set_default_account(&self, account_id: &str) -> Result<(), CodexOAuthError> {
        {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
        }

        {
            let mut default = self.default_account_id.write().await;
            *default = Some(account_id.to_string());
        }

        self.save_to_disk().await?;
        Ok(())
    }

    pub async fn clear_auth(&self) -> Result<(), CodexOAuthError> {
        log::info!("[CodexOAuth] 清除所有认证");

        {
            let mut accounts = self.accounts.write().await;
            accounts.clear();
        }
        {
            let mut default = self.default_account_id.write().await;
            *default = None;
        }
        {
            let mut tokens = self.access_tokens.write().await;
            tokens.clear();
        }
        {
            let mut locks = self.refresh_locks.write().await;
            locks.clear();
        }
        {
            let mut pending = self.pending_device_codes.write().await;
            pending.clear();
        }

        if self.storage_path.exists() {
            std::fs::remove_file(&self.storage_path)?;
        }

        Ok(())
    }

    pub async fn is_authenticated(&self) -> bool {
        let accounts = self.accounts.read().await;
        !accounts.is_empty()
    }

    /// 获取认证状态摘要（与 Copilot 的格式保持一致，便于复用前端）
    pub async fn get_status(&self) -> CodexOAuthStatus {
        let accounts_map = self.accounts.read().await.clone();
        let default_id = self.resolve_default_account_id().await;
        let account_list = Self::sorted_accounts(&accounts_map, default_id.as_deref());
        let authenticated = !account_list.is_empty();
        let username = default_id
            .as_ref()
            .and_then(|id| accounts_map.get(id))
            .and_then(|a| a.email.clone())
            .or_else(|| account_list.first().map(|a| a.login.clone()));

        CodexOAuthStatus {
            accounts: account_list,
            default_account_id: default_id,
            authenticated,
            username,
        }
    }

    // ==================== 内部方法 ====================

    async fn add_account_internal(
        &self,
        chatgpt_account_id: String,
        refresh_token: String,
        email: Option<String>,
        access_token: String,
        expires_in: Option<i64>,
    ) -> Result<GitHubAccount, CodexOAuthError> {
        let _guard = self.login_lock.lock().await;
        let now = chrono::Utc::now().timestamp();
        let credential_id = uuid::Uuid::new_v4().to_string();

        let data = CodexAccountData {
            credential_id: credential_id.clone(),
            chatgpt_account_id,
            email,
            refresh_token,
            authenticated_at: now,
        };

        let account = GitHubAccount::from(&data);

        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(credential_id.clone(), data);
        }

        {
            let mut tokens_cache = self.access_tokens.write().await;
            tokens_cache.insert(
                credential_id.clone(),
                CachedAccessToken {
                    token: access_token,
                    expires_at_ms: compute_expires_at_ms(expires_in),
                },
            );
        }

        {
            let mut default = self.default_account_id.write().await;
            if default.is_none() {
                *default = Some(credential_id);
            }
        }

        self.save_to_disk().await?;
        Ok(account)
    }

    fn fallback_default_account_id(accounts: &HashMap<String, CodexAccountData>) -> Option<String> {
        accounts
            .iter()
            .max_by(|(id_a, a), (id_b, b)| {
                a.authenticated_at
                    .cmp(&b.authenticated_at)
                    .then_with(|| id_b.cmp(id_a))
            })
            .map(|(id, _)| id.clone())
    }

    fn sorted_accounts(
        accounts: &HashMap<String, CodexAccountData>,
        default_account_id: Option<&str>,
    ) -> Vec<GitHubAccount> {
        let mut list: Vec<GitHubAccount> = accounts.values().map(GitHubAccount::from).collect();
        list.sort_by(|a, b| {
            let a_default = default_account_id == Some(a.id.as_str());
            let b_default = default_account_id == Some(b.id.as_str());
            b_default
                .cmp(&a_default)
                .then_with(|| b.authenticated_at.cmp(&a.authenticated_at))
                .then_with(|| a.login.cmp(&b.login))
        });
        list
    }

    async fn resolve_default_account_id(&self) -> Option<String> {
        let stored = self.default_account_id.read().await.clone();
        let accounts = self.accounts.read().await;

        if let Some(id) = stored {
            if accounts.contains_key(&id) {
                return Some(id);
            }
        }

        Self::fallback_default_account_id(&accounts)
    }

    async fn get_refresh_lock(&self, account_id: &str) -> Arc<Mutex<()>> {
        {
            let locks = self.refresh_locks.read().await;
            if let Some(lock) = locks.get(account_id) {
                return Arc::clone(lock);
            }
        }

        let mut locks = self.refresh_locks.write().await;
        Arc::clone(
            locks
                .entry(account_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }

    fn write_store_atomic(&self, content: &str) -> Result<(), CodexOAuthError> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let parent = self
            .storage_path
            .parent()
            .ok_or_else(|| CodexOAuthError::IoError("无效的存储路径".to_string()))?;
        let file_name = self
            .storage_path
            .file_name()
            .ok_or_else(|| CodexOAuthError::IoError("无效的存储文件名".to_string()))?
            .to_string_lossy()
            .to_string();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp_path = parent.join(format!("{file_name}.tmp.{ts}"));

        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;

            fs::rename(&tmp_path, &self.storage_path)?;
            fs::set_permissions(&self.storage_path, fs::Permissions::from_mode(0o600))?;
        }

        #[cfg(windows)]
        {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;

            if self.storage_path.exists() {
                let _ = fs::remove_file(&self.storage_path);
            }
            fs::rename(&tmp_path, &self.storage_path)?;
        }

        Ok(())
    }

    fn load_from_disk_sync(&self) -> Result<(), CodexOAuthError> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.storage_path)?;
        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);

        let store = if version >= u64::from(CODEX_OAUTH_STORE_VERSION) {
            serde_json::from_value::<CodexOAuthStore>(value)
                .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?
        } else {
            migrate_v1_store(&self.storage_path, &content)?
        };

        let mut accounts = self
            .accounts
            .try_write()
            .map_err(|_| CodexOAuthError::IoError("无法写入账号缓存".to_string()))?;
        let mut default = self
            .default_account_id
            .try_write()
            .map_err(|_| CodexOAuthError::IoError("无法写入默认账号缓存".to_string()))?;
        *accounts = store.accounts;
        log::info!("[CodexOAuth] 从磁盘加载 {} 个账号", accounts.len());
        *default = store.default_account_id;
        if default.is_none() {
            *default = Self::fallback_default_account_id(&accounts);
        }

        Ok(())
    }

    pub fn remap_provider_bindings(&self) {
        if !self.store_loaded {
            return;
        }
        let Ok(accounts) = self.accounts.try_read() else {
            return;
        };
        remap_codex_provider_bindings(&chatgpt_to_credential_map(&accounts));
    }

    async fn save_to_disk(&self) -> Result<(), CodexOAuthError> {
        if self.json_store_sealed.load(Ordering::SeqCst) {
            log::info!("[CodexOAuth] vault owns credentials; skipping plaintext store write");
            return Ok(());
        }
        let accounts = self.accounts.read().await.clone();
        let default = self.resolve_default_account_id().await;

        let store = CodexOAuthStore {
            version: CODEX_OAUTH_STORE_VERSION,
            accounts,
            default_account_id: default,
        };

        let content = serde_json::to_string_pretty(&store)
            .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;

        self.write_store_atomic(&content)?;

        log::info!(
            "[CodexOAuth] 保存到磁盘成功（{} 个账号）",
            store.accounts.len()
        );

        Ok(())
    }
}

fn backup_v1_store(path: &Path, content: &str) -> Result<(), CodexOAuthError> {
    let backup = path.with_extension("json.v1.bak");
    if backup.exists() {
        return Ok(());
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&backup, content)?;
    Ok(())
}

fn migrate_v1_store(path: &Path, content: &str) -> Result<CodexOAuthStore, CodexOAuthError> {
    backup_v1_store(path, content)?;
    let legacy: LegacyCodexOAuthStore =
        serde_json::from_str(content).map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;
    let mut accounts = HashMap::new();
    let mut old_to_new = HashMap::new();
    for (old_key, legacy_account) in legacy.accounts {
        let credential_id = uuid::Uuid::new_v4().to_string();
        let chatgpt_account_id = if legacy_account.account_id.is_empty() {
            old_key.clone()
        } else {
            legacy_account.account_id
        };
        old_to_new.insert(old_key, credential_id.clone());
        accounts.insert(
            credential_id.clone(),
            CodexAccountData {
                credential_id: credential_id.clone(),
                chatgpt_account_id,
                email: legacy_account.email,
                refresh_token: legacy_account.refresh_token,
                authenticated_at: legacy_account.authenticated_at,
            },
        );
    }
    let default_account_id = legacy
        .default_account_id
        .and_then(|id| old_to_new.get(&id).cloned());
    let store = CodexOAuthStore {
        version: CODEX_OAUTH_STORE_VERSION,
        accounts,
        default_account_id,
    };
    let migrated = serde_json::to_string_pretty(&store)
        .map_err(|e| CodexOAuthError::ParseError(e.to_string()))?;
    fs::write(path, migrated)?;
    Ok(store)
}

fn chatgpt_to_credential_map(
    accounts: &HashMap<String, CodexAccountData>,
) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for account in accounts.values() {
        map.entry(account.chatgpt_account_id.clone())
            .or_insert_with(Vec::new)
            .push(account.credential_id.clone());
    }
    map
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingRemap {
    Keep,
    Replace(String),
    Unbind,
}

fn remap_managed_account_binding(
    account_id: &str,
    valid_credential_ids: &std::collections::HashSet<String>,
    chatgpt_to_credentials: &HashMap<String, Vec<String>>,
) -> BindingRemap {
    if valid_credential_ids.contains(account_id) {
        return BindingRemap::Keep;
    }
    match chatgpt_to_credentials.get(account_id) {
        Some(ids) if ids.len() == 1 => BindingRemap::Replace(ids[0].clone()),
        _ => BindingRemap::Unbind,
    }
}

fn remap_codex_provider_bindings(chatgpt_to_credentials: &HashMap<String, Vec<String>>) {
    let Ok(mut config) = crate::app_config::MultiAppConfig::load() else {
        return;
    };
    let Some(manager) = config.get_manager_mut(&crate::app_config::AppType::Codex) else {
        return;
    };
    let valid: std::collections::HashSet<String> =
        chatgpt_to_credentials.values().flatten().cloned().collect();
    let mut changed = false;
    for provider in manager.providers.values_mut() {
        let Some(meta) = provider.meta.as_mut() else {
            continue;
        };
        let Some(binding) = meta.auth_binding.as_mut() else {
            continue;
        };
        if binding.source != crate::provider::AuthBindingSource::ManagedAccount
            || binding.auth_provider.as_deref() != Some("codex_oauth")
        {
            continue;
        }
        let Some(account_id) = binding.account_id.clone() else {
            continue;
        };
        match remap_managed_account_binding(&account_id, &valid, chatgpt_to_credentials) {
            BindingRemap::Keep => {}
            BindingRemap::Replace(credential_id) => {
                binding.account_id = Some(credential_id);
                changed = true;
            }
            BindingRemap::Unbind => {
                meta.auth_binding = None;
                changed = true;
            }
        }
    }
    if changed {
        let _ = config.save();
    }
}

/// Codex OAuth 状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexOAuthStatus {
    pub accounts: Vec<GitHubAccount>,
    pub default_account_id: Option<String>,
    pub authenticated: bool,
    pub username: Option<String>,
}

// ==================== 工具函数 ====================

/// 解析 OpenAI Device Code 响应中的 interval 字段
///
/// 服务端可能返回字符串或数字，需要兼容
fn parse_interval(value: Option<&serde_json::Value>) -> u64 {
    let raw = match value {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(5),
        Some(serde_json::Value::String(s)) => s.parse::<u64>().unwrap_or(5),
        _ => 5,
    };
    raw.max(1) + POLLING_SAFETY_MARGIN_SECS
}

/// 从 expires_in（秒）计算过期时间戳（毫秒）
fn compute_expires_at_ms(expires_in: Option<i64>) -> i64 {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let secs = expires_in.unwrap_or(3600);
    now_ms + secs * 1000
}

/// 解析 JWT 中的 claims
fn parse_jwt_claims(token: &str) -> Option<IdTokenClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    serde_json::from_slice(&decoded).ok()
}

/// 从 token 响应中提取 (account_id, email)
fn extract_identity_from_tokens(tokens: &OAuthTokenResponse) -> (Option<String>, Option<String>) {
    let mut account_id: Option<String> = None;
    let mut email: Option<String> = None;

    if let Some(id_token) = tokens.id_token.as_deref() {
        if let Some(claims) = parse_jwt_claims(id_token) {
            account_id = claims
                .chatgpt_account_id
                .clone()
                .or_else(|| {
                    claims
                        .openai_auth
                        .as_ref()
                        .and_then(|a| a.chatgpt_account_id.clone())
                })
                .or_else(|| claims.organizations.first().and_then(|o| o.id.clone()));
            email = claims.email.clone();
        }
    }

    if account_id.is_none() {
        if let Some(claims) = parse_jwt_claims(&tokens.access_token) {
            account_id = claims
                .chatgpt_account_id
                .clone()
                .or_else(|| {
                    claims
                        .openai_auth
                        .as_ref()
                        .and_then(|a| a.chatgpt_account_id.clone())
                })
                .or_else(|| claims.organizations.first().and_then(|o| o.id.clone()));
            if email.is_none() {
                email = claims.email.clone();
            }
        }
    }

    (account_id, email)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn test_parse_interval_number() {
        let v = serde_json::Value::Number(serde_json::Number::from(5));
        assert_eq!(parse_interval(Some(&v)), 5 + POLLING_SAFETY_MARGIN_SECS);
    }

    #[test]
    fn test_parse_interval_string() {
        let v = serde_json::Value::String("10".to_string());
        assert_eq!(parse_interval(Some(&v)), 10 + POLLING_SAFETY_MARGIN_SECS);
    }

    #[test]
    fn test_parse_interval_default() {
        assert_eq!(parse_interval(None), 5 + POLLING_SAFETY_MARGIN_SECS);
    }

    #[test]
    fn test_parse_interval_min() {
        let v = serde_json::Value::Number(serde_json::Number::from(0));
        // 0 应被提升到 1
        assert_eq!(parse_interval(Some(&v)), 1 + POLLING_SAFETY_MARGIN_SECS);
    }

    #[test]
    fn test_compute_expires_at_ms() {
        let result = compute_expires_at_ms(Some(3600));
        let now = chrono::Utc::now().timestamp_millis();
        // 应在未来约 3600 秒处（允许少量误差）
        assert!(result > now + 3500 * 1000);
        assert!(result < now + 3700 * 1000);
    }

    #[test]
    fn test_compute_expires_at_ms_default() {
        let result = compute_expires_at_ms(None);
        let now = chrono::Utc::now().timestamp_millis();
        assert!(result > now);
    }

    #[test]
    fn test_cached_token_expiring_soon() {
        let now = chrono::Utc::now().timestamp_millis();
        // 30 秒后过期 - 在缓冲期内
        let expiring = CachedAccessToken {
            token: "t".to_string(),
            expires_at_ms: now + 30_000,
        };
        assert!(expiring.is_expiring_soon());

        // 1 小时后过期 - 不在缓冲期内
        let valid = CachedAccessToken {
            token: "t".to_string(),
            expires_at_ms: now + 3_600_000,
        };
        assert!(!valid.is_expiring_soon());
    }

    #[test]
    fn test_parse_jwt_claims_invalid() {
        assert!(parse_jwt_claims("not-a-jwt").is_none());
        assert!(parse_jwt_claims("only.two").is_none());
    }

    #[test]
    fn test_parse_jwt_claims_valid() {
        // Header: {"alg":"none"}
        // Payload: {"chatgpt_account_id":"acc-123","email":"test@example.com"}
        // Signature: empty
        let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = URL_SAFE_NO_PAD
            .encode(b"{\"chatgpt_account_id\":\"acc-123\",\"email\":\"test@example.com\"}");
        let jwt = format!("{header}.{payload}.");
        let claims = parse_jwt_claims(&jwt).unwrap();
        assert_eq!(claims.chatgpt_account_id.as_deref(), Some("acc-123"));
        assert_eq!(claims.email.as_deref(), Some("test@example.com"));
    }

    #[test]
    fn test_parse_jwt_claims_organizations_fallback() {
        let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = URL_SAFE_NO_PAD.encode(b"{\"organizations\":[{\"id\":\"org-456\"}]}");
        let jwt = format!("{header}.{payload}.");
        let claims = parse_jwt_claims(&jwt).unwrap();
        assert_eq!(
            claims
                .organizations
                .first()
                .and_then(|o| o.id.clone())
                .as_deref(),
            Some("org-456")
        );
    }

    #[tokio::test]
    async fn test_manager_initial_state() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        assert!(!manager.is_authenticated().await);
        assert!(manager.list_accounts().await.is_empty());
    }

    #[tokio::test]
    async fn test_manager_save_and_load() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_path_buf();
        let credential_id;
        {
            let manager = CodexOAuthManager::new(path.clone());
            let account = manager
                .add_account_internal(
                    "acc-123".to_string(),
                    "rt-secret".to_string(),
                    Some("user@example.com".to_string()),
                    "at-secret".to_string(),
                    Some(3600),
                )
                .await
                .unwrap();
            credential_id = account.id.clone();
            assert_ne!(account.id, "acc-123");
            assert_eq!(account.chatgpt_account_id.as_deref(), Some("acc-123"));
            let json = serde_json::to_string(&account).unwrap();
            assert!(!json.contains("rt-secret"));
            assert!(!json.contains("at-secret"));
        }

        let manager2 = CodexOAuthManager::new(path);
        let accounts = manager2.list_accounts().await;
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, credential_id);
        assert_eq!(accounts[0].chatgpt_account_id.as_deref(), Some("acc-123"));
    }

    #[tokio::test]
    async fn test_remove_account() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());

        let first = manager
            .add_account_internal(
                "acc-123".to_string(),
                "rt".to_string(),
                Some("a@example.com".to_string()),
                "at".to_string(),
                Some(3600),
            )
            .await
            .unwrap();
        let second = manager
            .add_account_internal(
                "acc-456".to_string(),
                "rt2".to_string(),
                Some("b@example.com".to_string()),
                "at2".to_string(),
                Some(3600),
            )
            .await
            .unwrap();

        manager.remove_account(&first.id).await.unwrap();
        let accounts = manager.list_accounts().await;
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, second.id);
    }

    #[tokio::test]
    async fn same_workspace_two_users_coexist_under_distinct_credential_ids() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        let alice = manager
            .add_account_internal(
                "team-workspace".to_string(),
                "alice-rt".to_string(),
                Some("alice@example.com".to_string()),
                "alice-at".to_string(),
                Some(3600),
            )
            .await
            .unwrap();
        let bob = manager
            .add_account_internal(
                "team-workspace".to_string(),
                "bob-rt".to_string(),
                Some("bob@example.com".to_string()),
                "bob-at".to_string(),
                Some(3600),
            )
            .await
            .unwrap();
        assert_ne!(alice.id, bob.id);
        assert_eq!(alice.chatgpt_account_id, bob.chatgpt_account_id);
        let listed = manager.list_accounts().await;
        assert_eq!(listed.len(), 2);
        assert_eq!(
            manager.chatgpt_account_id_for(&alice.id).await.as_deref(),
            Some("team-workspace")
        );
    }

    #[tokio::test]
    async fn v1_store_migrates_to_credential_ids_and_keeps_backup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("codex_oauth_auth.json");
        let v1 = serde_json::json!({
            "version": 1,
            "default_account_id": "ws-shared",
            "accounts": {
                "ws-shared": {
                    "account_id": "ws-shared",
                    "email": "user@example.com",
                    "refresh_token": "legacy-rt",
                    "authenticated_at": 1
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&v1).unwrap()).unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        let backup = path.with_extension("json.v1.bak");
        assert!(backup.exists());
        let migrated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(migrated["version"], 2);
        let accounts = migrated["accounts"].as_object().unwrap();
        assert_eq!(accounts.len(), 1);
        let credential_id = accounts.keys().next().unwrap();
        assert_ne!(credential_id.as_str(), "ws-shared");
        assert_eq!(accounts[credential_id]["chatgpt_account_id"], "ws-shared");
        let accounts = manager.list_accounts().await;
        assert_eq!(accounts.len(), 1);
        assert_ne!(accounts[0].id, "ws-shared");
        assert_eq!(accounts[0].chatgpt_account_id.as_deref(), Some("ws-shared"));
    }

    #[test]
    fn account_debug_redacts_refresh_token() {
        let data = CodexAccountData {
            credential_id: "cred".to_string(),
            chatgpt_account_id: "ws".to_string(),
            email: None,
            refresh_token: "super-secret".to_string(),
            authenticated_at: 1,
        };
        let rendered = format!("{data:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn unique_workspace_binding_remaps_ambiguous_unbinds() {
        let mut map = HashMap::new();
        map.insert("ws-unique".to_string(), vec!["cred-a".to_string()]);
        map.insert(
            "ws-shared".to_string(),
            vec!["cred-b".to_string(), "cred-c".to_string()],
        );
        let valid: std::collections::HashSet<String> = map.values().flatten().cloned().collect();

        assert_eq!(
            remap_managed_account_binding("cred-a", &valid, &map),
            BindingRemap::Keep
        );
        assert_eq!(
            remap_managed_account_binding("ws-unique", &valid, &map),
            BindingRemap::Replace("cred-a".to_string())
        );
        assert_eq!(
            remap_managed_account_binding("ws-shared", &valid, &map),
            BindingRemap::Unbind
        );
        assert_eq!(
            remap_managed_account_binding("missing", &valid, &map),
            BindingRemap::Unbind
        );
    }

    #[test]
    fn corrupt_store_does_not_count_as_loaded() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("codex_oauth_auth.json"), "{not-json").unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        assert!(!manager.store_loaded());
    }

    #[test]
    fn missing_store_is_an_empty_successful_load() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        assert!(manager.store_loaded());
    }

    #[tokio::test]
    async fn v2_reload_is_idempotent_and_keeps_the_v1_backup() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("codex_oauth_auth.json");
        let v1 = serde_json::json!({
            "version": 1,
            "default_account_id": "ws-shared",
            "accounts": {
                "ws-shared": {
                    "account_id": "ws-shared",
                    "email": "user@example.com",
                    "refresh_token": "legacy-rt",
                    "authenticated_at": 1
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&v1).unwrap()).unwrap();
        let first = CodexOAuthManager::new(temp.path().to_path_buf());
        let first_id = first.list_accounts().await[0].id.clone();
        let backup = fs::read_to_string(path.with_extension("json.v1.bak")).unwrap();
        let second = CodexOAuthManager::new(temp.path().to_path_buf());
        assert_eq!(second.list_accounts().await[0].id, first_id);
        assert_eq!(
            fs::read_to_string(path.with_extension("json.v1.bak")).unwrap(),
            backup
        );
        assert!(backup.contains("ws-shared"));
        assert!(second.store_loaded());
    }

    #[tokio::test]
    async fn missing_bound_credential_does_not_yield_another_routing_id() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        let alice = manager
            .add_account_internal(
                "ws".to_string(),
                "alice-rt".to_string(),
                Some("alice@example.com".to_string()),
                "alice-at".to_string(),
                Some(3600),
            )
            .await
            .unwrap();
        assert!(manager.chatgpt_account_id_for("deleted-id").await.is_none());
        assert_eq!(
            manager.chatgpt_account_id_for(&alice.id).await.as_deref(),
            Some("ws")
        );
        assert!(matches!(
            manager.get_valid_token_for_account("deleted-id").await,
            Err(CodexOAuthError::AccountNotFound(_))
        ));
    }

    #[test]
    fn oauth_status_errors_do_not_embed_response_bodies() {
        let failed = CodexOAuthError::TokenFetchFailed("OAuth 轮询失败 (401)".to_string());
        let rendered = failed.to_string();
        assert!(!rendered.contains("access_token"));
        assert!(!rendered.contains("refresh_token"));
        assert!(!rendered.contains("{"));
        assert!(rendered.contains("401"));
    }
}
