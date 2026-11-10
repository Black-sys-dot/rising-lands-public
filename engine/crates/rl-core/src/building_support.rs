//! Intact GARAGE/ATELIER (0x42698d) and CRISTAL (0x427ed9) support.
//! Scheduling, ownership and publication remain in the original enclosing caller.
use crate::{frame::stop, FrameRegisters, MemoriaJogo, Result};

/// BatimentGetCapacite 0x41f3ed as a byte, for the repair's float rate.
fn repair_capacity(m: &MemoriaJogo, r: u32) -> Result<u8> {
    let t = m.u8(r + 8)?;
    let table = m.u8(0x495d9e + 52 * u32::from(t))?;
    Ok(match t {
        14 => table.wrapping_sub(1),
        0x13 => table.wrapping_sub(m.u8(0x495be8)?),
        _ => table,
    })
}

/// One repair tick (0x421ac5..0x421bfa): progress, unit health, and the cost
/// of GarageCalculCoutReparation 0x41b1c6.
fn repair_tick(m: &mut MemoriaJogo, structure: u16, r: u32, unit: u16) -> Result<()> {
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
    let capacity = repair_capacity(m, r)?;
    let coefficient = if m.u8(r + 8)? == 14 {
        0x4943d7
    } else {
        0x4943df
    };
    let coefficient = f64::from_le_bytes(m.read(coefficient, 8)?.try_into().expect("f64 width"));
    let value = (f64::from(qty) / f64::from(capacity)) * coefficient;
    // __CHP truncates, then FISTP stores integer-indefinite when out of range.
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
        m.u32(r + 0x8c)?.wrapping_add((converted & 65535) + 300),
    )?;
    let u = crate::player::unit(m, u32::from(unit))?;
    let health = crate::chain_recruit::status(m, u32::from(structure), u32::from(unit) & 255)?;
    m.set_u16(u + 0x9a, health as u16)?;
    // Only the kind-11 and kind-16 bodies reach 0x421bf6, the cost call.
    if matches!(m.u8(r + 0x7b)?, 11 | 16) {
        repair_cost(m, r, unit)?;
    }
    Ok(())
}

/// GarageCalculCoutReparation 0x41b1c6: one food each time the half-food cost
/// of the restored health passes the amount already paid at 0x4dfb5c + 2p.
fn repair_cost(m: &mut MemoriaJogo, r: u32, unit: u16) -> Result<()> {
    let u = crate::player::unit(m, u32::from(unit))?;
    let half = m.u16(0x4965b0 + 50 * u32::from(m.u8(u + 1)?))? >> 1;
    let cost = (u32::from(half).wrapping_mul(m.u32(r + 0x8c)?) / 10000) as u16;
    let owner = u32::from(m.u8(r + 4)?);
    if cost <= m.u16(0x4dfb5c + 2 * owner)? {
        return Ok(());
    }
    if !crate::player_orders::resources(m, owner, 1, 0, 0)? {
        return m.set_u8(0x4dfb54 + owner, 1);
    }
    if half == 0 {
        return Ok(());
    }
    crate::chain_restore::debit(m, 1, 1, owner as u8)?;
    let paid = (u32::from(half).wrapping_mul(m.u32(r + 0x8c)?) / 10000) as u16;
    m.set_u16(0x4dfb5c + 2 * owner, paid)
}

fn door_open(m: &MemoriaJogo, r: u32) -> Result<bool> {
    Ok(m.u8(r + 0x76)? == 7 && m.u8(r + 0x75)? != 0)
}

fn door_shut(m: &MemoriaJogo, r: u32) -> Result<bool> {
    Ok(m.u8(r + 0x76)? == 0 && m.u8(r + 0x75)? == 0)
}

fn recharge_bomb(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    // EnginRechargeBombe 0x4483aa: wrapping increment, then a signed cap at 5.
    let count = m.u8(u + 0xda)?.wrapping_add(1);
    m.set_u8(u + 0xda, if count >= 5 { 5 } else { count })
}

