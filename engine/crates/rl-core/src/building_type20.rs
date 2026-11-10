//! BatimentNextEtape type 20 body (table entry 0x427da4..0x427ed9) and its
//! helper 0x426418: habitation predicate, resource-paid progression steps
//! (TestRessource, EnleveRessource with the cave stock indicator), CalculTroupe
//! and the request timer that finally fills the owner's type-20 Bag.
use crate::{frame::stop, player::structure, MemoriaJogo, Result};

fn tech_bag(owner: u32) -> u32 {
    0x4dd6e8 + 0x8f0 * owner + 104 * 20
}

/// 0x426418: state +0x104 drives the paid progression; +0x108 counts steps.
fn progression(m: &mut MemoriaJogo, s: u32) -> Result<()> {
    let r = structure(s);
    match m.u8(r + 0x104)? {
        1 => {
            if u32::from(m.u16(0x495bf0)?) <= m.u32(r + 0x108)? {
                return m.set_u8(r + 0x104, 2);
            }
            let modulus = u32::from(m.u16(0x495bf2)?);
            if modulus == 0 {
                return stop("Nova progression tick division", 0x42647d);
            }
            if m.u32(0x508f7c)? % modulus != 0 {
                return Ok(());
            }
            let owner = u32::from(m.u8(r + 4)?);
            let food = u32::from(m.u16(0x495bf4)?);
            let parts = u32::from(m.u16(0x495bf6)?);
            if crate::frame_economy::call(m, "TestRessource", &[owner, food, parts, 0])? == 0 {
                return Ok(());
            }
            for (resource, cost) in [(1u32, food), (2u32, parts)] {
                crate::player_build::debit(m, owner, resource, cost)?;
                if m.u16(0x4dfb2c + 6 * owner)? as i16 > 0 {
                    crate::chain_habitation::level(m, owner, 0x426526)?;
                }
            }
            m.set_u32(r + 0x108, m.u32(r + 0x108)?.wrapping_add(1))?;
            m.set_u8(r + 0x104, 1)
        }
        2 => {
            if m.u8(r + 0x7a)? != 0 {
                return Ok(());
            }
            m.set_u8(r + 0x7a, 1)?;
            m.set_u16(r + 0x7e, 0)?;
            m.set_u8(r + 0x104, 3)?;
            m.set_u8(r + 0x76, 0)
        }
        _ => Ok(()),
    }
}

/// 0x427da4: inhabited structures start their progression once, run it with
/// CalculTroupe, then count a request timer; when the request limit for the
/// type is reached the owner's type-20 Bag is re-initialised (cap 2, unit 4,
/// tag 4 value 4) and +0x75 marks the structure.
pub fn next(m: &mut MemoriaJogo, s: u16) -> Result<()> {
    let s = u32::from(s);
    if !crate::frame_economy::inhabited(m, s)? {
        return Ok(());
    }
    let r = structure(s);
    if m.u8(r + 0x75)? != 0 && m.u8(r + 0x104)? == 0 {
        m.set_u32(r + 0x108, 1)?;
        m.set_u8(r + 0x104, 1)?;
    }
    progression(m, s)?;
    crate::frame_economy::call(m, "CalculTroupe", &[s])?;
    if (m.u16(r + 0x80)? as i16) > 0 {
        return m.set_u16(r + 0x80, m.u16(r + 0x80)?.wrapping_sub(1));
    }
    let kind = u32::from(m.u8(r + 8)?);
    let limit = i32::from(m.u16(0x495d90 + 52 * kind)?);
    if i32::from(m.u16(r + 0x82)? as i16) < limit {
        m.set_u16(r + 0x80, 0x14)?;
        return m.set_u16(r + 0x82, m.u16(r + 0x82)?.wrapping_add(0x14));
    }
    let owner = u32::from(m.u8(r + 4)?);
    let bag = tech_bag(owner);
    crate::chain_habitation::bag_init(m, bag, 2)?;
    crate::chain_habitation::bag_take(m, bag, 6, 4)?;
    crate::chain_habitation::bag_take(m, bag, 4, 4)?;
    m.set_u8(r + 0x75, 1)
}
