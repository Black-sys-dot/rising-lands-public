//! Intact CalculAttaqueEngin building shot continuation and launch leaves.
//! Existing movement/aim/damage/render providers remain unchanged.
use crate::{frame::stop, player::unit, EngineError, FrameRegisters, MemoriaJogo, Result};

fn double(m: &MemoriaJogo, p: u32) -> Result<f64> {
    Ok(f64::from_bits(
        u64::from(m.u32(p)?) | (u64::from(m.u32(p + 4)?) << 32),
    ))
}

/// AngleCalcul4796cb with the original constants and final truncation to a word.
/// The attack caller supplies sectors8/32 and byte coordinates from cell words.
fn angle(m: &MemoriaJogo, from: u16, to: u16, sectors: u8) -> Result<u8> {
    let (x, y, tx, ty) = (from & 255, from >> 8, to & 255, to >> 8);
    let dx = f64::from(x.abs_diff(tx));
    let dy = f64::from(y.abs_diff(ty));
    let full = double(m, 0x494e0c)?;
    let half = double(m, 0x494e14)?;
    let step = full / f64::from(sectors);
    let mut degrees = if dy == 0.0 {
        0.0
    } else if dx == 0.0 {
        90.0
    } else {
        (dx / (dx * dx + dy * dy).sqrt()).acos() * half / f64::from_bits(0x400921fb54411744)
    };
    if x > tx {
        degrees = if y >= ty {
            degrees + half
        } else {
            half - degrees
        };
    } else if y > ty {
        degrees = full - degrees;
    }
    degrees += step * double(m, 0x494e1c)?;
    if degrees >= full {
        degrees += double(m, 0x494e24)?;
    }
    let sector = (degrees / step) as u16;
    Ok(
        ((u32::from(sector) % u32::from(sectors) + u32::from(sectors) - u32::from(sectors / 2))
            % u32::from(sectors)
            + 1) as u8,
    )
}

fn target(m: &mut MemoriaJogo, r: u32) -> Result<u16> {
    let b = m.u16(r + 0xb8)?;
    if m.u8(r + 0xb0)? <= 1 {
        let cell = crate::ai_building_attack::adjacent_cell(m, b, m.u16(r + 0x7c)?)?;
        if cell != 65535 {
            return Ok(cell);
        }
    }
    Ok(crate::frame_core::building_cell(m, u32::from(b))? as u16)
}

/// TypeEnginGetSortieProjectile4511a5. Native out-of-range headings still
/// index the declared table bytes; direction0 uses signed truncation toward0.
pub fn muzzle(m: &MemoriaJogo, kind: u8, heading: u8) -> Result<(u16, u16)> {
    let table = match kind {
        2 => 0x496af0,
        8 => 0x496b10,
        18 => 0x496b30,
        23 => 0x496b50,
        24 => 0x496b70,
        21 => 0x496b90,
        22 => 0x496bb0,
        15 => 0x496bd0,
        12 => 0x496c50,
        _ => return Ok((0, 0)), // original clears both output words before dispatch
    };
    let index = if matches!(kind, 12 | 15) {
        heading.wrapping_sub(1)
    } else {
        ((i32::from(heading) - 1) / 2) as u8
    };
    Ok((
        m.u16(table + 4 * u32::from(index))?,
        m.u16(table + 4 * u32::from(index) + 2)?,
    ))
}
/// AffCorpsEnginDecalage410fcc, word subtraction; types>25 return untouched.
pub fn body_offset(kind: u8, x: u16, y: u16) -> (u16, u16) {
    let (bx, by) = match kind {
        0 => (40, 22),
        1 => (23, 9),
        2 => (32, 16),
        3 | 25 => (45, 56),
        4 => (40, 32),
        5 => (11, 11),
        6 => (16, 20),
        7 => (26, 26),
        8 => (32, 30),
        9 => (19, 14),
        10 => (24, 12),
        11 => (41, 52),
        12 => (52, 48),
        13 => (55, 56),
        14 => (42, 45),
        15 => (49, 66),
        16 => (78, 54),
        17 => (79, 212),
        18 => (36, 48),
        19 => (16, 16),
        20 => (46, 54),
        21 => (23, 15),
        22 => (54, 54),
        23 => (22, 16),
        24 => (32, 32),
        _ => return (x, y),
    };
    (x.wrapping_sub(bx), y.wrapping_sub(by))
}

