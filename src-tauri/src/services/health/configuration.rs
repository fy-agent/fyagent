//! Read-only configuration evidence; source documents never enter the wire DTO.
use super::types::{
    safe_value, HealthAction as Action, HealthCheck, HealthCheckId as Id,
    HealthCheckState as State, HealthReasonCode as Reason,
};
use crate::{app_config::AppType, database::Database};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Credential {
    Present,
    Missing,
    Native,
    Unknown,
    NotRequired,
}

/// Only the selected request route participates in drift checks.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct RoutingFacts {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub credential: Credential,
    pub official: bool,
    pub selected: Option<String>,
}

pub(super) struct ConfigurationObservation {
    pub checks: Vec<HealthCheck>,
    pub provider_id: Option<String>,
    pub facts: Option<RoutingFacts>,
    pub expected: Option<RoutingFacts>,
    pub live: Option<Value>,
}

pub(super) fn app_type(agent: crate::services::external_agents::AgentCatalogId) -> Option<AppType> {
    use crate::services::external_agents::AgentCatalogId::*;
    match agent {
        Codex => Some(AppType::Codex),
        ClaudeCode => Some(AppType::Claude),
        GrokBuild => Some(AppType::GrokBuild),
        OpenCode => Some(AppType::OpenCode),
        QoderWork | TraeWork | WorkBuddy => None,
    }
}

fn nonempty(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}
fn credential(value: Option<String>) -> Credential {
    match value {
        Some(value)
            if value.starts_with("{env:")
                || value.starts_with("{file:")
                || value == "PROXY_MANAGED" =>
        {
            Credential::Unknown
        }
        Some(_) => Credential::Present,
        None => Credential::Missing,
    }
}
fn toml_text(value: &Value) -> Result<(&str, toml::Value), ()> {
    let text = value.get("config").and_then(Value::as_str).ok_or(())?;
    Ok((text, text.parse().map_err(|_| ())?))
}

pub(super) fn routing(app: &AppType, value: &Value) -> Result<RoutingFacts, ()> {
    let mut facts = match app {
        AppType::Codex => {
            let (text, document) = toml_text(value)?;
            let selected = document
                .get("model_provider")
                .and_then(toml::Value::as_str)
                .map(str::to_owned);
            let local = selected
                .as_deref()
                .is_some_and(|s| ["ollama", "lmstudio", "oss", "ollama-chat"].contains(&s));
            let table = selected
                .as_deref()
                .and_then(|s| document.get("model_providers").and_then(|p| p.get(s)));
            let requires_native = table
                .and_then(|t| t.get("requires_openai_auth"))
                .and_then(toml::Value::as_bool)
                == Some(true);
            let official = selected.as_deref().is_none_or(|s| s == "openai") || requires_native;
            // An inactive API login retained in auth.json is not the active custom
            // provider's credential. Its own bearer token takes precedence.
            let material = crate::codex_config::extract_codex_experimental_bearer_token(text)
                .or_else(|| {
                    selected
                        .as_deref()
                        .is_none_or(|s| s == "openai")
                        .then(|| {
                            crate::codex_config::extract_codex_api_key(value.get("auth"), None)
                        })
                        .flatten()
                });
            let cred = if local {
                Credential::NotRequired
            } else if requires_native {
                Credential::Native
            } else if material.is_some() {
                credential(material)
            } else if official {
                Credential::Native
            } else {
                Credential::Unknown
            };
            RoutingFacts {
                endpoint: crate::codex_config::extract_codex_base_url(text),
                model: document
                    .get("model")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned),
                credential: cred,
                official,
                selected,
            }
        }
        AppType::Claude => {
            if !value.is_object() || value.get("env").is_some_and(|v| !v.is_object()) {
                return Err(());
            }
            let env = value.get("env");
            let key = nonempty(env.and_then(|v| v.get("ANTHROPIC_AUTH_TOKEN")))
                .or_else(|| nonempty(env.and_then(|v| v.get("ANTHROPIC_API_KEY"))));
            let endpoint = nonempty(env.and_then(|v| v.get("ANTHROPIC_BASE_URL")));
            let official = endpoint.is_none();
            RoutingFacts {
                endpoint,
                model: nonempty(value.get("model"))
                    .or_else(|| nonempty(env.and_then(|v| v.get("ANTHROPIC_MODEL")))),
                credential: if key.is_none() {
                    Credential::Native
                } else {
                    credential(key)
                },
                official,
                selected: None,
            }
        }
        AppType::GrokBuild => {
            let (text, _) = toml_text(value)?;
            let official = crate::grok_config::is_official_live_config(text);
            if official {
                RoutingFacts {
                    endpoint: None,
                    model: None,
                    credential: Credential::Native,
                    official,
                    selected: None,
                }
            } else {
                let model = crate::grok_config::extract_model_config(text).ok_or(())?;
                RoutingFacts {
                    endpoint: Some(model.base_url),
                    model: Some(model.model),
                    credential: if model.api_key.is_some() {
                        credential(model.api_key)
                    } else {
                        Credential::Unknown
                    },
                    official,
                    selected: Some(model.profile),
                }
            }
        }
        AppType::OpenCode => {
            if !value.is_object() || value.get("provider").is_some_and(|v| !v.is_object()) {
                return Err(());
            }
            let model = nonempty(value.get("model"));
            let selected = model
                .as_deref()
                .and_then(|s| s.split_once('/'))
                .map(|(p, _)| p.to_owned());
            let provider = selected
                .as_deref()
                .and_then(|p| value.get("provider").and_then(|v| v.get(p)));
            let options = provider.and_then(|p| p.get("options"));
            if provider.is_some_and(|v| !v.is_object()) || options.is_some_and(|v| !v.is_object()) {
                return Err(());
            }
            let key = nonempty(options.and_then(|o| o.get("apiKey")));
            RoutingFacts {
                endpoint: nonempty(options.and_then(|o| o.get("baseURL"))),
                model,
                credential: if key.is_some() {
                    credential(key)
                } else {
                    Credential::Native
                },
                // Only known built-in providers have an implicit endpoint.
                official: selected.as_deref().is_some_and(|id| {
                    ["openai", "xai", "github-copilot", "anthropic", "opencode"].contains(&id)
                }),
                selected,
            }
        }
        _ => return Err(()),
    };
    // A malicious model name must not echo any credential-valued field.
    if facts
        .model
        .as_ref()
        .is_some_and(|model| contains_secret(value, model))
    {
        facts.model = None;
    }
    Ok(facts)
}

