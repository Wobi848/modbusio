//! The recording: points added and removed, sampling started and stopped, the samples as chart
//! data, CSV and Excel. The sampling runs here, one round per interval, each device read once.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{ApiError, Shared, Zone, csv_file, xlsx_file};
use crate::export::recording_table;
use crate::storage;

/// How often the samples are saved to the data folder.
const BACKUP_EVERY: Duration = Duration::from_secs(60);

#[derive(Deserialize)]
pub struct Since {
    /// Milliseconds since 1970: only the samples later than this are wanted.
    after: Option<u64>,
}

pub async fn state(State(state): State<Shared>, Query(since): Query<Since>) -> Json<Value> {
    view(&state, since.after)
}

/// The recording as the page takes it. With `after`, each series holds only its samples later
/// than that: a page that has the rest asks for what is new instead of a day of samples every
/// two seconds. `total` says how many the series holds, so the page sees when its own do not fit.
fn view(state: &Shared, after: Option<u64>) -> Json<Value> {
    let recording = state.recording.lock().expect("recording lock");
    let series: Vec<Value> = recording
        .series
        .iter()
        .map(|series| {
            let held = after.map_or(0, |after| {
                series.samples.partition_point(|(time, _)| *time <= after)
            });
            let samples: Vec<_> = series.samples.range(held..).collect();
            json!({
                "device": series.device,
                "point": series.point,
                "label": series.label,
                "unit": series.unit,
                "samples": samples,
                "total": series.samples.len(),
            })
        })
        .collect();
    Json(json!({
        "series": series,
        "interval": recording.interval,
        "running": recording.running,
    }))
}

#[derive(Deserialize)]
pub struct Item {
    key: String,
    point: String,
}

pub async fn add(
    State(state): State<Shared>,
    Json(item): Json<Item>,
) -> Result<Json<Value>, ApiError> {
    let (device, _) = state.device(&item.key)?;
    let profile = super::devices::profile_of(&state, &item.key)?;
    let point = profile
        .points
        .iter()
        .find(|point| point.name == item.point)
        .ok_or_else(|| ApiError::not_found(format!("the profile has no point {}", item.point)))?;
    let device_name = if device.name.is_empty() {
        item.key.clone()
    } else {
        device.name.clone()
    };
    state
        .recording
        .lock()
        .expect("recording lock")
        .add(
            &item.key,
            &point.name,
            format!("{device_name} · {}", point.name),
            point.unit.clone(),
        )
        .map_err(ApiError::conflict)?;
    backup(&state);
    Ok(view(&state, None))
}

pub async fn remove(
    State(state): State<Shared>,
    Json(item): Json<Item>,
) -> Result<Json<Value>, ApiError> {
    if !state
        .recording
        .lock()
        .expect("recording lock")
        .remove(&item.key, &item.point)
    {
        return Err(ApiError::not_found("this point is not recorded"));
    }
    backup(&state);
    Ok(view(&state, None))
}

#[derive(Deserialize)]
pub struct Start {
    interval: u64,
}

pub async fn start(
    State(state): State<Shared>,
    Json(request): Json<Start>,
) -> Result<Json<Value>, ApiError> {
    let started = {
        let mut recording = state.recording.lock().expect("recording lock");
        recording
            .set_interval(request.interval)
            .map_err(ApiError::bad_request)?;
        if recording.series.is_empty() {
            return Err(ApiError::conflict("add points to record first"));
        }
        recording.start()
    };
    if let Some(run) = started {
        tokio::spawn(sample(state.clone(), run));
    }
    Ok(view(&state, None))
}

pub async fn stop(State(state): State<Shared>) -> Json<Value> {
    state.recording.lock().expect("recording lock").running = false;
    backup(&state);
    view(&state, None)
}

pub async fn clear(State(state): State<Shared>) -> Json<Value> {
    state.recording.lock().expect("recording lock").clear();
    backup(&state);
    view(&state, None)
}

/// Rounds of reading while this run of the recording goes on.
async fn sample(state: Shared, run: u64) {
    let mut last_backup = tokio::time::Instant::now();
    loop {
        let round_started = tokio::time::Instant::now();
        let (series, interval) = {
            let recording = state.recording.lock().expect("recording lock");
            if !recording.runs(run) {
                break;
            }
            let series: Vec<(String, String)> = recording
                .series
                .iter()
                .map(|series| (series.device.clone(), series.point.clone()))
                .collect();
            (series, recording.interval)
        };
        let mut by_device: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (device, point) in series {
            by_device.entry(device).or_default().push(point);
        }
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as u64);
        for (key, names) in by_device {
            let values = read_values(&state, &key, &names).await;
            let mut recording = state.recording.lock().expect("recording lock");
            if !recording.runs(run) {
                // stopped while the device was read: what came after the stop is not recorded
                return;
            }
            for (name, value) in names.iter().zip(values) {
                if let Some(index) = recording.find(&key, name) {
                    recording.push(index, time, value);
                }
            }
        }
        if last_backup.elapsed() >= BACKUP_EVERY {
            backup(&state);
            last_backup = tokio::time::Instant::now();
        }
        tokio::time::sleep_until(round_started + Duration::from_secs(interval)).await;
    }
}

/// The values of the named points of one device; `None` for each that failed.
async fn read_values(state: &Shared, key: &str, names: &[String]) -> Vec<Option<f64>> {
    let Ok(profile) = super::devices::profile_of(state, key) else {
        return vec![None; names.len()];
    };
    let points: Vec<_> = names
        .iter()
        .filter_map(|name| {
            profile
                .points
                .iter()
                .find(|point| &point.name == name)
                .cloned()
        })
        .collect();
    let Ok((readings, _)) = super::devices::read_device(state, key, &profile, &points).await else {
        return vec![None; names.len()];
    };
    names
        .iter()
        .map(|name| {
            points
                .iter()
                .position(|point| &point.name == name)
                .and_then(|index| readings[index].value)
        })
        .collect()
}

fn backup(state: &Shared) {
    let Some(dir) = &state.data_dir else {
        return;
    };
    let recording = state.recording.lock().expect("recording lock").clone();
    if let Err(error) = storage::save_json(&dir.join(storage::RECORDING), &recording) {
        // the samples stay in memory; the console window says why the backup did not happen
        eprintln!("cannot back up the recording: {error}");
    }
}

pub async fn csv(State(state): State<Shared>, Query(zone): Query<Zone>) -> Response {
    let table = recording_table(
        &state.recording.lock().expect("recording lock"),
        zone.offset,
    );
    csv_file("modbusio-recording.csv", &table)
}

pub async fn xlsx(
    State(state): State<Shared>,
    Query(zone): Query<Zone>,
) -> Result<Response, ApiError> {
    let table = recording_table(
        &state.recording.lock().expect("recording lock"),
        zone.offset,
    );
    xlsx_file("modbusio-recording.xlsx", &[table])
}
