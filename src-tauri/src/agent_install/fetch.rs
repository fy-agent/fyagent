//! Allowlisted HTTPS GET for Agent desktop metadata and artifacts.
//! Redirect targets never become IPC locators; every hop is rechecked.
//! Artifact bodies stream through the Codex transport persist owner into a
//! job-local `.part` file; metadata still uses a bounded in-memory body.

use futures::StreamExt;
use url::Url;

use super::sources::{
    https_url_on_allowlist, AgentPlatform, PackageFormat, ResolvedDesktopSource,
    SourceResolveError, MAX_SOURCE_METADATA_BYTES, OPENCODE_DOWNLOAD_HOSTS,
    QODERWORK_REDIRECT_HOSTS, TRAEWORK_DOWNLOAD_HOSTS, WORKBUDDY_DOWNLOAD_HOSTS,
};
use super::types::AgentReasonCode;
use crate::codex_desktop::{
    cancellation::{race_with_cancellation, Cancellation, NeverCancelled},
    download::{
        persist_transport_response, prepare_transport_download, resolve_redirect,
        DownloadProgressSink, DownloadedArtifact, HttpTransport, PersistDownloadError,
        TransportResponse, MAX_REDIRECTS,
    },
    runtime::{InstallerTransportPurpose, RuntimeInstallerTransport},
    temp::JobTempDir,
    verify::ArtifactKind,
};
use crate::services::external_agents::AgentCatalogId;

const USER_AGENT: &str = "fyagent-agent-installer";
const MAX_STREAMED_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub async fn fetch_metadata_bytes(url: Url, hosts: &[&str]) -> Result<Vec<u8>, SourceResolveError> {
    https_url_on_allowlist(&url, hosts)?;
    let transport = RuntimeInstallerTransport::new(InstallerTransportPurpose::Metadata, USER_AGENT);
    let response = get_allowlisted(&transport, url, hosts, &NeverCancelled).await?;
    collect_body(response, MAX_SOURCE_METADATA_BYTES, &NeverCancelled).await
}

pub(super) fn artifact_download_hosts(
    product: AgentCatalogId,
) -> Result<&'static [&'static str], AgentReasonCode> {
    match product {
        AgentCatalogId::QoderWork => Ok(QODERWORK_REDIRECT_HOSTS),
        AgentCatalogId::TraeWork => Ok(TRAEWORK_DOWNLOAD_HOSTS),
        AgentCatalogId::WorkBuddy => Ok(WORKBUDDY_DOWNLOAD_HOSTS),
        AgentCatalogId::OpenCode => Ok(OPENCODE_DOWNLOAD_HOSTS),
        _ => Err(AgentReasonCode::ExecutorNotImplemented),
    }
}

pub async fn download_macos_dmg_to_job(
    source: &ResolvedDesktopSource,
    job_directory: &JobTempDir,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
) -> Result<DownloadedArtifact, AgentReasonCode> {
    if cancellation.is_cancelled() {
        return Err(AgentReasonCode::Cancelled);
    }
    if source.format != PackageFormat::Dmg || source.platform != AgentPlatform::Macos {
        return Err(AgentReasonCode::PlatformUnsupported);
    }
    let hosts = artifact_download_hosts(source.product)?;
    fetch_artifact_to_job(
        source.download_url.clone(),
        hosts,
        job_directory,
        ArtifactKind::Dmg,
        cancellation,
        progress,
    )
    .await
}

pub async fn fetch_artifact_to_job(
    url: Url,
    hosts: &[&str],
    job_directory: &JobTempDir,
    artifact_kind: ArtifactKind,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
) -> Result<DownloadedArtifact, AgentReasonCode> {
    let transport = RuntimeInstallerTransport::new(InstallerTransportPurpose::Download, USER_AGENT);
    fetch_artifact_to_job_with(
        &transport,
        url,
        hosts,
        job_directory,
        artifact_kind,
        cancellation,
        progress,
    )
    .await
}

async fn fetch_artifact_to_job_with(
    transport: &dyn HttpTransport,
    url: Url,
    hosts: &[&str],
    job_directory: &JobTempDir,
    artifact_kind: ArtifactKind,
    cancellation: &dyn Cancellation,
    progress: &dyn DownloadProgressSink,
) -> Result<DownloadedArtifact, AgentReasonCode> {
    https_url_on_allowlist(&url, hosts).map_err(download_source_reason)?;
    let output = prepare_transport_download(job_directory, artifact_kind)
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    let response = match get_allowlisted(transport, url, hosts, cancellation).await {
        Ok(response) => response,
        Err(error) => {
            drop(output);
            let _ =
                job_directory.remove_artifact_if_present(&job_directory.part_path(artifact_kind));
            return Err(download_source_reason(error));
        }
    };
    let result = persist_transport_response(
        response,
        output,
        job_directory,
        artifact_kind,
        MAX_STREAMED_ARTIFACT_BYTES,
        cancellation,
        progress,
        1,
        1,
        None,
    )
    .await;
    match result {
        Ok(artifact) => Ok(artifact),
        Err(error) => {
            let _ =
                job_directory.remove_artifact_if_present(&job_directory.part_path(artifact_kind));
            let _ =
                job_directory.remove_artifact_if_present(&job_directory.final_path(artifact_kind));
            Err(match error {
                PersistDownloadError::Cancelled => AgentReasonCode::Cancelled,
                PersistDownloadError::Transport(_) => AgentReasonCode::SourceNotVerified,
                PersistDownloadError::Installer(_) => AgentReasonCode::InstallerArtifactUnavailable,
            })
        }
    }
}

