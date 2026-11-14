//! Sites and their devices: the open site, the others kept, a site as a file, devices added by
//! hand, named, given a profile, removed.

use axum::Json;
use axum::extract::State;
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::scanning::refuse_while_searching;
use super::state::new_site;
use super::views::DeviceView;
use super::{ApiError, Shared, download, profiles};
use crate::endpoint::Target;
use crate::links::ANSWER_TIMEOUT;
use crate::profile::Profile;
use crate::scan;
use crate::site::{Site, SiteDevice};

/// Longest name of a site or device.
const MOST_NAME: usize = 80;

fn check_name(name: &str) -> Result<String, ApiError> {
    let name = name.trim();
    if name.chars().count() > MOST_NAME {
        return Err(ApiError::bad_request(format!(
            "a name has at most {MOST_NAME} characters"
        )));
    }
    Ok(name.to_string())
}

pub async fn site(State(state): State<Shared>) -> Json<Value> {
    let site = state.site.lock().expect("site lock").clone();
    let profiles = state.profiles.lock().expect("profiles lock").clone();
    Json(site_view(&site, &profiles))
}

fn site_view(site: &Site, profiles: &[Profile]) -> Value {
    let devices: Vec<DeviceView> = site
        .devices
        .iter()
        .filter_map(|device| DeviceView::new(device, profiles))
        .collect();
    json!({ "id": site.id, "name": site.name, "devices": devices })
}

#[derive(Deserialize)]
pub struct Name {
    name: String,
}

pub async fn rename(
    State(state): State<Shared>,
    Json(request): Json<Name>,
) -> Result<Json<Value>, ApiError> {
    let name = check_name(&request.name)?;
    if name.is_empty() {
        return Err(ApiError::bad_request("a site needs a name"));
    }
    state.site.lock().expect("site lock").name = name;
    state.save_site()?;
    Ok(site(State(state)).await)
}

pub async fn list(State(state): State<Shared>) -> Json<Value> {
    let open = state.site.lock().expect("site lock").id.clone();
    let sites: Vec<Value> = state
        .sites()
        .iter()
        .map(|site| json!({ "id": site.id, "name": site.name, "devices": site.devices.len(), "open": site.id == open }))
        .collect();
    Json(json!(sites))
}

pub async fn create(
    State(state): State<Shared>,
    Json(request): Json<Name>,
) -> Result<Json<Value>, ApiError> {
    refuse_while_searching(&state)?;
    let name = check_name(&request.name)?;
    if name.is_empty() {
        return Err(ApiError::bad_request("a site needs a name"));
    }
    let site = new_site(&name, &state.sites());
    switch(&state, site)?;
    Ok(site_json(&state))
}

fn site_json(state: &Shared) -> Json<Value> {
    let site = state.site.lock().expect("site lock").clone();
    let profiles = state.profiles.lock().expect("profiles lock").clone();
    Json(site_view(&site, &profiles))
}

/// Saves the open site and opens `site`.
fn switch(state: &Shared, site: Site) -> Result<(), ApiError> {
    state.save_site()?;
    *state.site.lock().expect("site lock") = site;
    state.save_site()
}

#[derive(Deserialize)]
pub struct Id {
    id: String,
}

pub async fn open(
    State(state): State<Shared>,
    Json(request): Json<Id>,
) -> Result<Json<Value>, ApiError> {
    refuse_while_searching(&state)?;
    let site = state
        .sites()
        .into_iter()
        .find(|site| site.id == request.id)
        .ok_or_else(|| ApiError::not_found(format!("no site {}", request.id)))?;
    switch(&state, site)?;
    Ok(site_json(&state))
}

pub async fn delete(
    State(state): State<Shared>,
    Json(request): Json<Id>,
) -> Result<Json<Value>, ApiError> {
    let sites = state.sites();
    if !sites.iter().any(|site| site.id == request.id) {
        return Err(ApiError::not_found(format!("no site {}", request.id)));
    }
    let open = state.site.lock().expect("site lock").id.clone();
    if open == request.id {
        refuse_while_searching(&state)?;
    }
    state.remove_site_file(&request.id);
    if open == request.id {
        let next = sites
            .into_iter()
            .find(|site| site.id != request.id)
            .unwrap_or_else(|| new_site("Site", &[]));
        *state.site.lock().expect("site lock") = next;
        state.save_site()?;
    }
    Ok(list(State(state)).await)
}

/// The open site with the profiles its devices use, to take to another computer.
pub async fn export(State(state): State<Shared>) -> Response {
    let site = state.site.lock().expect("site lock").clone();
    let profiles: Vec<Profile> = state
        .profiles
        .lock()
        .expect("profiles lock")
        .iter()
        .filter(|profile| {
            site.devices
                .iter()
                .any(|device| device.profile.as_ref() == Some(&profile.id))
        })
        .cloned()
        .collect();
    let body = json!({ "modbusio_site": 1, "site": site, "profiles": profiles });
    let name = format!("{}.modbusio-site.json", crate::profile::slug(&site.name));
    download(&name, "application/json", body.to_string())
}

