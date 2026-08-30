//! Allowlisted HTTPS GET for Agent desktop metadata and artifacts.
//! Redirect targets never become IPC locators; every hop is rechecked.

use futures::StreamExt;
use url::Url;

use super::sources::{https_url_on_allowlist, SourceResolveError, MAX_SOURCE_METADATA_BYTES};
#[cfg(target_os = "windows")]
use super::types::AgentReasonCode;
use crate::codex_desktop::{
    cancellation::{race_with_cancellation, Cancellation, NeverCancelled},
    download::{resolve_redirect, HttpTransport, TransportResponse, MAX_REDIRECTS},
    runtime::{InstallerTransportPurpose, RuntimeInstallerTransport},
};
#[cfg(target_os = "windows")]
use crate::codex_desktop::{
    download::{
        persist_transport_response, prepare_transport_download, DownloadProgressUpdate,
        DownloadedArtifact, PersistDownloadError,
    },
    temp::JobTempDir,
    verify::ArtifactKind,
};

const USER_AGENT: &str = "fyagent-agent-installer";
const MAX_ARTIFACT_BYTES: usize = 2 * 1024 * 1024 * 1024;
#[cfg(target_os = "windows")]
const MAX_STREAMED_ARTIFACT_BYTES: u64 = MAX_ARTIFACT_BYTES as u64;

pub async fn fetch_metadata_bytes(url: Url, hosts: &[&str]) -> Result<Vec<u8>, SourceResolveError> {
    https_url_on_allowlist(&url, hosts)?;
    let transport = RuntimeInstallerTransport::new(InstallerTransportPurpose::Metadata, USER_AGENT);
    let response = get_allowlisted(&transport, url, hosts, &NeverCancelled).await?;
    collect_body(response, MAX_SOURCE_METADATA_BYTES, &NeverCancelled).await
}

pub async fn fetch_artifact_bytes(
    url: Url,
    hosts: &[&str],
    cancellation: &dyn Cancellation,
) -> Result<Vec<u8>, SourceResolveError> {
    https_url_on_allowlist(&url, hosts)?;
    let transport = RuntimeInstallerTransport::new(InstallerTransportPurpose::Download, USER_AGENT);
    let response = get_allowlisted(&transport, url, hosts, cancellation).await?;
    collect_body(response, MAX_ARTIFACT_BYTES, cancellation).await
}

#[cfg(target_os = "windows")]
pub async fn fetch_artifact_to_job(
    url: Url,
    hosts: &[&str],
    job_directory: &JobTempDir,
    cancellation: &dyn Cancellation,
) -> Result<DownloadedArtifact, AgentReasonCode> {
    https_url_on_allowlist(&url, hosts).map_err(download_source_reason)?;
    let output = prepare_transport_download(job_directory, ArtifactKind::Exe)
        .map_err(|_| AgentReasonCode::InstallerArtifactUnavailable)?;
    let transport = RuntimeInstallerTransport::new(InstallerTransportPurpose::Download, USER_AGENT);
    let response = match get_allowlisted(&transport, url, hosts, cancellation).await {
        Ok(response) => response,
        Err(error) => {
            drop(output);
            let _ = job_directory
                .remove_artifact_if_present(&job_directory.part_path(ArtifactKind::Exe));
            return Err(download_source_reason(error));
        }
    };
    let progress = |_update: DownloadProgressUpdate| {};
    let result = persist_transport_response(
        response,
        output,
        job_directory,
        ArtifactKind::Exe,
        MAX_STREAMED_ARTIFACT_BYTES,
        cancellation,
        &progress,
        1,
        1,
        None,
    )
    .await;
    match result {
        Ok(artifact) => Ok(artifact),
        Err(error) => {
            let _ = job_directory
                .remove_artifact_if_present(&job_directory.part_path(ArtifactKind::Exe));
            let _ = job_directory
                .remove_artifact_if_present(&job_directory.final_path(ArtifactKind::Exe));
            Err(match error {
                PersistDownloadError::Cancelled => AgentReasonCode::Cancelled,
                PersistDownloadError::Transport(_) => AgentReasonCode::SourceNotVerified,
                PersistDownloadError::Installer(_) => AgentReasonCode::InstallerArtifactUnavailable,
            })
        }
    }
}

#[cfg(target_os = "windows")]
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
}