/// ProjectileLineaire46e049. Signed low-word tests with full32-bit negation.
pub fn line(m: &mut MemoriaJogo, p: u32) -> Result<()> {
    let x = m.u16(p + 12)?;
    let y = m.u16(p + 14)?;
    let to = m.u16(p + 0x16)?;
    let mut dx = i32::from(to & 255) * 32 + 16 - i32::from(x);
    let mut dy = i32::from(to >> 8) * 32 + 16 - i32::from(y);
    let sx = if (dx as i16) < 0 {
        dx = dx.wrapping_neg();
        -1i32
    } else {
        1
    };
    let sy = if (dy as i16) < 0 {
        dy = dy.wrapping_neg();
        -8192i32
    } else {
        8192
    };
    let (major, minor, axial) = if (dx as i16) > (dy as i16) {
        (dx, dy, sx)
    } else {
        (dy, dx, sy)
    };
    m.set_u16(p + 0x34, major as u16)?;
    m.set_u16(p + 0x2c, axial as u16)?;
    m.set_u16(p + 0x32, minor.wrapping_mul(2) as u16)?;
    m.set_u16(p + 0x2a, sx.wrapping_add(sy) as u16)?;
    m.set_u16(
        p + 0x30,
        minor.wrapping_mul(2).wrapping_sub(major.wrapping_mul(2)) as u16,
    )?;
    m.set_u32(
        p + 8,
        (i32::from(y as i16).wrapping_shl(13)).wrapping_add(i32::from(x as i16)) as u32,
    )?;
    m.set_u16(p + 0x2e, minor.wrapping_mul(2).wrapping_sub(major) as u16)?;
    if m.u8(p + 0x23)? != 0 {
        let divisor = i32::from(major as i16);
        m.set_u16(
            p + 0x28,
            if divisor == 0 {
                12800
            } else {
                (12800 / divisor) as u16
            },
        )?;
        m.set_u32(p + 0x24, 0)?;
    }
    Ok(())
}

