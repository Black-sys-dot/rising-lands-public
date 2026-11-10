//! Intact AffFlamme 0x40323c..0x407dd3: the per-type flame layout (affflamme_table.rs,
//! read from the intact routine by affflamme_extract.py and checked against it).
//! Reuses ordinary PasteBobCmp and the caller's original adjusted coordinates.
use crate::affflamme_table::FLAMES;
use crate::render::{i16, Context, Result};

impl Context<'_> {
    /// AX=x, DX=y, BX=cell, CX=index, stack type=3. No RNG or world stores.
    pub fn damaged_structure_fire(&mut self, x: i32, y: i32, cell: u32, index: u32) -> Result<()> {
        // The pushed layout kind is the facade's (3); the record's own type
        // only enters through BatimentGetDegat inside aff_flamme.
        self.aff_flamme(x, y, cell, index, 3)
    }
    /// AffFlamme(EAX=x, EDX=y, EBX=cell, ECX=index, push kind), called from the shared
    /// AffBatiments tail 0x40d5e2 with the facade origin. BatimentGetDegat 0x41a940 returns
    /// AX = 200 * damage(+6) / maximum of the record's own type (0x495d90 + 52*type).
    pub fn aff_flamme(&mut self, x: i32, y: i32, cell: u32, index: u32, kind: u8) -> Result<()> {
        let entries = FLAMES[kind as usize];
        if entries.is_empty() {
            return Ok(());
        }
        let record = 0x49a0c8 + 276 * (index & 0xffff);
        let maximum = self.u16(0x495d90 + 52 * self.u8(record + 8)?)?;
        if maximum == 0 {
            return self.stop(0x41a93b, "BatimentGetDegat original division by zero");
        }
        // The native pure query repeats at each threshold. No intervening draw
        // changes its inputs; retain the low AX result and original draw order.
        let damage = (200 * self.u16(record + 6)? / maximum) & 0xffff;
        let clock = self.u16(0x508f80)?;
        for &(threshold, dx, dy, base, a, b, offset) in entries {
            if damage > u32::from(threshold) {
                let frame =
                    (u32::from(a) * clock + u32::from(b) * (cell & 0xffff) + u32::from(offset))
                        & 31;
                self.bob(
                    i16(x + i32::from(dx)),
                    i16(y + i32::from(dy)),
                    (u32::from(base) + frame) & 0xffff,
                    true,
                    None,
                )?;
            }
        }
        Ok(())
    }
}
