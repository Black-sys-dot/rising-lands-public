//! Retained BIBLIOTHEQUE (type 6) caller tail of BatimentNextEtape 0x4268d4
//! (type-table target 0x428095); scheduling/visibility remain caller-owned.
//! Turrets (types 10 and 11) dispatch to `turret_structure_chain` instead.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

fn inhabited(m: &mut MemoriaJogo, record: u32) -> Result<bool> {
    // Both supported native cases take BagContient(tag=1, type=9), not
    // Bag.count or BagContientTroupe. Scan capacity, including holes/duplicates.
    Ok(crate::chain_bags::body(
        m,
        "BagContient",
        FrameRegisters {
            eax: record + 12,
            edx: 1,
            ebx: 9,
            ..Default::default()
        },
    )?
    .eax as u16
        != 0)
}

/// Execute the actual switch body for type 6. AX selects the record;
/// the existing frame wrapper remains a void effects API (no scratch EAX claim).
pub fn next(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    match m.u8(r + 8)? {
        6 => library(m, r),
        _ => stop("Building loop tails type", 0x426910),
    }
}

fn library(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    if !inhabited(m, r)? {
        return Ok(());
    }
    // CalculTroupe dispatches type 6 to 0x425e99. Inactive returns without
    // writes; frame_economy::production uses unit recruitment tables, so it is
    // not a substitute for this spell-producing native branch.
    if m.u8(r + 0x74)? != 0 {
        learn(m, r)?;
    }
    for (offset, reset) in [(0x7c, 500), (0x80, 250), (0x82, 360), (0x7e, 160)] {
        let value = m.u16(r + offset)?;
        m.set_u16(
            r + offset,
            if value == reset {
                0
            } else {
                value.wrapping_add(1)
            },
        )?;
    }
    Ok(())
}

/// CalculTroupe 0x425e99..0x4260a5, active type 6: the learning progress
/// (scientists / capacity * 0x49443a, FISTP after __CHP) until the spell's
/// time * 100, then the spell is learned (0x474974), the owner's type 4 icon
/// Bag rebuilt (0x474a38), BatimentSetTechnologie and the structure reset.
fn learn(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    let time_of = |m: &MemoriaJogo| -> Result<u32> {
        Ok(100 * u32::from(m.u16(0x497174 + 46 * u32::from(m.u8(r + 0x7b)?))?))
    };
    if m.u32(r + 0x8c)? == 0 {
        return Ok(());
    }
    if m.u32(r + 0x8c)? < time_of(m)? {
        let qty = crate::chain_bags::body(
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
        let capacity = m.u8(0x495d9e + 52 * 6)?;
        if capacity == 0 {
            return stop("CalculTroupe learning capacity division", 0x425f81);
        }
        let coefficient = f64::from_le_bytes(m.read(0x49443a, 8)?.try_into().expect("f64 width"));
        let value = (f64::from(qty) / f64::from(capacity)) * coefficient;
        let converted = if value.is_finite()
            && value.trunc() >= f64::from(i32::MIN)
            && value.trunc() <= f64::from(i32::MAX)
        {
            value.trunc() as i32 as u32
        } else {
            0x80000000
        };
        m.set_u32(
            r + 0x8c,
            m.u32(r + 0x8c)?.wrapping_add(100 + (converted & 65535)),
        )?;
    }
    if m.u32(r + 0x8c)? < time_of(m)? {
        return Ok(());
    }
    let owner = u32::from(m.u8(r + 4)?);
    let spell = u32::from(m.u8(r + 0x7b)?);
    // 0x474974: the spell is learned.
    m.set_u8(0x508f30 + 5 * owner + spell, 1)?;
    // 0x474a38: the owner's per-type Bag 4 lists every learned spell, tag 4.
    crate::chain_habitation::spell_bag(m, owner)?;
    let t = u32::from(m.u8(r + 8)?);
    crate::chain_habitation::technology(m, owner, t)?;
    m.set_u16(r + 0x78, 0xffff)?;
    m.set_u8(r + 0x7b, 0xff)?;
    m.set_u16(r + 0x7c, 0)?;
    m.set_u16(r + 0x7e, 0)?;
    m.set_u16(r + 0x86, 0xffff)?;
    if t != 0x14 {
        m.set_u16(r + 0x80, 0)?;
        m.set_u16(r + 0x82, 0)?;
    }
    m.set_u8(r + 0x74, 0)?;
    m.set_u32(r + 0x8c, 0)?;
    if m.u8(r + 5)? != 3 {
        m.set_u8(r + 5, 5)?;
    }
    Ok(())
}
