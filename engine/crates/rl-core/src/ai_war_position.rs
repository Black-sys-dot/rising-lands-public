//! Intact iagWarSearchBattleFromPos 0x487594 and city message-4 continuation.
use crate::{ai_group_message::CityMessage, EngineError, MemoriaJogo, Result};

/// Actual iagWarAddEnnemyToBattle 0x487b78 call: EAX=player, EDX=battle, EBX=unit.
/// The city caller zero-extends its argument's low word, not the structure or
/// descriptor target. This consumer must run synchronously or return its stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarEnemy {
    pub player: u32,
    pub battle: u32,
    pub unit: u16,
}

/// EAX=full-width player, EDX=full-width signed position -> EAX=battle or -1.
/// EBX/ECX/ESI/EDI/EBP survive; EDX is scratch. Plain RET. No world stores, callbacks or RNG calls.
/// The first inclusive square match wins. -1 is the only free-position marker.
pub fn search(m: &MemoriaJogo, player: u32, position: u32) -> Result<u32> {
    Ok(search_state(m, player, position)?.0)
}

fn search_state(m: &MemoriaJogo, player: u32, position: u32) -> Result<(u32, u32)> {
    let position = position as i32;
    let x = position % 256;
    let y = position / 256;
    let base = 0x51bd24u32.wrapping_add(player.wrapping_mul(0x1144));
    let mut scratch = player.wrapping_mul(68);
    for index in 0..16u32 {
        let battle = base.wrapping_add(index.wrapping_mul(0x110));
        let other = m.u32(battle.wrapping_add(4))? as i32;
        scratch = other as u32;
        if other == -1 {
            continue;
        }
        let bx = other % 256;
        let by = other / 256;
        scratch = by as u32;
        let radius = i32::from(m.u16(battle.wrapping_add(12))?);
        if x < bx.wrapping_sub(radius)
            || x > bx.wrapping_add(radius)
            || y < by.wrapping_sub(radius)
            || y > by.wrapping_add(radius)
        {
            continue;
        }
        return Ok((index, scratch));
    }
    Ok((u32::MAX, scratch))
}

/// Exact query register transport, including the native scratch EDX exit.
pub fn search_registers(
    m: &MemoriaJogo,
    mut r: crate::FrameRegisters,
) -> Result<crate::FrameRegisters> {
    (r.eax, r.edx) = search_state(m, r.eax, r.edx)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

/// Compose the unchanged city's gates/messages and resume only its exact
/// 0x487594 dependency. No gate, cache, wake or list write is replayed.
/// Effects-only, matching ai_city_message::send's register contract.
pub fn send_with_enemy(
    m: &mut MemoriaJogo,
    structure: u16,
    message: u8,
    argument: u32,
    enemy: impl FnOnce(&mut MemoriaJogo, WarEnemy) -> Result<()>,
) -> Result<()> {
    match crate::ai_city_message::send(m, structure, message, argument) {
        Err(EngineError::UnportedCallee {
            name: "iagVilleSendMessage war query",
            address: 0x487594,
        }) => {}
        other => return other,
    }
    let owner = u32::from(
        m.u8(0x49a0c8u32
            .wrapping_add(276 * u32::from(structure))
            .wrapping_add(4))?,
    );
    let unit = argument as u16;
    let record = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(unit));
    let position = u32::from(m.u16(record.wrapping_add(0x7c))?);
    let battle = search(m, owner, position)?;
    if battle == u32::MAX {
        // 0x4848f4..0x484906: four stack zeros, ECX0, EBX remains
        // the structure index across the position getter and war search.
        crate::frame_tasks::post(m, &[owner, 0x13, u32::from(structure), 0, 0, 0, 0, 0])?;
        Ok(())
    } else {
        enemy(
            m,
            WarEnemy {
                player: owner,
                battle,
                unit,
            },
        )
    }
}

