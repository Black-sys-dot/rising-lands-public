//! Intact DecouvreTerrain/IllumineTerrain/EteintTerrain and their sight leaves.
//! Original-address memory only. The signed-word loop's nonterminating domain
//! remains an explicit entry dependency; no clipping shortcut replaces a scan.
use crate::{frame::stop, EngineError, FrameRegisters, MemoriaJogo, Result};
#[cfg(feature = "validation-trace")]
thread_local! {
    static CALLS: std::cell::RefCell<Vec<[u32; 5]>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(feature = "validation-trace")]
pub fn take_validation_calls() -> Vec<[u32; 5]> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
fn entered(address: u32, args: [u32; 4]) {
    #[cfg(feature = "validation-trace")]
    CALLS.with(|c| {
        c.borrow_mut()
            .push([address, args[0], args[1], args[2], args[3]])
    });
    #[cfg(not(feature = "validation-trace"))]
    let _ = (address, args);
}
fn grid(m: &MemoriaJogo, table: u32, owner: u8) -> Result<u32> {
    m.u32(table + 4 * u32::from(owner))
}
fn byte(m: &MemoriaJogo, table: u32, owner: u8, cell: u16) -> Result<u8> {
    m.u8(grid(m, table, owner)?.wrapping_add(u32::from(cell)))
}
fn bits(
    m: &mut MemoriaJogo,
    table: u32,
    owner: u8,
    offset: u32,
    bit: u8,
    erase: bool,
) -> Result<()> {
    let p = grid(m, table, owner)?.wrapping_add(offset);
    let v = m.u8(p)?;
    m.set_u8(p, if erase { v & !bit } else { v | bit })
}
fn borders(m: &mut MemoriaJogo, cell: u16, owner: u8, table: u32, erase: bool) -> Result<()> {
    let c = u32::from(cell);
    let (x, y) = (c & 255, c >> 8);
    bits(
        m,
        table,
        owner,
        if x == 0 { c } else { c.wrapping_sub(1) },
        if x == 0 { 8 } else { 2 },
        erase,
    )?;
    bits(
        m,
        table,
        owner,
        if y == 0 { c } else { c.wrapping_sub(256) },
        if y == 0 { 4 } else { 1 },
        erase,
    )?;
    let right = x == u32::from(m.u16(0x497168)?).wrapping_sub(1);
    bits(
        m,
        table,
        owner,
        if right { c } else { c.wrapping_add(1) },
        if right { 2 } else { 8 },
        erase,
    )?;
    let bottom = y == u32::from(m.u16(0x49716a)?).wrapping_sub(1);
    bits(
        m,
        table,
        owner,
        if bottom { c } else { c.wrapping_add(256) },
        if bottom { 1 } else { 4 },
        erase,
    )
}
fn terrain_color(m: &MemoriaJogo, cell: u16) -> Result<u8> {
    entered(0x4765f4, [u32::from(cell), 0, 0, 0]);
    let c = u32::from(cell);
    let object = m.u8(m.u32(0x507d74)?.wrapping_add(c))?;
    if (0x1d..0x35).contains(&object) {
        return Ok(30);
    }
    let tile = m.u16(m.u32(0x507d70)?.wrapping_add(2 * c))?;
    if tile >= 702 {
        Ok(30)
    } else {
        m.u8(0x509694 + u32::from(tile))
    }
}
// SetMiniMap's actual mode1/2/3 bodies. These modes do not call sight queries.
fn minimap(m: &mut MemoriaJogo, owner: u8, cell: u16, mode: u8) -> Result<()> {
    entered(
        0x46a388,
        [u32::from(owner), u32::from(mode), u32::from(cell), 0],
    );
    let (x, y) = (u32::from(cell & 255), u32::from(cell >> 8));
    let mw = u32::from(m.u16(0x496fe0)?);
    let w = u32::from(m.u16(0x497168)?);
    if w == 0 {
        return stop("SetMiniMap signed division", 0x46a3dc);
    }
    let left = x * mw / w;
    let mh = u32::from(m.u16(0x496fe2)?);
    let h = u32::from(m.u16(0x49716a)?);
    if h == 0 {
        return stop("SetMiniMap signed division", 0x46a401);
    }
    let top = y * mh / h;
    let (sx, sy) = ((mw / w).max(1), (mh / h).max(1));
    let start = top.wrapping_mul(mh).wrapping_add(left);
    for row in 0..sy {
        for column in 0..sx {
            let p = start.wrapping_add(row * 128).wrapping_add(column) as u16;
            if owner == m.u8(0x508041)? {
                let color = if mode == 2 {
                    let a = m.u8(m.u32(0x507d60)?.wrapping_add(u32::from(p)))?;
                    if a != 0 {
                        a
                    } else {
                        let b = m.u8(m.u32(0x507d64)?.wrapping_add(u32::from(p)))?;
                        if b != 0 {
                            b
                        } else {
                            terrain_color(m, cell)?
                        }
                    }
                } else {
                    let color = terrain_color(m, cell)?;
                    m.u8(m.u32(0x4e24b4)?.wrapping_add(u32::from(color)))?
                };
                m.set_u8(m.u32(0x507d5c)?.wrapping_add(u32::from(p)), color)?;
            }
        }
    }
    Ok(())
}
/// Full native cell bodies, including repeated pointer/count reads after SetMiniMap.
/// Owner4 does not change counts; owner200 still performs the first native read.
pub fn cell(m: &mut MemoriaJogo, c: u16, owner: u8, mode: u32) -> Result<()> {
    let address = match mode {
        0 => 0x47670a,
        1 => 0x476968,
        2 => 0x476adc,
        _ => return Err(EngineError::UnknownRoutine(mode)),
    };
    entered(address, [u32::from(c), u32::from(owner), 0, 0]);
    if mode == 0 {
        borders(m, c, owner, 0x507d78, true)?;
        if byte(m, 0x507d78, owner, c)? != 0 {
            minimap(m, owner, c, 1)?;
        }
        return bits(m, 0x507d78, owner, u32::from(c), 16, true);
    }
    let n = byte(m, 0x507d98, owner, c)?;
    if n == if mode == 1 { 0 } else { 1 } {
        borders(m, c, owner, 0x507d88, mode == 1)?;
        bits(m, 0x507d88, owner, u32::from(c), 16, mode == 1)?;
        minimap(m, owner, c, if mode == 1 { 2 } else { 3 })?;
    }
    if owner != 200 {
        let p = grid(m, 0x507d98, owner)?.wrapping_add(u32::from(c));
        let n = m.u8(p)?;
        if owner != 4 && if mode == 1 { n != 255 } else { n != 0 } {
            m.set_u8(
                p,
                if mode == 1 {
                    n.wrapping_add(1)
                } else {
                    n.wrapping_sub(1)
                },
            )?;
        }
    }
    Ok(())
}
fn distance(dy: i32, dx: i32) -> u16 {
    let (a, d) = (u32::from(dy as u16), u32::from(dx as u16));
    entered(0x47954c, [a, d, 0, 0]);
    a.wrapping_mul(a).wrapping_add(d.wrapping_mul(d)) as u16
}
/// AX=center, EDX=radius, BL=owner. Discovery increments full EDX first.
/// Signed-word loops terminate only when the effective low word is <32767.
/// Every first-pass store and second-pass read retains original row/cell order.
pub fn area(m: &mut MemoriaJogo, center: u16, radius: u32, owner: u8, mode: u32) -> Result<()> {
    let address = match mode {
        0 => 0x476851,
        1 => 0x476cb3,
        2 => 0x476e82,
        _ => return Err(EngineError::UnknownRoutine(mode)),
    };
    entered(address, [u32::from(center), radius, u32::from(owner), 0]);
    let radius = if mode == 0 {
        radius.wrapping_add(1)
    } else {
        radius
    } as u16;
    if radius >= 32767 {
        return stop(
            "Terrain sight nonterminating signed-word scan outside bounded domain",
            address,
        );
    }
    let rr = i32::from(radius);
    let limit = radius
        .wrapping_mul(radius)
        .wrapping_add(u16::from(radius >= 2));
    let (x, y) = (i32::from(center & 255), i32::from(center >> 8));
    for pass in 0..if mode == 0 { 1 } else { 2 } {
        for dy in -rr..=rr {
            // Discovery tests the wrapped signed absolute row, the others add
            // signed dy to unsigned center Y before checking map dimensions.
            let row = if mode == 0 {
                i32::from((y + dy) as i16)
            } else {
                y + dy
            };
            if row < 0 || row >= i32::from(m.u16(0x49716a)?) {
                continue;
            }
            for dx in -rr..=rr {
                // EteintTerrain first pass calls Distance before horizontal bounds.
                if mode == 2 && pass == 0 && distance(dy, dx) > limit {
                    continue;
                }
                let column = if mode == 0 {
                    i32::from((x + dx) as i16)
                } else {
                    x + dx
                };
                if column < 0 || column >= i32::from(m.u16(0x497168)?) {
                    continue;
                }
                if !(mode == 2 && pass == 0) && distance(dy, dx) > limit {
                    continue;
                }
                let c = (i32::from(center) + 256 * dy + dx) as u16;
                if pass == 0 {
                    cell(m, c, owner, mode)?;
                } else {
                    let _ = byte(m, 0x507d88, owner, c)? & 16;
                }
            }
        }
    }
    Ok(())
}
/// The four original boolean query aliases preserve EAX's cell high byte and
/// return the raw indexed table pointer in EDX. No owner normalization occurs.
pub fn query(m: &MemoriaJogo, address: u32, mut r: FrameRegisters) -> Result<FrameRegisters> {
    let (table, mask) = match address {
        0x4766d6 => (0x507d78, 16),
        0x4766f0 => (0x507d78, 31),
        0x47693a => (0x507d88, 16),
        0x476951 => (0x507d88, 31),
        _ => return Err(EngineError::UnknownRoutine(address)),
    };
    r.edx = grid(m, table, r.edx as u8)?;
    r.eax &= 65535;
    let v = m.u8(r.edx.wrapping_add(r.eax))?;
    r.eax = (r.eax & !255) | u32::from(v & mask == 0);
    Ok(r)
}
