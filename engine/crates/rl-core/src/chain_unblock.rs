//! Failed-path blocker displacement449176 and adjacent free-cell44330c.
use crate::{
    player::{unit, Memory},
    MemoriaJogo, Result,
};

/// Native AX. The lower corner clamps to zero, but index4 is still skipped;
/// at a map edge that can admit the original cell. No new center exclusion.
pub(crate) fn free_neighbor(m: &MemoriaJogo, center: u16, member: u16) -> Result<u16> {
    let r = unit(m, u32::from(member))?;
    let air = m.b(r + 0xab)? != 0;
    let x = (center & 255).saturating_sub(1);
    let y = (center >> 8).saturating_sub(1);
    // Native initializes unused stack-only dimension words from this byte.
    m.b(r + 1)?;
    let width = m.w(0x497168)?;
    let height = m.w(0x49716a)?;
    for index in 0..9u16 {
        if index == 4 {
            continue;
        }
        let cx = u32::from(x + index % 3);
        let cy = u32::from(y + index / 3);
        if cx >= width || cy >= height {
            continue;
        }
        let cell = cx + 256 * cy;
        // 0x44333e: BL is 1 on the ground and 2 for a flying unit.
        if crate::chain_search::passable_layer(m, u32::from(member), cell, if air { 2 } else { 1 })?
        {
            return Ok(cell as u16);
        }
    }
    Ok(65535)
}

/// Native AL: true for a stationary same-owner blocker even when no free
/// adjacent cell exists. Attempts are d, d-2, d, exactly as the intact loop.
pub(crate) fn unblock(m: &mut MemoriaJogo, member: u16) -> Result<bool> {
    let r = unit(m, u32::from(member))?;
    let mut direction =
        crate::frame_path::direction(m.w(r + 8)? as u16, m.w(r + 0x7e)? as u16, false);
    let mut found = false;
    for attempt in 0..3 {
        let cell = crate::frame_path::neighbor(m, m.w(r + 8)? as u16, direction)?;
        if cell != 65535 {
            let other =
                crate::frame_movement::call(m, "ChercheIndiceEngin", &[u32::from(cell)])? as u16;
            m.sw(0x4e1a34, u32::from(other))?;
            if other != 65535 {
                let q = unit(m, u32::from(other))?;
                if m.b(q)? == m.b(r)? && m.w(q + 8)? == m.w(q + 0x7e)? {
                    let free = free_neighbor(m, cell, other)?;
                    m.sw(0x4e1a36, u32::from(free))?;
                    if free != 65535 {
                        crate::frame_spawn::order(m, u32::from(other), free)?;
                    }
                    found = true;
                }
            }
        }
        if attempt == 0 {
            for _ in 0..2 {
                direction = if direction == 1 {
                    16
                } else {
                    direction.wrapping_sub(1)
                };
            }
        } else if attempt == 1 {
            for _ in 0..2 {
                direction = direction % 16 + 1;
            }
        }
    }
    m.sb(r + 0x8b, 0)?;
    m.sb(r + 0x88, 1)?;
    Ok(found)
}
