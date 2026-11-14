//! The bus analyser (docs/concept-analyser.md): what a trunk's own master asks and what its units
//! answer, heard without sending. Only `port.rs` reads a port; the rest takes bytes with the time
//! they arrived.

pub mod exchanges;
pub mod frames;
pub mod port;
pub mod registers;
pub mod tables;
pub mod units;

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, SystemTime};

use serde::Serialize;

use crate::capture::{self, Direction, MOST_FRAMES};
use exchanges::{Exchange, Outcome, Pairer};
use frames::{Framer, Heard};
use registers::Registers;
use units::UnitStats;

/// Counts for the whole trunk.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Bus {
    pub frames: u64,
    /// Runs of bytes that made no frame.
    pub damaged: u64,
    /// Every byte heard, for the bus load.
    pub bytes: u64,
    pub broadcasts: u64,
}

/// What a frame was on the trunk, known once it is paired.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Request,
    Answer,
    /// Bytes that made no frame.
    Damaged,
}

/// A frame as kept for the view *Frames* and the capture file.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    /// When its first byte arrived, in microseconds since listening began.
    pub at: u64,
    pub role: Role,
    pub bytes: Vec<u8>,
}

impl Seen {
    /// The frame for a capture file (`pcap.rs`): requests as sent, the rest as received.
    pub fn capture(&self, started: SystemTime) -> capture::Frame {
        capture::Frame {
            time: started + Duration::from_micros(self.at),
            direction: if self.role == Role::Request {
                Direction::Sent
            } else {
                Direction::Received
            },
            addresses: None,
            bytes: self.bytes.clone(),
            rtu: true,
        }
    }
}

/// One item of a list kept up to a limit, with a number that counts on, so a page can ask for
/// what came after the last one it has.
#[derive(Clone, Debug, PartialEq)]
pub struct Numbered<T> {
    pub number: u64,
    pub item: T,
}

#[derive(Default)]
pub struct Analysis {
    framer: Framer,
    pairer: Pairer,
    pub bus: Bus,
    pub units: BTreeMap<u8, UnitStats>,
    pub registers: Registers,
    frames: VecDeque<Numbered<Seen>>,
    exchanges: VecDeque<Numbered<Exchange>>,
    next_frame: u64,
    next_exchange: u64,
}

impl Analysis {
    /// Bytes of one chunk, arrived `at` microseconds after listening began.
    pub fn push(&mut self, at: u64, bytes: &[u8]) {
        self.bus.bytes += bytes.len() as u64;
        let heard = self.framer.push(at, bytes);
        self.take(heard);
    }

    /// The line was quiet for a while: bytes waiting cannot become a frame any more.
    pub fn quiet(&mut self) {
        let heard = self.framer.quiet();
        self.take(heard);
    }

    /// Counts and lists start again; the numbers count on, so a page asking for what came after
    /// its last item gets the new ones.
    pub fn clear(&mut self) {
        *self = Analysis {
            next_frame: self.next_frame,
            next_exchange: self.next_exchange,
            ..Analysis::default()
        };
    }

    pub fn frames(&self) -> &VecDeque<Numbered<Seen>> {
        &self.frames
    }

    pub fn exchanges(&self) -> &VecDeque<Numbered<Exchange>> {
        &self.exchanges
    }

    fn take(&mut self, heard: Vec<Heard>) {
        for item in heard {
            let (seen, done) = match item {
                Heard::Frame(frame) => {
                    self.bus.frames += 1;
                    let done = self.pairer.frame(&frame);
                    // the frame is an answer when it completed an exchange as its answer
                    let answer = done
                        .last()
                        .is_some_and(|exchange| exchange.answer_bytes == frame.bytes);
                    let role = if answer { Role::Answer } else { Role::Request };
                    let seen = Seen {
                        at: frame.at,
                        role,
                        bytes: frame.bytes,
                    };
                    (seen, done)
                }
                // listening began in the middle of a frame: a piece, not a fault on the line
                Heard::Damaged(_) if self.bus.frames == 0 => continue,
                Heard::Damaged(damaged) => {
                    self.bus.damaged += 1;
                    let done = self.pairer.damaged(&damaged).into_iter().collect();
                    let seen = Seen {
                        at: damaged.at,
                        role: Role::Damaged,
                        bytes: damaged.bytes,
                    };
                    (seen, done)
                }
            };
            keep(&mut self.frames, &mut self.next_frame, seen);
            for mut exchange in done {
                self.count(&mut exchange);
                keep(&mut self.exchanges, &mut self.next_exchange, exchange);
            }
        }
    }

    fn count(&mut self, exchange: &mut Exchange) {
        if exchange.outcome == Outcome::Broadcast {
            self.bus.broadcasts += 1;
            return;
        }
        if let Some(asked) = exchange.asked.as_ref().filter(|asked| asked.write) {
            exchange.before =
                self.registers
                    .before(exchange.unit, asked.area, asked.address, asked.count);
        }
        self.registers.add(exchange);
        self.units.entry(exchange.unit).or_default().add(exchange);
    }
}

