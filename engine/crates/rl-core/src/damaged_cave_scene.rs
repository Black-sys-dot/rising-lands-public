//! Narrow AffGame effects admission for the intact damaged CAVE caller.
//! This query does not draw, publish a draw, consume RNG or replace a renderer guard.
use crate::{MemoriaJogo, Result};

/// Only the actual slot0 sanctuary / slot1 CAVE / slot2 type1 scene may use this
/// extension. The parent retains its per-building, viewport and projectile checks.
/// AffBatiments407dd3 -> AffFlamme40323c and the selected gauge4021bf are
/// supported for this inhabited owner0/mode5/max100 CAVE. Other types, collapse,
/// neutral ownership, levels, bags and unknown unit/sprite/effect bodies stay closed.
pub fn admits(m: &MemoriaJogo, record: u32) -> Result<bool> {
    if record != 0x49a0c8 + 276
        || m.u8(0x508041)? != 0
        || m.u8(record + 8)? != 3
        || m.u8(record + 5)? != 5
        || m.u8(record + 4)? != 0
        || m.u16(record + 10)? != 100
        || m.u16(0x495d90 + 52 * 3)? != 100
        || !(1..=100).contains(&m.u16(record + 6)?)
        || m.u8(0x4dfb30)? != 0
        || m.u8(record + 0x74)? > 1
        || m.u8(record + 0x75)? > 1
        || m.u8(record + 0x76)? > 15
        || m.u16(record + 12)? != 1
        || m.u16(record + 14)? != 2
        || m.u8(record + 0x4e)? != 1
        || m.u8(record + 0x4f)? != 0
    {
        return Ok(false);
    }
    let sanctuary = 0x49a0c8;
    let neutral = sanctuary + 276 * 2;
    if m.u8(sanctuary + 8)? != 2
        || m.u8(sanctuary + 4)? != 0
        || !matches!(m.u8(sanctuary + 5)?, 4 | 5)
        || m.u16(sanctuary + 6)? != 0
        || m.u16(sanctuary + 10)? != 600
        || m.u8(neutral + 8)? != 1
        || m.u8(neutral + 4)? != 200
        || !matches!(m.u8(neutral + 5)?, 2 | 5)
        || m.u16(neutral + 6)? != 0
        || m.u16(neutral + 10)? > 400
    {
        return Ok(false);
    }
    for index in 3..1000 {
        if m.u8(sanctuary + 276 * index + 5)? != 0 {
            return Ok(false);
        }
    }
    // BatimentEstHabiter's real compact slot contains a colonist (type9).
    // Keep the original pointer/index read; never invent an occupied bag.
    let unit = u32::from(m.u16(record + 0x12)?);
    Ok(m.u8(m.u32(0x4e19a0)?.wrapping_add(276 * unit) + 1)? == 9)
}