pub fn send(m: &mut MemoriaJogo, structure: u16, message: u8, argument: u32) -> Result<()> {
    send_with_enemy(m, structure, message, argument, |_, _| {
        Err(EngineError::UnportedCallee {
            name: "iagWarAddEnnemyToBattle city position callback",
            address: 0x487b78,
        })
    })
}

/// The genuine group descriptor passes EBX0 and preserves full ECX target.
pub fn descriptor(m: &mut MemoriaJogo, message: CityMessage) -> Result<()> {
    send(m, message.structure, message.message, 0)
}

fn square(position: i32, radius: i32) -> (i32, i32, i32, i32) {
    let x = position % 256;
    let y = position / 256;
    (
        (x - radius).max(0),
        (y - radius).max(0),
        (x + radius).min(255),
        (y + radius).min(255),
    )
}

/// iagWarTestUnitDangerInZone 0x4888ae: first living enemy unit standing on a
/// cell of the clamped square whose type is in the mask 0x3f1991f, or 0xffff.
pub(crate) fn dangerous_unit(
    m: &MemoriaJogo,
    player: u32,
    position: i32,
    radius: i32,
) -> Result<u32> {
    let (x0, y0, x1, y1) = square(position, radius);
    let grid = m.u32(0x4e0f38)?;
    let mut row = y0 << 8;
    while row <= (y1 << 8) {
        let mut column = x0;
        while column <= x1 {
            let cell = (row + column) as u16;
            let raw = m.u16(grid.wrapping_add(2 * u32::from(cell)))?;
            if raw & 0xc000 == 0x4000 {
                let unit = (raw & 0x3ff8) >> 3;
                let r = crate::player::unit(m, u32::from(unit))?;
                if !matches!(m.u8(r.wrapping_add(2))?, 0 | 4)
                    && m.u16(r.wrapping_add(0x7c))? == cell
                    && crate::chain_battle::enemy_unit(m, unit, player as u8)?
                    && 1u32.wrapping_shl(u32::from(m.u8(r.wrapping_add(1))?)) & 0x3f1991f != 0
                {
                    return Ok(u32::from(unit));
                }
            }
            column += 1;
        }
        row += 256;
    }
    Ok(0xffff)
}

/// iagWarTestBuildingDangerInZone 0x4889e6: first live, non-collapsing enemy
/// structure of type 10 or 11 tagged on a cell of the clamped square, or 0xffff.
pub(crate) fn dangerous_structure(
    m: &MemoriaJogo,
    player: u32,
    position: i32,
    radius: i32,
) -> Result<u32> {
    let (x0, y0, x1, y1) = square(position, radius);
    let grid = m.u32(0x4e0f38)?;
    let mut row = y0 << 8;
    while row <= (y1 << 8) {
        let mut column = x0;
        while column <= x1 {
            let cell = (row + column) as u16;
            let raw = m.u16(grid.wrapping_add(2 * u32::from(cell)))?;
            if raw & 0x8000 != 0 {
                let structure = u32::from((raw & 0x7fe0) >> 5);
                let b = 0x49a0c8 + 276 * structure;
                let owner = m.u8(b + 4)?;
                let state = m.u8(b + 5)?;
                if state != 0
                    && state != 3
                    && owner != 200
                    && !crate::chain_battle::allied(m, owner, player as u8)?
                    && 1u32.wrapping_shl(u32::from(m.u8(b + 8)?)) & 0xc00 != 0
                {
                    return Ok(structure);
                }
            }
            column += 1;
        }
        row += 256;
    }
    Ok(0xffff)
}

/// B_TestUniteDanger 0x47ff65: arguments [position, radius], player at +0x1c.
pub(crate) fn danger(m: &MemoriaJogo, player: u32, position: u32, radius: u32) -> Result<u32> {
    if dangerous_unit(m, player, position as i32, radius as i32)? != 0xffff
        || dangerous_structure(m, player, position as i32, radius as i32)? != 0xffff
    {
        Ok(1)
    } else {
        Ok(u32::MAX)
    }
}
