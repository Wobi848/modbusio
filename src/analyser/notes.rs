//! What the analysis means, in words (docs/concept-analyser.md, *Diagnosis in words*; concept
//! section 8). Each note has a threshold, so a single slip on a long run says nothing.

use std::collections::HashMap;

use super::Analysis;
use super::exchanges::{Exchange, Outcome};
use crate::diagnosis::Note;

/// Unanswered requests a unit needs, and their share, before it is named.
const MISSING_AT_LEAST: u64 = 3;
const MISSING_SHARE: f64 = 0.01;
/// Share of a unit's misses that must follow the same other unit to name that one.
const AFTER_SHARE: f64 = 0.8;
/// The same request refused this often, and every time, before it is named.
const REFUSED_AT_LEAST: u64 = 3;
/// Answers slower than this are worth a word.
const SLOW_MICROS: u64 = 200_000;
/// Damaged answers before they are named.
const DAMAGED_AT_LEAST: u64 = 2;
/// Answers without a request before a second master is suspected.
const ANSWER_ONLY_AT_LEAST: u64 = 3;
/// Requests without any answer before the trunk is named silent.
const UNANSWERED_AT_LEAST: u64 = 5;
/// Share of time the trunk may carry bytes before it is named busy.
const BUSY: f64 = 0.7;

/// The notes for an analysis of the trunk on `line`, with its bus load (0 to 1).
pub fn notes(analysis: &Analysis, line: &str, load: f64) -> Vec<Note> {
    let mut notes = Vec::new();
    let requests: u64 = analysis.units.values().map(|unit| unit.requests).sum();
    let answers: u64 = analysis.units.values().map(|unit| unit.answers).sum();
    let answer_only: u64 = analysis.units.values().map(|unit| unit.answer_only).sum();
    // answers that came without their request are still answers: that is another note
    if requests >= UNANSWERED_AT_LEAST && answers == 0 && answer_only == 0 {
        notes.push(Note::MasterUnanswered { line: line.into() });
        return notes;
    }
    let exchanges: Vec<&Exchange> = analysis.exchanges().iter().map(|item| &item.item).collect();
    for (&unit, stats) in &analysis.units {
        // answers without their request: the unit does answer, see the note on two masters
        if stats.answers == 0 && stats.answer_only == 0 && stats.no_answer >= MISSING_AT_LEAST {
            notes.push(Note::NeverAnswers {
                unit,
                requests: stats.requests,
            });
            continue;
        }
        let share = stats.no_answer as f64 / stats.requests.max(1) as f64;
        if stats.no_answer >= MISSING_AT_LEAST && share >= MISSING_SHARE {
            let percent = (share * 100.0).round().max(1.0) as u32;
            notes.push(match missed_after(&exchanges, unit) {
                Some(after) => Note::MissesAfter {
                    unit,
                    missing: stats.no_answer,
                    requests: stats.requests,
                    percent,
                    after,
                },
                None => Note::MissesAnswers {
                    unit,
                    missing: stats.no_answer,
                    requests: stats.requests,
                    percent,
                },
            });
        }
        if let Some(times) = stats.answer_times.summary()
            && times.max >= SLOW_MICROS
        {
            notes.push(Note::SlowAnswers {
                unit,
                max_ms: times.max / 1000,
                median_ms: times.median / 1000,
            });
        }
    }
    notes.extend(refused_every_time(&exchanges));
    let damaged: Vec<(u8, u64)> = analysis
        .units
        .iter()
        .filter(|(_, stats)| stats.damaged > 0)
        .map(|(unit, stats)| (*unit, stats.damaged))
        .collect();
    match damaged[..] {
        [(unit, count)] if count >= DAMAGED_AT_LEAST => {
            notes.push(Note::DamagedFromOne { unit, count });
        }
        _ if analysis.bus.damaged >= DAMAGED_AT_LEAST && damaged.len() > 1 => {
            notes.push(Note::Damaged {
                line: line.into(),
                count: analysis.bus.damaged as usize,
            });
        }
        _ => {}
    }
    if answer_only >= ANSWER_ONLY_AT_LEAST {
        notes.push(Note::TwoMasters {
            line: line.into(),
            count: answer_only,
        });
    }
    if load >= BUSY {
        notes.push(Note::BusBusy {
            percent: (load * 100.0).round() as u32,
        });
    }
    if analysis.bus.broadcasts > 0 {
        notes.push(Note::Broadcasts {
            count: analysis.bus.broadcasts,
        });
    }
    notes
}

