//! Typed structure-entry effects, pinned Core entrada_estruturas54d148d.
//! Scratch GPRs are not part of these typed effect APIs.
use crate::{
    frame::stop,
    player::{self, alive, structure, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
/// GetCaseCentrale0x423b27 uses the geometric type rectangle, independently
/// of BatimentGetCase's collision-mask centre. Game consumers use only AX.
pub fn central_offset(m: &MemoriaJogo, s: u32) -> Result<u32> {
    let s = s & 65535;
    if s >= 1000 {
        return stop("GetCaseCentrale structure index", 0x423b27);
    }
    let a = structure(s);
    let t = m.b(a + 8)?;
    if t >= 22 {
        return stop("GetCaseCentrale structure type", 0x423b27);
    }
    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
    Ok(m.w(a + 2)?.wrapping_add((h >> 1) << 8).wrapping_add(w >> 1))
}
pub fn adjacent(m: &mut MemoriaJogo, s: u16, c: u16) -> Result<bool> {
    let a = structure(u32::from(s));
    let (x, y) = (i32::from(c & 255), i32::from(c >> 8));
    for dy in -1..=1 {
        if !(0..256).contains(&(y + dy)) {
            continue;
        }
        for dx in -1..=1 {
            if !(0..256).contains(&(x + dx)) {
                continue;
            }
            let neighbor = u32::from(c).wrapping_add((dy * 256 + dx) as u32) & 65535;
            let origin = m.w(a + 2)?;
            let t = m.b(a + 8)?;
            let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
            let width = ((u32::from(s) * 276) & 0xffff0000) | w;
            for cy in 0..h {
                for cx in 0..w {
                    if origin + (cy << 8) + cx == neighbor
                        && crate::frame_core::call(
                            m,
                            "BatimentCollision",
                            &[m.b(a + 8)?, (width * cy + cx) & 65535],
                        )? & 255
                            != 0
                    {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
pub fn entry_cell(m: &MemoriaJogo, s: u16, u: u16) -> Result<u32> {
    let a = structure(u32::from(s));
    let t = m.b(a + 8)?;
    Ok(if t == 14 {
        (((u32::from(s) * 276) & 0xffff0000) | m.w(a + 2)?) + 0x402
    } else if t == 8 {
        let k = m.b(unit(m, u32::from(u))? + 1)?;
        m.w(a + 2)?
            + if k == 3 || (11..=17).contains(&k) {
                0x403
            } else {
                0x303
            }
    } else {
        0
    })
}
pub fn set_enter(m: &mut MemoriaJogo, u: u16, s: u16) -> Result<()> {
    let a = unit(m, u32::from(u))?;
    let b = structure(u32::from(s));
    let t = m.b(b + 8)?;
    let k = if matches!(t, 8 | 14 | 18) {
        Some(m.b(a + 1)?)
    } else {
        None
    };
    if t == 8 && k != Some(9)
        || t == 14 && k.is_some_and(|v| (11..=17).contains(&v)) && m.b(b + 5)? != 4
    {
        let c = entry_cell(m, s, u)?;
        m.sw(a + 0xb2, c)?;
        m.sw(a + 0x7e, c)?;
        crate::frame_spawn::release(m, u32::from(u))?;
        m.sb(a + 0x88, 0)?;
        m.sw(a + 0xc0, u32::from(s))?;
        if alive(m, u32::from(u))? {
            m.sb(a + 2, if t == 8 { 21 } else { 13 })?;
            crate::frame_spawn::release(m, u32::from(u))?;
        }
        return Ok(());
    }
    let mut c = crate::frame_core::building_cell(m, u32::from(s))?;
    if t == 18 && k == Some(16) {
        c = c.wrapping_add(1);
        m.sw(a + 0xb2, c)?;
    }
    m.sw(a + 0x7e, c)?;
    crate::frame_spawn::release(m, u32::from(u))?;
    m.sb(a + 0x88, 0)?;
    m.sw(a + 0xc0, u32::from(s))?;
    if alive(m, u32::from(u))? {
        m.sb(a + 2, 13)?;
        crate::frame_spawn::release(m, u32::from(u))?;
    }
    Ok(())
}
pub fn retire(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let a = unit(m, u32::from(u))?;
    let cell = m.w(a + 8)?;
    crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0, 0, 0], 0x457058)?;
    crate::frame_movement::call(m, "ResetEnginCollision", &[u32::from(u), cell])?;
    if m.w(a + 8)? != m.w(a + 0x7c)? {
        crate::frame_movement::call(m, "ResetEnginCollision", &[u32::from(u), m.w(a + 0x7c)?])?;
    }
    m.sb(a + 0x72, 0)?;
    let p = m.b(a)?;
    crate::player_sight::minimap(m, p as u8, cell as u16, 5)?;
    if !matches!(p, 4 | 200) {
        crate::player_sight::area(
            m,
            m.w(a + 8)? as u16,
            m.b(a + 0xbb)? + m.b(a + 0xf5)?,
            p as u8,
            2,
        )?;
    }
    Ok(())
}
pub fn reset_type(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let a = unit(m, u32::from(u))?;
    for (off, v) in [
        (1, 9),
        (0x93, 1),
        (0x98, 2),
        (0xa7, 2),
        (0xa8, 4),
        (0xa5, 8),
        (0x9d, 255),
        (0xac, 3),
        (0xad, 3),
        (0x99, 0),
        (0xbb, 5),
    ] {
        m.sb(a + off, v)?;
    }
    let b = a + 10;
    for off in [0, 0x66, 4, 2] {
        m.sw(b + off, 0)?;
    }
    for i in 0..30 {
        m.sw(b + 6 + 2 * i, 0)?;
        m.sb(b + 0x42 + i, 0)?;
    }
    m.sw(b + 2, 0)?;
    let maximum = m.w(0x4965b6 + 50 * m.b(unit(m, u32::from(u))? + 1)?)?;
    if maximum < m.w(a + 0x9a)? {
        m.sw(a + 0x9a, maximum)?;
    }
    Ok(())
}

fn bag(m: &mut MemoriaJogo, name: &str, a: u32, d: u32, b: u32) -> Result<R> {
    crate::chain_bags::body(
        m,
        name,
        R {
            eax: a,
            edx: d,
            ebx: b,
            ..Default::default()
        },
    )
}
fn low(v: u32, b: u32) -> u32 {
    (v & !255) | (b & 255)
}
pub fn place_free(m: &mut MemoriaJogo, mut r: R) -> Result<R> {
    let (s, u) = (r.eax & 65535, r.edx & 65535);
    let a = structure(s);
    let t = m.b(a + 8)?;
    let b = a + 12;
    if t == 8 {
        let n = m.w(b)?;
        let limit = (m.b(0x495be9)? >> 1) + 1;
        let ok = u32::from(n < limit);
        r.eax = low(limit, ok);
        r.edx = low(n, ok);
        return Ok(r);
    }
    if t != 19 {
        let q = bag(m, "BagAccepte", b, 1, r.ebx)?;
        let ok = u32::from(q.eax & 255 != 0);
        r.eax = low(q.eax, ok);
        r.edx = low(q.edx, ok);
        return Ok(r);
    }
    let rec = unit(m, u)?;
    let life = m.w(rec + 0x9a)?;
    let k = m.b(rec + 1)?;
    let max = m.w(0x4965b6 + 50 * k)?;
    if life == max && k != 9 {
        r.eax = low(k, 0);
        r.edx = low(u, 0);
        return Ok(r);
    }
    let q = bag(m, "BagAccepte", b, 1, life)?;
    if q.eax & 255 == 0 {
        r.eax = low(q.eax, 0);
        r.edx = low(q.edx, 0);
        return Ok(r);
    }
    let (mut other, mut full) = (0, 0);
    for i in 0..m.w(b)? {
        if m.b(b + 0x42 + i)? != 1 {
            continue;
        }
        let v = unit(m, m.w(b + 6 + 2 * i)?)?;
        let vt = m.b(v + 1)?;
        if vt != 9 {
            other += 1
        } else if m.w(v + 0x9a)? == m.w(0x4965b6 + 50 * vt)? {
            full += 1
        } else {
            other += 1
        }
    }
    if k == 9 && m.w(rec + 0x9a)? == m.w(0x4965b6 + 50 * m.b(rec + 1)?)? {
        let limit = m
            .b(0x495d8c + 52 * m.b(a + 8)? + 18)?
            .wrapping_sub(m.b(0x495be8)?)
            & 255;
        let ok = u32::from(full < limit);
        r.eax = ok;
        r.edx = low(full, ok);
    } else {
        let limit = m.b(0x495be8)?;
        let ok = u32::from((other & 65535) < limit);
        r.eax = low(limit, ok);
        r.edx = low(other & 65535, ok);
    }
    Ok(r)
}

/// BatimentAjouteMatiere credit path. The unported excess-sack provider remains
/// an addressed dependency after the credit, as in the reference.
pub fn credit(m: &mut MemoriaJogo, resource: u8, quantity: u16, p: u8, s: u16) -> Result<()> {
    let a = structure(u32::from(s));
    let t = m.b(a + 8)?;
    if t == 4 {
        // 0x423511: resource and quantity are ignored. The structure takes one
        // more occupant (+0x78, capped below 5) while its bag accepts one
        // (BagAccepte(+0xc, 1)); otherwise BatimentRetireColon ejects the
        // colonists and TerrainDeposeSac drops one sack of resource 1 two rows
        // below the structure cell. The excess word stays 0, so the shared
        // tail does nothing for type 4.
        if bag(m, "BagAccepte", a + 12, 1, u32::from(s))?.eax as u8 != 0 {
            let n = m.w(a + 0x78)?;
            if n < 5 {
                m.sw(a + 0x78, n + 1)?;
            }
        } else {
            crate::chain_recruit::retire_colonist(m, u32::from(s))?;
            let cell = m.w(a + 2)?.wrapping_add(0x200) & 65535;
            crate::chain_cargo::deposit(m, cell as u16, 1)?;
        }
        return Ok(());
    }
    let cave = 0x4dfb2c + 6 * u32::from(p);
    let count = m.w(cave)?;
    let excess = if count == 0 {
        u32::from(quantity)
    } else {
        let base = 0x4e2ea8 + 32 * u32::from(p);
        let food = m.w(base + 10)?.wrapping_sub(m.w(base + 8)?) & 65535;
        let stone = m.w(base + 18)?.wrapping_sub(m.w(base + 16)?) & 65535;
        let stock = (food + stone) & 65535;
        let capacity = (count * m.w(cave + 2)?) & 65535;
        let (mut extra, mut amount) = (0, u32::from(quantity));
        if amount + stock > capacity {
            extra = amount.wrapping_add(stock).wrapping_sub(capacity);
            amount = capacity.wrapping_sub(stock);
        }
        let total = 0x4e2eaa + 32 * u32::from(p) + 8 * u32::from(resource);
        let sum = m.w(total)?.wrapping_add(amount & 65535) & 65535;
        m.sw(total, sum)?;
        if sum > 9999 {
            m.sw(total, 9999)?;
        }
        let per = m.w(cave + 2)?;
        if per == 0 {
            return stop("BatimentAjouteMatiere capacity division", 0x423501);
        }
        m.sb(
            cave + 4,
            (((stock.wrapping_add(amount)) & 65535) / count) * 7 / per,
        )?;
        extra & 65535
    };
    if excess != 0 && matches!(t, 2 | 3) {
        // 0x423575/0x423580/0x42359d: TerrainDeposeSac 0x4761a9 drops the
        // excess one sack at a time (SacDepot) two rows and two cells beyond a
        // cave for resource 1, three cells right of a cave otherwise, or three
        // rows and three cells beyond a sanctuary.
        let offset = match (t, resource) {
            (3, 1) => 0x202,
            (3, _) => 3,
            _ => 0x303,
        };
        let cell = m.w(a + 2)?.wrapping_add(offset) & 65535;
        for _ in 0..excess {
            crate::chain_cargo::deposit(m, cell as u16, resource)?;
        }
    }
    Ok(())
}
pub fn lodge(m: &mut MemoriaJogo, s: u16, u: u16) -> Result<bool> {
    let a = structure(u32::from(s));
    let rec = unit(m, u32::from(u))?;
    let kind = m.b(rec + 1)?;
    let t = m.b(a + 8)?;
    if kind == 10 && matches!(t, 2 | 16) && m.b(a + 4)? != m.b(rec)? {
        // 0x41f7ab: a type-10 unit entering another player's type-2 structure
        // returns 1 at once (0x41f901: no bag, no stock, no ownership change);
        // a type-16 market instead opens the trade (market::lodge).
        if t == 16 {
            return crate::market::lodge(m, s, u);
        }
        return Ok(true);
    }
    let b = a + 12;
    let occupied = if matches!(t, 2 | 21) {
        let troop = bag(m, "BagContientTroupe", b, 0, 0)?.eax & 255 != 0;
        troop || (t == 2 && bag(m, "BagContient", b, 1, 9)?.eax & 65535 != 0)
    } else {
        bag(m, "BagContient", b, 1, 9)?.eax & 65535 != 0
    };
    if occupied && m.b(rec)? != m.b(a + 4)? {
        return Ok(false);
    }
    if place_free(
        m,
        R {
            eax: u32::from(s),
            edx: u32::from(u),
            ..Default::default()
        },
    )?
    .eax & 255
        == 0
    {
        return Ok(false);
    }
    if kind != 9 {
        for resource in [2, 1] {
            while bag(m, "BagContient", rec + 10, 2, resource)?.eax & 65535 != 0 {
                credit(m, resource as u8, 1, m.b(rec)? as u8, s)?;
                bag(m, "BagRetire", rec + 10, 2, resource)?;
            }
        }
    }
    if m.b(a + 4)? == 200 {
        m.sb(a + 4, m.b(rec)?)?;
        crate::chain_habitation::habitable(m, u32::from(s))?;
        crate::chain_vision::structure_minimap(m, s, 1)?;
        crate::chain_habitation::own(
            m,
            "InitBatimentPlayer",
            R {
                eax: u32::from(s),
                edx: m.b(rec)?,
                ..Default::default()
            },
        )?;
        if m.b(a + 4)? == m.b(0x508041)? {
            let volume = crate::player_orders::volume(m, m.w(a + 2)?)?;
            if m.d(a + 0x100)? == u32::MAX {
                player::sound(
                    m,
                    0x41faeb,
                    true,
                    [0x42, (m.b(a + 8)? << 16) | 0x2700, a + 0x100, 0],
                    Some(volume),
                )?;
            }
        }
    }
    bag(m, "BagPrend", b, 1, u32::from(u))?;
    Ok(true)
}
/// BatimentRepareEngin 0x425516: the unit is taken in for repair; the repair
/// cost per step is half the unit's first resource over 10,000 of progress.
fn repair_start(m: &mut MemoriaJogo, s: u16, u: u16) -> Result<()> {
    let a = structure(u32::from(s));
    let r = unit(m, u32::from(u))?;
    let kind = m.b(r + 1)?;
    m.sb(a + 5, 4)?;
    m.sb(a + 0x7a, 1)?;
    m.sb(a + 0x74, 1)?;
    m.sw(a + 0x7e, 0)?;
    m.sw(a + 0x80, 0)?;
    m.sw(a + 0x82, 0)?;
    m.sb(a + 0x7b, kind)?;
    m.sw(a + 0x88, u32::from(u))?;
    m.sd(a + 0x8c, m.w(r + 0x9a)? * 100)?;
    let cost = (m.w(0x4965b0 + 50 * kind)? >> 1) * m.d(a + 0x8c)? / 10000;
    m.sw(0x4dfb5c + 2 * m.b(a + 4)?, cost)?;
    Ok(())
}
fn deselect_unit(m: &mut MemoriaJogo, u: u16, site: u32) -> Result<()> {
    let v = u32::from(u);
    crate::player_orders::call(m, "DeselectionEngin", [v, v, v, 0], site)?;
    Ok(())
}
/// CalculEntreBatiment 0x449c56 for kind 16 (and the common tail 0x449e2e):
/// the flying unit goes to the cell right of the building, faces it, descends
/// (+0xab down to 1) and starts its repair; with no building or nothing to
/// repair it climbs back to layer 0x32 and returns to idle.
fn enter_flying(m: &mut MemoriaJogo, u: u16, rec: u32) -> Result<()> {
    let target = m.w(rec + 0xc0)?;
    let at_site = if target == 0xffff {
        false
    } else {
        let cell = crate::frame_core::building_cell(m, target)?.wrapping_add(1) & 0xffff;
        if m.w(rec + 0x7e)? != m.w(rec + 8)? {
            crate::frame_movement::displace(m, u32::from(u))?;
            return Ok(());
        }
        if m.w(rec + 0x7e)? != cell {
            if crate::chain_search::passable_layer(m, u32::from(u), cell, 2)? {
                m.sw(rec + 0x7e, cell)?;
                crate::frame_spawn::release(m, u32::from(u))?;
                m.sb(rec + 0x88, 0)?;
            }
            return Ok(());
        }
        let full =
            m.b(rec + 0xda)? == 5 && m.w(rec + 0x9a)? == m.w(0x4965b6 + 50 * m.b(rec + 1)?)?;
        !full && m.b(structure(target) + 4)? == m.b(rec)?
    };
    if at_site {
        m.sb(rec + 0x90, 5)?;
        crate::frame_movement::call(m, "RotateEngin", &[u32::from(u)])?;
        if m.b(rec + 0x90)? != m.b(rec + 0x91)? {
            return Ok(());
        }
        deselect_unit(m, u, 0x449d35)?;
        let layer = m.b(rec + 0xab)?;
        if m.b(0x508f7c)? & 1 == 0 && layer != 1 {
            m.sb(rec + 0xab, layer.wrapping_sub(1) & 255)?;
        }
        if m.b(rec + 0xab)? == 1 && m.b(structure(target) + 0x74)? == 0 {
            repair_start(m, target as u16, u)?;
        }
        return Ok(());
    }
    deselect_unit(m, u, 0x449e35)?;
    let layer = m.b(rec + 0xab)?;
    if layer == 0x32 {
        if player::alive(m, u32::from(u))? {
            m.sb(rec + 2, 7)?;
            crate::frame_spawn::release(m, u32::from(u))?;
        }
    } else if m.b(0x508f7c)? & 1 == 0 {
        m.sb(rec + 0xab, (layer + 1) & 255)?;
    }
    Ok(())
}
/// CalculEntreBatiment 0x44a12f, kinds 11..=15 and 17..=24: drive to +0xb2,
/// face the door, open it and go in for repair.
fn enter_vehicle(m: &mut MemoriaJogo, u: u16, rec: u32) -> Result<()> {
    let s = m.w(rec + 0xc0)?;
    if s == 0xffff {
        return Ok(());
    }
    let (pos, dest) = (m.w(rec + 8)?, m.w(rec + 0x7e)?);
    if pos != dest {
        crate::frame_movement::displace(m, u32::from(u))?;
        return Ok(());
    }
    let site = m.w(rec + 0xb2)?;
    if dest != site {
        if crate::chain_search::passable(m, u32::from(u), site)? {
            m.sw(rec + 0x7e, site)?;
            crate::frame_spawn::release(m, u32::from(u))?;
            m.sb(rec + 0x88, 0)?;
        }
        return Ok(());
    }
    m.sb(rec + 0x90, 5)?;
    crate::frame_movement::call(m, "RotateEngin", &[u32::from(u)])?;
    if m.b(rec + 0x90)? != m.b(rec + 0x91)? {
        return Ok(());
    }
    let a = structure(s);
    m.sb(a + 0x75, 1)?;
    if m.b(a + 0x76)? != 7 || m.b(a + 0x75)? == 0 {
        return Ok(());
    }
    repair_start(m, s as u16, u)?;
    retire(m, u)?;
    let cell = crate::frame_core::building_cell(m, s)? & 0xffff;
    m.sw(rec + 0x7c, cell)?;
    m.sw(rec + 8, cell)?;
    m.sb(rec + 2, 0xe)?;
    deselect_unit(m, u, 0x44a1e9)
}
pub fn calculate(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let rec = unit(m, u32::from(u))?;
    let t = m.b(rec + 1)?;
    if t == 16 {
        return enter_flying(m, u, rec);
    }
    if t > 10 && t != 25 {
        return enter_vehicle(m, u, rec);
    }
    let s = m.w(rec + 0xc0)? as u16;
    if s == 65535 {
        return Ok(());
    }
    let a = structure(u32::from(s));
    if m.b(a + 8)? == 8 && m.b(a + 4)? == m.b(rec)? {
        let c = entry_cell(m, s, u)?;
        m.sw(rec + 0xb2, c)?;
        m.sw(rec + 0x7e, c)?;
        crate::frame_spawn::release(m, u32::from(u))?;
        m.sb(rec + 0x88, 0)?;
        if alive(m, u32::from(u))? {
            m.sb(rec + 2, 21)?;
            crate::frame_spawn::release(m, u32::from(u))?;
        }
        return Ok(());
    }
    let near =
        m.b(rec + 0x86)? == 0 && m.b(rec + 0x87)? == 0 && adjacent(m, s, m.w(rec + 8)? as u16)?;
    if !near {
        if m.w(rec + 0x7e)? == m.w(rec + 8)? {
            let c = crate::frame_core::building_cell(m, u32::from(s))?;
            m.sw(rec + 0x7e, c)?;
            crate::frame_spawn::release(m, u32::from(u))?;
            m.sb(rec + 0x88, 0)?;
        }
        crate::frame_movement::displace(m, u32::from(u))?;
        return Ok(());
    }
    if !lodge(m, s, u)? {
        if alive(m, u32::from(u))? {
            m.sb(rec + 2, 7)?;
            crate::frame_spawn::release(m, u32::from(u))?;
        }
        return Ok(());
    }
    let c = crate::frame_core::building_cell(m, u32::from(s))?;
    retire(m, u)?;
    let kind = m.b(rec + 1)?;
    let structure_kind = m.b(a + 8)?;
    if kind == 10 && (structure_kind == 16 || (structure_kind == 2 && m.b(a + 4)? != m.b(rec)?)) {
        // 0x4499d4 (market) / 0x449aab (another player's sanctuary, type 2): it first
        // gets PlayerInitPropositionAlliance(unit owner, structure owner); then
        // +0x7c = +8 = the structure cell, DeselectionEngin, iagUnitSendMessage
        // 0x44 (EBX 0), and a unit outside states 0/4 drops to state 0 and
        // loses the current selection when it is the selected unit.
        if structure_kind == 2 {
            propose_alliance(m, m.b(rec)? as u8, m.b(a + 4)? as u8)?;
        }
        m.sw(rec + 0x7c, c)?;
        m.sw(rec + 8, c)?;
        let site = if structure_kind == 2 {
            0x449af6
        } else {
            0x4499ec
        };
        crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0x44, 0, 0], site)?;
        crate::frame_ai::call(m, "iagUnitSendMessage", &[u32::from(u), 0x44, 0])?;
        if !matches!(m.b(rec + 2)?, 0 | 4) {
            m.sb(rec + 2, 0)?;
            if m.w(0x496304)? == u32::from(u) {
                m.sb(0x4e0f36, 0)?;
                m.sb(0x4e0f37, 0)?;
                m.sd(0x4e0f3c, 0)?;
                m.sw(0x496304, 0xffff)?;
            }
        }
        return Ok(());
    }
    if kind != 9 && !matches!(structure_kind, 19 | 21) {
        crate::frame_ai::call(m, "iagUnitSendMessage", &[u32::from(u), 0x47, 0])?;
        reset_type(m, u)?;
    }
    m.sw(rec + 0x7c, c)?;
    m.sw(rec + 8, c)?;
    crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0, 0, 0], 0x449be6)?;
    m.sb(rec + 2, 14)
}
/// PlayerInitPropositionAlliance 0x46dadb (a = proposer, b = receiver).
/// Allied (PlayerEstAllie != 0): PlayerResetAllie(a, b) then (b, a), each
/// with its local-player MessageBoxSet(10, other + 0x192). Otherwise by
/// PlayerGetType(b): 1 (human) on another machine does nothing, the local
/// human gets the proposal dialog (MessageBoxSet(0x21, 0x25d), counter 0x320 at
/// 0x5080b0 + 16 * b + 4 * a, InitGameZone(1)); any
/// other type runs IaPropositionAlliance 0x47fc1b: for type 2, iaProbPost(b,
/// 0x28, EBX a, ECX 0, four zero stack words).
pub fn propose_alliance(m: &mut MemoriaJogo, a: u8, b: u8) -> Result<()> {
    let allied =
        crate::frame_economy::call(m, "PlayerEstAllie", &[u32::from(a), u32::from(b)])? & 255;
    if allied != 0 {
        crate::messagebox_state::reset_allie(m, a, b)?;
        crate::messagebox_state::reset_allie(m, b, a)?;
        return Ok(());
    }
    let kind = crate::frame_economy::call(m, "PlayerGetType", &[u32::from(b)])? & 255;
    if kind == 1 {
        if u32::from(b) == m.b(0x508041)? {
            // 0x46dc23: the local human's proposal message box, an 800-frame
            // answer counter (PlayerInitAllianceCompteur) and the game zones.
            crate::messagebox_state::enqueue(m, 0x21, 0x25d)?;
            m.sd(0x5080b0 + 16 * u32::from(b) + 4 * u32::from(a), 0x320)?;
            crate::frame_core::call(m, "InitGameZone", &[1, 0])?;
        }
        return Ok(());
    }
    if kind == 2 {
        crate::frame_tasks::post(m, &[u32::from(b), 0x28, u32::from(a), 0, 0, 0, 0, 0])?;
    }
    Ok(())
}
/// CalculEntreEngin 0x44a2ed (EnginNextEtape state 13 with a unit target at
/// +0xb8, reached from 0x4559c6 when the target is alive and has the unit's
/// owner or is neutral 200). Kinds 0..=10 and 25 only. Not adjacent
/// (EnginAdjacent 0x4468e4): head for the target's cell when idle, then
/// DeplaceEngin. Adjacent: EnginLogeEngin 0x447694 into a live target, then
/// RetireEngin, the target's cell for +0x7c/+0x7e/+8, state 14 and
/// DeselectionEngin; refused: state 7 when alive.
pub fn calculate_unit(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    let rec = unit(m, u32::from(u))?;
    let kind = m.b(rec + 1)?;
    if kind > 10 && kind != 25 {
        return Ok(());
    }
    let target = m.w(rec + 0xb8)? as u16;
    if !crate::chain_battle::adjacent_units(m, rec, target)? {
        if m.w(rec + 0x7e)? == m.w(rec + 8)? {
            let c = m.w(unit(m, u32::from(target))? + 0x7c)?;
            m.sw(rec + 0x7e, c)?;
            crate::frame_spawn::release(m, u32::from(u))?;
            m.sb(rec + 0x88, 0)?;
        }
        crate::frame_movement::displace(m, u32::from(u))?;
        return Ok(());
    }
    if alive(m, u32::from(target))? && crate::spell_remaining::lodge_unit(m, target, u)? {
        retire(m, u)?;
        let c = m.w(unit(m, u32::from(target))? + 0x7c)?;
        m.sw(rec + 0x7c, c)?;
        m.sw(rec + 0x7e, c)?;
        m.sw(rec + 8, c)?;
        m.sb(rec + 2, 14)?;
        crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0, 0, 0], 0x44a460)?;
        return Ok(());
    }
    if alive(m, u32::from(u))? {
        m.sb(rec + 2, 7)?;
        crate::frame_spawn::release(m, u32::from(u))?;
    }
    Ok(())
}
/// State13 consumes the dispatcher's record pointer, without demanding an
/// unneeded pool pointer on its no-target prefix.
/// EnginNextEtape state 21 (0x4558b4): going to park at a hangar. A structure
/// being destroyed sends the unit on to its stored destination.
pub fn state21(m: &mut MemoriaJogo, u: u16, rec: u32) -> Result<()> {
    let s = m.w(rec + 0xc0)? as u16;
    if s == 65535 {
        return Ok(());
    }
    if m.b(structure(u32::from(s)) + 5)? == 3 {
        crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(u), m.w(rec + 0x7e)?])?;
        return Ok(());
    }
    calcul_gare(m, u, rec, s)
}

