//! Closed protocol and product-scope policy shared by quick setup and probes.
use crate::error::AppError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiProtocol {
    Anthropic,
    Responses,
    Chat,
}

impl ApiProtocol {
    pub fn for_target(target: &str, requested: Option<Self>) -> Result<Self, AppError> {
        let default = match target {
            "claude" => Self::Anthropic,
            "codex" | "grokbuild" => Self::Responses,
            "workbuddy" | "opencode" => Self::Chat,
            _ => return Err(AppError::Message("不支持此配置目标".into())),
        };
        let protocol = requested.unwrap_or(default);
        if protocol != default && !(target == "codex" && protocol == Self::Chat) {
            return Err(AppError::Message("目标工具不支持所选协议".into()));
        }
        Ok(protocol)
    }

    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Responses => "responses",
            Self::Chat => "chat",
        }
    }
}

fn is_aliyun_coding(host: &str) -> bool {
    matches!(
        host,
        "coding.dashscope.aliyuncs.com" | "coding-intl.dashscope.aliyuncs.com"
    )
}

pub fn validate_credential_scope(
    base_url: &str,
    api_key: &str,
    protocol: ApiProtocol,
) -> Result<(), AppError> {
    let url = url::Url::parse(base_url).map_err(|_| AppError::Message("服务地址无效".into()))?;
    let host = url.host_str().unwrap_or_default();
    if is_aliyun_coding(host) {
        if !api_key.trim().starts_with("sk-sp-") {
            return Err(AppError::Message("请使用 Coding Plan 专属 API Key".into()));
        }
        if protocol == ApiProtocol::Responses {
            return Err(AppError::Message(
                "此 Coding Plan 地址不支持 Responses 协议".into(),
            ));
        }
    } else if matches!(
        host,
        "dashscope.aliyuncs.com"
            | "dashscope-intl.aliyuncs.com"
            | "dashscope-us.aliyuncs.com"
            | "cn-hongkong.dashscope.aliyuncs.com"
    ) && api_key.trim().starts_with("sk-sp-")
    {
        return Err(AppError::Message(
            "Coding Plan API Key 不能用于按量地址".into(),
        ));
    }
    Ok(())
}

/// This app's generic discovery/probe is not an interactive coding-tool call.
/// Product keys with opaque formats cannot be inferred; the preset explains
/// their scope. Known restricted hosts/paths and dedicated keys fail closed.
pub fn ensure_generic_api_allowed(base_url: &str, api_key: &str) -> Result<(), AppError> {
    let url = url::Url::parse(base_url).map_err(|_| AppError::Message("服务地址无效".into()))?;
    let host = url.host_str().unwrap_or_default();
    let path = url.path();
    if api_key.trim().starts_with("sk-sp-")
        || is_aliyun_coding(host)
        || host == "token-plan.cn-beijing.maas.aliyuncs.com"
        || (host == "ark.cn-beijing.volces.com"
            && (path == "/api/coding" || path.starts_with("/api/coding/")))
    {
        return Err(AppError::Message(
            "此套餐请在其允许的目标编程工具中验证，不支持通用模型拉取或测试".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_compatible_and_protocols_are_closed() {
        assert_eq!(
            ApiProtocol::for_target("codex", None).unwrap(),
            ApiProtocol::Responses
        );
        assert_eq!(
            ApiProtocol::for_target("codex", Some(ApiProtocol::Chat)).unwrap(),
            ApiProtocol::Chat
        );
        assert!(ApiProtocol::for_target("claude", Some(ApiProtocol::Chat)).is_err());
        assert!(ApiProtocol::for_target("grokbuild", Some(ApiProtocol::Anthropic)).is_err());
        assert!(serde_json::from_str::<ApiProtocol>("\"arbitrary\"").is_err());
    }

    #[test]
    fn known_plan_credentials_and_endpoints_cannot_be_mixed() {
        let coding = "https://coding.dashscope.aliyuncs.com/v1";
        assert!(validate_credential_scope(coding, "sk-sp-fixture", ApiProtocol::Chat).is_ok());
        assert!(validate_credential_scope(coding, "ordinary-fixture", ApiProtocol::Chat).is_err());
        assert!(
            validate_credential_scope(coding, "sk-sp-fixture", ApiProtocol::Responses).is_err()
        );
        assert!(validate_credential_scope(
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "sk-sp-fixture",
            ApiProtocol::Chat
        )
        .is_err());
    }

    #[test]
    fn generic_checks_reject_restricted_plans_before_network() {
        for endpoint in [
            "https://coding.dashscope.aliyuncs.com/v1",
            "https://ark.cn-beijing.volces.com/api/coding/v3",
            "https://ark.cn-beijing.volces.com/api/coding",
            "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
        ] {
            assert!(ensure_generic_api_allowed(endpoint, "fixture").is_err());
        }
        assert!(ensure_generic_api_allowed("https://example.com/v1", "sk-sp-fixture").is_err());
        assert!(
            ensure_generic_api_allowed("https://ark.cn-beijing.volces.com/api/v3", "fixture")
                .is_ok()
        );
        assert!(ensure_generic_api_allowed(
            "https://ark.cn-beijing.volces.com.example.com/api/coding",
            "fixture"
        )
        .is_ok());
    }
}
