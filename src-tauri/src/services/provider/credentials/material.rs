//! Private, versioned native material. This type is never an IPC/DB DTO.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::*;
use crate::provider::UsageScript;

const FAMILY: &[u8] = b"fyagent-provider-credential:";
const V1: &[u8] = b"fyagent-provider-credential:1\n";
pub(super) const MASK: &str = "[REDACTED]";

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InferenceTarget {
    endpoint: String,
    provider: Option<String>,
    protocol: Option<String>,
    openai_auth: Option<bool>,
    category: Option<String>,
}

impl InferenceTarget {
    fn of(provider: &Provider) -> Result<Self, AppError> {
        let config = provider
            .settings_config
            .get("config")
            .and_then(Value::as_str)
            .unwrap_or("");
        let parsed = config.parse::<toml::Value>().map_err(|_| invalid())?;
        let active = parsed.get("model_provider").and_then(toml::Value::as_str);
        let route = active
            .and_then(|id| parsed.get("model_providers").and_then(|p| p.get(id)))
            .unwrap_or(&parsed);
        Ok(Self {
            endpoint: crate::codex_config::extract_codex_base_url(config)
                .unwrap_or_default()
                .trim()
                .trim_end_matches('/')
                .to_owned(),
            provider: active.map(str::to_owned),
            protocol: route
                .get("wire_api")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            openai_auth: route
                .get("requires_openai_auth")
                .and_then(toml::Value::as_bool),
            category: provider.category.clone(),
        })
    }
}

