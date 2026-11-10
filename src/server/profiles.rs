//! Profiles: the list, one profile, saving and removing, importing a vendor's register list with
//! a preview read from the device, and a profile as CSV or Excel file.

use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::views::PointView;
use super::{ApiError, Shared, csv_file, xlsx_file};
use crate::export::profile_table;
use crate::links::ANSWER_TIMEOUT;
use crate::profile::{Point, Profile};
use crate::profile_import::{self, Options};

/// Points a preview reads from the device at most.
const MOST_PREVIEW_POINTS: usize = 40;
/// Points one profile holds at most.
const MOST_POINTS: usize = 5000;

pub async fn list(State(state): State<Shared>) -> Json<Value> {
    let profiles = state.profiles.lock().expect("profiles lock").clone();
    let site = state.site.lock().expect("site lock").clone();
    let mut list: Vec<Value> = profiles
        .iter()
        .map(|profile| {
            json!({
                "id": profile.id,
                "name": profile.name,
                "vendor": profile.vendor,
                "model": profile.model,
                "origin": profile.origin,
                "points": profile.points.len(),
                "used_by": site.devices.iter().filter(|device| device.profile.as_ref() == Some(&profile.id)).count(),
            })
        })
        .collect();
    list.sort_by_key(|profile| profile["name"].as_str().unwrap_or_default().to_lowercase());
    Json(json!(list))
}

#[derive(Deserialize)]
pub struct Id {
    id: String,
}

pub async fn get(
    State(state): State<Shared>,
    Query(query): Query<Id>,
) -> Result<Json<Value>, ApiError> {
    let profile = state
        .profile(&query.id)
        .ok_or_else(|| ApiError::not_found(format!("no profile {}", query.id)))?;
    let points: Vec<PointView> = profile
        .points
        .iter()
        .map(|point| PointView::new(point, &profile, None))
        .collect();
    Ok(Json(json!({ "profile": profile, "points": points })))
}

/// Checks a profile from outside before it is kept: its points, their number and their names.
pub(super) fn check_to_keep(profile: &Profile) -> Result<(), ApiError> {
    profile.check().map_err(ApiError::bad_request)?;
    if profile.points.len() > MOST_POINTS {
        return Err(ApiError::bad_request(format!(
            "a profile holds at most {MOST_POINTS} points"
        )));
    }
    let names: std::collections::HashSet<&str> = profile
        .points
        .iter()
        .map(|point| point.name.as_str())
        .collect();
    if names.len() != profile.points.len() {
        return Err(ApiError::bad_request(
            "two points have the same name; each needs its own",
        ));
    }
    Ok(())
}

/// The id names the profile's file in the data folder, so it holds nothing a path could use.
pub(super) fn check_id(id: &str) -> Result<(), ApiError> {
    if !crate::profile::is_slug(id) {
        return Err(ApiError::bad_request(
            "a profile id is letters, digits and dashes",
        ));
    }
    Ok(())
}

/// Saves a profile whole; a new one (empty id) gets an id from its name. Returns it.
pub async fn save(
    State(state): State<Shared>,
    Json(mut profile): Json<Profile>,
) -> Result<Json<Value>, ApiError> {
    profile.name = profile.name.trim().to_string();
    check_to_keep(&profile)?;
    if profile.id.trim().is_empty() {
        profile.id = state.free_profile_id(&profile.name);
    } else {
        check_id(&profile.id)?;
    }
    state.save_profile(&profile)?;
    {
        let mut profiles = state.profiles.lock().expect("profiles lock");
        profiles.retain(|known| known.id != profile.id);
        profiles.push(profile.clone());
    }
    Ok(Json(json!(profile)))
}

