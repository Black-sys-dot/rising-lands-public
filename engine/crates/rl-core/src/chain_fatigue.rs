//! Core hunger entry and vegetation searches; semantic cell/effect APIs.
use crate::{
    frame::stop,
    player::{structure, unit, Memory},
    MemoriaJogo, Result,
};
pub fn free_veget(m: &MemoriaJogo, c: u32, radius: u32, lo: u32, hi: u32, u: u32) -> Result<u32> {
    let radius = radius & 65535;
    if radius >= 0x7fff {
        return stop("ChercheVegetLibre radius", 0x477bf0);
    }
    let c = c & 65535;
    let g = m.d(0x507d74)?;
    if (lo..=hi).contains(&m.b(g + c)?) && crate::chain_search::passable(m, u, c)? {
        return Ok(c);
    }
    let (x, y) = ((c & 255) as i32, (c >> 8) as i32);
    let (w, h) = (m.w(0x497168)? as i32, m.w(0x49716a)? as i32);
    for k in 1..=radius as i32 {
        for i in -k..=k {
            for (ok, offset) in [
                (x - k >= 0 && y + i >= 0 && y + i < h, i * 256 - k),
                (x + k < w && y + i >= 0 && y + i < h, i * 256 + k),
                (
                    i != -k && i != k && y - k >= 0 && x + i >= 0 && x + i < w,
                    -k * 256 + i,
                ),
                (
                    i != -k && i != k && y + k < h && x + i >= 0 && x + i < w,
                    k * 256 + i,
                ),
            ] {
                if !ok {
                    continue;
                }
                let q = c.wrapping_add(offset as u32);
                if (lo..=hi).contains(&m.b(g.wrapping_add(q))?)
                    && crate::chain_search::passable(m, u, q & 65535)?
                {
                    return Ok(q & 65535);
                }
            }
        }
    }
    Ok(65535)
}
pub fn nearby(m: &MemoriaJogo, c: u32, radius: u32, t: u32, p: u32) -> Result<u32> {
    let c = c & 65535;
    let radius = (radius & 65535) * (radius & 65535);
    let (mut best, mut found) = (u32::MAX, 65535);
    for s in 0..1000 {
        let a = structure(s);
        if m.b(a + 5)? == 0 || m.b(a + 4)? != p || m.b(a + 8)? != t {
            continue;
        }
        let target = crate::frame_core::building_cell(m, s)? & 65535;
        let dx = (c & 255) as i32 - (target & 255) as i32;
        let dy = (c >> 8) as i32 - (target >> 8) as i32;
        let d = (dx * dx + dy * dy) as u32;
        if d < radius && d < best {
            best = d;
            found = s
        }
    }
    Ok(found)
}
pub fn set(m: &mut MemoriaJogo, u: u32) -> Result<()> {
    if u >= 800 {
        return stop("EnginSetFatigue unit", 0x453b8b);
    }
    let r = unit(m, u)?;
    if m.w(r + 4)? == 4 || m.b(r + 2)? == 19 {
        return Ok(());
    }
    m.sw(r + 0xa0, m.b(r + 2)?)?;
    m.sw(r + 0xa2, m.w(r + 0x7e)?)?;
    let c = free_veget(m, m.w(r + 8)?, 20, 0x43, 0x43, u)?;
    if c != 65535 {
        m.sw(r + 0xc2, 65535)?;
        m.sb(r + 2, 19)?;
        m.sw(r + 0x7e, c)?;
        crate::frame_spawn::release(m, u)?;
        m.sb(r + 0x88, 0)?;
        return Ok(());
    }
    let p = m.b(r)?;
    if !crate::player_orders::resources(m, p, 3, 0, 0)? {
        return Ok(());
    }
    let s = nearby(m, m.w(r + 8)?, 40, 3, p)?;
    if s != 65535 {
        let c = crate::frame_core::building_cell(m, s)?;
        m.sb(r + 2, 19)?;
        m.sw(r + 0xc2, s)?;
        m.sw(r + 0x7e, c)?;
        crate::frame_spawn::release(m, u)?;
        m.sb(r + 0x88, 0)?;
    } else if p == m.b(0x508041)? && m.w(r + 0x74)? == 1 {
        crate::chain_restore::sound_at(m, u, 0xc00, 0x44b33d)?;
    }
    Ok(())
}

/// CalculFatigue0x452d6f: native warning, food consumption and starvation.
/// `limit` is the wrapping u16 threshold already used by the caller.
pub(crate) fn starve(m: &mut MemoriaJogo, u: u16, limit: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    let kind = m.b(r + 1)?;
    if m.b(r)? == m.b(0x508041)?
        && !matches!(kind, 0 | 3 | 10 | 25)
        && m.w(r + 0x74)? == u32::from(limit)
        && crate::frame_core::call(m, "TerrainEstDecouvert", &[m.w(r + 8)?, m.b(0x508041)?])? & 255
            != 0
    {
        let volume = crate::player_orders::unit_volume(m, u32::from(u))?;
        m.sb(r + 0xf9, 0)?;
        if m.d(r + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x452e2a,
                true,
                [0x50, (kind << 16) | 0xc00, r + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    let food = crate::chain_bags::body(
        m,
        "BagContient",
        crate::FrameRegisters {
            eax: r + 10,
            edx: 2,
            ebx: 1,
            ..Default::default()
        },
    )?
    .eax as u16;
    if food != 0 {
        crate::chain_bags::body(
            m,
            "BagRetire",
            crate::FrameRegisters {
                eax: r + 10,
                edx: 2,
                ebx: 1,
                ..Default::default()
            },
        )?;
        m.sw(r + 0x74, 0)?;
        m.sw(r + 4, m.w(r + 0x76)?)?;
    } else {
        let hp = m.w(r + 0x9a)?;
        if hp <= 20 {
            m.sw(r + 0x9a, 0)?;
            crate::chain_death::set(m, u)?;
        } else {
            m.sw(r + 0x9a, hp - 20)?;
            m.sw(r + 0x74, m.w(0x4965dc + 50 * kind)? / 2)?;
        }
    }
    Ok(())
}
