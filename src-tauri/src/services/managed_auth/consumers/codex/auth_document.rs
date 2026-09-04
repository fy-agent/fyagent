//! First-party ChatGPT `auth.json` adapter for Codex file-store projection.
//!
//! Schema follows OpenAI Codex pin
//! `8e6a44b428e31f91b21edc97904fcdf4f0931ade` (`AuthDotJson` / `TokenData`).
//! Tokens never appear in Debug/Display output.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::services::managed_auth::providers::openai::{self, OpenAiTokenGrant};

pub(crate) const MAX_AUTH_JSON_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CodexNativeAuthState {
    Missing,
    ChatGptKnown {
        account_id: String,
        revision: String,
    },
    ChatGptUnmanaged {
        identity_fingerprint: String,
        revision: String,
    },
    ThirdPartyApiKeyOnly {
        revision: String,
    },
    PersonalAccessToken {
        revision: String,
    },
    AgentIdentityOnly {
        revision: String,
    },
    Bedrock {
        revision: String,
    },
    Unsupported {
        revision: String,
    },
    Invalid {
        revision: String,
    },
    Unreadable,
    Oversized,
}

/// Minimal ChatGPT auth document. Secret fields are zeroized on drop.
pub(crate) struct CodexChatGptAuthDocument {
    id_token: Zeroizing<String>,
    access_token: Zeroizing<String>,
    refresh_token: Zeroizing<String>,
    account_id: Option<String>,
    last_refresh: String,
}

impl std::fmt::Debug for CodexChatGptAuthDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CodexChatGptAuthDocument")
            .field("account_id", &self.account_id)
            .field("last_refresh", &self.last_refresh)
            .field("id_token", &"<redacted>")
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize)]
struct AuthDotJsonWire<'a> {
    auth_mode: &'static str,
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<&'a str>,
    tokens: TokenDataWire<'a>,
    last_refresh: &'a str,
}

#[derive(Serialize)]
struct TokenDataWire<'a> {
    id_token: &'a str,
    access_token: &'a str,
    refresh_token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<&'a str>,
}

#[derive(Deserialize)]
struct AuthDotJsonRead {
    #[serde(default)]
    auth_mode: Option<String>,
    #[serde(default, rename = "OPENAI_API_KEY")]
    openai_api_key: Option<Value>,
    #[serde(default)]
    tokens: Option<TokenDataRead>,
    #[serde(default)]
    personal_access_token: Option<Value>,
    #[serde(default)]
    agent_identity: Option<Value>,
    #[serde(default)]
    bedrock_api_key: Option<Value>,
    #[serde(default)]
    last_refresh: Option<Value>,
}

#[derive(Deserialize)]
struct TokenDataRead {
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    account_id: Option<String>,
}

impl CodexChatGptAuthDocument {
    pub(crate) fn from_tokens(
        id_token: &str,
        access_token: &str,
        refresh_token: &str,
        account_id: Option<&str>,
        last_refresh_unix: Option<i64>,
    ) -> Option<Self> {
        if id_token.trim().is_empty()
            || access_token.trim().is_empty()
            || refresh_token.trim().is_empty()
        {
            return None;
        }
        let grant = OpenAiTokenGrant {
            access_token: access_token.to_string(),
            refresh_token: Some(refresh_token.to_string()),
            id_token: Some(id_token.to_string()),
            expires_in: None,
        };
        let identity = openai::extract_identity(&grant).ok()?;
        let account = account_id
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .or(Some(identity.subject.clone()));
        if account.as_deref() != Some(identity.subject.as_str()) {
            return None;
        }
        let last_refresh = match last_refresh_unix {
            Some(ts) => chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
                .unwrap_or_else(|| {
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                }),
            None => chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        };
        Some(Self {
            id_token: Zeroizing::new(id_token.to_string()),
            access_token: Zeroizing::new(access_token.to_string()),
            refresh_token: Zeroizing::new(refresh_token.to_string()),
            account_id: account,
            last_refresh,
        })
    }

    pub(crate) fn from_grant(grant: &OpenAiTokenGrant) -> Option<Self> {
        let id_token = grant.id_token.as_deref()?;
        let refresh = grant.refresh_token.as_deref()?;
        Self::from_tokens(id_token, &grant.access_token, refresh, None, None)
    }

    pub(crate) fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    pub(crate) fn serialize_bytes(&self) -> Result<Vec<u8>, ()> {
        let wire = AuthDotJsonWire {
            auth_mode: "chatgpt",
            openai_api_key: None,
            tokens: TokenDataWire {
                id_token: self.id_token.as_str(),
                access_token: self.access_token.as_str(),
                refresh_token: self.refresh_token.as_str(),
                account_id: self.account_id.as_deref(),
            },
            last_refresh: &self.last_refresh,
        };
        serde_json::to_vec_pretty(&wire).map_err(|_| ())
    }

    pub(crate) fn identity_matches(&self, provider_subject: &str) -> bool {
        self.account_id.as_deref() == Some(provider_subject)
    }

    pub(crate) fn id_token(&self) -> &str {
        self.id_token.as_str()
    }

    pub(crate) fn access_token(&self) -> &str {
        self.access_token.as_str()
    }

    pub(crate) fn refresh_token(&self) -> &str {
        self.refresh_token.as_str()
    }