pub async fn delete(
    State(state): State<Shared>,
    Json(request): Json<Id>,
) -> Result<Json<Value>, ApiError> {
    if state.profile(&request.id).is_none() {
        return Err(ApiError::not_found(format!("no profile {}", request.id)));
    }
    let users: Vec<String> = state
        .site
        .lock()
        .expect("site lock")
        .devices
        .iter()
        .filter(|device| device.profile.as_ref() == Some(&request.id))
        .map(|device| {
            if device.name.is_empty() {
                device.key.clone()
            } else {
                device.name.clone()
            }
        })
        .collect();
    if !users.is_empty() {
        return Err(ApiError::conflict(format!(
            "devices use this profile: {}; give them another first",
            users.join(", ")
        )));
    }
    state.remove_profile_file(&request.id);
    state
        .profiles
        .lock()
        .expect("profiles lock")
        .retain(|profile| profile.id != request.id);
    Ok(list(State(state)).await)
}

#[derive(Deserialize)]
pub struct Import {
    /// The file's name, for its kind (`.csv`, `.xlsx` …).
    file_name: String,
    /// The file, base64.
    content_base64: String,
    #[serde(default)]
    options: Options,
    /// A device to read the first points from, to see at once whether addresses and order fit.
    key: Option<String>,
    /// The word order of the device, for the preview.
    order: Option<crate::values::WordOrder>,
}

/// Reads a vendor's list into points and, with a device, reads the first of them from it.
/// Saves nothing: the page saves the profile when the user agrees.
pub async fn import(
    State(state): State<Shared>,
    Json(request): Json<Import>,
) -> Result<Json<Value>, ApiError> {
    let bytes = base64(&request.content_base64)
        .ok_or_else(|| ApiError::bad_request("the file is not base64"))?;
    let rows =
        profile_import::rows_of_file(&request.file_name, &bytes).map_err(ApiError::bad_request)?;
    let imported =
        profile_import::import(&rows, &request.options).map_err(ApiError::bad_request)?;
    let order = request.order.unwrap_or_default();
    let preview = match &request.key {
        Some(key) => {
            let (device, target) = state.device(key)?;
            let points: Vec<Point> = imported
                .points
                .iter()
                .take(MOST_PREVIEW_POINTS)
                .cloned()
                .collect();
            let mut limits = device.limits;
            Some(
                crate::read::read_points(
                    &state.links,
                    &target,
                    order,
                    &points,
                    &mut limits,
                    ANSWER_TIMEOUT,
                )
                .await,
            )
        }
        None => None,
    };
    Ok(Json(json!({
        "imported": imported,
        "preview": preview,
        "rows": rows.len(),
        "source": request.file_name,
    })))
}

pub async fn csv(
    State(state): State<Shared>,
    Query(query): Query<Id>,
) -> Result<Response, ApiError> {
    let profile = state
        .profile(&query.id)
        .ok_or_else(|| ApiError::not_found(format!("no profile {}", query.id)))?;
    Ok(csv_file(
        &format!("{}.csv", profile.id),
        &profile_table(&profile),
    ))
}

pub async fn xlsx(
    State(state): State<Shared>,
    Query(query): Query<Id>,
) -> Result<Response, ApiError> {
    let profile = state
        .profile(&query.id)
        .ok_or_else(|| ApiError::not_found(format!("no profile {}", query.id)))?;
    xlsx_file(&format!("{}.xlsx", profile.id), &[profile_table(&profile)])
}

/// Standard base64, with or without padding and line breaks; `None` when it is not.
fn base64(text: &str) -> Option<Vec<u8>> {
    let value = |byte: u8| -> Option<u32> {
        Some(match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=')
        .collect();
    let mut bytes = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut bits = 0u32;
        for &byte in chunk {
            bits = (bits << 6) | value(byte)?;
        }
        bits <<= 6 * (4 - chunk.len() as u32);
        let produced = match chunk.len() {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        bytes.extend(&bits.to_be_bytes()[1..1 + produced]);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decodes_with_and_without_padding() {
        assert_eq!(base64("TWFu"), Some(b"Man".to_vec()));
        assert_eq!(base64("TWE="), Some(b"Ma".to_vec()));
        assert_eq!(base64("TQ=="), Some(b"M".to_vec()));
        assert_eq!(base64("TW\nFu"), Some(b"Man".to_vec()));
        assert_eq!(base64("T"), None);
        assert_eq!(base64("T$Fu"), None);
    }
}
