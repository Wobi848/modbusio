//! Reading the points of a profile from a device (concept section 6): planned blocks, smaller
//! blocks where the device refuses, what was learnt kept in its limits.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use crate::access::{self, Limits};
use crate::endpoint::Target;
use crate::link::LinkError;
use crate::links::Links;
use crate::plan::{self, Block};
use crate::profile::{self, Point, Reading};
use crate::protocol::{Area, Exception};
use crate::values::WordOrder;

type Item = (Area, u16, u16);

/// Reads `points` (of a profile with word order `order`) and returns one reading per point, in
/// their order. `limits` learns what the device refused.
pub async fn read_points(
    links: &Links,
    target: &Target,
    order: WordOrder,
    points: &[Point],
    limits: &mut Limits,
    timeout: Duration,
) -> Vec<Reading> {
    let items: Vec<Item> = points
        .iter()
        .flat_map(|point| {
            let scale = point
                .scale_register
                .map(|register| (point.area, register, 1));
            std::iter::once((point.area, point.address, point.count())).chain(scale)
        })
        .collect();
    let (values, failed) = read_items(links, target, &items, limits, timeout).await;
    points
        .iter()
        .map(|point| {
            let registers: Option<Vec<u16>> = (0..point.count())
                .map(|offset| values.get(&(point.area, point.address + offset)).copied())
                .collect();
            let Some(registers) = registers else {
                return Reading::failed(failure(&failed, point.area, point.address));
            };
            let exponent = match point.scale_register {
                None => 0,
                Some(register) => match values.get(&(point.area, register)) {
                    Some(value) => *value as i16,
                    None => return Reading::failed(failure(&failed, point.area, register)),
                },
            };
            profile::evaluate(point, point.order.unwrap_or(order), &registers, exponent)
        })
        .collect()
}

fn failure(failed: &HashMap<(Area, u16), String>, area: Area, address: u16) -> String {
    failed
        .get(&(area, address))
        .cloned()
        .unwrap_or_else(|| "timeout".into())
}

/// The values of every item's addresses, and the error code of those that failed.
async fn read_items(
    links: &Links,
    target: &Target,
    items: &[Item],
    limits: &mut Limits,
    timeout: Duration,
) -> (HashMap<(Area, u16), u16>, HashMap<(Area, u16), String>) {
    let mut values = HashMap::new();
    let mut failed = HashMap::new();
    let mut pending: VecDeque<Vec<Item>> = VecDeque::from([items.to_vec()]);
    // until the device has answered once, silence means it is not there: no smaller blocks then
    let mut answered = false;
    while let Some(set) = pending.pop_front() {
        for block in plan::blocks(&set, limits) {
            let inside: Vec<Item> = set
                .iter()
                .copied()
                .filter(|(area, address, count)| block.contains(*area, *address, *count))
                .collect();
            let result = read_block(links, target, &block, limits, timeout).await;
            answered |= result.is_ok() || result.as_ref().is_err_and(LinkError::device_answered);
            match result {
                Ok(read) => {
                    for (offset, value) in read.into_iter().enumerate() {
                        values.insert((block.area, block.address + offset as u16), value);
                    }
                }
                Err(error) if inside.len() > 1 && answered && smaller_helps(&error) => {
                    learn(limits, &block, &error);
                    if plan::blocks(&inside, limits) == [block.clone()] {
                        pending.extend(inside.into_iter().map(|item| vec![item]));
                    } else {
                        pending.push_back(inside);
                    }
                }
                Err(error) => {
                    let code = error.code();
                    let stop = error.drops_connection()
                        || (!answered && error == LinkError::Timeout)
                        || matches!(&error, LinkError::Exception(exception) if exception.from_gateway());
                    mark(&mut failed, &inside, &code);
                    if stop {
                        for set in pending.drain(..) {
                            mark(&mut failed, &set, &code);
                        }
                        // the rest of this round's blocks fail alike
                        mark(&mut failed, &set, &code);
                        return (values, failed);
                    }
                }
            }
        }
    }
    (values, failed)
}

fn mark(failed: &mut HashMap<(Area, u16), String>, items: &[Item], code: &str) {
    for (area, address, count) in items {
        for offset in 0..*count {
            failed
                .entry((*area, address + offset))
                .or_insert_with(|| code.to_string());
        }
    }
}

fn smaller_helps(error: &LinkError) -> bool {
    matches!(
        error,
        LinkError::Exception(Exception::ILLEGAL_ADDRESS | Exception::ILLEGAL_VALUE)
            | LinkError::Timeout
    )
}

fn learn(limits: &mut Limits, block: &Block, error: &LinkError) {
    if matches!(error, LinkError::Exception(Exception::ILLEGAL_ADDRESS)) {
        limits.no_holes = true;
    } else {
        limits.lower(block.area, block.count);
    }
}

/// Reads one block, in pieces where it is larger than the device takes (one long text).
async fn read_block(
    links: &Links,
    target: &Target,
    block: &Block,
    limits: &Limits,
    timeout: Duration,
) -> Result<Vec<u16>, LinkError> {
    let most = limits.most(block.area).max(1);
    let mut read = Vec::with_capacity(block.count as usize);
    let mut at = 0;
    while at < block.count {
        let count = (block.count - at).min(most);
        read.extend(
            access::read(
                links,
                target,
                block.area,
                block.address + at,
                count,
                timeout,
            )
            .await?,
        );
        at += count;
    }
    Ok(read)
}