fn keep<T>(list: &mut VecDeque<Numbered<T>>, next: &mut u64, item: T) {
    if list.len() == MOST_FRAMES {
        list.pop_front();
    }
    list.push_back(Numbered {
        number: *next,
        item,
    });
    *next += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::rtu;

    #[test]
    fn a_poll_cycle_is_counted_per_unit() {
        let mut analysis = Analysis::default();
        let request = rtu::frame(5, &[0x04, 0, 0, 0, 1]);
        let answer = rtu::frame(5, &[0x04, 2, 0, 42]);
        let silent = rtu::frame(9, &[0x03, 0, 0, 0, 1]);
        for cycle in 0..3u64 {
            let start = cycle * 2_000_000;
            analysis.push(start, &request);
            analysis.push(start + 20_000, &answer);
            analysis.push(start + 500_000, &silent);
        }
        analysis.quiet();
        let answering = &analysis.units[&5];
        assert_eq!((answering.requests, answering.answers), (3, 3));
        assert_eq!(answering.cycle.summary().unwrap().median, 2_000_000);
        assert_eq!(answering.answer_times.summary().unwrap().max, 20_000);
        // the last request to unit 9 still waits: two of three are known to have no answer
        assert_eq!(analysis.units[&9].no_answer, 2);
        assert_eq!(analysis.bus.frames, 9);
        let roles: Vec<Role> = analysis
            .frames()
            .iter()
            .take(3)
            .map(|seen| seen.item.role)
            .collect();
        assert_eq!(roles, [Role::Request, Role::Answer, Role::Request]);
        assert_eq!(analysis.exchanges().back().unwrap().number, 4);
    }

    #[test]
    fn a_long_run_in_random_chunks_with_damaged_answers_counts_exactly() {
        let mut analysis = Analysis::default();
        let mut line = Vec::new();
        for cycle in 0..500u32 {
            let unit = (cycle % 4 + 1) as u8;
            line.extend(rtu::frame(unit, &[0x03, 0, 10, 0, 2]));
            let mut answer = rtu::frame(unit, &[0x03, 4, 0x42, cycle as u8, 0, 0]);
            if cycle % 10 == 3 {
                answer[3] ^= 0x5A;
            }
            line.extend(answer);
        }
        // a simple generator, so the cuts are the same on every run
        let mut seed = 7u32;
        let mut at = 0;
        let mut time = 0;
        while at < line.len() {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let size = (seed >> 16) as usize % 40 + 1;
            let end = (at + size).min(line.len());
            analysis.push(time, &line[at..end]);
            at = end;
            time += 1000;
        }
        analysis.quiet();
        let requests: u64 = analysis.units.values().map(|unit| unit.requests).sum();
        let answers: u64 = analysis.units.values().map(|unit| unit.answers).sum();
        let damaged: u64 = analysis.units.values().map(|unit| unit.damaged).sum();
        assert_eq!((requests, answers, damaged), (500, 450, 50));
        assert_eq!(analysis.bus.damaged, 50);
    }

    #[test]
    fn a_write_of_the_master_knows_the_value_read_before_it() {
        let mut analysis = Analysis::default();
        analysis.push(0, &rtu::frame(12, &[0x03, 0, 1, 0, 1]));
        analysis.push(1, &rtu::frame(12, &[0x03, 2, 0x10, 0x68]));
        let write = rtu::frame(12, &[0x06, 0, 1, 0x11, 0x94]);
        analysis.push(2, &write);
        analysis.push(3, &write);
        let written = &analysis.exchanges().back().unwrap().item;
        assert_eq!(written.outcome, Outcome::Written);
        assert_eq!(written.before, Some(vec![4200]));
        assert_eq!(written.asked.as_ref().unwrap().values, [4500]);
        let seen = analysis
            .registers
            .get(&(12, crate::protocol::Area::HoldingRegister, 1))
            .unwrap();
        assert_eq!((seen.reads, seen.writes, seen.last), (1, 1, Some(4500)));
    }

    #[test]
    fn the_piece_of_a_frame_heard_first_is_no_damage() {
        let mut analysis = Analysis::default();
        let answer = rtu::frame(5, &[0x04, 2, 0, 42]);
        analysis.push(0, &answer[3..]);
        analysis.push(1, &rtu::frame(5, &[0x04, 0, 0, 0, 1]));
        analysis.push(2, &[0x05, 0x04, 0x02]);
        analysis.quiet();
        assert_eq!(analysis.bus.damaged, 1);
        assert_eq!(analysis.frames().len(), 2);
    }

    #[test]
    fn the_lists_keep_the_newest_up_to_the_limit() {
        let mut analysis = Analysis::default();
        let request = rtu::frame(0, &[0x06, 0, 1, 0, 1]);
        let count = MOST_FRAMES + 5;
        for at in 0..count as u64 {
            analysis.push(at, &request);
        }
        assert_eq!(analysis.frames().len(), MOST_FRAMES);
        assert_eq!(analysis.frames().front().unwrap().number, 5);
        assert_eq!(analysis.bus.broadcasts, count as u64);
    }
}
