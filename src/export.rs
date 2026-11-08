//! Tables for the exports: one table, written as CSV (`csv.rs`) or as an Excel file
//! (`workbook.rs`), so both always hold the same.

use std::time::SystemTime;

use crate::profile::{Point, Profile, Reading};
use crate::record::Recording;
use crate::values::number_text;
use crate::write::Entry;

pub struct Table {
    pub title: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(title: &str, columns: &[&str]) -> Table {
        Table {
            title: title.into(),
            columns: columns.iter().map(|column| column.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    pub fn csv(&self) -> String {
        let mut lines = vec![self.columns.clone()];
        lines.extend(self.rows.iter().cloned());
        crate::csv::file(&lines)
    }
}

const PROFILE_COLUMNS: [&str; 15] = [
    "Name",
    "Group",
    "Area",
    "Address",
    "Type",
    "Order",
    "Factor",
    "Value offset",
    "Scale register",
    "Unit",
    "Description",
    "States",
    "Access",
    "Min",
    "Max",
];

/// A profile as a list modbusio reads back by import (same column names, addresses from 0).
pub fn profile_table(profile: &Profile) -> Table {
    let mut table = Table::new(&profile.name, &PROFILE_COLUMNS);
    table.rows = profile
        .points
        .iter()
        .map(|point| {
            vec![
                point.name.clone(),
                point.group.clone(),
                point.area.short().to_string(),
                point.address.to_string(),
                point.kind.name(),
                point.order(profile).name().to_string(),
                number_text(point.scale.factor),
                number_text(point.scale.offset),
                point
                    .scale_register
                    .map(|register| register.to_string())
                    .unwrap_or_default(),
                point.unit.clone(),
                point.description.clone(),
                point
                    .states
                    .iter()
                    .map(|(number, word)| format!("{number}={word}"))
                    .collect::<Vec<_>>()
                    .join("; "),
                if point.writable { "RW" } else { "R" }.to_string(),
                point.min.map(number_text).unwrap_or_default(),
                point.max.map(number_text).unwrap_or_default(),
            ]
        })
        .collect();
    table
}

/// A device's points with the values read.
pub fn points_table(title: &str, points: &[Point], readings: &[Reading]) -> Table {
    let mut table = Table::new(
        title,
        &[
            "Group", "Name", "Area", "Address", "Value", "Unit", "Raw", "Error",
        ],
    );
    table.rows = points
        .iter()
        .zip(readings)
        .map(|(point, reading)| {
            vec![
                point.group.clone(),
                point.name.clone(),
                point.area.short().to_string(),
                point.address.to_string(),
                reading
                    .value
                    .map(number_text)
                    .unwrap_or_else(|| reading.text.clone()),
                point.unit.clone(),
                reading.raw.join(" "),
                reading.error.clone().unwrap_or_default(),
            ]
        })
        .collect();
    table
}

/// Every write, oldest first; `offset_minutes` is the viewer's time zone.
pub fn journal_table(entries: &[Entry], offset_minutes: i32) -> Table {
    let mut table = Table::new(
        "Writes",
        &[
            "Number",
            "Time",
            "Device",
            "Point",
            "Area",
            "Address",
            "Before",
            "Written",
            "Read back",
            "Restores",
            "Error",
        ],
    );
    table.rows = entries
        .iter()
        .map(|entry| {
            let registers = |values: &Option<Vec<u16>>| {
                values
                    .as_ref()
                    .map(|values| {
                        values
                            .iter()
                            .map(u16::to_string)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default()
            };
            vec![
                entry.id.to_string(),
                date_time_text(seconds(entry.time), offset_minutes),
                entry.device.clone(),
                entry.point.clone().unwrap_or_default(),
                entry.area.short().to_string(),
                entry.address.to_string(),
                entry
                    .before_text
                    .clone()
                    .unwrap_or_else(|| registers(&entry.before)),
                entry.text.clone(),
                registers(&entry.after),
                entry.restores.map(|id| id.to_string()).unwrap_or_default(),
                entry.error.clone().unwrap_or_default(),
            ]
        })
        .collect();
    table
}

/// One row per sampling time, one column per point.
pub fn recording_table(recording: &Recording, offset_minutes: i32) -> Table {
    let mut columns = vec!["Time".to_string()];
    columns.extend(recording.series.iter().map(|series| {
        if series.unit.is_empty() {
            series.label.clone()
        } else {
            format!("{} [{}]", series.label, series.unit)
        }
    }));
    let rows = recording
        .times()
        .into_iter()
        .map(|time| {
            let mut row = vec![date_time_text(
                SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(time),
                offset_minutes,
            )];
            row.extend(recording.series.iter().map(|series| {
                series
                    .samples
                    .iter()
                    .find(|(at, _)| *at == time)
                    .and_then(|(_, value)| *value)
                    .map(number_text)
                    .unwrap_or_default()
            }));
            row
        })
        .collect();
    Table {
        title: "Recording".into(),
        columns,
        rows,
    }
}

fn seconds(since_epoch: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(since_epoch)
}

/// Copied from bacnetio's export.rs. "2026-11-01 09:30:00" at the given offset from UTC.
pub fn date_time_text(time: SystemTime, offset_minutes: i32) -> String {
    let seconds = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
        + i64::from(offset_minutes) * 60;
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_date(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// Days since 1970-01-01 to year, month, day (Howard Hinnant's `civil_from_days`).
fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Area;
    use crate::values::DataType;

    #[test]
    fn times_are_shown_in_the_viewers_zone() {
        let time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_793_000_000);
        assert_eq!(date_time_text(time, 0), "2026-10-26 07:33:20");
        assert_eq!(date_time_text(time, 60), "2026-10-26 08:33:20");
    }

    #[test]
    fn an_exported_profile_imports_back_the_same() {
        let mut flow = Point::new("Flow", Area::HoldingRegister, 2, DataType::F32);
        flow.unit = "°C".into();
        flow.scale.factor = 0.1;
        flow.writable = true;
        flow.states = vec![(0, "off".into())];
        flow.min = Some(5.0);
        let profile = Profile {
            id: "x".into(),
            name: "X".into(),
            vendor: String::new(),
            model: String::new(),
            origin: "own".into(),
            source: String::new(),
            order: crate::values::WordOrder::Cdab,
            points: vec![flow],
        };
        let csv = profile_table(&profile).csv();
        let rows = crate::csv::parse(&csv, ';');
        let imported =
            crate::profile_import::import(&rows, &crate::profile_import::Options::default())
                .expect("imports");
        let mut expected = profile.points[0].clone();
        expected.order = Some(crate::values::WordOrder::Cdab);
        expected.source_address = "2".into();
        assert_eq!(imported.points, [expected]);
    }
}
