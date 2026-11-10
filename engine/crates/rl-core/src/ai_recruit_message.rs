//! iagUnitSendMessage45/46, intact482cbb..482d77.
//! The caller has already checked the owner's AI type and message interval.
use crate::{MemoriaJogo, Result};

/// Resume the existing city prefix only for recruitment's message3.
/// Non-AI return values and every other boundary retain their previous contract.
pub(crate) fn city(m: &mut MemoriaJogo, structure: u32) -> Result<u32> {
    match crate::chain_queries::city(m, structure) {
        Err(crate::EngineError::UnportedCallee {
            name: "iagVilleSendMessage AI city handler",
            address: 0x48484c,
        }) => {
            // The existing full body repeats only the read-only owner/type gates.
            crate::ai_city_message::send(m, structure as u16, 3, 0)?;
            Ok(0)
        }
        other => other,
    }
}

pub(crate) fn available(m: &mut MemoriaJogo, member: u16, from_housed: bool) -> Result<()> {
    let r = m.u32(0x4e19a0)?.wrapping_add(276 * u32::from(member));
    if from_housed {
        let owner = u32::from(m.u8(r)?);
        let housed = m.u32(0x51b68c + 96 * owner)?;
        crate::frame_raid::remove_unit(m, housed, member)?;
    }
    let owner = u32::from(m.u8(r)?);
    let fallback = m.u32(0x51b664 + 96 * owner)?;
    crate::ai_stage25::assign_released(m, member, fallback)?;
    let kind = u32::from(m.u8(r + 1)?);
    if m.u8(0x4989f9 + 4 * kind)? & 2 != 0 {
        let owner = u32::from(m.u8(r)?);
        crate::frame_tasks::post(m, &[owner, 0x10, u32::from(member), 0, 0, 0, 0, 0])?;
    }
    Ok(())
}
