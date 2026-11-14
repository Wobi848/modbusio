//! `modbusio analyse`: the bus analyser in the terminal (docs/concept-analyser.md). Listens without
//! sending, shows each exchange as it happens, and ends with the units table.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use modbusio::analyser::Analysis;
use modbusio::analyser::exchanges::{Exchange, Outcome};
use modbusio::analyser::port::{self, ReadOnly};
use modbusio::analyser::units::TimesSummary;
use modbusio::protocol::Exception;
use tokio::time::Instant;

use super::table;

/// How often new exchanges are printed.
const PRINT_EVERY: Duration = Duration::from_millis(200);

pub async fn analyse(
    port: &str,
    given: Option<(u32, &str)>,
    seconds: Option<u64>,
    pcap: Option<PathBuf>,
) -> anyhow::Result<()> {
    println!("Listening to {port} without sending …");
    let settings = port::settings(port, given)
        .await
        .map_err(|error| anyhow!(error))?;
    let read_only = ReadOnly::open(&settings)?;
    println!(
        "{port} at {} {}. Ctrl+C ends with the summary.",
        settings.baud,
        settings.frame_text()
    );
    let analysis = Arc::new(Mutex::new(Analysis::default()));
    let started_at = SystemTime::now();
    let started = Instant::now();
    let mut hearing = tokio::spawn(port::hear(
        read_only,
        analysis.clone(),
        started,
        port::quiet_after(settings.baud),
    ));
    let until = seconds.map(|seconds| started + Duration::from_secs(seconds));
    let mut printed: Option<u64> = None;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            ended = &mut hearing => {
                println!("{}", ended.unwrap_or_else(|error| error.to_string()));
                break;
            }
            _ = tokio::time::sleep(PRINT_EVERY) => {}
        }
        print_new(&analysis, &mut printed);
        if until.is_some_and(|until| Instant::now() >= until) {
            break;
        }
    }
    hearing.abort();
    let analysis = {
        let mut analysis = analysis.lock().expect("analysis lock");
        analysis.quiet();
        std::mem::take(&mut *analysis)
    };
    print_new_of(&analysis, &mut printed);
    println!();
    print_units(&analysis);
    let elapsed = started.elapsed().as_micros().max(1) as f64;
    let load = (analysis.bus.bytes as f64
        * modbusio::protocol::rtu::character_micros(settings.baud) as f64
        / elapsed)
        .min(1.0);
    for note in modbusio::analyser::notes::notes(&analysis, port, load) {
        println!("{note}");
    }
    if let Some(path) = pcap {
        let frames: Vec<_> = analysis
            .frames()
            .iter()
            .map(|seen| seen.item.capture(started_at))
            .collect();
        std::fs::write(&path, modbusio::pcap::write(&frames))?;
        println!("{} frames saved in {}", frames.len(), path.display());
    }
    Ok(())
}

fn print_new(analysis: &Mutex<Analysis>, printed: &mut Option<u64>) {
    print_new_of(&analysis.lock().expect("analysis lock"), printed);
}

fn print_new_of(analysis: &Analysis, printed: &mut Option<u64>) {
    for numbered in analysis.exchanges() {
        if printed.is_some_and(|last| numbered.number <= last) {
            continue;
        }
        println!("{}", line(&numbered.item));
        *printed = Some(numbered.number);
    }
}

/// One exchange as a line: when, which unit, what was asked, what came back, how fast.
fn line(exchange: &Exchange) -> String {
    let asked = match &exchange.asked {
        Some(asked) if asked.write => {
            format!(
                "write {} {} ×{}",
                asked.area.short(),
                asked.address,
                asked.count
            )
        }
        Some(asked) => format!("{} {} ×{}", asked.area.short(), asked.address, asked.count),
        None => format!("function {:02X}", exchange.function),
    };
    let answer = match &exchange.outcome {
        Outcome::Registers { values } => values
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(" "),
        Outcome::Bits { values } => values
            .iter()
            .map(|on| if *on { "1" } else { "0" })
            .collect::<String>(),
        Outcome::Written => "written".into(),
        Outcome::Exception { code } => Exception(*code).to_string(),
        Outcome::Answered => "answered".into(),
        Outcome::Malformed { reason } => format!("answer does not fit: {reason}"),
        Outcome::Damaged => "answer damaged".into(),
        Outcome::NoAnswer => "no answer".into(),
        Outcome::Broadcast => "to all units, none answers".into(),
        Outcome::AnswerOnly => "answer without its request".into(),
    };
    let time = exchange
        .answer_micros
        .map(|micros| format!("  ({} ms)", micros / 1000))
        .unwrap_or_default();
    format!(
        "{:>10.3} s  unit {:<3}  {asked:<18}  {answer}{time}",
        exchange.at as f64 / 1e6,
        exchange.unit
    )
}

fn print_units(analysis: &Analysis) {
    let range = |summary: Option<TimesSummary>| {
        summary.map_or("–".into(), |times| {
            format!("{}–{}", times.min / 1000, times.max / 1000)
        })
    };
    let median = |summary: Option<TimesSummary>| {
        summary.map_or("–".into(), |times| (times.median / 1000).to_string())
    };
    let rows: Vec<Vec<String>> = analysis
        .units
        .iter()
        .map(|(unit, stats)| {
            let exceptions = stats
                .exceptions
                .iter()
                .map(|(code, count)| format!("{count}×{code:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            vec![
                unit.to_string(),
                stats.requests.to_string(),
                stats.answers.to_string(),
                stats.no_answer.to_string(),
                stats.damaged.to_string(),
                exceptions,
                range(stats.answer_times.summary()),
                median(stats.cycle.summary()),
            ]
        })
        .collect();
    table::print(
        &[
            "Unit",
            "Requests",
            "Answers",
            "No answer",
            "Damaged",
            "Exceptions",
            "Answer ms",
            "Cycle ms",
        ],
        &rows,
    );
    let bus = &analysis.bus;
    println!(
        "  {} frames, {} damaged, {} broadcasts",
        bus.frames, bus.damaged, bus.broadcasts
    );
}
