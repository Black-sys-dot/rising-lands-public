//! BatimentNextEtape population body, native0x426c10..0x426d5c (type4 table entry),
//! and the AI cheat V_TricheColon 0x482579 that shares its creation tail.
use crate::{FrameRegisters, MemoriaJogo, Result};
fn bag(m: &mut MemoriaJogo, a: u32, name: &str, tag: u32, value: u32) -> Result<u32> {
    Ok(crate::chain_bags::body(
        m,
        name,
        FrameRegisters {
            eax: a,
            edx: tag,
            ebx: value,
            ..Default::default()
        },
    )?
    .eax)
}
pub(crate) fn step(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(index);
    let b = r + 12;
    let kind = m.u8(r + 8)?;
    let eligible = if kind == 21 {
        bag(m, b, "BagContientTroupe", 0, 0)? as u8 != 0
    } else {
        kind == 2 && bag(m, b, "BagContientTroupe", 0, 0)? as u8 != 0
            || bag(m, b, "BagContient", 1, 9)? as u16 != 0
    };
    if !eligible {
        return Ok(());
    }
    let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(r + 4)?), 9, 2, 0)? as u16;
    if i32::from(m.u16(r + 0x78)?) < 5 - i32::from(bonus) {
        return Ok(());
    }
    // EnginLibre0x457e0c is a read-only first-free-slot scan; creation performs
    // its own scan after BagAccepte, in the same order as the intact routine.
    let pool = m.u32(0x4e19a0)?;
    let mut free = false;
    for i in 0..800 {
        if m.u8(pool + 276 * i + 2)? == 0 {
            free = true;
            break;
        }
    }
    if !free || bag(m, b, "BagAccepte", 1, 9)? as u8 == 0 {
        return Ok(());
    }
    m.set_u16(r + 0x78, 0)?;
    let newborn = crate::frame_spawn::call(
        m,
        "CreerEngin",
        &[u32::from(m.u16(r + 2)?), 1, 9, u32::from(m.u8(r + 4)?)],
    )? as u16;
    // Native ignores the lodging return value before EnginSetModeEtat(14).
    crate::chain_entry::lodge(m, index, newborn)?;
    let u = crate::player::unit(m, u32::from(newborn))?;
    if !matches!(m.u8(u + 2)?, 0 | 4) {
        m.set_u8(u + 2, 14)?;
        crate::frame_spawn::release(m, u32::from(newborn))?;
    }
    crate::frame_ai::call(m, "iagUnitSendMessage", &[u32::from(newborn), 0x43, 0])?;
    Ok(())
}

/// V_TricheColon 0x482579: RULE arguments [structure], player at +0x1c. The
/// free-slot scan and BagAccepte(1) gate a type-9 creation at the structure's
/// cell/owner, lodging, EnginSetEtat(14) and insertion into the player's
/// kind-10 group list head (iagUnitAddToGroup 0x482939). Returns 1 or -1.
pub(crate) fn cheat(m: &mut MemoriaJogo, structure: u32, player: u32) -> Result<u32> {
    let r = 0x49a0c8u32.wrapping_add(structure.wrapping_mul(276));
    let pool = m.u32(0x4e19a0)?;
    let mut free = false;
    for i in 0..800 {
        if m.u8(pool + 276 * i + 2)? == 0 {
            free = true;
            break;
        }
    }
    if !free || bag(m, r + 12, "BagAccepte", 1, 9)? as u8 == 0 {
        return Ok(u32::MAX);
    }
    let newborn = crate::frame_spawn::call(
        m,
        "CreerEngin",
        &[u32::from(m.u16(r + 2)?), 1, 9, u32::from(m.u8(r + 4)?)],
    )? as u16;
    crate::chain_entry::lodge(m, structure as u16, newborn)?;
    let u = crate::player::unit(m, u32::from(newborn))?;
    if !matches!(m.u8(u + 2)?, 0 | 4) {
        m.set_u8(u + 2, 14)?;
        crate::frame_spawn::release(m, u32::from(newborn))?;
    }
    let group = m.u32(
        0x51b664u32
            .wrapping_add(player.wrapping_mul(96))
            .wrapping_add(40),
    )?;
    crate::frame_raid::call(m, "iagUnitAddToGroup", &[group, u32::from(newborn)])?;
    Ok(1)
}
