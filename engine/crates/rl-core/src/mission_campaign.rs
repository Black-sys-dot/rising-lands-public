//! Campaign filename selection, progression and exact carry-over stores.
use crate::{
    mission_load::{copy, mission_name},
    MemoriaJogo, Result,
};

/// MissionNext 0x46c08c increments the stage even when the next file is absent.
/// `exists` is the host FileExists boundary; it must inspect the supplied original path.
pub fn next(m: &mut MemoriaJogo, mut exists: impl FnMut(&[u8]) -> Result<bool>) -> Result<bool> {
    let stage = m.u8(0x496fe4)?.wrapping_add(1);
    m.set_u8(0x496fe4, stage)?;
    let name = mission_name(m, stage)?;
    let mut path = crate::mission_startup::string(m, 0x509594)?;
    path.extend(name);
    exists(&path)
}
/// Engin_StoreExperimente 0x4414f8: five best active-unit experiences per owner.
/// Original owner arithmetic is retained, including the unguarded non-player-owner path.
pub fn store_experience(m: &mut MemoriaJogo) -> Result<()> {
    for p in 0..4 {
        for i in 0..5 {
            m.set_u16(0x4e19ec + 10 * p + 2 * i, 255)?;
        }
    }
    for i in 0..800 {
        let u = crate::frame_spawn::unit(m, i)?;
        if matches!(m.u8(u + 2)?, 0 | 4) {
            continue;
        }
        let saved = 0x4e19ec + 10 * u32::from(m.u8(u)?);
        let mut lowest = m.u8(saved + 1)?;
        let mut index = 0;
        for k in 1..5 {
            let value = m.u8(saved + 2 * k + 1)?;
            if lowest > value {
                lowest = value;
                index = k;
            }
        }
        let experience = m.u8(u + 0xaf)?;
        if lowest < experience {
            m.set_u8(saved + 2 * index + 1, experience)?;
            m.set_u8(saved + 2 * index, m.u8(u + 1)?)?;
        }
    }
    Ok(())
}
/// RechercheStore 0x4714f0 copies all four complete 16-byte records.
pub fn store_research(m: &mut MemoriaJogo) -> Result<()> {
    copy(m, 0x508134, 0x5080f4, 64)
}
/// PlayerStore 0x46dee5 stores alliances before the four sympathy words per player.
pub fn store_players(m: &mut MemoriaJogo) -> Result<()> {
    let current = m.u32(0x5080a8)?;
    let saved = m.u32(0x5080ac)?;
    for p in 0..4 {
        m.set_u8(saved + 20 * p + 6, m.u8(current + 20 * p + 6)?)?;
    }
    for p in 0..4 {
        for i in 0..4 {
            m.set_u16(
                saved + 20 * p + 12 + 2 * i,
                m.u16(current + 20 * p + 12 + 2 * i)?,
            )?;
        }
    }
    Ok(())
}
/// IaRechercheAjuste 0x47fa88. Its catch-up body is reached from stage 3.
/// Each completion uses the current branch and the original threshold byte.
pub fn adjust_ai_research(m: &mut MemoriaJogo) -> Result<()> {
    let count = u32::from(m.u8(0x50871c)?);
    for p in 1..4 {
        if m.u8(m.u32(0x5080a8)? + 20 * p + 1)? != 2 || m.u8(0x496fe4)? < 3 {
            continue;
        }
        let branch = u32::from(m.u8(0x5080f4 + 16 * p + 4)?);
        let threshold = m.u8(0x498464 + branch)?;
        for _ in 0..count {
            if m.u8(0x5080f4 + 16 * p + branch)? < threshold {
                crate::mission_research::complete(m, p)?;
            }
        }
    }
    Ok(())
}
/// CalculGame 0x42ca6e..0x42ca8c: only successful MissionNext performs these stores.
/// The host can then enter its briefing/research screen and prepare the next load frame.
pub fn advance_after_victory(
    m: &mut MemoriaJogo,
    exists: impl FnMut(&[u8]) -> Result<bool>,
) -> Result<bool> {
    if !next(m, exists)? {
        return Ok(false);
    }
    adjust_ai_research(m)?;
    store_experience(m)?;
    store_research(m)?;
    store_players(m)?;
    Ok(true)
}
