//! Effective Codex `cli_auth_credentials_store` resolver.
//!
//! Native `auth.json` projection is allowed for an explicit `file` value and
//! for unset (OpenAI Codex defaults unset to file). Explicit `auto`,
//! `keyring`, `ephemeral`, invalid, and future values fail closed. Existence
//! of `auth.json` is never a store hint.

use toml_edit::DocumentMut;

pub const CLI_AUTH_CREDENTIALS_STORE_FIELD: &str = "cli_auth_credentials_store";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexCredentialStore {
    File,
    Keyring,
    Auto,
    Ephemeral,
    Unset,
    Unknown,
}

impl CodexCredentialStore {
    #[allow(dead_code)]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Keyring => "keyring",
            Self::Auto => "auto",
            Self::Ephemeral => "ephemeral",
            Self::Unset => "unset",
            Self::Unknown => "unknown",
        }
    }

    /// OpenAI Codex treats an unset store as file. Explicit non-file values
    /// remain unsupported for FyAgent native projection.
    pub const fn allows_native_file_projection(self) -> bool {
        matches!(self, Self::File | Self::Unset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStoreResolveError {
    ConfigInvalid,
}

pub fn parse_cli_auth_credentials_store(
    config_toml: &str,
) -> Result<CodexCredentialStore, CredentialStoreResolveError> {
    let document = config_toml
        .parse::<DocumentMut>()
        .map_err(|_| CredentialStoreResolveError::ConfigInvalid)?;
    let Some(item) = document.get(CLI_AUTH_CREDENTIALS_STORE_FIELD) else {
        return Ok(CodexCredentialStore::Unset);
    };
    let Some(value) = item.as_str() else {
        return Ok(CodexCredentialStore::Unknown);
    };
    Ok(match value {
        "file" => CodexCredentialStore::File,
        "keyring" => CodexCredentialStore::Keyring,
        "auto" => CodexCredentialStore::Auto,
        "ephemeral" => CodexCredentialStore::Ephemeral,
        _ => CodexCredentialStore::Unknown,
    })
}

pub fn native_file_projection_allowed(
    config_toml: &str,
) -> Result<bool, CredentialStoreResolveError> {
    Ok(parse_cli_auth_credentials_store(config_toml)?.allows_native_file_projection())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_and_explicit_file_allow_native_projection() {
        assert_eq!(
            parse_cli_auth_credentials_store("cli_auth_credentials_store = \"file\"\n").unwrap(),
            CodexCredentialStore::File
        );
        assert_eq!(CodexCredentialStore::File.as_str(), "file");
        assert_eq!(CodexCredentialStore::Keyring.as_str(), "keyring");
        assert_eq!(CodexCredentialStore::Auto.as_str(), "auto");
        assert_eq!(CodexCredentialStore::Ephemeral.as_str(), "ephemeral");
        assert_eq!(CodexCredentialStore::Unset.as_str(), "unset");
        assert_eq!(CodexCredentialStore::Unknown.as_str(), "unknown");
        assert!(native_file_projection_allowed("cli_auth_credentials_store = \"file\"\n").unwrap());
        assert!(native_file_projection_allowed("").unwrap());
        assert!(native_file_projection_allowed("model = \"gpt-5\"\n").unwrap());
        for sample in [
            "cli_auth_credentials_store = \"keyring\"\n",
            "cli_auth_credentials_store = \"auto\"\n",
            "cli_auth_credentials_store = \"ephemeral\"\n",
            "cli_auth_credentials_store = \"future-store\"\n",
            "cli_auth_credentials_store = true\n",
        ] {
            assert!(
                !native_file_projection_allowed(sample).unwrap(),
                "unexpected projection for {sample:?}"
            );
        }
        assert_eq!(
            parse_cli_auth_credentials_store("[[[invalid"),
            Err(CredentialStoreResolveError::ConfigInvalid)
        );
    }

    #[test]
    fn auth_json_existence_is_not_consulted() {
        let parsed =
            parse_cli_auth_credentials_store("model = \"gpt-5\"\n# auth.json may exist on disk\n")
                .unwrap();
        assert_eq!(parsed, CodexCredentialStore::Unset);
        assert!(parsed.allows_native_file_projection());
    }
}
