//! Registers to values and back: data types, word and byte order, scaling (concept section 6).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Stored and sent as its name: `u16`, `f32`, `text20`, `bit3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataType {
    Bool,
    U16,
    S16,
    U32,
    S32,
    U64,
    S64,
    F32,
    F64,
    /// Text of this many registers, two characters each, high byte first.
    Text(u8),
    /// One bit (0–15) of a register.
    Bit(u8),
}

/// Longest text, in registers: it is read in one request.
const MOST_TEXT_REGISTERS: u32 = 120;

/// What lists call a text.
const TEXT_WORDS: [&str; 5] = ["text", "string", "str", "char", "ascii"];

/// A type's name as lists write it, without case, spaces and dashes.
fn lower_joined(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "")
}

impl DataType {
    pub fn registers(self) -> u16 {
        match self {
            DataType::Bool | DataType::U16 | DataType::S16 | DataType::Bit(_) => 1,
            DataType::U32 | DataType::S32 | DataType::F32 => 2,
            DataType::U64 | DataType::S64 | DataType::F64 => 4,
            DataType::Text(registers) => u16::from(registers.max(1)),
        }
    }

    pub fn is_numeric(self) -> bool {
        !matches!(self, DataType::Text(_))
    }

    /// Word order matters: more than one register and not text.
    pub fn has_order(self) -> bool {
        self.registers() > 1 && self.is_numeric()
    }

    pub fn name(self) -> String {
        match self {
            DataType::Bool => "bool".into(),
            DataType::U16 => "u16".into(),
            DataType::S16 => "s16".into(),
            DataType::U32 => "u32".into(),
            DataType::S32 => "s32".into(),
            DataType::U64 => "u64".into(),
            DataType::S64 => "s64".into(),
            DataType::F32 => "f32".into(),
            DataType::F64 => "f64".into(),
            DataType::Text(registers) => format!("text{}", u16::from(registers) * 2),
            DataType::Bit(bit) => format!("bit{bit}"),
        }
    }

    /// A text of that many registers, as a list gives it apart from the type's name.
    pub fn text(registers: u32) -> Option<DataType> {
        (1..=MOST_TEXT_REGISTERS)
            .contains(&registers)
            .then_some(DataType::Text(registers as u8))
    }

    /// Whether lists name a text so, without its length: `string`, `CHAR`.
    pub fn is_text_word(text: &str) -> bool {
        TEXT_WORDS.contains(&lower_joined(text).as_str())
    }

    /// Reads the names lists use: `uint16`, `INT32`, `float`, `real`, `string20`, `bit3` …
    pub fn parse(text: &str) -> Option<DataType> {
        let lower = lower_joined(text);
        let numbered = |prefixes: &[&str]| {
            prefixes.iter().find_map(|prefix| {
                lower
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.parse::<u32>().ok())
            })
        };
        if let Some(characters) = numbered(&TEXT_WORDS) {
            return DataType::text(characters.div_ceil(2).clamp(1, MOST_TEXT_REGISTERS));
        }
        if let Some(bit) = numbered(&["bit"]).filter(|bit| *bit < 16) {
            return Some(DataType::Bit(bit as u8));
        }
        Some(match lower.as_str() {
            "bool" | "boolean" | "coil" | "bit" | "digital" => DataType::Bool,
            "u16" | "uint16" | "uint" | "word" | "unsigned" | "unsigned16" | "ushort" => {
                DataType::U16
            }
            "s16" | "i16" | "int16" | "int" | "short" | "signed" | "signed16" => DataType::S16,
            "u32" | "uint32" | "dword" | "udint" | "unsigned32" | "ulong" => DataType::U32,
            "s32" | "i32" | "int32" | "dint" | "long" | "signed32" => DataType::S32,
            "u64" | "uint64" | "ulint" | "unsigned64" => DataType::U64,
            "s64" | "i64" | "int64" | "lint" | "signed64" => DataType::S64,
            "f32" | "float" | "float32" | "real" | "ieee754" | "single" => DataType::F32,
            "f64" | "double" | "float64" | "lreal" => DataType::F64,
            _ => return None,
        })
    }
}