pub(super) fn same_inference_target(
    previous: &Provider,
    previous_effective: &Provider,
    requested: &Provider,
    requested_effective: &Provider,
) -> Result<bool, AppError> {
    Ok(CredentialTargets::of(previous, previous_effective)?
        == CredentialTargets::of(requested, requested_effective)?)
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialTargets {
    configured: InferenceTarget,
    effective: InferenceTarget,
}

impl CredentialTargets {
    fn of(configured: &Provider, effective: &Provider) -> Result<Self, AppError> {
        Ok(Self {
            configured: InferenceTarget::of(configured)?,
            effective: InferenceTarget::of(effective)?,
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageTarget {
    base_url: String,
    language: String,
    code_digest: String,
    template: Option<String>,
    vendor: Option<String>,
    user: Option<String>,
    organization: Option<String>,
    project: Option<String>,
}

impl UsageTarget {
    fn of(provider: &Provider, script: &UsageScript) -> Self {
        let base_url = script
            .base_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                provider
                    .resolve_usage_credentials(&crate::app_config::AppType::Codex)
                    .0
            });
        Self {
            base_url: base_url.trim().trim_end_matches('/').to_owned(),
            language: script.language.clone(),
            code_digest: format!("{:x}", Sha256::digest(script.code.as_bytes())),
            template: script.template_type.clone(),
            vendor: script.coding_plan_provider.clone(),
            user: script.user_id.clone(),
            organization: script.team_organization_id.clone(),
            project: script.team_project_id.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageMaterial {
    target: UsageTarget,
    api_key: Option<String>,
    access_token: Option<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
}

impl Drop for UsageMaterial {
    fn drop(&mut self) {
        self.api_key.zeroize();
        self.access_token.zeroize();
        self.access_key_id.zeroize();
        self.secret_access_key.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CredentialMaterial {
    inference_key: String,
    inference_target: Option<CredentialTargets>,
    usage: Option<UsageMaterial>,
}

impl Drop for CredentialMaterial {
    fn drop(&mut self) {
        self.inference_key.zeroize();
    }
}

impl CredentialMaterial {
    /// Hash only route and script-target data, never credential values. This
    /// binds a non-serializable native draft through normalization to its
    /// actual destination at the final consumer boundary.
    pub(super) fn draft_fingerprint(
        provider: &Provider,
        effective: &Provider,
    ) -> Result<[u8; 32], AppError> {
        let usage = provider
            .meta
            .as_ref()
            .and_then(|m| m.usage_script.as_ref())
            .map(|script| UsageTarget::of(effective, script));
        let targets = (InferenceTarget::of(effective)?, usage);
        let encoded = serde_json::to_vec(&targets).map_err(|_| invalid())?;
        Ok(Sha256::digest(encoded).into())
    }

    pub(super) fn key(&self) -> &str {
        &self.inference_key
    }

    pub(super) fn capture(
        provider: &Provider,
        effective: &Provider,
        inference_key: &str,
        previous: Option<&Self>,
    ) -> Result<Self, AppError> {
        // None/empty explicitly clears auxiliary fields. A mask retains only
        // the same owner's value on exactly the same usage destination/script.
        let usage = provider.meta.as_ref().and_then(|m| m.usage_script.as_ref());
        let usage = usage
            .map(|script| {
                let target = UsageTarget::of(effective, script);
                let old = previous.and_then(|p| p.usage.as_ref());
                let field = |input: Option<&str>,
                             prior: Option<&str>|
                 -> Result<Option<String>, AppError> {
                    let Some(value) = input.map(str::trim).filter(|v| !v.is_empty()) else {
                        return Ok(None);
                    };
                    if blank_or_masked(value) {
                        if old.is_none_or(|old| old.target != target) {
                            return Err(invalid());
                        }
                        return prior.map(|v| Some(v.to_owned())).ok_or_else(invalid);
                    }
                    Ok(Some(value.to_owned()))
                };
                let api_key = field(
                    script.api_key.as_deref(),
                    old.and_then(|v| v.api_key.as_deref()),
                )?;
                let access_token = field(
                    script.access_token.as_deref(),
                    old.and_then(|v| v.access_token.as_deref()),
                )?;
                let access_key_id = field(
                    script.access_key_id.as_deref(),
                    old.and_then(|v| v.access_key_id.as_deref()),
                )?;
                let secret_access_key = field(
                    script.secret_access_key.as_deref(),
                    old.and_then(|v| v.secret_access_key.as_deref()),
                )?;
                Ok::<_, AppError>(UsageMaterial {
                    target,
                    api_key,
                    access_token,
                    access_key_id,
                    secret_access_key,
                })
            })
            .transpose()?;
        let material = Self {
            inference_key: inference_key.to_owned(),
            inference_target: Some(CredentialTargets::of(provider, effective)?),
            usage: usage.filter(|usage| Self::usage_values(usage).into_iter().any(|v| v.is_some())),
        };
        material.validate()?;
        Ok(material)
    }

    fn usage_values(usage: &UsageMaterial) -> [Option<&str>; 4] {
        [
            usage.api_key.as_deref(),
            usage.access_token.as_deref(),
            usage.access_key_id.as_deref(),
            usage.secret_access_key.as_deref(),
        ]
    }

    pub(super) fn values(&self) -> Vec<&str> {
        let mut values = vec![self.inference_key.as_str()];
        if let Some(usage) = &self.usage {
            values.extend(Self::usage_values(usage).into_iter().flatten());
        }
        values
    }

    pub(super) fn is_empty(&self) -> bool {
        self.inference_key.is_empty() && self.usage.is_none()
    }

    fn validate(&self) -> Result<(), AppError> {
        if (!self.inference_key.is_empty() && blank_or_masked(&self.inference_key))
            || self
                .values()
                .iter()
                .any(|value| value.chars().any(char::is_control))
            || self.usage.as_ref().is_some_and(|usage| {
                let values = Self::usage_values(usage);
                values.iter().all(Option::is_none)
                    || values.into_iter().flatten().any(blank_or_masked)
            })
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, AppError> {
        let material = if bytes.starts_with(FAMILY) {
            let encoded = bytes.strip_prefix(V1).ok_or_else(invalid)?;
            let decoded = serde_json::from_slice::<Self>(encoded).map_err(|_| invalid())?;
            if decoded.inference_target.is_none() {
                return Err(invalid());
            }
            decoded
        } else {
            // Historical records stored one opaque key. Structured/unknown
            // bundles and control-delimited values must never become API keys.
            let value = std::str::from_utf8(bytes).map_err(|_| invalid())?;
            if value.trim_start().starts_with('{') || value.trim_start().starts_with('[') {
                return Err(invalid());
            }
            Self {
                inference_key: value.to_owned(),
                inference_target: None,
                usage: None,
            }
        };
        material.validate()?;
        if material.is_empty() {
            return Err(invalid());
        }
        Ok(material)
    }

    pub(super) fn encode(&self) -> Result<Zeroizing<Vec<u8>>, AppError> {
        let mut bytes = Zeroizing::new(V1.to_vec());
        serde_json::to_writer(&mut *bytes, self).map_err(|_| invalid())?;
        Ok(bytes)
    }

    pub(super) fn matches(&self, bytes: &[u8]) -> Result<bool, AppError> {
        let mut decoded = Self::decode(bytes)?;
        // Legacy single-key records have no target envelope. Reuse still
        // requires this owner's freshly verified matching material.
        if decoded.inference_target.is_none() {
            decoded.inference_target = self.inference_target.clone();
        }
        let previous = decoded.encode()?;
        Ok(bool::from(
            previous.as_slice().ct_eq(self.encode()?.as_slice()),
        ))
    }

    pub(super) fn validate_projection(
        &self,
        provider: &Provider,
        effective: &Provider,
    ) -> Result<(), AppError> {
        self.validate_inference_projection(provider, effective)?;
        self.validate_usage_projection(provider, effective)
    }

    pub(super) fn validate_inference_projection(
        &self,
        provider: &Provider,
        effective: &Provider,
    ) -> Result<(), AppError> {
        if let Some(target) = &self.inference_target {
            if *target != CredentialTargets::of(provider, effective)? {
                return Err(invalid());
            }
        }
        Ok(())
    }

    pub(super) fn validate_usage_projection(
        &self,
        provider: &Provider,
        effective: &Provider,
    ) -> Result<(), AppError> {
        let script = provider.meta.as_ref().and_then(|m| m.usage_script.as_ref());
        if let Some(usage) = &self.usage {
            if script.is_none_or(|script| UsageTarget::of(effective, script) != usage.target) {
                return Err(invalid());
            }
        }
        Ok(())
    }

    pub(super) fn destination(effective: &Provider, script: &UsageScript) -> String {
        UsageTarget::of(effective, script).base_url
    }

    pub(super) fn same_request(
        effective: &Provider,
        stored_script: &UsageScript,
        requested: &UsageScript,
    ) -> bool {
        UsageTarget::of(effective, stored_script) == UsageTarget::of(effective, requested)
    }

    fn retained_usage_target(&self, stored_script: &UsageScript) -> UsageTarget {
        if let Some(usage) = &self.usage {
            return usage.target.clone();
        }
        let base_url = stored_script
            .base_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .or_else(|| {
                self.inference_target
                    .as_ref()
                    .map(|target| target.effective.endpoint.clone())
            })
            .unwrap_or_default();
        UsageTarget {
            base_url,
            language: stored_script.language.clone(),
            code_digest: format!("{:x}", Sha256::digest(stored_script.code.as_bytes())),
            template: stored_script.template_type.clone(),
            vendor: stored_script.coding_plan_provider.clone(),
            user: stored_script.user_id.clone(),
            organization: stored_script.team_organization_id.clone(),
            project: stored_script.team_project_id.clone(),
        }
    }

    pub(super) fn admit_test(
        &self,
        stored: &Provider,
        effective: &Provider,
        stored_script: &UsageScript,
        requested: &UsageScript,
    ) -> Result<(String, String, Option<String>), AppError> {
        let requested_target = UsageTarget::of(effective, requested);
        if self.retained_usage_target(stored_script) != requested_target {
            return Err(invalid());
        }
        if let Some(usage) = &self.usage {
            let api_key = usage.api_key.as_deref().filter(|value| !value.is_empty());
            let access_token = usage
                .access_token
                .as_deref()
                .filter(|value| !value.is_empty());
            if api_key.is_some() || access_token.is_some() {
                return Ok((
                    api_key.unwrap_or_default().to_owned(),
                    requested_target.base_url,
                    access_token.map(str::to_owned),
                ));
            }
        }
        self.validate_inference_projection(stored, effective)?;
        if self.key().is_empty() {
            return Err(invalid());
        }
        Ok((self.key().to_owned(), requested_target.base_url, None))
    }

    pub(super) fn project_usage(
        &self,
        provider: &mut Provider,
        native: bool,
    ) -> Result<(), AppError> {
        if let Some(script) = provider.meta.as_mut().and_then(|m| m.usage_script.as_mut()) {
            let field =
                |value: Option<&str>| value.map(|v| if native { v } else { MASK }.to_owned());
            let usage = self.usage.as_ref();
            script.api_key = field(usage.and_then(|u| u.api_key.as_deref()));
            script.access_token = field(usage.and_then(|u| u.access_token.as_deref()));
            script.access_key_id = field(usage.and_then(|u| u.access_key_id.as_deref()));
            script.secret_access_key = field(usage.and_then(|u| u.secret_access_key.as_deref()));
        }
        Ok(())
    }
}
