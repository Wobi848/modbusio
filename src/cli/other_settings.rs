//! `modbusio scan --serial PORT --other-settings`: devices at other serial settings
//! (docs/concept-other-settings.md), with the trunk's setting from `--baud` and `--frame`.

use std::sync::Arc;

use anyhow::anyhow;
use modbusio::endpoint::SerialSettings;
use modbusio::links::Links;
use modbusio::other_settings::{self, Asking, LISTEN_EACH};
use modbusio::scan::Search;

use super::table;

pub async fn scan(
    links: Arc<Links>,
    port: String,
    (baud, frame): (u32, &str),
    (from, to): (u8, u8),
    all_speeds: bool,
) -> anyhow::Result<()> {
    let (parity, stop_bits) = SerialSettings::parse_frame(frame)
        .ok_or_else(|| anyhow!("{frame} is not a frame like 8E1"))?;
    let asking = Asking {
        trunk: SerialSettings {
            port,
            baud,
            parity,
            stop_bits,
        },
        units: (from..=to).collect(),
        all_speeds,
        wait: None,
        master_asks: Default::default(),
        master_silent: Default::default(),
        listen_each: LISTEN_EACH,
    };
    println!(
        "Listening first, then asking units {from} to {to} at {} settings (at most {} s). Read requests only; the plant's master must be stopped.",
        other_settings::candidates(&asking.trunk, all_speeds).len(),
        asking.estimate().as_secs()
    );
    let search = Arc::new(Search::default());
    other_settings::search(links, asking, search.clone()).await;
    let report = search.report.lock().expect("report lock");
    let rows: Vec<Vec<String>> = report
        .other_settings
        .iter()
        .map(|unit| {
            vec![
                unit.unit.to_string(),
                unit.answers_at.join(", "),
                unit.damaged_at.join(", "),
            ]
        })
        .collect();
    if !rows.is_empty() {
        table::print(&["Unit", "Answers at", "Damaged at"], &rows);
    }
    for note in &report.notes {
        println!("{}", note.text);
    }
    Ok(())
}
