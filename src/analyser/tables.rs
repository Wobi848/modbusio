//! The analysis as tables for CSV and Excel (`export.rs`): units, exchanges, registers.

use std::time::{Duration, SystemTime};

use super::Analysis;
use super::exchanges::Outcome;
use super::units::TimesSummary;
use crate::export::{Table, date_time_text};

/// When something heard `at` microseconds after `started` happened, to the millisecond.
fn time_text(started: SystemTime, at: u64, offset_minutes: i32) -> String {
    let time = started + Duration::from_micros(at);
    let millis = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_millis());
    format!("{}.{millis:03}", date_time_text(time, offset_minutes))
}

fn ms(micros: u64) -> String {
    format!("{:.1}", micros as f64 / 1000.0)
}

fn times(summary: Option<TimesSummary>) -> [String; 4] {
    summary.map_or_else(Default::default, |times| {
        [
            ms(times.min),
            ms(times.median),
            ms(times.p95),
            ms(times.max),
        ]
    })
}

pub fn units(analysis: &Analysis) -> Table {
    let mut table = Table::new(
        "Units",
        &[
            "Unit",
            "Requests",
            "Answers",
            "No answer",
            "Damaged",
            "Exceptions",
            "Writes",
            "Answer ms min",
            "Answer ms median",
            "Answer ms 95 %",
            "Answer ms max",
            "Cycle s median",
        ],
    );
    for (unit, stats) in &analysis.units {
        let exceptions = stats
            .exceptions
            .iter()
            .map(|(code, count)| format!("{count}×{code:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut row = vec![
            unit.to_string(),
            stats.requests.to_string(),
            stats.answers.to_string(),
            stats.no_answer.to_string(),
            stats.damaged.to_string(),
            exceptions,
            stats.writes.to_string(),
        ];
        row.extend(times(stats.answer_times.summary()));
        row.push(
            stats
                .cycle
                .summary()
                .map(|cycle| format!("{:.2}", cycle.median as f64 / 1e6))
                .unwrap_or_default(),
        );
        table.rows.push(row);
    }
    table
}

fn result_text(outcome: &Outcome) -> (String, String) {
    let join = |values: &mut dyn Iterator<Item = String>| values.collect::<Vec<_>>().join(" ");
    match outcome {
        Outcome::Registers { values } => (
            "registers".into(),
            join(&mut values.iter().map(u16::to_string)),
        ),
        Outcome::Bits { values } => (
            "bits".into(),
            join(&mut values.iter().map(|on| u8::from(*on).to_string())),
        ),
        Outcome::Exception { code } => ("exception".into(), format!("{code:02X}")),
        Outcome::Malformed { reason } => ("malformed".into(), reason.clone()),
        Outcome::Written => ("written".into(), String::new()),
        Outcome::Answered => ("answered".into(), String::new()),
        Outcome::Damaged => ("damaged".into(), String::new()),
        Outcome::NoAnswer => ("no answer".into(), String::new()),
        Outcome::Broadcast => ("broadcast".into(), String::new()),
        Outcome::AnswerOnly => ("answer only".into(), String::new()),
    }
}

pub fn exchanges(analysis: &Analysis, started: SystemTime, offset_minutes: i32) -> Table {
    let mut table = Table::new(
        "Exchanges",
        &[
            "Time",
            "Unit",
            "Function",
            "Area",
            "Address",
            "Count",
            "Write",
            "Before",
            "Result",
            "Values",
            "Answer ms",
        ],
    );
    for numbered in analysis.exchanges() {
        let exchange = &numbered.item;
        let asked = exchange.asked.as_ref();
        let (result, mut values) = result_text(&exchange.outcome);
        if let Some(asked) = asked.filter(|asked| asked.write) {
            values = asked
                .values
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" ");
        }
        let before = exchange.before.as_ref().map_or_else(String::new, |before| {
            before
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        });
        table.rows.push(vec![
            time_text(started, exchange.at, offset_minutes),
            exchange.unit.to_string(),
            format!("{:02X}", exchange.function),
            asked
                .map(|asked| asked.area.short().to_string())
                .unwrap_or_default(),
            asked
                .map(|asked| asked.address.to_string())
                .unwrap_or_default(),
            asked
                .map(|asked| asked.count.to_string())
                .unwrap_or_default(),
            asked.map_or_else(String::new, |asked| {
                if asked.write { "yes" } else { "" }.into()
            }),
            before,
            result,
            values,
            exchange.answer_micros.map(ms).unwrap_or_default(),
        ]);
    }
    table
}

pub fn registers(analysis: &Analysis, started: SystemTime, offset_minutes: i32) -> Table {
    let mut table = Table::new(
        "Registers",
        &[
            "Unit",
            "Area",
            "Address",
            "Reads",
            "Writes",
            "Last",
            "Changes",
            "Min",
            "Max",
            "Last seen",
        ],
    );
    for unit in analysis.units.keys() {
        for ((_, area, address), seen) in analysis.registers.of_unit(*unit) {
            let number =
                |value: Option<u16>| value.map(|value| value.to_string()).unwrap_or_default();
            table.rows.push(vec![
                unit.to_string(),
                area.short().to_string(),
                address.to_string(),
                seen.reads.to_string(),
                seen.writes.to_string(),
                number(seen.last),
                seen.changes.to_string(),
                number(seen.min),
                number(seen.max),
                time_text(started, seen.last_at, offset_minutes),
            ]);
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::rtu;

    #[test]
    fn the_tables_hold_units_exchanges_and_registers() {
        let mut analysis = Analysis::default();
        analysis.push(0, &rtu::frame(7, &[0x03, 0, 0, 0, 2]));
        analysis.push(20_000, &rtu::frame(7, &[0x03, 4, 0, 1, 0, 2]));
        analysis.push(500_000, &rtu::frame(7, &[0x03, 0, 100, 0, 21]));
        analysis.push(521_000, &rtu::frame(7, &[0x83, 0x02]));
        let started = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400);
        assert_eq!(
            units(&analysis).rows[0][..7],
            ["7", "2", "2", "0", "0", "1×02", "0"]
        );
        let exchanges = exchanges(&analysis, started, 60);
        assert_eq!(exchanges.rows[0][0], "1970-01-02 01:00:00.000");
        assert_eq!(exchanges.rows[0][8..], ["registers", "1 2", "20.0"]);
        assert_eq!(
            exchanges.rows[1][1..9],
            ["7", "03", "HR", "100", "21", "", "", "exception"]
        );
        let registers = registers(&analysis, started, 0);
        assert_eq!(registers.rows.len(), 2);
        assert_eq!(registers.rows[1][..6], ["7", "HR", "1", "1", "0", "2"]);
    }
}
