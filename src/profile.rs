//! Profiles (concept section 6): what the registers of a kind of device mean. A profile is a list
//! of points; a point names an address, how to decode it and how to show it.

use serde::{Deserialize, Serialize};

use crate::protocol::Area;
use crate::values::{self, DataType, Raw, Scale, WordOrder};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub model: String,
    /// `own` (made in modbusio) or `imported` (from a vendor's list).
    #[serde(default = "own")]
    pub origin: String,
    /// The file or document it was imported from.
    #[serde(default)]
    pub source: String,
    /// The word order of multi-register values unless a point says otherwise.
    #[serde(default)]
    pub order: WordOrder,
    #[serde(default)]
    pub points: Vec<Point>,
}

fn own() -> String {
    "own".into()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub name: String,
    #[serde(default)]
    pub group: String,
    pub area: Area,
    /// On the wire, from 0.
    pub address: u16,
    #[serde(rename = "type")]
    pub kind: DataType,
    /// Overrides the profile's word order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<WordOrder>,
    #[serde(default, flatten)]
    pub scale: Scale,
    /// A register of the same area holding a power of ten (s16) for this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_register: Option<u16>,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub description: String,
    /// Numbers to words: `0 = off`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<(i64, String)>,
    #[serde(default)]
    pub writable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The address as the source wrote it (`40011`), kept beside the one on the wire.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_address: String,
}

impl Point {
    pub fn new(name: &str, area: Area, address: u16, kind: DataType) -> Point {
        Point {
            name: name.into(),
            group: String::new(),
            area,
            address,
            kind: if area.is_bits() { DataType::Bool } else { kind },
            order: None,
            scale: Scale::default(),
            scale_register: None,
            unit: String::new(),
            description: String::new(),
            states: Vec::new(),
            writable: false,
            min: None,
            max: None,
            source_address: String::new(),
        }
    }

    /// How many items it takes.
    pub fn count(&self) -> u16 {
        if self.area.is_bits() {
            1
        } else {
            self.kind.registers()
        }
    }

    pub fn order(&self, profile: &Profile) -> WordOrder {
        self.order.unwrap_or(profile.order)
    }

    /// Checks what a file or the interface brought: names, ranges, types that fit the area.
    pub fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("a point needs a name".into());
        }
        if u32::from(self.address) + u32::from(self.count()) > 0x1_0000 {
            return Err(format!("{} runs past address 65535", self.name));
        }
        if self.area.is_bits() && self.kind != DataType::Bool {
            return Err(format!(
                "{} is in {} and can only be bool",
                self.name, self.area
            ));
        }
        if !self.scale.factor.is_finite()
            || !self.scale.offset.is_finite()
            || self.scale.factor == 0.0
        {
            return Err(format!(
                "{}: factor must be a number other than 0",
                self.name
            ));
        }
        if self.writable && !self.area.writable() {
            return Err(format!("{}: {} cannot be written", self.name, self.area));
        }
        if let (Some(min), Some(max)) = (self.min, self.max)
            && min > max
        {
            return Err(format!("{}: min is above max", self.name));
        }
        Ok(())
    }
}

impl Profile {
    pub fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("a profile needs a name".into());
        }
        self.points.iter().try_for_each(Point::check)
    }
}

/// A point's value as read, ready to show.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Reading {
    /// The number after scaling; `None` for text or when reading failed.
    pub value: Option<f64>,
    /// For people: `23.15 °C`, `heating`, `HM5012345678`.
    pub text: String,
    /// The registers as received, hex, for the details.
    pub raw: Vec<String>,
    pub error: Option<String>,
}

impl Reading {
    pub fn failed(code: String) -> Reading {
        Reading {
            value: None,
            text: String::new(),
            raw: Vec::new(),
            error: Some(code),
        }
    }
}

