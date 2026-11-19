//! User-defined custom gateway / relay templates (`我的模板`).
//!
//! Stores strictly non-secret gateway metadata (name, OpenAI-compatible base URL,
//! optional Anthropic base URL, default model ID, Codex wire protocol) inside
//! SQLite `settings` so users can reuse private relays, internal gateways, or
//! local OpenAI-compatible servers in one-click multi-client batch setup.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{database::Database, services::provider_api::ApiProtocol};

const CUSTOM_GATEWAY_TEMPLATES_KEY: &str = "custom_gateway_templates_v1";
const MAX_CUSTOM_TEMPLATES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomGatewayTemplate {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic_base_url: Option<String>,
    pub default_model_id: String,
    pub codex_protocol: ApiProtocol,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_url: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveCustomGatewayTemplateRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub base_url: String,
    #[serde(default)]
    pub anthropic_base_url: Option<String>,
    pub default_model_id: String,
    #[serde(default)]
    pub codex_protocol: Option<ApiProtocol>,
    #[serde(default)]
    pub api_key_url: Option<String>,
}

fn contains_secret_pattern(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("sk-")
        || lower.contains("sk_")
        || lower.contains("bearer ")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("-----begin")
        || lower.contains("eyj")
}

fn validate_http_url(raw: &str, label: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 512 {
        return Err(format!("{label}不能为空且长度不能超过 512 字符"));
    }
    if contains_secret_pattern(trimmed) || trimmed.chars().any(char::is_control) {
        return Err(format!("{label}不能包含密钥或控制字符"));
    }
    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| format!("{label}不是有效的 HTTP(S) 地址"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(format!("{label}必须为 http 或 https 地址"));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(format!("{label}不能包含账号密码、查询参数或锚点"));
    }
    Ok(trimmed.trim_end_matches('/').to_string())
}

fn validate_model_id(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err("默认模型 ID 不能为空且不能超过 128 字符".to_string());
    }
    if contains_secret_pattern(trimmed)
        || trimmed.contains("..")
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '-'))
    {
        return Err("默认模型 ID 包含不支持的字符".to_string());
    }
    Ok(trimmed.to_string())
}

fn validate_template_id(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with("custom-")
        || trimmed.len() <= "custom-".len()
        || trimmed.len() > 64
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("模板 ID 格式无效".to_string());
    }
    Ok(trimmed.to_string())
}

pub fn list_custom_gateway_templates(db: &Database) -> Result<Vec<CustomGatewayTemplate>, String> {
    let Some(raw) = db
        .get_setting(CUSTOM_GATEWAY_TEMPLATES_KEY)
        .map_err(|e| format!("读取自定义模板失败: {e}"))?
    else {
        return Ok(Vec::new());
    };
    let mut items: Vec<CustomGatewayTemplate> =
        serde_json::from_str(&raw).map_err(|e| format!("解析自定义模板失败: {e}"))?;
    items.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(items)
}

fn persist_templates(db: &Database, items: &[CustomGatewayTemplate]) -> Result<(), String> {
    let json = serde_json::to_string(items).map_err(|e| format!("序列化自定义模板失败: {e}"))?;
    db.set_setting(CUSTOM_GATEWAY_TEMPLATES_KEY, &json)
        .map_err(|e| format!("保存自定义模板失败: {e}"))
}

pub fn save_custom_gateway_template(
    db: &Database,
    request: SaveCustomGatewayTemplateRequest,
) -> Result<CustomGatewayTemplate, String> {
    let name = request.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 60 {
        return Err("模板名称不能为空且不能超过 60 字".to_string());
    }
    if contains_secret_pattern(&name) || name.chars().any(char::is_control) {
        return Err("模板名称不能包含密钥或控制字符".to_string());
    }

    let summary = request.summary.trim().to_string();
    if summary.chars().count() > 160 || contains_secret_pattern(&summary) {
        return Err("模板说明过长或包含疑似密钥文本".to_string());
    }

    let base_url = validate_http_url(&request.base_url, "服务地址")?;
    let anthropic_base_url = match request
        .anthropic_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(url) => Some(validate_http_url(url, "Claude 专用服务地址")?),
        None => None,
    };
    let api_key_url = match request
        .api_key_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(url) => Some(validate_http_url(url, "控制台取 Key 地址")?),
        None => None,
    };
    let default_model_id = validate_model_id(&request.default_model_id)?;

    let codex_protocol = match request.codex_protocol {
        Some(ApiProtocol::Chat) => ApiProtocol::Chat,
        _ => ApiProtocol::Responses,
    };

    let mut existing = list_custom_gateway_templates(db)?;
    let id = match request
        .id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw_id) => validate_template_id(raw_id)?,
        None => format!("custom-{}", Uuid::new_v4().simple()),
    };

    let template = CustomGatewayTemplate {
        id: id.clone(),
        name,
        summary,
        base_url,
        anthropic_base_url,
        default_model_id,
        codex_protocol,
        api_key_url,
        updated_at: chrono::Utc::now().timestamp(),
    };

    if let Some(pos) = existing.iter().position(|item| item.id == id) {
        existing[pos] = template.clone();
    } else {
        if existing.len() >= MAX_CUSTOM_TEMPLATES {
            return Err(format!("自定义模板最多保存 {MAX_CUSTOM_TEMPLATES} 个"));
        }
        existing.push(template.clone());
    }

    persist_templates(db, &existing)?;
    Ok(template)
}