impl Serialize for DataType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for DataType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        DataType::parse(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown data type {text}")))
    }
}

impl Serialize for WordOrder {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for WordOrder {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        WordOrder::parse(&text).ok_or_else(|| serde::de::Error::custom("unknown word order"))
    }
}

/// Order of the bytes of a multi-register value, named by where the most significant byte A goes:
/// ABCD big endian as Modbus itself, CDAB words swapped, BADC bytes swapped, DCBA little endian.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WordOrder {
    #[default]
    Abcd,
    Cdab,
    Badc,
    Dcba,
}

impl WordOrder {
    pub const ALL: [WordOrder; 4] = [
        WordOrder::Abcd,
        WordOrder::Cdab,
        WordOrder::Badc,
        WordOrder::Dcba,
    ];

    pub fn name(self) -> &'static str {
        match self {
            WordOrder::Abcd => "ABCD",
            WordOrder::Cdab => "CDAB",
            WordOrder::Badc => "BADC",
            WordOrder::Dcba => "DCBA",
        }
    }

    pub fn parse(text: &str) -> Option<WordOrder> {
        let lower = text.trim().to_ascii_lowercase();
        Some(match lower.as_str() {
            "abcd" | "big" | "big endian" | "msw first" | "high word first" => WordOrder::Abcd,
            "cdab" | "word swap" | "words swapped" | "lsw first" | "low word first" => {
                WordOrder::Cdab
            }
            "badc" | "byte swap" | "bytes swapped" => WordOrder::Badc,
            "dcba" | "little" | "little endian" => WordOrder::Dcba,
            _ => return None,
        })
    }

    /// The registers as received, to the bytes of the value, most significant first.
    fn bytes_of(self, registers: &[u16]) -> Vec<u8> {
        let words: Vec<u16> = match self {
            WordOrder::Abcd | WordOrder::Badc => registers.to_vec(),
            WordOrder::Cdab | WordOrder::Dcba => registers.iter().rev().copied().collect(),
        };
        words
            .iter()
            .flat_map(|word| match self {
                WordOrder::Abcd | WordOrder::Cdab => word.to_be_bytes(),
                WordOrder::Badc | WordOrder::Dcba => word.to_le_bytes(),
            })
            .collect()
    }

    /// The bytes of a value, most significant first, to the registers to send.
    fn registers_of(self, bytes: &[u8]) -> Vec<u16> {
        let words: Vec<u16> = bytes
            .chunks(2)
            .map(|pair| match self {
                WordOrder::Abcd | WordOrder::Cdab => u16::from_be_bytes([pair[0], pair[1]]),
                WordOrder::Badc | WordOrder::Dcba => u16::from_le_bytes([pair[0], pair[1]]),
            })
            .collect();
        match self {
            WordOrder::Abcd | WordOrder::Badc => words,
            WordOrder::Cdab | WordOrder::Dcba => words.into_iter().rev().collect(),
        }
    }
}

/// A decoded value, before scaling.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Raw {
    Bool(bool),
    Integer(i128),
    Float(f64),
    Text(String),
}

impl Raw {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Raw::Bool(on) => Some(f64::from(u8::from(*on))),
            Raw::Integer(number) => Some(*number as f64),
            Raw::Float(number) => Some(*number),
            Raw::Text(_) => None,
        }
    }
}

impl fmt::Display for Raw {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Raw::Bool(on) => f.write_str(if *on { "1" } else { "0" }),
            Raw::Integer(number) => write!(f, "{number}"),
            Raw::Float(number) => f.write_str(&number_text(*number)),
            Raw::Text(text) => f.write_str(text),
        }
    }
}

