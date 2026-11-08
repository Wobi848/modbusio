//! Copied from bacnetio, the program's name changed (concept section 14, answer 6).
//!
//! One modbusio per data folder: a second start opens the running one in the browser instead of
//! starting a second server, which would write the same files and compete for the serial port.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Holds the web port of the modbusio serving the folder.
const PORT_FILE: &str = "web-port";
/// A modbusio on this computer answers at once; a stale port must not hold up the start.
const ASK_TIMEOUT: Duration = Duration::from_secs(2);

/// The address of a modbusio that already serves `data_dir`.
pub async fn already_running(data_dir: &Path) -> Option<SocketAddr> {
    let text = std::fs::read_to_string(data_dir.join(PORT_FILE)).ok()?;
    let port: u16 = text.trim().parse().ok()?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let answers = tokio::time::timeout(ASK_TIMEOUT, is_modbusio(address)).await;
    matches!(answers, Ok(true)).then_some(address)
}

/// Notes the port for the next start; after a crash the note stays, and the next start finds
/// nobody answering there.
pub fn remember(data_dir: &Path, address: SocketAddr) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    std::fs::write(data_dir.join(PORT_FILE), address.port().to_string())
}

/// Another program may have taken the port since: only modbusio's own answer counts.
async fn is_modbusio(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect(address).await else {
        return false;
    };
    let request = format!(
        "GET /api/info HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        address.port()
    );
    let mut answer = Vec::new();
    if stream.write_all(request.as_bytes()).await.is_err()
        || stream.read_to_end(&mut answer).await.is_err()
    {
        return false;
    }
    let answer = String::from_utf8_lossy(&answer);
    answer.split_whitespace().nth(1) == Some("200") && answer.contains("\"program\":\"modbusio\"")
}