fn contains_secret(value: &Value, model: &str) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            let sensitive = key.contains("token")
                || key.contains("secret")
                || key.contains("api_key")
                || key == "apikey";
            (sensitive
                && value
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && model.contains(s)))
                || contains_secret(value, model)
        }),
        Value::Array(values) => values.iter().any(|v| contains_secret(v, model)),
        // TOML credentials are never forwarded and must also taint model decorations.
        Value::String(text) => text
            .parse::<toml::Value>()
            .ok()
            .and_then(|t| serde_json::to_value(t).ok())
            .is_some_and(|t| {
                if t.is_object() {
                    contains_secret(&t, model)
                } else {
                    false
                }
            }),
        _ => false,
    }
}

pub(super) fn expected_routing(
    app: &AppType,
    id: &str,
    value: Value,
    live: Option<&RoutingFacts>,
) -> Result<RoutingFacts, ()> {
    if *app != AppType::OpenCode {
        return routing(app, &value);
    }
    let fragment = value
        .get("provider")
        .and_then(|v| v.get(id))
        .unwrap_or(&value);
    // OpenCode's provider rows don't own the selected model. Compare only the
    // selected provider's fragment, preserving this independent model choice.
    routing(
        app,
        &serde_json::json!({"model": live.and_then(|f| f.model.as_deref()), "provider": {id: fragment}}),
    )
}