/// BatimentStockEngin 0x424f5c: a free hangar accepts the unit and starts
/// the entry machine; AL = 1 when accepted.
fn stock_unit(m: &mut MemoriaJogo, s: u16, u: u16) -> Result<bool> {
    let a = structure(u32::from(s));
    let count = m.w(a + 12)?;
    let room = m.b(0x495be9)? / 2 + 1;
    if count >= room || m.b(a + 0x74)? != 0 || !matches!(m.w(a + 0x88)?, 0 | 65535) {
        return Ok(false);
    }
    m.sb(a + 5, 4)?;
    m.sb(a + 0x74, 1)?;
    m.sw(a + 0x7e, 0)?;
    m.sw(a + 0x80, 0)?;
    m.sw(a + 0x82, 0)?;
    m.sw(a + 0x88, u32::from(u))?;
    m.sb(a + 0x7b, m.b(unit(m, u32::from(u))? + 1)?)?;
    Ok(true)
}

/// CalculGareBatiment 0x449530: walk to the entry cell, turn to direction 5,
/// open the door, then hand over to the hangar or stand idle.
fn calcul_gare(m: &mut MemoriaJogo, u: u16, rec: u32, s: u16) -> Result<()> {
    if m.w(rec + 8)? != m.w(rec + 0x7e)? {
        crate::frame_movement::call(m, "DeplaceEngin", &[u32::from(u)])?;
        return Ok(());
    }
    let last = m.w(rec + 0xb2)?;
    if m.w(rec + 0x7e)? != last {
        if crate::chain_search::passable(m, u32::from(u), last)? {
            m.sw(rec + 0x7e, last)?;
            crate::frame_spawn::release(m, u32::from(u))?;
            m.sb(rec + 0x88, 0)?;
        }
        return Ok(());
    }
    m.sb(rec + 0x90, 5)?;
    crate::frame_movement::call(m, "RotateEngin", &[u32::from(u)])?;
    if m.b(rec + 0x90)? != m.b(rec + 0x91)? {
        return Ok(());
    }
    let a = structure(u32::from(s));
    if m.b(a + 0x74)? != 0 {
        return Ok(());
    }
    m.sb(a + 0x75, 1)?;
    if m.b(a + 0x76)? != 7 || m.b(a + 0x75)? == 0 {
        return Ok(());
    }
    if stock_unit(m, s, u)? {
        retire(m, u)?;
        let cell = crate::frame_core::call(m, "BatimentGetCase", &[u32::from(s)])? & 65535;
        m.sw(rec + 0x7c, cell)?;
        m.sw(rec + 8, cell)?;
        m.sb(rec + 2, 0xe)?;
        crate::player_orders::call(m, "DeselectionEngin", [u32::from(u), 0, 0, 0], 0x449627)?;
    } else if alive(m, u32::from(u))? {
        m.sb(rec + 2, 7)?;
        crate::frame_spawn::release(m, u32::from(u))?;
    }
    Ok(())
}

