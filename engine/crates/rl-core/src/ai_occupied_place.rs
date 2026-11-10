//! BatimentPlaceOccupe 0x4894f7, directly required by 0x4800f1.
//! See research/ai-occupied-place.md. No writes, RNG, bounds or grass test.
use crate::{MemoriaJogo, Result};

/// Return the first conflicting full-width scan cell, or exactly 65535.
/// Player is the original full EBX, never inferred from incoming DL.
pub fn occupied_cell(m: &MemoriaJogo, kind: u32, center: u32, player: u32) -> Result<u32> {
    let table = 0x495d8c + 52 * (kind & 255);
    let width = u32::from(m.u16(table)?);
    let height = u32::from(m.u16(table + 2)?);
    let mut cell = center
        .wrapping_sub(width >> 1)
        .wrapping_sub((height >> 1) << 8);
    for _ in 0..height {
        for _ in 0..width {
            if kind == 12 {
                let objects = m.u32(0x507d74)?;
                if m.u8(objects.wrapping_add(cell))? != m.u8(0x495bf8)?
                    || crate::player_build::gate(m, cell & 65535)? & 255 == 255
                {
                    return Ok(cell);
                }
            } else {
                let index = crate::player_input::selection_leaf(
                    m,
                    "ChercheIndiceBatiment",
                    cell & 65535,
                    0,
                    0,
                )?;
                if index != 65535 {
                    let owner = u32::from(m.u8(crate::player::structure(index) + 4)?);
                    if owner != player || owner == 200 {
                        return Ok(cell);
                    }
                }
            }
            cell = cell.wrapping_add(1);
        }
        cell = cell.wrapping_add(256).wrapping_sub(width);
    }
    Ok(65535)
}

/// The original two RUL operands plus the caller's full player metadata dword.
pub fn find(m: &mut MemoriaJogo, kind: u32, reference: u32, player: u32) -> Result<u32> {
    search(m, kind, Inputs::Values { reference, player })
}

enum Inputs {
    Values { reference: u32, player: u32 },
    Arguments(u32),
}

fn search(m: &mut MemoriaJogo, kind: u32, inputs: Inputs) -> Result<u32> {
    let (mut start, mut best_distance, mut best_building) = (0, u32::MAX, None);
    loop {
        let index = crate::ai_free_place::next_free(m, start, kind)?;
        if index == 65535 {
            break;
        }
        if m.u8(0x50c324u32.wrapping_add(index))? == 255 {
            // Native reads +0x1c before the center and occupied-cell calls.
            let player = match inputs {
                Inputs::Values { player, .. } => player,
                Inputs::Arguments(p) => m.u32(p.wrapping_add(0x1c))?,
            };
            let center = crate::ai_free_place::central_cell(m, index, kind)?;
            let conflict = occupied_cell(m, kind, center, player)?;
            if conflict != 65535 {
                let origin = u32::from(m.u16(0x50b18eu32.wrapping_add(index.wrapping_mul(2)))?);
                let reference = match inputs {
                    Inputs::Values { reference, .. } => reference,
                    Inputs::Arguments(p) => m.u32(p.wrapping_add(4))?,
                };
                let distance = crate::frame_movement::call(
                    m,
                    "Distance2d",
                    &[
                        reference & 255,
                        (reference >> 8) & 65535,
                        origin & 255,
                        origin >> 8,
                    ],
                )?;
                if distance <= best_distance {
                    best_distance = distance;
                    best_building = Some(crate::player_input::selection_leaf(
                        m,
                        "ChercheIndiceBatiment",
                        conflict & 65535,
                        0,
                        0,
                    )?);
                }
            }
        }
        start = index.wrapping_add(1);
    }
    match best_building {
        Some(index) => Ok(crate::frame_spawn::call(m, "GetCaseIndiceBatiment", &[index])? & 65535),
        None => Ok(u32::MAX),
    }
}

/// Original pointer ABI. Only EAX changes; supplied non-EAX GPRs are restored.
/// The reference and player fields are read lazily, as in the intact routine.
pub fn entry(m: &mut MemoriaJogo, mut r: crate::FrameRegisters) -> Result<crate::FrameRegisters> {
    let kind = m.u32(r.eax)?;
    r.eax = search(m, kind, Inputs::Arguments(r.eax))?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