#[derive(Deserialize)]
pub struct SiteFile {
    site: Site,
    #[serde(default)]
    profiles: Vec<Profile>,
}

/// A site file brought in: opened as a new site; its profiles join unless one of that id exists.
/// The file is checked whole before anything of it is kept.
pub async fn import(
    State(state): State<Shared>,
    Json(file): Json<SiteFile>,
) -> Result<Json<Value>, ApiError> {
    refuse_while_searching(&state)?;
    for profile in &file.profiles {
        profiles::check_to_keep(profile)?;
        profiles::check_id(&profile.id)?;
    }
    let name = check_name(&file.site.name)?;
    let mut site = new_site(if name.is_empty() { "Site" } else { &name }, &state.sites());
    for mut device in file.site.devices {
        // the key as modbusio writes it, so the device is found by it and is there once
        device.key = Target::parse_key(&device.key)
            .map_err(ApiError::bad_request)?
            .key();
        device.name = check_name(&device.name)?;
        site.add(device);
    }
    for profile in file.profiles {
        if state.profile(&profile.id).is_none() {
            state.save_profile(&profile)?;
            state.profiles.lock().expect("profiles lock").push(profile);
        }
    }
    switch(&state, site)?;
    Ok(site_json(&state))
}

#[derive(Deserialize)]
pub struct AddDevice {
    /// `tcp:10.0.0.5:502/1`, `serial:COM3:9600:8E1/5` …
    key: String,
    #[serde(default)]
    name: String,
}

pub async fn add_device(
    State(state): State<Shared>,
    Json(request): Json<AddDevice>,
) -> Result<Json<Value>, ApiError> {
    let target = Target::parse_key(request.key.trim()).map_err(ApiError::bad_request)?;
    let mut device = SiteDevice::new(target.key());
    device.name = check_name(&request.name)?;
    let added = state.site.lock().expect("site lock").add(device);
    if !added {
        return Err(ApiError::conflict(format!(
            "{target} is on the site already"
        )));
    }
    state.save_site()?;
    // asked once whether it answers, only on Modbus TCP: on a serial line another master may be
    // active, and nothing is sent there unasked
    let check = if target.endpoint.is_serial_line() {
        Value::Null
    } else {
        match scan::unanswered(&state.links, &target, ANSWER_TIMEOUT).await {
            None => json!({ "answered": true, "error": null }),
            Some(code) => json!({ "answered": false, "error": code }),
        }
    };
    let Json(mut site) = site_json(&state);
    site["check"] = check;
    Ok(Json(site))
}

#[derive(Deserialize)]
pub struct UpdateDevice {
    key: String,
    name: Option<String>,
    /// The id of its profile; an empty text removes it.
    profile: Option<String>,
}

pub async fn update_device(
    State(state): State<Shared>,
    Json(request): Json<UpdateDevice>,
) -> Result<Json<Value>, ApiError> {
    let name = request.name.as_deref().map(check_name).transpose()?;
    let profile = match request.profile.as_deref() {
        None => None,
        Some("") => Some(None),
        Some(id) => {
            if state.profile(id).is_none() {
                return Err(ApiError::not_found(format!("no profile {id}")));
            }
            Some(Some(id.to_string()))
        }
    };
    state.update_device(&request.key, |device| {
        if let Some(name) = name {
            device.name = name;
        }
        if let Some(profile) = profile {
            device.profile = profile;
        }
    })?;
    Ok(site_json(&state))
}

#[derive(Deserialize)]
pub struct Key {
    key: String,
}

#[derive(Deserialize)]
pub struct Line {
    /// The line as the site gives it: `COM3 9600 8E1`, `10.0.0.9:502`.
    line: String,
}

/// Removes every device of one line from the site at once. Nothing changes in the devices.
pub async fn remove_line(
    State(state): State<Shared>,
    Json(request): Json<Line>,
) -> Result<Json<Value>, ApiError> {
    {
        let mut site = state.site.lock().expect("site lock");
        let before = site.devices.len();
        site.devices.retain(|device| {
            Target::parse_key(&device.key).map_or(true, |target| {
                crate::scan::line_text(&target.endpoint) != request.line
            })
        });
        if site.devices.len() == before {
            return Err(ApiError::not_found(format!(
                "no line {} on this site",
                request.line
            )));
        }
    }
    state.save_site()?;
    Ok(site_json(&state))
}

pub async fn remove_device(
    State(state): State<Shared>,
    Json(request): Json<Key>,
) -> Result<Json<Value>, ApiError> {
    {
        let mut site = state.site.lock().expect("site lock");
        let before = site.devices.len();
        site.devices.retain(|device| device.key != request.key);
        if site.devices.len() == before {
            return Err(ApiError::not_found(format!(
                "no device {} on this site",
                request.key
            )));
        }
    }
    state.save_site()?;
    Ok(site_json(&state))
}