/// CalculReparation 0x42195c, a vehicle being mended. Kinds 11 (0x421d3d), 13
/// (0x422147) and 12, 14, 15 (0x4225c1) share a door, repair, exit machine
/// over +0x7e; the exit thresholds differ, and 12/14/15 reload bombs.
/// Kind 16 (0x4225ee) has no door: repair, then reload, then the reset.
pub fn calcul_reparation(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    let kind = m.u8(r + 0x7b)?;
    if !(11..=16).contains(&kind) {
        return Ok(());
    }
    let unit = m.u16(r + 0x88)?;
    let u = crate::player::unit(m, u32::from(unit))?;
    let owner = u32::from(m.u8(r + 4)?);
    let frame = m.u32(0x508f7c)?;
    let healed = {
        let k = m.u8(u + 1)?;
        100 * u32::from(m.u16(0x4965b6 + 50 * u32::from(k))?) <= m.u32(r + 0x8c)?
    };
    if kind == 16 {
        if !healed {
            if frame % 75 == 0 {
                repair_tick(m, structure, r, unit)?;
            }
            return Ok(());
        }
        if m.u8(u + 0xda)? == 5 {
            return reset_after_repair(m, r);
        }
        if crate::player_orders::resources(m, owner, 0, 2, 0)? && frame % 25 == 0 {
            recharge_bomb(m, u)?;
            crate::chain_restore::debit(m, 2, 2, owner as u8)?;
        }
        return Ok(());
    }
    match m.u16(r + 0x7e)? {
        0 => {
            if !door_open(m, r)? {
                return Ok(());
            }
            let offset = m.u16(r + 0x82)?;
            if offset as i16 > -0x50 {
                m.set_u16(r + 0x82, offset.wrapping_sub(4))?;
            } else {
                m.set_u8(r + 0x75, 0)?;
                m.set_u16(r + 0x7e, m.u16(r + 0x7e)?.wrapping_add(1))?;
            }
        }
        1 => {
            if !door_shut(m, r)? {
                return Ok(());
            }
            let unpaid = m.u8(0x4dfb54 + owner)? == 0;
            if unpaid && !healed {
                if frame % 75 == 0 {
                    repair_tick(m, structure, r, unit)?;
                }
                return Ok(());
            }
            if matches!(kind, 12 | 14 | 15) && unpaid && m.u8(u + 1)? == 14 && m.u8(u + 0xda)? != 5
            {
                if !crate::player_orders::resources(m, owner, 0, 2, 0)? {
                    return m.set_u8(0x4dfb54 + owner, 1);
                }
                if frame % 75 == 0 {
                    recharge_bomb(m, u)?;
                    crate::chain_restore::debit(m, 2, 2, owner as u8)?;
                }
                return Ok(());
            }
            if unpaid {
                let k = u32::from(m.u8(u + 1)?);
                m.set_u16(u + 0x9a, m.u16(0x4965b6 + 50 * k)?)?;
            }
            m.set_u8(0x4dfb54 + owner, 0)?;
            m.set_u8(u + 0x90, 0xd)?;
            m.set_u8(u + 0x91, 0xd)?;
            m.set_u8(r + 0x75, 1)?;
            m.set_u16(r + 0x7e, m.u16(r + 0x7e)?.wrapping_add(1))?;
            match kind {
                11 => m.set_u16(r + 0x80, m.u16(r + 0x80)?.wrapping_add(2))?,
                13 => m.set_u16(r + 0x80, m.u16(r + 0x80)?.wrapping_sub(12))?,
                _ => {}
            }
        }
        2 => {
            let cell = u32::from(m.u16(r + 2)?).wrapping_add(0x402) & 65535;
            if !crate::chain_search::passable(m, u32::from(unit), cell)? || !door_open(m, r)? {
                return crate::building_loop_completion::free_cell(m, cell as u16, unit);
            }
            let threshold: i16 = match kind {
                11 => -10,
                13 => -12,
                _ => 0,
            };
            let offset = m.u16(r + 0x82)?;
            if (offset as i16) < threshold {
                m.set_u16(r + 0x82, offset.wrapping_add(4))?;
                return Ok(());
            }
            m.set_u8(r + 0x75, 0)?;
            crate::chain_recruit::set_sort(m, u32::from(unit), cell)?;
            reset_after_repair(m, r)?;
        }
        _ => {}
    }
    Ok(())
}

