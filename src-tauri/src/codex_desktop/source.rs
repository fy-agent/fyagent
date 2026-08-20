//! AgentsMirror release metadata validation.
//!
//! The raw mirror schema intentionally stays private to this module.  In
//! particular, URL and delta fields are not represented here: a validated
//! release can only be downloaded through one of the fixed endpoint kinds.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use futures::{future::BoxFuture, StreamExt};
use serde::Deserialize;

use super::{
    cancellation::{cancellation_error, race_with_cancellation, Cancellation},
    download::{BodyStream, TransportError},
    error::{InstallerError, InstallerErrorCode},
    types::{
        CpuArchitecture, DesktopPlatform, PlatformVersion, ReleaseDescriptor,
        TrustedDownloadEndpoint,
    },
};

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_METADATA_ATTEMPTS: u8 = 3;
pub const RELEASE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

/// Caller intent for the process-local, already-validated descriptor cache.
/// `ForceRefresh` is required on the install revalidation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    UseCache,
    ForceRefresh,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use bytes::Bytes;
    use futures::stream;

    use super::*;

    const VALID_MANIFEST: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/codex_desktop/agentsmirror-v5-valid.json"
    ));

    #[test]
    fn manifest_only_resolution_ignores_upstream_content_admission_fields() {
        let mut value: serde_json::Value = serde_json::from_slice(VALID_MANIFEST).unwrap();
        let windows = &mut value["sources"]["windows"]["architectures"]["x64"];
        windows["sha256"] = serde_json::json!("changed");
        windows["packageMoniker"] = serde_json::json!("renamed-without-maintenance");
        windows["minimumOsVersion"] = serde_json::json!("999.0.0.0");
        windows["architecture"] = serde_json::json!("unexpected-publication-value");
        windows["signature"] = serde_json::json!({ "publisher": "changed" });
        windows["contentLength"] = serde_json::json!(7);
        let validated =
            validate_release_metadata(&serde_json::to_vec(&value).unwrap(), RawTarget::WindowsX64)
                .unwrap();
        assert_eq!(validated.download_size_hint, Some(7));
    }

    #[test]
    fn manifest_keeps_flow_fields_and_fixed_endpoint_selection_strict() {
        let validated = validate_release_metadata(VALID_MANIFEST, RawTarget::MacosArm64).unwrap();
        let descriptor = release_descriptor_from_validated(validated).unwrap();
        assert_eq!(
            descriptor.download_endpoint,
            TrustedDownloadEndpoint::MacArm64
        );

        let mut value: serde_json::Value = serde_json::from_slice(VALID_MANIFEST).unwrap();
        value["sources"]["macos"]["arm64"]["downloadable"] = serde_json::json!(false);
        assert_eq!(
            validate_release_metadata(&serde_json::to_vec(&value).unwrap(), RawTarget::MacosArm64,),
            Err(SourceValidationFailure::ReleaseNotAvailable)
        );
    }

    #[test]
    fn manifest_body_limit_remains_bounded() {
        let oversized = vec![b' '; MAX_MANIFEST_BYTES + 1];
        assert_eq!(
            validate_release_metadata(&oversized, RawTarget::WindowsX64),
            Err(SourceValidationFailure::MetadataInvalid(
                "release manifest response is too large"
            ))
        );
    }

    struct FakeFetcher {
        responses: Mutex<VecDeque<Result<Vec<u8>, TransportError>>>,
        calls: AtomicUsize,
    }

    impl FakeFetcher {
        fn new(responses: impl IntoIterator<Item = Result<Vec<u8>, TransportError>>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl MetadataFetcher for FakeFetcher {
        fn fetch<'a>(
            &'a self,
            endpoint: MetadataEndpoint,
        ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>> {
            assert_eq!(endpoint, MetadataEndpoint::Manifest);
            self.calls.fetch_add(1, Ordering::AcqRel);
            let response = self
                .responses
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pop_front()
                .expect("fake metadata response must be queued");
            Box::pin(async move {
                response.map(|body| MetadataResponse {
                    content_length: Some(body.len() as u64),
                    body: Box::pin(stream::once(async move { Ok(Bytes::from(body)) })),
                })
            })
        }
    }

    #[derive(Default)]
    struct NoSleep(AtomicUsize);

    impl MetadataRetrySleeper for NoSleep {
        fn sleep<'a>(&'a self, _duration: Duration) -> BoxFuture<'a, ()> {
            self.0.fetch_add(1, Ordering::AcqRel);
            Box::pin(async {})
        }
    }

    #[tokio::test]
    async fn cache_force_refresh_retry_and_cancellation_keep_the_fixed_manifest_flow() {
        let fetcher = Arc::new(FakeFetcher::new([
            Err(TransportError::retryable("transient fixture")),
            Ok(VALID_MANIFEST.to_vec()),
            Ok(VALID_MANIFEST.to_vec()),
        ]));
        let sleeper = Arc::new(NoSleep::default());
        let source = AgentsMirrorSource::with_dependencies(
            fetcher.clone(),
            Arc::new(SystemReleaseClock),
            sleeper.clone(),
        );
        let active = AtomicBool::new(false);

        source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::UseCache,
                &active,
            )
            .await
            .unwrap();
        source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::UseCache,
                &active,
            )
            .await
            .unwrap();
        assert_eq!(fetcher.calls.load(Ordering::Acquire), 2);
        assert_eq!(sleeper.0.load(Ordering::Acquire), 1);

        source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::ForceRefresh,
                &active,
            )
            .await
            .unwrap();
        assert_eq!(fetcher.calls.load(Ordering::Acquire), 3);

        let cancelled = AtomicBool::new(true);
        let error = source
            .resolve_latest(
                DesktopPlatform::Windows,
                CpuArchitecture::X86_64,
                CacheMode::ForceRefresh,
                &cancelled,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::DownloadCancelled);
        assert_eq!(fetcher.calls.load(Ordering::Acquire), 3);
    }
}

