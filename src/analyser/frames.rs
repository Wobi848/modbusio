//! Frames found in the bytes heard on a trunk (docs/concept-analyser.md). USB adapters hand bytes
//! over in chunks, so frames are found by their form and CRC, not by the gaps between them (as
//! `listen.rs`). Requests and answers pass alike; which one a frame is follows later, from the
//! order (`exchanges.rs`).

use crate::protocol::{pdu, rtu};

/// The longest RTU frame: unit, 253 bytes of PDU, CRC (Serial Line 1.02, 2.5.1.1).
pub const LONGEST_FRAME: usize = 256;
/// Bytes kept while waiting for a frame to complete; beyond this the oldest are given up as
/// damaged, so noise cannot fill the memory.
const MOST_WAITING: usize = 2 * LONGEST_FRAME;

/// Functions whose length modbusio does not know but that a master may use: the other public
/// codes and the user-defined ranges (Application Protocol 1.1b3, 5 and 6). Their end is found by
/// searching for a matching CRC.
fn searched_by_crc(function: u8) -> bool {
    matches!(
        function,
        0x07 | 0x0B | 0x0C | 0x11 | 0x14 | 0x15 | 0x16 | 0x17 | 0x18 | 0x41..=0x48 | 0x64..=0x6E
    )
}

/// What was heard: a frame with a good CRC, or bytes that made none.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Frame(Frame),
    Damaged(Damaged),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    /// When its first byte arrived, in microseconds since listening began.
    pub at: u64,
    /// When its last byte arrived.
    pub end: u64,
    /// Unit, PDU and CRC.
    pub bytes: Vec<u8>,
    /// It has the form of a request.
    pub as_request: bool,
    /// It has the form of an answer. Both for 05, 06 and 08, whose answer repeats the request;
    /// neither for a function found by its CRC alone.
    pub as_answer: bool,
}

impl Frame {
    pub fn unit(&self) -> u8 {
        self.bytes[0]
    }

    pub fn function(&self) -> u8 {
        self.bytes[1]
    }