pub fn delete_custom_gateway_template(db: &Database, id: &str) -> Result<bool, String> {
    let valid_id = validate_template_id(id)?;
    let mut existing = list_custom_gateway_templates(db)?;
    let before = existing.len();
    existing.retain(|item| item.id != valid_id);
    if existing.len() == before {
        return Ok(false);
    }
    persist_templates(db, &existing)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crud_custom_gateway_templates_roundtrips_cleanly() {
        let db = Database::memory().expect("in-memory db");
        assert!(list_custom_gateway_templates(&db).unwrap().is_empty());

        let created = save_custom_gateway_template(
            &db,
            SaveCustomGatewayTemplateRequest {
                id: None,
                name: "公司内部网关".to_string(),
                summary: "团队统一转发".to_string(),
                base_url: "https://llm.corp.example/v1/".to_string(),
                anthropic_base_url: Some("https://llm.corp.example/anthropic".to_string()),
                default_model_id: "gpt-5.4".to_string(),
                codex_protocol: Some(ApiProtocol::Responses),
                api_key_url: Some("https://llm.corp.example/keys".to_string()),
            },
        )
        .unwrap();

        assert!(created.id.starts_with("custom-"));
        assert_eq!(created.base_url, "https://llm.corp.example/v1");
        assert_eq!(
            created.anthropic_base_url.as_deref(),
            Some("https://llm.corp.example/anthropic")
        );

        let updated = save_custom_gateway_template(
            &db,
            SaveCustomGatewayTemplateRequest {
                id: Some(created.id.clone()),
                name: "公司网关 V2".to_string(),
                summary: String::new(),
                base_url: "https://llm2.corp.example/v1".to_string(),
                anthropic_base_url: None,
                default_model_id: "claude-sonnet-4-6".to_string(),
                codex_protocol: Some(ApiProtocol::Chat),
                api_key_url: None,
            },
        )
        .unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.codex_protocol, ApiProtocol::Chat);

        let list = list_custom_gateway_templates(&db).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "公司网关 V2");

        assert!(delete_custom_gateway_template(&db, &created.id).unwrap());
        assert!(!delete_custom_gateway_template(&db, &created.id).unwrap());
        assert!(list_custom_gateway_templates(&db).unwrap().is_empty());
    }

    #[test]
    fn rejects_embedded_secrets_and_invalid_urls() {
        let db = Database::memory().expect("in-memory db");
        let err = save_custom_gateway_template(
            &db,
            SaveCustomGatewayTemplateRequest {
                id: None,
                name: "sk-live-secret-name".to_string(),
                summary: String::new(),
                base_url: "https://relay.example/v1".to_string(),
                anthropic_base_url: None,
                default_model_id: "gpt-5.4".to_string(),
                codex_protocol: None,
                api_key_url: None,
            },
        )
        .unwrap_err();
        assert!(err.contains("密钥"));

        let err2 = save_custom_gateway_template(
            &db,
            SaveCustomGatewayTemplateRequest {
                id: None,
                name: "坏地址".to_string(),
                summary: String::new(),
                base_url: "https://user:pass@relay.example/v1".to_string(),
                anthropic_base_url: None,
                default_model_id: "gpt-5.4".to_string(),
                codex_protocol: None,
                api_key_url: None,
            },
        )
        .unwrap_err();
        assert!(err2.contains("账号密码"));

        let err3 = save_custom_gateway_template(
            &db,
            SaveCustomGatewayTemplateRequest {
                id: Some("deepseek".to_string()),
                name: "企图覆盖内置模板".to_string(),
                summary: String::new(),
                base_url: "https://relay.example/v1".to_string(),
                anthropic_base_url: None,
                default_model_id: "gpt-5.4".to_string(),
                codex_protocol: None,
                api_key_url: None,
            },
        )
        .unwrap_err();
        assert!(err3.contains("模板 ID"));
    }
}
