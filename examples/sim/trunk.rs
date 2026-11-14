//! A whole trunk played as bytes: a master's requests and its units' answers, with faults
//! (`simulated_trunk.rs` describes them). Frames are built here with an own CRC, apart from
//! modbusio's code, so the analyser's tests do not check modbusio against itself.

use std::time::Duration;

use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct Options {
    /// No faults: every unit answers in time, nothing damaged.
    pub clean: bool,
    /// Another master asks unit 1 in between.
    pub second_master: bool,
    pub cycle: Duration,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            clean: false,
            second_master: false,
            cycle: Duration::from_millis(2000),
        }
    }
}

/// Writes the trunk's traffic to `line`, cycle after cycle, until writing fails.
pub async fn play(mut line: impl AsyncWrite + Unpin, options: Options) -> std::io::Result<()> {
    let mut cycle = 0u32;
    loop {
        let started = tokio::time::Instant::now();
        for step in cycle_steps(cycle, &options) {
            line.write_all(&step.request).await?;
            tokio::time::sleep(step.wait).await;
            if let Some(answer) = step.answer {
                line.write_all(&answer).await?;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        tokio::time::sleep_until(started + options.cycle).await;
        cycle = cycle.wrapping_add(1);
    }
}

fn crc(bytes: &[u8]) -> [u8; 2] {
    let mut crc: u16 = 0xFFFF;
    for &byte in bytes {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            let low = crc & 1;
            crc >>= 1;
            if low == 1 {
                crc ^= 0xA001;
            }
        }
    }
    crc.to_le_bytes()
}

fn frame(unit: u8, pdu: &[u8]) -> Vec<u8> {
    let mut frame = vec![unit];
    frame.extend_from_slice(pdu);
    let check = crc(&frame);
    frame.extend(check);
    frame
}

fn read(unit: u8, function: u8, address: u16, count: u16) -> Vec<u8> {
    let [address_high, address_low] = address.to_be_bytes();
    let [count_high, count_low] = count.to_be_bytes();
    frame(
        unit,
        &[function, address_high, address_low, count_high, count_low],
    )
}

fn registers(unit: u8, function: u8, values: &[u16]) -> Vec<u8> {
    let mut pdu = vec![function, (values.len() * 2) as u8];
    for value in values {
        pdu.extend(value.to_be_bytes());
    }
    frame(unit, &pdu)
}

fn write_register(unit: u8, address: u16, value: u16) -> Vec<u8> {
    let [address_high, address_low] = address.to_be_bytes();
    let [value_high, value_low] = value.to_be_bytes();
    frame(
        unit,
        &[0x06, address_high, address_low, value_high, value_low],
    )
}

/// A float as two registers, high word first (ABCD).
fn float(value: f32) -> [u16; 2] {
    let bits = value.to_bits();
    [(bits >> 16) as u16, bits as u16]
}

/// One request of the master and what the trunk answers.
struct Step {
    request: Vec<u8>,
    answer: Option<Vec<u8>>,
    /// How long the unit takes to answer, or the master waits for an answer that does not come.
    wait: Duration,
}

fn cycle_steps(cycle: u32, options: &Options) -> Vec<Step> {
    let millis = Duration::from_millis;
    let energy = 1230.0 + cycle as f32 * 0.1;
    let [energy_high, energy_low] = float(energy);
    let [volume_high, volume_low] = float(88.21);
    let [flow_high, flow_low] = float(63.2 + (cycle % 7) as f32 * 0.1);
    let [return_high, return_low] = float(41.0);
    let mut room = registers(5, 0x04, &[214 + (cycle % 3) as u16, 220, 1, 0]);
    if !options.clean && cycle % 8 == 7 {
        room[4] ^= 0x5A;
    }
    let drive_missing = !options.clean && cycle % 3 == 2;
    let mut steps = vec![
        Step {
            request: read(1, 0x03, 0, 8),
            answer: Some(registers(
                1,
                0x03,
                &[
                    energy_high,
                    energy_low,
                    0,
                    0,
                    volume_high,
                    volume_low,
                    flow_high,
                    flow_low,
                ],
            )),
            wait: millis(19),
        },
        Step {
            request: read(5, 0x04, 0, 4),
            answer: Some(room),
            wait: millis(25),
        },
        Step {
            request: read(12, 0x03, 100, 2),
            answer: (!drive_missing).then(|| registers(12, 0x03, &[4200, 0])),
            wait: if drive_missing {
                millis(400)
            } else {
                millis(40 + u64::from(cycle % 6) * 50)
            },
        },
        Step {
            request: read(7, 0x03, 0, 10),
            answer: Some(registers(
                7,
                0x03,
                &[1, 2, 3, 4, 5, 6, 7, 8, return_high, return_low],
            )),
            wait: millis(20),
        },
        Step {
            request: read(7, 0x03, 100, 21),
            answer: Some(frame(7, &[0x83, 0x02])),
            wait: millis(21),
        },
        Step {
            request: read(9, 0x03, 0, 1),
            answer: None,
            wait: millis(400),
        },
    ];
    if cycle % 10 == 9 {
        let write = write_register(12, 1, if cycle % 20 == 9 { 4500 } else { 4200 });
        steps.push(Step {
            request: write.clone(),
            answer: Some(write),
            wait: millis(44),
        });
    }
    if cycle % 20 == 19 {
        steps.push(Step {
            request: write_register(0, 50, 1),
            answer: None,
            wait: millis(100),
        });
    }
    if options.second_master {
        steps.insert(
            1,
            Step {
                request: read(1, 0x03, 40, 2),
                answer: Some(registers(1, 0x03, &[0, 1])),
                wait: millis(19),
            },
        );
    }
    steps
}