pub(super) fn observe(db: &Database, app: &AppType, at: &str) -> ConfigurationObservation {
    let live = crate::services::provider::read_live_settings(app.clone());
    let facts = live
        .as_ref()
        .ok()
        .and_then(|value| routing(app, value).ok());
    // OpenCode is additive: its current model, not the last edited provider,
    // owns the active request route.
    let selection = if *app == AppType::OpenCode {
        Ok(facts.as_ref().and_then(|facts| facts.selected.clone()))
    } else {
        crate::settings::get_current_provider(app)
            .map(|id| Ok(Some(id)))
            .unwrap_or_else(|| db.get_current_provider(app.as_str()))
    };
    let providers = db.get_all_providers(app.as_str());
    let provider_id = selection.as_ref().ok().cloned().flatten();
    let selected = provider_id
        .as_ref()
        .and_then(|id| providers.as_ref().ok()?.get(id));
    // Never use get_effective_current_provider: it repairs stale settings while reading.
    let mut checks = Vec::new();
    let mut add =
        |id, state, reason, action| checks.push(HealthCheck::new(id, state, reason, action, at));
    let expected = selected.and_then(|p| {
        let value = crate::services::provider::build_health_settings_projection(db, app, p).ok()?;
        expected_routing(app, &p.id, value, facts.as_ref()).ok()
    });
    if let Some(facts) = &facts {
        add(
            Id::Configuration,
            State::Ok,
            Reason::ConfigurationPresent,
            Some(Action::Configuration),
        );
        let (state, reason) = match facts.credential {
            Credential::Present => (State::Ok, Reason::CredentialAvailable),
            Credential::Missing => (State::Blocked, Reason::CredentialMissing),
            Credential::Native | Credential::Unknown => (State::Unknown, Reason::CredentialUnknown),
            Credential::NotRequired => (State::Ok, Reason::CredentialNotRequired),
        };
        add(Id::Secret, state, reason, Some(Action::Configuration));
        let (state, reason) = match facts.endpoint.as_deref() {
            Some(endpoint) if valid_endpoint(endpoint) => (State::Ok, Reason::EndpointConfigured),
            Some(_) => (State::Blocked, Reason::EndpointInvalid),
            None if facts.official || facts.credential == Credential::NotRequired => {
                (State::Ok, Reason::EndpointConfigured)
            }
            None => (State::Unknown, Reason::EndpointMissing),
        };
        add(Id::Endpoint, state, reason, Some(Action::Configuration));
        add(
            Id::Model,
            if facts.model.is_some() {
                State::Ok
            } else {
                State::Unknown
            },
            if facts.model.is_some() {
                Reason::ModelConfigured
            } else {
                Reason::ModelUnknown
            },
            Some(Action::Configuration),
        );
        // Expected/live comparison excludes MCP, personal preferences and retained inactive providers.
        let (state, reason) = if selection.is_err() || providers.is_err() {
            (State::Unknown, Reason::ReadFailed)
        } else if provider_id.is_some() && selected.is_none() {
            (State::Attention, Reason::ConfigurationDrifted)
        } else if let Some(expected) = &expected {
            if expected == facts {
                (State::Ok, Reason::ConfigurationInSync)
            } else {
                (State::Attention, Reason::ConfigurationDrifted)
            }
        } else {
            (State::Unknown, Reason::ConfigurationDriftUnknown)
        };
        add(Id::Drift, state, reason, Some(Action::Configuration));
    } else {
        let missing = match app {
            AppType::Codex => {
                !crate::codex_config::get_codex_config_path().exists()
                    && !crate::codex_config::get_codex_auth_path().exists()
            }
            AppType::Claude => !crate::config::get_claude_settings_path().exists(),
            AppType::GrokBuild => !crate::grok_config::get_grok_config_path().exists(),
            AppType::OpenCode => !crate::opencode_config::get_opencode_config_path().exists(),
            _ => false,
        };
        add(
            Id::Configuration,
            if missing {
                State::NotConfigured
            } else {
                State::Blocked
            },
            if missing {
                Reason::ConfigurationMissing
            } else {
                Reason::ConfigurationUnreadable
            },
            Some(Action::Configuration),
        );
        for id in [Id::Secret, Id::Endpoint, Id::Model, Id::Drift] {
            add(
                id,
                State::Unknown,
                Reason::ReadFailed,
                Some(Action::Configuration),
            );
        }
    }
    if let Some(facts) = &facts {
        if let Some(check) = checks.iter_mut().find(|c| c.id == Id::Model) {
            check.value = facts.model.as_deref().and_then(safe_value);
        }
    }
    ConfigurationObservation {
        checks,
        provider_id,
        facts,
        expected,
        live: live.ok(),
    }
}

pub(super) fn valid_endpoint(endpoint: &str) -> bool {
    url::Url::parse(endpoint).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}
