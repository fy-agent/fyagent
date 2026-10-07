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
use uuid::Uuid;

use super::copilot_auth::{GitHubAccount, GitHubDeviceCodeResponse};
use crate::services::managed_auth::providers::openai::{
    self, OpenAiOAuthEndpoints, OpenAiOAuthError,
};

#[allow(dead_code)]
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
#[allow(dead_code)]
const DEVICE_AUTH_USERCODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
#[allow(dead_code)]
const DEVICE_AUTH_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
#[allow(dead_code)]
const OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";

/// Device Code 验证 URL（向用户展示）
const DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";

#[allow(dead_code)]
const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";

/// Token 刷新提前量（毫秒）
const TOKEN_REFRESH_BUFFER_MS: i64 = 60_000;

#[allow(dead_code)]
const DEVICE_CODE_DEFAULT_EXPIRES_IN: u64 = 900;

#[allow(dead_code)]
const POLLING_SAFETY_MARGIN_SECS: u64 = 3;

#[allow(dead_code)]
const CODEX_USER_AGENT: &str = "fyagent-codex-oauth";
#[allow(dead_code)]
const OAUTH_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

#[allow(dead_code)]
async fn send_bounded(
    request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, CodexOAuthError> {
    tokio::time::timeout(OAUTH_HTTP_TIMEOUT, request.send())
        .await
        .map_err(|_| CodexOAuthError::NetworkError("OAuth 请求超时".to_string()))?
        .map_err(CodexOAuthError::from)
}

// Shared by model discovery and generation: ChatGPT gates models by this
// client identity. gpt-6-astra requires >= 0.153.0 in the rust-v0.153.4 catalog.
// Bump together when a new model raises its minimal_client_version.
pub(crate) const CODEX_OAUTH_ORIGINATOR: &str = "codex_cli_rs";
pub(crate) const CODEX_OAUTH_CLIENT_VERSION: &str = "0.153.4";

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

    #[error("codex_oauth_duplicate_account")]
    DuplicateAccount,

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

impl From<OpenAiOAuthError> for CodexOAuthError {
    fn from(error: OpenAiOAuthError) -> Self {
        match error {
            OpenAiOAuthError::AuthorizationPending => Self::AuthorizationPending,
            OpenAiOAuthError::AccessDenied => Self::AccessDenied,
            OpenAiOAuthError::ExpiredToken => Self::ExpiredToken,
            OpenAiOAuthError::TokenFetchFailed => {
                Self::TokenFetchFailed("token exchange failed".into())
            }
            OpenAiOAuthError::RefreshTokenInvalid => Self::RefreshTokenInvalid,
            OpenAiOAuthError::NetworkError => Self::NetworkError("oauth network failed".into()),
            OpenAiOAuthError::ParseError => Self::ParseError("oauth parse failed".into()),
            OpenAiOAuthError::IoError => Self::IoError("oauth io failed".into()),
            OpenAiOAuthError::Cancelled => Self::Cancelled,
        }
    }
}

/// OpenAI Device Code 响应
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct DeviceCodeResponse {
    device_auth_id: String,
    user_code: String,
    #[serde(default)]
    interval: Option<serde_json::Value>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// OpenAI Device Code 轮询响应（成功）
#[allow(dead_code)]
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
    #[serde(default, rename = "https://api.openai.com/auth")]
    openai_auth: Option<OpenAiAuthClaim>,
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
    /// 获取（刷新）时间戳（毫秒）。用于写入托管 auth.json 的 `last_refresh`，
    /// 使其如实反映 access_token 的真实获取时间，而非写盘时刻——否则 Codex CLI
    /// 会误判一个旧 token 是刚刷新的。
    obtained_at_ms: i64,
}

impl CachedAccessToken {
    fn is_expiring_soon(&self) -> bool {
        let now = chrono::Utc::now().timestamp_millis();
        self.expires_at_ms - now < TOKEN_REFRESH_BUFFER_MS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshTokenAdoptionMode {
    /// Normal CLI synchronization: different token material must carry a
    /// strictly newer live timestamp before it can replace manager state.
    TimestampChecked,
    /// The OAuth server has just rejected the manager refresh token. A
    /// different same-account token observed on disk is therefore the only
    /// viable recovery generation and may bypass timestamp ambiguity.
    RejectedManagerToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshTokenAdoptionOutcome {
    /// The live and manager token material already describe the same
    /// generation. `state_changed` only reflects timestamp bookkeeping.
    Synchronized { state_changed: bool },
    /// Different live token material was accepted as the newer generation.
    Adopted,
    /// Different live token material carries a timestamp strictly older than
    /// the manager generation and may therefore be overwritten or removed.
    ProvablyOlder,
    /// Different token material could not be ordered safely. Callers that are
    /// about to overwrite/delete auth.json must abort instead of guessing.
    Ambiguous,
    /// The account is not owned by this manager.
    NotManaged,
}

/// Keep a deleted account distinct from an existing account with no managed live token.
pub(crate) enum CodexLiveAuthSwitchGuard {
    ExistingAccount(Option<String>),
    MissingAccount,
}

impl CodexLiveAuthSwitchGuard {
    pub(crate) fn ensure_unchanged(&self, account_id: &str) -> Result<(), crate::error::AppError> {
        if let Self::ExistingAccount(Some(token)) = self {
            crate::codex_config::ensure_codex_live_auth_unchanged_for_managed_account(
                account_id, token,
            )?;
        }
        Ok(())
    }

    pub(crate) fn clear_outgoing(&self, account_id: &str) -> Result<(), crate::error::AppError> {
        match self {
            Self::ExistingAccount(token) => {
                crate::codex_config::clear_codex_live_auth_for_managed_account_if_unchanged(
                    account_id,
                    token.as_deref(),
                )
            }
            Self::MissingAccount => {
                crate::codex_config::clear_codex_managed_oauth_live_auth_marker_for_account(
                    account_id,
                )
            }
        }
    }
}

impl RefreshTokenAdoptionOutcome {
    fn state_changed(self) -> bool {
        matches!(
            self,
            Self::Synchronized {
                state_changed: true
            } | Self::Adopted
        )
    }
}

/// 进行中的 Device Code 条目，带过期时间以便清理放弃的登录流程
#[derive(Debug, Clone)]
struct PendingDeviceCode {
    user_code: String,
    /// Unix 毫秒时间戳，超时后可清理
    expires_at_ms: i64,
    /// 仅重新认证时设置；登录完成后原位更新该本地账号，保留 provider 绑定。
    target_account_id: Option<String>,
    /// 同一目标账号只允许最新启动的重新认证流程提交。
    target_generation: Option<u64>,
}

#[derive(Default)]
struct AccountLoginContext<'a> {
    target_account_id: Option<&'a str>,
    pending_device_code: Option<&'a str>,
    target_generation: Option<u64>,
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
    /// ChatGPT id_token（JWT，持久化）。用于让托管写入的 Codex auth.json
    /// 与原生浏览器登录保持一致的 tokens 字段形状；刷新时若返回新值则更新。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// 最近一次取得或采纳这组 OAuth token 的时间。用于在 Codex CLI 与
    /// cc-switch 都可能轮换 refresh_token 时拒绝从 live 采纳更旧的一代。
    #[serde(default)]
    pub token_updated_at_ms: i64,
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
    /// 普通 token 解析/采纳持读锁，账号删除/清空持写锁。删除因此会等待
    /// 已在飞 refresh 完成，也不会因过早清理 refresh_locks 产生第二把账号锁。
    lifecycle_lock: Arc<RwLock<()>>,
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
            lifecycle_lock: Arc::new(RwLock::new(())),
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
    pub async fn start_device_flow(
        &self,
        target_account_id: Option<&str>,
    ) -> Result<GitHubDeviceCodeResponse, CodexOAuthError> {
        log::info!("[CodexOAuth] 启动 Device Code 流程");
        let login_epoch = self.login_epoch.load(Ordering::Acquire);
        let target_account_id = target_account_id
            .map(str::trim)
            .filter(|account_id| !account_id.is_empty())
            .map(str::to_string);
        let target_generation = if let Some(account_id) = target_account_id.as_deref() {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
            drop(accounts);
            let generation = self
                .next_target_login_generation
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
            let mut generations = self.target_login_generations.write().await;
            generations
                .entry(account_id.to_string())
                .and_modify(|current| *current = (*current).max(generation))
                .or_insert(generation);
            Some(generation)
        } else {
            None
        };

        let device = openai::request_device_usercode(&OpenAiOAuthEndpoints::production()).await?;
        let interval = device.interval;
        let expires_in = device.expires_in;
        let expires_at_ms = chrono::Utc::now().timestamp_millis() + (expires_in as i64) * 1000;

        self.register_pending_device_code(
            device.device_auth_id.clone(),
            device.user_code.clone(),
            expires_at_ms,
            login_epoch,
            target_account_id,
            target_generation,
        )
        .await?;

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

    async fn register_pending_device_code(
        &self,
        device_auth_id: String,
        user_code: String,
        expires_at_ms: i64,
        login_epoch: u64,
        target_account_id: Option<String>,
        target_generation: Option<u64>,
    ) -> Result<(), CodexOAuthError> {
        let mut pending = self.pending_device_codes.write().await;
        if self.login_epoch.load(Ordering::Acquire) != login_epoch {
            return Err(CodexOAuthError::ExpiredToken);
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        pending.retain(|_, entry| entry.expires_at_ms > now_ms);
        pending.insert(
            device_auth_id,
            PendingDeviceCode {
                user_code,
                expires_at_ms,
                target_account_id,
                target_generation,
            },
        );
        Ok(())
    }

    pub async fn cancel_device_flow(&self, device_code: &str) -> bool {
        self.pending_device_codes
            .write()
            .await
            .remove(device_code)
            .is_some()
    }

    /// 轮询 Device Code 状态
    ///
    /// 接收 device_code（即 device_auth_id），返回 Some(account) 表示授权成功
    pub async fn poll_for_token<BeforeCommit, CommitFuture, CommitGuard>(
        &self,
        device_code: &str,
        before_commit: BeforeCommit,
    ) -> Result<Option<GitHubAccount>, CodexOAuthError>
    where
        BeforeCommit: FnOnce() -> CommitFuture,
        CommitFuture: std::future::Future<Output = CommitGuard>,
    {
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

        let user_code = entry.user_code.clone();

        log::debug!("[CodexOAuth] 轮询 Device Code");

        let (authorization_code, code_verifier) = openai::poll_device_authorization(
            &OpenAiOAuthEndpoints::production(),
            device_code,
            &user_code,
        )
        .await?;

        log::info!("[CodexOAuth] 用户已授权，正在换取 OAuth Token");

        let tokens = self
            .exchange_code_for_tokens(&authorization_code, &code_verifier)
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

        let id_token = tokens
            .id_token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| {
                CodexOAuthError::TokenFetchFailed(
                    "登录响应缺少 id_token，账号未保存，请重新登录".to_string(),
                )
            })?;
        if crate::codex_config::extract_codex_id_token_subject(&id_token).is_none() {
            return Err(CodexOAuthError::TokenFetchFailed(
                "登录响应无法确认稳定用户身份，账号未保存，请重新登录".to_string(),
            ));
        }

        let obtained_at_ms = chrono::Utc::now().timestamp_millis();
        // Provider switching and managed live-auth writes use the same guard.
        // Acquire it only after the network exchange succeeds so ordinary
        // authorization-pending polls never block provider operations.
        let _commit_guard = before_commit().await;
        // 登录提交与该账号的 refresh/adopt 共用一把 generation 锁；账号和
        // access cache 一次写入，旧刷新响应因此不能覆盖新登录链。
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
        let grant = openai::exchange_authorization_code(
            &OpenAiOAuthEndpoints::production(),
            code,
            code_verifier,
            openai::OPENAI_DEVICE_REDIRECT_URI,
        )
        .await?;
        Ok(OAuthTokenResponse {
            access_token: grant.access_token,
            refresh_token: grant.refresh_token,
            id_token: grant.id_token,
            expires_in: grant.expires_in,
        })
    }

    /// Refresh an OpenAI grant. Callers must already own the unique refresh
    /// lease for the credential lineage.
    pub(crate) async fn refresh_with_token(
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, CodexOAuthError> {
        let grant = openai::refresh_oauth_grant(refresh_token).await?;
        Ok(OAuthTokenResponse {
            access_token: grant.access_token,
            refresh_token: grant.refresh_token,
            id_token: grant.id_token,
            expires_in: grant.expires_in,
        })
    }

    // ==================== Token 获取（含自动刷新） ====================

    /// 获取指定账号的有效 access_token（必要时自动刷新）
    pub async fn get_valid_token_for_account(
        &self,
        account_id: &str,
    ) -> Result<String, CodexOAuthError> {
        let _lifecycle = self.lifecycle_lock.read().await;
        self.ensure_account_ready_for_use(account_id).await?;
        Ok(self.resolve_valid_cached_token(account_id).await?.token)
    }

    async fn ensure_account_ready_for_use(&self, account_id: &str) -> Result<(), CodexOAuthError> {
        let accounts = self.accounts.read().await;
        let account = accounts
            .get(account_id)
            .ok_or_else(|| CodexOAuthError::AccountNotFound(account_id.to_string()))?;
        if account
            .id_token
            .as_deref()
            .and_then(crate::codex_config::extract_codex_id_token_user_identity)
            .is_none()
            || account.chatgpt_account_id.is_none()
        {
            return Err(CodexOAuthError::ParseError(format!(
                "当前账号缺少 id_token 中可证明的用户身份或 workspace，请重新认证"
            )));
        }
        Ok(())
    }

    async fn read_managed_live_auth_refresh_for_account(
        &self,
        account_id: &str,
    ) -> Result<Option<(String, Option<String>, Option<i64>)>, CodexOAuthError> {
        let (managed_id_token, managed_workspace) = {
            let accounts = self.accounts.read().await;
            let account = accounts
                .get(account_id)
                .ok_or_else(|| CodexOAuthError::AccountNotFound(account_id.to_string()))?;
            let workspace = account.chatgpt_account_id.clone().ok_or_else(|| {
                CodexOAuthError::ParseError(format!(
                    "当前账号缺少 workspace 身份，请重新认证"
                ))
            })?;
            (account.id_token.clone(), workspace)
        };
        let Some(live_refresh) =
            crate::codex_config::read_codex_live_auth_refresh_for_managed_account(
                account_id,
                managed_id_token.as_deref(),
            )
            .map_err(|error| CodexOAuthError::TokenFetchFailed(error.to_string()))?
        else {
            return Ok(None);
        };

        if managed_workspace != live_refresh.chatgpt_account_id {
            return Err(CodexOAuthError::TokenFetchFailed(format!(
                "Codex OAuth 账号的 workspace 与磁盘凭据不一致，本次操作已取消"
            )));
        }
        Ok(Some((
            live_refresh.refresh_token,
            live_refresh.id_token,
            live_refresh.last_refresh_ms,
        )))
    }

    /// 解析账号的有效缓存 token（含真实获取时间），必要时刷新。
    ///
    /// 返回完整 `CachedAccessToken`，使 token 与其 `obtained_at_ms` 天然配套（写托管
    /// auth.json 的 `last_refresh` 直接取用），避免分两次读缓存造成的错配。
    ///
    /// 并发正确性：调用方持 lifecycle 读锁；刷新在 account refresh mutex 下先短暂
    /// 提交 accounts → access_tokens，释放这些锁后再持久化。`save_to_disk` 的实际
    /// 持久化锁序是 storage_lock → accounts/default。remove/clear 持 lifecycle 写锁，
    /// 因而会等待在飞刷新并阻断同 account_id 的 ABA 重建。
    async fn resolve_valid_cached_token(
        &self,
        account_id: &str,
    ) -> Result<CachedAccessToken, CodexOAuthError> {
        // 快路径：确认账号存在后读缓存
        {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
            let tokens = self.access_tokens.read().await;
            if let Some(cached) = tokens.get(account_id) {
                if !cached.is_expiring_soon() {
                    return Ok(cached.clone());
                }
            }
        }

        log::info!("[CodexOAuth] 所选账号的 access_token 需要刷新");

        let refresh_lock = self.get_refresh_lock(account_id).await;
        let _guard = refresh_lock.lock().await;
        self.resolve_valid_cached_token_under_lock(account_id).await
    }

    /// Resolve a token while the caller owns this account's refresh mutex.
    /// Keeping this separate lets the full auth-bundle path hold one generation
    /// lock across access/id/refresh reads without recursively locking the mutex.
    async fn resolve_valid_cached_token_under_lock(
        &self,
        account_id: &str,
    ) -> Result<CachedAccessToken, CodexOAuthError> {
        // Codex CLI may have advanced the shared refresh-token generation since
        // this manager last used the account. Reload it under the same per-account
        // lock before deciding whether a network refresh is necessary.
        if let Some((live_refresh, live_id_token, live_last_refresh_ms)) = self
            .read_managed_live_auth_refresh_for_account(account_id)
            .await?
        {
            self.adopt_account_refresh_token_under_lock(
                account_id,
                live_refresh,
                live_id_token,
                live_last_refresh_ms,
                RefreshTokenAdoptionMode::TimestampChecked,
            )
            .await?;
        }

        // double-check（同样在 accounts 读锁下）
        {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
            let tokens = self.access_tokens.read().await;
            if let Some(cached) = tokens.get(account_id) {
                if !cached.is_expiring_soon() {
                    return Ok(cached.clone());
                }
            }
        }

        let mut refresh_token = {
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
                refresh_token = live_refresh;
                self.refresh_with_token(&refresh_token).await?
            }
            result => result?,
        };

        let obtained_at_ms = chrono::Utc::now().timestamp_millis();

        // 如果服务端返回了新的 refresh_token 或 id_token，更新存储
        let mut needs_save = false;
        let (stored_refresh_token, stored_id_token, chatgpt_account_id) = {
            let mut accounts = self.accounts.write().await;
            let account = accounts
                .get_mut(account_id)
                .ok_or_else(|| CodexOAuthError::AccountNotFound(account_id.to_string()))?;
            // Device re-login and CLI-token adoption use the same account lock,
            // but keep a generation CAS here as defense in depth: a response for
            // R0 must never overwrite a newly committed R1/N0 chain.
            if account.refresh_token != refresh_token {
                return Err(CodexOAuthError::TokenFetchFailed(
                    "账号凭据已更新，已丢弃旧刷新响应".to_string(),
                ));
            }
            if account.apply_refreshed_tokens(&new_tokens) {
                needs_save = true;
            }
            // 刷新使用 openid scope，正常会返回新 id_token；为空则视为缺失，
            // 保留旧值而非覆盖（旧值的 claims 仍可用于账号/套餐显示）。
            if let Some(new_id_token) = new_tokens
                .id_token
                .clone()
                .filter(|token| !token.trim().is_empty())
            {
                if account.id_token.as_deref() != Some(new_id_token.as_str()) {
                    account.id_token = Some(new_id_token);
                    needs_save = true;
                }
            }
            if account.token_updated_at_ms != obtained_at_ms {
                account.token_updated_at_ms = obtained_at_ms;
                needs_save = true;
            }
            (
                account.refresh_token.clone(),
                account.id_token.clone(),
                account.chatgpt_account_id.clone(),
            )
        };
        if needs_save {
            self.save_to_disk().await?;
        }
        let chatgpt_account_id = chatgpt_account_id.ok_or_else(|| {
            CodexOAuthError::ParseError(
                "无法从刷新后的 token 中提取 chatgpt_account_id".to_string(),
            )
        })?;

        let cached = CachedAccessToken {
            token: new_tokens.access_token.clone(),
            expires_at_ms: compute_expires_at_ms(new_tokens.expires_in),
            obtained_at_ms,
        };

        let last_refresh = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(obtained_at_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let refreshed_auth = crate::codex_config::codex_managed_oauth_auth_value(
            &chatgpt_account_id,
            &cached.token,
            stored_id_token.as_deref(),
            &stored_refresh_token,
            &last_refresh,
        );
        if let Err(err) = crate::codex_config::sync_codex_managed_oauth_live_auth_after_refresh(
            account_id,
            &refresh_token,
            &refreshed_auth,
        ) {
            // The manager token remains valid; a later provider write will
            // retry the live synchronization without rolling it back.
            log::warn!(
                "[CodexOAuth] 同步刷新后的 Codex live auth 失败（account={account_id}）: {err}"
            );
        }

        // 在 accounts 读锁下确认账号仍存在，再写缓存：与 remove/clear（持 accounts
        // 写锁并原子清缓存）互斥，杜绝把已删账号的 token 写回缓存。
        {
            let accounts = self.accounts.read().await;
            if !accounts.contains_key(account_id) {
                return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
            }
            let mut tokens = self.access_tokens.write().await;
            tokens.insert(account_id.to_string(), cached.clone());
        }

        Ok(cached)
    }

    /// 获取指定账号的有效 access_token 与 id_token（必要时自动刷新）
    ///
    /// id_token 用于让托管写入的 Codex auth.json 与原生浏览器登录保持
    /// 一致的 tokens 字段形状（仅托管绑定路径使用）。旧账号若无 id_token
    /// 会返回 `None`，前端据此提示重新登录。
    pub async fn get_valid_token_and_id_token_for_account(
        &self,
        account_id: &str,
    ) -> Result<(String, Option<String>), CodexOAuthError> {
        let bundle = self.get_valid_token_bundle_for_account(account_id).await?;
        Ok((bundle.access_token, bundle.id_token))
    }

    /// 获取写入托管 Codex `auth.json` 所需的完整可刷新 token 束
    /// （access_token + id_token + refresh_token）。
    ///
    /// 与仅返回 access_token 不同：写入 Codex CLI 的 auth.json 必须携带
    /// refresh_token，否则 CLI 在 access_token 过期后无法自刷新（详见托管直连
    /// 场景 “裸跑 codex”）。
    pub(crate) async fn get_valid_token_bundle_for_account(
        &self,
        account_id: &str,
    ) -> Result<ManagedTokenBundle, CodexOAuthError> {
        let _lifecycle = self.lifecycle_lock.read().await;
        self.ensure_account_ready_for_use(account_id).await?;
        let refresh_lock = self.get_refresh_lock(account_id).await;
        let _refresh_guard = refresh_lock.lock().await;

        // Resolve and read every persistent token field while holding the same
        // account generation lock. Otherwise an adoption between these reads
        // can create an invalid A0 + R1/ID1 mixed bundle.
        let cached = self
            .resolve_valid_cached_token_under_lock(account_id)
            .await?;

        // A managed bundle is about to overwrite auth.json. Re-read under the
        // same manager generation lock after token resolution so an ambiguous
        // same-account disk generation can never be hidden by a valid cached
        // access token. Keeping this check after resolution also preserves the
        // RefreshTokenInvalid recovery path: the server may disprove manager R0,
        // force-adopt disk R1, and only then produce a safe bundle.
        if let Some((live_refresh, live_id_token, live_last_refresh_ms)) = self
            .read_managed_live_auth_refresh_for_account(account_id)
            .await?
        {
            let outcome = self
                .adopt_account_refresh_token_under_lock(
                    account_id,
                    live_refresh,
                    live_id_token,
                    live_last_refresh_ms,
                    RefreshTokenAdoptionMode::TimestampChecked,
                )
                .await?;
            match outcome {
                RefreshTokenAdoptionOutcome::Synchronized { .. }
                | RefreshTokenAdoptionOutcome::ProvablyOlder => {}
                RefreshTokenAdoptionOutcome::Ambiguous => {
                    return Err(Self::ambiguous_live_refresh_error(account_id));
                }
                RefreshTokenAdoptionOutcome::Adopted => {
                    return Err(CodexOAuthError::TokenFetchFailed(format!(
                        "Codex CLI 账号的磁盘凭据在准备写入期间已刷新；为避免写入混合 token bundle，本次操作已取消，请重试"
                    )));
                }
                RefreshTokenAdoptionOutcome::NotManaged => {
                    return Err(CodexOAuthError::AccountNotFound(account_id.to_string()));
                }
            }
        }
        let last_refresh =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(cached.obtained_at_ms)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
        let (chatgpt_account_id, id_token, refresh_token) = {
            let accounts = self.accounts.read().await;
            let account = accounts
                .get(account_id)
                .ok_or_else(|| CodexOAuthError::AccountNotFound(account_id.to_string()))?;
            (
                account.chatgpt_account_id.clone().ok_or_else(|| {
                    CodexOAuthError::ParseError(
                        "账号缺少 chatgpt_account_id，请重新认证".to_string(),
                    )
                })?,
                account.id_token.clone(),
                account.refresh_token.clone(),
            )
        };
        Ok(ManagedTokenBundle {
            chatgpt_account_id,
            access_token: cached.token,
            id_token,
            refresh_token,
            last_refresh,
        })
    }

    /// 采纳（读回）Codex CLI 轮换后的 refresh_token / id_token。
    ///
    /// 托管账号以「完整 bundle」写入 auth.json 后，Codex CLI 会自行刷新并把新的
    /// refresh_token 回写 auth.json。切换回该 provider 前调用本方法，把盘上的最新
    /// refresh_token 采纳进本地存储，避免用陈腐 token 覆盖 CLI 的有效登录。
    ///
    /// 仅当账号确由本 manager 托管、且值确有变化时才更新并落盘；返回是否更新。
    pub async fn adopt_account_refresh_token(
        &self,
        account_id: &str,
        refresh_token: String,
        id_token: Option<String>,
        last_refresh_ms: Option<i64>,
    ) -> Result<bool, CodexOAuthError> {
        let _lifecycle = self.lifecycle_lock.read().await;
        let refresh_token = refresh_token.trim().to_string();
        if refresh_token.is_empty() {
            return Ok(false);
        }
        // 与该账号的刷新串行化：若一个 refresh 正持旧 refresh_token 在飞，避免它返回后
        // 覆盖我们刚采纳的 CLI 轮换值。
        let refresh_lock = self.get_refresh_lock(account_id).await;
        let _guard = refresh_lock.lock().await;
        self.adopt_account_refresh_token_under_lock(
            account_id,
            refresh_token,
            id_token,
            last_refresh_ms,
            RefreshTokenAdoptionMode::TimestampChecked,
        )
        .await
        .map(RefreshTokenAdoptionOutcome::state_changed)
    }

    fn ambiguous_live_refresh_error(_account_id: &str) -> CodexOAuthError {
        CodexOAuthError::TokenFetchFailed(
            "Codex CLI 账号的磁盘凭据已变化，但无法安全判断 refresh token 新旧；为避免覆盖或删除有效登录，本次操作已取消。请先在认证中心重新登录该账号；若仍失败，请移除后重新登录"
                .to_string(),
        )
    }

    /// Reconcile the same-account Codex CLI refresh generation before a
    /// provider transaction overwrites or removes live auth.json.
    ///
    /// For an existing account, carries the refresh token observed on disk.
    /// Callers compare it immediately before their live write/delete; the external Codex
    /// CLI does not participate in cc-switch's switch lock and may refresh in
    /// the adopt-to-write window.
    pub(crate) async fn prepare_live_auth_for_account_switch_away(
        &self,
        account_id: &str,
    ) -> Result<CodexLiveAuthSwitchGuard, CodexOAuthError> {
        let _lifecycle = self.lifecycle_lock.read().await;
        let refresh_lock = self.get_refresh_lock(account_id).await;
        let _guard = refresh_lock.lock().await;
        if !self.accounts.read().await.contains_key(account_id) {
            self.ensure_account_absent_from_store(account_id).await?;
            return Ok(CodexLiveAuthSwitchGuard::MissingAccount);
        }
        let Some((live_refresh, live_id_token, live_last_refresh_ms)) = self
            .read_managed_live_auth_refresh_for_account(account_id)
            .await?
        else {
            return Ok(CodexLiveAuthSwitchGuard::ExistingAccount(None));
        };

        let outcome = self
            .adopt_account_refresh_token_under_lock(
                account_id,
                live_refresh.clone(),
                live_id_token,
                live_last_refresh_ms,
                RefreshTokenAdoptionMode::TimestampChecked,
            )
            .await?;

        match outcome {
            RefreshTokenAdoptionOutcome::Synchronized { .. }
            | RefreshTokenAdoptionOutcome::Adopted
            | RefreshTokenAdoptionOutcome::ProvablyOlder => Ok(
                CodexLiveAuthSwitchGuard::ExistingAccount(Some(live_refresh)),
            ),
            RefreshTokenAdoptionOutcome::Ambiguous => {
                Err(Self::ambiguous_live_refresh_error(account_id))
            }
            RefreshTokenAdoptionOutcome::NotManaged => {
                Err(CodexOAuthError::AccountNotFound(account_id.to_string()))
            }
        }
    }

    /// Same as `adopt_account_refresh_token`, for callers already holding the
    /// per-account refresh lock.
    async fn adopt_account_refresh_token_under_lock(
        &self,
        account_id: &str,
        refresh_token: String,
        id_token: Option<String>,
        last_refresh_ms: Option<i64>,
        mode: RefreshTokenAdoptionMode,
    ) -> Result<RefreshTokenAdoptionOutcome, CodexOAuthError> {
        let incoming_id_token = id_token.filter(|token| !token.trim().is_empty());
        let mut changed = false;
        let mut material_replaced = false;
        let mut outcome;
        {
            let mut accounts = self.accounts.write().await;
            let Some(account) = accounts.get_mut(account_id) else {
                // 不是本 manager 托管的账号：不接管、不落盘。
                return Ok(RefreshTokenAdoptionOutcome::NotManaged);
            };

            // A manager refresh may already have advanced the token generation
            // while auth.json still contains the older one. Never roll that
            // state back during the preflight/write double-build sequence.
            let refresh_changed = account.refresh_token != refresh_token;
            let id_token_changed = incoming_id_token
                .as_ref()
                .is_some_and(|token| account.id_token.as_deref() != Some(token.as_str()));
            let material_changed = refresh_changed || id_token_changed;
            let manager_was_undated = account.token_updated_at_ms <= 0;
            // Once the manager has a dated generation, any different token
            // material must carry a *strictly newer* live timestamp. Equality is
            // ambiguous at millisecond precision and therefore cannot authorize
            // replacing the manager generation either. Stores upgraded from
            // before generation timestamps existed keep a different live
            // generation ambiguous across retries; only matching material may
            // establish a timestamp. The server-rejected mode is the sole
            // exception because it has disproved the manager generation.
            let observed_order =
                last_refresh_ms.map(|observed| observed.cmp(&account.token_updated_at_ms));
            let should_adopt = material_changed
                && (matches!(mode, RefreshTokenAdoptionMode::RejectedManagerToken)
                    || (!manager_was_undated
                        && matches!(observed_order, Some(std::cmp::Ordering::Greater))));

            if !material_changed {
                outcome = RefreshTokenAdoptionOutcome::Synchronized {
                    state_changed: false,
                };
            } else if should_adopt {
                if refresh_changed {
                    account.refresh_token = refresh_token;
                    changed = true;
                    material_replaced = true;
                }
                if let Some(id_token) = incoming_id_token {
                    if account.id_token.as_deref() != Some(id_token.as_str()) {
                        account.id_token = Some(id_token);
                        changed = true;
                        material_replaced = true;
                    }
                }
                outcome = RefreshTokenAdoptionOutcome::Adopted;
            } else if !manager_was_undated
                && matches!(observed_order, Some(std::cmp::Ordering::Less))
            {
                outcome = RefreshTokenAdoptionOutcome::ProvablyOlder;
            } else {
                outcome = RefreshTokenAdoptionOutcome::Ambiguous;
            }

            if matches!(outcome, RefreshTokenAdoptionOutcome::Adopted)
                && matches!(mode, RefreshTokenAdoptionMode::RejectedManagerToken)
            {
                let adopted_at = last_refresh_ms
                    .filter(|observed| *observed > account.token_updated_at_ms)
                    .unwrap_or_else(|| {
                        chrono::Utc::now()
                            .timestamp_millis()
                            .max(account.token_updated_at_ms.saturating_add(1))
                    });
                if account.token_updated_at_ms != adopted_at {
                    account.token_updated_at_ms = adopted_at;
                    changed = true;
                }
            } else if matches!(outcome, RefreshTokenAdoptionOutcome::Adopted) {
                if let Some(observed) = last_refresh_ms {
                    if account.token_updated_at_ms != observed {
                        account.token_updated_at_ms = observed;
                        changed = true;
                    }
                }
            } else if matches!(outcome, RefreshTokenAdoptionOutcome::Synchronized { .. }) {
                if manager_was_undated {
                    // Matching material establishes one generation, so dating
                    // it cannot turn an unresolved R0/R1 conflict into a false
                    // "live is older" decision on the next retry.
                    account.token_updated_at_ms = last_refresh_ms
                        .filter(|observed| *observed > 0)
                        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                    changed = true;
                } else if let Some(observed) = last_refresh_ms {
                    if observed > account.token_updated_at_ms {
                        account.token_updated_at_ms = observed;
                        changed = true;
                    }
                }
            }
            // 采纳了 CLI 轮换后的 refresh_token：与之配套的旧 access_token 可能已被
            // 服务端提前失效。在同一 accounts 写锁内（accounts -> access_tokens 顺序）
            // 清缓存，避免释放锁后被快路径读到旧 token；下次按新 refresh_token 重取。
            if material_replaced {
                self.access_tokens.write().await.remove(account_id);
            }

            if let RefreshTokenAdoptionOutcome::Synchronized { .. } = outcome {
                outcome = RefreshTokenAdoptionOutcome::Synchronized {
                    state_changed: changed,
                };
            }
        }
        if changed {
            self.save_to_disk().await?;
        }
        Ok(outcome)
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
        log::info!("[CodexOAuth] 移除所选账号");

        {
            // 在 accounts 写锁内原子清除该账号的 token 缓存（accounts -> access_tokens
            // 顺序），确保不存在「账号已删但缓存仍在」的窗口。
            let mut accounts = self.accounts.write().await;
            accounts.remove(account_id);
            self.access_tokens.write().await.remove(account_id);
        }
        {
            let mut locks = self.refresh_locks.write().await;
            locks.remove(account_id);
        }
        self.target_login_generations
            .write()
            .await
            .remove(account_id);

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
        let _lifecycle = self.lifecycle_lock.read().await;
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

        // Acquire lifecycle before storage. Refresh follows lifecycle(read) ->
        // account mutex -> storage, so this fixed order cannot deadlock and the
        // write guard guarantees no refresh can recreate live/disk state after
        // the clear has committed.
        let _lifecycle = self.lifecycle_lock.write().await;

        let accounts_to_clear = self
            .accounts
            .read()
            .await
            .iter()
            .map(|(account_id, account)| (account_id.clone(), account.id_token.clone()))
            .collect::<Vec<_>>();
        for (account_id, id_token) in &accounts_to_clear {
            crate::codex_config::prepare_codex_live_auth_for_managed_account_removal(
                account_id,
                id_token.as_deref(),
            )
            .map_err(|error| CodexOAuthError::TokenFetchFailed(error.to_string()))?;
        }
        for (account_id, _) in &accounts_to_clear {
            crate::codex_config::clear_codex_live_auth_for_managed_account(account_id)
                .map_err(|error| CodexOAuthError::IoError(error.to_string()))?;
        }

        // 与 save_to_disk 共用持久化锁：确保「清内存 + 删文件」相对于并发保存原子，
        // 不会被一个持有旧快照的 save 复活已清除的账号。
        let _persist = self.storage_lock.lock().await;

        {
            // 在 accounts 写锁内原子清除 accounts 与 token 缓存（accounts ->
            // access_tokens 顺序），杜绝「账号已清但缓存仍在」及并发 refresh 回填。
            let mut accounts = self.accounts.write().await;
            accounts.clear();
            self.access_tokens.write().await.clear();
        }
        {
            let mut default = self.default_account_id.write().await;
            *default = None;
        }
        {
            let mut locks = self.refresh_locks.write().await;
            locks.clear();
        }
        self.target_login_generations.write().await.clear();
        {
            let mut pending = self.pending_device_codes.write().await;
            self.login_epoch.fetch_add(1, Ordering::AcqRel);
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

    #[cfg(test)]
    pub(crate) async fn add_test_account_with_access_token(
        &self,
        account_id: &str,
        access_token: &str,
        id_token: Option<&str>,
    ) -> Result<(), CodexOAuthError> {
        self.add_test_account_with_workspace_and_access_token(
            account_id,
            account_id,
            access_token,
            id_token,
        )
        .await
    }

    #[cfg(test)]
    pub(crate) async fn add_test_account_with_user_identity(
        &self,
        account_id: &str,
        access_token: &str,
        subject: &str,
    ) -> Result<(), CodexOAuthError> {
        let id_token = crate::codex_config::test_codex_id_token(subject);
        self.add_test_account_with_access_token(account_id, access_token, Some(&id_token))
            .await
    }

    #[cfg(test)]
    pub(crate) async fn add_test_account_with_workspace_and_access_token(
        &self,
        account_id: &str,
        chatgpt_account_id: &str,
        access_token: &str,
        id_token: Option<&str>,
    ) -> Result<(), CodexOAuthError> {
        let obtained_at_ms = chrono::Utc::now().timestamp_millis();
        let data = CodexAccountData {
            account_id: account_id.to_string(),
            chatgpt_account_id: Some(chatgpt_account_id.to_string()),
            email: Some(format!("{account_id}@example.test")),
            refresh_token: "test-refresh-token".to_string(),
            authenticated_at: chrono::Utc::now().timestamp(),
            id_token: id_token.map(|token| token.to_string()),
            token_updated_at_ms: obtained_at_ms,
        };
        {
            let mut accounts = self.accounts.write().await;
            accounts.insert(account_id.to_string(), data);
            self.access_tokens.write().await.insert(
                account_id.to_string(),
                CachedAccessToken {
                    token: access_token.to_string(),
                    expires_at_ms: obtained_at_ms + 3_600_000,
                    obtained_at_ms,
                },
            );
        }
        {
            let mut default = self.default_account_id.write().await;
            if default.is_none() {
                *default = Some(account_id.to_string());
            }
        }
        self.save_to_disk().await?;

        Ok(())
    }

    #[cfg(test)]
    pub(crate) async fn test_cache_access_token(&self, account_id: &str, token: &str) {
        assert!(self.accounts.read().await.contains_key(account_id));
        let now = chrono::Utc::now().timestamp_millis();
        self.access_tokens.write().await.insert(
            account_id.to_string(),
            CachedAccessToken {
                token: token.to_string(),
                expires_at_ms: now + 3_600_000,
                obtained_at_ms: now,
            },
        );
    }

    #[cfg(test)]
    pub(crate) async fn test_refresh_token_for_account(&self, account_id: &str) -> Option<String> {
        self.accounts
            .read()
            .await
            .get(account_id)
            .map(|account| account.refresh_token.clone())
    }

    #[cfg(test)]
    pub(crate) async fn test_set_token_updated_at_ms(
        &self,
        account_id: &str,
        token_updated_at_ms: i64,
    ) {
        self.accounts
            .write()
            .await
            .get_mut(account_id)
            .expect("test account present")
            .token_updated_at_ms = token_updated_at_ms;
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
            id_token,
            token_updated_at_ms: now_ms,
        };

        let account = GitHubAccount::from(&data);

        // Linearize cancel/newer-flow against the actual commit, after waiting
        // for the account lock. Holding both guards through persistence means
        // cancellation cannot report success after this point, while a cancel
        // or newer generation that won the race makes this flow fail closed.
        let _pending_commit_guard = if let Some(device_code) = context.pending_device_code {
            let generations = self.target_login_generations.read().await;
            let mut pending_codes = self.pending_device_codes.write().await;
            let pending = pending_codes
                .get(device_code)
                .ok_or(CodexOAuthError::ExpiredToken)?;
            let generation_matches = match (
                pending.target_account_id.as_deref(),
                pending.target_generation,
            ) {
                (Some(account_id), Some(generation)) => {
                    context.target_account_id == Some(account_id)
                        && context.target_generation == Some(generation)
                        && generations.get(account_id) == Some(&generation)
                }
                (None, None) => {
                    context.target_account_id.is_none() && context.target_generation.is_none()
                }
                _ => false,
            };
            if pending.expires_at_ms <= chrono::Utc::now().timestamp_millis() || !generation_matches
            {
                return Err(CodexOAuthError::ExpiredToken);
            }
            pending_codes.remove(device_code);
            Some((generations, pending_codes))
        } else {
            None
        };

        // Persist a prospective snapshot before publishing new credentials to
        // readers. A failed atomic write therefore leaves the target account
        // and its access-token cache untouched.
        let _persist = self.storage_lock.lock().await;
        let mut persisted_accounts = self.accounts.read().await.clone();
        let new_identity = data
            .id_token
            .as_deref()
            .and_then(crate::codex_config::extract_codex_id_token_user_identity);
        let duplicate_exists = new_identity.as_deref().is_some_and(|new_identity| {
            persisted_accounts
                .iter()
                .filter(|(existing_id, _)| existing_id.as_str() != account_id.as_str())
                .any(|(_, existing)| {
                    existing
                        .chatgpt_account_id
                        .as_deref()
                        .unwrap_or(existing.account_id.as_str())
                        == data
                            .chatgpt_account_id
                            .as_deref()
                            .unwrap_or(data.account_id.as_str())
                        && existing
                            .id_token
                            .as_deref()
                            .and_then(crate::codex_config::extract_codex_id_token_user_identity)
                            .as_deref()
                            == Some(new_identity)
                })
        });
        if duplicate_exists {
            return Err(CodexOAuthError::DuplicateAccount);
        }
        persisted_accounts.insert(account_id.clone(), data.clone());
        let persisted_default = self
            .resolve_default_account_id()
            .await
            .or_else(|| Some(account_id.clone()));
        let store = CodexOAuthStore {
            version: 2,
            accounts: persisted_accounts,
            default_account_id: persisted_default,
        };
        let content = serde_json::to_string_pretty(&store)
            .map_err(|error| CodexOAuthError::ParseError(error.to_string()))?;
        self.write_store_atomic(&content)?;

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
        let mut default = self.default_account_id.write().await;
        if default.is_none() {
            *default = Some(account_id);
        }
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

    /// Validate a target binding without refreshing tokens or changing credentials.
    pub(crate) async fn ensure_account_exists(
        &self,
        account_id: &str,
    ) -> Result<(), CodexOAuthError> {
        let _lifecycle = self.lifecycle_lock.read().await;
        if self.accounts.read().await.contains_key(account_id) {
            return Ok(());
        }
        self.ensure_account_absent_from_store(account_id).await?;
        Err(CodexOAuthError::AccountUnavailable(account_id.to_string()))
    }

    // A failed load also leaves the manager empty. Only a valid persisted store
    // can distinguish deletion from unreadable credentials. The caller holds lifecycle_lock.
    async fn ensure_account_absent_from_store(
        &self,
        account_id: &str,
    ) -> Result<(), CodexOAuthError> {
        let _persist = self.storage_lock.lock().await;
        if !self.storage_path.try_exists()? {
            if self.accounts.read().await.is_empty() {
                return Ok(());
            }
            return Err(CodexOAuthError::TokenFetchFailed(
                "Codex 账号存储缺失但内存中仍有账号，请重启应用后重试".to_string(),
            ));
        }
        let raw: serde_json::Value = serde_json::from_str(&fs::read_to_string(&self.storage_path)?)
            .map_err(|error| CodexOAuthError::ParseError(error.to_string()))?;
        if !raw
            .get("accounts")
            .is_some_and(serde_json::Value::is_object)
        {
            return Err(CodexOAuthError::ParseError(
                "Codex 账号存储缺少有效 accounts 字段".to_string(),
            ));
        }
        let store: CodexOAuthStore = serde_json::from_value(raw)
            .map_err(|error| CodexOAuthError::ParseError(error.to_string()))?;
        if !matches!(store.version, 1 | 2)
            || store
                .accounts
                .iter()
                .any(|(key, account)| key.trim().is_empty() || key != &account.account_id)
        {
            return Err(CodexOAuthError::ParseError(
                "Codex 账号存储版本或账号索引无效".to_string(),
            ));
        }
        if store.accounts.contains_key(account_id) {
            return Err(CodexOAuthError::TokenFetchFailed(
                "Codex 账号仍在磁盘存储中，请重启应用后重试".to_string(),
            ));
        }
        Ok(())
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
#[allow(dead_code)]
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

fn extract_refresh_error_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| match error {
            serde_json::Value::Object(object) => object.get("code").and_then(|code| code.as_str()),
            serde_json::Value::String(code) => Some(code.as_str()),
            _ => None,
        })
        .or_else(|| value.get("code").and_then(|code| code.as_str()))
        .map(|code| code.to_ascii_lowercase())
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

/// 从 token 响应中提取 (chatgpt_account_id, email)
fn extract_account_metadata_from_tokens(
    tokens: &OAuthTokenResponse,
) -> (Option<String>, Option<String>) {
    let mut account_id: Option<String> = None;
    let mut email: Option<String> = None;

    if let Some(id_token) = tokens.id_token.as_deref() {
        if let Some(claims) = parse_jwt_claims(id_token) {
            account_id = claims.chatgpt_account_id.clone().or_else(|| {
                claims
                    .openai_auth
                    .as_ref()
                    .and_then(|a| a.chatgpt_account_id.clone())
            });
            email = claims.email.clone();
        }
    }

    if account_id.is_none() {
        if let Some(claims) = parse_jwt_claims(&tokens.access_token) {
            account_id = claims.chatgpt_account_id.clone().or_else(|| {
                claims
                    .openai_auth
                    .as_ref()
                    .and_then(|a| a.chatgpt_account_id.clone())
            });
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

    #[tokio::test]
    async fn missing_account_recovery_requires_valid_persisted_state() {
        let temp = tempfile::tempdir().unwrap();
        let manager = CodexOAuthManager::new(temp.path().to_path_buf());
        for content in [
            "{broken",
            "{}",
            r#"{"version":2}"#,
            r#"{"version":3,"accounts":{}}"#,
        ] {
            fs::write(&manager.storage_path, content).unwrap();
            assert!(manager
                .prepare_live_auth_for_account_switch_away("missing")
                .await
                .is_err());
            assert!(!matches!(
                manager.ensure_account_exists("missing").await,
                Err(CodexOAuthError::AccountUnavailable(_))
            ));
        }
        fs::write(&manager.storage_path, r#"{"version":2,"accounts":{}}"#).unwrap();
        assert!(matches!(
            manager
                .prepare_live_auth_for_account_switch_away("missing")
                .await,
            Ok(CodexLiveAuthSwitchGuard::MissingAccount)
        ));
        assert!(matches!(
            manager.ensure_account_exists("missing").await,
            Err(CodexOAuthError::AccountUnavailable(_))
        ));

        manager
            .add_test_account_with_access_token("present", "access", None)
            .await
            .unwrap();
        assert!(
            manager.ensure_account_exists("present").await.is_ok(),
            "reauth-required is not removed"
        );
        manager.accounts.write().await.clear();
        assert!(
            manager
                .prepare_live_auth_for_account_switch_away("present")
                .await
                .is_err(),
            "an account still on disk cannot be treated as deleted"
        );
    }

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
            obtained_at_ms: now,
        };
        assert!(expiring.is_expiring_soon());

        // 1 小时后过期 - 不在缓冲期内
        let valid = CachedAccessToken {
            token: "t".to_string(),
            expires_at_ms: now + 3_600_000,
            obtained_at_ms: now,
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
    fn test_extract_account_metadata_does_not_use_organization_id() {
        let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"none\"}");
        let payload = URL_SAFE_NO_PAD.encode(b"{\"organizations\":[{\"id\":\"org-456\"}]}");
        let jwt = format!("{header}.{payload}.");
        let tokens = OAuthTokenResponse {
            access_token: jwt,
            refresh_token: None,
            id_token: None,
            expires_in: None,
        };

        assert_eq!(extract_account_metadata_from_tokens(&tokens), (None, None));
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
                    "workspace-123".to_string(),
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