pub fn state13(m: &mut MemoriaJogo, u: u16, rec: u32) -> Result<()> {
    let s = m.w(rec + 0xc0)? as u16;
    if s == 65535 {
        let target = m.w(rec + 0xb8)? as u16;
        if target == 65535 {
            return Ok(());
        }
        // 0x4559c6: a live target with the unit's owner, or neutral 200.
        let t = unit(m, u32::from(target))?;
        if alive(m, u32::from(target))? && (m.b(t)? == m.b(rec)? || m.b(t)? == 200) {
            return calculate_unit(m, u);
        }
        // 0x455492 -> 0x4567b7: back to the stored destination, target cleared.
        crate::frame_spawn::call(m, "SetDeplaceEngin", &[u32::from(u), m.w(rec + 0x7e)?])?;
        m.sw(rec + 0xb8, 65535)?;
        return Ok(());
    }
    let a = structure(u32::from(s));
    let mut refused = m.b(a + 5)? == 3;
    if !refused {
        let (p, q) = (m.b(a + 4)?, m.b(rec)?);
        refused = if m.b(rec + 1)? == 10 {
            p == 200 || (p == q && m.b(a + 8)? == 16)
        } else {
            p != q && !(p == 200 && m.b(rec + 1)? == 9)
        };
    }
    if refused {
        crate::frame_spawn::order(m, u32::from(u), m.w(rec + 0x7e)? as u16)?;
        m.sw(rec + 0xc0, 65535)?;
        Ok(())
    } else {
        calculate(m, u)
    }
}
pub fn select_enter(m: &mut MemoriaJogo, p: u8, s: u16) -> Result<()> {
    let t = m.b(structure(u32::from(s)) + 8)?;
    let base = 0x508728 + (u32::from(p) << 9);
    let (mut i, mut spoke) = (0, false);
    while i < m.w(base)? {
        let u = m.w(base + 2 + 2 * i)?;
        i += 1;
        if !alive(m, u)? {
            continue;
        }
        let k = m.b(unit(m, u)? + 1)?;
        set_enter(m, u as u16, s)?;
        if spoke {
            continue;
        }
        spoke = true;
        if u32::from(p) != m.b(0x508041)? {
            continue;
        }
        let mode = match t {
            1 | 10 | 13 => Some(14),
            2 if k != 10 => Some(10),
            16 if k == 9 => Some(25),
            16 if k == 10 => Some(36),
            _ => None,
        };
        if let Some(mode) = mode {
            crate::player_orders::ask(m, u, mode, 0, u)?;
        }
    }
    Ok(())
}