/// Metadata routes that the source may ask its HTTP adapter to retrieve.
/// Artifact endpoints are deliberately unavailable through this trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataEndpoint {
    Manifest,
}

impl MetadataEndpoint {
    const fn trusted_endpoint(self) -> TrustedDownloadEndpoint {
        match self {
            Self::Manifest => TrustedDownloadEndpoint::Manifest,
        }
    }

    /// Fixed URL for an installer metadata document. No caller can provide a
    /// URL, and artifact routes are intentionally absent from this enum.
    pub(crate) const fn url(self) -> &'static str {
        self.trusted_endpoint().url()
    }

    pub(crate) const fn kind(self) -> &'static str {
        self.trusted_endpoint().kind()
    }
}

/// One metadata HTTP response. The source, rather than an adapter, owns the
/// bounded collection policy so an adapter cannot accidentally buffer an
/// unbounded response before the V5 parser sees it.
pub struct MetadataResponse {
    pub content_length: Option<u64>,
    pub body: BodyStream,
}

/// Narrow, object-safe boundary for the fixed release manifest.
/// The body is deliberately streamed so `AgentsMirrorSource` can apply its
/// one-mebibyte cap before allocating more metadata memory.
pub trait MetadataFetcher: Send + Sync {
    fn fetch<'a>(
        &'a self,
        endpoint: MetadataEndpoint,
    ) -> BoxFuture<'a, Result<MetadataResponse, TransportError>>;
}

/// Object-safe source boundary consumed by the installer service.
pub trait ReleaseSource: Send + Sync {
    fn resolve_latest<'a>(
        &'a self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &'a dyn Cancellation,
    ) -> BoxFuture<'a, Result<ReleaseDescriptor, InstallerError>>;
}

/// Clock injection keeps the five-minute cache deterministic in service tests.
pub trait ReleaseClock: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Debug, Default)]
struct SystemReleaseClock;

impl ReleaseClock for SystemReleaseClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A small injectable wait boundary keeps retry tests deterministic without
/// weakening the production backoff or cancellation behavior.
pub(crate) trait MetadataRetrySleeper: Send + Sync {
    fn sleep<'a>(&'a self, duration: Duration) -> BoxFuture<'a, ()>;
}

#[derive(Debug, Default)]
struct TokioMetadataRetrySleeper;

impl MetadataRetrySleeper for TokioMetadataRetrySleeper {
    fn sleep<'a>(&'a self, duration: Duration) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
        })
    }
}

