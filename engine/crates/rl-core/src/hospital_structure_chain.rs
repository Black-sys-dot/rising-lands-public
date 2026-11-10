//! Intact HOSPI19 / BatimentSoignePerso424c2e / HospiCalculCoutReparation41b327.
//! Effects APIs: original-address memory, low16 indices, no scratch GPR claims.
use crate::{frame::stop, player::unit, FrameRegisters, MemoriaJogo, Result};

fn record(index: u16) -> u32 {
    0x49a0c8 + 276 * u32::from(index)
}

/// Original type19 branch at42803c. Occupancy scans capacity, not Bag count.
pub fn next(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    let r = record(index);
    if m.u8(r + 8)? != 19 {
        return stop("Hospital structure type", 0x42803c);
    }
    let inhabited = crate::chain_bags::body(
        m,
        "BagContient",
        FrameRegisters {
            eax: r + 12,
            edx: 1,
            ebx: 9,
            ..Default::default()
        },
    )?
    .eax as u16;
    if inhabited != 0 {
        triage(m, index)?;
    }
    Ok(())
}

/// HospiCalculCoutReparation41b327, AX hospital, DX patient, void effects.
/// Heal first, then charge one food if the truncated native cost increases.
pub fn heal_cost(m: &mut MemoriaJogo, index: u16, patient: u16) -> Result<()> {
    let r = record(index);
    let u = unit(m, u32::from(patient))?;
    let kind = u32::from(m.u8(u + 1)?);
    let half_food = crate::ai_kernels::unit_costs(m, kind)?[0] >> 1;
    let before = ((u32::from(half_food) * u32::from(m.u16(u + 0x9a)?)) / 100) as u16;
    crate::chain_recruit::entry(
        m,
        "EnginAddVie",
        FrameRegisters {
            eax: u32::from(patient),
            edx: 1,
            ..Default::default()
        },
    )?;
    let after = ((u32::from(half_food) * u32::from(m.u16(u + 0x9a)?)) / 100) as u16;
    if before >= after {
        return Ok(());
    }
    let owner = m.u8(r + 4)?;
    if !crate::player_orders::resources(m, u32::from(owner), 1, 0, 0)? {
        return m.set_u8(0x4dfb58 + u32::from(owner), 1);
    }
    // EnleveMatiere46792c itself only debits; the caller updates the stock
    // indicator afterwards. chain_restore::debit includes a different caller's
    // indicator update and cannot be substituted here.
    let food = 0x4e2eb0 + 32 * u32::from(owner);
    m.set_u16(food + 2, m.u16(food + 2)?.saturating_sub(1))?;
    m.set_u16(food, m.u16(food)?.saturating_sub(1))?;
    if m.u16(0x4dfb2c + 6 * u32::from(owner))? != 0 {
        // Same native 16-bit stock differences, sum, DIV and final IDIV.
        // Keep the actual fault sites visible after the resource stores.
        if m.u16(0x4dfb2e + 6 * u32::from(owner))? == 0 {
            return stop("Hospi stock indicator division", 0x41b46b);
        }
        crate::chain_habitation::level(m, u32::from(owner), 0x41b44e)?;
    }
    Ok(())
}

/// BatimentSoignePerso424c2e. Snapshot count and increment the slot even after
/// compaction: the shifted patient is intentionally skipped until a later tick.
pub fn triage(m: &mut MemoriaJogo, index: u16) -> Result<()> {
    let r = record(index);
    let bag = r + 12;
    let count = m.u16(bag)?;
    if !m.u32(0x508f7c)?.is_multiple_of(75) {
        return Ok(());
    }
    for slot in 0..count {
        if m.u8(bag + 0x42 + u32::from(slot))? != 1 {
            continue;
        }
        let patient = m.u16(bag + 6 + 2 * u32::from(slot))?;
        let u = unit(m, u32::from(patient))?;
        let kind = m.u8(u + 1)?;
        // TypeEnginEstTroupe442347, including its >=26 transport branch.
        if matches!(kind, 3 | 11..=17 | 25) {
            continue;
        }
        let max = m.u16(0x4965b6 + 50 * u32::from(kind))?;
        if m.u16(u + 0x9a)? >= max {
            continue;
        }
        heal_cost(m, index, patient)?;
        if m.u16(u + 0x9a)? != max && m.u8(0x4dfb58 + u32::from(m.u8(r + 4)?))? == 0 {
            continue;
        }
        let removed = crate::chain_cargo::remove_index(m, bag, slot)?;
        let exit = crate::chain_vision::exit_cell(m, u32::from(index))?;
        m.set_u16(r + 0x7c, exit)?;
        let cell = crate::chain_vision::position(
            m,
            FrameRegisters {
                eax: u32::from(index),
                ..Default::default()
            },
        )? as u16;
        crate::chain_recruit::set_sort(m, u32::from(removed), u32::from(cell))?;
        crate::frame_spawn::call(
            m,
            "SetDeplaceEngin",
            &[u32::from(removed), u32::from(m.u16(r + 0x7c)?)],
        )?;
        m.set_u8(0x4dfb58 + u32::from(m.u8(r + 4)?), 0)?;
    }
    Ok(())
}
