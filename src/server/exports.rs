//! Exports of the site: the device list, a device's points with values read now, the whole site
//! in one Excel file with a sheet per device.

use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;

use super::{ApiError, Shared, csv_file, xlsx_file};
use crate::endpoint::Target;
use crate::export::{Table, points_table};
use crate::scan::line_text;

fn devices_table(state: &Shared) -> Table {
    let site = state.site.lock().expect("site lock").clone();
    let profiles = state.profiles.lock().expect("profiles lock").clone();
    let mut table = Table::new(
        "Devices",
        &[
            "Name", "Line", "Unit", "Profile", "Vendor", "Product", "Version", "Key",
        ],
    );
    table.rows = site
        .devices
        .iter()
        .filter_map(|device| {
            let target = Target::parse_key(&device.key).ok()?;
            let profile = device
                .profile
                .as_ref()
                .and_then(|id| profiles.iter().find(|profile| &profile.id == id))
                .map(|profile| profile.name.clone());
            Some(vec![
                device.name.clone(),
                line_text(&target.endpoint),
                target.unit.to_string(),
                profile.unwrap_or_default(),
                device.vendor.clone().unwrap_or_default(),
                device.product.clone().unwrap_or_default(),
                device.revision.clone().unwrap_or_default(),
                device.key.clone(),
            ])
        })
        .collect();
    table
}

pub async fn devices_csv(State(state): State<Shared>) -> Response {
    csv_file("modbusio-devices.csv", &devices_table(&state))
}

#[derive(Deserialize)]
pub struct Key {
    key: String,
}

/// A device's points, read now.
async fn device_table(state: &Shared, key: &str) -> Result<Table, ApiError> {
    let (device, _) = state.device(key)?;
    let profile = super::devices::profile_of(state, key)?;
    let (readings, _) = super::devices::read_device(state, key, &profile, &profile.points).await?;
    let title = if device.name.is_empty() {
        profile.name.clone()
    } else {
        device.name.clone()
    };
    Ok(points_table(&title, &profile.points, &readings))
}

fn file_name(table: &Table, ending: &str) -> String {
    format!("{}.{ending}", crate::profile::slug(&table.title))
}

pub async fn points_csv(
    State(state): State<Shared>,
    Query(query): Query<Key>,
) -> Result<Response, ApiError> {
    let table = device_table(&state, &query.key).await?;
    Ok(csv_file(&file_name(&table, "csv"), &table))
}

pub async fn points_xlsx(
    State(state): State<Shared>,
    Query(query): Query<Key>,
) -> Result<Response, ApiError> {
    let table = device_table(&state, &query.key).await?;
    xlsx_file(&file_name(&table, "xlsx"), &[table])
}

/// The device list and a sheet per device with a profile, each read now.
pub async fn site_xlsx(State(state): State<Shared>) -> Result<Response, ApiError> {
    let site = state.site.lock().expect("site lock").clone();
    let mut tables = vec![devices_table(&state)];
    for device in site
        .devices
        .iter()
        .filter(|device| device.profile.is_some())
    {
        if let Ok(table) = device_table(&state, &device.key).await {
            tables.push(table);
        }
    }
    xlsx_file(
        &format!("{}.xlsx", crate::profile::slug(&site.name)),
        &tables,
    )
}
