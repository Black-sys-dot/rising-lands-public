//! Original-first Core field searches and path blocking (buscas_bloqueios).
use crate::{
    frame::stop,
    player::{unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
pub fn gc(m: &MemoriaJogo, c: u32) -> Result<bool> {
    Ok((0x1d..0x35).contains(&m.b(m.d(0x507d74)?.wrapping_add(c & 65535))?))
}
pub fn passable(m: &MemoriaJogo, u: u32, c: u32) -> Result<bool> {
    Ok(passage(m, u, c, 0, 1)?.0)
}
/// EnginPeutPasser 0x442668 with an explicit BL layer and no occupant array.
pub fn passable_layer(m: &MemoriaJogo, u: u32, c: u32, layer: u8) -> Result<bool> {
    Ok(passage(m, u, c, 0, layer)?.0)
}
/// EnginPeutPasser's occupant lookup 0x446693: flying occupants return at
/// once; the last ground occupant wins; type 16 needs layer 0x32.
fn occupant(m: &MemoriaJogo, cell: u16) -> Result<u32> {
    let mut ground = 0xffffu32;
    for i in 0..800u32 {
        let q = unit(m, i)?;
        if matches!(m.b(q + 2)?, 0 | 4) || m.b(q + 0x72)? == 0 {
            continue;
        }
        let layer = m.b(q + 0xab)?;
        let filter = if layer != 0 { 2 } else { 1 };
        if crate::frame_units::occupies(m, i as u16, cell, filter)? == 0 {
            continue;
        }
        if m.b(q + 1)? == 16 && layer != 0x32 {
            continue;
        }
        ground = i;
        if layer != 0 {
            return Ok(i);
        }
    }
    Ok(ground)
}
/// Occupant types a type-12/15 unit may share a cell with (0x4428fe table).
fn yielding(kind: u32) -> bool {
    matches!(kind, 0..=2 | 4..=10 | 0x12 | 0x13 | 0x16) || kind >= 0x1a
}
/// 0x442b94 / 0x442722: two-by-two units (types 12..15) test the cell and
/// its west, north-west and north neighbours; row/column zero is impassable.
/// Each cell passes when the unit itself occupies it or attribute bit 0 is
/// clear; otherwise the occupant is recorded (ECX array) and only types 12/15
/// may pass through a yielding enemy occupant. Every cell is evaluated.
fn big_passage(
    m: &MemoriaJogo,
    u: u32,
    r: u32,
    t: u32,
    c: u32,
    ecx: u32,
    layer: u8,
) -> Result<(bool, u32, u32, u32)> {
    let cell = c & 65535;
    if cell.is_multiple_of(256) || cell / 256 == 0 {
        return Ok((false, 0x100, ecx & 0xffff0000, cell / 256));
    }
    let flags = m.d(0x507da8)?;
    let owner = m.b(r)?;
    let mut ok = true;
    let mut stored = 0u32;
    for d in [0u32, 1, 0x101, 0x100] {
        let cc = cell.wrapping_sub(d) & 65535;
        if crate::frame_units::occupies(m, u as u16, cc as u16, layer)? != 0 {
            continue;
        }
        if m.b(flags.wrapping_add(cc))? & u32::from(layer) == 0 {
            continue;
        }
        let other = occupant(m, cc as u16)?;
        if ecx != 0 {
            // Stores are ordered; the array pointer is caller memory.
            let _ = (ecx, stored, other);
            return stop("EnginPeutPasser occupant array", 0x442761);
        }
        stored += 1;
        if matches!(t, 12 | 15)
            && other != 0xffff
            && m.b(unit(m, other)?)? != owner
            && yielding(m.b(unit(m, other)? + 1)?)
        {
            continue;
        }
        ok = false;
    }
    Ok((ok, 0x100, (ecx & 0xffff0000) | u32::from(ok), cell))
}
fn passage(m: &MemoriaJogo, u: u32, c: u32, ecx: u32, layer: u8) -> Result<(bool, u32, u32, u32)> {
    let r = unit(m, u)?;
    let t = m.b(r + 1)?;
    // Dispatch 0x442698..0x442bc7: types 3, 11..=16 and 25 take the 2x2
    // test 0x442b94; 0..=2, 4..=10 and 17..=24 the single-cell test 0x4426c7
    // (EnginOccupeCase 0x4424fd: own cell or +0x7c). Above 25 the original
    // returns AL=1 at once; no unit type reaches it, so it stays addressed.
    if matches!(t, 3 | 11..=16 | 25) {
        return big_passage(m, u, r, t, c, ecx, layer);
    }
    if t > 25 {
        return stop("EnginPeutPasser search type", 0x442668);
    }
    // EnginOccupeCase 0x44246e: BL 1 ignores a flying unit, BL 2 a grounded one.
    let excluded = (if layer == 1 {
        m.b(r + 0xab)? != 0
    } else {
        m.b(r + 0xab)? == 0
    }) || m.b(r + 2)? == 14;
    let own = !excluded && (m.w(r + 8)? == c || m.w(r + 0x7c)? == c);
    let ebx = 0x100 | u32::from(excluded || own);
    if own {
        return Ok((true, ebx, (ecx & 0xffff0000) | 1, c & 65535));
    }
    let ok = m.b(m.d(0x507da8)?.wrapping_add(c & 65535))? & u32::from(layer) == 0;
    Ok((ok, ebx, (ecx & 0xffff0000) | u32::from(ok), 1))
}
pub fn veget(m: &MemoriaJogo, c: u32, d: u32, lo: u32, hi: u32) -> Result<u32> {
    Ok(veget_regs(
        m,
        R {
            eax: c,
            edx: d,
            ebx: lo,
            ecx: hi,
            ..Default::default()
        },
    )?
    .eax)
}
fn veget_regs(m: &MemoriaJogo, mut r: R) -> Result<R> {
    let a = r.eax;
    let d = r.edx & 65535;
    if d >= 0x7fff {
        return stop("ChercheVeget radius", 0x477962);
    }
    let lo = r.ebx & 255;
    let hi = r.ecx & 255;
    let g = m.d(0x507d74)?;
    let c = a & 65535;
    let (x, y) = ((c & 255) as i32, (c >> 8) as i32);
    let (w, h) = (m.w(0x497168)? as i32, m.w(0x49716a)? as i32);
    if (lo..=hi).contains(&m.b(g + c)?) {
        r.ebx = g + c;
        r.edx = x as u32;
        return Ok(r);
    }
    for k in 1..=d as i32 {
        for i in -k..=k {
            let candidates = [
                (x - k >= 0 && y + i >= 0 && y + i < h, i * 256 - k, k),
                (x + k < w && y + i >= 0 && y + i < h, i * 256 + k, k),
                (
                    i != -k && i != k && y - k >= 0 && x + i >= 0 && x + i < w,
                    -k * 256 + i,
                    k * 256,
                ),
                (
                    i != -k && i != k && y + k < h && x + i >= 0 && x + i < w,
                    k * 256 + i,
                    k * 256,
                ),
            ];
            for (ok, off, edx) in candidates {
                if !ok {
                    continue;
                }
                let q = c.wrapping_add(off as u32);
                let v = m.b(g.wrapping_add(q))?;
                r.ecx = (r.ecx & 0xffff00ff) | (v << 8);
                if (lo..=hi).contains(&v) {
                    r.eax = a.wrapping_add(off as u32);
                    r.ebx = g.wrapping_add(q);
                    r.edx = edx as u32;
                    return Ok(r);
                }
            }
        }
    }
    r.eax = 65535;
    r.ebx = d;
    r.edx = d + 1;
    Ok(r)
}
/// Semantic AX search for game callers: the original upper stack residue is not consumed.
pub fn free_gc(m: &MemoriaJogo, c: u32, d: u32, u: u32) -> Result<u32> {
    Ok(free_regs(
        m,
        R {
            eax: c,
            edx: d,
            ebx: u,
            ..Default::default()
        },
        false,
    )?
    .eax & 65535)
}
fn free_regs(m: &MemoriaJogo, mut r: R, full: bool) -> Result<R> {
    let a = r.eax;
    let d = r.edx & 65535;
    if d >= 0x7fff {
        return stop("ChercheGcLibre radius", 0x477f27);
    }
    let u = r.ebx & 65535;
    let g = m.d(0x507d74)?;
    let c = a & 65535;
    let (x, y) = ((c & 255) as i32, (c >> 8) as i32);
    let (w, h) = (m.w(0x497168)? as i32, m.w(0x49716a)? as i32);
    let mut ebx = 0x100;
    if (0x1e..=0x35).contains(&m.b(g + c)?) {
        let (ok, b, _, edx) = passage(m, u, c, 0, 1)?;
        ebx = b;
        if ok {
            r.eax = c;
            r.ebx = b;
            r.edx = edx;
            return Ok(r);
        }
    }
    for k in 1..d as i32 {
        for i in -k..=k {
            let candidates = [
                (
                    x - k >= 0 && y + i >= 0 && y + i < h,
                    i * 256 - k,
                    Some(0x18),
                ),
                (
                    x + k < w && y + i >= 0 && y + i < h,
                    i * 256 + k,
                    Some(0x1c),
                ),
                (
                    i != -k && i != k && y - k >= 0 && x + i >= 0 && x + i < w,
                    -k * 256 + i,
                    None,
                ),
                (
                    i != -k && i != k && y + k < h && x + i >= 0 && x + i < w,
                    k * 256 + i,
                    None,
                ),
            ];
            for (ok, off, local) in candidates {
                if !ok {
                    continue;
                }
                let target = c.wrapping_add(off as u32) & 65535;
                if !(0x1e..=0x35).contains(&m.b(g.wrapping_add(c.wrapping_add(off as u32)))?) {
                    continue;
                }
                let (ok, b, _, edx) = passage(m, u, target, 0, 1)?;
                if ok {
                    let mut value = a.wrapping_add(off as u32);
                    if let Some(local) = local {
                        value = target;
                        if full {
                            if r.esp == 0 {
                                return stop("ChercheGcLibre stack residue", 0x477ead);
                            }
                            value |= m.w(r.esp.wrapping_sub(0x44).wrapping_add(local + 2))? << 16
                        }
                    }
                    r.eax = value;
                    r.ebx = b;
                    r.edx = edx;
                    return Ok(r);
                }
            } // original last ring check's EBX
            ebx = if i == -k || i == k {
                (-k) as u32
            } else if y + k >= h {
                (y + k) as u32
            } else if x + i < 0 || x + i >= w {
                (x + i) as u32
            } else {
                (c & 0xffff00ff) | (0x1e << 8)
            };
        }
    }
    r.eax = 65535;
    r.ebx = ebx;
    r.edx = d.max(1);
    Ok(r)
}
pub fn entry(m: &MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    match name {
        "TerrainEstGc" => r.eax = u32::from(gc(m, r.eax)?),
        "ChercheVeget" => r = veget_regs(m, r)?,
        "ChercheGc" => {
            let saved = r;
            r = veget_regs(
                m,
                R {
                    eax: r.eax & 65535,
                    edx: r.edx & 65535,
                    ebx: 0x1e,
                    ecx: 0x35,
                    ..r
                },
            )?;
            r.ebx = saved.ebx;
            r.ecx = saved.ecx;
        }
        "ChercheGcLibre" => r = free_regs(m, r, true)?,
        "TypeEnginPeutEcraser" => {
            r.eax = (r.eax & !255) | u32::from(matches!(r.eax & 255, 12 | 15))
        }
        "IsDirectlyBlocking" | "IsIndirectlyBlocking" => {
            let search = m.w(0x496304)?;
            if search >= 800 || r.eax >= 800 {
                return stop("Blocking unit index", 0x42eba4);
            }
            let a = unit(m, search)?;
            let b = unit(m, r.eax)?;
            let mut diff = (m.b(b + 0x90)?.wrapping_sub(r.edx) as i32) / 2;
            if diff < -4 {
                diff += 8
            }
            if diff > 4 {
                diff -= 8
            }
            let aligned = (-1..=1).contains(&diff);
            let (p, q) = (m.b(a)?, m.b(b)?);
            let allied = if p == q {
                true
            } else if p > 4 || q > 4 {
                false
            } else {
                m.b(m.d(0x5080a8)? + 20 * p + 6)? & (1 << q) != 0
            };
            let (t, s) = (m.b(a + 1)?, m.b(b + 1)?);
            let crush = matches!(t, 12 | 15) && !matches!(s,3|11..=17|20|21|23..=25);
            let mut edx = m.w(b + 8)?;
            let mut moving = m.w(b + 0x7e)? != edx;
            if matches!(s, 0 | 1 | 20 | 21) {
                edx = m.b(b + 2)?;
                if edx == 1 {
                    edx = m.b(b + 0x8a)?;
                    if edx == 1 {
                        moving = false
                    }
                }
            } else if matches!(s, 6 | 7) {
                edx = m.b(b + 2)?;
                if edx == 15 {
                    edx = m.b(b + 0x89)?;
                    if edx == 1 {
                        moving = false
                    }
                }
            }
            let positive = if name == "IsDirectlyBlocking" { 2 } else { 1 };
            let al = if allied {
                if if moving { aligned } else { m.b(0x4e1771)? != 0 } {
                    positive
                } else {
                    0
                }
            } else if crush {
                1
            } else if moving && aligned {
                positive
            } else {
                0
            };
            r.eax = (u32::from(moving) << 8) | al;
            r.edx = edx;
        }
        _ => return stop("Core search", 0x47790d),
    }
    Ok(r)
}
