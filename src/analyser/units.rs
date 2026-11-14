//! What the exchanges say per unit: counts, answer times, poll cycle (docs/concept-analyser.md,
//! view *Units*).

use std::collections::{BTreeMap, HashMap, VecDeque};

use serde::Serialize;

use super::exchanges::{Exchange, Outcome};

/// Times kept per unit for the median and the 95 % mark; min and max cover all.
const RECENT_TIMES: usize = 1000;
/// Distinct requests remembered per unit for the poll cycle.
const MOST_REQUESTS: usize = 256;

#[derive(Clone, Debug, Default)]
pub struct Times {
    recent: VecDeque<u64>,
    min: Option<u64>,
    max: Option<u64>,
}

/// Times in microseconds.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TimesSummary {
    pub min: u64,
    pub median: u64,
    pub p95: u64,
    pub max: u64,
}

impl Times {
    pub fn add(&mut self, micros: u64) {
        if self.recent.len() == RECENT_TIMES {
            self.recent.pop_front();
        }
        self.recent.push_back(micros);
        self.min = Some(self.min.map_or(micros, |min| min.min(micros)));
        self.max = Some(self.max.map_or(micros, |max| max.max(micros)));
    }

    pub fn summary(&self) -> Option<TimesSummary> {
        let mut sorted: Vec<u64> = self.recent.iter().copied().collect();
        sorted.sort_unstable();
        let at = |share: usize| sorted[(sorted.len() - 1) * share / 100];
        Some(TimesSummary {
            min: self.min?,
            median: at(50),
            p95: at(95),
            max: self.max?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnitStats {
    pub requests: u64,
    /// Valid answers, exceptions included.
    pub answers: u64,
    pub no_answer: u64,
    pub damaged: u64,
    pub exceptions: BTreeMap<u8, u64>,
    /// Writes the master made to this unit.
    pub writes: u64,
    /// Answers heard without their request.
    pub answer_only: u64,
    pub answer_times: Times,
    /// Time between two of the same request.
    pub cycle: Times,
    /// When each distinct request was last heard, by its PDU.
    last_asked: HashMap<Vec<u8>, u64>,
    pub last_seen: u64,
}

impl UnitStats {
    pub fn add(&mut self, exchange: &Exchange) {
        self.last_seen = self.last_seen.max(exchange.at);
        match &exchange.outcome {
            Outcome::AnswerOnly => {
                self.answer_only += 1;
                return;
            }
            Outcome::NoAnswer => self.no_answer += 1,
            Outcome::Damaged => self.damaged += 1,
            Outcome::Exception { code } => {
                self.answers += 1;
                *self.exceptions.entry(*code).or_default() += 1;
            }
            Outcome::Broadcast => {}
            _ => self.answers += 1,
        }
        self.requests += 1;
        if exchange
            .request
            .as_ref()
            .is_some_and(|request| !request.only_reads())
        {
            self.writes += 1;
        }
        if let Some(micros) = exchange
            .answer_micros
            .filter(|_| !exchange.outcome.failed())
        {
            self.answer_times.add(micros);
        }
        if let Some(pdu) = exchange.request_pdu() {
            self.note_cycle(pdu, exchange.at);
        }
    }

    fn note_cycle(&mut self, pdu: &[u8], at: u64) {
        if let Some(before) = self.last_asked.get(pdu) {
            self.cycle.add(at.saturating_sub(*before));
        } else if self.last_asked.len() >= MOST_REQUESTS {
            return;
        }
        self.last_asked.insert(pdu.to_vec(), at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_and_95_percent_come_from_the_times_kept() {
        let mut times = Times::default();
        assert_eq!(times.summary(), None);
        for micros in (1..=100).rev() {
            times.add(micros * 1000);
        }
        assert_eq!(
            times.summary(),
            Some(TimesSummary {
                min: 1000,
                median: 50_000,
                p95: 95_000,
                max: 100_000
            })
        );
    }

    #[test]
    fn min_and_max_cover_more_than_the_times_kept() {
        let mut times = Times::default();
        times.add(1);
        for _ in 0..RECENT_TIMES {
            times.add(500);
        }
        let summary = times.summary().unwrap();
        assert_eq!((summary.min, summary.median), (1, 500));
    }
}
