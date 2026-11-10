//! Core VisaoMovimento: light reference counts, borders and minimap layers.
use crate::{frame::stop, player::Memory, MemoriaJogo, Result};
pub(crate) fn borders(m: &mut MemoriaJogo, grid: u32, c: u32, erase: bool) -> Result<()> {
    let (x, y) = (c & 255, c >> 8);
    let (w, h) = (m.w(0x497168)?, m.w(0x49716a)?);
    for (p, bit) in [
        (if x == 0 { c } else { c - 1 }, if x == 0 { 8 } else { 2 }),
        (if y == 0 { c } else { c - 256 }, if y == 0 { 4 } else { 1 }),
        (
            if x == w - 1 { c } else { c + 1 },
            if x == w - 1 { 2 } else { 8 },
        ),
        (
            if y == h - 1 { c } else { c + 256 },
            if y == h - 1 { 1 } else { 4 },
        ),
    ] {
        let b = m.b(grid + p)?;
        m.sb(grid + p, if erase { b & !bit } else { b | bit })?
    }
    Ok(())
}
fn visible(m: &MemoriaJogo, c: u32, lit: bool) -> Result<bool> {
    let local = m.b(0x508041)?;
    Ok(m.b(m.d((if lit { 0x507d88 } else { 0x507d78 }) + 4 * local)? + c)? & 31 == 0)
}
fn terrain_color(m: &MemoriaJogo, c: u32) -> Result<u32> {
    let object = m.b(m.d(0x507d74)? + c)?;
    if (0x1d..0x35).contains(&object) {
        return Ok(30);
    }
    let tile = m.w(m.d(0x507d70)? + 2 * c)?;
    if tile >= 702 {
        Ok(30)
    } else {
        m.b(0x509694 + tile)
    }
}
pub fn minimap(m: &mut MemoriaJogo, owner: u8, cell: u16, mode: u8) -> Result<u32> {
    let (w, h, mw, mh) = (
        m.w(0x497168)?,
        m.w(0x49716a)?,
        m.w(0x496fe0)?,
        m.w(0x496fe2)?,
    );
    if w == 0 || h == 0 {
        return stop("SetMiniMap map division", 0x46a3dc);
    }
    let (sx, sy) = ((mw / w).max(1), (mh / h).max(1));
    let c = u32::from(cell);
    let start = (((c >> 8) * mh / h) * mh + (c & 255) * mw / w) & 65535;
    for y in 0..sy {
        for x in 0..sx {
            let p = (start + y * 128 + x) & 65535;
            match mode {
                1 | 3 if u32::from(owner) == m.b(0x508041)? => {
                    let color = m.b(m.d(0x4e24b4)? + terrain_color(m, c)?)?;
                    m.sb(m.d(0x507d5c)? + p, color)?;
                }
                2 if u32::from(owner) == m.b(0x508041)? => {
                    let a = m.b(m.d(0x507d60)? + p)?;
                    let color = if a != 0 {
                        a
                    } else {
                        let b = m.b(m.d(0x507d64)? + p)?;
                        if b != 0 {
                            b
                        } else {
                            terrain_color(m, c)?
                        }
                    };
                    m.sb(m.d(0x507d5c)? + p, color)?;
                }
                1..=3 => {}
                4 | 6 => {
                    let color = (if owner == 200 {
                        30
                    } else {
                        m.b(0x4e24dc + u32::from(owner))?
                    } + if mode == 6 { 2 } else { 0 })
                        & 255;
                    m.sb(m.d(if mode == 4 { 0x507d60 } else { 0x507d64 })? + p, color)?;
                    if visible(m, c, true)? {
                        let a = if mode == 6 {
                            m.b(m.d(0x507d60)? + p)?
                        } else {
                            0
                        };
                        m.sb(m.d(0x507d5c)? + p, if a != 0 { a } else { color })?;
                    }
                }
                5 | 7 => {
                    m.sb(m.d(if mode == 5 { 0x507d60 } else { 0x507d64 })? + p, 0)?;
                    if visible(m, c, false)? {
                        let color = if visible(m, c, true)? {
                            let a = m.b(m.d(if mode == 5 { 0x507d64 } else { 0x507d60 })? + p)?;
                            if a != 0 {
                                a
                            } else {
                                terrain_color(m, c)?
                            }
                        } else {
                            m.b(m.d(0x4e24b4)? + terrain_color(m, c)?)?
                        };
                        m.sb(m.d(0x507d5c)? + p, color)?;
                    }
                }
                _ => return stop("SetMiniMap mode", 0x46a388),
            }
        }
    }
    Ok(sy)
}
/// Mode0 discovery (the body adds one to DX), mode1 illumination, mode2 extinction.
pub fn area(m: &mut MemoriaJogo, center: u16, radius: u32, owner: u8, mode: u32) -> Result<()> {
    if owner >= 4 {
        return stop("Terrain sight owner", 0x476851);
    }
    let radius = if mode == 0 {
        radius.wrapping_add(1) & 65535
    } else {
        radius
    };
    if radius > if mode == 0 { 24 } else { 20 } {
        return stop(
            "Terrain sight radius outside validated domain",
            if mode == 0 {
                0x476851
            } else if mode == 1 {
                0x476cb3
            } else {
                0x476e82
            },
        );
    }
    let (x, y) = (i32::from(center & 255), i32::from(center >> 8));
    let (w, h) = (m.w(0x497168)? as i32, m.w(0x49716a)? as i32);
    let rr = radius as i32;
    let limit = (radius * radius + u32::from(radius > 1)) & 65535;
    for dy in -rr..=rr {
        for dx in -rr..=rr {
            if x + dx < 0
                || x + dx >= w
                || y + dy < 0
                || y + dy >= h
                || ((dx * dx + dy * dy) as u32 & 65535) > limit
            {
                continue;
            }
            let c = ((y + dy) * 256 + x + dx) as u32;
            let p = u32::from(owner);
            if mode == 0 {
                let grid = m.d(0x507d78 + 4 * p)?;
                borders(m, grid, c, true)?;
                if m.b(grid + c)? != 0 {
                    minimap(m, owner, c as u16, 1)?;
                }
                m.sb(grid + c, m.b(grid + c)? & 0xef)?
            } else {
                let counts = m.d(0x507d98 + 4 * p)?;
                let grid = m.d(0x507d88 + 4 * p)?;
                let n = m.b(counts + c)?;
                if mode == 1 {
                    if n == 0 {
                        borders(m, grid, c, true)?;
                        m.sb(grid + c, m.b(grid + c)? & 0xef)?;
                        minimap(m, owner, c as u16, 2)?;
                    }
                    if n != 255 {
                        m.sb(counts + c, n + 1)?
                    }
                } else {
                    if n == 1 {
                        borders(m, grid, c, false)?;
                        m.sb(grid + c, m.b(grid + c)? | 16)?;
                        minimap(m, owner, c as u16, 3)?;
                    }
                    if n != 0 {
                        m.sb(counts + c, n - 1)?
                    }
                }
            }
        }
    }
    Ok(())
}
