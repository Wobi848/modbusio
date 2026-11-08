//! Listening to a serial line without sending (concept section 5.2): is another master active,
//! at which settings, which units talk. Frames are found by their form and CRC, not by timing,
//! because USB adapters deliver bytes in chunks.

use std::collections::BTreeSet;
use std::time::Duration;

use serde::Serialize;
use tokio::io::AsyncReadExt;

use crate::endpoint::{BAUD_RATES, SerialSettings};
use crate::link::LinkError;
use crate::protocol::{pdu, rtu};

/// What the bytes heard at one setting hold.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Heard {
    /// Frames with a valid CRC.
    pub frames: usize,
    /// Bytes that belonged to no valid frame: noise, or the wrong speed or parity.
    pub stray_bytes: usize,
    pub units: BTreeSet<u8>,
}

/// Finds the valid RTU frames in `bytes`, requests and answers alike.
pub fn frames(bytes: &[u8]) -> Heard {
    let mut heard = Heard::default();
    let mut at = 0;
    while at < bytes.len() {
        match frame_at(&bytes[at..]) {
            Some(length) => {
                heard.frames += 1;
                heard.units.insert(bytes[at]);
                at += length;
            }
            None => {
                heard.stray_bytes += 1;
                at += 1;
            }
        }
    }
    heard
}

/// The length of a valid frame starting at the first byte, as a request or as an answer.
fn frame_at(bytes: &[u8]) -> Option<usize> {
    let body = bytes.get(1..)?;
    [pdu::request_length(body), pdu::response_length(body)]
        .into_iter()
        .filter_map(|length| length.ok().flatten())
        .map(|length| length + 3)
        .find(|&length| bytes.len() >= length && rtu::check(&bytes[..length]))
}

/// What listening at one setting heard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Trial {
    pub settings: String,
    #[serde(flatten)]
    pub heard: Heard,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Listened {
    pub trials: Vec<Trial>,
    /// The settings at which valid frames passed: another master is active there.
    pub active: Option<String>,
    /// Bytes arrived at no setting: the line is quiet.
    pub quiet: bool,
}

/// Listens to `port` at each usual setting for `each` (8E1 and 8N1 per speed), sending nothing.
pub async fn listen(port: &str, each: Duration) -> Result<Listened, LinkError> {
    let mut trials = Vec::new();
    for baud in BAUD_RATES {
        for (parity, stop_bits) in SerialSettings::frames().into_iter().take(2) {
            let settings = SerialSettings {
                port: port.into(),
                baud,
                parity,
                stop_bits,
            };
            let bytes = record(&settings, each).await?;
            let heard = frames(&bytes);
            let found = heard.frames >= 2;
            trials.push(Trial {
                settings: format!("{baud} {}", settings.frame_text()),
                heard,
            });
            if found {
                return Ok(conclude(trials));
            }
        }
    }
    Ok(conclude(trials))
}

fn conclude(trials: Vec<Trial>) -> Listened {
    let active = trials
        .iter()
        .find(|trial| trial.heard.frames >= 2)
        .map(|trial| trial.settings.clone());
    let quiet = trials
        .iter()
        .all(|trial| trial.heard.frames == 0 && trial.heard.stray_bytes == 0);
    Listened {
        trials,
        active,
        quiet,
    }
}

async fn record(settings: &SerialSettings, each: Duration) -> Result<Vec<u8>, LinkError> {
    let mut port = crate::serial::open(settings)?;
    let mut bytes = Vec::new();
    let deadline = tokio::time::Instant::now() + each;
    let mut chunk = [0u8; 512];
    loop {
        match tokio::time::timeout_at(deadline, port.read(&mut chunk)).await {
            Err(_) | Ok(Ok(0)) => break,
            Ok(Ok(read)) => bytes.extend_from_slice(&chunk[..read]),
            Ok(Err(error)) => return Err(LinkError::Io(error.to_string())),
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_and_answers_are_found_between_noise() {
        let request = rtu::frame(5, &[0x04, 0x00, 0x00, 0x00, 0x02]);
        let answer = rtu::frame(5, &[0x04, 0x04, 0x00, 0xD7, 0xFF, 0xDD]);
        let write = rtu::frame(2, &[0x10, 0x00, 0x01, 0x00, 0x01, 0x02, 0x0B, 0xB8]);
        let mut bytes = vec![0xFF, 0x00];
        bytes.extend(&request);
        bytes.extend(&answer);
        bytes.push(0x55);
        bytes.extend(&write);
        let heard = frames(&bytes);
        assert_eq!(heard.frames, 3);
        assert_eq!(heard.stray_bytes, 3);
        assert_eq!(heard.units.into_iter().collect::<Vec<_>>(), [2, 5]);
    }

    #[test]
    fn bytes_at_the_wrong_speed_make_no_frames() {
        let heard = frames(&[0x12, 0x9A, 0x00, 0xF3, 0x44, 0x81, 0x7E, 0x03, 0x22, 0x10]);
        assert_eq!(heard.frames, 0);
        assert_eq!(heard.stray_bytes, 10);
    }
}
