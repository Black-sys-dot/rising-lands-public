//! Independent per-frame research leaves; callers remain responsible for dispatch.
use crate::{FrameRegisters, MemoriaJogo, Result};

/// RechercheAddScientifique 0x470bf7: add DX to the owner's wrapping u16 tally.
/// Ordinary campaign callers use owners 0..3. The native routine masks AL and
/// does not bounds-check it; other byte owners require that exact address mapped.
pub fn add_scientists(memory: &mut MemoriaJogo, owner: u8, count: u16) -> Result<()> {
    let address = 0x508174 + 2 * u32::from(owner);
    memory.set_u16(address, memory.u16(address)?.wrapping_add(count))
}

/// Exact GPR adapter for the original leaf, including AL masking and RET.
pub fn add_scientists_registers(
    memory: &mut MemoriaJogo,
    mut registers: FrameRegisters,
) -> Result<FrameRegisters> {
    registers.eax &= 255;
    add_scientists(memory, registers.eax as u8, registers.edx as u16)?;
    registers.esp = registers.esp.wrapping_add(4);
    Ok(registers)
}
