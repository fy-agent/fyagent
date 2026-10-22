//! Portable, closed OMP discovery inputs and bounded observation values.
use std::path::{Path, PathBuf};

pub const MAX_OMP_VERSION_BYTES: usize = 64;
pub const MAX_OMP_PATH_BYTES: usize = 1024;
pub const OMP_PROBE_TIMEOUT_SECS: u64 = 8;
pub const MAX_OMP_CANDIDATES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OmpProbeResult {
    pub detected: bool,
    pub version: Option<String>,
    pub executable_path: Option<String>,
}

impl OmpProbeResult {
    pub fn not_detected() -> Self {
        Self {
            detected: false,
            version: None,
            executable_path: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        match (&self.version, &self.executable_path) {
            (None, None) => !self.detected,
            (Some(version), Some(path)) => {
                self.detected
                    && parse_omp_version_output(version).as_deref() == Some(version.as_str())
                    && !path.is_empty()
                    && path.len() <= MAX_OMP_PATH_BYTES
                    && !path.chars().any(char::is_control)
                    && (Path::new(path).is_absolute() || is_absolute_windows_path(path))
            }
            _ => false,
        }
    }
}

fn is_absolute_windows_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

/// Only a complete version line is accepted; unrelated logs never enter a DTO.
pub fn parse_omp_version_output(output: &str) -> Option<String> {
    let line = output.trim();
    let value = line
        .strip_prefix("omp/")
        .or_else(|| line.strip_prefix("omp "))
        .unwrap_or(line);
    let value = value.strip_prefix('v').unwrap_or(value);
    if value.is_empty()
        || value.len() > MAX_OMP_VERSION_BYTES
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
    {
        return None;
    }
    let (without_build, build) = value
        .split_once('+')
        .map_or((value, None), |(a, b)| (a, Some(b)));
    let (core, pre) = without_build
        .split_once('-')
        .map_or((without_build, None), |(a, b)| (a, Some(b)));
    let numbers: Vec<_> = core.split('.').collect();
    if numbers.len() != 3
        || numbers.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|b| b.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return None;
    }
    for (suffix, prerelease) in [(pre, true), (build, false)] {
        if let Some(suffix) = suffix {
            if suffix.split('.').any(|part| {
                part.is_empty()
                    || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    || (prerelease
                        && part.len() > 1
                        && part.starts_with('0')
                        && part.bytes().all(|b| b.is_ascii_digit()))
            }) {
                return None;
            }
        }
    }
    Some(value.to_owned())
}

/// Pure ordering shared with macOS. Callers provide their own user's folders.
pub fn omp_candidates(
    local_app_data: Option<&Path>,
    home: Option<&Path>,
    path_directories: impl IntoIterator<Item = PathBuf>,
    windows: bool,
) -> Vec<PathBuf> {
    let names: &[&str] = if windows {
        &["omp.exe", "omp.cmd"]
    } else {
        &["omp"]
    };
    let mut candidates = Vec::new();
    if windows {
        if let Some(local) = local_app_data.filter(|p| p.is_absolute()) {
            candidates.push(local.join("omp").join("omp.exe"));
        }
    }
    if let Some(home) = home.filter(|p| p.is_absolute()) {
        candidates.extend(
            names
                .iter()
                .map(|name| home.join(".bun").join("bin").join(name)),
        );
    }
    for directory in path_directories
        .into_iter()
        .filter(|p| p.is_absolute())
        .take(MAX_OMP_CANDIDATES)
    {
        candidates.extend(names.iter().map(|name| directory.join(name)));
    }
    let mut seen = std::collections::HashSet::new();
    candidates
        .into_iter()
        .filter(|path| {
            let text = path.to_string_lossy();
            seen.insert(if windows {
                text.to_lowercase()
            } else {
                text.into_owned()
            })
        })
        .take(MAX_OMP_CANDIDATES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_accept_official_output_and_reject_log_or_secret_text() {
        assert_eq!(
            parse_omp_version_output("omp/18.12.5\r\n"),
            Some("18.12.5".into())
        );
        assert_eq!(
            parse_omp_version_output("omp v18.12.5-rc.1+build"),
            Some("18.12.5-rc.1+build".into())
        );
        for value in [
            "error: 18.12.5",
            "18.12.5 token=secret",
            "18.12.5\nsecret",
            "18.12",
            "018.12.5",
            "18.12.5-01",
            "18.12.5+",
            "18.12.5\x1b[0m",
        ] {
            assert!(parse_omp_version_output(value).is_none(), "{value:?}");
        }
    }
    #[test]
    fn observation_requires_a_version_and_absolute_bounded_path() {
        assert!(OmpProbeResult::not_detected().is_valid());
        let mut result = OmpProbeResult {
            detected: true,
            version: Some("18.12.5".into()),
            executable_path: Some("C:\\Users\\example\\omp.exe".into()),
        };
        assert!(result.is_valid());
        result.executable_path = Some("omp.exe".into());
        assert!(!result.is_valid());
        result.executable_path = Some(format!("C:\\{}", "x".repeat(MAX_OMP_PATH_BYTES)));
        assert!(!result.is_valid());
        result.version = None;
        assert!(!result.is_valid());
    }
    #[test]
    fn candidates_preserve_precedence_dedupe_and_ignore_relative_path_entries() {
        #[cfg(target_os = "windows")]
        let root = PathBuf::from("C:\\Users\\example");
        #[cfg(target_os = "macos")]
        let root = PathBuf::from("/Users/example");
        let local = root.join("AppData").join("Local");
        let bun = root.join(".bun").join("bin");
        let candidates = omp_candidates(
            Some(&local),
            Some(&root),
            [bun.clone(), PathBuf::from("relative")],
            true,
        );
        assert_eq!(
            candidates,
            vec![
                local.join("omp").join("omp.exe"),
                bun.join("omp.exe"),
                bun.join("omp.cmd")
            ]
        );
        let paths = omp_candidates(None, None, (0..100).map(|n| root.join(n.to_string())), true);
        assert_eq!(paths.len(), MAX_OMP_CANDIDATES);
    }
}
