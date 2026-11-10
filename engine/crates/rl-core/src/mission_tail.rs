//! MissionLoad economy/experience/player tails, with original selective writes.
use crate::{
    mission_load::{bag_reset, copy, require},
    player::Memory,
    FrameRegisters, MemoriaJogo, Result,
};
pub(crate) fn set_resource(m: &mut MemoriaJogo, p: u32, r: u32, n: u16) -> Result<()> {
    let a = 0x4e2ea8 + 32 * p + 8 * r;
    m.set_u16(a, n)?;
    m.set_u16(a + 2, n)?;
    m.set_u16(a + 4, 0)?;
    m.set_u8(a + 6, 0)
}
fn bag(m: &mut MemoriaJogo, a: u32, cap: u16, entries: &[(u8, u16)]) -> Result<()> {
    bag_reset(m, a)?;
    m.set_u16(a + 2, cap)?;
    for &(tag, value) in entries {
        let i = u32::from(m.u16(a)?);
        m.set_u16(a + 0x66, m.u16(a + 0x66)?.wrapping_add(1))?;
        m.set_u8(a + 0x42 + i, tag)?;
        m.set_u16(a + 6 + 2 * i, value)?;
        m.set_u16(a, m.u16(a)?.wrapping_add(1))?;
    }
    Ok(())
}
pub(crate) fn technology(m: &mut MemoriaJogo, p: u32, t: u32) -> Result<()> {
    if p >= 4 {
        return Ok(());
    }
    let a = 0x4dd6e8 + 0x8f0 * p + 104 * t;
    if t == 6 {
        let mut entries = Vec::new();
        for s in 0..5 {
            if m.u8(0x508f44 + 5 * p + s)? != 0 && m.u8(0x508f30 + 5 * p + s)? == 0 {
                entries.push((4, s as u16));
            }
        }
        return bag(m, a, 5, &entries);
    }
    let (cap, types): (u16, &[u32]) = match t {
        1 => (4, &[0, 1, 2, 8]),
        2 => (4, &[5, 6, 7]),
        3 => (3, &[]),
        5 => (5, &[11, 12, 13, 14, 15]),
        13 => (2, &[3, 25]),
        15 | 20 => (2, &[4]),
        16 => (1, &[10]),
        18 => (1, &[16]),
        _ => return Ok(()),
    };
    let mut entries = Vec::new();
    for &u in types {
        if m.u8(0x4e1934 + 26 * p + u)? != 0 {
            entries.push((6, u as u16));
        }
    }
    if t == 15 && m.u8(0x4dfad4 + 22 * p + 20)? != 0 {
        entries.push((3, 20));
    }
    if t == 20 {
        entries.push((4, 5));
    }
    bag(m, a, cap, &entries)
}
pub(crate) fn inhabited(m: &MemoriaJogo, s: u32) -> Result<bool> {
    let a = 0x49a0c8 + 276 * s;
    let t = m.u8(a + 8)?;
    let cap = u32::from(m.u16(a + 14)?);
    require(cap <= 30, "BatimentEstHabiter Bag capacity", 0x41a5b8)?;
    for i in 0..cap {
        if m.u8(a + 0x4e + i)? != 1 {
            continue;
        }
        let u = u32::from(m.u16(a + 0x12 + 2 * i)?);
        require(u < 800, "BatimentEstHabiter unit index", 0x44724a)?;
        let kind = m.u8(m.u32(0x4e19a0)? + 276 * u + 1)?;
        if (matches!(t, 2 | 21) && !matches!(kind, 3 | 11..=17 | 25)) || (t != 21 && kind == 9) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(crate) fn habitable(m: &mut MemoriaJogo, s: u32) -> Result<()> {
    let a = 0x49a0c8 + 276 * s;
    let p = u32::from(m.u8(a + 4)?);
    let t = u32::from(m.u8(a + 8)?);
    technology(m, p, t)?;
    if matches!(t, 9 | 12 | 17) {
        m.set_u16(a + 0x84, 200)?;
        return Ok(());
    }
    if t == 4 {
        m.set_u16(a + 0x78, 0)?;
    }
    if t == 3 && p < 4 && m.u16(0x4dfb2c + 6 * p)? != 0 {
        crate::chain_habitation::level(m, p, 0x41cd55)?;
    }
    const RADII: [u32; 22] = [
        20, 8, 10, 8, 5, 8, 8, 8, 8, 0, 7, 7, 0, 8, 8, 8, 8, 4, 8, 4, 8, 4,
    ];
    require(t < 22, "BatimentHabitable type", 0x41cd62)?;
    let c = crate::frame_core::building_cell(m, s)? as u16;
    let r = RADII[t as usize];
    crate::player_sight::area(m, c, r + 3, p as u8, 0)?;
    crate::player_sight::area(m, c, r, p as u8, 1)?;
    m.set_u16(a + 0x84, 200)
}
pub(crate) fn load_tail(m: &mut MemoriaJogo) -> Result<()> {
    let mut technology = crate::chain_technology::Technology::default();
    for p in 0..4 {
        technology.entry(
            m,
            "EnginSetTechnologie",
            FrameRegisters {
                eax: p,
                ..Default::default()
            },
        )?;
    }
    for p in 0..4 {
        for t in 0..22 {
            m.set_u8(0x49a070 + 22 * p + t, 0)?;
        }
        for i in 0..1000 {
            let a = 0x49a0c8 + 276 * i;
            if m.u8(a + 5)? != 0 && u32::from(m.u8(a + 4)?) == p {
                let t = u32::from(m.u8(a + 8)?);
                require(t < 22, "BatimentInitCave type", 0x4191bf)?;
                let c = 0x49a070 + 22 * p + t;
                m.set_u8(c, m.u8(c)?.wrapping_add(1))?;
            }
        }
        m.set_u8(0x4dfb4c + p, m.u8(0x49a070 + 22 * p + 18)?)?;
        m.set_u16(0x4dfb2c + 6 * p, u16::from(m.u8(0x49a070 + 22 * p + 3)?))?;
    }
    copy(m, 0x4e19c4, 0x4e19ec, 40)?;
    let (players, saved) = (m.u32(0x5080a8)?, m.u32(0x5080ac)?);
    for p in 0..4 {
        m.set_u8(players + 20 * p + 6, m.u8(saved + 20 * p + 6)?)?;
    }
    for p in 0..4 {
        copy(m, players + 20 * p + 12, saved + 20 * p + 12, 8)?;
    }
    Ok(())
}
pub(crate) fn market_reset(m: &mut MemoriaJogo) -> Result<()> {
    for p in 0..4 {
        m.sb(0x4e2e7c + p, 0)?;
        m.sw(0x4e2930 + 2 * p, 0)?;
        m.sb(0x496f8c + p, 255)?;
        m.sb(0x4e292c + p, 1)?;
        m.sb(0x4e2928 + p, 0)?;
        m.sw(0x4e2940 + 2 * p, 0)?;
        m.sw(0x4e2938 + 2 * p, 0)?;
    }
    Ok(())
}
