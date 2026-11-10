//! BatimentNextEtape gate body (type 12, table entry 0x427bb7..0x427d5d) and
//! its click-driven toggles: the structure under the mouse 0x426799, the
//! straight gate line 0x4193d4 and the gate chain 0x4195ca. Direct intact-asm
//! port, including the original quirks (player-indexed state gate, x-tested
//! vertical walk, 8-bit row increments). Grid tagging reuses player_build.
use crate::{MemoriaJogo, Result};

fn record(structure: u32) -> u32 {
    0x49a0c8u32.wrapping_add(structure.wrapping_mul(276))
}

/// Structure index tagged on a grid cell, or 0xffff.
fn structure_at(m: &MemoriaJogo, cell: u16) -> Result<u32> {
    let raw = m.u16(m.u32(0x4e0f38)?.wrapping_add(2 * u32::from(cell)))?;
    Ok(if raw & 0x8000 != 0 {
        u32::from((raw & 0x7fe0) >> 5)
    } else {
        0xffff
    })
}

/// Object byte at `objects + offset`; the original reads neighbouring heap
/// bytes when a walk leaves the 64 KiB map (row 0 upwards, cell 0 leftwards).
/// That content is unknowable here, so it stays an explicit stop.
fn object(m: &MemoriaJogo, objects: u32, offset: i64, site: u32) -> Result<u8> {
    if !(0..65536).contains(&offset) {
        return Err(crate::EngineError::UnportedCallee {
            name: "gate walk reads outside the object map",
            address: site,
        });
    }
    m.u8(objects.wrapping_add(offset as u32))
}

fn bump_phase(m: &mut MemoriaJogo, structure: u32) -> Result<()> {
    let p = record(structure).wrapping_add(0x76);
    m.set_u8(p, m.u8(p)?.wrapping_add(1))
}

/// 0x426799: CL=local player -> structure of that player, with nonzero state,
/// at the mouse cell carrying the gate object; otherwise 0xffff. The original
/// also requires the structure record indexed by the player byte to be in
/// state 5 (an intact quirk, reproduced).
pub(crate) fn under_mouse(m: &MemoriaJogo, player: u8) -> Result<u32> {
    let row = (u32::from(m.u16(0x5099aa)?) >> 5).wrapping_add(u32::from(m.u16(0x508722)?));
    let column = (u32::from(m.u16(0x5099a8)?) >> 5).wrapping_add(u32::from(m.u16(0x508720)?));
    let cell = (row << 8).wrapping_add(column) as u16;
    if m.u8(m.u32(0x507d74)?.wrapping_add(u32::from(cell)))? != m.u8(0x495bf9)? {
        return Ok(0xffff);
    }
    for i in 0..1000u32 {
        if m.u8(record(u32::from(player)).wrapping_add(5))? != 5 {
            continue;
        }
        let r = record(i);
        if m.u16(r.wrapping_add(2))? != cell
            || m.u8(r.wrapping_add(5))? == 0
            || m.u8(r.wrapping_add(4))? != player
        {
            continue;
        }
        return Ok(i);
    }
    Ok(0xffff)
}

/// 0x4193d4: walk the gate object line through `cell` and advance the phase of
/// every tagged structure on it. Horizontal when the structure's orientation
/// word is zero, vertical otherwise; the upward walk stops at row zero, the
/// downward walk uses the original 8-bit row increment.
pub(crate) fn toggle_line(m: &mut MemoriaJogo, cell: u16, structure: u16) -> Result<()> {
    let objects = m.u32(0x507d74)?;
    let gate = m.u8(0x495bf9)?;
    let mut bx = u32::from(cell);
    if m.u16(record(u32::from(structure)).wrapping_add(0x7c))? == 0 {
        loop {
            if object(m, objects, i64::from(bx & 0xffff) - 1, 0x4193d4)? != gate {
                break;
            }
            bx = bx.wrapping_sub(1);
            if (bx & 0xffff) % 256 == 0 {
                break;
            }
        }
        loop {
            if object(m, objects, i64::from(bx & 0xffff), 0x4193d4)? != gate {
                break;
            }
            let s = structure_at(m, bx as u16)?;
            if s != 0xffff {
                bump_phase(m, s)?;
            }
            bx = bx.wrapping_add(1);
            if (bx & 0xffff) % 256 == 0xff {
                break;
            }
        }
    } else {
        loop {
            if object(m, objects, i64::from(bx & 0xffff) - 0x100, 0x4193d4)? != gate {
                break;
            }
            bx = bx.wrapping_sub(0x100);
            if (bx & 0xffff) / 256 == 0 {
                break;
            }
        }
        loop {
            if object(m, objects, i64::from(bx & 0xffff), 0x4193d4)? != gate {
                break;
            }
            let s = structure_at(m, bx as u16)?;
            if s != 0xffff {
                bump_phase(m, s)?;
            }
            // INC BH: the row byte wraps without carrying into the upper half.
            bx = (bx & 0xffff_00ff) | ((bx.wrapping_add(0x100)) & 0xff00);
            if (bx & 0xffff) / 256 == 0xff {
                break;
            }
        }
    }
    Ok(())
}

