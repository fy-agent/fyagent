//! 错误类型到 HTTP 状态码的映射
//!
//! 将 ProxyError 映射到合适的 HTTP 状态码，用于日志记录和手动构建错误响应

use super::ProxyError;

/// 将 ProxyError 映射到响应与日志共用的 HTTP 状态码。
pub fn map_proxy_error_to_status(error: &ProxyError) -> u16 {
    error.status_code().as_u16()
}

/// 持久化和诊断日志不信任上游正文；只记录状态和固定分类。
/// 客户端响应仍由 get_error_message / IntoResponse 保留协议正文。
pub(super) fn get_log_error_message(error: &ProxyError) -> String {
    match error {
        ProxyError::UpstreamError { status, .. } => {
            let kind = match *status {
                401 => "unauthorized",
                403 => "forbidden",
                429 => "rate limit exceeded",
                500..=599 => "upstream server error",
                _ => "upstream error",
            };
            format!("上游 HTTP {status}: {kind}")
        }
        // HTTP 2xx 的错误信封也会携带不可信正文（包括错误类型字段）。
        // 保留错误对象供客户端/重试使用，日志和数据库只接收固定分类。
        ProxyError::TransformError(_) => "请求/响应转换错误: transformation failed".to_string(),
        // 认证交换、CONNECT 状态行和其他自由文本也可能携带上游正文或凭据。
        ProxyError::AuthError(_) => "认证失败: authentication failed".to_string(),
        ProxyError::ForwardFailed(message) if message.contains("connection reset by peer") => {
            "请求转发失败: connection reset by peer".to_string()
        }
        ProxyError::ForwardFailed(_) => "请求转发失败: forwarding failed".to_string(),
        ProxyError::Timeout(_) => "请求超时: request timed out".to_string(),
        ProxyError::ConfigError(_) => "配置错误: configuration failed".to_string(),
        ProxyError::InvalidRequest(_) => "无效的请求: invalid request".to_string(),
        ProxyError::Internal(_) => "内部错误: internal error".to_string(),
        ProxyError::ProviderUnhealthy(_) => "Provider 不健康: provider unhealthy".to_string(),
        ProxyError::DatabaseError(_) => "数据库错误: database error".to_string(),
        ProxyError::BindFailed(_) => "地址绑定失败: bind failed".to_string(),
        ProxyError::StopFailed(_) => "停止失败: stop failed".to_string(),
        ProxyError::RequestBodyTooLarge
        | ProxyError::ResponseBodyTooLarge(_)
        | ProxyError::AlreadyRunning
        | ProxyError::NotRunning
        | ProxyError::StopTimeout
        | ProxyError::NoAvailableProvider
        | ProxyError::AllProvidersCircuitOpen
        | ProxyError::NoProvidersConfigured
        | ProxyError::MaxRetriesExceeded
        | ProxyError::StreamIdleTimeout(_) => get_error_message(error),
    }
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
    fn free_text_diagnostics_use_fixed_categories_without_changing_details() {
        let detail = "500 Internal Server Error: Authorization: Bearer fake-copilot-key/+=private";
        for (error, expected) in [
            (
                ProxyError::AuthError(detail.into()),
                "认证失败: authentication failed",
            ),
            (
                ProxyError::ForwardFailed(detail.into()),
                "请求转发失败: forwarding failed",
            ),
            (
                ProxyError::Timeout(detail.into()),
                "请求超时: request timed out",
            ),
            (
                ProxyError::ConfigError(detail.into()),
                "配置错误: configuration failed",
            ),
            (
                ProxyError::InvalidRequest(detail.into()),
                "无效的请求: invalid request",
            ),
            (
                ProxyError::Internal(detail.into()),
                "内部错误: internal error",
            ),
            (
                ProxyError::ProviderUnhealthy(detail.into()),
                "Provider 不健康: provider unhealthy",
            ),
            (
                ProxyError::DatabaseError(detail.into()),
                "数据库错误: database error",
            ),
            (
                ProxyError::BindFailed(detail.into()),
                "地址绑定失败: bind failed",
            ),
            (
                ProxyError::StopFailed(detail.into()),
                "停止失败: stop failed",
            ),
            (
                ProxyError::TransformError(detail.into()),
                "请求/响应转换错误: transformation failed",
            ),
            (
                ProxyError::ForwardFailed(format!("connection reset by peer: {detail}")),
                "请求转发失败: connection reset by peer",
            ),
        ] {
            let summary = get_log_error_message(&error);
            assert_eq!(summary, expected);
            assert!(!summary.contains("fake-copilot-key"));
            assert!(!summary.contains("Authorization"));
            assert!(get_error_message(&error).contains(detail));
        }
    }

    #[tokio::test]
    async fn request_body_overflow_maps_to_413_with_readable_json() {
        let error = ProxyError::RequestBodyTooLarge;
        assert_eq!(map_proxy_error_to_status(&error), 413);
        let response = error.into_response();
        assert_eq!(response.status().as_u16(), 413);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["message"], "请求体超过大小上限");
    }

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
