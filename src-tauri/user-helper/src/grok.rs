//! Shared Grok Build owner, version, and Windows candidate rules.
//!
//! The host crate and the unelevated helper both consume this module so PATH
//! markers and version parsing cannot drift. It is pure: no process launch,
//! no filesystem I/O, and no command/URL fields.

pub const GROK_NPM_PACKAGE: &str = "@xai-official/grok";
pub const GROK_NPM_INSTALL_SPEC: &str = "@xai-official/grok@latest";
pub const GROK_NATIVE_WINDOWS_INSTALL_SCRIPT: &str = "irm https://x.ai/cli/install.ps1 | iex";
pub const MAX_NORMALIZED_VERSION_BYTES: usize = 32;
pub const GROK_OUTPUT_LIMIT: usize = 32 * 1024;
pub const GROK_VERSION_TIMEOUT_SECS: u64 = 20;
pub const GROK_LIFECYCLE_TIMEOUT_SECS: u64 = 300;
pub const TOOL_OPERATION_STARTED_IDENTITY: crate::protocol::PinnedPackageIdentity =
    crate::protocol::PinnedPackageIdentity::new(0, 0, 1);

const VERSION_CORE_PARTS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrokOwner {
    Native,
    Npm,
}

impl GrokOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native_internal",
            Self::Npm => "official_npm",
        }
    }

    pub const fn wire(self) -> u8 {
        match self {
            Self::Native => 1,
            Self::Npm => 2,
        }
    }

    pub const fn from_wire(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Native),
            2 => Some(Self::Npm),
            _ => None,
        }
    }

    pub fn parse_cli(value: &str) -> Option<Self> {
        match value {
            "native" => Some(Self::Native),
            "npm" => Some(Self::Npm),
            _ => None,
        }
    }

    pub const fn cli_token(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Npm => "npm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokToolAction {
    Observe,
    Install,
    Update,
}

