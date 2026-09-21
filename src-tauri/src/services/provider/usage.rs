//! Usage script execution
//!
//! Handles executing and formatting usage query results.

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::{UsageData, UsageResult, UsageScript};
use crate::settings;
use crate::store::AppState;
use crate::usage_script;

/// Execute usage script and format result (private helper method)
pub(crate) async fn execute_and_format_usage_result(
    script_code: &str,
    api_key: &str,
    base_url: &str,
    timeout: u64,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
) -> Result<UsageResult, AppError> {
    match usage_script::execute_usage_script(
        script_code,
        api_key,
        base_url,
        timeout,
        access_token,
        user_id,
        template_type,
    )
    .await
    {
        Ok(data) => {
            let usage_list: Vec<UsageData> = if data.is_array() {
                serde_json::from_value(data).map_err(|e| {
                    AppError::localized(
                        "usage_script.data_format_error",
                        format!("数据格式错误: {e}"),
                        format!("Data format error: {e}"),
                    )
                })?
            } else {
                let single: UsageData = serde_json::from_value(data).map_err(|e| {
                    AppError::localized(
                        "usage_script.data_format_error",
                        format!("数据格式错误: {e}"),
                        format!("Data format error: {e}"),
                    )
                })?;
                vec![single]
            };

            Ok(UsageResult {
                success: true,
                data: Some(usage_list),
                error: None,
            })
        }
        Err(err) => {
            // 瞬时传输失败（send 失败/超时、读体中断）以 Err 传播，让前端 invoke
            // reject → react-query retry 并保留上次成功值；按错误 key 判定而非
            // 文案匹配。其余脚本/配置/HTTP 业务错误折叠成 success:false 展示文案。
            if let AppError::Localized { key, .. } = &err {
                if matches!(
                    *key,
                    "usage_script.request_failed" | "usage_script.read_response_failed"
                ) {
                    return Err(err);
                }
            }

            let lang = settings::get_settings()
                .language
                .unwrap_or_else(|| "zh".to_string());

            let msg = match err {
                AppError::Localized { zh, en, .. } => {
                    if lang == "en" {
                        en
                    } else {
                        zh
                    }
                }
                other => other.to_string(),
            };

            Ok(UsageResult {
                success: false,
                data: None,
                error: Some(msg),
            })
        }
    }
}