/// The single V1 source reads one bounded fixed-endpoint manifest. It never
/// retains remote artifact URLs or treats upstream publication fields as
/// content-admission evidence.
pub struct AgentsMirrorSource {
    fetcher: Arc<dyn MetadataFetcher>,
    clock: Arc<dyn ReleaseClock>,
    retry_sleeper: Arc<dyn MetadataRetrySleeper>,
    cache: Mutex<ReleaseCache<ReleaseDescriptor>>,
}

impl AgentsMirrorSource {
    pub fn new(fetcher: Arc<dyn MetadataFetcher>) -> Self {
        Self::with_clock(fetcher, Arc::new(SystemReleaseClock))
    }

    pub fn with_clock(fetcher: Arc<dyn MetadataFetcher>, clock: Arc<dyn ReleaseClock>) -> Self {
        Self::with_dependencies(fetcher, clock, Arc::new(TokioMetadataRetrySleeper))
    }

    pub(crate) fn with_dependencies(
        fetcher: Arc<dyn MetadataFetcher>,
        clock: Arc<dyn ReleaseClock>,
        retry_sleeper: Arc<dyn MetadataRetrySleeper>,
    ) -> Self {
        Self {
            fetcher,
            clock,
            retry_sleeper,
            cache: Mutex::new(ReleaseCache::default()),
        }
    }

    async fn resolve_latest_inner(
        &self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &dyn Cancellation,
    ) -> Result<ReleaseDescriptor, InstallerError> {
        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }

        let target = raw_target(platform, architecture)?;
        if cache_mode == CacheMode::UseCache {
            let cached = self
                .cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(target, self.clock.now());
            if let Some(cached) = cached {
                return Ok(cached);
            }
        }

        let manifest = self
            .fetch_metadata(MetadataEndpoint::Manifest, cancellation)
            .await?;
        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }
        let validated =
            validate_release_metadata(&manifest, target).map_err(map_validation_failure)?;
        let descriptor = release_descriptor_from_validated(validated)?;

        if cancellation.is_cancelled() {
            return Err(cancellation_error());
        }

        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(target, descriptor.clone(), self.clock.now());
        Ok(descriptor)
    }

    async fn fetch_metadata(
        &self,
        endpoint: MetadataEndpoint,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>, InstallerError> {
        for attempt in 1..=MAX_METADATA_ATTEMPTS {
            let response = match race_with_cancellation(self.fetcher.fetch(endpoint), cancellation)
                .await
            {
                Ok(Ok(response)) => response,
                Ok(Err(error)) if error.is_retryable() && attempt < MAX_METADATA_ATTEMPTS => {
                    self.wait_for_metadata_retry(endpoint, attempt, cancellation)
                        .await?;
                    continue;
                }
                Ok(Err(error)) => return Err(metadata_transport_error(error, endpoint, attempt)),
                Err(_) => return Err(metadata_cancellation_error(endpoint, attempt)),
            };

            match race_with_cancellation(
                collect_metadata_response(response, metadata_response_limit(endpoint)),
                cancellation,
            )
            .await
            {
                Ok(Ok(bytes)) => return Ok(bytes),
                Ok(Err(MetadataResponseReadError::Transport(error)))
                    if error.is_retryable() && attempt < MAX_METADATA_ATTEMPTS =>
                {
                    self.wait_for_metadata_retry(endpoint, attempt, cancellation)
                        .await?;
                }
                Ok(Err(MetadataResponseReadError::Transport(error))) => {
                    return Err(metadata_transport_error(error, endpoint, attempt));
                }
                Ok(Err(MetadataResponseReadError::TooLarge)) => {
                    return Err(metadata_too_large_error(endpoint, attempt));
                }
                Err(_) => return Err(metadata_cancellation_error(endpoint, attempt)),
            }
        }

        Err(InstallerError::new(InstallerErrorCode::InternalError)
            .with_diagnostic_message("metadata retry loop exited unexpectedly")
            .with_endpoint_kind(endpoint.kind()))
    }

    async fn wait_for_metadata_retry(
        &self,
        endpoint: MetadataEndpoint,
        failed_attempt: u8,
        cancellation: &dyn Cancellation,
    ) -> Result<(), InstallerError> {
        let delay = metadata_retry_delay_after(failed_attempt);
        race_with_cancellation(self.retry_sleeper.sleep(delay), cancellation)
            .await
            .map_err(|_| metadata_cancellation_error(endpoint, failed_attempt))
    }
}

