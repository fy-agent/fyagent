use super::collect_provider_credentials;
use crate::app_config::AppType;
use crate::services::provider_api::ApiProtocol;
use crate::services::{ProviderService, QuickSetupWriteTarget};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum LiveState {
    Configured,
    NotConfigured,
    Missing,
    Unreadable,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LiveConnection {
    base_url: Option<String>,
    model_id: Option<String>,
    protocol: Option<ApiProtocol>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProviderLiveSummary {
    target: String,
    state: LiveState,
    exists: Option<bool>,
    connection: Option<LiveConnection>,
}

impl ProviderLiveSummary {
    pub(super) fn unreadable(app: &AppType, exists: Option<bool>) -> Self {
        Self {
            target: app.as_str().to_string(),
            state: LiveState::Unreadable,
            exists,
            connection: None,
        }
    }
}

pub(super) fn observe(
    app: &AppType,
    targets: &[QuickSetupWriteTarget],
    known_credentials: &[String],
) -> ProviderLiveSummary {
    // The existing target owner defines each supported app's primary file.
    // Its display path is never converted back into filesystem authority.
    let [target] = targets else {
        return ProviderLiveSummary::unreadable(app, None);
    };
    if !target.exists {
        return ProviderLiveSummary {
            target: app.as_str().to_string(),
            state: LiveState::Missing,
            exists: Some(false),
            connection: None,
        };
    }
    let projected = ProviderService::read_live_settings(app.clone())
        .map_err(|_| ())
        .and_then(|settings| project(app, &settings, known_credentials));
    match projected {
        Ok(connection) => ProviderLiveSummary {
            target: app.as_str().to_string(),
            state: if connection.is_some() {
                LiveState::Configured
            } else {
                LiveState::NotConfigured
            },
            exists: Some(true),
            connection,
        },
        // Errors may contain raw configuration or paths. Neither log nor
        // serialize them; keep the independently read saved sources usable.
        Err(()) => ProviderLiveSummary::unreadable(app, Some(true)),
    }
}

fn safe_text(
    value: Option<&str>,
    limit: usize,
    credentials: &[String],
) -> Result<Option<String>, ()> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.len() > limit
        || value.chars().any(char::is_control)
        || credentials.iter().any(|secret| value.contains(secret))
    {
        return Err(());
    }
    let value = value.trim();
    Ok((!value.is_empty()).then(|| value.to_string()))
}

fn require_string_field(value: Option<&toml::Value>) -> Result<(), ()> {
    match value {
        None => Ok(()),
        Some(value) if value.is_str() => Ok(()),
        Some(_) => Err(()),
    }
}

fn toml_text(
    value: Option<&toml::Value>,
    limit: usize,
    credentials: &[String],
) -> Result<Option<String>, ()> {
    safe_text(
        value.map(|value| value.as_str().ok_or(())).transpose()?,
        limit,
        credentials,
    )
}

fn json_text(
    value: Option<&serde_json::Value>,
    limit: usize,
    credentials: &[String],
) -> Result<Option<String>, ()> {
    safe_text(
        value.map(|value| value.as_str().ok_or(())).transpose()?,
        limit,
        credentials,
    )
}

fn safe_endpoint(value: Option<String>, credentials: &[String]) -> Result<Option<String>, ()> {
    if let Some(value) = &value {
        let url = url::Url::parse(value).map_err(|_| ())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(());
        }
        for secret in credentials {
            crate::services::workbuddy::url::reject_parsed_url_credential_collision(&url, secret)
                .map_err(|_| ())?;
        }
    }
    Ok(value)
}

fn explicit_protocol(value: Option<String>, app: &AppType) -> Result<Option<ApiProtocol>, ()> {
    value
        .map(|value| {
            let protocol = match value.as_str() {
                "responses" => ApiProtocol::Responses,
                "chat" => ApiProtocol::Chat,
                _ => return Err(()),
            };
            ApiProtocol::for_target(app.as_str(), Some(protocol)).map_err(|_| ())
        })
        .transpose()
}

