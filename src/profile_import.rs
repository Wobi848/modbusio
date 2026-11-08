//! A vendor's register list (CSV or Excel rows) into profile points (concept section 6): which
//! column is what, which address convention the list uses, each row checked. The interface shows
//! the result beside values read from the device before anything is saved.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::profile::Point;
use crate::protocol::Area;
use crate::values::{DataType, Scale, WordOrder};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Name,
    Group,
    Area,
    Address,
    Type,
    Count,
    Unit,
    Factor,
    Offset,
    ScaleRegister,
    Description,
    Access,
    States,
    Order,
    Min,
    Max,
}

impl Role {
    const ALL: [Role; 16] = [
        Role::Name,
        Role::Group,
        Role::Area,
        Role::Address,
        Role::Type,
        Role::Count,
        Role::Unit,
        Role::Factor,
        Role::Offset,
        Role::ScaleRegister,
        Role::Description,
        Role::Access,
        Role::States,
        Role::Order,
        Role::Min,
        Role::Max,
    ];

    /// Header words, English and German, lower case; the first match in a header wins.
    fn words(self) -> &'static [&'static str] {
        match self {
            Role::Name => &[
                "name",
                "bezeichnung",
                "parameter",
                "signal",
                "datenpunkt",
                "variable",
                "tag",
            ],
            Role::Group => &["group", "gruppe", "category", "kategorie", "section"],
            Role::Area => &[
                "area",
                "bereich",
                "register type",
                "registertyp",
                "object type",
                "function",
                "funktion",
                "fc",
            ],
            Role::Address => &["address", "adresse", "register", "reg.", "offset", "addr"],
            Role::Type => &["data type", "datatype", "datentyp", "type", "typ", "format"],
            Role::Count => &[
                "count",
                "length",
                "länge",
                "anzahl",
                "size",
                "words",
                "registers",
            ],
            Role::Unit => &["unit", "einheit", "eng. unit"],
            Role::Factor => &[
                "factor",
                "faktor",
                "scale",
                "skalierung",
                "multiplier",
                "gain",
                "resolution",
                "auflösung",
            ],
            Role::Offset => &["value offset", "wert-offset", "nullpunkt"],
            Role::ScaleRegister => &[
                "scale register",
                "skalierungsregister",
                "sf register",
                "scale factor register",
            ],
            Role::Description => &[
                "description",
                "beschreibung",
                "comment",
                "kommentar",
                "remark",
                "bemerkung",
                "info",
            ],
            Role::Access => &["access", "zugriff", "r/w", "rw", "read/write", "mode"],
            Role::States => &[
                "states",
                "values",
                "zustände",
                "werte",
                "enum",
                "meaning",
                "bedeutung",
            ],
            Role::Order => &[
                "word order",
                "byte order",
                "wortreihenfolge",
                "endian",
                "order",
            ],
            Role::Min => &["min", "minimum"],
            Role::Max => &["max", "maximum"],
        }
    }
}

