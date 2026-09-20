//! 错误类型到 HTTP 状态码的映射
//!
//! 将 ProxyError 映射到合适的 HTTP 状态码，用于日志记录和手动构建错误响应

use super::ProxyError;

/// 将 ProxyError 映射到响应与日志共用的 HTTP 状态码。
pub fn map_proxy_error_to_status(error: &ProxyError) -> u16 {
    error.status_code().as_u16()
}

/// 将 ProxyError 转换为用户友好的错误消息
pub fn get_error_message(error: &ProxyError) -> String {
    match error {
        ProxyError::UpstreamError { status, body } => {
            if let Some(body) = body {
                format!("上游错误 ({status}): {body}")
            } else {
                format!("上游错误 ({status})")
            }
        }
        ProxyError::Timeout(msg) => format!("请求超时: {msg}"),
        ProxyError::ForwardFailed(msg) => format!("转发失败: {msg}"),
        ProxyError::NoAvailableProvider => "无可用 Provider".to_string(),
        ProxyError::AllProvidersCircuitOpen => "所有供应商已熔断，无可用渠道".to_string(),
        ProxyError::NoProvidersConfigured => "未配置供应商".to_string(),
        ProxyError::MaxRetriesExceeded => "所有 Provider 都失败，重试耗尽".to_string(),
        ProxyError::ProviderUnhealthy(msg) => format!("Provider 不健康: {msg}"),
        ProxyError::DatabaseError(msg) => format!("数据库错误: {msg}"),
        ProxyError::TransformError(msg) => format!("请求/响应转换错误: {msg}"),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn test_map_upstream_error() {
        let error = ProxyError::UpstreamError {
            status: 401,
            body: Some("Unauthorized".to_string()),
        };
        assert_eq!(map_proxy_error_to_status(&error), 401);
    }

    #[test]
    fn test_map_timeout_error() {
        let error = ProxyError::Timeout("Request timeout".to_string());
        assert_eq!(map_proxy_error_to_status(&error), 504);
    }

    #[test]
    fn test_map_connection_error() {
        let error = ProxyError::ForwardFailed("Connection refused".to_string());
        assert_eq!(map_proxy_error_to_status(&error), 502);
    }

    #[test]
    fn test_map_no_provider_error() {
        let error = ProxyError::NoAvailableProvider;
        assert_eq!(map_proxy_error_to_status(&error), 503);
    }

    #[test]
    fn test_map_status_matches_proxy_error_response_semantics() {
        for (error, expected_status) in [
            (ProxyError::AlreadyRunning, 409),
            (ProxyError::NotRunning, 503),
            (ProxyError::BindFailed("bind".into()), 500),
            (ProxyError::StopTimeout, 500),
            (ProxyError::StopFailed("stop".into()), 500),
            (ProxyError::ForwardFailed("forward".into()), 502),
            (ProxyError::NoAvailableProvider, 503),
            (ProxyError::AllProvidersCircuitOpen, 503),
            (ProxyError::NoProvidersConfigured, 503),
            (ProxyError::ProviderUnhealthy("provider".into()), 503),
            (ProxyError::MaxRetriesExceeded, 503),
            (ProxyError::DatabaseError("database".into()), 500),
            (ProxyError::ConfigError("config".into()), 400),
            (ProxyError::TransformError("transform".into()), 422),
            (ProxyError::InvalidRequest("request".into()), 400),
            (ProxyError::Timeout("timeout".into()), 504),
            (ProxyError::StreamIdleTimeout(30), 504),
            (ProxyError::AuthError("auth".into()), 401),
            (ProxyError::Internal("internal".into()), 500),
            (ProxyError::ResponseBodyTooLarge(1024), 502),
            (
                ProxyError::UpstreamError {
                    status: 429,
                    body: None,
                },
                429,
            ),
            (
                ProxyError::UpstreamError {
                    status: 0,
                    body: None,
                },
                502,
            ),
        ] {
            assert_eq!(map_proxy_error_to_status(&error), expected_status);
            assert_eq!(error.into_response().status().as_u16(), expected_status);
        }
    }

    #[tokio::test]
    async fn error_responses_preserve_json_text_and_generated_messages() {
        let local = ProxyError::ResponseBodyTooLarge(1024);
        let expected_local = serde_json::json!({
            "error": {"message": local.to_string(), "type": "proxy_error"}
        });
        for (error, expected_body) in [
            (
                ProxyError::UpstreamError {
                    status: 429,
                    body: Some(r#"{"error":{"message":"rate limited","code":"limit"}}"#.into()),
                },
                serde_json::json!({"error": {"message": "rate limited", "code": "limit"}}),
            ),
            (
                ProxyError::UpstreamError {
                    status: 503,
                    body: Some("temporarily unavailable".into()),
                },
                serde_json::json!({
                    "error": {"message": "temporarily unavailable", "type": "upstream_error"}
                }),
            ),
            (
                ProxyError::UpstreamError {
                    status: 502,
                    body: None,
                },
                serde_json::json!({
                    "error": {"message": "Upstream error (status 502)", "type": "upstream_error"}
                }),
            ),
            (local, expected_local),
        ] {
            let response = error.into_response();
            assert_eq!(response.headers()["content-type"], "application/json");
            let body = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .expect("read response body");
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).expect("JSON response"),
                expected_body
            );
        }
    }

    #[test]
    fn test_get_error_message() {
        let error = ProxyError::UpstreamError {
            status: 500,
            body: Some("Internal Server Error".to_string()),
        };
        let msg = get_error_message(&error);
        assert!(msg.contains("上游错误"));
        assert!(msg.contains("500"));
        assert!(msg.contains("Internal Server Error"));
    }
}