fn download_source_reason(error: SourceResolveError) -> AgentReasonCode {
    match error {
        SourceResolveError::Cancelled => AgentReasonCode::Cancelled,
        SourceResolveError::PlatformUnsupported => AgentReasonCode::PlatformUnsupported,
        _ => AgentReasonCode::SourceNotVerified,
    }
}

async fn get_allowlisted(
    transport: &dyn HttpTransport,
    initial: Url,
    hosts: &[&str],
    cancellation: &dyn Cancellation,
) -> Result<TransportResponse, SourceResolveError> {
    https_url_on_allowlist(&initial, hosts)?;
    let mut current = initial;
    let mut followed = 0;
    loop {
        let response =
            match race_with_cancellation(transport.get(current.clone()), cancellation).await {
                Err(_) => return Err(SourceResolveError::Cancelled),
                Ok(Err(_)) => return Err(SourceResolveError::SchemaInvalid),
                Ok(Ok(response)) => response,
            };
        if !(300..=399).contains(&response.status) {
            if !(200..=299).contains(&response.status) {
                return Err(SourceResolveError::SchemaInvalid);
            }
            return Ok(response);
        }
        let location = response
            .location
            .clone()
            .ok_or(SourceResolveError::HostRejected)?;
        drop(response);
        current = resolve_redirect(&current, &location, followed)
            .map_err(|_| SourceResolveError::HostRejected)?;
        https_url_on_allowlist(&current, hosts)?;
        followed += 1;
        if followed > MAX_REDIRECTS {
            return Err(SourceResolveError::HostRejected);
        }
    }
}

