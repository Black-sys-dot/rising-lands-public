//! Economy restoration and tag19: typed effect APIs; x86 scratch values never escape them.
use crate::{
    frame::stop,
    player::{alive, structure, unit, Memory},
    MemoriaJogo, Result,
};
fn target(m: &mut MemoriaJogo, u: u32, c: u32) -> Result<()> {
    let r = unit(m, u)?;
    m.sw(r + 0x7e, c)?;
    crate::frame_spawn::release(m, u)?;
    m.sb(r + 0x88, 0)
}
pub fn debit(m: &mut MemoriaJogo, resource: u8, qty: u16, p: u8) -> Result<()> {
    let base = 0x4e2ea8 + 32 * u32::from(p) + 8 * u32::from(resource);
    let total = m.w(base + 2)?;
    let amount = u32::from(qty);
    m.sw(base + 2, total.saturating_sub(amount))?;
    m.sw(base, m.w(base)?.saturating_sub(amount))?;
    if m.w(0x4dfb2c + 6 * u32::from(p))? != 0 {
        crate::chain_habitation::level(m, u32::from(p), 0x423635)?;
    }
    Ok(())
}
pub fn unload(m: &mut MemoriaJogo, u: u32, s: u32) -> Result<()> {
    let r = unit(m, u)?;
    m.sb(r + 2, 20)?;
    m.sb(r + 0x89, 0)?;
    let c = crate::frame_core::building_cell(m, s)?;
    m.sw(r + 0xb2, c)?;
    m.sw(r + 0xc0, s)?;
    target(m, u, m.w(r + 0xb2)?)
}
pub fn select_unload(m: &mut MemoriaJogo, p: u32, s: u32) -> Result<()> {
    let base = 0x508728 + (p << 9);
    let (mut i, mut spoke) = (0, false);
    while i < m.w(base)? {
        let u = m.w(base + 2 + 2 * i)?;
        i += 1;
        if !alive(m, u)? {
            continue;
        }
        unload(m, u, s)?;
        if !spoke {
            spoke = true;
            if p == m.b(0x508041)? {
                crate::player_orders::ask(m, u, 15, 0, u)?;
            }
        }
    }
    Ok(())
}
pub fn restore(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    let r = unit(m, u)?;
    let state = m.w(r + 0xa0)?;
    match state {
        1 => {
            crate::player_orders::call(
                m,
                "SetAttaqueEngin",
                [u, m.b(r + 0xb7)?, m.w(r + 0xb8)?, 0],
                0x45349a,
            )?;
        }
        2 | 3 | 7 => {
            m.sb(r + 2, 2)?;
            target(m, u, m.w(r + 0xa2)?)?;
        }
        5 => {
            let (mode, depot, value, cell) = (
                m.b(r + 0x82)?,
                m.w(r + 0x80)?,
                m.b(r + 0x84)?,
                m.w(r + 0xb2)?,
            );
            let builder = m.b(r + 1)? == 7;
            crate::frame_spawn::reset_order(m, u)?;
            let c = if builder {
                cell.wrapping_sub(0x100).wrapping_add(0x100) & 65535
            } else {
                cell
            };
            target(m, u, c)?;
            m.sb(r + 0x89, 0)?;
            m.sb(r + 2, 5)?;
            m.sb(r + 0x82, 0)?;
            m.sw(r + 0xb2, c)?;
            m.sw(r + 0x80, depot)?;
            m.sb(r + 0x82, mode)?;
            m.sw(r + 0x84, value)?;
        }
        11 | 15 => {
            let t = m.b(r + 1)?;
            if t == 7 {
                let s = m.w(r + 0xc0)?;
                if s != 65535 {
                    crate::frame_spawn::reset_order(m, u)?;
                    let c = crate::frame_core::building_cell(m, s)?;
                    target(m, u, c)?;
                    m.sb(r + 0x88, 0)?;
                    m.sb(r + 0x89, 0)?;
                    m.sb(r + 2, 15)?;
                    m.sw(r + 0xc0, s)?;
                    m.sw(r + 0xb2, c)?;
                } else {
                    let t = m.b(r + 0xbe)?;
                    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
                    let offset = crate::player_orders::construction_offset(t);
                    let c = m.w(r + 0xb2)?.wrapping_add(offset).wrapping_sub(
                        (w - w / 2 - 1)
                            .wrapping_add((h - h / 2).wrapping_shl(8))
                            .wrapping_sub(0x100),
                    ) & 65535;
                    crate::player_orders::build_order(m, u, t, c)?;
                }
            } else if t == 6 {
                let other = m.w(r + 0xc0)?;
                crate::frame_spawn::reset_order(m, u)?;
                let c = m.w(unit(m, other)? + 0x7c)?;
                target(m, u, c)?;
                m.sb(r + 0x88, 0)?;
                m.sb(r + 0x89, 0)?;
                m.sb(r + 2, 15)?;
                m.sw(r + 0xb2, c)?;
                m.sw(r + 0xc0, other)?;
            }
        }
        13 => {
            let s = m.w(r + 0xc0)?;
            if s != 65535 {
                crate::chain_entry::set_enter(m, u as u16, s as u16)?;
            } else {
                let other = m.w(r + 0xb8)?;
                if other != 65535 {
                    target(m, u, m.w(unit(m, other)? + 8)?)?;
                    m.sw(r + 0xc0, 65535)?;
                    m.sb(r + 2, 13)?;
                    m.sw(r + 0xb8, other)?;
                }
            }
        }
        16 => {
            let c = m.w(r + 0xb2)?;
            crate::frame_spawn::reset_order(m, u)?;
            target(m, u, c)?;
            m.sb(r + 2, 16)?;
            m.sb(r + 0x89, 0)?;
            m.sw(r + 0xb2, c)?;
        }
        17 => {
            m.sb(r + 0x82, 2)?;
            let v = m.w(r + 0x84)?;
            m.sw(r + 0x84, 0)?;
            m.sw(r + 0x80, v)?;
        }
        18 => {
            m.sb(r + 2, 18)?;
            let c = m.w(r + 0xb2)?;
            m.sb(r + 0x89, 0)?;
            m.sw(r + 0xb2, c)?;
            target(m, u, c)?;
        }
        20 => {
            let (mode, depot, value, s) = (
                m.b(r + 0x82)?,
                m.w(r + 0x80)?,
                m.b(r + 0x84)?,
                m.w(r + 0xc0)?,
            );
            unload(m, u, s)?;
            m.sb(r + 0x82, mode)?;
            m.sw(r + 0x80, depot)?;
            m.sw(r + 0x84, value)?;
        }
        24 => {
            let (f, other, mode) = (m.b(r + 0xf4)?, m.w(r + 0xb8)?, m.b(r + 0xb7)?);
            crate::frame_spawn::reset_order(m, u)?;
            if m.b(r + 1)? == 4 {
                m.sb(r + 0xf4, f)?;
                if (mode == 1 && u != other) || mode == 5 {
                    m.sb(r + 2, 24)?;
                    m.sb(r + 0x8a, 0)?;
                    m.sb(r + 0xb7, mode)?;
                    m.sw(r + 0xb8, other)?;
                }
            }
        }
        _ => {
            crate::frame_spawn::order(m, u, m.w(r + 0x7c)? as u16)?;
        }
    }
    Ok(())
}
pub fn hunger(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    let r = unit(m, u)?;
    let state = m.w(r + 0xa0)?;
    if matches!(state, 1 | 24) {
        crate::chain_attack::validate_target(m, u as u16, r)?;
    }
    if matches!(state, 11 | 21) {
        let building = m.w(r + 0xc0)?;
        if building != 65535 && m.b(structure(building) + 5)? == 3 {
            crate::frame_spawn::order(m, u, m.w(r + 0x7e)? as u16)?;
            return Ok(());
        }
    }
    if state == 15 {
        let kind = m.b(r + 1)?;
        let target = m.w(r + 0xc0)?;
        let invalid = if kind == 6 {
            matches!(m.b(unit(m, target)? + 2)?, 0 | 4)
        } else {
            kind == 7 && m.b(structure(target) + 5)? == 3
        };
        if invalid {
            crate::frame_spawn::order(m, u, m.w(r + 0x7e)? as u16)?;
            m.sw(r + 0xc0, 65535)?;
            return Ok(());
        }
    }
    if state == 18 {
        // 0x454fad: a lost depot puts the worker back and ends the step;
        // otherwise the meal continues at 0x455554.
        if crate::transport_delivery_next::lost_depot(m, u as u16, 0x454fc7)? {
            return Ok(());
        }
    }
    if state == 13 {
        // 0x4553b4: the saved entry target must still be valid, otherwise
        // the unit heads for +0x7e and the target is cleared (step ends).
        let s = m.w(r + 0xc0)?;
        if s != 65535 {
            let a = structure(s);
            let p = m.b(a + 4)?;
            let refused = if m.b(a + 5)? == 3 {
                true
            } else if m.b(r + 1)? == 10 {
                p == 200 || p == m.b(r)?
            } else {
                p != m.b(r)? && !(p == 200 && m.b(r + 1)? == 9)
            };
            if refused {
                crate::frame_spawn::order(m, u, m.w(r + 0x7e)? as u16)?;
                m.sw(r + 0xc0, 65535)?;
                return Ok(());
            }
        } else {
            let t = m.w(r + 0xb8)?;
            if t != 65535 {
                let tr = unit(m, t)?;
                let kept =
                    !matches!(m.b(tr + 2)?, 0 | 4) && (m.b(tr)? == m.b(r)? || m.b(tr)? == 200);
                if !kept {
                    crate::frame_spawn::order(m, u, m.w(r + 0x7e)? as u16)?;
                    m.sw(r + 0xb8, 65535)?;
                    return Ok(());
                }
            }
        }
    }
    if state == 23 {
        // 0x4551f6: the saved building target (+0xc0) must stand and belong
        // to another real player, otherwise head for +0x7e and clear it.
        let a = structure(m.w(r + 0xc0)?);
        let p = m.b(a + 4)?;
        if m.b(a + 5)? == 3 || p == 200 || p == m.b(r)? {
            crate::frame_spawn::order(m, u, m.w(r + 0x7e)? as u16)?;
            m.sw(r + 0xc0, 65535)?;
            return Ok(());
        }
    }
    if state == 5 {
        // 0x4550ce: a lost work depot (mode 2, destroyed or foreign) puts the
        // unit back (MineEndWork first for a type-7 miner) and ends the step;
        // otherwise the meal continues at 0x455554.
        if crate::transport_delivery_next::lost_work_depot(m, u as u16)? {
            return Ok(());
        }
    }
    if state == 20 {
        valid(m, r, m.w(r + 0xc0)?, 0x454ddd)?;
        if m.b(r + 0x82)? == 2 {
            valid(m, r, m.w(r + 0x80)?, 0x454ef8)?;
        }
    }
    let s = m.w(r + 0xc2)?;
    let arrived = if s == 65535 {
        m.w(r + 8)? == m.w(r + 0x7e)?
    } else {
        crate::chain_entry::adjacent(m, s as u16, m.w(r + 8)? as u16)?
    };
    if !arrived || m.b(r + 0x86)? != 0 || m.b(r + 0x87)? != 0 {
        crate::frame_movement::displace(m, u)?;
        return Ok(());
    }
    let mut fed = false;
    if s == 65535 {
        let c = m.w(r + 0x7e)?;
        if m.b(m.d(0x507d74)? + c)?.wrapping_sub(0x42) & 255 == 1 {
            retire_food(m, c)?;
            fed = true;
        }
    } else {
        let e = structure(s);
        if m.b(e + 8)? == 3
            && m.b(e + 4)? == m.b(r)?
            && crate::player_orders::resources(m, m.b(r)?, 1, 0, 0)?
        {
            debit(m, 1, 1, m.b(r)? as u8)?;
            fed = true;
        }
    }
    if fed {
        m.sw(r + 4, m.w(r + 0x76)?)?;
        m.sw(r + 0x74, 0)?;
        meal_sound(m, u, 0)?;
        return restore(m, u);
    }
    let c = crate::chain_fatigue::free_veget(m, m.w(r + 0x7c)?, m.b(r + 0xbb)?, 0x43, 0x43, u)?;
    if c != 65535 {
        m.sb(r + 2, 19)?;
        target(m, u, c)?;
        sound_at(m, u, 0, 0x45582d)?;
    } else {
        if m.b(r)? == m.b(0x508041)? && m.w(r + 0x74)? == 1 {
            meal_sound(m, u, 0xc00)?;
        }
        restore(m, u)?;
    }
    Ok(())
}
fn valid(m: &MemoriaJogo, r: u32, s: u32, site: u32) -> Result<()> {
    let a = structure(s);
    if m.b(a + 5)? == 3 || m.b(a + 4)? != m.b(r)? {
        return stop("EstadoFome", site);
    }
    Ok(())
}
pub fn retire_food(m: &mut MemoriaJogo, c: u32) -> Result<u32> {
    let q = m.d(0x507dac)? + (c & 65535);
    let mut v = 0;
    if m.b(q)? != 0 {
        m.sb(q, m.b(q)?.wrapping_sub(1))?;
        v = m.b(m.d(0x507d74)? + (c & 65535))?.wrapping_sub(0x42) & 255;
    }
    if m.b(m.d(0x507dac)? + (c & 65535))? == 0 {
        m.sb(m.d(0x507d74)? + (c & 65535), 0)?;
    }
    Ok((c & 0xff00) | v)
}
pub fn meal_sound(m: &mut MemoriaJogo, u: u32, voice: u32) -> Result<()> {
    sound_at(m, u, voice, 0x455660)
}
pub fn sound_at(m: &mut MemoriaJogo, u: u32, voice: u32, site: u32) -> Result<()> {
    let r = unit(m, u)?;
    let local = m.b(0x508041)?;
    if crate::frame_core::call(m, "TerrainEstDecouvert", &[m.w(r + 8)?, local])? & 255 == 0 {
        return Ok(());
    }
    let volume = crate::player_orders::volume(m, m.w(r + 8)?)?;
    let h = m.d(r + 0x104)?;
    m.sb(r + 0xf9, 0)?;
    if h == u32::MAX {
        crate::player::sound(
            m,
            site,
            true,
            [0x50, (m.b(r + 1)? << 16) | voice, r + 0x104, 0],
            Some(volume),
        )?;
    }
    Ok(())
}
