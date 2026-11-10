//! TerrainEstGc 0x4762b9..0x4762de, translated from the intact executable.
//! This leaf reads the object grid, preserves every non-return GPR, writes no
//! game memory and consumes no RNG. Coordinates are packed in AX, not EAX.
use crate::{FrameRegisters, MemoriaJogo, Result};

pub const TERRAIN_EST_GC: u32 = 0x4762b9;

/// Original value ABI: EAX is exactly zero or one. The original makes no map
/// dimension, collision, owner, tile or visibility check. Missing declared
/// memory is an error; the object address uses explicit 32-bit wrapping.
pub fn terrain_est_gc(m: &MemoriaJogo, cell: u32) -> Result<u32> {
    let objects = m.u32(0x507d74)?;
    let object = m.u8(objects.wrapping_add(cell & 0xffff))?;
    Ok(u32::from((0x1d..0x35).contains(&object)))
}

/// Full GPR ABI at RET. The caller return address has been popped; native
/// PUSH/POP EDX stack scratch and arithmetic flags are outside this value API.
/// Neither the pointer slot nor the object byte may alias that private stack
/// save/return address. Such aliases require native stack context and are
/// outside the API. No original callees or existing providers are used.
pub fn entry(m: &MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    r.eax = terrain_est_gc(m, r.eax)?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