async fn collect_body(
    mut response: TransportResponse,
    cap: usize,
    cancellation: &dyn Cancellation,
) -> Result<Vec<u8>, SourceResolveError> {
    let mut buf = Vec::new();
    loop {
        if cancellation.is_cancelled() {
            return Err(SourceResolveError::Cancelled);
        }
        let next = race_with_cancellation(response.body.next(), cancellation).await;
        let chunk = match next {
            Err(_) => return Err(SourceResolveError::Cancelled),
            Ok(None) => break,
            Ok(Some(Err(_))) => return Err(SourceResolveError::SchemaInvalid),
            Ok(Some(Ok(chunk))) => chunk,
        };
        if buf.len().saturating_add(chunk.len()) > cap {
            return Err(SourceResolveError::SchemaInvalid);
        }
        buf.extend_from_slice(&chunk);
    }
    if buf.is_empty() {
        return Err(SourceResolveError::SchemaInvalid);
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::super::sources::QODERWORK_REDIRECT_HOSTS;
    use super::*;
    use crate::codex_desktop::download::{DownloadProgressUpdate, TransportError};
    use bytes::Bytes;
    use futures::stream;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use uuid::Uuid;

    struct FakeTransport {
        responses: Mutex<VecDeque<TransportResponse>>,
    }

    impl FakeTransport {
        fn new(responses: impl IntoIterator<Item = TransportResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
            }
        }
    }

    impl HttpTransport for FakeTransport {
        fn get(&self, _url: Url) -> crate::codex_desktop::download::TransportFuture<'_> {
            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::non_retryable("unexpected request"));
            Box::pin(async move { response })
        }
    }

    fn body_response<I>(status: u16, content_length: Option<u64>, chunks: I) -> TransportResponse
    where
        I: IntoIterator<Item = Result<Bytes, TransportError>>,
        I::IntoIter: Send + 'static,
    {
        TransportResponse {
            status,
            location: None,
            content_length,
            retry_after: None,
            body: Box::pin(stream::iter(chunks)),
        }
    }

    fn test_job_directory(root: &tempfile::TempDir) -> JobTempDir {
        JobTempDir::create(root.path(), &Uuid::new_v4().hyphenated().to_string()).unwrap()
    }

    fn qoderwork_macos_source(
        format: PackageFormat,
        platform: AgentPlatform,
    ) -> ResolvedDesktopSource {
        ResolvedDesktopSource {
            product: AgentCatalogId::QoderWork,
            platform,
            architecture: super::super::sources::AgentArch::Aarch64,
            format,
            release_id: "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            display_version: None,
            download_url: Url::parse(
                "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
            )
            .unwrap(),
            versionless_latest: true,
            official_page: "https://qoder.com.cn/download",
        }
    }

    #[test]
    fn metadata_url_must_stay_on_the_product_allowlist() {
        let ok = Url::parse("https://static.qoder.com.cn/qoder-work-cn/releases/latest-mac.yml")
            .unwrap();
        assert!(https_url_on_allowlist(&ok, QODERWORK_REDIRECT_HOSTS).is_ok());
        let alias = Url::parse(
            "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
        )
        .unwrap();
        assert!(https_url_on_allowlist(&alias, QODERWORK_REDIRECT_HOSTS).is_ok());
        let evil =
            Url::parse("https://evil.example/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg")
                .unwrap();
        assert_eq!(
            https_url_on_allowlist(&evil, QODERWORK_REDIRECT_HOSTS),
            Err(SourceResolveError::HostRejected)
        );
    }

    #[tokio::test]
    async fn macos_dmg_download_rejects_windows_exe_sources() {
        let source = ResolvedDesktopSource {
            product: AgentCatalogId::QoderWork,
            platform: AgentPlatform::Windows,
            architecture: super::super::sources::AgentArch::X86_64,
            format: PackageFormat::Exe,
            release_id: "v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            display_version: None,
            download_url: Url::parse(
                "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-Setup-User-x64.exe",
            )
            .unwrap(),
            versionless_latest: true,
            official_page: "https://qoder.com.cn/download",
        };
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        assert_eq!(
            download_macos_dmg_to_job(&source, &job_directory, &NeverCancelled, &|_| {},).await,
            Err(AgentReasonCode::PlatformUnsupported)
        );
        assert!(!job_directory.path().join("installer.dmg").exists());
        assert!(!job_directory.path().join("installer.dmg.part").exists());
    }

    #[tokio::test]
    async fn macos_dmg_streams_to_the_fixed_job_local_artifact() {
        let expected = b"streamed-dmg-bytes";
        let transport = FakeTransport::new([body_response(
            200,
            Some(expected.len() as u64),
            [
                Ok(Bytes::from_static(b"streamed-")),
                Ok(Bytes::from_static(b"dmg-bytes")),
            ],
        )]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let updates = Mutex::new(Vec::new());
        let progress = |update: DownloadProgressUpdate| updates.lock().unwrap().push(update);
        let url = qoderwork_macos_source(PackageFormat::Dmg, AgentPlatform::Macos).download_url;

        let artifact = fetch_artifact_to_job_with(
            &transport,
            url,
            QODERWORK_REDIRECT_HOSTS,
            &job_directory,
            ArtifactKind::Dmg,
            &NeverCancelled,
            &progress,
        )
        .await
        .unwrap();

        assert_eq!(artifact.path(), job_directory.path().join("installer.dmg"));
        assert_eq!(std::fs::read(artifact.path()).unwrap(), expected);
        assert!(!job_directory.path().join("installer.dmg.part").exists());
        assert!(!job_directory.path().join("installer.exe").exists());
        let updates = updates.lock().unwrap();
        assert!(updates
            .iter()
            .any(|update| update.completed_bytes == expected.len() as u64));
        assert!(updates
            .iter()
            .all(|update| update.total_bytes == expected.len() as u64));
    }

    #[tokio::test]
    async fn unknown_content_length_still_persists_without_inventing_a_total() {
        let expected = b"partial-length-body";
        let transport =
            FakeTransport::new([body_response(200, None, [Ok(Bytes::from_static(expected))])]);
        let directory = tempfile::tempdir().unwrap();
        let job_directory = test_job_directory(&directory);
        let updates = Mutex::new(Vec::new());
        let progress = |update: DownloadProgressUpdate| updates.lock().unwrap().push(update);

        let artifact = fetch_artifact_to_job_with(
            &transport,
            Url::parse(
                "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
            )
            .unwrap(),
            QODERWORK_REDIRECT_HOSTS,
            &job_directory,
            ArtifactKind::Dmg,
            &NeverCancelled,
            &progress,
        )
        .await
        .unwrap();

        assert_eq!(std::fs::read(artifact.path()).unwrap(), expected);
        let updates = updates.lock().unwrap();
        assert!(updates.iter().all(|update| update.total_bytes == 0));
        if let Some(last) = updates.last() {
            assert_eq!(
                super::super::types::AgentActionTransferSample::from_progress_bytes(
                    last.completed_bytes,
                    last.total_bytes,
                    last.attempt,
                    last.max_attempts,
                )
                .total_bytes,
                None
            );
        }
    }
}