fn codex_connection(config: &toml::Value, credentials: &[String]) -> Result<LiveConnection, ()> {
    // Honor an explicitly selected file profile. Process/CLI overrides are
    // outside this file observation and are never inferred from the DB.
    let profile = match config.get("profile") {
        None => None,
        Some(profile) => Some(
            config
                .get("profiles")
                .and_then(|profiles| profiles.get(profile.as_str()?))
                .and_then(toml::Value::as_table)
                .ok_or(())?,
        ),
    };
    let field = |key| {
        profile
            .and_then(|profile| profile.get(key))
            .or_else(|| config.get(key))
    };
    let model_id = toml_text(field("model"), 256, credentials)?;
    let selected = toml_text(field("model_provider"), 256, credentials)?;
    let provider = selected.as_deref().and_then(|selected| {
        config
            .get("model_providers")
            .and_then(|all| all.get(selected))
    });
    if provider.is_some_and(|provider| !provider.is_table()) {
        return Err(());
    }
    // Formal extract_codex_base_url only sees active provider then root. Fold
    // the selected profile's route fields into a temporary root so precedence
    // is active provider > selected profile > file root. Wrong types fail
    // closed instead of falling through. Inactive profiles stay unread.
    for key in ["base_url", "wire_api"] {
        require_string_field(provider.and_then(|provider| provider.get(key)))?;
        require_string_field(profile.and_then(|profile| profile.get(key)))?;
        require_string_field(config.get(key))?;
    }
    let route_text = {
        let mut view = config.clone();
        if let Some(table) = view.as_table_mut() {
            if let Some(profile) = profile {
                for key in ["model", "model_provider", "base_url", "wire_api"] {
                    if let Some(value) = profile.get(key) {
                        table.insert(key.to_string(), value.clone());
                    }
                }
            }
            match selected.as_deref() {
                Some(id) => {
                    table.insert("model_provider".into(), toml::Value::String(id.to_owned()));
                }
                None => {
                    table.remove("model_provider");
                }
            }
        }
        toml::to_string(&view).map_err(|_| ())?
    };
    let view = route_text.parse::<toml::Value>().map_err(|_| ())?;
    let base_url = crate::codex_config::extract_codex_base_url(&route_text);
    let wire_api = provider
        .and_then(|provider| provider.get("wire_api"))
        .or_else(|| view.get("wire_api"));
    Ok(LiveConnection {
        base_url: safe_endpoint(
            safe_text(base_url.as_deref(), 2048, credentials)?,
            credentials,
        )?,
        model_id,
        // An omitted wire_api is deliberately null: this reports the file,
        // not the installed consumer version's implicit protocol default.
        protocol: explicit_protocol(toml_text(wire_api, 32, credentials)?, &AppType::Codex)?,
    })
}

fn project(
    app: &AppType,
    settings: &serde_json::Value,
    known_credentials: &[String],
) -> Result<Option<LiveConnection>, ()> {
    if !settings.is_object() {
        return Err(());
    }
    let mut credentials = known_credentials.to_vec();
    collect_provider_credentials(settings, &mut credentials)?;
    let connection = if matches!(app, AppType::Claude) {
        let env = settings.get("env");
        if env.is_some_and(|env| !env.is_object()) {
            return Err(());
        }
        LiveConnection {
            base_url: safe_endpoint(
                json_text(
                    env.and_then(|env| env.get("ANTHROPIC_BASE_URL")),
                    2048,
                    &credentials,
                )?,
                &credentials,
            )?,
            model_id: json_text(
                env.and_then(|env| env.get("ANTHROPIC_MODEL")),
                256,
                &credentials,
            )?,
            protocol: Some(ApiProtocol::Anthropic),
        }
    } else {
        let config = settings
            .get("config")
            .and_then(|value| value.as_str())
            .ok_or(())?
            .parse::<toml::Value>()
            .map_err(|_| ())?;
        collect_provider_credentials(
            &serde_json::to_value(&config).map_err(|_| ())?,
            &mut credentials,
        )?;
        match app {
            AppType::Codex => codex_connection(&config, &credentials)?,
            AppType::GrokBuild => {
                let selected = toml_text(
                    config.get("models").and_then(|v| v.get("default")),
                    256,
                    &credentials,
                )?;
                let model = selected
                    .as_deref()
                    .and_then(|id| config.get("model").and_then(|all| all.get(id)));
                if model.is_some_and(|model| !model.is_table()) {
                    return Err(());
                }
                LiveConnection {
                    base_url: safe_endpoint(
                        toml_text(model.and_then(|m| m.get("base_url")), 2048, &credentials)?,
                        &credentials,
                    )?,
                    model_id: toml_text(model.and_then(|m| m.get("model")), 256, &credentials)?,
                    protocol: explicit_protocol(
                        toml_text(model.and_then(|m| m.get("api_backend")), 32, &credentials)?,
                        app,
                    )?,
                }
            }
            _ => return Err(()),
        }
    };
    Ok((connection.base_url.is_some() || connection.model_id.is_some()).then_some(connection))
}

#[cfg(test)]
mod tests;
