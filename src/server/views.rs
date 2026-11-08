//! What the API sends: devices, points with their readings, log entries (docs/api.md).

use serde::Serialize;

use crate::access::Limits;
use crate::endpoint::{Endpoint, Target};
use crate::profile::{Point, Profile, Reading};
use crate::scan;
use crate::site::SiteDevice;
use crate::values::number_text;
use crate::write::{Entry, Journal};

#[derive(Serialize)]
pub struct DeviceView {
    pub key: String,
    pub name: String,
    /// `tcp`, `rtu-tcp`, `serial`.
    pub kind: &'static str,
    /// The line for people: `10.0.0.5:502`, `COM3 9600 8E1`.
    pub line: String,
    pub unit: u8,
    pub profile: Option<String>,
    pub profile_name: Option<String>,
    pub points: usize,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub revision: Option<String>,
    pub limits: Limits,
}

impl DeviceView {
    pub fn new(device: &SiteDevice, profiles: &[Profile]) -> Option<DeviceView> {
        let target = Target::parse_key(&device.key).ok()?;
        let profile = device
            .profile
            .as_ref()
            .and_then(|id| profiles.iter().find(|profile| &profile.id == id));
        Some(DeviceView {
            key: device.key.clone(),
            name: device.name.clone(),
            kind: match target.endpoint {
                Endpoint::Tcp(_) => "tcp",
                Endpoint::RtuOverTcp(_) => "rtu-tcp",
                Endpoint::Serial(_) => "serial",
            },
            line: scan::line_text(&target.endpoint),
            unit: target.unit,
            profile: device.profile.clone(),
            profile_name: profile.map(|profile| profile.name.clone()),
            points: profile.map_or(0, |profile| profile.points.len()),
            vendor: device.vendor.clone(),
            product: device.product.clone(),
            revision: device.revision.clone(),
            limits: device.limits,
        })
    }
}

#[derive(Serialize)]
pub struct PointView<'a> {
    #[serde(flatten)]
    pub point: &'a Point,
    /// The word order it is read in.
    pub effective_order: &'static str,
    /// Items it takes on the wire.
    pub count: u16,
    pub reading: Option<&'a Reading>,
}

impl<'a> PointView<'a> {
    pub fn new(point: &'a Point, profile: &Profile, reading: Option<&'a Reading>) -> Self {
        PointView {
            point,
            effective_order: point.order(profile).name(),
            count: point.count(),
            reading,
        }
    }
}

#[derive(Serialize)]
pub struct EntryView {
    #[serde(flatten)]
    pub entry: Entry,
    /// Still written: not put back yet.
    pub standing: bool,
    /// The device confirmed but holds another value.
    pub kept_other: bool,
    /// The value restoring would put back, in registers.
    pub original: Option<Vec<u16>>,
}

impl EntryView {
    pub fn all(journal: &Journal) -> Vec<EntryView> {
        let standing: Vec<u64> = journal.changed().iter().map(|entry| entry.id).collect();
        journal
            .entries
            .iter()
            .map(|entry| EntryView {
                standing: standing.contains(&entry.id),
                kept_other: entry.kept_other(),
                original: standing
                    .contains(&entry.id)
                    .then(|| journal.original(entry))
                    .flatten(),
                entry: entry.clone(),
            })
            .collect()
    }
}

/// A raw value as text: registers in decimal.
pub fn registers_text(registers: &[u16]) -> String {
    registers
        .iter()
        .map(|register| number_text(f64::from(*register)))
        .collect::<Vec<_>>()
        .join(" ")
}
