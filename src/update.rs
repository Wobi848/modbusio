//! Copied from bacnetio, the program's name changed (concept section 11).
//!
//! Whether a newer release is out (concept section 11). Asks GitHub for its newest
//! release and reads only the redirect, which names that release's tag.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, bail};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const HOST: &str = "github.com";
/// The whole check, connect to answer: a plant without internet must not wait long.
const TIMEOUT: Duration = Duration::from_secs(10);
/// The head of the answer is all that is read; GitHub's is a few kilobytes.
const MOST_HEAD_BYTES: usize = 64 * 1024;

/// A published release: its version without the `v` and its page.
#[derive(Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
}

/// The newest release, or none when nothing is published yet.
pub async fn latest_release() -> anyhow::Result<Option<Release>> {
    let path = env!("CARGO_PKG_REPOSITORY")
        .strip_prefix("https://github.com")
        .context("the repository is not on GitHub")?;
    let head = tokio::time::timeout(TIMEOUT, ask(&format!("{path}/releases/latest")))
        .await
        .context("GitHub did not answer in time")??;
    let location = redirect_target(&head)?;
    Ok(release_from_location(&location))
}

/// Sends one GET and returns the head of the answer.
async fn ask(path: &str) -> anyhow::Result<String> {
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let tcp = TcpStream::connect((HOST, 443))
        .await
        .context("no connection to GitHub")?;
    let mut tls = TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from(HOST)?, tcp)
        .await
        .context("no secure connection to GitHub")?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {HOST}\r\nUser-Agent: modbusio/{}\r\nConnection: close\r\n\r\n",
        env!("CARGO_PKG_VERSION")
    );
    tls.write_all(request.as_bytes()).await?;
    let mut head = Vec::new();
    let mut chunk = [0u8; 4096];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = tls.read(&mut chunk).await?;
        if read == 0 || head.len() > MOST_HEAD_BYTES {
            bail!("GitHub's answer ended early");
        }
        head.extend_from_slice(&chunk[..read]);
    }
    Ok(String::from_utf8_lossy(&head).into_owned())
}

/// The address a redirect points to.
fn redirect_target(head: &str) -> anyhow::Result<String> {
    let status = head.split_whitespace().nth(1).unwrap_or_default();
    if !status.starts_with('3') {
        bail!("GitHub answered {status} instead of naming the newest release");
    }
    head.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("location")
                .then(|| value.trim().to_string())
        })
        .context("GitHub's redirect names no address")
}

/// `…/releases/tag/v0.2.0` names a release; `…/releases` alone means none is published.
fn release_from_location(location: &str) -> Option<Release> {
    let (_, tag) = location.rsplit_once("/releases/tag/")?;
    Some(Release {
        version: tag.strip_prefix('v').unwrap_or(tag).to_string(),
        url: location.to_string(),
    })
}

/// Whether `candidate` is a later release than `current`. Only plain `major.minor.patch` counts:
/// a pre-release such as `0.2.0-rc1` is never offered.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (numbers(candidate), numbers(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn numbers(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.').map(|part| part.parse().ok());
    let numbers = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_names_the_release() {
        let release =
            release_from_location("https://github.com/Wobi848/modbusio/releases/tag/v0.2.0");
        assert_eq!(
            release,
            Some(Release {
                version: "0.2.0".into(),
                url: "https://github.com/Wobi848/modbusio/releases/tag/v0.2.0".into(),
            })
        );
        assert_eq!(
            release_from_location("https://github.com/Wobi848/modbusio/releases"),
            None
        );
    }

    #[test]
    fn the_redirect_is_read_from_the_head() {
        let head = "HTTP/1.1 302 Found\r\nDate: today\r\nlocation: https://github.com/a/b/releases/tag/v1.0.0\r\n\r\n";
        assert_eq!(
            redirect_target(head).unwrap(),
            "https://github.com/a/b/releases/tag/v1.0.0"
        );
        assert!(redirect_target("HTTP/1.1 404 Not Found\r\n\r\n").is_err());
    }

    #[test]
    fn newer_by_numbers_not_by_text() {
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("0.2.0-rc1", "0.1.0"));
        assert!(!is_newer("nightly", "0.1.0"));
    }
}
