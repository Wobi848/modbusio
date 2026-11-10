//! Writing (concept section 7): the session unlock, a write with the old value read and logged,
//! the log, putting old values back. Every write is refused here while the session is locked,
//! whatever the page shows.

use std::sync::atomic::Ordering;
use std::time::SystemTime;

use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::views::{EntryView, registers_text};
use super::{ApiError, Shared, Zone, csv_file, xlsx_file};
use crate::export::journal_table;
use crate::links::ANSWER_TIMEOUT;
use crate::profile::{self, Point};
use crate::protocol::Area;
use crate::values::{DataType, WordOrder};
use crate::write::{self, Planned};

pub async fn session(State(state): State<Shared>) -> Json<Value> {
    Json(json!({ "unlocked": state.unlocked() }))
}

#[derive(Deserialize)]
pub struct Session {
    unlocked: bool,
}

pub async fn set_session(State(state): State<Shared>, Json(request): Json<Session>) -> Json<Value> {
    state.unlocked.store(request.unlocked, Ordering::SeqCst);
    Json(json!({ "unlocked": request.unlocked }))
}

fn require_unlocked(state: &Shared) -> Result<(), ApiError> {
    if state.unlocked() {
        Ok(())
    } else {
        Err(ApiError::conflict(
            "writing is locked: unlock it for this session first",
        ))
    }
}

