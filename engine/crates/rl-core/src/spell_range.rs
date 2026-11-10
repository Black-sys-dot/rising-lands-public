//! EnginAPortee 446cb5, including spell branch 446e4a and ground branch 446e0a.
//! Intact native target modes/argument order; no cast acceptance or effect lifecycle.
use crate::{frame::stop, player::unit, spell_queries, FrameRegisters, MemoriaJogo, Result};

pub const IN_RANGE: u32 = 0x446cb5;
pub const ADJACENT_CELL: u32 = 0x446b0c;

/// Required EnginAdjacentCase leaf: AX=unit, DX=cell. Native scans the whole
/// expanded rectangle, including its interior, clipping each axis at 256.
/// Movement bytes short-circuit before size dispatch. Wider types read native
/// uninitialized locals and remain an addressed dependency, never a size default.
pub fn adjacent_cell(m: &MemoriaJogo, source: u16, target: u16) -> Result<bool> {
    let r = unit(m, u32::from(source))?;
    if m.u8(r + 0x86)? != 0 || m.u8(r + 0x87)? != 0 {
        return Ok(false);
    }
    let kind = m.u8(r + 1)?;
    if kind > 25 {
        return stop("EnginAdjacentCase uninitialized size locals", 0x446b89);
    }
    let mut width = if matches!(kind, 3 | 11..=16 | 25) {
        2_u32
    } else {
        1
    };
    let mut height = width;
    let mut cursor =
        u32::from(m.u16(r + 8)?).wrapping_sub((height - 1).wrapping_shl(8).wrapping_add(width - 1));
    let initial = cursor as u16;
    if initial & 255 != 0 {
        cursor = cursor.wrapping_sub(1);
    } else {
        width = width.wrapping_sub(1);
    }
    if initial >> 8 != 0 {
        cursor = cursor.wrapping_sub(256);
    } else {
        height = height.wrapping_sub(1);
    }
    let x = u32::from(cursor as u16 & 255);
    let y = u32::from(cursor as u16 >> 8);
    for dy in 0..height + 2 {
        if y + dy < 256 {
            for dx in 0..width + 2 {
                if x + dx < 256 && cursor as u16 == target {
                    return Ok(true);
                }
                cursor = cursor.wrapping_add(1);
            }
        }
        cursor = cursor.wrapping_add(256 - width - 2);
    }
    Ok(false)
}

/// Original EnginAPortee: AX=source, DL=mode, BX=target (no encoded high bit).
/// Mode1 resolves unit+7c; mode3 uses BatimentGetCase; mode5 reads source+b8
/// for distance, but passes BX to adjacency. Native supports no initialized
/// destination local for other modes, so those modes stop explicitly.
pub fn in_range(m: &mut MemoriaJogo, source: u16, mode: u8, target: u16) -> Result<bool> {
    let r = unit(m, u32::from(source))?;
    let base = u32::from(m.u8(r + 0xb0)?);
    let kind = u32::from(m.u8(r + 1)?);
    let owner = u32::from(m.u8(r)?);
    let origin = m.u16(r + 8)?;
    // This first bonus query precedes destination resolution even for spells.
    let mut range = base.wrapping_add(crate::frame_economy::bonus(m, owner, kind, 4, 0)?);
    let cell = match mode {
        1 => m.u16(unit(m, u32::from(target))? + 0x7c)?,
        3 => crate::frame_core::building_cell(m, u32::from(target))? as u16,
        5 => m.u16(r + 0xb8)?,
        _ => return stop("EnginAPortee uninitialized destination local", 0x446e72),
    };
    if m.u8(r + 2)? == 24 {
        let spell = u32::from(m.u8(r + 0xf4)?);
        range = spell_queries::entry(
            m,
            spell_queries::GET_PORTEE,
            FrameRegisters {
                eax: spell,
                ..FrameRegisters::default()
            },
        )?
        .eax;
        // Native re-reads kind and owner after the range query.
        range = range.wrapping_add(crate::frame_economy::bonus(
            m,
            u32::from(m.u8(r)?),
            u32::from(m.u8(r + 1)?),
            4,
            0,
        )?);
    } else {
        let kind = m.u8(r + 1)?;
        if kind == 16 {
            return Ok(origin == cell);
        }
        if m.u8(r + 0x78)? == 0 || matches!(kind, 1 | 4 | 21) {
            let adjacent = match mode {
                1 => crate::chain_battle::adjacent_units(m, r, target)?,
                3 => crate::chain_entry::adjacent(m, target, origin)?,
                5 => adjacent_cell(m, source, target)?,
                _ => unreachable!(),
            };
            if adjacent {
                return Ok(true);
            }
        }
    }
    let distance = crate::frame_movement::call(
        m,
        "Distance2d",
        &[
            u32::from(origin & 255),
            u32::from(origin >> 8),
            u32::from(cell & 255),
            u32::from(cell >> 8),
        ],
    )?;
    Ok(distance <= (range & 65535))
}

/// Typed byte-result ABI. AL and native saved GPRs/ESP are contractual;
/// EAX upper bits, EBX/EDX scratch, flags and callee-local stack are excluded.
/// Inputs retain the original register signature, including arbitrary upper bits.
pub fn entry(m: &mut MemoriaJogo, address: u32, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let answer = match address {
        IN_RANGE => in_range(m, r.eax as u16, r.edx as u8, r.ebx as u16)?,
        ADJACENT_CELL => adjacent_cell(m, r.eax as u16, r.edx as u16)?,
        _ => return Err(crate::EngineError::UnknownRoutine(address)),
    };
    r.eax = (r.eax & 0xffffff00) | u32::from(answer);
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