    pub fn pdu(&self) -> &[u8] {
        &self.bytes[1..self.bytes.len() - 2]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Damaged {
    pub at: u64,
    pub end: u64,
    pub bytes: Vec<u8>,
}

/// Finds frames in bytes as they arrive.
#[derive(Default)]
pub struct Framer {
    waiting: Vec<u8>,
    /// When each waiting byte arrived.
    arrived: Vec<u64>,
}

enum Match {
    Frame {
        length: usize,
        as_request: bool,
        as_answer: bool,
    },
    /// More bytes may complete a frame; `searching` when only its CRC can tell where it ends.
    NeedMore {
        searching: bool,
    },
    Nothing,
}

impl Framer {
    /// Takes the bytes of one chunk, arrived at `at`, and gives the frames they complete.
    pub fn push(&mut self, at: u64, bytes: &[u8]) -> Vec<Heard> {
        self.waiting.extend_from_slice(bytes);
        self.arrived.extend(std::iter::repeat_n(at, bytes.len()));
        self.take(false)
    }

    /// The line went quiet: what waits cannot grow into a frame any more.
    pub fn quiet(&mut self) -> Vec<Heard> {
        self.take(true)
    }

    fn take(&mut self, quiet: bool) -> Vec<Heard> {
        let mut heard = Vec::new();
        while !self.waiting.is_empty() {
            match match_at(&self.waiting, quiet) {
                Match::Frame {
                    length,
                    as_request,
                    as_answer,
                } => {
                    let (bytes, at, end) = self.cut(length);
                    heard.push(Heard::Frame(Frame {
                        at,
                        end,
                        bytes,
                        as_request,
                        as_answer,
                    }));
                }
                // a noise byte before a frame (line turned around) looks like the start of a
                // function of unknown length; a frame of known form after it ends the wait
                Match::NeedMore { searching }
                    if self.waiting.len() <= MOST_WAITING
                        && !(searching && self.known_frame_later(quiet)) =>
                {
                    break;
                }
                Match::NeedMore { .. } | Match::Nothing => {
                    let next = (1..self.waiting.len()).find(|&at| {
                        matches!(match_at(&self.waiting[at..], quiet), Match::Frame { .. })
                    });
                    let damaged = match next {
                        Some(next) => next,
                        None if quiet => self.waiting.len(),
                        None if self.waiting.len() > MOST_WAITING => {
                            self.waiting.len() - LONGEST_FRAME
                        }
                        // a frame may still start in what waits
                        None => break,
                    };
                    let (bytes, at, end) = self.cut(damaged);
                    heard.push(Heard::Damaged(Damaged { at, end, bytes }));
                }
            }
        }
        heard
    }

    fn known_frame_later(&self, quiet: bool) -> bool {
        (1..self.waiting.len()).any(|at| {
            matches!(
                match_at(&self.waiting[at..], quiet),
                Match::Frame { as_request, as_answer, .. } if as_request || as_answer
            )
        })
    }

    fn cut(&mut self, length: usize) -> (Vec<u8>, u64, u64) {
        let bytes: Vec<u8> = self.waiting.drain(..length).collect();
        let times: Vec<u64> = self.arrived.drain(..length).collect();
        (bytes, times[0], times[length - 1])
    }
}

/// Whether a frame starts at the first byte of `bytes`.
fn match_at(bytes: &[u8], quiet: bool) -> Match {
    let need_more = if quiet {
        Match::Nothing
    } else {
        Match::NeedMore { searching: false }
    };
    let Some(body) = bytes.get(1..).filter(|body| !body.is_empty()) else {
        return need_more;
    };
    let request = pdu::request_length(body);
    let answer = pdu::response_length(body);
    if request.is_err() && answer.is_err() {
        return if searched_by_crc(body[0]) {
            by_crc(bytes, quiet)
        } else {
            Match::Nothing
        };
    }
    let complete = |length: &Result<Option<usize>, String>| match length {
        Ok(Some(length)) => {
            let total = length + 3;
            (bytes.len() >= total && rtu::check(&bytes[..total])).then_some(total)
        }
        _ => None,
    };
    let waits = |length: &Result<Option<usize>, String>| match length {
        Ok(None) => true,
        Ok(Some(length)) => bytes.len() < length + 3,
        Err(_) => false,
    };
    match (complete(&request), complete(&answer)) {
        (Some(as_request), Some(as_answer)) => Match::Frame {
            length: as_request.min(as_answer),
            as_request: as_request <= as_answer,
            as_answer: as_answer <= as_request,
        },
        (Some(length), None) => Match::Frame {
            length,
            as_request: true,
            as_answer: false,
        },
        (None, Some(length)) => Match::Frame {
            length,
            as_request: false,
            as_answer: true,
        },
        (None, None) if waits(&request) || waits(&answer) => need_more,
        (None, None) => Match::Nothing,
    }
}

/// The shortest prefix whose last two bytes are the CRC of the rest.
fn by_crc(bytes: &[u8], quiet: bool) -> Match {
    let mut crc = rtu::CRC_START;
    for (at, &byte) in bytes.iter().enumerate().take(LONGEST_FRAME - 2) {
        crc = rtu::crc_step(crc, byte);
        let total = at + 3;
        if at >= 1 && bytes.len() >= total && crc.to_le_bytes() == [bytes[at + 1], bytes[at + 2]] {
            return Match::Frame {
                length: total,
                as_request: false,
                as_answer: false,
            };
        }
    }
    if quiet || bytes.len() >= LONGEST_FRAME {
        Match::Nothing
    } else {
        Match::NeedMore { searching: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(heard: &[Heard]) -> Vec<Vec<u8>> {
        heard
            .iter()
            .filter_map(|item| match item {
                Heard::Frame(frame) => Some(frame.bytes.clone()),
                Heard::Damaged(_) => None,
            })
            .collect()
    }

    fn damaged(heard: &[Heard]) -> Vec<Vec<u8>> {
        heard
            .iter()
            .filter_map(|item| match item {
                Heard::Damaged(damaged) => Some(damaged.bytes.clone()),
                Heard::Frame(_) => None,
            })
            .collect()
    }

    fn read_request() -> Vec<u8> {
        rtu::frame(7, &[0x03, 0x00, 0x64, 0x00, 0x02])
    }

    fn read_answer() -> Vec<u8> {
        rtu::frame(7, &[0x03, 0x04, 0x42, 0x1F, 0x00, 0x00])
    }

    #[test]
    fn a_request_and_its_answer_are_found_however_the_bytes_are_cut() {
        let line: Vec<u8> = [read_request(), read_answer()].concat();
        for cut in 0..=line.len() {
            let mut framer = Framer::default();
            let mut heard = framer.push(10, &line[..cut]);
            heard.extend(framer.push(20, &line[cut..]));
            heard.extend(framer.quiet());
            assert_eq!(
                frames(&heard),
                [read_request(), read_answer()],
                "cut at {cut}"
            );
            assert!(damaged(&heard).is_empty(), "cut at {cut}");
        }
    }

    #[test]
    fn byte_by_byte_works_too_and_the_times_are_those_of_the_first_and_last_byte() {
        let line: Vec<u8> = [read_request(), read_answer()].concat();
        let mut framer = Framer::default();
        let heard: Vec<Heard> = line
            .iter()
            .enumerate()
            .flat_map(|(at, byte)| framer.push(at as u64, &[*byte]))
            .collect();
        let Heard::Frame(answer) = &heard[1] else {
            panic!("answer expected");
        };
        assert_eq!((answer.at, answer.end), (8, 16));
        assert!(answer.as_answer && !answer.as_request);
    }

    #[test]
    fn a_write_and_its_echo_have_both_forms() {
        let write = rtu::frame(12, &[0x06, 0x00, 0x01, 0x11, 0x94]);
        let heard = Framer::default().push(0, &[write.clone(), write.clone()].concat());
        assert_eq!(frames(&heard), [write.clone(), write]);
        let Heard::Frame(frame) = &heard[0] else {
            panic!("frame expected");
        };
        assert!(frame.as_request && frame.as_answer);
    }

    #[test]
    fn a_write_of_many_registers_and_its_short_answer() {
        let request = rtu::frame(
            3,
            &[0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02],
        );
        let answer = rtu::frame(3, &[0x10, 0x00, 0x01, 0x00, 0x02]);
        let heard = Framer::default().push(0, &[request.clone(), answer.clone()].concat());
        assert_eq!(frames(&heard), [request, answer]);
    }

    #[test]
    fn an_exception_answer_is_a_frame() {
        let exception = rtu::frame(7, &[0x83, 0x02]);
        let heard = Framer::default().push(0, &[read_request(), exception.clone()].concat());
        assert_eq!(frames(&heard), [read_request(), exception]);
    }

    #[test]
    fn a_damaged_answer_is_given_as_damaged_and_the_next_frame_still_found() {
        let mut broken = read_answer();
        broken[4] ^= 0x10;
        let mut framer = Framer::default();
        let mut heard = framer.push(
            0,
            &[read_request(), broken.clone(), read_request()].concat(),
        );
        heard.extend(framer.quiet());
        assert_eq!(frames(&heard), [read_request(), read_request()]);
        assert_eq!(damaged(&heard), [broken]);
    }

    #[test]
    fn damaged_bytes_at_the_end_wait_for_quiet() {
        let mut framer = Framer::default();
        let heard = framer.push(0, &[read_request(), vec![0x07, 0x03, 0x04, 0x42]].concat());
        assert_eq!(frames(&heard), [read_request()]);
        assert!(damaged(&heard).is_empty());
        assert_eq!(damaged(&framer.quiet()), [vec![0x07, 0x03, 0x04, 0x42]]);
    }

    #[test]
    fn a_noise_byte_before_an_answer_does_not_hold_it_back() {
        for noise in [0x00, 0xFF] {
            let heard =
                Framer::default().push(0, &[read_request(), vec![noise], read_answer()].concat());
            assert_eq!(
                frames(&heard),
                [read_request(), read_answer()],
                "noise {noise:02X}"
            );
            assert_eq!(damaged(&heard), [vec![noise]]);
        }
    }

    #[test]
    fn a_function_of_unknown_length_is_found_by_its_crc() {
        // 0x17 read/write multiple registers and a vendor function 0x41
        let read_write = rtu::frame(
            1,
            &[0x17, 0, 3, 0, 6, 0, 14, 0, 3, 6, 0, 0xFF, 0, 0xFF, 0, 0xFF],
        );
        let vendor = rtu::frame(
            1,
            &[
                0x41, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            ],
        );
        let mut framer = Framer::default();
        let mut heard = framer.push(0, &[read_write.clone(), vendor.clone()].concat());
        heard.extend(framer.quiet());
        assert_eq!(frames(&heard), [read_write, vendor]);
        let Heard::Frame(frame) = &heard[0] else {
            panic!("frame expected");
        };
        assert!(!frame.as_request && !frame.as_answer);
    }

    #[test]
    fn noise_never_fills_the_memory() {
        let mut framer = Framer::default();
        let noise: Vec<u8> = (0..10_000u32)
            .map(|n| (n * 37 % 251) as u8 | 0x80)
            .collect();
        let heard = framer.push(0, &noise);
        assert!(framer.waiting.len() <= MOST_WAITING);
        let given: usize = damaged(&heard).iter().map(Vec::len).sum();
        assert_eq!(given + framer.waiting.len(), noise.len());
    }
}