impl ReleaseSource for AgentsMirrorSource {
    fn resolve_latest<'a>(
        &'a self,
        platform: DesktopPlatform,
        architecture: CpuArchitecture,
        cache_mode: CacheMode,
        cancellation: &'a dyn Cancellation,
    ) -> BoxFuture<'a, Result<ReleaseDescriptor, InstallerError>> {
        Box::pin(async move {
            self.resolve_latest_inner(platform, architecture, cache_mode, cancellation)
                .await
        })
    }
}

#[derive(Debug)]
enum MetadataResponseReadError {
    TooLarge,
    Transport(TransportError),
}

async fn collect_metadata_response(
    mut response: MetadataResponse,
    maximum_bytes: usize,
) -> Result<Vec<u8>, MetadataResponseReadError> {
    if response
        .content_length
        .is_some_and(|content_length| content_length > maximum_bytes as u64)
    {
        return Err(MetadataResponseReadError::TooLarge);
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response.body.next().await {
        let chunk = chunk.map_err(MetadataResponseReadError::Transport)?;
        let collected_length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or(MetadataResponseReadError::TooLarge)?;
        if collected_length > maximum_bytes {
            return Err(MetadataResponseReadError::TooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(bytes)
}

const fn metadata_response_limit(endpoint: MetadataEndpoint) -> usize {
    match endpoint {
        MetadataEndpoint::Manifest => MAX_MANIFEST_BYTES,
    }
}

fn metadata_retry_delay_after(failed_attempt: u8) -> Duration {
    match failed_attempt {
        1 => Duration::from_secs(1),
        _ => Duration::from_secs(3),
    }
}

fn metadata_transport_error(
    error: TransportError,
    endpoint: MetadataEndpoint,
    attempt: u8,
) -> InstallerError {
    let code = if error.is_redirect_rejected() {
        InstallerErrorCode::RedirectRejected
    } else {
        InstallerErrorCode::SourceUnavailable
    };

    InstallerError::new(code)
        .with_diagnostic_message(error.diagnostic())
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

fn metadata_too_large_error(endpoint: MetadataEndpoint, attempt: u8) -> InstallerError {
    InstallerError::new(InstallerErrorCode::ReleaseMetadataInvalid)
        .with_diagnostic_message("metadata response exceeded the one-mebibyte limit")
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

fn metadata_cancellation_error(endpoint: MetadataEndpoint, attempt: u8) -> InstallerError {
    cancellation_error()
        .with_endpoint_kind(endpoint.kind())
        .with_attempt(attempt, MAX_METADATA_ATTEMPTS)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RawTarget {
    WindowsX64,
    WindowsArm64,
    MacosArm64,
}

#[derive(Debug, Clone)]
struct CachedRelease<T> {
    release: T,
    resolved_at: Instant,
}

#[derive(Debug)]
struct ReleaseCache<T> {
    entries: HashMap<RawTarget, CachedRelease<T>>,
}

impl<T> Default for ReleaseCache<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T: Clone> ReleaseCache<T> {
    fn get(&self, target: RawTarget, now: Instant) -> Option<T> {
        let cached = self.entries.get(&target)?;
        let age = now.checked_duration_since(cached.resolved_at)?;
        (age < RELEASE_CACHE_TTL).then(|| cached.release.clone())
    }

    fn insert(&mut self, target: RawTarget, release: T, now: Instant) {
        self.entries.insert(
            target,
            CachedRelease {
                release,
                resolved_at: now,
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValidatedPlatformVersion {
    WindowsMsix(String),
    MacBundle(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedRelease {
    target: RawTarget,
    display_version: String,
    platform_version: ValidatedPlatformVersion,
    download_size_hint: Option<u64>,
}

/// Deliberately contains only controlled descriptions.  Remote field values,
/// including arbitrary URLs, must not become diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceValidationFailure {
    MetadataInvalid(&'static str),
    ReleaseNotAvailable,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawReleaseManifest {
    schema_version: u64,
    sources: RawSources,
}

#[derive(Debug, Deserialize)]
struct RawSources {
    windows: Option<RawWindowsSource>,
    macos: Option<RawMacosSource>,
}

#[derive(Debug, Deserialize)]
struct RawWindowsSource {
    architectures: RawWindowsArchitectures,
}

#[derive(Debug, Deserialize)]
struct RawWindowsArchitectures {
    x64: Option<RawWindowsArtifact>,
    arm64: Option<RawWindowsArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWindowsArtifact {
    status: Option<String>,
    downloadable: Option<bool>,
    version: Option<String>,
    content_length: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RawMacosSource {
    arm64: Option<RawMacosArtifact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMacosArtifact {
    content_length: Option<u64>,
    bundle_short_version: Option<String>,
    bundle_version: Option<String>,
    downloadable: Option<bool>,
    status: Option<String>,
}

fn validate_release_metadata(
    manifest_bytes: &[u8],
    target: RawTarget,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let manifest = parse_manifest(manifest_bytes)?;
    match target {
        RawTarget::WindowsX64 => validate_windows_release(&manifest, "x64"),
        RawTarget::WindowsArm64 => validate_windows_release(&manifest, "arm64"),
        RawTarget::MacosArm64 => validate_macos_release(&manifest),
    }
}

fn parse_manifest(bytes: &[u8]) -> Result<RawReleaseManifest, SourceValidationFailure> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release manifest response is too large",
        ));
    }

    let manifest = serde_json::from_slice::<RawReleaseManifest>(bytes).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("release manifest is not valid schema-v5 JSON")
    })?;

    if manifest.schema_version != 5 {
        return Err(SourceValidationFailure::MetadataInvalid(
            "release manifest schema version is unsupported",
        ));
    }

    Ok(manifest)
}

fn validate_windows_release(
    manifest: &RawReleaseManifest,
    expected_architecture: &str,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let architectures = &manifest
        .sources
        .windows
        .as_ref()
        .ok_or(SourceValidationFailure::ReleaseNotAvailable)?
        .architectures;
    let artifact = match expected_architecture {
        "x64" => architectures.x64.as_ref(),
        "arm64" => architectures.arm64.as_ref(),
        _ => None,
    }
    .ok_or(SourceValidationFailure::ReleaseNotAvailable)?;

    validate_downloadable(artifact.downloadable, artifact.status.as_deref())?;
    let version = require_non_empty_option(artifact.version.as_deref(), "Windows version")?;
    let platform_version = validate_windows_version(version)?;
    Ok(ValidatedRelease {
        target: if expected_architecture == "x64" {
            RawTarget::WindowsX64
        } else {
            RawTarget::WindowsArm64
        },
        // The manifest-wide codexVersion may describe a different platform's
        // release. The Windows card displays the selected architecture's
        // release version without treating it as downloaded-content evidence.
        display_version: version.to_owned(),
        platform_version,
        download_size_hint: artifact.content_length.filter(|size| *size > 0),
    })
}

fn validate_macos_release(
    manifest: &RawReleaseManifest,
) -> Result<ValidatedRelease, SourceValidationFailure> {
    let artifact = manifest
        .sources
        .macos
        .as_ref()
        .and_then(|source| source.arm64.as_ref())
        .ok_or(SourceValidationFailure::ReleaseNotAvailable)?;

    validate_downloadable(artifact.downloadable, artifact.status.as_deref())?;
    let display_version = require_non_empty_option(
        artifact.bundle_short_version.as_deref(),
        "macOS bundleShortVersion",
    )?;
    let bundle_version =
        require_non_empty_option(artifact.bundle_version.as_deref(), "macOS bundleVersion")?;
    Ok(ValidatedRelease {
        target: RawTarget::MacosArm64,
        display_version: display_version.to_owned(),
        platform_version: validate_mac_bundle_version(bundle_version)?,
        download_size_hint: artifact.content_length.filter(|size| *size > 0),
    })
}

fn validate_downloadable(
    downloadable: Option<bool>,
    status: Option<&str>,
) -> Result<(), SourceValidationFailure> {
    match (downloadable, status) {
        (Some(true), Some("downloadable")) => Ok(()),
        (Some(false), Some(_)) => Err(SourceValidationFailure::ReleaseNotAvailable),
        (Some(true), Some(_)) => Err(SourceValidationFailure::MetadataInvalid(
            "release status conflicts with downloadable flag",
        )),
        _ => Err(SourceValidationFailure::MetadataInvalid(
            "release is missing downloadable status fields",
        )),
    }
}

fn validate_windows_version(
    value: &str,
) -> Result<ValidatedPlatformVersion, SourceValidationFailure> {
    PlatformVersion::parse_windows_msix(value).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("Windows version is not a valid MSIX version")
    })?;
    Ok(ValidatedPlatformVersion::WindowsMsix(value.to_owned()))
}

fn validate_mac_bundle_version(
    value: &str,
) -> Result<ValidatedPlatformVersion, SourceValidationFailure> {
    PlatformVersion::parse_mac_bundle(value.to_owned()).map_err(|_| {
        SourceValidationFailure::MetadataInvalid("macOS bundleVersion is not comparable")
    })?;
    Ok(ValidatedPlatformVersion::MacBundle(value.to_owned()))
}

fn require_non_empty(value: &str, name: &'static str) -> Result<(), SourceValidationFailure> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(SourceValidationFailure::MetadataInvalid(name));
    }
    Ok(())
}

fn require_non_empty_option<'a>(
    value: Option<&'a str>,
    name: &'static str,
) -> Result<&'a str, SourceValidationFailure> {
    let value = value.ok_or(SourceValidationFailure::MetadataInvalid(name))?;
    require_non_empty(value, name)?;
    Ok(value)
}

fn raw_target(
    platform: DesktopPlatform,
    architecture: CpuArchitecture,
) -> Result<RawTarget, InstallerError> {
    match (platform, architecture) {
        (DesktopPlatform::Windows, CpuArchitecture::X86_64) => Ok(RawTarget::WindowsX64),
        (DesktopPlatform::Windows, CpuArchitecture::Aarch64) => Ok(RawTarget::WindowsArm64),
        (DesktopPlatform::Macos, CpuArchitecture::Aarch64) => Ok(RawTarget::MacosArm64),
        _ => Err(
            InstallerError::new(InstallerErrorCode::ArchitectureUnsupported)
                .with_context("architecture", architecture.as_str())
                .with_diagnostic_message(
                    "release source has no package for this platform architecture",
                ),
        ),
    }
}

fn release_descriptor_from_validated(
    validated: ValidatedRelease,
) -> Result<ReleaseDescriptor, InstallerError> {
    let (platform, architecture, endpoint) = match validated.target {
        RawTarget::WindowsX64 => (
            DesktopPlatform::Windows,
            CpuArchitecture::X86_64,
            TrustedDownloadEndpoint::WinX64,
        ),
        RawTarget::WindowsArm64 => (
            DesktopPlatform::Windows,
            CpuArchitecture::Aarch64,
            TrustedDownloadEndpoint::WinArm64,
        ),
        RawTarget::MacosArm64 => (
            DesktopPlatform::Macos,
            CpuArchitecture::Aarch64,
            TrustedDownloadEndpoint::MacArm64,
        ),
    };
    let platform_version = match validated.platform_version {
        ValidatedPlatformVersion::WindowsMsix(value) => {
            PlatformVersion::parse_windows_msix(&value)?
        }
        ValidatedPlatformVersion::MacBundle(value) => PlatformVersion::parse_mac_bundle(value)?,
    };

    ReleaseDescriptor::new(
        platform,
        architecture,
        validated.display_version,
        platform_version,
        validated.download_size_hint,
        endpoint,
    )
}

fn map_validation_failure(failure: SourceValidationFailure) -> InstallerError {
    let (code, endpoint_kind, message) = match failure {
        SourceValidationFailure::MetadataInvalid(message) => (
            InstallerErrorCode::ReleaseMetadataInvalid,
            MetadataEndpoint::Manifest.kind(),
            message,
        ),
        SourceValidationFailure::ReleaseNotAvailable => (
            InstallerErrorCode::ReleaseNotAvailable,
            MetadataEndpoint::Manifest.kind(),
            "release is not downloadable for this platform architecture",
        ),
    };

    InstallerError::new(code)
        .with_endpoint_kind(endpoint_kind)
        .with_diagnostic_message(message)
}