/// EnginSelectPeutEntrer's semantic AL, original0x472b6c.
pub fn can_enter(m: &MemoriaJogo, p: u32, mode: u32, target: u32) -> Result<bool> {
    let base = 0x508728 + (p << 9);
    let mut kinds = Vec::new();
    let mut records = Vec::new();
    for i in 0..m.w(base)? {
        let u = m.w(base + 2 + 2 * i)?;
        if !alive(m, u)? {
            continue;
        }
        let a = unit(m, u)?;
        if m.b(a + 0xdc)? & 4 != 0 {
            return Ok(false);
        }
        kinds.push(m.b(a + 1)?);
        records.push(a);
    }
    let all = |test: fn(u32) -> bool| !kinds.is_empty() && kinds.iter().copied().all(test);
    let troop = |t| !matches!(t, 3 | 11..=17 | 25);
    let engine = |t| matches!(t, 3 | 11..=17);
    if mode == 1 {
        let t = m.b(unit(m, target)? + 1)?;
        return Ok(t != 16 && engine(t) && all(troop) && (t != 17 || !kinds.contains(&8)));
    }
    if mode != 3 {
        return Ok(false);
    }
    let s = structure(target);
    let t = m.b(s + 8)?;
    let owner = m.b(s + 4)?;
    if (owner == p || owner == 200) && all(|t| t == 9) && t != 9 {
        return Ok(true);
    }
    if owner != p {
        return Ok(t == 16 && all(|t| t == 10));
    }
    if t == 21 && kinds.iter().all(|v| !matches!(v, 3 | 11..=17)) {
        return Ok(true);
    }
    let mut injured = false;
    for a in &records {
        if m.w(*a + 0x9a)? != m.w(0x4965b6 + 50 * m.b(*a + 1)?)? {
            injured = true;
        }
    }
    if all(troop) && (t == 2 || (t == 19 && injured)) {
        return Ok(true);
    }
    if t == 14 && all(engine) {
        if injured {
            return Ok(true);
        }
        for a in &records {
            if m.b(*a + 0xda)? != 5 {
                return Ok(true);
            }
        }
    }
    if t == 18 && all(|t| t == 16) {
        return Ok(true);
    }
    Ok(t == 8)
}
