//! V_VideEngin 0x481e88 -> SetEnginDepose 0x444533 mode 1 -> EnginRetireType
//! 0x4438d8 mode 1: a transport unloads its troops one by one next to it.
use crate::{player::unit, MemoriaJogo, Result};

/// TypeEnginEstTroupe 0x442347 (also inlined in SetEnginDepose/EnginRetireType).
fn troop(kind: u8) -> bool {
    !matches!(kind, 3 | 11..=17 | 25)
}
/// First bag slot of kind 1 holding a troop (0x4446d7 / 0x443b7d scans).
fn has_troop(m: &MemoriaJogo, r: u32) -> Result<bool> {
    let bag = r + 0xa;
    let pool = m.u32(0x4e19a0)?;
    for i in 0..u32::from(m.u16(bag + 2)?) {
        if m.u8(bag + 0x42 + i)? != 1 {
            continue;
        }
        let v = u32::from(m.u16(bag + 6 + 2 * i)?);
        if troop(m.u8(pool.wrapping_add(276 * v) + 1)?) {
            return Ok(true);
        }
    }
    Ok(false)
}
fn sight(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    let owner = m.u8(r)?;
    let cell = m.u16(r + 8)?;
    if !matches!(owner, 4 | 200) {
        let radius = u32::from(m.u8(r + 0xbb)?) + u32::from(m.u8(r + 0xf5)?);
        crate::player_sight::area(m, cell, radius + 2, owner, 0)?;
        crate::player_sight::area(m, cell, radius, owner, 1)?;
    }
    crate::player_sight::minimap(m, owner, cell, 4)?;
    Ok(())
}
/// EnginRetireType 0x4438d8 mode 1 (`kind` is the caller's BX, passed on to
/// EnginCherchePlace as its unit argument, as the original does).
pub(crate) fn retire_type(m: &mut MemoriaJogo, u: u16, kind: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    let place = crate::spell_remaining::find_place(m, m.u16(r + 8)?, kind)?;
    if place != 0xffff {
        let v = crate::chain_bags::body(
            m,
            "BagRetire",
            crate::FrameRegisters {
                eax: r + 0xa,
                edx: 1,
                ebx: u32::from(kind),
                ..Default::default()
            },
        )?
        .eax as u16;
        let rv = unit(m, u32::from(v))?;
        m.set_u16(rv + 0x7e, place)?;
        m.set_u16(rv + 0x7c, place)?;
        m.set_u16(rv + 8, place)?;
        crate::frame_spawn::release(m, u32::from(v))?;
        m.set_u8(rv + 0x88, 0)?;
        if !matches!(m.u8(rv + 2)?, 0 | 4) {
            m.set_u8(rv + 2, 7)?;
            crate::frame_spawn::release(m, u32::from(v))?;
        }
        m.set_u16(rv + 0xc0, 0xffff)?;
        m.set_u16(rv + 0xb8, 0xffff)?;
        m.set_u8(rv + 0x87, 0)?;
        m.set_u8(rv + 0x86, 0)?;
        crate::frame_core::call(
            m,
            "SetEnginCollision",
            &[u32::from(v), u32::from(m.u16(rv + 8)?)],
        )?;
        m.set_u8(rv + 0x72, 1)?;
        sight(m, rv)?;
    }
    // 0x443b41: a group transport left without troops (not the balloon)
    // turns neutral 200 and is put back on the map.
    let kind = m.u8(r + 1)?;
    if !(11..=17).contains(&kind) || has_troop(m, r)? || kind == 17 {
        return Ok(());
    }
    let local = m.u8(0x508041)?;
    if m.u8(r)? == local
        && crate::frame_core::call(
            m,
            "TerrainEstDecouvert",
            &[u32::from(m.u16(r + 8)?), u32::from(local)],
        )? as u8
            != 0
    {
        let volume = crate::player_orders::unit_volume(m, u32::from(u))?;
        m.set_u8(r + 0xf9, 0)?;
        if m.u32(r + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x443c89,
                true,
                [0x50, (u32::from(kind) << 16) | 0x2600, r + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    crate::frame_spawn::reset_order(m, u32::from(u))?;
    crate::player_orders::call(
        m,
        "DeselectionEngin",
        [u32::from(u), u32::from(u), u32::from(u), 0],
        0x443ca0,
    )?;
    crate::chain_entry::retire(m, u)?;
    m.set_u8(r, 200)?;
    crate::frame_core::call(
        m,
        "SetEnginCollision",
        &[u32::from(u), u32::from(m.u16(r + 8)?)],
    )?;
    m.set_u8(r + 0x72, 1)?;
    sight(m, r)
}
/// SetEnginDepose 0x444533 mode 1: while a troop is aboard, unload with the
/// kind of the unit in bag slot 0 (not necessarily the troop found).
pub fn unload_all(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let r = unit(m, u32::from(u))?;
    for _ in 0..1000 {
        if !has_troop(m, r)? {
            return Ok(());
        }
        let first = u32::from(m.u16(r + 0xa + 6)?);
        let kind = m.u8(m.u32(0x4e19a0)?.wrapping_add(276 * first) + 1)?;
        retire_type(m, u, u16::from(kind))?;
    }
    crate::frame::stop("SetEnginDepose unload loop", 0x4446da)
}

/// SetEnginDepose 0x444533 (ProcessInputGame tag 9, 0x45f1b3: word +2 unit,
/// byte +4 mode, byte +6 resource). Mode 1 unloads a transport's troops
/// (`unload_all`). Mode 2 (a worker drops its load): ResetOrdre, then with
/// resource 0 the nearest free cell (ChercheVeget radius 10, objects 0..=0)
/// becomes the destination; otherwise the nearest cell already holding that
/// resource's sack (ChercheVeget radius 10, object resource + 0x42), or the
/// unit's own cell, seeds SacCherchePlace(resource + 0x42); with no place the
/// unit just walks to its cell (SetDeplaceEngin(+0x7c)). A destination sets
/// state 0x11, +0x82 = mode, +0x84 = resource, +0x7e = cell, releases the
/// pathfinding slot and clears +0x88. Other modes return.
pub fn depose(m: &mut MemoriaJogo, u: u16, mode: u8, resource: u16) -> Result<()> {
    match mode {
        1 => unload_all(m, u),
        2 => {
            let r = unit(m, u32::from(u))?;
            crate::frame_spawn::reset_order(m, u32::from(u))?;
            let cell = if resource == 0 {
                let c = crate::chain_search::veget(m, u32::from(m.u16(r + 8)?), 10, 0, 0)? as u16;
                if c == 0xffff {
                    return Ok(());
                }
                c
            } else {
                let wanted = (resource as u8).wrapping_add(0x42);
                let mut seed = crate::chain_search::veget(
                    m,
                    u32::from(m.u16(r + 8)?),
                    10,
                    u32::from(wanted),
                    u32::from(wanted),
                )? as u16;
                if seed == 0xffff {
                    seed = m.u16(r + 8)?;
                }
                let c = crate::chain_cargo::find(m, seed, wanted)?;
                if c == 0xffff {
                    crate::frame_spawn::order(m, u32::from(u), m.u16(r + 0x7c)?)?;
                    return Ok(());
                }
                c
            };
            m.set_u8(r + 2, 0x11)?;
            m.set_u8(r + 0x82, mode)?;
            m.set_u16(r + 0x84, resource)?;
            m.set_u16(r + 0x7e, cell)?;
            crate::frame_spawn::release(m, u32::from(u))?;
            m.set_u8(r + 0x88, 0)
        }
        _ => Ok(()),
    }
}
