use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const DEFAULT_MAX_SUMMARIES: usize = 8;
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_SUMMARIES_CAP: usize = 32;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub url: String,
    pub model: String,
    pub api_key: String,
    #[serde(default = "default_max_summaries")]
    pub max_summaries: usize,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

fn default_max_summaries() -> usize {
    DEFAULT_MAX_SUMMARIES
}

fn default_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件 {}", path.display()))?;
        let mut cfg: Self = toml::from_str(&raw).context("config.toml 解析失败")?;
        cfg.normalize();
        cfg.validate()?;
        Ok(cfg)
    }

    fn normalize(&mut self) {
        self.url = self.url.trim().trim_end_matches('/').to_string();
        self.model = self.model.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self.max_summaries = self.max_summaries.min(MAX_SUMMARIES_CAP);
        if self.timeout_secs == 0 {
            self.timeout_secs = DEFAULT_TIMEOUT_SECS;
        }
    }

    fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("config.toml 里的 url 不能为空");
        }
        if self.model.is_empty() {
            bail!("config.toml 里的 model 不能为空");
        }
        if self.api_key.is_empty() || self.api_key.contains("REPLACE") {
            bail!("请在 config.toml 中填写有效的 api_key");
        }
        Ok(())
    }

    pub fn responses_endpoint(&self) -> String {
        if self.url.ends_with("/responses") {
            self.url.clone()
        } else {
            format!("{}/responses", self.url)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_responses_path() {
        let cfg = Config {
            url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            api_key: "sk-test".into(),
            max_summaries: 8,
            timeout_secs: 60,
        };
        assert_eq!(
            cfg.responses_endpoint(),
            "https://api.openai.com/v1/responses"
        );
    }

    #[test]
    fn does_not_double_responses_path() {
        let cfg = Config {
            url: "https://example.com/v1/responses".into(),
            model: "gpt-4o-mini".into(),
            api_key: "sk-test".into(),
            max_summaries: 8,
            timeout_secs: 60,
        };
        assert_eq!(cfg.responses_endpoint(), "https://example.com/v1/responses");
    }

    #[test]
    fn parses_minimal_toml() {
        let raw = r#"
url = "https://api.openai.com/v1/"
model = "gpt-4o-mini"
api_key = "sk-test"
"#;
        let mut cfg: Config = toml::from_str(raw).unwrap();
        cfg.normalize();
        cfg.validate().unwrap();
        assert_eq!(cfg.url, "https://api.openai.com/v1");
        assert_eq!(cfg.max_summaries, 8);
        assert_eq!(cfg.timeout_secs, 60);
    }
}
