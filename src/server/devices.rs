//! Reading devices: the points of a device's profile, chosen points across devices (watch list,
//! live values), the register explorer, and who a device is.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::{Value, json};

use super::views::PointView;
use super::{ApiError, Shared};
use crate::access::Limits;
use crate::diagnosis::{Note, NoteView};
use crate::endpoint::Target;
use crate::explore;
use crate::links::ANSWER_TIMEOUT;
use crate::profile::{Point, Profile, Reading};
use crate::protocol::{Area, pdu};
use crate::read;

/// What changed in a device's limits, as notes.
fn learnt(before: &Limits, after: &Limits) -> Vec<NoteView> {
    let mut notes = Vec::new();
    if after.most_registers < before.most_registers {
        notes.push(
            Note::BlockLimit {
                count: after.most_registers,
            }
            .into(),
        );
    }
    if after.no_holes && !before.no_holes {
        notes.push(Note::NoHoles.into());
    }
    if after.single_register_write && !before.single_register_write {
        notes.push(Note::SingleWrites.into());
    }
    notes
}

/// The device's profile, or why there is none.
pub fn profile_of(state: &Shared, key: &str) -> Result<Profile, ApiError> {
    let (device, _) = state.device(key)?;
    let id = device.profile.ok_or_else(|| {
        ApiError::conflict("this device has no profile yet: choose one or make one in the explorer")
    })?;
    state
        .profile(&id)
        .ok_or_else(|| ApiError::not_found(format!("the profile {id} is gone")))
}

/// Reads `points` of a site device, keeps what was learnt about its limits.
pub async fn read_device(
    state: &Shared,
    key: &str,
    profile: &Profile,
    points: &[Point],
) -> Result<(Vec<Reading>, Vec<NoteView>), ApiError> {
    let (device, target) = state.device(key)?;
    let mut limits = device.limits;
    let readings = read::read_points(
        &state.links,
        &target,
        profile.order,
        points,
        &mut limits,
        ANSWER_TIMEOUT,
    )
    .await;
    let notes = learnt(&device.limits, &limits);
    if limits != device.limits {
        state.update_device(key, |device| device.limits = limits)?;
    }
    Ok((readings, notes))
}

#[derive(Deserialize)]
pub struct ReadRequest {
    key: String,
    /// Only these points, by name; all when left out.
    points: Option<Vec<String>>,
}

pub async fn read(
    State(state): State<Shared>,
    Json(request): Json<ReadRequest>,
) -> Result<Json<Value>, ApiError> {
    let profile = profile_of(&state, &request.key)?;
    let points: Vec<Point> = match &request.points {
        None => profile.points.clone(),
        Some(names) => profile
            .points
            .iter()
            .filter(|point| names.contains(&point.name))
            .cloned()
            .collect(),
    };
    let (readings, notes) = read_device(&state, &request.key, &profile, &points).await?;
    let views: Vec<PointView> = points
        .iter()
        .zip(&readings)
        .map(|(point, reading)| PointView::new(point, &profile, Some(reading)))
        .collect();
    let failed = readings
        .iter()
        .filter(|reading| reading.error.is_some())
        .count();
    Ok(Json(json!({
        "profile": { "id": profile.id, "name": profile.name, "order": profile.order },
        "points": views,
        "failed": failed,
        "notes": notes,
    })))
}

#[derive(Deserialize)]
pub struct Item {
    key: String,
    point: String,
}

#[derive(Deserialize)]
pub struct ValuesRequest {
    items: Vec<Item>,
}

/// Points across devices, each device read once.
pub async fn values(
    State(state): State<Shared>,
    Json(request): Json<ValuesRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.items.len() > 500 {
        return Err(ApiError::bad_request("at most 500 points at once"));
    }
    let mut by_device: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in &request.items {
        by_device
            .entry(item.key.clone())
            .or_default()
            .push(item.point.clone());
    }
    let mut answers = Vec::new();
    for (key, names) in by_device {
        let profile = match profile_of(&state, &key) {
            Ok(profile) => profile,
            Err(error) => {
                for name in names {
                    answers.push(json!({ "key": key, "point": name, "error": error.message() }));
                }
                continue;
            }
        };
        let points: Vec<Point> = names
            .iter()
            .filter_map(|name| {
                profile
                    .points
                    .iter()
                    .find(|point| &point.name == name)
                    .cloned()
            })
            .collect();
        let (readings, _) = read_device(&state, &key, &profile, &points).await?;
        for (point, reading) in points.iter().zip(readings) {
            answers.push(
                json!({ "key": key, "point": point.name, "unit": point.unit, "reading": reading }),
            );
        }
        for name in names
            .iter()
            .filter(|name| !points.iter().any(|point| &point.name == *name))
        {
            answers.push(json!({ "key": key, "point": name, "error": "the profile has no such point any more" }));
        }
    }
    Ok(Json(json!(answers)))
}

#[derive(Deserialize)]
pub struct ExploreRequest {
    key: String,
    area: Area,
    start: u16,
    count: u32,
}

pub async fn explore(
    State(state): State<Shared>,
    Json(request): Json<ExploreRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.count == 0 || request.count > explore::MOST_ADDRESSES {
        return Err(ApiError::bad_request(format!(
            "explore 1 to {} addresses at once",
            explore::MOST_ADDRESSES
        )));
    }
    let target = Target::parse_key(&request.key).map_err(ApiError::bad_request)?;
    let on_site = state.device(&request.key).ok().map(|(device, _)| device);
    let before = on_site
        .as_ref()
        .map(|device| device.limits)
        .unwrap_or_default();
    let mut limits = before;
    let explored = explore::explore(
        &state.links,
        &target,
        request.area,
        request.start,
        request.count,
        &mut limits,
        ANSWER_TIMEOUT,
    )
    .await;
    let (rows, order) = explore::interpret(&explored.cells);
    let mut notes = learnt(&before, &limits);
    if let Some(order) = order {
        notes.push(
            Note::WordOrder {
                order: order.name().into(),
            }
            .into(),
        );
    }
    if on_site.is_some() && limits != before {
        state.update_device(&request.key, |device| device.limits = limits)?;
    }
    Ok(Json(json!({
        "rows": rows,
        "order": order,
        "found": explored.cells.iter().filter(|cell| cell.value.is_some()).count(),
        "stopped": explored.stopped.map(|error| json!({ "code": error.code(), "text": error.to_string() })),
        "limits": limits,
        "most_per_read": if request.area.is_bits() { pdu::MOST_BITS } else { pdu::MOST_REGISTERS },
        "notes": notes,
    })))
}

#[derive(Deserialize)]
pub struct Key {
    key: String,
}

pub async fn identify(
    State(state): State<Shared>,
    Json(request): Json<Key>,
) -> Result<Json<Value>, ApiError> {
    let (_, target) = state.device(&request.key)?;
    let identity = crate::scan::identify(&state.links, &target, ANSWER_TIMEOUT)
        .await
        .ok_or_else(|| {
            ApiError::device("the device does not answer Read Device Identification (43/14)")
        })?;
    let objects: Vec<Value> = identity
        .objects
        .iter()
        .map(|(id, text)| json!({ "id": id, "text": text }))
        .collect();
    state.update_device(&request.key, |device| {
        device.vendor = identity.get(0).map(str::to_string);
        device.product = identity.get(1).map(str::to_string);
        device.revision = identity.get(2).map(str::to_string);
    })?;
    Ok(Json(json!({ "objects": objects })))
}
