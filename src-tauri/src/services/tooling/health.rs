//! Filesystem-only observations. Finding a file does not establish runnability.

use std::collections::HashSet;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use super::{build_tool_search_paths, infer_install_source, tool_executable_candidates};

const MAX_PACKAGE_BYTES: u64 = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LocalToolHealth {
    pub observed_count: usize,
    pub version: Option<String>,
    pub source: Option<String>,
}

/// Only the closed tool names are accepted, before consulting any user paths.
/// This path must never delegate to version, shell, helper or network probes.
pub(crate) fn observe_local_tool_health(tool: &str) -> Result<LocalToolHealth, ()> {
    package_for_health(tool).ok_or(())?;
    observe_in_directories(tool, &build_tool_search_paths(tool))
}

fn package_for_health(tool: &str) -> Option<(&'static str, &'static str)> {
    match tool {
        "claude" => Some(("@anthropic-ai", "claude-code")),
        "grok" => Some(("@xai-official", "grok")),
        "codex" => Some(("@openai", "codex")),
        _ => None,
    }
}

fn observe_in_directories(tool: &str, directories: &[PathBuf]) -> Result<LocalToolHealth, ()> {
    let package = package_for_health(tool).ok_or(())?;
    let mut seen = HashSet::new();
    let mut version = None;
    let mut source = None;
    for directory in directories {
        if !directory.is_absolute() {
            continue;
        }
        #[cfg(target_os = "windows")]
        if !crate::windows_runtime::is_local_command_path(directory) {
            continue;
        }
        for candidate in tool_executable_candidates(tool, directory) {
            let metadata = match std::fs::metadata(&candidate) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(_) => return Err(()),
            };
            if !metadata.is_file() {
                continue;
            }
            let real = std::fs::canonicalize(&candidate).map_err(|_| ())?;
            #[cfg(target_os = "windows")]
            {
                // canonicalize emits a verbatim drive prefix on Windows. Only
                // remove that generated prefix for the existing local-drive
                // policy; UNC/device results still fail its drive-root check.
                let policy_path = real
                    .to_str()
                    .and_then(|path| path.strip_prefix(r"\\?\"))
                    .map(Path::new)
                    .unwrap_or(&real);
                if !crate::windows_runtime::is_local_command_path(policy_path) {
                    continue;
                }
            }
            // mise shims may all resolve to the same dispatcher, even when the
            // named package is absent. Installed Node package files remain valid.
            if is_mise_dispatcher(&real) || !seen.insert(real.clone()) {
                continue;
            }
            if seen.len() == 1 {
                version = package_version(&real, package);
                source = Some(infer_install_source(&real).to_string());
            } else {
                // No process was run to determine a PATH default. Do not attach
                // one candidate's metadata to an ambiguous installation set.
                version = None;
                source = None;
            }
        }
    }
    Ok(LocalToolHealth {
        observed_count: seen.len(),
        version,
        source,
    })
}

fn is_mise_dispatcher(path: &Path) -> bool {
    let components: Vec<_> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    matches!(
        components.last().map(String::as_str),
        Some("mise" | "mise.exe")
    ) || components
        .windows(2)
        .any(|pair| matches!(pair[0].as_str(), "mise" | ".mise") && pair[1] == "shims")
}

fn package_version(real: &Path, (scope, package): (&str, &str)) -> Option<String> {
    // Read only the matching npm package containing this actual file. A nearby
    // package or a Windows launcher beside node_modules is insufficient proof.
    let root = real.ancestors().skip(1).take(10).find(|ancestor| {
        ancestor.file_name().is_some_and(|name| name == package)
            && ancestor
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == scope)
            && ancestor
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .is_some_and(|name| name == "node_modules")
    })?;
    let manifest = root.join("package.json");
    // A manifest symlink must not turn this bounded package read into arbitrary
    // user-file access. The containing package path is already canonical.
    let metadata = std::fs::symlink_metadata(&manifest).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_PACKAGE_BYTES {
        return None;
    }
    if std::fs::canonicalize(&manifest).ok()?.parent()? != root {
        return None;
    }
    let file = File::open(manifest).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_PACKAGE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if value.get("name")?.as_str()? != format!("{scope}/{package}") {
        return None;
    }
    let version = value.get("version")?.as_str()?;
    // Never forward arbitrary metadata, including valid SemVer build/prerelease
    // identifiers that can contain credentials or user-defined labels.
    if version.len() > 32
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return None;
    }
    semver::Version::parse(version).ok()?;
    Some(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fake_tool(directory: &Path, tool: &str) -> PathBuf {
        std::fs::create_dir_all(directory).unwrap();
        #[cfg(target_os = "windows")]
        let path = directory.join(format!("{tool}.cmd"));
        #[cfg(target_os = "macos")]
        let path = directory.join(tool);
        let marker = directory.join("executed-marker");
        #[cfg(target_os = "windows")]
        let script = format!("@echo off\r\necho executed > \"{}\"\r\n", marker.display());
        #[cfg(target_os = "macos")]
        let script = format!("#!/bin/sh\nprintf executed > '{}'\n", marker.display());
        std::fs::write(&path, script).unwrap();
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn package_fixture(base: &Path, tool: &str, version: &str) -> PathBuf {
        let (scope, package) = package_for_health(tool).unwrap();
        let root = base.join("node_modules").join(scope).join(package);
        let bin = root.join("bin");
        fake_tool(&bin, tool);
        std::fs::write(
            root.join("package.json"),
            serde_json::json!({
                "name": format!("{scope}/{package}"),
                "version": version,
                "token": "fixture-private-token",
                "source": "/private/fixture-path"
            })
            .to_string(),
        )
        .unwrap();
        bin
    }

    #[test]
    fn finds_files_without_executing_them_or_claiming_a_version() {
        let temp = TempDir::new().unwrap();
        for tool in ["claude", "grok", "codex"] {
            let bin = temp.path().join(tool);
            fake_tool(&bin, tool);
            let observation = observe_in_directories(tool, &[bin.clone(), bin.clone()]).unwrap();
            assert_eq!(observation.observed_count, 1);
            assert_eq!(observation.version, None);
            assert_eq!(observation.source.as_deref(), Some("system"));
            assert!(!bin.join("executed-marker").exists());
        }
        assert!(observe_local_tool_health("codex; touch marker").is_err());
        assert!(observe_in_directories("../codex", &[]).is_err());
    }

    #[test]
    fn missing_files_and_directories_are_not_installations() {
        let temp = TempDir::new().unwrap();
        let bin = temp.path().join("bin");
        std::fs::create_dir_all(bin.join("codex")).unwrap();
        let observation =
            observe_in_directories("codex", &[bin, temp.path().join("absent")]).unwrap();
        assert_eq!(observation.observed_count, 0);
        assert_eq!(observation.source, None);
        assert_eq!(observation.version, None);
    }

    #[test]
    fn multiple_files_do_not_invent_a_default_installation() {
        let temp = TempDir::new().unwrap();
        let first = package_fixture(&temp.path().join("one"), "codex", "1.2.3");
        let second = package_fixture(&temp.path().join("two"), "codex", "2.3.4");
        let observation =
            observe_in_directories("codex", &[first.clone(), second.clone()]).unwrap();
        assert_eq!(observation.observed_count, 2);
        assert_eq!(observation.version, None);
        assert_eq!(observation.source, None);
        assert!(!first.join("executed-marker").exists());
        assert!(!second.join("executed-marker").exists());
    }

    #[test]
    fn metadata_is_bounded_and_only_matching_release_versions_escape() {
        let temp = TempDir::new().unwrap();
        let bin = package_fixture(temp.path(), "codex", "1.2.3");
        assert_eq!(
            observe_in_directories("codex", std::slice::from_ref(&bin))
                .unwrap()
                .version
                .as_deref(),
            Some("1.2.3")
        );
        for version in [
            "1.2.3+fixture-private-token",
            "1.2.3-secret",
            "/private/fixture-path",
            "01.2.3",
            "$(touch marker)",
        ] {
            package_fixture(temp.path(), "codex", version);
            let observation = observe_in_directories("codex", std::slice::from_ref(&bin)).unwrap();
            assert_eq!(observation.version, None);
            let debug = format!("{observation:?}");
            assert!(!debug.contains("fixture-private"));
            assert!(!debug.contains("/private/"));
        }
        let manifest = bin.parent().unwrap().join("package.json");
        std::fs::write(&manifest, r#"{"name":"@other/package","version":"1.2.3"}"#).unwrap();
        assert_eq!(
            observe_in_directories("codex", std::slice::from_ref(&bin))
                .unwrap()
                .version,
            None
        );
        std::fs::write(&manifest, vec![b' '; MAX_PACKAGE_BYTES as usize + 1]).unwrap();
        assert_eq!(
            observe_in_directories("codex", &[bin]).unwrap().version,
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn canonical_aliases_are_deduplicated_and_mise_dispatchers_are_skipped() {
        use std::os::unix::fs::symlink;
        let temp = TempDir::new().unwrap();
        let bin = package_fixture(&temp.path().join("mise/installs/node/22"), "codex", "1.2.3");
        let alias = temp.path().join("aliases");
        std::fs::create_dir_all(&alias).unwrap();
        symlink(bin.join("codex"), alias.join("codex")).unwrap();
        let observation = observe_in_directories("codex", &[bin.clone(), alias]).unwrap();
        assert_eq!(observation.observed_count, 1);
        assert_eq!(observation.source.as_deref(), Some("mise"));
        assert_eq!(observation.version.as_deref(), Some("1.2.3"));

        let dispatcher_bin = temp.path().join("manager/bin");
        let dispatcher = fake_tool(&dispatcher_bin, "mise");
        let shims = temp.path().join("mise/shims");
        std::fs::create_dir_all(&shims).unwrap();
        symlink(dispatcher, shims.join("codex")).unwrap();
        let observation = observe_in_directories("codex", &[shims, bin]).unwrap();
        assert_eq!(observation.observed_count, 1);
        assert!(!dispatcher_bin.join("executed-marker").exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn standalone_mise_shims_and_symlinked_manifests_are_not_trusted() {
        use std::os::unix::fs::symlink;
        let temp = TempDir::new().unwrap();
        let shims = temp.path().join("mise/shims");
        fake_tool(&shims, "codex");
        assert_eq!(
            observe_in_directories("codex", &[shims])
                .unwrap()
                .observed_count,
            0
        );
        let bin = package_fixture(temp.path(), "codex", "1.2.3");
        let manifest = bin.parent().unwrap().join("package.json");
        let outside = temp.path().join("outside.json");
        std::fs::rename(&manifest, &outside).unwrap();
        symlink(outside, &manifest).unwrap();
        assert_eq!(
            observe_in_directories("codex", &[bin]).unwrap().version,
            None
        );
    }
}