fn reset_after_repair(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    m.set_u16(r + 0x78, 0xffff)?;
    m.set_u8(r + 0x7b, 0xff)?;
    m.set_u16(r + 0x7c, 0)?;
    m.set_u16(r + 0x7e, 0)?;
    m.set_u16(r + 0x86, 0xffff)?;
    if m.u8(r + 8)? != 0x14 {
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

/// Type18 BatimentNextEtape, original 0x427eec..0x42803c.
/// Empty buildings return before owner-dependent tables. Inhabited buildings
/// update the native one-slot type16 production request after activity work.
pub fn next_type18(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    if m.u8(r + 8)? != 18 {
        return stop("Type18 building dispatch", 0x427eec);
    }
    let occupied = crate::chain_bags::body(
        m,
        "BagContient",
        FrameRegisters {
            eax: r + 12,
            edx: 1,
            ebx: 9,
            ..Default::default()
        },
    )?
    .eax as u16
        != 0;
    if !occupied {
        return Ok(());
    }
    if m.u8(r + 0x74)? != 0 {
        if m.u8(r + 0x7a)? != 0 {
            calcul_reparation(m, structure)?;
        } else if m.u32(r + 0x8c)? != 0 {
            // CalculTroupe 0x4259e6, type 18 (0x4260aa): a positive progress
            // counts up like any producer; complete_type18 is its completion.
            crate::frame_economy::production(m, u32::from(structure), false, true)?;
        }
    }
    let owner = u32::from(m.u8(r + 4)?);
    let available = u16::from(m.u8(0x4dfb4c + owner)?) > m.u16(0x4e1a3c + 52 * owner + 32)?;
    let bag = 0x4dd6e8 + 2288 * owner + 0x750;
    if available
        && crate::chain_bags::body(
            m,
            "BagContient",
            FrameRegisters {
                eax: bag,
                edx: 6,
                ebx: 16,
                ..Default::default()
            },
        )?
        .eax as u16
            != 0
    {
        return Ok(());
    }
    // BagInit447148(capacity1) leaves the six bytes at+0x60..+0x65 untouched.
    // Both native branches reset the bag before their Prend/Retire call.
    for offset in [0, 0x66, 4, 2] {
        m.set_u16(bag + offset, 0)?;
    }
    for i in 0..30 {
        m.set_u16(bag + 6 + 2 * i, 0)?;
        m.set_u8(bag + 0x42 + i, 0)?;
    }
    m.set_u16(bag + 2, 1)?;
    crate::chain_bags::body(
        m,
        if available { "BagPrend" } else { "BagRetire" },
        FrameRegisters {
            eax: bag,
            edx: 6,
            ebx: 16,
            ..Default::default()
        },
    )?;
    Ok(())
}

/// CalculTroupe 0x4261cc..0x426313: the ordered unit (kind 16, balloon) is
/// created on row 0 at a random column, placed and sent to enter the
/// structure; the order fields reset; the one-slot type-16 recruit Bag closes
/// when the player has no spare heliport capacity left.
pub fn complete_type18(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    let width = u32::from(m.u16(0x497168)?);
    if width == 0 {
        return stop("CalculTroupe type18 map width division", 0x4261e1);
    }
    let column = crate::frame_core::random(m)? % width;
    let owner = u32::from(m.u8(r + 4)?);
    let unit = crate::frame_spawn::call(m, "CreerEngin", &[column, 1, 0x10, owner])? & 65535;
    crate::frame_spawn::call(m, "PlaceEngin", &[unit])?;
    crate::chain_entry::set_enter(m, unit as u16, structure)?;
    m.set_u16(r + 0x78, 0xffff)?;
    m.set_u8(r + 0x7b, 0xff)?;
    m.set_u16(r + 0x7c, 0)?;
    m.set_u16(r + 0x7e, 0)?;
    m.set_u16(r + 0x86, 0xffff)?;
    m.set_u16(r + 0x80, 0)?;
    m.set_u16(r + 0x82, 0)?;
    m.set_u8(r + 0x74, 0)?;
    m.set_u32(r + 0x8c, 0)?;
    if m.u8(r + 5)? != 3 {
        m.set_u8(r + 5, 5)?;
    }
    let limit = i32::from(m.u8(0x4dfb4c + owner)?);
    let count = i32::from(m.u16(0x4e1a3c + 52 * owner + 32)?);
    if limit > count + 1 {
        return Ok(());
    }
    let bag = 0x4dd6e8 + 2288 * owner + 0x750;
    for offset in [0, 0x66, 4, 2] {
        m.set_u16(bag + offset, 0)?;
    }
    for i in 0..30 {
        m.set_u16(bag + 6 + 2 * i, 0)?;
        m.set_u8(bag + 0x42 + i, 0)?;
    }
    m.set_u16(bag + 2, 1)?;
    crate::chain_bags::body(
        m,
        "BagRetire",
        FrameRegisters {
            eax: bag,
            edx: 6,
            ebx: 16,
            ..Default::default()
        },
    )?;
    Ok(())
}

/// Original-address void effects API; AX is the structure index.
pub fn next(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    match m.u8(r + 8)? {
        5 | 14 => {
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
            .eax as u16
                != 0;
            if inhabited {
                if m.u8(r + 8)? == 14 {
                    // CalculTroupe's GARAGE branch tests activity alone, even
                    // when progress is zero. Repair precedes door animation.
                    if m.u8(r + 0x74)? != 0 {
                        calcul_reparation(m, structure)?;
                    }
                } else {
                    // ATELIER shares this switch target, but has its own
                    // vehicle-production body. Preserve its actual boundary.
                    return stop("CalculTroupe ATELIER", 0x4259e6);
                }
            }
            animate_door(m, structure)
        }
        17 => {
            let target = crystal_priests(m, structure)?;
            m.set_u16(r + 0x86, target)
        }
        _ => stop("Building support type", 0x426910),
    }
}

/// AnimPorteBatiment 0x420d3a for any structure type. Types 5, 8 and 14
/// (0x420d84) move the door only on even frames (bit 0 of 0x508f80); every
/// other type takes the generic branch (0x420d69) on every frame. Both open
/// (+0x76 up to 7, signed compare) while +0x75 is set and close (+0x76 down
/// to 0) while it is clear. The native callers (0x4269fe, 0x426ac0) only pass
/// types 5, 8 and 14.
pub fn anim_porte(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    if matches!(m.u8(r + 8)?, 5 | 8 | 14) && m.u8(0x508f80)? & 1 != 0 {
        return Ok(());
    }
    let door = m.u8(r + 0x76)?;
    if m.u8(r + 0x75)? != 0 {
        if i32::from(door) < 7 {
            m.set_u8(r + 0x76, door.wrapping_add(1))?;
        }
    } else if door != 0 {
        m.set_u8(r + 0x76, door.wrapping_sub(1))?;
    }
    Ok(())
}

/// The door step of the type-5/14 support branch.
pub fn animate_door(m: &mut MemoriaJogo, structure: u16) -> Result<()> {
    anim_porte(m, structure)
}

/// CristalCherchePretre 0x4249de scans every slot in ascending index order.
/// Native return AX is always ffff; it recharges nearby live type4 units.
pub fn crystal_priests(m: &mut MemoriaJogo, structure: u16) -> Result<u16> {
    let r = 0x49a0c8 + 276 * u32::from(structure);
    let cell = m.u16(r + 2)?;
    for index in 0..800u16 {
        let u = crate::frame_core::unit(m, u32::from(index))?;
        if matches!(m.u8(u + 2)?, 0 | 4) || m.u8(u + 1)? != 4 {
            continue;
        }
        let other = m.u16(u + 8)?;
        let dx = i32::from(cell & 255) - i32::from(other & 255);
        let dy = i32::from(cell >> 8) - i32::from(other >> 8);
        if dx * dx + dy * dy > 16 {
            continue;
        }
        let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(u)?), 4, 9, 0)? as u16;
        // Native bonus-zero branch chooses60; selector9 is a byte (<=255).
        let period = 600u16.checked_div(bonus).unwrap_or(60);
        if m.u32(0x508f7c)? % u32::from(period) == 0 {
            add_mana(m, index, 1)?;
        }
    }
    Ok(u16::MAX)
}

/// EnginAjouteMana 0x4482d3: byte addition occurs before both bonus queries.
pub fn add_mana(m: &mut MemoriaJogo, unit: u16, amount: u8) -> Result<()> {
    let u = crate::frame_core::unit(m, u32::from(unit))?;
    m.set_u8(u + 0xda, m.u8(u + 0xda)?.wrapping_add(amount))?;
    let mana = u32::from(m.u8(u + 0xda)?);
    let bonus =
        crate::frame_economy::bonus(m, u32::from(m.u8(u)?), u32::from(m.u8(u + 1)?), 12, 0)?;
    if mana >= (bonus & 65535) + 10 {
        let bonus =
            crate::frame_economy::bonus(m, u32::from(m.u8(u)?), u32::from(m.u8(u + 1)?), 12, 0)?;
        m.set_u8(u + 0xda, (bonus as u8).wrapping_add(10))?;
    }
    Ok(())
}
