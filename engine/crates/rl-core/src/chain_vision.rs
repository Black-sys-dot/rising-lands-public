//! Core structure vision, visao_grupos at pinned main 54d148d.
use crate::{
    frame::stop,
    player::{structure, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};

fn cell(m: &mut MemoriaJogo, c: u32, p: u32, mode: u32) -> Result<(u32, u32)> {
    // Player 4 (monsters) reaches DecouvreCase only through a tower shot
    // (BatimentDecouvre 0x42724a..0x427a3a): the original indexes 0x507d78[4],
    // which is player 0's light map 0x507d88[0], and edits it like a discovery
    // map (user decision 2026-11-04: replicate the original).
    if p > 4 || (p == 4 && mode != 0) {
        return stop("Structure vision player", 0x419233);
    }
    if mode == 0 {
        let base = m.d(0x507d78 + 4 * p)?;
        crate::player_sight::borders(m, base, c, true)?;
        if m.b(base + c)? != 0 && p == m.b(0x508041)? {
            crate::player_sight::minimap(m, p as u8, c as u16, 1)?;
        }
        m.sb(base + c, m.b(base + c)? & 0xef)?;
        return Ok((p, c));
    }
    let count = m.d(0x507d98 + 4 * p)?.wrapping_add(c);
    let n = m.b(count)?;
    if (mode == 1 && n == 0) || (mode == 2 && n == 1) {
        let base = m.d(0x507d88 + 4 * p)?;
        crate::player_sight::borders(m, base, c, mode == 1)?;
        let b = m.b(base + c)?;
        m.sb(base + c, if mode == 1 { b & 0xef } else { b | 0x10 })?;
        if p == m.b(0x508041)? {
            crate::player_sight::minimap(m, p as u8, c as u16, if mode == 1 { 2 } else { 3 })?;
        }
    }
    if mode == 1 {
        if n != 255 {
            m.sb(count, n + 1)?;
        }
        Ok((p, if n == 255 { n } else { n | ((n + 1) << 8) }))
    } else {
        if n != 0 {
            m.sb(count, n - 1)?;
        }
        Ok((count, (c & 255) | (n << 8)))
    }
}

pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    let p = r.edx & 255;
    match name {
        "DecouvreCase" | "IllumineCase" | "EteintCase" => {
            (r.eax, r.edx) = cell(
                m,
                r.eax & 65535,
                p,
                match name {
                    "DecouvreCase" => 0,
                    "IllumineCase" => 1,
                    _ => 2,
                },
            )?;
        }
        "BatimentDecouvre" | "BatimentIllumine" => {
            let s = r.eax & 65535;
            if name == "BatimentDecouvre" && p == 200 {
                r.eax = 200;
                r.edx = s;
                return Ok(r);
            }
            if p > 4 || (p == 4 && name != "BatimentDecouvre") {
                return stop("Structure vision player", 0x419233);
            }
            let e = structure(s);
            let t = m.b(e + 8)?;
            let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
            let origin = m.w(e + 2)?;
            let (x, y) = ((origin & 255) as i32, (origin >> 8) as i32);
            if name == "BatimentDecouvre" {
                let (w, h) = (w + u32::from(w < 256), h + u32::from(h < 256));
                for dy in -1..h as i32 {
                    if !(0..256).contains(&(y + dy)) {
                        continue;
                    }
                    for dx in -1..w as i32 {
                        if (0..256).contains(&(x + dx)) {
                            cell(m, (origin as i32 + dx + (dy << 8)) as u32 & 65535, p, 0)?;
                        }
                    }
                }
                r.eax = h;
                r.edx = h;
            } else {
                let mode = if r.ebx & 255 != 0 { 1 } else { 2 };
                r.ebx = origin;
                r.edx = 0;
                for dy in 0..h {
                    r.edx = dy;
                    if y + dy as i32 >= 256 {
                        continue;
                    }
                    r.ebx = 0;
                    for dx in 0..w {
                        r.edx = x as u32;
                        if x + (dx as i32) < 256 {
                            (_, r.edx) =
                                cell(m, (m.w(e + 2)? + dx + ((dy & 255) << 8)) & 65535, p, mode)?;
                        }
                        r.ebx += 1;
                    }
                }
                r.eax = h;
            }
        }
        _ => return stop("Unknown structure vision entry", 0x419233),
    }
    Ok(r)
}
fn low_word(v: u32, w: u32) -> u32 {
    (v & 0xffff0000) | (w & 65535)
}
fn recentre(m: &mut MemoriaJogo, r: R) -> Result<u32> {
    let (mut ebx, mut ecx, mut esi, mut edi) = (r.ebx, r.ecx, r.esi, r.edi);
    let margin = m.b(0x508f90)? as u8 as i8 as i32;
    for (pos, view, display) in [
        (0x507d68, 0x508724, 0x496fe0),
        (0x507d6a, 0x508726, 0x496fe2),
    ] {
        let edx = (margin << 5) as u32;
        let mut eax = m.w(pos)?;
        if (eax as i64) < i64::from(edx as i32) {
            m.sw(pos, ((margin as u32 & 65535) << 5) & 65535)?;
            continue;
        }
        ebx = (m.w(view)? << 5).wrapping_sub(edx);
        eax = eax.wrapping_add(m.w(display)?).wrapping_add(2);
        if (eax as i32) > (ebx as i32) {
            let d = m.w(display)?;
            let sub = if pos == 0x507d68 {
                ebx = low_word(ebx, d);
                ebx
            } else {
                ecx = low_word(ecx, d);
                ecx
            };
            m.sw(
                pos,
                (m.w(view)? << 5)
                    .wrapping_sub((margin as u32 & 65535) << 5)
                    .wrapping_sub(sub)
                    .wrapping_sub(2),
            )?;
        }
    }
    let mut eax = m.w(0x497168)?;
    if m.w(0x508720)? + m.w(0x508724)? >= eax {
        esi = low_word(esi, m.w(0x508724)?);
        edi = low_word(edi, m.w(0x497168)?);
        eax = m.w(0x508720)?.wrapping_add(esi).wrapping_sub(edi);
        m.sw(0x508720, m.w(0x508720)?.wrapping_sub(eax))?;
    }
    eax = m.w(0x49716a)?;
    if m.w(0x508722)? + m.w(0x508726)? >= eax {
        ebx = low_word(ebx, m.w(0x508726)?);
        ecx = low_word(ecx, m.w(0x49716a)?);
        eax = m.w(0x508722)?.wrapping_add(ebx).wrapping_sub(ecx);
        esi = low_word(esi, m.w(0x508722)?);
        m.sw(0x508722, esi.wrapping_sub(eax))?;
    }
    Ok(eax)
}
pub fn extra(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    match name {
        "MiniMapRecentre" => r.eax = recentre(m, r)?,
        "MiniMapSetPos" => {
            let mut eax = low_word(r.eax, r.eax.wrapping_add(m.w(0x507d6c)?));
            let ecx = low_word(r.ecx, m.w(0x507d6e)?);
            let ebx = eax as u16 as i16 as i32 as u32;
            let mut edx = r.edx.wrapping_add(ecx);
            if (ebx as i32) < 0 {
                eax = 0;
            }
            m.sw(0x507d68, eax)?;
            if (edx as u16 as i16) < 0 {
                edx = 0;
            }
            m.sw(0x507d6a, edx)?;
            r.eax = recentre(
                m,
                R {
                    eax,
                    ebx,
                    ecx,
                    edx,
                    ..r
                },
            )?;
            r.edx = edx;
        }
        "CalculPosition" => r.eax = position(m, r)?,
        "RechercheDecalageSortie" => {
            let cp = position(
                m,
                R {
                    eax: r.eax & 65535,
                    ..r
                },
            )?;
            let c = exit_cell(m, r.eax & 65535)?;
            if u32::from(c) == (cp & 65535) {
                r.eax = cp;
            } else {
                if r.esp == 0 {
                    return stop("RechercheDecalageSortie stack residue", 0x41d2a6);
                }
                let row = (i32::from(c >> 8) - ((cp & 65535) >> 8) as i32) << 8;
                let col = i32::from(c & 255) - (cp & 255) as i32;
                let residue = m.w(r.esp.wrapping_sub(0x1e))? << 16;
                r.eax = cp
                    .wrapping_add(residue | (row as u32 & 65535))
                    .wrapping_add(col as u32);
            }
        }
        _ => return stop("Unknown vision query", 0x420698),
    }
    Ok(r)
}
type Geometry = (u32, u32, u32, u32, u8);
const GEOMETRY: [Option<Geometry>; 22] = [
    Some((3, 2, 0x201, 0x102, 1)),
    Some((1, 1, 0x300, 0xff, 2)),
    Some((4, 2, 0x101, 0x102, 0)),
    Some((4, 2, 0x202, 0x102, 0)),
    Some((4, 2, 0x100, 0x102, 0)),
    Some((1, 0, 0x202, 0x102, 0)),
    Some((4, 2, 0x100, 0x102, 0)),
    Some((4, 1, 0x300, 0x102, 0)),
    Some((6, 4, 0x202, 0x102, 3)),
    None,
    Some((3, 2, 0, 0x102, 1)),
    Some((3, 2, 0, 0x102, 1)),
    None,
    Some((4, 1, 0x200, 0x102, 0)),
    Some((6, 4, 0x300, 0x102, 0)),
    Some((4, 1, 0x100, 0x102, 0)),
    Some((4, 2, 0x100, 0x102, 0)),
    None,
    Some((1, 1, 0x201, 1, 4)),
    Some((4, 2, 0x100, 0x102, 0)),
    Some((4, 1, 0x100, 0x102, 0)),
    Some((4, 2, 0x100, 0x102, 0)),
];
pub fn position(m: &MemoriaJogo, r: R) -> Result<u32> {
    let e = structure(r.eax);
    let t = m.b(e + 8)?;
    let Some(Some((dx, dy, correction, addback, high))) = GEOMETRY.get(t as usize).copied() else {
        return Ok(0);
    };
    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
    let upper = match high {
        1 => r.esi,
        2 => r.edi,
        3 => h.wrapping_sub(4),
        4 => w.wrapping_sub(1),
        _ => 0,
    };
    Ok(low_word(upper, m.w(e + 2)?)
        .wrapping_add(h.wrapping_sub(dy) << 8)
        .wrapping_add(w)
        .wrapping_sub(dx)
        .wrapping_sub(correction.wrapping_sub(addback)))
}
/// Caller-consumed AX only; the uninitialized upper half of the original is not an API value here.
pub fn exit_cell(m: &MemoriaJogo, s: u32) -> Result<u16> {
    let t = m.b(structure(s) + 8)?;
    if t > 21 {
        return stop("RechercheDecalageSortie", 0x41d23d);
    }
    if matches!(t, 9 | 12 | 17) {
        // 0x41d23d: these types skip the setup, so the radius and result are
        // uninitialized locals. The port models a clean stack (0), as for
        // CalculRepareEngin 0x44b772; see validation/claude-20261108/README.md.
        return Ok(0);
    }
    let cp = position(
        m,
        R {
            eax: s,
            ..Default::default()
        },
    )? & 65535;
    let (x, y) = ((cp & 255) as i32, (cp >> 8) as i32);
    for dy in -1..=1 {
        if !(0..256).contains(&(y + dy)) {
            continue;
        }
        for dx in -1..=1 {
            if !(0..256).contains(&(x + dx)) || (dx == 0 && dy == 0) {
                continue;
            }
            let c = (cp as i32 + (dy << 8) + dx) as u32 & 65535;
            if m.b(m.d(0x507da8)? + c)? & 1 == 0 {
                return Ok(c as u16);
            }
        }
    }
    Ok(cp as u16)
}

