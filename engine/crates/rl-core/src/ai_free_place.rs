//! I_TrouvePlaceLibreProche 0x480064 and its bounded placement query leaves.
//! Direct intact-asm port; see research/ai-free-place.md. No allocation or RNG.
use crate::{FrameRegisters, MemoriaJogo, Result};

fn dimensions(m: &MemoriaJogo, kind: u32) -> Result<(u32, u32)> {
    let p = 0x495d8cu32.wrapping_add(52 * (kind & 255));
    Ok((u32::from(m.u16(p)?), u32::from(m.u16(p + 2)?)))
}

fn ghost_cell(m: &MemoriaJogo, index: u32) -> Result<u32> {
    Ok(u32::from(
        m.u16(0x50b18eu32.wrapping_add(index.wrapping_mul(2)))?,
    ))
}

pub(crate) fn central_cell(m: &MemoriaJogo, index: u32, kind: u32) -> Result<u32> {
    let (w, h) = dimensions(m, kind)?;
    Ok(ghost_cell(m, index)?
        .wrapping_add(w >> 1)
        .wrapping_add((h >> 1) << 8))
}

/// ChercheCaseBatiment 0x41b489 tests the entire rectangle, even mask holes
/// and destroyed buildings. Reuse the existing provider without changing it.
fn rectangle_building(m: &mut MemoriaJogo, cell: u32) -> Result<u32> {
    crate::chain_s1::entry(
        m,
        "ChercheCaseBatiment",
        // Only AX is read by this leaf. No DL/player argument exists here.
        FrameRegisters {
            eax: cell,
            ..FrameRegisters::default()
        },
    )
    .map(|r| r.eax)
}

/// GetNumIndiceTypeGhostFullFree 0x47baac. The type bit uses x86's masked
/// shift count; the rectangle query is deliberately before occupancy testing.
pub(crate) fn next_free(m: &mut MemoriaJogo, mut index: u32, kind: u32) -> Result<u32> {
    let bit = 1u32.wrapping_shl(kind & 255);
    while index < u32::from(m.u16(0x50b18c)?) {
        let cell = ghost_cell(m, index)?;
        if cell != 65535
            && m.u32(0x50b76cu32.wrapping_add(index.wrapping_mul(4)))? & bit == 0
            && rectangle_building(m, cell)? == 65535
        {
            return Ok(index);
        }
        index = index.wrapping_add(1);
    }
    Ok(65535)
}

/// BatimentOccupeCase 0x41aff8 uses the original collision mask, including
/// nonzero bytes other than 1. Its low-word mask index wraps before lookup.
fn occupies(m: &mut MemoriaJogo, index: u32, cell: u32) -> Result<bool> {
    let p = 0x49a0c8 + 276 * (index & 65535);
    let origin = u32::from(m.u16(p + 2)?);
    let kind = u32::from(m.u8(p + 8)?);
    let (w, h) = dimensions(m, kind)?;
    for y in 0..h {
        for x in 0..w {
            if origin + (y << 8) + x == cell & 65535
                && crate::frame_core::call(m, "BatimentCollision", &[kind, (w * y + x) & 65535])?
                    & 255
                    != 0
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// BatimentPlaceLibre 0x489376. Bounds are the original unsigned <=256
/// comparisons, independent of mission dimensions. Gate kind is full-width 12.
fn place_free(m: &mut MemoriaJogo, kind: u32, center: u32) -> Result<bool> {
    let (w, h) = dimensions(m, kind)?;
    let (half_w, half_h) = (w >> 1, h >> 1);
    let (x, y) = (center & 255, center >> 8);
    let mut cell = center.wrapping_sub(half_w).wrapping_sub(half_h << 8);
    if half_w > x
        || y < half_h
        || w.wrapping_sub(1).wrapping_add(x.wrapping_sub(half_w)) > 256
        || h.wrapping_sub(1).wrapping_add(y.wrapping_sub(half_h)) > 256
    {
        return Ok(false);
    }
    for _ in 0..h {
        for _ in 0..w {
            if kind == 12 {
                let objects = m.u32(0x507d74)?;
                if m.u8(objects.wrapping_add(cell))? != m.u8(0x495bf8)?
                    || crate::player_build::gate(m, cell)? == 255
                {
                    return Ok(false);
                }
            } else {
                let c = cell & 65535;
                let terrain = m.u32(0x507d70)?;
                let tile = u32::from(m.u16(terrain.wrapping_add(2 * c))?);
                if !(0x15d..0x1a5).contains(&tile) {
                    return Ok(false);
                }
                let occupancy = m.u32(0x4e0f38)?;
                let address = occupancy.wrapping_add(2 * c);
                if m.u8(address.wrapping_add(1))? & 128 != 0 {
                    let tag = u32::from(m.u16(address)?);
                    if occupies(m, (tag & 0x7fe0) >> 5, c)? {
                        return Ok(false);
                    }
                }
            }
            cell = cell.wrapping_add(1);
        }
        cell = cell.wrapping_add(256).wrapping_sub(w);
    }
    Ok(true)
}

/// Typed RULE API: the two original argument dwords, un-narrowed. The native
/// routine reads neither player nor incoming DL, and makes no callbacks/writes.
pub fn find(m: &mut MemoriaJogo, kind: u32, reference: u32) -> Result<u32> {
    search(m, kind, Reference::Value(reference))
}

enum Reference {
    Value(u32),
    Argument(u32),
}

fn search(m: &mut MemoriaJogo, kind: u32, reference: Reference) -> Result<u32> {
    let (mut start, mut best_distance, mut best) = (0, u32::MAX, None);
    loop {
        let index = next_free(m, start, kind)?;
        if index == 65535 {
            break;
        }
        if m.u8(0x50c324u32.wrapping_add(index))? == 255 {
            let center = central_cell(m, index, kind)?;
            if place_free(m, kind, center)? {
                let origin = ghost_cell(m, index)?;
                let reference = match reference {
                    Reference::Value(value) => value,
                    Reference::Argument(address) => m.u32(address)?,
                };
                // Distance2dFromCases takes 8 x bits and 16 y bits from each
                // full dword. Do not silently narrow the reference to u16.
                let dx = (reference & 255).abs_diff(origin & 255);
                let dy = ((reference >> 8) & 65535).abs_diff((origin >> 8) & 65535);
                let distance = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
                if distance <= best_distance {
                    best_distance = distance;
                    best = Some(index);
                }
            }
        }
        start = index.wrapping_add(1);
    }
    match best {
        Some(index) => central_cell(m, index, kind),
        None => Ok(u32::MAX),
    }
}

/// EAX points to the two dwords. All other data registers are restored,
/// including the supplied DL. ESP advances by exactly the return-address pop.
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let kind = m.u32(r.eax)?;
    r.eax = search(m, kind, Reference::Argument(r.eax.wrapping_add(4)))?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
