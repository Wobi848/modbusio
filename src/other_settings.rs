//! Devices at other serial settings (docs/concept-other-settings.md): every unit asked at every
//! setting, and the units named that answer only at settings other than the trunk's. `verdict`
//! has no I/O; `search` asks the line, after listening first.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use crate::diagnosis::Note;
use crate::endpoint::{BAUD_RATES, Endpoint, SerialSettings, Target};
use crate::links::Links;
use crate::protocol::rtu;
use crate::scan::{Found, Probed, Search, probe_unit};

/// Speeds asked by default; the others (1200, 2400, 4800, 57600) only when asked for.
pub const USUAL_SPEEDS: [u32; 4] = [9600, 19200, 38400, 115200];
/// Answer waited for at least, and the device's own time on top of 30 characters on the line.
const LEAST_WAIT: Duration = Duration::from_millis(100);
const DEVICE_WAIT: Duration = Duration::from_millis(50);

/// What one setting heard: the units that answered, and those whose answers came damaged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heard {
    pub settings: String,
    pub answered: BTreeSet<u8>,
    pub damaged: BTreeSet<u8>,
}

/// Where one unit answered, for the report.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UnitSettings {
    pub unit: u8,
    pub answers_at: Vec<String>,
    pub damaged_at: Vec<String>,
}

/// The settings as people write them: `19200 8E1`.
pub fn settings_text(settings: &SerialSettings) -> String {
    format!("{} {}", settings.baud, settings.frame_text())
}

/// What the line said, per unit, and in words. `trunk` is the setting the master uses;
/// `master_asks` the units it asks and `master_silent` those of them that never answered, where
/// the bus analyser heard the same port (empty otherwise).
pub fn verdict(
    heard: &[Heard],
    trunk: &str,
    master_asks: &BTreeSet<u8>,
    master_silent: &BTreeSet<u8>,
) -> (Vec<UnitSettings>, Vec<Note>) {
    let mut units: BTreeMap<u8, UnitSettings> = BTreeMap::new();
    for setting in heard {
        let entry = |units: &mut BTreeMap<u8, UnitSettings>, unit: u8| {
            units.entry(unit).or_insert_with(|| UnitSettings {
                unit,
                answers_at: Vec::new(),
                damaged_at: Vec::new(),
            });
        };
        for &unit in &setting.answered {
            entry(&mut units, unit);
            units
                .get_mut(&unit)
                .expect("entered")
                .answers_at
                .push(setting.settings.clone());
        }
        for &unit in &setting.damaged {
            entry(&mut units, unit);
            units
                .get_mut(&unit)
                .expect("entered")
                .damaged_at
                .push(setting.settings.clone());
        }
    }
    let mut notes = Vec::new();
    let mut right = 0;
    for found in units.values() {
        let at_trunk = found.answers_at.iter().any(|settings| settings == trunk);
        if at_trunk {
            right += 1;
            continue;
        }
        if !found.answers_at.is_empty() {
            notes.push(Note::AtOtherSettings {
                unit: found.unit,
                found: found.answers_at.join(", "),
                trunk: trunk.to_string(),
            });
        } else if found.damaged_at.iter().any(|settings| settings == trunk) {
            notes.push(Note::DamagedAtTrunk {
                unit: found.unit,
                trunk: trunk.to_string(),
            });
        }
    }
    if !master_asks.is_empty() && !master_silent.is_empty() {
        let silent = master_silent
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        for found in units.values() {
            let at_trunk = found.answers_at.iter().any(|settings| settings == trunk);
            if at_trunk && !master_asks.contains(&found.unit) {
                notes.push(Note::MeantAs {
                    unit: found.unit,
                    silent: silent.clone(),
                });
            }
        }
    }
    if notes.is_empty() && right > 0 {
        notes.push(Note::AllAtTrunk {
            count: right,
            trunk: trunk.to_string(),
        });
    }
    (units.into_values().collect(), notes)
}

/// The settings to ask, the trunk's first, then the other frames at its speed, then the other
/// speeds; slow and rare speeds only with `all_speeds`.
pub fn candidates(trunk: &SerialSettings, all_speeds: bool) -> Vec<SerialSettings> {
    let mut speeds = vec![trunk.baud];
    let others: Vec<u32> = if all_speeds {
        BAUD_RATES.to_vec()
    } else {
        USUAL_SPEEDS.to_vec()
    };
    speeds.extend(others.into_iter().filter(|baud| *baud != trunk.baud));
    let mut settings = vec![trunk.clone()];
    for baud in speeds {
        for (parity, stop_bits) in SerialSettings::frames() {
            let candidate = SerialSettings {
                port: trunk.port.clone(),
                baud,
                parity,
                stop_bits,
            };
            if candidate != *trunk {
                settings.push(candidate);
            }
        }
    }
    settings
}

