//! Which blocks to read for a set of points (concept section 6): points close together in one
//! request, never more than the device takes, never across a gap where the device refuses holes.

use crate::access::Limits;
use crate::protocol::Area;

/// Unused addresses a block may span between two points when the device allows holes.
pub const MOST_HOLE: u16 = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub area: Area,
    pub address: u16,
    pub count: u16,
}

impl Block {
    pub fn contains(&self, area: Area, address: u16, count: u16) -> bool {
        self.area == area
            && address >= self.address
            && u32::from(address) + u32::from(count)
                <= u32::from(self.address) + u32::from(self.count)
    }
}

/// Blocks covering every `(area, address, count)` item.
pub fn blocks(items: &[(Area, u16, u16)], limits: &Limits) -> Vec<Block> {
    let mut sorted: Vec<(Area, u16, u16)> = items.to_vec();
    sorted.sort();
    sorted.dedup();
    let hole = if limits.no_holes { 0 } else { MOST_HOLE };
    let mut blocks: Vec<Block> = Vec::new();
    for (area, address, count) in sorted {
        let end = u32::from(address) + u32::from(count);
        if let Some(last) = blocks.last_mut().filter(|last| last.area == area) {
            let last_end = u32::from(last.address) + u32::from(last.count);
            let span = end.max(last_end) - u32::from(last.address);
            if u32::from(address) <= last_end + u32::from(hole)
                && span <= u32::from(limits.most(area))
            {
                last.count = span as u16;
                continue;
            }
        }
        blocks.push(Block {
            area,
            address,
            count,
        });
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    const HR: Area = Area::HoldingRegister;

    #[test]
    fn neighbours_share_a_block_and_far_points_do_not() {
        let items = [(HR, 0, 2), (HR, 2, 2), (HR, 6, 1), (HR, 100, 1)];
        let planned = blocks(&items, &Limits::default());
        assert_eq!(
            planned,
            [
                Block {
                    area: HR,
                    address: 0,
                    count: 7
                },
                Block {
                    area: HR,
                    address: 100,
                    count: 1
                }
            ]
        );
    }

    #[test]
    fn a_device_without_holes_gets_only_touching_points_together() {
        let limits = Limits {
            no_holes: true,
            ..Limits::default()
        };
        let planned = blocks(&[(HR, 0, 2), (HR, 2, 1), (HR, 4, 1)], &limits);
        assert_eq!(planned.len(), 2);
        assert_eq!(
            planned[0],
            Block {
                area: HR,
                address: 0,
                count: 3
            }
        );
    }

    #[test]
    fn blocks_stay_within_the_device_limit_and_their_area() {
        let limits = Limits {
            most_registers: 4,
            ..Limits::default()
        };
        let items = [
            (HR, 0, 2),
            (HR, 2, 2),
            (HR, 4, 2),
            (Area::InputRegister, 6, 1),
        ];
        let planned = blocks(&items, &limits);
        assert_eq!(planned.len(), 3);
        assert!(planned.iter().all(|block| block.count <= 4));
        assert_eq!(
            planned
                .iter()
                .filter(|block| block.area == Area::InputRegister)
                .count(),
            1
        );
    }
}