/// Decodes a point from its registers (bits as 0 and 1) and the power of ten its scale register
/// holds.
pub fn evaluate(point: &Point, order: WordOrder, registers: &[u16], exponent: i16) -> Reading {
    let raw_hex = registers
        .iter()
        .map(|register| format!("{register:04X}"))
        .collect();
    let raw = values::decode(registers, point.kind, order);
    let text_of = |number: f64| {
        let state = point
            .states
            .iter()
            .find(|(state, _)| (*state as f64 - number).abs() < 1e-9)
            .map(|(_, text)| text.clone());
        state.unwrap_or_else(|| with_unit(&values::number_text(number), &point.unit))
    };
    match raw {
        Raw::Text(text) => Reading {
            value: None,
            text,
            raw: raw_hex,
            error: None,
        },
        Raw::Float(number) if !number.is_finite() => Reading {
            value: None,
            text: values::number_text(number),
            raw: raw_hex,
            error: None,
        },
        other => {
            let number = other.as_f64().unwrap_or_default();
            let value = point.scale.apply(number, exponent);
            Reading {
                value: Some(value),
                text: text_of(value),
                raw: raw_hex,
                error: None,
            }
        }
    }
}

/// The registers (bits as 0 and 1) to write so the device holds `input`: a number in the point's
/// unit (decimal point or comma), a state word, or on/off for bits. Checks the point's limits.
pub fn registers_for(
    point: &Point,
    order: WordOrder,
    input: &str,
    exponent: i16,
) -> Result<Vec<u16>, String> {
    let input = input.trim();
    if point.area.is_bits() || point.kind == DataType::Bool {
        let on = match input.to_lowercase().as_str() {
            "1" | "on" | "true" | "ein" | "an" => true,
            "0" | "off" | "false" | "aus" => false,
            _ => return Err(format!("{input} is not on or off (1 or 0)")),
        };
        return Ok(vec![u16::from(on)]);
    }
    if !point.kind.is_numeric() || matches!(point.kind, DataType::Bit(_)) {
        return Err(format!(
            "{} cannot be written as a value",
            point.kind.name()
        ));
    }
    let value = match point
        .states
        .iter()
        .find(|(_, word)| word.eq_ignore_ascii_case(input))
    {
        Some((state, _)) => *state as f64,
        None => input
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("{input} is not a number"))?,
    };
    if point.min.is_some_and(|min| value < min) || point.max.is_some_and(|max| value > max) {
        return Err(format!(
            "{input} is outside {} … {}",
            point.min.map_or("".into(), values::number_text),
            point.max.map_or("".into(), values::number_text)
        ));
    }
    let raw = point
        .scale
        .invert(value, exponent)
        .ok_or("the point's factor is 0")?;
    let raw = if matches!(point.kind, DataType::F32 | DataType::F64) {
        raw
    } else {
        round_near(raw)
    };
    values::encode(raw, point.kind, order).ok_or_else(|| {
        format!(
            "{input} does not fit {} with factor {}",
            point.kind.name(),
            values::number_text(point.scale.factor)
        )
    })
}

/// Takes away the float noise of a division (`235.00000000000003`) before an integer is encoded.
fn round_near(value: f64) -> f64 {
    let rounded = value.round();
    if (rounded - value).abs() < 1e-6 {
        rounded
    } else {
        value
    }
}

fn with_unit(number: &str, unit: &str) -> String {
    if unit.is_empty() {
        number.into()
    } else {
        format!("{number} {unit}")
    }
}