/// ProjectileSet46e208. Arguments: slot,kind,from,to,mode,unit,heading,dx,dy,flag.
/// Common stores and callback order follow intact instructions, including the
/// newborn=0 arm for kind9 and untouched nonselected bytes.
pub fn projectile_set(m: &mut MemoriaJogo, a: [u32; 10]) -> Result<()> {
    let p = m.u32(0x5080f0)?.wrapping_add(u32::from(a[0] as u16) * 56);
    let kind = a[1] as u8;
    let from = a[2] as u16;
    let flag = a[9] as u8;
    m.set_u8(p, kind)?;
    m.set_u16(p + 0x14, from)?;
    m.set_u16(p + 0x16, a[3] as u16)?;
    m.set_u8(p + 0x23, flag)?;
    m.set_u16(
        p + 12,
        (from & 255).wrapping_mul(32).wrapping_add(a[7] as u16),
    )?;
    m.set_u8(p + 1, 1)?;
    m.set_u32(p + 0x24, 0)?;
    m.set_u16(p + 0x28, 0)?;
    m.set_u8(p + 2, 1)?;
    m.set_u8(p + 0x18, 0)?;
    m.set_u8(p + 3, 0)?;
    m.set_u16(p + 0x1a, 12)?;
    m.set_u16(
        p + 14,
        (from >> 8).wrapping_mul(32).wrapping_add(a[8] as u16),
    )?;
    m.set_u16(p + 0x10, a[7] as u16)?;
    m.set_u16(p + 0x12, a[8] as u16)?;
    m.set_u8(p + 0x1f, a[4] as u8)?;
    m.set_u16(p + 0x20, a[5] as u16)?;
    m.set_u8(p + 0x22, a[6] as u8)?;
    m.set_u8(p + 0x19, 0)?;
    m.set_u16(p + 0x1c, 0)?;
    match kind {
        1..=5 | 9..=13 | 17 | 20..=23 | 29 | 30 => {
            if matches!(kind, 1 | 9 | 17 | 20..=23) {
                m.set_u8(p + 1, 0)?;
            }
            if matches!(kind, 9..=13) {
                m.set_u16(p + 0x1a, 5)?;
            }
            let count = match kind {
                1 | 17 | 20..=23 => Some(15),
                2 => Some(10),
                3 => Some(8),
                4 => Some(4),
                9..=13 => Some(2),
                _ => None,
            };
            if let Some(count) = count {
                m.set_u8(p + 4, count)?;
            }
            m.set_u8(p + 0x19, u8::from(matches!(kind,1..=4|9..=12)))?;
            line(m, p)?;
            if kind == 9 {
                m.set_u16(p + 0x1c, 5)?;
            } else if matches!(kind, 1 | 17 | 20..=23) {
                m.set_u16(p + 0x1c, 10)?;
            }
            if matches!(kind, 20..=23 | 29 | 30) {
                m.set_u16(p + 0x1a, 20)?;
            }
        }
        15 | 16 if flag != 0 => m.set_u32(p + 0x24, 12800)?,
        26 => {
            if a[4] as u8 != 1 {
                return stop("ProjectileSet damage mode", 0x450874);
            }
            crate::chain_damage::impact(m, a[3] as u16, a[5] as u16)?;
        }
        28 => return stop("ProjectileSet SortilegeGetRayonEffet", 0x474734),
        _ => {} // native jump table return, with common stores already complete
    }
    Ok(())
}
/// ProjectileInit46e44c. Full pool is an actual native no-allocation return.
pub fn launch(m: &mut MemoriaJogo, a: [u32; 9]) -> Result<()> {
    let pool = m.u32(0x5080f0)?;
    let mut slot = 0;
    while slot < 100 && m.u8(pool.wrapping_add(slot * 56 + 2))? != 0 {
        slot += 1;
    }
    if slot == 100 {
        return Ok(());
    }
    let mut args = [0; 10];
    args[0] = slot;
    args[1..].copy_from_slice(&a);
    projectile_set(m, args)
}
/// CalculDirCase4770ee: native equality-only edge checks and default return.
fn next_cell(m: &MemoriaJogo, c: u16, d: u8) -> Result<u16> {
    let x = i32::from(c & 255);
    let y = i32::from(c >> 8);
    let right = i32::from(m.u16(0x497168)?) - 1;
    let bottom = i32::from(m.u16(0x49716a)?) - 1;
    let blocked = match d {
        1 => x == 0,
        3 => x == 0 || y == 0,
        5 => y == 0,
        7 => y == 0 || x == right,
        9 => x == right,
        11 => x == right || y == bottom,
        13 => y == bottom,
        15 => x == 0 || y == bottom,
        _ => return Ok(c),
    };
    if blocked {
        return Ok(65535);
    }
    Ok(match d {
        1 => c.wrapping_sub(1),
        3 => c.wrapping_sub(257),
        5 => c.wrapping_sub(256),
        7 => c.wrapping_sub(255),
        9 => c.wrapping_add(1),
        11 => c.wrapping_add(257),
        13 => c.wrapping_add(256),
        15 => c.wrapping_add(255),
        _ => c,
    })
}
fn shot(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    let cell = target(m, r)?;
    if m.u8(r + 2)? == 24 {
        return stop("SortilegeEffet", 0x473510);
    }
    if m.u8(r + 2)? != 1 {
        return Ok(());
    }
    let kind = m.u8(r + 1)?;
    let from = m.u16(r + 8)?;
    let light = m.u32(0x507d78 + 4 * u32::from(m.u8(0x508041)?))?;
    if m.u8(light + u32::from(from))? & 31 == 0 {
        let volume = crate::player_orders::unit_volume(m, u32::from(u))?;
        m.set_u8(r + 0xf9, 0)?;
        if m.u32(r + 0x104)? == u32::MAX {
            crate::player::sound(
                m,
                0x4525a1,
                true,
                [0x50, (u32::from(kind) << 16) | 0x2300, r + 0x104, 0],
                Some(volume),
            )?;
        }
    }
    if matches!(kind, 14 | 16) && m.u8(r + 0xda)? != 0 {
        m.set_u8(r + 0xda, m.u8(r + 0xda)?.wrapping_sub(1))?;
    }
    if m.u8(r + 0x78)? == 0 {
        let hit = if matches!(kind, 3 | 11..=16 | 25) {
            cell
        } else {
            next_cell(m, from, m.u8(r + 0x91)?)?
        };
        if kind == 16 {
            launch(
                m,
                [
                    8,
                    u32::from(hit),
                    u32::from(hit),
                    1,
                    u32::from(u),
                    0,
                    0,
                    0,
                    0,
                ],
            )?;
        }
        return crate::chain_damage::impact(m, hit, u);
    }
    let (dx, dy) = if matches!(kind, 1 | 11 | 21) {
        (0, 0)
    } else {
        let heading = m.u8(r + if matches!(kind, 12 | 15) { 0x94 } else { 0x91 })?;
        let (x, y) = muzzle(m, kind, heading)?;
        body_offset(kind, x, y)
    };
    let heading = u32::from(if kind == 11 {
        angle(m, from, cell, 32)?
    } else {
        m.u8(r + 0x91)?
    });
    if !matches!(kind, 3 | 25) {
        launch(
            m,
            [
                u32::from(m.u8(r + 0x79)?),
                u32::from(from),
                u32::from(cell),
                1,
                u32::from(u),
                heading,
                dx as i16 as i32 as u32,
                dy as i16 as i32 as u32,
                0,
            ],
        )?;
    }
    if matches!(kind, 3 | 11 | 25) {
        launch(
            m,
            [
                if kind == 25 { 33 } else { 26 },
                u32::from(cell),
                u32::from(cell),
                1,
                u32::from(u),
                heading,
                0,
                0,
                0,
            ],
        )?;
    }
    Ok(())
}
fn tail(m: &mut MemoriaJogo, r: u32) -> Result<()> {
    if m.u8(r + 0xa9)? == 0 && m.u8(r + 0xa4)? == 0 {
        let kind = m.u8(r + 1)?;
        let remaining = m.u8(r + 0x8b)?;
        if remaining == 0 {
            m.set_u8(r + 0x8a, 0)?;
            if kind != 16 {
                m.set_u8(r + 0xa4, 0)?;
            }
            m.set_u16(r + 0x102, 0)?;
        } else {
            m.set_u8(r + 0x8b, remaining.wrapping_sub(1))?;
            if kind == 3 {
                m.set_u8(r + 0xa4, 5)?;
            }
            if kind == 11 {
                m.set_u8(r + 0xa4, 0)?;
            }
        }
    }
    Ok(())
}
pub fn approach(m: &mut MemoriaJogo, u: u16) -> Result<()> {
    match crate::ai_attack_aim::approach(m, u) {
        Err(EngineError::UnportedCallee {
            name: "CalculAttaqueEngin building shot target",
            address: 0x452503,
        }) => {
            let r = unit(m, u32::from(u))?;
            shot(m, u, r)?;
            tail(m, r)
        }
        result => result,
    }
}
pub fn state1(m: &mut MemoriaJogo, u: u16, r: u32) -> Result<()> {
    crate::chain_attack::state1(m, u, r)
}
/// Void effects transport paired with native PUSH EAX/CALL/POP.
pub fn entry(m: &mut MemoriaJogo, r: FrameRegisters) -> Result<FrameRegisters> {
    approach(m, r.eax as u16)?;
    Ok(FrameRegisters {
        esp: r.esp.wrapping_add(4),
        ..r
    })
}