/// How the list counts addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Convention {
    /// As on the wire, from 0.
    ZeroBased,
    /// From 1: address 1 is 0 on the wire.
    OneBased,
    /// Five digits with the area in front: 40001 is holding register 0, 30001 input register 0,
    /// 10001 discrete input 0, 00001 coil 0. Six digits likewise (400001).
    Reference,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Options {
    /// Which row holds the column names; guessed when left out.
    pub header_row: Option<usize>,
    /// Which column is what; guessed when left out.
    pub columns: Option<BTreeMap<Role, usize>>,
    /// How the list counts; guessed when left out.
    pub convention: Option<Convention>,
    /// The area of rows that name none.
    pub area: Option<Area>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Imported {
    pub header_row: usize,
    pub headers: Vec<String>,
    pub columns: BTreeMap<Role, usize>,
    pub convention: Convention,
    /// Why this convention was taken, for the preview.
    pub convention_reason: String,
    pub points: Vec<Point>,
    /// Rows that could not be taken: row number from 1, as the spreadsheet shows it, and why.
    pub skipped: Vec<(usize, String)>,
}

pub fn import(rows: &[Vec<String>], options: &Options) -> Result<Imported, String> {
    let header_row = options
        .header_row
        .or_else(|| find_header(rows))
        .ok_or("no row looks like column names (name, address, type …)")?;
    let headers = rows
        .get(header_row)
        .cloned()
        .ok_or("the header row does not exist")?;
    let columns = options
        .columns
        .clone()
        .unwrap_or_else(|| guess_columns(&headers));
    if !columns.contains_key(&Role::Address) {
        return Err("no column holds the address".into());
    }
    let body: Vec<(usize, &Vec<String>)> = rows.iter().enumerate().skip(header_row + 1).collect();
    let cell = |row: &Vec<String>, role: Role| -> String {
        columns
            .get(&role)
            .and_then(|&column| row.get(column))
            .map(|text| text.trim().to_string())
            .unwrap_or_default()
    };
    let addresses: Vec<String> = body
        .iter()
        .map(|(_, row)| cell(row, Role::Address))
        .collect();
    let has_area = body
        .iter()
        .any(|(_, row)| area_of(&cell(row, Role::Area)).is_some());
    let (convention, convention_reason) = match options.convention {
        Some(convention) => (convention, "chosen".to_string()),
        None => guess_convention(&addresses, has_area),
    };
    let mut points = Vec::new();
    let mut skipped = Vec::new();
    for (index, row) in body {
        let address_text = cell(row, Role::Address);
        if address_text.is_empty() {
            continue;
        }
        match point_of(&|role| cell(row, role), convention, options.area) {
            Ok(point) => points.push(point),
            Err(reason) => skipped.push((index + 1, reason)),
        }
    }
    Ok(Imported {
        header_row,
        headers,
        columns,
        convention,
        convention_reason,
        points,
        skipped,
    })
}

/// The rows of an uploaded list: the first sheet of an Excel or OpenDocument file, or CSV text.
pub fn rows_of_file(name: &str, bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let lower = name.to_lowercase();
    if [".xlsx", ".xlsm", ".xls", ".ods"]
        .iter()
        .any(|ending| lower.ends_with(ending))
    {
        return spreadsheet_rows(bytes);
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        // lists saved by Excel on Windows are often Windows-1252; its letters above 127 map to
        // Latin-1 well enough for ä, ö, ü and °
        Err(_) => bytes.iter().map(|&byte| byte as char).collect(),
    };
    Ok(crate::csv::parse(&text, crate::csv::guess_separator(&text)))
}

fn spreadsheet_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    use calamine::{Reader, open_workbook_auto_from_rs};
    let mut workbook = open_workbook_auto_from_rs(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|error| format!("cannot read the spreadsheet: {error}"))?;
    let sheet = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("the spreadsheet has no sheet")?;
    let range = workbook
        .worksheet_range(&sheet)
        .map_err(|error| format!("cannot read sheet {sheet}: {error}"))?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect())
}

fn header_matches(text: &str, role: Role) -> bool {
    let text = text.trim().to_lowercase();
    !text.is_empty()
        && role.words().iter().any(|word| {
            text == *word
                || text.starts_with(&format!("{word} "))
                || text.contains(word) && word.len() > 3
        })
}

fn find_header(rows: &[Vec<String>]) -> Option<usize> {
    rows.iter().take(30).position(|row| {
        let roles = Role::ALL
            .iter()
            .filter(|role| row.iter().any(|cell| header_matches(cell, **role)))
            .count();
        roles >= 2 && row.iter().any(|cell| header_matches(cell, Role::Address))
    })
}

fn guess_columns(headers: &[String]) -> BTreeMap<Role, usize> {
    let mut columns = BTreeMap::new();
    // roles with narrow words first, so "register type" goes to the area before "register" takes it
    let order = [
        Role::ScaleRegister,
        Role::Area,
        Role::Order,
        Role::Offset,
        Role::Type,
        Role::Count,
        Role::Address,
        Role::Name,
        Role::Group,
        Role::Unit,
        Role::Factor,
        Role::Description,
        Role::Access,
        Role::States,
        Role::Min,
        Role::Max,
    ];
    for role in order {
        let taken: Vec<usize> = columns.values().copied().collect();
        let found = headers
            .iter()
            .enumerate()
            .filter(|(column, _)| !taken.contains(column))
            .find(|(_, header)| header_matches(header, role));
        if let Some((column, _)) = found {
            columns.insert(role, column);
        }
    }
    columns
}

