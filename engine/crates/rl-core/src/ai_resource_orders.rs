//! Native AI group resource orders: V_Recolter482020, V_ExploiteMine48213c and
//! V_SetNavetteCaveToCulture4820e7.
use crate::{frame::stop, MemoriaJogo, Result};

// SetMineEngin444f48, with the caller's zero-extended WORD cell.
fn mine_order(m: &mut MemoriaJogo, member: u32, cell: u16) -> Result<()> {
    let old = m.u32(0x4e19a0)?.wrapping_add(276 * member);
    crate::frame_spawn::reset_order(m, member)?;
    let current = m.u32(0x4e19a0)?.wrapping_add(276 * member);
    let entrance = cell.wrapping_add(256);
    m.set_u16(current + 0x7e, entrance)?;
    crate::frame_spawn::release(m, member)?;
    m.set_u8(current + 0x88, 0)?;
    m.set_u8(old + 0x89, 0)?;
    m.set_u8(old + 2, 5)?;
    m.set_u8(old + 0x82, 0)?;
    m.set_u16(old + 0xb2, entrance)
}

/// Traverse the native group cursor, skipping dead and already-working units.
/// Each resource order precedes SetDepotEngin's kind, cave and resource stores.
/// The original leaves the group's descriptor unchanged and returns1 even empty.
pub fn issue(m: &mut MemoriaJogo, arguments: &[u32], mine: bool) -> Result<u32> {
    let address = if mine { 0x48213c } else { 0x482020 };
    let [cell, cave, group, ..] = arguments else {
        return stop("resource group order missing arguments", address);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        let unit = u32::from(member);
        let record = m.u32(0x4e19a0)?.wrapping_add(276 * unit);
        if !matches!(m.u8(record + 2)?, 0 | 4 | 5) {
            if mine {
                mine_order(m, unit, *cell as u16)?;
            } else {
                crate::player_orders::call(
                    m,
                    "SetRecolteEngin",
                    [unit, *cell & 65535, 0, 1],
                    0x482069,
                )?;
            }
            // SetDepotEngin444dad: DL=2, BX=cave, zero-extended CL=resource.
            let record = m.u32(0x4e19a0)?.wrapping_add(276 * unit);
            m.set_u8(record + 0x82, 2)?;
            m.set_u16(record + 0x80, *cave as u16)?;
            m.set_u16(record + 0x84, if mine { 2 } else { 1 })?;
        }
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("resource order nonprogress group", address);
        }
    }
    Ok(1)
}

/// V_SetNavetteCaveToCulture 0x4820e7: every live group member (EnginEstVivant
/// 0x443dad) receives SetNavetteEngin 0x444dee towards the cave, then
/// SetDepotEngin 0x444dad with DL=2, BX=field and CL=1. Returns 1 even when the
/// group is empty; the native cursor is the shared group iterator.
pub fn shuttle(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [cave, field, group, ..] = arguments else {
        return stop("shuttle group order missing arguments", 0x4820e7);
    };
    let mut member = crate::frame_raid::first_unit(m, *group)?;
    let mut traversed = 0;
    while member != u16::MAX {
        let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
        if !matches!(m.u8(r + 2)?, 0 | 4) {
            m.set_u8(r + 2, 0x14)?;
            m.set_u8(r + 0x89, 0)?;
            let cell = crate::frame_core::building_cell(m, *cave & 65535)? as u16;
            m.set_u16(r + 0xb2, cell)?;
            m.set_u16(r + 0xc0, *cave as u16)?;
            m.set_u16(r + 0x7e, cell)?;
            if m.u16(0x496304)? == member {
                m.set_u8(0x4e0f36, 0)?;
                m.set_u8(0x4e0f37, 0)?;
                m.set_u16(0x496304, 0xffff)?;
                m.set_u32(0x4e0f3c, 0)?;
            }
            m.set_u8(r + 0x88, 0)?;
            m.set_u8(r + 0x82, 2)?;
            m.set_u16(r + 0x80, *field as u16)?;
            m.set_u16(r + 0x84, 1)?;
        }
        member = crate::frame_raid::next_unit(m)?;
        traversed += 1;
        if traversed > 10000 {
            return stop("shuttle order nonprogress group", 0x4820e7);
        }
    }
    Ok(1)
}
