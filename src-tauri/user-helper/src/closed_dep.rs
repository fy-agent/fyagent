//! Bounded admission for the closed `@iarna/toml@3.0.0` global package.

use std::{
    fmt,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde::de::{self, Deserializer, MapAccess, Visitor};

pub const CLOSED_IARNA_TOML_NAME: &str = "@iarna/toml";
pub const CLOSED_IARNA_TOML_VERSION: &str = "3.0.0";
const MAX_PACKAGE_JSON_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosedDepDocumentError {
    Unreadable,
    InvalidDocument,
    MissingVersion,
    WrongType,
    DifferentVersion,
}

enum VersionField {
    String(String),
    Other,
}

struct PackageVersionDoc {
    version: Option<VersionField>,
}

impl<'de> serde::Deserialize<'de> for PackageVersionDoc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PackageVisitor;

        impl<'de> Visitor<'de> for PackageVisitor {
            type Value = PackageVersionDoc;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut version = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "version" {
                        if version.is_some() {
                            return Err(de::Error::custom("duplicate version"));
                        }
                        version = Some(match map.next_value::<serde_json::Value>()? {
                            serde_json::Value::String(value) => VersionField::String(value),
                            _ => VersionField::Other,
                        });
                    } else {
                        let _: serde_json::Value = map.next_value()?;
                    }
                }
                Ok(PackageVersionDoc { version })
            }
        }

        deserializer.deserialize_map(PackageVisitor)
    }
}

pub fn closed_iarna_toml_package_paths(prefix: &Path) -> [PathBuf; 2] {
    [
        prefix
            .join("lib")
            .join("node_modules")
            .join("@iarna")
            .join("toml")
            .join("package.json"),
        prefix
            .join("node_modules")
            .join("@iarna")
            .join("toml")
            .join("package.json"),
    ]
}

pub fn admit_closed_iarna_toml_package_json(content: &str) -> Result<(), ClosedDepDocumentError> {
    if content.len() > MAX_PACKAGE_JSON_BYTES || content.contains('\0') {
        return Err(ClosedDepDocumentError::InvalidDocument);
    }
    let document: PackageVersionDoc =
        serde_json::from_str(content).map_err(|_| ClosedDepDocumentError::InvalidDocument)?;
    match document.version {
        None => Err(ClosedDepDocumentError::MissingVersion),
        Some(VersionField::Other) => Err(ClosedDepDocumentError::WrongType),
        Some(VersionField::String(value)) if value == CLOSED_IARNA_TOML_VERSION => Ok(()),
        Some(VersionField::String(_)) => Err(ClosedDepDocumentError::DifferentVersion),
    }
}

pub fn admit_closed_iarna_toml_at_prefix(prefix: &Path) -> Result<(), ClosedDepDocumentError> {
    for candidate in closed_iarna_toml_package_paths(prefix) {
        match File::open(&candidate) {
            Ok(file) => admit_closed_iarna_toml_package_json(&read_bounded_package_json(file)?)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ClosedDepDocumentError::Unreadable),
        }
    }
    Ok(())
}

fn read_bounded_package_json(file: File) -> Result<String, ClosedDepDocumentError> {
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_JSON_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ClosedDepDocumentError::Unreadable)?;
    if bytes.len() > MAX_PACKAGE_JSON_BYTES {
        return Err(ClosedDepDocumentError::InvalidDocument);
    }
    String::from_utf8(bytes).map_err(|_| ClosedDepDocumentError::InvalidDocument)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn admits_exact_version_and_rejects_malformed_documents() {
        assert_eq!(
            admit_closed_iarna_toml_package_json(
                r#"{ "name": "@iarna/toml", "version": "3.0.0" }"#
            ),
            Ok(())
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.1"}"#),
            Err(ClosedDepDocumentError::DifferentVersion)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"name":"@iarna/toml"}"#),
            Err(ClosedDepDocumentError::MissingVersion)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":3}"#),
            Err(ClosedDepDocumentError::WrongType)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json("{"),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.0""#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
    }

    #[test]
    fn rejects_trailing_garbage_invalid_nested_values_and_duplicate_version() {
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.0"} garbage"#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.0","x":{garbage}}"#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.0","x":1e+}"#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"3.0.0","version":"3.0.0"}"#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"[]"#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#""3.0.0""#),
            Err(ClosedDepDocumentError::InvalidDocument)
        );
    }

    #[test]
    fn admits_unicode_fields_but_not_a_unicode_version() {
        assert_eq!(
            admit_closed_iarna_toml_package_json(
                r#"{"description":"中文\u4e13用","version":"3.0.0"}"#
            ),
            Ok(())
        );
        assert_eq!(
            admit_closed_iarna_toml_package_json(r#"{"version":"三.〇.〇"}"#),
            Err(ClosedDepDocumentError::DifferentVersion)
        );
    }

    #[test]
    fn prefix_probe_distinguishes_absent_exact_conflict_unreadable_and_oversize() {
        let unique = format!(
            "fyagent-closed-dep-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        fs::create_dir_all(&root).unwrap();
        let prefix = root.as_path();
        assert_eq!(admit_closed_iarna_toml_at_prefix(prefix), Ok(()));

        let package = prefix.join("node_modules").join("@iarna").join("toml");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.json"), r#"{"version":"3.0.0"}"#).unwrap();
        assert_eq!(admit_closed_iarna_toml_at_prefix(prefix), Ok(()));

        fs::write(package.join("package.json"), r#"{"version":"3.0.1"}"#).unwrap();
        assert_eq!(
            admit_closed_iarna_toml_at_prefix(prefix),
            Err(ClosedDepDocumentError::DifferentVersion)
        );

        fs::write(package.join("package.json"), "{").unwrap();
        assert_eq!(
            admit_closed_iarna_toml_at_prefix(prefix),
            Err(ClosedDepDocumentError::InvalidDocument)
        );

        fs::write(
            package.join("package.json"),
            format!(
                "{{\"version\":\"3.0.0\",\"pad\":\"{}\"}}",
                "x".repeat(MAX_PACKAGE_JSON_BYTES)
            ),
        )
        .unwrap();
        assert_eq!(
            admit_closed_iarna_toml_at_prefix(prefix),
            Err(ClosedDepDocumentError::InvalidDocument)
        );

        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::write(package.join("package.json"), r#"{"version":"3.0.0"}"#).unwrap();
            fs::set_permissions(
                package.join("package.json"),
                fs::Permissions::from_mode(0o000),
            )
            .unwrap();
            assert_eq!(
                admit_closed_iarna_toml_at_prefix(prefix),
                Err(ClosedDepDocumentError::Unreadable)
            );
            fs::set_permissions(
                package.join("package.json"),
                fs::Permissions::from_mode(0o644),
            )
            .unwrap();
        }
        let _ = fs::remove_dir_all(&root);
    }
}