/// BatimentCalculCaseSortie 0x41d2fc (type-5 exit of BatimentEnleveUnColon).
/// The returned cell is the low word of origin + (H - 1) * 256 + W - 2 - 0x202
/// (callers consume DX only). Structure +0x7c gets that cell, then every
/// neighbour of it (rows/columns -1..=1, centre excluded) whose row/column
/// offset from the structure centre (origin + (H / 2) * 256 + W / 2) stays on
/// the map and whose collision byte has bit 0 clear; the last such neighbour
/// in row-major order wins (no break in the original loop).
pub fn case_sortie(m: &mut MemoriaJogo, s: u32) -> Result<u16> {
    let e = structure(s);
    let t = m.b(e + 8)?;
    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
    let origin = m.w(e + 2)?;
    let centre = origin.wrapping_add((h >> 1) << 8).wrapping_add(w >> 1) & 65535;
    let (cx, cy) = ((centre & 255) as i32, (centre >> 8) as i32);
    let exit = origin
        .wrapping_add(h.wrapping_sub(1) << 8)
        .wrapping_add(w.wrapping_sub(2))
        .wrapping_sub(0x202)
        & 65535;
    m.sw(e + 0x7c, exit)?;
    let map = m.d(0x507da8)?;
    for dy in -1..=1i32 {
        if !(0..256).contains(&(cy + dy)) {
            continue;
        }
        for dx in -1..=1i32 {
            if !(0..256).contains(&(cx + dx)) || (dx == 0 && dy == 0) {
                continue;
            }
            let c = (exit as i32).wrapping_add(dy << 8).wrapping_add(dx) as u32 & 65535;
            if m.b(map.wrapping_add(c))? & 1 == 0 {
                m.sw(e + 0x7c, c)?;
            }
        }
    }
    Ok(exit as u16)
}

