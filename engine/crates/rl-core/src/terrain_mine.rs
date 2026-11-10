//! Intact TerrainEstMine 0x476263 and its InputGame hover continuation.
use crate::{chain_work, text, FrameRegisters, MemoriaJogo, Result};

/// Exact EAX, including the branch-dependent upper bytes. Only AL is a bool.
/// The second tile-pointer/read executes only after the first range misses.
pub fn terrain_est_mine(m: &MemoriaJogo, eax: u32) -> Result<u32> {
    let offset = (eax & 0xffff).wrapping_mul(2);
    let tile = m.u16(m.u32(0x507d70)?.wrapping_add(offset))?;
    if (0x251..=0x252).contains(&tile) {
        return Ok((eax & !0xff) | 1);
    }
    let tile = u32::from(m.u16(m.u32(0x507d70)?.wrapping_add(offset))?);
    Ok((tile & !0xff) | u32::from((0x255..=0x256).contains(&tile)))
}

/// MineGetIndice 0x47544c. Reuse the existing exact 20-slot ordered search.
/// Its +1/+256/+257 additions are 32-bit, so edge cells do not wrap to zero.
pub fn mine_index(m: &MemoriaJogo, cell: u32) -> Result<u32> {
    chain_work::mine_index(m, cell)
}

/// MineGetCapacite 0x475541 has no index guard, including for AX=ffff.
/// Preserve the actual addressed read/failure instead of inventing capacity.
pub fn mine_capacity(m: &MemoriaJogo, index: u32) -> Result<u32> {
    Ok(u32::from(m.u16(
        0x509956u32.wrapping_add((index & 0xffff).wrapping_mul(4)),
    )?))
}

/// Full original return GPR ABI. Private saved registers/flags excluded.
pub fn entry(m: &MemoriaJogo, mode: u32, mut r: FrameRegisters) -> Result<FrameRegisters> {
    r.eax = match mode {
        0 => terrain_est_mine(m, r.eax)?,
        1 => mine_index(m, r.eax)?,
        2 => mine_capacity(m, r.eax)?,
        _ => return crate::frame::stop("unknown terrain mine fixture entry", 0x476263),
    };
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hover {
    pub message: u32,
    pub capacity: u32,
}

/// Caller 0x4299fe..0x429a31/0x429a5b. False continues to the pit query;
/// true sets message91, then gets index, then capacity, then immediate display.
/// This routine has no world stores and draws no RNG. Errors retain that order.
pub fn lookup(m: &MemoriaJogo, cell: u32) -> Result<Option<Hover>> {
    if terrain_est_mine(m, cell)? as u8 == 0 {
        return Ok(None);
    }
    let message = text::get(m, 91)?;
    let index = mine_index(m, cell)?;
    let capacity = mine_capacity(m, index)?;
    Ok(Some(Hover { message, capacity }))
}