/// 0x4195ca: collect the contiguous gate cells through `cell` into a 255-entry
/// list, give up when any of them carries attribute bit 0, otherwise advance the
/// phase of every structure whose origin cell is listed.
pub(crate) fn toggle_chain(m: &mut MemoriaJogo, cell: u16, structure: u16) -> Result<()> {
    let objects = m.u32(0x507d74)?;
    let gate = m.u8(0x495bf9)?;
    let mut list = [0xffffu16; 255];
    list[0] = cell;
    let horizontal = m.u16(record(u32::from(structure)).wrapping_add(0x7c))? == 0;
    let mut count = 0usize;
    let x = u32::from(cell) % 256;
    // First pass: backwards (left or up) from the clicked cell.
    loop {
        let v = list[count];
        let previous = i64::from(v) - if horizontal { 1 } else { 0x100 };
        if object(m, objects, previous, 0x4195ca)? != gate {
            break;
        }
        if count + 1 >= list.len() {
            return Err(crate::EngineError::UnportedCallee {
                name: "gate chain list overflow",
                address: 0x4195ca,
            });
        }
        list[count + 1] = v;
        count += 1;
        list[count] = if horizontal {
            v.wrapping_sub(1)
        } else {
            v.wrapping_sub(0x100)
        };
        // The vertical walk tests the clicked column, not the row (intact quirk).
        let test = if horizontal {
            u32::from(list[count]) % 256
        } else {
            x
        };
        if test == 0 {
            break;
        }
    }
    // Second pass: forwards (right or down) from the far end.
    list[0] = list[count];
    count = 0;
    loop {
        let v = list[count];
        let next = i64::from(v) + if horizontal { 1 } else { 0x100 };
        if object(m, objects, next, 0x4195ca)? != gate {
            break;
        }
        if count + 1 >= list.len() {
            return Err(crate::EngineError::UnportedCallee {
                name: "gate chain list overflow",
                address: 0x4195ca,
            });
        }
        list[count + 1] = v;
        count += 1;
        list[count] = if horizontal {
            v.wrapping_add(1)
        } else {
            // INC on the row byte only.
            (v & 0x00ff) | (v.wrapping_add(0x100) & 0xff00)
        };
        let test = if horizontal {
            u32::from(list[count]) % 256
        } else {
            u32::from(list[count]) / 256
        };
        if test == 0 {
            break;
        }
    }
    let flags = m.u32(0x507da8)?;
    for &v in list.iter() {
        if v != 0xffff && m.u8(flags.wrapping_add(u32::from(v)))? & 1 != 0 {
            return Ok(());
        }
    }
    for s in 0..1000u32 {
        let origin = m.u16(record(s).wrapping_add(2))?;
        for &v in list.iter() {
            if v != 0xffff && origin == v {
                bump_phase(m, s)?;
            }
        }
    }
    Ok(())
}

/// Type-12 BatimentNextEtape body. A fresh click (0x5099af set, 0x4dfb50
/// clear) on an own gate in phase 0, 15 or 31 toggles its line or chain; then
/// this gate's own phase advances between those rest phases, opening the grid
/// cell at 15 and closing it again at 31 (wrapping to 0).
pub fn next(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = record(u32::from(structure));
    if m.u8(0x5099af)? != 0 && m.u8(0x4dfb50)? == 0 {
        m.set_u8(0x4dfb50, 1)?;
        let local = m.u8(0x508041)?;
        let clicked = under_mouse(m, local)?;
        if clicked != 0xffff {
            let c = record(clicked);
            let cell = m.u16(c.wrapping_add(2))?;
            let phase = m.u8(c.wrapping_add(0x76))?;
            if matches!(phase, 0 | 0xf | 0x1f) {
                if phase == 0xf {
                    toggle_chain(m, cell, clicked as u16)?;
                } else {
                    toggle_line(m, cell, clicked as u16)?;
                }
            }
        }
    }
    let phase = m.u8(r.wrapping_add(0x76))?;
    if matches!(phase, 0 | 0xf | 0x1f) {
        return Ok(());
    }
    let mut next = phase.wrapping_add(1);
    m.set_u8(r.wrapping_add(0x76), next)?;
    if next >= 0x1f {
        next = 0;
        m.set_u8(r.wrapping_add(0x76), 0)?;
    }
    let cell = m.u16(r.wrapping_add(2))?;
    let flag = m.u32(0x507da8)?.wrapping_add(u32::from(cell));
    if next == 0xf {
        m.set_u8(flag, m.u8(flag)? & !1)?;
        crate::player_build::untag_cell(m, u32::from(structure), u32::from(cell), 0)?;
    } else if next == 0 {
        m.set_u8(flag, m.u8(flag)? | 1)?;
        crate::player_build::tag_cell(m, u32::from(structure), u32::from(cell), 0)?;
    }
    Ok(())
}