fn guess_convention(addresses: &[String], has_area: bool) -> (Convention, String) {
    let numbers: Vec<u32> = addresses
        .iter()
        .filter_map(|text| parse_number(text))
        .collect();
    if numbers.is_empty() {
        return (Convention::ZeroBased, "no addresses to judge by".into());
    }
    let references = numbers
        .iter()
        .filter(|&&number| {
            (10_001..=49_999).contains(&number) || (100_001..=465_536).contains(&number)
        })
        .count();
    if !has_area && references * 2 > numbers.len() {
        return (
            Convention::Reference,
            "addresses like 40001 with the area in front".into(),
        );
    }
    if !numbers.contains(&0) && numbers.contains(&1) {
        return (
            Convention::OneBased,
            "the list starts at 1 and never names 0".into(),
        );
    }
    (Convention::ZeroBased, "the list names address 0".into())
}

/// `40011`, `0x000A`, `000A h`, `10.0` (as Excel stores numbers).
fn parse_number(text: &str) -> Option<u32> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).ok();
    }
    if let Some(hex) = lower.strip_suffix('h').map(str::trim) {
        return u32::from_str_radix(hex, 16).ok();
    }
    if let Ok(number) = text.parse::<u32>() {
        return Some(number);
    }
    text.parse::<f64>()
        .ok()
        .filter(|number| number.fract() == 0.0 && *number >= 0.0)
        .map(|number| number as u32)
}

fn area_of(text: &str) -> Option<Area> {
    let lower = text.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    // a number is a function code, as lists write them: 3, 03, FC03, 0x03, "3/6/16"
    let code = lower
        .trim_start_matches("fc")
        .trim_start_matches("0x")
        .trim();
    let digits: String = code.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() {
        return match digits.trim_start_matches('0') {
            "1" | "5" | "15" => Some(Area::Coil),
            "2" => Some(Area::DiscreteInput),
            "3" | "6" | "16" => Some(Area::HoldingRegister),
            "4" => Some(Area::InputRegister),
            _ => None,
        };
    }
    if let Some(area) = Area::parse(&lower) {
        return Some(area);
    }
    let has = |words: &[&str]| words.iter().any(|word| lower.contains(word));
    if has(&["holding", "halte"]) {
        Some(Area::HoldingRegister)
    } else if has(&["input reg", "eingangsreg"]) {
        Some(Area::InputRegister)
    } else if has(&["coil", "spule"]) {
        Some(Area::Coil)
    } else if has(&["discrete", "input status", "digital input"]) {
        Some(Area::DiscreteInput)
    } else {
        None
    }
}