/// The other unit asked right before nearly every request `unit` left unanswered.
fn missed_after(exchanges: &[&Exchange], unit: u8) -> Option<u8> {
    let mut before: HashMap<u8, u64> = HashMap::new();
    let mut missed = 0;
    for pair in exchanges.windows(2) {
        if pair[1].unit == unit && pair[1].outcome == Outcome::NoAnswer {
            missed += 1;
            if pair[0].unit != unit {
                *before.entry(pair[0].unit).or_default() += 1;
            }
        }
    }
    let (after, count) = before.into_iter().max_by_key(|(_, count)| *count)?;
    (count as f64 >= missed as f64 * AFTER_SHARE).then_some(after)
}

/// How one read went each time it was asked.
#[derive(Default)]
struct Tally {
    times: u64,
    exception: Option<u8>,
    /// Every answer was that exception.
    always: bool,
}

/// Reads that got the same exception every time they were asked.
fn refused_every_time(exchanges: &[&Exchange]) -> Vec<Note> {
    // by unit, area, first address and count
    let mut asked: HashMap<(u8, &str, u16, u16), Tally> = HashMap::new();
    for exchange in exchanges {
        let Some(read) = exchange.asked.as_ref().filter(|asked| !asked.write) else {
            continue;
        };
        if exchange.outcome == Outcome::NoAnswer {
            continue;
        }
        let key = (exchange.unit, read.area.short(), read.address, read.count);
        let tally = asked.entry(key).or_insert(Tally {
            always: true,
            ..Tally::default()
        });
        tally.times += 1;
        match exchange.outcome {
            Outcome::Exception { code } if tally.exception.is_none_or(|first| first == code) => {
                tally.exception = Some(code);
            }
            _ => tally.always = false,
        }
    }
    let mut notes: Vec<Note> = asked
        .into_iter()
        .filter_map(|((unit, area, first, count), tally)| {
            let code = tally
                .exception
                .filter(|_| tally.always && tally.times >= REFUSED_AT_LEAST)?;
            Some(Note::RefusedEveryTime {
                unit,
                area: area.into(),
                first,
                last: first.saturating_add(count.saturating_sub(1)),
                exception: format!("{code:02X}"),
            })
        })
        .collect();
    notes.sort_by_key(|note| match note {
        Note::RefusedEveryTime { unit, first, .. } => (*unit, *first),
        _ => (0, 0),
    });
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::rtu;

    /// Plays requests and answers, 100 ms apart; `None` for a request left unanswered.
    fn analyse(steps: &[(Vec<u8>, Option<Vec<u8>>)], cycles: u64) -> Analysis {
        let mut analysis = Analysis::default();
        let mut at = 0;
        for _ in 0..cycles {
            for (request, answer) in steps {
                analysis.push(at, request);
                at += 30_000;
                if let Some(answer) = answer {
                    analysis.push(at, answer);
                }
                at += 70_000;
            }
        }
        analysis.quiet();
        analysis
    }

    fn read(unit: u8) -> Vec<u8> {
        rtu::frame(unit, &[0x03, 0, 0, 0, 1])
    }

    fn answer(unit: u8) -> Option<Vec<u8>> {
        Some(rtu::frame(unit, &[0x03, 2, 0, 1]))
    }

    #[test]
    fn a_healthy_trunk_has_nothing_to_say() {
        let analysis = analyse(&[(read(1), answer(1)), (read(2), answer(2))], 20);
        assert_eq!(notes(&analysis, "COM3", 0.2), []);
    }

    #[test]
    fn a_unit_that_never_answers_is_named() {
        let analysis = analyse(&[(read(1), answer(1)), (read(9), None)], 5);
        // the last request still waits for its answer
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::NeverAnswers {
                unit: 9,
                requests: 4
            }]
        );
    }

    #[test]
    fn misses_right_after_a_neighbour_name_the_neighbour() {
        let mut analysis = Analysis::default();
        let mut at = 0;
        for cycle in 0..30u64 {
            for unit in [1u8, 5, 12] {
                analysis.push(at, &read(unit));
                if !(unit == 12 && cycle % 5 == 0) {
                    analysis.push(at + 30_000, &answer(unit).unwrap());
                }
                at += 100_000;
            }
        }
        let said = notes(&analysis, "COM3", 0.2);
        assert_eq!(
            said,
            [Note::MissesAfter {
                unit: 12,
                missing: 6,
                requests: 30,
                percent: 20,
                after: 5
            }]
        );
        assert!(
            said[0]
                .to_string()
                .contains("right after a request to unit 5")
        );
    }

    #[test]
    fn misses_without_a_pattern_are_named_plainly() {
        // unit 3 alone on the trunk: nothing comes before its misses but itself
        let steps: Vec<_> = (0..10)
            .map(|step| (read(3), if step % 3 == 0 { None } else { answer(3) }))
            .collect();
        let said = notes(&analyse(&steps, 1), "COM3", 0.2);
        assert!(
            matches!(
                said[..],
                [Note::MissesAnswers {
                    unit: 3,
                    missing: 3,
                    ..
                }]
            ),
            "{said:?}"
        );
    }

    #[test]
    fn a_block_refused_every_time_is_named_with_its_range() {
        let refused = rtu::frame(7, &[0x03, 0, 100, 0, 21]);
        let exception = Some(rtu::frame(7, &[0x83, 0x02]));
        let analysis = analyse(&[(read(7), answer(7)), (refused, exception)], 4);
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::RefusedEveryTime {
                unit: 7,
                area: "HR".into(),
                first: 100,
                last: 120,
                exception: "02".into()
            }]
        );
    }

    #[test]
    fn slow_answers_are_named() {
        let mut analysis = Analysis::default();
        for cycle in 0..5u64 {
            let at = cycle * 1_000_000;
            analysis.push(at, &read(12));
            let wait = if cycle == 2 { 310_000 } else { 40_000 };
            analysis.push(at + wait, &answer(12).unwrap());
        }
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::SlowAnswers {
                unit: 12,
                max_ms: 310,
                median_ms: 40
            }]
        );
    }

    #[test]
    fn damage_from_one_unit_points_to_its_wiring_and_from_several_to_the_trunk() {
        let mut broken = answer(5).unwrap();
        broken[3] ^= 0x40;
        let analysis = analyse(&[(read(1), answer(1)), (read(5), Some(broken.clone()))], 3);
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::DamagedFromOne { unit: 5, count: 3 }]
        );
        let mut other = answer(1).unwrap();
        other[3] ^= 0x40;
        let analysis = analyse(&[(read(1), Some(other)), (read(5), Some(broken))], 2);
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::Damaged {
                line: "COM3".into(),
                count: 4
            }]
        );
    }

    #[test]
    fn a_master_nobody_answers_is_named_alone() {
        let analysis = analyse(&[(read(1), None), (read(2), None)], 3);
        assert_eq!(
            notes(&analysis, "COM3", 0.2),
            [Note::MasterUnanswered {
                line: "COM3".into()
            }]
        );
    }

    #[test]
    fn answers_without_their_request_point_to_a_second_master() {
        // the second master asks unit 1 while the first one's answer from unit 5 is due
        let analysis = analyse(
            &[(read(5), None), (read(1), answer(5)), (read(2), answer(1))],
            3,
        );
        let said = notes(&analysis, "COM3", 0.2);
        assert!(
            said.contains(&Note::TwoMasters {
                line: "COM3".into(),
                count: 6
            }),
            "{said:?}"
        );
    }

    #[test]
    fn broadcasts_and_a_busy_trunk_are_said() {
        let write = rtu::frame(0, &[0x06, 0, 50, 0, 1]);
        let analysis = analyse(&[(read(1), answer(1)), (write, None)], 2);
        assert_eq!(
            notes(&analysis, "COM3", 0.85),
            [Note::BusBusy { percent: 85 }, Note::Broadcasts { count: 2 }]
        );
    }
}