/// Decodes `registers` (exactly `kind.registers()` of them) as `kind` in `order`.
pub fn decode(registers: &[u16], kind: DataType, order: WordOrder) -> Raw {
    let bytes = order.bytes_of(registers);
    let array = |n: usize| -> [u8; 8] {
        let mut out = [0u8; 8];
        out[8 - n..].copy_from_slice(&bytes[..n]);
        out
    };
    match kind {
        DataType::Bool => Raw::Bool(registers[0] != 0),
        DataType::Bit(bit) => Raw::Bool(registers[0] & (1 << bit) != 0),
        DataType::U16 => Raw::Integer(i128::from(registers[0])),
        DataType::S16 => Raw::Integer(i128::from(registers[0] as i16)),
        DataType::U32 => Raw::Integer(i128::from(u64::from_be_bytes(array(4)))),
        DataType::S32 => Raw::Integer(i128::from(u64::from_be_bytes(array(4)) as u32 as i32)),
        DataType::U64 => Raw::Integer(i128::from(u64::from_be_bytes(array(8)))),
        DataType::S64 => Raw::Integer(i128::from(i64::from_be_bytes(array(8)))),
        DataType::F32 => {
            let value = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            // the shortest text that reads back as this float32, so 230.1 does not become 230.100006
            Raw::Float(value.to_string().parse().unwrap_or(f64::from(value)))
        }
        DataType::F64 => Raw::Float(f64::from_be_bytes(array(8))),
        DataType::Text(_) => {
            // text reads register after register, high byte first, whatever the numbers' order
            let text: String = registers
                .iter()
                .flat_map(|register| register.to_be_bytes())
                .map(|byte| byte as char)
                .filter(|character| *character != '\0')
                .collect();
            Raw::Text(text.trim().to_string())
        }
    }
}

/// 2^53 − 1: up to here a whole number typed is the number kept. Beyond it a neighbour would be
/// written, so a 64-bit value is not written there.
pub const MOST_EXACT_WHOLE: f64 = 9_007_199_254_740_991.0;

/// Encodes a number for writing; `None` when it does not fit `kind`.
pub fn encode(value: f64, kind: DataType, order: WordOrder) -> Option<Vec<u16>> {
    if !value.is_finite() {
        return None;
    }
    let whole = |low: f64, high: f64| -> Option<i128> {
        let rounded = value.round();
        ((rounded - value).abs() < 1e-9 && rounded >= low && rounded <= high)
            .then_some(rounded as i128)
    };
    let bytes: Vec<u8> = match kind {
        DataType::Bool | DataType::Bit(_) | DataType::Text(_) => return None,
        DataType::U16 => (whole(0.0, 65535.0)? as u16).to_be_bytes().to_vec(),
        DataType::S16 => (whole(-32768.0, 32767.0)? as i16).to_be_bytes().to_vec(),
        DataType::U32 => (whole(0.0, 4_294_967_295.0)? as u32).to_be_bytes().to_vec(),
        DataType::S32 => (whole(-2_147_483_648.0, 2_147_483_647.0)? as i32)
            .to_be_bytes()
            .to_vec(),
        DataType::U64 => (whole(0.0, MOST_EXACT_WHOLE)? as u64)
            .to_be_bytes()
            .to_vec(),
        DataType::S64 => (whole(-MOST_EXACT_WHOLE, MOST_EXACT_WHOLE)? as i64)
            .to_be_bytes()
            .to_vec(),
        DataType::F32 => {
            if value.abs() > f64::from(f32::MAX) {
                return None;
            }
            (value as f32).to_be_bytes().to_vec()
        }
        DataType::F64 => value.to_be_bytes().to_vec(),
    };
    if kind.registers() == 1 {
        return Some(vec![u16::from_be_bytes([bytes[0], bytes[1]])]);
    }
    Some(order.registers_of(&bytes))
}

/// How a raw number becomes the value the user reads: `raw × factor × 10^exponent + offset`.
/// `exponent` comes from a scale register where the device has one (SunSpec and many meters).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scale {
    #[serde(default = "one")]
    pub factor: f64,
    #[serde(default)]
    pub offset: f64,
}

fn one() -> f64 {
    1.0
}

impl Default for Scale {
    fn default() -> Self {
        Scale {
            factor: 1.0,
            offset: 0.0,
        }
    }
}

