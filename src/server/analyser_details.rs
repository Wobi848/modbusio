//! The bus analyser's details and exports (docs/concept-analyser.md): a unit's registers with
//! their readings and the profile's points, a register's changes over time, and the tables as CSV
//! and Excel. `analysing.rs` starts and stops listening.

use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use serde_json::{Value, json};

use super::analysing::Listening;
use super::{ApiError, Shared, csv_file, xlsx_file};
use crate::analyser::registers::Registers;
use crate::analyser::{Analysis, tables};
use crate::endpoint::Target;
use crate::explore::{self, Cell};
use crate::profile::{self, Profile};
use crate::protocol::Area;

const NOTHING_YET: &str = "nothing heard yet: start the bus analyser";

/// Runs `with` on the analysis kept, running or ended.
fn with_analysis<T>(
    state: &Shared,
    with: impl FnOnce(&Listening, &Analysis) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let analyser = state.analyser.lock().expect("analyser lock");
    let listening = analyser
        .as_ref()
        .ok_or_else(|| ApiError::conflict(NOTHING_YET))?;
    let analysis = listening.analysis.lock().expect("analysis lock");
    with(listening, &analysis)
}

#[derive(Deserialize)]
pub struct UnitQuery {
    unit: u8,
}

/// A unit's registers as the master uses them, with every reading, the answer times over time,
/// and the points of the profile the site gives it.
pub async fn unit(
    State(state): State<Shared>,
    Query(query): Query<UnitQuery>,
) -> Result<Json<Value>, ApiError> {
    let profile = site_profile(&state, query.unit)?;
    with_analysis(&state, |_, analysis| {
        let stats = analysis
            .units
            .get(&query.unit)
            .ok_or_else(|| ApiError::not_found(format!("unit {} was not heard", query.unit)))?;
        let mut rows = Vec::new();
        let mut order = None;
        for area in [
            Area::HoldingRegister,
            Area::InputRegister,
            Area::Coil,
            Area::DiscreteInput,
        ] {
            let seen: Vec<_> = analysis
                .registers
                .of_unit(query.unit)
                .filter(|((_, of, _), _)| *of == area)
                .collect();
            let cells: Vec<Cell> = seen
                .iter()
                .map(|((_, _, address), seen)| Cell {
                    address: *address,
                    value: seen.last,
                    error: None,
                })
                .collect();
            let (readings, likely) = explore::interpret(&cells);
            order = order.or(likely.filter(|_| !area.is_bits()));
            for (reading, ((_, _, _), seen)) in readings.into_iter().zip(seen) {
                rows.push(json!({
                    "area": area,
                    "reads": seen.reads,
                    "writes": seen.writes,
                    "changes": seen.changes,
                    "min": seen.min,
                    "max": seen.max,
                    "last_at": seen.last_at,
                    "reading": reading,
                }));
            }
        }
        let points = profile
            .as_ref()
            .map(|profile| points(profile, query.unit, &analysis.registers))
            .unwrap_or_default();
        Ok(Json(json!({
            "unit": query.unit,
            "order": order,
            "registers": rows,
            "points": points,
            "answer_times": stats.answer_history,
        })))
    })
}

/// The profile of the site's device on the analyser's port with this unit, if it has one.
fn site_profile(state: &Shared, unit: u8) -> Result<Option<Profile>, ApiError> {
    let port = {
        let analyser = state.analyser.lock().expect("analyser lock");
        let listening = analyser
            .as_ref()
            .ok_or_else(|| ApiError::conflict(NOTHING_YET))?;
        // a file names no port the site knows
        let Some(settings) = listening.port() else {
            return Ok(None);
        };
        settings.port.clone()
    };
    let site = state.site.lock().expect("site lock");
    let device = site.devices.iter().find(|device| {
        Target::parse_key(&device.key).is_ok_and(|target| {
            target.unit == unit && device.key.starts_with(&format!("serial:{port}:"))
        })
    });
    Ok(device
        .and_then(|device| device.profile.clone())
        .and_then(|id| state.profile(&id)))
}

/// The profile's points whose registers were all seen, with their value as the profile reads it.
fn points(profile: &Profile, unit: u8, registers: &Registers) -> Vec<Value> {
    let last = |area: Area, address: u16| registers.get(&(unit, area, address))?.last;
    profile
        .points
        .iter()
        .filter_map(|point| {
            let values: Vec<u16> = (0..point.count())
                .map(|offset| last(point.area, point.address.checked_add(offset)?))
                .collect::<Option<_>>()?;
            let exponent = match point.scale_register {
                Some(address) => last(point.area, address)? as i16,
                None => 0,
            };
            let reading = profile::evaluate(point, point.order(profile), &values, exponent);
            Some(json!({
                "name": point.name,
                "group": point.group,
                "area": point.area,
                "address": point.address,
                "count": point.count(),
                "text": reading.text,
            }))
        })
        .collect()
}

#[derive(Deserialize)]
pub struct RegisterQuery {
    unit: u8,
    area: Area,
    address: u16,
}

/// A register's first value and its changes, with when they were heard.
pub async fn register(
    State(state): State<Shared>,
    Query(query): Query<RegisterQuery>,
) -> Result<Json<Value>, ApiError> {
    with_analysis(&state, |_, analysis| {
        let seen = analysis
            .registers
            .get(&(query.unit, query.area, query.address))
            .ok_or_else(|| ApiError::not_found("this register was not heard"))?;
        Ok(Json(json!({ "history": seen.history })))
    })
}

#[derive(Deserialize)]
pub struct Export {
    /// `units`, `exchanges` or `registers`.
    table: Option<String>,
    /// The viewer's time zone, as JavaScript gives it negated (`-getTimezoneOffset()`).
    #[serde(default)]
    offset: i32,
}

fn all_tables(state: &Shared, offset: i32) -> Result<Vec<crate::export::Table>, ApiError> {
    with_analysis(state, |listening, analysis| {
        Ok(vec![
            tables::units(analysis),
            tables::exchanges(analysis, listening.started, offset),
            tables::registers(analysis, listening.started, offset),
        ])
    })
}

pub async fn csv(
    State(state): State<Shared>,
    Query(export): Query<Export>,
) -> Result<Response, ApiError> {
    let index = match export.table.as_deref().unwrap_or("exchanges") {
        "units" => 0,
        "exchanges" => 1,
        "registers" => 2,
        other => {
            return Err(ApiError::bad_request(format!(
                "{other} is not units, exchanges or registers"
            )));
        }
    };
    let table = all_tables(&state, export.offset)?.swap_remove(index);
    let name = format!("modbusio-trunk-{}.csv", table.title.to_lowercase());
    Ok(csv_file(&name, &table))
}

pub async fn xlsx(
    State(state): State<Shared>,
    Query(export): Query<Export>,
) -> Result<Response, ApiError> {
    xlsx_file("modbusio-trunk.xlsx", &all_tables(&state, export.offset)?)
}
