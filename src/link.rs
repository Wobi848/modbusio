//! One open connection to a line: a TCP connection, or a byte stream carrying RTU frames (serial
//! port, or a gateway passing RTU over TCP). Sends a request, waits for its answer.

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout_at};

use crate::capture::{Capture, Direction};
use crate::endpoint::Endpoint;
use crate::protocol::{Exception, PduError, Request, Response, pdu, rtu, tcp};

pub trait ByteStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> ByteStream for T {}

#[derive(Clone, Debug, PartialEq)]
pub enum LinkError {
    /// No answer in time.
    Timeout,
    /// Bytes came back, but no valid answer: damaged on the line, or wrong speed or parity.
    Damaged {
        bytes: usize,
    },
    /// An answer whose CRC does not match.
    Crc,
    Exception(Exception),
    /// The answer does not fit the request.
    Malformed(String),
    /// Another unit answered: a second master, or two devices with one address.
    OtherUnit(u8),
    /// The device closed or reset the connection.
    Closed,
    /// The connection could not be opened.
    Open(String),
    Io(String),
}

impl LinkError {
    /// The connection is no longer usable and is opened anew for the next request.
    pub fn drops_connection(&self) -> bool {
        matches!(
            self,
            LinkError::Closed | LinkError::Io(_) | LinkError::Open(_)
        )
    }

    /// Something answered: the device is there, even if it refused.
    pub fn device_answered(&self) -> bool {
        matches!(
            self,
            LinkError::Exception(exception) if !exception.from_gateway()
        ) || matches!(self, LinkError::Malformed(_))
    }

    /// A stable code for the interface to translate (docs/api.md).
    pub fn code(&self) -> String {
        match self {
            LinkError::Timeout => "timeout".into(),
            LinkError::Damaged { .. } => "damaged".into(),
            LinkError::Crc => "crc".into(),
            LinkError::Exception(exception) => format!("exception_{:02x}", exception.0),
            LinkError::Malformed(_) => "malformed".into(),
            LinkError::OtherUnit(_) => "other_unit".into(),
            LinkError::Closed => "closed".into(),
            LinkError::Open(_) => "open".into(),
            LinkError::Io(_) => "io".into(),
        }
    }
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkError::Timeout => f.write_str("no answer in time"),
            LinkError::Damaged { bytes } => write!(
                f,
                "{bytes} bytes came back but no valid answer: wrong speed or parity, or damaged on the line"
            ),
            LinkError::Crc => f.write_str("answer damaged on the line (CRC does not match)"),
            LinkError::Exception(exception) => exception.fmt(f),
            LinkError::Malformed(reason) => write!(f, "answer does not fit the request: {reason}"),
            LinkError::OtherUnit(unit) => write!(
                f,
                "unit {unit} answered instead: another master on the line, or two devices with one address"
            ),
            LinkError::Closed => f.write_str("the device closed the connection"),
            LinkError::Open(reason) => write!(f, "cannot open the connection: {reason}"),
            LinkError::Io(reason) => write!(f, "connection failed: {reason}"),
        }
    }
}

impl std::error::Error for LinkError {}

impl From<PduError> for LinkError {
    fn from(error: PduError) -> Self {
        match error {
            PduError::Exception(exception) => LinkError::Exception(exception),
            PduError::Malformed(reason) => LinkError::Malformed(reason),
        }
    }
}

fn io_error(error: io::Error) -> LinkError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::BrokenPipe => LinkError::Closed,
        _ => LinkError::Io(error.to_string()),
    }
}

enum Io {
    Tcp(TcpStream),
    Rtu(Box<dyn ByteStream>),
}

pub struct Link {
    pub endpoint: Endpoint,
    io: Io,
    transaction: u16,
    capture: Arc<Capture>,
    /// The silence that has to pass on a serial line before the next request.
    gap: Duration,
    last_activity: Instant,
    /// Local and remote address of a TCP connection, for the capture.
    addresses: Option<(SocketAddr, SocketAddr)>,
}

impl Link {
    pub async fn open(
        endpoint: &Endpoint,
        connect_timeout: Duration,
        capture: Arc<Capture>,
    ) -> Result<Link, LinkError> {
        let (io, addresses, gap) = match endpoint {
            Endpoint::Tcp(address) | Endpoint::RtuOverTcp(address) => {
                let stream = connect(*address, connect_timeout).await?;
                let _ = stream.set_nodelay(true);
                let addresses = stream.local_addr().ok().map(|local| (local, *address));
                let io = if matches!(endpoint, Endpoint::Tcp(_)) {
                    Io::Tcp(stream)
                } else {
                    Io::Rtu(Box::new(stream))
                };
                // a gateway puts the frame on its line; a short pause between requests helps it
                (io, addresses, Duration::from_millis(5))
            }
            Endpoint::Serial(settings) => {
                let port = crate::serial::open(settings)?;
                let gap = Duration::from_micros(rtu::frame_gap_micros(settings.baud));
                (Io::Rtu(port), None, gap)
            }
        };
        Ok(Link {
            endpoint: endpoint.clone(),
            io,
            transaction: 0,
            capture,
            gap,
            last_activity: Instant::now(),
            addresses,
        })
    }

