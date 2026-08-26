//! Allowlisted HTTPS GET for Agent desktop metadata and artifacts.
//! Redirect targets never become IPC locators; every hop is rechecked.

use futures::StreamExt;
use url::Url;

use super::sources::{https_url_on_allowlist, SourceResolveError, MAX_SOURCE_METADATA_BYTES};
use crate::codex_desktop::{
    cancellation::{race_with_cancellation, Cancellation, NeverCancelled},
    download::{resolve_redirect, HttpTransport, TransportResponse, MAX_REDIRECTS},
    runtime::{InstallerTransportPurpose, RuntimeInstallerTransport},
};

const USER_AGENT: &str = "fyagent-agent-installer";
const MAX_ARTIFACT_BYTES: usize = 2 * 1024 * 1024 * 1024;

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
