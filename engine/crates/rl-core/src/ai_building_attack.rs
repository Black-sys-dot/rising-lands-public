//! Building-target movement in CalculAttaqueEngin (intact 0x451914).
//! Mode 3 only. Aim/firing/recovery remain explicit native boundaries.
use crate::{frame::stop, player::unit, FrameRegisters, MemoriaJogo, Result};

/// Typed AX of BatimentGetCaseAdjacent4239a0. Its native high word contains
/// private stack residue; the caller compares/stores only AX. Scan order is y/x.
pub fn adjacent_cell(m: &mut MemoriaJogo, building: u16, cell: u16) -> Result<u16> {
    let a = crate::player::structure(u32::from(building));
    let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
    for dy in -1..=1 {
        if !(0..256).contains(&(y + dy)) {
            continue;
        }
        for dx in -1..=1 {
            if !(0..256).contains(&(x + dx)) {
                continue;
            }
            let candidate = (i32::from(cell) + dy * 256 + dx) as u32;
            let kind = u32::from(m.u8(a + 8)?);
            let width = u32::from(m.u16(0x495d8c + 52 * kind)?);
            let height = u32::from(m.u16(0x495d8e + 52 * kind)?);
            let origin = u32::from(m.u16(a + 2)?);
            for cy in 0..height {
                for cx in 0..width {
                    if origin + (cy << 8) + cx == candidate
                        && crate::frame_core::call(
                            m,
                            "BatimentCollision",
                            &[kind, (width * cy + cx) & 65535],
                        )? as u8
                            != 0
                    {
                        return Ok(candidate as u16);
                    }
                }
            }
        }
    }
    Ok(65535)
}

fn target(m: &mut MemoriaJogo, r: u32) -> Result<u16> {
    let building = m.u16(r + 0xb8)?;
    if m.u8(r + 0xb0)? <= 1 {
        let cell = adjacent_cell(m, building, m.u16(r + 0x7c)?)?;
        if cell != 65535 {
            return Ok(cell);
        }
    }
    Ok(crate::frame_core::building_cell(m, u32::from(building))? as u16)
}

/// EnginViseCible 0x451464 for a structure target (mode 3): the adjacent or
/// central target cell is resolved first for every kind; turret kinds 12 and
/// 15 then compare the 32-sector angle to the structure's centre with +0x94,
/// the others the doubled 8-sector heading with +0x91.
pub(crate) fn faces_target(m: &mut MemoriaJogo, r: u32) -> Result<bool> {
    let cell = target(m, r)?;
    let kind = m.u8(r + 1)?;
    let origin = m.u16(r + 8)?;
    if matches!(kind, 12 | 15) {
        let centre = crate::frame_core::building_cell(m, u32::from(m.u16(r + 0xb8)?))? as u16;
        return Ok(crate::ai_attack_aim::angle(m, origin, centre, 32)? == m.u8(r + 0x94)?);
    }
    let mut d = crate::ai_attack_aim::angle(m, origin, cell, 8)?
        .wrapping_mul(2)
        .wrapping_sub(1);
    if kind == 3 {
        d = ((u32::from(d) + 11) % 16 + 1) as u8;
    }
    Ok(d == m.u8(r + 0x91)?)
}

/// Mode-3 EnginAPortee446cb5, using existing battle range/adjacency/bonus.
/// The battle encoding cannot represent indices >=8000h; keep their genuine
/// native address access rather than narrowing such indices to another target.
pub fn in_range(m: &mut MemoriaJogo, u: u16) -> Result<bool> {
    let r = unit(m, u32::from(u))?;
    let building = m.u16(r + 0xb8)?;
    if building < 0x8000 {
        return crate::chain_battle::in_range(m, u, building | 0x8000);
    }
    let kind = m.u8(r + 1)?;
    let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(r)?), u32::from(kind), 4, 0)?;
    let cell = crate::frame_core::building_cell(m, u32::from(building))? as u16;
    if m.u8(r + 2)? == 24 {
        return stop("EnginAPortee spell range", 0x4746c9);
    }
    let origin = m.u16(r + 8)?;
    if kind == 16 {
        return Ok(origin == cell);
    }
    if (m.u8(r + 0x78)? == 0 || matches!(kind, 1 | 4 | 21))
        && crate::chain_entry::adjacent(m, building, origin)?
    {
        return Ok(true);
    }
    let dx = i32::from(origin & 255) - i32::from(cell & 255);
    let dy = i32::from(origin >> 8) - i32::from(cell >> 8);
    Ok((dx * dx + dy * dy) as u32 <= (u32::from(m.u8(r + 0xb0)?).wrapping_add(bonus) & 65535))
}

