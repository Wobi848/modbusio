//! What the master uses, register by register (docs/concept-analyser.md, view *Registers*): how
//! often each address is read and written, its last value, how often it changed, min and max, and
//! its recent changes for a chart. Coils and discrete inputs are kept as 0 and 1.

use std::collections::{BTreeMap, VecDeque};

use super::exchanges::{Exchange, Outcome};
use crate::protocol::Area;

/// Changes kept per register for the chart.
pub const HISTORY: usize = 500;
/// Registers kept at most, so a master sweeping the whole map cannot fill the memory.
pub const MOST_REGISTERS: usize = 20_000;

/// A register by its unit, area and address.
pub type Place = (u8, Area, u16);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub reads: u64,
    pub writes: u64,
    pub last: Option<u16>,
    pub changes: u64,
    pub min: Option<u16>,
    pub max: Option<u16>,
    /// When it was last read or written, in microseconds since listening began.
    pub last_at: u64,
    /// The first value and every change after it, with its time.
    pub history: VecDeque<(u64, u16)>,
}

impl Seen {
    fn take(&mut self, at: u64, value: u16) {
        if self.last.is_some_and(|last| last != value) {
            self.changes += 1;
        }
        if self.last != Some(value) {
            if self.history.len() == HISTORY {
                self.history.pop_front();
            }
            self.history.push_back((at, value));
        }
        self.last = Some(value);
        self.min = Some(self.min.map_or(value, |min| min.min(value)));
        self.max = Some(self.max.map_or(value, |max| max.max(value)));
        self.last_at = at;
    }
}

#[derive(Default)]
pub struct Registers {
    seen: BTreeMap<Place, Seen>,
}

impl Registers {
    /// The values last seen at `count` addresses from `address`, where every one was seen.
    pub fn before(&self, unit: u8, area: Area, address: u16, count: u16) -> Option<Vec<u16>> {
        (0..count)
            .map(|offset| {
                let place = (unit, area, address.checked_add(offset)?);
                self.seen.get(&place)?.last
            })
            .collect()
    }

    /// Takes what a finished exchange read or wrote.
    pub fn add(&mut self, exchange: &Exchange) {
        let Some(asked) = &exchange.asked else {
            return;
        };
        let values: Vec<u16> = match (&exchange.outcome, asked.write) {
            (Outcome::Registers { values }, false) => values.clone(),
            (Outcome::Bits { values }, false) => values.iter().map(|on| u16::from(*on)).collect(),
            (Outcome::Written, true) => asked.values.clone(),
            _ => return,
        };
        for (offset, value) in values.into_iter().enumerate() {
            let Some(address) = asked.address.checked_add(offset as u16) else {
                break;
            };
            let place = (exchange.unit, asked.area, address);
            if !self.seen.contains_key(&place) && self.seen.len() >= MOST_REGISTERS {
                continue;
            }
            let seen = self.seen.entry(place).or_default();
            if asked.write {
                seen.writes += 1;
            } else {
                seen.reads += 1;
            }
            seen.take(exchange.at, value);
        }
    }

    /// The registers of one unit, in the order of area and address.
    pub fn of_unit(&self, unit: u8) -> impl Iterator<Item = (&Place, &Seen)> {
        self.seen
            .range((unit, Area::Coil, 0)..=(unit, Area::HoldingRegister, u16::MAX))
    }

    pub fn get(&self, place: &Place) -> Option<&Seen> {
        self.seen.get(place)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::exchanges::Asked;

    fn exchange(at: u64, asked: Asked, outcome: Outcome) -> Exchange {
        Exchange {
            at,
            unit: 12,
            function: 3,
            request: None,
            asked: Some(asked),
            outcome,
            answer_micros: None,
            before: None,
            request_bytes: Vec::new(),
            answer_bytes: Vec::new(),
        }
    }

    fn read(address: u16, count: u16) -> Asked {
        Asked {
            area: Area::HoldingRegister,
            address,
            count,
            write: false,
            values: Vec::new(),
        }
    }

    #[test]
    fn reads_count_values_changes_and_keep_the_changes_with_their_time() {
        let mut registers = Registers::default();
        for (at, value) in [(0, 4200), (1, 4200), (2, 4500), (3, 4200)] {
            registers.add(&exchange(
                at,
                read(1, 2),
                Outcome::Registers {
                    values: vec![value, 7],
                },
            ));
        }
        let seen = registers.get(&(12, Area::HoldingRegister, 1)).unwrap();
        assert_eq!(
            (seen.reads, seen.changes, seen.min, seen.max),
            (4, 2, Some(4200), Some(4500))
        );
        assert_eq!(
            Vec::from(seen.history.clone()),
            [(0, 4200), (2, 4500), (3, 4200)]
        );
        assert_eq!(
            registers
                .get(&(12, Area::HoldingRegister, 2))
                .unwrap()
                .changes,
            0
        );
        assert_eq!(registers.of_unit(12).count(), 2);
        assert_eq!(registers.of_unit(5).count(), 0);
    }

    #[test]
    fn a_write_is_counted_and_what_was_there_before_is_known() {
        let mut registers = Registers::default();
        registers.add(&exchange(
            0,
            read(1, 1),
            Outcome::Registers { values: vec![4200] },
        ));
        assert_eq!(
            registers.before(12, Area::HoldingRegister, 1, 1),
            Some(vec![4200])
        );
        assert_eq!(registers.before(12, Area::HoldingRegister, 1, 2), None);
        let write = Asked {
            write: true,
            values: vec![4500],
            ..read(1, 1)
        };
        registers.add(&exchange(1, write, Outcome::Written));
        let seen = registers.get(&(12, Area::HoldingRegister, 1)).unwrap();
        assert_eq!((seen.reads, seen.writes, seen.last), (1, 1, Some(4500)));
    }

    #[test]
    fn a_refused_read_leaves_nothing() {
        let mut registers = Registers::default();
        registers.add(&exchange(0, read(100, 21), Outcome::Exception { code: 2 }));
        assert_eq!(registers.of_unit(12).count(), 0);
    }
}