impl Scale {
    pub fn is_identity(self) -> bool {
        self.factor == 1.0 && self.offset == 0.0
    }

    pub fn apply(self, raw: f64, exponent: i16) -> f64 {
        raw * self.factor * 10f64.powi(i32::from(exponent)) + self.offset
    }

    /// The raw number to send so the device shows `value`; `None` when the factor is zero.
    pub fn invert(self, value: f64, exponent: i16) -> Option<f64> {
        let factor = self.factor * 10f64.powi(i32::from(exponent));
        (factor != 0.0).then(|| (value - self.offset) / factor)
    }
}

/// A number for people: no float noise (`23.15`, not `23.149999618530273`).
pub fn number_text(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 {
            "∞".into()
        } else {
            "−∞".into()
        };
    }
    if value != 0.0 && (value.abs() >= 1e15 || value.abs() < 1e-6) {
        return format!("{value:.6e}");
    }
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}

/// Whether a float looks like a measured value rather than bytes read in the wrong order: finite,
/// zero, or between 1e-6 and 1e9 in size.
pub fn plausible(value: f64) -> bool {
    value == 0.0 || (value.is_finite() && (1e-6..1e9).contains(&value.abs()))
}

/// The word order under which the most of these register pairs read as plausible floats, and how
/// many did; `None` when no order is clearly better than ABCD.
pub fn likely_order(pairs: &[[u16; 2]]) -> Option<(WordOrder, usize)> {
    let nonzero: Vec<&[u16; 2]> = pairs.iter().filter(|pair| **pair != [0, 0]).collect();
    if nonzero.len() < 2 {
        return None;
    }
    let count = |order: WordOrder| {
        nonzero
            .iter()
            .filter(|pair| {
                decode(&pair[..], DataType::F32, order)
                    .as_f64()
                    .is_some_and(plausible)
            })
            .count()
    };
    let abcd = count(WordOrder::Abcd);
    WordOrder::ALL
        .into_iter()
        .map(|order| (order, count(order)))
        .max_by_key(|(_, plausible)| *plausible)
        .filter(|(order, best)| {
            *order != WordOrder::Abcd && *best > abcd && *best * 2 > nonzero.len()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_reads_right_in_each_order() {
        // 63.2 as float32 is 0x427CCCCD
        let cases = [
            (WordOrder::Abcd, [0x427C, 0xCCCD]),
            (WordOrder::Cdab, [0xCCCD, 0x427C]),
            (WordOrder::Badc, [0x7C42, 0xCDCC]),
            (WordOrder::Dcba, [0xCDCC, 0x7C42]),
        ];
        for (order, registers) in cases {
            let value = decode(&registers, DataType::F32, order)
                .as_f64()
                .expect("number");
            assert!((value - 63.2).abs() < 1e-5, "{order:?} gave {value}");
            assert_eq!(encode(63.2, DataType::F32, order), Some(registers.to_vec()));
        }
    }

    #[test]
    fn signed_and_wide_integers_decode() {
        assert_eq!(
            decode(&[0xFFFE], DataType::S16, WordOrder::Abcd),
            Raw::Integer(-2)
        );
        assert_eq!(
            decode(&[0x0001, 0x0000], DataType::U32, WordOrder::Abcd),
            Raw::Integer(65536)
        );
        assert_eq!(
            decode(&[0x0000, 0x0001], DataType::U32, WordOrder::Cdab),
            Raw::Integer(65536)
        );
        assert_eq!(
            decode(&[0xFFFF, 0xFFFF], DataType::S32, WordOrder::Abcd),
            Raw::Integer(-1)
        );
        assert_eq!(
            decode(&[0, 0, 0x0001, 0x0000], DataType::U64, WordOrder::Abcd),
            Raw::Integer(65536)
        );
    }

    #[test]
    fn bits_and_texts_decode() {
        assert_eq!(
            decode(&[0b100], DataType::Bit(2), WordOrder::Abcd),
            Raw::Bool(true)
        );
        assert_eq!(
            decode(&[0x4142, 0x4300], DataType::Text(2), WordOrder::Abcd),
            Raw::Text("ABC".into())
        );
    }

    #[test]
    fn integers_that_do_not_fit_are_not_encoded() {
        assert_eq!(encode(70000.0, DataType::U16, WordOrder::Abcd), None);
        assert_eq!(encode(1.5, DataType::S16, WordOrder::Abcd), None);
        assert_eq!(encode(f64::NAN, DataType::F32, WordOrder::Abcd), None);
        // beyond 2^53 the number typed is not the number a 64-bit register would get
        assert_eq!(encode(1e16, DataType::U64, WordOrder::Abcd), None);
        assert_eq!(encode(-1e16, DataType::S64, WordOrder::Abcd), None);
        assert_eq!(
            encode(MOST_EXACT_WHOLE, DataType::U64, WordOrder::Abcd),
            Some(vec![0x001F, 0xFFFF, 0xFFFF, 0xFFFF])
        );
        assert_eq!(
            encode(-1.0, DataType::S16, WordOrder::Abcd),
            Some(vec![0xFFFF])
        );
        assert_eq!(
            encode(65536.0, DataType::U32, WordOrder::Cdab),
            Some(vec![0x0000, 0x0001])
        );
    }

    #[test]
    fn scaling_works_both_ways() {
        let scale = Scale {
            factor: 0.1,
            offset: -40.0,
        };
        assert!((scale.apply(632.0, 0) - 23.2).abs() < 1e-9);
        assert!((scale.invert(23.2, 0).expect("factor") - 632.0).abs() < 1e-9);
        // a scale register of -2 makes 2315 into 23.15
        assert!((Scale::default().apply(2315.0, -2) - 23.15).abs() < 1e-9);
    }

    #[test]
    fn numbers_read_without_float_noise() {
        assert_eq!(number_text(23.15), "23.15");
        let float = decode(&[0x4366, 0x199A], DataType::F32, WordOrder::Abcd);
        assert_eq!(float.to_string(), "230.1");
        assert_eq!(number_text(100.0), "100");
        assert_eq!(number_text(-0.0), "0");
        assert_eq!(number_text(1.2e-38), "1.200000e-38");
    }

    #[test]
    fn the_word_order_of_swapped_floats_is_recognised() {
        let swapped = [21.5f32, 63.2, 1013.25, 0.75].map(|value| {
            let bits = value.to_bits();
            [(bits & 0xFFFF) as u16, (bits >> 16) as u16]
        });
        assert_eq!(
            likely_order(&swapped).map(|(order, _)| order),
            Some(WordOrder::Cdab)
        );
        let straight = [21.5f32, 63.2].map(|value| {
            let bits = value.to_bits();
            [(bits >> 16) as u16, (bits & 0xFFFF) as u16]
        });
        assert_eq!(likely_order(&straight), None);
    }

    #[test]
    fn types_are_read_from_the_names_lists_use() {
        assert_eq!(DataType::parse("UINT16"), Some(DataType::U16));
        assert_eq!(DataType::parse("Float"), Some(DataType::F32));
        assert_eq!(DataType::parse("int32"), Some(DataType::S32));
        assert_eq!(DataType::parse("string20"), Some(DataType::Text(10)));
        assert_eq!(DataType::parse("bit 3"), Some(DataType::Bit(3)));
        assert_eq!(DataType::parse("banana"), None);
        assert_eq!(WordOrder::parse("cdab"), Some(WordOrder::Cdab));
        for kind in [
            DataType::F32,
            DataType::Text(10),
            DataType::Bit(3),
            DataType::S64,
        ] {
            let text = serde_json::to_string(&kind).expect("serialises");
            assert_eq!(
                serde_json::from_str::<DataType>(&text).expect("loads"),
                kind,
                "{text}"
            );
        }
        assert_eq!(
            serde_json::to_string(&DataType::Text(10)).expect("serialises"),
            "\"text20\""
        );
    }
}
