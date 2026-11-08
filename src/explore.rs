//! The register explorer (concept section 5.3): which addresses of an area exist, read in blocks
//! that are halved where the device refuses, and what each register could mean.

use std::time::Duration;

use serde::Serialize;

use crate::access::{self, Limits};
use crate::endpoint::Target;
use crate::link::LinkError;
use crate::links::Links;
use crate::protocol::{Area, Exception};
use crate::values::{self, DataType, WordOrder};

/// Most addresses one exploration covers, so a slow line is not busy for minutes.
pub const MOST_ADDRESSES: u32 = 2000;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cell {
    pub address: u16,
    /// `None` where the device has nothing at this address or did not answer.
    pub value: Option<u16>,
    /// Why there is no value: `exception_02`, `timeout` … (`LinkError::code`).
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Explored {
    pub cells: Vec<Cell>,
    /// The exploration stopped early: the line or the gateway failed.
    pub stopped: Option<LinkError>,
}

/// Reads `count` addresses of `area` from `start`, learning the device's limits on the way.
pub async fn explore(
    links: &Links,
    target: &Target,
    area: Area,
    start: u16,
    count: u32,
    limits: &mut Limits,
    timeout: Duration,
) -> Explored {
    let end = (u32::from(start) + count.min(MOST_ADDRESSES)).min(0x1_0000);
    let mut cells = Vec::new();
    let mut at = u32::from(start);
    while at < end {
        let size = (end - at).min(u32::from(limits.most(area))) as u16;
        if let Err(error) = block(
            links, target, area, at as u16, size, limits, timeout, &mut cells,
        )
        .await
        {
            return Explored {
                cells,
                stopped: Some(error),
            };
        }
        at += u32::from(size);
    }
    Explored {
        cells,
        stopped: None,
    }
}

/// Reads one block into `cells`, halving it where the device refuses. An error that ends the whole
/// exploration (line closed, gateway without path) comes back as `Err`.
#[allow(clippy::too_many_arguments)]
async fn block(
    links: &Links,
    target: &Target,
    area: Area,
    address: u16,
    count: u16,
    limits: &mut Limits,
    timeout: Duration,
    cells: &mut Vec<Cell>,
) -> Result<(), LinkError> {
    let mut pending = vec![(address, count)];
    while let Some((address, count)) = pending.pop() {
        match access::read(links, target, area, address, count, timeout).await {
            Ok(values) => cells.extend(values.into_iter().zip(address..).map(
                |(value, address)| Cell {
                    address,
                    value: Some(value),
                    error: None,
                },
            )),
            Err(error) if count > 1 && splits(&error) => {
                if !matches!(error, LinkError::Exception(Exception::ILLEGAL_ADDRESS)) {
                    limits.lower(area, count);
                }
                let first = count / 2;
                // the stack takes the second half last, so cells stay in address order
                pending.push((address + first, count - first));
                pending.push((address, first));
            }
            Err(error) if ends_exploration(&error) => return Err(error),
            Err(error) => cells.extend((address..address + count).map(|address| Cell {
                address,
                value: None,
                error: Some(error.code()),
            })),
        }
    }
    Ok(())
}

/// A smaller block may succeed: a gap in the map, too many items, or silence instead of refusal.
fn splits(error: &LinkError) -> bool {
    matches!(
        error,
        LinkError::Exception(Exception::ILLEGAL_ADDRESS | Exception::ILLEGAL_VALUE)
            | LinkError::Timeout
            | LinkError::Damaged { .. }
            | LinkError::Crc
    )
}

fn ends_exploration(error: &LinkError) -> bool {
    error.drops_connection()
        || matches!(error, LinkError::Exception(exception) if exception.from_gateway())
        || matches!(error, LinkError::Exception(Exception::ILLEGAL_FUNCTION))
}

/// One register and what it could mean, with the next one for 32-bit readings.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Row {
    pub address: u16,
    pub value: Option<u16>,
    pub error: Option<String>,
    pub hex: Option<String>,
    pub signed: Option<i16>,
    /// Two characters, where both are printable.
    pub text: Option<String>,
    /// This register and the next as u32, s32 and float32, in each word order.
    pub pairs: Vec<PairReading>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PairReading {
    pub order: WordOrder,
    pub u32: u32,
    pub s32: i32,
    pub float: String,
    pub plausible: bool,
}

/// The readings of every cell, and the word order that makes floats plausible, where one does.
pub fn interpret(cells: &[Cell]) -> (Vec<Row>, Option<WordOrder>) {
    let rows: Vec<Row> = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            let next = cells
                .get(index + 1)
                .filter(|next| u32::from(next.address) == u32::from(cell.address) + 1)
                .and_then(|next| next.value);
            row(cell, next)
        })
        .collect();
    let pairs: Vec<[u16; 2]> = cells
        .windows(2)
        .step_by(2)
        .filter_map(|pair| Some([pair[0].value?, pair[1].value?]))
        .collect();
    let order = values::likely_order(&pairs).map(|(order, _)| order);
    (rows, order)
}