    /// Parse a complete ChatGPT auth document from live `auth.json` bytes.
    /// Unknown fields are ignored; incomplete or mismatched material returns None.
    pub(crate) fn try_from_live_bytes(bytes: &[u8]) -> Option<Self> {
        let parsed: AuthDotJsonRead = serde_json::from_slice(bytes).ok()?;
        if parsed.auth_mode.as_deref() != Some("chatgpt") {
            return None;
        }
        if value_present(&parsed.openai_api_key)
            || value_present(&parsed.personal_access_token)
            || value_present(&parsed.bedrock_api_key)
        {
            return None;
        }
        let tokens = parsed.tokens?;
        let id_token = tokens.id_token.filter(|value| !value.trim().is_empty())?;
        let access_token = tokens
            .access_token
            .filter(|value| !value.trim().is_empty())?;
        let refresh_token = tokens
            .refresh_token
            .filter(|value| !value.trim().is_empty())?;
        let last_refresh_unix = parsed.last_refresh.as_ref().and_then(|value| match value {
            Value::String(text) => chrono::DateTime::parse_from_rfc3339(text)
                .ok()
                .map(|dt| dt.timestamp()),
            Value::Number(number) => number.as_i64(),
            _ => None,
        });
        Self::from_tokens(
            &id_token,
            &access_token,
            &refresh_token,
            tokens.account_id.as_deref(),
            last_refresh_unix,
        )
    }
}

pub(crate) fn classify_auth_bytes(bytes: &[u8], revision: String) -> CodexNativeAuthState {
    let Ok(parsed) = serde_json::from_slice::<AuthDotJsonRead>(bytes) else {
        return CodexNativeAuthState::Invalid { revision };
    };
    if value_present(&parsed.bedrock_api_key) {
        return CodexNativeAuthState::Bedrock { revision };
    }
    if value_present(&parsed.personal_access_token) {
        return CodexNativeAuthState::PersonalAccessToken { revision };
    }

    let tokens = parsed.tokens.as_ref();
    let has_oauth = tokens.is_some_and(|tokens| {
        non_empty(&tokens.id_token)
            || non_empty(&tokens.access_token)
            || non_empty(&tokens.refresh_token)
    });
    let has_api_key = match &parsed.openai_api_key {
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Null) | None => false,
        Some(_) => true,
    };

    if !has_oauth {
        if value_present(&parsed.agent_identity) {
            return CodexNativeAuthState::AgentIdentityOnly { revision };
        }
        if has_api_key {
            return CodexNativeAuthState::ThirdPartyApiKeyOnly { revision };
        }
        if parsed.auth_mode.as_deref() == Some("chatgpt") {
            return CodexNativeAuthState::Invalid { revision };
        }
        return CodexNativeAuthState::Unsupported { revision };
    }

    let Some(tokens) = tokens else {
        return CodexNativeAuthState::Invalid { revision };
    };
    if !(non_empty(&tokens.id_token)
        && non_empty(&tokens.access_token)
        && non_empty(&tokens.refresh_token))
    {
        return CodexNativeAuthState::Invalid { revision };
    }

    let grant = OpenAiTokenGrant {
        access_token: tokens.access_token.clone().unwrap_or_default(),
        refresh_token: tokens.refresh_token.clone(),
        id_token: tokens.id_token.clone(),
        expires_in: None,
    };
    match openai::extract_identity(&grant) {
        Ok(identity) => {
            let account_id = tokens
                .account_id
                .clone()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| identity.subject.clone());
            if account_id != identity.subject {
                return CodexNativeAuthState::Invalid { revision };
            }
            CodexNativeAuthState::ChatGptKnown {
                account_id,
                revision,
            }
        }
        Err(_) => CodexNativeAuthState::ChatGptUnmanaged {
            identity_fingerprint: fingerprint_unmanaged(&tokens.id_token, &tokens.access_token),
            revision,
        },
    }
}

fn non_empty(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|text| !text.trim().is_empty())
}

fn value_present(value: &Option<Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::String(text)) => !text.trim().is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

fn fingerprint_unmanaged(id_token: &Option<String>, access_token: &Option<String>) -> String {
    let mut digest = Sha256::new();
    digest.update(id_token.as_deref().unwrap_or("").as_bytes());
    digest.update(b"|");
    digest.update(access_token.as_deref().unwrap_or("").as_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_json::json;

    fn jwt_with_account(account: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(
            json!({
                "chatgpt_account_id": account,
                "email": "user@example.com",
                "organizations": [{"id": "org-1"}]
            })
            .to_string()
            .as_bytes(),
        );
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn complete_chatgpt_document_round_trips_identity() {
        let id = jwt_with_account("acct-1");
        let access = jwt_with_account("acct-1");
        let doc = CodexChatGptAuthDocument::from_tokens(
            &id,
            &access,
            "refresh-token-value",
            Some("acct-1"),
            Some(1_700_000_000),
        )
        .unwrap();
        let bytes = doc.serialize_bytes().unwrap();
        let state = classify_auth_bytes(&bytes, "mr1:test".into());
        assert_eq!(
            state,
            CodexNativeAuthState::ChatGptKnown {
                account_id: "acct-1".into(),
                revision: "mr1:test".into(),
            }
        );
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"auth_mode\": \"chatgpt\""));
        assert!(!text.contains("OPENAI_API_KEY\": \""));
    }

    #[test]
    fn api_key_only_is_classified() {
        let bytes = serde_json::to_vec(&json!({
            "OPENAI_API_KEY": "sk-test",
            "tokens": null
        }))
        .unwrap();
        assert!(matches!(
            classify_auth_bytes(&bytes, "mr1:x".into()),
            CodexNativeAuthState::ThirdPartyApiKeyOnly { .. }
        ));
    }

    #[test]
    fn incomplete_tokens_are_invalid() {
        let bytes = serde_json::to_vec(&json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "refresh_token": "only-refresh"
            }
        }))
        .unwrap();
        assert!(matches!(
            classify_auth_bytes(&bytes, "mr1:x".into()),
            CodexNativeAuthState::Invalid { .. }
        ));
    }
}
