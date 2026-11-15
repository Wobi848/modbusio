//! Reading a trunk for the analyser. Rule one of docs/concept-analyser.md: the analyser never
//! sends. The port it gets can only be read: [`ReadOnly`] has no way to write, so sending is
//! impossible by its type, not by care. An adapter with automatic direction only drives the line
//! when bytes are written, so it stays silent.

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::time::Instant;

use super::Analysis;
use crate::endpoint::SerialSettings;
use crate::link::{ByteStream, LinkError};
use crate::protocol::rtu;

/// A serial port opened to be read only.
pub struct ReadOnly(Box<dyn ByteStream>);

impl ReadOnly {
    pub fn open(settings: &SerialSettings) -> Result<ReadOnly, LinkError> {
        crate::serial::open(settings).map(ReadOnly)
    }
}

impl AsyncRead for ReadOnly {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(context, buffer)
    }
}

/// How long after opening bytes still count as held from before.
const STALE: Duration = Duration::from_millis(20);

/// How long the line must be silent before bytes waiting cannot grow into a frame: longer than a
/// USB adapter's wait before it hands over a small chunk (16 ms for an FTDI chip by default), and
/// than ten characters at slow speeds.
pub fn quiet_after(baud: u32) -> Duration {
    let ten_characters = Duration::from_micros(rtu::character_micros(baud) * 10);
    ten_characters.max(Duration::from_millis(50))
}

/// The settings of the trunk on `port`: the ones given, or found by listening (`listen.rs`).
pub async fn settings(port: &str, given: Option<(u32, &str)>) -> Result<SerialSettings, String> {
    let (baud, frame) = match given {
        Some(given) => (given.0, given.1.to_string()),
        None => {
            let listened = crate::listen::listen(port, Duration::from_millis(1500))
                .await
                .map_err(|error| error.to_string())?;
            let active = listened.active.ok_or_else(|| {
                if listened.quiet {
                    format!(
                        "nothing heard on {port} at any speed: is the master polling, A/B swapped?"
                    )
                } else {
                    format!("no valid frames on {port} at any speed: check A/B, termination, bias")
                }
            })?;
            let (baud, frame) = active.split_once(' ').unwrap_or((&active, ""));
            (
                baud.parse().map_err(|_| format!("no speed in {active}"))?,
                frame.to_string(),
            )
        }
    };
    let (parity, stop_bits) = SerialSettings::parse_frame(&frame)
        .ok_or_else(|| format!("frame {frame} must be like 8E1 or 8N2"))?;
    Ok(SerialSettings {
        port: port.to_string(),
        baud,
        parity,
        stop_bits,
    })
}

/// What the port's reader hands on, with the time it arrived at.
enum Arrived {
    Bytes(u64, Vec<u8>),
    /// The line was quiet for a while.
    Quiet,
}

/// Reads `port` into `analysis` until it fails or closes; the time of each chunk is counted from
/// `started`. Returns why it ended.
///
/// Reading never waits for the analysis: the page reads it once a second and holds it meanwhile,
/// and bytes arriving then would lose the time they arrived at (a request and its answer seeming
/// to come together). So the reader only stamps and queues; a thread of its own takes the queue
/// into the analysis.
pub async fn hear(
    port: impl AsyncRead + Unpin,
    analysis: Arc<Mutex<Analysis>>,
    started: Instant,
    quiet: Duration,
) -> String {
    let (arrived, queue) = std::sync::mpsc::channel();
    let taking = std::thread::spawn(move || {
        for item in queue {
            let mut analysis = analysis.lock().expect("analysis lock");
            match item {
                Arrived::Bytes(at, bytes) => analysis.push(at, &bytes),
                Arrived::Quiet => analysis.quiet(),
            }
        }
    });
    let why = read(port, started, quiet, &arrived).await;
    // what is queued still goes in before the end is told
    drop(arrived);
    let _ = tokio::task::spawn_blocking(move || taking.join()).await;
    why
}

