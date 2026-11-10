//! Intact InfligeDegat0x450874 and EnginInfligeDegat0x4502f0, modes 1 and 3.
//! Unsupported cargo/flying-death and AI consumers retain addressed stops.
use crate::{frame::stop, player::unit, FrameRegisters, MemoriaJogo, Result};
fn player_type(m: &mut MemoriaJogo, p: u8) -> Result<u8> {
    Ok(crate::frame_economy::call(m, "PlayerGetType", &[u32::from(p)])? as u8)
}
fn move_to(m: &mut MemoriaJogo, u: u16, c: u16) -> Result<()> {
    crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(u), u32::from(c)])?;
    Ok(())
}
fn react(m: &mut MemoriaJogo, victim: u16, attacker: u16, mode: u8) -> Result<()> {
    let t = unit(m, u32::from(victim))?;
    let s = unit(m, u32::from(attacker))?;
    let owner = m.u8(t)?;
    if owner == 200 {
        return Ok(());
    }
    let kind = m.u8(t + 1)?;
    if player_type(m, owner)? == 1 && matches!(m.u8(t + 2)?, 5 | 18) {
        let building = crate::chain_s1::entry(
            m,
            "ChercheBatimentProximiteCaveOuSanctuaire",
            FrameRegisters {
                eax: u32::from(m.u16(t + 8)?),
                edx: u32::from(owner),
                ..Default::default()
            },
        )?
        .eax as u16;
        if building != 65535 {
            let cell = crate::frame_core::building_cell(m, u32::from(building))? as u16;
            move_to(m, victim, cell)?;
        }
        return Ok(());
    }
    let source_kind = m.u8(s + 1)?;
    let eligible = matches!(kind,0..=3|8|11|12|15|16|18..=25)
        && m.u8(t + 0xdc)? & 2 == 0
        && mode == 1
        && !matches!(source_kind, 10 | 17)
        && (source_kind != 16 || matches!(kind, 2 | 8))
        && m.u8(s)? != owner;
    if eligible {
        let pt = player_type(m, owner)?;
        let attack = pt == 1
            && (m.u8(t + 2)? == 7 || m.u32(0x508f7c)?.wrapping_sub(m.u32(t + 0x110)?) > 250)
            || pt == 2 && m.u8(t + 2)? == 7;
        if attack {
            crate::player_orders::call(
                m,
                "SetAttaqueEngin",
                [u32::from(victim), 1, u32::from(attacker), 0],
                0x450cab,
            )?;
        }
    } else {
        // 0x450ce4: flee away from the shooter: a structure's BatimentGetCase
        // for mode 3, the attacking unit's cell for mode 1.
        let cell = m.u16(t + 0x7c)?;
        let source = match mode {
            3 => crate::frame_core::building_cell(m, u32::from(attacker))? as u16,
            // Mode 8 never fills the local read at 0x450d54. Traced on the
            // native AvanceEngin path it holds 1, left by the earlier callees.
            8 => 1,
            _ => m.u16(s + 0x7c)?,
        };
        let direction = crate::frame_path::direction(source, cell, false);
        let next = crate::frame_path::neighbor(m, cell, direction)?;
        if next != 65535 {
            move_to(m, victim, next)?;
        }
    }
    Ok(())
}
/// ChercheTypeEnginProximite0x4500e4 actually tests grid occupants' owner;
/// it does not read the nominal type argument. Preserve that native behavior.
fn nearby(m: &MemoriaJogo, owner: u8, cell: u16) -> Result<bool> {
    let (x, y) = (i32::from(cell & 255), i32::from(cell >> 8));
    let grid = m.u32(0x4e0f38)?;
    for yy in (y - 10).max(0)..=(y + 10).min(255) {
        for xx in (x - 10).max(0)..=(x + 10).min(255) {
            let c = (xx + 256 * yy) as u16;
            let raw = m.u16(grid + 2 * u32::from(c))?;
            if raw & 0xc000 != 0x4000 {
                continue;
            }
            let u = (raw & 0x3ff8) >> 3;
            if m.u8(unit(m, u32::from(u))?)? == owner
                && (xx - x) * (xx - x) + (yy - y) * (yy - y) <= 100
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
pub fn consume(m: &mut MemoriaJogo, victim: u16, attacker: u16) -> Result<()> {
    consume_mode(m, victim, attacker, 1)
}
/// EnginInfligeDegat 0x4502f0 for mode 1 (unit attacker) and mode 3 (structure
/// attacker, 0x450377). The shared tail at 0x45059c keeps reading the unit
/// record whose index is the attacker, even for a structure attacker.
fn consume_mode(m: &mut MemoriaJogo, victim: u16, attacker: u16, mode: u8) -> Result<()> {
    let t = unit(m, u32::from(victim))?;
    let s = unit(m, u32::from(attacker))?;
    let impact_cell = m.u16(t + 0x7c)?;
    let (raw, source_owner) = if mode == 8 {
        // 0x450475: the trap row of the damage table. The attacker-owner local
        // is never set in this mode; traced natively it holds the victim index.
        let row = m.u32(m.u32(0x4e199c)?.wrapping_add(0x68))?;
        (
            m.u8(row.wrapping_add(u32::from(m.u8(t + 1)?)))?,
            victim as u32 as u8,
        )
    } else if mode == 3 {
        let structure = crate::player::structure(u32::from(attacker));
        let raw = crate::projectile_building_impact::structure_damage(
            m,
            m.u8(structure + 8)?,
            1,
            m.u8(t + 1)?,
        )?;
        let owner = m.u8(structure + 4)?;
        crate::frame_ai::call(
            m,
            "iagUnitSendMessage",
            &[u32::from(victim), 0x4c, u32::from(attacker)],
        )?;
        (raw, owner)
    } else {
        let source_owner = m.u8(s)?;
        let (kind, source_kind) = (m.u8(t + 1)?, m.u8(s + 1)?);
        if kind >= 26 || source_kind >= 26 {
            return stop("EnginInfligeDegat type", 0x4503b8);
        }
        let row = m.u32(m.u32(0x4e199c)? + 4 * u32::from(source_kind))?;
        let mut raw = m.u8(row + u32::from(kind))?.wrapping_add(m.u8(s + 0xf7)?);
        raw = raw.wrapping_add(crate::frame_economy::bonus(
            m,
            u32::from(m.u8(s)?),
            u32::from(source_kind),
            5,
            0,
        )? as u8);
        if nearby(m, m.u8(s)?, m.u16(s + 0x7c)?)? {
            raw = raw.wrapping_add(m.u8(0x4965ac)?);
        }
        crate::frame_ai::call(
            m,
            "iagUnitSendMessage",
            &[u32::from(victim), 0x48, u32::from(attacker)],
        )?;
        (raw, source_owner)
    };
    let mut damage = raw.wrapping_sub(m.u8(t + 0xf6)?);
    damage = damage.wrapping_sub(crate::frame_economy::bonus(
        m,
        u32::from(m.u8(s)?),
        u32::from(m.u8(s + 1)?),
        6,
        0,
    )? as u8);
    if (damage as i8) < 0 {
        damage = 1;
    }
    if crate::frame_core::call(
        m,
        "TerrainEstDecouvert",
        &[u32::from(m.u16(t + 8)?), u32::from(m.u8(0x508041)?)],
    )? as u8
        != 0
    {
        let volume = crate::player_orders::unit_volume(m, u32::from(victim))?;
        m.set_u8(t + 0xf9, 0)?;
        if m.u32(t + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x45063f,
                true,
                [0x50, (u32::from(m.u8(t + 1)?) << 16) | 0x100, t + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    if m.u8(t + 0xdc)? & 8 != 0 {
        damage = damage.wrapping_sub(1);
    }
    let signed = i32::from(damage as i8);
    let hp = i32::from(m.u16(t + 0x9a)?);
    if hp <= signed {
        return crate::chain_death::lethal(
            m,
            victim,
            attacker,
            source_owner,
            impact_cell,
            mode == 1,
        );
    }
    m.set_u16(t + 0x9a, (hp - signed) as u16)
}
pub fn impact(m: &mut MemoriaJogo, cell: u16, attacker: u16) -> Result<()> {
    impact_flagged(m, cell, attacker, false)
}
/// InfligeDegat normalizes its stack byte to a collision layer: 1 for ground,
/// 2 for airborne. The same flag also skips the building lookup.
pub fn impact_flagged(
    m: &mut MemoriaJogo,
    cell: u16,
    attacker: u16,
    skip_building: bool,
) -> Result<()> {
    impact_mode(m, cell, 1, attacker, skip_building)
}
/// InfligeDegat 0x450874 for mode 1 (unit attacker) and mode 3 (structure
/// attacker: the turret shots of structure types 10/11). Other modes stop.
pub fn impact_mode(
    m: &mut MemoriaJogo,
    cell: u16,
    mode: u8,
    attacker: u16,
    skip_building: bool,
) -> Result<()> {
    if !matches!(mode, 1 | 3 | 8) {
        return stop("InfligeDegat damage mode", 0x450874);
    }
    let grid = m.u32(0x4e0f38)?;
    if !skip_building && m.u16(grid + 2 * u32::from(cell))? & 0x8000 != 0 {
        return crate::projectile_building_impact::impact(m, cell, mode, attacker, 0);
    }
    let mut victim = None;
    for u in 0..800 {
        let r = unit(m, u)?;
        if !matches!(m.u8(r + 2)?, 0 | 4)
            && m.u8(r + 0x72)? != 0
            && crate::frame_units::occupies(m, u as u16, cell, 1 + u8::from(skip_building))? != 0
        {
            victim = Some(u as u16);
            if m.u8(r + 0xab)? != 0 {
                break;
            }
        }
    }
    let Some(victim) = victim else {
        return Ok(());
    };
    let t = unit(m, u32::from(victim))?;
    let kind = m.u8(t + 1)?;
    react(m, victim, attacker, mode)?;
    consume_mode(m, victim, attacker, mode)?;
    if matches!(kind,0..=2|4..=10|18..=24) {
        crate::frame_combat::call(
            m,
            "ProjectileInit",
            &[14, u32::from(cell), u32::from(cell), 0, 0, 0, 0, 0, 0],
        )?;
    } else if (11..=17).contains(&kind) {
        let random = crate::frame_core::call(m, "Random", &[0x22])?;
        crate::frame_combat::call(
            m,
            "ProjectileInit",
            &[
                15 + (random & 1),
                u32::from(cell),
                u32::from(cell),
                0,
                0,
                0,
                0,
                0,
                u32::from(kind == 16),
            ],
        )?;
    }
    if mode == 8 {
        return Ok(());
    }
    if mode == 3 {
        // 0x450e8f: relations with the shooter structure's owner, then
        // PlayerAAttaqueSet; no attack sound for a structure shooter.
        let owner = m.u8(crate::player::structure(u32::from(attacker)) + 4)?;
        let target_owner = m.u8(t)?;
        if crate::frame_economy::call(
            m,
            "PlayerEstAllie",
            &[u32::from(owner), u32::from(target_owner)],
        )? as u8
            != 0
        {
            crate::frame_economy::call(
                m,
                "MAJSympathie",
                &[u32::from(target_owner), u32::from(owner), 10],
            )?;
            crate::frame_economy::call(
                m,
                "MAJSympathie",
                &[u32::from(target_owner), u32::from(owner), 6],
            )?;
            crate::message_box::reset_allie(m, owner, target_owner)?;
            crate::message_box::reset_allie(m, target_owner, owner)?;
        }
        if owner < 4 && target_owner < 4 {
            let a = 0x508044 + 20 * u32::from(owner) + 4 * u32::from(target_owner);
            m.set_u32(a, m.u32(a)?.wrapping_add(1))?;
        }
        return Ok(());
    }
    let s = unit(m, u32::from(attacker))?;
    let (owner, target_owner) = (m.u8(s)?, m.u8(t)?);
    if crate::frame_economy::call(
        m,
        "PlayerEstAllie",
        &[u32::from(owner), u32::from(target_owner)],
    )? as u8
        != 0
    {
        crate::frame_economy::call(
            m,
            "MAJSympathie",
            &[u32::from(target_owner), u32::from(owner), 10],
        )?;
        crate::frame_economy::call(
            m,
            "MAJSympathie",
            &[u32::from(target_owner), u32::from(owner), 6],
        )?;
        crate::message_box::reset_allie(m, owner, target_owner)?;
        crate::message_box::reset_allie(m, target_owner, owner)?;
    }
    if owner < 4 && target_owner < 4 {
        let a = 0x508044 + 20 * u32::from(owner) + 4 * u32::from(target_owner);
        m.set_u32(a, m.u32(a)?.wrapping_add(1))?;
    }
    if owner != target_owner && target_owner == m.u8(0x508041)? {
        let now = m.u32(0x508f7c)?;
        let previous = m.u32(0x4e19c0)?;
        if previous > now || now.wrapping_sub(previous) >= 200 {
            m.set_u32(0x4e19c0, now)?;
            m.set_u16(0x496acc, m.u16(t + 0x7c)?)?;
            crate::player::sound(
                m,
                0x4510b1,
                true,
                [0x50, (u32::from(m.u8(t + 1)?) << 16) | 0x700, 0x496ac8, 0],
                Some(256),
            )?;
        }
    }
    Ok(())
}
pub fn fixture(
    m: &mut MemoriaJogo,
    mut r: FrameRegisters,
    consumer: bool,
) -> Result<FrameRegisters> {
    if r.edx as u8 != 1 {
        return stop("Damage fixture mode", 0x450874);
    }
    if consumer {
        consume(m, r.eax as u16, r.ebx as u16)?;
    } else {
        impact(m, r.eax as u16, r.ebx as u16)?;
    }
    r.esp = r.esp.wrapping_add(if consumer { 4 } else { 8 });
    Ok(r)
}
