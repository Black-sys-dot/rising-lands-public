//! Intact iab* behaviour handlers reached through iagUnitSendMessage 0x4829c8:
//! battle handlers 0x48986f (behaviour 5), 0x489957 (behaviour 6) and 0x489a0f
//! (behaviours 4 and 7) for messages 0x48/0x49, the filtered detection 0x489ace
//! (behaviours 1, 3, 8, 11, message 0x49), the structure detection 0x489c36
//! (message 0x4c), the behaviour-4 attack-ended handler 0x489d6c (0x4a) and the
//! behaviour 4-7 hunger handler 0x4897c9 (0x4b) with its cave search 0x4860a3,
//! with their callees iagWarSearchBatimentFromPos 0x487a7b and iagGroupSendMessage
//! 0x488cea; iagWarNewBattle 0x487643 reuses frame_raid's port. Direct asm port.
use crate::{player::unit, MemoriaJogo, Result};

fn war(owner: u32, conflict: u32) -> u32 {
    0x51bd24u32
        .wrapping_add(owner.wrapping_mul(0x1144))
        .wrapping_add(conflict.wrapping_mul(0x110))
}

/// 0x487a7b: EAX=player, EDX=signed position, EBX=radius -> structure or 0xffff.
/// Rows then columns, clamped to the 256x256 map; the first structure owned
/// by the player or by the neutral owner 200 wins.
pub(crate) fn nearby_structure(
    m: &MemoriaJogo,
    player: u32,
    position: i32,
    radius: i32,
) -> Result<u32> {
    let x = position % 256;
    let y = position / 256;
    let x0 = (x - radius).max(0);
    let y0 = (y - radius).max(0);
    let x1 = (x + radius).min(255);
    let y1 = (y + radius).min(255);
    let grid = m.u32(0x4e0f38)?;
    let mut row = y0 << 8;
    while row <= (y1 << 8) {
        let mut column = x0;
        while column <= x1 {
            let cell = (row + column) as u16;
            let raw = m.u16(grid.wrapping_add(2 * u32::from(cell)))?;
            if raw & 0x8000 != 0 {
                let structure = u32::from((raw & 0x7fe0) >> 5);
                let owner = u32::from(m.u8(0x49a0c8 + 276 * structure + 4)?);
                if owner == player || owner == 200 {
                    return Ok(structure);
                }
            }
            column += 1;
        }
        row += 256;
    }
    Ok(0xffff)
}

/// 0x488cea: wake the battle's task, if any, with EBX as its result.
fn group_message(m: &mut MemoriaJogo, player: u32, battle: u32, result: u32) -> Result<()> {
    let task = m.u32(war(player, battle).wrapping_add(8))?;
    if task != u32::MAX {
        crate::frame_tasks::wake(m, task, result)?;
    }
    Ok(())
}

/// Shared body of 0x48986f/0x489957/0x489a0f; `order` is the posted task kind
/// (0x1f, 0x1e or 0x20) when no battle or nearby structure exists.
pub(crate) fn battle(m: &mut MemoriaJogo, member: u16, target: u16, order: u32) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    let player = u32::from(m.u8(r)?);
    let t = unit(m, u32::from(target))?;
    let cell = m.u16(t.wrapping_add(0x7c))?;
    let battle = crate::ai_war_position::search(m, player, u32::from(cell))?;
    if battle != u32::MAX {
        if crate::chain_battle::add_enemy(m, player as u8, battle, target)? {
            group_message(m, player, battle, u32::MAX)?;
        }
        return Ok(());
    }
    let structure = nearby_structure(m, player, i32::from(cell), 5)?;
    if structure != 0xffff {
        crate::frame_tasks::post(m, &[player, 0x13, structure, 0, 0, 0, 0, 0])?;
        return Ok(());
    }
    crate::frame_raid::new_battle(m, player, u32::from(cell), 5, u32::MAX)?;
    let group = m.u32(r.wrapping_add(0x10c))?;
    let target_owner = u32::from(m.u8(t)?);
    crate::frame_tasks::post(
        m,
        &[player, order, target_owner, u32::from(cell), group, 0, 0, 0],
    )?;
    Ok(())
}

