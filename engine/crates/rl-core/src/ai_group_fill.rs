//! I_CompleteMilitairesDansGroupe, intact SBL entry 0x47fce9.
//! Moves existing list members only. Source, ABI and proofs: validation/ai-group-fill.
use crate::{frame_raid, frame_spawn, EngineError, FrameRegisters as R, MemoriaJogo, Result};

fn group(index: u32) -> u32 {
    0x51a9e0u32.wrapping_add(index.wrapping_shl(5))
}
fn unit(m: &MemoriaJogo, member: u16) -> Result<u32> {
    Ok(m.u32(0x4e19a0)?.wrapping_add(u32::from(member) * 276))
}
fn stop(address: u32) -> EngineError {
    EngineError::UnportedCallee {
        name: "I_CompleteMilitairesDansGroupe nonprogress list",
        address,
    }
}
fn strength(m: &mut MemoriaJogo, index: u32) -> Result<u32> {
    let g = group(index);
    if g == 0 || m.u16(g.wrapping_add(0x12))? == 0 {
        return Ok(0);
    }
    let mut member = frame_raid::first_unit(m, index)?;
    let mut value = 0u32;
    let mut seen = std::collections::BTreeSet::new();
    while member != u16::MAX {
        if !seen.insert(member) {
            return Err(stop(0x48752f));
        }
        if frame_spawn::call(m, "EnginEstVivant", &[u32::from(member)])? as u8 != 0 {
            let kind = u32::from(m.u8(unit(m, member)?.wrapping_add(1))?);
            value = value.wrapping_add(m.u32(0x498cf4u32.wrapping_add(4 * kind))?);
        }
        member = frame_raid::next_unit(m)?;
    }
    Ok(value)
}

enum Arguments<'a> {
    Values(&'a [u32]),
    Pointer(u32),
}
impl Arguments<'_> {
    fn get(&self, m: &MemoriaJogo, index: u32) -> Result<u32> {
        match self {
            Self::Values(values) => Ok(values[index as usize]),
            Self::Pointer(pointer) => m.u32(pointer.wrapping_add(index * 4)),
        }
    }
}
/// Original four argument dwords. The task player/context never adds an owner
/// predicate: the native routine trusts the source list. Full capacity returns
/// the actual transferred strength (possibly zero), not an invented success.
pub fn fill(m: &mut MemoriaJogo, a: &[u32]) -> Result<u32> {
    if a.len() < 4 {
        return Err(EngineError::UnportedCallee {
            name: "I_CompleteMilitairesDansGroupe incomplete arguments",
            address: 0x47fce9,
        });
    }
    fill_arguments(m, Arguments::Values(a))
}
fn fill_arguments(m: &mut MemoriaJogo, a: Arguments<'_>) -> Result<u32> {
    let dest = group(a.get(m, 1)?);
    if dest == 0 {
        return Ok(0);
    }
    let source = group(a.get(m, 0)?);
    if source == 0 || m.u16(source.wrapping_add(0x12))? == 0 {
        return Ok(0);
    }
    let initial = if m.u16(dest.wrapping_add(0x12))? == 0 {
        0
    } else {
        strength(m, a.get(m, 1)?)?
    };
    let mut transferred = 0u32;
    let mut iterations = 0u32;
    loop {
        if (transferred.wrapping_add(initial) as i32) >= a.get(m, 2)? as i32
            || m.u16(dest.wrapping_add(0x12))? >= m.u16(dest.wrapping_add(0x14))?
        {
            return Ok(transferred);
        }
        let mut best_distance = 1_000_000u32;
        let mut best = u16::MAX;
        let mut member = frame_raid::first_unit(m, a.get(m, 0)?)?;
        let mut seen = std::collections::BTreeSet::new();
        while member != u16::MAX {
            if !seen.insert(member) {
                return Err(stop(0x47fd80));
            }
            let kind = u32::from(m.u8(unit(m, member)?.wrapping_add(1))?);
            let mask = 1u32.wrapping_shl(u32::from(m.u8(dest.wrapping_add(0x19))?));
            if m.u32(0x4989f8u32.wrapping_add(4 * kind))? & mask != 0
                && frame_spawn::call(m, "EnginEstVivant", &[u32::from(member)])? as u8 != 0
            {
                let cell = frame_spawn::call(m, "EnginGetCase", &[u32::from(member)])? & 65535;
                let reference = a.get(m, 3)?;
                let distance = crate::frame_movement::call(
                    m,
                    "Distance2d",
                    &[
                        reference & 255,
                        (reference >> 8) & 65535,
                        cell & 255,
                        cell >> 8,
                    ],
                )?;
                if (distance as i32) < best_distance as i32 {
                    best_distance = distance;
                    best = member;
                }
            }
            member = frame_raid::next_unit(m)?;
        }
        if best == u16::MAX {
            return Ok(transferred);
        }
        frame_raid::remove_unit(m, a.get(m, 0)?, best)?;
        frame_raid::add_unit(m, a.get(m, 1)?, best)?;
        let kind = u32::from(m.u8(unit(m, best)?.wrapping_add(1))?);
        transferred = transferred.wrapping_add(m.u32(0x498cf4u32.wrapping_add(4 * kind))?);
        if m.u16(source.wrapping_add(0x12))? == 0 {
            return Ok(transferred);
        }
        iterations += 1;
        if iterations > 65536 {
            return Err(stop(0x47fd45));
        }
    }
}
/// Exact completed GPR contract: EAX result, six saved GPRs, RET advances ESP4.
/// Memory/fault prefixes are exposed; no transaction or exception suppression.
pub fn entry(m: &mut MemoriaJogo, mut r: R) -> Result<R> {
    r.eax = fill_arguments(m, Arguments::Pointer(r.eax))?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