fn row(cell: &Cell, next: Option<u16>) -> Row {
    let Some(value) = cell.value else {
        return Row {
            address: cell.address,
            value: None,
            error: cell.error.clone(),
            hex: None,
            signed: None,
            text: None,
            pairs: Vec::new(),
        };
    };
    let [high, low] = value.to_be_bytes();
    let printable = |byte: u8| byte.is_ascii_graphic() || byte == b' ';
    let pairs = next
        .map(|next| {
            WordOrder::ALL
                .into_iter()
                .map(|order| {
                    let registers = [value, next];
                    let number = |kind| values::decode(&registers, kind, order);
                    let float = number(DataType::F32).as_f64().unwrap_or(f64::NAN);
                    PairReading {
                        order,
                        u32: number(DataType::U32).as_f64().unwrap_or_default() as u32,
                        s32: number(DataType::S32).as_f64().unwrap_or_default() as i32,
                        float: values::number_text(float),
                        plausible: values::plausible(float),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Row {
        address: cell.address,
        value: Some(value),
        error: None,
        hex: Some(format!("{value:04X}")),
        signed: Some(value as i16),
        text: (printable(high) && printable(low))
            .then(|| String::from_utf8_lossy(&[high, low]).into_owned()),
        pairs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(address: u16, value: Option<u16>) -> Cell {
        Cell {
            address,
            value,
            error: value.is_none().then(|| "exception_02".into()),
        }
    }

    #[test]
    fn a_register_reads_as_number_hex_and_text() {
        let (rows, _) = interpret(&[cell(0, Some(0x4142)), cell(1, Some(0xFFFF))]);
        assert_eq!(rows[0].hex.as_deref(), Some("4142"));
        assert_eq!(rows[0].text.as_deref(), Some("AB"));
        assert_eq!(rows[1].signed, Some(-1));
        assert_eq!(rows[1].pairs.len(), 0, "no next register, no pair");
        assert_eq!(rows[0].pairs.len(), 4);
    }

    #[test]
    fn a_missing_neighbour_gives_no_pair() {
        let (rows, _) = interpret(&[cell(0, Some(1)), cell(1, None)]);
        assert!(rows[0].pairs.is_empty());
        assert_eq!(rows[1].error.as_deref(), Some("exception_02"));
    }

    #[test]
    fn swapped_floats_suggest_cdab() {
        let mut cells = Vec::new();
        for (index, value) in [63.2f32, 41.0, 1234.5, 0.85].into_iter().enumerate() {
            let bits = value.to_bits();
            let address = index as u16 * 2;
            cells.push(cell(address, Some((bits & 0xFFFF) as u16)));
            cells.push(cell(address + 1, Some((bits >> 16) as u16)));
        }
        let (_, order) = interpret(&cells);
        assert_eq!(order, Some(WordOrder::Cdab));
    }
}
