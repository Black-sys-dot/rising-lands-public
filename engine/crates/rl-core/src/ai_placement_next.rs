//! V_AttaquerBatiment, intact 0x4822ed..0x48232f and 0x481d74..0x481d7e.
//! The historical task name is placement-next; this RULE issues group attacks.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

fn attack_group(
    m: &mut MemoriaJogo,
    group: u32,
    mut target: impl FnMut(&MemoriaJogo) -> Result<u16>,
) -> Result<u32> {
    let mut member = crate::frame_raid::first_unit(m, group)?;
    while member != 0xffff {
        if crate::frame_spawn::call(m, "EnginEstVivant", &[u32::from(member)])? as u8 != 0 {
            // Native reads WORD[ESI] only for a living member, on each iteration.
            // Reuse the real order/reset/search-release provider in native order.
            let building = target(m)?;
            crate::player_orders::call(
                m,
                "SetAttaqueEngin",
                [u32::from(member), 3, u32::from(building), 0],
                0x482323,
            )?;
        }
        member = crate::frame_raid::next_unit(m)?;
    }
    Ok(1)
}

/// Scheduler operands: building index (low word), then full-width group index.
/// Player metadata is not read by this RULE. No callback, allocation or RNG draw.
pub fn from_arguments(m: &mut MemoriaJogo, arguments: &[u32]) -> Result<u32> {
    let [building, group, ..] = arguments else {
        return stop("V_AttaquerBatiment missing operands", 0x4822ed);
    };
    attack_group(m, *group, |_| Ok(*building as u16))
}

/// Exact pointer ABI: group dword at EAX+4; building word lazily at EAX.
/// Preserves all seven non-EAX data GPRs and wraps ESP+4; EAX returns one.
pub fn entry(m: &mut MemoriaJogo, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let pointer = r.eax;
    let group = m.u32(pointer.wrapping_add(4))?;
    r.eax = attack_group(m, group, |m| m.u16(pointer))?;
    r.esp = r.esp.wrapping_add(4);
    Ok(r)
}