    pub async fn exchange(
        &mut self,
        unit: u8,
        request: &Request,
        timeout: Duration,
    ) -> Result<Response, LinkError> {
        request.check().map_err(LinkError::Malformed)?;
        let pdu = request.encode();
        let deadline = Instant::now() + timeout;
        let result = match self.io {
            Io::Tcp(_) => self.exchange_tcp(unit, &pdu, request, deadline).await,
            Io::Rtu(_) => self.exchange_rtu(unit, &pdu, request, deadline).await,
        };
        self.last_activity = Instant::now();
        result
    }

    fn record(&self, direction: Direction, bytes: &[u8]) {
        self.capture
            .record(&self.endpoint, self.addresses, direction, bytes);
    }

    async fn exchange_tcp(
        &mut self,
        unit: u8,
        pdu: &[u8],
        request: &Request,
        deadline: Instant,
    ) -> Result<Response, LinkError> {
        self.transaction = self.transaction.wrapping_add(1);
        let transaction = self.transaction;
        let frame = tcp::frame(transaction, unit, pdu);
        self.record(Direction::Sent, &frame);
        let Io::Tcp(stream) = &mut self.io else {
            unreachable!("exchange_tcp on a TCP link")
        };
        stream.write_all(&frame).await.map_err(io_error)?;
        loop {
            let mut header = [0u8; tcp::HEADER];
            read_exact(stream, &mut header, deadline).await?;
            let parsed = tcp::parse_header(&header).map_err(LinkError::Malformed)?;
            let mut answer = vec![0u8; parsed.pdu_length];
            read_exact(stream, &mut answer, deadline).await?;
            let mut whole = header.to_vec();
            whole.extend_from_slice(&answer);
            self.capture
                .record(&self.endpoint, self.addresses, Direction::Received, &whole);
            // a late answer to an earlier request that timed out: skip it
            if parsed.transaction != transaction {
                continue;
            }
            return Ok(pdu::decode_response(request, &answer)?);
        }
    }

    async fn exchange_rtu(
        &mut self,
        unit: u8,
        pdu: &[u8],
        request: &Request,
        deadline: Instant,
    ) -> Result<Response, LinkError> {
        let frame = rtu::frame(unit, pdu);
        let quiet_until = self.last_activity + self.gap;
        let Io::Rtu(stream) = &mut self.io else {
            unreachable!("exchange_rtu on an RTU link")
        };
        drain(stream).await;
        tokio::time::sleep_until(quiet_until).await;
        self.capture
            .record(&self.endpoint, self.addresses, Direction::Sent, &frame);
        stream.write_all(&frame).await.map_err(io_error)?;
        stream.flush().await.map_err(io_error)?;
        let mut buffer: Vec<u8> = Vec::new();
        let mut skipped = 0;
        loop {
            // adapters that hear their own sending give the request back first
            if buffer.len() >= frame.len() && buffer.starts_with(&frame) {
                buffer.drain(..frame.len());
            }
            match rtu::parse_answer(&buffer) {
                rtu::Received::Incomplete => {}
                rtu::Received::Frame {
                    unit: answered,
                    length,
                } => {
                    self.capture.record(
                        &self.endpoint,
                        self.addresses,
                        Direction::Received,
                        &buffer[..length],
                    );
                    if answered != unit {
                        return Err(LinkError::OtherUnit(answered));
                    }
                    return Ok(pdu::decode_response(request, &buffer[1..length - 2])?);
                }
                rtu::Received::BadCrc { length } => {
                    self.capture.record(
                        &self.endpoint,
                        self.addresses,
                        Direction::Received,
                        &buffer[..length],
                    );
                    drain(stream).await;
                    return Err(LinkError::Crc);
                }
                rtu::Received::Garbage(_) => {
                    buffer.remove(0);
                    skipped += 1;
                    continue;
                }
            }
            let mut chunk = [0u8; 512];
            match timeout_at(deadline, stream.read(&mut chunk)).await {
                Err(_) => {
                    let bytes = skipped + buffer.len();
                    return Err(if bytes > 0 {
                        LinkError::Damaged { bytes }
                    } else {
                        LinkError::Timeout
                    });
                }
                Ok(Ok(0)) => return Err(LinkError::Closed),
                Ok(Ok(read)) => buffer.extend_from_slice(&chunk[..read]),
                Ok(Err(error)) => return Err(io_error(error)),
            }
        }
    }
}

async fn connect(address: SocketAddr, connect_timeout: Duration) -> Result<TcpStream, LinkError> {
    match tokio::time::timeout(connect_timeout, TcpStream::connect(address)).await {
        Err(_) => Err(LinkError::Open(format!("{address} does not answer"))),
        Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => {
            Err(LinkError::Open(format!(
                "{address} refuses the connection (no Modbus server on this port, or it takes no more connections)"
            )))
        }
        Ok(Err(error)) => Err(LinkError::Open(format!("{address}: {error}"))),
        Ok(Ok(stream)) => Ok(stream),
    }
}

async fn read_exact(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
) -> Result<(), LinkError> {
    match timeout_at(deadline, stream.read_exact(buffer)).await {
        Err(_) => Err(LinkError::Timeout),
        Ok(Err(error)) => Err(io_error(error)),
        Ok(Ok(_)) => Ok(()),
    }
}

/// Throws away what is waiting: a late answer to an earlier request would be taken for this one.
async fn drain(stream: &mut Box<dyn ByteStream>) {
    let mut chunk = [0u8; 512];
    while let Ok(Ok(read)) =
        tokio::time::timeout(Duration::from_millis(1), stream.read(&mut chunk)).await
    {
        if read == 0 {
            break;
        }
    }
}