/// How long to wait for an answer at `baud`: 30 characters and the device's own time, at least
/// 100 ms.
pub fn wait_at(baud: u32) -> Duration {
    (Duration::from_micros(rtu::character_micros(baud) * 30) + DEVICE_WAIT).max(LEAST_WAIT)
}

/// What the search is given: the trunk's setting, the units and speeds to ask, the answer time
/// (none: [`wait_at`]), and what the bus analyser knows of the master on this port.
pub struct Asking {
    pub trunk: SerialSettings,
    pub units: Vec<u8>,
    pub all_speeds: bool,
    pub wait: Option<Duration>,
    pub master_asks: BTreeSet<u8>,
    pub master_silent: BTreeSet<u8>,
    /// Listening first, per setting.
    pub listen_each: Duration,
}

/// Listening first takes this long at each usual setting.
pub const LISTEN_EACH: Duration = Duration::from_millis(1500);

impl Asking {
    /// The time the search takes at most, for the page to say before.
    pub fn estimate(&self) -> Duration {
        let listening = self.listen_each * (BAUD_RATES.len() * 2) as u32;
        listening
            + candidates(&self.trunk, self.all_speeds)
                .iter()
                .map(|settings| {
                    self.wait.unwrap_or_else(|| wait_at(settings.baud)) * self.units.len() as u32
                })
                .sum()
    }
}