/// 0x489ace: detection restricted to target types in the mask 0x3f1991f.
pub(crate) fn filtered_detection(m: &mut MemoriaJogo, member: u16, target: u16) -> Result<()> {
    let kind = m.u8(unit(m, u32::from(target))?.wrapping_add(1))?;
    if 1u32.wrapping_shl(u32::from(kind)) & 0x3f1991f == 0 {
        return Ok(());
    }
    crate::chain_battle::detection(m, member, target)
}

/// 0x489c36: structure detection; the target is encoded with bit 15 set.
pub(crate) fn structure_detection(m: &mut MemoriaJogo, member: u16, target: u16) -> Result<()> {
    crate::chain_battle::detection(m, member, target | 0x8000)
}

/// 0x489d6c: behaviour-4 attack ended; move to the current cell, clear the target.
pub(crate) fn ended(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    let r = unit(m, u32::from(member))?;
    let cell = m.u16(r.wrapping_add(0x7c))?;
    crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(member), u32::from(cell)])?;
    m.set_u16(r.wrapping_add(0xb8), 0xffff)
}

/// 0x4860a3: nearest listed CAVE (owner slots 0x51ba3c+168p+0x8c, eight
/// entries) within squared distance 1024 of the unit, or 0xffff.
fn nearest_cave(m: &MemoriaJogo, member: u16) -> Result<u32> {
    let r = unit(m, u32::from(member))?;
    let owner = u32::from(m.u8(r)?);
    if owner >= 4 {
        return Ok(0xffff);
    }
    let cell = m.u16(r.wrapping_add(0x7c))?;
    let base = 0x51ba3cu32
        .wrapping_add(owner.wrapping_mul(168))
        .wrapping_add(0x8c);
    let mut best = 0x1ff_ffffu32;
    let mut found = 0xffffu32;
    for slot in 0..8u32 {
        let structure = m.u16(base.wrapping_add(2 * slot))?;
        if structure == 0xffff {
            continue;
        }
        let other = crate::frame_core::building_cell(m, u32::from(structure))? as u16;
        let dx = u32::from((cell & 255).abs_diff(other & 255));
        let dy = u32::from((cell >> 8).abs_diff(other >> 8));
        let distance = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
        if distance >= best || distance >= 0x400 {
            continue;
        }
        best = distance;
        found = u32::from(structure);
    }
    Ok(found)
}

/// 0x4897c9: hunger for behaviours 4–7 (message 0x4b). After EnginSetFatigue,
/// a unit not already in state 19 heads to the nearest listed cave (saving its
/// state/destination at +0xa0/+0xa2, cave at +0xc2) or, without one, copies
/// its +0x76 word into +4 and clears +0x74.
pub(crate) fn hunger(m: &mut MemoriaJogo, member: u16) -> Result<()> {
    crate::chain_fatigue::set(m, u32::from(member))?;
    let r = unit(m, u32::from(member))?;
    if m.u8(r.wrapping_add(2))? == 0x13 {
        return Ok(());
    }
    let cave = nearest_cave(m, member)?;
    if cave == 0xffff {
        let phase = m.u16(r.wrapping_add(0x76))?;
        m.set_u16(r.wrapping_add(4), phase)?;
        return m.set_u16(r.wrapping_add(0x74), 0);
    }
    m.set_u16(r.wrapping_add(0xa0), u16::from(m.u8(r.wrapping_add(2))?))?;
    m.set_u16(r.wrapping_add(0xa2), m.u16(r.wrapping_add(0x7e))?)?;
    let cell = crate::frame_core::building_cell(m, cave)? as u16;
    m.set_u16(r.wrapping_add(0xc2), cave as u16)?;
    m.set_u8(r.wrapping_add(2), 0x13)?;
    m.set_u16(r.wrapping_add(0x7e), cell)?;
    if m.u16(0x496304)? == member {
        m.set_u8(0x4e0f36, 0)?;
        m.set_u8(0x4e0f37, 0)?;
        m.set_u16(0x496304, 0xffff)?;
        m.set_u32(0x4e0f3c, 0)?;
    }
    m.set_u8(r.wrapping_add(0x88), 0)
}