/// Resolve `(api_key, base_url)` for the JS-script path: explicit non-empty
/// script values win, otherwise fall back to the provider's stored config via
/// `Provider::resolve_usage_credentials` — the same per-app resolver the
/// native balance/coding-plan path and the frontend `getProviderCredentials`
/// use, so `{{apiKey}}`/`{{baseUrl}}` match what the UI shows for them.
fn resolve_script_credentials(
    app_type: &AppType,
    provider: &crate::provider::Provider,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> (String, String) {
    let (provider_base_url, provider_api_key) = provider.resolve_usage_credentials(app_type);

    let api_key = api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or(provider_api_key);

    let base_url = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        // Trim like the provider path so `{{baseUrl}}/path` never doubles the slash.
        .map(|value| value.trim_end_matches('/').to_owned())
        .unwrap_or(provider_base_url);

    (api_key, base_url)
}

/// Query provider usage (using saved script configuration)
pub async fn query_usage(
    state: &AppState,
    app_type: AppType,
    provider_id: &str,
) -> Result<UsageResult, AppError> {
    let (script_code, timeout, api_key, base_url, access_token, user_id, template_type) = {
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers.get(provider_id).ok_or_else(|| {
            AppError::localized(
                "provider.not_found",
                format!("供应商不存在: {provider_id}"),
                format!("Provider not found: {provider_id}"),
            )
        })?;

        let provider = super::ProviderCredentials::resolve(&state.db, app_type.as_str(), provider)?;
        let usage_script = provider
            .meta
            .as_ref()
            .and_then(|m| m.usage_script.as_ref())
            .ok_or_else(|| {
                AppError::localized(
                    "provider.usage.script.missing",
                    "未配置用量查询脚本",
                    "Usage script is not configured",
                )
            })?;
        if !usage_script.enabled {
            return Err(AppError::localized(
                "provider.usage.disabled",
                "用量查询未启用",
                "Usage query is disabled",
            ));
        }

        // Get credentials: prioritize UsageScript values, fallback to provider config
        let (api_key, base_url) = resolve_script_credentials(
            &app_type,
            &provider,
            usage_script.api_key.as_deref(),
            usage_script.base_url.as_deref(),
        );

        (
            usage_script.code.clone(),
            usage_script.timeout.unwrap_or(10),
            api_key,
            base_url,
            usage_script.access_token.clone(),
            usage_script.user_id.clone(),
            usage_script.template_type.clone(),
        )
    };

    execute_and_format_usage_result(
        &script_code,
        &api_key,
        &base_url,
        timeout,
        access_token.as_deref(),
        user_id.as_deref(),
        template_type.as_deref(),
    )
    .await
}

/// Test usage script (using temporary script content, not saved)
#[allow(clippy::too_many_arguments)]
pub async fn test_usage_script(
    state: &AppState,
    app_type: AppType,
    provider_id: &str,
    script_code: &str,
    timeout: u64,
    api_key: Option<&str>,
    base_url: Option<&str>,
    access_token: Option<&str>,
    user_id: Option<&str>,
    template_type: Option<&str>,
) -> Result<UsageResult, AppError> {
    let providers = state.db.get_all_providers(app_type.as_str())?;
    let provider = providers.get(provider_id).ok_or_else(|| {
        AppError::localized(
            "provider.not_found",
            format!("供应商不存在: {provider_id}"),
            format!("Provider not found: {provider_id}"),
        )
    })?;

    if matches!(app_type, AppType::Codex) {
        let admitted = super::ProviderCredentials::admit_usage_test(
            &state.db,
            app_type.as_str(),
            provider,
            &super::credentials::UsageTestInput {
                script_code,
                api_key,
                base_url,
                access_token,
                user_id,
                template_type,
            },
        )?;
        return execute_and_format_usage_result(
            script_code,
            &admitted.api_key,
            &admitted.base_url,
            timeout,
            admitted.access_token.as_deref(),
            admitted.user_id.as_deref(),
            admitted.template_type.as_deref(),
        )
        .await;
    }

    let provider = super::ProviderCredentials::resolve(&state.db, app_type.as_str(), provider)?;
    let (api_key, base_url) = resolve_script_credentials(&app_type, &provider, api_key, base_url);
    execute_and_format_usage_result(
        script_code,
        &api_key,
        &base_url,
        timeout,
        access_token,
        user_id,
        template_type,
    )
    .await
}

/// Validate UsageScript configuration (boundary checks)
pub(crate) fn validate_usage_script(script: &UsageScript) -> Result<(), AppError> {
    // Validate auto query interval (0-1440 minutes, max 24 hours)
    if let Some(interval) = script.auto_query_interval {
        if interval > 1440 {
            return Err(AppError::localized(
                "usage_script.interval_too_large",
                format!("自动查询间隔不能超过 1440 分钟（24小时），当前值: {interval}"),
                format!(
                    "Auto query interval cannot exceed 1440 minutes (24 hours), current: {interval}"
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_script_credentials;
    use crate::app_config::AppType;
    use crate::provider::Provider;
    use serde_json::json;

    fn provider_with_settings(settings_config: serde_json::Value) -> Provider {
        Provider::with_id(
            "provider-1".to_string(),
            "Provider".to_string(),
            settings_config,
            None,
        )
    }

    #[test]
    fn script_values_override_provider_credentials() {
        let provider = provider_with_settings(json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "provider-key",
                "ANTHROPIC_BASE_URL": "https://provider.example.com/"
            }
        }));

        let (api_key, base_url) = resolve_script_credentials(
            &AppType::Claude,
            &provider,
            Some(" script-key "),
            Some(" https://script.example.com/ "),
        );
        assert_eq!(api_key, "script-key");
        assert_eq!(base_url, "https://script.example.com");
    }

    #[test]
    fn empty_script_values_fall_back_to_provider_credentials() {
        let provider = provider_with_settings(json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "provider-key",
                "ANTHROPIC_BASE_URL": "https://provider.example.com/"
            }
        }));

        let (api_key, base_url) =
            resolve_script_credentials(&AppType::Claude, &provider, Some(""), None);
        assert_eq!(api_key, "provider-key");
        assert_eq!(base_url, "https://provider.example.com");
    }

    #[test]
    fn codex_fallback_reads_auth_and_config_toml() {
        let provider = provider_with_settings(json!({
            "auth": {
                "OPENAI_API_KEY": "openai-key"
            },
            "config": r#"model_provider = "azure"

[model_providers.azure]
base_url = "https://azure.example.com/v1/"

[model_providers.other]
base_url = "https://other.example.com/v1"
"#
        }));

        let (api_key, base_url) =
            resolve_script_credentials(&AppType::Codex, &provider, None, None);
        assert_eq!(api_key, "openai-key");
        assert_eq!(base_url, "https://azure.example.com/v1");
    }

    const USAGE_KEY: &str = "fixture-usage-api-key-canary";
    const USAGE_TOKEN: &str = "fixture-usage-access-token-canary";
    const INFERENCE_KEY: &str = "fixture-inference-canary";

    fn https_reject_script() -> String {
        "(function() { if (!'{{apiKey}}') throw new Error('fixture missing native material'); return {request:{url:'http://example.invalid',method:'GET'}}; })()".into()
    }

    fn usage_provider(include_usage_key: bool) -> Provider {
        use crate::provider::{ProviderMeta, UsageScript};
        let mut provider = Provider::with_id(
            "fixture-codex".into(),
            "Fixture".into(),
            json!({
                "auth": {"OPENAI_API_KEY": INFERENCE_KEY},
                "config": "model_provider = 'custom'\nmodel = 'fixture-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n"
            }),
            None,
        );
        provider.meta = Some(ProviderMeta {
            usage_script: Some(UsageScript {
                enabled: true,
                language: "javascript".into(),
                code: https_reject_script(),
                timeout: None,
                api_key: include_usage_key.then(|| USAGE_KEY.to_owned()),
                base_url: Some("https://usage.example.invalid/api".into()),
                access_token: None,
                user_id: None,
                template_type: Some("token_plan".into()),
                auto_query_interval: None,
                coding_plan_provider: Some("volcengine".into()),
                access_key_id: None,
                secret_access_key: None,
                team_organization_id: None,
                team_project_id: None,
            }),
            ..Default::default()
        });
        provider
    }

    fn run_test(
        state: &crate::store::AppState,
        provider_id: &str,
        script: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
        user_id: Option<&str>,
        template_type: Option<&str>,
    ) -> Result<crate::provider::UsageResult, crate::error::AppError> {
        run_test_with(
            state,
            provider_id,
            super::super::credentials::UsageTestInput {
                script_code: script,
                api_key,
                base_url,
                access_token: None,
                user_id,
                template_type,
            },
        )
    }

    fn run_test_with(
        state: &crate::store::AppState,
        provider_id: &str,
        input: super::super::credentials::UsageTestInput<'_>,
    ) -> Result<crate::provider::UsageResult, crate::error::AppError> {
        futures::executor::block_on(super::test_usage_script(
            state,
            AppType::Codex,
            provider_id,
            input.script_code,
            5,
            input.api_key,
            input.base_url,
            input.access_token,
            input.user_id,
            input.template_type,
        ))
    }

    fn assert_https_reject(result: crate::provider::UsageResult) {
        assert!(!result.success, "{result:?}");
        assert!(
            result.error.as_deref().unwrap().contains("HTTPS"),
            "{result:?}"
        );
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains(USAGE_KEY));
        assert!(!text.contains(USAGE_TOKEN));
        assert!(!text.contains(INFERENCE_KEY));
    }

    fn assert_rejected_without_execution(
        result: Result<crate::provider::UsageResult, crate::error::AppError>,
    ) {
        let err = result.expect_err("changed target must not execute");
        assert_eq!(err.to_string(), "provider_secret_invalid");
    }

    #[test]
    #[serial_test::serial]
    fn usage_script_test_binds_saved_secrets_to_the_same_target() {
        super::super::tests::with_test_home(|state, _| {
            let provider = usage_provider(true);
            let script = provider
                .meta
                .as_ref()
                .unwrap()
                .usage_script
                .as_ref()
                .unwrap()
                .code
                .clone();
            state.db.save_provider("codex", &provider).unwrap();
            for mask in [None, Some(""), Some("********"), Some("[REDACTED]")] {
                assert_https_reject(
                    run_test(state, &provider.id, &script, mask, None, None, None).unwrap(),
                );
            }
            assert_https_reject(
                run_test(
                    state,
                    &provider.id,
                    &script,
                    Some("fixture-fresh-usage-key"),
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
        });
    }

    #[test]
    #[serial_test::serial]
    fn usage_script_test_rejects_blank_key_when_target_changes() {
        super::super::tests::with_test_home(|state, _| {
            let provider = usage_provider(true);
            let script = provider
                .meta
                .as_ref()
                .unwrap()
                .usage_script
                .as_ref()
                .unwrap()
                .code
                .clone();
            state.db.save_provider("codex", &provider).unwrap();
            for (code, url, user, template) in [
                (format!("{};0", https_reject_script()), None, None, None),
                (
                    script.clone(),
                    Some("https://changed.example.invalid/v1"),
                    None,
                    None,
                ),
                (script.clone(), None, Some("other-user"), None),
                (script.clone(), None, None, Some("custom")),
            ] {
                assert_rejected_without_execution(run_test(
                    state,
                    &provider.id,
                    &code,
                    Some(""),
                    url,
                    user,
                    template,
                ));
                assert_rejected_without_execution(run_test(
                    state,
                    &provider.id,
                    &code,
                    Some("********"),
                    url,
                    user,
                    template,
                ));
            }
        });
    }

    #[test]
    #[serial_test::serial]
    fn usage_script_test_allows_fresh_credentials_for_a_changed_target() {
        super::super::tests::with_test_home(|state, _| {
            let provider = usage_provider(true);
            state.db.save_provider("codex", &provider).unwrap();
            let fresh = "fixture-fresh-usage-key";
            assert_https_reject(
                run_test(
                    state,
                    &provider.id,
                    &https_reject_script(),
                    Some(fresh),
                    Some("https://changed.example.invalid/v1"),
                    Some("other-user"),
                    Some("newapi"),
                )
                .unwrap(),
            );
        });
    }

    #[test]
    #[serial_test::serial]
    fn usage_script_test_falls_back_to_inference_key_only_for_the_bound_target() {
        super::super::tests::with_test_home(|state, _| {
            let mut provider = usage_provider(false);
            provider.meta.as_mut().unwrap().common_config_enabled = Some(true);
            let script = provider
                .meta
                .as_ref()
                .unwrap()
                .usage_script
                .as_ref()
                .unwrap()
                .code
                .clone();
            state.db.save_provider("codex", &provider).unwrap();
            assert_https_reject(
                run_test(state, &provider.id, &script, Some(""), None, None, None).unwrap(),
            );
            state
                .db
                .set_config_snippet(
                    "codex",
                    Some(
                        "[model_providers.custom]\nbase_url = 'https://redirect.example.invalid/v1'\n"
                            .into(),
                    ),
                )
                .unwrap();
            assert_rejected_without_execution(run_test(
                state,
                &provider.id,
                &script,
                Some(""),
                None,
                None,
                None,
            ));
        });
    }

    fn token_only_provider() -> Provider {
        use crate::provider::{ProviderMeta, UsageScript};
        let mut provider = Provider::with_id(
            "fixture-codex".into(),
            "Fixture".into(),
            json!({
                "auth": {},
                "config": "model_provider = 'custom'\nmodel = 'fixture-model'\n[model_providers.custom]\nname = 'Fixture'\nbase_url = 'https://example.invalid/v1'\nwire_api = 'responses'\n"
            }),
            None,
        );
        provider.meta = Some(ProviderMeta {
            usage_script: Some(UsageScript {
                enabled: true,
                language: "javascript".into(),
                code: "(function() { if (!'{{accessToken}}') throw new Error('fixture missing native material'); return {request:{url:'http://example.invalid',method:'GET'}}; })()".into(),
                timeout: None,
                api_key: None,
                base_url: Some("https://usage.example.invalid/api".into()),
                access_token: Some(USAGE_TOKEN.to_owned()),
                user_id: None,
                template_type: Some("newapi".into()),
                auto_query_interval: None,
                coding_plan_provider: None,
                access_key_id: None,
                secret_access_key: None,
                team_organization_id: None,
                team_project_id: None,
            }),
            ..Default::default()
        });
        provider
    }

    #[test]
    #[serial_test::serial]
    fn usage_script_test_allows_token_only_same_target_and_rejects_changes() {
        super::super::tests::with_test_home(|state, _| {
            let provider = token_only_provider();
            let script = provider
                .meta
                .as_ref()
                .unwrap()
                .usage_script
                .as_ref()
                .unwrap()
                .code
                .clone();
            state.db.save_provider("codex", &provider).unwrap();
            assert_https_reject(
                run_test_with(
                    state,
                    &provider.id,
                    super::super::credentials::UsageTestInput {
                        script_code: &script,
                        api_key: Some(""),
                        base_url: None,
                        access_token: Some("********"),
                        user_id: None,
                        template_type: None,
                    },
                )
                .unwrap(),
            );
            assert_rejected_without_execution(run_test_with(
                state,
                &provider.id,
                super::super::credentials::UsageTestInput {
                    script_code: &script,
                    api_key: Some(""),
                    base_url: Some("https://changed.example.invalid/v1"),
                    access_token: Some("********"),
                    user_id: None,
                    template_type: None,
                },
            ));
            let fresh = "fixture-fresh-access-token";
            assert_https_reject(
                run_test_with(
                    state,
                    &provider.id,
                    super::super::credentials::UsageTestInput {
                        script_code: &script,
                        api_key: Some(""),
                        base_url: Some("https://changed.example.invalid/v1"),
                        access_token: Some(fresh),
                        user_id: None,
                        template_type: Some("newapi"),
                    },
                )
                .unwrap(),
            );
        });
    }
}
