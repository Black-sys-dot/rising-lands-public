//! CalculRepareEngin0x44b5f5 and active repair state15 from intact instructions.
use crate::{
    player::{alive, structure, unit, Memory},
    MemoriaJogo, Result,
};
fn target(m: &mut MemoriaJogo, u: u32, r: u32, cell: u32) -> Result<()> {
    m.sw(r + 0x7e, cell)?;
    crate::frame_spawn::release(m, u)?;
    m.sb(r + 0x88, 0)
}
fn sound(m: &mut MemoriaJogo, u: u32, code: u32, site: u32) -> Result<()> {
    let r = unit(m, u)?;
    if m.b(r)? != m.b(0x508041)? {
        return Ok(());
    }
    if crate::frame_core::call(m, "TerrainEstDecouvert", &[m.w(r + 8)?, m.b(0x508041)?])? & 255 == 0
    {
        return Ok(());
    }
    let volume = crate::player_orders::unit_volume(m, u)?;
    m.sb(r + 0xf9, 0)?;
    if m.d(r + 0x104)? == u32::MAX {
        crate::player::sound(
            m,
            site,
            true,
            [0x50, (m.b(r + 1)? << 16) | code, r + 0x104, 0],
            Some(volume),
        )?;
    }
    Ok(())
}
fn finish(m: &mut MemoriaJogo, u: u32, r: u32) -> Result<()> {
    if m.b(r)? == m.b(0x508041)? {
        crate::player::sound(m, 0x44badc, false, [r + 0x104, 0, 0, 0], None)?;
        sound(m, u, 0x1500, 0x44bb45)?;
    }
    m.sb(r + 0xbe, 255)?;
    m.sw(r + 0xb2, 65535)?;
    if alive(m, u)? {
        m.sb(r + 2, 7)?;
        crate::frame_spawn::release(m, u)?;
    }
    target(m, u, r, m.w(r + 0x7c)?)
}
fn building_sound(m: &mut MemoriaJogo, s: u32, site: u32) -> Result<()> {
    let r = structure(s);
    if m.b(r + 4)? == m.b(0x508041)? {
        let volume = crate::player_orders::volume(m, m.w(r + 2)?)?;
        if m.d(r + 0x100)? == u32::MAX {
            crate::player::sound(
                m,
                site,
                true,
                [0x42, (m.b(r + 8)? << 16) | 0x2500, r + 0x100, 0],
                Some(volume),
            )?;
        }
    }
    Ok(())
}
/// Native41cec4 decreases damage;41cf59 increases construction health.
fn repair_building(m: &mut MemoriaJogo, s: u32) -> Result<()> {
    let r = structure(s);
    let damage = m.w(r + 6)?;
    if damage != 0 {
        m.sw(r + 6, damage.saturating_sub(2))?;
        return building_sound(m, s, 0x41cf50);
    }
    m.sw(r + 10, m.w(r + 10)?.wrapping_add(2))?;
    building_sound(m, s, 0x41cfbc)?;
    let kind = m.b(r + 8)?;
    let limit = m.w(0x495d90 + 52 * kind)?;
    if m.w(r + 10)? >= limit {
        m.sb(r + 5, 5)?;
        m.sw(r + 10, limit)?;
        if kind == 17 {
            // 0x41cf59 path: BatimentHabitable (type 17: technology, +0x84 = 200).
            crate::chain_habitation::habitable(m, s)?;
        }
        if kind == 9 {
            // 0x41d012: SetMurAllentour(+2), then BatimentModifyMap.
            crate::player_build::wall_surround(m, m.w(r + 2)?)?;
        }
        crate::chain_vision::structure_minimap(m, s as u16, 1)?;
    }
    Ok(())
}
pub(crate) fn calculate(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    let r = unit(m, u)?;
    let kind = m.b(r + 1)?;
    let work = m.b(r + 0x89)?;
    let t = m.w(r + 0xc0)?;
    if work == 0 {
        if m.b(r + 2)? != 15 {
            return Ok(());
        }
        if kind == 6 {
            let cell = m.w(unit(m, t)? + 8)?;
            if cell != m.w(r + 0x7e)? {
                target(m, u, r, cell)?;
            }
        }
        if m.b(r + 0x86)? != 0
            || m.b(r + 0x87)? != 0
            || !(match kind {
                7 => crate::chain_entry::adjacent(m, t as u16, m.w(r + 8)? as u16)?,
                6 => crate::chain_battle::adjacent_units(m, r, t as u16)?,
                _ => true, // 0x44b6eb: no target test for the other kinds.
            })
        {
            crate::frame_movement::displace(m, u)?;
            return Ok(());
        }
        m.sb(r + 0x89, 1)?;
        m.sb(r + 0x8b, 50)?;
        // 0x44b762: only kinds 6 and 7 set the local cell. For the others the
        // original reads an uninitialized stack word at 0x44b772 (0 on a stack
        // nothing has dirtied; `validation/claude-20261108/README.md`).
        let cell = match kind {
            6 => m.w(unit(m, t)? + 0x7c)?,
            7 => crate::frame_core::building_cell(m, t)?,
            _ => 0,
        };
        m.sb(
            r + 0x90,
            u32::from(crate::frame_path::direction(
                m.w(r + 8)? as u16,
                cell as u16,
                false,
            )),
        )?;
        return Ok(());
    }
    if work != 1 {
        return Ok(());
    }
    if m.b(r + 0x90)? != m.b(r + 0x91)? {
        crate::frame_movement::call(m, "RotateEngin", &[u])?;
        return Ok(());
    }
    if kind == 6 {
        let cell = m.w(unit(m, t)? + 8)?;
        if cell != m.w(r + 0x7e)? {
            target(m, u, r, cell)?;
            m.sb(r + 0x89, 0)?;
            return Ok(());
        }
    }
    if m.b(r + 0xa4)? != 2 || crate::frame_core::random(m)? % 5 == 0 {
        crate::player_build::anim(m, u)?;
    }
    let active = match kind {
        7 => {
            let s = structure(t);
            m.w(s + 10)? != m.w(0x495d90 + 52 * m.b(s + 8)?)? || m.w(s + 6)? != 0
        }
        6 => {
            let other = unit(m, t)?;
            m.w(other + 0x9a)? < m.w(0x4965b6 + 50 * m.b(other + 1)?)? / 2
        }
        _ => false, // 0x44b882: any other kind ends the repair at once.
    };
    if !active {
        return finish(m, u, r);
    }
    let timer = m.b(r + 0x8b)?;
    if timer != 0 {
        m.sb(r + 0x8b, timer - 1)?;
    }
    if m.b(r + 0x8b)? == 0 && m.b(r + 0xa4)? == 2 {
        let cell = crate::frame_path::neighbor(m, m.w(r + 8)? as u16, m.b(r + 0x91)? as u8)?;
        let seed = crate::frame_core::random(m)?;
        crate::frame_combat::call(
            m,
            "ProjectileInit",
            &[
                if seed & 1 == 0 { 15 } else { 16 },
                u32::from(cell),
                u32::from(cell),
                0,
                0,
                0,
                0,
                0,
                0,
            ],
        )?;
        if kind == 6 {
            let other = unit(m, t)?;
            let hp = m.w(other + 0x9a)?.wrapping_add(2) & 65535;
            m.sw(other + 0x9a, hp)?;
            let limit = m.w(0x4965b6 + 50 * m.b(other + 1)?)?;
            if hp > limit {
                m.sw(other + 0x9a, limit)?;
            }
        } else {
            repair_building(m, t)?;
        }
        m.sb(r + 0x8b, 20)?;
    }
    sound(m, u, 0x2500, 0x44bab9)
}
pub(crate) fn state15(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    let kind = m.b(r + 1)?;
    let t = m.w(r + 0xc0)?;
    if (kind == 6 && !alive(m, t)?) || (kind == 7 && m.b(structure(t) + 5)? == 3) {
        crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(u), m.w(r + 0x7e)?])?;
        return m.sw(r + 0xc0, 65535);
    }
    calculate(m, u32::from(u))
}
