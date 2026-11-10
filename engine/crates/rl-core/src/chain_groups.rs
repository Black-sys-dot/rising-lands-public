//! Core visao_grupos group selection, pinned 54d148d; original entries 4726f7/472769.
use crate::{
    frame::stop,
    player::{self, alive, unit, Memory},
    FrameRegisters as R, MemoriaJogo, Result,
};
fn voice(m: &mut MemoriaJogo, u: u32, mode: u32) -> Result<(u32, u32)> {
    let rec = unit(m, u)?;
    let cell = m.w(rec + 8)?;
    let local = m.b(0x508041)?;
    let grid = m.d(0x507d78 + 4 * local)?;
    if m.b(grid + cell)? & 31 != 0 {
        return Ok((cell & 0xff00, grid));
    }
    let handle = m.d(rec + 0x104)?;
    let code = (m.b(rec + 1)? << 16) | (mode << 8);
    let volume = crate::player_orders::unit_volume(m, u)?;
    m.sb(rec + 0xf9, 0)?;
    if handle != u32::MAX {
        return Ok((volume, handle));
    }
    let eax = player::sound(
        m,
        0x440de7,
        true,
        [0x50, code, rec + 0x104, 0],
        Some(volume),
    )?;
    Ok((eax, code))
}
pub fn entry(m: &mut MemoriaJogo, name: &str, mut r: R) -> Result<R> {
    let (p, g) = (r.eax & 255, r.edx & 255);
    let mut eax = r.eax;
    let mut edx = r.edx;
    match name {
        "SauveSelectionGroupe" => {
            for u in 0..800 {
                let rec = unit(m, u)?;
                if !alive(m, u)? {
                    eax = 0;
                    continue;
                }
                eax = (m.d(0x4e19a0)? & !255) | m.b(rec)?;
                if eax & 255 != p {
                    continue;
                }
                if m.b(rec + 0xbc)? == g {
                    m.sb(rec + 0xbc, 0)?;
                }
                eax = m.d(0x4e19a0)?;
                if m.b(rec + 0xbd)? != 0 {
                    m.sb(rec + 0xbc, g)?;
                }
            }
            edx = 800;
        }
        "RestaureSelectionGroupe" => {
            let mut spoke = false;
            for u in 0..800 {
                let rec = unit(m, u)?;
                if !alive(m, u)? {
                    eax = 0;
                    continue;
                }
                eax = (m.d(0x4e19a0)? & !255) | m.b(rec)?;
                if eax & 255 != p {
                    continue;
                }
                if m.b(rec + 0xbd)? != 0 {
                    m.sb(rec + 0xbd, 0)?;
                    crate::player_orders::call(
                        m,
                        "RetireSelectionEngin",
                        [u, m.b(rec)?, 0, 0],
                        0x4727ff,
                    )?;
                }
                eax = u;
                edx = u * 276;
                if m.b(rec + 0xbc)? != g {
                    continue;
                }
                eax = m.b(rec + 2)?;
                if eax == 14 {
                    continue;
                }
                let base = 0x508728 + (p << 9);
                eax = m.w(base)?;
                if eax < 255 {
                    m.sw(base, eax + 1)?;
                    m.sw(base + 2 * (eax + 1), u)?;
                    eax = m.d(0x4e19a0)?;
                    m.sb(rec + 0xbd, 1)?;
                }
                if spoke {
                    continue;
                }
                let kind = m.b(rec + 1)?;
                eax = kind;
                if kind == 10 {
                    continue;
                }
                eax = p;
                if p != m.b(0x508041)? {
                    continue;
                }
                if (11..=16).contains(&kind) {
                    eax = 1;
                    continue;
                }
                spoke = true;
                let mode = if kind == 7 && matches!(m.b(rec + 2)?, 11 | 15) {
                    player::sound(m, 0x4728e4, false, [rec + 0x104, 0x16, 0, u], None)?;
                    0x16
                } else {
                    8
                };
                (eax, edx) = voice(m, u, mode)?;
            }
        }
        _ => return stop("Unknown group entry", 0x4726f7),
    }
    r.eax = eax;
    r.edx = edx;
    Ok(r)
}
fn half(v: u32) -> (u32, u32) {
    let s = v as i32;
    ((s / 2) as u32, if s < 0 { u32::MAX } else { 0 })
}
pub fn centre(m: &mut MemoriaJogo, mut r: R) -> Result<R> {
    let (p, g, px, py) = (r.eax & 255, r.ecx & 255, r.edx, r.ebx);
    let mut found = None;
    for u in 0..800 {
        let rec = unit(m, u)?;
        if alive(m, u)? && m.b(rec + 0xbc)? == g && m.b(rec)? == p {
            found = Some((u, m.w(rec + 8)?));
            break;
        }
    }
    let (minx, miny, maxx, maxy, bx) = if let Some((u, cell)) = found {
        let (x, y) = (cell & 255, cell >> 8);
        (x, y, x, y, u)
    } else {
        let y = m.w(py)?;
        let x = (px & 0xffff0000) | m.w(px)?;
        let bx = m.w(0x508726)?;
        (
            x,
            y,
            m.w(0x508724)?.wrapping_add(x).wrapping_sub(1),
            y.wrapping_add(bx).wrapping_sub(1),
            bx,
        )
    };
    let cx = minx & 65535;
    let (a, _) = half((maxx & 65535).wrapping_sub(cx));
    m.sw(px, a.wrapping_add(cx))?;
    r.ecx = miny & 65535;
    let (a, d) = half((maxy & 65535).wrapping_sub(r.ecx));
    r.eax = a.wrapping_add(r.ecx);
    r.edx = d;
    r.ebx = bx;
    m.sw(py, r.eax)?;
    Ok(r)
}