/// A place the explorer writes to directly, without a profile point.
#[derive(Deserialize)]
pub struct Raw {
    area: Area,
    address: u16,
    #[serde(rename = "type")]
    kind: DataType,
    #[serde(default)]
    order: WordOrder,
    #[serde(default = "one")]
    factor: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
pub struct WriteRequest {
    key: String,
    /// The point of the device's profile, by name.
    point: Option<String>,
    /// Or a place without a profile; needs `confirm_raw`.
    raw: Option<Raw>,
    #[serde(default)]
    confirm_raw: bool,
    /// The new value as the user typed it: a number, a state word, on or off.
    value: String,
    /// Only check the value, as the confirmation is asked: nothing is read or written.
    #[serde(default)]
    check: bool,
}

/// The point to write and the word order to write it in.
fn point_to_write(state: &Shared, request: &WriteRequest) -> Result<(Point, WordOrder), ApiError> {
    match (&request.point, &request.raw) {
        (Some(name), None) => {
            let profile = super::devices::profile_of(state, &request.key)?;
            let point = profile
                .points
                .iter()
                .find(|point| &point.name == name)
                .cloned()
                .ok_or_else(|| ApiError::not_found(format!("the profile has no point {name}")))?;
            if !point.writable {
                return Err(ApiError::conflict(format!(
                    "{name} is not marked writable in the profile"
                )));
            }
            let order = point.order(&profile);
            Ok((point, order))
        }
        (None, Some(raw)) => {
            if !request.confirm_raw {
                return Err(ApiError::bad_request(
                    "a write without a profile point needs confirm_raw",
                ));
            }
            if !raw.area.writable() {
                return Err(ApiError::bad_request(format!(
                    "{} cannot be written",
                    raw.area
                )));
            }
            let mut point = Point::new(
                &format!("{} {}", raw.area.short(), raw.address),
                raw.area,
                raw.address,
                raw.kind,
            );
            point.scale.factor = raw.factor;
            point.order = Some(raw.order);
            point.writable = true;
            point.check().map_err(ApiError::bad_request)?;
            Ok((point, raw.order))
        }
        _ => Err(ApiError::bad_request("give either a point or a raw place")),
    }
}

pub async fn write(
    State(state): State<Shared>,
    Json(request): Json<WriteRequest>,
) -> Result<Json<Value>, ApiError> {
    require_unlocked(&state)?;
    let (device, target) = state.device(&request.key)?;
    if target.unit == 0 {
        return Err(ApiError::bad_request(write::BROADCAST));
    }
    let (point, order) = point_to_write(&state, &request)?;
    if point.scale_register.is_some() {
        return Err(ApiError::conflict(format!(
            "{} has a scale register; writing such points is not offered",
            point.name
        )));
    }
    let values =
        profile::registers_for(&point, order, &request.value, 0).map_err(ApiError::bad_request)?;
    let text = profile::evaluate(&point, order, &values, 0).text;
    if request.check {
        return Ok(Json(json!({ "checked": true, "text": text })));
    }
    let planned = Planned {
        target,
        point: request.point.clone(),
        area: point.area,
        address: point.address,
        text,
        values,
        restores: None,
    };
    let entry = journaled(&state, planned, device.limits, &request.key, |registers| {
        profile::evaluate(&point, order, registers, 0).text
    })
    .await?;
    Ok(Json(json!(entry)))
}

/// Does a planned write, logs it, keeps what was learnt about the device.
async fn journaled(
    state: &Shared,
    planned: Planned,
    before: crate::access::Limits,
    key: &str,
    describe: impl Fn(&[u16]) -> String,
) -> Result<write::Entry, ApiError> {
    let mut limits = before;
    let path = state.journal_path();
    let entry = write::journaled(
        &state.links,
        planned,
        &mut limits,
        ANSWER_TIMEOUT,
        &state.journal,
        path.as_deref(),
        describe,
        SystemTime::now(),
    )
    .await
    .map_err(ApiError::internal)?;
    state.keep_limits(key, before, limits)?;
    Ok(entry)
}

pub async fn journal(State(state): State<Shared>) -> Json<Value> {
    let journal = state.journal.lock().expect("journal lock");
    let entries = EntryView::all(&journal);
    let standing = entries.iter().filter(|entry| entry.standing).count();
    Json(json!({ "entries": entries, "standing": standing }))
}

#[derive(Deserialize)]
pub struct Restore {
    /// Entries to put back; every standing one when left out and `all` is set.
    #[serde(default)]
    ids: Vec<u64>,
    #[serde(default)]
    all: bool,
}

/// Puts back the old values of standing writes, each logged as a write of its own.
pub async fn restore(
    State(state): State<Shared>,
    Json(request): Json<Restore>,
) -> Result<Json<Value>, ApiError> {
    require_unlocked(&state)?;
    let plans: Vec<(u64, Option<Planned>)> = {
        let journal = state.journal.lock().expect("journal lock");
        let standing = journal.changed();
        if !request.all {
            if request.ids.is_empty() {
                return Err(ApiError::bad_request("name the writes to put back, or all"));
            }
            if let Some(id) = request
                .ids
                .iter()
                .find(|id| !standing.iter().any(|entry| entry.id == **id))
            {
                return Err(ApiError::not_found(format!(
                    "write {id} is not among those still standing"
                )));
            }
        }
        standing
            .into_iter()
            .filter(|entry| request.all || request.ids.contains(&entry.id))
            .map(|entry| Ok((entry.id, journal.restoring(entry)?)))
            .collect::<Result<_, String>>()
            .map_err(ApiError::internal)?
    };
    let mut results = Vec::new();
    for (id, planned) in plans {
        let Some(planned) = planned else {
            results.push(json!({ "id": id, "error": "the old value is not known" }));
            continue;
        };
        let key = planned.target.key();
        let limits = state
            .device(&key)
            .map(|(device, _)| device.limits)
            .unwrap_or_default();
        let restored = journaled(&state, planned, limits, &key, registers_text).await;
        match restored {
            Ok(done) => results.push(json!({ "id": id, "entry": done })),
            Err(error) => results.push(json!({ "id": id, "error": error.message() })),
        }
    }
    Ok(Json(json!({ "results": results })))
}

pub async fn journal_csv(State(state): State<Shared>, Query(zone): Query<Zone>) -> Response {
    let table = journal_table(
        &state.journal.lock().expect("journal lock").entries,
        zone.offset,
    );
    csv_file("modbusio-writes.csv", &table)
}

pub async fn journal_xlsx(
    State(state): State<Shared>,
    Query(zone): Query<Zone>,
) -> Result<Response, ApiError> {
    let table = journal_table(
        &state.journal.lock().expect("journal lock").entries,
        zone.offset,
    );
    xlsx_file("modbusio-writes.xlsx", &[table])
}
