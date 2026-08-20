//! MCP 服务器配置验证模块

use serde_json::Value;

use crate::error::AppError;

/// 基础校验：允许 stdio/http/sse；或省略 type（视为 stdio）。对应必填字段存在
pub fn validate_server_spec(spec: &Value) -> Result<(), AppError> {
    if !spec.is_object() {
        return Err(AppError::McpValidation(
            "MCP 服务器连接定义必须为 JSON 对象".into(),
        ));
    }
    let t_opt = spec.get("type").and_then(|x| x.as_str());
    // 支持三种：stdio/http/sse；若缺省 type 则按 stdio 处理（与社区常见 .mcp.json 一致）
    let is_stdio = t_opt.map(|t| t == "stdio").unwrap_or(true);
    let is_http = t_opt.map(|t| t == "http").unwrap_or(false);
    let is_sse = t_opt.map(|t| t == "sse").unwrap_or(false);

    if !(is_stdio || is_http || is_sse) {
        return Err(AppError::McpValidation(
            "MCP 服务器 type 必须是 'stdio'、'http' 或 'sse'（或省略表示 stdio）".into(),
        ));
    }

    if is_stdio {
        let cmd = spec.get("command").and_then(|x| x.as_str()).unwrap_or("");
        if cmd.trim().is_empty() {
            return Err(AppError::McpValidation(
                "stdio 类型的 MCP 服务器缺少 command 字段".into(),
            ));
        }
    }
    if is_http {
        let url = spec.get("url").and_then(|x| x.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            return Err(AppError::McpValidation(
                "http 类型的 MCP 服务器缺少 url 字段".into(),
            ));
        }
    }
    if is_sse {
        let url = spec.get("url").and_then(|x| x.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            return Err(AppError::McpValidation(
                "sse 类型的 MCP 服务器缺少 url 字段".into(),
            ));
        }
    }
    Ok(())
}

/// 从 MCP 条目中提取服务器规范
pub fn extract_server_spec(entry: &Value) -> Result<Value, AppError> {
    let obj = entry
        .as_object()
        .ok_or_else(|| AppError::McpValidation("MCP 服务器条目必须为 JSON 对象".into()))?;
    let server = obj
        .get("server")
        .ok_or_else(|| AppError::McpValidation("MCP 服务器条目缺少 server 字段".into()))?;

    if !server.is_object() {
        return Err(AppError::McpValidation(
            "MCP 服务器 server 字段必须为 JSON 对象".into(),
        ));
    }

    Ok(server.clone())
}

/// Compare unified MCP specs after removing only representation-level
/// differences produced by supported adapters. Executable values and unknown
/// fields remain exact so a real cross-application conflict still fails closed.
pub(crate) fn server_specs_are_equivalent(left: &Value, right: &Value) -> bool {
    fn comparable(spec: &Value) -> Option<Value> {
        let mut object = spec.as_object()?.clone();

        object
            .entry("type".to_string())
            .or_insert_with(|| Value::String("stdio".to_string()));

        if let Some(http_headers) = object.remove("http_headers") {
            match object.get("headers") {
                None => {
                    object.insert("headers".to_string(), http_headers);
                }
                Some(headers) if headers == &http_headers => {}
                Some(_) => {
                    object.insert("http_headers".to_string(), http_headers);
                }
            }
        }

        for key in ["args", "env", "headers"] {
            let is_empty = object.get(key).is_some_and(|value| match value {
                Value::Array(values) => values.is_empty(),
                Value::Object(values) => values.is_empty(),
                _ => false,
            });
            if is_empty {
                object.remove(key);
            }
        }
        if object
            .get("cwd")
            .and_then(Value::as_str)
            .is_some_and(|cwd| cwd.trim().is_empty())
        {
            object.remove("cwd");
        }

        Some(Value::Object(object))
    }

    comparable(left)
        .zip(comparable(right))
        .is_some_and(|(left, right)| left == right)
}

/// Source clients treat a missing enable flag as active. Only an explicit
/// boolean false is authority to keep an imported assignment disabled.
pub(crate) fn source_server_is_enabled(spec: &Value) -> bool {
    spec.get("enabled").and_then(Value::as_bool) != Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn source_enablement_requires_an_explicit_boolean_false_to_disable() {
        assert!(!source_server_is_enabled(&json!({ "enabled": false })));
        assert!(source_server_is_enabled(&json!({ "enabled": true })));
        assert!(source_server_is_enabled(&json!({})));
        assert!(source_server_is_enabled(&json!({ "enabled": "false" })));
    }
}
