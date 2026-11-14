//! The bus analyser against the simulated trunk (`examples/sim/trunk.rs`), played through memory in
//! tokio's paused time, so forty cycles of two seconds take a moment.

mod common;

use std::time::Duration;

use common::sim::trunk::{self, Options};
use modbusio::analyser::Analysis;
use tokio::io::AsyncReadExt;
use tokio::time::Instant;

/// Plays `cycles` cycles of the trunk and analyses what arrives, as the server will from a port.
async fn analyse(cycles: u64, options: Options) -> Analysis {
    let cycle = options.cycle;
    let (line, mut heard) = tokio::io::duplex(4096);
    let playing = tokio::spawn(async move {
        // the last cycle's steps end long before the next cycle would start
        let _ = tokio::time::timeout(
            cycle * cycles as u32 - Duration::from_millis(500),
            trunk::play(line, options),
        )
        .await;
    });
    let started = Instant::now();
    let mut analysis = Analysis::default();
    let mut chunk = [0u8; 256];
    loop {
        match tokio::time::timeout(Duration::from_millis(50), heard.read(&mut chunk)).await {
            Ok(Ok(0)) | Ok(Err(_)) => break,
            Ok(Ok(count)) => analysis.push(started.elapsed().as_micros() as u64, &chunk[..count]),
            Err(_) => analysis.quiet(),
        }
    }
    analysis.quiet();
    playing.await.expect("played");
    analysis
}

#[tokio::test(start_paused = true)]
async fn forty_cycles_of_the_trunk_are_counted_unit_by_unit() {
    let analysis = analyse(40, Options::default()).await;
    let unit = |id: u8| &analysis.units[&id];

    assert_eq!((unit(1).requests, unit(1).answers), (40, 40));
    // every eighth answer of unit 5 is damaged: cycles 7, 15, 23, 31, 39
    assert_eq!(
        (unit(5).requests, unit(5).answers, unit(5).damaged),
        (40, 35, 5)
    );
    // unit 12: 40 reads and 4 writes; every third read unanswered
    assert_eq!(
        (unit(12).requests, unit(12).answers, unit(12).no_answer),
        (44, 31, 13)
    );
    assert_eq!(unit(12).writes, 4);
    assert_eq!(unit(7).requests, 80);
    assert_eq!(unit(7).exceptions.get(&2), Some(&40));
    assert_eq!(
        (unit(9).requests, unit(9).answers, unit(9).no_answer),
        (40, 0, 40)
    );
    assert_eq!(analysis.bus.broadcasts, 2);
    assert_eq!(analysis.bus.damaged, 5);

    let drive = unit(12).answer_times.summary().expect("answer times");
    assert!(drive.min >= 40_000 && drive.max <= 300_000, "{drive:?}");
    let meter = unit(1).cycle.summary().expect("cycle");
    assert_eq!(meter.median, 2_000_000);
}

#[tokio::test(start_paused = true)]
async fn a_clean_trunk_has_no_faults_but_the_silent_unit_and_the_refused_block() {
    let analysis = analyse(
        10,
        Options {
            clean: true,
            ..Options::default()
        },
    )
    .await;
    assert_eq!(analysis.bus.damaged, 0);
    assert_eq!(analysis.units[&5].damaged, 0);
    assert_eq!(analysis.units[&12].no_answer, 0);
    assert_eq!(analysis.units[&9].no_answer, 10);
}

#[tokio::test(start_paused = true)]
async fn the_trunks_faults_are_said_in_words() {
    use modbusio::diagnosis::Note;
    let analysis = analyse(40, Options::default()).await;
    let said = modbusio::analyser::notes::notes(&analysis, "COM3", 0.3);
    let has = |wanted: fn(&Note) -> bool| said.iter().any(wanted);
    assert!(
        has(|note| matches!(
            note,
            Note::MissesAfter {
                unit: 12,
                after: 5,
                ..
            }
        )),
        "{said:?}"
    );
    assert!(
        has(|note| matches!(note, Note::NeverAnswers { unit: 9, .. })),
        "{said:?}"
    );
    assert!(
        has(|note| matches!(
            note,
            Note::RefusedEveryTime {
                unit: 7,
                first: 100,
                last: 120,
                ..
            }
        )),
        "{said:?}"
    );
    assert!(
        has(|note| matches!(note, Note::SlowAnswers { unit: 12, .. })),
        "{said:?}"
    );
    assert!(
        has(|note| matches!(note, Note::DamagedFromOne { unit: 5, count: 5 })),
        "{said:?}"
    );
    assert!(
        has(|note| matches!(note, Note::Broadcasts { count: 2 })),
        "{said:?}"
    );
    assert!(
        !has(|note| matches!(note, Note::TwoMasters { .. })),
        "{said:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_second_master_is_noticed() {
    use modbusio::diagnosis::Note;
    let analysis = analyse(
        10,
        Options {
            clean: true,
            second_master: true,
            ..Options::default()
        },
    )
    .await;
    let said = modbusio::analyser::notes::notes(&analysis, "COM3", 0.3);
    assert!(
        said.iter()
            .any(|note| matches!(note, Note::TwoMasters { count: 10, .. })),
        "{said:?}"
    );
}
