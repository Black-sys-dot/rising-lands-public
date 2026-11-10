//! BIBLIOTHEQUE research production in BatimentNextEtape 0x4268d4.
//! Runtime dispatch and the enclosing structure tick remain caller-owned.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

/// Execute the complete type-7 branch at 0x426d61, including its occupancy
/// predicate. Returns the original EAX; no active-state filter exists here.
/// The enclosing CalculBatimentNextEtape owns active/dismantling guards.
pub fn next(memory: &mut MemoriaJogo, structure: u16) -> Result<u32> {
    let record = 0x49a0c8 + 276 * u32::from(structure);
    if memory.u8(record + 8)? != 7 {
        return stop(
            "Research building requires type 7 at native switch",
            0x426910,
        );
    }
    let count = |memory: &mut MemoriaJogo| {
        crate::chain_bags::body(
            memory,
            "BagContient",
            FrameRegisters {
                eax: record + 12,
                edx: 1,
                ebx: 9,
                ..Default::default()
            },
        )
        .map(|r| r.eax as u16)
    };
    // 0x426deb counts for habitation; 0x426e27 independently recounts for
    // production. Keep both calls, including the native zero-occupancy return.
    if count(memory)? == 0 {
        return Ok(0);
    }
    let scientists = count(memory)?;
    let owner = memory.u8(record + 4)?;
    crate::research_progress::add_scientists(memory, owner, scientists)?;
    Ok(u32::from(owner))
}

/// Full native entry/RET adapter for BatimentNextEtape, restricted to type 7.
/// AX selects the structure; the other six data GPRs are callee-preserved.
/// No global tick, research completion, reset, RNG draw or UI callback occurs.
pub fn next_registers(
    memory: &mut MemoriaJogo,
    mut registers: FrameRegisters,
) -> Result<FrameRegisters> {
    registers.eax = next(memory, registers.eax as u16)?;
    registers.esp = registers.esp.wrapping_add(4);
    Ok(registers)
}