fn point_of(
    cell: &dyn Fn(Role) -> String,
    convention: Convention,
    default_area: Option<Area>,
) -> Result<Point, String> {
    let address_text = cell(Role::Address);
    let number =
        parse_number(&address_text).ok_or_else(|| format!("{address_text} is not an address"))?;
    let named_area = area_of(&cell(Role::Area));
    let (area, address) = on_wire(number, convention)?;
    let area = named_area.or(area).or(default_area);
    let area = area.ok_or("no area (holding, input, coil …) for this row")?;
    let scale_text = cell(Role::ScaleRegister);
    let scale_register = match parse_number(&scale_text) {
        None if scale_text.is_empty() => None,
        None => return Err(format!("{scale_text} is not a scale register address")),
        Some(number) => Some(
            u16::try_from(on_wire(number, convention)?.1)
                .map_err(|_| format!("{scale_text} is beyond 65535"))?,
        ),
    };
    let address = u16::try_from(address).map_err(|_| format!("{address_text} is beyond 65535"))?;
    let name = Some(cell(Role::Name))
        .filter(|name| !name.is_empty())
        .or_else(|| Some(cell(Role::Description)).filter(|text| !text.is_empty()))
        .unwrap_or_else(|| format!("{} {address}", area.short()));
    let kind = kind_of(&cell(Role::Type), &cell(Role::Count), area)?;
    let mut point = Point::new(&name, area, address, kind);
    point.group = cell(Role::Group);
    point.unit = cell(Role::Unit);
    point.description = cell(Role::Description);
    point.source_address = address_text;
    point.scale = Scale {
        factor: factor_of(&cell(Role::Factor))?,
        offset: decimal(&cell(Role::Offset)).unwrap_or(0.0),
    };
    point.order = WordOrder::parse(&cell(Role::Order));
    point.scale_register = scale_register;
    point.states = states_of(&cell(Role::States));
    let access = cell(Role::Access).to_lowercase();
    point.writable = area.writable() && (access.contains('w') || access.contains("schreib"));
    point.min = decimal(&cell(Role::Min));
    point.max = decimal(&cell(Role::Max));
    point.check()?;
    Ok(point)
}

/// An address of the list on the wire, with the area a reference address names.
fn on_wire(number: u32, convention: Convention) -> Result<(Option<Area>, u32), String> {
    match convention {
        Convention::ZeroBased => Ok((None, number)),
        Convention::OneBased => number
            .checked_sub(1)
            .map(|address| (None, address))
            .ok_or_else(|| "address 0 in a list that counts from 1".to_string()),
        Convention::Reference => {
            Ok(reference(number).map_or((None, number), |(area, address)| (Some(area), address)))
        }
    }
}

fn reference(number: u32) -> Option<(Area, u32)> {
    let (area, base) = match number {
        1..=9_999 => (Area::Coil, 1),
        10_001..=19_999 => (Area::DiscreteInput, 10_001),
        30_001..=39_999 => (Area::InputRegister, 30_001),
        40_001..=49_999 => (Area::HoldingRegister, 40_001),
        100_001..=165_536 => (Area::DiscreteInput, 100_001),
        300_001..=365_536 => (Area::InputRegister, 300_001),
        400_001..=465_536 => (Area::HoldingRegister, 400_001),
        _ => return None,
    };
    Some((area, number - base))
}

fn kind_of(text: &str, count: &str, area: Area) -> Result<DataType, String> {
    if area.is_bits() {
        return Ok(DataType::Bool);
    }
    if !text.is_empty() {
        return DataType::parse(text)
            .ok_or_else(|| format!("{text} is not a data type modbusio knows"));
    }
    Ok(match parse_number(count) {
        Some(2) => DataType::U32,
        Some(4) => DataType::U64,
        _ => DataType::U16,
    })
}