impl GrokToolAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Install => "install",
            Self::Update => "update",
        }
    }

    pub fn parse_cli(value: &str) -> Option<Self> {
        match value {
            "observe" => Some(Self::Observe),
            "install" => Some(Self::Install),
            "update" => Some(Self::Update),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokOwnerObservation {
    Native,
    Npm,
    Ambiguous,
    Absent,
}

impl GrokOwnerObservation {
    pub const fn owner(self) -> Option<GrokOwner> {
        match self {
            Self::Native => Some(GrokOwner::Native),
            Self::Npm => Some(GrokOwner::Npm),
            Self::Ambiguous | Self::Absent => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokOutcome {
    Observed,
    Installed,
    Updated,
    NoChange,
}

impl GrokOutcome {
    pub const fn wire(self) -> u8 {
        match self {
            Self::Observed => 0,
            Self::Installed => 1,
            Self::Updated => 2,
            Self::NoChange => 3,
        }
    }

    pub const fn from_wire(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Observed),
            1 => Some(Self::Installed),
            2 => Some(Self::Updated),
            3 => Some(Self::NoChange),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolOperationResult {
    pub detected: bool,
    pub normalized_version: Option<String>,
    pub owner: Option<GrokOwner>,
    pub outcome: GrokOutcome,
}

impl ToolOperationResult {
    pub fn observed(
        detected: bool,
        owner: Option<GrokOwner>,
        normalized_version: Option<String>,
    ) -> Self {
        Self {
            detected,
            normalized_version,
            owner,
            outcome: GrokOutcome::Observed,
        }
    }
}

pub fn is_native_install_path(bin_path: &str, real_target: &str) -> bool {
    [bin_path, real_target].iter().any(|path| {
        let normalized = path.replace('\\', "/").to_ascii_lowercase();
        normalized.contains("/.grok/bin/") || normalized.contains("/.grok/downloads/grok-")
    })
}

pub fn owner_from_install_paths(
    bin_path: &str,
    real_target: &str,
    install_source: &str,
    config_owner: Option<GrokOwner>,
) -> GrokOwner {
    if is_native_install_path(bin_path, real_target) {
        return config_owner.unwrap_or(GrokOwner::Native);
    }
    match install_source {
        "nvm" | "fnm" | "volta" | "mise" | "bun" | "pnpm" => GrokOwner::Npm,
        _ => {
            let combined = format!("{bin_path}\n{real_target}").replace('\\', "/");
            if combined.contains("/node_modules/") || combined.contains("/.nvm/") {
                GrokOwner::Npm
            } else {
                config_owner.unwrap_or(GrokOwner::Npm)
            }
        }
    }
}

pub fn observe_owner_from_candidates<I>(owners: I) -> GrokOwnerObservation
where
    I: IntoIterator<Item = GrokOwner>,
{
    let unique: std::collections::BTreeSet<GrokOwner> = owners.into_iter().collect();
    match unique.len() {
        0 => GrokOwnerObservation::Absent,
        1 => match unique.iter().next().copied() {
            Some(GrokOwner::Native) => GrokOwnerObservation::Native,
            Some(GrokOwner::Npm) => GrokOwnerObservation::Npm,
            None => GrokOwnerObservation::Absent,
        },
        _ => GrokOwnerObservation::Ambiguous,
    }
}

pub fn owner_from_installer_value(value: &str) -> Option<GrokOwner> {
    match value.trim() {
        "internal" | "gh-release" => Some(GrokOwner::Native),
        "npm" => Some(GrokOwner::Npm),
        _ => None,
    }
}

/// Bounded parser for `cli.installer` without taking a TOML dependency.
pub fn parse_cli_installer_hint(config_text: &str) -> Option<GrokOwner> {
    let mut in_cli = false;
    for raw in config_text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_cli = line == "[cli]";
            continue;
        }
        if !in_cli {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "installer" {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        return owner_from_installer_value(value);
    }
    None
}

pub fn parse_normalized_version(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if let Some((version, consumed)) = match_version_at(&raw[index..]) {
            if version.len() <= MAX_NORMALIZED_VERSION_BYTES {
                return Some(version);
            }
            index += consumed;
            continue;
        }
        index += 1;
    }
    None
}

fn match_version_at(input: &str) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    if bytes.first().is_none_or(|byte| !byte.is_ascii_digit()) {
        return None;
    }
    let mut index = 0;
    let mut parts = 0;
    while parts < VERSION_CORE_PARTS {
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return None;
        }
        parts += 1;
        if parts < VERSION_CORE_PARTS {
            if bytes.get(index).copied() != Some(b'.') {
                return None;
            }
            index += 1;
        }
    }
    if bytes.get(index).copied() == Some(b'-') {
        index += 1;
        let pre_start = index;
        while index < bytes.len() {
            let byte = bytes[index];
            if byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'_' {
                index += 1;
            } else {
                break;
            }
        }
        if index == pre_start {
            index = pre_start - 1;
        }
    }
    Some((input[..index].to_owned(), index))
}

pub fn powershell_encoded_command(script: &str) -> String {
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    encode_standard_base64(&bytes)
}

fn encode_standard_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let a = u32::from(chunk[0]);
        let b = u32::from(chunk.get(1).copied().unwrap_or(0));
        let c = u32::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (a << 16) | (b << 8) | c;
        encoded.push(char::from(TABLE[((triple >> 18) & 0x3f) as usize]));
        encoded.push(char::from(TABLE[((triple >> 12) & 0x3f) as usize]));
        encoded.push(if chunk.len() > 1 {
            char::from(TABLE[((triple >> 6) & 0x3f) as usize])
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            char::from(TABLE[(triple & 0x3f) as usize])
        } else {
            '='
        });
    }
    encoded
}

pub fn grok_native_windows_powershell_command() -> String {
    format!(
        "powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}",
        powershell_encoded_command(GROK_NATIVE_WINDOWS_INSTALL_SCRIPT)
    )
}

/// Closed relative segments under the interactive user's profile.
pub const GROK_PROFILE_BIN_SEGMENTS: &[&[&str]] = &[
    &[".grok", "bin"],
    &[".local", "bin"],
    &[".npm-global", "bin"],
    &[".volta", "bin"],
    &["n", "bin"],
];

/// Closed relative segments under LocalAppData.
pub const GROK_LOCAL_APP_DATA_BIN_SEGMENTS: &[&[&str]] = &[&["Volta", "bin"], &["pnpm"], &["npm"]];

/// Closed relative segments under RoamingAppData.
pub const GROK_ROAMING_APP_DATA_BIN_SEGMENTS: &[&[&str]] = &[&["npm"]];

pub fn grok_windows_executable_names() -> &'static [&'static str] {
    &["grok.cmd", "grok.exe", "grok"]
}

