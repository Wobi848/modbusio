//! Capture files read back for the bus analyser (docs/concept-analyser.md, *Saving and opening*):
//! pcapng as `pcap.rs` writes it and as Wireshark saves it, and the older pcap format. RTU frames
//! are the packets of the user link types 147 to 162, which modbusio and serial capture tools use;
//! packets of other link types (Modbus TCP) are counted and left out.

use std::time::{Duration, SystemTime};

use crate::pcap::LINKTYPE_USER0;

/// The user link types, 147 to 162 (DLT_USER0 to DLT_USER15).
const USER_LINK_TYPES: std::ops::RangeInclusive<u16> = LINKTYPE_USER0..=LINKTYPE_USER0 + 15;

#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub time: SystemTime,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Opened {
    /// The RTU frames, in the order of the file.
    pub packets: Vec<Packet>,
    /// Packets of other link types, left out.
    pub skipped: usize,
}

/// Reads a pcapng or pcap file.
pub fn read(file: &[u8]) -> Result<Opened, String> {
    match file.get(..4) {
        Some([0x0A, 0x0D, 0x0D, 0x0A]) => pcapng(file),
        Some(magic) if Classic::of(magic).is_some() => classic(file),
        _ => Err("not a pcapng or pcap file".into()),
    }
}

fn number(bytes: &[u8], little: bool) -> u32 {
    let array: [u8; 4] = bytes.try_into().expect("four bytes");
    if little {
        u32::from_le_bytes(array)
    } else {
        u32::from_be_bytes(array)
    }
}

fn short(bytes: &[u8], little: bool) -> u16 {
    let array: [u8; 2] = bytes.try_into().expect("two bytes");
    if little {
        u16::from_le_bytes(array)
    } else {
        u16::from_be_bytes(array)
    }
}

const CUT: &str = "the file is cut off";

/// One interface of a pcapng section: its link type and how many time units make a second.
struct Interface {
    link_type: u16,
    units_per_second: u64,
}

fn pcapng(file: &[u8]) -> Result<Opened, String> {
    let mut opened = Opened::default();
    let mut little = true;
    let mut interfaces: Vec<Interface> = Vec::new();
    let mut at = 0;
    while at + 12 <= file.len() {
        let kind = number(&file[at..at + 4], little);
        if kind == 0x0A0D_0D0A {
            // a section header names the byte order of what follows it
            let magic = file.get(at + 8..at + 12).ok_or(CUT)?;
            little = match magic {
                [0x4D, 0x3C, 0x2B, 0x1A] => true,
                [0x1A, 0x2B, 0x3C, 0x4D] => false,
                _ => return Err("a pcapng section of unknown byte order".into()),
            };
            interfaces.clear();
        }
        let length = number(&file[at + 4..at + 8], little) as usize;
        if length < 12 || !length.is_multiple_of(4) {
            return Err("a pcapng block of wrong length".into());
        }
        let body = file.get(at + 8..at + length - 4).ok_or(CUT)?;
        match kind {
            1 if body.len() >= 8 => interfaces.push(Interface {
                link_type: short(&body[0..2], little),
                units_per_second: resolution(&body[8..], little),
            }),
            6 if body.len() >= 20 => {
                let interface = interfaces
                    .get(number(&body[0..4], little) as usize)
                    .ok_or("a packet of an interface the file does not name")?;
                let captured = number(&body[12..16], little) as usize;
                let data = body.get(20..20 + captured).ok_or(CUT)?;
                if !USER_LINK_TYPES.contains(&interface.link_type) {
                    opened.skipped += 1;
                } else {
                    let units = (u64::from(number(&body[4..8], little)) << 32)
                        | u64::from(number(&body[8..12], little));
                    opened.packets.push(Packet {
                        time: at_units(units, interface.units_per_second),
                        bytes: data.to_vec(),
                    });
                }
            }
            _ => {}
        }
        at += length;
    }
    Ok(opened)
}

/// The time resolution among an interface's options (`if_tsresol`, 9): microseconds by default.
fn resolution(options: &[u8], little: bool) -> u64 {
    let mut at = 0;
    while at + 4 <= options.len() {
        let code = short(&options[at..at + 2], little);
        let length = short(&options[at + 2..at + 4], little) as usize;
        if code == 0 {
            break;
        }
        if code == 9
            && length >= 1
            && let Some(&value) = options.get(at + 4)
        {
            return if value & 0x80 == 0 {
                10u64.saturating_pow(u32::from(value))
            } else {
                1u64 << (value & 0x7F).min(63)
            };
        }
        at += 4 + length.div_ceil(4) * 4;
    }
    1_000_000
}