/// A number with a decimal point or comma.
fn decimal(text: &str) -> Option<f64> {
    let text = text.trim().replace(',', ".");
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

/// `0.1`, `0,01`, `x10`, `/10`, `1/100`, empty for 1.
fn factor_of(text: &str) -> Result<f64, String> {
    let trimmed = text.trim().trim_start_matches(['x', 'X', '×', '*']).trim();
    if trimmed.is_empty() {
        return Ok(1.0);
    }
    let factor = if let Some((top, bottom)) = trimmed.split_once('/') {
        let top = if top.trim().is_empty() {
            Some(1.0)
        } else {
            decimal(top)
        };
        match (top, decimal(bottom)) {
            (Some(top), Some(bottom)) if bottom != 0.0 => Some(top / bottom),
            _ => None,
        }
    } else {
        decimal(trimmed)
    };
    factor
        .filter(|factor| *factor != 0.0)
        .ok_or_else(|| format!("{text} is not a factor"))
}

/// `0=off; 1=on`, `0: Aus, 1: Ein`.
fn states_of(text: &str) -> Vec<(i64, String)> {
    text.split([';', ',', '\n', '|'])
        .filter_map(|part| {
            let (number, word) = part.split_once(['=', ':'])?;
            Some((number.trim().parse().ok()?, word.trim().to_string()))
        })
        .filter(|(_, word): &(i64, String)| !word.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(text: &str) -> Vec<Vec<String>> {
        crate::csv::parse(text, crate::csv::guess_separator(text))
    }

    #[test]
    fn a_list_with_reference_addresses_is_read() {
        let list = rows(
            "Heat meter HM-50 register list\n\
             Register;Name;Data type;Unit;Factor;Access\n\
             40001;Energy;float;kWh;;R\n\
             40003;Flow temperature;FLOAT32;°C;1;R\n\
             40009;Volume;uint16;m³;0,01;R\n\
             40101;Address;uint16;;;RW\n\
             30001;Room temperature;int16;°C;x0.1;R\n",
        );
        let imported = import(&list, &Options::default()).expect("imported");
        assert_eq!(imported.header_row, 1);
        assert_eq!(imported.convention, Convention::Reference);
        assert_eq!(imported.points.len(), 5);
        let energy = &imported.points[0];
        assert_eq!(
            (energy.area, energy.address, energy.kind),
            (Area::HoldingRegister, 0, DataType::F32)
        );
        assert_eq!(energy.source_address, "40001");
        assert!((imported.points[2].scale.factor - 0.01).abs() < 1e-12);
        assert!(imported.points[3].writable);
        assert!(!imported.points[2].writable);
        let room = &imported.points[4];
        assert_eq!((room.area, room.address), (Area::InputRegister, 0));
        assert!((room.scale.factor - 0.1).abs() < 1e-12);
    }

    #[test]
    fn function_codes_name_the_area_and_a_list_from_1_is_shifted() {
        let list = rows(
            "Name,FC,Address,Type,Values\n\
             Mode,3,1,UINT16,0=off;1=heating;2=cooling\n\
             Run,1,2,,\n",
        );
        let imported = import(&list, &Options::default()).expect("imported");
        assert_eq!(imported.convention, Convention::OneBased);
        let mode = &imported.points[0];
        assert_eq!((mode.area, mode.address), (Area::HoldingRegister, 0));
        assert_eq!(mode.states.len(), 3);
        assert_eq!(imported.points[1].area, Area::Coil);
        assert_eq!(imported.points[1].kind, DataType::Bool);
    }

    #[test]
    fn rows_that_cannot_be_points_are_named_with_their_row() {
        let list = rows(
            "Name;Address;Area;Type\n\
             Good;0;HR;u16\n\
             Bad type;1;HR;banana\n\
             No area;2;;u16\n",
        );
        let imported = import(&list, &Options::default()).expect("imported");
        assert_eq!(imported.points.len(), 1);
        assert_eq!(imported.skipped.len(), 2);
        assert_eq!(imported.skipped[0].0, 3);
        assert!(imported.skipped[1].1.contains("area"));
    }

    #[test]
    fn the_chosen_convention_and_columns_win() {
        let list = rows("Name;Address;Area\nFlow;10;HR\n");
        let options = Options {
            convention: Some(Convention::OneBased),
            ..Options::default()
        };
        let imported = import(&list, &options).expect("imported");
        assert_eq!(imported.points[0].address, 9);
        assert_eq!(imported.convention_reason, "chosen");
    }

    #[test]
    fn a_list_without_address_column_is_refused() {
        assert!(import(&rows("Name;Unit\nFlow;°C\n"), &Options::default()).is_err());
    }

    #[test]
    fn factors_are_read_as_lists_write_them() {
        assert_eq!(factor_of(""), Ok(1.0));
        assert_eq!(factor_of("0,1"), Ok(0.1));
        assert_eq!(factor_of("/10"), Ok(0.1));
        assert_eq!(factor_of("1/100"), Ok(0.01));
        assert_eq!(factor_of("x10"), Ok(10.0));
        assert!(factor_of("0").is_err());
    }

    #[test]
    fn hex_addresses_are_read() {
        assert_eq!(parse_number("0x000A"), Some(10));
        assert_eq!(parse_number("000Ah"), Some(10));
        assert_eq!(parse_number("12.0"), Some(12));
    }
}