/// TestCaseSortieCollisionHangar 0x41d499: the cell and its west, north and
/// north-west neighbours are free of collision bit 0, each checked before
/// BatimentCollision(type, cell) (the cell word itself is the mask index in the
/// original, four times).
fn hangar_cell_free(m: &mut MemoriaJogo, kind: u32, cell: u32) -> Result<bool> {
    for d in [0, 1, 0x100, 0x101] {
        let c = cell.wrapping_sub(d) & 65535;
        if crate::frame_spawn::call(m, "TerrainEnCollision", &[c, 1])? & 255 != 0 {
            return Ok(false);
        }
        if crate::frame_core::call(m, "BatimentCollision", &[kind, cell])? & 255 == 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// RechercheEmplacementSortieHangar 0x41d570 (type-8 exit of
/// BatimentEnleveUnColon). E = the low word of origin + (H - 1) * 256 + W -
/// 2 - 0x202. Rows and columns -2..=4 around E (centre excluded) whose offset from
/// the structure centre stays on the map are tried in row-major order: the
/// first cell free of collision bit 0 whose BatimentCollision(type, (W - 2) *
/// row + column) is set wins for a unit whose type is not a vehicle (11..=15);
/// a vehicle also needs TestCaseSortieCollisionHangar. None: E. Callers consume
/// the low word.
pub fn hangar_exit(m: &mut MemoriaJogo, s: u32, u: u32) -> Result<u16> {
    exit_search(m, s, u, None)
}
/// RechercheEmplacementSortieTente 0x41d74f: the same search, 0xffff when none.
pub fn tent_exit(m: &mut MemoriaJogo, s: u32, u: u32) -> Result<u16> {
    exit_search(m, s, u, Some(65535))
}
fn exit_search(m: &mut MemoriaJogo, s: u32, u: u32, none: Option<u16>) -> Result<u16> {
    let e = structure(s);
    let kind = m.b(e + 8)?;
    let (w, h) = (m.w(0x495d8c + 52 * kind)?, m.w(0x495d8e + 52 * kind)?);
    let origin = m.w(e + 2)?;
    let centre = origin.wrapping_add((h >> 1) << 8).wrapping_add(w >> 1) & 65535;
    let (cx, cy) = ((centre & 255) as i32, (centre >> 8) as i32);
    let exit = origin
        .wrapping_add(h.wrapping_sub(1) << 8)
        .wrapping_add(w.wrapping_sub(2))
        .wrapping_sub(0x202);
    let stride = w.wrapping_sub(2) as i32;
    for dy in -2..=4i32 {
        if !(0..256).contains(&(cy + dy)) {
            continue;
        }
        for dx in -2..=4i32 {
            if !(0..256).contains(&(cx + dx)) || (dx == 0 && dy == 0) {
                continue;
            }
            let c = (exit as i32).wrapping_add(dy << 8).wrapping_add(dx) as u32 & 65535;
            if crate::frame_spawn::call(m, "TerrainEnCollision", &[c, 1])? & 255 != 0 {
                continue;
            }
            let index = stride.wrapping_mul(dy).wrapping_add(dx) as u32 & 65535;
            if crate::frame_core::call(m, "BatimentCollision", &[kind, index])? & 255 == 0 {
                continue;
            }
            let t = crate::frame_core::call(m, "EnginGetType", &[u & 65535])? & 255;
            if !(11..=15).contains(&t) || hangar_cell_free(m, kind, c)? {
                return Ok(c as u16);
            }
        }
    }
    Ok(none.unwrap_or(exit as u16))
}

/// BatimentModifyMap0x419e97, effect-only API: the owner's contract leaves
/// EAX/EDX loop scratch unspecified; game callers consume no register result.
pub fn structure_minimap(m: &mut MemoriaJogo, s: u16, active: u8) -> Result<()> {
    if s >= 1000 {
        return stop("BatimentModifyMap structure index", 0x419e97);
    }
    let e = structure(u32::from(s));
    let t = m.b(e + 8)?;
    if t >= 22 {
        return stop("BatimentModifyMap structure type", 0x419ec0);
    }
    let (w, h) = (m.w(0x495d8c + 52 * t)?, m.w(0x495d8e + 52 * t)?);
    let origin = m.w(e + 2)?;
    let (x, y) = (origin & 255, origin >> 8);
    for dy in 0..h {
        if y + dy >= 256 {
            continue;
        }
        for dx in 0..w {
            if x + dx < 256 {
                crate::player_sight::minimap(
                    m,
                    m.b(e + 4)? as u8,
                    (m.w(e + 2)? + (dy << 8) + dx) as u16,
                    if active != 0 { 6 } else { 7 },
                )?;
            }
        }
    }
    Ok(())
}
