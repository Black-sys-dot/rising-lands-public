//! Mechanical frame providers from `chamadas_quadro_core.py`.
//! Widths and call order follow the original; absent memory is never synthesized.
use crate::{CrtRng, EngineError, MemoriaJogo, Result, WorldRng};

fn stop(name: &'static str, address: u32) -> EngineError {
    EngineError::UnportedCallee { name, address }
}
pub(crate) fn unit(m: &MemoriaJogo, index: u32) -> Result<u32> {
    let base = m
        .u32(0x4e19a0)?
        .wrapping_add((index & 0xffff).wrapping_mul(276));
    base.checked_add(275).ok_or(EngineError::AddressOverflow)?;
    Ok(base)
}
pub fn random(m: &mut MemoriaJogo) -> Result<u32> {
    let mut r = WorldRng {
        seed: m.u32(0x497164)?,
    };
    let out = r.draw();
    m.set_u32(0x497164, r.seed)?;
    Ok(out)
}
pub fn crt_random(m: &mut MemoriaJogo) -> Result<u32> {
    if m.u32(0x4990cc)? != 0x48e7c7 {
        return Err(stop("rand thread-data getter", 0x48c513));
    }
    let a = m.u32(0x522244)?.wrapping_add(12);
    if a == 0 {
        return Ok(0);
    }
    let mut r = CrtRng { seed: m.u32(a)? };
    let out = r.draw();
    m.set_u32(a, r.seed)?;
    Ok(out)
}
fn perm(m: &mut MemoriaJogo) -> Result<u8> {
    let c = m.u8(0x4e1d18)?;
    let b = m.u8(0x4e1c18 + u32::from(c))?;
    m.set_u8(0x4e1d18, c.wrapping_add(1))?;
    Ok(b)
}
pub fn animate_terrain(m: &mut MemoriaJogo, x: u32, y: u32) -> Result<u32> {
    let mut cell = x.wrapping_add(y.wrapping_shl(8)) as u16;
    let mut row = 0u32;
    while (row as u16) < m.u16(0x508726)? {
        let mut col = 0u16;
        while col < m.u16(0x508724)? {
            let a = m.u32(0x507d70)?.wrapping_add(u32::from(cell) * 2);
            let t = m.u16(a)?;
            if t == 0x5d || t == 0x94 {
                if perm(m)? % 0x40 == 0 {
                    m.set_u16(a, 0x95)?;
                }
            } else if (0x95..=0x9c).contains(&t) && m.u8(0x508f7c)? & 1 == 0 {
                m.set_u16(a, t.wrapping_add(1))?;
                let a = m.u32(0x507d70)?.wrapping_add(u32::from(cell) * 2);
                if m.u16(a)? == 0x9d {
                    let b = perm(m)?;
                    m.set_u16(a, if b & 1 == 0 { 0x5d } else { 0x94 })?;
                }
            } else {
                let a = m.u32(0x507d70)?.wrapping_add(u32::from(cell) * 2);
                let t = m.u16(a)?;
                if t == 0x251 || t == 0x252 {
                    if m.u8(m.u32(0x507dac)?.wrapping_add(u32::from(cell)))? != 0 {
                        m.set_u16(a, t.wrapping_add(4))?;
                    }
                } else if t == 0x255 || t == 0x256 {
                    m.set_u16(a, t.wrapping_sub(4))?;
                } else if crt_random(m)? % 0x400 == 0 {
                    let t = m.u16(m.u32(0x507d70)?.wrapping_add(u32::from(cell) * 2))?;
                    if (0x8b..0x93).contains(&t) || t == 0x6d {
                        let a = m.u32(0x507d74)?.wrapping_add(u32::from(cell));
                        if m.u8(a)? == 0 {
                            m.set_u8(a, 0x1b)?;
                        }
                    }
                }
            }
            cell = cell.wrapping_add(1);
            col = col.wrapping_add(1);
        }
        cell = cell.wrapping_add(0x100).wrapping_sub(m.u16(0x508724)?);
        row = row.wrapping_add(1);
    }
    Ok(row)
}
fn animate_object(m: &mut MemoriaJogo, cell: u16) -> Result<()> {
    let c = u32::from(cell);
    let o = m.u8(m.u32(0x507d74)?.wrapping_add(c))?;
    if o == 0 {
        return Ok(());
    }
    if (1..=0xb).contains(&o) {
        if m.u8(0x508f7c)? & 3 == 0 && perm(m)? & 3 == 0 {
            let a = m.u32(0x507dac)?.wrapping_add(c);
            m.set_u8(a, m.u8(a)?.wrapping_add(1) & 3)?;
        }
    } else if (0x17..=0x1a).contains(&o) {
        if m.u8(0x508f7c)? & 1 == 0 {
            let a = m.u32(0x507dac)?.wrapping_add(c);
            m.set_u8(a, m.u8(a)?.wrapping_add(1) & 0xf)?;
        }
    } else if o == 0x1b || o == 0x37 {
        if m.u8(0x508f7c)? & 1 == 0 {
            let a = m.u32(0x507dac)?.wrapping_add(c);
            m.set_u8(a, m.u8(a)?.wrapping_add(1))?;
        }
        let a = m.u32(0x507dac)?.wrapping_add(c);
        if m.u8(a)? == 0x10 {
            m.set_u8(m.u32(0x507d74)?.wrapping_add(c), 0)?;
            m.set_u8(a, 0)?;
        }
    } else if (0x38..=0x3d).contains(&o) {
        if m.u8(0x508f7c)? & 1 == 0 {
            let a = m.u32(0x507dac)?.wrapping_add(c);
            m.set_u8(a, m.u8(a)?.wrapping_add(1))?;
        }
        let a = m.u32(0x507dac)?.wrapping_add(c);
        if m.u8(a)? == 0x40 {
            let ob = m.u32(0x507d74)?.wrapping_add(c);
            let current = m.u8(ob)?;
            m.set_u8(ob, 0)?;
            if current != 0x3b && current != 0x38 {
                m.set_u8(a, 0)?;
                let ob = m.u32(0x507d74)?.wrapping_add(c);
                if m.u8(ob)? == 0 {
                    m.set_u8(ob, current.wrapping_sub(1))?;
                }
            }
            m.set_u8(m.u32(0x507dac)?.wrapping_add(c), 0)?;
        }
    } else if (0x3e..=0x41).contains(&o) {
        let a = m.u32(0x507dac)?.wrapping_add(c);
        m.set_u8(a, m.u8(a)?.wrapping_add(1))?;
        if m.u8(m.u32(0x507dac)?.wrapping_add(c))? == 0x20 {
            m.set_u8(m.u32(0x507d74)?.wrapping_add(c), 0)?;
            m.set_u8(m.u32(0x507dac)?.wrapping_add(c), 0)?;
        }
    }
    Ok(())
}
pub fn animate_vegetation(m: &mut MemoriaJogo, x: u32, y: u32) -> Result<u32> {
    let mut cell = x.wrapping_add(y.wrapping_shl(8)) as u16;
    let mut row = 0u16;
    while row < m.u16(0x508726)? {
        let mut col = 0u16;
        while col < m.u16(0x508724)? {
            animate_object(m, cell)?;
            cell = cell.wrapping_add(1);
            col = col.wrapping_add(1);
        }
        cell = cell.wrapping_add(0x100).wrapping_sub(m.u16(0x508724)?);
        row = row.wrapping_add(1);
    }
    // EAX depends on caller ECX; this memory-effect adapter exposes no register claim.
    Ok(u32::from(row))
}
const MASKS: [u32; 22] = [
    0x495c04, 0x495c10, 0x495c30, 0x495c50, 0x495cec, 0x495c80, 0x495d6c, 0x495d30, 0x495cf8,
    0x495d8a, 0x495d64, 0x495d5c, 0x495d8b, 0x495c6c, 0x495ca4, 0x495d1c, 0x495cd4, 0x495ce8,
    0x495d48, 0x495cbc, 0x495d1c, 0x495cc8,
];
fn collision(m: &MemoriaJogo, kind: u32, i: u32) -> Result<u32> {
    let t = (kind & 0xff) as usize;
    Ok(if t > 21 {
        i & 0xffffff00
    } else {
        (i & 0xff00) | u32::from(m.u8(MASKS[t].wrapping_add(i & 0xffff))?)
    })
}
pub fn building_cell(m: &MemoriaJogo, index: u32) -> Result<u32> {
    let i = index & 0xffff;
    let a = 0x49a0c8u32.wrapping_add(i * 276);
    let t = u32::from(m.u8(a + 8)?);
    let w = u32::from(m.u16(0x495d8c + 52 * t)?);
    let h = u32::from(m.u16(0x495d8e + 52 * t)?);
    let (mut minx, mut miny, mut maxx, mut maxy) = (0xffffu32, 0xffffu32, 0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            if collision(m, t, (w * y + x) & 0xffff)? & 0xff != 0 {
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
            }
        }
    }
    let half = |v: u32| -> u32 { ((v as i32) / 2) as u32 };
    let origin = ((i * 276) & 0xffff0000) | u32::from(m.u16(a + 2)?);
    Ok(half(maxy.wrapping_sub(miny))
        .wrapping_add(miny)
        .wrapping_shl(8)
        .wrapping_add(origin)
        .wrapping_add(minx)
        .wrapping_add(half(maxx.wrapping_sub(minx))))
}
fn building_visible(m: &MemoriaJogo, index: u32, player: u32, ring: bool) -> Result<u32> {
    let a = 0x49a0c8u32.wrapping_add((index & 0xffff) * 276);
    let kind = u32::from(m.u8(a + 8)?);
    let w = i32::from(m.u16(0x495d8c + 52 * kind)?);
    let h = i32::from(m.u16(0x495d8e + 52 * kind)?);
    let origin = m.u16(a + 2)?;
    let x0 = i32::from(origin & 255);
    let y0 = i32::from(origin >> 8);
    let start = if ring { -1 } else { 0 };
    let endx = if ring { w + 1 } else { w };
    let endy = if ring { h + 1 } else { h };
    for dy in start..endy {
        if !(0..256).contains(&(y0 + dy)) {
            continue;
        }
        for dx in start..endx {
            if !(0..256).contains(&(x0 + dx)) {
                continue;
            }
            if ring && dx != -1 && dx != w && dy != -1 && dy != h {
                continue;
            }
            let cell = origin
                .wrapping_add(dx as u16)
                .wrapping_add((dy as u16 & 255) << 8);
            let table = if ring { 0x507d88 } else { 0x507d78 };
            let ptr = m.u32(table + 4 * (player & 255))?;
            if m.u8(ptr.wrapping_add(u32::from(cell)))? & 0x10 == 0 {
                return Ok((u32::from(cell) & 0xff00) | 1);
            }
        }
    }
    Ok(if ring {
        (h + 1) as u32 & 0xffffff00
    } else {
        h as u32 & 0xff00
    })
}
pub fn set_unit_collision(m: &mut MemoriaJogo, index: u32, cell: u32) -> Result<u32> {
    let i = index & 0xffff;
    let a = unit(m, i)?;
    let t = m.u8(a + 1)?;
    if t > 25 {
        return Ok(((i * 276) & 0xffffff00) | u32::from(t));
    }
    let quad = matches!(t, 3 | 11 | 12 | 13 | 14 | 15 | 16 | 25);
    let mask = if t == 16 || t == 17 { 2 } else { 1 };
    let cells = if quad {
        vec![
            cell as u16,
            (cell as u16).wrapping_sub(1),
            (cell as u16).wrapping_sub(0x101),
            (cell as u16).wrapping_sub(0x100),
        ]
    } else {
        vec![cell as u16]
    };
    let mut out = 0;
    for (sub, c) in cells.into_iter().enumerate() {
        let attr = m.u32(0x507da8)?.wrapping_add(u32::from(c));
        m.set_u8(attr, m.u8(attr)? | mask)?;
        out = m.u32(0x507da8)?;
        if mask == 1 {
            let mut tag = (i << 3).wrapping_add(sub as u32);
            tag = (tag & 0xffff00ff) | ((((tag >> 8) & 0xff).wrapping_add(0x40) & 0xff) << 8);
            if m.u8(0x4962c8 + u32::from(t))? != 1 {
                tag |= 4;
            }
            let reg = m.u32(0x4e0f38)?.wrapping_add(u32::from(c) * 2);
            let old = m.u16(reg)?;
            if old < 0x4000 {
                m.set_u16(a + 0xfa + 2 * sub as u32, old)?;
                m.set_u16(reg, tag as u16)?;
                out = (m.u32(0x4e0f38)? & 0xffffff00) | 1;
            } else {
                out = reg & 0xffffff00;
            }
        }
    }
    Ok(out)
}
pub fn fatigue(m: &mut MemoriaJogo, index: u32) -> Result<u32> {
    let i = index & 0xffff;
    let a = unit(m, i)?;
    if m.u8(a + 0x72)? == 0 {
        return Ok((i * 276) & 0xffffff00);
    }
    let t = m.u8(a + 1)?;
    if t.wrapping_sub(11) <= 6 {
        return Ok(1);
    }
    let bonus = crate::frame_economy::bonus(m, u32::from(m.u8(a)?), u32::from(t), 0, 0)?;
    let limit = m
        .u16(0x4965dc + 50 * u32::from(t))?
        .wrapping_add(u16::from(m.u8(a + 0xf8)?))
        .wrapping_add(bonus as u16);
    let count = m.u16(a + 0x74)?.wrapping_add(1);
    m.set_u16(a + 0x74, count)?;
    if limit > count {
        return Ok(bonus);
    }
    let provisions = m.u16(a + 4)?;
    if provisions == 0 {
        crate::chain_fatigue::starve(m, i as u16, limit)?;
        return Ok(0); // Native void continuation; callers do not consume EAX.
    }
    m.set_u16(a + 0x74, 0)?;
    m.set_u16(a + 4, provisions.wrapping_sub(1))?;
    Ok(u32::from(provisions))
}
pub(crate) type ZoneWrites = Vec<(u32, Vec<u8>)>;
fn zone(m: &mut MemoriaJogo, i: u32, x: u32, y: u32, xx: u32, yy: u32) -> Result<()> {
    let i = i & 65535;
    for (n, v) in [x, y, xx, yy].into_iter().enumerate() {
        m.set_u32(0x5099b4 + i * 16 + 4 * n as u32, v & 65535)?;
    }
    if i > u32::from(m.u16(0x50a9b8)?) {
        m.set_u16(0x50a9b8, i as u16)?;
    }
    Ok(())
}
fn contact(m: &MemoriaJogo, p: u32) -> Result<u32> {
    let p = p & 0xff;
    Ok(if m.u8(0x4e2928 + p)? == 1 {
        p
    } else {
        u32::from(m.u8(0x496f8c + p)?)
    })
}
fn panel(m: &MemoriaJogo) -> Result<u32> {
    Ok((i32::from(m.u8(0x508f90)? as i8) as u32).wrapping_shl(5))
}
pub(crate) fn init_zone_plan(m: &MemoriaJogo, mode: u32, writes: &mut ZoneWrites) -> Result<u32> {
    let mode = mode.wrapping_sub(1);
    if mode as u16 > 3 {
        return Ok(mode);
    }
    if mode != 0 {
        return Err(stop("InitGameZone menu mode", 0x471830));
    }
    let mut max = None;
    let mut zone = |i: u32, x: u32, y: u32, xx: u32, yy: u32| -> Result<()> {
        let i = i & 65535;
        for (n, v) in [x, y, xx, yy].into_iter().enumerate() {
            writes.push((
                0x5099b4 + i * 16 + 4 * n as u32,
                (v & 65535).to_le_bytes().to_vec(),
            ));
        }
        let current = match max {
            Some(v) => v,
            None => m.u16(0x50a9b8)?,
        };
        if i > u32::from(current) {
            max = Some(i as u16);
            writes.push((0x50a9b8, (i as u16).to_le_bytes().to_vec()));
        } else {
            max = Some(current);
        }
        Ok(())
    };
    let w = u32::from(m.u16(0x49a018)?);
    let h = u32::from(m.u16(0x49a01a)?);
    let coords = [
        (0, 0, 1, 1),
        (2, 0, w.wrapping_sub(3), 1),
        (w.wrapping_sub(2), 0, w.wrapping_sub(1), 1),
        (w.wrapping_sub(2), 2, w.wrapping_sub(1), h.wrapping_sub(3)),
        (
            w.wrapping_sub(2),
            h.wrapping_sub(2),
            w.wrapping_sub(1),
            h.wrapping_sub(1),
        ),
        (2, h.wrapping_sub(2), w.wrapping_sub(3), h.wrapping_sub(1)),
        (0, h.wrapping_sub(2), 1, h.wrapping_sub(1)),
        (0, 2, 2, h.wrapping_sub(3)),
        (0, 0, 0, 0),
    ];
    for (i, (x, y, xx, yy)) in coords.into_iter().enumerate() {
        zone(i as u32 + 1, x, y, xx, yy)?;
    }
    if m.u8(0x497285)? != 0 {
        let x = u32::from(m.u16(0x507d68)?);
        let y = u32::from(m.u16(0x507d6a)?);
        zone(
            10,
            x,
            y,
            x.wrapping_add(u32::from(m.u16(0x496fe0)?)),
            y.wrapping_add(u32::from(m.u16(0x496fe2)?)),
        )?;
    } else {
        zone(10, 0, 0, 1, 1)?;
    }
    let vh = u32::from(m.u16(0x508726)?);
    let local = u32::from(m.u8(0x508041)?);
    let t = m.u8(0x4e2e7c + contact(m, local)?)?;
    if t == 1 || t == 2 {
        let d = if w == 0x280 { 0 } else { panel(m)? };
        zone(
            0x1a,
            d.wrapping_add(0x2d),
            (vh.wrapping_sub(1) << 5).wrapping_sub(d),
            d.wrapping_add(0x4c),
            ((vh << 5).wrapping_sub(1)).wrapping_sub(d),
        )?;
    } else if t == 3 {
        let x = u32::from(m.u16(0x4e2906)?);
        let y = u32::from(m.u16(0x496f80)?);
        zone(
            0x1a,
            x,
            y,
            x.wrapping_add(u32::from(m.u16(0x496f7c)?)),
            y.wrapping_add(u32::from(m.u16(0x496f7e)?)),
        )?;
    } else {
        zone(0x1a, 0, 0, 1, 1)?;
    }
    if m.u16(0x507d56)? != m.u16(0x507d54)? {
        let d = panel(m)?;
        zone(
            0x1b,
            d,
            (vh.wrapping_sub(1) << 5).wrapping_sub(d),
            d.wrapping_add(0x2d),
            (vh << 5).wrapping_sub(d),
        )?;
    } else {
        zone(0x1b, 0, 0, 1, 1)?;
    }
    for p in 0..4 {
        if p != local && m.u32(0x5080b0 + 16 * local + 4 * p)? != 0 {
            let d = panel(m)?;
            let base = (u32::from(m.u16(0x508724)?).wrapping_sub(1) << 5)
                .wrapping_sub(0x50)
                .wrapping_add(20 * p)
                .wrapping_sub(d);
            zone(
                0x1c + p,
                base,
                (vh.wrapping_sub(1) << 5).wrapping_sub(d),
                base.wrapping_add(0x13),
                ((vh << 5).wrapping_sub(1)).wrapping_sub(d),
            )?;
        } else {
            zone(0x1c + p, 0, 0, 1, 1)?;
        }
    }
    zone(11, 0, vh << 5, w, h)?;
    writes.push((0x50a9b4, 1u16.to_le_bytes().to_vec()));
    writes.push((0x50a9b6, 31u16.to_le_bytes().to_vec()));
    Ok((vh << 5) & 0xffff)
}
/// Shared explicit writes, including native prefix effects if a later read fails.
pub fn init_zone(m: &mut MemoriaJogo, mode: u32) -> Result<u32> {
    let mut writes = Vec::new();
    let result = init_zone_plan(m, mode, &mut writes);
    for (address, bytes) in writes {
        m.write(address, &bytes)?;
    }
    result
}
pub fn call(m: &mut MemoriaJogo, name: &str, a: &[u32]) -> Result<u32> {
    match name {
        "Random" => random(m),
        "LongRand" | "rand" => crt_random(m),
        "NextAnimTerrain" => animate_terrain(m, a[0], a[1]),
        "NextAnimVeget" => animate_vegetation(m, a[0], a[1]),
        "NextAnimVegetFullMap" => {
            for c in 0..=u16::MAX {
                animate_object(m, c)?;
            }
            Ok(256)
        }
        "GameNextFrame" => {
            m.set_u32(0x496d40, m.u32(0x496d40)?.wrapping_add(1))?;
            Ok(0)
        }
        "GetBit" => Ok((a[0] & 0xff).wrapping_shr(a[1] & 31) & 1),
        "SetBit" => {
            let mask = 1u32.wrapping_shl(a[1] & 31) as u8;
            if a[2] & 0xff <= 1 {
                let v = m.u8(a[0])?;
                m.set_u8(
                    a[0],
                    if a[2] & 0xff == 0 {
                        v & !mask
                    } else {
                        v | mask
                    },
                )?;
            }
            Ok(a[0])
        }
        "CaseEstDecouverte" | "CaseEstIlluminee" | "TerrainEstDecouvert" | "TerrainEstIllumine" => {
            let table = if name == "CaseEstDecouverte" || name == "TerrainEstDecouvert" {
                0x507d78
            } else {
                0x507d88
            };
            let mask = if name.starts_with("Terrain") {
                0x1f
            } else {
                0x10
            };
            let p = m.u32(table + 4 * (a[1] & 0xff))?;
            Ok((a[0] & 0xff00) | u32::from(m.u8(p.wrapping_add(a[0] & 0xffff))? & mask == 0))
        }
        "BatimentGetCase" => building_cell(m, a[0]),
        "BatimentCollision" => collision(m, a[0], a[1]),
        "BatimentEstDecouvert" => building_visible(m, a[0], a[1], false),
        "BatimentContourEstIllumine" => building_visible(m, a[0], a[1], true),
        "BatimentDecouvre" | "BatimentIllumine" => {
            let r = crate::chain_vision::entry(
                m,
                name,
                crate::FrameRegisters {
                    eax: a[0],
                    edx: a[1],
                    ebx: if name == "BatimentIllumine" { a[2] } else { 0 },
                    ..Default::default()
                },
            )?;
            Ok(r.eax)
        }
        "SetCollision" => {
            let ptr = m.u32(0x507da8)?;
            let addr = ptr.wrapping_add(a[0] & 0xffff);
            m.set_u8(addr, m.u8(addr)? | (a[1] as u8))?;
            Ok(ptr)
        }
        "SetEnginCollision" => set_unit_collision(m, a[0], a[1]),
        "CalculFatigue" => fatigue(m, a[0]),
        "SetClipping" => {
            for (p, v) in [
                (0x49a020, a[0]),
                (0x49a022, a[2]),
                (0x49a01c, a[1]),
                (0x49a01e, a[3]),
            ] {
                m.set_u16(p, v as u16)?;
            }
            Ok(0)
        }
        "InitGameZone" => init_zone(m, a[0]),
        "MouseSetZone" => {
            zone(m, a[0], a[1], a[2], a[3], a[4])?;
            Ok((a[0] & 0xffff) << 4)
        }
        "MouseTestZone" => {
            m.set_u16(0x50a9b4, a[0] as u16)?;
            m.set_u16(0x50a9b6, a[1] as u16)?;
            Ok(0)
        }
        "ChercheContacteur" => contact(m, a[0]),
        "MessageBoxExiste" => {
            let cur = m.u16(0x507d56)?;
            Ok((a[0] & 0xffff0000) | u32::from(cur & 0xff00) | u32::from(cur != m.u16(0x507d54)?))
        }
        "PlayerEstContacterPourAlliance" => Ok(u32::from(
            m.u32(0x5080b0 + 16 * (a[0] & 0xff) + 4 * (a[1] & 0xff))? != 0,
        )),
        "ExisteEngin" => Ok(u32::from(m.u8(unit(m, a[0])? + 2)? != 0)),
        "EnginGetType" | "EnginGetPlayer" => {
            let p = m.u32(0x4e19a0)?;
            Ok((p & 0xffffff00)
                | u32::from(m.u8(unit(m, a[0])? + u32::from(name == "EnginGetType"))?))
        }
        "EnginEstTerreur" | "EnginEstMiroir" | "EnginEstArmure" => {
            let bit = match name {
                "EnginEstTerreur" => 1,
                "EnginEstMiroir" => 2,
                _ => 3,
            };
            Ok(u32::from(m.u8(unit(m, a[0])? + 0xdc)? >> bit & 1))
        }
        "TypeEnginSeFatigue" => {
            let t = a[0] & 0xff;
            Ok(if t > 25 {
                a[0] & 0xffffff00
            } else {
                u32::from(t <= 10 || t == 25)
            })
        }
        "MissionGetNum" => Ok((a[0] & 0xffff0000) | u32::from(m.u8(0x496fe4)?)),
        _ => crate::frame_movement::call(m, name, a),
    }
}