/// Listens first and stops when another master talks; then asks every unit at every setting.
/// Units found join the report at the setting they answer at (the trunk's where they do).
pub async fn search(links: Arc<Links>, asking: Asking, search: Arc<Search>) {
    let port = asking.trunk.port.clone();
    search.update(|report| {
        report.phase = "listening".into();
        report.estimate_seconds = Some(asking.estimate().as_secs());
    });
    // modbusio's own connection would hold the port while listening
    links.close(&Endpoint::Serial(asking.trunk.clone())).await;
    match crate::listen::listen(&port, asking.listen_each).await {
        Ok(listened) => {
            if let Some(settings) = listened.active {
                search.note(Note::OtherMaster { port, settings });
                return;
            }
        }
        Err(error) => {
            search.note(Note::LineFailed {
                line: port,
                reason: error.to_string(),
            });
            return;
        }
    }
    let settings = candidates(&asking.trunk, asking.all_speeds);
    search.update(|report| {
        report.phase = "other_settings".into();
        report.total = settings.len() * asking.units.len();
        report.done = 0;
    });
    let mut heard = Vec::new();
    for setting in &settings {
        if search.stopped() {
            break;
        }
        let endpoint = Endpoint::Serial(setting.clone());
        let wait = asking.wait.unwrap_or_else(|| wait_at(setting.baud));
        let mut this = Heard {
            settings: settings_text(setting),
            ..Heard::default()
        };
        for &unit in &asking.units {
            if search.stopped() {
                break;
            }
            let target = Target {
                endpoint: endpoint.clone(),
                unit,
            };
            match probe_unit(&links, &target, wait, 0).await {
                Probed::Answered => {
                    this.answered.insert(unit);
                }
                Probed::Damaged => {
                    this.damaged.insert(unit);
                }
                Probed::Failed(error) => {
                    search.note(Note::LineFailed {
                        line: port,
                        reason: error.to_string(),
                    });
                    return;
                }
                Probed::Gateway | Probed::Silent | Probed::OtherUnit(_) => {}
            }
            search.update(|report| report.done += 1);
        }
        heard.push(this);
    }
    links.close(&Endpoint::Serial(asking.trunk.clone())).await;
    let trunk = settings_text(&asking.trunk);
    let (units, notes) = verdict(&heard, &trunk, &asking.master_asks, &asking.master_silent);
    for found in &units {
        let at = found
            .answers_at
            .iter()
            .find(|settings| **settings == trunk)
            .or(found.answers_at.first());
        let Some(setting) =
            at.and_then(|text| settings.iter().find(|item| settings_text(item) == *text))
        else {
            continue;
        };
        search.found(Found::new(Target {
            endpoint: Endpoint::Serial(setting.clone()),
            unit: found.unit,
        }));
    }
    if units.is_empty() {
        search.note(Note::NothingAnswered { line: port });
    }
    for note in notes {
        search.note(note);
    }
    search.update(|report| report.other_settings = units);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::Parity;

    fn heard(settings: &str, answered: &[u8], damaged: &[u8]) -> Heard {
        Heard {
            settings: settings.into(),
            answered: answered.iter().copied().collect(),
            damaged: damaged.iter().copied().collect(),
        }
    }

    const TRUNK: &str = "19200 8E1";

    fn none() -> BTreeSet<u8> {
        BTreeSet::new()
    }

    #[test]
    fn a_unit_at_another_speed_is_named_with_where_it_answers() {
        let (units, notes) = verdict(
            &[
                heard(TRUNK, &[1, 5], &[]),
                heard("9600 8E1", &[9], &[]),
                heard("9600 8N1", &[9], &[]),
            ],
            TRUNK,
            &none(),
            &none(),
        );
        assert_eq!(
            notes,
            [Note::AtOtherSettings {
                unit: 9,
                found: "9600 8E1, 9600 8N1".into(),
                trunk: TRUNK.into()
            }]
        );
        assert_eq!(
            units.iter().map(|unit| unit.unit).collect::<Vec<_>>(),
            [1, 5, 9]
        );
    }

    #[test]
    fn a_lenient_unit_answering_at_the_trunks_setting_too_is_right() {
        let (_, notes) = verdict(
            &[heard(TRUNK, &[3], &[]), heard("19200 8N2", &[3], &[])],
            TRUNK,
            &none(),
            &none(),
        );
        assert_eq!(
            notes,
            [Note::AllAtTrunk {
                count: 1,
                trunk: TRUNK.into()
            }]
        );
    }

    #[test]
    fn a_strict_unit_with_other_parity_answers_only_at_its_frame() {
        // the trunk runs 8E1; the device was left at 8N1 and refuses frames with a parity bit
        let (_, notes) = verdict(
            &[heard(TRUNK, &[1], &[]), heard("19200 8N1", &[1, 4], &[])],
            TRUNK,
            &none(),
            &none(),
        );
        assert_eq!(
            notes,
            [Note::AtOtherSettings {
                unit: 4,
                found: "19200 8N1".into(),
                trunk: TRUNK.into()
            }]
        );
    }

    #[test]
    fn damaged_answers_at_the_trunks_setting_point_to_parity_or_stop_bits() {
        let (units, notes) = verdict(&[heard(TRUNK, &[1], &[5])], TRUNK, &none(), &none());
        assert_eq!(
            notes,
            [Note::DamagedAtTrunk {
                unit: 5,
                trunk: TRUNK.into()
            }]
        );
        assert_eq!(units[1].damaged_at, [TRUNK]);
    }

    #[test]
    fn a_unit_the_master_never_asks_while_others_are_silent_may_be_meant_there() {
        let asks: BTreeSet<u8> = [1, 9].into();
        let silent: BTreeSet<u8> = [9].into();
        let (_, notes) = verdict(&[heard(TRUNK, &[1, 19], &[])], TRUNK, &asks, &silent);
        assert_eq!(
            notes,
            [Note::MeantAs {
                unit: 19,
                silent: "9".into()
            }]
        );
        assert!(
            notes[0]
                .to_string()
                .contains("units the master asks stay silent (9)")
        );
    }

    #[test]
    fn nothing_heard_says_nothing_here() {
        let (units, notes) = verdict(&[heard(TRUNK, &[], &[])], TRUNK, &none(), &none());
        assert!(units.is_empty() && notes.is_empty());
    }

    fn trunk() -> SerialSettings {
        SerialSettings {
            port: "COM3".into(),
            baud: 19200,
            parity: Parity::Even,
            stop_bits: 1,
        }
    }

    #[test]
    fn the_trunks_setting_comes_first_and_the_usual_speeds_follow() {
        let asked: Vec<String> = candidates(&trunk(), false)
            .iter()
            .map(settings_text)
            .collect();
        assert_eq!(asked[0], TRUNK);
        assert_eq!(asked[1..4], ["19200 8N1", "19200 8N2", "19200 8O1"]);
        assert_eq!(asked.len(), 16);
        assert!(!asked.iter().any(|text| text.starts_with("1200 ")));
        assert_eq!(candidates(&trunk(), true).len(), 32);
    }

    #[test]
    fn the_wait_is_longer_at_slow_speeds() {
        assert_eq!(wait_at(19200), Duration::from_millis(100));
        assert_eq!(
            wait_at(1200),
            Duration::from_micros(9166 * 30) + DEVICE_WAIT
        );
        let asking = Asking {
            trunk: trunk(),
            units: (1..=247).collect(),
            all_speeds: false,
            wait: None,
            master_asks: none(),
            master_silent: none(),
            listen_each: LISTEN_EACH,
        };
        // listening at 16 settings, then 247 units at 16 settings
        assert_eq!(asking.estimate().as_secs(), 24 + 395);
    }
}
