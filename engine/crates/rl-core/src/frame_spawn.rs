//! Live Core raid creation, placement, queries and initial movement orders.
use crate::{frame_core, frame_economy, EngineError, MemoriaJogo, Result};
fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
pub fn unit(m: &MemoriaJogo, i: u32) -> Result<u32> {
    frame_core::unit(m, i)
}
pub fn release(m: &mut MemoriaJogo, i: u32) -> Result<()> {
    if m.u16(0x496304)? == i as u16 {
        m.set_u8(0x4e0f36, 0)?;
        m.set_u8(0x4e0f37, 0)?;
        m.set_u32(0x4e0f3c, 0)?;
        m.set_u16(0x496304, 0xffff)?;
    }
    Ok(())
}
pub fn region(m: &MemoriaJogo, c: u16) -> Result<u16> {
    let r = m.u16(m.u32(0x4e0f38)?.wrapping_add(2 * u32::from(c)))?;
    Ok(if r & 0x8000 != 0 {
        m.u16(0x49a194 + 276 * u32::from((r & 0x7fe0) >> 5) + 2 * u32::from(r & 31))?
    } else if r & 0x4000 != 0 {
        m.u16(unit(m, u32::from((r & 0x3ff8) >> 3))? + 0xfa + 2 * u32::from(r & 3))?
    } else {
        r
    })
}
pub fn accessible(m: &MemoriaJogo, a: u16, b: u16) -> Result<bool> {
    let w = m.u16(0x497168)?;
    let h = m.u16(0x49716a)?;
    Ok((a & 255) < w
        && (b & 255) < w
        && (a >> 8) < h
        && (b >> 8) < h
        && region(m, a)? == region(m, b)?)
}
pub fn reset_order(m: &mut MemoriaJogo, i: u32) -> Result<()> {
    let u = unit(m, i)?;
    let t = m.u8(u + 1)?;
    if m.u8(u + 0x86)? == 0 && m.u8(u + 0x87)? == 0 && t != 16 {
        m.set_u8(u + 0xa9, 0)?;
        m.set_u8(u + 0xa4, 0)?;
    }
    if t == 13 && m.u16(u + 0xc0)? != 65535 {
        let structure = m.u16(u + 0xc0)?;
        let s = 0x49a0c8 + 276 * u32::from(structure);
        m.set_u8(s + 0x74, 0)?;
        if m.u8(s + 0x76)? == 15 && m.u8(s + 0x75)? != 0 {
            m.set_u8(s + 0x75, 0)?;
        }
    }
    m.set_u16(u + 0x7e, m.u16(u + 0x7c)?)?;
    release(m, i)?;
    m.set_u8(u + 0x88, 0)?;
    m.set_u8(u + 0x82, 0)?;
    m.set_u16(u + 0xb2, 65535)?;
    m.set_u8(u + 2, 2)?;
    Ok(())
}
pub fn order(m: &mut MemoriaJogo, i: u32, dest: u16) -> Result<u32> {
    reset_order(m, i)?;
    let u = unit(m, i)?;
    m.set_u8(u + 0x88, 0)?;
    release(m, i)?;
    if m.u8(u + 1)? == 6
        && matches!(
            m.u8(m.u32(0x507d74)?.wrapping_add(u32::from(dest)))?,
            0x36 | 0x42
        )
    {
        reset_order(m, i)?;
        m.set_u8(u + 2, 5)?;
        m.set_u8(u + 0x89, 1)?;
        m.set_u16(u + 0x7e, dest)?;
        release(m, i)?;
        m.set_u8(u + 0x88, 0)?;
        m.set_u16(u + 0xb2, dest)?;
        return Ok(u);
    }
    m.set_u16(u + 0x7e, dest)?;
    release(m, i)?;
    m.set_u8(u + 0x88, 0)?;
    m.set_u8(u + 2, 2)?;
    m.set_u16(u + 0xb8, 65535)?;
    m.set_u16(u + 0xb2, 65535)?;
    m.set_u8(u + 0xbe, 255)?;
    m.set_u16(u + 0xc0, 65535)?;
    Ok(u)
}
pub fn en_arret(m: &mut MemoriaJogo, i: u32) -> Result<u32> {
    let u = unit(m, i)?;
    let t = m.u8(u + 1)?;
    if t > 25 {
        return Ok(u & 0xffffff00);
    }
    let frame = match t {
        0 | 6 => Some(2),
        2 | 3 | 25 => Some(7),
        1 | 4 | 7 | 18 | 19 | 23 | 24 => Some(3),
        10 | 16 => Some(6),
        5 | 9 => Some(5),
        8 | 21 | 22 => Some(0),
        20 => Some(1),
        _ => None,
    };
    if let Some(f) = frame {
        m.set_u8(u + 0xa4, f)?;
    }
    Ok((u & 0xffffff00) | 1)
}
fn capacity(t: u8) -> u16 {
    match t {
        5 | 6 | 7 | 11 | 14 | 16 => 1,
        12 => 2,
        13 => 5,
        15 => 3,
        17 => 6,
        _ => 0,
    }
}
fn characteristics(m: &mut MemoriaJogo, i: u32) -> Result<u32> {
    let u = unit(m, i)?;
    let t = m.u8(u + 1)?;
    m.set_u8(u + 0xbc, 255)?;
    m.set_u8(u + 0xdb, 8)?;
    m.set_u8(u + 0xda, 1)?;
    m.set_u8(u + 0xf9, 0)?;
    m.set_u16(
        u + 0x102,
        u16::from(m.u8(0x4965ae + 50 * u32::from(t) + 0x30)?),
    )?;
    if t > 25 {
        return Ok((m.u32(0x4e19a0)? & 0xffffff00) | u32::from(t));
    }
    for (o, v) in [
        0x93, 0x98, 0xa7, 0xa8, 0xa5, 0xa6, 0xdb, 0x9d, 0xac, 0xad, 0x99,
    ]
    .into_iter()
    .zip(PARAMETERS[t as usize])
    {
        if v >= 0 {
            m.set_u8(u + o, v as u8)?;
        }
    }
    let projectile = match t {
        1 => Some(25),
        2 | 15 => Some(1),
        3 | 25 => Some(26),
        8 => Some(9),
        11 => Some(27),
        12 => Some(22),
        18 => Some(17),
        21 => Some(24),
        22 | 23 => Some(23),
        _ => None,
    };
    if let Some(p) = projectile {
        m.set_u8(u + 0x78, 1)?;
        m.set_u8(u + 0x79, p)?;
    } else if matches!(t, 19 | 20 | 24) {
        m.set_u8(u + 0x78, 0)?;
    }
    let f = match t {
        0 | 8 | 21 | 22 => Some(6),
        1 | 11 | 25 => Some(1),
        2 => Some(7),
        3 => Some(4),
        4 | 23 => Some(5),
        12 | 15 => Some(0),
        18 => Some(2),
        _ => None,
    };
    if t == 4 {
        let b = frame_economy::bonus(m, u32::from(m.u8(u)?), 4, 12, 0)?;
        m.set_u8(u + 0x7a, 5)?;
        m.set_u8(u + 0xda, b.wrapping_add(10) as u8)?;
        return Ok((b & 0xffffff00) | u32::from((b as u8).wrapping_add(10)));
    }
    if let Some(f) = f {
        m.set_u8(u + 0x7a, f)?;
    }
    if t == 6 {
        m.set_u8(u + 0xbb, 5)?;
    }
    if t == 16 || t == 17 {
        m.set_u8(u + 0xab, 50)?;
        if t == 17 {
            for (o, v) in [(0x94, 0), (0x95, 1), (0x96, 0), (0xa4, 0)] {
                m.set_u8(u + o, v)?;
            }
        }
    }
    Ok(u32::from(t))
}
fn set_type(m: &mut MemoriaJogo, i: u32, cell: u16, d: u8, t: u8) -> Result<u32> {
    let u = unit(m, i)?;
    for o in [8, 0x7c, 0x7e] {
        m.set_u16(u + o, cell)?;
    }
    release(m, i)?;
    for o in [
        0x88, 0xab, 0x97, 0x95, 0x96, 0x92, 0x86, 0x87, 0xb6, 0xaa, 0xa4, 0xa9, 0x9e, 0x8e, 0x8b,
        0xbc, 0xbd,
    ] {
        m.set_u8(u + o, 0)?;
    }
    m.set_u16(u + 0xb2, 65535)?;
    m.set_u16(u + 0xb8, 65535)?;
    m.set_u16(u + 0x8c, 0)?;
    for (o, v) in [(0x9c, 99), (0x90, d), (0x91, d), (0x94, d), (1, t)] {
        m.set_u8(u + o, v)?;
    }
    if !matches!(m.u8(u + 2)?, 0 | 4) {
        m.set_u8(u + 2, 7)?;
        release(m, i)?;
    }
    for (o, v) in [(4, 4), (0x74, 0), (0x76, 4), (0xc2, 0)] {
        m.set_u16(u + o, v)?;
    }
    m.set_u8(u + 6, 4)?;
    m.set_u8(u + 0x78, 0)?;
    m.set_u8(u + 0xbb, m.u8(0x4965ae + 50 * u32::from(t) + 0x2a)?)?;
    m.set_u8(u + 0xb0, m.u8(0x4965ae + 50 * u32::from(t) + 0x2c)?)?;
    characteristics(m, i)?;
    let cap = u32::from(capacity(t)).wrapping_add(frame_economy::bonus(
        m,
        u32::from(m.u8(u)?),
        u32::from(t),
        1,
        0,
    )?);
    for o in [0xa, 0xc, 0xe, 0x70] {
        m.set_u16(u + o, 0)?;
    }
    for j in 0..30 {
        m.set_u16(u + 0x10 + 2 * j, 0)?;
        m.set_u8(u + 0x4c + j, 0)?;
    }
    m.set_u16(u + 0xc, cap as u16)?;
    Ok(cap)
}
fn create(m: &mut MemoriaJogo, cell: u16, d: u8, t: u8, owner: u8) -> Result<u32> {
    let mut i = 0;
    while i < 800 && m.u8(unit(m, i)? + 2)? != 0 {
        i += 1;
    }
    if i == 800 {
        return Ok(65535);
    }
    let u = unit(m, i)?;
    m.set_u8(
        u,
        if t.wrapping_sub(11) <= 6 && !matches!(t, 16 | 17) {
            200
        } else {
            owner
        },
    )?;
    set_type(m, i, cell, d, t)?;
    for (o, v) in [
        (0xdc, 0),
        (2, 7),
        (0x72, 0),
        (0xbe, 255),
        (0xbc, 255),
        (0xbd, 0),
        (0xaf, 0),
        (0xf5, 0),
        (0xf6, 0),
        (0xf7, 0),
        (0xf8, 0),
    ] {
        m.set_u8(u + o, v)?;
    }
    for (o, v) in [(0x9a, 100), (0x74, 0), (0xc0, 65535)] {
        m.set_u16(u + o, v)?;
    }
    for (o, v) in [(0x104, u32::MAX), (0x10c, u32::MAX), (0x110, 0)] {
        m.set_u32(u + o, v)?;
    }
    m.set_u8(u + 0x94, 1)?;
    m.set_u16(u + 4, m.u16(u + 0x76)?)?;
    for j in 0..u32::from(m.u16(u + 0xc)?) {
        m.set_u8(u + 0x4c + j, 0)?;
        m.set_u16(u + 0x10 + 2 * j, 0)?;
    }
    en_arret(m, i)?;
    let t = m.u8(u + 1)?;
    if t == 4 {
        let b = frame_economy::bonus(m, u32::from(m.u8(u)?), 4, 12, 0)?;
        m.set_u8(u + 0xda, b.wrapping_add(10) as u8)?;
    }
    if t == 14 || t == 16 {
        m.set_u8(u + 0xda, 0)?;
    }
    Ok(i)
}
pub fn minimap(m: &mut MemoriaJogo, owner: u8, cell: u16, mode: u8) -> Result<u32> {
    if matches!(mode, 1..=3 | 6 | 7) {
        return crate::player_sight::minimap(m, owner, cell, mode);
    }
    let mw = u32::from(m.u16(0x496fe0)?);
    let mh = u32::from(m.u16(0x496fe2)?);
    let w = u32::from(m.u16(0x497168)?);
    let h = u32::from(m.u16(0x49716a)?);
    if w == 0 || h == 0 {
        return Err(stop("SetMiniMap map division", 0x46a3dc));
    }
    let sx = (mw / w).max(1);
    let sy = (mh / h).max(1);
    let start = (u32::from(cell >> 8)
        .wrapping_mul(mh)
        .wrapping_div(h)
        .wrapping_mul(mh)
        .wrapping_add(u32::from(cell & 255).wrapping_mul(mw).wrapping_div(w)))
        & 65535;
    let local = u32::from(m.u8(0x508041)?);
    let lit = m.u8(m.u32(0x507d88 + 4 * local)?.wrapping_add(u32::from(cell)))? & 31 == 0;
    let seen = if mode == 5 {
        m.u8(m.u32(0x507d78 + 4 * local)?.wrapping_add(u32::from(cell)))? & 31 == 0
    } else {
        false
    };
    for y in 0..sy {
        for x in 0..sx {
            let p = start.wrapping_add(y * 128 + x) & 65535;
            let units = m.u32(0x507d60)?.wrapping_add(p);
            let finalp = m.u32(0x507d5c)?.wrapping_add(p);
            match mode {
                4 => {
                    let c = if owner == 200 {
                        30
                    } else {
                        m.u8(0x4e24dc + u32::from(owner))?
                    };
                    m.set_u8(units, c)?;
                    if lit {
                        m.set_u8(finalp, c)?;
                    }
                }
                5 => {
                    m.set_u8(units, 0)?;
                    if seen {
                        let obj = m.u8(m.u32(0x507d74)?.wrapping_add(u32::from(cell)))?;
                        let tile = m.u16(m.u32(0x507d70)?.wrapping_add(2 * u32::from(cell)))?;
                        let cor = if (0x1d..0x35).contains(&obj) || tile >= 702 {
                            30
                        } else {
                            m.u8(0x509694 + u32::from(tile))?
                        };
                        let c = if lit {
                            let structure = m.u8(m.u32(0x507d64)?.wrapping_add(p))?;
                            if structure != 0 {
                                structure
                            } else {
                                cor
                            }
                        } else {
                            m.u8(m.u32(0x4e24b4)?.wrapping_add(u32::from(cor)))?
                        };
                        m.set_u8(finalp, c)?;
                    }
                }
                _ => return Err(stop("SetMiniMap mode", 0x46a388)),
            }
        }
    }
    Ok(sy)
}
fn free_cell(m: &MemoriaJogo, center: u32, t: u32) -> Result<u32> {
    let x = (center & 255) as i32;
    let y = ((center & 65535) >> 8) as i32;
    let w = i32::from(m.u16(0x497168)?);
    let h = i32::from(m.u16(0x49716a)?);
    if x >= w || y >= h {
        return Ok(65535);
    }
    let class = m.u8(0x4962c8 + (t & 65535))?;
    let mask = if t & 65535 == 16 || t & 65535 == 17 {
        2
    } else {
        1
    };
    let free = |c: u16| -> Result<bool> {
        if !(1..=3).contains(&class) {
            return Ok(true);
        }
        if class != 1 && (c & 255 == 0 || c >> 8 == 0) {
            return Ok(false);
        }
        for d in if class == 1 {
            &[0u16][..]
        } else {
            &[0, 1, 257, 256][..]
        } {
            if m.u8(m.u32(0x507da8)?.wrapping_add(u32::from(c.wrapping_sub(*d))))? & mask != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    };
    for k in 1..30i32 {
        let corner = (center as u16)
            .wrapping_sub(k as u16)
            .wrapping_sub((k as u16) << 8);
        if y - k >= 0 {
            for j in 0..=2 * k {
                if x + j - k >= 0 && x + j - k <= w {
                    let c = corner.wrapping_add(j as u16);
                    if free(c)? {
                        return Ok(u32::from(c));
                    }
                }
            }
        }
        if y + k < h {
            for j in 0..=2 * k {
                if x + j - k >= 0 && x + j - k <= w {
                    let c = corner.wrapping_add((k as u16) << 9).wrapping_add(j as u16);
                    if free(c)? {
                        return Ok(u32::from(c));
                    }
                }
            }
        }
        for side in 0..2 {
            if (side == 0 && x - k < 0) || (side == 1 && x + k >= w) {
                continue;
            }
            let mut c = corner.wrapping_add(if side == 0 { 0 } else { 2 * k as u16 });
            for j in 0..2 * k - 1 {
                if y + 1 - k + j >= 0 && x + 1 - k + j < h && free(c)? {
                    return Ok(u32::from(c));
                }
                c = (c & 255) | ((c >> 8).wrapping_add(1) & 255) << 8;
            }
        }
    }
    Ok(65535)
}
pub fn call(m: &mut MemoriaJogo, name: &str, a: &[u32]) -> Result<u32> {
    match name {
        "PlayerGetColor" => Ok(if a[0] as u8 == 200 {
            30
        } else {
            u32::from(m.u8(0x4e24dc + u32::from(a[0] as u8))?)
        }),
        "EnginTypeGetCapacite" => Ok(u32::from(capacity(a[0] as u8))),
        "GetCaseIndiceBatiment" => Ok((((a[0] & 65535) * 69) & 0xffff0000)
            | u32::from(m.u16(0x49a0c8 + 276 * (a[0] & 65535) + 2)?)),
        "GetNumIndiceTypeBatiment" => {
            let mut i = a[0];
            while (i as u16) < 1000 {
                let u = 0x49a0c8 + 276 * u32::from(i as u16);
                if m.u8(u + 4)? == a[2] as u8 && m.u8(u + 8)? == a[1] as u8 {
                    return Ok(i);
                }
                i = i.wrapping_add(1);
            }
            Ok(65535)
        }
        "TerrainEnCollision" => Ok(u32::from(
            m.u8(m.u32(0x507da8)?.wrapping_add(a[0] & 65535))? & (a[1] as u8) != 0,
        )),
        "ResetCollision" => {
            let p = m.u32(0x507da8)?.wrapping_add(a[0] & 65535);
            m.set_u8(p, m.u8(p)? & !(a[1] as u8))?;
            Ok(a[0] & 65535)
        }
        "memset" => {
            if a[2] != 0 {
                m.u8(a[0])?;
            }
            for j in 0..a[2] {
                m.set_u8(a[0].wrapping_add(j), a[1] as u8)?;
            }
            Ok(a[0])
        }
        "GetFreeCell" => free_cell(m, a[0], a[1]),
        "CreerEngin" => create(m, a[0] as u16, a[1] as u8, a[2] as u8, a[3] as u8),
        "EnginSetType" => set_type(m, a[0], a[1] as u16, a[2] as u8, a[3] as u8),
        "EnginSetTypeCaract" => characteristics(m, a[0]),
        "EnginEnArret" => en_arret(m, a[0]),
        "EnginGetCase" => {
            Ok((m.u32(0x4e19a0)? & 0xffff0000) | u32::from(m.u16(unit(m, a[0])? + 0x7c)?))
        }
        "EnginEstVivant" => Ok(u32::from(!matches!(m.u8(unit(m, a[0])? + 2)?, 0 | 4))),
        "EnginGetType" => frame_core::call(m, name, a),
        "EnginSetPlayer" => {
            let u = unit(m, a[0])?;
            let owner = a[1] as u8;
            if m.u8(u + 1)?.wrapping_sub(11) <= 6 {
                for j in 0..u32::from(m.u16(u + 0xa)?) {
                    if m.u8(u + 0x4c + j)? == 1 {
                        m.set_u8(unit(m, u32::from(m.u16(u + 0x10 + 2 * j)?))?, owner)?;
                    }
                }
            }
            let group = m.u8(u + 1)?.wrapping_sub(11) <= 6;
            let high = if group {
                u32::from(m.u16(u + 0xa)?) & 0xffffff00
            } else {
                a[0] & 0xffffff00
            };
            m.set_u8(u, owner)?;
            Ok(high | u32::from(owner))
        }
        "PlaceEngin" => {
            let u = unit(m, a[0])?;
            frame_core::set_unit_collision(m, a[0], u32::from(m.u16(u + 8)?))?;
            m.set_u8(u + 0x72, 1)?;
            let owner = m.u8(u)?;
            let cell = m.u16(u + 8)?;
            if !matches!(owner, 4 | 200) {
                // 0x456ff8: DecouvreTerrain(cell, sight + 2) then
                // IllumineTerrain(cell, sight), sight = +0xbb + +0xf5.
                let sight = u32::from(m.u8(u + 0xbb)?) + u32::from(m.u8(u + 0xf5)?);
                crate::player_sight::area(m, cell, sight + 2, owner, 0)?;
                crate::player_sight::area(m, cell, sight, owner, 1)?;
            }
            minimap(m, owner, cell, 4)
        }
        "SetMiniMap" => minimap(m, a[0] as u8, a[2] as u16, a[1] as u8),
        "SetDeplaceEngin" => order(m, a[0], a[1] as u16),
        "ResetOrdre" => {
            reset_order(m, a[0])?;
            unit(m, a[0])
        }
        "Chemin_Accessible" | "Chemin_MemeZone" => {
            Ok(u32::from(accessible(m, a[0] as u16, a[1] as u16)?))
        }
        "GetRandCaseChamps" => {
            let base = m.u32(0x507d74)?;
            let mut cells = Vec::new();
            for x in 0..256 {
                if cells.len() > 200 {
                    break;
                }
                for y in 0..256 {
                    if cells.len() > 200 {
                        break;
                    }
                    let c = x + y * 256;
                    if m.u8(base.wrapping_add(c))? == 0x1c {
                        cells.push(c);
                    }
                }
            }
            if cells.is_empty() {
                return Ok(65535);
            }
            if cells.len() == 201 {
                cells[200] += 1;
            }
            let r = frame_core::random(m)? as usize % cells.len();
            Ok(cells[r])
        }
        "GetRandCaseTypeBatPlayer" => {
            let mut found = Vec::new();
            for i in 0..1000 {
                let u = 0x49a0c8 + 276 * i;
                if m.u8(u + 4)? == a[0] as u8 && m.u8(u + 8)? == a[1] as u8 {
                    if found.len() == 200 {
                        return Err(stop("GetRandCaseTypeBatPlayer stack capacity", 0x48933f));
                    }
                    found.push(i);
                }
            }
            if found.is_empty() {
                return Ok(65535);
            }
            let i = found[frame_core::random(m)? as usize % found.len()];
            Ok(u32::from(m.u16(0x49a0c8 + 276 * i + 2)?))
        }
        _ => Err(stop("Core raid spawn provider", 0x457e6c)),
    }
}

const PARAMETERS: [[i16; 11]; 26] = [
    [1, 2, 2, 2, 8, 10, 8, 255, 3, 3, 0],
    [1, 2, 2, 4, 8, 4, 8, 255, 3, 3, 0],
    [1, 3, 2, 3, 8, 10, 10, 255, 3, 3, 0],
    [1, 4, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 1, 3, 2, 8, 10, 10, 255, 3, 3, 0],
    [1, 2, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 3, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 2, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 2, 2, 2, 10, 10, 8, 255, 3, 3, 0],
    [1, 2, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 3, 2, 4, 8, 8, 8, 255, 3, 3, 0],
    [1, 8, 1, 1, 3, 3, -1, 100, 2, 10, 3],
    [2, 3, 2, 4, 3, 8, -1, 100, 15, 20, 2],
    [3, 3, 3, 0, 3, 8, -1, 100, 6, 10, 3],
    [4, 3, 3, 4, 3, 2, -1, 100, 0, 3, 2],
    [3, 4, 2, 2, 3, 1, -1, 100, 0, 3, 2],
    [2, 8, 2, 2, 8, 8, -1, 48, 10, 8, 0],
    [2, 1, 4, 4, 3, 4, -1, 100, 6, 10, 0],
    [1, 2, 1, 2, 10, 4, 8, 255, 3, 3, 0],
    [1, 2, 1, 2, 15, 15, 15, 255, 3, 3, 0],
    [1, 4, 2, 2, 8, 8, 8, 255, 3, 3, 0],
    [1, 8, 2, 2, 8, 8, 8, 255, 3, 3, 0],
    [1, 4, 2, 2, 8, 8, 8, 255, 3, 3, 0],
    [1, 2, 2, 2, 8, 8, 8, 255, 3, 3, 0],
    [3, 1, 3, 3, 8, 8, 8, 255, 3, 3, 0],
    [1, 4, 2, 4, 8, 8, 8, 255, 3, 3, 0],
];