/// Stable isolated-provider ids. Void routines return an adapter value only.
pub fn isolated(id: u32) -> Option<(&'static str, usize)> {
    Some(match id {
        200 => ("Random", 0),
        201 => ("rand", 0),
        202 => ("GetBit", 2),
        203 => ("SetBit", 3),
        204 => ("CaseEstDecouverte", 2),
        205 => ("CaseEstIlluminee", 2),
        206 => ("BatimentCollision", 2),
        207 => ("BatimentGetCase", 1),
        208 => ("BatimentEstDecouvert", 2),
        209 => ("BatimentContourEstIllumine", 2),
        210 => ("SetEnginCollision", 2),
        211 => ("CalculFatigue", 1),
        212 => ("SetClipping", 4),
        213 => ("InitGameZone", 1),
        214 => ("MouseSetZone", 5),
        215 => ("MouseTestZone", 2),
        216 => ("NextAnimTerrain", 2),
        217 => ("NextAnimVeget", 2),
        218 => ("NextAnimVegetFullMap", 0),
        219 => ("GameNextFrame", 0),
        220 => ("SetCollision", 2),
        221 => ("ChercheContacteur", 1),
        222 => ("MessageBoxExiste", 1),
        223 => ("PlayerEstContacterPourAlliance", 2),
        _ => return None,
    })
}