fn at_units(units: u64, per_second: u64) -> SystemTime {
    let per_second = per_second.max(1);
    let seconds = units / per_second;
    let nanos = (units % per_second) as u128 * 1_000_000_000 / per_second as u128;
    SystemTime::UNIX_EPOCH + Duration::new(seconds, nanos as u32)
}

/// The older pcap format: byte order and whether fractions are micro- or nanoseconds.
struct Classic {
    little: bool,
    nanos: bool,
}

impl Classic {
    fn of(magic: &[u8]) -> Option<Classic> {
        let (little, nanos) = match magic {
            [0xD4, 0xC3, 0xB2, 0xA1] => (true, false),
            [0xA1, 0xB2, 0xC3, 0xD4] => (false, false),
            [0x4D, 0x3C, 0xB2, 0xA1] => (true, true),
            [0xA1, 0xB2, 0x3C, 0x4D] => (false, true),
            _ => return None,
        };
        Some(Classic { little, nanos })
    }
}

fn classic(file: &[u8]) -> Result<Opened, String> {
    let format = Classic::of(&file[..4]).expect("checked by read");
    let little = format.little;
    let header = file.get(..24).ok_or(CUT)?;
    let link_type = short(&header[20..22], little);
    let mut opened = Opened::default();
    let mut at = 24;
    while at + 16 <= file.len() {
        let seconds = u64::from(number(&file[at..at + 4], little));
        let fraction = u64::from(number(&file[at + 4..at + 8], little));
        let captured = number(&file[at + 8..at + 12], little) as usize;
        let data = file.get(at + 16..at + 16 + captured).ok_or(CUT)?;
        if USER_LINK_TYPES.contains(&link_type) {
            let nanos = if format.nanos {
                fraction
            } else {
                fraction * 1000
            };
            opened.packets.push(Packet {
                time: SystemTime::UNIX_EPOCH
                    + Duration::from_secs(seconds)
                    + Duration::from_nanos(nanos),
                bytes: data.to_vec(),
            });
        } else {
            opened.skipped += 1;
        }
        at += 16 + captured;
    }
    Ok(opened)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::{Direction, Frame};

    fn frame(rtu: bool, micros: u64, bytes: &[u8]) -> Frame {
        Frame {
            time: SystemTime::UNIX_EPOCH + Duration::from_micros(micros),
            direction: Direction::Sent,
            addresses: (!rtu).then(|| {
                (
                    "10.0.0.1:50000".parse().expect("address"),
                    "10.0.0.5:502".parse().expect("address"),
                )
            }),
            bytes: bytes.to_vec(),
            rtu,
        }
    }

    #[test]
    fn a_file_modbusio_wrote_is_read_back_with_its_rtu_frames_and_times() {
        let file = crate::pcap::write(&[
            frame(true, 1_500_000, &[1, 3, 0, 0, 0, 2, 0xC4, 0x0B]),
            frame(false, 1_600_000, &[0, 1, 0, 0, 0, 6, 1, 3, 0, 0, 0, 1]),
            frame(true, 1_700_123, &[1, 3, 4, 0, 1, 0, 2, 0x2A, 0x32]),
        ]);
        let opened = read(&file).expect("read");
        assert_eq!(opened.skipped, 1);
        assert_eq!(opened.packets.len(), 2);
        assert_eq!(opened.packets[0].bytes, [1, 3, 0, 0, 0, 2, 0xC4, 0x0B]);
        assert_eq!(
            opened.packets[1].time,
            SystemTime::UNIX_EPOCH + Duration::from_micros(1_700_123)
        );
    }

    #[test]
    fn a_classic_pcap_of_user_link_type_148_in_nanoseconds_is_read() {
        let mut file = vec![0x4D, 0x3C, 0xB2, 0xA1, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        file.extend(65535u32.to_le_bytes());
        file.extend(148u32.to_le_bytes());
        let packet = [5, 4, 0, 0, 0, 1, 0x30, 0x4E];
        file.extend(7u32.to_le_bytes());
        file.extend(250u32.to_le_bytes());
        file.extend((packet.len() as u32).to_le_bytes());
        file.extend((packet.len() as u32).to_le_bytes());
        file.extend(packet);
        let opened = read(&file).expect("read");
        assert_eq!(opened.packets[0].bytes, packet);
        assert_eq!(
            opened.packets[0].time,
            SystemTime::UNIX_EPOCH + Duration::new(7, 250)
        );
    }

    #[test]
    fn other_files_and_cut_ones_are_refused_in_words() {
        assert_eq!(
            read(b"PK\x03\x04 a zip"),
            Err("not a pcapng or pcap file".into())
        );
        let file = crate::pcap::write(&[frame(true, 0, &[1, 3, 0, 0, 0, 2, 0xC4, 0x0B])]);
        assert_eq!(read(&file[..file.len() - 6]), Err(CUT.into()));
    }
}