fn clear_line(m: &MemoriaJogo, r: u32) -> Result<bool> {
    if matches!(m.u8(r + 1)?, 4 | 8 | 12 | 22 | 23) {
        return Ok(true);
    }
    let target = crate::frame_core::building_cell(m, u32::from(m.u16(r + 0xb8)?))? as u16;
    let mut cell = m.u16(r + 0x7c)?;
    loop {
        let direction = crate::frame_path::direction(cell, target, false);
        cell = crate::frame_path::neighbor(m, cell, direction)?;
        let terrain = m.u16(m.u32(0x507d70)?.wrapping_add(2 * u32::from(cell)))?;
        if (0x1dd..0x260).contains(&terrain) || cell == target {
            return Ok(cell == target);
        }
        // The original repeats at ffff on an out-of-map ray. Keep a named
        // dependency rather than turning a native non-return into success.
        if cell == 65535 {
            return stop("CalculAttaqueEngin out-of-map firing ray", 0x45185d);
        }
    }
}

fn destination(m: &mut MemoriaJogo, u: u16, r: u32, cell: u16) -> Result<()> {
    if cell != m.u16(r + 0x7e)? {
        m.set_u16(r + 0x7e, cell)?;
        crate::frame_spawn::release(m, u32::from(u))?;
        m.set_u8(r + 0x88, 0)?;
    }
    Ok(())
}

/// Real movement effects. No damage, fake attack completion, RNG draw or
/// suppression of an existing movement/path error is introduced.
pub fn approach(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    if m.u8(r + 0xb7)? != 3 {
        return stop("CalculAttaqueEngin building mode", 0x451914);
    }
    match m.u8(r + 0x8a)? {
        0 => {}
        1 => return stop("CalculAttaqueEngin building firing", 0x4523dc),
        // 0x452aa7: the recovery phase is shared by every target mode.
        2 => return crate::chain_attack::recovery(m, u, r),
        3 => {
            if !matches!(m.u8(r + 2)?, 0 | 4) {
                m.set_u8(r + 2, 7)?;
                crate::frame_spawn::release(m, u32::from(u))?;
            }
            m.set_u16(r + 0xb8, 65535)?;
            m.set_u8(r + 0xa4, 0)?;
            return m.set_u8(r + 0xa9, 0);
        }
        _ => return Ok(()),
    }
    if m.u8(r + 0x86)? == 0 && m.u8(r + 0x87)? == 0 {
        if in_range(m, u)? && clear_line(m, r)? {
            return stop("CalculAttaqueEngin building aim", 0x4519bd);
        }
        if !in_range(m, u)? {
            let cell = target(m, r)?;
            destination(m, u, r, cell)?;
        } else {
            // Building mode takes the ordinary (non-unit16) blocked-line arm.
            // Native bypass types return here without movement or goal stores.
            if matches!(m.u8(r + 1)?, 4 | 8 | 12 | 22 | 23) {
                return Ok(());
            }
            let cell = target(m, r)?;
            destination(m, u, r, cell)?;
        }
    }
    crate::frame_movement::displace(m, u32::from(u))?;
    Ok(())
}

/// Existing state1 validation is reused unchanged before dispatching mode3.
pub fn state1(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    if crate::chain_attack::validate_target(m, u, r)? && m.u8(r + 2)? == 1 {
        approach(m, u)?;
    }
    Ok(())
}

/// Void effects fixture ABI: preserve all GPRs; only the return stack advances.
/// The native proof uses an explicit PUSH EAX/CALL/POP EAX/RET transport shim.
pub fn entry(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    approach(m, r.eax as u16)?;
    Ok(FrameRegisters {
        esp: r.esp.wrapping_add(4),
        ..r
    })
}
