//! Requests and answers paired (docs/concept-analyser.md). An RTU trunk has one master, which waits
//! for an answer before it asks again; so the frame after a request is its answer when it comes
//! from the unit asked with the same function, and a new request means the old one got none.

use serde::Serialize;

use super::frames::{Damaged, Frame};
use crate::protocol::pdu::{self, Area, PduError, Request, Response};

/// What a request asked for, for the views: area, range, and whether it writes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Asked {
    pub area: Area,
    pub address: u16,
    pub count: u16,
    pub write: bool,
}

impl Asked {
    fn of(request: &Request) -> Option<Asked> {
        let (area, address, count, write) = match request {
            Request::Read {
                area,
                address,
                count,
            } => (*area, *address, *count, false),
            Request::WriteCoil { address, .. } => (Area::Coil, *address, 1, true),
            Request::WriteRegister { address, .. } => (Area::HoldingRegister, *address, 1, true),
            Request::WriteCoils { address, values } => {
                (Area::Coil, *address, values.len() as u16, true)
            }
            Request::WriteRegisters { address, values } => {
                (Area::HoldingRegister, *address, values.len() as u16, true)
            }
            Request::Identify { .. } | Request::Echo { .. } => return None,
        };
        Some(Asked {
            area,
            address,
            count,
            write,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Exchange {
    /// When the request's first byte arrived, in microseconds since listening began.
    pub at: u64,
    pub unit: u8,
    pub function: u8,
    /// The request decoded; `None` for a function modbusio does not decode, or an answer heard
    /// without its request.
    #[serde(skip)]
    pub request: Option<Request>,
    pub asked: Option<Asked>,
    pub outcome: Outcome,
    /// From the request's last byte to the answer's first, as the adapter delivered them.
    pub answer_micros: Option<u64>,
    /// The frames as heard, unit to CRC; empty where none was heard.
    #[serde(skip)]
    pub request_bytes: Vec<u8>,
    #[serde(skip)]
    pub answer_bytes: Vec<u8>,
}

impl Exchange {
    /// The request's PDU, without unit and CRC.
    pub fn request_pdu(&self) -> Option<&[u8]> {
        let length = self.request_bytes.len();
        (length >= 4).then(|| &self.request_bytes[1..length - 2])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Registers {
        values: Vec<u16>,
    },
    Bits {
        values: Vec<bool>,
    },
    Written,
    Exception {
        code: u8,
    },
    /// An answer modbusio does not decode further: identification, echo, another function.
    Answered,
    /// The answer does not fit the request.
    Malformed {
        reason: String,
    },
    /// The answer arrived damaged.
    Damaged,
    /// The master asked again before any answer came.
    NoAnswer,
    /// Unit 0: every unit takes it, none answers (Serial Line 1.02, 2.1).
    Broadcast,
    /// An answer without a request heard before it: listening began in between, or another
    /// master asked.
    AnswerOnly,
}

impl Outcome {
    /// The master did not get what it asked for.
    pub fn failed(&self) -> bool {
        matches!(
            self,
            Outcome::Exception { .. }
                | Outcome::Malformed { .. }
                | Outcome::Damaged
                | Outcome::NoAnswer
        )
    }
}

/// The request waiting for its answer.
struct Waiting {
    frame: Frame,
    request: Option<Request>,
}

#[derive(Default)]
pub struct Pairer {
    waiting: Option<Waiting>,
}

impl Pairer {
    /// Takes the next frame and gives the exchanges it completes.
    pub fn frame(&mut self, frame: &Frame) -> Vec<Exchange> {
        if let Some(waiting) = self.waiting.take() {
            if answers(&waiting.frame, frame) {
                return vec![answered(waiting, frame)];
            }
            let mut done = vec![finished(waiting, Outcome::NoAnswer)];
            done.extend(self.request_or_answer(frame));
            return done;
        }
        self.request_or_answer(frame)
    }

    /// Damaged bytes while an answer is due are taken as that answer.
    pub fn damaged(&mut self, damaged: &Damaged) -> Option<Exchange> {
        let waiting = self.waiting.take()?;
        let end = waiting.frame.end;
        let mut exchange = finished(waiting, Outcome::Damaged);
        exchange.answer_micros = Some(damaged.at.saturating_sub(end));
        exchange.answer_bytes = damaged.bytes.clone();
        Some(exchange)
    }

    fn request_or_answer(&mut self, frame: &Frame) -> Vec<Exchange> {
        let request = frame.as_request || !frame.as_answer;
        if !request {
            return vec![Exchange {
                at: frame.at,
                unit: frame.unit(),
                function: frame.function(),
                request: None,
                asked: None,
                outcome: Outcome::AnswerOnly,
                answer_micros: None,
                request_bytes: Vec::new(),
                answer_bytes: frame.bytes.clone(),
            }];
        }
        let waiting = Waiting {
            frame: frame.clone(),
            request: pdu::decode_request(frame.pdu()),
        };
        if frame.unit() == 0 {
            return vec![finished(waiting, Outcome::Broadcast)];
        }
        self.waiting = Some(waiting);
        Vec::new()
    }
}

/// `frame` is the answer to `request`: same unit, same function or its exception, answer form.
fn answers(request: &Frame, frame: &Frame) -> bool {
    let function = frame.function();
    frame.unit() == request.unit()
        && (function == request.function() || function == request.function() | 0x80)
        && (frame.as_answer || function & 0x80 != 0 || !frame.as_request)
}

fn answered(waiting: Waiting, answer: &Frame) -> Exchange {
    let outcome = match &waiting.request {
        Some(request) => match pdu::decode_response(request, answer.pdu()) {
            Ok(Response::Registers(values)) => Outcome::Registers { values },
            Ok(Response::Bits(values)) => Outcome::Bits { values },
            Ok(Response::Written) => Outcome::Written,
            Ok(Response::Identity(_) | Response::Echo(_)) => Outcome::Answered,
            Err(PduError::Exception(exception)) => Outcome::Exception { code: exception.0 },
            Err(PduError::Malformed(reason)) => Outcome::Malformed { reason },
        },
        None if answer.function() & 0x80 != 0 => Outcome::Exception {
            code: answer.pdu().get(1).copied().unwrap_or(0),
        },
        None => Outcome::Answered,
    };
    let end = waiting.frame.end;
    let mut exchange = finished(waiting, outcome);
    exchange.answer_micros = Some(answer.at.saturating_sub(end));
    exchange.answer_bytes = answer.bytes.clone();
    exchange
}

fn finished(waiting: Waiting, outcome: Outcome) -> Exchange {
    Exchange {
        at: waiting.frame.at,
        unit: waiting.frame.unit(),
        function: waiting.frame.function(),
        asked: waiting.request.as_ref().and_then(Asked::of),
        request: waiting.request,
        outcome,
        answer_micros: None,
        request_bytes: waiting.frame.bytes,
        answer_bytes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::frames::{Framer, Heard};
    use crate::protocol::rtu;

    /// Plays frames, one chunk each 10 ms, and pairs them.
    fn play(frames: &[Vec<u8>]) -> Vec<Exchange> {
        let mut framer = Framer::default();
        let mut pairer = Pairer::default();
        let mut exchanges = Vec::new();
        for (index, bytes) in frames.iter().enumerate() {
            for heard in framer.push(index as u64 * 10_000, bytes) {
                match heard {
                    Heard::Frame(frame) => exchanges.extend(pairer.frame(&frame)),
                    Heard::Damaged(damaged) => exchanges.extend(pairer.damaged(&damaged)),
                }
            }
        }
        exchanges
    }

    fn read(unit: u8, address: u16, count: u16) -> Vec<u8> {
        rtu::frame(
            unit,
            &[0x03, (address >> 8) as u8, address as u8, 0, count as u8],
        )
    }

    fn registers(unit: u8, values: &[u16]) -> Vec<u8> {
        let mut pdu = vec![0x03, (values.len() * 2) as u8];
        for value in values {
            pdu.extend(value.to_be_bytes());
        }
        rtu::frame(unit, &pdu)
    }

    #[test]
    fn a_read_and_its_answer_make_one_exchange_with_the_values() {
        let exchanges = play(&[read(1, 100, 2), registers(1, &[0x421F, 0])]);
        assert_eq!(exchanges.len(), 1);
        assert_eq!(exchanges[0].unit, 1);
        assert_eq!(
            exchanges[0].outcome,
            Outcome::Registers {
                values: vec![0x421F, 0]
            }
        );
        assert_eq!(exchanges[0].answer_micros, Some(10_000));
        assert_eq!(
            exchanges[0].asked,
            Some(Asked {
                area: Area::HoldingRegister,
                address: 100,
                count: 2,
                write: false
            })
        );
    }

    #[test]
    fn a_new_request_means_the_one_before_got_no_answer() {
        let exchanges = play(&[read(12, 100, 2), read(1, 0, 1), registers(1, &[7])]);
        assert_eq!(exchanges[0].unit, 12);
        assert_eq!(exchanges[0].outcome, Outcome::NoAnswer);
        assert_eq!(exchanges[1].outcome, Outcome::Registers { values: vec![7] });
    }

    #[test]
    fn the_same_request_asked_again_is_a_retry_not_an_answer() {
        let exchanges = play(&[read(5, 0, 4), read(5, 0, 4), registers(5, &[1, 2, 3, 4])]);
        assert_eq!(exchanges[0].outcome, Outcome::NoAnswer);
        assert!(matches!(exchanges[1].outcome, Outcome::Registers { .. }));
    }

    #[test]
    fn an_exception_is_paired_with_its_code() {
        let exchanges = play(&[read(7, 100, 21), rtu::frame(7, &[0x83, 0x02])]);
        assert_eq!(exchanges[0].outcome, Outcome::Exception { code: 2 });
    }

    #[test]
    fn a_write_and_its_echo_are_one_written_exchange() {
        let write = rtu::frame(12, &[0x06, 0x00, 0x01, 0x11, 0x94]);
        let exchanges = play(&[write.clone(), write]);
        assert_eq!(exchanges.len(), 1);
        assert_eq!(exchanges[0].outcome, Outcome::Written);
        assert_eq!(
            exchanges[0].request,
            Some(Request::WriteRegister {
                address: 1,
                value: 4500
            })
        );
    }

    #[test]
    fn a_broadcast_expects_no_answer() {
        let exchanges = play(&[
            rtu::frame(0, &[0x06, 0x00, 0x01, 0x00, 0x05]),
            read(1, 0, 1),
        ]);
        assert_eq!(exchanges[0].outcome, Outcome::Broadcast);
        assert_eq!(exchanges.len(), 1);
    }

    #[test]
    fn a_damaged_answer_ends_the_exchange_as_damaged() {
        let mut broken = registers(5, &[1, 2]);
        broken[3] ^= 0xFF;
        let exchanges = play(&[read(5, 0, 2), broken, read(1, 0, 1), registers(1, &[3])]);
        assert_eq!(exchanges[0].unit, 5);
        assert_eq!(exchanges[0].outcome, Outcome::Damaged);
        assert_eq!(exchanges[1].outcome, Outcome::Registers { values: vec![3] });
    }

    #[test]
    fn an_answer_heard_without_its_request_is_named_so() {
        let exchanges = play(&[registers(1, &[3]), read(1, 0, 1), registers(1, &[3])]);
        assert_eq!(exchanges[0].outcome, Outcome::AnswerOnly);
        assert!(matches!(exchanges[1].outcome, Outcome::Registers { .. }));
    }

    #[test]
    fn a_function_found_by_crc_is_paired_by_unit_and_function() {
        let request = rtu::frame(4, &[0x41, 1, 2, 3]);
        let answer = rtu::frame(4, &[0x41, 9, 9, 9, 9, 9, 9]);
        let exchanges = play(&[request, answer, read(1, 0, 1)]);
        assert_eq!(exchanges[0].function, 0x41);
        assert_eq!(exchanges[0].outcome, Outcome::Answered);
    }
}