async fn read(
    mut port: impl AsyncRead + Unpin,
    started: Instant,
    quiet: Duration,
    arrived: &std::sync::mpsc::Sender<Arrived>,
) -> String {
    let mut chunk = [0u8; 1024];
    // bytes the port held before listening began arrived at times nobody knows: give them up
    let fresh = Instant::now() + STALE;
    while let Ok(read) = tokio::time::timeout_at(fresh, port.read(&mut chunk)).await {
        match read {
            Ok(0) => return "the port closed: adapter unplugged?".into(),
            Ok(_) => {}
            Err(error) => return format!("reading the port failed: {error}"),
        }
    }
    loop {
        let item = match tokio::time::timeout(quiet, port.read(&mut chunk)).await {
            Ok(Ok(0)) => return "the port closed: adapter unplugged?".into(),
            Ok(Ok(count)) => Arrived::Bytes(
                started.elapsed().as_micros() as u64,
                chunk[..count].to_vec(),
            ),
            Ok(Err(error)) => return format!("reading the port failed: {error}"),
            Err(_) => Arrived::Quiet,
        };
        if arrived.send(item).is_err() {
            return "the analysis ended".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_is_at_least_50_ms_and_ten_characters_at_slow_speeds() {
        assert_eq!(quiet_after(19_200), Duration::from_millis(50));
        // 1200 baud: 11 bits a character, 9.17 ms
        assert_eq!(quiet_after(1200), Duration::from_micros(91_660));
    }

    #[tokio::test(start_paused = true)]
    async fn bytes_are_heard_and_a_closed_port_says_so() {
        let (mut line, port) = tokio::io::duplex(256);
        let analysis = Arc::new(Mutex::new(Analysis::default()));
        use tokio::io::AsyncWriteExt;
        line.write_all(&[0x55; 9]).await.unwrap();
        let hearing = tokio::spawn(hear(
            port,
            analysis.clone(),
            Instant::now(),
            quiet_after(9600),
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;
        line.write_all(&rtu::frame(3, &[0x03, 0, 0, 0, 1]))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        line.write_all(&rtu::frame(3, &[0x03, 2, 0, 9]))
            .await
            .unwrap();
        drop(line);
        assert_eq!(
            hearing.await.unwrap(),
            "the port closed: adapter unplugged?"
        );
        let analysis = analysis.lock().unwrap();
        assert_eq!(analysis.units[&3].answers, 1);
        // the bytes held from before listening were given up
        assert_eq!(analysis.bus.bytes, 15);
        let answer_micros = analysis.exchanges()[0].item.answer_micros.unwrap();
        assert!((30_000..31_000).contains(&answer_micros), "{answer_micros}");
    }

    /// The page reads the analysis once a second and holds it meanwhile. Bytes arriving then
    /// must keep the time they arrived at, or request and answer seem to come together.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn times_are_those_of_arrival_also_while_the_analysis_is_being_read() {
        use tokio::io::AsyncWriteExt;
        let (mut line, port) = tokio::io::duplex(256);
        let analysis = Arc::new(Mutex::new(Analysis::default()));
        let hearing = tokio::spawn(hear(
            port,
            analysis.clone(),
            Instant::now(),
            quiet_after(9600),
        ));
        tokio::time::sleep(Duration::from_millis(60)).await;
        let held = analysis.clone();
        let reader = std::thread::spawn(move || {
            let _held = held.lock().unwrap();
            std::thread::sleep(Duration::from_millis(150));
        });
        // written by a thread with its own clock: a blocked worker would stall this test's timers
        let runtime = tokio::runtime::Handle::current();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            runtime
                .block_on(line.write_all(&rtu::frame(3, &[0x03, 0, 0, 0, 1])))
                .unwrap();
            std::thread::sleep(Duration::from_millis(40));
            runtime
                .block_on(line.write_all(&rtu::frame(3, &[0x03, 2, 0, 9])))
                .unwrap();
            std::thread::sleep(Duration::from_millis(200));
            drop(line);
        });
        tokio::task::spawn_blocking(move || {
            reader.join().unwrap();
            writer.join().unwrap();
        })
        .await
        .unwrap();
        hearing.await.unwrap();
        let analysis = analysis.lock().unwrap();
        let answer_micros = analysis.exchanges()[0].item.answer_micros.unwrap();
        assert!((35_000..70_000).contains(&answer_micros), "{answer_micros}");
    }
}