pub fn infer_source_marker(path: &str) -> &'static str {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.contains("/volta/") {
        "volta"
    } else if normalized.contains("/pnpm/") {
        "pnpm"
    } else if normalized.contains("/.nvm/") || normalized.contains("/nvm/") {
        "nvm"
    } else if normalized.contains("/fnm/") {
        "fnm"
    } else if normalized.contains("/.mise/") {
        "mise"
    } else if normalized.contains("/bun/") {
        "bun"
    } else {
        "system"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokPlanKind {
    Observe,
    NativeFresh,
    NativeUpdate,
    OfficialNpm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrokPlanFailure {
    OwnerMismatch,
    NotDetected,
}

pub fn plan_grok_operation(
    action: GrokToolAction,
    observation: GrokOwnerObservation,
    expected_owner: Option<GrokOwner>,
) -> Result<GrokPlanKind, GrokPlanFailure> {
    match action {
        GrokToolAction::Observe => Ok(GrokPlanKind::Observe),
        GrokToolAction::Install => plan_install(observation, expected_owner),
        GrokToolAction::Update => plan_update(observation, expected_owner),
    }
}

fn plan_install(
    observation: GrokOwnerObservation,
    expected_owner: Option<GrokOwner>,
) -> Result<GrokPlanKind, GrokPlanFailure> {
    if observation == GrokOwnerObservation::Ambiguous {
        return Err(GrokPlanFailure::OwnerMismatch);
    }
    match expected_owner {
        Some(GrokOwner::Npm) => Ok(GrokPlanKind::OfficialNpm),
        Some(GrokOwner::Native) => match observation {
            GrokOwnerObservation::Npm => Err(GrokPlanFailure::OwnerMismatch),
            GrokOwnerObservation::Native
            | GrokOwnerObservation::Absent
            | GrokOwnerObservation::Ambiguous => Ok(GrokPlanKind::NativeFresh),
        },
        None => match observation {
            GrokOwnerObservation::Npm => Err(GrokPlanFailure::OwnerMismatch),
            GrokOwnerObservation::Native
            | GrokOwnerObservation::Absent
            | GrokOwnerObservation::Ambiguous => Ok(GrokPlanKind::NativeFresh),
        },
    }
}

fn plan_update(
    observation: GrokOwnerObservation,
    expected_owner: Option<GrokOwner>,
) -> Result<GrokPlanKind, GrokPlanFailure> {
    match observation {
        GrokOwnerObservation::Absent => Err(GrokPlanFailure::NotDetected),
        GrokOwnerObservation::Ambiguous => Err(GrokPlanFailure::OwnerMismatch),
        GrokOwnerObservation::Native => {
            if expected_owner == Some(GrokOwner::Npm) {
                Err(GrokPlanFailure::OwnerMismatch)
            } else {
                Ok(GrokPlanKind::NativeUpdate)
            }
        }
        GrokOwnerObservation::Npm => {
            if expected_owner == Some(GrokOwner::Native) {
                Err(GrokPlanFailure::OwnerMismatch)
            } else {
                Ok(GrokPlanKind::OfficialNpm)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_path_markers_are_slash_normalized() {
        assert!(is_native_install_path(
            r"C:\Users\a\.grok\bin\grok.exe",
            r"C:\Users\a\.grok\bin\grok.exe"
        ));
        assert!(is_native_install_path(
            "/tmp/launcher",
            "/Users/a/.grok/downloads/grok-windows-x64"
        ));
        assert!(!is_native_install_path(
            r"C:\Users\a\AppData\Roaming\npm\grok.cmd",
            r"C:\Users\a\AppData\Roaming\npm\node_modules\@xai-official\grok\grok.exe"
        ));
    }

    #[test]
    fn owner_classification_prefers_native_markers_over_npm_siblings() {
        assert_eq!(
            owner_from_install_paths(
                r"C:\Users\a\.grok\bin\grok.exe",
                r"C:\Users\a\.grok\bin\grok.exe",
                "system",
                None
            ),
            GrokOwner::Native
        );
        assert_eq!(
            owner_from_install_paths(
                r"C:\Users\a\AppData\Local\Volta\bin\grok.cmd",
                r"C:\Users\a\AppData\Local\Volta\bin\grok.cmd",
                "volta",
                None
            ),
            GrokOwner::Npm
        );
        assert_eq!(
            owner_from_install_paths(
                "/opt/homebrew/bin/grok",
                "/opt/homebrew/lib/node_modules/@xai-official/grok/bin/grok",
                "system",
                None
            ),
            GrokOwner::Npm
        );
    }

    #[test]
    fn installer_hint_parser_reads_only_the_cli_table() {
        assert_eq!(
            parse_cli_installer_hint("[cli]\ninstaller = \"internal\"\n"),
            Some(GrokOwner::Native)
        );
        assert_eq!(
            parse_cli_installer_hint(
                "[other]\ninstaller = \"npm\"\n[cli]\ninstaller = \"gh-release\"\n"
            ),
            Some(GrokOwner::Native)
        );
        assert_eq!(
            parse_cli_installer_hint("[cli]\ninstaller = \"npm\"\n"),
            Some(GrokOwner::Npm)
        );
        assert_eq!(
            parse_cli_installer_hint("[cli]\ninstaller = \"other\"\n"),
            None
        );
    }

    #[test]
    fn version_parser_accepts_semver_and_rejects_unbounded_text() {
        assert_eq!(
            parse_normalized_version("grok 1.2.3").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            parse_normalized_version("v2.3.4-beta.1").as_deref(),
            Some("2.3.4-beta.1")
        );
        assert_eq!(parse_normalized_version("no version here"), None);
        assert_eq!(
            parse_normalized_version(&format!("9.9.9-{}", "a".repeat(40))),
            None
        );
    }

    #[test]
    fn owner_observation_is_closed() {
        assert_eq!(
            observe_owner_from_candidates([]),
            GrokOwnerObservation::Absent
        );
        assert_eq!(
            observe_owner_from_candidates([GrokOwner::Native, GrokOwner::Native]),
            GrokOwnerObservation::Native
        );
        assert_eq!(
            observe_owner_from_candidates([GrokOwner::Native, GrokOwner::Npm]),
            GrokOwnerObservation::Ambiguous
        );
    }

    #[test]
    fn plan_preserves_observed_owner_and_rejects_silent_native_to_npm_install() {
        assert_eq!(
            plan_grok_operation(GrokToolAction::Install, GrokOwnerObservation::Absent, None),
            Ok(GrokPlanKind::NativeFresh)
        );
        assert_eq!(
            plan_grok_operation(GrokToolAction::Install, GrokOwnerObservation::Npm, None),
            Err(GrokPlanFailure::OwnerMismatch)
        );
        assert_eq!(
            plan_grok_operation(
                GrokToolAction::Install,
                GrokOwnerObservation::Native,
                Some(GrokOwner::Npm)
            ),
            Ok(GrokPlanKind::OfficialNpm)
        );
        assert_eq!(
            plan_grok_operation(GrokToolAction::Update, GrokOwnerObservation::Native, None),
            Ok(GrokPlanKind::NativeUpdate)
        );
        assert_eq!(
            plan_grok_operation(GrokToolAction::Update, GrokOwnerObservation::Npm, None),
            Ok(GrokPlanKind::OfficialNpm)
        );
        assert_eq!(
            plan_grok_operation(
                GrokToolAction::Update,
                GrokOwnerObservation::Native,
                Some(GrokOwner::Npm)
            ),
            Err(GrokPlanFailure::OwnerMismatch)
        );
        assert_eq!(
            plan_grok_operation(GrokToolAction::Update, GrokOwnerObservation::Absent, None),
            Err(GrokPlanFailure::NotDetected)
        );
        assert_eq!(
            plan_grok_operation(GrokToolAction::Observe, GrokOwnerObservation::Absent, None),
            Ok(GrokPlanKind::Observe)
        );
    }

    #[test]
    fn powershell_encoder_uses_utf16le_standard_base64() {
        let encoded = powershell_encoded_command("a");
        assert_eq!(encoded, "YQA=");
        assert!(grok_native_windows_powershell_command()
            .starts_with("powershell -NoProfile -ExecutionPolicy Bypass -EncodedCommand "));
    }
}