/// A file name of letters, digits and dashes from a profile's name.
pub fn slug(name: &str) -> String {
    let slug: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "profile".into()
    } else {
        slug.chars().take(60).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(kind: DataType) -> Point {
        Point::new("Flow", Area::HoldingRegister, 10, kind)
    }

    #[test]
    fn a_scaled_value_reads_with_its_unit() {
        let mut flow = point(DataType::S16);
        flow.scale.factor = 0.1;
        flow.unit = "°C".into();
        let reading = evaluate(&flow, WordOrder::Abcd, &[632], 0);
        assert_eq!(reading.text, "63.2 °C");
        assert_eq!(reading.raw, ["0278"]);
    }

    #[test]
    fn a_state_number_reads_as_its_word() {
        let mut mode = point(DataType::U16);
        mode.states = vec![(0, "off".into()), (1, "heating".into())];
        assert_eq!(evaluate(&mode, WordOrder::Abcd, &[1], 0).text, "heating");
        assert_eq!(evaluate(&mode, WordOrder::Abcd, &[7], 0).text, "7");
    }

    #[test]
    fn a_scale_register_shifts_the_decimal_point() {
        let mut volume = point(DataType::U16);
        volume.unit = "m³".into();
        assert_eq!(
            evaluate(&volume, WordOrder::Abcd, &[8821], -2).text,
            "88.21 m³"
        );
    }

    #[test]
    fn points_that_cannot_be_are_refused() {
        let mut coil = Point::new("Run", Area::Coil, 0, DataType::F32);
        assert_eq!(coil.kind, DataType::Bool, "bits are always bool");
        coil.scale.factor = 0.0;
        assert!(coil.check().is_err());
        let mut input = Point::new("Temp", Area::InputRegister, 65535, DataType::F32);
        assert!(input.check().is_err(), "runs past the end");
        input.address = 0;
        input.writable = true;
        assert!(input.check().is_err(), "input registers cannot be written");
    }

    #[test]
    fn a_profile_file_keeps_its_fields() {
        let mut flow = point(DataType::F32);
        flow.order = Some(WordOrder::Cdab);
        flow.scale_register = Some(11);
        flow.states = vec![(1, "on".into())];
        let profile = Profile {
            id: "hm-50".into(),
            name: "HM-50".into(),
            vendor: String::new(),
            model: String::new(),
            origin: "own".into(),
            source: String::new(),
            order: WordOrder::Abcd,
            points: vec![flow],
        };
        let text = serde_json::to_string(&profile).expect("serialises");
        assert!(text.contains("\"type\":\"f32\""), "{text}");
        assert!(text.contains("\"order\":\"CDAB\""), "{text}");
        let back: Profile = serde_json::from_str(&text).expect("loads");
        assert_eq!(back, profile);
        let old: Profile = serde_json::from_str(r#"{"id":"x","name":"X"}"#).expect("loads");
        assert_eq!(old.origin, "own");
        let short: Point =
            serde_json::from_str(r#"{"name":"a","area":"coil","address":0,"type":"bool"}"#)
                .expect("loads");
        assert_eq!(short.scale, Scale::default());
    }

    #[test]
    fn an_input_becomes_the_registers_to_write() {
        let mut set_point = point(DataType::S16);
        set_point.scale.factor = 0.1;
        set_point.min = Some(5.0);
        set_point.max = Some(30.0);
        assert_eq!(
            registers_for(&set_point, WordOrder::Abcd, "23,5", 0),
            Ok(vec![235])
        );
        assert!(registers_for(&set_point, WordOrder::Abcd, "31", 0).is_err());
        assert!(registers_for(&set_point, WordOrder::Abcd, "warm", 0).is_err());
        let mut mode = point(DataType::U16);
        mode.states = vec![(2, "cooling".into())];
        assert_eq!(
            registers_for(&mode, WordOrder::Abcd, "Cooling", 0),
            Ok(vec![2])
        );
        let run = Point::new("Run", Area::Coil, 0, DataType::Bool);
        assert_eq!(registers_for(&run, WordOrder::Abcd, "on", 0), Ok(vec![1]));
        let fraction = point(DataType::U16);
        assert!(registers_for(&fraction, WordOrder::Abcd, "2.5", 0).is_err());
    }

    #[test]
    fn slugs_are_safe_file_names() {
        assert_eq!(slug("HM-50 Heat meter (v2)"), "hm-50-heat-meter-v2");
        assert_eq!(slug("../.."), "profile");
    }
}
